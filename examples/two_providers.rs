//! Opt-in read-only gRPC demonstration. Credentials come from environment variables only.
//! Prints reconciled metadata; it does not implement a durable application sink.
use futures_util::StreamExt;
use std::{collections::HashMap, env, time::Duration};
use tokio::sync::{mpsc, oneshot};
use yellowstone_block_machine::{
    dragonsmouth::block_accumulator::{BankBuffer, DragonsmouthBlockCumulator},
    stream::{BlockMachineOutput, BlockStream},
    yellowstone_grpc_client::{ClientTlsConfig, GeyserGrpcBuilder},
    yellowstone_grpc_proto::geyser::{
        SubscribeRequest, SubscribeRequestFilterAccounts, SubscribeRequestFilterBlockFooter,
        SubscribeRequestFilterSlots, SubscribeUpdate,
    },
};
use yellowstone_reconcile::{
    upstream::UpstreamBridge,
    wire::{GuardedError, GuardedSource, WireError},
    Config, Outcome, ProviderId, Session,
};

enum Message {
    Connected {
        provider: ProviderId,
        reply: oneshot::Sender<Session>,
    },
    Event {
        session: Session,
        event: Box<BlockMachineOutput<BankBuffer>>,
    },
    Exhausted {
        provider: ProviderId,
    },
    InputRejected {
        provider: ProviderId,
        reason: WireError,
    },
}

// Subscription recipe follows the pinned upstream GeyserGrpcExt implementation (Apache-2.0).
fn request() -> SubscribeRequest {
    let name = "_block-machine".to_owned();
    SubscribeRequest {
        commitment: Some(0),
        slots: HashMap::from([(
            name.clone(),
            SubscribeRequestFilterSlots {
                interslot_updates: Some(true),
                ..Default::default()
            },
        )]),
        blocks_meta: HashMap::from([(name.clone(), Default::default())]),
        entry: HashMap::from([(name.clone(), Default::default())]),
        accounts: HashMap::from([(
            name.clone(),
            SubscribeRequestFilterAccounts {
                owner: vec!["Sysvar1111111111111111111111111111111111111".into()],
                ..Default::default()
            },
        )]),
        block_footer: HashMap::from([(
            name.clone(),
            SubscribeRequestFilterBlockFooter {
                include_certificates: Some(false),
            },
        )]),
        transactions: HashMap::from([(name, Default::default())]),
        ..Default::default()
    }
}

async fn provider_task(
    provider: ProviderId,
    endpoint: String,
    token: Option<String>,
    sender: mpsc::Sender<Message>,
) {
    // Each reconnection creates a fresh upstream BlockStream and a fresh local session.
    for _ in 0..3 {
        let Ok(builder) = GeyserGrpcBuilder::from_shared(endpoint.clone()) else {
            break;
        };
        let Ok(builder) = builder.x_token(token.clone()) else {
            break;
        };
        let Ok(builder) = builder.tls_config(ClientTlsConfig::new().with_native_roots()) else {
            break;
        };
        if let Ok(mut client) = builder.connect().await {
            if let Ok((_sink, source)) = client.subscribe_with_request(Some(request())).await {
                let (reply, receive) = oneshot::channel();
                if sender
                    .send(Message::Connected { provider, reply })
                    .await
                    .is_err()
                {
                    return;
                }
                let Ok(session) = receive.await else {
                    return;
                };
                let Ok(source) = GuardedSource::new(source, 8192) else {
                    return;
                };
                let mut stream = BlockStream::<_, SubscribeUpdate, _>::new(
                    source,
                    DragonsmouthBlockCumulator::default(),
                    solana_commitment_config::CommitmentLevel::Finalized,
                );
                while let Some(item) = stream.next().await {
                    let event = match item {
                        Ok(event) => event,
                        Err(GuardedError::InvalidWire(reason)) => {
                            let _ = sender
                                .send(Message::InputRejected { provider, reason })
                                .await;
                            return;
                        }
                        Err(GuardedError::Source(_)) => break,
                    };
                    if sender
                        .send(Message::Event {
                            session,
                            event: Box::new(event),
                        })
                        .await
                        .is_err()
                    {
                        return;
                    }
                }
            }
        }
        // Errors may contain endpoint credentials, so the example deliberately emits no raw error.
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    let _ = sender.send(Message::Exhausted { provider }).await;
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a = env::var("YELLOWSTONE_A_URL").map_err(|_| "set YELLOWSTONE_A_URL")?;
    let b = env::var("YELLOWSTONE_B_URL").map_err(|_| "set YELLOWSTONE_B_URL")?;
    if a == b {
        return Err("configure independent providers, not the same endpoint twice".into());
    }
    for url in [&a, &b] {
        if !url.starts_with("https://") {
            return Err("this example requires HTTPS endpoints".into());
        }
    }
    let max_blocks: u64 = env::var("RECONCILE_MAX_BLOCKS")
        .unwrap_or_else(|_| "100".into())
        .parse()?;
    if max_blocks == 0 || max_blocks > 1000 {
        return Err("RECONCILE_MAX_BLOCKS must be 1..=1000".into());
    }
    let max_seconds: u64 = env::var("RECONCILE_MAX_SECONDS")
        .unwrap_or_else(|_| "300".into())
        .parse()?;
    // The wire guard ends a connection after 8192 distinct banks, which at 212 ms per slot and one
    // bank per slot is about 1,737 s. Forks add banks, so stay well under. Longer runs need
    // connection rotation, which this example does not implement.
    if max_seconds == 0 || max_seconds > 1200 {
        return Err("RECONCILE_MAX_SECONDS must be 1..=1200".into());
    }
    let started = tokio::time::Instant::now();
    let deadline = started + Duration::from_secs(max_seconds);
    let mut connections = [0u64; 2];
    let mut events = [0u64; 2];
    let mut first_slot = None;
    let mut last_slot = None;
    let (sender, mut receive) = mpsc::channel(64);
    let mut workers = Vec::new();
    for (provider, endpoint, token_key) in [
        (ProviderId(1), a, "YELLOWSTONE_A_TOKEN"),
        (ProviderId(2), b, "YELLOWSTONE_B_TOKEN"),
    ] {
        workers.push(tokio::spawn(provider_task(
            provider,
            endpoint,
            env::var(token_key).ok(),
            sender.clone(),
        )));
    }
    drop(sender);
    let mut bridge = UpstreamBridge::new(Config::default(), 128, 100_000)?;
    let result: Result<(), Box<dyn std::error::Error>> = loop {
        let message = tokio::select! {
            _ = tokio::signal::ctrl_c() => break Err("pilot interrupted before target".into()),
            _ = tokio::time::sleep_until(deadline) => break Err("pilot deadline reached before target".into()),
            message = receive.recv() => message,
            _ = tokio::time::sleep(Duration::from_secs(60)) => break Err("no provider activity for 60 seconds".into()),
        };
        match message {
            Some(Message::Connected { provider, reply }) => {
                connections[usize::from(provider.0 - 1)] += 1;
                match bridge.connect(provider) {
                    Ok(session) => {
                        let _ = reply.send(session);
                    }
                    Err(_) => break Err("bridge refused a provider connection".into()),
                }
            }
            Some(Message::Event { session, event }) => {
                events[usize::from(session.provider.0 - 1)] += 1;
                let outcome = match bridge.ingest(session, *event) {
                    Ok(outcome) => outcome,
                    Err(error) => break Err(format!("bridge rejected input: {error}").into()),
                };
                match outcome {
                    Some(Outcome::Emitted { block, providers }) => {
                        first_slot = Some(
                            first_slot.map_or(block.meta.slot, |old: u64| old.min(block.meta.slot)),
                        );
                        last_slot = Some(
                            last_slot.map_or(block.meta.slot, |old: u64| old.max(block.meta.slot)),
                        );
                        println!(
                            "{}",
                            serde_json::json!({"kind":"emitted","slot":block.meta.slot,"blockhash":block.meta.blockhash,"providers":providers})
                        );
                        if bridge.core().stats().emitted >= max_blocks {
                            break Ok(());
                        }
                    }
                    Some(Outcome::Conflict { .. } | Outcome::Quarantined { .. }) => {
                        break Err("provider disagreement: stop and investigate".into())
                    }
                    _ => (),
                }
            }
            Some(Message::Exhausted { provider }) => {
                break Err(format!("provider {} exhausted reconnect attempts", provider.0).into())
            }
            Some(Message::InputRejected { provider, reason }) => {
                break Err(format!("provider {} input rejected: {reason}", provider.0).into())
            }
            None => break Err("both provider tasks stopped".into()),
        }
    };
    for worker in workers {
        worker.abort();
    }
    println!(
        "{}",
        serde_json::json!({
            "kind":"summary", "target_reached":result.is_ok(),
            // Every error above is a fixed message, a WireError or a BridgeError; none holds an endpoint.
            "failure":result.as_ref().err().map(|error| error.to_string()),
            "elapsed_seconds":started.elapsed().as_secs_f64(),
            "connections":connections, "upstream_events":events,
            "first_slot":first_slot, "last_slot":last_slot,
            "stats":bridge.core().stats(), "bridge_pending":bridge.pending_blocks(),
            "upstream_fork_reports":bridge.upstream_fork_reports()
        })
    );
    result
}
