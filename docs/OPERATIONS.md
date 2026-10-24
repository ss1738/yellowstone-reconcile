# Operations and local development

## Build policy for this owner

Use `mini-test -m 3 -- 'bash scripts/verify.sh'` from the repository. Mini 3 maps to
the owner's Ironman host in the installed tooling. Use `-m 2` for Apsarth if reachable.
Do not use automatic node selection: the tooling's Mini 1 is not an authorized host
for this owner's project data. Rust builds and tests must not run on the MacBook.

The toolchain is pinned in `rust-toolchain.toml`. `Cargo.lock` records resolved package
versions once generated on the Mini. Upstream is pinned by Git revision as well.

## Limits and monitoring

Track core `Stats` (emitted, duplicate, conflict, retained slot and pending payload counts)
and `UpstreamBridge::pending_blocks()`. Treat conflicts and capacity errors as actionable.
Do not swallow them and continue committing output. A stuck peer will stop quorum;
this is an explicit consistency/availability tradeoff, not automatic failover.

With three providers and quorum two, two agreeing healthy providers can make progress
unless a conflicting qualified observation quarantines the slot. Quarantine is deliberately
conservative; the application needs an operator policy for recovery.

The transport example retries each endpoint three times and requests complete transaction
events plus upstream reconstruction metadata/sysvars/footer. Both endpoints must support
the pinned proto and these filters. The demonstration exits after 100 accepted blocks by
default, or after `RECONCILE_MAX_BLOCKS` in 1..=1000. It does not advance retention or save
an output checkpoint and should not be used as a long-running ingestion service.

## Credentials

Export `YELLOWSTONE_A_URL`, `YELLOWSTONE_B_URL`, and optional separate `_TOKEN` variables.
Use HTTPS and distinct provider trust domains. Do not put tokens in fixtures, config
checked into Git, logs or shell arguments. `.env` and common key-file extensions are ignored.
The example suppresses raw connection errors because they can include endpoint details.

## Deployment and publication

The current work is local. Do not create a GitHub remote, publish a crate, deploy a service,
send outreach, or submit the grant until the owner explicitly requests those actions.
`publish = false` guards accidental crate publication. The optional GitHub workflow is
manual and requires an owner-provisioned Mini runner; no runner is assumed to exist.

## Release checks

1. Run the complete verification script on a Mini and save its raw output in `evidence/`.
2. Review the source against the correctness contract, including negative and positive cases.
3. Review dependencies and licenses; recheck upstream branches and PR overlap.
4. Obtain an independent code review and run credentialed provider pilots before production.
5. Populate application identity/contact fields and replace draft adoption hypotheses with evidence.
6. Request permission to publish the repository and separately to submit the application.
