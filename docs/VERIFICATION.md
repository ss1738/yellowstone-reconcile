# Verification status: 25 October 2026

## Current source

MEASURED: `evidence/verification-current.log` records run `20261025-124903` on Ironman,
Rust/Cargo 1.95.0. `scripts/verify.sh` completed with exit 0:

- Default-feature tests: 20 reported passing tests.
- All features/all targets: 48 reported passing tests in debug and the same 48 in release.
- Formatting, Clippy with warnings denied, documentation with warnings denied.
- Agreement replay succeeded; conflict and malformed replay each exited exactly 1.
- Synthetic benchmark, upstream bridge and upstream defect probe.
- SQLite unordered-delivery/restart, in-process sink-ordering, stall-budget and payload-size examples.
- Credential-free pilot preflight.

MEASURED: all-feature tests include 14 core, 2 reconstruction, 3 CLI, 3 restart, 12 SQLite,
7 upstream bridge and 7 wire tests. The 12 reported SQLite tests include `crash_child`, a helper
re-executed by the process-abort test; counts are runner reports, not independent proof obligations.
Evidence: test-result sections in `evidence/verification-current.log` and source files in `tests/`.

MEASURED: the 33 paths in `evidence/source-manifest.sha256` identify this run's executable inputs,
including workflows. The formatted transport example and manifest were retrieved from Ironman;
a local regeneration matched exactly. Recheck with:

```sh
python3 scripts/source_manifest.py > /tmp/yellowstone-current.sha256
diff -u evidence/source-manifest.sha256 /tmp/yellowstone-current.sha256
```

The manifest excludes prose, generated PDFs and evidence; updating this page does not change
which executable source was tested. It includes its generator script, not the generated manifest.

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

MEASURED (`.github/workflows/ci.yml`, `.github/workflows/verify.yml`): both local workflows now
require explicit owner dispatch on a self-hosted Ironman-labelled Mini, check hostname and call
the full verification script. This respects the owner's Mini-only Rust-build requirement and
does not execute public pull-request code automatically on a personal machine. Runner registration
and a subsequent Actions run are outstanding. No runner was registered or workflow published here.

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
