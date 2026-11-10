import type { FinalExecutionOutcome } from 'near-api-js';

/**
 * The contract returns method values through `SuccessValue` (base64 of JSON).
 * Used for `create_project`, which returns the new project id (SAS §6).
 */
export function readSuccessValue<T>(outcome: FinalExecutionOutcome): T | null {
  const status = outcome.status as { SuccessValue?: string };
  if (typeof status?.SuccessValue !== 'string') return null;
  try {
    return JSON.parse(atob(status.SuccessValue)) as T;
  } catch {
    return null;
  }
}
