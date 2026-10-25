//! Counterexample attempts from the 25 October 2026 audit. Each names the sequence it tries.
use yellowstone_reconcile::*;

fn hash(n: u64) -> BlockHash {
    let mut bytes = [9; 32];
    bytes[..8].copy_from_slice(&n.to_le_bytes());
    BlockHash(bytes)
}

fn sealed(slot: u64, parent: u64, tag: u64) -> SealedBlock<u64> {
    SealedBlock {
        meta: BlockMeta {
            slot,
            blockhash: hash(slot.wrapping_mul(1000).wrapping_add(tag)),
            parent_slot: parent,
            parent_blockhash: hash(parent),
            entry_count: 3,
            executed_transaction_count: 2,
        },
        commitment: Commitment::Finalized,
        payload: tag,
    }
}

/// A provider that already voted for one block reconnects and reports another for the same slot.
/// The reconnect must not turn a vote into agreement or hide the disagreement.
#[test]
fn a_reconnecting_provider_that_changes_its_report_is_a_conflict() {
    let mut core: Reconciler<u64> = Reconciler::new(Config::default()).unwrap();
    let a = core.connect(ProviderId(1)).unwrap();
    assert!(matches!(
        core.observe(a, sealed(5, 4, 1)).unwrap(),
        Outcome::Pending { .. }
    ));
    let a2 = core.connect(ProviderId(1)).unwrap();
    assert!(matches!(
        core.observe(a2, sealed(5, 4, 2)).unwrap(),
        Outcome::Conflict {
            previously_emitted: false,
            ..
        }
    ));
    let b = core.connect(ProviderId(2)).unwrap();
    assert!(matches!(
        core.observe(b, sealed(5, 4, 1)).unwrap(),
        Outcome::Quarantined { slot: 5 }
    ));
    assert_eq!(core.stats().emitted, 0);
    assert_eq!(core.stats().pending_blocks, 0);
}

/// Retention advances past a slot that is still pending, then the second provider arrives.
/// The first block must be reported as discarded and the late vote must not emit it.
#[test]
fn a_late_vote_after_retention_passed_the_slot_cannot_emit() {
    let mut core: Reconciler<u64> = Reconciler::new(Config::default()).unwrap();
    let a = core.connect(ProviderId(1)).unwrap();
    let b = core.connect(ProviderId(2)).unwrap();
    core.observe(a, sealed(5, 4, 1)).unwrap();
    let pruned = core.advance(6).unwrap();
    assert_eq!(pruned.pending_slots, vec![5]);
    assert_eq!(
        core.observe(b, sealed(5, 4, 1)).unwrap_err(),
        Error::StaleSlot {
            slot: 5,
            first_slot: 6
        }
    );
    assert_eq!(core.stats().emitted, 0);
}

/// Integer boundary: the largest slot, a parent equal to it and the largest checkpoint.
#[test]
fn slot_boundaries_neither_wrap_nor_emit_invalid_blocks() {
    let mut core: Reconciler<u64> = Reconciler::new(Config::default()).unwrap();
    let a = core.connect(ProviderId(1)).unwrap();
    let b = core.connect(ProviderId(2)).unwrap();
    assert_eq!(
        core.observe(a, sealed(u64::MAX, u64::MAX, 1)).unwrap_err(),
        Error::InvalidBlock
    );
    core.observe(a, sealed(u64::MAX, u64::MAX - 1, 1)).unwrap();
    assert!(matches!(
        core.observe(b, sealed(u64::MAX, u64::MAX - 1, 1)).unwrap(),
        Outcome::Emitted { .. }
    ));
    core.advance(u64::MAX).unwrap();
    assert_eq!(
        core.observe(a, sealed(u64::MAX - 1, u64::MAX - 2, 1))
            .unwrap_err(),
        Error::StaleSlot {
            slot: u64::MAX - 1,
            first_slot: u64::MAX
        }
    );
}

/// Quorum of one: a conflicting second provider must still be reported after the single
/// provider's block was emitted, because that emission may already be downstream.
#[test]
fn quorum_of_one_still_reports_a_later_conflict_as_after_emission() {
    let mut core: Reconciler<u64> = Reconciler::new(Config {
        minimum_providers: 1,
        ..Config::default()
    })
    .unwrap();
    let a = core.connect(ProviderId(1)).unwrap();
    let b = core.connect(ProviderId(2)).unwrap();
    assert!(matches!(
        core.observe(a, sealed(5, 4, 1)).unwrap(),
        Outcome::Emitted { .. }
    ));
    assert!(matches!(
        core.observe(b, sealed(5, 4, 2)).unwrap(),
        Outcome::Conflict {
            previously_emitted: true,
            ..
        }
    ));
}
