//! A real reconstructed block payload (protobuf events from the upstream simulation, not
//! hand-made bytes) goes through two providers, the reconciler and the SQLite sink, and comes
//! back identical after the file is closed and reopened. The wire is simulated; no network.
#![cfg(all(feature = "simulation", feature = "sqlite"))]
use futures_util::StreamExt;
use solana_commitment_config::CommitmentLevel;
use yellowstone_block_machine::{
    dragonsmouth::{
        block_accumulator::DragonsmouthBlockCumulator,
        simulation::{seeded_rng, RandomBlockPlan, SimulatedBank, SimulationBuilder},
    },
    stream::{BlockEventStore, BlockStream},
    yellowstone_grpc_proto::geyser::SubscribeUpdate,
};
use yellowstone_reconcile::{
    sink::{Commit, SqliteSink},
    upstream::UpstreamBridge,
    wire::{decode_events, encode_events},
    Config, Outcome, ProviderId, SealedBlock,
};

#[tokio::test]
async fn reconstructed_block_events_survive_the_sink_and_a_reopen() {
    let path = std::env::temp_dir().join(format!("yr-real-payload-{}.db", std::process::id()));
    let _ = std::fs::remove_file(&path);

    // Slots 10..=14 chained by blockhash. The first block's parent slot (9) is the sink anchor.
    let mut parent = (9u64, SimulatedBank::new(9, 6).blockhash);
    let mut plans = Vec::new();
    for slot in 10..=14u64 {
        let blockhash = SimulatedBank::new(slot, 100 + slot).blockhash;
        plans.push((slot, blockhash, parent));
        parent = (slot, blockhash);
    }

    let mut bridge = UpstreamBridge::new(Config::default(), 16, 10_000).unwrap();
    let mut sink = SqliteSink::open(&path, 9).unwrap();
    let mut expected = Vec::new();
    for (provider, bank_offset) in [(ProviderId(1), 0u64), (ProviderId(2), 50)] {
        let session = bridge.connect(provider).unwrap();
        let mut source = SimulationBuilder::new();
        for (slot, blockhash, (parent_slot, parent_hash)) in &plans {
            let plan = RandomBlockPlan::new(*slot, slot + bank_offset)
                .with_parent(*parent_slot, *parent_hash)
                .with_blockhash(*blockhash)
                .with_transaction_count(4)
                .with_entry_count(2)
                .with_commitment(CommitmentLevel::Finalized);
            source = source.random_bank(&mut seeded_rng(*slot + bank_offset), &plan);
        }
        let mut stream = BlockStream::<_, SubscribeUpdate, _>::new(
            source.build(),
            DragonsmouthBlockCumulator::default(),
            CommitmentLevel::Finalized,
        );
        while let Some(event) = stream.next().await {
            if let Some(Outcome::Emitted { block, .. }) =
                bridge.ingest(session, event.unwrap()).unwrap()
            {
                let events: Vec<SubscribeUpdate> = block.payload.events.iter().cloned().collect();
                let bytes = encode_events(events.iter());
                let sealed = SealedBlock {
                    meta: block.meta.clone(),
                    commitment: block.commitment,
                    payload: bytes,
                };
                assert_eq!(sink.commit(&sealed).unwrap(), Commit::Inserted);
                expected.push((block.meta.slot, events));
            }
        }
    }
    assert_eq!(expected.len(), 5, "every slot should reach quorum once");
    assert!(
        expected.iter().all(|(_, events)| events.len() > 4),
        "payloads should hold real transaction, entry and account events"
    );
    drop(sink);

    // Reopen from disk: the watermark chained all five blocks and each payload decodes unchanged.
    let sink = SqliteSink::open(&path, 9).unwrap();
    assert_eq!(sink.watermark().unwrap(), 14);
    for (slot, events) in &expected {
        let stored = sink.payload(*slot).unwrap().expect("payload stored");
        assert_eq!(&decode_events(&stored).unwrap(), events, "slot {slot}");
    }
    let _ = std::fs::remove_file(&path);
}

/// A plain sequence of length-delimited messages has no count, so a payload cut exactly between
/// two messages would decode as a valid, shorter block. Every cut and any trailing byte must fail.
#[test]
fn a_damaged_payload_is_an_error_and_never_a_shorter_block() {
    let event = SubscribeUpdate {
        filters: vec!["x".into()],
        ..Default::default()
    };
    let bytes = encode_events([&event, &event, &event]);
    assert_eq!(decode_events(&bytes).unwrap().len(), 3);
    for cut in 0..bytes.len() {
        assert!(
            decode_events(&bytes[..cut]).is_err(),
            "cut at {cut} decoded"
        );
    }
    let mut longer = bytes.clone();
    longer.push(0);
    assert!(decode_events(&longer).is_err(), "trailing byte accepted");
    assert_eq!(decode_events(&encode_events([])).unwrap().len(), 0);
}
