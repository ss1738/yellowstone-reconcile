# Evidence index

Raw logs and manifests behind the claims in the documentation. Nothing here is edited except that
a private host address and account name were replaced by placeholders on 25 and 26 October 2026.
Each file keeps the meaning it had when written, so older ones describe older source. A log is
**current** only if its manifest matches the files in the repository now; check that with the
commands under "Check currency" below.

## How to read test counts

`scripts/verify.sh` runs the same Rust tests three times: default features, all features in debug,
and all features in release. A total such as "152" counts executions, not distinct tests.

| Run on 26 October 2026 | Executions |
|---|---|
| Default features | 24 (a subset of the next row) |
| All features, debug | 64 |
| All features, release | 64 (the same tests again) |
| Sum | 152 |

Distinct Rust tests: **64**. Two of them (`crash_child`, `mid_transaction_child` in `tests/sink.rs`) are
helper subprocesses that the abort tests start, so there are 62 independent scenarios. The 4 Python
harness tests (`scripts/test_pilot.py`) are counted separately and are not part of the 152. Counts are
what the test runner reported, not proof obligations.

## Current verification (matches today's source)

| File | What it is |
|---|---|
| `verification-20261026-ci-wording.log` | Full locked `scripts/verify.sh` run on the current tree, 26 October 2026, exit 0 (24 / 64 / 64 executions) |
| `source-manifest.sha256` | SHA-256 of the 37 executable inputs of the tree that run tested |
| `verification-pass2.log`, `source-manifest-pass2.sha256` | Earlier same-day run that tested the SQLite fix (exit 0; same counts). It differs from the current tree only by a comment in `.github/workflows/ci.yml` |
| `pass2-regressions-before-fix.log` | The new schema-refusal test failing on the old `SqliteSink::open` (2 passed, 1 failed). The fix is in the next commit |
| `demo/` | Separate scope: the browser demo crate. Log, manifest and the hash of the built artifact; see its README |

## Historical runs (recorded for an older tree; kept as history)

| File | What it describes |
|---|---|
| `verification-reverify.log`, `source-manifest-reverify.sha256` | 25 October, commit `b5d883b` (exit 0; 24 / 61 / 61 executions, 61 distinct tests) |
| `verification-final.log`, `source-manifest-final.sha256` | Earlier 25 October tree (differs in `Cargo.toml` and two workflow files) |
| `verification-audit.log`, `source-manifest-audit.sha256` | First audit pass |
| `verification-current.log`, `verification.log`, `verification-initial.log`, `verification-grpc-repair.log`, `locked-verification.log` | Earlier runs |
| `source-manifest-pre-audit.sha256`, `source-manifest-initial.sha256` | Earlier manifests |

## Regression evidence (a test failing on old behaviour, then passing)

`audit-regressions-before-fix.log` and `audit-regressions-after-fix.log`;
`sink-metadata-regression-before-fix.log`; `pass2-regressions-before-fix.log` (above).

## Grant research (dated facts, not code evidence)

`grant-research-20261026.md`: what the application form, template, Foundation page, Alpenglow page and
upstream projects said on 26 October 2026, what could not be verified, and the provider price basis.
`dependency-review/published-0.12.0-check-20261026.log`: the library tested against the published
`yellowstone-block-machine` 0.12.0 in a scratch copy (64 tests pass; the dead-slot scenario emits 2 slots
where the pinned revision delivers 70). It is a measurement, not a verification run of the repository tree.

## Real-provider and upstream evidence

`publicnode-access.log` and `publicnode-access-initial.log`: the one real single-provider test. It
authenticated and streamed updates and reconstructed no block; the cause is unresolved. It is not a
pilot. `dependency-review/` reproduces the two upstream defects against the pinned crate.
`readiness-status-2026-10-25.json` is GitHub state read on 25 October. `pilot-preflight.log` shows the
pilot harness refusing to start without credentials.

## Check currency

```sh
python3 scripts/source_manifest.py | diff -u evidence/source-manifest.sha256 -
python3 demo/manifest.py | diff -u evidence/demo/source-manifest-demo.sha256 -
```

An empty diff means the archived run still describes those source files. A difference does not mean
a regression; it means the run no longer identifies the current source and a new one is needed.
