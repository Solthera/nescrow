/**
 * All configuration comes from VITE_ variables. None of these are secrets:
 * a testnet contract id and a public RPC URL are visible in the bundle by design
 * (Integration Spec §3.1, criterion 10).
 */
export const ENV = {
  networkId: (import.meta.env.VITE_NETWORK_ID ?? 'testnet') as 'testnet',
  contractId: import.meta.env.VITE_CONTRACT_ID as string,
  rpcUrl: import.meta.env.VITE_RPC_URL as string,
  explorerUrl: (import.meta.env.VITE_EXPLORER_URL ?? 'https://testnet.nearblocks.io') as string,
  aiEndpoint: import.meta.env.VITE_AI_ENDPOINT as string | undefined,
} as const;

// Hard guard: this app has no mainnet path (SAS S-4).
if (ENV.networkId !== 'testnet') {
  throw new Error('This application only supports NEAR testnet.');
}
if (!ENV.contractId) {
  throw new Error('VITE_CONTRACT_ID is required.');
}
if (!ENV.rpcUrl) {
  throw new Error('VITE_RPC_URL is required.');
}

export const HELPER_URL = 'https://helper.testnet.near.org';
export const INDEXER_URL = 'https://testnet.neardata.xyz';
