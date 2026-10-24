# yellowstone-reconcile

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

The upstream dependency is pinned to `b828807d45522505cfdd6b4710d2287b29e5d99a`.
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

INFERRED: this is an application prototype, not an audited production release. No adopter,
maintainer endorsement, funding award or completed live provider pilot is claimed.

Apache-2.0. See `LICENSE` and `NOTICE`.
