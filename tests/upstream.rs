#![cfg(feature = "upstream")]
use solana_commitment_config::CommitmentLevel;
use yellowstone_block_machine::{
    state_machine::{BankDiscarded, SlotCommitmentStatusUpdate},
    stream::{Block, BlockEventStore, BlockMachineOutput},
};
use yellowstone_reconcile::{
    upstream::{BridgeError, UpstreamBridge},
    *,
};

#[derive(Debug)]
struct Store(Vec<u8>);
impl BlockEventStore for Store {
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
fn frozen(bank_id: u64, marker: u8) -> BlockMachineOutput<Store> {
    BlockMachineOutput::FrozenBlock(Block {
        slot: 10,
        bank_id,
        blockhash: [3; 32],
        events: Store(vec![marker]),
        entry_count: 2,
        executed_transaction_count: 1,
        parent_slot: 9,
        parent_blockhash: [2; 32],
        blocktime_unix_ts: 0,
    })
}
fn status(bank_id: u64, commitment: CommitmentLevel) -> BlockMachineOutput<Store> {
    BlockMachineOutput::SlotCommitmentUpdate(SlotCommitmentStatusUpdate {
        slot: 10,
        bank_id,
        parent_slot: Some(9),
        commitment,
    })
}

#[test]
fn same_local_bank_id_across_connections_never_combines_payloads() {
    let mut bridge = UpstreamBridge::new(Config::default(), 8, 100).unwrap();
    let a = bridge.connect(ProviderId(1)).unwrap();
    let b = bridge.connect(ProviderId(2)).unwrap();
    bridge.ingest(a, frozen(7, 41)).unwrap();
    bridge.ingest(b, frozen(7, 99)).unwrap();
    assert_eq!(bridge.pending_blocks(), 2);
    assert!(bridge
        .ingest(a, status(7, CommitmentLevel::Processed))
        .unwrap()
        .is_none());
    assert!(bridge
        .ingest(a, status(7, CommitmentLevel::Confirmed))
        .unwrap()
        .is_none());
    assert!(matches!(
        bridge
            .ingest(a, status(7, CommitmentLevel::Finalized))
            .unwrap(),
        Some(Outcome::Pending { .. })
    ));
    let Some(Outcome::Emitted { block, .. }) = bridge
        .ingest(b, status(7, CommitmentLevel::Finalized))
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(block.payload.events.0, vec![41]);
    assert_eq!(bridge.pending_blocks(), 0);
}

#[test]
fn different_local_bank_ids_reconcile_by_hash() {
    let mut bridge = UpstreamBridge::new(Config::default(), 8, 100).unwrap();
    let a = bridge.connect(ProviderId(1)).unwrap();
    let b = bridge.connect(ProviderId(2)).unwrap();
    bridge.ingest(a, frozen(7, 1)).unwrap();
    bridge.ingest(b, frozen(12, 2)).unwrap();
    bridge
        .ingest(a, status(7, CommitmentLevel::Finalized))
        .unwrap();
    assert!(matches!(
        bridge
            .ingest(b, status(12, CommitmentLevel::Finalized))
            .unwrap(),
        Some(Outcome::Emitted { .. })
    ));
}

#[test]
fn reconnect_discards_old_unqualified_blocks_and_rejects_old_events() {
    let mut bridge = UpstreamBridge::new(Config::default(), 8, 100).unwrap();
    let a = bridge.connect(ProviderId(1)).unwrap();
    bridge.ingest(a, frozen(7, 1)).unwrap();
    let next = bridge.connect(a.provider).unwrap();
    assert_eq!(bridge.pending_blocks(), 0);
    assert!(matches!(
        bridge.ingest(a, status(7, CommitmentLevel::Finalized)),
        Err(BridgeError::Core(Error::UnknownSession))
    ));
    bridge.ingest(next, frozen(7, 2)).unwrap();
    assert_eq!(bridge.pending_blocks(), 1);
}

#[test]
fn discard_is_scoped_to_connection_and_bank() {
    let mut bridge = UpstreamBridge::new(Config::default(), 8, 100).unwrap();
    let a = bridge.connect(ProviderId(1)).unwrap();
    let b = bridge.connect(ProviderId(2)).unwrap();
    bridge.ingest(a, frozen(7, 1)).unwrap();
    bridge.ingest(b, frozen(7, 2)).unwrap();
    bridge
        .ingest(
            a,
            BlockMachineOutput::BankDiscarded(BankDiscarded {
                slot: 10,
                bank_id: 7,
            }),
        )
        .unwrap();
    assert_eq!(bridge.pending_blocks(), 1);
    assert!(matches!(
        bridge
            .ingest(b, status(7, CommitmentLevel::Finalized))
            .unwrap(),
        Some(Outcome::Pending { .. })
    ));
}

#[test]
fn bridge_capacity_is_explicit_and_advance_reports_eviction() {
    let mut bridge = UpstreamBridge::new(Config::default(), 1, 1).unwrap();
    let a = bridge.connect(ProviderId(1)).unwrap();
    let b = bridge.connect(ProviderId(2)).unwrap();
    bridge.ingest(a, frozen(7, 1)).unwrap();
    assert!(matches!(
        bridge.ingest(b, frozen(7, 2)),
        Err(BridgeError::BufferCapacity)
    ));
    assert_eq!(bridge.advance(11).unwrap().pending_slots, vec![10]);
    assert_eq!(bridge.pending_blocks(), 0);
    assert!(matches!(
        bridge.ingest(a, frozen(7, 1)),
        Err(BridgeError::Core(Error::StaleSlot { .. }))
    ));
}

#[test]
fn conflicting_bank_reuse_and_oversized_event_store_are_rejected() {
    let mut bridge = UpstreamBridge::new(Config::default(), 8, 1).unwrap();
    let a = bridge.connect(ProviderId(1)).unwrap();
    bridge.ingest(a, frozen(7, 1)).unwrap();
    let BlockMachineOutput::FrozenBlock(mut changed) = frozen(7, 2) else {
        panic!()
    };
    changed.blockhash = [9; 32];
    assert!(matches!(
        bridge.ingest(a, BlockMachineOutput::FrozenBlock(changed)),
        Err(BridgeError::ConflictingBankMetadata)
    ));
    let BlockMachineOutput::FrozenBlock(mut large) = frozen(8, 2) else {
        panic!()
    };
    large.events.0.push(3);
    assert!(matches!(
        bridge.ingest(a, BlockMachineOutput::FrozenBlock(large)),
        Err(BridgeError::EventCapacity)
    ));
}
