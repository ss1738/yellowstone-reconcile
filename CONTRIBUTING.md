# Contributing

Agree on scope before implementing broad changes: open an issue first. Keep the reconciliation
contract separate from transport, reconstruction, chain ordering and durable sink policies.

Read `docs/CORRECTNESS.md`. Add positive and negative tests for any changed decision rule, and
write a failing test before the fix when you are correcting a defect. Do not claim stronger
delivery or verification guarantees than the tests and assumptions support.

Run `bash scripts/verify.sh` before opening a pull request (Rust 1.95; it formats, tests in debug
and release, lints with warnings denied, builds the docs and runs the fixtures and examples).
Live provider testing is a separate opt-in pilot described in `docs/PILOT.md`. Keep provider
credentials out of Git.

Use small, reviewed commits. Follow the contribution requirements of an upstream repository
before opening pull requests there.
