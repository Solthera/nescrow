import { Link } from 'react-router-dom';
import { ENV } from '../config/env';
import { shortAddress } from '../lib/format';
import { useWallet } from '../hooks/useWallet';

export function Header() {
  const { accountId, connecting, connect, disconnect } = useWallet();

  return (
    <header className="header">
      <div className="brand">
        <span className="brand-mark">◈</span>
        <div>
          <strong>nescrow</strong>
          <span className="brand-sub">escrow for freelancers</span>
        </div>
      </div>

      <div className="header-right">
        <span className="pill pill-quiet">{ENV.networkId}</span>
        <a
          className="pill pill-link"
          href={`${ENV.explorerUrl}/address/${ENV.contractId}`}
          target="_blank"
          rel="noreferrer"
        >
          {shortAddress(ENV.contractId, 10)}
        </a>
        {accountId ? (
          <>
            <Link className="btn" to="/new">
              New escrow
            </Link>
            <button className="btn btn-ghost" onClick={disconnect} title="Disconnect">
              {shortAddress(accountId)}
            </button>
          </>
        ) : (
          <button className="btn" onClick={connect} disabled={connecting}>
            {connecting ? 'Loading…' : 'Connect wallet'}
          </button>
        )}
      </div>
    </header>
  );
}
