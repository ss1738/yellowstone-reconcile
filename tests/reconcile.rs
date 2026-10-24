use yellowstone_reconcile::*;

fn hash(n: u64) -> BlockHash {
    let mut bytes = [1; 32];
    bytes[..8].copy_from_slice(&n.to_le_bytes());
    BlockHash(bytes)
}
fn block(slot: u64, id: u64, payload: &str) -> SealedBlock<String> {
    SealedBlock {
        meta: BlockMeta {
            slot,
            blockhash: hash(id),
            parent_slot: slot - 1,
            parent_blockhash: hash(id.saturating_sub(1)),
            entry_count: 3,
            executed_transaction_count: 2,
        },
        commitment: Commitment::Finalized,
        payload: payload.into(),
    }
}
fn setup(config: Config) -> (Reconciler<String>, Session, Session) {
    let mut core = Reconciler::new(config).unwrap();
    let a = core.connect(ProviderId(1)).unwrap();
    let b = core.connect(ProviderId(2)).unwrap();
    (core, a, b)
}

#[test]
fn independent_providers_emit_one_unchanged_payload() {
    let (mut core, a, b) = setup(Config::default());
    assert!(matches!(
        core.observe(a, block(10, 10, "provider-a-only")),
        Ok(Outcome::Pending {
            agreeing_providers: 1,
            ..
        })
    ));
    let Outcome::Emitted { block, providers } =
        core.observe(b, block(10, 10, "provider-b-only")).unwrap()
    else {
        panic!()
    };
    assert_eq!(block.payload, "provider-a-only");
    assert_eq!(providers, vec![ProviderId(1), ProviderId(2)]);
    assert_eq!(core.stats().pending_blocks, 0);
}

#[test]
fn reconnect_is_not_an_independent_vote_and_invalidates_old_session() {
    let (mut core, a, b) = setup(Config::default());
    core.observe(a, block(10, 10, "original")).unwrap();
    let next = core.connect(a.provider).unwrap();
    assert_eq!(next.generation, 2);
    assert!(matches!(
        core.observe(next, block(10, 10, "replay")).unwrap(),
        Outcome::Pending {
            agreeing_providers: 1,
            ..
        }
    ));
    assert_eq!(
        core.observe(a, block(10, 10, "stale")).unwrap_err(),
        Error::UnknownSession
    );
    assert!(matches!(
        core.observe(b, block(10, 10, "peer")).unwrap(),
        Outcome::Emitted { .. }
    ));
    assert!(matches!(
        core.observe(next, block(10, 10, "duplicate")).unwrap(),
        Outcome::Duplicate { .. }
    ));
    assert_eq!(core.stats().emitted, 1);
}

#[test]
fn conflict_before_quorum_quarantines_and_releases_payload() {
    let (mut core, a, b) = setup(Config::default());
    core.observe(a, block(10, 10, "x")).unwrap();
    assert!(matches!(
        core.observe(b, block(10, 99, "y")).unwrap(),
        Outcome::Conflict {
            previously_emitted: false,
            ..
        }
    ));
    assert_eq!(core.stats().pending_blocks, 0);
    assert!(matches!(
        core.observe(b, block(10, 10, "x")).unwrap(),
        Outcome::Quarantined { .. }
    ));
    assert_eq!(core.stats().emitted, 0);
}

#[test]
fn conflict_after_emission_is_explicit_and_never_reemits() {
    let (mut core, a, b) = setup(Config::default());
    core.observe(a, block(10, 10, "x")).unwrap();
    core.observe(b, block(10, 10, "x")).unwrap();
    assert!(matches!(
        core.observe(a, block(10, 11, "y")).unwrap(),
        Outcome::Conflict {
            previously_emitted: true,
            ..
        }
    ));
    assert!(matches!(
        core.observe(a, block(10, 10, "x")).unwrap(),
        Outcome::Quarantined { .. }
    ));
    assert_eq!(core.stats().emitted, 1);
}

#[test]
fn equal_hash_different_metadata_is_a_conflict() {
    for field in 0..4 {
        let (mut core, a, b) = setup(Config::default());
        core.observe(a, block(10, 10, "x")).unwrap();
        let mut different = block(10, 10, "y");
        match field {
            0 => different.meta.entry_count += 1,
            1 => different.meta.parent_slot -= 1,
            2 => different.meta.parent_blockhash = hash(88),
            _ => different.meta.executed_transaction_count += 1,
        }
        assert!(matches!(
            core.observe(b, different).unwrap(),
            Outcome::Conflict { .. }
        ));
    }
}

#[test]
fn below_commitment_cannot_vote_or_poison_slot() {
    let (mut core, a, b) = setup(Config::default());
    for level in [Commitment::Processed, Commitment::Confirmed] {
        let mut input = block(10, 99, "bad");
        input.commitment = level;
        assert_eq!(core.observe(a, input).unwrap_err(), Error::BelowCommitment);
    }
    assert_eq!(core.stats().retained_slots, 0);
    core.observe(a, block(10, 10, "a")).unwrap();
    assert!(matches!(
        core.observe(b, block(10, 10, "b")).unwrap(),
        Outcome::Emitted { .. }
    ));
}

#[test]
fn confirmed_mode_is_explicit_and_never_accepts_processed() {
    let config = Config {
        minimum_commitment: Commitment::Confirmed,
        minimum_providers: 1,
        ..Config::default()
    };
    let (mut core, a, _) = setup(config);
    let mut input = block(10, 10, "a");
    input.commitment = Commitment::Confirmed;
    assert!(matches!(
        core.observe(a, input).unwrap(),
        Outcome::Emitted {
            block: SealedBlock {
                commitment: Commitment::Confirmed,
                ..
            },
            ..
        }
    ));
}

#[test]
fn explicit_retention_reports_gaps_and_rejects_old_replay() {
    let (mut core, a, b) = setup(Config::default());
    core.observe(a, block(10, 10, "pending")).unwrap();
    core.observe(a, block(11, 11, "conflict")).unwrap();
    core.observe(b, block(11, 12, "conflict")).unwrap();
    core.observe(a, block(12, 12, "keep")).unwrap();
    let report = core.advance(12).unwrap();
    assert_eq!(report.pending_slots, vec![10]);
    assert_eq!(report.quarantined_slots, vec![11]);
    assert_eq!(core.stats().retained_slots, 1);
    assert_eq!(core.stats().pending_blocks, 1);
    assert_eq!(
        core.observe(b, block(10, 10, "replay")).unwrap_err(),
        Error::StaleSlot {
            slot: 10,
            first_slot: 12
        }
    );
    assert_eq!(core.advance(11).unwrap_err(), Error::CheckpointRegression);
}

#[test]
fn pending_capacity_does_not_lose_existing_quorum_progress() {
    let config = Config {
        max_pending_blocks: 1,
        ..Config::default()
    };
    let (mut core, a, b) = setup(config);
    core.observe(a, block(10, 10, "a")).unwrap();
    assert_eq!(
        core.observe(a, block(11, 11, "b")).unwrap_err(),
        Error::PendingCapacity
    );
    assert!(matches!(
        core.observe(b, block(10, 10, "a")).unwrap(),
        Outcome::Emitted { .. }
    ));
    assert!(matches!(
        core.observe(a, block(11, 11, "b")).unwrap(),
        Outcome::Pending { .. }
    ));
}

#[test]
fn slot_and_provider_capacities_are_recoverable() {
    let config = Config {
        minimum_providers: 1,
        max_retained_slots: 1,
        max_pending_blocks: 1,
        max_providers: 2,
        ..Config::default()
    };
    let (mut core, a, _) = setup(config);
    assert_eq!(
        core.connect(ProviderId(3)).unwrap_err(),
        Error::ProviderCapacity
    );
    core.observe(a, block(10, 10, "a")).unwrap();
    assert_eq!(
        core.observe(a, block(11, 11, "b")).unwrap_err(),
        Error::SlotCapacity
    );
    core.advance(11).unwrap();
    assert!(matches!(
        core.observe(a, block(11, 11, "b")).unwrap(),
        Outcome::Emitted { .. }
    ));
}

#[test]
fn malformed_hash_and_parent_are_rejected_without_allocating_slot() {
    assert!("é".repeat(32).parse::<BlockHash>().is_err());
    assert!("g".repeat(64).parse::<BlockHash>().is_err());
    assert!("".parse::<BlockHash>().is_err());
    assert_eq!(hash(42).to_string().parse::<BlockHash>().unwrap(), hash(42));
    let (mut core, a, _) = setup(Config::default());
    for field in 0..3 {
        let mut input = block(10, 10, "x");
        match field {
            0 => input.meta.blockhash = BlockHash([0; 32]),
            1 => input.meta.parent_slot = 10,
            _ => input.meta.parent_blockhash = BlockHash([0; 32]),
        }
        assert_eq!(core.observe(a, input).unwrap_err(), Error::InvalidBlock);
    }
    assert_eq!(core.stats().retained_slots, 0);
}

#[test]
fn invalid_configuration_is_rejected() {
    for config in [
        Config {
            minimum_commitment: Commitment::Processed,
            ..Config::default()
        },
        Config {
            minimum_providers: 0,
            ..Config::default()
        },
        Config {
            minimum_providers: 9,
            ..Config::default()
        },
        Config {
            max_pending_blocks: 0,
            ..Config::default()
        },
        Config {
            max_retained_slots: 1,
            ..Config::default()
        },
    ] {
        assert!(matches!(
            Reconciler::<()>::new(config),
            Err(Error::InvalidConfig)
        ));
    }
}

#[test]
fn randomized_interleavings_emit_each_slot_once() {
    // Reference: 3 providers report the same 500 blocks, twice each, in seeded random order.
    for seed in 1..=32u64 {
        let mut rng = seed;
        let mut core = Reconciler::new(Config {
            max_pending_blocks: 4096,
            ..Config::default()
        })
        .unwrap();
        let sessions: Vec<_> = (0..3)
            .map(|p| core.connect(ProviderId(p)).unwrap())
            .collect();
        let mut events = Vec::new();
        for slot in 1..=500 {
            for session in &sessions {
                for _ in 0..2 {
                    events.push((*session, slot));
                }
            }
        }
        for i in (1..events.len()).rev() {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            events.swap(i, rng as usize % (i + 1));
        }
        let mut emitted = std::collections::BTreeSet::new();
        for (session, slot) in events {
            if let Outcome::Emitted { block, .. } =
                core.observe(session, block(slot, slot, "payload")).unwrap()
            {
                assert!(emitted.insert(block.meta.slot), "duplicate output");
            }
        }
        assert_eq!(emitted, (1..=500).collect());
        assert_eq!(core.stats().pending_blocks, 0);
        assert_eq!(core.stats().conflicts, 0);
    }
}

#[test]
fn retention_stays_bounded_under_long_running_replay() {
    let config = Config {
        max_retained_slots: 16,
        max_pending_blocks: 16,
        ..Config::default()
    };
    let (mut core, a, b) = setup(config);
    for slot in 1..=100_000 {
        if slot > 16 {
            core.advance(slot - 15).unwrap();
        }
        core.observe(a, block(slot, slot, "a")).unwrap();
        core.observe(b, block(slot, slot, "b")).unwrap();
        assert!(core.stats().retained_slots <= 16);
    }
    assert_eq!(core.stats().emitted, 100_000);
}
