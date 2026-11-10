import { JsonRpcProvider } from 'near-api-js';
import type { FinalExecutionOutcome } from 'near-api-js';
import { actionCreators } from '@near-js/transactions';
import { ENV } from '../../config/env';
import { getSelector } from './setup';
import type {
  Config,
  FundInvariants,
  Project,
  ProjectClaim,
  Role,
  Stats,
  U128,
  WithdrawalRecord,
} from '../../types/contract';

/**
 * The single door to the contract (Integration Spec I-P1). No component calls
 * the provider or the wallet directly.
 *
 * Note on the shape: near-api-js v7 dropped the v5 `Contract({ signer: wallet })`
 * convenience, and wallet-selector v10 wallets sign transactions themselves. So
 * reads go straight to the provider and writes go through
 * `wallet.signAndSendTransaction`. Same guarantees, one adapter.
 */

const TGAS = 1_000_000_000_000n;

export const GAS_DEFAULT = 30n * TGAS;
/** Not 30: the surplus pays for the `on_withdraw` callback (SCTS §8.4). */
export const GAS_WITHDRAW = 50n * TGAS;
export const DEPOSIT_ZERO = 0n;

let provider: JsonRpcProvider | null = null;

function getProvider(): JsonRpcProvider {
  if (!provider) {
    provider = new JsonRpcProvider({ url: ENV.rpcUrl });
  }
  return provider;
}

export async function view<T>(method: string, args: Record<string, unknown> = {}): Promise<T> {
  // The provider constrains its generic to `SerializedReturnValue`, which is a
  // looser shape than our DTOs; the cast is the boundary, the types behind it are
  // checked at every call site.
  const result = (await getProvider().callFunction({
    contractId: ENV.contractId,
    method,
    args,
  })) as unknown as T | undefined;
  if (result === undefined || result === null) {
    throw new Error(`contract returned no value for ${method}`);
  }
  return result;
}

export interface CallOptions {
  gas?: bigint;
  deposit?: bigint;
}

export async function call(
  method: string,
  args: Record<string, unknown> = {},
  options: CallOptions = {},
): Promise<FinalExecutionOutcome> {
  const selector = await getSelector();
  if (!selector.isSignedIn()) {
    throw new Error('Connect a wallet first.');
  }
  const wallet = await selector.wallet();
  const outcome = await wallet.signAndSendTransaction({
    receiverId: ENV.contractId,
    // One action only: the deposit *is* the escrow, so a separate Transfer
    // action would send it twice (SAS §5, the bug the spec audit caught).
    actions: [
      actionCreators.functionCall(method, args, options.gas ?? GAS_DEFAULT, options.deposit ?? DEPOSIT_ZERO),
    ],
  });
  if (!outcome) {
    throw new Error('The wallet did not return a transaction outcome.');
  }
  return outcome;
}

// --- views ----------------------------------------------------------------

export const getConfig = () => view<Config>('get_config');
export const getFundInvariants = () => view<FundInvariants>('get_fund_invariants');
export const getProject = (projectId: number) => view<Project>('get_project', { project_id: projectId });
export const getClaimable = (account: string) => view<U128>('get_claimable', { account });
export const getStats = (account: string) => view<Stats>('get_stats', { account });
export const getWithdrawalState = (account: string) =>
  view<WithdrawalRecord | null>('get_withdrawal_state', { account });
export const getProjects = (account: string, role: Role, fromIndex = 0, limit = 20) =>
  view<ProjectClaim[]>('get_projects', { account, role, from_index: fromIndex, limit });

// --- writes ---------------------------------------------------------------

export const withdraw = () => call('withdraw', {}, { gas: GAS_WITHDRAW });
