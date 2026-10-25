# Two-provider validation protocol

MEASURED: `scripts/pilot.py` prepares a bounded read-only smoke run of
`examples/two_providers.rs`. The example has a total deadline, a block target, independent
connection generations, and JSON summary counters. No completed live pilot is claimed.

## Access and compatibility

Obtain two operator-independent feeds on the same cluster. Record operator names, plan quotes,
cluster/genesis identity, region, validator/plugin versions, subscription permissions, history
window and replay limits in a private pilot record. Confirm both expose the pinned protocol's
bank IDs, entries, slot updates, transactions, block metadata and block footer. A provider's
advertised generic Yellowstone support alone does not establish this compatibility.

Use identical subscription semantics. Different endpoint strings or provider labels are not
proof of operational independence. No paid subscriptions are purchased by this preparation.

## Owner Mini execution

Keep credentials outside the repository, directly on Ironman, in a private mode-600 file such as
`$HOME/.config/yellowstone-reconcile/pilot.env`. Export the variables in `.env.example` there,
including the two provider names. Do not paste values into chat, command arguments or evidence.
The Python runner does not load an env file automatically.

Sync code and verify offline from the MacBook:

```sh
mini-test -m 3 -- 'bash scripts/verify.sh'
```

On Ironman, after exporting those variables in its shell:

```sh
cd "$HOME/mini-work/yellowstone-reconcile"
export PATH="$HOME/.cargo/bin:$PATH"
python3 scripts/pilot.py --preflight
python3 scripts/pilot.py --run
```

Or use `clusterrun` pinned to `-m 3` with a private wrapper that sources the private file there.
No Rust compilation runs on the MacBook. Default target: 100 agreed finalized blocks within
300 seconds. A zero exit requires that target and a successful summary. Interrupt, idle timeout,
capacity exhaustion, malformed input and conflicts fail the run, and the report states why
(`failure`; a sanitized message, never an endpoint or token). `RECONCILE_MAX_SECONDS` is capped at
1,200: see the next section. Inspect `pilot-results/` on
the Mini; it is gitignored. Reports omit raw transport errors and endpoint/token values.
Review all evidence before copying any of it into the public repository.

## Settling the PublicNode question without the guard

`PROBE_OBSERVE_ONLY=1 cargo run --locked --release --features grpc --example access_probe` connects,
subscribes and, for 30 seconds, only counts what arrives: slot statuses with and without a bank id,
and entries and transactions at `bank_id == 0`. It applies no guard and reconstructs nothing, so its
`observed` result is not a pass. It needs a valid token in the environment, which the earlier
credential no longer is. Not run.

## Acceptance and evidence

1. Smoke: both providers connect; 100 blocks agree; record slot range, event/reconnect counts,
   duplicates, conflicts, elapsed time, sampled peak RSS and exact executable-input manifest.
2. Reconnect: deliberately disconnect one feed in a controlled environment; verify a new
   generation, unchanged provider identity, no extra quorum vote, and explicit replay-gap handling.
3. Outage: exceed the pending budget; require explicit capacity failure rather than silent loss.
   Recover by replay/backfill from a durable anchor, not by dropping pending slots silently.
4. Recovery: integrate real payload serialization into the durable sink, kill the process at
   transaction boundaries, replay, and compare complete stored identities and payload counts.
5. Soak: after replay/rotation is implemented, run 24 hours with both providers; publish sanitized
   memory/lag distributions, reconnects, data gaps, failures and unresolved limitations. Capture
   raw protobuf fixtures only with provider permission and a separate retention/redaction policy.

GUESS: 100 blocks/300 seconds and 24 hours are proposed test targets, not observed results or
guarantees. A successful smoke does not establish later acceptance criteria.

## Known implementation limits

MEASURED (`src/wire.rs`, `src/lib.rs`, `examples/two_providers.rs`): the wire guard stops at
8,192 distinct bank identities per connection; the core retains at most 4,096 slot records,
and this example does not advance its retention floor. Its target is capped at 1,000 blocks.
INFERRED (arithmetic on the 212 ms/slot measurement): 8,192 banks is reached after about
1,737 seconds (29 minutes) at one bank per slot, sooner if forks add banks. The connection then ends
with `BankCapacity` and this example does not rotate, so a 24-hour soak is impossible until safe
rotation exists, and a run longer than that would fail for that reason rather than a provider fault.
Hence the 1,200-second cap. Consequently this harness is a smoke test, not a continuous ingestion
or soak implementation.
Automatic safe rotation, replay/history negotiation and real-payload durable integration remain
future work. Repeated short smoke runs do not prove gap-free continuity.

## Integrator review packet

Send only after owner authorization: README, correctness contract, dependency review, fresh
verification log and pilot protocol. Ask for current multi-provider reconciliation behavior,
the missing requirement, willingness to reproduce the example, acceptable failure behavior,
and whether upstream placement would be preferable. Record named written feedback verbatim
with permission; unanswered outreach is not an adopter or endorsement.

## Dependency and review gate

MEASURED: `evidence/readiness-status-2026-10-25.json` snapshots upstream PR #22 as open and
unmerged. The experimental pin is not a supported release. Recheck merge/release status before
the pilot and before release; run the locked full suite for any pin change. Require an independent
review of commitment assumptions, replay gaps, watermark anchoring, transactional boundaries,
capacity/rotation behavior and credential handling. Funding for remediation does not purchase
a formal audit. No external review has been completed by this preparation.
