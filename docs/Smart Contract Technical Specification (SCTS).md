# SMART CONTRACT TECHNICAL SPECIFICATION (SCTS)

**Project:** NEAR Escrow
**Document Version:** 1.6 — Final
**Status:** Approved
**Date:** November 8, 2026
**Revision:** v1.7 records the first testnet deployment: errors only reach the chain as `E-xx` text when the build sets `cfg(near)`, so the error type derives `FunctionError` and the flag is pinned per-target (D-26). v1.6 records the corrections found by the first real compile (D-25) and marks the snippets as toolchain-verified. v1.5: crate renamed to `nescrow` (matches the repository) and MVP acceptance IDs moved to `MA-xx` to stop colliding with the Test Plan's `AC-xx`. v1.4 applies the fourth audit: `Balance` replaced with a local `Yocto` alias plus `NearToken` boundary unwrapping (D-24 — near-sdk 5 exports no `Balance` and `NearToken` has no `Sub`/`Mul`, so the old snippets could not compile), storage reserve figures corrected to the 6,000-byte bound (0.06 NEAR/project, 20 NEAR recommended), `credit()` plus Rust `Config`/`FundInvariants` defined, and `MAX_PROJECT_STORAGE_BYTES` demoted from "measured" to a declared bound guarded by AD-10. v1.3 applies the third audit: `work_deadline` typed `U64` so the frontend's string payload deserialises (D-22), deterministic `MAX_PROJECT_STORAGE_BYTES` so the `E-14` reserve check is implementable (D-23), `get_withdrawal_state` returns `WithdrawalRecordView` (the D-15 precision fix had missed it), missing `DeliverableView`/`EvidenceView`/`ResolutionView` defined, §7.3 numbering repaired, transition count corrected to 13, and the `ContractPaused` precedence raised to match `create_project`'s validation order. v1.2 applies the second independent audit: JSON DTOs so amounts and timestamps are strings (D-15, §4.3.1), explicit reserve pre-check (D-16), `withdraw` returning `Result<Promise, _>` with a documented fallback (D-17), lowercase `Role` wire format (D-18), `E-22` for admin recovery paths (D-19), and a work-deadline floor that prevents born-dead projects (D-20). v1.1 fixes: `resolution` is now written only on dispute paths (D-11), error codes split and given a precedence order (D-12, §11.1), `E-12` documented as event-only (§11.2), `get_claimable_per_project` renamed semantics fixed (§6.3), dependency versions updated to near-sdk 5.30 / near-workspaces 0.24.
**Based on:** Functional Specification Document (FSD) v1.5 · State Transition Specification (STS) v1.5
**Network:** NEAR Testnet
**Target SDK:** `near-sdk` 5.x · Rust stable (2021 edition)

---

# 1. Purpose

This document translates the business rules defined in the FSD and the state machine defined in the STS into an implementable technical design for the NEAR smart contract.

In scope:

1. Data model and structs.
2. Storage strategy and fund accounting.
3. Full method surface (mutating and view) with signatures, authorization and guards.
4. Role and access control.
5. Deposit, refund and payout mechanisms (pull payment) including asynchronous callbacks.
6. Events (NEP-297).
7. Error handling and mapping to FSD error codes.
8. Security invariants and how they are verified.
9. Testing, build, deploy and upgrade strategy.

Out of scope (covered by other documents): frontend architecture (document 04), detailed test cases (document 05), frontend integration guide (document 06).

Every open question raised in the review draft has been resolved; see **§17 Decisions Record**. Those decisions are normative.

---

# 2. Design Principles

| # | Principle | Technical consequence |
|---|---|---|
| P-1 | **State first, money second** | Status and balances are persisted before any transfer is initiated. Transfers happen in a separate receipt. |
| P-2 | **Pull payment, not push** | Settlement only allocates `claimable` balances. Funds leave the contract only when the recipient calls `withdraw()`, so a failed transfer can never invalidate a settlement decision. |
| P-3 | **Blockchain time is the only clock** | All time validation uses `env::block_timestamp()` (nanoseconds). The frontend only renders countdowns. |
| P-4 | **No scheduler** | No cron jobs, no backend worker. Time-based transitions are invoked manually by an authorized actor or any public caller. |
| P-5 | **Funds are never commingled untracked** | `total_locked` and `total_claimable` are maintained explicitly, and solvency is asserted on every fund-moving call. |
| P-6 | **Fail closed** | Any unsatisfied precondition aborts the transaction with a typed error. No silent no-ops. |
| P-7 | **Integer arithmetic only** | All amounts are `u128` yoctoNEAR with `checked_mul` / `checked_div`. No floats anywhere. |
| P-8 | **Reentrancy-safe** | Marking a withdrawal as in-progress is part of the same state write that debits the balance, before the transfer receipt is created. |
| P-9 | **AI never touches the contract** | The contract exposes no AI-facing method. Dispute summaries are a frontend-side concern (P1). |

---

# 3. Toolchain and Project Layout

```toml
# Cargo.toml
[package]
name = "nescrow"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
near-sdk = "5.30"
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[dev-dependencies]
near-sdk = { version = "5.30", features = ["unit-testing"] }
near-workspaces = "0.24"
tokio = { version = "1", features = ["full"] }

[profile.release]
codegen-units = 1
opt-level = "z"
lto = true
debug = false
panic = "abort"
overflow-checks = true
strip = true
```

Layout:

```
contract/
├── Cargo.toml
├── src/
│   ├── lib.rs          # contract entry points
│   ├── model.rs        # Project, enums, auxiliary structs
│   ├── events.rs       # NEP-297 event emitters
│   ├── errors.rs       # ContractError + Display
│   ├── constants.rs    # time windows, gas, limits
│   └── internal.rs     # settlement, accounting, solvency assertions
└── tests/
    ├── unit.rs         # VMContextBuilder tests
    └── integration.rs  # near-workspaces sandbox tests
```

Notes:

- `crate-type` must include both `cdylib` (wasm artifact) and `rlib` (unit tests).
- `overflow-checks = true` in release: a panic is always preferable to a silent wrap on money paths.
- Build with `cargo near build` (preferred) or `cargo build --target wasm32-unknown-unknown --release`. Target artifact size: under 300 KB.
- `state_version` is stored in state so future migrations are safe.

---

# 4. Data Model

## 4.1 Money Type

near-sdk 5 **does not export `Balance`** — it was removed when the SDK moved to `NearToken`. Verified against docs.rs 5.30.1: the crate root has no `Balance` item, and both `env::account_balance()` and `env::storage_byte_cost()` return `NearToken`.

`NearToken` also implements no `Add`/`Sub`/`Mul`/`Div` — only `checked_*` / `saturating_*` and comparisons. Writing `env::account_balance() - stake` does **not** compile.

**Decision (D-24):** internal state stores plain `u128` yoctoNEAR behind a local alias; `NearToken` appears only at the SDK boundary, unwrapped with `.as_yoctonear()`.

```rust
use near_sdk::{env, NearToken};

/// Local alias — NOT `near_sdk::Balance` (does not exist in v5).
/// Plain u128 keeps all arithmetic on integers with checked_* helpers.
pub type Yocto = u128;

fn spendable_yocto() -> Yocto {
    // saturating: a contract can never be "negatively funded"
    env::account_balance()
        .as_yoctonear()
        .saturating_sub(storage_stake_yocto())
}

fn storage_stake_yocto() -> Yocto {
    env::storage_byte_cost()
        .as_yoctonear()
        .checked_mul(env::storage_usage() as u128)
        .unwrap_or(Yocto::MAX)
}
```

Never write `NearToken` into state: it would require Borsh coverage the SDK version does not guarantee, and every arithmetic site would then need unwrapping anyway.

## 4.2 Project Status Enum

```rust
#[near(serializers = [borsh, json])]
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ProjectStatus {
    Funded,             // 0
    Accepted,           // 1
    Delivered,          // 2
    RevisionRequested,  // 3
    Disputed,           // 4
    Released,           // 5
    Refunded,           // 6
    Resolved,           // 7
}
```

**Serialization contract:** Borsh serializes the variant as its `u8` index. The order is frozen after the first deployment — reordering corrupts all existing on-chain project data. New variants may only be appended.

```rust
impl ProjectStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Released | Self::Refunded | Self::Resolved)
    }
}
```

## 4.3 `Project`

```rust
#[near(serializers = [borsh, json])]
#[derive(Clone)]
pub struct Project {
    pub id: u64,

    // participants
    pub client: AccountId,
    pub freelancer: AccountId,
    pub arbiter: AccountId,

    // content
    pub title: String,        // 1..=128 bytes
    pub description: String,  // 1..=2000 bytes

    // funds and status
    pub amount: Yocto,        // yoctoNEAR locked in escrow
    pub status: ProjectStatus,

    // timestamps, nanoseconds, env::block_timestamp()
    pub created_at: u64,
    pub accept_deadline: u64,    // created_at + 48h
    pub work_deadline: u64,      // client input; shifted +3d per revision
    pub review_deadline: u64,    // 0 = not applicable
    pub revision_deadline: u64,  // 0 = not applicable
    pub dispute_opened_at: u64,  // 0 = no dispute

    // revisions
    pub revision_count: u8,      // max 2

    // deliverable
    pub deliverable: Option<Deliverable>,

    // dispute
    pub client_evidence: Option<Evidence>,
    pub freelancer_evidence: Option<Evidence>,
    pub client_settlement_proposal: Option<u8>,     // freelancer share, percent
    pub freelancer_settlement_proposal: Option<u8>, // freelancer share, percent

    // outcome
    pub resolution: Option<Resolution>,
    pub distribution: Option<Distribution>,
}
```

### 4.3.1 JSON Serialization — the precision trap

`#[near(serializers = [borsh, json])]` does **not** turn `u128` or `u64` into strings. It derives Borsh and serde; serde writes them as JSON **numbers**. Two consequences:

1. `amount` in yoctoNEAR is around `1e25`, far above `Number.MAX_SAFE_INTEGER` (`9e15`). `JSON.parse` in the browser silently truncates it, so the UI would show a wrong amount and send a wrong amount back.
2. Timestamps in nanoseconds are around `1.7e18` — also above `2^53`. They truncate too, which corrupts every countdown and deadline comparison.

**Decision (D-15):** internal state keeps native `Yocto` (u128) / `u64` (fast, arithmetic-friendly), and every view call returns a **JSON DTO** built with `near_sdk::json_types::{U128, U64}`. The internal `Project` is never serialised to JSON.

```rust
use near_sdk::json_types::{U128, U64};

/// JSON-facing mirror of `Project`. Every integer above 2^53 is a string wrapper.
#[near(serializers = [json])]
pub struct ProjectView {
    pub id: u64,
    pub client: AccountId,
    pub freelancer: AccountId,
    pub arbiter: AccountId,
    pub title: String,
    pub description: String,
    pub amount: U128,
    pub status: ProjectStatus,
    pub created_at: U64,
    pub accept_deadline: U64,
    pub work_deadline: U64,
    pub review_deadline: U64,      // "0" = not applicable
    pub revision_deadline: U64,
    pub dispute_opened_at: U64,
    pub revision_count: u8,
    pub deliverable: Option<DeliverableView>,
    pub client_evidence: Option<EvidenceView>,
    pub freelancer_evidence: Option<EvidenceView>,
    pub client_settlement_proposal: Option<u8>,
    pub freelancer_settlement_proposal: Option<u8>,
    pub resolution: Option<ResolutionView>,
    pub distribution: Option<DistributionView>,
}

#[near(serializers = [json])]
pub struct DeliverableView {
    pub link: String, pub note: String, pub submitted_at: U64, pub revision_index: u8,
}

#[near(serializers = [json])]
pub struct EvidenceView {
    pub link: String, pub note: String, pub submitted_at: U64,
}

#[near(serializers = [json])]
pub struct ResolutionView {
    pub freelancer_percent: u8, pub resolved_by: ResolutionKind, pub resolved_at: U64,
}

#[near(serializers = [json])]
pub struct DistributionView { pub freelancer_amount: U128, pub client_amount: U128 }

#[near(serializers = [json])]
pub struct WithdrawalRecordView { pub amount: U128, pub started_at: U64 }

impl From<&Project> for ProjectView { /* .into() on each numeric field */ }
```

Same rule applies to `Stats`, `Config`, `FundInvariants` and `ProjectClaim`: any field that is a `u128`/`u64` in state is `U128`/`U64` on the wire. `id` and small counters may stay plain numbers — they cannot exceed `2^53`.

**Verification:** the contract must carry at least one test that parses a real `get_project` JSON payload and asserts `typeof amount === "string"` (Test Plan INV-11). A Rust unit test comparing `u128` values passes happily while the browser is already broken; only a JSON-level assertion catches this.

## 4.4 Auxiliary Structs

```rust
#[near(serializers = [borsh, json])]
#[derive(Clone)]
pub struct Deliverable {
    pub link: String,         // required, 1..=512 bytes
    pub note: String,         // optional, 0..=1024 bytes
    pub submitted_at: u64,
    pub revision_index: u8,   // 0 = initial submission, 1..=2 = revision
}

#[near(serializers = [borsh, json])]
#[derive(Clone)]
pub struct Evidence {
    pub link: String,         // 1..=512 bytes
    pub note: String,         // 0..=1024 bytes
    pub submitted_at: u64,
}

#[near(serializers = [borsh, json])]
#[derive(Clone)]
pub struct Resolution {
    pub freelancer_percent: u8,  // 0..=100
    pub resolved_by: ResolutionKind,
    pub resolved_at: u64,
}

#[near(serializers = [borsh, json])]
#[derive(Clone, PartialEq, Eq)]
pub enum ResolutionKind {
    Arbiter,           // 0
    MutualSettlement,  // 1
}

#[near(serializers = [borsh, json])]
#[derive(Clone)]
pub struct Distribution {
    pub freelancer_amount: Yocto,
    pub client_amount: Yocto,
}

#[near(serializers = [borsh, json])]
#[derive(Clone)]
pub struct WithdrawalRecord {
    pub amount: Yocto,
    pub started_at: u64,
}
```

`ResolutionKind` describes **who decided a dispute**, so it is only ever populated on the `DISPUTED → RESOLVED` path. Approvals, automatic releases and refunds are rule-based: they carry a `Distribution` but no `Resolution` (§8.2, D-11).

`Distribution` is stored explicitly rather than recomputed at withdrawal time. This guarantees that:

- rounding is decided exactly once, at settlement;
- the split can never change after a terminal status is reached;
- `freelancer_amount + client_amount == project.amount` is trivially auditable on-chain.

## 4.5 Contract State

```rust
#[near(contract_state)]
pub struct Contract {
    pub owner: AccountId,
    pub state_version: u16,

    pub next_project_id: u64,
    /// `store::LookupMap`, whose storage key is `Vec<u8>` — pass
    /// `b"p".to_vec()`, not `b"p"` (an `&[u8; N]` does not satisfy
    /// `IntoStorageKey`). Verified by compiler, see D-25.
    pub projects: LookupMap<u64, Project>,

    // dashboard indexes (append-only)
    pub client_index: LookupMap<AccountId, Vec<u64>>,
    pub freelancer_index: LookupMap<AccountId, Vec<u64>>,
    pub arbiter_index: LookupMap<AccountId, Vec<u64>>,

    // fund accounting
    pub claimable: LookupMap<AccountId, Yocto>,
    pub withdrawals: LookupMap<AccountId, WithdrawalRecord>,
    pub total_locked: Yocto,     // escrowed in non-terminal projects
    pub total_claimable: Yocto,  // allocated but not yet withdrawn

    // admin
    pub paused: bool,
}
```

Storage notes:

- `LookupMap` cannot be iterated; the three indexes above are the iteration surface and must be maintained by the contract itself.
- Each index is a `Vec<u64>` per account, growing linearly. Acceptable at MVP scale (a few hundred projects per account). If that assumption ever breaks, the index must move to a paginated structure — recorded as a known scalability limit, not a bug.
- `claimable` is **global per account**, aggregated across projects. This keeps `withdraw()` parameterless. Per-project attribution is derivable from each project's `distribution`.
- Projects are never deleted, so storage is never reclaimed. See §17 D-01 for the consequence.

---

# 5. Fund Security Model

## 5.1 Accounting Variables

| Variable | Meaning |
|---|---|
| `total_locked` | yoctoNEAR escrowed in **non-terminal** projects (Funded, Accepted, Delivered, RevisionRequested, Disputed). |
| `total_claimable` | yoctoNEAR already allocated to recipients but not yet withdrawn. |
| `env::account_balance()` | Total contract balance, including storage staking. |
| `storage_stake` | `env::storage_byte_cost() * env::storage_usage()`. Not spendable. |

**Solvency invariant (checked on every fund-moving call, enforced by assertion):**

```rust
// Env values are NearToken; unwrap to u128 before comparing (D-24).
let spendable: Yocto = env::account_balance()
    .as_yoctonear()
    .saturating_sub(storage_stake_yocto());
assert!(
    spendable >= self.total_locked.checked_add(self.total_claimable).unwrap(),
    "E-14: insufficient reserve"
);
```

and at every settlement:

```
freelancer_amount + client_amount == project.amount
total_locked    -= project.amount
total_claimable += project.amount
```

Because a NEAR transaction is atomic, a failed assertion reverts every write in the same call — the invariant can never be violated on-chain.

The contract exposes `get_fund_invariants()` so the UI and tests can audit these figures at any time.

## 5.2 Storage Reserve

NEAR's storage staking means part of the contract balance is locked to pay for state. Escrow funds must never silently pay for it.

**Decision (D-01):** the escrow amount equals the attached deposit exactly, and storage costs are absorbed by the contract from a pre-funded **storage reserve**. The reserve is topped up by the owner (or any donor) through a payable method.

Consequences:

- `MIN_ESCROW = 0.1 NEAR`, chosen so each project's escrow exceeds its own storage footprint. At `storage_byte_cost() = 1e19` yocto/byte (0.00001 NEAR/byte), the `MAX_PROJECT_STORAGE_BYTES = 6_000` bound (D-23) costs **0.06 NEAR per project** — see §5.2.1 for the reserve sizing that follows.
- The contract must be funded with a reserve at deploy time (recommended: 10 NEAR on testnet for demo volume) via `top_up_reserve()`.
- If the reserve runs out, `create_project` fails with `E-14 InsufficientReserve` instead of eating escrow funds. This is the safe failure mode.

## 5.2.1 Reserve Sizing

Because the bound is 6,000 bytes and the byte cost is `1e19` yocto, each project consumes **0.06 NEAR** of reserve:

| Reserve funded | Projects supported |
|---:|---:|
| 5 NEAR | ≈ 83 |
| 10 NEAR | ≈ 166 |
| 20 NEAR | ≈ 333 |
| 50 NEAR | ≈ 833 |

**Recommended demo funding: 20 NEAR** — enough for a rehearsal cycle plus the recording without a top-up. This is deliberately more conservative than an "average project" estimate, because the reserve check uses the worst-case bound and the worst case is what must be funded.

If the reserve runs dry, `create_project` returns `E-14` while every existing project keeps working: the failure is loud, bounded, and recoverable with `top_up_reserve`.

### 5.2.2 `credit()` — the only writer of `claimable`

```rust
fn credit(&mut self, account: &AccountId, amount: Yocto) {
    if amount == 0 { return; }             // never create a zero-balance entry
    let current = self.claimable.get(account).copied().unwrap_or(0);
    let next = current.checked_add(amount).expect("E-12: claimable overflow");
    self.claimable.insert(account, &next);
    self.total_claimable = self.total_claimable.checked_add(amount).expect("E-12: overflow");
}
```

No other function may write `claimable`. Withdrawal is the only place that decrements it, and it also decrements `total_claimable` in the same state write.

## 5.3 Pull Payment Flow

Settlement (every terminal path), in order:

1. Validate transition preconditions.
2. Compute `Distribution`.
3. Persist `status`, `resolution`, `distribution`.
4. Adjust `total_locked`, `total_claimable`, `claimable[recipient]`.
5. Emit `settled`.
6. Return. **No transfer occurs in this method.**

Withdrawal:

1. Validate `claimable[caller] > 0` and no withdrawal in progress.
2. Write `claimable[caller] = 0`, `withdrawals[caller] = WithdrawalRecord { amount, started_at: now }`.
3. `total_claimable -= amount`.
4. Create `Promise::new(caller).transfer(amount).then(on_withdraw)`.
5. Callback `#[private]`:
   - success → clear `withdrawals[caller]`, emit `withdraw_succeeded`;
   - failure → restore `claimable[caller] += amount`, `total_claimable += amount`, clear `withdrawals[caller]`, emit `withdraw_failed`.

Per STS §9, the settlement decision is never touched by a transfer outcome, and the recipient can simply retry.

---

# 6. Method Surface

Conventions:

- All mutating methods return `Result<T, ContractError>` via `#[handle_result]`, so errors surface as readable strings rather than opaque wasm panics.
- `#[payable]` is used only where a deposit is genuinely required: `create_project` and `top_up_reserve`.
- Every method validates authorization first, then state, then time, then inputs.

## 6.1 Mutating Methods

| # | Method | Actor | Required state | Payable | Effect |
|---|---|---|---|---|---|
| M-01 | `create_project` | anyone (becomes client) | — | ✅ | Creates project in `Funded`, `total_locked += amount` |
| M-02 | `accept_project` | freelancer | `Funded`, before `accept_deadline` | ❌ | → `Accepted` |
| M-03 | `reject_project` | freelancer | `Funded`, before `accept_deadline` | ❌ | → `Refunded`, 100% to client |
| M-04 | `claim_no_response_refund` | client | `Funded`, `now >= accept_deadline` | ❌ | → `Refunded`, 100% to client |
| M-05 | `submit_deliverable` | freelancer | `Accepted` **or** `RevisionRequested`, before active deadline | ❌ | → `Delivered`, new 72h review window |
| M-06 | `approve_deliverable` | client | `Delivered`, before `review_deadline` | ❌ | → `Released`, 100% to freelancer |
| M-07 | `request_revision` | client | `Delivered`, before `review_deadline`, `revision_count < 2` | ❌ | → `RevisionRequested`, `revision_count += 1`, `work_deadline = now + 3d` |
| M-08 | `claim_deadline_refund` | client | `Accepted`, `now >= work_deadline`, no deliverable ever submitted | ❌ | → `Refunded`, 100% to client |
| M-09 | `release_after_review` | any public caller | `Delivered`, `now >= review_deadline` | ❌ | → `Released`, 100% to freelancer |
| M-10 | `open_dispute` | client or freelancer | `Delivered` (before review expiry) or `RevisionRequested` (**no time limit**, D-14) | ❌ | → `Disputed`, `dispute_opened_at = now` |
| M-11 | `submit_evidence` | client or freelancer | `Disputed`, own evidence not yet submitted | ❌ | Stores `Evidence`; status unchanged |
| M-12 | `arbiter_resolve` | arbiter | `Disputed`, `percent <= 100`, `now <= dispute_opened_at + 7d` | ❌ | → `Resolved`, `ResolutionKind::Arbiter`, allocation |
| M-13 | `propose_settlement` | client or freelancer | `Disputed` | ❌ | Stores caller's proposal |
| M-14 | `settle_mutual` | client or freelancer | `Disputed`, **counterparty's proposal present and equal to the argument** | ❌ | → `Resolved`, `ResolutionKind::MutualSettlement`, allocation |
| M-15 | `withdraw` | account with `claimable > 0` | not already in progress | ❌ | Async transfer + callback |
| M-16 | `on_withdraw` | `#[private]` | — | — | Reconciles success/failure |
| M-17 | `top_up_reserve` | anyone | **always available, including while paused** | ✅ | Adds to storage reserve; emits event |
| M-18 | `set_paused` | owner | — | ❌ | Toggles `paused` (blocks `create_project` only) |
| M-19 | `clear_stuck_withdrawal` | owner | `started_at + 1h <= now` | ❌ | Restores a stuck claimable balance |
| M-20 | `migrate` | `#[private]` | after code upgrade | — | State migration |

## 6.2 Detailed Signatures

### M-01 `create_project`

```rust
#[payable]
pub fn create_project(
    &mut self,
    freelancer: AccountId,
    arbiter: AccountId,
    title: String,
    description: String,
    work_deadline: U64,   // nanoseconds, string on the wire; must be >= now + ACCEPT_WINDOW
) -> Result<u64, ContractError>
```

- `amount = env::attached_deposit()`.
- Validation order:
  1. `!paused` → else `E-15 ContractPaused`.
  2. `amount >= MIN_ESCROW` → else `E-04`.
  3. `freelancer != client`, `arbiter != client`, `freelancer != arbiter` → else `E-04`/`InvalidParticipants`.
  4. `title.len() in 1..=128`, `description.len() in 1..=2000`.
  5. `work_deadline.0 >= now + ACCEPT_WINDOW` → else `E-03`. A work deadline shorter than the acceptance window creates a project that can be accepted but never delivered: the freelancer accepts at hour 47 and the work deadline is already gone. The only exit would be a dispute, on a project where no work was ever possible. The floor makes that state unreachable.
  6. **Reserve pre-check (D-16):** `env::account_balance() - storage_stake_after_write - amount >= total_locked + total_claimable` → else `E-14 InsufficientReserve`. This runs **before** any state write, so the failure is a typed `E-14` rather than a generic panic from `assert_solvent()` at the end.

     `storage_stake_after_write` is **not** a value the SDK provides — `env::storage_usage()` only reports current usage. It must be computed from a fixed upper bound:

     ```rust
     /// Upper bound on the bytes a single project adds: all string fields at their
     /// maximum length, both index entries, and the claimable/withdrawal maps.
     /// Estimated from the field limits; MUST be re-measured once the contract exists
     /// (see AD-10) — do not treat it as an observed value.
     const MAX_PROJECT_STORAGE_BYTES: u64 = 6_000;

     fn storage_stake_after_write() -> Yocto {
         let bytes = env::storage_usage() as u128
             + MAX_PROJECT_STORAGE_BYTES as u128;
         env::storage_byte_cost().as_yoctonear().saturating_mul(bytes)
     }
     ```

     Consequences of using an upper bound: a project with short strings reserves more than it truly needs (conservative, safe), and the constant must be re-measured whenever `Project` gains a field. Both are acceptable; an unpredictable reserve check is not.
- `id = next_project_id`, then increment.
- `accept_deadline = now + 48h`.
- Writes project, appends `id` to all three indexes, `total_locked += amount`.
- Calls `assert_solvent()`.
- Emits `project_created`. No excess deposit is refunded — the whole attached deposit is escrow.

### M-05 `submit_deliverable`

```rust
pub fn submit_deliverable(
    &mut self,
    project_id: u64,
    link: String,
    note: String,
) -> Result<(), ContractError>
```

- `Accepted` branch: requires `now < work_deadline`; `revision_index = 0`; `revision_deadline` untouched (0).
- `RevisionRequested` branch: requires `now < revision_deadline`; `revision_index = revision_count`; `revision_deadline` reset to 0.
- Both branches: `link` 1..=512 bytes (else `InvalidDeliverable`), `note` 0..=1024 bytes.
- Sets `review_deadline = now + 72h`.
- Status → `Delivered`.

The old `review_deadline` is always overwritten on resubmission; it is never reused across a revision cycle.

### M-07 `request_revision`

```rust
pub fn request_revision(&mut self, project_id: u64) -> Result<(), ContractError>
```

- Guard: `Delivered`; `now < review_deadline`; `revision_count < 2`.
- `revision_count += 1`.
- `work_deadline = now + 3 days` — **relative to now**, not to the previous deadline (FSD §4.5 rule 4: "extends the work deadline by three days"). A revision requested on the last hour of the review window still grants a full three days of work.
- `revision_deadline = work_deadline`.
- `review_deadline = 0`.
- Status → `RevisionRequested`. No funds move.

### M-09 `release_after_review`

```rust
pub fn release_after_review(&mut self, project_id: u64) -> Result<(), ContractError>
```

- Callable by **anyone**, including the client, the freelancer, the arbiter, or an unrelated account (FSD §4.6 rule 4, STS §5.8).
- Guard: `Delivered`; `review_deadline != 0`; `now >= review_deadline`.
- Settlement to freelancer 100% (`release_after_review` maps to `Released`, not `Resolved`).

There is no incentive problem: the destination and amount are fixed, so the caller cannot redirect funds.

### M-10 `open_dispute`

```rust
pub fn open_dispute(&mut self, project_id: u64) -> Result<(), ContractError>
```

- Caller must be client or freelancer.
- `Delivered` branch: `now < review_deadline`.
- `RevisionRequested` branch: **no time condition** (D-14). A missed revision deadline must not block the dispute path, otherwise the project has no reachable transition at all.
- Status → `Disputed`, `dispute_opened_at = now`.
- Review and revision deadlines are cleared (0) so no time-based path can fire while disputed.

### M-12 `arbiter_resolve`

```rust
pub fn arbiter_resolve(
    &mut self,
    project_id: u64,
    freelancer_percent: u8,
) -> Result<(), ContractError>
```

- `freelancer_percent <= 100` → else `E-09`.
- Deadline: `now <= dispute_opened_at + 7 days` → else `E-13 ResolutionWindowExpired`.
- Status → `Resolved`, `ResolutionKind::Arbiter`, allocation via `settle_by_decision()`.
- Once `Resolved`, the decision is immutable — there is no method that writes `resolution` on a terminal project.

### M-13 / M-14 Mutual Settlement

```rust
pub fn propose_settlement(
    &mut self, project_id: u64, freelancer_percent: u8,
) -> Result<(), ContractError>

pub fn settle_mutual(
    &mut self, project_id: u64, freelancer_percent: u8,
) -> Result<(), ContractError>
```

Two-step design, so the parties never need to be online simultaneously:

1. `propose_settlement(p)` stores the value in the caller's own field (`client_settlement_proposal` or `freelancer_settlement_proposal`) and emits `settlement_proposed`. A party may overwrite its own proposal freely while the project is `Disputed`.
2. `settle_mutual(p)` is callable by either party and succeeds only when:
   - the counterparty's proposal exists,
   - it equals `p` exactly,
   - `p <= 100`.
   Otherwise the project stays `Disputed` and the call returns `E-16 SettlementMismatch`.

The FSD §5.11 rule "the same party cannot be counted as both approvals" is structurally satisfied: approvals live in two separate role-scoped fields, so a single account can never populate both.

**Consent semantics, stated exactly:** the `settle_mutual(p)` call *is* the caller's approval of `p`. The caller's own earlier proposal is superseded by that call and is not re-checked. Only the counterparty's stored proposal must equal `p`. This means a party may accept a percentage different from what they last proposed — that is a concession, not a violation — and it keeps the flow to a single transaction instead of "update proposal, then wait, then settle".

### M-15 `withdraw`

```rust
#[handle_result]
pub fn withdraw(&mut self) -> Result<Promise, ContractError>
```

**If the pinned SDK rejects `Promise` as the `Ok` type** (one-line compile check before writing the rest of the method), use the documented fallback: `pub fn withdraw(&mut self) -> Promise` that panics with the string `"E-19: withdrawal already in progress"` / `"E-18: nothing to withdraw"`. The frontend error mapper parses both `ExecutionError` text and `Err` payloads, so the mapping table in the Integration Specification §9 is identical either way. What is **not** acceptable is a bare `assert!` with no code — it is the only contract surface where the UI must tell the user why the withdraw button did nothing.

- No arguments; withdraws the caller's entire claimable balance.
- Requires `Gas >= 30 Tgas` (transfer 10 + callback 10 + buffer).
- `claimable == 0` → `E-18 NothingToWithdraw`.
- Already in progress → `E-19 WithdrawInProgress`.
- Marks in-progress and debits **before** creating the promise (P-8).
- Calls `assert_solvent()` after the debit, exactly like every other fund-moving path.

### M-17 `top_up_reserve`

```rust
#[payable]
pub fn top_up_reserve(&mut self) -> Result<(), ContractError>
```

- Accepts any positive deposit. No state change beyond the balance itself; emits `reserve_topped_up`.
- Open to anyone: donations are harmless and help keep `MIN_ESCROW` viable.

### M-19 `clear_stuck_withdrawal`

```rust
pub fn clear_stuck_withdrawal(&mut self, account: AccountId) -> Result<(), ContractError>
```

- Owner only.
- Guard: a `WithdrawalRecord` exists **and** `now >= record.started_at + 1 hour`.
- If **no** record exists → `Err(E-22 NoWithdrawalInProgress)`. It is never a silent no-op: P-6 (fail closed) applies to admin paths as much as to user paths, and a silent success would tell the operator a stuck withdrawal was cleared when nothing happened.
- If a record exists but the grace period has not elapsed → `Err(E-17 GraceNotElapsed)`.
- Restores `claimable[account] += record.amount`, `total_claimable += record.amount`, removes the record, emits `withdrawal_cleared`.

This exists solely for the case where the callback receipt ran out of gas (D-05). It cannot create money — it only re-credits a balance the contract itself debited. **Operational rule:** the owner must verify on a block explorer that the transfer did not actually land before calling this.

## 6.3 View Methods

| Method | Returns | Notes |
|---|---|---|
| `get_project(project_id)` | `Option<ProjectView>` | JSON DTO with `U128`/`U64` fields (§4.3.1) |
| `get_projects(account, role, from_index, limit)` | `Vec<ProjectView>` | `role` is the `Role` enum serialised **lowercase** (`"client"`, `"freelancer"`, `"arbiter"`); `limit` max 50 |
| `get_claimable(account)` | `U128` | Wrapped for JS safety |
| `get_claimable_per_project(account)` | `Vec<ProjectClaim>` | **Allocated** amount per project, derived from `distribution`. See the naming warning below. |
| `get_project_count()` | `u64` | |
| `get_stats(account)` | `Stats` | Reputation report, F-12 / P1-02 |
| `get_config()` | `Config` | Owner, `min_escrow`, all time windows, `paused` |
| `get_fund_invariants()` | `FundInvariants` | `locked`, `claimable`, `balance`, `storage_stake`, `ok` |
| `get_withdrawal_state(account)` | `Option<WithdrawalRecordView>` | JSON DTO (§4.3.1) — `amount` and `started_at` as strings |
| `get_state_version()` | `u16` | |

```rust
/// Wire format is lowercase: "client" | "freelancer" | "arbiter".
/// The Integration Specification types this as `'client' | 'freelancer' | 'arbiter'`
/// and sends those exact strings as the `role` argument of `get_projects`.
#[near(serializers = [json])]
#[serde(rename_all = "lowercase")]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Client,
    Freelancer,
    Arbiter,
}

#[near(serializers = [json])]
pub struct ProjectClaim {
    pub project_id: u64,
    pub role: Role,             // "client" | "freelancer"
    pub allocated: U128,        // total allocated to this account by this project
    pub status: ProjectStatus,  // status of the project at read time
}

#[near(serializers = [json])]
pub struct Stats {
    pub projects_as_client: u32,
    pub projects_as_freelancer: u32,
    pub completed_as_freelancer: u32,   // Released or Resolved with share > 0
    pub total_value_as_freelancer: U128,
    pub disputes_as_freelancer: u32,
    pub disputes_released_fully: u32,   // percent == 100
    pub refunds_as_client: u32,
}

#[near(serializers = [json])]
pub struct Config {
    pub owner: AccountId,
    pub min_escrow: U128,
    pub accept_window: U64,
    pub review_window: U64,
    pub revision_extension: U64,
    pub arbiter_window: U64,
    pub max_revisions: u8,
    pub paused: bool,
    pub state_version: u16,
}

#[near(serializers = [json])]
pub struct FundInvariants {
    pub locked: U128,
    pub claimable: U128,
    pub balance: U128,        // env::account_balance().as_yoctonear()
    pub storage_stake: U128,  // storage_byte_cost() * storage_usage()
    pub spendable: U128,      // balance - storage_stake (saturating)
    pub ok: bool,             // spendable >= locked + claimable
}
```

All four view structs are `#[near(serializers = [json])]` only — they are never stored, so they carry no Borsh requirement and no `near_sdk::store` key constraint.

**`total_value_as_freelancer` is defined as the sum of `distribution.freelancer_amount` over the account's terminal projects — i.e. what was *allocated* to the freelancer, not what has been *withdrawn*.** The two diverge between settlement and withdrawal, and allocated is the honest figure for a reputation profile: it is decided on-chain and cannot be reduced by forgetting to press Withdraw. The UI must label it "Total earned", not "Total received".

**Naming warning — `allocated` is not "still withdrawable".** Because `claimable` is stored per account and not per project, the contract cannot know which project a past withdrawal came from. `get_claimable_per_project` therefore returns the **historical allocation** per project, and its values do **not** decrease when the account withdraws. The authoritative "money you can still withdraw right now" figure is `get_claimable(account)` and nothing else. The frontend must never sum `allocated` and present it as a withdrawable balance.

Statistics are computed on the fly from the per-account indexes plus each project's `distribution`. No separate counters are stored (they would drift). Because index iteration is linear, `get_stats` is documented as non-paginated and intended for accounts with a modest project history; the frontend may cap displayed values.

---

# 7. Access Control

## 7.1 Assertion Helpers

```rust
fn predecessor() -> AccountId { env::predecessor_account_id() }

fn assert_client(p: &Project)     { require!(predecessor() == p.client, Unauthorized); }
fn assert_freelancer(p: &Project) { require!(predecessor() == p.freelancer, Unauthorized); }
fn assert_arbiter(p: &Project)    { require!(predecessor() == p.arbiter, Unauthorized); }
fn assert_party(p: &Project) {
    let who = predecessor();
    require!(who == p.client || who == p.freelancer, Unauthorized);
}
fn assert_state(p: &Project, expected: ProjectStatus) {
    require!(p.status == expected, InvalidProjectState);
}
fn assert_not_terminal(p: &Project) {
    require!(!p.status.is_terminal(), AlreadyFinalized);
}
fn assert_owner(&self) { require!(predecessor() == self.owner, NotOwner); }
```

`require!` expands to `if !cond { return Err(ContractError::Variant) }`, keeping every failure typed and revertable.

## 7.2 Actor × Action Matrix

| Action | Client | Freelancer | Arbiter | Public | Owner |
|---|:--:|:--:|:--:|:--:|:--:|
| `create_project` | ✅ (as client) | — | — | ✅ | ✅ |
| `accept_project` | ❌ | ✅ | ❌ | ❌ | ❌ |
| `reject_project` | ❌ | ✅ | ❌ | ❌ | ❌ |
| `claim_no_response_refund` | ✅ | ❌ | ❌ | ❌ | ❌ |
| `submit_deliverable` | ❌ | ✅ | ❌ | ❌ | ❌ |
| `approve_deliverable` | ✅ | ❌ | ❌ | ❌ | ❌ |
| `request_revision` | ✅ | ❌ | ❌ | ❌ | ❌ |
| `claim_deadline_refund` | ✅ | ❌ | ❌ | ❌ | ❌ |
| `release_after_review` | ✅ | ✅ | ✅ | ✅ | ✅ |
| `open_dispute` | ✅ | ✅ | ❌ | ❌ | ❌ |
| `submit_evidence` | ✅ | ✅ | ❌ | ❌ | ❌ |
| `arbiter_resolve` | ❌ | ❌ | ✅ | ❌ | ❌ |
| `propose_settlement` | ✅ | ✅ | ❌ | ❌ | ❌ |
| `settle_mutual` | ✅ | ✅ | ❌ | ❌ | ❌ |
| `withdraw` | ✅ if balance | ✅ if balance | ✅ if balance | ✅ if balance | ✅ if balance |
| `top_up_reserve` | ✅ | ✅ | ✅ | ✅ | ✅ |
| `set_paused` / `clear_stuck_withdrawal` / `migrate` | ❌ | ❌ | ❌ | ❌ | ✅ |

## 7.3 Identity Rules

1. Client, freelancer and arbiter must be three distinct accounts, validated at `create_project`.
2. Both implicit and named accounts are valid; `AccountId` handles format validation. **The frontend must use an equivalent validator, not a regex that only matches named accounts** — a 64-character hex implicit account is legal and a named-only pattern would reject it before the user reached the wallet (Integration Specification §6.4).
3. The arbiter cannot be replaced after acceptance — enforced by the absence of any method that writes `project.arbiter`.
4. The client cannot replace the freelancer or arbiter for the same reason.
5. While `Funded`, the arbiter has no reachable method (no dispute can be opened before a deliverable).

---

# 8. Fund Mechanics

## 8.1 Deposit

`create_project` converts the entire attached deposit into `project.amount` and `total_locked`.

- Native NEAR only (FSD §4.1).
- No platform fee: 0%.
- No separate `amount` parameter (D-06) — the deposit *is* the amount, which removes an entire class of "wrong deposit" bugs.
- `MIN_ESCROW = 0.1 NEAR` (D-01).

## 8.2 Allocation Formula

Two internal helpers split the two genuinely different cases. **A rule-based settlement never writes a `Resolution`** — there is no arbiter and no negotiation, so a `resolved_by` value would be a lie.

```rust
/// Shared allocation + accounting. Percent is the freelancer's share.
fn allocate(&mut self, p: &mut Project, freelancer_percent: u8) {
    // floor to the freelancer, remainder to the client
    let f: u128 = (p.amount / 100).checked_mul(freelancer_percent as u128).unwrap()
        + (p.amount % 100).checked_mul(freelancer_percent as u128).unwrap() / 100;
    let c: u128 = p.amount.checked_sub(f).unwrap();

    debug_assert_eq!(f.checked_add(c).unwrap(), p.amount);

    p.distribution = Some(Distribution { freelancer_amount: f, client_amount: c });

    self.total_locked    = self.total_locked.checked_sub(p.amount).unwrap();
    self.total_claimable = self.total_claimable.checked_add(p.amount).unwrap();

    if f > 0 { self.credit(&p.freelancer, f); }
    if c > 0 { self.credit(&p.client, c); }

    self.assert_solvent();
}

/// Rule-based paths: client approve, automatic release, reject, all refunds.
/// Percent is fixed by the rules (100 or 0), so `resolution` stays None.
fn settle_by_rule(&mut self, p: &mut Project, freelancer_percent: u8) {
    debug_assert!(freelancer_percent == 0 || freelancer_percent == 100);
    p.resolution = None;
    self.allocate(p, freelancer_percent);
}

/// Dispute paths: arbiter decision or mutual settlement.
/// These are the only paths that produce a `Resolution`.
fn settle_by_decision(&mut self, p: &mut Project, freelancer_percent: u8, by: ResolutionKind) {
    p.resolution = Some(Resolution {
        freelancer_percent,
        resolved_by: by,
        resolved_at: env::block_timestamp(),
    });
    self.allocate(p, freelancer_percent);
}
```

**Rule:** `project.resolution` is `Some` **only** for `DISPUTED → RESOLVED`. For `RELEASED` and `REFUNDED` the split is fully described by `distribution` plus the status, and `resolution` stays `None`. The frontend renders the "who decided" block only when `resolution != null`.

**Rounding rule:** `freelancer_amount = floor(amount × percent / 100)`; the client receives the remainder. Rounding residue always favours the client, which guarantees the split can never exceed the escrow. The division-then-multiplication order avoids `u128` overflow on large amounts.

Example: `amount = 10 NEAR`, `percent = 33` → freelancer `3.3`, client `6.7`.

Zero-percent and hundred-percent cases produce a single recipient; the helper must not credit a zero balance (it would create a pointless storage entry).

## 8.3 Refund Paths

Refunds reuse `settle_by_rule()` with `freelancer_percent = 0`, then set status to `Refunded` (no `Resolution` is written, per D-11):

| Trigger | Freelancer % | Resulting status |
|---|---|---|
| `reject_project` | 0 | `Refunded` |
| `claim_no_response_refund` | 0 | `Refunded` |
| `claim_deadline_refund` | 0 | `Refunded` |

`Refunded` (not `Resolved`) matches STS §9.2.

## 8.4 Withdrawal Constants

```rust
const GAS_FOR_WITHDRAW: Gas = Gas::from_tgas(10);
const GAS_FOR_CALLBACK: Gas = Gas::from_tgas(10);
const MIN_WITHDRAW_GAS: Gas = Gas::from_tgas(30);
const STUCK_WITHDRAWAL_GRACE_NS: u64 = 3_600 * NS_PER_SEC; // 1 hour
```

The callback verifies `env::current_account_id() == env::predecessor_account_id()` before mutating state.

**Known risk (§17 D-05):** if the callback receipt itself runs out of gas, the in-progress record can persist. Mitigation stack: (a) generous gas budget, (b) `MIN_WITHDRAW_GAS` enforced on entry, (c) owner-only `clear_stuck_withdrawal` after a one-hour grace period, (d) events emitted for both outcomes so off-chain monitoring can detect a stuck account.

---

# 9. Time Rules

```rust
const NS_PER_SEC: u64 = 1_000_000_000;
const ACCEPT_WINDOW:      u64 = 48 * 3_600 * NS_PER_SEC;      // 48 hours
const REVIEW_WINDOW:      u64 = 72 * 3_600 * NS_PER_SEC;      // 72 hours
const REVISION_EXTENSION: u64 = 3 * 24 * 3_600 * NS_PER_SEC;  // 3 days
const ARBITER_WINDOW:     u64 = 7 * 24 * 3_600 * NS_PER_SEC;  // 7 days
const MAX_REVISIONS:      u8  = 2;
```

Rules:

1. Expiry is `now >= deadline`; validity is `now < deadline` (STS §6.1: a deadline is expired when the timestamp reaches or exceeds it).
2. `deadline == 0` means "not applicable" and must never pass a time check. Every time guard asserts the field is non-zero first.
3. No transition fires by itself. Every time-based path requires a transaction from an authorized actor or any public caller.
4. `release_after_review` may be called by the client too; the outcome is identical, so this is not treated as a conflict of interest.
5. The frontend countdown is cosmetic. Contract timestamps are authoritative even if the UI is stale.

---

# 10. Events (NEP-297)

Format:

```
EVENT_JSON:{"standard":"near_escrow","version":"1.0.0","event":"<name>","data":{...}}
```

| Event | Emitted by | Data |
|---|---|---|
| `project_created` | M-01 | `project_id`, `client`, `freelancer`, `arbiter`, `amount`, `work_deadline` |
| `project_accepted` | M-02 | `project_id`, `freelancer` |
| `project_rejected` | M-03 | `project_id`, `freelancer` |
| `deliverable_submitted` | M-05 | `project_id`, `revision_index`, `review_deadline` |
| `revision_requested` | M-07 | `project_id`, `revision_count`, `work_deadline` |
| `dispute_opened` | M-10 | `project_id`, `opened_by` |
| `evidence_submitted` | M-11 | `project_id`, `by`, `role` |
| `settlement_proposed` | M-13 | `project_id`, `by`, `freelancer_percent` |
| `settled` | all terminal paths | `project_id`, `status`, `freelancer_amount`, `client_amount`, `resolved_by` |
| `withdraw_succeeded` | M-16 | `account`, `amount` |
| `withdraw_failed` | M-16 | `account`, `amount` |
| `withdrawal_cleared` | M-19 | `account`, `amount`, `by` |
| `reserve_topped_up` | M-17 | `account`, `amount` |
| `paused` / `unpaused` | M-18 | `by` |

Events drive UI feedback and off-chain monitoring. They are **not** the source of truth: the frontend must still read the project from a view call before rendering a final state.

---

# 11. Error Handling

```rust
#[derive(Debug, PartialEq, Clone)]
pub enum ContractError {
    // authorization
    Unauthorized,                // E-01
    NotOwner,                    // E-01
    // state
    InvalidProjectState,         // E-02
    AlreadyFinalized,            // E-08
    // time
    DeadlineExpired,             // E-03
    ReviewPeriodNotExpired,      // E-11
    ResolutionWindowExpired,     // E-13
    GraceNotElapsed,             // E-17
    // amounts
    InvalidEscrowAmount,         // E-04
    InvalidSettlementPercentage, // E-09
    InsufficientReserve,         // E-14
    // flow
    AlreadyAccepted,             // E-05
    RevisionLimitExceeded,       // E-06
    DisputeAlreadyActive,        // E-07
    DisputeNotAllowed,           // E-20
    EvidenceAlreadySubmitted,    // E-10
    SettlementMismatch,          // E-16
    // input validation
    InvalidParticipants,         // E-04
    InvalidDeliverable,          // E-10
    InvalidEvidence,             // E-10
    InvalidInput,                // E-10
    // funds
    PayoutFailed,                // E-12 (event only, see note)
    NothingToWithdraw,           // E-18
    WithdrawInProgress,          // E-19
    NoWithdrawalInProgress,      // E-22 (admin path only)
    // admin
    ContractPaused,              // E-15
    UnknownProject,              // E-21
}
```

`Display` must embed the FSD code so QA and the frontend can map failures deterministically:

```
"E-04: invalid escrow amount"
```

**The error type derives `FunctionError`.** `#[handle_result]` routes an `Err` through `FunctionError::panic(&err)`, and the reference implementation derives that trait on `ContractError`, whose `panic` always goes to `env::panic_str(&self.to_string())`. `AsRef<str>` is deliberately **not** implemented: near-sdk has a blanket `impl<T: AsRef<str>> FunctionError for T` that only calls `env::panic_str` under `cfg(near)` and otherwise does `eprintln!` + `panic!()`, which reports as an opaque wasm trap with no code at all (D-26). Deriving also collides with that blanket impl, so the two cannot coexist.

The message that reaches a caller is therefore:

```
Smart contract panicked: E-04: invalid escrow amount
```

Codes `E-01`–`E-12` come from FSD §7. `E-13`–`E-22` are additions introduced by this specification and must be documented in the frontend error catalogue.

## 11.1 Error Selection Precedence

Several variants could plausibly fire for one call. The contract resolves this with a fixed priority so the frontend message is always the most specific true statement:

| Priority | Condition | Variant | Code |
|---:|---|---|---|
| 1 | `project_id` is not in storage | `UnknownProject` | E-21 |
| 2 | Caller is not authorized | `Unauthorized` / `NotOwner` | E-01 |
| 3 | Contract paused (only where it applies) | `ContractPaused` | E-15 |
| 4 | Project is in a terminal status | `AlreadyFinalized` | E-08 |
| 5 | Status matches a more specific flow error | `AlreadyAccepted`, `DisputeAlreadyActive` | E-05, E-07 |
| 6 | Status is legal for the method but cannot accept this action | `DisputeNotAllowed` | E-20 |
| 7 | Time condition violated | `DeadlineExpired`, `ReviewPeriodNotExpired`, `ResolutionWindowExpired`, `GraceNotElapsed` | E-03, E-11, E-13, E-17 |
| 8 | Any other status mismatch | `InvalidProjectState` | E-02 |
| 9 | Input validation | `InvalidEscrowAmount`, `InvalidParticipants`, `InvalidDeliverable`, `InvalidEvidence`, `InvalidInput`, `InvalidSettlementPercentage` | E-04, E-09, E-10 |
| 10 | Reserve too thin | `InsufficientReserve` | E-14 |
| 11 | Admin path targets a non-existent withdrawal record | `NoWithdrawalInProgress` | E-22 |

**Scope of this table.** It resolves ambiguity when several failure groups fire at once and the method's own order does not already separate them — for example paused **and** deposit below minimum → `E-15`, because paused outranks input validation.

For any single method, **the validation order in §6.2 is authoritative**, and it may interleave these groups. `create_project` checks paused → amount → participants → strings → deadline → reserve, so an invalid amount (`E-04`) is reported before an expired deadline (`E-03`) even though time ranks above input validation in this table. That is deliberate: an unusable amount is the more actionable complaint. `submit_deliverable` does the reverse — it checks the deadline before the link, because a late submission cannot be fixed by editing a URL.

Concretely:

- `accept_project` on an `Accepted` project → `AlreadyAccepted` (E-05), not a generic E-02.
- `reject_project` on an `Accepted` project → `AlreadyAccepted` (E-05).
- `open_dispute` on a `Disputed` project → `DisputeAlreadyActive` (E-07).
- `open_dispute` on `Funded` / `Accepted` → `DisputeNotAllowed` (E-20).
- Any method on `Released` / `Refunded` / `Resolved` → `AlreadyFinalized` (E-08), even if the status would otherwise read as a plain mismatch.
- Any mutating method with an unknown `project_id` → `UnknownProject` (E-21).

## 11.2 `E-12 PayoutFailed` Is Event-Only

`withdraw()` never returns `PayoutFailed`. It debits the balance and returns a `Promise`; the transfer outcome is known only in `on_withdraw`. `E-12` therefore appears **exclusively** in the `withdraw_failed` event payload, and the frontend surfaces it from the event/refetch, not from a transaction failure. Testing this code means asserting the event (WD-07), not an `Err` return.

Frontend obligation (FSD §7): never show a bare code — always pair it with a human sentence.

---

# 12. Security Invariants

## 12.1 State Invariants

1. Exactly one `ProjectStatus` per project at any time (enforced by the type).
2. A terminal status never transitions back — no code path writes `status` on a terminal project.
3. Every transition asserts its required source state.
4. No transition succeeds twice: the source state no longer matches on the second attempt, and terminal states are unreachable again.
5. `Disputed` blocks `release_after_review`, `approve_deliverable` and `request_revision` by state guard.
6. `Released`, `Refunded`, `Resolved` cannot open a dispute.
7. No project can enter a state with zero reachable transitions. Specifically, `REVISION_REQUESTED` remains disputable after its revision deadline (D-14) — this is the only guard against a self-inflicted permanent lock.

## 12.2 Fund Invariants

1. No funds are claimable before a settlement exists (`claimable` is written only inside `allocate()`, which only `settle_by_rule()` and `settle_by_decision()` call).
2. `freelancer_amount + client_amount == project.amount` at every settlement (`debug_assert` plus deterministic arithmetic).
3. Funds are never claimed twice (`withdraw` zeroes the balance before transferring).
4. Distribution is immutable after settlement (written once inside `allocate()`, no later writer).
5. The arbiter cannot revise a finalized decision (no method writes `resolution` post-terminal).
6. Unclaimed funds stay recorded as `claimable` liabilities, visible via `get_fund_invariants()`.
7. `total_locked + total_claimable <= spendable_balance` after every fund-moving call.

## 12.3 Identity Invariants

1. Three distinct accounts per project.
2. A freelancer only acts on projects assigned to them (guard reads `project.freelancer`).
3. A client only acts within client permissions.
4. Only the registered arbiter finalizes an arbiter decision.
5. Arbiter and freelancer are immutable after creation.

## 12.4 Revision Invariants

1. `revision_count <= 2`.
2. Each `request_revision` increments by exactly one.
3. Each revision grants exactly three days from the request.
4. Each resubmission starts a fresh 72-hour review window.
5. A missed revision deadline never triggers a refund — only `open_dispute` is available.

## 12.5 Verification Strategy

- Unit tests assert each invariant after each scenario.
- A dedicated `assert_solvent()` runs inside the contract, so an invariant breach aborts the transaction atomically.
- Integration tests run randomized action sequences and then check `get_fund_invariants()`.

---

# 13. Testing Strategy

Three levels (granular cases belong to document 05):

1. **Unit tests** with `VMContextBuilder` — every guard, every error variant, time manipulation via `context.block_timestamp`.
2. **Integration tests** with `near-workspaces` — deploy to sandbox, exercise real async `withdraw` and callback behaviour, assert balances from `view_account`.
3. **Invariant tests** — randomized sequences ending with a solvency and distribution check.

Minimum coverage targets:

- one positive test per FSD feature `F-01`…`F-15`;
- one negative test per error code `E-01`…`E-22`;
- one test per STS transition (**13** transitions, including both `FUNDED → REFUNDED` paths and both `DISPUTED → RESOLVED` paths);
- one test per state invariant and per fund invariant.

Expected volume: the Test Plan v1.5 catalogue is the authority — **138 tests, 102 P0**.

Mandatory cases (historically the ones that leak):

- double `withdraw` → second call fails;
- `release_after_review` before expiry → rejected;
- `arbiter_resolve` by a non-arbiter → rejected;
- third `request_revision` → rejected;
- `settle_mutual` with mismatched percentages → stays `Disputed`;
- failed transfer → balance restored and retryable;
- `arbiter_resolve` after 7 days → `E-13`;
- `create_project` with duplicate participants → rejected.

---

# 14. Build, Deploy and Upgrade

```bash
# build
cargo near build

# deploy
near deploy escrow.<account>.testnet \
  target/near/nescrow/nescrow.wasm \
  --initFunction 'new' \
  --initArgs '{"owner":"<account>.testnet"}'

# fund the storage reserve (required, see D-01)
near call escrow.<account>.testnet top_up_reserve '{}' \
  --accountId <account>.testnet --deposit 10
```

```rust
#[init]
pub fn new(owner: AccountId) -> Self { ... }
```

Upgrade:

```rust
#[init(ignore_state)]
#[private]
pub fn migrate() -> Self {
    let old: OldContract = env::state_read().expect("no state");
    // transform
}
```

Rules:

- `state_version` increments on every migration.
- `migrate` is `#[private]` — callable only by the contract itself, triggered through the deploy transaction.
- Re-deployment must use the contract's own account.
- After any migration, `get_fund_invariants()` must return `ok: true` before traffic resumes.

---

# 15. Traceability Matrix

| FSD | STS transition | Method |
|---|---|---|
| F-01 Wallet Connection | — | frontend |
| F-02 / F-03 Create + Fund | → `FUNDED` | M-01, M-17 |
| F-04 Accept | 5.1 | M-02 |
| F-05 Reject | 5.2A | M-03 |
| — no response refund | 5.2B | M-04 |
| F-06 Submit Deliverable | 5.3, 5.9 | M-05 |
| F-07 Approve | 5.5 | M-06 |
| F-08 Request Revision | 5.6 | M-07 |
| — deadline miss refund | 5.4 | M-08 |
| F-09 Automatic Release | 5.8 | M-09 |
| F-11 Open Dispute | 5.7, 5.10 | M-10 |
| F-12 Submit Evidence | — | M-11 |
| F-13 Arbiter Resolution | 5.11 | M-12 |
| F-14 Mutual Settlement | 5.12 | M-13, M-14 |
| F-10 Claim Refund | 5.2B, 5.4 | M-04, M-08 |
| §9 Withdrawal | §9 | M-15, M-16, M-19 |
| F-15 Dashboard | — | §6.3 views |
| F-12 (PRD) / P1-02 Rapor | — | `get_stats` |

---

# 16. Out of Scope for This Document

1. Final UI copy for each error (document 06).
2. AI dispute summary hosting and prompt design (document 04).
3. Granular test case listing (document 05).
4. FT/USDC payment support (P1-03).
5. Index pagination redesign for accounts with thousands of projects.

---

# 17. Decisions Record

All items below were open in the review draft and are now **final**.

### D-01 — Storage cost is absorbed by the contract, funded by a pre-funded reserve

Escrow amount equals the attached deposit; storage staking is paid from a contract-level reserve topped up via `top_up_reserve()`. `MIN_ESCROW = 0.1 NEAR`.

*Rationale:* keeps the user-facing invariant trivial ("deposit X, X is locked") and avoids a second number in the UI plus refund-of-excess logic, which is a common source of escrow bugs.

*Rejected alternative:* deposit = escrow + storage buffer. More economically precise, but adds UI complexity and a refund path for the residual.

*Accepted cost:* the operator must fund a reserve (10 NEAR recommended for demo volume) and monitor `get_fund_invariants()`. Failure mode is explicit and safe: `E-14 InsufficientReserve`, never silent fund erosion.

### D-02 — `pause()` exists, owner-only, narrow scope

`set_paused(true)` blocks `create_project` only. Accept, deliver, approve, dispute, resolve and withdraw all keep working.

*Rationale:* a safety valve for demo incidents without giving the owner any power over funds in flight. Pausing withdrawals or settlements would hand the owner a de-facto custodian role, contradicting the product's core claim.

### D-03 — The arbiter's 7-day window is enforced; after it, only mutual settlement remains

`arbiter_resolve` returns `E-13` once `now > dispute_opened_at + 7 days`. Funds then stay locked unless both parties reach a mutual settlement.

*Rationale:* STS §6.2 is explicit and the FSD §10.3 already documents locked funds as an acknowledged MVP limitation. Allowing the arbiter to decide at any time would make the seven-day rule meaningless.

*Rejected alternative:* an owner `force_resolve` escape hatch — rejected because it would concentrate control over user funds in one key, which is exactly what this product exists to avoid.

*Consequence, disclosed in UI and README:* an inactive arbiter plus a non-cooperative counterparty can leave funds locked indefinitely. This is a known, documented limitation, not an implementation gap.

### D-04 — Reputation stats ship in MVP, computed on-chain

`get_stats(account)` exists in the MVP. It derives `completed_as_freelancer`, `total_value_as_freelancer`, `disputes_as_freelancer` and `refunds_as_client` from the indexes.

*Rationale:* PRD places it in P0 (F-12) while FSD places it in P1 (P1-02). The computation is cheap, it is a strong demo beat (STS/PRD §15 step 4, 15 seconds of the two-minute demo), and it cannot drift because nothing is cached.

*Cost:* `get_stats` is linear in project count; documented as non-paginated and acceptable at MVP scale.

### D-05 — `clear_stuck_withdrawal` exists, owner-only, one-hour grace

Recovery path for a callback that failed to execute. It re-credits a balance the contract itself debited, so it can never create money.

*Operational rule:* the owner must confirm on a block explorer that the transfer did not land, then wait out the one-hour grace period. The grace period exists so the call can never race a still-pending transfer.

### D-06 — The attached deposit *is* the escrow amount

No separate `amount` parameter.

*Rationale:* removes the highest-frequency escrow bug class (deposit ≠ declared amount), and eliminates refund-of-excess handling. Frontend passes the amount as the deposit, nothing else.

### D-07 — Mutual settlement is available only in `Disputed`

No pre-dispute "mutual cancellation". That belongs to P2 (FSD §2.3) and is not implemented.

### D-08 — Rounding favours the client

`freelancer = floor(amount × percent / 100)`, remainder to the client.

*Rationale:* guarantees `sum == amount` with no overflow and no possibility of allocating more than the escrow. A one-yocto asymmetry is economically irrelevant but must be deterministic and documented.

### D-09 — Revision deadline extends from the request time

`work_deadline = now + 3 days` at `request_revision`, not `old_deadline + 3 days`.

*Rationale:* FSD §4.5 rule 4 states the deadline is extended by three days from the request. This also protects the freelancer from a nominal extension that has already elapsed.

### D-10 — Projects are never deleted

No project deletion or archiving method; storage is never reclaimed.

*Rationale:* on-chain history is the product's proof-of-work. Deleting would allow reputation laundering.

*Consequence:* storage grows monotonically; the reserve must be sized accordingly.

### D-11 — Rule-based settlements carry no `Resolution`

`resolution` is populated **only** on `DISPUTED → RESOLVED`. Client approval, automatic release, rejection and all refunds write `distribution` and nothing else.

*Rationale:* `Resolution.resolved_by` answers "who decided the dispute". Approvals and refunds are not decisions by anyone — filling that field would put a false statement on-chain and force the UI to special-case it. Two helpers (`settle_by_rule`, `settle_by_decision`) make the distinction structural instead of a convention.

### D-12 — Errors split by meaning, with a fixed precedence order

`DisputeNotAllowed` moved from E-10 to **E-20**, `UnknownProject` from E-02 to **E-21**, and §11.1 defines the exact priority used to pick one variant per failure.

*Rationale:* sharing a code across unrelated conditions produces actively wrong UI copy — a dispute opened too early would have told the user "your evidence is invalid". A precedence table also removes ambiguity for implementers, who otherwise guess which guard fires first.

### D-13 — `E-12 PayoutFailed` is event-only

Documented as event-only rather than a returned error (§11.2).

*Rationale:* `withdraw()` returns a `Promise`; the transfer outcome exists only in the callback. Pretending it can be an `Err` would lead a test author to write an impossible test.

### D-14 — A missed revision deadline never blocks a dispute

`open_dispute` has no time guard in `REVISION_REQUESTED`. It may be called before or after `revision_deadline`.

*Rationale:* without this, a silent freelancer produces a permanent fund lock with no bad actor involved. After the revision deadline, `submit_deliverable` fails (E-03), `claim_deadline_refund` fails (E-02) and mutual settlement is unreachable because it requires `DISPUTED` — so the escrow could never be released, refunded or arbitrated. The STS originally carried a "revision deadline has not expired" precondition while its own note said the client may still open a dispute; the precondition was the bug.

*Consequence:* a project can sit in `REVISION_REQUESTED` indefinitely and still be disputed later. That is the intended safety valve, not an oversight.

### D-15 — JSON DTOs with `U128`/`U64`, internal state stays native

View methods return `ProjectView` and friends, never the internal `Project`. Every integer that can exceed `2^53` crosses the boundary as a string wrapper.

*Rationale:* `#[near(serializers = [json])]` on `u128`/`u64` emits JSON **numbers**. Escrow amounts (~`1e25`) and nanosecond timestamps (~`1.7e18`) both exceed `Number.MAX_SAFE_INTEGER`, so `JSON.parse` in the browser truncates them silently. Rust unit tests never see this — the corruption happens after the value leaves the contract. A JSON-level assertion is the only test that catches it.

### D-16 — Reserve pre-check before any state write

`create_project` verifies the reserve explicitly (step 6) and returns `E-14` instead of relying on the final `assert_solvent()`.

*Rationale:* `assert_solvent()` is a safety net, not an error contract. Letting it fire first means the user sees an opaque panic where the specification promised `E-14`, and AD-01 could never pass.

### D-17 — `withdraw` returns `Result<Promise, ContractError>`, with a documented fallback

Preferred form is `#[handle_result] pub fn withdraw(&mut self) -> Result<Promise, ContractError>`. If the pinned SDK rejects `Promise` as the `Ok` type, fall back to a `Promise`-returning method that panics with the string `"E-1x: message"`.

*Rationale:* `E-18`/`E-19` are user-visible states that must reach the UI with a readable code. The fallback keeps the same wire contract for the frontend, which parses both `Err` payloads and `ExecutionError` text. A bare `assert!` with no code is explicitly rejected.

### D-18 — `Role` is lowercase on the wire

`Role` is serialised with `#[serde(rename_all = "lowercase")]`, so the `get_projects` argument and `ProjectClaim.role` are `"client" | "freelancer" | "arbiter"`.

*Rationale:* the Integration Specification already types and sends lowercase. One casing, used by both the argument and the response, removes a deserialisation failure that would look like a broken dashboard.

### D-19 — `clear_stuck_withdrawal` fails loudly when nothing is stuck

Returns `E-22 NoWithdrawalInProgress`.

*Rationale:* fail-closed applies to admin paths too. A silent `Ok` would tell the operator the stuck withdrawal was handled when nothing happened, which is the worst possible outcome for a recovery function.

---

### D-20 — `work_deadline` must be at least the acceptance window away

`create_project` requires `work_deadline >= now + ACCEPT_WINDOW`.

*Rationale:* without the floor, a client can create a project whose work deadline falls inside the 48-hour acceptance window. If the freelancer accepts at hour 47, `submit_deliverable` can never succeed, no refund path applies (a deliverable is expected but impossible), and the only exit is a dispute. The floor makes the born-dead project unreachable.

### D-21 — `settle_mutual` compares only the counterparty's proposal

The percentage passed to `settle_mutual` is compared against the **counterparty's** stored proposal. The caller's own proposal is superseded by the call and is not re-checked.

*Rationale:* this is what makes "a party concedes" expressible in one transaction. Two differing stored proposals are not a dead end — one side calls `settle_mutual` with the other side's number and the dispute closes. FSD §4.9 originally read as "different proposals block settlement", which contradicted the very mechanism it described; MS-08 locks the corrected behaviour.

### D-22 — Method parameters use `U64`/`U128` too, not just return values

`create_project` takes `work_deadline: U64`. Every method argument that carries a `u128`/`u64` is wrapped the same way.

**Scope:** the wrapper applies to every argument whose value can exceed `2^53` — i.e. amounts and nanosecond timestamps. Small pagination arguments (`from_index`, `limit`, `project_id`, percentages) stay plain `u64`/`u8`, because converting them buys nothing and makes the frontend payload noisier.

*Rationale:* D-15 only fixed the output direction. Inbound it matters just as much, for two independent reasons: near-sdk deserialises `u64` from a JSON **number**, so a string payload fails outright with `expected u64, found string` and `create_project` could never be called; and a JS number above `2^53` is already truncated before it is sent, so even a number-shaped payload would carry a corrupted deadline. Input and output must use the same wrapper.

### D-23 — The reserve check uses a declared storage upper bound

`MAX_PROJECT_STORAGE_BYTES` (6,000) is a constant; `storage_stake_after_write = storage_byte_cost() × (storage_usage() + MAX_PROJECT_STORAGE_BYTES)`.

*Rationale:* "storage usage after the write" is not a value the SDK exposes — `env::storage_usage()` reports current usage only. Without a fixed bound the check cannot be written at all, so an implementer would fall back on the final `assert_solvent()` and produce a panic where `E-14` was promised, failing AD-01. The bound is conservative (over-reserves slightly for short strings) and must be re-measured whenever `Project` gains a field.

### D-25 — Compiler-verified corrections to the snippets

The first slice was compiled against `near-sdk` 5.30.1 with `cargo check --target wasm32-unknown-unknown`. Four things in this document's code blocks were wrong and are now fixed in the contract:

| Was | Compiler said | Now |
|---|---|---|
| `#[near_sdk::serde(rename_all = "lowercase")]` | `cannot find serde in near_sdk` | plain `#[serde(rename_all = "lowercase")]` — `serde` is a direct dependency, and near-sdk 5.30 does not re-export the module |
| `LookupMap::new(b"p")` | `&[u8; 2]` does not satisfy `IntoStorageKey` | `LookupMap::new(b"p".to_vec())` |
| `map.insert(key, &value)` | expected `Project`, found `&Project` | `store::LookupMap::insert` takes the value **by value** |
| `Result<T, ContractError>` with `#[handle_result]` | `ContractError` does not implement `FunctionError` | implement `AsRef<str>` returning a `&'static str` per variant, with the error code baked in |

*Rationale for recording this at all:* a specification whose code blocks do not compile is worse than no code blocks — it invites copy-paste bugs. This entry is the boundary line between "reviewed by reading" and "verified by toolchain", and it replaces a sixth documentation audit with `cargo check`.

Also confirmed by the build, not by inspection: `env::account_balance() -> NearToken` and `env::storage_byte_cost() -> NearToken` (D-24 holds), and the release artifact is 176 KB, under the 300 KB budget in §12.

### D-24 — Money is `u128` internally, `NearToken` only at the SDK boundary

Internal state uses `pub type Yocto = u128`. Values from `env::account_balance()` and `env::storage_byte_cost()` are `NearToken` and are unwrapped with `.as_yoctonear()` before any arithmetic.

*Rationale:* near-sdk 5 removed `Balance` (verified against docs.rs 5.30.1 — the crate root has no such item; `near-contract-standards` keeps a local `pub type Balance = u128` that is not re-exported). `NearToken` also implements no `Add`/`Sub`/`Mul`/`Div`, so both `env::account_balance() - stake` and `env::storage_byte_cost() * bytes` fail to compile. Storing `NearToken` in state would additionally require Borsh coverage the SDK version does not guarantee for it, and would still force unwrapping at every arithmetic site — so the boundary unwrap is the minimum viable fix, not a stylistic choice.

### D-26 — Errors reach the chain as `E-xx` text, which requires `cfg(near)`

`ContractError` derives `near_sdk::FunctionError`, and the wasm build pins `--cfg near` via `contract/.cargo/config.toml`.

*Rationale:* the first testnet deployment returned **every** failure as `WebAssembly trap: An unreachable opcode was executed` — no code, no message, so the frontend's error catalogue had nothing to map and each rejection would have looked like a broken contract. Unit tests could not catch it: they call the method and match the Rust `Err`, never crossing the `FunctionError::panic` boundary.

The cause is a compile-time switch inside near-sdk. `cfg(near)` means "use on-chain host functions"; `target_family = "wasm"` without it means "pure-Rust fallbacks, not for on-chain use" (near-sdk states this in its own `compile_error!` text). The blanket `FunctionError` impl takes the `env::panic_str` branch only under `cfg(near)`; otherwise it runs `eprintln!` + `panic!()`, and `panic = "abort"` compiles that to a bare `unreachable`. `cfg(near)` is injected by `cargo near build` — near-sdk ships no build script that sets it — so a plain `cargo build --target wasm32-unknown-unknown` silently produces the wrong mode.

Two changes, both verified on testnet: deriving `FunctionError` makes error reporting independent of the flag, and pinning `--cfg near` for the wasm target only (not globally, so `cargo test` keeps the unit-testing path) keeps the rest of the SDK in on-chain mode. Confirmed after redeploy: `Smart contract panicked: E-08: project already finalized`, `E-01: unauthorized action`, `E-18: nothing to withdraw`.

*Consequence for the frontend:* the code must be extracted from the panic text (match `E-\d\d`), not compared against the whole string.

# 18. Document Status

This specification is **approved and final (v1.7)**. It supersedes all earlier versions and is the authoritative implementation reference for the NEAR Escrow smart contract.

Next document: **04 — System Architecture Specification**.
