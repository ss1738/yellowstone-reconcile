# Evidence index

Raw logs and manifests behind the claims in the documentation. Nothing here is edited except that
a private host address and account name were replaced by placeholders on 25 October 2026. Each
file keeps the meaning it had when written, so older ones describe older source.

## Current (read these)

| File | What it is |
|---|---|
| `verification-final.log` | Full locked `scripts/verify.sh` run on the final tree (exit 0; 24 / 61 / 61 tests) |
| `verification-reverify.log` | Second full run after the metadata and workflow edits, commit `b5d883b` (exit 0; 24 / 61 / 61 tests) |
| `source-manifest.sha256` | SHA-256 of the 36 executable inputs of the current tree, as tested by `verification-reverify.log` |
| `source-manifest-final.sha256` | The same for `verification-final.log` (differs in `Cargo.toml` and two workflow files) |
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
