# Independent review packet

Status: no independent reviewer engaged and no review completed.

## Reproduce

Run `bash scripts/verify.sh` on a machine with Rust 1.95.
Compare executable inputs with `evidence/source-manifest.sha256`; inspect
`evidence/verification-audit.log` and the deliberately failing pre-fix logs (`evidence/audit-regressions-before-fix.log`, `evidence/sink-metadata-regression-before-fix.log`).

Read `docs/CORRECTNESS.md`, `docs/DEPENDENCY-REVIEW.md`, `docs/PILOT.md` and `src/` before
assessing the grant's claims. `grant/BUDGET.md` excludes this delivered preparation from billing.

## Questions requiring an independent decision

- Does the sealed/commitment contract match each actual provider's subscription semantics?
  Can missing or reordered updates lead to a false qualification or silent payload loss?
- Is reconnect generation isolation complete? What replay/history guarantees and reset protocol
  are needed when the wire guard or core reaches capacity?
- Does conflict quarantine remain visible after emission, restart, retention pruning and sink commit?
  Which downstream effects require compensation or an explicit operator halt?
- Is the caller's trusted anchor sufficiently explicit? The sink does not validate external history
  or verify ledger validity. It rejects blocks below `Finalized` at runtime (not in the type), and
  a unique canonical child is a precondition: with finalized input only one child can exist.
- Does replay compare all stored identity metadata, without accidentally claiming byte equality for
  opaque payloads? Review `replay_with_the_same_hash_but_different_metadata_is_refused`.
- Are SQLite transaction boundaries, disk errors, corruption handling, concurrent access and
  schema/version migration adequate for the intended integrator? Power loss is not tested.
- Can payload sizes, transport buffers or reconstruction state exceed operational memory budgets?
  Existing count bounds and synthetic payload examples are not byte/RSS guarantees.
- Is experimental upstream PR #22 acceptable for a pilot? What supported release or maintenance
  decision is required, and what malformed-wire coverage remains incomplete?
- Does the bounded smoke harness distinguish success, interruption, timeout and partial output?
  Are endpoint credentials absent from evidence and failure output?

## Required review record

Record reviewer identity and relationship to the project, date, exact source manifest, scenarios
reproduced, findings with severity and source locations, accepted limitations, remediation commits
and reviewer disposition. A tool's own review or passing tests is not an independent review.

INFERRED: release should wait for disposition of correctness and recovery findings and for real
provider validation. Grant acceptance and review acceptance are separate decisions.
