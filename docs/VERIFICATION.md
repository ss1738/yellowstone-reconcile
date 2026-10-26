# Verification

## Current executable inputs

MEASURED ([raw run](../evidence/verification-public-cleanup-20261026.log),
[manifest](../evidence/source-manifest.sha256)): the recorded 26 October 2026 run of
scripts/verify.sh on the maintainer's Apple-silicon macOS build machine exited 0.

| Configuration | Passing test executions |
|---|---:|
| Default features | 24 |
| All features, debug | 64 |
| All features, release | 64 |
| Total executions | 152 |

MEASURED ([test sources](../tests/), [run](../evidence/verification-public-cleanup-20261026.log)):
the configurations repeat tests. There are 64 distinct Rust tests, including two SQLite
subprocess helpers; four offline Python harness tests are counted separately. Formatting,
Clippy, documentation, fixture exit codes and offline examples passed in that recorded run.
These counts are not proof obligations or evidence of real-provider compatibility.

The current manifest is generated from source files, tests, examples, fixtures, verification
scripts, dependency/toolchain pins and workflows. Documentation and the browser demo are outside
the main manifest. Compare before relying on an archived result:

    python3 scripts/source_manifest.py | diff -u evidence/source-manifest.sha256 -

An empty diff identifies the same inputs. A difference requires new verification; it is not
itself proof of a regression. The run above includes removal of the unused self-hosted workflow and correction of
manual-workflow comments during repository cleanup. Its 36-input manifest matches the resulting tree.

## Reproduce

On a build machine with the pinned Rust toolchain:

    bash scripts/verify.sh

The script prints results without creating an evidence file. Preserve pipeline failure status
when capturing output:

    bash -o pipefail -c 'bash scripts/verify.sh 2>&1 | tee /tmp/yellowstone-verify.log'

MEASURED ([workflow](../.github/workflows/ci.yml)): the hosted workflow is manual only and covers
a narrower feature set than scripts/verify.sh. Historical CI is not coverage of this tree.
A configured workflow is not evidence of a pass. No self-hosted runner is required to run the
verification script directly.

## Browser demo: separate scope

MEASURED ([demo record](../evidence/demo/README.md)): demo/verify.sh passed four native tests,
formatting and Clippy and produced two identical clean WASM builds on 26 October 2026.
The recorded artifact is 135,855 bytes, SHA-256
a41f378f69efda10e92ee443e958e459bb9b9e1a2cebaa3c58046bf51984ea2b.
The record includes the deployment check made that day; a later deployment must be checked separately.

    python3 demo/manifest.py | diff -u evidence/demo/source-manifest-demo.sha256 -

A main-suite pass does not verify the demo or a deployed website. Use the
[demo build instructions](../demo/README.md) for demo changes.

## Live validation and coverage limits

MEASURED ([provider result](PUBLICNODE-RESULT.md), [raw access log](../evidence/publicnode-access.log)):
one single-provider attempt authenticated and received updates, then stopped at a missing-bank-ID
rejection with zero reconstructed blocks. It is not a successful pilot.

INFERRED: finite synthetic tests support review of the exercised behaviors. They do not establish
continuous lossless ingestion, provider independence, cryptographic payload validity, power-loss
safety, production adoption or independent audit. See the [contract](CORRECTNESS.md),
[self-review](../AUDIT.md), [pilot protocol](PILOT.md) and [review questions](REVIEW-PACKET.md).
