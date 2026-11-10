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
// The mock does NOT credit `attached_deposit` to `account_balance`, so the test
// environment has to hold a balance large enough for every project a test opens.
// On-chain these two are linked; here they are set independently.
const RESERVE: u128 = 1_000 * NEAR;

// `near_sdk::test_utils::accounts` only provides six entries (0..=5).
const OWNER: usize = 5;

/// Contract balance is deliberately generous: the reserve check uses a worst
/// case storage bound, so a starved balance would fail every creation with E-14.
fn env(account: usize, deposit: u128, now: u64) {
    testing_env!(VMContextBuilder::new()
        .current_account_id(accounts(0))
        .predecessor_account_id(accounts(account))
        .attached_deposit(NearToken::from_yoctonear(deposit))
        .account_balance(NearToken::from_yoctonear(RESERVE))
        .block_timestamp(now)
        .build());
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
    assert_eq!(c.get_projects(accounts(1), Role::Client, 0, 10)[0].id, id);
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
