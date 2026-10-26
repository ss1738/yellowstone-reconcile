# Dependency review, 25 October 2026

## Baseline and public reports

MEASURED: GitHub API responses in `evidence/dependency-review/` record upstream main at
`b828807d45522505cfdd6b4710d2287b29e5d99a`. Issues #19 and #21 and their proposed fixes
[#20](https://github.com/rpcpool/yellowstone-block-machine/pull/20) and
[#22](https://github.com/rpcpool/yellowstone-block-machine/pull/22) were open and unmerged
when retrieved. PR #22's inspected head was `0c732a049d6d18008a541f47b09ddb7b949cbb65`.

MEASURED: `evidence/dependency-review/pinned-upstream-reproduction.log` records both
malformed-hash adapter panic paths on the baseline; the probe deliberately caught them.
The new WireGuard rejected those inputs before the upstream adapter was called.

MEASURED: the same probe generated a repaired-dead-slot lineage covering slots 41..110.
The baseline produced 68 fork reports for descendants and delivered only slots 41 and 42.
This is synthetic reproduction, not a production incident or estimated incidence rate.

## Companion regression

MEASURED: `adapter-regression-before-fix.log` shows the new focused regression failing
against our previous adapter: a slot-level ForkDetected report removed a pending sealed
candidate. The adapter now counts those reports as diagnostics and relies on explicit
BankDiscarded and DeadBlockDetected notifications for pruning.

MEASURED: the baseline probe still delivered only two slots after that companion correction.
Source inspection located an earlier prune inside upstream `BlockStream` itself. Ignoring
the report after reconstruction cannot recover a payload already removed upstream. An
upstream correction is required for the repaired-lineage scenario.

## Input boundary

Source contract (`src/wire.rs`): GuardedSource is scoped to one connection and must be
replaced together with BlockStream on reconnect. It checks malformed block/entry/footer
hashes, missing bank ids on bank-specific slot statuses, unknown slot statuses and bank-id
reuse across slots. A rejection is a terminal typed error, not a silent filter. The demo
stops on invalid wire input and does not expose endpoint details in that error message.

Bank identities are not evicted while the upstream machine might retain them. Reaching
the configured bank-map capacity terminates the connection. This bounds count and avoids
forgetting a collision guard early; it is not a long-running persistence strategy. A host
must implement safe connection rotation/replay before continuous production operation.

These checks target the identified boundary defects. They are not exhaustive protobuf,
payload, fork-choice or cryptographic verification.

## Patch review and pin policy

Source inspection: PR #20 rejects malformed hashes and slot-mismatched summaries before
mutating bank state. PR #22 clears stale fork markers along a confirmed repair's ancestry,
preserving dead-bank tombstones and sibling fork markers. The proposed regression cases
include late parent metadata and garbage collection. Neither author reports nor this
inspection alone establishes that the patches work in our consumer.

The baseline main pin cannot pass our complete repaired-lineage probe. An experimental
PR-head pin may be used locally only after independent upstream checks and our full
verification. This does not imply maintainer approval or a supported upstream release.
Publication remains gated on an upstream merge/release or an explicit reviewed dependency
maintenance decision; do not present an experimental pin as production-ready.

## Existing downstream consumer

MEASURED: Shipstern main at `33d62d23208c74593a5b508e81cdd38554af9de8` was inspected in
`crates/block-coordinator/src/source.rs` and `state.rs`. Its coordinator buffers records
per slot in that snapshot. The body of
[PR #345](https://github.com/solana-rpc/shipstern/pull/345) describes a bank-aware migration
and treats slot-level fork reports as diagnostic. The API returned that PR closed and
unmerged at the inspected head `88e4970b46bb923b8bd149a255488ed098752d08`.

INFERRED: this is a useful implementation reference and a possible integration conversation,
not evidence that Shipstern wants our companion or lacks another private/branch solution.
The inspection did not establish global absence of competing cross-provider reconciliation.

## Remaining gate

No live endpoint or token variables were configured in this execution environment when checked.
That limits this review to offline source and synthetic/replay validation. A credentialed
two-provider pilot, upstream feedback and independent review remain separate requirements.

## Recheck, 25 October 2026 (afternoon, GitHub API, read-only)

MEASURED: upstream `main` is still `b828807d45522505cfdd6b4710d2287b29e5d99a` (committed
13 October). Issues #19, #20 (PR), #21, #22 (PR) and #23 are all open with PR #22 unmerged at head
`0c732a049d6d18008a541f47b09ddb7b949cbb65`; #23 has no comments. No GitHub release exists.

MEASURED: `compare` shows the pinned revision is `main` plus exactly two commits, touching
`forks.rs`, `state_machine.rs` and `AGENTS.md`. It does **not** contain PR #20's head
(`e0a4029`, diverged), so upstream issue #19, the malformed-input panic, is **not** fixed in the
pin. `WireGuard` is the only protection against it, and `examples/dependency_probe.rs` reproduces
both panics on the pinned crate. The experimental pin covers #21 only.

MEASURED: a search of GitHub repositories for "yellowstone reconcile" returned only this
repository. That is one search term. It does not establish that no other implementation exists,
including private ones or ones inside consumers such as Shipstern.

Related existing solutions (see [alternatives](ALTERNATIVES.md)): the upstream
Yellowstone client's `DedupStream` and `subscribe_with_reconnect` deduplicate replays for one
logical subscription and forget banks at or below the finalized slot; a keyword search of its
dedup code found no quorum or conflict handling. This does not establish that no equivalent exists.
