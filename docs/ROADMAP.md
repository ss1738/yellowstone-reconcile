# Readiness and remaining work

MEASURED (`src/`, `tests/`, `scripts/`): the local prototype includes reconciliation, typed
upstream integration, guarded transport, replay, SQLite with a linked transactional watermark,
synthetic restart/abort tests and an offline pilot harness. Current run evidence is indexed in
`docs/VERIFICATION.md`.

## Local preparation

The revised workflows use owner-triggered execution on a runner labelled `ironman`, with an
identity check. Full verification covers default/all features, debug/release, lint/docs, fixture
exit codes, upstream defect probes and SQLite recovery. No runner registration is performed here;
`mini-test -m 3 -- 'bash scripts/verify.sh'` remains the working verification route.

## External validation and release

1. Obtain two independent compatible provider feeds and plan quotations. Run the smoke protocol
   in `docs/PILOT.md`; record source identity and failures. Access is not currently available.
2. Obtain a named integrator's recovery requirements and scope feedback. Outreach drafts are
   preparation, not evidence of an adopter. No messages are sent by local preparation.
3. Integrate actual reconstructed payloads into SQLite; implement replay/history negotiation,
   safe rotation and backfill; validate controlled outages and a sustained soak.
4. Obtain independent review and remediate findings. Resolve the experimental upstream pin via
   a supported release or an explicit independently reviewed maintenance decision.
5. Re-estimate the provisional $12,210 request around genuinely remaining work. Confirm private
   applicant details, public links and current form; obtain owner approval for submission.

INFERRED: a working prototype is a basis for validation, not proof of demand or funding likelihood.
No production deployment, paid service purchase or release publication is part of this preparation.
