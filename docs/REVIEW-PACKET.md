# Review checklist

MEASURED ([self-review](../AUDIT.md), [verification](VERIFICATION.md)):
the repository records maintainer reviews and finite regression tests. No independent review is
recorded. This checklist helps a reviewer assess the implementation without implying an audit.

## Reproduce first

    bash scripts/verify.sh
    python3 scripts/source_manifest.py | diff -u evidence/source-manifest.sha256 -

Read the [contract](CORRECTNESS.md), [integration guide](INTEGRATION.md),
[dependency review](DEPENDENCY-REVIEW.md) and [pilot protocol](PILOT.md).

## Questions

- Do commitment and sealing refer to the exact bank being compared?
- Are provider identity and reconnect generations isolated in all failure paths?
- Can capacity, quarantine or retention hide a pending block or a late conflict?
- Are payload equality and ledger validity kept outside the metadata-agreement claim?
- Does the trusted anchor and linked watermark handle unordered delivery and skipped slots?
- Are replay/history gaps reported rather than silently skipped?
- Are SQLite transaction, schema-refusal and replay-identity semantics adequate for the consumer?
- Do transport/upstream allocations exceed the count-based limits?
- Is the dependency revision supported for the target deployment?
- Does actual provider input satisfy the subscription and bank-identity assumptions?

Record the reviewer relationship, exact source/manifest, scenarios reproduced, findings,
remediation commits and unresolved limits. A passing suite or a tool reviewing its own output is
not an independent review.
