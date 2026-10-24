//! What does a lagging provider cost in memory?
//!
//! `Config` bounds pending work by *count* (`max_pending_blocks`), not by bytes. Real Solana
//! blocks are large, so the operator-visible ceiling is that count multiplied by their payload
//! size. This example measures the retained payload bytes while one provider runs ahead of
//! another, and shows that exceeding the bound fails closed instead of growing without limit.
//!
//! Run: cargo run --release --example lag_memory [payload_bytes] [lag_slots]

use std::{env, time::Instant};
use yellowstone_reconcile::*;

fn hash(slot: u64) -> BlockHash {
    let mut h = [7; 32];
    h[..8].copy_from_slice(&slot.to_le_bytes());
    BlockHash(h)
}

fn meta(slot: u64) -> BlockMeta {
    BlockMeta {
        slot,
        blockhash: hash(slot),
        parent_slot: slot.saturating_sub(1),
        parent_blockhash: hash(slot.saturating_sub(1)),
        entry_count: 4,
        executed_transaction_count: 2,
    }
}

fn sealed(slot: u64, bytes: usize) -> SealedBlock<Vec<u8>> {
    SealedBlock { meta: meta(slot), commitment: Commitment::Finalized, payload: vec![0u8; bytes] }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    // 2 MiB is a plausible mainnet block payload; override to match your own capture.
    let payload_bytes: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(2 * 1024 * 1024);
    let lag: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(64);

    let config = Config::default();
    let max_pending = config.max_pending_blocks;
    let mut core: Reconciler<Vec<u8>> = Reconciler::new(config).unwrap();
    let fast = core.connect(ProviderId(1)).unwrap();
    let slow = core.connect(ProviderId(2)).unwrap();

    // Phase 1: the fast provider runs `lag` slots ahead. Nothing can be emitted until the slow
    // provider agrees, so every one of those payloads is retained.
    let mut peak_pending = 0usize;
    let start = Instant::now();
    for slot in 1..=lag {
        core.observe(fast, sealed(slot, payload_bytes)).unwrap();
        peak_pending = peak_pending.max(core.stats().pending_blocks);
    }
    let held = core.stats().pending_blocks;
    println!(
        "MEASURED lag_slots={lag} payload_bytes={payload_bytes} pending_blocks={held} \
         retained_payload_bytes={} retained_mib={:.1}",
        held * payload_bytes,
        (held * payload_bytes) as f64 / (1024.0 * 1024.0)
    );

    // Phase 2: the slow provider catches up; everything drains.
    for slot in 1..=lag {
        core.observe(slow, sealed(slot, payload_bytes)).unwrap();
    }
    println!(
        "MEASURED after_catchup pending_blocks={} emitted={} elapsed_seconds={:.4}",
        core.stats().pending_blocks,
        core.stats().emitted,
        start.elapsed().as_secs_f64()
    );

    // Phase 3: push past the configured bound with a provider that never agrees.
    let mut core2: Reconciler<Vec<u8>> = Reconciler::new(Config::default()).unwrap();
    let only = core2.connect(ProviderId(1)).unwrap();
    let mut accepted = 0usize;
    let mut stopped_with = None;
    for slot in 1..=(max_pending as u64 + 16) {
        match core2.observe(only, sealed(slot, payload_bytes)) {
            Ok(_) => accepted += 1,
            Err(e) => {
                stopped_with = Some(e);
                break;
            }
        }
    }
    match stopped_with {
        Some(e) => println!(
            "MEASURED bound_enforced accepted_blocks={accepted} max_pending_blocks={max_pending} \
             rejected_with={e:?} implied_ceiling_mib={:.1}",
            (accepted * payload_bytes) as f64 / (1024.0 * 1024.0)
        ),
        None => println!(
            "UNEXPECTED no bound hit after {accepted} blocks; max_pending_blocks={max_pending}"
        ),
    }

    println!(
        "LIMIT: synthetic payloads of uniform size, no gRPC, no reconstruction, no persistence. \
         Retained bytes are computed from pending_blocks times payload size, not from allocator \
         accounting. Operators must size max_pending_blocks against their own payload sizes, \
         because the bound counts blocks and not bytes."
    );
}
