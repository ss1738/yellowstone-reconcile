# Verification status: 25 October 2026

## Current source (post-audit)

MEASURED: `evidence/verification-audit.log` is the raw output of `mini-test -m 3 -- bash
scripts/verify.sh` on Ironman, Rust/Cargo 1.95.0, run 25 October 2026 against commit `7874365`
plus the audit changes (committed afterwards; the manifest identifies them). `scripts/verify.sh`
exited 0. Passing tests reported by the runner, summed from that log:

- Default features: 24 tests.
- All features, all targets: 59 tests in debug and the same 59 in release.
- Formatting, Clippy with warnings denied, documentation with warnings denied, all with `--locked`.
- Agreement replay succeeded; conflict and malformed replay each exited exactly 1.
- Benchmark, upstream bridge, upstream defect probe, SQLite recovery, in-process sink ordering,
  stall-budget and payload-size examples, and the credential-free pilot preflight.

MEASURED: the 59 are 14 core, 4 audit sequences, 2 reconstruction, 3 CLI, 3 restart, 17 SQLite,
7 upstream bridge and 9 wire. The 17 SQLite tests include `crash_child` and `mid_transaction_child`,
helpers re-executed by the two abort tests, so 15 are real. Counts are runner reports, not
independent proof obligations: `randomized_interleavings_emit_each_slot_once` samples a finite set
of schedules and is one test, not a proof over all interleavings.

MEASURED: `evidence/source-manifest.sha256` lists the 34 executable inputs of that run, including
workflows. It was regenerated locally and diffed against the hashes the Mini printed; they match.
The previous manifest is kept as `source-manifest-pre-audit.sha256`. Recheck with:

```sh
python3 scripts/source_manifest.py | diff -u evidence/source-manifest.sha256 -
```

The manifest excludes prose, generated PDFs and evidence; editing docs does not change which
executable source was tested.

MEASURED: the earlier run, `evidence/verification-current.log` (`20261025-124903`, commit
`7874365`, 48 tests, 33-path manifest `source-manifest-pre-audit.sha256`), is preserved unchanged
as the pre-audit result. Regressions that failed before their fixes are in
`evidence/audit-regressions-before-fix.log`, and their passing runs in `-after-fix.log`.

## Recovery and dependency observations

MEASURED: the SQLite example first stored slots `[4,3,2,1,8,7]`; the watermark remained 4 despite
the highest stored slot being 8. After reopening and replay, it reported 20 rows and watermark 20,
with two already-stored redeliveries absorbed. `tests/sink.rs` separately exercises SIGABRT;
the example itself closes/reopens cleanly. Evidence: `evidence/verification-current.log`.

MEASURED: `evidence/sink-metadata-regression-before-fix.log` records a focused regression failing
because same-hash replay with changed parent/count metadata returned `AlreadyPresent`. The
current sink checks parent slot/hash and both counts as well as blockhash. The regression now
passes for all three mutated fields; the original row and watermark are preserved. Opaque payload
bytes are intentionally not compared. Trusted-anchor preconditions are documented explicitly.

MEASURED: the dependency probe caught both known malformed-hash panic paths and the wire guard
rejected those same inputs. With the experimental PR #22 pin, the repaired-dead-slot scenario
reported zero fork diagnostics and delivered 70 slots. Historical baseline reproduction delivered
only 2 of 70; see `docs/DEPENDENCY-REVIEW.md` and `evidence/dependency-review/`.
These are synthetic cases, not production incidence or a complete audit of upstream code.

MEASURED: the payload example retained 598,486,305 synthetic bytes (570.8 MiB) when cycling its
hard-coded RPC-size inputs through the 128-block bound. That is not measured process RSS,
a statistical mainnet size distribution or a Geyser wire-memory ceiling. At the supplied 212 ms
input, the stall example derives 27.1 seconds. Neither is an operational guarantee.

## Pilot and CI readiness

MEASURED: a subsequent owner-authorized single-provider PublicNode check connected and subscribed
using the supplied token, but stopped on `MissingBankId` with no reconstructed blocks.
See `evidence/publicnode-access.log`, `examples/access_probe.rs` and `docs/PUBLICNODE-RESULT.md`.
This is actual network access evidence, not a successful reconstruction or two-provider pilot.
The private credential wrappers were removed; remote absence is recorded in `pilot-preflight.log`.

MEASURED: `evidence/pilot-preflight.log` shows owner-Mini detection succeeded but provider access
checks failed. `scripts/pilot.py --run` exited 2 before build or connection when those credentials
were absent. No live network pilot has been executed. The owner requested offline preparation.

MEASURED: `evidence/readiness-status-2026-10-25.json` records historical GitHub CI successes on
commit `7999f67`, zero registered self-hosted runners, upstream PR #22 open/unmerged and scoping
issue #23 with zero comments. Historical CI tested default/upstream features only; it is not
coverage of this revised local workflow or the SQLite/wire features.

MEASURED (`.github/workflows/`): `ci.yml` runs on GitHub-hosted Ubuntu with a read-only token and
covers default, `sqlite` and `upstream` features, Clippy and fixture exit codes. The same command
set passed on Ironman (macOS), recorded in the audit session, but **the workflow itself has not
run on GitHub** and Ubuntu behavior is unverified. `verify.yml` is manual dispatch on a
self-hosted Ironman-labelled runner and checks the hostname; with zero registered runners it
cannot run. Neither workflow executes pull-request code on a personal machine. The grpc and
simulation features have no CI at all and rely on `scripts/verify.sh` on a Mini. Nothing here is
an operational green run of either workflow.

## Reproduce on the owner's machine

```sh
mini-test -m 3 -- 'bash scripts/verify.sh'
```

Do not use automatic Mini selection; Mini 1 is not the owner's device. See `docs/PILOT.md` for
credential preparation and the opt-in smoke protocol after compatible provider access is available.

## Historical evidence and limits

`evidence/verification.log`, `locked-verification.log`, `verification-initial.log` and
`verification-grpc-repair.log` are historical runs, not current-source verification. The initial
manifest is preserved in `evidence/source-manifest-initial.sha256`.

INFERRED: passing finite synthetic tests supports prototype review. It does not establish provider
compatibility, continuous gap-free ingestion, power-loss safety, payload validity, independence,
adoption, external review or funding likelihood. Full upstream payload persistence, replay/rotation,
a live pilot, sustained soak and independent review remain future gates. Publishing is disabled.
