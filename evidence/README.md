# Evidence index

Raw logs and manifests behind the claims in the documentation. Nothing here is edited except that
a private host address and account name were replaced by placeholders on 25 October 2026. Each
file keeps the meaning it had when written, so older ones describe older source.

## Current (read these)

| File | What it is |
|---|---|
| `verification-pass2.log` | Full locked `scripts/verify.sh` run on the current tree, 26 October (exit 0; 24 / 64 / 64 tests) |
| `source-manifest.sha256` | SHA-256 of the 37 executable inputs of the current tree, as tested by `verification-pass2.log` |
| `pass2-regressions-before-fix.log` | The new schema-refusal test failing on the old `SqliteSink::open` (2 passed, 1 failed) |
| `demo/` | Separate evidence for the browser demo crate: log, manifest and the artifact hash (see its README) |
| `verification-reverify.log`, `source-manifest-reverify.sha256` | Previous full run, 25 October, commit `b5d883b` (exit 0; 24 / 61 / 61 tests) |
| `verification-final.log`, `source-manifest-final.sha256` | Earlier run on the final 25 October tree (differs in `Cargo.toml` and two workflow files) |
| `audit-regressions-before-fix.log`, `audit-regressions-after-fix.log` | New tests failing on the old behaviour, then passing after the fix |
| `publicnode-access.log`, `publicnode-access-initial.log` | The one real single-provider test (authenticated, streamed, reconstructed nothing) |
| `readiness-status-2026-10-25.json` | GitHub state read on 25 October: CI runs, runners, upstream issue and PR status |

## Superseded runs (kept unchanged as history)

`verification-audit.log` (first audit pass), `verification-current.log`,
`verification.log`, `verification-initial.log`,
`verification-grpc-repair.log`, `locked-verification.log`, and the manifests
`source-manifest-audit.sha256`, `source-manifest-pre-audit.sha256`, `source-manifest-initial.sha256`.

## Other

`dependency-review/` reproduces the two upstream defects against the pinned crate;
`sink-metadata-regression-before-fix.log` is a regression that failed before its fix;
`pilot-preflight.log` shows the pilot refusing to start without credentials;
`upstream-*.json` are saved GitHub API responses.
