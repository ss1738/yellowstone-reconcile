//! Read-only single-provider access check. Never prints credentials or raw transport errors.
use futures_util::StreamExt;
use std::{
    collections::HashMap,
    env,
    sync::{Arc, Mutex},
    time::Duration,
};
use yellowstone_block_machine::{
    dragonsmouth::block_accumulator::DragonsmouthBlockCumulator,
    stream::{BlockMachineOutput, BlockStream},
    yellowstone_grpc_client::{ClientTlsConfig, GeyserGrpcBuilder},
    yellowstone_grpc_proto::geyser::{
        subscribe_update::UpdateOneof, SubscribeRequest, SubscribeRequestFilterAccounts,
        SubscribeRequestFilterBlockFooter, SubscribeRequestFilterSlots, SubscribeUpdate,
    },
};
use yellowstone_reconcile::wire::{GuardedError, GuardedSource};

#[tokio::main]
async fn main() -> Result<(), &'static str> {
    let endpoint = env::var("YELLOWSTONE_A_URL").map_err(|_| "missing endpoint")?;
    let token = env::var("YELLOWSTONE_A_TOKEN").map_err(|_| "missing token")?;
    let builder = GeyserGrpcBuilder::from_shared(endpoint)
        .map_err(|_| "invalid endpoint")?
        .x_token(Some(token))
        .map_err(|_| "invalid token format")?
        .tls_config(ClientTlsConfig::new().with_native_roots())
        .map_err(|_| "TLS config failed")?;
    let mut client = tokio::time::timeout(Duration::from_secs(15), builder.connect())
        .await
        .map_err(|_| "connection timeout")?
        .map_err(|_| "connection failed")?;
    println!("{}", serde_json::json!({"stage":"connected"}));
    let name = "access-probe".to_owned();
    let request = SubscribeRequest {
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
    };
    let subscription = tokio::time::timeout(
        Duration::from_secs(15),
        client.subscribe_with_request(Some(request)),
    )
    .await
    .map_err(|_| "subscribe timeout")?;
    let (_sink, source) = match subscription {
        Ok(value) => value,
        Err(_) => {
            println!("{}", serde_json::json!({"stage":"subscribe_rejected"}));
            return Err("subscription rejected; raw error withheld");
        }
    };
    println!("{}", serde_json::json!({"stage":"subscribed"}));
    // Counts only; no transaction/account contents or authentication metadata are logged.
    let counts = Arc::new(Mutex::new([0u64; 6]));
    let shared = counts.clone();
    let source = source.inspect(move |item| {
        if let Ok(update) = item {
            let index = match &update.update_oneof {
                Some(UpdateOneof::Slot(_)) => 0,
                Some(UpdateOneof::BlockMeta(_)) => 1,
                Some(UpdateOneof::Entry(_)) => 2,
                Some(UpdateOneof::BlockFooter(_)) => 3,
                Some(UpdateOneof::Transaction(_)) => 4,
                Some(UpdateOneof::Account(_)) => 5,
                _ => return,
            };
            shared.lock().expect("counter lock")[index] += 1;
        }
    });
    let source = GuardedSource::new(source, 8192).map_err(|_| "guard setup failed")?;
    let mut stream = BlockStream::<_, SubscribeUpdate, _>::new(
        source,
        DragonsmouthBlockCumulator::default(),
        solana_commitment_config::CommitmentLevel::Finalized,
    );
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    let mut frozen = 0u64;
    let mut finalized_updates = 0u64;
    let mut failure = None;
    loop {
        let item = match tokio::time::timeout_at(deadline, stream.next()).await {
            Ok(Some(item)) => item,
            Ok(None) => {
                failure = Some("stream ended");
                break;
            }
            Err(_) => break,
        };
        match item {
            Ok(BlockMachineOutput::FrozenBlock(_)) => frozen += 1,
            Ok(BlockMachineOutput::SlotCommitmentUpdate(_)) => finalized_updates += 1,
            Ok(_) => (),
            Err(GuardedError::InvalidWire(reason)) => {
                println!(
                    "{}",
                    serde_json::json!({"stage":"wire_rejected","reason":reason.to_string()})
                );
                failure = Some("wire guard rejected input");
                break;
            }
            Err(GuardedError::Source(_)) => {
                failure = Some("stream transport error");
                break;
            }
        }
        if frozen >= 3 && finalized_updates >= 3 {
            break;
        }
    }
    let counts = *counts.lock().expect("counter lock");
    let passed = failure.is_none() && frozen >= 3 && finalized_updates >= 3;
    println!(
        "{}",
        serde_json::json!({"stage":"result","passed":passed,
        "slots":counts[0],"block_meta":counts[1],"entries":counts[2],"footers":counts[3],
        "transactions":counts[4],"accounts":counts[5],"reconstructed":frozen,
        "commitment_updates":finalized_updates,"failure":failure,
        "limit":"Single-provider access/reconstruction check; not a two-provider reconciliation pilot."})
    );
    if passed {
        Ok(())
    } else {
        Err("single-provider check incomplete")
    }
}
