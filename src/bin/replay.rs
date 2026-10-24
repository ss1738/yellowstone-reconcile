use serde::Deserialize;
use std::{
    env,
    fs::File,
    io::{self, BufRead, BufReader, Read, Write},
};
use yellowstone_reconcile::{
    BlockMeta, Commitment, Config, Outcome, ProviderId, Reconciler, SealedBlock, Session,
};

#[derive(Deserialize)]
#[serde(tag = "event", rename_all = "snake_case", deny_unknown_fields)]
enum Event {
    Connect {
        provider: ProviderId,
    },
    Block {
        provider: ProviderId,
        generation: u64,
        bank_id: u64,
        commitment: Commitment,
        meta: BlockMeta,
        payload: String,
    },
    Advance {
        before_slot: u64,
    },
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let path = args
        .next()
        .ok_or("usage: yellowstone-replay FILE.jsonl [MIN_PROVIDERS]")?;
    let minimum_providers = args.next().map(|s| s.parse()).transpose()?.unwrap_or(2);
    if args.next().is_some() {
        return Err("too many arguments".into());
    }
    let mut core = Reconciler::<String>::new(Config {
        minimum_providers,
        ..Config::default()
    })?;
    let file = File::open(path)?;
    if file.metadata()?.len() > 64 * 1024 * 1024 {
        return Err("replay file exceeds 64 MiB".into());
    }
    let mut input = BufReader::new(file);
    let mut output = io::BufWriter::new(io::stdout().lock());
    // Replay inputs are local fixtures; reject oversize lines before JSON allocation.
    let mut bytes = Vec::new();
    let mut line = 0usize;
    loop {
        bytes.clear();
        let read = (&mut input)
            .take(1024 * 1024 + 1)
            .read_until(b'\n', &mut bytes)?;
        if read == 0 {
            break;
        }
        line += 1;
        if bytes.len() > 1024 * 1024 {
            return Err(format!("line {line}: exceeds 1 MiB").into());
        }
        if bytes.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        let event: Event =
            serde_json::from_slice(&bytes).map_err(|e| format!("line {line}: {e}"))?;
        let value = match event {
            Event::Connect { provider } => serde_json::json!({"session":core.connect(provider)?}),
            Event::Block {
                provider,
                generation,
                bank_id,
                commitment,
                meta,
                payload,
            } => {
                let session = Session {
                    provider,
                    generation,
                };
                match core.observe(
                    session,
                    SealedBlock {
                        meta,
                        commitment,
                        payload,
                    },
                ) {
                    Ok(outcome) => {
                        let stop = matches!(
                            outcome,
                            Outcome::Conflict { .. } | Outcome::Quarantined { .. }
                        );
                        let value = serde_json::json!({"local_bank_id":bank_id,"outcome":outcome});
                        serde_json::to_writer(&mut output, &value)?;
                        writeln!(output)?;
                        if stop {
                            output.flush()?;
                            return Err(
                                format!("line {line}: provider conflict; replay halted").into()
                            );
                        }
                        continue;
                    }
                    Err(e) => return Err(format!("line {line}: {e}").into()),
                }
            }
            Event::Advance { before_slot } => {
                serde_json::json!({"pruned":core.advance(before_slot)?})
            }
        };
        serde_json::to_writer(&mut output, &value)?;
        writeln!(output)?;
    }
    serde_json::to_writer(&mut output, &serde_json::json!({"stats":core.stats()}))?;
    writeln!(output)?;
    output.flush()?;
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
