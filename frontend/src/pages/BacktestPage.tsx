import React, { useState, useEffect, useMemo } from 'react';
import { useQuery, useMutation } from '@tanstack/react-query';
import { getAvailableStrategies, runBacktest } from '../api/backtestApi';
import type { AvailableStrategy, BacktestRequest, BacktestResultSlice, BacktestTrade, StrategyConfig } from '../api/backtestApi';
import type { StrategyRulePayload } from '../components/strategy/StrategyRuleBuilder';
import { EquityChart } from '../components/analytics/EquityChart';
import { MetricResultValue } from '../components/analytics/MetricResultValue';
import { formatPercentPoints } from '../utils/formatters';

const CUSTOM_RULE_FILENAME = 'custom_ast_rule.yaml';

function loadPendingRuleFromStorage(): StrategyRulePayload | null {
  if (typeof window === 'undefined') return null;
  try {
    const raw =
      window.sessionStorage?.getItem('sumi_pending_strategy_rule') ||
      window.localStorage?.getItem('sumi_pending_strategy_rule');
    if (!raw) return null;
    const parsed = JSON.parse(raw) as StrategyRulePayload;
    if (parsed && parsed.expression) {
      return parsed;
    }
  } catch {
    // ignore storage parse error
  }
  return null;
}

function buildCustomStrategyFromPayload(payload: StrategyRulePayload): AvailableStrategy {
  const cleanName = payload.ruleName || 'Bo_Dieu_Kien_Mau';
  const signals = payload.signalsRequired || [];
  const config: StrategyConfig = payload.strategyConfig ?? {
    name: cleanName,
    version: '1.0',
    description: `Quy tắc AST tùy chỉnh: ${payload.expression}`,
    indicators: signals.map((alias) => ({ name: alias, type: 'signal' })),
    entry_rules: [{ condition: payload.expression }],
    exit_rules: [],
    position_sizing: { method: 'fixed_quantity', quantity: 1000 },
    risk_management: { stop_loss_pct: 7.0, take_profit_pct: 15.0 },
  };
  return {
    filename: CUSTOM_RULE_FILENAME,
    name: `${cleanName} (Custom AST)`,
    description: `Biểu thức AST: ${payload.expression}`,
    config,
  };
}

export interface BacktestPageProps {
  embedded?: boolean;
}

export const BacktestPage: React.FC<BacktestPageProps> = ({ embedded = false }) => {
  const [symbol, setSymbol] = useState('FPT');
  const [startDate, setStartDate] = useState('2020-01-01');
  const [endDate, setEndDate] = useState('2023-12-31');
  const [initialCash, setInitialCash] = useState(100000000);
  const [pendingRule, setPendingRule] = useState<StrategyRulePayload | null>(() => loadPendingRuleFromStorage());
  const [selectedStrategyFilename, setSelectedStrategyFilename] = useState<string>(() =>
    loadPendingRuleFromStorage() ? CUSTOM_RULE_FILENAME : ''
  );

  useEffect(() => {
    const handleRuleEvent = (e: Event) => {
      const detail = (e as CustomEvent<StrategyRulePayload>).detail || loadPendingRuleFromStorage();
      if (detail && detail.expression) {
        setPendingRule(detail);
        setSelectedStrategyFilename(CUSTOM_RULE_FILENAME);
      }
    };
    window.addEventListener('sumi:run-strategy-rule', handleRuleEvent);
    return () => window.removeEventListener('sumi:run-strategy-rule', handleRuleEvent);
  }, []);

  const clearPendingRule = () => {
    setPendingRule(null);
    if (selectedStrategyFilename === CUSTOM_RULE_FILENAME) {
      setSelectedStrategyFilename('');
    }
    try {
      window.sessionStorage?.removeItem('sumi_pending_strategy_rule');
      window.localStorage?.removeItem('sumi_pending_strategy_rule');
    } catch {
      // ignore
    }
  };

  const { data: strategies, isLoading: isLoadingStrategies } = useQuery({
    queryKey: ['strategies'],
    queryFn: getAvailableStrategies,
  });

  const allStrategies = useMemo<AvailableStrategy[]>(() => {
    const base = strategies ? [...strategies] : [];
    if (pendingRule) {
      return [buildCustomStrategyFromPayload(pendingRule), ...base];
    }
    return base;
  }, [strategies, pendingRule]);

  const effectiveStrategyFilename =
    selectedStrategyFilename ||
    (pendingRule ? CUSTOM_RULE_FILENAME : allStrategies[0]?.filename || '');
  const selectedStrategy = allStrategies.find(s => s.filename === effectiveStrategyFilename);
  const selectedStrategyConfig = selectedStrategy?.config as StrategyConfig | undefined;

  const mutation = useMutation({
    mutationFn: runBacktest,
  });

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (allStrategies.length === 0) return;
    
    const strategy = allStrategies.find(s => s.filename === effectiveStrategyFilename);
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

  const formatMoney = (val?: number | null) =>
    val !== undefined && val !== null
      ? val.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })
      : '0.00';
  const formatPercent = (val?: number | null) =>
    val !== undefined && val !== null ? `${(val * 100).toFixed(2)}%` : '0.00%';
  const formatDate = (val?: string | null) => {
    if (!val) return '—';
    return val.includes('T') ? val.split('T')[0] : val;
  };

  const { data: result, isPending } = mutation;
  const analytics = result?.analytics;
  const sliceRows = analytics ? result?.slices || [] : result?.runs?.flatMap(run => run.slices || []) || [];

  const executedTrades: BacktestTrade[] = useMemo(() => {
    if (result?.trades && result.trades.length > 0) {
      return result.trades;
    }
    if (result?.runs) {
      return result.runs.flatMap(run => run.trades || []);
    }
    return [];
  }, [result]);

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

  const renderExecutedTradesTable = (trades: BacktestTrade[]) => (
    <div className="glass-panel" style={{ padding: '20px', marginTop: '24px' }} data-testid="backtest-executed-trades">
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '16px' }}>
        <h3 style={{ margin: 0, fontSize: '16px' }}>Danh sách Lệnh Đã Thực Hiện ({trades.length} lệnh)</h3>
        <span style={{ color: 'var(--text-muted)', fontSize: '13px' }}>Chi tiết từng giao dịch trong kỳ kiểm thử</span>
      </div>
      {trades.length > 0 ? (
        <div style={{ overflowX: 'auto', maxHeight: '420px', overflowY: 'auto' }}>
          <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '13px' }}>
            <thead>
              <tr style={{ borderBottom: '1px solid rgba(255,255,255,0.1)', color: 'var(--text-muted)', textAlign: 'left', position: 'sticky', top: 0, background: 'var(--bg-panel, #161b22)' }}>
                <th style={{ padding: '8px 6px', fontWeight: 500 }}>#</th>
                <th style={{ padding: '8px 6px', fontWeight: 500 }}>Mã</th>
                <th style={{ padding: '8px 6px', fontWeight: 500 }}>Ngày Vào</th>
                <th style={{ padding: '8px 6px', fontWeight: 500, textAlign: 'right' }}>Giá Vào</th>
                <th style={{ padding: '8px 6px', fontWeight: 500 }}>Ngày Ra</th>
                <th style={{ padding: '8px 6px', fontWeight: 500, textAlign: 'right' }}>Giá Ra</th>
                <th style={{ padding: '8px 6px', fontWeight: 500, textAlign: 'right' }}>Khối Lượng</th>
                <th style={{ padding: '8px 6px', fontWeight: 500, textAlign: 'right' }}>Nắm Giữ</th>
                <th style={{ padding: '8px 6px', fontWeight: 500, textAlign: 'right' }}>Lãi/Lỗ Ròng</th>
                <th style={{ padding: '8px 6px', fontWeight: 500, textAlign: 'right' }}>% PnL</th>
                <th style={{ padding: '8px 6px', fontWeight: 500, textAlign: 'center' }}>Kết Quả</th>
              </tr>
            </thead>
            <tbody>
              {trades.map((trade, idx) => {
                const isWin = (trade.net_pnl ?? 0) > 0;
                const isLoss = (trade.net_pnl ?? 0) < 0;
                const pnlColor = isWin ? 'var(--color-buy)' : isLoss ? 'var(--color-sell)' : 'var(--text-muted)';
                const resultLabel = trade.result ? trade.result.toUpperCase() : (isWin ? 'WIN' : isLoss ? 'LOSS' : 'BREAKEVEN');

                return (
                  <tr
                    key={`${trade.symbol ?? 'sym'}-${trade.id ?? idx}-${idx}`}
                    data-testid={`trade-row-${trade.id || idx + 1}`}
                    style={{ borderBottom: '1px solid rgba(255,255,255,0.05)' }}
                  >
                    <td style={{ padding: '10px 6px', color: 'var(--text-muted)' }}>{trade.id || idx + 1}</td>
                    <td style={{ padding: '10px 6px', fontWeight: 600 }}>{trade.symbol}</td>
                    <td style={{ padding: '10px 6px', color: 'var(--text-muted)' }}>{formatDate(trade.entry_date)}</td>
                    <td style={{ padding: '10px 6px', textAlign: 'right' }}>{formatMoney(trade.entry_price)}</td>
                    <td style={{ padding: '10px 6px', color: 'var(--text-muted)' }}>{formatDate(trade.exit_date)}</td>
                    <td style={{ padding: '10px 6px', textAlign: 'right' }}>{trade.exit_price !== null && trade.exit_price !== undefined ? formatMoney(trade.exit_price) : 'Đang mở'}</td>
                    <td style={{ padding: '10px 6px', textAlign: 'right' }}>{trade.quantity?.toLocaleString() || 0}</td>
                    <td style={{ padding: '10px 6px', textAlign: 'right' }}>{trade.holding_candles ?? 0} nến</td>
                    <td style={{ padding: '10px 6px', textAlign: 'right', fontWeight: 600, color: pnlColor }}>
                      {formatMoney(trade.net_pnl)}
                    </td>
                    <td style={{ padding: '10px 6px', textAlign: 'right', fontWeight: 600, color: pnlColor }}>
                      {formatPercentPoints(trade.pnl_percent)}
                    </td>
                    <td style={{ padding: '10px 6px', textAlign: 'center' }}>
                      <span
                        style={{
                          padding: '2px 8px',
                          borderRadius: '4px',
                          fontSize: '11px',
                          fontWeight: 600,
                          background: isWin ? 'rgba(0, 230, 118, 0.15)' : isLoss ? 'rgba(255, 82, 82, 0.15)' : 'rgba(255, 255, 255, 0.08)',
                          color: pnlColor,
                        }}
                      >
                        {resultLabel}
                      </span>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      ) : (
        <p data-testid="no-executed-trades" style={{ color: 'var(--text-muted)', fontSize: '14px', margin: 0 }}>
          Không có giao dịch nào được thực hiện trong kỳ kiểm thử này.
        </p>
      )}
    </div>
  );

  return (
    <div className="animate-fade-in" style={{ maxWidth: '1200px', margin: '0 auto', paddingBottom: '40px' }} data-testid="single-backtest-surface">
      {!embedded && (
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1.25rem' }}>
          <h2 style={{ margin: 0, fontSize: '28px', background: 'linear-gradient(90deg, #fff, #8B949E)', WebkitBackgroundClip: 'text', WebkitTextFillColor: 'transparent' }}>
            Backtest Engine
          </h2>
        </div>
      )}

      {/* Single vs Battle vs Sweep Explanatory Callout (DEC-06, GAP-U-05) */}
      <div
        data-testid="single-vs-battle-banner"
        className="glass-panel"
        style={{
          padding: '12px 16px',
          marginBottom: '16px',
          borderColor: 'rgba(88, 166, 255, 0.35)',
          background: 'rgba(41, 98, 255, 0.08)',
          fontSize: '12px',
          color: 'var(--text-muted)',
          lineHeight: 1.5,
        }}
      >
        <strong style={{ color: '#58A6FF' }}>🎯 Single Backtest vs. ⚔️ Strategy Battle:</strong>{' '}
        Chế độ <strong>Single Backtest</strong> phân tích chuyên sâu 1 chiến lược đơn lẻ (đường cong vốn chi tiết, lát cắt theo Mã/Chu kỳ/Regime &amp; Run Manifest). Dùng tab <strong>Battle &amp; Sweep</strong> để đối đầu nhiều chiến lược cùng lúc hoặc quét tối ưu tham số IS/OOS.
      </div>

      {/* Pending Custom Rule Prefill Banner (GAP-U-03, LAB-002) */}
      {pendingRule && (
        <div
          data-testid="pending-rule-prefill-banner"
          className="glass-panel"
          style={{
            padding: '12px 16px',
            marginBottom: '16px',
            borderColor: '#00E676',
            background: 'rgba(0, 230, 118, 0.08)',
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'center',
            gap: '12px',
          }}
        >
          <div>
            <div style={{ color: '#00E676', fontWeight: 600, fontSize: '13px' }}>
              ✓ Đã nạp quy tắc từ Rule Builder: <strong>{pendingRule.ruleName}</strong>
            </div>
            <div style={{ fontSize: '12px', color: 'var(--text-muted)', marginTop: '4px' }}>
              Biểu thức AST: <code style={{ color: '#58A6FF' }}>{pendingRule.expression}</code>
            </div>
          </div>
          <button
            type="button"
            data-testid="clear-pending-rule-btn"
            onClick={clearPendingRule}
            style={{
              padding: '4px 10px',
              fontSize: '11px',
              background: 'rgba(255, 255, 255, 0.08)',
              color: 'var(--text-muted)',
              border: '1px solid var(--border-color)',
              borderRadius: '4px',
            }}
          >
            Gỡ quy tắc tùy chỉnh
          </button>
        </div>
      )}

      <div className="glass-panel" style={{ padding: '24px', marginBottom: '20px' }}>
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
              value={effectiveStrategyFilename}
              onChange={e => setSelectedStrategyFilename(e.target.value)}
              style={{ width: '100%', padding: '10px' }}
              required
            >
              <option value="" disabled>Select a strategy</option>
              {allStrategies.map(s => (
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
        <div
          data-testid="market-assumptions-banner"
          style={{
            marginTop: '14px',
            paddingTop: '10px',
            borderTop: '1px solid rgba(255, 255, 255, 0.08)',
            fontSize: '11px',
            color: 'var(--text-muted)',
          }}
        >
          Giả định thị trường VN (MarketProfile): Phí mua 0.15% · Phí bán 0.15% · Thuế bán 0.10% · Lô chẵn 100 CP · Khớp giá Mở cửa phiên kế (Next-Open) · Thanh toán T+1.5 (bán khả dụng từ phiên kế theo lịch giao dịch VN)
        </div>
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
            {renderExecutedTradesTable(executedTrades)}
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

          {(analytics.low_sample_warning || analytics.total_trades < 30) && (
            <div
              role="status"
              data-testid="backtest-low-sample-banner"
              style={{
                padding: '10px 14px',
                marginBottom: '16px',
                background: 'rgba(255, 209, 102, 0.1)',
                border: '1px solid rgba(255, 209, 102, 0.35)',
                borderRadius: '6px',
                fontSize: '12px',
                color: '#FFD166',
              }}
            >
              ⚠️ <strong>Cảnh báo thống kê ({analytics.total_trades}/30 lệnh):</strong>{' '}
              {analytics.low_sample_message || 'mẫu nhỏ (<30), độ tin cậy thấp'}
            </div>
          )}

          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(180px, 1fr))', gap: '16px', marginBottom: '24px' }}>
            <div className="glass-panel" style={{ padding: '20px' }}>
              <h4 style={{ color: 'var(--text-muted)', margin: '0 0 8px 0', fontSize: '13px', textTransform: 'uppercase', letterSpacing: '1px' }}>Net PnL</h4>
              <p style={{ margin: 0, fontSize: '28px', fontWeight: 'bold', color: analytics.total_net_pnl >= 0 ? 'var(--color-buy)' : 'var(--color-sell)' }}>
                {formatMoney(analytics.total_net_pnl)}
              </p>
            </div>
            <div className="glass-panel" style={{ padding: '20px' }}>
              <h4 style={{ color: 'var(--text-muted)', margin: '0 0 8px 0', fontSize: '13px', textTransform: 'uppercase', letterSpacing: '1px' }}>Win Rate</h4>
              <MetricResultValue metric={analytics.metrics?.win_rate} format={formatPercent} testId="backtest-win-rate" />
            </div>
            <div className="glass-panel" style={{ padding: '20px' }}>
              <h4 style={{ color: 'var(--text-muted)', margin: '0 0 8px 0', fontSize: '13px', textTransform: 'uppercase', letterSpacing: '1px' }}>Sharpe Ratio</h4>
              <MetricResultValue metric={analytics.metrics?.sharpe_ratio} testId="backtest-sharpe" />
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
              <div style={{ margin: 0 }}>
                <MetricResultValue metric={analytics.metrics?.profit_factor} testId="backtest-profit-factor" />
                {analytics.profit_factor_display && (
                  <div
                    data-testid="backtest-profit-factor-display"
                    title="Profit Factor = Tổng lãi ròng / |Tổng lỗ ròng|. Khi chưa có lệnh thua, tỷ số không xác định (N/A)."
                    style={{ fontSize: '12px', color: '#FFD166', marginTop: '4px', fontWeight: 600 }}
                  >
                    {analytics.profit_factor_display}
                  </div>
                )}
              </div>
            </div>
            <div className="glass-panel" style={{ padding: '16px' }}>
              <h4 style={{ color: 'var(--text-muted)', margin: '0 0 8px 0', fontSize: '13px' }}>SQN</h4>
              <div style={{ margin: 0 }}>
                <MetricResultValue metric={analytics.metrics?.sqn} testId="backtest-sqn" />
              </div>
            </div>
          </div>

          {renderExecutedTradesTable(executedTrades)}

          {result.data_coverage && (
            <div className="glass-panel" style={{ padding: '16px', marginTop: '24px' }} data-testid="backtest-run-contract">
              <h3 style={{ margin: '0 0 8px', fontSize: '16px' }}>Run coverage and assumptions</h3>
              <p style={{ margin: '4px 0', color: 'var(--text-muted)', fontSize: '13px' }}>
                Requested {result.data_coverage.requested_start} → {result.data_coverage.requested_end}; actual {result.data_coverage.actual_start || 'n/a'} → {result.data_coverage.actual_end || 'n/a'}; {result.data_coverage.candle_count} candles; {result.data_coverage.warmup_candles} warm-up.
              </p>
              <p style={{ margin: '4px 0', color: 'var(--text-muted)', fontSize: '13px' }}>
                {result.execution_assumptions?.execution_timing}; {result.execution_assumptions?.price_basis}; settlement: {result.execution_assumptions?.settlement}.
              </p>
              <p style={{ margin: '4px 0', color: 'var(--text-muted)', fontSize: '12px' }}>Run manifest: {result.run_manifest?.input_hash?.slice(0, 12) || 'n/a'}</p>
            </div>
          )}

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
