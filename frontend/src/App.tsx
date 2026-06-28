import { BrowserRouter, Routes, Route } from 'react-router-dom';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { ImportPage } from './pages/ImportPage';
import { ReplayPage } from './pages/ReplayPage';
import { AnalyticsPage } from './pages/AnalyticsPage';
import { BacktestPage } from './pages/BacktestPage';
import { JournalPage } from './pages/JournalPage';
import { Toaster } from 'react-hot-toast';
import { Sidebar } from './components/layout/Sidebar';

const queryClient = new QueryClient();

function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        <div style={{ display: 'flex', height: '100vh', width: '100vw', overflow: 'hidden' }}>
          <Sidebar />
          <main style={{ flex: 1, marginLeft: '292px', padding: '24px', overflowY: 'auto' }}>
            <Routes>
              <Route path="/" element={<div className="glass-panel" style={{padding: '2rem'}}><h2>Welcome to Sumi Replay Lab</h2><p>Select an option from the menu.</p></div>} />
              <Route path="/import" element={<ImportPage />} />
              <Route path="/replay" element={<ReplayPage />} />
              <Route path="/backtest" element={<BacktestPage />} />
              <Route path="/journal" element={<JournalPage />} />
              <Route path="/analytics" element={<AnalyticsPage />} />
            </Routes>
          </main>
        </div>
      </BrowserRouter>
      <Toaster position="bottom-right" toastOptions={{
        style: {
          background: 'var(--bg-panel)',
          color: 'var(--text-main)',
          border: '1px solid var(--border-color)',
          backdropFilter: 'var(--backdrop-blur)',
        }
      }} />
    </QueryClientProvider>
  );
}

export default App;
