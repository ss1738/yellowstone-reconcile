# Changelog

## Unreleased prototype

MEASURED ([core](src/lib.rs), [bridge](src/upstream.rs), [guard](src/wire.rs), [sink](src/sink.rs)):

- Sealed metadata agreement with stable provider identity and reconnect generations.
- Conflict quarantine, explicit retention and count-based capacity errors.
- Optional per-connection reconstruction bridge and guarded gRPC input.
- Replay CLI, fixtures and a separate browser demo.
- Finalized-only SQLite persistence with a linked transactional resume watermark.
- Replay metadata checks and schema-version refusal before journal-mode mutation.

See [fixed regressions](AUDIT.md), [verification](docs/VERIFICATION.md) and
[remaining work](docs/ROADMAP.md). No supported production package release is claimed.
