# Audit, 25 October 2026

Tags: MEASURED has a path or archived log beside it, INFERRED is reasoning from measured facts,
GUESS is neither. All Rust ran on the maintainer's Mac mini (Ironman); nothing was compiled elsewhere.

## Audited source identity

- Starting point: `7874365` on `main`, clean tree, three commits ahead of `origin/main`
  (`e8d74d6`, `6ea1bce`, `7874365`; all three exist in `git log`). `origin/main` is `7999f67`.
- Baseline: `scripts/verify.sh` at `7874365` exited 0 on Ironman before any change. Nothing was
  broken beforehand, so every failure below is a finding about coverage or claims, not a pre-existing
  red build. The baseline log was kept on Ironman (`~/yr-baseline-7874365.log`) and is not archived here.
- Final: the commits after `7874365` on `main`. `evidence/verification-audit.log` is the raw run of
  that tree and `evidence/source-manifest.sha256` (34 inputs) matches the hashes the Mini printed.

## Findings, by consequence

### 1. The SQLite sink stored forkable blocks (fixed)
`src/sink.rs` ignored `SealedBlock::commitment`. The watermark follows `parent_slot` and picks the
lowest child, so two non-finalized blocks sharing a parent could move it along a branch later
abandoned, and abandoned data would be stored as output. Reconciler can emit `Confirmed` blocks
when configured to.
Reproduced: `blocks_below_finalized_are_refused` failed (`evidence/audit-regressions-before-fix.log`).
Fix: `commit` returns `SinkError::NotFinalized` below `Finalized`. Decision: reject rather than
document, because the failure is silent and the check is one comparison.

### 2. A soak cannot outlive the wire guard, and the harness allowed it (fixed in the harness)
`WireGuard` ends a connection at `max_banks` and never evicts, and every example sets 8,192.
INFERRED: at the measured 212 ms per slot and one bank per slot that is about 1,737 s (29 min);
forks shorten it. `two_providers` and `pilot.py` accepted `RECONCILE_MAX_SECONDS` up to 3,600, so a
long run would have failed on `BankCapacity`, which looks like a provider fault. The proposal's
24-hour soak therefore depends on rotation that does not exist; the drafts said "after rotation"
but never gave the time.
Fix: bound lowered to 1,200 s in both, reason documented (`docs/PILOT.md`). Not fixed: rotation.
The capacity behaviour itself is MEASURED by `capacity_failure_preserves_old_bank_identity`; the
29-minute figure is arithmetic and no live run confirms it.

### 3. CI claims and workflows did not match each other (fixed in text, workflow unrun)
Codex's earlier commit replaced the only GitHub-hosted workflow with two manual workflows for a
self-hosted runner. `evidence/readiness-status-2026-10-25.json` records 0 registered runners, so
neither can run, and pushing would leave the repository with no runnable CI. The hosted runs that
did succeed covered default and upstream features only, so they never exercised the SQLite sink or
the wire guard, yet the earlier page and PDF said "CI passing".
Fix: `ci.yml` is hosted again (push and PR, read-only token, no secrets, safe for untrusted PRs),
covering default, `sqlite` and `upstream` features, Clippy and fixture exit codes; `verify.yml`
stays manual. The same commands passed on Ironman. **The hosted workflow has not run on GitHub.**

### 4. A failed pilot run said nothing about why (fixed)
`two_providers` printed no failure reason, discarded errors from `bridge.connect(..)?` by exiting
`main` before the summary, and `pilot.py` sent stderr to `/dev/null`; an unparseable child line
raised and wrote no report.
Reproduced (offline, two unreachable loopback endpoints, no credentials): report had
`passed: false`, connections `[0, 0]` and no reason (`audit-regressions-before-fix.log`).
Fix: summary and report now carry a sanitized `failure`; after the fix it reads
`provider 1 exhausted reconnect attempts` (`audit-regressions-after-fix.log`).
The malformed-child-output branch was written but **not exercised**; there is no offline way to
feed the harness a fake child without refactoring it.

### 5. The PublicNode failure could not be diagnosed afterwards (diagnostics fixed, cause open)
Details in `docs/PUBLICNODE-RESULT.md`. `MissingBankId` carried only the slot, so the status that
lacked a bank was never recorded. Reproduced: `missing_bank_id_names_the_status_that_lacked_it`
failed. Fix: the error carries the status; `access_probe` now reports statuses with and without a
bank id and the count of entries and transactions at `bank_id == 0`.
MEASURED from the pinned sources: only FIRST_SHRED_RECEIVED and COMPLETED are bankless by
protocol, the guard matches upstream's own documentation, and upstream **ignores** a bankless
CreatedBank or commitment status instead of ending the stream. The guard is stricter than upstream,
by design; it was not weakened. Which status failed, and whether the provider predates bank ids,
is **unknown**.

### 6. The sink did not check its schema version (fixed)
Reproduced: a file whose tables were correct but whose `user_version` was 99 opened normally
(`a_database_from_a_newer_schema_version_is_refused` failed). A *foreign* table was already
refused, by accident: `CREATE INDEX` errored. Fix: `PRAGMA user_version = 1`, higher refused.

### 7. The published application material contradicted the implementation (fixed locally)
The deployed PDF and project page still say USD 15,000, "CI passing", "never" for payload fusion
and restart, and present synthetic payload bytes as a memory ceiling. The earlier PDF table also
listed the USD 1,500 allowance twice (fixed in `ba3a7d0`). The PublicNode attempt appeared in no
grant document. Fixed in `grant/*.md`, regenerated PDF and the local site files. **The deployed
site and PDF are unchanged until a deploy is approved.**

### 8. Evidence was stale (fixed)
`source-manifest.sha256` described the tree before this audit. Regenerated and checked against the
Mini; the old manifest is kept as `source-manifest-pre-audit.sha256`, and `verification-current.log`
keeps its original meaning as the pre-audit run.

## Attempts that found nothing

`tests/audit_sequences.rs` (4 tests) tried: a provider that reconnects and changes its report,
retention passing a pending slot before the second vote, `u64::MAX` slots and an invalid
self-parent, and a quorum of one followed by a conflict. All behaved as the contract states, with
no core defect. Together with the existing suite this is **absence of evidence**, not proof: the
sequence list was mine, and the randomized test samples finitely many schedules.
Also checked, no defect: two writer threads on one file store every block exactly once.

## Unproven or unexamined

- PublicNode: which status failed; the provider's protocol version; whether any other plan works.
- The guard cannot tell an absent `bank_id` from `0` on entries, transactions and block metadata,
  because those fields are not optional on the wire. Whether any server sends real bank 0 is unknown.
- Frozen-before-commitment ordering in the bridge relies on upstream's state machine; only the
  existing reconstruction tests exercise it.
- Sink: power loss, disk-full and a corrupted file were not exercised. The mid-transaction abort
  test models the uncommitted state with raw SQL and does not call `commit`. Payload bytes are
  neither compared nor validated.
- The Ubuntu CI run; the live form's current field list (the page is script-rendered, so a plain
  fetch returned none; the 10-field list is Codex's 24 October DOM check); every hour estimate.
- Upstream: one GitHub search is not evidence that no equivalent exists.

## Design decisions on unsafe inputs

Rejected in the API: non-finalized blocks, a different block for a stored slot, slots above
`i64::MAX`, newer schema versions. Documented as caller preconditions: the anchor (the sink cannot
validate it), payload content, provider history reaching back to `resume_slot()`, and any external
effect (a primary key does not make one exactly-once).

## Dependency status (dated 25 October, GitHub API)

Upstream `main` is still `b828807`; issues #19 to #23 are open; PR #22 is unmerged at `0c732a0`;
no release. The pin is `main` plus PR #22's two commits and does not contain PR #20, so #19
(malformed-input panic) is **not fixed in the pin** and `WireGuard` is the only defence.
The pin was not changed.

## Changes to grant claims and scope

Claims removed or softened: "CI passing", "never" (payload fusion, restart), memory figures
presented as a ceiling, and any suggestion the free feed worked. Added: the PublicNode result
and the 29-minute guard lifetime. The request stayed USD 13,200 in this pass (superseded by 12,210 in the second pass), arithmetic rechecked (130 h x $90
+ $1,500); `grant/BUDGET.md` now shows the basis and says why the audit did not raise it. No
completed work is billed.

## How this audit went wrong, for the record

- I appended the first new tests to two Rust test files with a Python script, against my own rule
  of not editing Rust that way. Every other Rust edit used the editor.
- An example edit failed to compile (type inference) and was caught only when the pilot built it,
  because the targeted `cargo test` run does not build examples. The failing build is kept in the
  after-fix log.
- A test helper overflowed at `u64::MAX`; that was my test, not the library.
- No agents were spawned, no credentials were used, and nothing was pushed, deployed or sent.

## Second pass, same day

Sources are in `docs/RESEARCH-2026-10-25.md`.
One finding above is reversed: finding 3 restored an automatic GitHub-hosted build, which
conflicts with the maintainer's practice of building and testing Rust on their own machines. `ci.yml` is now
manual-only and there is no automatic CI (`docs/RUNNER-SETUP.md`). Also changed: real block payloads
are encoded with a count and tested through the sink (`tests/real_payload.rs`); the pilot
harness's malformed-output parsing has offline tests (`scripts/test_pilot.py`); the request is
US$12,210; published evidence has the Mini's address and account name replaced by placeholders
(raw copies kept privately; earlier commits in the pushed range and six files already public
still contain them, and history was not rewritten). Verification: `evidence/verification-final.log`.
Not run: any live check, since no valid credential exists. The mutation check of the payload
decoder was abandoned when the Mini ran out of disk copying its `target/` directory; I removed
my own copy only, and no result from that check is claimed.

Later the same day the request was rewritten to US$7,470 (83 hours, no provider allowance) because a live pilot is no longer planned; see `grant/BUDGET.md`.

## Pass 3, 26 October

One confirmed defect, found by a failing test before any change. `SqliteSink::open` set `PRAGMA journal_mode =
WAL` before reading `user_version`, so refusing a newer-schema file still switched it to WAL
(`evidence/pass2-regressions-before-fix.log`). Fixed by reading the version first; the test passes and the
full suite exits 0 (`evidence/verification-pass2.log`). Other candidates examined (event order between a
block and its commitment, commit-time capacity failure, quarantine and prune state, payload count bounds)
produced no defect; two are now pinned by tests. Separately, the deployed demo binary embeds the build
machine's account name and path; a path-remapped build is verified (`evidence/demo/`) but not deployed.
Not done: any live-provider check, and nothing was run on a second machine.

