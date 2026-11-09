//! Data model: internal state and the JSON-facing view DTOs.
//!
//! Two rules live here and nowhere else:
//!
//! 1. **Internal state uses native integers** (`Yocto = u128`, `u64` timestamps)
//!    so arithmetic stays plain `checked_*` on integers (SCTS D-24).
//! 2. **Every view returns a DTO with `U128`/`U64`**, because serde would
//!    otherwise emit JSON numbers and JavaScript truncates anything above
//!    `2^53` — escrow amounts are ~1e25 and nanosecond timestamps ~1.7e18
//!    (SCTS D-15). The internal `Project` is never serialised to JSON.

use near_sdk::json_types::{U128, U64};
use near_sdk::AccountId;

/// YoctoNEAR. Local alias on purpose: `near_sdk::Balance` does not exist in
/// near-sdk 5 (see SCTS D-24).
pub type Yocto = u128;

// ---------------------------------------------------------------------------
// Status
// ---------------------------------------------------------------------------

#[near_sdk::near(serializers = [borsh, json])]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ProjectStatus {
    Funded,            // 0 — never reorder: Borsh stores the index
    Accepted,          // 1
    Delivered,         // 2
    RevisionRequested, // 3
    Disputed,          // 4
    Released,          // 5
    Refunded,          // 6
    Resolved,          // 7
}

impl ProjectStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Released | Self::Refunded | Self::Resolved)
    }

    /// Statuses in which the escrow is still held by a live project.
    pub fn is_locked(&self) -> bool {
        !self.is_terminal()
    }
}

/// Wire format is lowercase (`"client" | "freelancer" | "arbiter"`), matching
/// the frontend's `Role` type and the `get_projects` argument (SCTS D-18).
#[near_sdk::near(serializers = [json])]
#[near_sdk::serde(rename_all = "lowercase")]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    Client,
    Freelancer,
    Arbiter,
}

/// Who decided a dispute. Only set on `DISPUTED -> RESOLVED` (SCTS D-11).
#[near_sdk::near(serializers = [borsh, json])]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ResolutionKind {
    Arbiter,          // 0
    MutualSettlement, // 1
}

// ---------------------------------------------------------------------------
// Internal state structs (Borsh only)
// ---------------------------------------------------------------------------

#[near_sdk::near(serializers = [borsh, json])]
#[derive(Clone)]
pub struct Deliverable {
    pub link: String,
    pub note: String,
    pub submitted_at: u64,
    pub revision_index: u8,
}

#[near_sdk::near(serializers = [borsh, json])]
#[derive(Clone)]
pub struct Evidence {
    pub link: String,
    pub note: String,
    pub submitted_at: u64,
}

#[near_sdk::near(serializers = [borsh, json])]
#[derive(Clone)]
pub struct Resolution {
    pub freelancer_percent: u8,
    pub resolved_by: ResolutionKind,
    pub resolved_at: u64,
}

#[near_sdk::near(serializers = [borsh, json])]
#[derive(Clone)]
pub struct Distribution {
    pub freelancer_amount: Yocto,
    pub client_amount: Yocto,
}

#[near_sdk::near(serializers = [borsh, json])]
#[derive(Clone)]
pub struct WithdrawalRecord {
    pub amount: Yocto,
    pub started_at: u64,
}

#[near_sdk::near(serializers = [borsh, json])]
#[derive(Clone)]
pub struct Project {
    pub id: u64,
    pub client: AccountId,
    pub freelancer: AccountId,
    pub arbiter: AccountId,
    pub title: String,
    pub description: String,
    pub amount: Yocto,
    pub status: ProjectStatus,
    pub created_at: u64,
    pub accept_deadline: u64,
    pub work_deadline: u64,
    pub review_deadline: u64,
    pub revision_deadline: u64,
    pub dispute_opened_at: u64,
    pub revision_count: u8,
    pub deliverable: Option<Deliverable>,
    pub client_evidence: Option<Evidence>,
    pub freelancer_evidence: Option<Evidence>,
    pub client_settlement_proposal: Option<u8>,
    pub freelancer_settlement_proposal: Option<u8>,
    pub resolution: Option<Resolution>,
    pub distribution: Option<Distribution>,
}

impl Project {
    pub fn is_party(&self, account: &AccountId) -> bool {
        account == &self.client || account == &self.freelancer
    }
}

// ---------------------------------------------------------------------------
// JSON view DTOs — never stored, so `json` serializers are enough
// ---------------------------------------------------------------------------

#[near_sdk::near(serializers = [json])]
#[derive(Clone)]
pub struct DeliverableView {
    pub link: String,
    pub note: String,
    pub submitted_at: U64,
    pub revision_index: u8,
}

#[near_sdk::near(serializers = [json])]
#[derive(Clone)]
pub struct EvidenceView {
    pub link: String,
    pub note: String,
    pub submitted_at: U64,
}

#[near_sdk::near(serializers = [json])]
#[derive(Clone)]
pub struct ResolutionView {
    pub freelancer_percent: u8,
    pub resolved_by: ResolutionKind,
    pub resolved_at: U64,
}

#[near_sdk::near(serializers = [json])]
#[derive(Clone)]
pub struct DistributionView {
    pub freelancer_amount: U128,
    pub client_amount: U128,
}

#[near_sdk::near(serializers = [json])]
#[derive(Clone)]
pub struct WithdrawalRecordView {
    pub amount: U128,
    pub started_at: U64,
}

#[near_sdk::near(serializers = [json])]
#[derive(Clone)]
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
    pub review_deadline: U64,
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

impl From<&Project> for ProjectView {
    fn from(p: &Project) -> Self {
        Self {
            id: p.id,
            client: p.client.clone(),
            freelancer: p.freelancer.clone(),
            arbiter: p.arbiter.clone(),
            title: p.title.clone(),
            description: p.description.clone(),
            amount: U128::from(p.amount),
            status: p.status,
            created_at: U64::from(p.created_at),
            accept_deadline: U64::from(p.accept_deadline),
            work_deadline: U64::from(p.work_deadline),
            review_deadline: U64::from(p.review_deadline),
            revision_deadline: U64::from(p.revision_deadline),
            dispute_opened_at: U64::from(p.dispute_opened_at),
            revision_count: p.revision_count,
            deliverable: p.deliverable.as_ref().map(|d| DeliverableView {
                link: d.link.clone(),
                note: d.note.clone(),
                submitted_at: U64::from(d.submitted_at),
                revision_index: d.revision_index,
            }),
            client_evidence: p.client_evidence.as_ref().map(|e| EvidenceView {
                link: e.link.clone(),
                note: e.note.clone(),
                submitted_at: U64::from(e.submitted_at),
            }),
            freelancer_evidence: p.freelancer_evidence.as_ref().map(|e| EvidenceView {
                link: e.link.clone(),
                note: e.note.clone(),
                submitted_at: U64::from(e.submitted_at),
            }),
            client_settlement_proposal: p.client_settlement_proposal,
            freelancer_settlement_proposal: p.freelancer_settlement_proposal,
            resolution: p.resolution.as_ref().map(|r| ResolutionView {
                freelancer_percent: r.freelancer_percent,
                resolved_by: r.resolved_by,
                resolved_at: U64::from(r.resolved_at),
            }),
            distribution: p.distribution.as_ref().map(|d| DistributionView {
                freelancer_amount: U128::from(d.freelancer_amount),
                client_amount: U128::from(d.client_amount),
            }),
        }
    }
}

#[near_sdk::near(serializers = [json])]
#[derive(Clone)]
pub struct ProjectClaim {
    pub project_id: u64,
    pub role: Role,
    pub allocated: U128,
    pub status: ProjectStatus,
}

#[near_sdk::near(serializers = [json])]
#[derive(Clone)]
pub struct Stats {
    pub projects_as_client: u32,
    pub projects_as_freelancer: u32,
    pub completed_as_freelancer: u32,
    pub total_value_as_freelancer: U128,
    pub disputes_as_freelancer: u32,
    pub disputes_released_fully: u32,
    pub refunds_as_client: u32,
}

#[near_sdk::near(serializers = [json])]
#[derive(Clone)]
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

#[near_sdk::near(serializers = [json])]
#[derive(Clone)]
pub struct FundInvariants {
    pub locked: U128,
    pub claimable: U128,
    pub balance: U128,
    pub storage_stake: U128,
    pub spendable: U128,
    pub ok: bool,
}
