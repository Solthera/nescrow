import type { Project } from '../types/contract';
import { countdown } from '../lib/time';

interface Props {
  project: Project;
  nowNanos: bigint;
}

/**
 * Which window is live depends on the status, and each one has a different start:
 * the review window starts when the deliverable was submitted, the dispute window
 * when the dispute was opened. Using the wrong start would make the meter lie.
 */
export function WindowMeter({ project, nowNanos }: Props) {
  const s = project.status;

  let start = project.created_at;
  let end = project.accept_deadline;
  let label = 'Accept window closes in';

  if (s === 'Accepted') {
    label = 'Work deadline in';
    end = project.work_deadline;
  } else if (s === 'Delivered') {
    label = 'Review window closes in';
    start = project.deliverable?.submitted_at ?? project.created_at;
    end = project.review_deadline;
  } else if (s === 'RevisionRequested') {
    label = 'Revision due in';
    start = project.deliverable?.submitted_at ?? project.created_at;
    end = project.revision_deadline;
  } else if (s === 'Disputed') {
    label = 'Arbiter window closes in';
    start = project.dispute_opened_at;
    end = '0';
  }

  if (s === 'Released' || s === 'Refunded' || s === 'Resolved') return null;

  const cd = countdown(start, end, nowNanos);
  if (cd.text === '—') return null;

  return (
    <div className="meter">
      <div className="meter-top">
        <span>{label}</span>
        <b>{cd.text}</b>
      </div>
      <div className="track">
        <div className={`fill ${cd.expired ? 'danger' : ''}`} style={{ width: `${Math.round(cd.ratio * 100)}%` }} />
      </div>
    </div>
  );
}
