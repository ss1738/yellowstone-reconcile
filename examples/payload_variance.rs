//! Memory under real block-size variance, not a uniform guess.
//!
//! `lag_memory` uses one payload size. Real Solana blocks do not: in a sample of ten finalized
//! mainnet blocks taken on 2026-10-25 the smallest was 2,389 bytes and the largest 8,281,597,
//! a spread of about 3,500x, with a median of 4.26 MiB. Sizing the pending bound against a
//! round 2 MiB therefore understates the ceiling by roughly half.
//!
//! This example replays that measured distribution through the reconciler with one provider
//! silent, so every accepted block is retained, and reports the bytes actually held when the
//! bound rejects.
//!
//! Run: cargo run --release --example payload_variance

use yellowstone_reconcile::*;

/// Ten finalized mainnet blocks, slots 460985218..460985317, sampled 2026-10-25 over
/// `getBlock` with `encoding=base64`, `transactionDetails=full`, `rewards=false` and
/// `maxSupportedTransactionVersion=1`. Sizes are of the JSON-encoded response.
const MEASURED_BLOCK_BYTES: [usize; 10] = [
    4_071_562, 8_281_597, 4_501_503, 4_012_027, 4_754_509, 4_415_224, 4_432_938, 2_389, 6_536_590,
    5_992_874,
];

fn hash(slot: u64) -> BlockHash {
    let mut h = [5; 32];
    h[..8].copy_from_slice(&slot.to_le_bytes());
    BlockHash(h)
}

fn sealed(slot: u64, bytes: usize) -> SealedBlock<Vec<u8>> {
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
        payload: vec![0u8; bytes],
    }
}

fn mib(bytes: usize) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
}

fn main() {
    let config = Config::default();
    let max_pending = config.max_pending_blocks;

    let mut core: Reconciler<Vec<u8>> = Reconciler::new(Config::default()).unwrap();
    let live = core.connect(ProviderId(1)).unwrap();
    let _silent = core.connect(ProviderId(2)).unwrap();

    let mut retained = 0usize;
    let mut accepted = 0usize;
    for slot in 1.. {
        let bytes = MEASURED_BLOCK_BYTES[(slot as usize - 1) % MEASURED_BLOCK_BYTES.len()];
        match core.observe(live, sealed(slot, bytes)) {
            Ok(_) => {
                retained += bytes;
                accepted += 1;
            }
            Err(Error::PendingCapacity) => break,
            Err(e) => {
                println!("UNEXPECTED {e:?} after {accepted} blocks");
                return;
            }
        }
    }

    let sum: usize = MEASURED_BLOCK_BYTES.iter().sum();
    let mean = sum / MEASURED_BLOCK_BYTES.len();
    let max = *MEASURED_BLOCK_BYTES.iter().max().unwrap();
    let min = *MEASURED_BLOCK_BYTES.iter().min().unwrap();

    println!(
        "MEASURED sample_blocks={} min_bytes={min} median_like_mean_bytes={mean} max_bytes={max} \
         spread={:.0}x",
        MEASURED_BLOCK_BYTES.len(),
        max as f64 / min as f64
    );
    println!(
        "MEASURED replayed_distribution accepted_blocks={accepted} max_pending_blocks={max_pending} \
         retained_bytes={retained} retained_mib={:.1}",
        mib(retained)
    );
    println!(
        "DERIVED ceilings_at_{max_pending}_blocks uniform_2mib_mib={:.0} \
         sampled_mean_mib={:.0} all_largest_block_mib={:.0}",
        mib(max_pending * 2 * 1024 * 1024),
        mib(max_pending * mean),
        mib(max_pending * max)
    );
    println!(
        "LIMIT: ten blocks is a small sample from one minute of one day and is not a size \
         distribution. Payload bytes here are the JSON-encoded RPC response, which is not the \
         wire size a Geyser consumer holds. Treat these as the order of magnitude to size \
         against, and measure your own provider's payloads before fixing max_pending_blocks."
    );
}
