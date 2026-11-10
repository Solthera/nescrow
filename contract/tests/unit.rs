//! Unit tests (L1 in the Test Plan) against the contract's public surface.
//!
//! Every test sets an explicit context: predecessor, attached deposit, block
//! timestamp and contract balance. Forgetting to reset one of those is the most
//! common source of false passes with `VMContextBuilder`, so all of them go
//! through `env_as` / `env_view` below instead of being set inline.

use near_sdk::test_utils::{accounts, VMContextBuilder};
use near_sdk::testing_env;
use near_sdk::{AccountId, NearToken};
use nescrow::model::{ProjectStatus, Role};
use nescrow::Contract;

const NEAR: u128 = 1_000_000_000_000_000_000_000_000;
const HOUR_NS: u64 = 3_600 * 1_000_000_000;
const DAY_NS: u64 = 24 * HOUR_NS;
const ACCEPT_WINDOW: u64 = 48 * HOUR_NS;
const REVIEW_WINDOW: u64 = 72 * HOUR_NS;
const AMOUNT: u128 = 10 * NEAR;
const MIN_ESCROW: u128 = NEAR / 10;
// `VMContextBuilder::account_balance` is the balance *before* the call, and the
// mock adds `attached_deposit` on top (probe: builder 10 NEAR + deposit 10 NEAR
// was reported as 20 NEAR). Keeping this generous means the reserve check is out
// of the way for tests that are not about the reserve.
const RESERVE: u128 = 1_000 * NEAR;

// `near_sdk::test_utils::accounts` only provides six entries (0..=5).
const OWNER: usize = 5;

/// Contract balance is deliberately generous: the reserve check uses a worst
/// case storage bound, so a starved balance would fail every creation with E-14.
fn env_bal(account: usize, deposit: u128, now: u64, balance: u128) {
    testing_env!(VMContextBuilder::new()
        .current_account_id(accounts(0))
        .predecessor_account_id(accounts(account))
        .attached_deposit(NearToken::from_yoctonear(deposit))
        .account_balance(NearToken::from_yoctonear(balance))
        .block_timestamp(now)
        .build());
}

fn env(account: usize, deposit: u128, now: u64) {
    env_bal(account, deposit, now, RESERVE);
}

fn env_view(now: u64) {
    testing_env!(VMContextBuilder::new()
        .current_account_id(accounts(0))
        .account_balance(NearToken::from_yoctonear(RESERVE))
        .block_timestamp(now)
        .build());
}

fn new_contract() -> Contract {
    env(OWNER, 0, 0);
    Contract::new(accounts(OWNER))
}

/// Create the default project: alice pays, bob works, carol arbitrates,
/// 10 NEAR, deadline seven days out.
fn create_default(c: &mut Contract, now: u64) -> u64 {
    env(1, AMOUNT, now);
    c.create_project(
        accounts(2),
        accounts(3),
        "Logo design".to_string(),
        "Three concepts and a final SVG".to_string(),
        (now + 7 * DAY_NS).into(),
    )
    .expect("project should be created")
}

fn status(c: &Contract, id: u64) -> ProjectStatus {
    env_view(0);
    c.get_project(id).expect("project exists").status
}

fn claimable(c: &Contract, account: AccountId) -> u128 {
    env_view(0);
    c.get_claimable(account).0
}

// ===========================================================================
// CP — create project (FSD F-02/F-03, SCTS M-01)
// ===========================================================================

#[test]
fn cp01_create_project_funds_escrow() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = create_default(&mut c, now);

    assert_eq!(id, 0);
    assert_eq!(status(&c, id), ProjectStatus::Funded);

    env_view(now);
    let p = c.get_project(id).unwrap();
    assert_eq!(p.amount.0, AMOUNT);
    assert_eq!(p.client, accounts(1));
    assert_eq!(p.freelancer, accounts(2));
    assert_eq!(p.arbiter, accounts(3));
    // accept deadline is exactly the acceptance window after creation
    assert_eq!(p.accept_deadline.0, now + ACCEPT_WINDOW);

    let inv = c.get_fund_invariants();
    assert_eq!(inv.locked.0, AMOUNT);
    assert_eq!(inv.claimable.0, 0);
    assert!(inv.ok, "solvency invariant must hold after creation");
}

#[test]
fn cp02_deposit_below_minimum_is_rejected() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    env(1, MIN_ESCROW - 1, now);
    let err = c
        .create_project(
            accounts(2),
            accounts(3),
            "Logo".into(),
            "Brief".into(),
            (now + 7 * DAY_NS).into(),
        )
        .unwrap_err();
    assert_eq!(err.code(), "E-04");
    env_view(now);
    assert_eq!(c.get_project_count(), 0, "no project may be created");
}

#[test]
fn cp03_zero_deposit_is_rejected() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    env(1, 0, now);
    let err = c
        .create_project(
            accounts(2),
            accounts(3),
            "Logo".into(),
            "Brief".into(),
            (now + 7 * DAY_NS).into(),
        )
        .unwrap_err();
    assert_eq!(err.code(), "E-04");
}

#[test]
fn cp04_freelancer_cannot_be_client() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    env(1, AMOUNT, now);
    let err = c
        .create_project(
            accounts(1),
            accounts(3),
            "Logo".into(),
            "Brief".into(),
            (now + 7 * DAY_NS).into(),
        )
        .unwrap_err();
    assert_eq!(err.code(), "E-04");
}

#[test]
fn cp05_arbiter_cannot_be_client() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    env(1, AMOUNT, now);
    let err = c
        .create_project(
            accounts(2),
            accounts(1),
            "Logo".into(),
            "Brief".into(),
            (now + 7 * DAY_NS).into(),
        )
        .unwrap_err();
    assert_eq!(err.code(), "E-04");
}

#[test]
fn cp06_freelancer_cannot_be_arbiter() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    env(1, AMOUNT, now);
    let err = c
        .create_project(
            accounts(2),
            accounts(2),
            "Logo".into(),
            "Brief".into(),
            (now + 7 * DAY_NS).into(),
        )
        .unwrap_err();
    assert_eq!(err.code(), "E-04");
}

#[test]
fn cp07_deadline_in_the_past_is_rejected() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    env(1, AMOUNT, now);
    let err = c
        .create_project(
            accounts(2),
            accounts(3),
            "Logo".into(),
            "Brief".into(),
            (now - HOUR_NS).into(),
        )
        .unwrap_err();
    assert_eq!(err.code(), "E-03");
}

#[test]
fn cp12_deadline_inside_acceptance_window_is_rejected() {
    // A deadline shorter than the 48h acceptance window would create a project
    // that can be accepted but never delivered (SCTS D-20).
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    env(1, AMOUNT, now);
    let err = c
        .create_project(
            accounts(2),
            accounts(3),
            "Logo".into(),
            "Brief".into(),
            (now + 30 * HOUR_NS).into(),
        )
        .unwrap_err();
    assert_eq!(err.code(), "E-03");
}

#[test]
fn cp08_oversized_title_is_rejected() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    env(1, AMOUNT, now);
    let err = c
        .create_project(
            accounts(2),
            accounts(3),
            "x".repeat(129),
            "Brief".into(),
            (now + 7 * DAY_NS).into(),
        )
        .unwrap_err();
    assert_eq!(err.code(), "E-10");
}

#[test]
fn cp09_paused_blocks_creation() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    env(OWNER, 0, now);
    c.set_paused(true).unwrap();

    env(1, AMOUNT, now);
    let err = c
        .create_project(
            accounts(2),
            accounts(3),
            "Logo".into(),
            "Brief".into(),
            (now + 7 * DAY_NS).into(),
        )
        .unwrap_err();
    assert_eq!(err.code(), "E-15");
}

#[test]
fn cp10_ids_are_sequential() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let a = create_default(&mut c, now);
    let b = create_default(&mut c, now);
    let d = create_default(&mut c, now);
    assert_eq!((a, b, d), (0, 1, 2));
    env_view(now);
    assert_eq!(c.get_project_count(), 3);
}

#[test]
fn cp_creates_indexes_per_role() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = create_default(&mut c, now);
    env_view(now);
    assert_eq!(c.get_projects(accounts(1), Role::Client, 0, 10).len(), 1);
    assert_eq!(c.get_projects(accounts(2), Role::Freelancer, 0, 10).len(), 1);
    assert_eq!(c.get_projects(accounts(3), Role::Arbiter, 0, 10).len(), 1);
    // Integration §4.1: the list returns ProjectClaim, not ProjectView. This
    // assertion used to read `.id` — the shape the contract actually returned —
    // which is why a spec/implementation mismatch survived a green suite.
    let claims = c.get_projects(accounts(1), Role::Client, 0, 10);
    assert_eq!(claims[0].project_id, id);
    assert_eq!(claims[0].role, Role::Client);
    assert_eq!(claims[0].status, ProjectStatus::Funded);
    assert_eq!(claims[0].allocated.0, 0, "nothing allocated before settlement");
}

// ===========================================================================
// AC — accept / reject (FSD F-04/F-05, STS 5.1 / 5.2A)
// ===========================================================================

#[test]
fn ac01_freelancer_accepts() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = create_default(&mut c, now);

    env(2, 0, now + 10 * 60 * 1_000_000_000);
    c.accept_project(id).unwrap();
    assert_eq!(status(&c, id), ProjectStatus::Accepted);
}

#[test]
fn ac02_non_freelancer_cannot_accept() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = create_default(&mut c, now);

    env(4, 0, now);
    let err = c.accept_project(id).unwrap_err();
    assert_eq!(err.code(), "E-01");
    assert_eq!(status(&c, id), ProjectStatus::Funded);
}

#[test]
fn ac03_reject_refunds_client() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = create_default(&mut c, now);

    env(2, 0, now + HOUR_NS);
    c.reject_project(id).unwrap();

    assert_eq!(status(&c, id), ProjectStatus::Refunded);
    assert_eq!(claimable(&c, accounts(1)), AMOUNT);
    assert_eq!(claimable(&c, accounts(2)), 0);

    env_view(now);
    let inv = c.get_fund_invariants();
    assert_eq!(inv.locked.0, 0, "escrow left the locked bucket");
    assert_eq!(inv.claimable.0, AMOUNT);
    assert!(inv.ok);
}

#[test]
fn ac04_reject_after_accept_is_already_accepted() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = create_default(&mut c, now);

    env(2, 0, now + HOUR_NS);
    c.accept_project(id).unwrap();
    env(2, 0, now + HOUR_NS);
    let err = c.reject_project(id).unwrap_err();
    assert_eq!(err.code(), "E-05");
}

#[test]
fn ac05_accept_after_window_expires() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = create_default(&mut c, now);

    env(2, 0, now + ACCEPT_WINDOW + 1);
    let err = c.accept_project(id).unwrap_err();
    assert_eq!(err.code(), "E-03");
    assert_eq!(status(&c, id), ProjectStatus::Funded);
}

#[test]
fn ac06_accept_at_exact_boundary_is_expired() {
    // The boundary itself counts as expired: `now >= deadline` (STS §6.1).
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = create_default(&mut c, now);

    env(2, 0, now + ACCEPT_WINDOW);
    let err = c.accept_project(id).unwrap_err();
    assert_eq!(err.code(), "E-03");
}

#[test]
fn ac07_client_cannot_accept_own_project() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = create_default(&mut c, now);

    env(1, 0, now);
    let err = c.accept_project(id).unwrap_err();
    assert_eq!(err.code(), "E-01");
}

#[test]
fn sc08_unknown_project_is_reported() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    env(2, 0, now);
    let err = c.accept_project(9999).unwrap_err();
    assert_eq!(err.code(), "E-21");
    env_view(now);
    assert!(c.get_project(9999).is_none());
}

/// Project taken all the way to `Delivered`.
fn delivered_project(c: &mut Contract, now: u64) -> u64 {
    let id = accepted_project(c, now);
    let submit_at = now + 2 * DAY_NS;
    env(2, 0, submit_at);
    c.submit_deliverable(id, "https://example.com/logo".into(), "v1".into())
        .unwrap();
    id
}

/// Project taken all the way to `Released` (client approved).
fn released_project(c: &mut Contract, now: u64) -> u64 {
    let id = delivered_project(c, now);
    env(1, 0, now + 2 * DAY_NS + HOUR_NS);
    c.approve_deliverable(id).unwrap();
    id
}

/// Project taken all the way to `Disputed`.
fn disputed_project(c: &mut Contract, now: u64) -> u64 {
    let id = delivered_project(c, now);
    env(1, 0, now + 2 * DAY_NS + HOUR_NS);
    c.open_dispute(id).unwrap();
    id
}

// ===========================================================================
// SD — submit deliverable (FSD F-06, STS 5.3 / 5.9)
// ===========================================================================

fn accepted_project(c: &mut Contract, now: u64) -> u64 {
    let id = create_default(c, now);
    env(2, 0, now + HOUR_NS);
    c.accept_project(id).unwrap();
    id
}

#[test]
fn sd01_submit_sets_delivered_and_review_window() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = accepted_project(&mut c, now);

    let submit_at = now + 2 * DAY_NS;
    env(2, 0, submit_at);
    c.submit_deliverable(id, "https://example.com/logo".into(), "v1".into())
        .unwrap();

    assert_eq!(status(&c, id), ProjectStatus::Delivered);
    env_view(submit_at);
    let p = c.get_project(id).unwrap();
    assert_eq!(p.review_deadline.0, submit_at + REVIEW_WINDOW);
    assert_eq!(p.deliverable.as_ref().unwrap().revision_index, 0);
}

#[test]
fn sd02_empty_link_is_rejected() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = accepted_project(&mut c, now);

    env(2, 0, now + HOUR_NS);
    let err = c
        .submit_deliverable(id, "".into(), "".into())
        .unwrap_err();
    assert_eq!(err.code(), "E-10");
}

#[test]
fn sd03_note_is_optional() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = accepted_project(&mut c, now);

    env(2, 0, now + HOUR_NS);
    c.submit_deliverable(id, "https://example.com".into(), "".into())
        .unwrap();
    assert_eq!(status(&c, id), ProjectStatus::Delivered);
}

#[test]
fn sd04_submit_after_work_deadline_is_rejected() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = accepted_project(&mut c, now);

    env(2, 0, now + 8 * DAY_NS);
    let err = c
        .submit_deliverable(id, "https://example.com".into(), "".into())
        .unwrap_err();
    assert_eq!(err.code(), "E-03");
}

#[test]
fn sd05_submit_in_wrong_state() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = create_default(&mut c, now);

    env(2, 0, now + HOUR_NS);
    let err = c
        .submit_deliverable(id, "https://example.com".into(), "".into())
        .unwrap_err();
    assert_eq!(err.code(), "E-02");
}

#[test]
fn sd06_non_freelancer_cannot_submit() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = accepted_project(&mut c, now);

    env(4, 0, now + HOUR_NS);
    let err = c
        .submit_deliverable(id, "https://example.com".into(), "".into())
        .unwrap_err();
    assert_eq!(err.code(), "E-01");
}

#[test]
fn sd07_oversized_link_is_rejected() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = accepted_project(&mut c, now);

    env(2, 0, now + HOUR_NS);
    let err = c
        .submit_deliverable(id, "x".repeat(513), "".into())
        .unwrap_err();
    assert_eq!(err.code(), "E-10");
}

// ===========================================================================
// WD — pull-payment withdrawal (SCTS §5.3 / §8.4, STS §9)
//
// The callback itself needs a real receipt, so `on_withdraw` is exercised at L2.
// Here we prove the state machine: the debit, the in-progress guard and the
// no-double-pay guarantee.
// ===========================================================================

#[test]
fn wd01_withdraw_moves_claimable_to_wallet_pending() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let _id = released_project(&mut c, now);
    assert_eq!(claimable(&c, accounts(2)), AMOUNT, "allocation recorded");

    env(2, 0, now + 2 * DAY_NS + 2 * HOUR_NS);
    // `Promise` implements no `Debug`, so unwrap/unwrap_err are unavailable.
    assert!(c.withdraw().is_ok(), "withdraw should be accepted");

    // Debit happens before the transfer is created.
    assert_eq!(claimable(&c, accounts(2)), 0);

    env_view(now);
    let inv = c.get_fund_invariants();
    assert_eq!(inv.claimable.0, 0, "claimable bucket emptied");
    assert_eq!(inv.locked.0, 0);
    assert!(inv.ok);

    let record = c.get_withdrawal_state(accounts(2)).expect("record stays until the callback");
    assert_eq!(record.amount.0, AMOUNT);
}

#[test]
fn wd02_withdraw_twice_is_rejected() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    released_project(&mut c, now);

    env(2, 0, now + 2 * DAY_NS + 2 * HOUR_NS);
    assert!(c.withdraw().is_ok());
    let err = match c.withdraw() {
        Ok(_) => panic!("second withdraw must fail"),
        Err(e) => e,
    };
    assert_eq!(err.code(), "E-18", "nothing left to withdraw");
}

#[test]
fn wd03_withdraw_without_balance_is_rejected() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    released_project(&mut c, now);

    env(4, 0, now + 2 * DAY_NS + 2 * HOUR_NS);
    let err = match c.withdraw() {
        Ok(_) => panic!("withdraw must fail without a balance"),
        Err(e) => e,
    };
    assert_eq!(err.code(), "E-18");
}

#[test]
fn sc07_withdraw_in_progress_blocks_a_second_transfer() {
    // A second settlement lands while the first withdrawal is still awaiting its
    // callback. Without the in-progress marker this would start a second transfer
    // for the same account.
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    released_project(&mut c, now);

    env(2, 0, now + 2 * DAY_NS + 2 * HOUR_NS);
    assert!(c.withdraw().is_ok());

    // Second project settles to the same freelancer while the first is in flight.
    env(1, AMOUNT, now + 3 * DAY_NS);
    let id2 = c
        .create_project(
            accounts(2),
            accounts(3),
            "Second job".into(),
            "Brief".into(),
            (now + 10 * DAY_NS).into(),
        )
        .unwrap();
    env(2, 0, now + 3 * DAY_NS + HOUR_NS);
    c.accept_project(id2).unwrap();
    env(2, 0, now + 4 * DAY_NS);
    c.submit_deliverable(id2, "https://example.com/2".into(), "".into())
        .unwrap();
    env(1, 0, now + 4 * DAY_NS + HOUR_NS);
    c.approve_deliverable(id2).unwrap();

    assert_eq!(claimable(&c, accounts(2)), AMOUNT, "new money credited");
    env(2, 0, now + 5 * DAY_NS);
    let err = match c.withdraw() {
        Ok(_) => panic!("must not start a second transfer"),
        Err(e) => e,
    };
    assert_eq!(err.code(), "E-19");
}

// ===========================================================================
// DS — disputes and evidence (FSD F-11/F-12, STS 5.7 / 5.10)
// ===========================================================================

#[test]
fn ds01_client_opens_dispute() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = delivered_project(&mut c, now);

    let at = now + 2 * DAY_NS + HOUR_NS;
    env(1, 0, at);
    c.open_dispute(id).unwrap();

    assert_eq!(status(&c, id), ProjectStatus::Disputed);
    env_view(at);
    let p = c.get_project(id).unwrap();
    assert_eq!(p.dispute_opened_at.0, at);
    assert_eq!(p.review_deadline.0, 0, "review window cleared");
}

#[test]
fn ds02_freelancer_opens_dispute_during_revision() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = delivered_project(&mut c, now);

    let at = now + 2 * DAY_NS + HOUR_NS;
    env(1, 0, at);
    c.request_revision(id).unwrap();

    env(2, 0, at + HOUR_NS);
    c.open_dispute(id).unwrap();
    assert_eq!(status(&c, id), ProjectStatus::Disputed);
}

#[test]
fn ds03_dispute_before_deliverable_is_not_allowed() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = accepted_project(&mut c, now);

    env(1, 0, now + HOUR_NS);
    let err = c.open_dispute(id).unwrap_err();
    assert_eq!(err.code(), "E-20");
}

#[test]
fn ds04_dispute_after_review_expiry_is_rejected() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = delivered_project(&mut c, now);

    env(1, 0, now + 2 * DAY_NS + REVIEW_WINDOW + 1);
    let err = c.open_dispute(id).unwrap_err();
    assert_eq!(err.code(), "E-03");
}

#[test]
fn ds10_dispute_twice_is_rejected() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = disputed_project(&mut c, now);

    env(1, 0, now + 2 * DAY_NS + 2 * HOUR_NS);
    let err = c.open_dispute(id).unwrap_err();
    assert_eq!(err.code(), "E-07");
}

#[test]
fn ds11_dispute_allowed_after_revision_deadline() {
    // D-14: a missed revision deadline must not strand the escrow. After the
    // deadline the freelancer cannot resubmit and the client cannot refund, so
    // the dispute is the only reachable transition and must stay open.
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = delivered_project(&mut c, now);

    let at = now + 2 * DAY_NS + HOUR_NS;
    env(1, 0, at);
    c.request_revision(id).unwrap();

    env(1, 0, at + 4 * DAY_NS); // revision window is 3 days
    c.open_dispute(id).unwrap();
    assert_eq!(status(&c, id), ProjectStatus::Disputed);
}

#[test]
fn ds05_both_parties_submit_evidence() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = disputed_project(&mut c, now);

    env(1, 0, now + 2 * DAY_NS + 2 * HOUR_NS);
    c.submit_evidence(id, "https://client.example/evidence".into(), "late".into())
        .unwrap();
    env(2, 0, now + 2 * DAY_NS + 3 * HOUR_NS);
    c.submit_evidence(id, "https://bob.example/proof".into(), "on time".into())
        .unwrap();

    env_view(now);
    let p = c.get_project(id).unwrap();
    assert!(p.client_evidence.is_some());
    assert!(p.freelancer_evidence.is_some());
    assert_eq!(status(&c, id), ProjectStatus::Disputed);
}

#[test]
fn ds06_evidence_twice_is_rejected() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = disputed_project(&mut c, now);

    env(1, 0, now + 2 * DAY_NS + 2 * HOUR_NS);
    c.submit_evidence(id, "https://a.example".into(), "".into())
        .unwrap();
    let err = c
        .submit_evidence(id, "https://b.example".into(), "".into())
        .unwrap_err();
    assert_eq!(err.code(), "E-10");
}

#[test]
fn ds07_non_party_cannot_submit_evidence() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = disputed_project(&mut c, now);

    env(4, 0, now + 2 * DAY_NS + 2 * HOUR_NS);
    let err = c
        .submit_evidence(id, "https://x.example".into(), "".into())
        .unwrap_err();
    assert_eq!(err.code(), "E-01");
}

// ===========================================================================
// AB — arbiter resolution (FSD F-13, STS 5.11)
// ===========================================================================

#[test]
fn ab01_arbiter_splits_seventy_thirty() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = disputed_project(&mut c, now);

    env(3, 0, now + 2 * DAY_NS + 2 * HOUR_NS);
    c.arbiter_resolve(id, 70).unwrap();

    assert_eq!(status(&c, id), ProjectStatus::Resolved);

    let seven = AMOUNT * 70 / 100;
    let three = AMOUNT - seven;
    assert_eq!(claimable(&c, accounts(2)), seven);
    assert_eq!(claimable(&c, accounts(1)), three);

    env_view(now);
    let p = c.get_project(id).unwrap();
    let r = p.resolution.expect("resolution recorded");
    assert_eq!(r.freelancer_percent, 70);

    let inv = c.get_fund_invariants();
    assert_eq!(inv.locked.0, 0, "escrow left the locked bucket");
    assert_eq!(inv.claimable.0, AMOUNT, "nothing created or lost");
    assert!(inv.ok);
}

#[test]
fn ab02_percentage_above_hundred_is_rejected() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = disputed_project(&mut c, now);

    env(3, 0, now + 2 * DAY_NS + 2 * HOUR_NS);
    let err = c.arbiter_resolve(id, 101).unwrap_err();
    assert_eq!(err.code(), "E-09");
    assert_eq!(status(&c, id), ProjectStatus::Disputed);
}

#[test]
fn ab03_non_arbiter_cannot_resolve() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = disputed_project(&mut c, now);

    env(4, 0, now + 2 * DAY_NS + 2 * HOUR_NS);
    let err = c.arbiter_resolve(id, 50).unwrap_err();
    assert_eq!(err.code(), "E-01");
}

#[test]
fn ab04_other_arbiter_cannot_resolve() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = disputed_project(&mut c, now);

    // accounts(4) is not the registered arbiter for this project.
    env(4, 0, now + 2 * DAY_NS + 2 * HOUR_NS);
    assert_eq!(c.arbiter_resolve(id, 50).unwrap_err().code(), "E-01");
}

#[test]
fn ab05_resolution_after_seven_days_is_rejected() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = disputed_project(&mut c, now);
    let opened = now + 2 * DAY_NS + HOUR_NS;
    let window = 7 * DAY_NS;

    env(3, 0, opened + window + 1);
    let err = c.arbiter_resolve(id, 50).unwrap_err();
    assert_eq!(err.code(), "E-13");
    assert_eq!(status(&c, id), ProjectStatus::Disputed);
}

#[test]
fn ab06_resolution_at_exact_seven_day_boundary_is_allowed() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = disputed_project(&mut c, now);
    let opened = now + 2 * DAY_NS + HOUR_NS;
    let window = 7 * DAY_NS;

    env(3, 0, opened + window);
    c.arbiter_resolve(id, 50).unwrap();
    assert_eq!(status(&c, id), ProjectStatus::Resolved);
}

#[test]
fn ab07_decision_is_immutable() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = disputed_project(&mut c, now);
    let at = now + 2 * DAY_NS + 2 * HOUR_NS;

    env(3, 0, at);
    c.arbiter_resolve(id, 40).unwrap();
    let before = claimable(&c, accounts(2));

    env(3, 0, at + HOUR_NS);
    let err = c.arbiter_resolve(id, 100).unwrap_err();
    assert_eq!(err.code(), "E-08");

    assert_eq!(claimable(&c, accounts(2)), before, "split unchanged");
}

#[test]
fn ab08_extreme_percentages() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = disputed_project(&mut c, now);

    env(3, 0, now + 2 * DAY_NS + 2 * HOUR_NS);
    c.arbiter_resolve(id, 0).unwrap();

    assert_eq!(claimable(&c, accounts(2)), 0, "freelancer gets nothing");
    assert_eq!(claimable(&c, accounts(1)), AMOUNT, "client gets everything");
}

// ===========================================================================
// MS — mutual settlement (FSD F-14, STS 5.12, D-21)
// ===========================================================================

#[test]
fn ms01_both_parties_agree() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = disputed_project(&mut c, now);
    let at = now + 2 * DAY_NS + 2 * HOUR_NS;

    env(1, 0, at);
    c.propose_settlement(id, 60).unwrap();
    env(2, 0, at + HOUR_NS);
    c.settle_mutual(id, 60).unwrap();

    assert_eq!(status(&c, id), ProjectStatus::Resolved);
    let six = AMOUNT * 60 / 100;
    assert_eq!(claimable(&c, accounts(2)), six);
    assert_eq!(claimable(&c, accounts(1)), AMOUNT - six);

    env_view(at);
    let r = c.get_project(id).unwrap().resolution.unwrap();
    assert_eq!(r.freelancer_percent, 60);
}

#[test]
fn ms02_mismatched_percentage_does_not_settle() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = disputed_project(&mut c, now);
    let at = now + 2 * DAY_NS + 2 * HOUR_NS;

    env(1, 0, at);
    c.propose_settlement(id, 60).unwrap();
    env(2, 0, at + HOUR_NS);
    let err = c.settle_mutual(id, 70).unwrap_err();

    assert_eq!(err.code(), "E-16");
    assert_eq!(status(&c, id), ProjectStatus::Disputed);
    assert_eq!(claimable(&c, accounts(2)), 0, "nothing allocated");
}

#[test]
fn ms03_one_party_cannot_count_as_both_approvals() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = disputed_project(&mut c, now);
    let at = now + 2 * DAY_NS + 2 * HOUR_NS;

    env(1, 0, at);
    c.propose_settlement(id, 50).unwrap();
    env(1, 0, at + HOUR_NS);
    let err = c.settle_mutual(id, 50).unwrap_err();
    assert_eq!(err.code(), "E-16", "needs the counterparty's proposal");
}

#[test]
fn ms04_proposal_can_be_updated_before_settling() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = disputed_project(&mut c, now);
    let at = now + 2 * DAY_NS + 2 * HOUR_NS;

    env(1, 0, at);
    c.propose_settlement(id, 40).unwrap();
    env(1, 0, at + HOUR_NS);
    c.propose_settlement(id, 55).unwrap();
    env(2, 0, at + 2 * HOUR_NS);
    c.settle_mutual(id, 55).unwrap();

    assert_eq!(status(&c, id), ProjectStatus::Resolved);
}

#[test]
fn ms08_a_party_can_concede_to_the_counterparty() {
    // D-21: alice proposed 60, bob proposed 70, then bob accepts alice's 60.
    // Bob's own stored 70 is superseded by the call itself.
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = disputed_project(&mut c, now);
    let at = now + 2 * DAY_NS + 2 * HOUR_NS;

    env(1, 0, at);
    c.propose_settlement(id, 60).unwrap();
    env(2, 0, at + HOUR_NS);
    c.propose_settlement(id, 70).unwrap();

    env(2, 0, at + 2 * HOUR_NS);
    c.settle_mutual(id, 60).unwrap();

    assert_eq!(status(&c, id), ProjectStatus::Resolved);
    env_view(at);
    let r = c.get_project(id).unwrap().resolution.unwrap();
    assert_eq!(r.freelancer_percent, 60, "settled at the accepted number");
}

#[test]
fn ms07_arbiter_inactivity_leaves_funds_locked() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = disputed_project(&mut c, now);
    let opened = now + 2 * DAY_NS + HOUR_NS;

    env_view(opened + 8 * DAY_NS);
    let inv = c.get_fund_invariants();
    assert_eq!(inv.locked.0, AMOUNT, "still escrowed, not auto-split");
    assert_eq!(inv.claimable.0, 0, "no 50/50 fallback exists");
    assert_eq!(status(&c, id), ProjectStatus::Disputed);
}

// ===========================================================================
// AP — approve deliverable (FSD F-07, STS 5.5)
// ===========================================================================

#[test]
fn ap01_approve_releases_the_escrow() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = delivered_project(&mut c, now);

    env(1, 0, now + 2 * DAY_NS + HOUR_NS);
    c.approve_deliverable(id).unwrap();

    assert_eq!(status(&c, id), ProjectStatus::Released);
    assert_eq!(claimable(&c, accounts(2)), AMOUNT);
    assert_eq!(claimable(&c, accounts(1)), 0);

    env_view(now);
    let p = c.get_project(id).unwrap();
    let d = p.distribution.expect("distribution recorded");
    assert_eq!(d.freelancer_amount.0, AMOUNT);
    assert_eq!(d.client_amount.0, 0);
    assert!(p.resolution.is_none(), "rule-based settlement has no resolution");
}

#[test]
fn ap02_non_client_cannot_approve() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = delivered_project(&mut c, now);

    env(4, 0, now + 2 * DAY_NS + HOUR_NS);
    assert_eq!(c.approve_deliverable(id).unwrap_err().code(), "E-01");
}

#[test]
fn ap03_approve_in_wrong_state() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = accepted_project(&mut c, now);

    env(1, 0, now + HOUR_NS);
    assert_eq!(c.approve_deliverable(id).unwrap_err().code(), "E-02");
}

#[test]
fn ap04_double_approval_is_rejected() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = released_project(&mut c, now);

    env(1, 0, now + 2 * DAY_NS + 2 * HOUR_NS);
    assert_eq!(c.approve_deliverable(id).unwrap_err().code(), "E-08");
    assert_eq!(claimable(&c, accounts(2)), AMOUNT, "still paid exactly once");
}

#[test]
fn ap05_approval_at_review_boundary_is_expired() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = delivered_project(&mut c, now);
    let submitted = now + 2 * DAY_NS;

    env(1, 0, submitted + REVIEW_WINDOW);
    assert_eq!(c.approve_deliverable(id).unwrap_err().code(), "E-03");
}

// ===========================================================================
// RV — request revision (FSD F-08, STS 5.6 / 5.9)
// ===========================================================================

#[test]
fn rv01_revision_extends_from_now() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = delivered_project(&mut c, now);
    let at = now + 2 * DAY_NS + HOUR_NS;

    env(1, 0, at);
    c.request_revision(id).unwrap();

    assert_eq!(status(&c, id), ProjectStatus::RevisionRequested);
    env_view(at);
    let p = c.get_project(id).unwrap();
    assert_eq!(p.revision_count, 1);
    assert_eq!(p.work_deadline.0, at + 3 * DAY_NS, "three days from now, not from the old deadline");
    assert_eq!(p.review_deadline.0, 0);
}

#[test]
fn rv02_third_revision_is_rejected() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = delivered_project(&mut c, now);
    let mut at = now + 2 * DAY_NS + HOUR_NS;

    for i in 0..2 {
        env(1, 0, at);
        c.request_revision(id).unwrap();
        env(2, 0, at + HOUR_NS);
        c.submit_deliverable(id, format!("https://example.com/v{}", i + 1), "".into())
            .unwrap();
        at += 2 * HOUR_NS;
    }

    env_view(at);
    let p = c.get_project(id).unwrap();
    assert_eq!(p.revision_count, 2);

    env(1, 0, at);
    assert_eq!(c.request_revision(id).unwrap_err().code(), "E-06");
}

#[test]
fn rv03_resubmission_restarts_review_window() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = delivered_project(&mut c, now);
    let at = now + 2 * DAY_NS + HOUR_NS;

    env(1, 0, at);
    c.request_revision(id).unwrap();

    let resubmit_at = at + DAY_NS;
    env(2, 0, resubmit_at);
    c.submit_deliverable(id, "https://example.com/v2".into(), "fixed".into())
        .unwrap();

    assert_eq!(status(&c, id), ProjectStatus::Delivered);
    env_view(resubmit_at);
    let p = c.get_project(id).unwrap();
    assert_eq!(p.deliverable.unwrap().revision_index, 1);
    assert_eq!(p.review_deadline.0, resubmit_at + REVIEW_WINDOW);
    assert_eq!(p.revision_deadline.0, 0);
}

#[test]
fn rv04_revision_after_review_expiry_is_rejected() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = delivered_project(&mut c, now);
    let submitted = now + 2 * DAY_NS;

    env(1, 0, submitted + REVIEW_WINDOW);
    assert_eq!(c.request_revision(id).unwrap_err().code(), "E-03");
}

#[test]
fn rv05_non_client_cannot_request_revision() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = delivered_project(&mut c, now);

    env(2, 0, now + 2 * DAY_NS + HOUR_NS);
    assert_eq!(c.request_revision(id).unwrap_err().code(), "E-01");
}

#[test]
fn rv08_revision_in_wrong_state() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = accepted_project(&mut c, now);

    env(1, 0, now + HOUR_NS);
    assert_eq!(c.request_revision(id).unwrap_err().code(), "E-02");
}

// ===========================================================================
// AR — automatic release (FSD F-09, STS 5.8)
// ===========================================================================

#[test]
fn ar01_anyone_can_release_after_review_expires() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = delivered_project(&mut c, now);
    let submitted = now + 2 * DAY_NS;

    env(4, 0, submitted + REVIEW_WINDOW + 1);
    c.release_after_review(id).unwrap();

    assert_eq!(status(&c, id), ProjectStatus::Released);
    assert_eq!(claimable(&c, accounts(2)), AMOUNT, "caller gets nothing");
    assert_eq!(claimable(&c, accounts(4)), 0);
}

#[test]
fn ar02_release_before_expiry_is_rejected() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = delivered_project(&mut c, now);
    let submitted = now + 2 * DAY_NS;

    env(4, 0, submitted + REVIEW_WINDOW - 1);
    assert_eq!(c.release_after_review(id).unwrap_err().code(), "E-11");
}

#[test]
fn ar03_release_while_disputed_is_rejected() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = disputed_project(&mut c, now);

    env(4, 0, now + 5 * DAY_NS);
    assert_eq!(c.release_after_review(id).unwrap_err().code(), "E-02");
}

#[test]
fn ar05_release_at_exact_boundary_is_allowed() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = delivered_project(&mut c, now);
    let submitted = now + 2 * DAY_NS;

    env(4, 0, submitted + REVIEW_WINDOW);
    c.release_after_review(id).unwrap();
    assert_eq!(status(&c, id), ProjectStatus::Released);
}

// ===========================================================================
// RF — refunds (FSD F-10, STS 5.2B / 5.4)
// ===========================================================================

#[test]
fn rf01_no_response_refund() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = create_default(&mut c, now);

    env(1, 0, now + ACCEPT_WINDOW + 1);
    c.claim_no_response_refund(id).unwrap();

    assert_eq!(status(&c, id), ProjectStatus::Refunded);
    assert_eq!(claimable(&c, accounts(1)), AMOUNT);
}

#[test]
fn rf02_deadline_refund_when_nothing_was_submitted() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = accepted_project(&mut c, now);

    env(1, 0, now + 8 * DAY_NS);
    c.claim_deadline_refund(id).unwrap();

    assert_eq!(status(&c, id), ProjectStatus::Refunded);
    assert_eq!(claimable(&c, accounts(1)), AMOUNT);
}

#[test]
fn rf03_no_response_refund_before_window_expires() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = create_default(&mut c, now);

    env(1, 0, now + HOUR_NS);
    assert_eq!(c.claim_no_response_refund(id).unwrap_err().code(), "E-03");
}

#[test]
fn rf04_deadline_refund_while_work_is_still_due() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = accepted_project(&mut c, now);

    env(1, 0, now + HOUR_NS);
    assert_eq!(c.claim_deadline_refund(id).unwrap_err().code(), "E-03");
}

#[test]
fn rf05_refund_is_closed_once_a_deliverable_exists() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = delivered_project(&mut c, now);

    env(1, 0, now + 8 * DAY_NS);
    assert_eq!(c.claim_deadline_refund(id).unwrap_err().code(), "E-02");
}

#[test]
fn rf06_non_client_cannot_claim_a_refund() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = create_default(&mut c, now);

    env(2, 0, now + ACCEPT_WINDOW + 1);
    assert_eq!(c.claim_no_response_refund(id).unwrap_err().code(), "E-01");
}

#[test]
fn rf07_missed_revision_deadline_is_not_a_refund() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = delivered_project(&mut c, now);
    let at = now + 2 * DAY_NS + HOUR_NS;

    env(1, 0, at);
    c.request_revision(id).unwrap();

    env(1, 0, at + 4 * DAY_NS);
    assert_eq!(c.claim_deadline_refund(id).unwrap_err().code(), "E-02");
}

#[test]
fn rf08_double_refund_is_rejected() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = create_default(&mut c, now);

    env(1, 0, now + ACCEPT_WINDOW + 1);
    c.claim_no_response_refund(id).unwrap();
    let err = c.claim_no_response_refund(id).unwrap_err();
    assert_eq!(err.code(), "E-08");
    assert_eq!(claimable(&c, accounts(1)), AMOUNT, "refunded once, not twice");
}

// ===========================================================================
// AD — admin surface (SCTS D-01 / D-02 / D-05 / D-19)
// ===========================================================================

#[test]
fn ad01_thin_reserve_is_a_typed_error() {
    // Pre-call balance zero, so the contract sees only the deposit and has no
    // buffer for storage. The reserve pre-check must fire as E-14 rather than a
    // panic from assert_solvent.
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    env_bal(1, AMOUNT, now, 0);
    let err = c
        .create_project(
            accounts(2),
            accounts(3),
            "Logo".into(),
            "Brief".into(),
            (now + 7 * DAY_NS).into(),
        )
        .unwrap_err();
    assert_eq!(err.code(), "E-14");
}

#[test]
fn ad02_top_up_reserve_is_accepted() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    env(4, NEAR, now);
    c.top_up_reserve().unwrap();

    env_view(now);
    assert!(c.get_fund_invariants().ok);
}

#[test]
fn ad02b_top_up_reserve_without_deposit_is_rejected() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    env(4, 0, now);
    assert_eq!(c.top_up_reserve().unwrap_err().code(), "E-04");
}

#[test]
fn ad03_pause_blocks_creation_but_not_existing_money() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = released_project(&mut c, now);

    env(OWNER, 0, now + 3 * DAY_NS);
    c.set_paused(true).unwrap();

    // creation blocked
    env(1, AMOUNT, now + 3 * DAY_NS);
    assert_eq!(
        c.create_project(
            accounts(2),
            accounts(3),
            "Blocked".into(),
            "Brief".into(),
            (now + 10 * DAY_NS).into(),
        )
        .unwrap_err()
        .code(),
        "E-15"
    );

    // withdrawal still works while paused
    env(2, 0, now + 3 * DAY_NS);
    assert!(c.withdraw().is_ok(), "pause must never trap funds");
    assert_eq!(claimable(&c, accounts(2)), 0);
    let _ = id;
}

#[test]
fn ad04_non_owner_cannot_pause() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    env(1, 0, now);
    assert_eq!(c.set_paused(true).unwrap_err().code(), "E-01");
}

#[test]
fn ad05_clear_stuck_withdrawal_before_grace() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    released_project(&mut c, now);

    env(2, 0, now + 2 * DAY_NS + 2 * HOUR_NS);
    assert!(c.withdraw().is_ok());

    env(OWNER, 0, now + 2 * DAY_NS + 2 * HOUR_NS + 60 * 1_000_000_000);
    assert_eq!(
        c.clear_stuck_withdrawal(accounts(2)).unwrap_err().code(),
        "E-17"
    );
}

#[test]
fn ad06_clear_stuck_withdrawal_restores_the_balance() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    released_project(&mut c, now);

    let withdrawn_at = now + 2 * DAY_NS + 2 * HOUR_NS;
    env(2, 0, withdrawn_at);
    assert!(c.withdraw().is_ok());
    assert_eq!(claimable(&c, accounts(2)), 0);

    env(OWNER, 0, withdrawn_at + HOUR_NS + 1);
    c.clear_stuck_withdrawal(accounts(2)).unwrap();

    assert_eq!(claimable(&c, accounts(2)), AMOUNT, "balance restored");
    env_view(now);
    assert!(c.get_withdrawal_state(accounts(2)).is_none());
}

#[test]
fn ad09_clear_stuck_withdrawal_without_a_record() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    env(OWNER, 0, now);
    assert_eq!(
        c.clear_stuck_withdrawal(accounts(2)).unwrap_err().code(),
        "E-22"
    );
}

// ===========================================================================
// INV — invariants and view correctness
// ===========================================================================

#[test]
fn inv01_solvency_after_creation() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    for _ in 0..5 {
        create_default(&mut c, now);
    }
    env_view(now);
    let inv = c.get_fund_invariants();
    assert_eq!(inv.locked.0, 5 * AMOUNT);
    assert_eq!(inv.claimable.0, 0);
    assert!(inv.ok);
    assert!(inv.spendable.0 >= inv.locked.0 + inv.claimable.0);
}

#[test]
fn inv02_solvency_after_settlement() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    released_project(&mut c, now);

    env_view(now);
    let inv = c.get_fund_invariants();
    assert_eq!(inv.locked.0, 0);
    assert_eq!(inv.claimable.0, AMOUNT);
    assert!(inv.ok);
}

#[test]
fn inv03_thirty_three_percent_sums_exactly() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = disputed_project(&mut c, now);

    env(3, 0, now + 2 * DAY_NS + 2 * HOUR_NS);
    c.arbiter_resolve(id, 33).unwrap();

    env_view(now);
    let p = c.get_project(id).unwrap();
    let d = p.distribution.unwrap();
    assert_eq!(d.freelancer_amount.0 + d.client_amount.0, AMOUNT);
    // floor to the freelancer, remainder to the client (D-08)
    assert_eq!(d.freelancer_amount.0, AMOUNT / 100 * 33);
    assert!(d.client_amount.0 >= d.freelancer_amount.0);
}

#[test]
fn inv04_rounding_never_loses_a_yocto() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    // smallest allowed escrow: 0.1 NEAR
    env(1, MIN_ESCROW, now);
    let id = c
        .create_project(
            accounts(2),
            accounts(3),
            "Tiny".into(),
            "Brief".into(),
            (now + 7 * DAY_NS).into(),
        )
        .unwrap();
    env(2, 0, now + HOUR_NS);
    c.accept_project(id).unwrap();
    env(2, 0, now + 2 * DAY_NS);
    c.submit_deliverable(id, "https://example.com".into(), "".into())
        .unwrap();
    env(1, 0, now + 2 * DAY_NS + HOUR_NS);
    c.open_dispute(id).unwrap();
    env(3, 0, now + 2 * DAY_NS + 2 * HOUR_NS);
    c.arbiter_resolve(id, 3).unwrap();

    env_view(now);
    let d = c.get_project(id).unwrap().distribution.unwrap();
    assert_eq!(d.freelancer_amount.0 + d.client_amount.0, MIN_ESCROW);
}

#[test]
fn inv05_terminal_state_rejects_state_transitions() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = released_project(&mut c, now);
    let at = now + 2 * DAY_NS + 2 * HOUR_NS;

    env(2, 0, at);
    assert_eq!(c.accept_project(id).unwrap_err().code(), "E-08");
    env(2, 0, at);
    assert_eq!(c.submit_deliverable(id, "x".into(), "".into()).unwrap_err().code(), "E-08");
    env(1, 0, at);
    assert_eq!(c.approve_deliverable(id).unwrap_err().code(), "E-08");
    env(1, 0, at);
    assert_eq!(c.open_dispute(id).unwrap_err().code(), "E-08");
    env(3, 0, at);
    assert_eq!(c.arbiter_resolve(id, 50).unwrap_err().code(), "E-08");
}

#[test]
fn inv10_allocation_survives_withdrawal() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = released_project(&mut c, now);

    env_view(now);
    let before = c.get_claimable_per_project(accounts(2));
    assert_eq!(before.len(), 1);
    assert_eq!(before[0].allocated.0, AMOUNT);

    env(2, 0, now + 2 * DAY_NS + 2 * HOUR_NS);
    assert!(c.withdraw().is_ok());

    env_view(now);
    let after = c.get_claimable_per_project(accounts(2));
    assert_eq!(after[0].allocated.0, AMOUNT, "allocation is historical");
    assert_eq!(after[0].project_id, id);
    assert_eq!(c.get_claimable(accounts(2)).0, 0, "withdrawable is empty");
}

#[test]
fn inv11_amounts_cross_the_json_boundary_as_strings() {
    // The only test that catches the precision trap in D-15: a Rust-side u128
    // comparison is happy either way, but JSON.parse in a browser is not.
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = create_default(&mut c, now);

    env_view(now);
    let view = c.get_project(id).unwrap();
    let json = serde_json::to_string(&view).unwrap();

    assert!(json.contains("\"amount\":\""), "amount must be a JSON string: {}", json);
    assert!(json.contains("\"created_at\":\""), "timestamps must be JSON strings");
    assert!(json.contains("\"work_deadline\":\""));
    assert!(!json.contains("\"amount\":1"), "amount must not be a bare number");
    assert!((view.amount.0 as f64) > 9e15, "value is genuinely above 2^53");
}

#[test]
fn inv09_stats_are_derived_from_state() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;

    // two released projects for bob
    released_project(&mut c, now);
    released_project(&mut c, now);

    // one dispute resolved at 70%
    let id = disputed_project(&mut c, now);
    env(3, 0, now + 2 * DAY_NS + 30 * HOUR_NS);
    c.arbiter_resolve(id, 70).unwrap();

    env_view(now);
    let stats = c.get_stats(accounts(2));
    assert_eq!(stats.projects_as_freelancer, 3);
    assert_eq!(stats.completed_as_freelancer, 3);
    assert_eq!(stats.disputes_as_freelancer, 1);
    assert_eq!(stats.disputes_released_fully, 0, "70% is not a full release");
    assert_eq!(stats.total_value_as_freelancer.0, AMOUNT + AMOUNT + AMOUNT * 70 / 100);
}

#[test]
fn views_report_project_lists_per_role() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    create_default(&mut c, now);
    create_default(&mut c, now);

    env_view(now);
    assert_eq!(c.get_projects(accounts(1), Role::Client, 0, 10).len(), 2);
    // pagination is capped and offset works
    assert_eq!(c.get_projects(accounts(1), Role::Client, 1, 10).len(), 1);
    assert_eq!(c.get_projects(accounts(1), Role::Client, 5, 10).len(), 0);
    // an account with no projects gets an empty list, not a panic
    assert_eq!(c.get_projects(accounts(4), Role::Client, 0, 10).len(), 0);
}

// ===========================================================================
// Remaining L1 cases from the Test Plan
// ===========================================================================

#[test]
fn cp11_excess_deposit_becomes_the_escrow() {
    // D-06: there is no separate amount field, so a larger deposit simply funds a
    // larger escrow. No refund of the difference, no mismatch to reconcile.
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let bigger = 2 * AMOUNT;
    env(1, bigger, now);
    let id = c
        .create_project(
            accounts(2),
            accounts(3),
            "Logo".into(),
            "Brief".into(),
            (now + 7 * DAY_NS).into(),
        )
        .unwrap();
    env_view(now);
    assert_eq!(c.get_project(id).unwrap().amount.0, bigger);
}

#[test]
fn sd08_double_submission_in_accepted_is_rejected() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = delivered_project(&mut c, now);

    env(2, 0, now + 2 * DAY_NS + HOUR_NS);
    assert_eq!(
        c.submit_deliverable(id, "https://example.com/again".into(), "".into())
            .unwrap_err()
            .code(),
        "E-02"
    );
}

#[test]
fn sd09_resubmission_resets_the_review_deadline() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = delivered_project(&mut c, now);
    let first_review = {
        env_view(now);
        c.get_project(id).unwrap().review_deadline.0
    };

    let at = now + 2 * DAY_NS + HOUR_NS;
    env(1, 0, at);
    c.request_revision(id).unwrap();
    let resubmit = at + HOUR_NS;
    env(2, 0, resubmit);
    c.submit_deliverable(id, "https://example.com/v2".into(), "".into())
        .unwrap();

    env_view(resubmit);
    let second_review = c.get_project(id).unwrap().review_deadline.0;
    assert_ne!(first_review, second_review);
    assert_eq!(second_review, resubmit + REVIEW_WINDOW);
}

#[test]
fn rv06_extension_is_relative_to_now_not_the_old_deadline() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = delivered_project(&mut c, now);
    let submitted = now + 2 * DAY_NS;

    // Ask on the last second of the review window.
    let at = submitted + REVIEW_WINDOW - 1;
    env(1, 0, at);
    c.request_revision(id).unwrap();

    env_view(at);
    assert_eq!(
        c.get_project(id).unwrap().work_deadline.0,
        at + 3 * DAY_NS,
        "a full three days from the request, not from the old deadline"
    );
}

#[test]
fn rv07_full_two_revision_cycle() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = delivered_project(&mut c, now);
    let mut at = now + 2 * DAY_NS + HOUR_NS;

    for round in 1..=2u8 {
        env(1, 0, at);
        c.request_revision(id).unwrap();
        env(2, 0, at + HOUR_NS);
        c.submit_deliverable(id, format!("https://example.com/r{}", round), "".into())
            .unwrap();
        at += 2 * HOUR_NS;
        env_view(at);
        assert_eq!(c.get_project(id).unwrap().revision_count, round);
        assert_eq!(status(&c, id), ProjectStatus::Delivered);
    }
}

#[test]
fn ap06_balance_buckets_after_approval() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    released_project(&mut c, now);

    env_view(now);
    let inv = c.get_fund_invariants();
    assert_eq!(inv.locked.0, 0, "nothing left escrowed");
    assert_eq!(inv.claimable.0, AMOUNT, "all of it allocated");
    assert!(inv.ok);
}

#[test]
fn ar04_client_may_trigger_automatic_release() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = delivered_project(&mut c, now);
    let submitted = now + 2 * DAY_NS;

    env(1, 0, submitted + REVIEW_WINDOW + 1);
    c.release_after_review(id).unwrap();
    assert_eq!(status(&c, id), ProjectStatus::Released);
    assert_eq!(claimable(&c, accounts(2)), AMOUNT, "funds still go to the freelancer");
}

#[test]
fn ds08_evidence_stays_on_its_own_project() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let a = disputed_project(&mut c, now);
    let b = disputed_project(&mut c, now);

    env(1, 0, now + 2 * DAY_NS + 2 * HOUR_NS);
    c.submit_evidence(a, "https://example.com/a".into(), "".into())
        .unwrap();

    env_view(now);
    assert!(c.get_project(a).unwrap().client_evidence.is_some());
    assert!(c.get_project(b).unwrap().client_evidence.is_none());
}

#[test]
fn ds09_dispute_on_a_terminal_project_is_rejected() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = released_project(&mut c, now);

    env(1, 0, now + 2 * DAY_NS + 2 * HOUR_NS);
    assert_eq!(c.open_dispute(id).unwrap_err().code(), "E-08");
}

#[test]
fn ab09_resolve_in_wrong_state() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = delivered_project(&mut c, now);

    env(3, 0, now + 2 * DAY_NS + HOUR_NS);
    assert_eq!(c.arbiter_resolve(id, 50).unwrap_err().code(), "E-02");
}

#[test]
fn ms05_proposal_outside_a_dispute_is_rejected() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = delivered_project(&mut c, now);

    env(1, 0, now + 2 * DAY_NS + HOUR_NS);
    assert_eq!(c.propose_settlement(id, 50).unwrap_err().code(), "E-02");
}

#[test]
fn ms06_settlement_after_an_arbiter_decision_is_rejected() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = disputed_project(&mut c, now);

    env(3, 0, now + 2 * DAY_NS + 2 * HOUR_NS);
    c.arbiter_resolve(id, 60).unwrap();

    env(1, 0, now + 2 * DAY_NS + 3 * HOUR_NS);
    assert_eq!(c.settle_mutual(id, 60).unwrap_err().code(), "E-08");
}

#[test]
fn inv06_exactly_one_status_at_a_time() {
    // The status is a single enum value, so "two statuses at once" is not
    // representable. This walks a full lifecycle and asserts the terminal value.
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = released_project(&mut c, now);
    let s = status(&c, id);
    assert!(s.is_terminal());
    assert_eq!(s, ProjectStatus::Released);
}

#[test]
fn wd04_both_parties_withdraw_after_resolution() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = disputed_project(&mut c, now);

    env(3, 0, now + 2 * DAY_NS + 2 * HOUR_NS);
    c.arbiter_resolve(id, 70).unwrap();

    let seven = AMOUNT * 70 / 100;
    let three = AMOUNT - seven;

    env(1, 0, now + 2 * DAY_NS + 3 * HOUR_NS);
    assert!(c.withdraw().is_ok());
    assert_eq!(claimable(&c, accounts(1)), 0);

    env(2, 0, now + 2 * DAY_NS + 4 * HOUR_NS);
    assert!(c.withdraw().is_ok());
    assert_eq!(claimable(&c, accounts(2)), 0);

    env_view(now);
    let inv = c.get_fund_invariants();
    assert_eq!(inv.claimable.0, 0, "everything allocated has been claimed");
    assert!(inv.ok);
    let _ = (seven, three);
}

#[test]
fn wd08_withdrawal_after_refund() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = create_default(&mut c, now);

    env(1, 0, now + ACCEPT_WINDOW + 1);
    c.claim_no_response_refund(id).unwrap();
    assert_eq!(claimable(&c, accounts(1)), AMOUNT);

    env(1, 0, now + ACCEPT_WINDOW + HOUR_NS);
    assert!(c.withdraw().is_ok());
    assert_eq!(claimable(&c, accounts(1)), 0);
}

#[test]
fn sc02_intruder_cannot_drive_freelancer_actions() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = create_default(&mut c, now);

    env(4, 0, now + HOUR_NS);
    assert_eq!(c.accept_project(id).unwrap_err().code(), "E-01");
    env(4, 0, now + HOUR_NS);
    assert_eq!(c.reject_project(id).unwrap_err().code(), "E-01");
    env(4, 0, now + HOUR_NS);
    assert_eq!(
        c.submit_deliverable(id, "https://x.example".into(), "".into())
            .unwrap_err()
            .code(),
        "E-01"
    );
}

#[test]
fn sc03_intruder_cannot_act_as_arbiter() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = disputed_project(&mut c, now);

    env(4, 0, now + 2 * DAY_NS + 2 * HOUR_NS);
    assert_eq!(c.arbiter_resolve(id, 50).unwrap_err().code(), "E-01");
}

#[test]
fn sc06_a_settled_project_cannot_be_settled_again() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    let id = released_project(&mut c, now);
    let at = now + 2 * DAY_NS + 2 * HOUR_NS;

    env(1, 0, at);
    assert_eq!(c.approve_deliverable(id).unwrap_err().code(), "E-08");
    env(4, 0, at);
    assert_eq!(c.release_after_review(id).unwrap_err().code(), "E-08");
    assert_eq!(claimable(&c, accounts(2)), AMOUNT, "paid exactly once");
}

#[test]
fn ad08_fund_invariants_match_manual_arithmetic() {
    let mut c = new_contract();
    let now = 1_000 * HOUR_NS;
    create_default(&mut c, now);
    released_project(&mut c, now);

    env_view(now);
    let inv = c.get_fund_invariants();
    // One project still escrowed + one settled and awaiting withdrawal.
    assert_eq!(inv.locked.0, AMOUNT);
    assert_eq!(inv.claimable.0, AMOUNT);
    assert_eq!(inv.locked.0 + inv.claimable.0, 2 * AMOUNT);
    assert_eq!(inv.spendable.0, inv.balance.0 - inv.storage_stake.0);
    assert!(inv.ok);
}
