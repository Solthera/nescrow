import { useMemo, useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useWallet } from '../hooks/useWallet';
import { createProject, getConfig } from '../lib/near/contract';
import { formatNear, nowNanos, parseNearToYocto } from '../lib/format';
import { createProjectSchema } from '../lib/validation';
import { readSuccessValue } from '../lib/tx';
import { mapError } from '../lib/errors';

const DAY_MS = 86_400_000;

function defaultDeadline(): string {
  // A local <input type="date"> value, seven days out.
  const date = new Date(Date.now() + 7 * DAY_MS);
  return date.toISOString().slice(0, 10);
}

/** Local midnight of the chosen day, as a chain timestamp string. */
function dateToNanos(date: string): string {
  const parsed = new Date(`${date}T23:59:59`);
  return (BigInt(parsed.getTime()) * 1_000_000n).toString();
}

export function CreateProject() {
  const { accountId, connect } = useWallet();
  const navigate = useNavigate();
  const queryClient = useQueryClient();

  const config = useQuery({ queryKey: ['config'], queryFn: getConfig });

  const [form, setForm] = useState({
    freelancer: '',
    arbiter: '',
    title: '',
    description: '',
    amountNear: '1',
    deadline: defaultDeadline(),
  });
  const [errors, setErrors] = useState<Record<string, string>>({});

  const depositYocto = useMemo(() => {
    try {
      return parseNearToYocto(form.amountNear);
    } catch {
      return 0n;
    }
  }, [form.amountNear]);

  const mutation = useMutation<number | null, Error, void>({
    mutationFn: async () => {
      if (!accountId || !config.data) throw new Error('Connect a wallet first.');
      const schema = createProjectSchema({
        clientId: accountId,
        minEscrowYocto: BigInt(config.data.min_escrow),
        acceptWindowNs: BigInt(config.data.accept_window),
        nowNanos: nowNanos(),
      });
      const parsed = schema.safeParse({
        freelancer: form.freelancer.trim(),
        arbiter: form.arbiter.trim(),
        title: form.title.trim(),
        description: form.description.trim(),
        amountNear: form.amountNear.trim(),
        workDeadline: dateToNanos(form.deadline),
      });

      if (!parsed.success) {
        const fieldErrors: Record<string, string> = {};
        for (const issue of parsed.error.issues) {
          const key = String(issue.path[0] ?? 'form');
          fieldErrors[key] ??= issue.message;
        }
        setErrors(fieldErrors);
        throw new Error('check the form');
      }

      setErrors({});
      const outcome = await createProject(
        {
          freelancer: parsed.data.freelancer,
          arbiter: parsed.data.arbiter,
          title: parsed.data.title,
          description: parsed.data.description,
          work_deadline: parsed.data.workDeadline,
        },
        parseNearToYocto(parsed.data.amountNear),
      );
      return readSuccessValue<number>(outcome);
    },
    onSuccess: async (projectId) => {
      await queryClient.invalidateQueries({ queryKey: ['projects'] });
      await queryClient.invalidateQueries({ queryKey: ['invariants'] });
      navigate(typeof projectId === 'number' ? `/p/${projectId}` : '/');
    },
    onError: (error: unknown) => {
      // Form-level validation failures are already rendered per field.
      if (error instanceof Error && error.message === 'check the form') return;
    },
  });

  if (!accountId) {
    return (
      <main className="wrap">
        <section className="empty">
          <h2>Connect a wallet to fund an escrow</h2>
          <p>The escrow is deposited in the same transaction that creates the project.</p>
          <button className="btn primary" onClick={connect}>
            Connect wallet
          </button>
        </section>
      </main>
    );
  }

  // Validation failures are already rendered per field; only chain/wallet errors
  // get the E-code treatment.
  const mapped =
    mutation.error && mutation.error.message !== 'check the form'
      ? mapError(mutation.error)
      : null;

  return (
    <main className="wrap">
      <Link className="back" to="/app">
        ← all projects
      </Link>
      <h1 className="page-title" style={{ marginTop: 4 }}>Fund a project</h1>
      <p className="sub">
        You are the client. The money leaves your wallet now and is held by the contract —
        not by us, not by the freelancer — until the work is approved or a decision is made.
      </p>

      <form
        className="form-grid"
        onSubmit={(event) => {
          event.preventDefault();
          mutation.mutate();
        }}
      >
        <label className="field">
          <span>Freelancer account</span>
          <input
            value={form.freelancer}
            onChange={(event) => setForm((prev) => ({ ...prev, freelancer: event.target.value }))}
            placeholder="nescrow-freelancer.testnet"
            autoComplete="off"
          />
          {errors.freelancer && <small className="field-error">{errors.freelancer}</small>}
        </label>

        <label className="field">
          <span>Arbiter account</span>
          <input
            value={form.arbiter}
            onChange={(event) => setForm((prev) => ({ ...prev, arbiter: event.target.value }))}
            placeholder="nescrow-arbiter.testnet"
            autoComplete="off"
          />
          {errors.arbiter && <small className="field-error">{errors.arbiter}</small>}
        </label>

        <label className="field">
          <span>Title</span>
          <input
            value={form.title}
            onChange={(event) => setForm((prev) => ({ ...prev, title: event.target.value }))}
            placeholder="Logo for a Berlin client"
            maxLength={128}
          />
          {errors.title && <small className="field-error">{errors.title}</small>}
        </label>

        <label className="field field-wide">
          <span>Brief</span>
          <textarea
            rows={3}
            value={form.description}
            onChange={(event) => setForm((prev) => ({ ...prev, description: event.target.value }))}
            placeholder="What is being delivered, and what counts as done."
            maxLength={2000}
          />
          {errors.description && <small className="field-error">{errors.description}</small>}
        </label>

        <label className="field">
          <span>Escrow amount (NEAR)</span>
          <input
            value={form.amountNear}
            onChange={(event) => setForm((prev) => ({ ...prev, amountNear: event.target.value }))}
            inputMode="decimal"
          />
          <small className="muted">
            {depositYocto > 0n
              ? `deposits exactly ${formatNear(depositYocto)} NEAR`
              : 'a decimal amount, at most 24 decimals'}
            {config.data ? ` · minimum ${formatNear(config.data.min_escrow)} NEAR` : ''}
          </small>
          {errors.amountNear && <small className="field-error">{errors.amountNear}</small>}
        </label>

        <label className="field">
          <span>Work deadline</span>
          <input
            type="date"
            value={form.deadline}
            onChange={(event) => setForm((prev) => ({ ...prev, deadline: event.target.value }))}
          />
          <small className="muted">must be at least 48 hours away</small>
          {errors.workDeadline && <small className="field-error">{errors.workDeadline}</small>}
        </label>

        {mapped && (
          <div className="notice notice-error field-wide">
            <strong>
              {mapped.code ? `${mapped.code}: ` : ''}
              {mapped.message}
            </strong>
            {mapped.action && <span>{mapped.action}</span>}
          </div>
        )}

        <div className="form-foot field-wide">
          <button className="btn primary" type="submit" disabled={mutation.isPending}>
            {mutation.isPending ? 'Waiting for the wallet…' : 'Fund escrow'}
          </button>
          <span className="muted small">
            one transaction · funds are not released until you approve or an arbiter decides
          </span>
        </div>
      </form>
    </main>
  );
}
