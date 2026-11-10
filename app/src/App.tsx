import { BrowserRouter, Route, Routes } from 'react-router-dom';
import { Header } from './components/Header';
import { Dashboard } from './pages/Dashboard';
import { ProjectDetail } from './pages/ProjectDetail';

export default function App() {
  return (
    <BrowserRouter>
      <div className="shell">
        <Header />
        <Routes>
          <Route path="/" element={<Dashboard />} />
          <Route path="/p/:id" element={<ProjectDetail />} />
        </Routes>
        <footer className="footer">
          <span>nescrow · NEAR testnet · no admin key can move escrow</span>
        </footer>
      </div>
    </BrowserRouter>
  );
}
