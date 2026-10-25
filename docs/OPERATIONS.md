# Operations and local development

## Build policy for this owner

Use `mini-test -m 3 -- 'bash scripts/verify.sh'` from the repository. Mini 3 maps to
the owner's Ironman host in the installed tooling. Use `-m 2` for Apsarth if reachable.
Do not use automatic node selection: the tooling's Mini 1 is not an authorized host
for this owner's project data. Rust builds and tests must not run on the MacBook.

The toolchain is pinned in `rust-toolchain.toml`. `Cargo.lock` records resolved package
versions once generated on the Mini. Upstream is pinned by Git revision as well.

## Limits and monitoring

Track core `Stats` (emitted, duplicate, conflict, retained slot and pending payload counts)
and `UpstreamBridge::pending_blocks()`. Treat conflicts and capacity errors as actionable.
Do not swallow them and continue committing output. A stuck peer will stop quorum;
this is an explicit consistency/availability tradeoff, not automatic failover.

With three providers and quorum two, two agreeing healthy providers can make progress
unless a conflicting qualified observation quarantines the slot. Quarantine is deliberately
conservative; the application needs an operator policy for recovery.

The transport example retries each endpoint three times and requests complete transaction
events plus upstream reconstruction metadata/sysvars/footer. Both endpoints must support
the pinned proto and these filters. The demonstration exits after 100 accepted blocks by
default, or after `RECONCILE_MAX_BLOCKS` in 1..=1000. It does not advance retention or save
an output checkpoint and should not be used as a long-running ingestion service.

## Credentials

Export `YELLOWSTONE_A_URL`, `YELLOWSTONE_B_URL`, and optional separate `_TOKEN` variables.
Use HTTPS and distinct provider trust domains. Do not put tokens in fixtures, config
checked into Git, logs or shell arguments. `.env` and common key-file extensions are ignored.
The example suppresses raw connection errors because they can include endpoint details.

## Deployment and publication

The current work is local. Do not create a GitHub remote, publish a crate, deploy a service,
send outreach, or submit the grant until the owner explicitly requests those actions.
`publish = false` guards accidental crate publication. The optional GitHub workflow is
manual and requires an owner-provisioned Mini runner; no runner is assumed to exist.

## Release checks

1. Run the complete verification script on a Mini and save its raw output in `evidence/`.
2. Review the source against the correctness contract, including negative and positive cases.
3. Review dependencies and licenses; recheck upstream branches and PR overlap.
4. Obtain an independent code review and run credentialed provider pilots before production.
5. Populate application identity/contact fields and replace draft adoption hypotheses with evidence.
6. Request permission to publish the repository and separately to submit the application.

## Sizing memory against a lagging provider

`Config` bounds pending work by **count**, not by bytes: `max_pending_blocks` (default 128) and
`max_retained_slots` (default 4096). Nothing can be emitted for a slot until the required number
of providers agree, so while one provider runs ahead its payloads are all retained. The ceiling an
operator actually cares about is therefore `max_pending_blocks` multiplied by their own payload
size.

MEASURED (`cargo run --release --example lag_memory`, 2026-10-25, owner Mini):

| scenario | result |
|---|---|
| 64-slot lag, 2 MiB payloads | `pending_blocks=64`, 134,217,728 bytes retained (128.0 MiB) |
| slow provider catches up | drains to `pending_blocks=0`, `emitted=64` |
| one provider only, past the bound | accepted exactly 128 blocks, then rejected with `PendingCapacity` |

So at 2 MiB blocks the default configuration implies roughly a **256 MiB** payload ceiling, and it
**fails closed** rather than growing without limit. **2 MiB is optimistic**: real mainnet blocks
measured at a median of 4.26 MiB, so see the variance section below for the corrected ceiling. Set `max_pending_blocks` from your own measured
payload size and the memory you are willing to commit, not from the default.

LIMIT: synthetic payloads of uniform size. Retained bytes are computed as `pending_blocks` times
payload size rather than from allocator accounting, and the figures exclude gRPC, reconstruction
and persistence. Reproduce with your own payload size and lag:
`cargo run --release --example lag_memory <payload_bytes> <lag_slots>`.

## Stall and retention budgets in seconds

The same count bounds, divided by the chain's slot rate, give the budgets an operator actually
plans against.

MEASURED slot rate: **212 ms/slot** on mainnet-beta, 2026-10-25, sampled over 1,000 and 5,000
slot windows (212 and 213 ms/slot). Alpenglow activated on 21 October 2026; this is well below
the pre-Alpenglow figure, so any older sizing guidance understates how fast the bounds are
reached. Slot time is not a constant, so pass your own measurement.

MEASURED (`cargo run --release --example stall_budget`, owner Mini, default `Config`):

| budget | count | at 212 ms/slot |
|---|---|---|
| One provider silent, before `PendingCapacity` | 128 blocks | **27.1 seconds** |
| Retention window before a slot is refused as stale | 4,096 slots | 868 seconds (~14.5 min) |

So on the defaults a single stalled provider is tolerated for **under half a minute**. Recovery
was also measured: after the bound rejects, the first agreement from the returning provider emits
immediately and the accumulated quorum is not lost (`emitted_on_first_agreement=true`,
pending 128 to 127).

Two consequences for operators:

- Alert on `pending_blocks` rising, not on `PendingCapacity`. By the time the error fires you are
  roughly 27 seconds into an outage on default settings.
- Raise `max_pending_blocks` only against measured payload size. The byte ceiling binds first on
  large blocks: see the lag-memory section above, where 128 blocks at 2 MiB is already 256 MiB.

LIMIT: assumes one block per slot per provider and no payload pressure, excludes gRPC,
reconstruction and persistence, and the slot rate is an input rather than a guarantee.

## Restarting from a checkpoint

Providers replay from the start of their own retention on reconnect, not from wherever you left
off. So the property that matters across a process restart is that nothing already committed is
emitted twice, because a second emission is a duplicate downstream write.

The contract is:

1. Persist a checkpoint that is **contiguous**: every slot at or below it is durably committed. Output is unordered across slots, so the highest committed slot is not safe, see the SQLite sink below.
2. On restart, construct the reconciler with `first_slot = checkpoint + 1`.
3. Treat `Error::StaleSlot` on replayed slots as expected and not as a fault.
4. Keep calling `advance` forward only. `Error::CheckpointRegression` means the checkpoint you
   read is older than the retention already in effect; do not retry it lower.

MEASURED (`cargo test --test restart`, owner Mini, 2026-10-25, 3 tests passing):

| property | result |
|---|---|
| Emit slots 1 to 10, persist checkpoint 10, drop the reconciler, rebuild at `first_slot = 11`, replay slots 1 to 10 | every replayed slot refused with `StaleSlot`; `emitted = 0`; `pending_blocks = 0` |
| Slots after the checkpoint | emit normally (11, 12, 13) |
| `advance` backwards from 15 to 14 | `CheckpointRegression`, and `first_slot` stays 15 |
| `advance` re-applied at the same slot | idempotent |
| Checkpoint ahead of the stream (`first_slot = 100`, slot 50 arrives) | refused, no state allocated |

Note that refused replays allocate no slot state, so a provider replaying a long history after
restart does not consume the pending budget described above.

These tests passed on first run against the existing implementation. They prove the behaviour
rather than correct it; the gap was that the restart path was untested, not that it was wrong.

LIMIT: in-memory only. Durability of the checkpoint itself, and exactly-once delivery into the
sink, remain the integrating application's responsibility and are not exercised here.

## Ordering the sink write against the checkpoint

Refusing replayed slots is not by itself exactly-once. The checkpoint has to be persisted
**after** the sink commits. Persist it first and a crash in between loses the slot; persist it
after and a crash in between re-delivers one slot, which the sink must absorb idempotently.

MEASURED (`cargo run --release --example durable_sink`, owner Mini, 2026-10-25). Twenty slots,
two providers, a crash after slot 12, a checkpoint file on disk, then a restart that replays the
whole stream from slot 1:

| stage | result |
|---|---|
| Lifetime 1, crash after 12 | 12 slots committed, checkpoint on disk 12 |
| Lifetime 2, resumed at 13 | 24 replayed observations refused (12 slots across 2 providers), checkpoint reached 20 |
| Sink totals | 20 rows, 20 writes, **no slot written twice, no slot lost** |

So the ordering to implement is:

1. Receive `Outcome::Emitted`.
2. Commit to the sink, idempotently keyed on slot.
3. Only then persist the slot as the checkpoint.

LIMIT: the sink is in-process and the checkpoint is a plain file, so this demonstrates the
ordering contract rather than real durability. A crash between step 2 and step 3 still
re-delivers that one slot, which is exactly why step 2 must be idempotent. No gRPC,
reconstruction or real provider is involved.

## Correcting the ceiling for real block-size variance

The section above uses a round 2 MiB payload. Real blocks are larger and very uneven.

MEASURED, ten finalized mainnet blocks (slots 460985218 to 460985317) sampled 2026-10-25 via
`getBlock` with `encoding=base64`, `transactionDetails=full`, `rewards=false` and
`maxSupportedTransactionVersion=1`:

| | |
|---|---|
| smallest | 2,389 bytes (1 transaction) |
| median | 4.26 MiB |
| mean | 4.48 MiB |
| largest | 7.90 MiB (1,135 transactions) |
| spread | about 3,500x |

Note that `maxSupportedTransactionVersion: 0` rejects every one of these blocks, because mainnet
now carries version-1 transactions. Tooling pinned to 0 sees nothing at all.

MEASURED (`cargo run --release --example payload_variance`), replaying that distribution with one
provider silent so everything is retained:

| ceiling at `max_pending_blocks = 128` | bytes held |
|---|---|
| uniform 2 MiB, as first documented | 256 MiB |
| **replaying the measured sizes** | **570.8 MiB** |
| if every block were the largest sampled | **1,011 MiB** |

So the default configuration can hold on the order of **half a gigabyte** of payloads, and close
to a gigabyte in an unlucky run. The earlier 256 MiB figure understates it by roughly 2.2x. Size
`max_pending_blocks` against your own measured payloads.

LIMIT: ten blocks from one minute of one day is not a size distribution. These are JSON-encoded
RPC responses rather than the wire payloads a Geyser consumer holds, so treat them as an order of
magnitude and measure your own provider.

## Correction: the highest committed slot is not a safe checkpoint

The two sections above treat "the highest slot committed" as the checkpoint. That holds only when
slots are emitted in order, which `Reconciler` does not promise: a slot emits when its second
provider arrives, so a lagging provider delivering out of order produces out-of-order emission. If
slot 8 is committed before slot 5 and the process crashes, resuming at 9 loses slots 5 and 6.
`restart` and `durable_sink` did not show this because they emit in order.

## SQLite sink with an atomic resume point

`sink::SqliteSink` (feature `sqlite`) stores the block and advances a watermark in one transaction.
The watermark moves only along `parent_slot` and `parent_blockhash` links back to a trusted anchor.
Linked ancestors after that anchor are stored; skipped slots need no row. Restart with `Config::first_slot =
sink.resume_slot()`. Blocks above the watermark may be redelivered and are absorbed as
`Commit::AlreadyPresent`. A different block for a stored slot returns `SinkError::Mismatch` and is
not overwritten. Replay compares blockhash, parent slot/hash and both counts; opaque payloads are
not compared byte-for-byte. A broken link stalls advancement and costs replay. An anchor ahead
of required history is unsafe: `open` treats that history as already done and `resume_slot` skips
it. The caller must independently establish the anchor or deliberately exclude earlier history.

MEASURED (`cargo run --release --features sqlite --example sqlite_sink`, owner Mini, 2026-10-25).
Twenty slots, one provider leading and the other delivering in reversed groups of four, a crash
after six emissions (slots 4, 3, 2, 1, 8, 7), then a restart that replays the whole stream:

| | result |
|---|---|
| Watermark after the crash | 4, so resume at 5 |
| A highest-committed-slot checkpoint | would resume at 9 and lose slots 5 and 6 |
| After restart | 14 inserted, 2 redelivered and absorbed (7, 8), 8 replays refused |
| Sink totals | 20 rows, watermark 20, every slot stored once |

MEASURED (`evidence/verification-current.log`, `tests/sink.rs`): the suite covers a child process
killed with SIGABRT after committing, full reconciliation with unordered emission, and replay
metadata drift. `evidence/sink-metadata-regression-before-fix.log` preserves the new metadata
regression failing before its correction.

LIMIT: one local SQLite file and synthetic blocks, so no gRPC and no real provider. Process death
is exercised, power loss is not. A networked database such as Postgres is not implemented, and no
pilot integrator has agreed this contract. The watermark assumes one canonical child per
`parent_slot`; with `Confirmed` blocks only the parent hash match separates forks.
