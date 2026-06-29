import React, { useState } from 'react';
import { useQuery, useMutation } from '@tanstack/react-query';
import { getAvailableStrategies, runBacktest } from '../api/backtestApi';
import type { BacktestRequest, BacktestResultSlice, StrategyConfig } from '../api/backtestApi';
import { EquityChart } from '../components/analytics/EquityChart';

export const BacktestPage: React.FC = () => {
  const [symbol, setSymbol] = useState('FPT');
  const [startDate, setStartDate] = useState('2020-01-01');
  const [endDate, setEndDate] = useState('2023-12-31');
  const [initialCash, setInitialCash] = useState(100000000);
  const [selectedStrategyFilename, setSelectedStrategyFilename] = useState('');

  const { data: strategies, isLoading: isLoadingStrategies } = useQuery({
    queryKey: ['strategies'],
    queryFn: getAvailableStrategies,
  });
  const selectedStrategy = strategies?.find(s => s.filename === selectedStrategyFilename);
  const selectedStrategyConfig = selectedStrategy?.config as StrategyConfig | undefined;

  const mutation = useMutation({
    mutationFn: runBacktest,
  });

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!strategies) return;
    
    const strategy = strategies.find(s => s.filename === selectedStrategyFilename);
    if (!strategy) {
      alert("Vui lòng chọn một chiến lược (Strategy)!");
      return;
    }

    const symbols = symbol.split(',').map(item => item.trim().toUpperCase()).filter(Boolean);
    const request: BacktestRequest = {
      start_date: startDate,
      end_date: endDate,
      initial_cash: initialCash,
      strategy: strategy.config,
    };
    if (symbols.length > 1) {
      request.symbols = symbols;
    } else {
      request.symbol = symbols[0] || symbol;
    }

    mutation.mutate(request);
  };

  const formatMoney = (val?: number) => val ? val.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 }) : '0.00';
  const formatPercent = (val?: number) => val ? `${(val * 100).toFixed(2)}%` : '0.00%';

  const { data: result, isPending } = mutation;
  const analytics = result?.analytics;
  const sliceRows = analytics ? result?.slices || [] : result?.runs?.flatMap(run => run.slices || []) || [];

  const renderSliceTable = (title: string, rows: BacktestResultSlice[]) => (
    <div className="glass-panel" style={{ padding: '20px' }}>
      <h3 style={{ marginTop: 0, marginBottom: '16px', fontSize: '16px' }}>{title}</h3>
      {rows.length > 0 ? (
        <div style={{ overflowX: 'auto' }}>
          <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '13px' }}>
            <thead>
              <tr style={{ borderBottom: '1px solid rgba(255,255,255,0.1)', color: 'var(--text-muted)', textAlign: 'left' }}>
                <th style={{ padding: '8px 0', fontWeight: 500 }}>Group</th>
                <th style={{ padding: '8px 0', fontWeight: 500, textAlign: 'right' }}>Trades</th>
                <th style={{ padding: '8px 0', fontWeight: 500, textAlign: 'right' }}>Win %</th>
                <th style={{ padding: '8px 0', fontWeight: 500, textAlign: 'right' }}>Avg</th>
                <th style={{ padding: '8px 0', fontWeight: 500, textAlign: 'right' }}>Net</th>
              </tr>
            </thead>
            <tbody>
              {rows.map((row, index) => (
                <tr key={`${row.group_type}-${row.key}-${index}`} style={{ borderBottom: '1px solid rgba(255,255,255,0.05)' }}>
                  <td style={{ padding: '12px 0', fontWeight: 600 }}>{row.key}</td>
                  <td style={{ padding: '12px 0', textAlign: 'right' }}>{row.trades}</td>
                  <td style={{ padding: '12px 0', textAlign: 'right', color: row.win_rate > 0.5 ? 'var(--color-buy)' : 'var(--color-sell)' }}>{formatPercent(row.win_rate)}</td>
                  <td style={{ padding: '12px 0', textAlign: 'right', color: row.average_pnl >= 0 ? 'var(--color-buy)' : 'var(--color-sell)' }}>{formatMoney(row.average_pnl)}</td>
                  <td style={{ padding: '12px 0', textAlign: 'right', fontWeight: 600, color: row.net_pnl >= 0 ? 'var(--color-buy)' : 'var(--color-sell)' }}>{formatMoney(row.net_pnl)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      ) : (
        <p style={{ color: 'var(--text-muted)', fontSize: '14px', margin: 0 }}>No slice data available.</p>
      )}
    </div>
  );

  return (
    <div className="animate-fade-in" style={{ maxWidth: '1200px', margin: '0 auto', paddingBottom: '40px' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '2rem' }}>
        <h2 style={{ margin: 0, fontSize: '28px', background: 'linear-gradient(90deg, #fff, #8B949E)', WebkitBackgroundClip: 'text', WebkitTextFillColor: 'transparent' }}>
          Backtest Engine
        </h2>
      </div>

      <div className="glass-panel" style={{ padding: '24px', marginBottom: '32px' }}>
        <form onSubmit={handleSubmit} style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))', gap: '20px', alignItems: 'end' }}>
          <div>
            <label htmlFor="symbol" style={{ display: 'block', marginBottom: '8px', color: 'var(--text-muted)', fontSize: '14px' }}>Symbol(s)</label>
            <input 
              id="symbol"
              type="text" 
              value={symbol} 
              onChange={e => setSymbol(e.target.value.toUpperCase())} 
              style={{ width: '100%', padding: '10px' }}
              required
            />
          </div>
          <div>
            <label htmlFor="start_date" style={{ display: 'block', marginBottom: '8px', color: 'var(--text-muted)', fontSize: '14px' }}>Start Date</label>
            <input 
              id="start_date"
              type="date" 
              value={startDate} 
              onChange={e => setStartDate(e.target.value)} 
              style={{ width: '100%', padding: '10px' }}
              required
            />
          </div>
          <div>
            <label htmlFor="end_date" style={{ display: 'block', marginBottom: '8px', color: 'var(--text-muted)', fontSize: '14px' }}>End Date</label>
            <input 
              id="end_date"
              type="date" 
              value={endDate} 
              onChange={e => setEndDate(e.target.value)} 
              style={{ width: '100%', padding: '10px' }}
              required
            />
          </div>
          <div>
            <label htmlFor="initial_cash" style={{ display: 'block', marginBottom: '8px', color: 'var(--text-muted)', fontSize: '14px' }}>Initial Cash (VND)</label>
            <input 
              id="initial_cash"
              type="number" 
              value={initialCash} 
              onChange={e => setInitialCash(Number(e.target.value))} 
              style={{ width: '100%', padding: '10px' }}
              required
            />
          </div>
          <div>
            <label htmlFor="strategy" style={{ display: 'block', marginBottom: '8px', color: 'var(--text-muted)', fontSize: '14px' }}>Strategy</label>
            <select 
              id="strategy"
              value={selectedStrategyFilename}
              onChange={e => setSelectedStrategyFilename(e.target.value)}
              style={{ width: '100%', padding: '10px' }}
              required
            >
              <option value="" disabled>Select a strategy</option>
              {strategies?.map(s => (
                <option key={s.filename} value={s.filename}>{s.name} ({s.filename})</option>
              ))}
            </select>
          </div>
          <div>
            <button type="submit" className="btn-primary" style={{ width: '100%', height: '42px', display: 'flex', alignItems: 'center', justifyContent: 'center' }} disabled={isPending || isLoadingStrategies}>
              {isPending ? 'Running...' : 'Run Backtest'}
            </button>
          </div>
        </form>
      </div>

      {selectedStrategy && (
        <div className="glass-panel" style={{ padding: '18px', marginBottom: '24px', borderColor: 'rgba(41, 98, 255, 0.35)' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', gap: '16px', alignItems: 'flex-start' }}>
            <div>
              <h3 style={{ margin: '0 0 8px 0', fontSize: '16px' }}>{selectedStrategy.name}</h3>
              <p style={{ margin: 0, color: 'var(--text-muted)', fontSize: '13px', whiteSpace: 'pre-line' }}>{selectedStrategy.description || 'No description available.'}</p>
            </div>
            <div style={{ display: 'flex', gap: '8px', flexShrink: 0 }}>
              <span style={{ padding: '4px 8px', background: 'rgba(255,255,255,0.05)', borderRadius: '4px', color: 'var(--text-muted)', fontSize: '12px' }}>
                {(selectedStrategyConfig?.indicators || []).length} indicators
              </span>
              <span style={{ padding: '4px 8px', background: 'rgba(255,255,255,0.05)', borderRadius: '4px', color: 'var(--text-muted)', fontSize: '12px' }}>
                {(selectedStrategyConfig?.entry_rules || []).length} entry rules
              </span>
            </div>
          </div>
        </div>
      )}

      {result && !analytics && (
        result.summary ? (
          <div className="animate-slide-up">
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(180px, 1fr))', gap: '16px', marginBottom: '24px' }}>
              <div className="glass-panel" style={{ padding: '20px' }}>
                <h4 style={{ color: 'var(--text-muted)', margin: '0 0 8px 0', fontSize: '13px', textTransform: 'uppercase', letterSpacing: '1px' }}>Net PnL</h4>
                <p style={{ margin: 0, fontSize: '28px', fontWeight: 'bold', color: result.summary.total_net_pnl >= 0 ? 'var(--color-buy)' : 'var(--color-sell)' }}>{formatMoney(result.summary.total_net_pnl)}</p>
              </div>
              <div className="glass-panel" style={{ padding: '20px' }}>
                <h4 style={{ color: 'var(--text-muted)', margin: '0 0 8px 0', fontSize: '13px', textTransform: 'uppercase', letterSpacing: '1px' }}>Win Rate</h4>
                <p style={{ margin: 0, fontSize: '28px', fontWeight: 'bold' }}>{formatPercent(result.summary.win_rate)}</p>
              </div>
              <div className="glass-panel" style={{ padding: '20px' }}>
                <h4 style={{ color: 'var(--text-muted)', margin: '0 0 8px 0', fontSize: '13px', textTransform: 'uppercase', letterSpacing: '1px' }}>Symbols</h4>
                <p style={{ margin: 0, fontSize: '28px', fontWeight: 'bold' }}>{result.summary.succeeded_symbols}/{result.summary.total_symbols}</p>
              </div>
              <div className="glass-panel" style={{ padding: '20px' }}>
                <h4 style={{ color: 'var(--text-muted)', margin: '0 0 8px 0', fontSize: '13px', textTransform: 'uppercase', letterSpacing: '1px' }}>Trades</h4>
                <p style={{ margin: 0, fontSize: '28px', fontWeight: 'bold' }}>{result.summary.total_trades}</p>
              </div>
            </div>
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(320px, 1fr))', gap: '16px' }}>
              {renderSliceTable('Symbol Slices', sliceRows.filter(row => row.group_type === 'symbol'))}
              {renderSliceTable('Period Slices', sliceRows.filter(row => row.group_type === 'period'))}
              {renderSliceTable('Regime Slices', sliceRows.filter(row => row.group_type === 'regime'))}
            </div>
          </div>
        ) : (
        <div className="glass-panel" style={{ borderColor: 'var(--color-sell)', padding: '16px' }}>
          <p style={{ color: 'var(--color-sell)', margin: 0 }}>
            {result.message || result.error || 'Backtest returned no analytics. (Maybe no trades were executed or no data found).'}
          </p>
        </div>
        )
      )}

      {analytics && (
        <div className="animate-slide-up">
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '24px' }}>
            <h3 style={{ margin: 0 }}>Results (Session #{result.session_id || 'N/A'})</h3>
            <span style={{ color: 'var(--text-muted)', fontSize: '14px' }}>Total Candles: {result.total_candles || 0}</span>
          </div>

          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(180px, 1fr))', gap: '16px', marginBottom: '24px' }}>
            <div className="glass-panel" style={{ padding: '20px' }}>
              <h4 style={{ color: 'var(--text-muted)', margin: '0 0 8px 0', fontSize: '13px', textTransform: 'uppercase', letterSpacing: '1px' }}>Net PnL</h4>
              <p style={{ margin: 0, fontSize: '28px', fontWeight: 'bold', color: analytics.total_net_pnl >= 0 ? 'var(--color-buy)' : 'var(--color-sell)' }}>
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
              <h4 style={{ color: 'var(--text-muted)', margin: '0 0 8px 0', fontSize: '13px', textTransform: 'uppercase', letterSpacing: '1px' }}>Max Drawdown</h4>
              <p style={{ margin: 0, fontSize: '28px', fontWeight: 'bold', color: 'var(--color-sell)' }}>
                {formatMoney(analytics.max_drawdown)}
              </p>
            </div>
          </div>

          <div className="glass-panel" style={{ padding: '20px', marginBottom: '24px' }}>
            <h3 style={{ marginTop: 0, marginBottom: '16px', fontSize: '16px' }}>Equity Curve</h3>
            {analytics.equity_curve && analytics.equity_curve.length > 0 ? (
              <EquityChart data={analytics.equity_curve} />
            ) : (
              <div style={{ height: '300px', display: 'flex', alignItems: 'center', justifyContent: 'center', color: 'var(--text-muted)' }}>
                Not enough data to draw equity curve.
              </div>
            )}
          </div>

          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))', gap: '16px' }}>
            <div className="glass-panel" style={{ padding: '16px' }}>
              <h4 style={{ color: 'var(--text-muted)', margin: '0 0 8px 0', fontSize: '13px' }}>Total Trades</h4>
              <p style={{ margin: 0, fontSize: '20px', fontWeight: '600' }}>{analytics.total_trades}</p>
            </div>
            <div className="glass-panel" style={{ padding: '16px' }}>
              <h4 style={{ color: 'var(--text-muted)', margin: '0 0 8px 0', fontSize: '13px' }}>Avg Win / Loss</h4>
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
              <h4 style={{ color: 'var(--text-muted)', margin: '0 0 8px 0', fontSize: '13px' }}>SQN</h4>
              <p style={{ margin: 0, fontSize: '20px', fontWeight: '600', color: (analytics.sqn || 0) > 1.6 ? 'var(--color-buy)' : 'var(--text-main)' }}>
                {analytics.sqn?.toFixed(2) || 'N/A'}
              </p>
            </div>
          </div>

          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(320px, 1fr))', gap: '16px', marginTop: '24px' }}>
            {renderSliceTable('Symbol Slices', sliceRows.filter(row => row.group_type === 'symbol'))}
            {renderSliceTable('Period Slices', sliceRows.filter(row => row.group_type === 'period'))}
            {renderSliceTable('Regime Slices', sliceRows.filter(row => row.group_type === 'regime'))}
          </div>
        </div>
      )}
    </div>
  );
};
