# Contributing

Agree on scope before implementing broad changes: open an issue first. Keep the reconciliation
contract separate from transport, reconstruction, chain ordering and durable sink policies.

Read `docs/CORRECTNESS.md`. Add positive and negative tests for any changed decision rule, and
write a failing test before the fix when you are correcting a defect. Do not claim stronger
delivery or verification guarantees than the tests and assumptions support.

Run `bash scripts/verify.sh` before opening a pull request (Rust 1.95; it checks formatting, tests in debug
and release, lints with warnings denied, builds the docs and runs the fixtures and examples).
Live provider testing is a separate opt-in pilot described in `docs/PILOT.md`. Keep provider
credentials out of Git.

The script prints its results; it does not write an evidence log. Capture a fresh run with Bash
pipefail as shown in README, and tie any claimed result to its source manifest. Do not overwrite
historical evidence to make it appear to describe a newer tree. The separate browser demo is
outside the main verification script and manifest; verify it using `demo/README.md` if you change it.

For documentation-only changes, check relative links and consistency with the current code.
Do not claim a fresh Rust pass if no Rust commands were run. Source code changes need the
appropriate regression and a verified build-machine run before they are described as fixed.

Use small, reviewed commits. Follow the contribution requirements of an upstream repository
before opening pull requests there.
