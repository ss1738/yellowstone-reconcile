//! Offline reproductions of public reports #19 and #21 against the pinned dependency.
use futures_util::StreamExt;
use solana_commitment_config::CommitmentLevel;
use yellowstone_block_machine::{
    dragonsmouth::{
        block_accumulator::DragonsmouthBlockCumulator,
        simulation::{SimulatedBank, SimulationBuilder},
    },
    event::GeyserEventAdapter,
    stream::{BlockMachineOutput, BlockStream},
    yellowstone_grpc_proto::geyser::{
        subscribe_update::UpdateOneof, SubscribeUpdate, SubscribeUpdateBlockMeta,
        SubscribeUpdateEntry,
    },
};
use yellowstone_reconcile::{
    upstream::UpstreamBridge,
    wire::{GuardedSource, WireGuard},
    Commitment, Config, Outcome, ProviderId,
};

#[tokio::main]
async fn main() {
    // These panics are deliberately caught to measure the reported dependency behavior.
    let prior_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let bad_meta = SubscribeUpdate {
        update_oneof: Some(UpdateOneof::BlockMeta(SubscribeUpdateBlockMeta {
            blockhash: "invalid".into(),
            ..Default::default()
        })),
        ..Default::default()
    };
    let bad_entry = SubscribeUpdate {
        update_oneof: Some(UpdateOneof::Entry(SubscribeUpdateEntry {
            hash: vec![1; 31],
            ..Default::default()
        })),
        ..Default::default()
    };
    let meta_panicked =
        std::panic::catch_unwind(|| SubscribeUpdate::extract_geyser_ev_info(&bad_meta)).is_err();
    let entry_panicked =
        std::panic::catch_unwind(|| SubscribeUpdate::extract_geyser_ev_info(&bad_entry)).is_err();
    std::panic::set_hook(prior_hook);
    let mut guard = WireGuard::new(8192).unwrap();
    let meta_rejected = guard.check(&bad_meta).is_err();
    let entry_rejected = guard.check(&bad_entry).is_err();
    println!("MEASURED issue19 invalid_blockhash_panicked={meta_panicked} invalid_entry_hash_panicked={entry_panicked} guarded_blockhash_rejected={meta_rejected} guarded_entry_rejected={entry_rejected}");
    assert!(meta_rejected && entry_rejected);

    let parent = SimulatedBank::new(40, 400);
    let first = SimulatedBank::new(41, 410).with_parent(40, parent.blockhash);
    let dead = SimulatedBank::new(42, 10)
        .with_parent(41, first.blockhash)
        .with_skip_footer();
    let repair = SimulatedBank::new(42, 11).with_parent(41, first.blockhash);
    let mut source = SimulationBuilder::new()
        .bank(&first)
        .confirmed(41, 410)
        .bank(&dead)
        .dead(42, Some(10), "synthetic dead bank")
        .bank(&repair)
        .confirmed(42, 11);
    let mut parent_hash = repair.blockhash;
    for slot in 43..=110 {
        let bank = SimulatedBank::new(slot, slot * 10).with_parent(slot - 1, parent_hash);
        parent_hash = bank.blockhash;
        source = source.bank(&bank).confirmed(slot, bank.bank_id);
        if slot >= 74 {
            let rooted = slot - 32;
            source = source.finalized(rooted, if rooted == 42 { 11 } else { rooted * 10 });
        }
    }
    let source = GuardedSource::new(source.build(), 8192).unwrap();
    let mut stream = BlockStream::<_, SubscribeUpdate, _>::new(
        source,
        DragonsmouthBlockCumulator::default(),
        CommitmentLevel::Confirmed,
    );
    let mut bridge = UpstreamBridge::new(
        Config {
            minimum_commitment: Commitment::Confirmed,
            minimum_providers: 1,
            ..Config::default()
        },
        128,
        100,
    )
    .unwrap();
    let session = bridge.connect(ProviderId(1)).unwrap();
    let mut fork_reports = 0;
    let mut emitted_slots = Vec::new();
    while let Some(event) = stream.next().await {
        let event = event.unwrap();
        if matches!(event, BlockMachineOutput::ForkDetected(_)) {
            fork_reports += 1;
        }
        if let Some(Outcome::Emitted { block, .. }) = bridge.ingest(session, event).unwrap() {
            emitted_slots.push(block.meta.slot);
        }
    }
    println!("MEASURED issue21 upstream_fork_reports={fork_reports} emitted_slots={} bridge_fork_diagnostics={}", emitted_slots.len(), bridge.upstream_fork_reports());
    assert_eq!(emitted_slots, (41..=110).collect::<Vec<_>>());
    println!("LIMIT: synthetic reproduction against pinned 0.12.0; no production incidence or complete dependency correctness claim");
}
