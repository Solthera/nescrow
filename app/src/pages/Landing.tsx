import { Link } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { getConfig, getFundInvariants, getProjectCount } from '../lib/near/contract';
import { formatNear, formatNearFixed } from '../lib/format';
import { ENV } from '../config/env';

const STEPS = [
  {
    n: '01',
    title: 'The client funds the job',
    body: 'One transaction deposits the escrow into the contract. No invoice, no platform balance, no middleman account.',
    meta: 'funded',
  },
  {
    n: '02',
    title: 'The freelancer accepts',
    body: '48 hours to say yes. Silence is not money: if nobody accepts, the client takes the escrow back in full.',
    meta: '48h window',
  },
  {
    n: '03',
    title: 'Work is delivered',
    body: 'A link and a note, on-chain. Two revisions are allowed, each adding three days — and the escrow stays locked.',
    meta: '2 revisions',
  },
  {
    n: '04',
    title: 'It settles, then you withdraw',
    body: 'Approve, wait out the 72-hour review, or dispute it. Settlement records an allocation; each side pulls its own money out.',
    meta: '72h review',
  },
];

/** 168 hours is correct and unreadable; 7 days is the same fact. */
function humanDuration(nanos: string): string {
  const hours = Number(BigInt(nanos) / 3_600_000_000_000n);
  if (hours % 24 === 0) {
    const days = hours / 24;
    return `${days} day${days === 1 ? '' : 's'}`;
  }
  return `${hours} hours`;
}

const CANNOT = [
  ['Move your money', 'No owner key, no pause on withdrawals, no upgrade that can touch escrow.'],
  ['Take a fee', 'The platform fee is zero. The contract has no fee field to configure.'],
  ['Force a split', 'An absent arbiter leaves funds locked — it never silently divides them.'],
  ['Hold your funds', 'Payments are pull, not push. The contract allocates, you withdraw.'],
];

export function Landing() {
  const invariants = useQuery({ queryKey: ['invariants'], queryFn: getFundInvariants, refetchInterval: 20_000 });
  const config = useQuery({ queryKey: ['config'], queryFn: getConfig });
  const count = useQuery({ queryKey: ['projectCount'], queryFn: getProjectCount, refetchInterval: 20_000 });

  return (
    <main className="landing">
      <section className="hero-l">
        <div className="hero-l-copy">
          <span className="chip quiet">NEAR testnet · live</span>
          <h1 className="display">
            Escrow that neither side
            <br />
            can walk away from.
          </h1>
          <p className="lede">
            A client funds the job before work starts. The money leaves only when the work is
            approved — or when an arbiter both sides named decides. Nobody in between ever holds it.
          </p>
          <div className="cta-row">
            <Link className="btn primary" to="/app">
              Open the app
            </Link>
            <a
              className="btn ghost"
              href={`${ENV.explorerUrl}/address/${ENV.contractId}`}
              target="_blank"
              rel="noreferrer"
            >
              View the contract
            </a>
          </div>
          <p className="fineprint">
            No sign-up. A NEAR wallet is the account. Testnet funds only.
          </p>
        </div>

        <aside className="proof">
          <div className="proof-head">
            <span className="label">contract state</span>
            <span className={`chip ${invariants.data?.ok ? '' : 'danger'}`}>
              {invariants.data ? (invariants.data.ok ? 'solvent' : 'violation') : 'reading'}
            </span>
          </div>
          <div className="proof-amount">
            {invariants.data ? formatNearFixed(invariants.data.locked) : '—'}
            <small>NEAR escrowed</small>
          </div>
          <dl className="proof-rows">
            <div>
              <dt>awaiting withdrawal</dt>
              <dd>{invariants.data ? formatNear(invariants.data.claimable) : '—'}</dd>
            </div>
            <div>
              <dt>projects created</dt>
              <dd>{count.data ?? '—'}</dd>
            </div>
            <div>
              <dt>minimum escrow</dt>
              <dd>{config.data ? formatNear(config.data.min_escrow) : '—'}</dd>
            </div>
          </dl>
          <a className="proof-link" href={`${ENV.explorerUrl}/address/${ENV.contractId}`} target="_blank" rel="noreferrer">
            {ENV.contractId} ↗
          </a>
        </aside>
      </section>

      <section className="section">
        <div className="section-head">
          <h2 className="h">how it works</h2>
          <p className="sub">Four states, each with a deadline the contract enforces.</p>
        </div>
        <div className="steps">
          {STEPS.map((step) => (
            <article className="step-item" key={step.n}>
              <span className="step-n">{step.n}</span>
              <h3>{step.title}</h3>
              <p>{step.body}</p>
              <span className="step-meta">{step.meta}</span>
            </article>
          ))}
        </div>
      </section>

      <section className="section">
        <div className="section-head">
          <h2 className="h">what it cannot do</h2>
          <p className="sub">
            The product is defined by what it refuses. These are enforced by the contract, not by
            policy.
          </p>
        </div>
        <div className="cannot">
          {CANNOT.map(([title, body]) => (
            <div className="cannot-item" key={title}>
              <span className="cannot-mark">×</span>
              <div>
                <b>{title}</b>
                <p>{body}</p>
              </div>
            </div>
          ))}
        </div>
      </section>

      <section className="section">
        <div className="section-head">
          <h2 className="h">the rules, as deployed</h2>
          <p className="sub">Read from the contract, not from this page.</p>
        </div>
        <dl className="spec-table">
          <div>
            <dt>acceptance window</dt>
            <dd>{config.data ? humanDuration(config.data.accept_window) : '—'}</dd>
          </div>
          <div>
            <dt>review window</dt>
            <dd>{config.data ? humanDuration(config.data.review_window) : '—'}</dd>
          </div>
          <div>
            <dt>revision extension</dt>
            <dd>{config.data ? `${humanDuration(config.data.revision_extension)} each` : '—'}</dd>
          </div>
          <div>
            <dt>arbiter window</dt>
            <dd>{config.data ? humanDuration(config.data.arbiter_window) : '—'}</dd>
          </div>
          <div>
            <dt>maximum revisions</dt>
            <dd>{config.data?.max_revisions ?? '—'}</dd>
          </div>
          <div>
            <dt>platform fee</dt>
            <dd>0%</dd>
          </div>
        </dl>
      </section>

      <section className="landing-cta">
        <h2>Fund a job, or take one.</h2>
        <p className="sub">
          Two accounts, one arbiter, and a contract that does not need to trust either of them.
        </p>
        <Link className="btn primary" to="/app">
          Open the app
        </Link>
      </section>
    </main>
  );
}
