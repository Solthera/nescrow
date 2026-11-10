import { Header } from './components/Header';
import { Dashboard } from './pages/Dashboard';

export default function App() {
  return (
    <div className="shell">
      <Header />
      <Dashboard />
      <footer className="footer">
        <span>nescrow · NEAR testnet · no admin key can move escrow</span>
      </footer>
    </div>
  );
}
