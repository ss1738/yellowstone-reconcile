# Integration path

This guide maps existing interfaces and examples to a host application's responsibilities.
It is not a live-provider validation claim. Read `CORRECTNESS.md` for the actual contract.

## 1. Decide what your input represents

MEASURED (`src/lib.rs`, `src/upstream.rs`): the core consumes `SealedBlock<P>` with complete
reported metadata and a qualified commitment. If input is raw Yellowstone updates, use one
independent upstream reconstructor per connection and feed its outputs through `UpstreamBridge`.
Never combine raw events across providers before reconstruction.

The shortest complete caller of the core is `src/bin/replay.rs`, about 100 lines. A runnable
bridge reference needs no credentials:

```sh
cargo run --locked --features upstream --example upstream_bridge
```

INFERRED (`src/upstream.rs` `ingest`; upstream `state_machine.rs` `deliver_or_queue` at the
pinned revision): `UpstreamBridge` needs each connection's outputs in the order its `BlockStream`
yields them. That stream emits a block before any commitment update for it. A commitment update for
a bank the bridge has not seen is dropped, not replayed. If events arrive out of order, or from
anything other than one `BlockStream` per connection, a block can stay in the bridge's pending
buffer until a reconnect, a bank discard or `advance` removes it; it is never emitted in the meantime.

MEASURED (`examples/upstream_bridge.rs`): both mock connections use bank ID 7, yet count as two
providers because connection identity is separate. The example uses an illustrative `Events`
store; it is not a live protobuf/transport integration. `tests/reconstruction.rs` and
`tests/real_payload.rs` exercise upstream-simulated wire events separately.

## 2. Assign identity and preserve the configuration boundary

Assign one stable `ProviderId` per independent operator/trust domain. Two endpoints from the same
operator share an ID. A URL difference alone does not establish independence.

Call `connect` on each connection/reconnection, keep the returned `Session`, and discard the old
upstream reconstructor. Pass only that connection's events with its session. Do not reuse the same
reconciler across clusters, subscription filters or schema changes.

MEASURED (`src/lib.rs`, `tests/reconcile.rs`): older connection generations are rejected; a reconnect
does not give the same provider another vote. Finalized and two providers are the defaults.

## 3. Handle every relevant outcome

| Outcome | Host action |
|---|---|
| `Pending` | Wait for another qualified provider; monitor capacity and lag |
| `Emitted` | Process one selected payload; do not assume global slot order |
| `Duplicate` | Avoid repeating downstream effects |
| `Conflict` | Stop affected downstream processing, preserve evidence and investigate; check `previously_emitted` |
| `Quarantined` | Keep the slot halted; do not silently select another payload |
| Error or prune report | Classify the failure or gap and arrange replay/backfill; do not silently drop it |

MEASURED (`src/lib.rs`, `src/upstream.rs`): the library reports conflicts and prune gaps, but does
not undo earlier host effects, automatically establish continuity, or supply an operational
backfill service. A durable restart-visible conflict record is not currently implemented.

## 4. Connect a sink without inventing a checkpoint

```sh
cargo run --locked --release --features sqlite --example sqlite_sink
```

MEASURED (`src/sink.rs`, `tests/sink.rs`): the sink accepts only finalized byte payloads and commits
a block and parent-linked watermark atomically. It rejects conflicting stored identity metadata.
The caller must establish a trusted anchor whose earlier history is complete or deliberately
excluded. A wrong anchor ahead of required history can skip that history on restart.

Use the sink's `resume_slot()` with the reconciler's `Config::first_slot`. Ask the provider for
history/replay that reaches that point and handle skipped/absent history explicitly. Do not use
the maximum stored slot: output may be unordered and lower blocks may still be missing.

MEASURED (`src/wire.rs`, `tests/real_payload.rs`): `encode_events`/`decode_events` provide an encoding
for reconstructed protobuf events that the byte sink can store. The end-to-end regression uses
simulation; real-feed persistence is not established. Opaque payload integrity is a separate concern.

## 5. Add transport only after the offline path is understood

Read `PILOT.md` and the repository-root `.env.example` before the opt-in transport example. Credentials belong outside
Git. Use identical subscriptions on independently operated providers on the same cluster.

MEASURED (`examples/two_providers.rs`): the example prints metadata, limits duration and block count,
and stops on conflicts/capacity failures. It is not a durable indexer. The wire guard's finite bank
map and the core's retention window make indefinite operation unsafe without a reviewed rotation,
replay and backfill policy. Do not bypass these limits to make a demo appear successful.

## Evidence and review

Use `docs/VERIFICATION.md` from the repository root to locate dated logs and source manifests.
Run `bash scripts/verify.sh` on an appropriate build machine for the main suite; evaluate the
separate demo as documented in `demo/README.md`. Do not call successful authentication a
successful reconstruction, and do not call offline agreement a successful two-provider pilot.
