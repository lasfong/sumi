import React from 'react';
import { useQuery } from '@tanstack/react-query';
import { useNavigate, Link } from 'react-router-dom';
import { getDataReadiness } from '../api/symbolsApi';
import { listReplaySessions } from '../api/replayApi';
import { listStrategyLabRuns } from '../api/strategyLabApi';
import { formatVietnameseDate, formatVietnameseDateTime, formatVietnameseVolume } from '../utils/formatters';
import {
  Activity,
  Database,
  FlaskConical,
  Search,
  Play,
  ArrowRight,
  RefreshCw,
  AlertCircle,
  Cpu,
} from 'lucide-react';
import './DashboardPage.css';

export const DashboardPage: React.FC = () => {
  const navigate = useNavigate();

  const {
    data: readiness,
    isLoading: isLoadingReadiness,
    isError: isErrorReadiness,
    refetch: refetchReadiness,
  } = useQuery({
    queryKey: ['data-readiness-summary'],
    queryFn: () => getDataReadiness(),
  });

  const {
    data: recentSessions = [],
    isLoading: isLoadingSessions,
    isError: isErrorSessions,
    refetch: refetchSessions,
  } = useQuery({
    queryKey: ['recent-sessions-summary'],
    queryFn: () => listReplaySessions(6),
  });

  const {
    data: recentResearch = [],
    isLoading: isLoadingResearch,
    isError: isErrorResearch,
    refetch: refetchResearch,
  } = useQuery({
    queryKey: ['recent-research-summary'],
    queryFn: () => listStrategyLabRuns(5),
  });

  const isReady = readiness?.status === 'ready';
  const isPartial = readiness?.status === 'partial';

  return (
    <div className="dashboard-page animate-fade-in">
      <header className="dashboard-header">
        <div>
          <h1 className="dashboard-title">Workstation Overview</h1>
          <p className="dashboard-subtitle">
            Local-first technical analysis, manual replay, and quantitative practice station
          </p>
        </div>

        <div className="header-actions">
          <Link to="/replay" className="btn-primary-action">
            <Play size={16} />
            <span>Start Replay Practice</span>
          </Link>
        </div>
      </header>

      {/* Grid Section 1: Market Data Readiness & Quick Nav */}
      <div className="dashboard-grid top-grid">
        {/* Data Readiness Card (PRO-UX-01 / R02-02) */}
        <div className="glass-panel readiness-card">
          <div className="card-header">
            <div className="card-title-group">
              <Database className="card-icon" size={20} />
              <h3>Market Data Readiness</h3>
            </div>
            <span
              className={`status-badge ${
                isErrorReadiness
                  ? 'error'
                  : isLoadingReadiness
                  ? 'loading'
                  : isReady
                  ? 'ready'
                  : isPartial
                  ? 'warning'
                  : 'empty'
              }`}
            >
              {isErrorReadiness
                ? 'Error'
                : isLoadingReadiness
                ? 'Checking...'
                : isReady
                ? 'Data Ready'
                : isPartial
                ? 'Partial Data'
                : 'No Data'}
            </span>
          </div>

          {isLoadingReadiness ? (
            <div className="card-loading">
              <RefreshCw className="animate-spin" size={20} />
              <span>Checking local market data readiness...</span>
            </div>
          ) : isErrorReadiness ? (
            <div className="card-error">
              <AlertCircle size={20} />
              <div>
                <p>Failed to query market data readiness.</p>
                <button type="button" className="btn-inline-retry" onClick={() => refetchReadiness()}>
                  Retry query
                </button>
              </div>
            </div>
          ) : !readiness || readiness.total_candles === 0 ? (
            <div className="card-empty-actionable">
              <p>No market candle data imported in local database.</p>
              <Link to="/import" className="btn-secondary-action">
                <span>Import Market Data</span>
                <ArrowRight size={14} />
              </Link>
            </div>
          ) : (
            <div className="readiness-metrics">
              <div className="metric-box">
                <span className="metric-value">
                  {readiness.symbols_with_candles.length} / {readiness.symbols_count}
                </span>
                <span className="metric-label">Active Symbols</span>
              </div>
              <div className="metric-box">
                <span className="metric-value" data-testid="readiness-candles">{formatVietnameseVolume(readiness.total_candles)}</span>
                <span className="metric-label">Total Candles</span>
              </div>
              <div className="metric-box">
                <span className="metric-value">{readiness.timeframes.join(', ') || 'None'}</span>
                <span className="metric-label">Timeframes</span>
              </div>
            </div>
          )}
        </div>

        {/* Quick Workflow Jump */}
        <div className="glass-panel quick-nav-card">
          <h3>Quick Workstation Navigation</h3>
          <div className="quick-nav-links">
            <Link to="/replay" className="quick-link">
              <Activity className="link-icon" size={18} />
              <div className="link-content">
                <strong>Trading Lab</strong>
                <span>Manual replay & position execution</span>
              </div>
            </Link>
            <Link to="/backtest" className="quick-link">
              <Cpu className="link-icon" size={18} />
              <div className="link-content">
                <strong>Backtest Engine</strong>
                <span>Declarative strategy performance</span>
              </div>
            </Link>
            <Link to="/strategy-lab" className="quick-link">
              <FlaskConical className="link-icon" size={18} />
              <div className="link-content">
                <strong>Strategy Lab</strong>
                <span>Parameter sweeps & comparison</span>
              </div>
            </Link>
            <Link to="/scanner" className="quick-link">
              <Search className="link-icon" size={18} />
              <div className="link-content">
                <strong>Signal Scanner</strong>
                <span>Technical setups & signal search</span>
              </div>
            </Link>
          </div>
        </div>
      </div>

      {/* Grid Section 2: Recent Replay Practice Sessions */}
      <section className="glass-panel section-card">
        <div className="section-header">
          <div className="card-title-group">
            <Activity className="card-icon" size={20} />
            <h2>Recent Practice Sessions</h2>
          </div>
          <Link to="/replay" className="view-all-link">
            <span>Open Replay Workspace</span>
            <ArrowRight size={14} />
          </Link>
        </div>

        {isLoadingSessions ? (
          <div className="card-loading">
            <RefreshCw className="animate-spin" size={20} />
            <span>Loading recent sessions...</span>
          </div>
        ) : isErrorSessions ? (
          <div className="card-error">
            <AlertCircle size={20} />
            <div>
              <p>Error loading recent practice sessions.</p>
              <button type="button" className="btn-inline-retry" onClick={() => refetchSessions()}>
                Retry
              </button>
            </div>
          </div>
        ) : recentSessions.length === 0 ? (
          <div className="card-empty-actionable">
            <p>No replay practice sessions created yet.</p>
            <Link to="/replay" className="btn-secondary-action">
              <Play size={14} />
              <span>Start New Replay Practice</span>
            </Link>
          </div>
        ) : (
          <div className="sessions-grid">
            {recentSessions.map((session) => {
              const modeIntent = session.source_context?.replay_intent;
              const modeText =
                modeIntent === 'signal_review'
                  ? 'Signal Review'
                  : modeIntent === 'blind_practice'
                  ? 'Blind Practice'
                  : 'Practice Session';

              return (
                <div key={session.id} className="session-card-item">
                  <div className="session-item-header">
                    <div>
                      <strong className="session-item-id">#{session.id}</strong>
                      <span className="session-item-symbol">{session.symbol}</span>
                    </div>
                    <span className={`mode-badge ${modeIntent || 'blind_practice'}`}>
                      {modeText}
                    </span>
                  </div>

                  <div className="session-item-body">
                    <div className="info-row">
                      <span className="info-label">Timeframe:</span>
                      <span className="info-val">{session.timeframe || 'Daily'}</span>
                    </div>
                    <div className="info-row">
                      <span className="info-label">Progress:</span>
                      <span className="info-val">Bar #{session.current_index}</span>
                    </div>
                    <div className="info-row">
                      <span className="info-label">Start Date:</span>
                      <span className="info-val">
                        {session.start_date ? formatVietnameseDate(session.start_date) : '—'}
                      </span>
                    </div>
                  </div>

                  <button
                    type="button"
                    className="btn-continue-session"
                    onClick={() => navigate(`/replay?session=${session.id}`)}
                  >
                    <span>Continue Practice</span>
                    <ArrowRight size={14} />
                  </button>
                </div>
              );
            })}
          </div>
        )}
      </section>

      {/* Grid Section 3: Recent Research Runs */}
      <section className="glass-panel section-card">
        <div className="section-header">
          <div className="card-title-group">
            <FlaskConical className="card-icon" size={20} />
            <h2>Recent Strategy Research Runs</h2>
          </div>
          <Link to="/strategy-lab" className="view-all-link">
            <span>Go to Strategy Lab</span>
            <ArrowRight size={14} />
          </Link>
        </div>

        {isLoadingResearch ? (
          <div className="card-loading">
            <RefreshCw className="animate-spin" size={20} />
            <span>Loading strategy lab runs...</span>
          </div>
        ) : isErrorResearch ? (
          <div className="card-error">
            <AlertCircle size={20} />
            <div>
              <p>Error loading recent research runs.</p>
              <button type="button" className="btn-inline-retry" onClick={() => refetchResearch()}>
                Retry
              </button>
            </div>
          </div>
        ) : recentResearch.length === 0 ? (
          <div className="card-empty-actionable">
            <p>No saved strategy research runs found.</p>
            <Link to="/strategy-lab" className="btn-secondary-action">
              <FlaskConical size={14} />
              <span>Run Parameter Sweep or Comparison</span>
            </Link>
          </div>
        ) : (
          <div className="research-list">
            {recentResearch.map((run) => (
              <div key={run.id} className="research-item">
                <div className="research-info">
                  <span className="research-type-tag">{run.run_type}</span>
                  <strong className="research-label">{run.label}</strong>
                  <span className="research-date">
                    {run.created_at ? formatVietnameseDateTime(run.created_at) : '—'}
                  </span>
                </div>
                <button
                  type="button"
                  className="btn-open-research"
                  onClick={() => navigate('/strategy-lab')}
                >
                  <span>Inspect</span>
                  <ArrowRight size={14} />
                </button>
              </div>
            ))}
          </div>
        )}
      </section>
    </div>
  );
};
