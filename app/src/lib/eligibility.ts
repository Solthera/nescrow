import type { Config, Project } from '../types/contract';

/**
 * The single source of truth for which buttons exist (Integration Spec §8).
 * Pure on purpose: no React, no fetching, so it can be unit-tested against the
 * matrix row by row.
 */
export type ActionId =
  | 'accept_project'
  | 'reject_project'
  | 'submit_deliverable'
  | 'approve_deliverable'
  | 'request_revision'
  | 'release_after_review'
  | 'claim_no_response_refund'
  | 'claim_deadline_refund'
  | 'open_dispute'
  | 'submit_evidence'
  | 'propose_settlement'
  | 'settle_mutual'
  | 'arbiter_resolve'
  | 'withdraw';

export interface Action {
  id: ActionId;
  label: string;
  style: 'primary' | 'secondary' | 'destructive';
  hint?: string;
}

export interface EligibilityResult {
  actions: Action[];
  /** Shown when there is nothing to do (matrix rule 4): never an empty area. */
  reason: string;
}

const LABELS: Record<ActionId, Omit<Action, 'id'>> = {
  accept_project: { label: 'Accept project', style: 'primary' },
  reject_project: { label: 'Reject project', style: 'destructive' },
  submit_deliverable: { label: 'Submit deliverable', style: 'primary' },
  approve_deliverable: { label: 'Approve & release funds', style: 'primary' },
  request_revision: { label: 'Request revision', style: 'secondary' },
  release_after_review: { label: 'Release to freelancer', style: 'secondary' },
  claim_no_response_refund: { label: 'Refund (no response)', style: 'secondary' },
  claim_deadline_refund: { label: 'Refund (deadline missed)', style: 'secondary' },
  open_dispute: { label: 'Open dispute', style: 'destructive' },
  submit_evidence: { label: 'Submit evidence', style: 'secondary' },
  propose_settlement: { label: 'Propose settlement', style: 'secondary' },
  settle_mutual: { label: 'Settle at that split', style: 'primary' },
  arbiter_resolve: { label: 'Decide the split', style: 'primary' },
  withdraw: { label: 'Withdraw', style: 'primary' },
};

function action(id: ActionId, hint?: string): Action {
  return { id, ...LABELS[id], hint };
}

function days(ms: number): string {
  const d = Math.floor(ms / 86_400_000);
  if (d >= 1) return `${d} day${d === 1 ? '' : 's'}`;
  const h = Math.max(1, Math.floor(ms / 3_600_000));
  return `${h} hour${h === 1 ? '' : 's'}`;
}

export function availableActions(
  project: Project,
  me: string | null,
  nowNanos: bigint,
  config: Config,
  claimable: bigint,
): EligibilityResult {
  const role: 'client' | 'freelancer' | 'arbiter' | 'other' =
    me === project.client
      ? 'client'
      : me === project.freelancer
        ? 'freelancer'
        : me === project.arbiter
          ? 'arbiter'
          : 'other';

  const actions: Action[] = [];
  const now = nowNanos;
  const acceptDeadline = BigInt(project.accept_deadline);
  const workDeadline = BigInt(project.work_deadline);
  const reviewDeadline = BigInt(project.review_deadline || '0');
  const revisionDeadline = BigInt(project.revision_deadline || '0');

  // Withdraw is driven by balance, not by status (matrix rule 2).
  if (me && claimable > 0n) {
    actions.push(action('withdraw', 'Your allocation is ready to move to your wallet.'));
  }

  const reviewLive = reviewDeadline > 0n && now < reviewDeadline;
  const reviewOver = reviewDeadline > 0n && now >= reviewDeadline;
  const timestamp = (nanos: bigint) => new Date(Number(nanos / 1_000_000n)).toLocaleString();

  let reason = '';

  switch (project.status) {
    case 'Funded': {
      if (role === 'freelancer') {
        if (now < acceptDeadline) {
          actions.push(action('accept_project', 'Starts the 48-hour acceptance clock turning into real work.'));
          actions.push(action('reject_project', 'Returns the escrow to the client in full.'));
        } else {
          reason = `The acceptance window closed. The client can reclaim the escrow.`;
        }
      } else if (role === 'client') {
        if (now >= acceptDeadline) {
          actions.push(action('claim_no_response_refund', 'The freelancer did not respond in time.'));
        } else {
          reason = `Waiting for the freelancer to accept. Window closes in ${days(Number(acceptDeadline - now) / 1e6)}.`;
        }
      } else {
        reason = 'Waiting for the freelancer to accept the project.';
      }
      break;
    }
    case 'Accepted': {
      if (role === 'freelancer') {
        if (now < workDeadline) {
          actions.push(action('submit_deliverable', `Due in ${days(Number(workDeadline - now) / 1e6)}.`));
        } else {
          reason = 'The work deadline has passed. The client can reclaim the escrow.';
        }
      } else if (role === 'client') {
        if (now >= workDeadline) {
          actions.push(action('claim_deadline_refund', 'Nothing was submitted before the deadline.'));
        } else {
          reason = `Waiting for the deliverable. Deadline in ${days(Number(workDeadline - now) / 1e6)}.`;
        }
      } else {
        reason = 'Waiting for the freelancer to submit the deliverable.';
      }
      break;
    }
    case 'Delivered': {
      if (role === 'client' && reviewLive) {
        actions.push(action('approve_deliverable', 'Releases the full amount to the freelancer. This is final.'));
        if (project.revision_count < config.max_revisions) {
          actions.push(action('request_revision', `Adds ${config.revision_extension ? '3 days' : 'time'} to the deadline. ${config.max_revisions - project.revision_count} left.`));
        }
        actions.push(action('open_dispute', 'Freezes the funds until the arbiter or a mutual settlement decides.'));
      } else if (role === 'freelancer' && reviewLive) {
        actions.push(action('open_dispute', 'Freezes the funds until the arbiter or a mutual settlement decides.'));
      } else if (reviewOver) {
        actions.push(action('release_after_review', `The client did not review in time. Anyone can release now.`));
      } else if (role === 'client' || role === 'freelancer') {
        reason = `Review period ends ${timestamp(reviewDeadline)}.`;
      } else {
        reason = `Waiting for the client to review. Review period ends ${timestamp(reviewDeadline)}.`;
      }
      break;
    }
    case 'RevisionRequested': {
      if (role === 'freelancer') {
        if (now < revisionDeadline) {
          actions.push(action('submit_deliverable', `Revision due in ${days(Number(revisionDeadline - now) / 1e6)}.`));
        }
        // Deliberate: no time limit on disputing a stuck revision (D-14).
        actions.push(action('open_dispute', 'Available even after the revision deadline — funds never get stuck.'));
      } else if (role === 'client') {
        actions.push(action('open_dispute', 'Available at any time while a revision is open.'));
        if (!(now < revisionDeadline)) {
          reason = 'The freelancer missed the revision deadline. A dispute is the only way to settle it.';
        } else {
          reason = `Waiting for the revised work. Due in ${days(Number(revisionDeadline - now) / 1e6)}.`;
        }
      } else {
        reason = 'Waiting for the revised deliverable.';
      }
      break;
    }
    case 'Disputed': {
      const arbiterExpiry = BigInt(project.dispute_opened_at) + BigInt(config.arbiter_window);
      if (role === 'client' || role === 'freelancer') {
        const mine = role === 'client' ? project.client_evidence : project.freelancer_evidence;
        if (!mine) {
          actions.push(action('submit_evidence', 'One submission per party. Be specific.'));
        }
        actions.push(action('propose_settlement', 'An offer the other side can accept.'));
        actions.push(action('settle_mutual', 'Accepts the other side’s proposal and closes the dispute.'));
      } else if (role === 'arbiter') {
        if (now <= arbiterExpiry) {
          actions.push(action('arbiter_resolve', `Decision window closes ${timestamp(arbiterExpiry)}.`));
        } else {
          reason = 'The 7-day decision window has passed, so the arbiter can no longer decide.';
        }
      } else {
        reason = 'Waiting for the arbiter or a mutual settlement.';
      }
      break;
    }
    case 'Released':
    case 'Resolved':
    case 'Refunded': {
      if (actions.length === 0) {
        reason = 'This project is settled. Nothing is left to do on-chain.';
      }
      break;
    }
  }

  return { actions, reason };
}
