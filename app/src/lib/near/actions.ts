import type { FinalExecutionOutcome } from 'near-api-js';
import type { Project } from '../../types/contract';
import type { ActionId } from '../eligibility';
import { call, GAS_WITHDRAW, withdraw } from './contract';

export interface ActionInput {
  link?: string;
  note?: string;
  percent?: number;
}

/**
 * Maps a UI action to exactly one contract call. Kept separate from the button so
 * the eligibility matrix can be tested without a wallet.
 */
export async function submitAction(
  id: ActionId,
  project: Project,
  input: ActionInput = {},
): Promise<FinalExecutionOutcome | void> {
  const projectId = project.id;

  switch (id) {
    case 'accept_project':
      return call('accept_project', { project_id: projectId });
    case 'reject_project':
      return call('reject_project', { project_id: projectId });
    case 'submit_deliverable':
      return call('submit_deliverable', {
        project_id: projectId,
        link: input.link ?? '',
        note: input.note ?? '',
      });
    case 'approve_deliverable':
      return call('approve_deliverable', { project_id: projectId });
    case 'request_revision':
      return call('request_revision', { project_id: projectId });
    case 'release_after_review':
      return call('release_after_review', { project_id: projectId });
    case 'claim_no_response_refund':
      return call('claim_no_response_refund', { project_id: projectId });
    case 'claim_deadline_refund':
      return call('claim_deadline_refund', { project_id: projectId });
    case 'open_dispute':
      return call('open_dispute', { project_id: projectId });
    case 'submit_evidence':
      return call('submit_evidence', {
        project_id: projectId,
        link: input.link ?? '',
        note: input.note ?? '',
      });
    case 'propose_settlement':
      return call('propose_settlement', {
        project_id: projectId,
        freelancer_percent: input.percent ?? 0,
      });
    case 'settle_mutual':
      return call('settle_mutual', {
        project_id: projectId,
        freelancer_percent: input.percent ?? 0,
      });
    case 'arbiter_resolve':
      return call('arbiter_resolve', {
        project_id: projectId,
        freelancer_percent: input.percent ?? 0,
      });
    case 'withdraw':
      return withdraw();
  }
}

/** Actions that need typed input before they can be submitted. */
export function actionFields(id: ActionId): Array<'link' | 'note' | 'percent'> {
  switch (id) {
    case 'submit_deliverable':
    case 'submit_evidence':
      return ['link', 'note'];
    case 'propose_settlement':
    case 'settle_mutual':
    case 'arbiter_resolve':
      return ['percent'];
    default:
      return [];
  }
}

export { GAS_WITHDRAW };
