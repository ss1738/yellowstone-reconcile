//! Durable SQLite sink that commits a block and its resume point in one transaction.
//!
//! `Reconciler` output is unordered across slots, so "the highest slot committed" is not a safe
//! checkpoint: committing slot 12 before slot 11 and then crashing would resume at 13 and lose 11.
//! This sink keeps a watermark that only advances along parent links back to a trusted anchor,
//! and writes it in the same transaction as the block. A restart can redeliver
//! blocks above the watermark, and `commit` absorbs those as `Commit::AlreadyPresent`.
//!
//! Failure direction: a missing parent link stalls the watermark, which costs extra replay. It
//! never moves the watermark past a missing ancestor after the trusted anchor. Skipped slots
//! need no row. An anchor ahead of required history is unsafe: it declares that history done.
use crate::{BlockHash, Commitment, SealedBlock};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use std::{fmt, path::Path};

#[derive(Debug)]
pub enum SinkError {
    Sqlite(rusqlite::Error),
    /// SQLite integers are signed 64-bit, so slots above `i64::MAX` cannot be stored.
    SlotOutOfRange(u64),
    /// The slot is already stored with different identity metadata. Not overwritten.
    Mismatch {
        slot: u64,
    },
    Corrupt(&'static str),
    /// Only finalized blocks may be stored: the watermark follows parent links, and forkable
    /// blocks sharing a parent would let it advance along a branch that is later abandoned.
    NotFinalized {
        slot: u64,
        commitment: Commitment,
    },
    /// The file was written by a sink with a schema this code does not know.
    SchemaVersion(i64),
}

/// Value of `PRAGMA user_version` for the schema created by this module.
const SCHEMA_VERSION: i64 = 1;

impl fmt::Display for SinkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sqlite(e) => write!(f, "sqlite: {e}"),
            Self::SlotOutOfRange(slot) => write!(f, "slot {slot} does not fit a SQLite integer"),
            Self::Mismatch { slot } => write!(f, "slot {slot} is stored with a different block"),
            Self::Corrupt(what) => write!(f, "corrupt sink: {what}"),
            Self::NotFinalized { slot, commitment } => {
                write!(
                    f,
                    "slot {slot} is {commitment:?}; only Finalized blocks are stored"
                )
            }
            Self::SchemaVersion(version) => {
                write!(f, "sink schema version {version} is not supported")
            }
        }
    }
}

impl std::error::Error for SinkError {}

impl From<rusqlite::Error> for SinkError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Sqlite(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Commit {
    Inserted,
    AlreadyPresent,
}

pub struct SqliteSink {
    conn: Connection,
}

fn to_i64(slot: u64) -> Result<i64, SinkError> {
    i64::try_from(slot).map_err(|_| SinkError::SlotOutOfRange(slot))
}

fn to_u64(value: i64) -> Result<u64, SinkError> {
    u64::try_from(value).map_err(|_| SinkError::Corrupt("negative slot"))
}

/// Move the watermark forward while the next block links to the current one.
fn advance(conn: &Connection) -> Result<(), SinkError> {
    let (start, start_hash): (i64, Option<Vec<u8>>) = conn.query_row(
        "SELECT slot, blockhash FROM watermark WHERE id = 0",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let (mut slot, mut hash) = (start, start_hash);
    loop {
        // The anchor has no known hash, so the first link is matched on slot alone.
        let next: Option<(i64, Vec<u8>)> = conn
            .query_row(
                "SELECT slot, blockhash FROM blocks \
                 WHERE parent_slot = ?1 AND slot > ?1 AND (?2 IS NULL OR parent_blockhash = ?2) \
                 ORDER BY slot LIMIT 1",
                params![slot, hash],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        match next {
            Some((s, h)) => {
                slot = s;
                hash = Some(h);
            }
            None => break,
        }
    }
    if slot != start {
        conn.execute(
            "UPDATE watermark SET slot = ?1, blockhash = ?2 WHERE id = 0",
            params![slot, hash],
        )?;
    }
    Ok(())
}

impl SqliteSink {
    /// Open or create the database. `anchor` is the slot already considered done when the
    /// database is created, normally the `parent_slot` of the first block you will store. It is
    /// ignored when the database already exists. The caller must establish that history through
    /// the anchor is complete or deliberately excluded. An anchor ahead of required history can
    /// skip that history on restart; the sink cannot validate this external precondition.
    pub fn open(path: impl AsRef<Path>, anchor: u64) -> Result<Self, SinkError> {
        let conn = Connection::open(path)?;
        conn.query_row("PRAGMA journal_mode = WAL", [], |row| {
            row.get::<_, String>(0)
        })?;
        // 0 is a new file or one created before versioning, which used this same schema.
        let version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if version != 0 && version != SCHEMA_VERSION {
            return Err(SinkError::SchemaVersion(version));
        }
        conn.execute_batch(
            "PRAGMA synchronous = FULL;
             CREATE TABLE IF NOT EXISTS blocks (
                 slot INTEGER PRIMARY KEY,
                 blockhash BLOB NOT NULL,
                 parent_slot INTEGER NOT NULL,
                 parent_blockhash BLOB NOT NULL,
                 entry_count INTEGER NOT NULL,
                 executed_transaction_count INTEGER NOT NULL,
                 payload BLOB NOT NULL
             );
             CREATE INDEX IF NOT EXISTS blocks_parent ON blocks (parent_slot);
             CREATE TABLE IF NOT EXISTS watermark (
                 id INTEGER PRIMARY KEY CHECK (id = 0),
                 slot INTEGER NOT NULL,
                 blockhash BLOB
             );",
        )?;
        conn.execute(
            "INSERT OR IGNORE INTO watermark (id, slot, blockhash) VALUES (0, ?1, NULL)",
            [to_i64(anchor)?],
        )?;
        conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
        Ok(Self { conn })
    }

    /// Store the block and advance the watermark atomically. Idempotent on slot.
    ///
    /// The payload is opaque and is not validated. The block must be `Finalized`: configure the
    /// reconciler with `minimum_commitment = Finalized` (the default) when feeding this sink.
    /// A block with an equal slot but different identity metadata is an error, never an update.
    pub fn commit(&mut self, block: &SealedBlock<Vec<u8>>) -> Result<Commit, SinkError> {
        let meta = &block.meta;
        if block.commitment < Commitment::Finalized {
            return Err(SinkError::NotFinalized {
                slot: meta.slot,
                commitment: block.commitment,
            });
        }
        let slot = to_i64(meta.slot)?;
        let parent_slot = to_i64(meta.parent_slot)?;
        let entry_count = to_i64(meta.entry_count)?;
        let transaction_count = to_i64(meta.executed_transaction_count)?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        type StoredIdentity = (Vec<u8>, Vec<u8>, i64, i64, i64);
        let stored: Option<StoredIdentity> = tx
            .query_row(
                "SELECT blockhash, parent_blockhash, parent_slot, entry_count, \
                 executed_transaction_count FROM blocks WHERE slot = ?1",
                [slot],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .optional()?;
        let outcome = match stored {
            Some((hash, parent, stored_parent_slot, entries, transactions)) => {
                if hash != meta.blockhash.0
                    || parent != meta.parent_blockhash.0
                    || stored_parent_slot != parent_slot
                    || entries != entry_count
                    || transactions != transaction_count
                {
                    return Err(SinkError::Mismatch { slot: meta.slot });
                }
                Commit::AlreadyPresent
            }
            None => {
                tx.execute(
                    "INSERT INTO blocks (slot, blockhash, parent_slot, parent_blockhash, \
                     entry_count, executed_transaction_count, payload) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        slot,
                        meta.blockhash.0.as_slice(),
                        parent_slot,
                        meta.parent_blockhash.0.as_slice(),
                        entry_count,
                        transaction_count,
                        block.payload,
                    ],
                )?;
                Commit::Inserted
            }
        };
        advance(&tx)?;
        tx.commit()?;
        Ok(outcome)
    }

    /// Highest slot such that it and every linked ancestor back to the anchor are stored.
    pub fn watermark(&self) -> Result<u64, SinkError> {
        let slot: i64 =
            self.conn
                .query_row("SELECT slot FROM watermark WHERE id = 0", [], |row| {
                    row.get(0)
                })?;
        to_u64(slot)
    }

    /// The `Config::first_slot` to restart with.
    pub fn resume_slot(&self) -> Result<u64, SinkError> {
        Ok(self.watermark()?.saturating_add(1))
    }

    pub fn len(&self) -> Result<u64, SinkError> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM blocks", [], |row| row.get(0))?;
        to_u64(count)
    }

    pub fn is_empty(&self) -> Result<bool, SinkError> {
        Ok(self.len()? == 0)
    }

    pub fn payload(&self, slot: u64) -> Result<Option<Vec<u8>>, SinkError> {
        Ok(self
            .conn
            .query_row(
                "SELECT payload FROM blocks WHERE slot = ?1",
                [to_i64(slot)?],
                |row| row.get(0),
            )
            .optional()?)
    }

    pub fn blockhash(&self, slot: u64) -> Result<Option<BlockHash>, SinkError> {
        let stored: Option<Vec<u8>> = self
            .conn
            .query_row(
                "SELECT blockhash FROM blocks WHERE slot = ?1",
                [to_i64(slot)?],
                |row| row.get(0),
            )
            .optional()?;
        stored
            .map(|bytes| {
                <[u8; 32]>::try_from(bytes)
                    .map(BlockHash)
                    .map_err(|_| SinkError::Corrupt("blockhash is not 32 bytes"))
            })
            .transpose()
    }
}
