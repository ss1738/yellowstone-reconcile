# PublicNode access check, 25 October 2026

MEASURED: `evidence/publicnode-access.log` records a real read-only test, run from the maintainer's build machine, using a
token I supplied and `https://solana-yellowstone-grpc.publicnode.com:443`. The diagnostic
source is `examples/access_probe.rs`. Endpoint/token values are not printed by the probe.

- Connection succeeded and the server accepted the subscription.
- Observed 20 transaction updates, 44 entries and 2 slot updates before termination.
- The wire guard reported `MissingBankId { slot: 461169824 }`.
- No block metadata/footer was observed before that early termination; zero blocks reconstructed.
- The probe exited nonzero. `evidence/publicnode-access-initial.log` preserves the first run,
  which similarly connected/subscribed but stopped before the guard reason was logged.

MEASURED (`src/wire.rs`): the guard requires a bank ID on CreatedBank, Processed, Confirmed
and Finalized slot updates. The observed failure means a bank-specific update lacked the
identity required by this consumer. It does not establish that all PublicNode subscriptions,
regions or plans omit this field, or that no footer would ever arrive if the stream continued.

INFERRED: this tested endpoint is not yet usable for our current reconstruction pilot.
Ask PublicNode whether the free feed can expose the bank-aware Yellowstone protocol, including
bank IDs on bank-specific slot updates and standalone block footers. A valid token and a
successful connection alone do not establish compatibility. Do not bypass the guard or invent
bank IDs to force this feed through.

MEASURED: no second provider was connected and no two-provider reconciliation was performed.
The token was used only via a private one-use wrapper outside the repository. Local cleanup
removed that wrapper; the remote wrapper removes itself on exit. Replace the token in Allnodes
as planned. No account, subscription purchase, transaction submission or deployment
was performed by this test.

## Audit of the failure (25 October 2026, no new network access)

Question: is the provider alone at fault? Answer from the archived evidence and the pinned
sources: this evidence cannot say, and the diagnostics were too thin to say.

MEASURED (`evidence/publicnode-access.log`): the probe counted 2 slot updates, the second of
which tripped the guard. The log records the slot, 461169824, but not the status. The earlier run
(`publicnode-access-initial.log`) recorded 3 slot updates and no reason at all. Which of
CreatedBank, Processed, Confirmed or Finalized lacked the bank id is therefore **unknown**.

MEASURED (pinned protocol, `yellowstone-grpc-proto-14.0.1/proto/geyser.proto`): `bank_id` on a
slot update is `optional`, and the comment says only FIRST_SHRED_RECEIVED and COMPLETED "do not
belong to any bank". The guard exempts those two and DEAD (upstream documents DEAD as bankless
when the bank was never created). Its requirement for the other four statuses matches the
upstream event documentation: "present for CreatedBank and the commitment statuses".

MEASURED (pinned upstream `0c732a0`): the reconstructor does not treat the same absence as
fatal. `state_machine.rs` logs "CreatedBank status ... carries no bank_id; ignoring" and
`wrapper.rs` does the same for a commitment status. Ignoring one is not unsafe, because a block
whose commitment update is dropped is never emitted; it costs liveness, not correctness. Our guard
ends the connection instead. That is stricter than upstream, deliberately: a server that omits
the field on bank-scoped statuses is probably not speaking the bank-aware protocol at all. This is
an inference, not a measurement, and the guard was not weakened to pass this feed.

Absence versus zero: the slot `bank_id` is `Option`, so `None` and `Some(0)` differ, and the guard
accepts `Some(0)` (`tests/wire.rs`). Entries, transactions, block metadata and footers carry a
plain `uint64` that an old server would leave at 0. The first run could not tell those cases apart.

Changes made: `WireError::MissingBankId` now carries the status, and `examples/access_probe.rs`
reports slot statuses split by bank-id present or absent plus the count of entries and
transactions with `bank_id == 0`. These are counts only, with no content or credentials.

GUESS: the free feed may run a Yellowstone version that predates bank ids. Nothing measured
confirms it. A fixed protocol version string is not exposed by the probe.

Next live check, not run because the earlier credential must be replaced by its holder first:

```sh
# On the build machine, with a NEW token in a private mode-600 file sourced into the shell
# (YELLOWSTONE_A_URL, YELLOWSTONE_A_TOKEN); never paste values into chat or arguments.
cd "$HOME/mini-work/yellowstone-reconcile" && export PATH="$HOME/.cargo/bin:$PATH"
cargo run --locked --release --features grpc --example access_probe
```

Read `reason` (now names the status), `slot_status_without_and_with_bank_id` and
`entries_and_transactions_with_bank_id_zero`. All-zero entries with bankless commitment statuses
indicate a pre-bank-id server. Only the commitment statuses lacking it would suggest a partial
implementation.
