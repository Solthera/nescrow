import type { Project } from '../types/contract';
import { formatNearFixed } from '../lib/format';

/**
 * Shows where the money goes before the click. After a settlement it shows the
 * recorded distribution; before one it shows what approval would do (the whole
 * escrow to the freelancer).
 */
export function SplitBar({ project }: { project: Project }) {
  const total = BigInt(project.amount || '0');
  const freelancer = project.distribution ? BigInt(project.distribution.freelancer_amount) : total;
  const client = project.distribution ? BigInt(project.distribution.client_amount) : 0n;

  const percent = total > 0n ? Number((freelancer * 100n) / total) : 0;
  const settled = !!project.distribution;
  const caption = settled ? 'settled' : 'if you approve';

  return (
    <div className="split">
      <div className="split-bar">
        <div className="a" style={{ width: `${percent}%` }} />
        <div className="b" style={{ width: `${100 - percent}%` }} />
      </div>
      <div className="split-legend">
        <span>
          {caption} — freelancer <b>{formatNearFixed(freelancer)}</b>
        </span>
        <span>
          client <b>{formatNearFixed(client)}</b>
        </span>
      </div>
    </div>
  );
}
