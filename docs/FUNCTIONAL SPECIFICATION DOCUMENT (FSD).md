# FUNCTIONAL SPECIFICATION DOCUMENT (FSD)
## NEAR Escrow

| Document Information | Details |
|---|---|
| Version | 1.5 |
| Revision | v1.5: MVP acceptance criteria renumbered `AC-xx` → `MA-xx` to stop colliding with the Test Plan's `AC-xx` accept/reject tests. v1.4: amounts and deadlines now match what the contract actually enforces (0.1 NEAR minimum, 48-hour deadline floor) instead of "greater than zero" and "in the future"; mutual settlement acceptance criteria aligned with SCTS D-21; status set to Approved. v1.3: settlement records an allocation, not a transfer (§4.10 rule 6 — the old wording forbade the pull-payment model the contract uses), and a party may concede to the counterparty's percentage (§4.9 rule 4). v1.2: mutual settlement no longer implies auto-settlement (§4.9 rule 6), the revision extension is stated as measured from the request (§4.5 rule 4), and duplicate list numbering in §4.8 is fixed. v1.1 corrects the dispute timing rule (§4.8): a missed revision deadline must not block opening a dispute, otherwise funds can be locked with no reachable resolution. Also records the extended error codes E-13–E-21 in §7. |
| Date | November 8, 2026 |
| Status | Approved |
| Product | NEAR Escrow |
| Network | NEAR Testnet |
| Hackathon Track | `realworld` |
| Reference Document | NEAR Escrow PRD v1.0 |

---

## 1. Introduction

### 1.1 Purpose

This Functional Specification Document (FSD) defines the functional behavior and business rules of NEAR Escrow.

It serves as the primary reference for:
- Smart contract development.
- Frontend development.
- User role and permission management.
- Transaction validation.
- Test case preparation.
- Error handling and edge case management.

This document translates the product requirements defined in the PRD into specific, implementable, and testable system requirements.

### 1.2 Product Description

NEAR Escrow is a blockchain-based escrow application built on the NEAR network. It enables clients and freelancers to conduct transactions while funds are securely locked in a smart contract.

Funds can only be released according to predefined rules:
- The client deposits funds before work begins.
- The freelancer accepts the project before starting work.
- The client approves the deliverables to release payment.
- If a dispute occurs, funds remain locked until a resolution is reached.
- A mutually agreed arbiter can determine how the funds are distributed.

NEAR Escrow aims to reduce direct counterparty trust requirements through transparent and verifiable blockchain-based rules.

### 1.3 System Principles

1. **Fund Security:** Project funds must be protected against unauthorized withdrawals.
2. **Role-Based Authorization:** Each action can only be performed by an authorized participant.
3. **State-Based Validation:** Actions are only permitted when the project is in a valid state.
4. **Transparent Settlement:** Fund distribution must follow verifiable settlement rules.
5. **No Double Spending:** Project funds must never be refunded or released more than once.
6. **Human-Controlled Arbitration:** A human arbiter remains responsible for dispute decisions. AI is only an assistive tool.

---

## 2. Functional Scope

### 2.1 P0 Features — MVP Requirements

| ID | Feature | Description |
|---|---|---|
| F-01 | Wallet Connection | Users connect their NEAR wallets. |
| F-02 | Create Project | Clients create projects and define work requirements. |
| F-03 | Fund Escrow | Clients deposit funds into the smart contract. |
| F-04 | Accept Project | Freelancers accept assigned projects. |
| F-05 | Reject Project | Freelancers reject projects and trigger a refund. |
| F-06 | Submit Deliverable | Freelancers submit their completed work. |
| F-07 | Approve Deliverable | Clients approve submitted work. |
| F-08 | Request Revision | Clients request revisions, up to a maximum of two times. |
| F-09 | Automatic Release | Funds can be released after the review period expires without client action. |
| F-10 | Claim Refund | Clients request refunds when predefined conditions are met. |
| F-11 | Open Dispute | Either party can initiate a dispute. |
| F-12 | Submit Evidence | Both parties submit supporting evidence. |
| F-13 | Arbiter Resolution | The arbiter determines the percentage distribution of funds. |
| F-14 | Mutual Settlement | Both parties can mutually agree on a fund distribution. |
| F-15 | Project Dashboard | Users view their project lists and project details. |

### 2.2 P1 Features — Post-MVP Enhancements

| ID | Feature | Description |
|---|---|---|
| P1-01 | AI Dispute Summary | AI summarizes evidence and both parties' positions. |
| P1-02 | Freelancer Reputation | Displays freelancer activity statistics derived from on-chain data. |
| P1-03 | USDC Payment | Supports payments using the USDC stablecoin. |

### 2.3 P2 Features — Out of MVP Scope

- Milestone-based payments.
- Freelancer and arbiter marketplace.
- Platform fees.
- Email or Telegram notifications.
- Deadline extensions through mutual agreement.
- Mutual project cancellation.

P1 and P2 features must not delay the completion of P0 requirements.

---

## 3. Actors and Access Permissions

The system consists of three primary actors.

### 3.1 Client

The client creates a project and provides the escrow funds.

Permissions:
- Create projects.
- Specify the freelancer and arbiter.
- Set the project deadline.
- Deposit escrow funds.
- Claim refunds when eligible.
- Approve deliverables.
- Request revisions.
- Open disputes.
- Submit dispute evidence.
- Propose or approve mutual settlements.

### 3.2 Freelancer

The freelancer is responsible for accepting and completing the assigned project.

Permissions:
- View projects assigned to their account.
- Accept projects.
- Reject projects.
- Submit deliverables.
- Resubmit deliverables after revisions.
- Trigger fund release after the review period expires.
- Open disputes.
- Submit dispute evidence.
- Propose or approve mutual settlements.

### 3.3 Arbiter

The arbiter is a neutral party selected and agreed upon by the client and freelancer.

Permissions:
- View information about disputed projects.
- Review evidence submitted by both parties.
- Determine the percentage distribution of funds.
- Submit a dispute resolution decision.

The arbiter is not permitted to:
- Modify project details.
- Change the escrow amount.
- Replace themselves.
- Withdraw funds without a valid settlement decision.
- Modify a decision after settlement has been initiated.

### 3.4 Identity Rules

- User identities are determined by their NEAR Account IDs.
- The client, freelancer, and arbiter must use three different accounts.
- The account that creates a project becomes its client.
- The freelancer and arbiter are assigned during project creation.
- The arbiter cannot be replaced after the freelancer accepts the project.

---

## 4. Core Business Rules

This section defines the primary business rules that must be followed by both the frontend and smart contract.

### 4.1 Project Creation Rules

1. Only a connected wallet can create a project.
2. The client must provide a project title and description.
3. The client must specify the freelancer's NEAR Account ID.
4. The client must specify the arbiter's NEAR Account ID.
5. The client must define the project deadline.
6. The client must specify the escrow amount.
7. The escrow amount must be at least the contract minimum (`MIN_ESCROW = 0.1 NEAR`); anything below is rejected with `E-04`. Gathered from `get_config().min_escrow`, never hardcoded on the client.
8. The client, freelancer, and arbiter must use different accounts.
9. The deadline must be at least the 48-hour acceptance window in the future (`>= now + accept_window`), not merely in the future.
10. A project is considered successfully created only after the escrow deposit transaction is confirmed.
11. Every project must have a unique Project ID.

**MVP Payment Configuration:**
- Payment asset: Native NEAR.
- Platform fee: 0%.
- Project funds are held and managed by the smart contract.
- No automatic fiat conversion is supported.

### 4.2 Project Acceptance Rules

1. Only the assigned freelancer can accept or reject a project.
2. The freelancer has a maximum of 48 hours to accept or reject the project.
3. Acceptance changes the project status to `ACCEPTED`.
4. Rejection initiates the refund process.
5. If the freelancer does not respond within 48 hours, the client can claim a refund.
6. A project cannot be accepted again after rejection, refund, or settlement initiation.

### 4.3 Deliverable Submission Rules

1. Only the freelancer can submit deliverables.
2. Deliverables must be submitted before the active project deadline.
3. The freelancer must provide a deliverable link.
4. An additional note is optional.
5. Submission changes the project status to `DELIVERED`.
6. After submission, the client receives a 72-hour review period.
7. Every resubmission after a revision starts a new 72-hour review period.

**Additional Note:** Links and notes stored on-chain are publicly accessible. Users must not upload personal information or confidential documents directly into contract storage.

### 4.4 Deliverable Approval Rules

1. Only the client can approve deliverables.
2. Approval is only permitted when the project status is `DELIVERED`.
3. Approval initiates fund release to the freelancer.
4. After successful settlement, the project status becomes `RELEASED`.
5. A finalized project cannot be reopened or disputed.

### 4.5 Revision Rules

1. Only the client can request revisions.
2. Revisions can only be requested when the project status is `DELIVERED`.
3. A maximum of two revisions is allowed per project.
4. Each revision request extends the work deadline by three days, measured from the moment the revision is requested (not from the previous deadline). A revision requested in the last hour of the review window still grants a full three days of work (SCTS D-09).
5. A revision request changes the project status to `REVISION_REQUESTED`.
6. The freelancer must resubmit the deliverable before the updated deadline.
7. After resubmission, the project status returns to `DELIVERED`.
8. A new 72-hour review period begins after resubmission.
9. The client cannot request additional revisions after the revision limit has been reached.

**Additional Condition:**

If the freelancer fails to submit the revised deliverable before the updated deadline, the client is not automatically entitled to a refund. The client can open a dispute so the arbiter can determine an appropriate resolution based on the available evidence.

### 4.6 Automatic Release Rules

1. Automatic release applies when the project status is `DELIVERED`.
2. The client has 72 hours to approve, request a revision, or open a dispute.
3. If the review period expires without client action, funds become eligible for release to the freelancer.
4. Anyone can call the automatic release function after the review period expires.
5. The caller is not entitled to receive any project funds.
6. All funds must be transferred to the freelancer according to the settlement rules.
7. Automatic release is not permitted if the project has entered a dispute or settlement has already been initiated.

### 4.7 Refund Rules

A refund returns the escrow funds to the client.

Refunds are only permitted under the following conditions:

| Condition | Authorized Caller | Outcome |
|---|---|---|
| Freelancer rejects the project | Freelancer | Funds returned to client |
| Freelancer does not respond within 48 hours | Client | Funds returned to client |
| Freelancer fails to submit the initial deliverable before the deadline | Client | Funds returned to client |

Additional rules:
- Refunds are only permitted before project settlement.
- A project under active dispute cannot use the regular refund mechanism.
- A refund cannot be executed more than once.
- A refund cannot be executed after funds have been successfully released to the freelancer.

### 4.8 Dispute Rules

1. A dispute can only be opened after the freelancer has submitted a deliverable.
2. Either the client or freelancer can open a dispute.
3. A dispute can only be opened when the project status is `DELIVERED` or `REVISION_REQUESTED`.
4. A dispute can be opened while the project is `DELIVERED` before the review period expires, or at any time while the project is `REVISION_REQUESTED` — including after the revision deadline has passed.
5. The revision deadline must not block a dispute. Blocking it would let a silent freelancer strand the funds permanently: after the deadline the freelancer cannot resubmit and the client cannot claim a refund (see §4.5), so the dispute must remain reachable.
6. Once a dispute is opened, regular fund release mechanisms are disabled.
7. Both parties can submit supporting evidence through links and notes.
8. The arbiter has seven days from the dispute opening timestamp to submit a decision.
9. The arbiter determines the percentage of funds allocated to the freelancer, from 0% to 100%.
10. The remaining percentage is allocated to the client.

Example distribution:

| Freelancer | Client | Outcome |
|---:|---:|---|
| 100% | 0% | All funds go to the freelancer |
| 70% | 30% | Funds are distributed 70:30 |
| 50% | 50% | Funds are distributed equally |
| 0% | 100% | All funds are returned to the client |

### 4.9 Mutual Settlement Rules

Mutual settlement allows both parties to resolve a dispute without waiting for an arbiter decision.

1. Either party can propose a fund distribution percentage.
2. The proposed percentage represents the freelancer's share.
3. Settlement can only proceed if the other party approves the exact same percentage.
4. A party may accept the counterparty's percentage by calling `settle_mutual` with it, which supersedes their own earlier proposal. Two stored proposals that differ block nothing on their own: what matters is the percentage passed to `settle_mutual` and whether the counterparty has proposed that same value (SCTS §6.2, D-21).
5. Either party may update their proposal before matching approval is recorded.
6. Mutual settlement completes when either party calls `settle_mutual` with the percentage that the other party has already proposed. Two matching proposals alone do **not** move funds: the settlement transaction itself is the caller's approval. This manual step is deliberate — a contract that settles the instant two numbers coincide lets a single mistyped proposal resolve a dispute with no confirmation from the person it affects.
7. After successful settlement, the project status becomes `RESOLVED`.

**If the arbiter fails to submit a decision within seven days:**
- No automatic 50/50 fund distribution occurs.
- Both parties can still use mutual settlement.
- If no agreement is reached, the funds remain locked.
- Neither party can withdraw the funds unilaterally.

Funds remaining locked without resolution are an acknowledged MVP limitation. This is safer than forcing a distribution without agreement or a clearly defined mechanism.

### 4.10 Settlement Rules

Settlement is the final process of distributing or releasing escrow funds.

1. Settlement can only be initiated through an authorized path:
   - Client approval.
   - Automatic release.
   - Arbiter resolution.
   - Mutual settlement.
2. The distribution percentage must be between 0% and 100%.
3. The total distribution must equal 100%.
4. Settlement cannot be executed more than once.
5. Funds cannot be released while a dispute is active, except through an authorized resolution mechanism.
6. Settlement records an **allocation**, not a transfer. The final status is written when the split is recorded; the money moves later, when each recipient calls `withdraw` (STS §9, SCTS §5.3). Requiring a confirmed transfer before recording the status would make every multi-recipient settlement impossible and would couple the dispute decision to a transfer that can fail for reasons the decision never considered.
7. A failed transfer must never roll back, modify or invalidate a recorded settlement. The allocation stands; the recipient retries the withdrawal from their recorded claimable balance.
8. Transfer failures must not result in lost funds or duplicate payouts.

---

## 5. Detailed Functional Requirements

### 5.1 Wallet Connection

**ID:** F-01

Description:

Users must connect a NEAR wallet to access application features.

Acceptance Criteria:
- Users can connect their wallets.
- The application displays the connected Account ID.
- Users can disconnect their wallets.
- Users cannot initiate transactions without the appropriate connected wallet.
- The application displays transaction status to users.

### 5.2 Create Project and Fund Escrow

**ID:** F-02, F-03

Description:

The client creates a project and deposits funds into the smart contract.

Inputs:
- Project title.
- Project description.
- Freelancer Account ID.
- Arbiter Account ID.
- Project deadline.
- Escrow amount.

Acceptance Criteria:
- All inputs are validated.
- The escrow amount must be at least 0.1 NEAR (`MIN_ESCROW`), not merely non-zero.
- Account IDs must be valid.
- All three participants must be different.
- The deadline must be at least 48 hours in the future.
- The deposit must succeed before the project is considered `FUNDED`.
- The system generates a unique Project ID.

### 5.3 Accept or Reject Project

**ID:** F-04, F-05

Description:

The freelancer decides whether to accept the assigned project.

Acceptance Criteria:
- Only the assigned freelancer can accept or reject the project.
- Actions are only permitted within the acceptance period.
- Rejection returns funds to the client.
- Once accepted, the project cannot be rejected again.
- Unauthorized accounts must be rejected.

### 5.4 Submit Deliverable

**ID:** F-06

Description:

The freelancer submits the completed work.

Acceptance Criteria:
- Only the freelancer can submit deliverables.
- A deliverable link is required.
- Additional notes are optional.
- The active deadline must not have expired.
- The project status must be `ACCEPTED` or `REVISION_REQUESTED`.
- Submission starts a 72-hour review period.

### 5.5 Approve Deliverable

**ID:** F-07

Description:

The client approves the submitted work.

Acceptance Criteria:
- Only the client can approve the deliverable.
- The project status must be `DELIVERED`.
- Approval initiates payment to the freelancer.
- Duplicate payments must be prevented.
- If a transfer fails, the system must provide a safe recovery mechanism.

### 5.6 Request Revision

**ID:** F-08

Description:

The client requests improvements to the submitted work.

Acceptance Criteria:
- Only the client can request a revision.
- The project status must be `DELIVERED`.
- The revision count must not exceed two.
- The work deadline is extended by three days.
- The freelancer can resubmit the deliverable before the updated deadline.
- Resubmission starts a new 72-hour review period.

### 5.7 Automatic Release

**ID:** F-09

Description:

Funds are released to the freelancer when the client does not respond before the review period expires.

Acceptance Criteria:
- The project status must be `DELIVERED`.
- The 72-hour review period must have expired.
- No active dispute must exist.
- Anyone can call the release function.
- Funds are only transferred to the freelancer.
- Duplicate releases must be prevented.

### 5.8 Claim Refund

**ID:** F-10

Description:

The client requests a refund when predefined project conditions are met.

Acceptance Criteria:
- Only the client can claim a refund.
- Refund timing requirements must be satisfied.
- For a refund caused by a missed initial deadline, no initial deliverable must have been submitted.
- The project must not be under active dispute.
- Funds must be returned to the client.
- Duplicate refunds must be prevented.

### 5.9 Open Dispute and Submit Evidence

**ID:** F-11, F-12

Description:

Both parties can open a dispute and provide supporting evidence.

Acceptance Criteria:
- Either the client or freelancer can open a dispute.
- Disputes are only allowed in permitted project states.
- Project funds are locked against regular settlement.
- Both parties can submit evidence.
- Evidence is associated with the correct Project ID.
- Evidence cannot be unilaterally modified after final resolution.
- The arbiter can access relevant evidence.

### 5.10 Arbiter Resolution

**ID:** F-13

Description:

The arbiter determines the distribution of escrow funds.

Acceptance Criteria:
- Only the assigned arbiter can submit a decision.
- The project status must be `DISPUTED`.
- The freelancer's percentage must be between 0% and 100%.
- The client's percentage is the remainder of the total.
- The decision cannot be changed after settlement has been initiated.
- Settlement follows the arbiter's decision.

### 5.11 Mutual Settlement

**ID:** F-14

Description:

Both parties agree on a dispute resolution without waiting for the arbiter's decision.

Acceptance Criteria:
- Both parties must end up on the same percentage: one via a stored proposal, the other via the `settle_mutual` call.
- Differing stored proposals must not trigger settlement on their own, but must not block it either — a party can concede by calling `settle_mutual` with the counterparty's percentage.
- The same percentage proposed by one party alone must not settle anything.
- The same party cannot be counted as both approvals.
- Settlement can only occur once.
- If no agreement is reached, funds remain locked.

### 5.12 Project Dashboard

**ID:** F-15

Description:

Users view projects associated with their wallets.

Acceptance Criteria:
- Clients can view projects they created.
- Freelancers can view projects assigned to them.
- Arbiters can view projects requiring their action.
- Project details display status, amount, deadline, and action history.
- Users cannot perform actions outside their assigned roles.

---

## 6. System Timing Rules

| Activity | Duration | Description |
|---|---:|---|
| Freelancer acceptance | 48 hours | Maximum time to accept or reject a project |
| Client review | 72 hours | Time allowed to review deliverables |
| Revision extension | 3 days | Additional work time for each revision |
| Arbiter resolution | 7 days | Time allowed for an arbiter decision |

Additional rules:
- All time calculations must use blockchain timestamps.
- The frontend only displays countdowns; it is not the source of truth for time.
- Deadline validation must be enforced by the smart contract.
- An expired deadline must not be treated as valid because the UI has not refreshed.
- Time-dependent functions must validate the timestamp when the transaction is executed.

---

## 7. Validation and Error Handling

The system must reject actions that do not satisfy the defined rules.

| Code | Condition | Expected Response |
|---|---|---|
| E-01 | Unauthorized wallet | Unauthorized action |
| E-02 | Invalid project state | Invalid project state |
| E-03 | Deadline has expired | Deadline expired |
| E-04 | Invalid escrow amount | Invalid escrow amount |
| E-05 | Freelancer already accepted the project | Project already accepted |
| E-06 | Revision limit exceeded | Revision limit exceeded |
| E-07 | Dispute already opened | Dispute already active |
| E-08 | Settlement already completed | Project already finalized |
| E-09 | Invalid distribution percentage | Invalid settlement percentage |
| E-10 | Invalid evidence | Invalid evidence |
| E-11 | Review period has not expired | Review period not expired |
| E-12 | Transfer failed | Payout failed / retry required |

Frontend error messages must be understandable and must not rely solely on displaying raw error codes.

**Extended code set.** Codes `E-13`–`E-22` (ten codes) were added by the Smart Contract Technical Specification §11 to cover conditions this document did not enumerate (resolution window expiry, insufficient storage reserve, paused contract, settlement mismatch, withdrawal grace period, nothing to withdraw, withdrawal in progress, dispute not allowed in this state, unknown project, and no pending withdrawal record to clear). They are part of the contract's error contract and are mapped to user-facing copy in the API and Frontend Integration Specification §9. `E-12` is event-only: it is reported through the `withdraw_failed` event rather than as a returned error.

---

## 8. Functional Security Requirements

1. All sensitive actions must verify the caller's identity.
2. All actions must validate the project state before modifying data.
3. Finalized projects cannot be modified.
4. Escrow funds cannot be withdrawn by unauthorized accounts.
5. Payment amounts must use integer-based representations, not floating-point values.
6. The system must prevent duplicate refunds and releases.
7. The arbiter cannot modify a decision after settlement has been initiated.
8. The client cannot replace the freelancer or arbiter after project acceptance.
9. Dispute evidence must be associated with the correct project.
10. Transfer failures must have a safe recovery mechanism.
11. Sensitive information must not be stored publicly on the blockchain.
12. AI must not have the authority to execute payments or modify settlement states.

---

## 9. MVP Acceptance Criteria

The MVP is considered functionally complete when the following scenarios work as expected.

> **ID prefix note:** these are `MA-xx` (**M**VP **A**cceptance), not `AC-xx`. The Test Plan uses `AC-xx` for the accept/reject test cases, and two different ID schemes sharing one prefix is a reliable way to mis-cite the wrong requirement. Only `MA-` IDs belong to this table.

| ID | Scenario | Expected Result |
|---|---|---|
| MA-01 | Client creates a project and deposits funds | Project status becomes `FUNDED` |
| MA-02 | Freelancer accepts the project | Project status becomes `ACCEPTED` |
| MA-03 | Freelancer rejects the project | Funds are returned to the client |
| MA-04 | Freelancer submits a deliverable | Project status becomes `DELIVERED` |
| MA-05 | Client approves the deliverable | Funds are released to the freelancer |
| MA-06 | Client requests a revision | Project status becomes `REVISION_REQUESTED` |
| MA-07 | Freelancer submits a revised deliverable | Project returns to `DELIVERED` |
| MA-08 | Client does not respond within 72 hours | Funds become eligible for release to the freelancer |
| MA-09 | Freelancer fails to submit the initial deliverable before the deadline | Client can claim a refund |
| MA-10 | Client opens a dispute | Project status becomes `DISPUTED` |
| MA-11 | Arbiter determines fund distribution | Settlement follows the arbiter's decision |
| MA-12 | Both parties agree on a distribution | Mutual settlement succeeds |
| MA-13 | Arbiter remains inactive for seven days | Funds remain locked until resolution |
| MA-14 | Unauthorized user attempts to release funds | Transaction is rejected |
| MA-15 | User attempts to release funds twice | Second transaction is rejected |

---

## 10. MVP Constraints and Assumptions

### 10.1 Constraints

- Payments are limited to native NEAR.
- The application operates on NEAR Testnet.
- Milestone-based payments are not supported.
- No marketplace is included.
- No platform fee is charged.
- Fiat withdrawals are not supported.
- No in-app chat system is included.
- AI does not determine dispute outcomes.
- The system does not guarantee arbiter availability.

### 10.2 Assumptions

- Clients and freelancers already have NEAR Account IDs.
- Both parties understand that blockchain transactions require a wallet and gas fees.
- The arbiter is selected and agreed upon before project acceptance.
- Both parties can access the evidence they submit.
- Deliverable links remain accessible throughout the review and dispute processes.

### 10.3 Unresolved Risks

**The arbiter is inactive and both parties cannot reach an agreement.**

In this situation, funds remain locked. The MVP does not provide a unilateral recovery mechanism.

This limitation must be documented transparently and must not be hidden behind claims that every dispute can always be resolved.

---

## 11. Definition of Done

A feature is considered complete when:

- Its business rules have been implemented.
- Its access permissions have been validated.
- Failure conditions have been handled.
- Relevant test cases have passed.
- No invalid state transitions are possible.
- Duplicate settlement is prevented.
- The frontend correctly displays transaction results.
- Fund movements can be verified through a blockchain explorer.