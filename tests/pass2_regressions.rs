//! Failure semantics the code states but no earlier test pinned. Each test names the promise it
//! checks; a failing test demonstrates a defect in that promise.

#[cfg(feature = "upstream")]
mod bridge {
    use solana_commitment_config::CommitmentLevel;
    use yellowstone_block_machine::{
        state_machine::SlotCommitmentStatusUpdate,
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

    fn frozen(slot: u64, bank_id: u64) -> BlockMachineOutput<Store> {
        BlockMachineOutput::FrozenBlock(Block {
            slot,
            bank_id,
            blockhash: [3; 32],
            events: Store(vec![bank_id as u8]),
            entry_count: 2,
            executed_transaction_count: 1,
            parent_slot: slot - 1,
            parent_blockhash: [2; 32],
            blocktime_unix_ts: 0,
        })
    }

    fn finalized(slot: u64, bank_id: u64) -> BlockMachineOutput<Store> {
        BlockMachineOutput::SlotCommitmentUpdate(SlotCommitmentStatusUpdate {
            slot,
            bank_id,
            parent_slot: Some(slot - 1),
            commitment: CommitmentLevel::Finalized,
        })
    }

    /// src/upstream.rs promises that a commitment update refused for capacity keeps its payload,
    /// so the caller can retry after capacity is freed.
    #[test]
    fn a_commitment_update_refused_for_capacity_keeps_its_payload_and_a_retry_succeeds() {
        let config = Config {
            max_pending_blocks: 1,
            max_retained_slots: 4,
            ..Config::default()
        };
        let mut bridge = UpstreamBridge::new(config, 8, 100).unwrap();
        let a = bridge.connect(ProviderId(1)).unwrap();
        let b = bridge.connect(ProviderId(2)).unwrap();

        // Provider 1's slot 10 takes the core's only pending place.
        bridge.ingest(a, frozen(10, 7)).unwrap();
        let first = bridge.ingest(a, finalized(10, 7)).unwrap();
        assert!(matches!(first, Some(Outcome::Pending { slot: 10, .. })));

        // Slot 11 has no place left: refused, and nothing may change.
        bridge.ingest(a, frozen(11, 8)).unwrap();
        let refused = bridge.ingest(a, finalized(11, 8));
        assert!(matches!(
            refused,
            Err(BridgeError::Core(Error::PendingCapacity))
        ));
        assert_eq!(bridge.pending_blocks(), 1, "the refused payload stays buffered");
        assert_eq!(bridge.core().stats().pending_blocks, 1);
        assert_eq!(bridge.core().stats().retained_slots, 1);

        // Provider 2 agrees on slot 10: it is emitted and the pending place is free again.
        bridge.ingest(b, frozen(10, 7)).unwrap();
        let emitted = bridge.ingest(b, finalized(10, 7)).unwrap();
        assert!(matches!(emitted, Some(Outcome::Emitted { .. })));
        assert_eq!(bridge.core().stats().pending_blocks, 0);

        // The same update, retried, now succeeds with the payload that was kept.
        let retried = bridge.ingest(a, finalized(11, 8)).unwrap();
        assert!(matches!(
            retried,
            Some(Outcome::Pending {
                slot: 11,
                agreeing_providers: 1,
                ..
            })
        ));
        assert_eq!(bridge.pending_blocks(), 0);
        assert_eq!(bridge.core().stats().pending_blocks, 1);
    }

    /// Characterisation, not a defect: the bridge needs each connection's events in the order the
    /// upstream stream delivers them (block first, then its commitment). A commitment update
    /// for an unknown bank is dropped, and a block that arrives afterwards waits for an update
    /// that will not repeat. The upstream state machine queues early updates until the bank
    /// freezes, so a real `BlockStream` does not produce this order.
    #[test]
    fn a_commitment_update_that_precedes_its_block_is_dropped_not_replayed() {
        let mut bridge = UpstreamBridge::new(Config::default(), 8, 100).unwrap();
        let a = bridge.connect(ProviderId(1)).unwrap();

        assert!(bridge.ingest(a, finalized(10, 7)).unwrap().is_none());
        bridge.ingest(a, frozen(10, 7)).unwrap();

        assert_eq!(bridge.pending_blocks(), 1, "the block waits in the bridge");
        assert_eq!(bridge.core().stats().pending_blocks, 0, "nothing reached the core");
    }
}

#[cfg(feature = "sqlite")]
mod sink {
    use std::path::PathBuf;
    use yellowstone_reconcile::sink::*;

    fn db(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("yr-pass2-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("sink.db")
    }

    fn journal_mode(path: &PathBuf) -> String {
        rusqlite::Connection::open(path)
            .unwrap()
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .unwrap()
    }

    /// A file written by a newer schema must be refused without being changed. The existing
    /// newer-schema test starts from a file this sink created, which is already in WAL mode, so
    /// it cannot see a side effect of the refused open. This one starts from a file in SQLite's
    /// default journal mode, as another tool would leave it.
    #[test]
    fn refusing_a_newer_schema_leaves_the_file_as_found() {
        let path = db("foreign-newer");
        rusqlite::Connection::open(&path)
            .unwrap()
            .execute_batch("PRAGMA user_version = 99;")
            .unwrap();
        assert_eq!(journal_mode(&path), "delete", "precondition");

        assert!(matches!(
            SqliteSink::open(&path, 0),
            Err(SinkError::SchemaVersion(99))
        ));

        assert_eq!(
            journal_mode(&path),
            "delete",
            "a refused open must not switch the file to WAL"
        );
        let objects: i64 = rusqlite::Connection::open(&path)
            .unwrap()
            .query_row("SELECT COUNT(*) FROM sqlite_master", [], |row| row.get(0))
            .unwrap();
        assert_eq!(objects, 0, "a refused open must not create tables");
    }
}
