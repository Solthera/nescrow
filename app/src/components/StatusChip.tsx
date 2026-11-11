import type { ProjectStatus } from '../types/contract';

const LABEL: Record<ProjectStatus, string> = {
  Funded: 'Funded',
  Accepted: 'Accepted',
  Delivered: 'Delivered',
  RevisionRequested: 'Revision',
  Disputed: 'Disputed',
  Released: 'Released',
  Refunded: 'Refunded',
  Resolved: 'Resolved',
};

export function StatusChip({ status, quiet = false }: { status: ProjectStatus; quiet?: boolean }) {
  const tone = status === 'Disputed' ? ' chip danger' : quiet ? ' chip quiet' : ' chip';
  return <span className={tone}>{LABEL[status]}</span>;
}
