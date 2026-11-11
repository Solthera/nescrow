import { Link, useParams } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { useWallet } from '../hooks/useWallet';
import { getClaimable, getConfig, getProject } from '../lib/near/contract';
import { availableActions } from '../lib/eligibility';
import { formatNear, formatNearFixed, nowNanos } from '../lib/format';
import { mapError } from '../lib/errors';
import { formatStamp } from '../lib/time';
import { ENV } from '../config/env';
import { StatusChip } from '../components/StatusChip';
import { Rail } from '../components/Rail';
import { WindowMeter } from '../components/WindowMeter';
import { SplitBar } from '../components/SplitBar';
import { ActionDock } from '../components/ActionDock';

const REFETCH_MS = 15_000;

function initials(account: string): string {
  const name = account.split('.')[0] ?? account;
  return name.slice(0, 2);
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

  if (!Number.isFinite(projectId)) {
    return (
      <main className="wrap">
        <div className="top">
          <Link className="back" to="/app">
            ← projects
          </Link>
        </div>
        <div className="notice notice-error">
          <strong>This project id is not a number, so there is nothing to read.</strong>
          <span>Open a project from your dashboard instead.</span>
        </div>
      </main>
    );
  }

  if (project.isLoading) {
    return (
      <main className="wrap">
        <p className="dim">Reading project #{projectId} from the chain…</p>
      </main>
    );
  }

  if (project.error || !project.data) {
    const mapped = project.error ? mapError(project.error) : null;
    return (
      <main className="wrap">
        <div className="top">
          <Link className="back" to="/app">
            ← projects
          </Link>
        </div>
        <div className="notice notice-error">
          <strong>
            {mapped?.code ? `${mapped.code}: ` : ''}
            {mapped?.message ?? 'Project not found.'}
          </strong>
          {mapped?.action && <span>{mapped.action}</span>}
        </div>
      </main>
    );
  }

  const p = project.data;
  const now = nowNanos();
  const claimableAmount = BigInt(claimable.data ?? '0');
  const eligibility = config.data && availableActions(p, accountId, now, config.data, claimableAmount);
  const isParty = accountId === p.client || accountId === p.freelancer || accountId === p.arbiter;
  const counterpartyProposal =
    accountId === p.client
      ? p.freelancer_settlement_proposal
      : accountId === p.freelancer
        ? p.client_settlement_proposal
        : null;

  const settled = !!p.distribution;

  return (
    <main className="wrap">
      <div className="top">
        <div className="top-left">
          <Link className="back" to="/app">
            ← projects
          </Link>
          <a className="cid" href={`${ENV.explorerUrl}/address/${ENV.contractId}`} target="_blank" rel="noreferrer">
            {ENV.contractId}
          </a>
        </div>
        <StatusChip status={p.status} />
      </div>

      <div className="grid">
        <div className="col">
          <div className="card">
            <div className="label">escrowed</div>
            <div className="amount">
              {formatNearFixed(p.amount)}
              <small>NEAR</small>
            </div>
            <div className="project-title">{p.title}</div>
            <div className="sub">
              client <code>{p.client}</code> · freelancer <code>{p.freelancer}</code>
            </div>

            <WindowMeter project={p} nowNanos={now} />
            <SplitBar project={p} />
          </div>

          <div className="card">
            <h2 className="h">progress</h2>
            <Rail project={p} />
            <p className="sub" style={{ marginTop: 16 }}>
              {p.description}
            </p>
          </div>
        </div>

        <div className="col">
          <div className="card tight">
            <h2 className="h">parties</h2>
            <div className="rows">
              {(
                [
                  ['Client', p.client, settled ? p.distribution?.client_amount : null],
                  ['Freelancer', p.freelancer, settled ? p.distribution?.freelancer_amount : null],
                  ['Arbiter', p.arbiter, null],
                ] as const
              ).map(([label, account, allocation]) => (
                <div className="row" key={label}>
                  <div className="avatar">{initials(account)}</div>
                  <div className="row-main">
                    <b>
                      {label}
                      {account === accountId ? ' (you)' : ''}
                    </b>
                    <span>
                      <a href={`${ENV.explorerUrl}/address/${account}`} target="_blank" rel="noreferrer">
                        {account}
                      </a>
                    </span>
                  </div>
                  <div className={`row-val ${allocation ? 'good' : ''}`}>
                    {allocation ? formatNear(allocation) : label === 'Arbiter' ? 'idle' : '—'}
                  </div>
                </div>
              ))}
            </div>
          </div>

          <div className="card tight">
            <h2 className="h">work</h2>
            <div className="rows">
              <div className="row">
                <div className="avatar">DL</div>
                <div className="row-main">
                  <b>Deliverable</b>
                  <span>
                    {p.deliverable ? (
                      <a href={p.deliverable.link} target="_blank" rel="noreferrer">
                        {p.deliverable.link}
                      </a>
                    ) : (
                      'nothing submitted yet'
                    )}
                  </span>
                </div>
                <div className={`row-val ${p.deliverable ? 'good' : ''}`}>
                  {p.deliverable ? `v${p.deliverable.revision_index}` : '—'}
                </div>
              </div>
              <div className="row">
                <div className="avatar">REV</div>
                <div className="row-main">
                  <b>Revisions</b>
                  <span>{config.data ? `${config.data.max_revisions} allowed by the contract` : '—'}</span>
                </div>
                <div className="row-val">
                  {p.revision_count} / {config.data?.max_revisions ?? 2}
                </div>
              </div>
              <div className="row">
                <div className="avatar">DUE</div>
                <div className="row-main">
                  <b>Work deadline</b>
                  <span>{formatStamp(p.work_deadline)}</span>
                </div>
                <div className="row-val">{formatStamp(p.work_deadline)}</div>
              </div>
              {accountId && (
                <div className="row">
                  <div className="avatar">WD</div>
                  <div className="row-main">
                    <b>Withdrawable by you</b>
                    <span>allocations you can pull to your wallet</span>
                  </div>
                  <div className={`row-val ${claimableAmount > 0n ? 'good' : ''}`}>
                    {formatNear(claimableAmount)}
                  </div>
                </div>
              )}
            </div>
          </div>

          {(p.client_evidence || p.freelancer_evidence || p.client_settlement_proposal !== null ||
            p.freelancer_settlement_proposal !== null) && (
            <div className="card tight">
              <h2 className="h">dispute</h2>
              <div className="rows">
                {p.client_evidence && (
                  <div className="row">
                    <div className="avatar">CE</div>
                    <div className="row-main">
                      <b>Client evidence</b>
                      <span>
                        <a href={p.client_evidence.link} target="_blank" rel="noreferrer">
                          {p.client_evidence.link}
                        </a>
                      </span>
                    </div>
                    <div className="row-val">submitted</div>
                  </div>
                )}
                {p.freelancer_evidence && (
                  <div className="row">
                    <div className="avatar">FE</div>
                    <div className="row-main">
                      <b>Freelancer evidence</b>
                      <span>
                        <a href={p.freelancer_evidence.link} target="_blank" rel="noreferrer">
                          {p.freelancer_evidence.link}
                        </a>
                      </span>
                    </div>
                    <div className="row-val">submitted</div>
                  </div>
                )}
                {p.client_settlement_proposal !== null && (
                  <div className="row">
                    <div className="avatar">CP</div>
                    <div className="row-main">
                      <b>Client proposes</b>
                      <span>freelancer share</span>
                    </div>
                    <div className="row-val">{p.client_settlement_proposal}%</div>
                  </div>
                )}
                {p.freelancer_settlement_proposal !== null && (
                  <div className="row">
                    <div className="avatar">FP</div>
                    <div className="row-main">
                      <b>Freelancer proposes</b>
                      <span>freelancer share</span>
                    </div>
                    <div className="row-val">{p.freelancer_settlement_proposal}%</div>
                  </div>
                )}
              </div>
            </div>
          )}

          {p.resolution && (
            <div className="card tight">
              <h2 className="h">decision</h2>
              <div className="rows">
                <div className="row">
                  <div className="avatar">BY</div>
                  <div className="row-main">
                    <b>{p.resolution.resolved_by === 'Arbiter' ? 'Arbiter decision' : 'Mutual settlement'}</b>
                    <span>{formatStamp(p.resolution.resolved_at)}</span>
                  </div>
                  <div className="row-val good">{p.resolution.freelancer_percent}% to freelancer</div>
                </div>
              </div>
            </div>
          )}
        </div>
      </div>

      {!accountId ? (
        <div className="dock">
          <div className="dock-inner">
            <span className="dock-hint">Connect a wallet to see what you can do here.</span>
            <button className="btn primary" onClick={connect}>
              Connect wallet
            </button>
          </div>
        </div>
      ) : eligibility ? (
        <ActionDock
          project={p}
          actions={eligibility.actions}
          reason={
            !isParty && eligibility.actions.length === 0
              ? 'You are not part of this project. Only the parties and the arbiter can act on it.'
              : eligibility.reason
          }
          counterpartyPercent={counterpartyProposal}
          claimable={claimableAmount}
        />
      ) : null}
    </main>
  );
}
