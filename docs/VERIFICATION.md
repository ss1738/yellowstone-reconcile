# Verification status

MEASURED: the complete run on Ironman (Rust/Cargo 1.95.0) on 25 October 2026 passed
25 tests in debug and the same 25 in release, formatting, all-feature/example compilation,
Clippy with warnings denied, and documentation with warnings denied.
Evidence: `evidence/verification.log` (run `20261025-003729`).

MEASURED: `evidence/locked-verification.log` records a follow-up passing all-feature test
run using `--locked`. `Cargo.lock` was generated on Ironman and retrieved unchanged.
The 18 executable-input hashes in `evidence/source-manifest.sha256` matched the local
source tree after retrieval. Reproduce the comparison with `python3 scripts/source_manifest.py`.

MEASURED: the randomized-order test covers 32 seeds, each with 500 slots, 3 providers
and 2 observations per provider/slot (96,000 observations). The retention regression
processes 100,000 slots with a 16-slot retained-state cap. Sources: `tests/reconcile.rs`
and the passing test output in `evidence/verification.log`.

MEASURED: both synthetic wire-reconstruction scenarios passed through the pinned upstream
`BlockStream`: agreeing providers with different local bank ids emitted one block, while a
provider missing a footer did not contribute a quorum vote. Source: `tests/reconstruction.rs`.

The script checks formatting, debug and release tests, all-feature compilation, Clippy,
documentation, successful agreement replay, expected conflict failure and a synthetic benchmark.

MEASURED: agreement replay emitted one block and classified replay after reconnection as a
duplicate. Conflicting replay exited nonzero before emission. The upstream bridge example
selected one provider's unchanged payload. Evidence: `evidence/verification.log`.

MEASURED: the synthetic release benchmark processed 200,000 metadata blocks with unit payload
in 0.037012 seconds in the recorded run (approximately 5.40 million per second). This microbenchmark
excludes gRPC, reconstruction, real payloads, persistence and end-to-end latency; it is not a claim
of network ingestion capacity. Source: `examples/benchmark.rs`, `evidence/verification.log`.

MEASURED: the earlier gRPC verification run passed its tests but failed Clippy on a large message
enum. The subsequent source boxes that message payload and the complete run passed.
The failed run remains available in `evidence/verification-grpc-repair.log`.

Limits:

- Synthetic metadata benchmark timings are not end-to-end ingestion throughput.
- The live two-provider example requires credentials and is opt-in; no live pilot is claimed.
- Finite randomized schedules are not exhaustive verification or a security audit.
- Upstream code is a dependency; this companion's tests do not re-audit its reconstruction rules.
- Passing tests does not establish adoption, ecosystem impact, upstream acceptance or funding.

The source manifest excludes prose and evidence files so documenting results does not change
the identity of the code and fixtures that were tested.
