# Readiness and remaining work

MEASURED (`src/`, `tests/`, `scripts/`): the local prototype includes reconciliation, typed
upstream integration, guarded transport, replay, SQLite with a linked transactional watermark,
synthetic restart/abort tests and an offline pilot harness. Current run evidence is indexed in
`docs/VERIFICATION.md`.

## Verification today

Full verification (`scripts/verify.sh`) covers default and all features, debug and release,
lint and docs, fixture exit codes, upstream defect probes and SQLite recovery. There is no
automatic CI and no registered runner (`docs/RUNNER-SETUP.md`); running the script by hand is the
working verification route.

## External validation and release

1. Obtain two independent compatible provider feeds and plan quotations. Run the smoke protocol
   in `docs/PILOT.md`; record source identity and failures. Access is not currently available.
2. Obtain a named integrator's recovery requirements and scope feedback. No message has been sent;
   none of this is evidence of an adopter.
3. Store payloads from a real feed (the encoding is done and tested on simulated events);
   implement replay/history negotiation, safe rotation and backfill; validate controlled outages
   and a sustained soak.
4. Obtain independent review and remediate findings. Resolve the experimental upstream pin via
   a supported release or an explicit independently reviewed maintenance decision.
5. Re-estimate the provisional $12,210 request around genuinely remaining work. Confirm the
   current form and public links before submission.

INFERRED: a working prototype is a basis for validation, not proof of demand or funding likelihood.
No production deployment, paid service purchase or release publication has happened.
