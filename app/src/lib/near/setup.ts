import { setupWalletSelector, type WalletSelector } from '@near-wallet-selector/core';
import { setupModal, type WalletSelectorModal } from '@near-wallet-selector/modal-ui';
import { setupMyNearWallet } from '@near-wallet-selector/my-near-wallet';
import { setupMeteorWallet } from '@near-wallet-selector/meteor-wallet';
import { setupHereWallet } from '@near-wallet-selector/here-wallet';
import { setupSender } from '@near-wallet-selector/sender';
import { ENV, HELPER_URL, INDEXER_URL } from '../../config/env';

let selectorPromise: Promise<WalletSelector> | null = null;
let modal: WalletSelectorModal | null = null;

/**
 * One selector per page load. Two instances produce two wallet popups and two
 * competing stores (Integration Spec §3.2, rule 1).
 */
export function getSelector(): Promise<WalletSelector> {
  if (!selectorPromise) {
    selectorPromise = setupWalletSelector({
      network: {
        networkId: ENV.networkId,
        nodeUrl: ENV.rpcUrl,
        helperUrl: HELPER_URL,
        explorerUrl: ENV.explorerUrl,
        indexerUrl: INDEXER_URL,
      },
      modules: [
        setupMyNearWallet(),
        setupMeteorWallet(),
        setupHereWallet(),
        setupSender(),
      ],
      // Ask for a function-call key scoped to our contract: enough to transact,
      // not enough to move funds out of the user's wallet.
      createAccessKeyFor: ENV.contractId,
    });
  }
  return selectorPromise;
}

export async function getModal(): Promise<WalletSelectorModal> {
  if (!modal) {
    const selector = await getSelector();
    modal = setupModal(selector, { contractId: ENV.contractId });
  }
  return modal;
}
