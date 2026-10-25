# Verification status: 25 October 2026

## Current source (final, second audit pass)

MEASURED: `evidence/verification-final.log` is the raw output of `bash scripts/verify.sh`
on the maintainer's Mac mini (Ironman), Rust/Cargo 1.95.0, 25 October 2026, against the tree whose 36
executable inputs are listed in `evidence/source-manifest.sha256`. I diffed that manifest against
the hashes the build machine printed and they match; its formatter had rewritten two files, which
were copied back first. `scripts/verify.sh` exited 0. Passing tests the runner reported, summed
from that log:

- Default features: 24. All features, all targets: 61 in debug and the same 61 in release.
- 4 Python harness tests (`scripts/test_pilot.py`), which run offline with no build.
- Formatting, Clippy with warnings denied and documentation with warnings denied, all `--locked`.
- Agreement replay succeeded; conflict and malformed replay each exited exactly 1.
- Benchmark, upstream bridge, upstream defect probe, SQLite recovery, in-process sink ordering,
  stall-budget and payload-size examples, and the credential-free pilot preflight.

MEASURED: the 61 are 14 core, 4 audit sequences, 2 real-payload, 2 reconstruction, 3 CLI,
3 restart, 17 SQLite, 7 upstream bridge and 9 wire. Of those, 2 SQLite tests are helpers re-executed
by the abort tests. They are independent scenarios only in part: the randomized interleaving test
samples finitely many schedules and counts as one; the real-payload test uses a simulated wire,
not a network. Counts are runner reports, not proof obligations.

Earlier runs keep their original meaning: `verification-current.log` (`7874365`, 48 tests),
`verification-audit.log` (first audit pass, 59 tests, manifest `source-manifest-audit.sha256`) and
the pre-audit manifest `source-manifest-pre-audit.sha256`. Private host identifiers (address and
account name) were replaced with placeholders in published logs on 25 October; raw originals are
kept outside the repository.

**Known difference, 25 October 2026 (afternoon):** three executable inputs changed after the run
above and have **not** been re-verified, because the build machine was unreachable:
`Cargo.toml` (added `repository`, `readme`, `keywords`, `categories`; changed one comment) and the
two workflow files (comments and one name). No dependency, feature, source or test changed, and
`Cargo.lock` is untouched. Until `scripts/verify.sh` is re-run, the diff below will list exactly
those three paths.

Recheck the manifest with:

```sh
python3 scripts/source_manifest.py | diff -u evidence/source-manifest.sha256 -
```

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

MEASURED: a subsequent single-provider PublicNode check connected and subscribed
using the supplied token, but stopped on `MissingBankId` with no reconstructed blocks.
See `evidence/publicnode-access.log`, `examples/access_probe.rs` and `docs/PUBLICNODE-RESULT.md`.
This is actual network access evidence, not a successful reconstruction or two-provider pilot.
The private credential wrappers were removed; remote absence is recorded in `pilot-preflight.log`.

MEASURED: `evidence/pilot-preflight.log` shows build-machine detection succeeded but provider access
checks failed. `scripts/pilot.py --run` exited 2 before build or connection when those credentials
were absent. No live network pilot has been executed. Provider access was not available, so only offline preparation was done.

MEASURED: `evidence/readiness-status-2026-10-25.json` records historical GitHub CI successes on
commit `7999f67`, zero registered self-hosted runners, upstream PR #22 open/unmerged and scoping
issue #23 with zero comments. Historical CI tested default/upstream features only; it is not
coverage of this revised local workflow or the SQLite/wire features.

MEASURED (`docs/RUNNER-SETUP.md`): there is no automatic CI. `ci.yml` is a GitHub-hosted workflow that
starts only by hand, because the maintainer's practice is to build and test Rust on their own machines; it has
never run. `verify.yml` targets a self-hosted `ironman` runner that is not registered (0 runners,
`gh api`, 25 October), so it cannot run either. Neither is triggered by a push or a pull request.
The grpc and simulation features are covered only by `scripts/verify.sh`. Operational verification
is the manual run above, nothing more.

## Reproduce

```sh
bash scripts/verify.sh
```

See `docs/PILOT.md` for
credential preparation and the opt-in smoke protocol after compatible provider access is available.

## Historical evidence and limits

`evidence/verification.log`, `locked-verification.log`, `verification-initial.log` and
`verification-grpc-repair.log` are historical runs, not current-source verification. The initial
manifest is preserved in `evidence/source-manifest-initial.sha256`.

INFERRED: passing finite synthetic tests supports prototype review. It does not establish provider
compatibility, continuous gap-free ingestion, power-loss safety, payload validity, independence,
adoption, external review or funding likelihood. Full upstream payload persistence, replay/rotation,
a live pilot, sustained soak and independent review remain future gates. Publishing is disabled.
