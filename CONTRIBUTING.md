# Contributing

Agree on scope before implementing broad changes. Keep the reconciliation contract separate
from transport, reconstruction, chain ordering and durable sink policies.

Read `docs/CORRECTNESS.md`. Add positive and negative tests for any changed decision rule.
Do not claim stronger delivery or verification guarantees than those tests and assumptions support.

For this owner's environment, run Rust verification on the Mini through the commands in
`docs/OPERATIONS.md`. The complete check script is `scripts/verify.sh`; live provider testing
is a separate opt-in pilot. Keep provider credentials and private application details out of Git.

Use small, reviewed commits. Do not add tool-attribution or Co-Authored-By trailers.
Follow the actual contribution requirements of an upstream repository before opening PRs there.
