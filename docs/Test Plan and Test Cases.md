# TEST PLAN AND TEST CASES

**Project:** NEAR Escrow
**Document Version:** 1.5 — Final
**Status:** Approved
**Date:** November 8, 2026
**Revision:** v1.4 from the fourth audit: AD-10 added to guard the storage bound, traceability ranges for F-04 and F-10 completed, totals now 138 / 102 P0 / 36 P1. v1.3 from the third audit: MS-08 added (a party conceding to the counterparty's percentage), F-02/F-06/F-08/F-11/F-13/F-14 and AD/INV traceability ranges extended to cover the newer tests, and totals now 137 / 102 P0. v1.2 from the second independent audit: error coverage matrix rebuilt (it still listed AC-04 under E-02, DS-03 under both E-10 and E-20, and omitted AP-05/AR-03/AB-09/MS-05/DS-09), INV-09 expects `disputes_released_fully = 0`, INV-05 scope limited to state transitions, SC-01 names the four client-only methods, AP-05 is now a boundary test, and three tests were added (CP-12 work-deadline floor, AD-09 `E-22`, INV-11 JSON-is-a-string) for 136 / 101 P0. v1.1: terminal-state tests now expect `E-08` (they previously mixed `E-02` with the `AlreadyFinalized` label), AC-04 expects `E-05`, DS-03 expects `E-20`, SC-08 expects `E-21`, DS-10 covers `E-07`, DS-11 covers the revision-deadline deadlock fix, INV-09 covers `get_stats`, INV-10 covers the `allocated` naming rule, and SC-10 points at UI-06. Totals after all revisions: **138 test cases, 102 P0, 36 P1**.
**Based on:** FSD v1.5 · STS v1.5 · SCTS v1.5 · SAS v1.5
**Network:** NEAR Testnet

---

# 1. Purpose and Scope

This document defines how NEAR Escrow is tested: what is verified, at which level, against which scenario, and with which expected result.

In scope:

1. Testing strategy and levels.
2. Environments, tooling and fixtures.
3. Requirement → test case traceability matrices.
4. Test case catalogue: contract (unit and integration), invariants, security/negative, and manual E2E for the demo.
5. Error code coverage matrix.
6. Exit criteria (definition of done) and execution order.

Out of scope: implementation details of each test (written during development) and the per-function frontend integration guide (document 06).

---

# 2. Testing Strategy

## 2.1 Principles

| # | Principle | Consequence |
|---|---|---|
| T-P1 | **Every business rule has a positive and a negative test** | A rule without a negative test is considered unfinished (FSD §9). |
| T-P2 | **Assert invariants after every scenario, not just at the end** | Fund accounting bugs surface on rarely exercised paths. |
| T-P3 | **Money is never tested implicitly** | Every test that moves funds asserts the balances of the affected parties, not only the status. |
| T-P4 | **Time is manipulated, never awaited** | All time-dependent tests use `context.block_timestamp` (unit) or sandbox time (integration). No `sleep`. |
| T-P5 | **A failing test must fail for the right reason** | Negative tests assert a **specific error code**, never just "it failed". |
| T-P6 | **The demo flow is tested as a script, not a hope** | The two-minute demo runs end-to-end at least twice before submission. |

## 2.2 Test Levels

| Level | Tooling | Coverage | Speed |
|---|---|---|---|
| L1 — Unit | `near-sdk` + `VMContextBuilder` | All guards, all error variants, all state transitions, fund arithmetic | Milliseconds |
| L2 — Integration | `near-workspaces` (sandbox node) | Real deploy, real transactions, async transfers and callbacks, real wallet balances | Seconds |
| L3 — Invariant | In-contract assertions + test helpers | `total_locked + total_claimable <= spendable`, `f1 + c1 == amount` | Milliseconds |
| L4 — Manual E2E | Browser + testnet wallet | UI flows, button eligibility, error toasts, two-step withdraw, demo rehearsal | Minutes |

L1 and L3 run on every commit (CI). L2 runs on every pull request. L4 runs before recording the demo and before submission.

## 2.3 Priority

- **P0 / Mandatory** — must pass before submission. Marked `[P0]`.
- **P1 / Extended** — increases confidence, may be cut if time is short. Marked `[P1]`.

Catalogue total: **138 test cases**, consisting of **102 P0** and **36 P1**.

The P0 count exceeds the volume estimate in SCTS §13 (60–80 tests) because every time-boundary rule is split into three test points (`deadline - 1s`, `deadline`, `deadline + 1s`). If time is short, follow the group priority in §7.4 and complete CP, AC, SD, AP and WD first — that set already covers the demo flow and the most critical money rules.

---

# 3. Environments and Fixtures

## 3.1 Environments

| Environment | Contract | Notes |
|---|---|---|
| `unit` | In-memory, `VMContextBuilder` | No node, fast, used by CI |
| `sandbox` | `near-workspaces` (local node) | Used for async flows (`withdraw`) |
| `testnet-staging` | `escrow-staging.<account>.testnet` | For manual L4; may be reset at any time |
| `testnet-demo` | `escrow.<account>.testnet` | The contract used during the demo; rehearsal and demo only |

Rule: automated tests **must never** touch `testnet-demo`. The demo contract is kept clean.

## 3.2 Fixture Accounts

| Alias | Role | Used in |
|---|---|---|
| `alice.testnet` | Client | Almost every test |
| `bob.testnet` | Freelancer | Almost every test |
| `carol.testnet` | Arbiter | Almost every test |
| `dave.testnet` | Intruder (unauthorized) | Negative tests |
| `eve.testnet` | Unrelated public caller | Auto-release tests |
| `owner.testnet` | Contract owner | Admin tests |

Recommended: a `setup() -> (Contract, Context)` helper returning an initialised contract with `alice` as predecessor, so no test repeats boilerplate.

## 3.3 Required Test Helpers

```rust
fn setup() -> Contract                                    // fresh contract, owner.testnet
fn as_(account: &str)                                     // set predecessor
fn at_(ns: u64)                                           // set block_timestamp
fn create_project_default() -> u64                        // 10 NEAR, deadline +7 days
fn accept(id: u64) / deliver(id: u64)                     // state shortcuts
fn assert_claimable(account: &str, expected: u128)
fn assert_solvent(c: &Contract)                           // fund invariant
fn expect_err(result: Result<_, ContractError>, code: ContractError)
```

## 3.4 Test Constants

```
AMOUNT         = 10 NEAR = 10_000_000_000_000_000_000_000_000 yocto
MIN_ESCROW     = 0.1 NEAR
ACCEPT_WINDOW  = 48 hours
REVIEW_WINDOW  = 72 hours
REVISION_EXT   = 3 days
ARBITER_WINDOW = 7 days
MAX_REVISIONS  = 2
```

---

# 4. Traceability Matrices

## 4.1 FSD Feature → Test Case

| FSD | Feature | Primary test cases |
|---|---|---|
| F-01 | Wallet Connection | UI-01, UI-02, UI-03 |
| F-02 | Create Project | CP-01 … CP-12 |
| F-03 | Fund Escrow | CP-01, CP-05, INV-01 |
| F-04 | Accept Project | AC-01, AC-02, AC-05, AC-06, AC-07 |
| F-05 | Reject Project | AC-03, AC-04 |
| F-06 | Submit Deliverable | SD-01 … SD-09 |
| F-07 | Approve Deliverable | AP-01 … AP-06 |
| F-08 | Request Revision | RV-01 … RV-08 |
| F-09 | Automatic Release | AR-01 … AR-05 |
| F-10 | Claim Refund | RF-01 … RF-08 |
| F-11 | Open Dispute | DS-01 … DS-11 |
| F-12 | Submit Evidence | DS-05, DS-06, DS-07, DS-08 |
| F-13 | Arbiter Resolution | AB-01 … AB-09 |
| F-14 | Mutual Settlement | MS-01 … MS-08 |
| F-15 | Project Dashboard | UI-04 … UI-07 |
| P1-01 | AI Dispute Summary | UI-11, UI-12 |
| (SCTS) | Withdrawal / pull payment | WD-01 … WD-09 |
| (SCTS) | Admin and reserve | AD-01 … AD-10 |
| (SCTS) | Fund invariants | INV-01 … INV-11 |

## 4.2 STS Transition → Test Case

| Transition | Test case |
|---|---|
| 5.1 FUNDED → ACCEPTED | AC-01 |
| 5.2A FUNDED → REFUNDED (reject) | AC-03 |
| 5.2B FUNDED → REFUNDED (48h silence) | RF-01 |
| 5.3 ACCEPTED → DELIVERED | SD-01 |
| 5.4 ACCEPTED → REFUNDED (deadline) | RF-02 |
| 5.5 DELIVERED → RELEASED (approve) | AP-01 |
| 5.6 DELIVERED → REVISION_REQUESTED | RV-01 |
| 5.7 DELIVERED → DISPUTED | DS-01 |
| 5.8 DELIVERED → RELEASED (automatic) | AR-01 |
| 5.9 REVISION_REQUESTED → DELIVERED | RV-03 |
| 5.10 REVISION_REQUESTED → DISPUTED | DS-02 |
| 5.11 DISPUTED → RESOLVED (arbiter) | AB-01 |
| 5.12 DISPUTED → RESOLVED (mutual) | MS-01 |

---

# 5. Test Case Catalogue

The **Ref** column points to FSD (`F-xx`), STS (`5.x`), an error code (`E-xx`), or an invariant (`INV`).

## 5.1 CP — Create Project (F-02, F-03)

| ID | Scenario | Precondition | Action | Expected result | Ref |
|---|---|---|---|---|---|
| CP-01 `[P0]` | Normal project creation | alice connected, reserve sufficient | `create_project(bob, carol, "Logo", "Logo design", now+7d)` with 10 NEAR | `Ok(id=0)`; status `Funded`; `amount = 10 NEAR`; `total_locked += 10`; alice/bob/carol indexes updated; event `project_created` | F-02, F-03 |
| CP-02 `[P0]` | Deposit below minimum | — | deposit 0.05 NEAR | `Err(E-04)`; no project created; `total_locked` unchanged | E-04 |
| CP-03 `[P0]` | Zero deposit | — | deposit 0 | `Err(E-04)` | E-04 |
| CP-04 `[P0]` | Freelancer equals client | — | `freelancer = alice` | `Err(E-04 InvalidParticipants)` | F-02 |
| CP-05 `[P0]` | Arbiter equals client | — | `arbiter = alice` | `Err(E-04 InvalidParticipants)` | F-02 |
| CP-06 `[P0]` | Freelancer equals arbiter | — | `arbiter = bob` | `Err(E-04 InvalidParticipants)` | F-02 |
| CP-07 `[P0]` | Deadline in the past | — | `work_deadline = now - 1h` | `Err(E-03 DeadlineExpired)` | F-02 |
| CP-08 `[P1]` | Title or description too long | — | title 129 bytes / description 2001 bytes | `Err(InvalidInput)`; limits of 128/2000 enforced | F-02 |
| CP-09 `[P1]` | Creation while paused | owner called `set_paused(true)` | `create_project(...)` | `Err(E-15 ContractPaused)`; no state changed | E-15 |
| CP-10 `[P1]` | IDs are sequential and unique | create 3 projects | — | ids 0, 1, 2; `get_project_count() == 3` | F-02 |
| CP-11 `[P1]` | Deposit exceeds the required amount | — | deposit 0.2 NEAR | No refund of the excess; `amount == 0.2 NEAR` (SCTS D-06) | D-06 |
| CP-12 `[P0]` | Work deadline inside the acceptance window | — | `work_deadline = now + 30h` (window is 48h) | `Err(E-03)`; no project created. Prevents a project that could be accepted but never delivered (SCTS D-20) | SCTS D-20 |

## 5.2 AC — Accept / Reject (F-04, F-05)

| ID | Scenario | Precondition | Action | Expected result | Ref |
|---|---|---|---|---|---|
| AC-01 `[P0]` | Freelancer accepts | `Funded`, 10 minutes after funding | bob calls `accept_project` | `Ok`; status `Accepted`; event `project_accepted` | STS 5.1 |
| AC-02 `[P0]` | Non-freelancer attempts to accept | `Funded` | dave calls `accept_project` | `Err(E-01 Unauthorized)`; status stays `Funded` | E-01 |
| AC-03 `[P0]` | Freelancer rejects | `Funded`, within 48h | bob calls `reject_project` | status `Refunded`; `claimable[alice] += 10 NEAR`; event `settled` | STS 5.2A |
| AC-04 `[P0]` | Reject after acceptance | status `Accepted` | bob calls `reject_project` | `Err(E-05 AlreadyAccepted)`; status unchanged | E-05 |
| AC-05 `[P0]` | Accept after 48 hours | `now = created_at + 48h + 1s` | bob calls `accept_project` | `Err(E-03 DeadlineExpired)`; status stays `Funded` | STS 5.1 |
| AC-06 `[P0]` | Accept exactly at the 48h boundary | `now == created_at + 48h` | bob calls `accept_project` | `Err(E-03)` — a deadline is expired when `now >= deadline` | STS 6.1 |
| AC-07 `[P1]` | Client attempts to accept their own project | `Funded` | alice calls `accept_project` | `Err(E-01)` | E-01 |

## 5.3 SD — Submit Deliverable (F-06)

| ID | Scenario | Precondition | Action | Expected result | Ref |
|---|---|---|---|---|---|
| SD-01 `[P0]` | Normal submission | `Accepted`, before deadline | bob calls `submit_deliverable(link, note)` | `Ok`; status `Delivered`; `review_deadline = now + 72h`; `revision_index = 0` | STS 5.3 |
| SD-02 `[P0]` | Submission without a link | — | `link = ""` | `Err(InvalidDeliverable)` | E-10 |
| SD-03 `[P0]` | Submission without a note | — | `note = ""` | `Ok` — the note is optional | F-06 |
| SD-04 `[P0]` | Submission after the work deadline | `now = work_deadline + 1s` | bob submits | `Err(E-03 DeadlineExpired)` | E-03 |
| SD-05 `[P0]` | Submission in the wrong state | status `Funded` | bob submits | `Err(E-02 InvalidProjectState)` | E-02 |
| SD-06 `[P0]` | Non-freelancer submits | `Accepted` | dave submits | `Err(E-01)` | E-01 |
| SD-07 `[P1]` | Link too long | — | link of 513 bytes | `Err(InvalidDeliverable)` | E-10 |
| SD-08 `[P1]` | Double submission in `Accepted` | after the first submission | bob submits again | `Err(E-02)` — status is already `Delivered` | E-02 |
| SD-09 `[P1]` | `review_deadline` resets on every revision submission | see RV-03 | inspect the value | New `review_deadline = now + 72h`, never the previous value | STS 5.9 |

## 5.4 RV — Request Revision (F-08)

| ID | Scenario | Precondition | Action | Expected result | Ref |
|---|---|---|---|---|---|
| RV-01 `[P0]` | First revision | `Delivered`, within 72h | alice calls `request_revision` | status `RevisionRequested`; `revision_count = 1`; `work_deadline = now + 3 days`; `review_deadline = 0` | STS 5.6 |
| RV-02 `[P0]` | Revision when two are already used | `revision_count = 2`, `Delivered` | alice calls `request_revision` | `Err(E-06 RevisionLimitExceeded)` | E-06 |
| RV-03 `[P0]` | Submit the revised deliverable | `RevisionRequested` | bob submits | status `Delivered`; `revision_index = 1`; new `review_deadline`; `revision_deadline = 0` | STS 5.9 |
| RV-04 `[P0]` | Revision after the review period expired | `now = review_deadline + 1s` | alice calls `request_revision` | `Err(E-03 DeadlineExpired)` | E-03 |
| RV-05 `[P0]` | Non-client requests a revision | `Delivered` | bob calls `request_revision` | `Err(E-01)` | E-01 |
| RV-06 `[P1]` | Extension is relative to `now`, not the previous deadline | `Delivered` on the third day of review | alice calls `request_revision` | `work_deadline = now + 3 days` (not `old_work_deadline + 3 days`) | SCTS D-09 |
| RV-07 `[P1]` | Full two-revision cycle | — | revision → submit → revision → submit | Both revisions succeed; `revision_count = 2`; final status `Delivered` | F-08 |
| RV-08 `[P0]` | Revision in the wrong state | `Accepted` | alice calls `request_revision` | `Err(E-02)` | E-02 |

## 5.5 AP — Approve Deliverable (F-07)

| ID | Scenario | Precondition | Action | Expected result | Ref |
|---|---|---|---|---|---|
| AP-01 `[P0]` | Normal approval | `Delivered`, within 72h | alice calls `approve_deliverable` | status `Released`; `claimable[bob] = 10 NEAR`; `claimable[alice] = 0`; `distribution {f:10, c:0}`; event `settled` | STS 5.5 |
| AP-02 `[P0]` | Non-client approves | `Delivered` | dave calls `approve_deliverable` | `Err(E-01)` | E-01 |
| AP-03 `[P0]` | Approval in the wrong state | `Accepted` | alice approves | `Err(E-02)` | E-02 |
| AP-04 `[P0]` | Double approval | already `Released` | alice approves again | `Err(E-08 AlreadyFinalized)`; `claimable[bob]` unchanged | E-08 |
| AP-05 `[P0]` | Approval exactly at the review boundary | `now == review_deadline` | alice approves | `Err(E-03)` — validity requires `now < review_deadline`; the boundary is already expired | E-03 |
| AP-06 `[P0]` | Contract balance after approval | `Released`, before withdrawal | call `get_fund_invariants()` | `locked` drops by 10, `claimable` rises by 10, `balance` unchanged | INV-02 |

## 5.6 AR — Automatic Release (F-09)

| ID | Scenario | Precondition | Action | Expected result | Ref |
|---|---|---|---|---|---|
| AR-01 `[P0]` | Auto release after 72 hours | `Delivered`, `now >= review_deadline` | eve (unrelated account) calls `release_after_review` | status `Released`; `claimable[bob] = 10 NEAR`; eve receives nothing | STS 5.8 |
| AR-02 `[P0]` | Auto release too early | `now = review_deadline - 1s` | eve calls `release_after_review` | `Err(E-11 ReviewPeriodNotExpired)` | E-11 |
| AR-03 `[P0]` | Auto release while disputed | status `Disputed` | eve calls `release_after_review` | `Err(E-02)` | INV 7.1.5 |
| AR-04 `[P1]` | Client triggers auto release themselves | `now >= review_deadline` | alice calls `release_after_review` | `Ok`; identical outcome, funds still go to bob | SCTS §9 rule 4 |
| AR-05 `[P1]` | Auto release exactly at the boundary | `now == review_deadline` | eve calls | `Ok` — expired when `>=` | STS 6.1 |

## 5.7 RF — Refund (F-10)

| ID | Scenario | Precondition | Action | Expected result | Ref |
|---|---|---|---|---|---|
| RF-01 `[P0]` | Refund because the freelancer was silent for 48h | `Funded`, `now > accept_deadline` | alice calls `claim_no_response_refund` | status `Refunded`; `claimable[alice] = 10 NEAR` | STS 5.2B |
| RF-02 `[P0]` | Refund because the initial deliverable was never submitted | `Accepted`, `now > work_deadline`, never submitted | alice calls `claim_deadline_refund` | status `Refunded`; `claimable[alice] = 10 NEAR` | STS 5.4 |
| RF-03 `[P0]` | Refund claimed too early | `Funded`, `now < accept_deadline` | alice calls `claim_no_response_refund` | `Err(E-03 DeadlineExpired)` | E-03 |
| RF-04 `[P0]` | Refund while the work deadline is still running | `Accepted`, before deadline | alice calls `claim_deadline_refund` | `Err(E-03)` | E-03 |
| RF-05 `[P0]` | Refund after a deliverable was submitted | status `Delivered` | alice calls `claim_deadline_refund` | `Err(E-02)` — the refund path is closed | STS 5.4 note |
| RF-06 `[P0]` | Non-client claims a refund | `Funded`, past 48h | bob calls `claim_no_response_refund` | `Err(E-01)` | E-01 |
| RF-07 `[P1]` | Missing a revision deadline does NOT grant a refund | `RevisionRequested`, `now > revision_deadline` | alice calls `claim_deadline_refund` | `Err(E-02)`; the only remaining path is `open_dispute` | STS 5.10 note |
| RF-08 `[P0]` | Double refund | already `Refunded` | alice refunds again | `Err(E-08 AlreadyFinalized)`; balance unchanged | E-08 |

## 5.8 DS — Dispute and Evidence (F-11, F-12)

| ID | Scenario | Precondition | Action | Expected result | Ref |
|---|---|---|---|---|---|
| DS-01 `[P0]` | Client opens a dispute | `Delivered`, within review | alice calls `open_dispute` | status `Disputed`; `dispute_opened_at = now`; `review_deadline = 0` | STS 5.7 |
| DS-02 `[P0]` | Freelancer opens a dispute during revision | `RevisionRequested`, within revision deadline | bob calls `open_dispute` | status `Disputed`; `revision_deadline = 0` | STS 5.10 |
| DS-03 `[P0]` | Dispute before any deliverable | `Accepted` | alice calls `open_dispute` | `Err(E-20 DisputeNotAllowed)` | F-11 |
| DS-04 `[P0]` | Dispute after the review period expired | `Delivered`, `now > review_deadline` | alice calls `open_dispute` | `Err(E-03)` | STS 5.7 |
| DS-05 `[P0]` | Both parties submit evidence | `Disputed` | alice and bob call `submit_evidence` | Both fields populated; status stays `Disputed`; event `evidence_submitted` twice | F-12 |
| DS-06 `[P0]` | Same party submits evidence twice | `Disputed`, alice already submitted | alice submits again | `Err(E-10 EvidenceAlreadySubmitted)`; the original evidence is unchanged | E-10 |
| DS-07 `[P0]` | A non-party submits evidence | `Disputed` | dave calls `submit_evidence` | `Err(E-01)` | E-01 |
| DS-08 `[P1]` | Evidence is stored on the correct project | two `Disputed` projects | submit evidence on project A | Evidence appears only on project A; project B remains empty | INV 8.9 |
| DS-09 `[P0]` | Dispute in a terminal state | `Released` | alice calls `open_dispute` | `Err(E-08 AlreadyFinalized)` | INV 7.1.6 |
| DS-10 `[P0]` | Dispute opened twice | `Disputed` | alice calls `open_dispute` again | `Err(E-07 DisputeAlreadyActive)`; `dispute_opened_at` unchanged | E-07 |
| DS-11 `[P0]` | Dispute opened after the revision deadline passed | `RevisionRequested`, `now > revision_deadline`, freelancer never resubmitted | alice calls `open_dispute` | `Ok` — status `Disputed`; the project is no longer deadlocked; arbiter can now resolve | STS D-01, RF-07 |

## 5.9 AB — Arbiter Resolution (F-13)

| ID | Scenario | Precondition | Action | Expected result | Ref |
|---|---|---|---|---|---|
| AB-01 `[P0]` | Arbiter decides 70/30 | `Disputed` | carol calls `arbiter_resolve(id, 70)` | status `Resolved`; `claimable[bob] = 7 NEAR`; `claimable[alice] = 3 NEAR`; `resolution.resolved_by = Arbiter` | STS 5.11 |
| AB-02 `[P0]` | Percentage above 100 | `Disputed` | carol calls `arbiter_resolve(id, 101)` | `Err(E-09 InvalidSettlementPercentage)`; status stays `Disputed` | E-09 |
| AB-03 `[P0]` | Non-arbiter decides | `Disputed` | dave calls `arbiter_resolve(id, 50)` | `Err(E-01)` | E-01 |
| AB-04 `[P0]` | Arbiter decides on a project outside their authority | `Disputed` on another project | a different arbiter decides | `Err(E-01)` | E-01 |
| AB-05 `[P0]` | Arbiter decides after 7 days | `now = dispute_opened_at + 7d + 1s` | carol resolves | `Err(E-13 ResolutionWindowExpired)`; funds stay in `Disputed` | SCTS D-03 |
| AB-06 `[P0]` | Arbiter decides exactly at the 7-day boundary | `now == dispute_opened_at + 7d` | carol resolves | `Ok` (inclusive boundary per SCTS §6.2, M-12) | SCTS D-03 |
| AB-07 `[P0]` | Decision is immutable after settlement | already `Resolved` | carol resolves again with 100 | `Err(E-08 AlreadyFinalized)`; distribution unchanged | INV 7.2.5 |
| AB-08 `[P1]` | 0% and 100% extremes | two `Disputed` projects | resolve 0 and 100 | 0% → everything to the client, no `claimable[bob]` entry; 100% → everything to bob | F-13 |
| AB-09 `[P1]` | Call in the wrong state | `Delivered` | carol resolves | `Err(E-02)` | E-02 |

## 5.10 MS — Mutual Settlement (F-14)

| ID | Scenario | Precondition | Action | Expected result | Ref |
|---|---|---|---|---|---|
| MS-01 `[P0]` | Both parties agree on 60/40 | `Disputed` | alice `propose_settlement(60)`, bob `settle_mutual(60)` | status `Resolved`; `claimable[bob] = 6 NEAR`; `claimable[alice] = 4 NEAR`; `resolved_by = MutualSettlement` | STS 5.12 |
| MS-02 `[P0]` | Mismatched percentages | `Disputed` | alice proposes 60, bob calls `settle_mutual(70)` | `Err(E-16 SettlementMismatch)`; status stays `Disputed`; nothing allocated | F-14 |
| MS-03 `[P0]` | One party cannot count as both approvals | `Disputed` | alice `propose_settlement(50)` then alice `settle_mutual(50)` | `Err(E-16)`; the counterparty must approve | F-14 rule 3 |
| MS-04 `[P0]` | Proposal can be updated before settling | `Disputed` | alice proposes 40, then 55; bob settles 55 | `Ok`; the latest proposal applies | F-14 rule 5 |
| MS-05 `[P1]` | Proposal outside `Disputed` | `Delivered` | alice calls `propose_settlement(50)` | `Err(E-02)` | SCTS D-07 |
| MS-06 `[P1]` | Settlement after an arbiter decision | `Resolved` by arbiter | alice calls `settle_mutual(50)` | `Err(E-08 AlreadyFinalized)` | INV 7.2.5 |
| MS-07 `[P1]` | Arbiter inactive for 7 days, funds stay locked | `Disputed`, `now > +7d`, no proposals | inspect state | `Disputed`; `total_locked` still holds the amount; no automatic 50/50 | STS 6.2 |
| MS-08 `[P0]` | A party accepts the counterparty's percentage, superseding its own | `Disputed`; alice proposed 60, bob proposed 70 | bob calls `settle_mutual(60)` | `Ok` — status `Resolved` at 60/40. Bob's own stored 70 is superseded by the call itself; only alice's 60 is compared (SCTS D-21) | FSD §4.9 rule 4, SCTS D-21 |

## 5.11 WD — Withdrawal / Pull Payment

| ID | Scenario | Precondition | Action | Expected result | Ref |
|---|---|---|---|---|---|
| WD-01 `[P0]` | Freelancer withdraws after `Released` | `Released`, `claimable[bob] = 10` | bob calls `withdraw()` with 30 Tgas | bob's wallet balance +10 NEAR (minus gas); `claimable[bob] = 0`; `total_claimable = 0`; event `withdraw_succeeded` | STS §9 |
| WD-02 `[P0]` | Double withdrawal | after WD-01 | bob calls `withdraw()` again | `Err(E-18 NothingToWithdraw)`; no transfer | INV 7.2.3 |
| WD-03 `[P0]` | Withdrawal with no balance | account without claimable | dave calls `withdraw()` | `Err(E-18)` | E-18 |
| WD-04 `[P0]` | Both parties withdraw after `Resolved` | 70/30 split | alice and bob both withdraw | Each receives exactly their share; the sum equals the escrow amount | STS 9.2 |
| WD-05 `[P0]` | Insufficient gas | `claimable > 0`, only 10 Tgas attached | bob calls `withdraw()` | Transaction fails; `claimable[bob]` remains intact and retryable | SCTS §8.4 |
| WD-06 `[P1]` | Concurrent withdrawals (two parallel transactions) | `claimable[bob] = 10` | two `withdraw()` calls nearly simultaneous | One succeeds, the other returns `Err(E-19)` or `E-18`; total transferred is 10, not 20 | INV 7.2.3 |
| WD-07 `[P1]` | Failed transfer restores the balance | force a failure (contract lacks free balance) | bob calls `withdraw()` | Event `withdraw_failed`; `claimable[bob]` restored to 10; retry possible | STS 9.3.5 |
| WD-08 `[P1]` | Withdrawal after a refund | `Refunded` | alice calls `withdraw()` | alice receives 10 NEAR | STS 9.2 |
| WD-09 `[P1]` | In-progress flag is cleared | callback succeeded | call `get_withdrawal_state(bob)` | `None` after the callback | SCTS §8.4 |

## 5.12 AD — Admin, Reserve and Pause

| ID | Scenario | Precondition | Action | Expected result | Ref |
|---|---|---|---|---|---|
| AD-01 `[P0]` | Insufficient reserve | drain the reserve below the requirement | alice calls `create_project` | `Err(E-14 InsufficientReserve)`; no escrow funds consumed | SCTS D-01 |
| AD-02 `[P0]` | Top up the reserve | — | dave calls `top_up_reserve` with 5 NEAR | `Ok`; event `reserve_topped_up`; `get_fund_invariants().ok == true` | SCTS M-17 |
| AD-03 `[P0]` | Pause blocks creation only | `paused = true` | (a) alice creates, (b) bob accepts, (c) alice approves, (d) bob withdraws | (a) `E-15`; (b)(c)(d) `Ok` — funds can still exit | SCTS D-02 |
| AD-04 `[P0]` | Non-owner pauses | — | alice calls `set_paused(true)` | `Err(NotOwner)` | E-01 |
| AD-05 `[P0]` | `clear_stuck_withdrawal` before the grace period | a withdrawal in progress for 10 minutes | owner clears | `Err(E-17 GraceNotElapsed)` | SCTS D-05 |
| AD-06 `[P0]` | `clear_stuck_withdrawal` after the grace period | a withdrawal in progress for over an hour | owner clears | `claimable` restored; event `withdrawal_cleared` | SCTS D-05 |
| AD-07 `[P1]` | `migrate` is not public | — | an external account calls `migrate` | Rejected (private method) | SCTS §14 |
| AD-08 `[P1]` | `get_fund_invariants` is accurate | three projects in different states | call the view | `locked + claimable + storage_stake <= balance`; figures match a manual computation | INV-01 |
| AD-09 `[P0]` | `clear_stuck_withdrawal` with no record | no withdrawal in progress for the account | owner calls `clear_stuck_withdrawal(dave)` | `Err(E-22 NoWithdrawalInProgress)` — never a silent success (SCTS D-19) | E-22 |
| AD-10 `[P1]` | Storage bound is still valid | deploy, then create one project with maximum-length strings | measure `storage_usage()` before and after | The delta is **below** `MAX_PROJECT_STORAGE_BYTES = 6_000`. If it exceeds the constant, the reserve check under-estimates and a shortfall escapes as a panic instead of a typed error — this test is what keeps D-23 honest instead of a magic number nobody re-checks | SCTS D-23 |

## 5.13 INV — Fund and State Invariants

| ID | Scenario | Action | Expected result | Ref |
|---|---|---|---|---|
| INV-01 `[P0]` | Solvency after creation | create 5 projects | `balance - storage_stake >= locked + claimable` | SCTS §5.1 |
| INV-02 `[P0]` | Solvency after settlement | approve one project | Invariant still holds; `locked` drops and `claimable` rises by the amount | SCTS §5.1 |
| INV-03 `[P0]` | Distribution sums exactly | resolve 33% on 10 NEAR | `3.3 + 6.7 == 10 NEAR`; the freelancer gets the floor, the client gets the remainder | SCTS D-08 |
| INV-04 `[P0]` | Rounding on awkward amounts | 1 NEAR split at 3% (0.03 NEAR) | `f + c == amount`; no yocto created or lost | SCTS D-08 |
| INV-05 `[P0]` | Terminal status is immutable | after `Released`, invoke every **project-state** transition (accept, reject, submit, approve, revise, dispute, evidence, resolve, settle, both refunds) | All rejected with E-08/E-02; status stays `Released`; `distribution` unchanged. `top_up_reserve` and `withdraw` are **excluded** — they are balance operations and must still succeed on a terminal project | INV 7.1.2 |
| INV-06 `[P0]` | Exactly one status at a time | every transition | Status is always exactly one valid enum value | INV 7.1.1 |
| INV-07 `[P1]` | Random chains stay solvent | 200 random valid actions | Invariant holds at every step; no panic | SCTS §12.5 |
| INV-08 `[P1]` | All terminal projects reconcile with the ledger | random walk, then sum | `Σ distribution == Σ amount` for terminal projects; `total_locked == Σ amount` for non-terminal ones | INV 7.2 |
| INV-09 `[P0]` | `get_stats` matches the project history | as bob: 2 released + 1 resolved at 70% + 1 disputed at 0%; as alice: 1 refunded | call `get_stats` for both | bob: `completed_as_freelancer = 3` (two released + the 70% share, which is above zero; the 0% one is not completed), `total_value_as_freelancer` = Σ `distribution.freelancer_amount` **allocated** (not withdrawn), `disputes_as_freelancer = 2`, `disputes_released_fully = 0` (the definition is `percent == 100`; 70% and 0% do not count — this is exactly the assertion that catches an off-by-definition stat); alice: `refunds_as_client = 1` | SCTS §6.3 |
| INV-10 `[P1]` | `get_claimable_per_project` does not shrink after a withdrawal | `Released` project, `claimable = 10`, then withdraw | call `get_claimable_per_project` before and after | `allocated` stays 10 for that project (historical); `get_claimable` goes 10 → 0. Confirms the naming rule in SCTS §6.3 | SCTS §6.3 |
| INV-11 `[P0]` | Amounts and timestamps cross the JSON boundary as strings | one project, `amount = 10 NEAR` | inspect the raw `get_project` JSON (`serde_json::to_string`) | `amount` is a **string** (`"10000000000000000000000000"`), every timestamp is a string, and `JSON.parse` + `BigInt` round-trips exactly. Guards SCTS D-15 — a Rust-side `u128` comparison passes while the browser would be truncated | SCTS §4.3.1 |

## 5.14 SC — Security and Negative Tests

| ID | Scenario | Action | Expected result | Ref |
|---|---|---|---|---|
| SC-01 `[P0]` | Intruder attempts every client-only action | dave calls `approve_deliverable`, `request_revision`, `claim_no_response_refund`, `claim_deadline_refund` | All return `Err(E-01)`. Party-shared methods are covered by DS-07 and SC-02 | FSD §8.1 |
| SC-02 `[P0]` | Intruder attempts every freelancer action | dave calls the freelancer methods | All return `Err(E-01)` | FSD §8.1 |
| SC-03 `[P0]` | Intruder attempts an arbiter action | dave calls `arbiter_resolve` | `Err(E-01)` | FSD §8.1 |
| SC-04 `[P0]` | Replace the arbiter after acceptance | search for any method or field write | No such method; the arbiter never changes | INV 7.3.5 |
| SC-05 `[P0]` | Replace the freelancer after acceptance | same | No such method; the freelancer never changes | FSD §4.1 rule 8, §8 item 8 |
| SC-06 `[P0]` | Double spending via replay | submit the same transaction twice | The second fails on nonce/status | INV 7.2.3 |
| SC-07 `[P0]` | Withdraw reentrancy | two withdrawal calls without waiting for the callback | The second returns `Err(E-19 WithdrawInProgress)` | SCTS P-8 |
| SC-08 `[P1]` | Unknown project | `get_project(9999)`, `accept_project(9999)` | View returns `None`; call returns `Err(E-21 UnknownProject)` | E-21 |
| SC-09 `[P1]` | Amounts never overflow or go negative | amounts near `u128::MAX` | `Err`/controlled panic; no wraparound | SCTS P-7 |
| SC-10 `[P1]` | On-chain strings are never executed | set the title to `<script>` | The UI renders it as text (verified manually in UI-06) | SAS S-5 |

## 5.15 UI — Manual E2E and Demo Rehearsal

| ID | Scenario | Steps | Expected result | Ref |
|---|---|---|---|---|
| UI-01 `[P0]` | Connect wallet | open the app, click Connect, choose MyNearWallet | Account ID shown in the header; button becomes Disconnect | F-01 |
| UI-02 `[P0]` | Disconnect | click Disconnect | Header returns to Connect; query cache cleared | F-01 |
| UI-03 `[P0]` | Switch accounts in the wallet | switch account in the extension | All data refetches; the project list follows the new account | SAS §7.1 |
| UI-04 `[P0]` | Create a project from the UI | complete the form and submit | Wallet prompt shows `receiverId` = the contract and the correct deposit; redirect to detail; status `Funded` | F-02, F-03 |
| UI-05 `[P0]` | Role-scoped dashboard | open `/dashboard` as alice, then as bob | alice sees the Client tab, bob sees the Freelancer tab, with correct contents | F-15 |
| UI-06 `[P0]` | Read-only project detail | open a project link without connecting | Data renders; action buttons prompt a connect | F-15 |
| UI-07 `[P0]` | Button eligibility from status × role × time | open a project as the client in every status | Only valid actions render; invalid buttons disappear with an explanation | SAS §10.2 |
| UI-08 `[P0]` | Two-step withdrawal is explained | after approval | UI shows "allocated, click Withdraw"; after withdrawing the wallet balance increases | SAS §5.6 |
| UI-09 `[P0]` | Errors surface as human messages | attempt an invalid action | Toast shows a clear sentence, not "Something went wrong"; the E-xx code appears in the detail | FSD §7 |
| UI-10 `[P0]` | Full dispute flow through the UI | open a dispute, submit evidence from both sides, arbiter decides 70/30 | Status changes accordingly; both parties can withdraw | F-11…F-13 |
| UI-11 `[P1]` | AI summary available | click Generate on the dispute page | Summary plus disclaimer renders in under 20 seconds; no control can move funds | P1-01 |
| UI-12 `[P1]` | AI summary unavailable | disable the endpoint | "AI summary unavailable" shown; the dispute flow still works | SAS §8 |
| UI-13 `[P1]` | Reject the transaction in the wallet | cancel in the popup | State returns to normal, no red error, nothing changed | SAS §5.1 |
| UI-14 `[P0]` | Two-minute demo rehearsal | run the full demo script twice in a row | Both runs complete without errors; total time ≤ 2 minutes | PRD §15 |
| UI-15 `[P1]` | RPC rate limiting | 50 rapid requests | App stays responsive; retry/backoff observed | SAS D-02 |

---

# 6. Error Code Coverage Matrix

Every error code requires at least one negative test (FSD §9, SCTS §13).

| Code | Meaning | Test cases |
|---|---|---|
| E-01 | Unauthorized / not owner | AC-02, AC-07, AP-02, AB-03, AB-04, RF-06, DS-07, RV-05, SD-06, AD-04, SC-01, SC-02, SC-03 |
| E-02 | Invalid project state | SD-05, SD-08, AP-03, RV-08, RF-05, RF-07, AR-03, AB-09, MS-05, INV-05 |
| E-03 | Deadline expired | CP-07, AC-05, AC-06, SD-04, RV-04, RF-03, RF-04, DS-04, AP-05, CP-12 |
| E-04 | Invalid escrow amount / participants | CP-02, CP-03, CP-04, CP-05, CP-06 |
| E-05 | Already accepted | AC-04 |
| E-06 | Revision limit exceeded | RV-02 |
| E-07 | Dispute already active | DS-10 |
| E-08 | Already finalized | AP-04, RF-08, AB-07, MS-06, DS-09, INV-05 |
| E-09 | Invalid settlement percentage | AB-02 |
| E-10 | Invalid evidence / deliverable | SD-02, SD-07, DS-06 |
| E-11 | Review period not expired | AR-02 |
| E-12 | Payout failed (event-only, never an `Err`) | WD-07 asserts the `withdraw_failed` event |
| E-13 | Resolution window expired | AB-05 |
| E-14 | Insufficient reserve | AD-01 |
| E-15 | Contract paused | CP-09, AD-03 |
| E-16 | Settlement mismatch | MS-02, MS-03 |
| E-17 | Grace not elapsed | AD-05 |
| E-18 | Nothing to withdraw | WD-02, WD-03, WD-06 (alternative outcome) |
| E-19 | Withdraw in progress | SC-07, WD-06 |
| E-20 | Dispute not allowed in this state | DS-03 |
| E-21 | Unknown project | SC-08 |
| E-22 | No withdrawal record to clear (admin) | AD-09 |

All 22 codes are covered. No code is left without a test, and no code is listed under a test that expects a different one.

> Note on E-02 vs E-08/DS-09: `DS-09` (dispute on a `Released` project) is listed once, under E-08. E-02 is reserved for the plain "wrong state, not terminal" cases.

---

# 7. Execution Order

## 7.1 CI — every commit

```
1. cargo fmt --check
2. cargo clippy -- -D warnings
3. cargo test            # L1 + L3, groups CP…INV
```

Target duration: under 60 seconds.

## 7.2 CI — every pull request

```
4. cargo near build                   # wasm size < 300 KB
5. cargo test --test integration      # L2, WD-01…WD-09
```

Target duration: under 5 minutes.

## 7.3 Before submission — manual, once

```
6. Deploy to testnet-staging
7. Run L4: UI-01 … UI-15
8. Rehearse the demo twice (UI-14) on testnet-demo
9. Reset the demo contract and stage clean demo data
10. Record the video
```

## 7.4 Recommended implementation order for tests

Follow business-flow dependency, not alphabetical order:

1. CP-01 → INV-01 (foundation: a project can be created and funds are tracked)
2. AC-01, SD-01, AP-01, WD-01 (the main happy path)
3. All negative tests per group
4. RV, AR, RF
5. DS, AB, MS
6. AD, full INV, SC
7. UI

Rationale: the happy path first establishes the scaffolding; negative tests follow once positive behaviour is stable, so a negative failure is unambiguous between "wrong logic" and "guard not written yet".

---

# 8. Exit Criteria (Definition of Done)

Testing is complete only when **all** of the following hold:

| # | Criterion |
|---|---|
| 1 | 100% of P0 tests pass (102 tests). |
| 2 | Every error code E-01…E-22 has at least one passing negative test. |
| 3 | Every STS transition §5.1–§5.12 has a passing positive test. |
| 4 | Every FSD feature F-01…F-15 has at least one test (automated or manual E2E). |
| 5 | Fund invariants are proven on every test that moves money. |
| 6 | No test is skipped/ignored without a documented reason and a tracking issue. |
| 7 | The two-minute demo succeeds twice in a row on `testnet-demo` (UI-14). |
| 8 | `cargo clippy` is clean with `-D warnings`. |
| 9 | Wasm size is under 300 KB. |
| 10 | `get_fund_invariants().ok == true` after the entire demo run. |

---

# 9. Testing Risks

| Risk | Impact | Mitigation |
|---|---|---|
| Time-dependent tests rely on the real clock | Flaky and slow | Always use `block_timestamp`; never `sleep` in L1 |
| Async callbacks are hard to unit test | `withdraw` is not fully covered | All withdrawal paths are tested at L2 (`near-workspaces`) |
| Reserve drains during rehearsal | `create_project` fails before the demo | AD-01, plus resetting the demo contract before recording (§7.3 step 9) |
| Negative tests pass for the wrong reason | Hidden bugs | Assert the specific error code, never `is_err()` |
| UI tested only manually | Regressions go undetected | At minimum one Playwright smoke test for CP and AP if time allows |
| 138 tests is too many for the remaining time | P0 unfinished | Cut P1 first; if still short, follow the group order in §7.4 |

---

# 10. Implementation Notes

1. `assert_solvent()` runs **inside the contract**, so invariant tests do not recompute it — they only confirm no panic occurs.
2. Negative tests must verify that **no state changed** (status, `total_locked`, `claimable`), not merely that an error was returned.
3. For boundary-time tests, always check three points: `deadline - 1s` (not yet allowed), `deadline` (the boundary), `deadline + 1s` (allowed). This is the most common implementation mistake — `>=` versus `>`.
4. For withdrawals, assert the recipient wallet balance via `view_account` before and after, net of gas.
5. Always reset `context.predecessor_account_id` between tests — forgetting it is the most common source of false passes with `VMContextBuilder`.

---

# 11. Decisions Record

All items below were open in the review draft and are now **final**.

### D-01 — 102 P0 mandatory, 36 P1 optional

The P0 set covers every business rule, all 22 error codes and all 13 STS transitions. P1 adds depth (concurrency, random walks, extremes) and is the first thing cut if time runs short.

### D-02 — Playwright smoke tests, two scenarios only

One flow creates a project end to end; one approves a deliverable. Two scenarios cover the highest-risk integration path (wallet → transaction → refetch → render) without spending time building a full E2E suite.

*Rationale:* manual E2E catches layout bugs but never catches regressions, because nobody reruns it. Two automated scenarios buy most of that value for a fraction of the cost.

### D-03 — A dedicated staging contract, separate from the demo contract

`escrow-staging` absorbs all manual testing; `escrow-demo` is used exclusively for rehearsal and recording, and is reset beforehand.

*Rationale:* manual testing dirties state and consumes storage reserve. A dedicated staging contract makes "clean demo data" a guaranteed property rather than a cleanup chore performed under time pressure.

### D-04 — Randomized invariant testing, P1

INV-07 and INV-08 stay in the catalogue at P1.

*Rationale:* they are the most likely tests to find a real accounting bug, but they need a mature action generator. They are worth the time only once the deterministic suite is green.

### D-05 — No automated tests for the AI service

Coverage is manual through UI-11 and UI-12 only.

*Rationale:* the AI service is optional, stateless and has no authority over funds. Automated tests there would cost more than the risk they remove.

### D-06 — No test for `migrate` beyond AD-07

Only the private-method guarantee is tested.

*Rationale:* the MVP has no real migration. Testing a fictional migration proves nothing about safety.

---

# 12. Summary Figures

| Metric | Value |
|---|---|
| Total test case | 138 |
| Mandatory (P0) | 102 |
| Extended (P1) | 36 |
| Groups | 15 (CP, AC, SD, RV, AP, AR, RF, DS, AB, MS, WD, AD, INV, SC, UI) |
| Error codes covered | 22 / 22 |
| STS transitions covered | 13 / 13 |
| FSD features covered | F-01 … F-15, complete |

---

# 13. Document Status

This specification is **approved and final (v1.5)**. It supersedes all earlier versions and is the authoritative testing reference for NEAR Escrow.

Next document: **06 — API and Frontend Integration Specification**.
