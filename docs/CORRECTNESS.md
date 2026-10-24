# Correctness contract

This document specifies the intended contract. Tests and logs establish only the
behaviors they exercise; source existence alone does not establish correctness.

## Input assumptions

Each observation is a fully sealed block, and its commitment describes that exact
bank. Raw account or transaction fragments must not enter the reconciler. Reconstruct
each connection separately with the pinned upstream crate. Provider ids describe
independent trust domains assigned by the operator. Multiple streams from one provider
must reuse the same provider id. A single Reconciler covers exactly one cluster and one
subscription/schema configuration. Never reuse it across clusters or filter changes.

The host authenticates endpoints and must enforce transport and payload size limits.
The core accepts opaque payloads and does not recompute a ledger blockhash from them.
Matching metadata therefore establishes agreement on reported identity, not payload
validity. Two dishonest providers can agree on false data.

## Identity and qualification

Connection identity is `(provider_id, generation)`. A new connection invalidates events
from every earlier generation of that provider. The upstream adapter keys its temporary
bank buffers on `(Session, bank_id)`, so equal bank ids across streams do not collide.

Cross-provider comparisons use slot, blockhash, parent slot, parent hash, entry count
and executed transaction count. Local bank id and transport timestamp are excluded.
Zero hashes, non-older parents and observations below the configured commitment are
rejected. Genesis slot is outside this implementation's supported input domain.

Only Confirmed or Finalized is configurable. Finalized and two providers are defaults.
A provider votes at most once per slot, even after reconnection. Every counted vote
has individually met the minimum commitment. The emitted commitment records this
configured floor, not a stronger level inferred from one provider.

## State transitions

| Existing state | Incoming qualified observation | Result |
|---|---|---|
| None | Valid block, available capacity | Retain first payload and provider |
| Pending | Matching metadata, same provider | Keep pending; do not increase votes |
| Pending | Matching metadata, new provider, quorum reached | Emit first payload once |
| Emitted | Matching metadata | Duplicate, no new payload output |
| Pending or Emitted | Any differing compared metadata | Quarantine, release pending payload, report conflict |
| Quarantined | Any later qualified observation | Remain quarantined |
| Below replay floor | Any observation | Reject as stale |

There is no automatic majority-based conflict resolution. Disagreement may mean stale,
incorrect or malicious data; silently selecting the larger group would hide the incident.
If a conflict follows an emission, downstream writes already performed must be handled
by the operator. The sample CLI halts on the first conflict.

## Retention and delivery

The core limits retained slots, pending payloads and registered providers. The bridge
also limits unqualified blocks and per-block event count. These are count bounds, not
a total heap-byte bound. Capacity errors do not silently evict data or advance retention.

`advance(before_slot)` is explicit. It rejects backwards movement, drops older state,
and returns pending/quarantined slots that were evicted. Older replay remains rejected
by the floor rather than being emitted again. The host must backfill or alert on gaps.

Retention is not a durable checkpoint. A process restart loses in-memory deduplication.
Persisting only the floor is safe only for data committed to the sink; advancing past
uncommitted data can lose it. A durable adapter must atomically persist delivery identity
with sink writes or use idempotent sink keys such as `(cluster, slot, blockhash)`.

Output is not globally slot-ordered, does not verify complete ancestry, and does not
distinguish missing from skipped slots. Those requirements need a separate chain-aware
consumer. No pre-seal prefix matching or rollback of speculative execution is attempted.

## Regression map

`tests/reconcile.rs`: quorum, reconnect vote identity, stale sessions, metadata conflicts,
commitment gating, retention, capacity, randomized event order and long-running retention.
`tests/upstream.rs`: connection-local bank buffers, differing bank ids for the same block,
reconnect cleanup, bank discard scope, capacity and malformed bank reuse.
`tests/replay.rs`: CLI agreement, conflict exit and malformed-input rejection.
