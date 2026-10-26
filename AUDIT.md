# Self-review and fixed regressions

This is a maintainer self-review, not an independent audit. The project remains an unaudited prototype.

## Fixed cases

MEASURED (sources and regression logs linked below):

| Defect | Correction | Regression evidence |
|---|---|---|
| SQLite replay accepted matching hashes with different identity metadata | Compare parent slot/hash and entry/transaction counts as well as blockhash | [Failing reproduction](evidence/sink-metadata-regression-before-fix.log); [sink tests](tests/sink.rs) |
| SQLite accepted forkable blocks for its linked resume point | Reject blocks below Finalized commitment | [Before](evidence/audit-regressions-before-fix.log), [after](evidence/audit-regressions-after-fix.log); [sink](src/sink.rs) |
| Refusing a newer SQLite schema still changed its journal mode | Read schema version before switching to WAL | [Before](evidence/pass2-regressions-before-fix.log); [regression](tests/pass2_regressions.rs); [current run](evidence/verification-public-cleanup-20261026.log) |
| Smoke harness could be configured beyond its connection bank limit | Cap the smoke duration and document its bounded scope | [Pilot protocol](docs/PILOT.md); [harness](scripts/pilot.py) |
| Failed pilot output omitted a useful failure reason | Include a sanitized reason and reject malformed/partial summaries | [Harness tests](scripts/test_pilot.py); [current run](evidence/verification-public-cleanup-20261026.log) |
| Missing-bank-ID diagnostics omitted the slot status | Include status in the typed error and count status/identity combinations in the probe | [Wire guard](src/wire.rs), [probe](examples/access_probe.rs), [wire tests](tests/wire.rs) |

MEASURED ([pass-2 tests](tests/pass2_regressions.rs)): capacity-retry and commitment-before-block
tests characterize existing bridge behavior. They were not failing defect reproductions.

## Remaining limitations

MEASURED ([contract](docs/CORRECTNESS.md), [wire guard](src/wire.rs), [pilot](docs/PILOT.md)):

- Pending payloads have count bounds; opaque payload bytes and transport/upstream allocations are not byte bounded here.
- Connection bank tracking terminates at its configured capacity. Safe continuous rotation and replay-gap handling remain unbuilt.
- A late conflict does not reverse earlier downstream effects. Durable conflict handling requires additional host policy.
- The sink needs a caller-established trusted anchor; it cannot verify external history or ledger/payload validity.
- Process-abort recovery is exercised; power loss, disk-full and networked databases are outside the demonstrated coverage.
- The experimental upstream dependency and actual provider compatibility remain release gates.
- One PublicNode attempt failed reconstruction. No successful two-provider pilot or external independent review is recorded.

Read the [dependency review](docs/DEPENDENCY-REVIEW.md), [provider result](docs/PUBLICNODE-RESULT.md)
and [verification record](docs/VERIFICATION.md) before assessing the prototype.
