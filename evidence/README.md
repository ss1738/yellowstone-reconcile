# Verification and regression evidence

This directory retains the current verification run, focused failure reproductions, dependency
checks, demo evidence and the unsuccessful real-provider access result.
Logs describe the source and scope recorded with them; do not treat older reproductions as tests
of newer code. Some published logs replace private build-account/address details with placeholders.

## Current verification

MEASURED ([run](verification-public-cleanup-20261026.log), [manifest](source-manifest.sha256)):
the recorded suite passed with 24 default, 64 all-feature debug and 64 release test executions.
Those 152 executions repeat 64 distinct Rust tests, including two subprocess helpers.
Four Python harness tests are separate. See [verification](../docs/VERIFICATION.md) for currency.

    python3 scripts/source_manifest.py | diff -u evidence/source-manifest.sha256 -
    python3 demo/manifest.py | diff -u evidence/demo/source-manifest-demo.sha256 -

## Focused evidence

| Files | Scope |
|---|---|
| [audit before](audit-regressions-before-fix.log), [after](audit-regressions-after-fix.log) | Original failing regressions and their corrections |
| [metadata replay regression](sink-metadata-regression-before-fix.log) | Same hash with different identity metadata refused after correction |
| [schema refusal regression](pass2-regressions-before-fix.log) | Refusing a newer schema must not intentionally change journal mode |
| [dependency-review/](dependency-review/) | Pinned and published-upstream probes; dated issue/PR snapshots |
| [demo/](demo/) | Separate demo tests, source manifest and reproducible WASM artifact |
| [PublicNode access](publicnode-access.log) | Authenticated stream followed by guard rejection; zero reconstructed blocks |

MEASURED ([published-base check](dependency-review/published-0.12.0-check-20261026.log)):
the scratch published-upstream comparison is not a verification run of the pinned repository.
Read the [dependency review](../docs/DEPENDENCY-REVIEW.md) before interpreting it.
