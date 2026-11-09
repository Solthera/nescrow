//! Time windows, gas budgets and storage limits.
//!
//! Every value here is normative: it comes from the State Transition
//! Specification (STS) §6 and the Smart Contract Technical Specification
//! (SCTS) §9. Changing one of these is a specification change, not a tweak.

/// Nanoseconds per second. NEAR block timestamps are nanoseconds.
pub const NS_PER_SEC: u64 = 1_000_000_000;

/// 48 hours — the freelancer's window to accept or reject (STS §6).
pub const ACCEPT_WINDOW: u64 = 48 * 3_600 * NS_PER_SEC;

/// 72 hours — the client's review window after a deliverable (STS §6).
pub const REVIEW_WINDOW: u64 = 72 * 3_600 * NS_PER_SEC;

/// 3 days — work time granted by each revision request (STS §6).
pub const REVISION_EXTENSION: u64 = 3 * 24 * 3_600 * NS_PER_SEC;

/// 7 days — the arbiter's window to decide a dispute (STS §6.2).
pub const ARBITER_WINDOW: u64 = 7 * 24 * 3_600 * NS_PER_SEC;

/// Maximum revision requests per project (FSD §4.5).
pub const MAX_REVISIONS: u8 = 2;

/// Minimum escrow: 0.1 NEAR.
///
/// Chosen so a project's escrow always exceeds its own storage footprint
/// (SCTS D-01). `MIN_ESCROW = 100_000_000_000_000_000_000_000` yoctoNEAR.
pub const MIN_ESCROW: u128 = 100_000_000_000_000_000_000_000;

/// Upper bound on the bytes one project can add to storage (SCTS D-23).
///
/// Every string field at maximum length plus map overhead. Deliberately a
/// declared bound rather than a measurement: `env::storage_usage()` only
/// reports current usage, so the reserve check needs a fixed number. Test
/// AD-10 exists to prove this bound still holds once the contract is built.
pub const MAX_PROJECT_STORAGE_BYTES: u128 = 6_000;

/// Maximum lengths, in bytes (SCTS §4.3).
pub const MAX_TITLE_LEN: usize = 128;
pub const MAX_DESCRIPTION_LEN: usize = 2_000;
pub const MAX_LINK_LEN: usize = 512;
pub const MAX_NOTE_LEN: usize = 1_024;

/// Gas handed to the `on_withdraw` callback receipt.
///
/// The 30 Tgas *minimum* on `withdraw` itself is a caller-side requirement, not
/// something the contract can enforce as a typed error — sending too little
/// simply fails the transaction, and the balance is untouched because the debit
/// and the transfer happen in the same call. Frontend sends 50 Tgas
/// (Integration Spec §3.3).
pub const CALLBACK_GAS_TGAS: u64 = 10;

/// Grace period before the owner may clear a stuck withdrawal (SCTS D-05).
pub const STUCK_WITHDRAWAL_GRACE: u64 = 3_600 * NS_PER_SEC;

/// Maximum page size for `get_projects` (SCTS §6.3).
pub const MAX_PAGE_LIMIT: u32 = 50;

/// Current on-chain state layout version. Bump on every migration.
pub const STATE_VERSION: u16 = 1;
