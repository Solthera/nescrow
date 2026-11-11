import type { Project } from '../types/contract';
import { formatStamp } from '../lib/time';

interface Node {
  label: string;
  when: string;
  state: 'done' | 'now' | 'pending';
  alarm?: boolean;
}

/**
 * The escrow's clock, not a generic progress bar. Each node carries the timestamp
 * the contract actually stores — no invented "accepted at" that the contract does
 * not record.
 */
function buildNodes(project: Project): Node[] {
  const s = project.status;
  const terminal = s === 'Released' || s === 'Refunded' || s === 'Resolved';

  const funded: Node = { label: 'Funded', when: formatStamp(project.created_at), state: 'done' };
  const accepted: Node = {
    label: 'Accepted',
    when: formatStamp(project.accept_deadline),
    state: s === 'Funded' ? 'pending' : 'done',
  };
  const workDue: Node = {
    label: s === 'RevisionRequested' ? 'Revision due' : 'Work due',
    when: formatStamp(s === 'RevisionRequested' ? project.revision_deadline : project.work_deadline),
    state: s === 'Funded' ? 'pending' : s === 'Accepted' ? 'now' : 'done',
  };
  const review: Node = {
    label: s === 'Disputed' ? 'Dispute' : 'Review by',
    when: formatStamp(s === 'Disputed' ? project.dispute_opened_at : project.review_deadline),
    state: terminal ? 'done' : s === 'Delivered' || s === 'Disputed' ? 'now' : 'pending',
    alarm: s === 'Disputed',
  };
  const settled: Node = {
    label: 'Settled',
    when: s === 'Resolved' && project.resolution ? formatStamp(project.resolution.resolved_at) : '—',
    state: terminal ? 'done' : 'pending',
  };

  return [funded, accepted, workDue, review, settled];
}

export function Rail({ project }: { project: Project }) {
  return (
    <div className="rail">
      {buildNodes(project).map((node) => (
        <div
          key={node.label}
          className={`node ${node.state === 'done' ? 'done' : node.state === 'now' ? 'now' : ''} ${
            node.alarm ? 'alarm' : ''
          }`}
        >
          <div className="dot" />
          <b>{node.label}</b>
          <span>{node.when}</span>
        </div>
      ))}
    </div>
  );
}
