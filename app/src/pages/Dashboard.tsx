import { Link } from 'react-router-dom';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useWallet } from '../hooks/useWallet';
import { getClaimable, getConfig, getFundInvariants, getProjects, withdraw } from '../lib/near/contract';
import { formatNear, shortAddress } from '../lib/format';
import { mapError } from '../lib/errors';
import { ProjectCard } from '../components/ProjectCard';

/** Chain state is the only source of truth; polling keeps counts honest. */
const REFETCH_MS = 15_000;

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
    <main className="page">
      <section className="hero">
        <h1>Money held by code, not by a platform.</h1>
        <p>
          The client locks the escrow before work starts. The freelancer is paid on approval, and
          nobody in the middle can move the funds.
        </p>
        <div className="hero-stats">
          <div className="stat">
            <span className="stat-label">Your withdrawable</span>
            <span className="stat-value">
              {accountId ? (claimable.data ? `${formatNear(claimable.data)} NEAR` : '—') : 'connect'}
            </span>
            {accountId && withdrawable > 0n && (
              <button
                className="btn btn-small"
                onClick={() => withdrawal.mutate()}
                disabled={withdrawal.isPending}
              >
                {withdrawal.isPending ? 'Withdrawing…' : 'Withdraw'}
              </button>
            )}
            {withdrawalError && (
              <small className="field-error">
                {withdrawalError.code ? `${withdrawalError.code}: ` : ''}
                {withdrawalError.message}
              </small>
            )}
          </div>
          <div className="stat">
            <span className="stat-label">Minimum escrow</span>
            <span className="stat-value">
              {config.data ? `${formatNear(config.data.min_escrow)} NEAR` : '—'}
            </span>
          </div>
          <div className="stat">
            <span className="stat-label">Projects you are in</span>
            <span className="stat-value">{accountId ? claims.length : '—'}</span>
          </div>
        </div>
      </section>

      {queryError && (
        <div className="notice notice-error">
          <strong>{mapError(queryError).message}</strong>
          <span>{mapError(queryError).raw}</span>
        </div>
      )}

      {!accountId ? (
        <section className="empty">
          <h2>Connect a wallet to see your projects</h2>
          <p>
            Every project you are part of — as client or freelancer — is read from the contract, not
            from a local profile.
          </p>
          <button className="btn" onClick={connect}>
            Connect wallet
          </button>
        </section>
      ) : loadingProjects ? (
        <section className="empty">
          <p>Reading projects from the chain…</p>
        </section>
      ) : claims.length === 0 ? (
        <section className="empty">
          <h2>No projects yet</h2>
          <p>
            When someone names you as the freelancer — or you fund an escrow — the project appears
            here.
          </p>
          <Link className="btn" to="/new">
            Fund your first escrow
          </Link>
        </section>
      ) : (
        <section className="projects">
          <h2>Your projects</h2>
          <div className="project-grid">
            {claims.map((claim) => (
              <ProjectCard key={`${claim.project_id}-${claim.role}`} claim={claim} />
            ))}
          </div>
        </section>
      )}

      <section className="health">
        <h2>Contract health</h2>
        <p className="muted">
          Read straight from <code>{shortAddress('nescrow-escrow.testnet', 10)}</code>. The reserve is
          the contract's own balance — it pays for storage and never touches escrow.
        </p>
        <dl className="health-grid">
          <div>
            <dt>Locked in escrow</dt>
            <dd>{invariants.data ? `${formatNear(invariants.data.locked)} NEAR` : '—'}</dd>
          </div>
          <div>
            <dt>Awaiting withdrawal</dt>
            <dd>{invariants.data ? `${formatNear(invariants.data.claimable)} NEAR` : '—'}</dd>
          </div>
          <div>
            <dt>Reserve (spendable)</dt>
            <dd>{invariants.data ? `${formatNear(invariants.data.spendable)} NEAR` : '—'}</dd>
          </div>
          <div>
            <dt>Solvency check</dt>
            <dd>
              {invariants.data ? (
                <span className={invariants.data.ok ? 'ok' : 'bad'}>
                  {invariants.data.ok ? 'balanced' : 'violated'}
                </span>
              ) : (
                '—'
              )}
            </dd>
          </div>
        </dl>
        {config.data?.paused && (
          <div className="notice notice-warn">
            New projects are paused. Existing escrows can still settle and be withdrawn.
          </div>
        )}
      </section>
    </main>
  );
}
