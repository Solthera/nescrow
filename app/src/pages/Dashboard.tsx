import { Link } from 'react-router-dom';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useWallet } from '../hooks/useWallet';
import { getClaimable, getConfig, getFundInvariants, getProject, getProjects, withdraw } from '../lib/near/contract';
import { formatNear, formatNearFixed } from '../lib/format';
import { mapError } from '../lib/errors';
import type { ProjectClaim } from '../types/contract';
import { StatusChip } from '../components/StatusChip';

const REFETCH_MS = 15_000;

/** One row per project. The title lives on the project, not on the claim, so the
 *  row fetches it — the list stays a list instead of turning into cards. */
function ProjectRow({ claim }: { claim: ProjectClaim }) {
  const project = useQuery({
    queryKey: ['project', claim.project_id],
    queryFn: () => getProject(claim.project_id),
  });

  const counterparty = claim.role === 'client' ? project.data?.freelancer : project.data?.client;
  const amount = project.data?.amount ?? claim.allocated;

  return (
    <Link className="list-row" to={`/p/${claim.project_id}`}>
      <div className="avatar">{String(claim.project_id).padStart(2, '0')}</div>
      <div className="list-main">
        <b>{project.data?.title ?? '…'}</b>
        <span>
          {claim.role} · {counterparty ?? '…'}
        </span>
      </div>
      <div className="row-val">{formatNear(amount)} NEAR</div>
      <StatusChip status={claim.status} quiet={claim.status !== 'Disputed'} />
    </Link>
  );
}

export function Dashboard() {
  const { accountId, connect } = useWallet();

  const config = useQuery({ queryKey: ['config'], queryFn: getConfig, refetchInterval: REFETCH_MS });
  const invariants = useQuery({
    queryKey: ['invariants'],
    queryFn: getFundInvariants,
    refetchInterval: REFETCH_MS,
  });
  const asClient = useQuery({
    queryKey: ['projects', accountId, 'client'],
    queryFn: () => getProjects(accountId as string, 'client', 0, 50),
    enabled: !!accountId,
  });
  const asFreelancer = useQuery({
    queryKey: ['projects', accountId, 'freelancer'],
    queryFn: () => getProjects(accountId as string, 'freelancer', 0, 50),
    enabled: !!accountId,
  });
  const claimable = useQuery({
    queryKey: ['claimable', accountId],
    queryFn: () => getClaimable(accountId as string),
    enabled: !!accountId,
    refetchInterval: REFETCH_MS,
  });

  const queryClient = useQueryClient();
  const withdrawal = useMutation({
    mutationFn: withdraw,
    onSuccess: async () => {
      // The contract debits before transferring; refetch instead of assuming.
      await queryClient.invalidateQueries({ queryKey: ['claimable'] });
      await queryClient.invalidateQueries({ queryKey: ['invariants'] });
      await queryClient.invalidateQueries({ queryKey: ['projects'] });
    },
  });

  const withdrawable = BigInt(claimable.data ?? '0');
  const withdrawalError = withdrawal.error ? mapError(withdrawal.error) : null;
  const claims = [...(asClient.data ?? []), ...(asFreelancer.data ?? [])].sort(
    (a, b) => b.project_id - a.project_id,
  );
  const loadingProjects = asClient.isLoading || asFreelancer.isLoading;
  const queryError = asClient.error ?? asFreelancer.error ?? claimable.error ?? invariants.error;

  return (
    <main className="wrap">
      <div className="top">
        <div>
          <h1 className="page-title">Your escrow</h1>
          <p className="sub">
            {accountId ? (
              <code>{accountId}</code>
            ) : (
              'Connect a wallet — every project below is read from the contract.'
            )}
          </p>
        </div>
        <Link className="btn primary" to="/new">
          New escrow
        </Link>
      </div>

      <div className="card">
        <div className="label">withdrawable</div>
        <div className="amount">
          {accountId ? formatNearFixed(withdrawable) : '—'}
          <small>NEAR</small>
        </div>
        <div className="split-legend" style={{ marginTop: 12 }}>
          <span>
            as client <b>{claims.filter((c) => c.role === 'client').length}</b>
          </span>
          <span>
            as freelancer <b>{claims.filter((c) => c.role === 'freelancer').length}</b>
          </span>
          <span>
            minimum escrow <b>{config.data ? formatNear(config.data.min_escrow) : '—'}</b>
          </span>
        </div>
        {withdrawable > 0n && (
          <button
            className="btn primary"
            style={{ marginTop: 16 }}
            onClick={() => withdrawal.mutate()}
            disabled={withdrawal.isPending}
          >
            {withdrawal.isPending ? 'Withdrawing…' : `Withdraw ${formatNear(withdrawable)} NEAR`}
          </button>
        )}
        {withdrawalError && (
          <div className="notice notice-error">
            <strong>
              {withdrawalError.code ? `${withdrawalError.code}: ` : ''}
              {withdrawalError.message}
            </strong>
            {withdrawalError.action && <span>{withdrawalError.action}</span>}
          </div>
        )}
      </div>

      {queryError && (
        <div className="notice notice-error">
          <strong>{mapError(queryError).message}</strong>
          <span>{mapError(queryError).raw}</span>
        </div>
      )}

      {!accountId ? (
        <section className="empty">
          <h2>Connect a wallet to see your projects</h2>
          <p>Every project you are part of is read from the contract, not from a local profile.</p>
          <button className="btn primary" onClick={connect}>
            Connect wallet
          </button>
        </section>
      ) : loadingProjects ? (
        <section className="empty">
          <p className="dim">Reading projects from the chain…</p>
        </section>
      ) : claims.length === 0 ? (
        <section className="empty">
          <h2>No projects yet</h2>
          <p>When someone names you as the freelancer, or you fund an escrow, it shows up here.</p>
          <Link className="btn primary" to="/new">
            Fund your first escrow
          </Link>
        </section>
      ) : (
        <>
          <h2 className="h" style={{ marginTop: 26 }}>
            projects
          </h2>
          <div className="list">
            {claims.map((claim) => (
              <ProjectRow key={`${claim.project_id}-${claim.role}`} claim={claim} />
            ))}
          </div>
        </>
      )}

      <div className="card tight" style={{ marginTop: 26 }}>
        <h2 className="h">contract health</h2>
        <div className="rows">
          <div className="row">
            <div className="avatar">LK</div>
            <div className="row-main">
              <b>Locked in escrow</b>
              <span>held by live projects</span>
            </div>
            <div className="row-val">{invariants.data ? formatNear(invariants.data.locked) : '—'}</div>
          </div>
          <div className="row">
            <div className="avatar">AW</div>
            <div className="row-main">
              <b>Awaiting withdrawal</b>
              <span>allocated but not yet pulled</span>
            </div>
            <div className="row-val">{invariants.data ? formatNear(invariants.data.claimable) : '—'}</div>
          </div>
          <div className="row">
            <div className="avatar">RS</div>
            <div className="row-main">
              <b>Reserve</b>
              <span>pays storage, never escrow</span>
            </div>
            <div className="row-val">{invariants.data ? formatNear(invariants.data.spendable) : '—'}</div>
          </div>
          <div className="row">
            <div className="avatar">OK</div>
            <div className="row-main">
              <b>Solvency</b>
              <span>escrow + allocations vs balance</span>
            </div>
            <div className={`row-val ${invariants.data?.ok ? 'good' : 'alarm'}`}>
              {invariants.data ? (invariants.data.ok ? 'balanced' : 'violated') : '—'}
            </div>
          </div>
        </div>
        {config.data?.paused && (
          <div className="notice notice-error">
            <strong>New projects are paused.</strong>
            <span>existing escrows can still settle and be withdrawn</span>
          </div>
        )}
      </div>
    </main>
  );
}
