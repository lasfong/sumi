import { lazy, Suspense } from 'react';
import { BrowserRouter, Routes, Route } from 'react-router-dom';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { Toaster } from 'react-hot-toast';
import { Sidebar } from './components/layout/Sidebar';

const queryClient = new QueryClient();
const ImportPage = lazy(() => import('./pages/ImportPage').then(module => ({ default: module.ImportPage })));
const ReplayPage = lazy(() => import('./pages/ReplayPage').then(module => ({ default: module.ReplayPage })));
const AnalyticsPage = lazy(() => import('./pages/AnalyticsPage').then(module => ({ default: module.AnalyticsPage })));
const BacktestPage = lazy(() => import('./pages/BacktestPage').then(module => ({ default: module.BacktestPage })));
const ScannerPage = lazy(() => import('./pages/ScannerPage').then(module => ({ default: module.ScannerPage })));
const StrategyLabPage = lazy(() => import('./pages/StrategyLabPage').then(module => ({ default: module.StrategyLabPage })));
const JournalPage = lazy(() => import('./pages/JournalPage').then(module => ({ default: module.JournalPage })));

const PageFallback = () => (
  <div className="glass-panel" style={{ padding: '2rem', color: 'var(--text-muted)' }}>
    Loading...
  </div>
);

function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        <div style={{ display: 'flex', height: '100vh', width: '100vw', overflow: 'hidden' }}>
          <Sidebar />
          <main style={{ flex: 1, marginLeft: '292px', padding: '24px', overflowY: 'auto' }}>
            <Suspense fallback={<PageFallback />}>
              <Routes>
                <Route path="/" element={<div className="glass-panel" style={{padding: '2rem'}}><h2>Welcome to Sumi Replay Lab</h2><p>Select an option from the menu.</p></div>} />
                <Route path="/import" element={<ImportPage />} />
                <Route path="/replay" element={<ReplayPage />} />
                <Route path="/backtest" element={<BacktestPage />} />
                <Route path="/strategy-lab" element={<StrategyLabPage />} />
                <Route path="/scanner" element={<ScannerPage />} />
                <Route path="/journal" element={<JournalPage />} />
                <Route path="/analytics" element={<AnalyticsPage />} />
              </Routes>
            </Suspense>
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
