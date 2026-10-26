# yellowstone-reconcile

Reconcile sealed Solana blocks reported by independent Yellowstone providers.
**Prototype, unaudited; a successful live two-provider pilot is still outstanding.**

[Start with a fixture](#try-a-fixture) · [Integration guide](docs/INTEGRATION.md) ·
[Documentation](docs/README.md) · [Correctness contract](docs/CORRECTNESS.md) ·
[Verification evidence](docs/VERIFICATION.md)

## What it does

MEASURED ([core](src/lib.rs), [bridge](src/upstream.rs)): each connection reconstructs its own
blocks. The library compares slot, blockhash, parent slot/hash and entry/transaction counts.
By default, two distinct providers must report the same metadata at Finalized commitment before
one provider's unchanged payload is emitted. Disagreement quarantines the slot and reports a
conflict; a later majority does not repair it automatically.

INFERRED: this may help indexer and infrastructure teams that already consume multiple feeds.
Demand for this particular library remains unvalidated. Provider agreement is not ledger
validation, payload equality or proof that operators are independent.

## Where it fits

MEASURED ([architecture](docs/INTEGRATION.md), [existing tools](docs/ALTERNATIVES.md)):
Yellowstone reconstructs each connection independently; this library adds a configurable
agreement decision over sealed metadata. Reconnect and deduplication tools already exist.
INFERRED: use this policy when your consumer needs agreement between independent feeds;
it can wait indefinitely when agreement is unavailable.

```text
provider A -> reconstructor A --+
                              +-> sealed metadata agreement -> one payload -> your consumer
provider B -> reconstructor B --+             |
                                            +-> conflict / capacity / stale-session result
```

## Try a fixture

On a build machine with the pinned Rust 1.95 toolchain, Git and network access for dependencies:

```sh
git clone https://github.com/ss1738/yellowstone-reconcile.git
cd yellowstone-reconcile
cargo run --locked --bin yellowstone-replay -- fixtures/agreement.jsonl
```

MEASURED ([agreement fixture](fixtures/agreement.jsonl), [CLI regression](tests/replay.rs)):
this emits one block and classifies the reconnect replay as a duplicate. The command prints one
JSON line per event (`pending`, then `emitted`, then `duplicate`) and ends with a `stats` line
showing `"emitted":1` and `"duplicates":1`. No provider account, API token, database or running
Solana validator is required for this offline command.

Try disagreement separately; exit status **1** is the expected result:

```sh
cargo run --locked --bin yellowstone-replay -- fixtures/conflict.jsonl
```

The CLI input is sealed, qualified block observations in JSONL, not raw Geyser events. Fixture
hashes are hexadecimal. Do not feed optimistic fragments directly into the agreement core.

## Choose the integration layer

MEASURED (source files linked below):

| Use case | Interface / feature | Runnable example |
|---|---|---|
| Already have sealed block metadata and an opaque payload | `Reconciler<P>`; default features | [Replay CLI](src/bin/replay.rs) |
| Have one upstream reconstructor per connection | `UpstreamBridge`; `upstream` | [Offline bridge](examples/upstream_bridge.rs) |
| Connect raw Yellowstone gRPC streams | `GuardedSource` plus a separate upstream `BlockStream` per connection; `grpc` | [Two-provider smoke example](examples/two_providers.rs) |
| Store finalized byte payloads with a linked watermark | `SqliteSink`; `sqlite` | [SQLite recovery](examples/sqlite_sink.rs) |
| Replay fixtures in a browser build | Separate WebAssembly crate | [Demo source and build instructions](demo/README.md) |

```sh
cargo run --locked --features upstream --example upstream_bridge
cargo run --locked --features sqlite --example sqlite_sink
```

To use it from your own crate, depend on a pinned commit. It is not published to crates.io:

```toml
[dependencies]
yellowstone-reconcile = { git = "https://github.com/ss1738/yellowstone-reconcile", rev = "<commit>", features = ["upstream"] }
```

Drop `features` for the core alone, or add `sqlite` or `grpc` for the sink or guarded transport.
INFERRED (one check on 26 October 2026, no log kept): a throwaway crate with this dependency at
commit `0333b9e` (the same source as the earlier id `cfbf952`, before the history was rewritten), using the `upstream` feature, compiled and ran `Reconciler::new` and `connect`.
That check is not part of the verification suite.

Read the [integration guide](docs/INTEGRATION.md) before adapting either example. The offline
bridge uses a small illustrative event store; the SQLite example uses synthetic byte payloads.
[Real-payload regressions](tests/real_payload.rs) use upstream-simulated protobuf events, not a live feed.

## Limits to understand before use

MEASURED ([contract](docs/CORRECTNESS.md), [sink](src/sink.rs), [wire guard](src/wire.rs)):

- Provider IDs are operator-assigned trust identities. Reconnects create new generations, not votes.
- Output follows arrival/quorum, not slot order. The host owns chain continuity and backfill.
- Halt downstream handling on conflicts, including ones reported after an earlier emission.
  An emitted block or stored row is not automatically reversed.
- SQLite requires finalized blocks and a trusted anchor. Block writes and linked watermark updates
  share a transaction; tests do not establish power-loss safety or arbitrary external exactly-once effects.
- Counts are bounded, but opaque payload bytes and upstream/transport allocations need separate limits.
- `GuardedSource::new(source, max_banks)` takes the bank-identity limit from the caller and ends the
  connection when it is reached; the shipped examples pass 8,192. Safe continuous rotation is unbuilt.
  The transport example is a bounded smoke tool, not a continuous ingestion service.

MEASURED ([Cargo.toml](Cargo.toml), [dependency review](docs/DEPENDENCY-REVIEW.md)): upstream is pinned
at `0c732a049d6d18008a541f47b09ddb7b949cbb65`, the experimental PR #22 revision. At the
26 October check it remained open and unmerged. The pin lacks the upstream issue #19 malformed-input
fix; our boundary checks address identified paths, not all possible dependency defects.

MEASURED ([PublicNode result](docs/PUBLICNODE-RESULT.md)): one real feed authenticated and streamed
updates but produced no reconstructed blocks before a missing-bank-ID rejection. Its cause remains
unresolved. No successful two-provider pilot, production adopter or independent audit is claimed.

## Verify the repository

On a build machine:

```sh
bash scripts/verify.sh
```

MEASURED ([script](scripts/verify.sh)): this checks formatting, default/all-feature debug/release
tests, lint, docs, fixture exit codes and offline examples. It prints to the terminal; it does
**not** create or refresh an evidence file automatically. To capture a new run while preserving
its failure status, use Bash with pipefail and a new log path:

```sh
bash -o pipefail -c 'bash scripts/verify.sh 2>&1 | tee /tmp/yellowstone-verify.log'
```

Compare source identity separately:

```sh
python3 scripts/source_manifest.py | diff -u evidence/source-manifest.sha256 -
```

A changed manifest is not automatically a regression: it means the archived evidence no longer
identifies the current executable inputs and must be re-established by a new verified run.
MEASURED ([verification record](docs/VERIFICATION.md)): the main manifest excludes the separate
browser demo. Its commands and evidence must be evaluated separately; a main-suite pass is not
proof that the WebAssembly build or deployed site matches that source.

MEASURED ([verification record](docs/VERIFICATION.md)): verification is manual. Neither a configured YAML
workflow nor old CI runs establish coverage of a newer tree. The recorded runner/CI status is dated;
no current automatic CI pass is implied here.

## Project information

- [Contributing](CONTRIBUTING.md) and [security reporting](SECURITY.md).
- [Operations](docs/OPERATIONS.md), [pilot protocol](docs/PILOT.md) and [self-audit](AUDIT.md).
- [Existing tools and boundaries](docs/ALTERNATIVES.md), [roadmap](docs/ROADMAP.md)
  and [changelog](CHANGELOG.md).

MEASURED ([LICENSE](LICENSE)): Apache-2.0. Publishing to a package registry is disabled in
[Cargo.toml](Cargo.toml). INFERRED: treat the current repository as an inspectable prototype,
not a supported production release.
