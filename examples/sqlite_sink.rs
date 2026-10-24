//! Crash and restart against a real SQLite file, with unordered emission.
//!
//! `durable_sink` shows the ordering contract with an in-process sink and emits in slot order.
//! The reconciler does not: output follows whichever provider completes a slot second. This run
//! lets one provider lead and the other deliver out of order, crashes mid-stream, and restarts
//! from the sink's own resume slot. It also reports what a "highest committed slot" checkpoint
//! would have done to the same run.
//!
//! Run: cargo run --release --features sqlite --example sqlite_sink

use std::fs;
use yellowstone_reconcile::{sink::*, *};

fn hash(n: u64) -> BlockHash {
    let mut b = [7; 32];
    b[..8].copy_from_slice(&n.to_le_bytes());
    BlockHash(b)
}

fn block(slot: u64, who: &str) -> SealedBlock<Vec<u8>> {
    SealedBlock {
        meta: BlockMeta {
            slot,
            blockhash: hash(slot),
            parent_slot: slot - 1,
            parent_blockhash: hash(slot - 1),
            entry_count: 3,
            executed_transaction_count: 2,
        },
        commitment: Commitment::Finalized,
        payload: format!("slot-{slot}-from-{who}").into_bytes(),
    }
}

fn main() {
    let dir = std::env::temp_dir().join("yr-sqlite-sink");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("sink.db");

    let total = 20u64;
    let crash_after = 6usize;
    // Provider B delivers in reversed groups of four, so slots emit as 4,3,2,1,8,7,6,5,...
    let scrambled: Vec<u64> = (1..=total)
        .collect::<Vec<_>>()
        .chunks(4)
        .flat_map(|c| c.iter().rev().copied())
        .collect();

    let mut naive_checkpoint = 0u64;
    let mut stored_before_crash = Vec::new();
    {
        let mut sink = SqliteSink::open(&path, 0).unwrap();
        let mut core: Reconciler<Vec<u8>> = Reconciler::new(Config::default()).unwrap();
        let a = core.connect(ProviderId(1)).unwrap();
        let b = core.connect(ProviderId(2)).unwrap();
        for slot in 1..=total {
            core.observe(a, block(slot, "a")).unwrap();
        }
        for &slot in scrambled.iter().take(crash_after) {
            if let Outcome::Emitted { block, .. } = core.observe(b, block(slot, "b")).unwrap() {
                sink.commit(&block).unwrap();
                naive_checkpoint = naive_checkpoint.max(block.meta.slot);
                stored_before_crash.push(block.meta.slot);
            }
        }
        println!(
            "MEASURED lifetime=1 stored_in_emission_order={stored_before_crash:?} \
             watermark={} naive_max_checkpoint={naive_checkpoint}",
            sink.watermark().unwrap()
        );
    } // the connection is dropped here without any further work: the file is what survives

    let mut sink = SqliteSink::open(&path, 0).unwrap();
    let resume = sink.resume_slot().unwrap();
    let naive_lost: Vec<u64> = (1..=naive_checkpoint)
        .filter(|s| !stored_before_crash.contains(s))
        .collect();
    println!(
        "MEASURED lifetime=2 resume_slot={resume} naive_resume_slot={} \
         naive_would_lose_slots={naive_lost:?}",
        naive_checkpoint + 1
    );

    let mut core: Reconciler<Vec<u8>> = Reconciler::new(Config {
        first_slot: resume,
        ..Config::default()
    })
    .unwrap();
    let a = core.connect(ProviderId(1)).unwrap();
    let b = core.connect(ProviderId(2)).unwrap();
    let (mut inserted, mut absorbed, mut refused) = (0u64, 0u64, 0u64);
    for slot in 1..=total {
        for (session, who) in [(a, "a"), (b, "b")] {
            match core.observe(session, block(slot, who)) {
                Ok(Outcome::Emitted { block, .. }) => match sink.commit(&block).unwrap() {
                    Commit::Inserted => inserted += 1,
                    Commit::AlreadyPresent => absorbed += 1,
                },
                Ok(_) => {}
                Err(Error::StaleSlot { .. }) => refused += 1,
                Err(e) => panic!("unexpected {e:?} at slot {slot}"),
            }
        }
    }

    let rows = sink.len().unwrap();
    println!(
        "MEASURED lifetime=2 inserted={inserted} redelivered_and_absorbed={absorbed} \
         replays_refused={refused} sink_rows={rows} watermark={} every_slot_stored_once={}",
        sink.watermark().unwrap(),
        rows == total
    );
    assert_eq!(rows, total, "a slot was lost");
    assert_eq!(sink.watermark().unwrap(), total);

    let _ = fs::remove_dir_all(&dir);
    println!(
        "LIMIT: SQLite on one local disk, synthetic blocks, no gRPC or real provider. Process death \
         is covered by tests/sink.rs (SIGABRT); power loss and a networked database are not \
         exercised. The watermark follows parent links, so it assumes the stored chain is \
         finalized: with Confirmed blocks a fork sharing a parent_slot would not be told apart \
         by slot alone, only by the parent_blockhash match."
    );
}
