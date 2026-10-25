# yellowstone-reconcile

Reconcile complete Solana blocks across independent Yellowstone gRPC connections. Rust, Apache-2.0.
**Status: prototype. Unaudited, not yet run against two live providers.**
[Grant proposal (PDF)](docs/yellowstone-reconcile-proposal.pdf) |
[Correctness contract](docs/CORRECTNESS.md) | [Verification](docs/VERIFICATION.md) |
[Self-audit](AUDIT.md) | [Research record](docs/RESEARCH-2026-10-25.md)

## What it does

Solana's [Alpenglow integration guidance](https://solana.com/upgrades/alpenglow) says a bank id means
nothing outside the connection it arrived on, and that reconciling across connections is the
consumer's job. This library takes one upstream
[block-machine](https://github.com/rpcpool/yellowstone-block-machine) stream per connection,
compares the sealed blocks on blockhash, parent and counts instead of bank id, and emits a block
only when a configurable number of distinct providers (default two, at Finalized) agree. A
disagreement is quarantined and reported, not resolved by majority. A SQLite sink stores each
block and its resume point in one transaction, so a crash neither loses nor double-stores a slot.
It is off-chain tooling; it does not verify ledger validity, only that providers agree.

## Status at a glance

| Tested offline (synthetic or simulated input) | Not done |
|---|---|
| Agreement, conflict, reconnect and retention rules (`tests/`) | A run against two live providers |
| Bridge from upstream streams; raw-input guard (`src/wire.rs`) | A consumer who has asked for it |
| SQLite sink: finalized-only, crash and unordered recovery | Connection rotation (the guard ends a stream after 8,192 banks) |
| Real reconstructed events stored and read back (`tests/real_payload.rs`) | Independent code review |
| Replay CLI and fixtures | A supported upstream release (the pin is an open pull request) |

## Quick start

Needs Rust 1.95 (pinned by `rust-toolchain.toml`) and network access once, for the pinned git
dependency. A full build uses about 3 GB.

```sh
bash scripts/verify.sh        # the complete locked suite; its output is evidence/verification-final.log
cargo run --bin yellowstone-replay -- fixtures/agreement.jsonl
cargo run --bin yellowstone-replay -- fixtures/conflict.jsonl   # exits 1: provider conflict
cargo run --features upstream --example upstream_bridge
```

The agreement fixture uses the same local bank id on two providers and reconnects one. The replay
CLI reads sealed, qualified block observations as JSON lines, not raw Geyser fragments; hashes in
the fixtures are hexadecimal, whereas upstream wire hashes are binary or base58.

## Integration

Use one upstream `BlockStream` per connection. Feed its outputs into `UpstreamBridge`:

1. Assign a stable `ProviderId` per independent provider, not per endpoint or reconnect.
2. Call `connect` for each connection/reconnection; discard the old reconstructor.
3. Pass each stream's `BlockMachineOutput` with its returned `Session` to `ingest`.
4. Process `Emitted` outputs. Halt the affected downstream processing on `Conflict`.
5. Advance retention explicitly and handle pending/quarantined slots in the prune report.

The upstream dependency is currently pinned to `0c732a049d6d18008a541f47b09ddb7b949cbb65`,
the head of upstream [PR #22](https://github.com/rpcpool/yellowstone-block-machine/pull/22),
which is **open and unmerged**. Upstream `main` is
`b828807d45522505cfdd6b4710d2287b29e5d99a`; on that baseline the repaired-dead-slot
lineage described in [docs/DEPENDENCY-REVIEW.md](docs/DEPENDENCY-REVIEW.md) is not handled,
because the relevant prune happens inside upstream `BlockStream` and cannot be corrected
downstream. Anyone depending on this crate should decide deliberately between the merged
baseline and the unmerged PR.
`upstream` enables the typed bridge; `grpc` additionally enables the opt-in transport example.

### Two-provider transport demonstration

Configure distinct HTTPS providers on the same cluster. Export the environment variables
listed in `.env.example`, keeping tokens out of command-line arguments and source files.
The example reads the environment; it does not load `.env` automatically.

```sh
cargo run --release --features grpc --example two_providers
```

MEASURED (source: `examples/two_providers.rs`): the example subscribes independently,
recreates connection state on retry, uses a bounded event channel, and stops on conflicts
or configured block count. It prints accepted metadata; it is not a persistent indexer.
Live provider operation is not established by compilation or offline fixture tests.

For the bounded smoke harness and evidence capture, see [pilot protocol](docs/PILOT.md).
`python3 scripts/pilot.py --preflight` checks configuration without connecting or compiling;
`--run` requires provider access. No live pilot is claimed. The harness does
not implement continuous rotation/replay or a live durable payload sink.

### Storing real block payloads

`wire::encode_events` and `wire::decode_events` turn a reconstructed block's protobuf events
(`Block<BankBuffer>.events`) into the opaque bytes the sink stores and back. The encoding carries an
event count, so a payload cut at any point, or padded, is an error and never a shorter block.
MEASURED (`tests/real_payload.rs`): events from the upstream simulation survive two providers, the
reconciler, SQLite and a reopen unchanged. The wire is simulated; no live feed has been stored.

### SQLite recovery example

MEASURED (`src/sink.rs`, `tests/sink.rs`): the optional `sqlite` feature atomically stores byte
payloads and follows parent links when advancing a durable watermark. It handles synthetic
unordered delivery and restart; the highest stored slot alone is not a safe checkpoint.
Only `Finalized` blocks are stored. See operations for anchor assumptions and what it does not do. Actual upstream payload serialization is still future work.

```sh
cargo run --locked --release --features sqlite --example sqlite_sink
```

`scripts/verify.sh` runs it.

## Correctness boundaries

- Provider agreement is not cryptographic validation or proof that providers are independent.
- Payload completeness depends on upstream reconstruction and matching subscription semantics.
- Accepted payload is from the first qualifying observation; it is not byte-verified against peers.
- Output order follows arrival/quorum, not slot order. Chain continuity and skipped-slot recovery
  belong to the host application.
- Deduplication is in memory within the retained window. Durable exactly-once delivery requires
  transactional coordination with the downstream sink; this library does not promise it.
- Counts of retained blocks/providers/events are bounded. Opaque payload byte sizes and upstream
  transport buffers require their own limits.
- A disagreement after an earlier emission is reported, not silently undone.

See [correctness contract](docs/CORRECTNESS.md), [operations](docs/OPERATIONS.md),
[verification](docs/VERIFICATION.md), and [grant draft](grant/APPLICATION.md).

## Project status

MEASURED: this repository includes the reconciliation core, upstream bridge, replay CLI,
fixtures, transport example, tests, benchmark and grant drafts (see their source paths).
Test execution evidence and limitations are recorded separately in `docs/VERIFICATION.md`.

MEASURED (`docs/RUNNER-SETUP.md`, `gh api`, 25 October 2026): there is **no automatic CI**. Verification
is `scripts/verify.sh`, run by hand on the maintainer's build machine (`evidence/verification-final.log`). Neither
workflow has run: `ci.yml` is a GitHub-hosted check that starts only by hand, and `verify.yml`
needs a self-hosted runner that is not registered (0 runners). The runner configuration is
written up for owner review and nothing enables it.

INFERRED: this is an application prototype, not an audited production release. No adopter,
maintainer endorsement, funding award or completed live provider pilot is claimed.

Apache-2.0. See `LICENSE` and `NOTICE`.
