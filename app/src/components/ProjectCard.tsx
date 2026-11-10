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
        <span className="amount">{formatNear(claim.allocated)} NEAR</span>
        <span className="muted small">
          {claim.role === 'client' ? 'client' : 'freelancer'}
        </span>
      </div>
    </Link>
  );
}
