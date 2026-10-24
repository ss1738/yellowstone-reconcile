# Security and responsible reporting

This is an unaudited local development release. Do not use it as the sole source of truth
for moving funds, trading, bridging, consensus verification or durable accounting.

Provider agreement does not prove payload integrity, provider independence or chain validity.
The public API relies on upstream sealing/commitment and host-controlled provider identity.
See `docs/CORRECTNESS.md` for the full trust boundary.

Before public publication, configure a private vulnerability reporting channel in the hosting
repository. Do not publish exploit details or credentials in public issues. No active private
reporting URL exists for this local-only repository yet.

Dependencies include a pinned upstream with known open correctness/input-validation reports;
see `docs/RESEARCH.md`. Review and re-pin after testing fixes before any production release.
