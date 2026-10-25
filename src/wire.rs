//! Fail-closed input boundary for the pinned upstream reconstructor's known issue #19.
//!
//! A guard belongs to exactly one connection and must be replaced along with BlockStream.
//! It validates known hash panic paths and prevents a local bank id from changing slots.
//! It is not a complete protobuf/ledger validator or a replacement for upstream fixes.

use futures_util::Stream;
use std::{
    collections::BTreeMap,
    fmt,
    pin::Pin,
    task::{Context, Poll},
};
use yellowstone_block_machine::yellowstone_grpc_proto::{
    geyser::{subscribe_update::UpdateOneof, SlotStatus, SubscribeUpdate},
    prost::{self, Message},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WireError {
    InvalidHash {
        field: &'static str,
    },
    /// A bank-scoped slot status arrived without `bank_id`. The status is recorded because the
    /// pinned protocol makes only `FirstShredReceived`, `Completed` and `Dead` bankless.
    MissingBankId {
        slot: u64,
        status: SlotStatus,
    },
    UnknownSlotStatus,
    BankSlotMismatch {
        bank_id: u64,
        original_slot: u64,
        received_slot: u64,
    },
    BankCapacity,
}
impl fmt::Display for WireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for WireError {}

/// Retains connection-local bank identities without eviction. At capacity the connection must end.
/// Forgetting a bank while the upstream machine still retains it would re-open the collision path.
pub struct WireGuard {
    bank_slots: BTreeMap<u64, u64>,
    max_banks: usize,
}
impl WireGuard {
    pub fn new(max_banks: usize) -> Result<Self, WireError> {
        if max_banks == 0 {
            return Err(WireError::BankCapacity);
        }
        Ok(Self {
            bank_slots: BTreeMap::new(),
            max_banks,
        })
    }
    pub fn tracked_banks(&self) -> usize {
        self.bank_slots.len()
    }

    /// Reject malformed input before it can reach the upstream adapter or mutate the bank map.
    pub fn check(&mut self, event: &SubscribeUpdate) -> Result<(), WireError> {
        let bank = match event.update_oneof.as_ref() {
            Some(UpdateOneof::BlockMeta(meta)) => {
                validate_hash(&meta.blockhash, "blockhash")?;
                // Preserve the upstream wire convention for an empty genesis parent hash.
                if !meta.parent_blockhash.is_empty() {
                    validate_hash(&meta.parent_blockhash, "parent_blockhash")?;
                }
                Some((meta.bank_id, meta.slot))
            }
            Some(UpdateOneof::Entry(entry)) => {
                if entry.hash.len() != 32 {
                    return Err(WireError::InvalidHash {
                        field: "entry.hash",
                    });
                }
                Some((entry.bank_id, entry.slot))
            }
            Some(UpdateOneof::BlockFooter(footer)) => {
                if footer.bank_hash.len() != 32 {
                    return Err(WireError::InvalidHash {
                        field: "footer.bank_hash",
                    });
                }
                Some((footer.bank_id, footer.slot))
            }
            Some(UpdateOneof::Slot(slot)) => {
                let status =
                    SlotStatus::try_from(slot.status).map_err(|_| WireError::UnknownSlotStatus)?;
                if matches!(
                    status,
                    SlotStatus::SlotCreatedBank
                        | SlotStatus::SlotProcessed
                        | SlotStatus::SlotConfirmed
                        | SlotStatus::SlotFinalized
                ) && slot.bank_id.is_none()
                {
                    return Err(WireError::MissingBankId {
                        slot: slot.slot,
                        status,
                    });
                }
                slot.bank_id.map(|bank| (bank, slot.slot))
            }
            Some(UpdateOneof::Account(account)) => account.bank_id.map(|bank| (bank, account.slot)),
            Some(UpdateOneof::Transaction(tx)) => Some((tx.bank_id, tx.slot)),
            Some(UpdateOneof::TransactionStatus(tx)) => Some((tx.bank_id, tx.slot)),
            Some(UpdateOneof::EntryUpdateParent(update)) => {
                Some((update.cleared_bank_id, update.slot))
            }
            _ => None,
        };
        if let Some((bank_id, slot)) = bank {
            if let Some(original) = self.bank_slots.get(&bank_id) {
                if *original != slot {
                    return Err(WireError::BankSlotMismatch {
                        bank_id,
                        original_slot: *original,
                        received_slot: slot,
                    });
                }
            } else {
                if self.bank_slots.len() >= self.max_banks {
                    return Err(WireError::BankCapacity);
                }
                self.bank_slots.insert(bank_id, slot);
            }
        }
        Ok(())
    }
}

fn validate_hash(value: &str, field: &'static str) -> Result<(), WireError> {
    if value.len() > 44
        || !value.is_ascii()
        || bs58::decode(value)
            .into_vec()
            .map_or(true, |bytes| bytes.len() != 32)
    {
        return Err(WireError::InvalidHash { field });
    }
    Ok(())
}

/// A stored block payload that does not decode to exactly the events it was written with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PayloadError;
impl fmt::Display for PayloadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "stored block payload is damaged or truncated")
    }
}
impl std::error::Error for PayloadError {}

/// Serialize a block's reconstructed events as an opaque sink payload: an event count, then each
/// event length-delimited. The count lets `decode_events` reject a payload cut between two events
/// instead of reading it as a shorter block. Holds the encoded copy alongside the originals, so
/// a multi-megabyte block needs about twice its size while being written.
pub fn encode_events<'a>(events: impl IntoIterator<Item = &'a SubscribeUpdate>) -> Vec<u8> {
    let events: Vec<&SubscribeUpdate> = events.into_iter().collect();
    let mut out = Vec::new();
    prost::encoding::encode_varint(events.len() as u64, &mut out);
    for event in events {
        event
            .encode_length_delimited(&mut out)
            .expect("a Vec grows without bound");
    }
    out
}

/// Inverse of `encode_events`. Any cut, trailing byte or wrong count is an error.
pub fn decode_events(mut bytes: &[u8]) -> Result<Vec<SubscribeUpdate>, PayloadError> {
    let count = prost::encoding::decode_varint(&mut bytes).map_err(|_| PayloadError)?;
    // The count is untrusted, so grow the vector as events decode instead of reserving it.
    let mut events = Vec::new();
    for _ in 0..count {
        events
            .push(SubscribeUpdate::decode_length_delimited(&mut bytes).map_err(|_| PayloadError)?);
    }
    if bytes.is_empty() {
        Ok(events)
    } else {
        Err(PayloadError)
    }
}

#[derive(Debug)]
pub enum GuardedError<E> {
    Source(E),
    InvalidWire(WireError),
}
impl<E> fmt::Display for GuardedError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Source(_) => write!(f, "upstream source error"),
            Self::InvalidWire(error) => write!(f, "invalid stream input: {error}"),
        }
    }
}
impl<E: std::error::Error + 'static> std::error::Error for GuardedError<E> {}

/// Emits an explicit terminal error on malformed input; never drops it silently and continues.
pub struct GuardedSource<S> {
    source: S,
    guard: WireGuard,
    terminal: bool,
}
impl<S> GuardedSource<S> {
    pub fn new(source: S, max_banks: usize) -> Result<Self, WireError> {
        Ok(Self {
            source,
            guard: WireGuard::new(max_banks)?,
            terminal: false,
        })
    }
    pub fn tracked_banks(&self) -> usize {
        self.guard.tracked_banks()
    }
}
impl<S, E> Stream for GuardedSource<S>
where
    S: Stream<Item = Result<SubscribeUpdate, E>> + Unpin,
{
    type Item = Result<SubscribeUpdate, GuardedError<E>>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        if this.terminal {
            return Poll::Ready(None);
        }
        match Pin::new(&mut this.source).poll_next(cx) {
            Poll::Ready(Some(Ok(event))) => match this.guard.check(&event) {
                Ok(()) => Poll::Ready(Some(Ok(event))),
                Err(error) => {
                    this.terminal = true;
                    Poll::Ready(Some(Err(GuardedError::InvalidWire(error))))
                }
            },
            Poll::Ready(Some(Err(error))) => {
                this.terminal = true;
                Poll::Ready(Some(Err(GuardedError::Source(error))))
            }
            Poll::Ready(None) => {
                this.terminal = true;
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}
