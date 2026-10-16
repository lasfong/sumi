import { lazy, Suspense, useState } from 'react';
import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { QueryClient, QueryClientProvider, type QueryClientConfig } from '@tanstack/react-query';
import { Toaster } from 'react-hot-toast';
import { Sidebar } from './components/layout/Sidebar';
import { ErrorBoundary } from './components/common/ErrorBoundary';

export const DEFAULT_QUERY_CLIENT_CONFIG: QueryClientConfig = {
  defaultOptions: {
    queries: {
      retry: 1,
      staleTime: 10_000,
      gcTime: 5 * 60 * 1000,
      refetchOnWindowFocus: false,
    },
    mutations: {
      retry: 0,
    },
  },
};

export const createAppQueryClient = () => new QueryClient(DEFAULT_QUERY_CLIENT_CONFIG);

const queryClient = createAppQueryClient();
const DashboardPage = lazy(() => import('./pages/DashboardPage').then(module => ({ default: module.DashboardPage })));
const ImportPage = lazy(() => import('./pages/ImportPage').then(module => ({ default: module.ImportPage })));
const ReplayPage = lazy(() => import('./pages/ReplayPage').then(module => ({ default: module.ReplayPage })));
const AnalyticsPage = lazy(() => import('./pages/AnalyticsPage').then(module => ({ default: module.AnalyticsPage })));
const ScannerPage = lazy(() => import('./pages/ScannerPage').then(module => ({ default: module.ScannerPage })));
const StrategyLabPage = lazy(() => import('./pages/StrategyLabPage').then(module => ({ default: module.StrategyLabPage })));
const JournalPage = lazy(() => import('./pages/JournalPage').then(module => ({ default: module.JournalPage })));

const PageFallback = () => (
  <div className="glass-panel" style={{ padding: '2rem', color: 'var(--text-muted)' }}>
    Loading...
  </div>
);

function App() {
  const [sidebarCollapsed, setSidebarCollapsed] = useState(() => {
    try {
      return localStorage.getItem('sumi-sidebar-collapsed') === 'true';
    } catch {
      return false;
    }
  });

  const handleToggleSidebar = () => {
    setSidebarCollapsed((prev) => {
      const next = !prev;
      try {
        localStorage.setItem('sumi-sidebar-collapsed', String(next));
      } catch {
        // ignore
      }
      return next;
    });
  };

  return (
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        <div className={`app-shell ${sidebarCollapsed ? 'sidebar-collapsed' : ''}`}>
          <Sidebar isCollapsed={sidebarCollapsed} onToggleCollapse={handleToggleSidebar} />
          <main className="app-main">
            <ErrorBoundary>
              <Suspense fallback={<PageFallback />}>
                <Routes>
                  <Route path="/" element={<DashboardPage />} />
                  <Route path="/import" element={<ImportPage />} />
                  <Route path="/replay" element={<ReplayPage />} />
                  <Route path="/backtest" element={<Navigate to="/strategy-lab?tab=single" replace />} />
                  <Route path="/strategy-lab" element={<StrategyLabPage />} />
                  <Route path="/scanner" element={<ScannerPage />} />
                  <Route path="/journal" element={<JournalPage />} />
                  <Route path="/analytics" element={<AnalyticsPage />} />
                </Routes>
              </Suspense>
            </ErrorBoundary>
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
