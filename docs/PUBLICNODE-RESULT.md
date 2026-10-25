# PublicNode access check, 25 October 2026

MEASURED: `evidence/publicnode-access.log` records a real read-only test on Ironman using the
owner-supplied token and `https://solana-yellowstone-grpc.publicnode.com:443`. The diagnostic
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
as the owner planned. No account, subscription purchase, transaction submission or deployment
was performed by this test.
