/**
 * Wire types for the contract. Every large number arrives as a string
 * (SCTS D-15): `json.parse`-ing them into numbers would silently truncate above
 * 2^53. Keep them as strings and convert with BigInt in lib/format.ts only.
 */
export type U128 = string;
export type U64 = string;
export type Nanos = string;

export type ProjectStatus =
  | 'Funded'
  | 'Accepted'
  | 'Delivered'
  | 'RevisionRequested'
  | 'Disputed'
  | 'Released'
  | 'Refunded'
  | 'Resolved';

/** Wire format is lowercase in both directions (SCTS D-18). */
export type Role = 'client' | 'freelancer' | 'arbiter';

export interface Deliverable {
  link: string;
  note: string;
  submitted_at: Nanos;
  revision_index: number;
}

export interface Evidence {
  link: string;
  note: string;
  submitted_at: Nanos;
}

export interface Resolution {
  freelancer_percent: number;
  resolved_by: 'Arbiter' | 'MutualSettlement';
  resolved_at: Nanos;
}

export interface Distribution {
  freelancer_amount: U128;
  client_amount: U128;
}

export interface Project {
  id: number;
  client: string;
  freelancer: string;
  arbiter: string;
  title: string;
  description: string;
  amount: U128;
  status: ProjectStatus;
  created_at: Nanos;
  accept_deadline: Nanos;
  work_deadline: Nanos;
  review_deadline: Nanos;
  revision_deadline: Nanos;
  dispute_opened_at: Nanos;
  revision_count: number;
  deliverable: Deliverable | null;
  client_evidence: Evidence | null;
  freelancer_evidence: Evidence | null;
  client_settlement_proposal: number | null;
  freelancer_settlement_proposal: number | null;
  resolution: Resolution | null;
  distribution: Distribution | null;
}

export interface Stats {
  projects_as_client: number;
  projects_as_freelancer: number;
  completed_as_freelancer: number;
  total_value_as_freelancer: U128;
  disputes_as_freelancer: number;
  disputes_released_fully: number;
  refunds_as_client: number;
}

export interface Config {
  owner: string;
  min_escrow: U128;
  accept_window: Nanos;
  review_window: Nanos;
  revision_extension: Nanos;
  arbiter_window: Nanos;
  max_revisions: number;
  paused: boolean;
  state_version: number;
}

export interface FundInvariants {
  locked: U128;
  claimable: U128;
  balance: U128;
  storage_stake: U128;
  spendable: U128;
  ok: boolean;
}

export interface WithdrawalRecord {
  amount: U128;
  started_at: Nanos;
}

export interface ProjectClaim {
  project_id: number;
  /** get_projects accepts 'arbiter' as well, and echoes it back. */
  role: Role;
  /** Historical allocation, NOT a remaining balance (SCTS §6.3). */
  allocated: U128;
  status: ProjectStatus;
}

/** `0` always means "not applicable", never "the epoch". */
export function isApplicable(nanos: Nanos | null | undefined): nanos is Nanos {
  return !!nanos && nanos !== '0';
}
