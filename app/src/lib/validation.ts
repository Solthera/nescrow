import { z } from 'zod';
import { parseNearToYocto } from './format';

/**
 * Accepts BOTH implicit (64 hex chars) and named accounts, matching the contract's
 * AccountId rules. A named-only regex would reject a legitimate user before the
 * wallet ever opens (Integration Spec §6).
 */
export const accountIdSchema = z.string().refine(
  (value) => {
    if (/^[0-9a-f]{64}$/.test(value)) return true;
    return /^(([a-z\d]+[-_])*[a-z\d]+\.)*[a-z\d]+[-_]*[a-z\d]+\.(testnet|near)$/.test(value);
  },
  { message: 'Enter a valid NEAR account ID (e.g. alice.testnet).' },
);

/**
 * Values the form cannot know arrive as arguments — never read from the device:
 *
 * - `clientId`: a refine reading a `me` field compares against `undefined`, and
 *   {freelancer, arbiter, undefined} has size 3, so the check would always pass.
 * - `nowNanos`: `Date.now()` breaks the chain-clock rule (I-P6). A device clock two
 *   hours fast lets a 47-hour deadline through, opens the wallet, and the contract
 *   rejects it with E-03 — the user pays for that mistake in gas.
 * - `minEscrow` / `acceptWindow` come from get_config(), not from literals.
 */
export function createProjectSchema(params: {
  clientId: string;
  minEscrowYocto: bigint;
  acceptWindowNs: bigint;
  nowNanos: bigint;
}) {
  return z
    .object({
      freelancer: accountIdSchema,
      arbiter: accountIdSchema,
      title: z.string().min(1, 'A title is required.').max(128, 'Keep the title under 128 characters.'),
      description: z
        .string()
        .min(1, 'A short brief is required.')
        .max(2000, 'Keep the brief under 2000 characters.'),
      amountNear: z.string().refine((value) => {
        try {
          return parseNearToYocto(value) >= params.minEscrowYocto;
        } catch {
          return false;
        }
      }, { message: `Enter at least the contract minimum.` }),
      // String on the wire: the contract takes U64, and a JS number would be lossy.
      workDeadline: z.string().regex(/^\d+$/, 'A deadline is required as a chain timestamp.'),
    })
    .refine((value) => new Set([value.freelancer, value.arbiter, params.clientId]).size === 3, {
      message: 'The client, freelancer and arbiter must be three different accounts.',
      path: ['arbiter'],
    })
    .refine((value) => BigInt(value.workDeadline) >= params.nowNanos + params.acceptWindowNs, {
      message: 'The work deadline must be at least one acceptance window away.',
      path: ['workDeadline'],
    });
}
