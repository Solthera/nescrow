import { useEffect, useState } from 'react';
import type { WalletSelector } from '@near-wallet-selector/core';
import { getModal, getSelector } from '../lib/near/setup';

export interface WalletState {
  selector: WalletSelector | null;
  accountId: string | null;
  connecting: boolean;
  error: string | null;
}

export function useWallet() {
  const [state, setState] = useState<WalletState>({
    selector: null,
    accountId: null,
    connecting: true,
    error: null,
  });

  useEffect(() => {
    let cancelled = false;
    let subscription: { unsubscribe: () => void } | undefined;

    getSelector()
      .then((selector) => {
        if (cancelled) return;
        const sync = () => {
          setState((prev) => ({
            ...prev,
            selector,
            accountId: selector.store.getState().accounts[0]?.accountId ?? null,
            connecting: false,
          }));
        };
        sync();
        subscription = selector.store.observable.subscribe(sync);
      })
      .catch((error: unknown) => {
        if (cancelled) return;
        setState((prev) => ({
          ...prev,
          connecting: false,
          error: error instanceof Error ? error.message : String(error),
        }));
      });

    return () => {
      cancelled = true;
      subscription?.unsubscribe();
    };
  }, []);

  async function connect() {
    const modal = await getModal();
    modal.show();
  }

  async function disconnect() {
    const selector = state.selector ?? (await getSelector());
    const wallet = await selector.wallet();
    await wallet.signOut();
  }

  return { ...state, connect, disconnect };
}
