import { ENV } from '../config/env';
import { shortAddress } from '../lib/format';
import { useWallet } from '../hooks/useWallet';

export function Header() {
  const { accountId, connecting, connect, disconnect } = useWallet();

  return (
    <header className="header">
      <div className="brand">
        <span className="brand-mark">N</span>
        <strong>nescrow</strong>
      </div>

      <div className="header-right">
        <span className="chip quiet">{ENV.networkId}</span>
        <a
          className="cid"
          href={`${ENV.explorerUrl}/address/${ENV.contractId}`}
          target="_blank"
          rel="noreferrer"
        >
          {shortAddress(ENV.contractId, 12)}
        </a>
        {accountId ? (
          <button className="btn ghost" onClick={disconnect} title="Disconnect">
            {shortAddress(accountId)}
          </button>
        ) : (
          <button className="btn primary" onClick={connect} disabled={connecting}>
            {connecting ? 'Loading…' : 'Connect wallet'}
          </button>
        )}
      </div>
    </header>
  );
}
