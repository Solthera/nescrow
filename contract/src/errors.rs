//! Typed contract errors.
//!
//! Every variant carries the code published in FSD §7 / SCTS §11 so the
//! frontend can map a failure to a human sentence without string matching on
//! prose. The codes are part of the public contract: do not renumber them.

use near_sdk::serde::Serialize;

/// `#[handle_result]` turns an `Err` into `FunctionError::panic(&err)`.
///
/// The blanket impl in near-sdk (for `T: AsRef<str>`) only calls the
/// `env::panic_str` host function when `cfg(near)` is set; without it — and
/// near-sdk ships no build script that sets it — the fallback is a bare Rust
/// `panic!()`, which the wasm runtime reports as an opaque
/// "WebAssembly trap: unreachable opcode" with **no** error code. Deriving
/// `FunctionError` instead always goes through `panic_str(to_string())`, so the
/// published `E-xx` code reaches the caller regardless of build flags.
///
/// Deriving also means `AsRef<str>` must NOT be implemented here: it would
/// collide with near-sdk's blanket impl (coherence error).
#[derive(Debug, PartialEq, Eq, Clone, Serialize, near_sdk::FunctionError)]
#[serde(crate = "near_sdk::serde")]
pub enum ContractError {
    // --- authorization -----------------------------------------------------
    Unauthorized,          // E-01
    NotOwner,              // E-01

    // --- state -------------------------------------------------------------
    InvalidProjectState,   // E-02
    AlreadyFinalized,      // E-08

    // --- time --------------------------------------------------------------
    DeadlineExpired,       // E-03
    ReviewPeriodNotExpired, // E-11
    ResolutionWindowExpired, // E-13
    GraceNotElapsed,       // E-17

    // --- amounts -----------------------------------------------------------
    InvalidEscrowAmount,   // E-04
    InvalidSettlementPercentage, // E-09
    InsufficientReserve,   // E-14

    // --- flow --------------------------------------------------------------
    AlreadyAccepted,       // E-05
    RevisionLimitExceeded, // E-06
    DisputeAlreadyActive,  // E-07
    DisputeNotAllowed,     // E-20
    EvidenceAlreadySubmitted, // E-10
    SettlementMismatch,    // E-16

    // --- input validation --------------------------------------------------
    InvalidParticipants,   // E-04
    InvalidDeliverable,    // E-10
    InvalidEvidence,       // E-10
    InvalidInput,          // E-10

    // --- funds -------------------------------------------------------------
    PayoutFailed,          // E-12 (event only, never returned)
    NothingToWithdraw,     // E-18
    WithdrawInProgress,    // E-19
    NoWithdrawalInProgress, // E-22 (admin path)

    // --- admin -------------------------------------------------------------
    ContractPaused,        // E-15
    UnknownProject,        // E-21
}

impl ContractError {
    /// The published code, e.g. `"E-14"`.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Unauthorized | Self::NotOwner => "E-01",
            Self::InvalidProjectState => "E-02",
            Self::DeadlineExpired => "E-03",
            Self::InvalidEscrowAmount | Self::InvalidParticipants => "E-04",
            Self::AlreadyAccepted => "E-05",
            Self::RevisionLimitExceeded => "E-06",
            Self::DisputeAlreadyActive => "E-07",
            Self::AlreadyFinalized => "E-08",
            Self::InvalidSettlementPercentage => "E-09",
            Self::InvalidDeliverable | Self::InvalidEvidence | Self::InvalidInput
            | Self::EvidenceAlreadySubmitted => "E-10",
            Self::ReviewPeriodNotExpired => "E-11",
            Self::PayoutFailed => "E-12",
            Self::ResolutionWindowExpired => "E-13",
            Self::InsufficientReserve => "E-14",
            Self::ContractPaused => "E-15",
            Self::SettlementMismatch => "E-16",
            Self::GraceNotElapsed => "E-17",
            Self::NothingToWithdraw => "E-18",
            Self::WithdrawInProgress => "E-19",
            Self::DisputeNotAllowed => "E-20",
            Self::UnknownProject => "E-21",
            Self::NoWithdrawalInProgress => "E-22",
        }
    }

    /// Stable, human-readable text used in `Display` and in UI copy fallbacks.
    pub fn message(&self) -> &'static str {
        match self {
            Self::Unauthorized => "unauthorized action",
            Self::NotOwner => "only the contract owner can do this",
            Self::InvalidProjectState => "invalid project state",
            Self::AlreadyFinalized => "project already finalized",
            Self::DeadlineExpired => "deadline expired",
            Self::ReviewPeriodNotExpired => "review period not expired",
            Self::ResolutionWindowExpired => "arbiter resolution window expired",
            Self::GraceNotElapsed => "grace period has not elapsed",
            Self::InvalidEscrowAmount => "invalid escrow amount",
            Self::InvalidSettlementPercentage => "invalid settlement percentage",
            Self::InsufficientReserve => "insufficient storage reserve",
            Self::AlreadyAccepted => "project already accepted",
            Self::RevisionLimitExceeded => "revision limit exceeded",
            Self::DisputeAlreadyActive => "dispute already active",
            Self::DisputeNotAllowed => "dispute not allowed in this state",
            Self::EvidenceAlreadySubmitted => "evidence already submitted",
            Self::SettlementMismatch => "proposals do not match",
            Self::InvalidParticipants => "client, freelancer and arbiter must be distinct",
            Self::InvalidDeliverable => "invalid deliverable",
            Self::InvalidEvidence => "invalid evidence",
            Self::InvalidInput => "invalid input",
            Self::PayoutFailed => "payout failed",
            Self::NothingToWithdraw => "nothing to withdraw",
            Self::WithdrawInProgress => "withdrawal already in progress",
            Self::NoWithdrawalInProgress => "no withdrawal in progress",
            Self::ContractPaused => "contract paused",
            Self::UnknownProject => "unknown project",
        }
    }
}

impl std::fmt::Display for ContractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // `code: message` is the exact string the caller receives on-chain, e.g.
        // "E-18: nothing to withdraw". The Integration Spec matches on the code
        // prefix, so the format must not change without a spec revision.
        write!(f, "{}: {}", self.code(), self.message())
    }
}
