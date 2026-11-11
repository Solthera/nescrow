import { useState } from 'react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import type { Project } from '../types/contract';
import type { Action, ActionId } from '../lib/eligibility';
import { actionFields, submitAction, type ActionInput } from '../lib/near/actions';
import { mapError } from '../lib/errors';
import { formatNear, formatNearFixed } from '../lib/format';

interface Props {
  project: Project;
  actions: Action[];
  reason: string;
  /** The counterparty's proposal, used to prefill `settle_mutual`. */
  counterpartyPercent: number | null;
  /** What this account can actually pull; the project amount is not it. */
  claimable: bigint;
}

const CONFIRM: Partial<Record<ActionId, { title: string; body: string }>> = {
  approve_deliverable: {
    title: 'Release the escrow?',
    body: 'The full escrow moves to the freelancer. This cannot be undone — the contract has no admin key.',
  },
  reject_project: {
    title: 'Reject this project?',
    body: 'The escrow returns to the client and the project closes.',
  },
  claim_no_response_refund: {
    title: 'Reclaim the escrow?',
    body: 'The freelancer did not accept in time, so the full amount returns to you.',
  },
  claim_deadline_refund: {
    title: 'Reclaim the escrow?',
    body: 'Nothing was submitted before the deadline, so the full amount returns to you.',
  },
  open_dispute: {
    title: 'Open a dispute?',
    body: 'The funds freeze until the arbiter decides or both sides settle. Neither party can move them alone.',
  },
  arbiter_resolve: {
    title: 'Decide the split?',
    body: 'This is final. The contract records the allocation and both sides withdraw from it.',
  },
  settle_mutual: {
    title: 'Close the dispute at this split?',
    body: 'This is final. Both sides must agree, and the allocation is recorded immediately.',
  },
  release_after_review: {
    title: 'Release to the freelancer?',
    body: 'The review window has closed, so the escrow can be released by anyone.',
  },
  accept_project: {
    title: 'Accept this project?',
    body: 'You are committing to the brief and the deadline.',
  },
  withdraw: {
    title: 'Withdraw to your wallet?',
    body: 'Your allocated balance moves from the contract to your account.',
  },
};

export function ActionDock({ project, actions, reason, counterpartyPercent, claimable }: Props) {
  const [open, setOpen] = useState<Action | null>(null);
  const [input, setInput] = useState<ActionInput>({});
  const queryClient = useQueryClient();

  const mutation = useMutation({
    mutationFn: (action: Action) => submitAction(action.id, project, input),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ['project', project.id] });
      await queryClient.invalidateQueries({ queryKey: ['projects'] });
      await queryClient.invalidateQueries({ queryKey: ['claimable'] });
      await queryClient.invalidateQueries({ queryKey: ['invariants'] });
      setOpen(null);
      setInput({});
    },
  });

  if (actions.length === 0) {
    return (
      <div className="dock">
        <div className="dock-inner">
          <span className="dock-hint">{reason || 'No action is available for your account right now.'}</span>
        </div>
      </div>
    );
  }

  const ordered = [...actions].sort((a, b) => (a.style === 'primary' ? -1 : b.style === 'primary' ? 1 : 0));
  const [first, ...rest] = ordered;
  const confirm = open ? CONFIRM[open.id] : undefined;
  const fields = open ? actionFields(open.id) : [];
  const mapped = mutation.error ? mapError(mutation.error) : null;
  const showSplit = fields.includes('percent');

  function start(action: Action) {
    mutation.reset();
    setOpen(action);
    setInput(action.id === 'settle_mutual' && counterpartyPercent !== null ? { percent: counterpartyPercent } : {});
  }

  return (
    <>
      <div className="dock">
        <div className="dock-inner">
          <span className="dock-hint">{reason || 'Signed by your wallet. The contract pays from escrow, never from an operator.'}</span>
          {rest.map((action) => (
            <button
              key={action.id}
              className={`btn ${action.style === 'destructive' ? 'danger' : 'ghost'}`}
              onClick={() => start(action)}
            >
              {action.label}
            </button>
          ))}
          <button className="btn primary" onClick={() => start(first)}>
            {first.id === 'withdraw' ? `Withdraw ${formatNearFixed(claimable)} NEAR` : first.label}
          </button>
        </div>
      </div>

      {open && (
        <div className="backdrop" onClick={(event) => event.target === event.currentTarget && setOpen(null)}>
          <div className="modal">
            <h3>{confirm?.title ?? open.label}</h3>
            <p>{confirm?.body ?? open.hint ?? 'Sign this action with your wallet.'}</p>

            {fields.includes('percent') && (
              <label className="field">
                <span>Freelancer share (%)</span>
                <input
                  type="number"
                  min={0}
                  max={100}
                  value={input.percent ?? 0}
                  onChange={(event) => setInput((prev) => ({ ...prev, percent: Number(event.target.value) }))}
                />
                <small className="muted">{100 - (input.percent ?? 0)}% goes to the client.</small>
              </label>
            )}

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
                  rows={2}
                  placeholder="What changed, or what is wrong."
                  value={input.note ?? ''}
                  onChange={(event) => setInput((prev) => ({ ...prev, note: event.target.value }))}
                />
              </label>
            )}

            {showSplit && (
              <p className="note">
                freelancer {input.percent ?? 0}% · client {100 - (input.percent ?? 0)}%
              </p>
            )}

            {!fields.length && (
              <>
                <div className="big">{formatNear(project.amount)} NEAR</div>
                <p className="note">one transaction · signed by your wallet · gas ≈ 0.0003 NEAR</p>
              </>
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

            <div className="modal-actions">
              <button className="btn" onClick={() => setOpen(null)} disabled={mutation.isPending}>
                Cancel
              </button>
              <button className="btn primary" onClick={() => mutation.mutate(open)} disabled={mutation.isPending}>
                {mutation.isPending ? 'Waiting for wallet…' : 'Sign and send'}
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
