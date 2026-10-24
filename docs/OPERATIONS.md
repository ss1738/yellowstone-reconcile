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

## Sizing memory against a lagging provider

`Config` bounds pending work by **count**, not by bytes: `max_pending_blocks` (default 128) and
`max_retained_slots` (default 4096). Nothing can be emitted for a slot until the required number
of providers agree, so while one provider runs ahead its payloads are all retained. The ceiling an
operator actually cares about is therefore `max_pending_blocks` multiplied by their own payload
size.

MEASURED (`cargo run --release --example lag_memory`, 2026-10-25, owner Mini):

| scenario | result |
|---|---|
| 64-slot lag, 2 MiB payloads | `pending_blocks=64`, 134,217,728 bytes retained (128.0 MiB) |
| slow provider catches up | drains to `pending_blocks=0`, `emitted=64` |
| one provider only, past the bound | accepted exactly 128 blocks, then rejected with `PendingCapacity` |

So the default configuration implies roughly a **256 MiB** payload ceiling at 2 MiB blocks, and it
**fails closed** rather than growing without limit. Set `max_pending_blocks` from your own measured
payload size and the memory you are willing to commit, not from the default.

LIMIT: synthetic payloads of uniform size. Retained bytes are computed as `pending_blocks` times
payload size rather than from allocator accounting, and the figures exclude gRPC, reconstruction
and persistence. Reproduce with your own payload size and lag:
`cargo run --release --example lag_memory <payload_bytes> <lag_slots>`.
