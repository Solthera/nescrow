//! NEAR Escrow — escrow and dispute resolution for freelance work.
//!
//! This crate is a direct implementation of `docs/Smart Contract Technical
//! Specification (SCTS).md`. Section references in comments point there.
//!
//! Layout of the first slice: state, fund accounting, `create_project` and the
//! read-only views. Remaining transitions land on top of the same helpers.

use near_sdk::store::LookupMap;
use near_sdk::json_types::{U128, U64};
use near_sdk::{env, near, AccountId, PanicOnDefault};

mod constants;
mod errors;
mod model;

use constants::*;
use errors::ContractError;
use model::*;

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Contract {
    pub owner: AccountId,
    pub state_version: u16,

    pub next_project_id: u64,
    pub projects: LookupMap<u64, Project>,

    /// Append-only per-account project indexes. `LookupMap` cannot be
    /// iterated, so these are the only iteration surface.
    pub client_index: LookupMap<AccountId, Vec<u64>>,
    pub freelancer_index: LookupMap<AccountId, Vec<u64>>,
    pub arbiter_index: LookupMap<AccountId, Vec<u64>>,

    /// Allocated but not yet withdrawn, per account.
    pub claimable: LookupMap<AccountId, Yocto>,
    pub withdrawals: LookupMap<AccountId, WithdrawalRecord>,

    /// Escrowed in non-terminal projects.
    pub total_locked: Yocto,
    /// Allocated and not yet withdrawn.
    pub total_claimable: Yocto,

    pub paused: bool,
}

#[near]
impl Contract {
    // -----------------------------------------------------------------------
    // Init
    // -----------------------------------------------------------------------

    #[init]
    pub fn new(owner: AccountId) -> Self {
        Self {
            owner,
            state_version: STATE_VERSION,
            next_project_id: 0,
            projects: LookupMap::new(b"p".to_vec()),
            client_index: LookupMap::new(b"ci".to_vec()),
            freelancer_index: LookupMap::new(b"fi".to_vec()),
            arbiter_index: LookupMap::new(b"ai".to_vec()),
            claimable: LookupMap::new(b"c".to_vec()),
            withdrawals: LookupMap::new(b"w".to_vec()),
            total_locked: 0,
            total_claimable: 0,
            paused: false,
        }
    }

    // -----------------------------------------------------------------------
    // Fund accounting (SCTS §5.1)
    // -----------------------------------------------------------------------

    /// Bytes locked to pay for current state. Not spendable.
    fn storage_stake() -> Yocto {
        env::storage_byte_cost()
            .as_yoctonear()
            .saturating_mul(env::storage_usage() as u128)
    }

    /// Worst-case storage stake after one more project is written (SCTS D-23).
    fn storage_stake_after_write() -> Yocto {
        env::storage_byte_cost()
            .as_yoctonear()
            .saturating_mul(env::storage_usage() as u128 + MAX_PROJECT_STORAGE_BYTES)
    }

    fn spendable() -> Yocto {
        env::account_balance()
            .as_yoctonear()
            .saturating_sub(Self::storage_stake())
    }

    /// Hard invariant: escrow + claimable never exceeds what the contract can
    /// actually pay out. A violation aborts the whole transaction.
    fn assert_solvent(&self) -> Result<(), ContractError> {
        let owed = self
            .total_locked
            .checked_add(self.total_claimable)
            .unwrap_or(Yocto::MAX);
        if Self::spendable() < owed {
            return Err(ContractError::InsufficientReserve);
        }
        Ok(())
    }

    /// The only writer of `claimable` (SCTS §5.2.2).
    fn credit(&mut self, account: &AccountId, amount: Yocto) {
        if amount == 0 {
            return;
        }
        let current = self.claimable.get(account).copied().unwrap_or(0);
        let next = current.checked_add(amount).unwrap_or(Yocto::MAX);
        self.claimable.insert(account.clone(), next);
        self.total_claimable = self.total_claimable.checked_add(amount).unwrap_or(Yocto::MAX);
    }

    fn project(&self, project_id: u64) -> Result<Project, ContractError> {
        self.projects
            .get(&project_id)
            .cloned()
            .ok_or(ContractError::UnknownProject)
    }

    fn now() -> u64 {
        env::block_timestamp()
    }

    // -----------------------------------------------------------------------
    // M-01 create_project (SCTS §6.2)
    // -----------------------------------------------------------------------

    /// Create a project and fund its escrow in a single transaction.
    ///
    /// The attached deposit *is* the escrow amount — there is no separate
    /// amount argument (SCTS D-06), which removes the "deposit does not match
    /// declared amount" bug class entirely.
    #[payable]
    #[handle_result]
    pub fn create_project(
        &mut self,
        freelancer: AccountId,
        arbiter: AccountId,
        title: String,
        description: String,
        work_deadline: U64,
    ) -> Result<u64, ContractError> {
        let client = env::predecessor_account_id();
        let amount = env::attached_deposit().as_yoctonear();
        let now = Self::now();

        // Validation order is normative (SCTS §11.1): paused -> amount ->
        // participants -> strings -> deadline floor -> reserve.
        if self.paused {
            return Err(ContractError::ContractPaused);
        }
        if amount < MIN_ESCROW {
            return Err(ContractError::InvalidEscrowAmount);
        }
        if freelancer == client || arbiter == client || freelancer == arbiter {
            return Err(ContractError::InvalidParticipants);
        }
        if title.is_empty()
            || title.len() > MAX_TITLE_LEN
            || description.is_empty()
            || description.len() > MAX_DESCRIPTION_LEN
        {
            return Err(ContractError::InvalidInput);
        }

        let accept_deadline = now.checked_add(ACCEPT_WINDOW).unwrap_or(u64::MAX);
        // A deadline inside the acceptance window would let the freelancer
        // accept a project that can never be delivered (SCTS D-20).
        if work_deadline.0 < accept_deadline {
            return Err(ContractError::DeadlineExpired);
        }

        // Reserve pre-check before any write, so a thin reserve surfaces as a
        // typed E-14 instead of a panic from assert_solvent (SCTS D-16).
        let needed = self
            .total_locked
            .checked_add(self.total_claimable)
            .and_then(|v| v.checked_add(amount))
            .unwrap_or(Yocto::MAX);
        if env::account_balance().as_yoctonear()
            < Self::storage_stake_after_write().saturating_add(needed)
        {
            return Err(ContractError::InsufficientReserve);
        }

        let id = self.next_project_id;
        self.next_project_id = id.checked_add(1).unwrap_or(u64::MAX);

        let project = Project {
            id,
            client: client.clone(),
            freelancer: freelancer.clone(),
            arbiter: arbiter.clone(),
            title,
            description,
            amount,
            status: ProjectStatus::Funded,
            created_at: now,
            accept_deadline,
            work_deadline: work_deadline.0,
            review_deadline: 0,
            revision_deadline: 0,
            dispute_opened_at: 0,
            revision_count: 0,
            deliverable: None,
            client_evidence: None,
            freelancer_evidence: None,
            client_settlement_proposal: None,
            freelancer_settlement_proposal: None,
            resolution: None,
            distribution: None,
        };

        self.projects.insert(id, project.clone());
        self.total_locked = self.total_locked.checked_add(amount).unwrap_or(Yocto::MAX);

        self.index_add(&client, Role::Client, id);
        self.index_add(&freelancer, Role::Freelancer, id);
        self.index_add(&arbiter, Role::Arbiter, id);

        self.assert_solvent()?;

        // Event (SCTS §10). Emitted after the state is final.
        env::log_str(&format!(
            "EVENT_JSON:{{\"standard\":\"nescrow\",\"version\":\"1.0.0\",\"event\":\"project_created\",\"data\":{{\"project_id\":{},\"client\":\"{}\",\"freelancer\":\"{}\",\"arbiter\":\"{}\",\"amount\":\"{}\",\"work_deadline\":\"{}\"}}}}",
            id, project.client, project.freelancer, project.arbiter, amount, project.work_deadline
        ));

        Ok(id)
    }

    fn index_add(&mut self, account: &AccountId, role: Role, id: u64) {
        let map = match role {
            Role::Client => &mut self.client_index,
            Role::Freelancer => &mut self.freelancer_index,
            Role::Arbiter => &mut self.arbiter_index,
        };
        let mut ids = map.get(account).cloned().unwrap_or_default();
        ids.push(id);
        map.insert(account.clone(), ids);
    }

    // -----------------------------------------------------------------------
    // Settlement (SCTS §8.2, D-11)
    // -----------------------------------------------------------------------

    /// Shared allocation + accounting. Percent is the freelancer's share.
    ///
    /// Rounding: `floor(amount * pct / 100)` to the freelancer, remainder to the
    /// client, so the two parts always sum back to the escrow exactly (D-08).
    /// Division before multiplication keeps `u128` from overflowing.
    fn allocate(&mut self, p: &mut Project, freelancer_percent: u8) -> Result<(), ContractError> {
        let pct = freelancer_percent as u128;
        let share = (p.amount / 100)
            .checked_mul(pct)
            .and_then(|v| {
                (p.amount % 100)
                    .checked_mul(pct)
                    .map(|r| v + r / 100)
            })
            .ok_or(ContractError::InvalidSettlementPercentage)?;
        let remainder = p
            .amount
            .checked_sub(share)
            .ok_or(ContractError::InvalidSettlementPercentage)?;

        p.distribution = Some(Distribution {
            freelancer_amount: share,
            client_amount: remainder,
        });

        self.total_locked = self.total_locked.saturating_sub(p.amount);
        self.total_claimable = self.total_claimable.saturating_add(p.amount);

        let freelancer = p.freelancer.clone();
        let client = p.client.clone();
        if share > 0 {
            self.credit(&freelancer, share);
        }
        if remainder > 0 {
            self.credit(&client, remainder);
        }

        self.assert_solvent()
    }

    /// Rule-based settlement: approval, automatic release, rejection, refunds.
    /// The percentage is fixed by the rules, so no `Resolution` is written —
    /// there was no decision by a person (D-11).
    fn settle_by_rule(&mut self, p: &mut Project, freelancer_percent: u8) -> Result<(), ContractError> {
        p.resolution = None;
        self.allocate(p, freelancer_percent)
    }

    /// Dispute settlement: arbiter decision or mutual agreement. The only
    /// paths that produce a `Resolution`.
    fn settle_by_decision(
        &mut self,
        p: &mut Project,
        freelancer_percent: u8,
        resolved_by: ResolutionKind,
    ) -> Result<(), ContractError> {
        p.resolution = Some(Resolution {
            freelancer_percent,
            resolved_by,
            resolved_at: Self::now(),
        });
        self.allocate(p, freelancer_percent)
    }

    fn emit(event: &str, data: serde_json::Value) {
        env::log_str(&format!(
            "EVENT_JSON:{{\"standard\":\"nescrow\",\"version\":\"1.0.0\",\"event\":\"{}\",\"data\":{}}}",
            event, data
        ));
    }

    // -----------------------------------------------------------------------
    // M-02 / M-03 accept and reject (SCTS §6.2, STS §5.1 / §5.2A)
    // -----------------------------------------------------------------------

    /// The freelancer accepts the project. Starts the work clock, which is the
    /// deadline the client set at creation.
    #[handle_result]
    pub fn accept_project(&mut self, project_id: u64) -> Result<(), ContractError> {
        let mut p = self.project(project_id)?;
        let caller = env::predecessor_account_id();

        if caller != p.freelancer {
            return Err(ContractError::Unauthorized);
        }
        if p.status.is_terminal() {
            return Err(ContractError::AlreadyFinalized);
        }
        if p.status == ProjectStatus::Accepted {
            return Err(ContractError::AlreadyAccepted);
        }
        if p.status != ProjectStatus::Funded {
            return Err(ContractError::InvalidProjectState);
        }
        if Self::now() >= p.accept_deadline {
            return Err(ContractError::DeadlineExpired);
        }

        p.status = ProjectStatus::Accepted;
        self.projects.insert(project_id, p.clone());

        Self::emit(
            "project_accepted",
            serde_json::json!({ "project_id": project_id, "freelancer": p.freelancer.to_string() }),
        );
        Ok(())
    }

    /// The freelancer rejects the project, which refunds the client in full.
    #[handle_result]
    pub fn reject_project(&mut self, project_id: u64) -> Result<(), ContractError> {
        let mut p = self.project(project_id)?;
        let caller = env::predecessor_account_id();

        if caller != p.freelancer {
            return Err(ContractError::Unauthorized);
        }
        if p.status.is_terminal() {
            return Err(ContractError::AlreadyFinalized);
        }
        if p.status == ProjectStatus::Accepted {
            return Err(ContractError::AlreadyAccepted);
        }
        if p.status != ProjectStatus::Funded {
            return Err(ContractError::InvalidProjectState);
        }
        if Self::now() >= p.accept_deadline {
            return Err(ContractError::DeadlineExpired);
        }

        self.settle_by_rule(&mut p, 0)?;
        p.status = ProjectStatus::Refunded;
        self.projects.insert(project_id, p.clone());

        Self::emit(
            "settled",
            serde_json::json!({
                "project_id": project_id,
                "status": "Refunded",
                "freelancer_amount": "0",
                "client_amount": p.amount.to_string(),
                "resolved_by": null
            }),
        );
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Views (SCTS §6.3)
    // -----------------------------------------------------------------------

    pub fn get_project(&self, project_id: u64) -> Option<ProjectView> {
        self.projects.get(&project_id).map(ProjectView::from)
    }

    pub fn get_project_count(&self) -> u64 {
        self.next_project_id
    }

    pub fn get_claimable(&self, account: AccountId) -> U128 {
        U128::from(self.claimable.get(&account).copied().unwrap_or(0))
    }

    pub fn get_withdrawal_state(&self, account: AccountId) -> Option<WithdrawalRecordView> {
        self.withdrawals.get(&account).map(|w| WithdrawalRecordView {
            amount: U128::from(w.amount),
            started_at: U64::from(w.started_at),
        })
    }

    pub fn get_config(&self) -> Config {
        Config {
            owner: self.owner.clone(),
            min_escrow: U128::from(MIN_ESCROW),
            accept_window: U64::from(ACCEPT_WINDOW),
            review_window: U64::from(REVIEW_WINDOW),
            revision_extension: U64::from(REVISION_EXTENSION),
            arbiter_window: U64::from(ARBITER_WINDOW),
            max_revisions: MAX_REVISIONS,
            paused: self.paused,
            state_version: self.state_version,
        }
    }

    /// Audit view: the numbers behind the solvency invariant.
    pub fn get_fund_invariants(&self) -> FundInvariants {
        let balance = env::account_balance().as_yoctonear();
        let storage_stake = Self::storage_stake();
        let spendable = balance.saturating_sub(storage_stake);
        let owed = self
            .total_locked
            .checked_add(self.total_claimable)
            .unwrap_or(Yocto::MAX);
        FundInvariants {
            locked: U128::from(self.total_locked),
            claimable: U128::from(self.total_claimable),
            balance: U128::from(balance),
            storage_stake: U128::from(storage_stake),
            spendable: U128::from(spendable),
            ok: spendable >= owed,
        }
    }

    pub fn get_state_version(&self) -> u16 {
        self.state_version
    }
}
