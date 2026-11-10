import { Link, useParams } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { useWallet } from '../hooks/useWallet';
import { getClaimable, getConfig, getProject } from '../lib/near/contract';
import { availableActions } from '../lib/eligibility';
import { formatNear, nowNanos } from '../lib/format';
import { mapError } from '../lib/errors';
import { ENV } from '../config/env';
import { StatusBadge } from '../components/StatusBadge';
import { ActionBar } from '../components/ActionBar';

const REFETCH_MS = 15_000;

function when(nanos: string | null | undefined): string {
  if (!nanos || nanos === '0') return '—';
  return new Date(Number(BigInt(nanos) / 1_000_000n)).toLocaleString();
}

function accountLink(account: string) {
  return `${ENV.explorerUrl}/address/${account}`;
}

export function ProjectDetail() {
  const { id } = useParams();
  const projectId = Number(id);
  const { accountId, connect } = useWallet();

  const project = useQuery({
    queryKey: ['project', projectId],
    queryFn: () => getProject(projectId),
    enabled: Number.isFinite(projectId),
    refetchInterval: REFETCH_MS,
  });
  const config = useQuery({ queryKey: ['config'], queryFn: getConfig, refetchInterval: REFETCH_MS });
  const claimable = useQuery({
    queryKey: ['claimable', accountId],
    queryFn: () => getClaimable(accountId as string),
    enabled: !!accountId,
    refetchInterval: REFETCH_MS,
  });

  if (project.isLoading) {
    return (
      <main className="page">
        <p className="muted">Reading project #{projectId} from the chain…</p>
      </main>
    );
  }

  if (project.error || !project.data) {
    const mapped = project.error ? mapError(project.error) : null;
    return (
      <main className="page">
        <Link className="back" to="/">
          ← back
        </Link>
        <div className="notice notice-error">
          <strong>{mapped?.code ? `${mapped.code}: ` : ''}{mapped?.message ?? 'Project not found.'}</strong>
          {mapped?.action && <span>{mapped.action}</span>}
        </div>
      </main>
    );
  }

  const p = project.data;
  const claimableAmount = BigInt(claimable.data ?? '0');
  const eligibility =
    config.data &&
    availableActions(p, accountId, nowNanos(), config.data, claimableAmount);

  const isParty = accountId === p.client || accountId === p.freelancer || accountId === p.arbiter;
  const counterpartyProposal =
    accountId === p.client
      ? p.freelancer_settlement_proposal
      : accountId === p.freelancer
        ? p.client_settlement_proposal
        : null;

  return (
    <main className="page">
      <Link className="back" to="/">
        ← all projects
      </Link>

      <header className="detail-head">
        <div>
          <span className="card-id">project #{p.id}</span>
          <h1>{p.title}</h1>
        </div>
        <div className="detail-head-right">
          <StatusBadge status={p.status} />
          <span className="amount">{formatNear(p.amount)} NEAR</span>
        </div>
      </header>

      <p className="detail-desc">{p.description}</p>

      <section className="detail-grid">
        <div className="panel">
          <h2>Parties</h2>
          <ul className="party-list">
            {(
              [
                ['Client', p.client],
                ['Freelancer', p.freelancer],
                ['Arbiter', p.arbiter],
              ] as const
            ).map(([label, account]) => (
              <li key={label}>
                <span className="muted small">{label}</span>
                <a href={accountLink(account)} target="_blank" rel="noreferrer">
                  {account === accountId ? `${account} (you)` : account}
                </a>
              </li>
            ))}
          </ul>
        </div>

        <div className="panel">
          <h2>Timeline</h2>
          <dl className="kv">
            <dt>Created</dt>
            <dd>{when(p.created_at)}</dd>
            <dt>Accept deadline</dt>
            <dd>{when(p.accept_deadline)}</dd>
            <dt>Work deadline</dt>
            <dd>{when(p.work_deadline)}</dd>
            <dt>Review deadline</dt>
            <dd>{when(p.review_deadline)}</dd>
            <dt>Revision deadline</dt>
            <dd>{when(p.revision_deadline)}</dd>
            {p.dispute_opened_at !== '0' && (
              <>
                <dt>Dispute opened</dt>
                <dd>{when(p.dispute_opened_at)}</dd>
              </>
            )}
          </dl>
        </div>

        <div className="panel">
          <h2>Deliverable</h2>
          {p.deliverable ? (
            <div className="stack">
              <a href={p.deliverable.link} target="_blank" rel="noreferrer">
                {p.deliverable.link}
              </a>
              {p.deliverable.note && <p className="muted">{p.deliverable.note}</p>}
              <span className="muted small">
                submitted {when(p.deliverable.submitted_at)} · revision {p.deliverable.revision_index}
              </span>
            </div>
          ) : (
            <p className="muted">Nothing submitted yet.</p>
          )}
          <p className="muted small">Revisions used: {p.revision_count}</p>
        </div>

        <div className="panel">
          <h2>Evidence</h2>
          <div className="stack">
            <div className="evidence-block">
              <span className="muted small">Client</span>
              {p.client_evidence ? (
                <a href={p.client_evidence.link} target="_blank" rel="noreferrer">
                  {p.client_evidence.link}
                </a>
              ) : (
                <p className="muted">not submitted</p>
              )}
            </div>
            <div className="evidence-block">
              <span className="muted small">Freelancer</span>
              {p.freelancer_evidence ? (
                <a href={p.freelancer_evidence.link} target="_blank" rel="noreferrer">
                  {p.freelancer_evidence.link}
                </a>
              ) : (
                <p className="muted">not submitted</p>
              )}
            </div>
          </div>
        </div>

        {(p.client_settlement_proposal !== null || p.freelancer_settlement_proposal !== null) && (
          <div className="panel">
            <h2>Settlement proposals</h2>
            <dl className="kv">
              <dt>Client proposes</dt>
              <dd>{p.client_settlement_proposal !== null ? `${p.client_settlement_proposal}% to freelancer` : '—'}</dd>
              <dt>Freelancer proposes</dt>
              <dd>
                {p.freelancer_settlement_proposal !== null
                  ? `${p.freelancer_settlement_proposal}% to freelancer`
                  : '—'}
              </dd>
            </dl>
          </div>
        )}

        {p.distribution && (
          <div className="panel">
            <h2>Outcome</h2>
            <dl className="kv">
              <dt>To freelancer</dt>
              <dd>{formatNear(p.distribution.freelancer_amount)} NEAR</dd>
              <dt>To client</dt>
              <dd>{formatNear(p.distribution.client_amount)} NEAR</dd>
              {p.resolution && (
                <>
                  <dt>Decided by</dt>
                  <dd>
                    {p.resolution.resolved_by === 'Arbiter' ? 'arbiter' : 'mutual settlement'} ·{' '}
                    {p.resolution.freelancer_percent}% to freelancer
                  </dd>
                </>
              )}
            </dl>
          </div>
        )}
      </section>

      {!accountId ? (
        <section className="actionbar actionbar-quiet">
          <span className="muted">Connect a wallet to see what you can do here.</span>
          <button className="btn" onClick={connect}>
            Connect wallet
          </button>
        </section>
      ) : eligibility ? (
        <ActionBar
          project={p}
          actions={eligibility.actions}
          reason={
            !isParty && eligibility.actions.length === 0
              ? 'You are not part of this project. Only the parties and the arbiter can act on it.'
              : eligibility.reason
          }
          counterpartyPercent={counterpartyProposal}
        />
      ) : null}
    </main>
  );
}
