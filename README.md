# yellowstone-reconcile

**Grant proposal (PDF, current):** [docs/yellowstone-reconcile-proposal.pdf](docs/yellowstone-reconcile-proposal.pdf).
Status: working prototype, unaudited, not yet run against two live providers. See [AUDIT.md](AUDIT.md) and
[SUBMISSION-READINESS.md](SUBMISSION-READINESS.md).

A conservative Rust companion for reconciling complete Solana blocks from independent
Yellowstone connections. Local development release; publishing is disabled.

MEASURED (source: `src/lib.rs`, `src/upstream.rs`): the implementation compares block
hashes, parents and counts after per-connection reconstruction, requires a configurable
number of distinct providers, and quarantines disagreements. The default requires two
providers at Finalized commitment. It never combines payload fragments from different
providers.

## Why build this?

MEASURED: the Foundation's [Alpenglow integration guidance](https://solana.com/upgrades/alpenglow)
identifies cross-connection reconciliation as a consumer responsibility. The existing
[Yellowstone Block Machine](https://github.com/rpcpool/yellowstone-block-machine) supplies
the per-connection reconstruction layer. See [research evidence](docs/RESEARCH.md).

INFERRED: a reusable companion and an adversarial fixture corpus could reduce duplicate
integration effort. Demand, upstream acceptance and grant support remain unverified.

## Local demonstration

Run Rust commands on a Mac Mini in this owner's environment. From this repository:

```sh
mini-test -m 3 -- 'bash scripts/verify.sh'
```

On an appropriately provisioned build machine:

```sh
cargo run --bin yellowstone-replay -- fixtures/agreement.jsonl
cargo run --features upstream --example upstream_bridge
```

The agreement fixture uses the same local bank id on two providers and reconnects one.
The conflicting fixture should exit with status 1:

```sh
cargo run --bin yellowstone-replay -- fixtures/conflict.jsonl
```

The replay CLI accepts a JSONL filename and optional minimum-provider count. Its input
must contain sealed, qualified block observations, not raw Geyser fragments. Hashes in
these human-readable fixtures use hexadecimal; upstream wire hashes are binary/base58.

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
`--run` requires provider access on an owner Mini. No live pilot is claimed. The harness does
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

Run this command on the owner Mini. The complete local verification command above covers it.

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
is `scripts/verify.sh`, run by hand on an owner Mini (`evidence/verification-final.log`). Neither
workflow has run: `ci.yml` is a GitHub-hosted check that starts only by hand, and `verify.yml`
needs a self-hosted runner that is not registered (0 runners). The runner configuration is
written up for owner review and nothing enables it.

INFERRED: this is an application prototype, not an audited production release. No adopter,
maintainer endorsement, funding award or completed live provider pilot is claimed.

Apache-2.0. See `LICENSE` and `NOTICE`.
