# Existing tools and integration boundaries

Checked 26 October 2026. This is a scoped comparison, not a claim that no equivalent exists.

MEASURED ([Solana guidance](https://solana.com/upgrades/alpenglow)):
bank IDs are local to their connection; sealed blockhashes are used for cross-connection comparison.
The guidance separates per-connection reconstruction from cross-connection reconciliation.

| Tool | MEASURED source description | INFERRED relationship |
|---|---|---|
| [Yellowstone Block Machine](https://github.com/rpcpool/yellowstone-block-machine) | Block reconstruction and lifecycle/order handling for a stream | Dependency used separately for each connection |
| [Yellowstone client](https://github.com/rpcpool/yellowstone-grpc/blob/2a5486693a7ccf900ff7ccc2de1e6c6570899245/yellowstone-grpc-client/src/dedup.rs) | Duplicate/replay handling with metadata checks | Reconnect/deduplication is already available; that alone is not this project's differentiator |
| [Fumarole](https://blog.triton.one/yellowstone-fumarole-persistent-solana-streams-built-for-completeness-2/) | Provider describes persistent cursors, replay and aggregating data from multiple Solana nodes into one timeline (page updated 26 August 2026) | An alternative when the goal is a managed recoverable stream; the aggregation happens inside one provider |
| [Helius LaserStream](https://www.helius.dev/docs/laserstream) | Documents streaming from multiple aggregated nodes and automatic multi-node failover | Same boundary: one operator aggregates its own nodes. The page does not describe comparing data across nodes or reporting conflicts |
| [Alchemy streaming](https://www.alchemy.com/blog/zero-downtime-zero-gaps-solana-grpc-streaming) | Describes servers fed by several upstream nodes that de-duplicate updates and serve the fastest healthy result, with a replay buffer | Same boundary; no cross-operator comparison is described |
| [Carbon](https://github.com/sevenlabs-hq/carbon), [Shipstern](https://github.com/solana-rpc/shipstern) | Indexing and parsing toolkits | Possible consumer contexts, not evidenced adopters or partners |

MEASURED ([core](../src/lib.rs), [bridge](../src/upstream.rs)):
this prototype compares sealed metadata reported under stable provider identities, waits for its
configured agreement threshold and exposes disagreement. It selects one opaque payload, without
combining raw connection events or verifying ledger contents.

INFERRED: this policy may suit a consumer that already needs independent streams and explicitly
prefers waiting for agreement. Aggregating nodes inside one provider is not the same as comparing
independent operators: a consumer of a single provider cannot see that provider's aggregation being
wrong. A single authoritative stream with backups for liveness may be the better fit for other
consumers. More providers do not automatically improve availability or validity.

MEASURED ([upstream scope question](https://github.com/rpcpool/yellowstone-block-machine/issues/23)):
this repository's maintainer opened issue #23 to ask where this layer belongs; it has no comments.
No upstream acceptance is established. An upstream contribution or companion example may be a better
eventual home than a separate crate.
