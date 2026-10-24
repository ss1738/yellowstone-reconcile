//! Offline example using the real upstream BlockMachineOutput API, without provider credentials.
use solana_commitment_config::CommitmentLevel;
use yellowstone_block_machine::{
    state_machine::SlotCommitmentStatusUpdate,
    stream::{Block, BlockEventStore, BlockMachineOutput},
};
use yellowstone_reconcile::{upstream::UpstreamBridge, Config, Outcome, ProviderId};

struct Events(Vec<u8>);
impl BlockEventStore for Events {
    type EventT = u8;
    type Iter<'a> = std::slice::Iter<'a, u8>;
    type IntoIter = Vec<u8>;
    fn len(&self) -> usize {
        self.0.len()
    }
    fn blockhash(&self) -> [u8; 32] {
        [3; 32]
    }
    fn iter(&self) -> Self::Iter<'_> {
        self.0.iter()
    }
    fn into_iter(self) -> Self::IntoIter {
        self.0
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut bridge = UpstreamBridge::new(Config::default(), 128, 10_000)?;
    for provider in [ProviderId(1), ProviderId(2)] {
        let session = bridge.connect(provider)?;
        // Both connections may use bank id 7. It is not a global block identity.
        let block = Block {
            slot: 10,
            bank_id: 7,
            blockhash: [3; 32],
            events: Events(vec![provider.0 as u8]),
            entry_count: 1,
            executed_transaction_count: 1,
            parent_slot: 9,
            parent_blockhash: [2; 32],
            blocktime_unix_ts: 0,
        };
        bridge.ingest(session, BlockMachineOutput::FrozenBlock(block))?;
        if let Some(Outcome::Emitted { block, providers }) = bridge.ingest(
            session,
            BlockMachineOutput::SlotCommitmentUpdate(SlotCommitmentStatusUpdate {
                parent_slot: Some(9),
                slot: 10,
                commitment: CommitmentLevel::Finalized,
                bank_id: 7,
            }),
        )? {
            println!(
                "slot={} providers={providers:?} selected_payload={:?}",
                block.meta.slot, block.payload.events.0
            );
        }
    }
    Ok(())
}
