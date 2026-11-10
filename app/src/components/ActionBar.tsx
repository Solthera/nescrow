import { useState } from 'react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import type { Project } from '../types/contract';
import type { Action } from '../lib/eligibility';
import { actionFields, submitAction, type ActionInput } from '../lib/near/actions';
import { mapError } from '../lib/errors';

interface Props {
  project: Project;
  actions: Action[];
  reason: string;
  /** The counterparty's proposal, used to prefill `settle_mutual`. */
  counterpartyPercent: number | null;
}

export function ActionBar({ project, actions, reason, counterpartyPercent }: Props) {
  const [open, setOpen] = useState<Action | null>(null);
  const [input, setInput] = useState<ActionInput>({});
  const queryClient = useQueryClient();

  const mutation = useMutation({
    mutationFn: (action: Action) => submitAction(action.id, project, input),
    onSuccess: async () => {
      // Transaction success is not UI success: refetch everything (I-P3).
      await queryClient.invalidateQueries({ queryKey: ['project', project.id] });
      await queryClient.invalidateQueries({ queryKey: ['projects'] });
      await queryClient.invalidateQueries({ queryKey: ['claimable'] });
      await queryClient.invalidateQueries({ queryKey: ['invariants'] });
      setOpen(null);
      setInput({});
    },
  });

  function start(action: Action) {
    mutation.reset();
    setOpen(action);
    setInput(
      action.id === 'settle_mutual' && counterpartyPercent !== null
        ? { percent: counterpartyPercent }
        : {},
    );
  }

  if (actions.length === 0) {
    return (
      <section className="actionbar actionbar-quiet">
        <span className="muted">{reason || 'No action available for your account right now.'}</span>
      </section>
    );
  }

  const fields = open ? actionFields(open.id) : [];
  const mapped = mutation.error ? mapError(mutation.error) : null;

  return (
    <section className="actionbar">
      {!open && (
        <div className="actionbar-buttons">
          {actions.map((action) => (
            <button
              key={action.id}
              className={
                action.style === 'primary'
                  ? 'btn'
                  : action.style === 'destructive'
                    ? 'btn btn-danger'
                    : 'btn btn-ghost'
              }
              onClick={() => start(action)}
            >
              {action.label}
            </button>
          ))}
        </div>
      )}

      {open && (
        <div className="action-form">
          <div className="action-form-head">
            <strong>{open.label}</strong>
            <button className="linkish" onClick={() => setOpen(null)} disabled={mutation.isPending}>
              cancel
            </button>
          </div>
          {open.hint && <p className="muted small">{open.hint}</p>}

          {fields.includes('link') && (
            <label className="field">
              <span>Link</span>
              <input
                type="url"
                placeholder="https://…"
                value={input.link ?? ''}
                onChange={(event) => setInput((prev) => ({ ...prev, link: event.target.value }))}
                autoFocus
              />
            </label>
          )}
          {fields.includes('note') && (
            <label className="field">
              <span>Note</span>
              <textarea
                rows={3}
                placeholder="What changed, or what is wrong. Keep it short."
                value={input.note ?? ''}
                onChange={(event) => setInput((prev) => ({ ...prev, note: event.target.value }))}
              />
            </label>
          )}
          {fields.includes('percent') && (
            <label className="field">
              <span>Freelancer share (%)</span>
              <input
                type="number"
                min={0}
                max={100}
                value={input.percent ?? 0}
                onChange={(event) =>
                  setInput((prev) => ({ ...prev, percent: Number(event.target.value) }))
                }
              />
              <small className="muted">{100 - (input.percent ?? 0)}% goes to the client.</small>
            </label>
          )}

          {mapped && (
            <div className="notice notice-error">
              <strong>
                {mapped.code ? `${mapped.code}: ` : ''}
                {mapped.message}
              </strong>
              {mapped.action && <span>{mapped.action}</span>}
            </div>
          )}

          <div className="action-form-foot">
            <button className="btn" onClick={() => mutation.mutate(open)} disabled={mutation.isPending}>
              {mutation.isPending ? 'Waiting for the wallet…' : 'Sign and send'}
            </button>
            {mutation.isSuccess && <span className="ok small">Confirmed on-chain.</span>}
          </div>
        </div>
      )}
    </section>
  );
}
