#![cfg(feature = "simulation")]
use futures_util::StreamExt;
use solana_commitment_config::CommitmentLevel;
use yellowstone_block_machine::{
    dragonsmouth::{
        block_accumulator::DragonsmouthBlockCumulator,
        simulation::{seeded_rng, RandomBlockPlan, SimulatedBank, SimulationBuilder},
    },
    stream::BlockStream,
    yellowstone_grpc_proto::geyser::SubscribeUpdate,
};
use yellowstone_reconcile::{upstream::UpstreamBridge, Config, Outcome, ProviderId};

#[tokio::test]
async fn wire_events_reconstruct_independently_before_cross_provider_agreement() {
    let parent = SimulatedBank::new(9, 6);
    let blockhash = SimulatedBank::new(10, 7).blockhash;
    let mut bridge = UpstreamBridge::new(Config::default(), 16, 100).unwrap();
    let mut emitted = 0;
    for (provider, bank_id) in [(ProviderId(1), 7), (ProviderId(2), 12)] {
        let session = bridge.connect(provider).unwrap();
        let plan = RandomBlockPlan::new(10, bank_id)
            .with_parent(9, parent.blockhash)
            .with_blockhash(blockhash)
            .with_transaction_count(4)
            .with_entry_count(2)
            .with_commitment(CommitmentLevel::Finalized);
        let source = SimulationBuilder::new()
            .random_bank(&mut seeded_rng(42), &plan)
            .build();
        let mut stream = BlockStream::<_, SubscribeUpdate, _>::new(
            source,
            DragonsmouthBlockCumulator::default(),
            CommitmentLevel::Finalized,
        );
        while let Some(event) = stream.next().await {
            if let Some(Outcome::Emitted { block, providers }) =
                bridge.ingest(session, event.unwrap()).unwrap()
            {
                assert_eq!(block.meta.slot, 10);
                assert_eq!(block.meta.executed_transaction_count, 4);
                assert_eq!(providers.len(), 2);
                assert_eq!(block.payload.bank_id, 7);
                emitted += 1;
            }
        }
    }
    assert_eq!(emitted, 1);
}

#[tokio::test]
async fn incomplete_provider_bank_cannot_supply_a_quorum_vote() {
    let parent = SimulatedBank::new(9, 6);
    let reference = SimulatedBank::new(10, 7).with_parent(9, parent.blockhash);
    let mut bridge = UpstreamBridge::new(Config::default(), 16, 100).unwrap();
    for (provider, skip_footer) in [(ProviderId(1), false), (ProviderId(2), true)] {
        let session = bridge.connect(provider).unwrap();
        let mut bank = reference.clone();
        bank.skip_footer = skip_footer;
        let source = SimulationBuilder::new()
            .bank(&bank)
            .finalized(bank.slot, bank.bank_id)
            .build();
        let mut stream = BlockStream::<_, SubscribeUpdate, _>::new(
            source,
            DragonsmouthBlockCumulator::default(),
            CommitmentLevel::Finalized,
        );
        while let Some(event) = stream.next().await {
            assert!(!matches!(
                bridge.ingest(session, event.unwrap()).unwrap(),
                Some(Outcome::Emitted { .. })
            ));
        }
    }
    assert_eq!(bridge.core().stats().emitted, 0);
    assert_eq!(bridge.core().stats().pending_blocks, 1);
}
