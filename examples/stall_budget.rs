//! How long can a provider stall before reconciliation starts failing?
//!
//! `max_pending_blocks` and `max_retained_slots` are counts. Divided by the chain's slot rate
//! they become the operator's real budgets: how many seconds a silent provider can be tolerated,
//! and how far back a slot can still be replayed. This example runs both bounds to their limit
//! and reports them in seconds.
//!
//! Run: cargo run --release --example stall_budget [slot_ms]
//!
//! Default slot_ms is 212, measured on mainnet-beta on 2026-10-25 over 1,000 and 5,000 slot
//! windows (212 and 213 ms/slot). Slot time is not a constant; pass your own measurement.

use std::env;
use yellowstone_reconcile::*;

fn hash(slot: u64) -> BlockHash {
    let mut h = [9; 32];
    h[..8].copy_from_slice(&slot.to_le_bytes());
    BlockHash(h)
}

fn sealed(slot: u64) -> SealedBlock<()> {
    SealedBlock {
        meta: BlockMeta {
            slot,
            blockhash: hash(slot),
            parent_slot: slot.saturating_sub(1),
            parent_blockhash: hash(slot.saturating_sub(1)),
            entry_count: 4,
            executed_transaction_count: 2,
        },
        commitment: Commitment::Finalized,
        payload: (),
    }
}

fn main() {
    let slot_ms: f64 = env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(212.0);

    let config = Config::default();
    let max_pending = config.max_pending_blocks;
    let max_retained = config.max_retained_slots;

    // How many blocks does one silent provider buffer before the bound rejects?
    let mut core: Reconciler<()> = Reconciler::new(Config::default()).unwrap();
    let live = core.connect(ProviderId(1)).unwrap();
    let _silent = core.connect(ProviderId(2)).unwrap();
    let mut accepted = 0u64;
    for slot in 1.. {
        match core.observe(live, sealed(slot)) {
            Ok(_) => accepted += 1,
            Err(Error::PendingCapacity) => break,
            Err(e) => {
                println!("UNEXPECTED {e:?} after {accepted} blocks");
                return;
            }
        }
    }
    let stall_seconds = accepted as f64 * slot_ms / 1000.0;
    println!(
        "MEASURED slot_ms={slot_ms} max_pending_blocks={max_pending} \
         blocks_buffered_before_rejection={accepted} stall_budget_seconds={stall_seconds:.1}"
    );

    // Does the live provider recover once the silent one returns, without losing the quorum?
    let recovered = core
        .observe(_silent, sealed(1))
        .map(|o| matches!(o, Outcome::Emitted { .. }));
    println!(
        "MEASURED recovery_after_rejection emitted_on_first_agreement={:?} pending_now={}",
        recovered,
        core.stats().pending_blocks
    );

    // Retention window: how far back can a slot still arrive before it is refused as stale?
    let mut core2: Reconciler<()> = Reconciler::new(Config::default()).unwrap();
    let a = core2.connect(ProviderId(1)).unwrap();
    let b = core2.connect(ProviderId(2)).unwrap();
    let mut emitted = 0u64;
    for slot in 1..=(max_retained as u64) {
        core2.observe(a, sealed(slot)).unwrap();
        core2.observe(b, sealed(slot)).unwrap();
        emitted += 1;
    }
    let pruned = core2.advance(emitted - 10).unwrap();
    let retention_seconds = max_retained as f64 * slot_ms / 1000.0;
    println!(
        "MEASURED max_retained_slots={max_retained} retention_window_seconds={retention_seconds:.0} \
         emitted={emitted} pruned_before_slot={} pending_dropped={} quarantined_dropped={}",
        pruned.before_slot,
        pruned.pending_slots.len(),
        pruned.quarantined_slots.len()
    );

    println!(
        "LIMIT: slot_ms is an input, not a guarantee; it was measured once and varies. \
         The budgets assume one block per slot per provider and no payload-size pressure, which \
         binds first on large blocks (see the lag_memory example). Nothing here exercises gRPC, \
         reconstruction or persistence."
    );
}
