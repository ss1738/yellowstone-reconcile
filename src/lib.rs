//! Reconcile complete, commitment-qualified blocks from independent connections.
//!
//! Every connection must reconstruct independently. This library accepts only sealed blocks,
//! never optimistic account/transaction fragments. Provider agreement is a trust policy,
//! not cryptographic block verification. See docs/CORRECTNESS.md for the contract.
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    str::FromStr,
};

#[cfg(feature = "upstream")]
pub mod upstream;

/// A block identity in binary form, serialized as 64 hexadecimal characters for replay files.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct BlockHash(pub [u8; 32]);

impl fmt::Display for BlockHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}
impl From<BlockHash> for String {
    fn from(value: BlockHash) -> Self {
        value.to_string()
    }
}
impl TryFrom<String> for BlockHash {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}
impl FromStr for BlockHash {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.len() != 64 || !value.is_ascii() {
            return Err("hash must be 64 ASCII hex characters".into());
        }
        let mut bytes = [0; 32];
        for (i, byte) in bytes.iter_mut().enumerate() {
            *byte =
                u8::from_str_radix(&value[i * 2..i * 2 + 2], 16).map_err(|_| "invalid hex hash")?;
        }
        Ok(Self(bytes))
    }
}

/// Stable operator-assigned identity. Two endpoints of the same provider should share an id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ProviderId(pub u16);

/// Bank ids are scoped to this exact connection generation, never shared across providers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Session {
    pub provider: ProviderId,
    pub generation: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Commitment {
    Processed,
    Confirmed,
    Finalized,
}

/// Metadata that must agree across providers. Local bank ids and transport timestamps are excluded.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BlockMeta {
    pub slot: u64,
    pub blockhash: BlockHash,
    pub parent_slot: u64,
    pub parent_blockhash: BlockHash,
    pub entry_count: u64,
    pub executed_transaction_count: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SealedBlock<P> {
    pub meta: BlockMeta,
    pub commitment: Commitment,
    /// Opaque content from the first accepted provider. No cross-provider content union is performed.
    pub payload: P,
}

#[derive(Clone, Debug)]
pub struct Config {
    pub minimum_commitment: Commitment,
    pub minimum_providers: usize,
    pub max_providers: usize,
    pub max_retained_slots: usize,
    pub max_pending_blocks: usize,
    pub first_slot: u64,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            minimum_commitment: Commitment::Finalized,
            minimum_providers: 2,
            max_providers: 8,
            max_retained_slots: 4096,
            max_pending_blocks: 128,
            first_slot: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Outcome<P> {
    Pending {
        slot: u64,
        blockhash: BlockHash,
        agreeing_providers: usize,
    },
    Emitted {
        block: SealedBlock<P>,
        providers: Vec<ProviderId>,
    },
    Duplicate {
        slot: u64,
        blockhash: BlockHash,
    },
    /// Stop committing this slot downstream. An earlier emission is not automatically reversed.
    Conflict {
        slot: u64,
        expected: BlockMeta,
        received: BlockMeta,
        previously_emitted: bool,
    },
    Quarantined {
        slot: u64,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidConfig,
    ProviderCapacity,
    UnknownSession,
    SessionExhausted,
    BelowCommitment,
    InvalidBlock,
    StaleSlot { slot: u64, first_slot: u64 },
    SlotCapacity,
    PendingCapacity,
    CheckpointRegression,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stats {
    pub emitted: u64,
    pub duplicates: u64,
    pub conflicts: u64,
    pub pending_blocks: usize,
    pub retained_slots: usize,
}

/// Result of explicit retention pruning. Discarded pending slots must trigger a backfill/alert.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pruned {
    pub before_slot: u64,
    pub pending_slots: Vec<u64>,
    pub quarantined_slots: Vec<u64>,
}

struct SlotState<P> {
    meta: BlockMeta,
    providers: BTreeSet<ProviderId>,
    payload: Option<P>,
    emitted: bool,
    quarantined: bool,
}

/// Bounded-count, in-memory reconciliation. Output is unordered across slots.
///
/// Exactly-once durable delivery, provider authentication, payload completeness and cryptographic
/// validation are responsibilities of the integrating application. No network requests are made.
pub struct Reconciler<P> {
    config: Config,
    sessions: BTreeMap<ProviderId, u64>,
    slots: BTreeMap<u64, SlotState<P>>,
    stats: Stats,
}

impl<P> Reconciler<P> {
    pub fn new(config: Config) -> Result<Self, Error> {
        if config.minimum_commitment == Commitment::Processed
            || config.minimum_providers == 0
            || config.minimum_providers > config.max_providers
            || config.max_retained_slots == 0
            || config.max_pending_blocks == 0
            || config.max_pending_blocks > config.max_retained_slots
        {
            return Err(Error::InvalidConfig);
        }
        Ok(Self {
            config,
            sessions: BTreeMap::new(),
            slots: BTreeMap::new(),
            stats: Stats::default(),
        })
    }

    /// Replace a connection without resetting stable provider votes or slot deduplication.
    pub fn connect(&mut self, provider: ProviderId) -> Result<Session, Error> {
        let generation = match self.sessions.get(&provider) {
            Some(current) => current.checked_add(1).ok_or(Error::SessionExhausted)?,
            None => {
                if self.sessions.len() >= self.config.max_providers {
                    return Err(Error::ProviderCapacity);
                }
                1
            }
        };
        self.sessions.insert(provider, generation);
        Ok(Session {
            provider,
            generation,
        })
    }

    pub fn validate_session(&self, session: Session) -> Result<(), Error> {
        if self.sessions.get(&session.provider) == Some(&session.generation) {
            Ok(())
        } else {
            Err(Error::UnknownSession)
        }
    }

    pub fn minimum_commitment(&self) -> Commitment {
        self.config.minimum_commitment
    }
    pub fn stats(&self) -> &Stats {
        &self.stats
    }
    pub fn first_slot(&self) -> u64 {
        self.config.first_slot
    }

    /// Accept metadata only after the connection's reconstructor has sealed its block.
    /// Invalid/capacity-limited inputs return errors without changing the retained slot state.
    pub fn observe(
        &mut self,
        session: Session,
        block: SealedBlock<P>,
    ) -> Result<Outcome<P>, Error> {
        self.validate_session(session)?;
        if block.commitment < self.config.minimum_commitment {
            return Err(Error::BelowCommitment);
        }
        let slot = block.meta.slot;
        if slot < self.config.first_slot {
            return Err(Error::StaleSlot {
                slot,
                first_slot: self.config.first_slot,
            });
        }
        if block.meta.blockhash.0 == [0; 32]
            || block.meta.parent_blockhash.0 == [0; 32]
            || block.meta.parent_slot >= slot
        {
            return Err(Error::InvalidBlock);
        }

        if let Some(state) = self.slots.get_mut(&slot) {
            if state.quarantined {
                return Ok(Outcome::Quarantined { slot });
            }
            if state.meta != block.meta {
                state.quarantined = true;
                if state.payload.take().is_some() {
                    self.stats.pending_blocks -= 1;
                }
                self.stats.conflicts = self.stats.conflicts.saturating_add(1);
                return Ok(Outcome::Conflict {
                    slot,
                    expected: state.meta.clone(),
                    received: block.meta,
                    previously_emitted: state.emitted,
                });
            }
            if state.emitted {
                self.stats.duplicates = self.stats.duplicates.saturating_add(1);
                return Ok(Outcome::Duplicate {
                    slot,
                    blockhash: state.meta.blockhash,
                });
            }
            state.providers.insert(session.provider);
        } else {
            if self.slots.len() >= self.config.max_retained_slots {
                return Err(Error::SlotCapacity);
            }
            if self.stats.pending_blocks >= self.config.max_pending_blocks {
                return Err(Error::PendingCapacity);
            }
            self.slots.insert(
                slot,
                SlotState {
                    meta: block.meta.clone(),
                    providers: BTreeSet::from([session.provider]),
                    payload: Some(block.payload),
                    emitted: false,
                    quarantined: false,
                },
            );
            self.stats.retained_slots = self.slots.len();
            self.stats.pending_blocks += 1;
        }
        let state = self
            .slots
            .get_mut(&slot)
            .expect("slot was just inserted or found");
        if state.providers.len() >= self.config.minimum_providers {
            state.emitted = true;
            self.stats.pending_blocks -= 1;
            self.stats.emitted = self.stats.emitted.saturating_add(1);
            Ok(Outcome::Emitted {
                block: SealedBlock {
                    meta: state.meta.clone(),
                    commitment: self.config.minimum_commitment,
                    payload: state
                        .payload
                        .take()
                        .expect("non-emitted slot retains its first payload"),
                },
                providers: state.providers.iter().copied().collect(),
            })
        } else {
            Ok(Outcome::Pending {
                slot,
                blockhash: state.meta.blockhash,
                agreeing_providers: state.providers.len(),
            })
        }
    }

    /// Advance the exclusive replay floor explicitly. Never infer a watermark from one fast provider.
    /// Older replay is rejected rather than emitted again. Persist the floor alongside sink state.
    pub fn advance(&mut self, before_slot: u64) -> Result<Pruned, Error> {
        if before_slot < self.config.first_slot {
            return Err(Error::CheckpointRegression);
        }
        let retained = self.slots.split_off(&before_slot);
        let mut report = Pruned {
            before_slot,
            pending_slots: Vec::new(),
            quarantined_slots: Vec::new(),
        };
        for (slot, state) in &self.slots {
            if state.payload.is_some() {
                report.pending_slots.push(*slot);
                self.stats.pending_blocks -= 1;
            }
            if state.quarantined {
                report.quarantined_slots.push(*slot);
            }
        }
        self.slots = retained;
        self.config.first_slot = before_slot;
        self.stats.retained_slots = self.slots.len();
        Ok(report)
    }
}
