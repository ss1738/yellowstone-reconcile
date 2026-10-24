//! Bridge from one upstream `BlockStream` per connection into the shared reconciler.
//! Feed the outputs of independent streams; never merge raw events before reconstruction.

use crate::{
    BlockHash, BlockMeta, Commitment, Config, Error, Outcome, ProviderId, Reconciler, SealedBlock,
    Session,
};
use std::collections::BTreeMap;
use yellowstone_block_machine::stream::{Block, BlockEventStore, BlockMachineOutput};

#[derive(Debug)]
pub enum BridgeError {
    Core(Error),
    BufferCapacity,
    EventCapacity,
    ConflictingBankMetadata,
}
impl From<Error> for BridgeError {
    fn from(value: Error) -> Self {
        Self::Core(value)
    }
}
impl std::fmt::Display for BridgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for BridgeError {}

/// Retains sealed payloads until their upstream commitment notification arrives.
/// Limits bound block and event counts, not event byte sizes. Transport must bound message sizes.
pub struct UpstreamBridge<S> {
    core: Reconciler<Block<S>>,
    pending: BTreeMap<(Session, u64), Block<S>>,
    max_pending: usize,
    max_events_per_block: usize,
    upstream_fork_reports: u64,
}

pub fn block_meta<S>(block: &Block<S>) -> BlockMeta {
    BlockMeta {
        slot: block.slot,
        blockhash: BlockHash(block.blockhash),
        parent_slot: block.parent_slot,
        parent_blockhash: BlockHash(block.parent_blockhash),
        entry_count: block.entry_count,
        executed_transaction_count: block.executed_transaction_count,
    }
}

fn commitment(value: solana_commitment_config::CommitmentLevel) -> Commitment {
    match value {
        solana_commitment_config::CommitmentLevel::Finalized => Commitment::Finalized,
        solana_commitment_config::CommitmentLevel::Confirmed => Commitment::Confirmed,
        _ => Commitment::Processed,
    }
}

impl<S: BlockEventStore> UpstreamBridge<S> {
    pub fn new(
        config: Config,
        max_pending: usize,
        max_events_per_block: usize,
    ) -> Result<Self, BridgeError> {
        if max_pending == 0 || max_events_per_block == 0 {
            return Err(Error::InvalidConfig.into());
        }
        Ok(Self {
            core: Reconciler::new(config)?,
            pending: BTreeMap::new(),
            max_pending,
            max_events_per_block,
            upstream_fork_reports: 0,
        })
    }
    pub fn connect(&mut self, provider: ProviderId) -> Result<Session, BridgeError> {
        let session = self.core.connect(provider)?;
        self.pending.retain(|(old, _), _| old.provider != provider);
        Ok(session)
    }
    pub fn core(&self) -> &Reconciler<Block<S>> {
        &self.core
    }
    pub fn pending_blocks(&self) -> usize {
        self.pending.len()
    }
    pub fn upstream_fork_reports(&self) -> u64 {
        self.upstream_fork_reports
    }

    /// On reconnect discard the old BlockStream and call connect, creating a new session.
    /// A FrozenBlock alone does not qualify: the following commitment update supplies its level.
    pub fn ingest(
        &mut self,
        session: Session,
        event: BlockMachineOutput<S>,
    ) -> Result<Option<Outcome<Block<S>>>, BridgeError> {
        self.core.validate_session(session)?;
        match event {
            BlockMachineOutput::FrozenBlock(block) => {
                if block.slot < self.core.first_slot() {
                    return Err(Error::StaleSlot {
                        slot: block.slot,
                        first_slot: self.core.first_slot(),
                    }
                    .into());
                }
                if block.events.len() > self.max_events_per_block {
                    return Err(BridgeError::EventCapacity);
                }
                let key = (session, block.bank_id);
                if let Some(existing) = self.pending.get(&key) {
                    if block_meta(existing) != block_meta(&block) {
                        return Err(BridgeError::ConflictingBankMetadata);
                    }
                } else if self.pending.len() >= self.max_pending {
                    return Err(BridgeError::BufferCapacity);
                }
                self.pending.insert(key, block);
                Ok(None)
            }
            BlockMachineOutput::SlotCommitmentUpdate(update) => {
                let level = commitment(update.commitment);
                if level < self.core.minimum_commitment() {
                    return Ok(None);
                }
                let key = (session, update.bank_id);
                // Later commitment updates for an already submitted bank have no payload to release.
                let Some(block) = self.pending.get(&key) else {
                    return Ok(None);
                };
                if block.slot != update.slot {
                    return Err(BridgeError::ConflictingBankMetadata);
                }
                let meta = block_meta(block);
                // Check capacity before removing the payload: a caller can retry after advancing.
                if !self.core.slots.contains_key(&meta.slot) {
                    if self.core.stats().retained_slots >= self.core.config.max_retained_slots {
                        return Err(Error::SlotCapacity.into());
                    }
                    if self.core.stats().pending_blocks >= self.core.config.max_pending_blocks {
                        return Err(Error::PendingCapacity.into());
                    }
                }
                let block = self.pending.remove(&key).expect("payload checked above");
                Ok(Some(self.core.observe(
                    session,
                    SealedBlock {
                        meta,
                        commitment: level,
                        payload: block,
                    },
                )?))
            }
            BlockMachineOutput::BankDiscarded(event) => {
                self.pending.remove(&(session, event.bank_id));
                Ok(None)
            }
            BlockMachineOutput::ForkDetected(_) => {
                // This slot-level diagnostic is not a bank discard. Upstream issue #21 can
                // report it for a repaired canonical lineage. Specific discards arrive separately.
                self.upstream_fork_reports = self.upstream_fork_reports.saturating_add(1);
                Ok(None)
            }
            BlockMachineOutput::DeadBlockDetected(event) => {
                for bank in event.bank_ids {
                    self.pending.remove(&(session, bank));
                }
                Ok(None)
            }
        }
    }

    pub fn advance(&mut self, before_slot: u64) -> Result<crate::Pruned, BridgeError> {
        let mut report = self.core.advance(before_slot)?;
        for block in self
            .pending
            .values()
            .filter(|block| block.slot < before_slot)
        {
            report.pending_slots.push(block.slot);
        }
        self.pending.retain(|_, block| block.slot >= before_slot);
        report.pending_slots.sort_unstable();
        report.pending_slots.dedup();
        Ok(report)
    }
}
