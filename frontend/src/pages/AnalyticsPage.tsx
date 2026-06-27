import React, { useState, useEffect } from 'react';
import { useQuery } from '@tanstack/react-query';
import { getSessionAnalytics } from '../api/analyticsApi';
import { getTrades } from '../api/decisionApi';
import { useReplayStore } from '../store/replayStore';
import type { Trade } from '../types';
import { EquityChart } from '../components/analytics/EquityChart';

export const AnalyticsPage: React.FC = () => {
  const store = useReplayStore();
  const [inputVal, setInputVal] = useState<string>(store.sessionId?.toString() || '1');
  const [sessionId, setSessionId] = useState<number>(store.sessionId || 1);

  // Sync when store.sessionId changes
  useEffect(() => {
    if (store.sessionId) {
      setSessionId(store.sessionId);
      setInputVal(store.sessionId.toString());
    }
  }, [store.sessionId]);

  const { data: analytics, isLoading, isError } = useQuery({
    queryKey: ['analytics', sessionId],
    queryFn: () => getSessionAnalytics(sessionId),
    enabled: !!sessionId,
  });

  const { data: trades } = useQuery({
    queryKey: ['trades', sessionId],
    queryFn: () => getTrades(sessionId),
    enabled: !!sessionId,
  });

  const handleSearch = () => {
    const id = parseInt(inputVal, 10);
    if (!isNaN(id)) {
      setSessionId(id);
    }
  };

  const formatMoney = (val?: number) => val ? val.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 }) : '0.00';
  const formatPercent = (val?: number) => val ? `${(val * 100).toFixed(2)}%` : '0.00%';

  return (
    <div className="animate-fade-in" style={{ maxWidth: '1200px', margin: '0 auto' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '2rem' }}>
        <h2 style={{ margin: 0, fontSize: '28px', background: 'linear-gradient(90deg, #fff, #8B949E)', WebkitBackgroundClip: 'text', WebkitTextFillColor: 'transparent' }}>
          Performance Analytics
        </h2>
        <div style={{ display: 'flex', gap: '0.5rem' }}>
          <input 
            type="number" 
            value={inputVal} 
            onChange={(e) => setInputVal(e.target.value)} 
            placeholder="Session ID"
            style={{ width: '120px' }}
          />
          <button className="btn-primary" onClick={handleSearch}>
            Load Session
          </button>
        </div>
      </div>

      {isLoading && <div style={{ textAlign: 'center', padding: '2rem', color: 'var(--text-muted)' }}>Loading analytics...</div>}
      {isError && <div className="glass-panel" style={{ borderColor: 'var(--color-sell)', padding: '16px' }}><p style={{ color: 'var(--color-sell)', margin: 0 }}>Error loading analytics. Make sure the session exists and has closed trades.</p></div>}

      {analytics && (
        <>
          {/* Main KPI Grid */}
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(180px, 1fr))', gap: '16px', marginBottom: '24px' }}>
            <div className="glass-panel" style={{ padding: '20px' }}>
              <h4 style={{ color: 'var(--text-muted)', margin: '0 0 8px 0', fontSize: '13px', textTransform: 'uppercase', letterSpacing: '1px' }}>Net PnL</h4>
              <p style={{ margin: 0, fontSize: '28px', fontWeight: 'bold', color: analytics.total_net_pnl >= 0 ? 'var(--color-buy)' : 'var(--color-sell)', textShadow: analytics.total_net_pnl >= 0 ? 'var(--shadow-glow)' : 'none' }}>
                {formatMoney(analytics.total_net_pnl)}
              </p>
            </div>
            <div className="glass-panel" style={{ padding: '20px' }}>
              <h4 style={{ color: 'var(--text-muted)', margin: '0 0 8px 0', fontSize: '13px', textTransform: 'uppercase', letterSpacing: '1px' }}>Win Rate</h4>
              <p style={{ margin: 0, fontSize: '28px', fontWeight: 'bold', color: analytics.win_rate > 0.5 ? 'var(--color-buy)' : 'var(--color-sell)' }}>
                {formatPercent(analytics.win_rate)}
              </p>
            </div>
            <div className="glass-panel" style={{ padding: '20px' }}>
              <h4 style={{ color: 'var(--text-muted)', margin: '0 0 8px 0', fontSize: '13px', textTransform: 'uppercase', letterSpacing: '1px' }}>Sharpe Ratio</h4>
              <p style={{ margin: 0, fontSize: '28px', fontWeight: 'bold', color: (analytics.sharpe_ratio || 0) > 1 ? 'var(--color-primary)' : 'var(--text-main)' }}>
                {analytics.sharpe_ratio?.toFixed(2) || 'N/A'}
              </p>
            </div>
            <div className="glass-panel" style={{ padding: '20px' }}>
              <h4 style={{ color: 'var(--text-muted)', margin: '0 0 8px 0', fontSize: '13px', textTransform: 'uppercase', letterSpacing: '1px' }}>SQN</h4>
              <p style={{ margin: 0, fontSize: '28px', fontWeight: 'bold', color: (analytics.sqn || 0) > 1.6 ? 'var(--color-buy)' : 'var(--text-main)' }}>
                {analytics.sqn?.toFixed(2) || 'N/A'}
              </p>
            </div>
            <div className="glass-panel" style={{ padding: '20px' }}>
              <h4 style={{ color: 'var(--text-muted)', margin: '0 0 8px 0', fontSize: '13px', textTransform: 'uppercase', letterSpacing: '1px' }}>Max Drawdown</h4>
              <p style={{ margin: 0, fontSize: '28px', fontWeight: 'bold', color: 'var(--color-sell)' }}>
                {formatMoney(analytics.max_drawdown)}
              </p>
            </div>
          </div>

          {/* Equity Curve */}
          <div className="glass-panel" style={{ padding: '20px', marginBottom: '24px' }}>
            <h3 style={{ marginTop: 0, marginBottom: '16px', fontSize: '16px' }}>Equity Curve & Drawdown</h3>
            {analytics.equity_curve && analytics.equity_curve.length > 0 ? (
              <EquityChart data={analytics.equity_curve} />
            ) : (
              <div style={{ height: '300px', display: 'flex', alignItems: 'center', justifyContent: 'center', color: 'var(--text-muted)' }}>
                Not enough data to draw equity curve.
              </div>
            )}
          </div>

          {/* Secondary Stats */}
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))', gap: '16px', marginBottom: '24px' }}>
            <div className="glass-panel" style={{ padding: '16px' }}>
              <h4 style={{ color: 'var(--text-muted)', margin: '0 0 8px 0', fontSize: '13px' }}>Total Trades</h4>
              <p style={{ margin: 0, fontSize: '20px', fontWeight: '600' }}>{analytics.total_trades}</p>
            </div>
            <div className="glass-panel" style={{ padding: '16px' }}>
              <h4 style={{ color: 'var(--text-muted)', margin: '0 0 8px 0', fontSize: '13px' }}>Avg Win / Avg Loss</h4>
              <p style={{ margin: 0, fontSize: '18px', fontWeight: '600' }}>
                <span style={{ color: 'var(--color-buy)' }}>{formatMoney(analytics.average_win)}</span>
                <span style={{ color: 'var(--text-muted)' }}> / </span>
                <span style={{ color: 'var(--color-sell)' }}>{formatMoney(analytics.average_loss)}</span>
              </p>
            </div>
            <div className="glass-panel" style={{ padding: '16px' }}>
              <h4 style={{ color: 'var(--text-muted)', margin: '0 0 8px 0', fontSize: '13px' }}>Profit Factor</h4>
              <p style={{ margin: 0, fontSize: '20px', fontWeight: '600' }}>
                {analytics.profit_factor === null ? 'N/A' : (analytics.profit_factor === Infinity ? 'INF' : analytics.profit_factor.toFixed(2))}
              </p>
            </div>
            <div className="glass-panel" style={{ padding: '16px' }}>
              <h4 style={{ color: 'var(--text-muted)', margin: '0 0 8px 0', fontSize: '13px' }}>Sortino Ratio</h4>
              <p style={{ margin: 0, fontSize: '20px', fontWeight: '600', color: (analytics.sortino_ratio || 0) > 1 ? 'var(--color-primary)' : 'var(--text-main)' }}>
                {analytics.sortino_ratio?.toFixed(2) || 'N/A'}
              </p>
            </div>
          </div>

          <div style={{ display: 'grid', gridTemplateColumns: '1fr 2fr', gap: '24px' }}>
            {/* Setup Performance */}
            <div className="glass-panel" style={{ padding: '20px' }}>
              <h3 style={{ marginTop: 0, marginBottom: '16px', fontSize: '16px' }}>Setup Performance</h3>
              {analytics.setup_performance && analytics.setup_performance.length > 0 ? (
                <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '14px' }}>
                  <thead>
                    <tr style={{ borderBottom: '1px solid rgba(255,255,255,0.1)', textAlign: 'left', color: 'var(--text-muted)' }}>
                      <th style={{ padding: '8px 0', fontWeight: '500' }}>Setup</th>
                      <th style={{ padding: '8px 0', fontWeight: '500' }}>Trades</th>
                      <th style={{ padding: '8px 0', fontWeight: '500' }}>Win %</th>
                      <th style={{ padding: '8px 0', fontWeight: '500', textAlign: 'right' }}>PnL</th>
                    </tr>
                  </thead>
                  <tbody>
                    {analytics.setup_performance.map((setup, idx) => (
                      <tr key={idx} style={{ borderBottom: '1px solid rgba(255,255,255,0.05)' }}>
                        <td style={{ padding: '12px 0', fontWeight: '600' }}>{setup.setup_type}</td>
                        <td style={{ padding: '12px 0' }}>{setup.trades}</td>
                        <td style={{ padding: '12px 0', color: setup.win_rate > 0.5 ? 'var(--color-buy)' : 'var(--color-sell)' }}>
                          {formatPercent(setup.win_rate)}
                        </td>
                        <td style={{ padding: '12px 0', textAlign: 'right', fontWeight: '600', color: setup.net_pnl >= 0 ? 'var(--color-buy)' : 'var(--color-sell)' }}>
                          {formatMoney(setup.net_pnl)}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              ) : (
                <p style={{ color: 'var(--text-muted)', fontSize: '14px' }}>No setup data available.</p>
              )}
            </div>

            {/* Trade List */}
            <div className="glass-panel" style={{ padding: '20px' }}>
              <h3 style={{ marginTop: 0, marginBottom: '16px', fontSize: '16px' }}>Trade History</h3>
              {trades && trades.length > 0 ? (
                <div style={{ overflowX: 'auto' }}>
                  <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '13px' }}>
                    <thead>
                      <tr style={{ borderBottom: '1px solid rgba(255,255,255,0.1)', textAlign: 'left', color: 'var(--text-muted)' }}>
                        <th style={{ padding: '8px', fontWeight: '500' }}>Symbol</th>
                        <th style={{ padding: '8px', fontWeight: '500' }}>Entry Date</th>
                        <th style={{ padding: '8px', fontWeight: '500' }}>Exit Date</th>
                        <th style={{ padding: '8px', fontWeight: '500', textAlign: 'right' }}>Qty</th>
                        <th style={{ padding: '8px', fontWeight: '500', textAlign: 'right' }}>Entry</th>
                        <th style={{ padding: '8px', fontWeight: '500', textAlign: 'right' }}>Exit</th>
                        <th style={{ padding: '8px', fontWeight: '500', textAlign: 'center' }}>Result</th>
                        <th style={{ padding: '8px', fontWeight: '500', textAlign: 'right' }}>Net PnL</th>
                      </tr>
                    </thead>
                    <tbody>
                      {trades.filter(t => t.status === 'closed').map((t: Trade) => (
                        <tr key={t.id} style={{ borderBottom: '1px solid rgba(255,255,255,0.05)', transition: 'background 0.2s', cursor: 'pointer' }} className="table-row-hover">
                          <td style={{ padding: '12px 8px', fontWeight: '600' }}>{t.symbol}</td>
                          <td style={{ padding: '12px 8px', color: 'var(--text-muted)' }}>{new Date(t.entry_date).toLocaleDateString()}</td>
                          <td style={{ padding: '12px 8px', color: 'var(--text-muted)' }}>{t.exit_date ? new Date(t.exit_date).toLocaleDateString() : '—'}</td>
                          <td style={{ padding: '12px 8px', textAlign: 'right' }}>{t.quantity}</td>
                          <td style={{ padding: '12px 8px', textAlign: 'right', fontFamily: 'monospace' }}>{t.entry_price.toLocaleString()}</td>
                          <td style={{ padding: '12px 8px', textAlign: 'right', fontFamily: 'monospace' }}>{t.exit_price ? t.exit_price.toLocaleString() : '—'}</td>
                          <td style={{ padding: '12px 8px', textAlign: 'center' }}>
                            <span style={{ 
                              padding: '4px 10px', borderRadius: '6px', fontSize: '11px', fontWeight: 'bold', textTransform: 'uppercase',
                              background: t.result === 'win' ? 'rgba(0, 230, 118, 0.15)' : (t.result === 'loss' ? 'rgba(255, 23, 68, 0.15)' : 'rgba(255, 255, 255, 0.1)'),
                              color: t.result === 'win' ? 'var(--color-buy)' : (t.result === 'loss' ? 'var(--color-sell)' : 'var(--text-muted)'),
                              border: `1px solid ${t.result === 'win' ? 'rgba(0, 230, 118, 0.3)' : (t.result === 'loss' ? 'rgba(255, 23, 68, 0.3)' : 'rgba(255, 255, 255, 0.2)')}`
                            }}>
                              {t.result}
                            </span>
                          </td>
                          <td style={{ padding: '12px 8px', textAlign: 'right', fontWeight: 'bold', color: (t.net_pnl || 0) >= 0 ? 'var(--color-buy)' : 'var(--color-sell)' }}>
                            {formatMoney(t.net_pnl)}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              ) : (
                <p style={{ color: 'var(--text-muted)', fontSize: '14px' }}>No closed trades available.</p>
              )}
            </div>
          </div>
        </>
      )}
    </div>
  );
};
