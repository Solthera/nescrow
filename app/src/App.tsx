import { BrowserRouter, Route, Routes } from 'react-router-dom';
import { Header } from './components/Header';
import { Landing } from './pages/Landing';
import { Dashboard } from './pages/Dashboard';
import { ProjectDetail } from './pages/ProjectDetail';
import { CreateProject } from './pages/CreateProject';

export default function App() {
  return (
    <BrowserRouter>
      <div className="shell">
        <Header />
        <Routes>
          <Route path="/" element={<Landing />} />
          <Route path="/app" element={<Dashboard />} />
          <Route path="/p/:id" element={<ProjectDetail />} />
          <Route path="/new" element={<CreateProject />} />
        </Routes>
        <footer className="footer">
          <div className="footer-inner">
            nescrow · NEAR testnet · the contract holds the escrow, no admin key can move it
          </div>
        </footer>
      </div>
    </BrowserRouter>
  );
}
