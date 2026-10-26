# Operations

Read the [correctness contract](CORRECTNESS.md) before adapting these prototype examples.

## Monitor agreement and capacity

MEASURED ([core](../src/lib.rs), [bridge](../src/upstream.rs)): Stats reports emitted,
duplicate, conflict, retained-slot and pending counts; UpstreamBridge also exposes buffered-block
count. A peer that stops contributing can stop agreement. Treat conflict, capacity and stale-session
errors as actionable; swallowing them can hide incomplete handling.

The host owns retention advancement, chain continuity, replay and backfill. Output arrives when
agreement is reached and is not globally slot-ordered. Advancing retention is not committing data.

## Store finalized blocks and a safe resume point

MEASURED ([sink](../src/sink.rs), [recovery tests](../tests/sink.rs)):
the optional SQLite sink accepts Finalized blocks and transactionally stores each block with a
watermark that follows parent slot/hash links back to a caller-supplied trusted anchor.
Replay compares the stored identity metadata and rejects mismatch. Opaque payload bytes are not
compared or cryptographically validated.

Use sink.resume_slot() to configure replay and the reconciler's first slot. Provider history must
reach that point. An anchor ahead of required history skips it; a broken link stalls the watermark.
The highest stored slot is not a safe checkpoint for unordered emissions.

    cargo run --locked --features sqlite --example sqlite_sink

MEASURED ([example](../examples/sqlite_sink.rs), [verification](VERIFICATION.md)): this offline
example exercises reversed arrival groups, restart and replay using synthetic byte payloads.
Process-abort regressions cover committed and uncommitted transaction states. Power loss, disk-full,
corruption repair and arbitrary external effects are not established by these tests.

An application receiving a conflict after emission must halt or apply its own reviewed recovery
policy. The library does not undo earlier database writes or external actions.

## Bound resources

MEASURED ([core](../src/lib.rs), [bridge](../src/upstream.rs), [guard](../src/wire.rs)):
count limits cover retained slots, pending blocks, providers and bank identities. They are not
a total memory ceiling. Size the pending count against your actual payloads and separately bound
transport messages, reconstruction buffers and retained bytes.

GuardedSource terminates at its caller-configured bank limit; examples use 8,192.
It must be replaced with its per-connection BlockStream when reconnecting.
Safe continuous rotation, history negotiation and gap handling remain planned work.

MEASURED ([smoke example](../examples/two_providers.rs), [harness](../scripts/pilot.py)):
the default target is 100 blocks, capped at 1,000, and the harness limits duration to 1,200 seconds.
It does not advance retention or persist output. Do not use it as a continuous ingestion service.
A number of minutes inferred from slot rate is not a measured connection lifetime.

## Credentials and provider configuration

Keep endpoint/token values outside Git and command arguments. Use the variable names in
[.env.example](../.env.example), distinct operator trust identities, the same cluster and matching
subscription semantics. Never publish raw transport errors that contain endpoint credentials.
The [pilot protocol](PILOT.md) describes compatibility checks and sanitized reporting.

## Build and release

    bash scripts/verify.sh

See [verification](VERIFICATION.md) for reproducibility and separate demo scope.
MEASURED ([manifest](../Cargo.toml)): package publication is disabled. This is a library prototype,
not a hosted ingestion service. A supported dependency decision, successful live validation,
sustained operation and independent review remain release gates in the [roadmap](ROADMAP.md).
