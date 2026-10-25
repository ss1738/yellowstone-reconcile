//! SQLite sink: atomic commit, linked watermark, and recovery after a process abort.
#![cfg(feature = "sqlite")]

use std::{path::PathBuf, process::Command};
use yellowstone_reconcile::{sink::*, *};

fn hash(n: u64) -> BlockHash {
    let mut bytes = [5; 32];
    bytes[..8].copy_from_slice(&n.to_le_bytes());
    BlockHash(bytes)
}

fn block_with_parent(slot: u64, parent: u64, tag: &str) -> SealedBlock<Vec<u8>> {
    SealedBlock {
        meta: BlockMeta {
            slot,
            blockhash: hash(slot),
            parent_slot: parent,
            parent_blockhash: hash(parent),
            entry_count: 3,
            executed_transaction_count: 2,
        },
        commitment: Commitment::Finalized,
        payload: format!("slot-{slot}-{tag}").into_bytes(),
    }
}

fn block(slot: u64) -> SealedBlock<Vec<u8>> {
    block_with_parent(slot, slot - 1, "a")
}

fn db(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("yr-sink-test-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("sink.db")
}

#[test]
fn commit_is_idempotent_on_slot() {
    let mut sink = SqliteSink::open(db("idem"), 0).unwrap();
    assert_eq!(sink.commit(&block(1)).unwrap(), Commit::Inserted);
    assert_eq!(sink.commit(&block(1)).unwrap(), Commit::AlreadyPresent);
    assert_eq!(sink.len().unwrap(), 1);
}

#[test]
fn a_different_block_for_a_stored_slot_is_refused_and_not_overwritten() {
    let mut sink = SqliteSink::open(db("mismatch"), 0).unwrap();
    sink.commit(&block(1)).unwrap();
    let mut other = block(1);
    other.meta.blockhash = hash(999);
    other.payload = b"other".to_vec();
    assert!(matches!(
        sink.commit(&other),
        Err(SinkError::Mismatch { slot: 1 })
    ));
    assert_eq!(sink.payload(1).unwrap().unwrap(), b"slot-1-a");
    assert_eq!(sink.blockhash(1).unwrap(), Some(hash(1)));
}

#[test]
fn replay_with_the_same_hash_but_different_metadata_is_refused() {
    let mut sink = SqliteSink::open(db("metadata-mismatch"), 0).unwrap();
    sink.commit(&block(1)).unwrap();
    for field in 0..3 {
        let mut changed = block(1);
        match field {
            0 => changed.meta.parent_slot = 99,
            1 => changed.meta.entry_count += 1,
            _ => changed.meta.executed_transaction_count += 1,
        }
        assert!(matches!(
            sink.commit(&changed),
            Err(SinkError::Mismatch { slot: 1 })
        ));
    }
    assert_eq!(sink.len().unwrap(), 1);
    assert_eq!(sink.watermark().unwrap(), 1);
    assert_eq!(sink.payload(1).unwrap().unwrap(), b"slot-1-a");
}

/// The watermark must not jump to the highest slot when lower slots are still missing.
#[test]
fn out_of_order_commits_hold_the_watermark_at_the_first_gap() {
    let mut sink = SqliteSink::open(db("order"), 0).unwrap();
    let expected = [(3, 0), (1, 1), (2, 3), (5, 3), (4, 5)];
    for (slot, watermark) in expected {
        sink.commit(&block(slot)).unwrap();
        assert_eq!(sink.watermark().unwrap(), watermark, "after slot {slot}");
    }
    assert_eq!(sink.resume_slot().unwrap(), 6);
}

#[test]
fn skipped_slots_chain_through_parent_slot() {
    let mut sink = SqliteSink::open(db("skip"), 0).unwrap();
    sink.commit(&block(1)).unwrap();
    // Slots 2 and 3 were skipped by the cluster: slot 4 names slot 1 as its parent.
    sink.commit(&block_with_parent(4, 1, "a")).unwrap();
    assert_eq!(sink.watermark().unwrap(), 4);
}

/// A child that does not link to the stored parent's hash must not advance the watermark.
#[test]
fn a_hash_mismatched_parent_link_stalls_the_watermark() {
    let mut sink = SqliteSink::open(db("fork"), 0).unwrap();
    sink.commit(&block(1)).unwrap();
    let mut orphan = block(2);
    orphan.meta.parent_blockhash = hash(777);
    sink.commit(&orphan).unwrap();
    assert_eq!(sink.watermark().unwrap(), 1);
}

#[test]
fn an_anchor_ahead_of_required_history_excludes_that_history() {
    let mut sink = SqliteSink::open(db("anchor"), 50).unwrap();
    sink.commit(&block(1)).unwrap();
    sink.commit(&block(2)).unwrap();
    assert_eq!(sink.watermark().unwrap(), 50);
    assert_eq!(sink.resume_slot().unwrap(), 51);
    // The caller declared history through 50 done. This is unsafe if it still needs slots 3..50.
}

#[test]
fn reopening_keeps_rows_and_watermark_and_ignores_the_anchor() {
    let path = db("reopen");
    {
        let mut sink = SqliteSink::open(&path, 0).unwrap();
        sink.commit(&block(1)).unwrap();
        sink.commit(&block(2)).unwrap();
    }
    let sink = SqliteSink::open(&path, 999).unwrap();
    assert_eq!(sink.watermark().unwrap(), 2);
    assert_eq!(sink.len().unwrap(), 2);
}

#[test]
fn slots_beyond_sqlite_integers_are_rejected() {
    let mut sink = SqliteSink::open(db("range"), 0).unwrap();
    let mut huge = block(1);
    huge.meta.slot = u64::MAX;
    huge.meta.parent_slot = u64::MAX - 1;
    assert!(matches!(
        sink.commit(&huge),
        Err(SinkError::SlotOutOfRange(_))
    ));
    assert_eq!(sink.len().unwrap(), 0);
}

const CHILD_ENV: &str = "YR_SINK_CRASH_DB";

/// Not a test on its own: the parent test below re-runs this binary with the env var set.
#[test]
fn crash_child() {
    let Ok(path) = std::env::var(CHILD_ENV) else {
        return;
    };
    let mut sink = SqliteSink::open(path, 0).unwrap();
    for slot in [3, 1, 2, 5] {
        sink.commit(&block(slot)).unwrap();
    }
    // SIGABRT: no destructors, no connection close, no WAL checkpoint.
    std::process::abort();
}

#[test]
fn committed_blocks_and_watermark_survive_a_process_abort() {
    let path = db("abort");
    let status = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "crash_child", "--nocapture"])
        .env(CHILD_ENV, &path)
        .status()
        .unwrap();
    assert!(!status.success(), "child was expected to abort");

    let mut sink = SqliteSink::open(&path, 0).unwrap();
    assert_eq!(sink.len().unwrap(), 4);
    assert_eq!(sink.watermark().unwrap(), 3);
    // Slot 4 never arrived before the crash. Redelivery of the stored slots is absorbed.
    assert_eq!(sink.commit(&block(3)).unwrap(), Commit::AlreadyPresent);
    assert_eq!(sink.commit(&block(4)).unwrap(), Commit::Inserted);
    assert_eq!(sink.watermark().unwrap(), 5);
}

/// Reconciler output is unordered. Crash mid-stream, restart at the sink's resume slot, replay
/// everything, and require that every slot ends up stored exactly once with none lost.
#[test]
fn reconciler_with_unordered_emission_loses_nothing_across_a_restart() {
    let path = db("e2e");
    let total = 20u64;
    // Provider B delivers in reversed groups of four, so slots emit out of order.
    let scrambled: Vec<u64> = (1..=total)
        .collect::<Vec<_>>()
        .chunks(4)
        .flat_map(|c| c.iter().rev().copied())
        .collect();

    let mut naive_checkpoint = 0u64;
    {
        let mut sink = SqliteSink::open(&path, 0).unwrap();
        let mut core: Reconciler<Vec<u8>> = Reconciler::new(Config::default()).unwrap();
        let a = core.connect(ProviderId(1)).unwrap();
        let b = core.connect(ProviderId(2)).unwrap();
        for slot in 1..=total {
            core.observe(a, block(slot)).unwrap();
        }
        for &slot in scrambled.iter().take(6) {
            if let Outcome::Emitted { block, .. } = core.observe(b, block_b(slot)).unwrap() {
                naive_checkpoint = naive_checkpoint.max(block.meta.slot);
                sink.commit(&block).unwrap();
            }
        }
    }
    // The naive "highest committed slot" checkpoint would already be past slots never stored.
    assert_eq!(naive_checkpoint, 8);

    let mut sink = SqliteSink::open(&path, 0).unwrap();
    let resume = sink.resume_slot().unwrap();
    assert_eq!(
        resume, 5,
        "watermark holds at 4 because 5 and 6 are missing"
    );
    let mut core: Reconciler<Vec<u8>> = Reconciler::new(Config {
        first_slot: resume,
        ..Config::default()
    })
    .unwrap();
    let a = core.connect(ProviderId(1)).unwrap();
    let b = core.connect(ProviderId(2)).unwrap();
    let (mut inserted, mut absorbed, mut refused) = (0, 0, 0);
    for slot in 1..=total {
        for (session, who) in [(a, block(slot)), (b, block_b(slot))] {
            match core.observe(session, who) {
                Ok(Outcome::Emitted { block, .. }) => match sink.commit(&block).unwrap() {
                    Commit::Inserted => inserted += 1,
                    Commit::AlreadyPresent => absorbed += 1,
                },
                Ok(_) => {}
                Err(Error::StaleSlot { .. }) => refused += 1,
                Err(e) => panic!("unexpected {e:?}"),
            }
        }
    }
    assert_eq!(sink.len().unwrap(), total, "a slot was lost");
    assert_eq!(sink.watermark().unwrap(), total);
    assert_eq!(inserted + absorbed, total - 4);
    assert_eq!(absorbed, 2, "slots 7 and 8 were stored before the crash");
    assert!(refused > 0);
}

fn block_b(slot: u64) -> SealedBlock<Vec<u8>> {
    block_with_parent(slot, slot - 1, "b")
}

/// A forkable block must not steer the watermark or be stored as final output.
#[test]
fn blocks_below_finalized_are_refused() {
    let mut sink = SqliteSink::open(db("commitment"), 0).unwrap();
    for commitment in [Commitment::Processed, Commitment::Confirmed] {
        let mut weak = block(1);
        weak.commitment = commitment;
        assert!(sink.commit(&weak).is_err(), "{commitment:?} was stored");
    }
    assert_eq!(sink.len().unwrap(), 0);
    assert_eq!(sink.watermark().unwrap(), 0);
}

/// A file whose tables look right but which a newer version of this sink marked as its own must
/// not be written to by code that does not know what changed.
#[test]
fn a_database_from_a_newer_schema_version_is_refused() {
    let path = db("newer");
    SqliteSink::open(&path, 0).unwrap();
    rusqlite::Connection::open(&path)
        .unwrap()
        .execute_batch("PRAGMA user_version = 99;")
        .unwrap();
    assert!(SqliteSink::open(&path, 0).is_err());
}

/// Two handles on one file, as two processes or threads would be. Every block lands once and
/// the watermark, computed inside the transaction, ends at the top.
#[test]
fn concurrent_writers_store_every_block_once() {
    let path = db("concurrent");
    SqliteSink::open(&path, 0).unwrap();
    let handles: Vec<_> = [1u64, 2]
        .into_iter()
        .map(|start| {
            let path = path.clone();
            std::thread::spawn(move || {
                let mut sink = SqliteSink::open(&path, 0).unwrap();
                for slot in (start..=40).step_by(2) {
                    sink.commit(&block(slot)).unwrap();
                }
            })
        })
        .collect();
    for handle in handles {
        handle.join().unwrap();
    }
    let sink = SqliteSink::open(&path, 0).unwrap();
    assert_eq!(sink.len().unwrap(), 40);
    assert_eq!(sink.watermark().unwrap(), 40);
}

const MID_TX_ENV: &str = "YR_SINK_MID_TX_DB";

/// Child: begin a write transaction the way `commit` does, write the row and the watermark, then
/// die before COMMIT. Models the uncommitted state; it does not call `commit` itself.
#[test]
fn mid_transaction_child() {
    let Ok(path) = std::env::var(MID_TX_ENV) else {
        return;
    };
    let conn = rusqlite::Connection::open(path).unwrap();
    conn.execute_batch(
        "BEGIN IMMEDIATE;
         INSERT INTO blocks VALUES (1, x'05', 0, x'05', 3, 2, x'00');
         UPDATE watermark SET slot = 1 WHERE id = 0;",
    )
    .unwrap();
    std::process::abort();
}

#[test]
fn a_process_dying_inside_a_write_transaction_leaves_nothing_and_no_lock() {
    let path = db("mid-tx");
    SqliteSink::open(&path, 0).unwrap();
    let status = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "mid_transaction_child", "--nocapture"])
        .env(MID_TX_ENV, &path)
        .status()
        .unwrap();
    assert!(!status.success(), "child was expected to abort");
    let mut sink = SqliteSink::open(&path, 0).unwrap();
    assert_eq!(sink.len().unwrap(), 0);
    assert_eq!(sink.watermark().unwrap(), 0);
    assert_eq!(sink.commit(&block(1)).unwrap(), Commit::Inserted);
}
