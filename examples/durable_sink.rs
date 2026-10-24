//! End-to-end checkpoint contract against a sink that counts its writes.
//!
//! The reconciler refuses replayed slots, but that alone does not make an integration
//! exactly-once: the checkpoint has to be persisted *after* the sink commits, or a crash in
//! between re-delivers a slot the sink already has. This example runs a real crash and restart
//! over a checkpoint file on disk and counts the writes per slot.
//!
//! Run: cargo run --release --example durable_sink

use std::{collections::BTreeMap, fs, path::PathBuf};
use yellowstone_reconcile::*;

/// Stands in for a durable store. Records how many times each slot was written.
#[derive(Default)]
struct Sink {
    rows: BTreeMap<u64, String>,
    writes: BTreeMap<u64, u32>,
}

impl Sink {
    fn commit(&mut self, slot: u64, payload: String) {
        self.rows.insert(slot, payload);
        *self.writes.entry(slot).or_default() += 1;
    }
    fn double_written(&self) -> Vec<u64> {
        self.writes
            .iter()
            .filter(|(_, n)| **n > 1)
            .map(|(s, _)| *s)
            .collect()
    }
}

fn hash(n: u64) -> BlockHash {
    let mut b = [3; 32];
    b[..8].copy_from_slice(&n.to_le_bytes());
    BlockHash(b)
}

fn block(slot: u64, who: &str) -> SealedBlock<String> {
    SealedBlock {
        meta: BlockMeta {
            slot,
            blockhash: hash(slot),
            parent_slot: slot.saturating_sub(1),
            parent_blockhash: hash(slot.saturating_sub(1)),
            entry_count: 3,
            executed_transaction_count: 2,
        },
        commitment: Commitment::Finalized,
        payload: format!("slot-{slot}-from-{who}"),
    }
}

fn read_checkpoint(path: &PathBuf) -> u64 {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0)
}

/// Commit to the sink first, then persist the checkpoint. Never the other way round.
fn write_checkpoint(path: &PathBuf, slot: u64) {
    fs::write(path, slot.to_string()).expect("checkpoint write");
}

fn main() {
    let dir = std::env::temp_dir().join("yr-durable-sink");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let checkpoint_path = dir.join("checkpoint");

    let mut sink = Sink::default();
    let total = 20u64;
    let crash_after = 12u64;

    // Lifetime 1: run until a crash partway through the stream.
    {
        let mut core: Reconciler<String> = Reconciler::new(Config {
            first_slot: read_checkpoint(&checkpoint_path) + 1,
            ..Config::default()
        })
        .unwrap();
        let a = core.connect(ProviderId(1)).unwrap();
        let b = core.connect(ProviderId(2)).unwrap();
        for slot in 1..=crash_after {
            core.observe(a, block(slot, "a")).unwrap();
            if let Outcome::Emitted { block, .. } = core.observe(b, block(slot, "b")).unwrap() {
                sink.commit(block.meta.slot, block.payload);
                write_checkpoint(&checkpoint_path, block.meta.slot);
            }
        }
        println!(
            "MEASURED lifetime=1 committed_slots={} checkpoint_on_disk={}",
            sink.rows.len(),
            read_checkpoint(&checkpoint_path)
        );
    } // the reconciler is dropped here: process gone, checkpoint file survives

    // Lifetime 2: resume from the file and replay the WHOLE stream from slot 1.
    let resumed_from = read_checkpoint(&checkpoint_path);
    let mut refused = 0u32;
    {
        let mut core: Reconciler<String> = Reconciler::new(Config {
            first_slot: resumed_from + 1,
            ..Config::default()
        })
        .unwrap();
        let a = core.connect(ProviderId(1)).unwrap();
        let b = core.connect(ProviderId(2)).unwrap();
        for slot in 1..=total {
            for (session, who) in [(a, "a"), (b, "b")] {
                match core.observe(session, block(slot, who)) {
                    Ok(Outcome::Emitted { block, .. }) => {
                        sink.commit(block.meta.slot, block.payload);
                        write_checkpoint(&checkpoint_path, block.meta.slot);
                    }
                    Ok(_) => {}
                    Err(Error::StaleSlot { .. }) => refused += 1,
                    Err(e) => panic!("unexpected {e:?} at slot {slot}"),
                }
            }
        }
        println!(
            "MEASURED lifetime=2 resumed_from={resumed_from} replayed_slots_refused={refused} \
             checkpoint_on_disk={}",
            read_checkpoint(&checkpoint_path)
        );
    }

    let doubles = sink.double_written();
    println!(
        "MEASURED sink_rows={} distinct_slots={} double_written_slots={:?} \
         every_slot_written_once={}",
        sink.writes.values().sum::<u32>(),
        sink.rows.len(),
        doubles,
        doubles.is_empty() && sink.rows.len() as u64 == total
    );
    assert!(doubles.is_empty(), "a slot was written twice: {doubles:?}");
    assert_eq!(sink.rows.len() as u64, total, "a slot was lost");

    let _ = fs::remove_dir_all(&dir);
    println!(
        "LIMIT: the sink is in-process and the checkpoint is a plain file, so this shows the \
         ordering contract (commit, then checkpoint) rather than real durability. A crash between \
         the sink commit and the checkpoint write still re-delivers that one slot, which is why \
         the sink write must be idempotent on slot. No gRPC, reconstruction or real provider."
    );
}
