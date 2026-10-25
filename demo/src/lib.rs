//! Browser build of the replay driver. The reconciliation is the library's own `Reconciler`;
//! this file only parses events the way `src/bin/replay.rs` does and returns one JSON document.
//!
//! The interface is plain C ABI so no JavaScript glue generator is needed:
//! `alloc` a buffer, write the JSONL into it, call `replay`, read `result_ptr`/`result_len`.

use serde::Deserialize;
use std::cell::RefCell;
use yellowstone_reconcile::{
    BlockMeta, Commitment, Config, Outcome, ProviderId, Reconciler, SealedBlock, Session,
};

const MAX_INPUT: usize = 2 * 1024 * 1024;
const MAX_LINE: usize = 1024 * 1024;

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

/// Returns the lines to show and, if the run stopped early, why. Stops exactly where the CLI does:
/// on a conflict, a quarantined slot, or any error.
fn run(input: &[u8], minimum_providers: usize) -> serde_json::Value {
    let mut steps = Vec::new();
    let mut halted: Option<String> = None;
    let mut core = match Reconciler::<String>::new(Config {
        minimum_providers,
        ..Config::default()
    }) {
        Ok(core) => core,
        Err(e) => {
            return serde_json::json!({"steps": steps, "halted": e.to_string(), "stats": null})
        }
    };
    if input.len() > MAX_INPUT {
        return serde_json::json!({"steps": steps, "halted": "input exceeds 2 MiB", "stats": null});
    }
    for (index, bytes) in input.split(|b| *b == b'\n').enumerate() {
        let line = index + 1;
        if bytes.len() > MAX_LINE {
            halted = Some(format!("line {line}: exceeds 1 MiB"));
            break;
        }
        if bytes.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        let event: Event = match serde_json::from_slice(bytes) {
            Ok(event) => event,
            Err(e) => {
                halted = Some(format!("line {line}: {e}"));
                break;
            }
        };
        let step = match event {
            Event::Connect { provider } => match core.connect(provider) {
                Ok(session) => {
                    serde_json::json!({"line": line, "kind": "connect", "provider": provider, "session": session})
                }
                Err(e) => {
                    halted = Some(format!("line {line}: {e}"));
                    break;
                }
            },
            Event::Block {
                provider,
                generation,
                bank_id,
                commitment,
                meta,
                payload,
            } => {
                let slot = meta.slot;
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
                        let step = serde_json::json!({
                            "line": line, "kind": "block", "provider": provider, "slot": slot,
                            "local_bank_id": bank_id, "outcome": outcome,
                        });
                        if stop {
                            halted = Some(format!("line {line}: provider conflict; replay halted"));
                            steps.push(step);
                            break;
                        }
                        step
                    }
                    Err(e) => {
                        halted = Some(format!("line {line}: {e}"));
                        break;
                    }
                }
            }
            Event::Advance { before_slot } => match core.advance(before_slot) {
                Ok(pruned) => {
                    serde_json::json!({"line": line, "kind": "advance", "pruned": pruned})
                }
                Err(e) => {
                    halted = Some(format!("line {line}: {e}"));
                    break;
                }
            },
        };
        steps.push(step);
    }
    serde_json::json!({"steps": steps, "halted": halted, "stats": core.stats()})
}

thread_local! {
    static RESULT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// Reserves `len` bytes the caller can write into. Free it with `dealloc`.
#[no_mangle]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    let mut buffer = Vec::<u8>::with_capacity(len.max(1));
    let ptr = buffer.as_mut_ptr();
    std::mem::forget(buffer);
    ptr
}

/// # Safety
/// `ptr` and `len` must be a pair previously given out by `alloc(len)`.
#[no_mangle]
pub unsafe extern "C" fn dealloc(ptr: *mut u8, len: usize) {
    drop(Vec::from_raw_parts(ptr, 0, len.max(1)));
}

/// Replays `len` bytes of JSONL at `ptr` and stores the JSON result. Returns its length.
///
/// # Safety
/// `ptr` must point at `len` initialised bytes.
#[no_mangle]
pub unsafe extern "C" fn replay(ptr: *const u8, len: usize, minimum_providers: usize) -> usize {
    let input = std::slice::from_raw_parts(ptr, len);
    let out = serde_json::to_vec(&run(input, minimum_providers)).unwrap_or_default();
    RESULT.with(|r| {
        *r.borrow_mut() = out;
        r.borrow().len()
    })
}

#[no_mangle]
pub extern "C" fn result_ptr() -> *const u8 {
    RESULT.with(|r| r.borrow().as_ptr())
}

#[cfg(test)]
mod tests {
    use super::run;

    fn fixture(name: &str) -> Vec<u8> {
        std::fs::read(format!(
            "{}/../fixtures/{name}.jsonl",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap()
    }

    #[test]
    fn agreement_replays_to_completion() {
        let out = run(&fixture("agreement"), 2);
        assert!(out["halted"].is_null(), "{out}");
        assert!(out["stats"]["emitted"].as_u64().unwrap() >= 1);
    }

    #[test]
    fn conflict_halts_on_the_conflicting_line() {
        let out = run(&fixture("conflict"), 2);
        assert!(
            out["halted"].as_str().unwrap().contains("conflict"),
            "{out}"
        );
        assert_eq!(out["stats"]["conflicts"], 1);
    }

    #[test]
    fn malformed_names_the_line_and_emits_nothing() {
        let out = run(&fixture("malformed"), 2);
        assert!(
            out["halted"].as_str().unwrap().starts_with("line 1:"),
            "{out}"
        );
        assert_eq!(out["steps"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn oversize_input_is_refused() {
        let big = vec![b' '; 2 * 1024 * 1024 + 1];
        assert_eq!(run(&big, 2)["halted"], "input exceeds 2 MiB");
    }
}
