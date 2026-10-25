#![cfg(feature = "grpc")]
use futures_util::{stream, StreamExt};
use yellowstone_block_machine::yellowstone_grpc_proto::geyser::{
    subscribe_update::UpdateOneof, SlotStatus, SubscribeUpdate, SubscribeUpdateBlockMeta,
    SubscribeUpdateEntry, SubscribeUpdateSlot,
};
use yellowstone_reconcile::wire::{GuardedError, GuardedSource, WireError, WireGuard};

fn created(slot: u64, bank: u64) -> SubscribeUpdate {
    SubscribeUpdate {
        update_oneof: Some(UpdateOneof::Slot(SubscribeUpdateSlot {
            slot,
            bank_id: Some(bank),
            status: SlotStatus::SlotCreatedBank as i32,
            ..Default::default()
        })),
        ..Default::default()
    }
}
fn meta(slot: u64, bank: u64, hash: &str) -> SubscribeUpdate {
    SubscribeUpdate {
        update_oneof: Some(UpdateOneof::BlockMeta(SubscribeUpdateBlockMeta {
            slot,
            bank_id: bank,
            blockhash: hash.into(),
            parent_blockhash: "1".repeat(32),
            ..Default::default()
        })),
        ..Default::default()
    }
}

#[test]
fn malformed_blockhashes_are_rejected_without_bank_state_mutation() {
    let mut guard = WireGuard::new(4).unwrap();
    for bad in [
        "".into(),
        "invalid".into(),
        "1".repeat(31),
        "1".repeat(33),
        "é".repeat(32),
        "z".repeat(45),
    ] {
        assert!(matches!(
            guard.check(&meta(10, 7, &bad)),
            Err(WireError::InvalidHash { field: "blockhash" })
        ));
        assert_eq!(guard.tracked_banks(), 0);
    }
    guard.check(&meta(10, 7, &"1".repeat(32))).unwrap();
    assert_eq!(guard.tracked_banks(), 1);
}

#[test]
fn only_exact_length_entry_hashes_pass() {
    for len in 0..=64 {
        let mut guard = WireGuard::new(1).unwrap();
        let event = SubscribeUpdate {
            update_oneof: Some(UpdateOneof::Entry(SubscribeUpdateEntry {
                slot: 10,
                bank_id: 7,
                hash: vec![1; len],
                ..Default::default()
            })),
            ..Default::default()
        };
        assert_eq!(guard.check(&event).is_ok(), len == 32);
        assert_eq!(guard.tracked_banks(), usize::from(len == 32));
    }
}

#[test]
fn one_connection_cannot_reassign_bank_id_to_another_slot() {
    let mut guard = WireGuard::new(2).unwrap();
    guard.check(&created(10, 7)).unwrap();
    assert_eq!(
        guard.check(&meta(11, 7, &"1".repeat(32))).unwrap_err(),
        WireError::BankSlotMismatch {
            bank_id: 7,
            original_slot: 10,
            received_slot: 11
        }
    );
    guard.check(&meta(10, 7, &"1".repeat(32))).unwrap();
    assert_eq!(guard.tracked_banks(), 1);
    let mut fresh_connection = WireGuard::new(2).unwrap();
    fresh_connection.check(&created(11, 7)).unwrap();
}

#[test]
fn missing_bank_id_and_unknown_status_are_not_silently_accepted() {
    let mut guard = WireGuard::new(2).unwrap();
    let mut event = created(10, 7);
    let Some(UpdateOneof::Slot(update)) = event.update_oneof.as_mut() else {
        panic!()
    };
    update.bank_id = None;
    assert_eq!(
        guard.check(&event).unwrap_err(),
        WireError::MissingBankId {
            slot: 10,
            status: SlotStatus::SlotCreatedBank
        }
    );
    let Some(UpdateOneof::Slot(update)) = event.update_oneof.as_mut() else {
        panic!()
    };
    update.status = 999;
    assert_eq!(
        guard.check(&event).unwrap_err(),
        WireError::UnknownSlotStatus
    );
}

#[test]
fn capacity_failure_preserves_old_bank_identity() {
    assert!(WireGuard::new(0).is_err());
    let mut guard = WireGuard::new(1).unwrap();
    guard.check(&created(10, 7)).unwrap();
    assert_eq!(
        guard.check(&created(11, 8)).unwrap_err(),
        WireError::BankCapacity
    );
    guard.check(&created(10, 7)).unwrap();
    assert_eq!(guard.tracked_banks(), 1);
}

#[tokio::test]
async fn guarded_stream_reports_invalid_input_then_terminates() {
    let source = stream::iter(vec![
        Ok::<_, std::convert::Infallible>(created(10, 7)),
        Ok(meta(11, 7, &"1".repeat(32))),
        Ok(created(10, 7)),
    ]);
    let mut source = GuardedSource::new(source, 8).unwrap();
    assert!(source.next().await.unwrap().is_ok());
    assert!(matches!(
        source.next().await,
        Some(Err(GuardedError::InvalidWire(
            WireError::BankSlotMismatch { .. }
        )))
    ));
    assert!(source.next().await.is_none());
}

#[tokio::test]
async fn guarded_stream_reports_source_error_without_leaking_its_display() {
    let mut source = GuardedSource::new(
        stream::iter(vec![Err::<SubscribeUpdate, _>("SECRET_ENDPOINT_DETAIL")]),
        8,
    )
    .unwrap();
    let error = source.next().await.unwrap().unwrap_err();
    assert_eq!(error.to_string(), "upstream source error");
    assert!(source.next().await.is_none());
}

fn slot_update(status: SlotStatus, bank: Option<u64>) -> SubscribeUpdate {
    SubscribeUpdate {
        update_oneof: Some(UpdateOneof::Slot(SubscribeUpdateSlot {
            slot: 10,
            bank_id: bank,
            status: status as i32,
            ..Default::default()
        })),
        ..Default::default()
    }
}

/// The PublicNode run stopped on `MissingBankId` but logged only the slot, so the status that
/// lacked a bank could not be identified afterwards. The error must carry it.
#[test]
fn missing_bank_id_names_the_status_that_lacked_it() {
    for (status, name) in [
        (SlotStatus::SlotCreatedBank, "SlotCreatedBank"),
        (SlotStatus::SlotProcessed, "SlotProcessed"),
        (SlotStatus::SlotConfirmed, "SlotConfirmed"),
        (SlotStatus::SlotFinalized, "SlotFinalized"),
    ] {
        let error = WireGuard::new(2)
            .unwrap()
            .check(&slot_update(status, None))
            .unwrap_err();
        assert!(
            error.to_string().contains(name),
            "{error} does not name {name}"
        );
    }
}

/// Slot-scoped statuses carry no bank by protocol definition, and a present bank id of zero is
/// a bank, not an absence.
#[test]
fn slot_scoped_statuses_need_no_bank_and_zero_is_a_present_bank() {
    for status in [
        SlotStatus::SlotFirstShredReceived,
        SlotStatus::SlotCompleted,
        SlotStatus::SlotDead,
    ] {
        let mut guard = WireGuard::new(2).unwrap();
        guard.check(&slot_update(status, None)).unwrap();
        assert_eq!(guard.tracked_banks(), 0);
    }
    let mut guard = WireGuard::new(2).unwrap();
    guard
        .check(&slot_update(SlotStatus::SlotFinalized, Some(0)))
        .unwrap();
    assert_eq!(guard.tracked_banks(), 1);
}
