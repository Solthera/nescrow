/**
 * Every failure the contract produces carries a published code inside its panic
 * text, e.g.
 *
 *     Smart contract panicked: E-18: nothing to withdraw
 *
 * The wrapper text is not part of the interface: extract the code with a regex
 * and map it. Anything without a code (wallet rejection, RPC timeout, a bare
 * wasm trap) falls through to the generic row (Integration Spec §9).
 */
const CATALOGUE: Record<string, { message: string; action?: string }> = {
  'E-01': { message: 'You are not allowed to do this for this project.', action: 'Connect with the correct account.' },
  'E-02': { message: "This action is not possible in the project's current state.", action: 'Refresh the page.' },
  'E-03': { message: 'The deadline for this action has passed.', action: 'See the project timeline.' },
  'E-04': { message: 'The escrow amount is invalid.', action: 'The amount must be at least the contract minimum.' },
  'E-05': { message: 'The project has already been accepted.', action: 'Refresh.' },
  'E-06': { message: 'You have used both available revisions.', action: 'Open a dispute instead.' },
  'E-07': { message: 'A dispute is already active for this project.' },
  'E-08': { message: 'This project is already finalized.' },
  'E-09': { message: 'The percentage must be between 0 and 100.' },
  'E-10': { message: 'The link or note you submitted is not valid.', action: 'A link is required and must be under 512 characters; notes are capped at 1024.' },
  'E-11': { message: 'The review period has not ended yet.', action: 'Try again after the review deadline.' },
  'E-12': { message: 'The payout transfer failed. Your balance was restored.', action: 'Try withdrawing again.' },
  'E-13': { message: "The arbiter's 7-day decision window has ended.", action: 'Use mutual settlement with the other party.' },
  'E-14': { message: "The contract's storage reserve is insufficient.", action: 'Contact the operator. Your funds are safe.' },
  'E-15': { message: 'New projects are temporarily paused.', action: 'Try again later.' },
  'E-16': { message: 'Settlement requires both parties to agree on the same percentage.', action: "Check the other party's proposal." },
  'E-17': { message: 'This withdrawal is still pending. Wait before clearing it.' },
  'E-18': { message: 'You have nothing to withdraw.' },
  'E-19': { message: 'A withdrawal is already in progress for your account.', action: 'Wait a few seconds.' },
  'E-20': { message: 'A dispute cannot be opened at this stage.', action: 'You can only dispute after the work has been submitted.' },
  'E-21': { message: 'This project does not exist.', action: 'Check the link or return to your dashboard.' },
  'E-22': { message: 'There is no pending withdrawal to clear.' },
};

export interface MappedError {
  code: string | null;
  message: string;
  action?: string;
  raw: string;
}

function collectText(error: unknown): string {
  if (typeof error === 'string') return error;
  if (error instanceof Error) {
    const withCause = error as Error & { cause?: unknown };
    const cause = withCause.cause ? collectText(withCause.cause) : '';
    return `${error.message} ${cause}`.trim();
  }
  if (error && typeof error === 'object') {
    const record = error as Record<string, unknown>;
    const parts: string[] = [];
    for (const key of ['message', 'kind', 'ExecutionError', 'error', 'cause', 'data']) {
      if (key in record) parts.push(collectText(record[key]));
    }
    if (parts.length === 0) {
      try {
        return JSON.stringify(error);
      } catch {
        return String(error);
      }
    }
    return parts.join(' ');
  }
  return String(error);
}

export function mapError(error: unknown): MappedError {
  const raw = collectText(error).replace(/\s+/g, ' ').trim();
  const match = raw.match(/E-\d{2}/);
  const code = match ? match[0] : null;
  const entry = code ? CATALOGUE[code] : undefined;

  if (!entry) {
    return {
      code,
      message: 'Something went wrong while communicating with the contract.',
      // Only a bare wasm trap means the build lost its error reporting (SCTS D-26).
      // Anything else without a code is a normal failure: rejected wallet, timeout,
      // or arguments the contract could not deserialise.
      action: /unreachable|wasm trap/i.test(raw)
        ? 'Every action failing with an unreachable trap means the deployed build was compiled without cfg(near).'
        : undefined,
      raw,
    };
  }
  return { code, message: entry.message, action: entry.action, raw };
}

/** Short log-friendly form: "E-18" or "unmapped". */
export function errorCode(error: unknown): string {
  return mapError(error).code ?? 'unmapped';
}
