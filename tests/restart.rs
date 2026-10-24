//! Process restart from a persisted checkpoint.
//!
//! An indexer records the highest slot it has durably committed, exits, and comes back. On
//! reconnect a provider replays from well before that point. The property that matters is that
//! nothing at or below the checkpoint is emitted a second time, because a second emission means
//! a duplicate downstream write.

use yellowstone_reconcile::*;

fn hash(n: u64) -> BlockHash {
    let mut bytes = [1; 32];
    bytes[..8].copy_from_slice(&n.to_le_bytes());
    BlockHash(bytes)
}

fn block(slot: u64, payload: &str) -> SealedBlock<String> {
    SealedBlock {
        meta: BlockMeta {
            slot,
            blockhash: hash(slot),
            parent_slot: slot - 1,
            parent_blockhash: hash(slot.saturating_sub(1)),
            entry_count: 3,
            executed_transaction_count: 2,
        },
        commitment: Commitment::Finalized,
        payload: payload.into(),
    }
}

/// Emit a run of slots, persist the checkpoint, drop the reconciler, rebuild from the
/// checkpoint, and replay the whole stream from the beginning.
#[test]
fn restart_from_checkpoint_does_not_reemit_committed_slots() {
    let mut committed: Vec<u64> = Vec::new();

    // First process lifetime.
    let mut core: Reconciler<String> = Reconciler::new(Config::default()).unwrap();
    let a = core.connect(ProviderId(1)).unwrap();
    let b = core.connect(ProviderId(2)).unwrap();
    for slot in 1..=10 {
        core.observe(a, block(slot, "a")).unwrap();
        if let Outcome::Emitted { block, .. } = core.observe(b, block(slot, "b")).unwrap() {
            committed.push(block.meta.slot);
        }
    }
    assert_eq!(committed, (1..=10).collect::<Vec<_>>());

    // The operator durably recorded the highest committed slot, then the process exited.
    let checkpoint = *committed.last().unwrap();
    drop(core);

    // Second lifetime: resume with first_slot one past the checkpoint.
    let mut core: Reconciler<String> = Reconciler::new(Config {
        first_slot: checkpoint + 1,
        ..Config::default()
    })
    .unwrap();
    let a = core.connect(ProviderId(1)).unwrap();
    let b = core.connect(ProviderId(2)).unwrap();

    // Providers replay from the start of their retention, not from our checkpoint.
    for slot in 1..=checkpoint {
        assert_eq!(
            core.observe(a, block(slot, "replay")).unwrap_err(),
            Error::StaleSlot {
                slot,
                first_slot: checkpoint + 1
            },
            "slot {slot} at or below the checkpoint must be refused"
        );
    }
    assert_eq!(
        core.stats().emitted,
        0,
        "no slot was re-emitted after restart"
    );
    assert_eq!(
        core.stats().pending_blocks,
        0,
        "refused slots allocate no state"
    );

    // New slots still work.
    let mut after = Vec::new();
    for slot in (checkpoint + 1)..=(checkpoint + 3) {
        core.observe(a, block(slot, "a")).unwrap();
        if let Outcome::Emitted { block, .. } = core.observe(b, block(slot, "b")).unwrap() {
            after.push(block.meta.slot);
        }
    }
    assert_eq!(after, vec![11, 12, 13]);
    assert_eq!(committed.iter().chain(after.iter()).count(), 13);
}

/// A checkpoint read back from a stale store must not move retention backwards.
#[test]
fn a_stale_checkpoint_cannot_move_retention_backwards() {
    let mut core: Reconciler<String> = Reconciler::new(Config::default()).unwrap();
    let a = core.connect(ProviderId(1)).unwrap();
    let b = core.connect(ProviderId(2)).unwrap();
    for slot in 1..=20 {
        core.observe(a, block(slot, "a")).unwrap();
        core.observe(b, block(slot, "b")).unwrap();
    }
    core.advance(15).unwrap();
    assert_eq!(core.first_slot(), 15);
    assert_eq!(core.advance(14).unwrap_err(), Error::CheckpointRegression);
    assert_eq!(
        core.first_slot(),
        15,
        "a rejected advance leaves retention intact"
    );
    core.advance(15).unwrap();
    assert_eq!(
        core.first_slot(),
        15,
        "re-applying the same checkpoint is idempotent"
    );
}

/// Restarting with a checkpoint ahead of the stream refuses the gap rather than inventing it.
#[test]
fn a_checkpoint_ahead_of_the_stream_refuses_the_gap() {
    let mut core: Reconciler<String> = Reconciler::new(Config {
        first_slot: 100,
        ..Config::default()
    })
    .unwrap();
    let a = core.connect(ProviderId(1)).unwrap();
    assert_eq!(
        core.observe(a, block(50, "behind")).unwrap_err(),
        Error::StaleSlot {
            slot: 50,
            first_slot: 100
        }
    );
    assert_eq!(core.stats().emitted, 0);
    assert_eq!(core.stats().pending_blocks, 0);
}
