# Security

This is an unaudited prototype. Do not use it as the sole source of truth for moving funds,
trading, bridging, consensus verification or durable accounting.

Provider agreement does not prove payload integrity, provider independence or chain validity.
The public API relies on upstream sealing and commitment and on host-controlled provider
identity. `docs/CORRECTNESS.md` sets out the trust boundary.

## Reporting a vulnerability

Use GitHub's private vulnerability reporting for this repository (the Security tab, "Report a
vulnerability"). If that option is unavailable, open an issue titled "security contact request"
with no technical detail and the maintainer will arrange a private channel. Do not post exploit
details or credentials in a public issue.

## Known dependency risk

The upstream block-machine is pinned to an open, unmerged pull request, and that revision lacks
the fix for upstream's malformed-input panic (issue 19), which `src/wire.rs` guards against.
`docs/DEPENDENCY-REVIEW.md` records the exact revisions and what is and is not covered.
