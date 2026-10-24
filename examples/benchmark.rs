use std::{hint::black_box, time::Instant};
use yellowstone_reconcile::*;

fn hash(slot: u64) -> BlockHash {
    let mut h = [1; 32];
    h[..8].copy_from_slice(&slot.to_le_bytes());
    BlockHash(h)
}
fn main() {
    let mut core = Reconciler::new(Config::default()).unwrap();
    let a = core.connect(ProviderId(1)).unwrap();
    let b = core.connect(ProviderId(2)).unwrap();
    let start = Instant::now();
    let count = 200_000u64;
    for slot in 1..=count {
        if slot % 1000 == 0 {
            core.advance(slot.saturating_sub(100)).unwrap();
        }
        let meta = BlockMeta {
            slot,
            blockhash: hash(slot),
            parent_slot: slot - 1,
            parent_blockhash: hash(slot - 1),
            entry_count: 4,
            executed_transaction_count: 2,
        };
        for session in [a, b] {
            black_box(
                core.observe(
                    session,
                    SealedBlock {
                        meta: meta.clone(),
                        commitment: Commitment::Finalized,
                        payload: (),
                    },
                )
                .unwrap(),
            );
        }
    }
    let seconds = start.elapsed().as_secs_f64();
    println!("MEASURED synthetic_metadata_blocks={count} elapsed_seconds={seconds:.6} blocks_per_second={:.0} stats={:?}", count as f64 / seconds, core.stats());
    println!("LIMIT: in-memory metadata with unit payload; excludes gRPC, reconstruction, full payloads, persistence and end-to-end latency");
}
