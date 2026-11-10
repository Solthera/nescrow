import { Link } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import type { ProjectClaim } from '../types/contract';
import { getProject } from '../lib/near/contract';
import { formatNear } from '../lib/format';
import { StatusBadge } from './StatusBadge';

export function ProjectCard({ claim }: { claim: ProjectClaim }) {
  const project = useQuery({
    queryKey: ['project', claim.project_id],
    queryFn: () => getProject(claim.project_id),
  });

  const counterparty =
    claim.role === 'client' ? project.data?.freelancer : project.data?.client;
  // `allocated` is 0 until a settlement writes a distribution, so it is not the
  // project's value — the escrow amount is (SCTS §6.3).
  const allocation = BigInt(claim.allocated ?? '0');
  const escrowAmount = project.data?.amount ?? claim.allocated;

  // Defensive: an undefined id used to render "/p/undefined" and then hit the
  // contract with `project_id: null`. Better to show nothing than a dead card.
  if (claim.project_id === undefined || claim.project_id === null) return null;

  return (
    <Link className="card card-link" to={`/p/${claim.project_id}`}>
      <div className="card-top">
        <span className="card-id">#{claim.project_id}</span>
        <StatusBadge status={claim.status} />
      </div>
      <h3>{project.data?.title ?? '…'}</h3>
      <p className="card-line">
        {claim.role === 'client' ? 'you hired' : 'you were hired by'}{' '}
        <code>{counterparty ?? '…'}</code>
      </p>
      <div className="card-bottom">
        <span className="amount">{formatNear(escrowAmount)} NEAR</span>
        <span className="muted small">
          {allocation > 0n
            ? `allocated ${formatNear(allocation)}`
            : claim.role === 'client'
              ? 'client'
              : claim.role === 'freelancer'
                ? 'freelancer'
                : 'arbiter'}
        </span>
      </div>
    </Link>
  );
}
