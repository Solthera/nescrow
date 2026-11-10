import type { ProjectStatus } from '../types/contract';

const LABELS: Record<ProjectStatus, string> = {
  Funded: 'Funded',
  Accepted: 'Accepted',
  Delivered: 'Delivered',
  RevisionRequested: 'Revision requested',
  Disputed: 'Disputed',
  Released: 'Released',
  Refunded: 'Refunded',
  Resolved: 'Resolved',
};

const TONE: Record<ProjectStatus, string> = {
  Funded: 'tone-wait',
  Accepted: 'tone-work',
  Delivered: 'tone-work',
  RevisionRequested: 'tone-warn',
  Disputed: 'tone-alarm',
  Released: 'tone-done',
  Refunded: 'tone-done',
  Resolved: 'tone-done',
};

export function StatusBadge({ status }: { status: ProjectStatus }) {
  return <span className={`badge ${TONE[status]}`}>{LABELS[status]}</span>;
}
