# Roadmap

## Available for prototype evaluation

MEASURED ([source](../src/), [tests](../tests/), [examples](../examples/)):
the project includes the agreement core, per-connection upstream bridge, input guard, replay CLI,
finalized SQLite sink, payload encoding and browser fixture demo.
The [verification record](VERIFICATION.md) separates exercised behavior from unvalidated operation.

## Planned engineering

INFERRED: these address the current implementation's operating limits:

- Safe connection/bank lifecycle and rotation with explicit replay-gap reporting.
- Byte accounting and refusal for retained opaque payloads.
- Restart-visible conflict handling and an operator recovery example.
- A supported upstream dependency or explicitly maintained patch path.
- Actual independent-provider compatibility, recovery and sustained-run validation.
- Independent review, remediation and a supported package release.

These are planned tasks, not delivered features or promised release dates.

## Integration and maintenance

INFERRED: validate requirements with external indexing maintainers, publish reproducible integration
examples and maintain a supported-version policy. No adopter or upstream endorsement is claimed.
Development and release status will be recorded in [CHANGELOG](../CHANGELOG.md).

Report a reproducible failure through the [issue tracker](https://github.com/ss1738/yellowstone-reconcile/issues).
Use the [security policy](../SECURITY.md) for sensitive findings.
