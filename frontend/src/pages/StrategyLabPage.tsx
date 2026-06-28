import React, { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { getAvailableStrategies, runBacktest } from '../api/backtestApi';
import type { BacktestRequest, BacktestResponse, AvailableStrategy } from '../api/backtestApi';

interface LabResult {
  filename: string;
  name: string;
  response: BacktestResponse;
}

export const StrategyLabPage: React.FC = () => {
  const [symbolsInput, setSymbolsInput] = useState('FPT, SSI, VCI');
  const [startDate, setStartDate] = useState('2020-01-01');
  const [endDate, setEndDate] = useState('2023-12-31');
  const [initialCash, setInitialCash] = useState(100000000);
  const [benchmarkSymbol, setBenchmarkSymbol] = useState('VNINDEX');
  const [selectedFilenames, setSelectedFilenames] = useState<string[]>([]);
  const [isRunning, setIsRunning] = useState(false);
  const [results, setResults] = useState<LabResult[]>([]);
  const [error, setError] = useState<string | null>(null);

  const { data: strategies, isLoading: isLoadingStrategies } = useQuery({
    queryKey: ['strategies'],
    queryFn: getAvailableStrategies,
  });

  const toggleStrategy = (filename: string) => {
    setSelectedFilenames((current) => (
      current.includes(filename)
        ? current.filter(item => item !== filename)
        : [...current, filename]
    ));
  };

  const handleRun = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!strategies) return;

    const selected = strategies.filter(strategy => selectedFilenames.includes(strategy.filename));
    if (selected.length === 0) {
      setError('Select at least one strategy.');
      return;
    }

    const symbols = symbolsInput.split(',').map(item => item.trim().toUpperCase()).filter(Boolean);
    if (symbols.length === 0) {
      setError('Enter at least one symbol.');
      return;
    }

    setIsRunning(true);
    setError(null);
    try {
      const nextResults = await Promise.all(selected.map(async (strategy) => {
        const request: BacktestRequest = {
          start_date: startDate,
          end_date: endDate,
          initial_cash: initialCash,
          benchmark_symbol: benchmarkSymbol.trim().toUpperCase() || undefined,
          strategy: strategy.config,
        };
        if (symbols.length > 1) {
          request.symbols = symbols;
        } else {
          request.symbol = symbols[0];
        }
        const response = await runBacktest(request);
        return {
          filename: strategy.filename,
          name: strategy.name,
          response,
        };
      }));
      setResults(nextResults);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Strategy comparison failed.');
    } finally {
      setIsRunning(false);
    }
  };

  const selectAll = () => {
    setSelectedFilenames((strategies || []).map(strategy => strategy.filename));
  };

  const clearSelection = () => {
    setSelectedFilenames([]);
  };

  const getTrades = (response: BacktestResponse) => response.summary?.total_trades ?? response.analytics?.total_trades ?? 0;
  const getWinRate = (response: BacktestResponse) => response.summary?.win_rate ?? response.analytics?.win_rate ?? 0;
  const getNetPnl = (response: BacktestResponse) => response.summary?.total_net_pnl ?? response.analytics?.total_net_pnl ?? 0;
  const getProfitFactor = (response: BacktestResponse) => response.analytics?.profit_factor ?? null;
  const getExpectancy = (response: BacktestResponse) => response.analytics?.expectancy ?? null;
  const formatMoney = (value?: number | null) => value ? value.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 }) : '0.00';
  const formatPercent = (value?: number) => value ? `${(value * 100).toFixed(2)}%` : '0.00%';

  const sortedResults = [...results].sort((left, right) => getNetPnl(right.response) - getNetPnl(left.response));
  const bestFilename = sortedResults[0]?.filename;

  return (
    <div className="animate-fade-in" style={{ maxWidth: '1200px', margin: '0 auto', paddingBottom: '40px' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '2rem' }}>
        <h2 style={{ margin: 0, fontSize: '28px', background: 'linear-gradient(90deg, #fff, #8B949E)', WebkitBackgroundClip: 'text', WebkitTextFillColor: 'transparent' }}>
          Strategy Lab
        </h2>
      </div>

      <div className="glass-panel" style={{ padding: '24px', marginBottom: '24px' }}>
        <form onSubmit={handleRun} style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))', gap: '20px', alignItems: 'end' }}>
          <div>
            <label htmlFor="lab-symbols" style={{ display: 'block', marginBottom: '8px', color: 'var(--text-muted)', fontSize: '14px' }}>Symbols</label>
            <input id="lab-symbols" type="text" value={symbolsInput} onChange={event => setSymbolsInput(event.target.value.toUpperCase())} style={{ width: '100%', padding: '10px' }} required />
          </div>
          <div>
            <label htmlFor="lab-start" style={{ display: 'block', marginBottom: '8px', color: 'var(--text-muted)', fontSize: '14px' }}>Start Date</label>
            <input id="lab-start" type="date" value={startDate} onChange={event => setStartDate(event.target.value)} style={{ width: '100%', padding: '10px' }} required />
          </div>
          <div>
            <label htmlFor="lab-end" style={{ display: 'block', marginBottom: '8px', color: 'var(--text-muted)', fontSize: '14px' }}>End Date</label>
            <input id="lab-end" type="date" value={endDate} onChange={event => setEndDate(event.target.value)} style={{ width: '100%', padding: '10px' }} required />
          </div>
          <div>
            <label htmlFor="lab-cash" style={{ display: 'block', marginBottom: '8px', color: 'var(--text-muted)', fontSize: '14px' }}>Initial Cash</label>
            <input id="lab-cash" type="number" value={initialCash} onChange={event => setInitialCash(Number(event.target.value))} style={{ width: '100%', padding: '10px' }} required />
          </div>
          <div>
            <label htmlFor="lab-benchmark" style={{ display: 'block', marginBottom: '8px', color: 'var(--text-muted)', fontSize: '14px' }}>Benchmark</label>
            <input id="lab-benchmark" type="text" value={benchmarkSymbol} onChange={event => setBenchmarkSymbol(event.target.value.toUpperCase())} style={{ width: '100%', padding: '10px' }} />
          </div>
          <button type="submit" className="btn-primary" style={{ height: '42px' }} disabled={isRunning || isLoadingStrategies}>
            {isRunning ? 'Running...' : 'Compare Strategies'}
          </button>
        </form>
      </div>

      <div className="glass-panel" style={{ padding: '20px', marginBottom: '24px' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '16px' }}>
          <h3 style={{ margin: 0, fontSize: '16px' }}>Strategies</h3>
          <div style={{ display: 'flex', gap: '8px' }}>
            <button type="button" onClick={selectAll} style={{ padding: '6px 10px', fontSize: '12px' }}>Select All</button>
            <button type="button" onClick={clearSelection} style={{ padding: '6px 10px', fontSize: '12px' }}>Clear</button>
          </div>
        </div>
        {strategies && strategies.length > 0 ? (
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(220px, 1fr))', gap: '12px' }}>
            {strategies.map((strategy: AvailableStrategy) => (
              <label key={strategy.filename} className="glass-panel" style={{ padding: '12px', display: 'flex', gap: '10px', alignItems: 'flex-start', cursor: 'pointer' }}>
                <input type="checkbox" checked={selectedFilenames.includes(strategy.filename)} onChange={() => toggleStrategy(strategy.filename)} />
                <span>
                  <strong style={{ display: 'block' }}>{strategy.name}</strong>
                  <span style={{ color: 'var(--text-muted)', fontSize: '12px' }}>{strategy.filename}</span>
                </span>
              </label>
            ))}
          </div>
        ) : (
          <p style={{ color: 'var(--text-muted)', fontSize: '14px', margin: 0 }}>No strategies available.</p>
        )}
      </div>

      {error && (
        <div className="glass-panel" style={{ borderColor: 'var(--color-sell)', padding: '16px', marginBottom: '24px' }}>
          <p style={{ color: 'var(--color-sell)', margin: 0 }}>{error}</p>
        </div>
      )}

      {sortedResults.length > 0 && (
        <div className="glass-panel" style={{ padding: '20px' }}>
          <h3 style={{ marginTop: 0, marginBottom: '16px', fontSize: '16px' }}>Comparison</h3>
          <div style={{ overflowX: 'auto' }}>
            <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '13px' }}>
              <thead>
                <tr style={{ borderBottom: '1px solid rgba(255,255,255,0.1)', color: 'var(--text-muted)', textAlign: 'left' }}>
                  <th style={{ padding: '8px', fontWeight: 500 }}>Strategy</th>
                  <th style={{ padding: '8px', fontWeight: 500 }}>Status</th>
                  <th style={{ padding: '8px', fontWeight: 500, textAlign: 'right' }}>Trades</th>
                  <th style={{ padding: '8px', fontWeight: 500, textAlign: 'right' }}>Win %</th>
                  <th style={{ padding: '8px', fontWeight: 500, textAlign: 'right' }}>Net PnL</th>
                  <th style={{ padding: '8px', fontWeight: 500, textAlign: 'right' }}>Profit Factor</th>
                  <th style={{ padding: '8px', fontWeight: 500, textAlign: 'right' }}>Expectancy</th>
                </tr>
              </thead>
              <tbody>
                {sortedResults.map((item) => {
                  const netPnl = getNetPnl(item.response);
                  const isBest = item.filename === bestFilename;
                  return (
                    <tr key={item.filename} style={{ borderBottom: '1px solid rgba(255,255,255,0.05)' }}>
                      <td style={{ padding: '12px 8px', fontWeight: 600 }}>
                        {item.name}
                        {isBest && <span style={{ marginLeft: '8px', color: 'var(--color-buy)', fontSize: '12px' }}>Best</span>}
                      </td>
                      <td style={{ padding: '12px 8px', textTransform: 'uppercase', color: item.response.status === 'failed' ? 'var(--color-sell)' : 'var(--text-main)' }}>{item.response.status || 'succeeded'}</td>
                      <td style={{ padding: '12px 8px', textAlign: 'right' }}>{getTrades(item.response)}</td>
                      <td style={{ padding: '12px 8px', textAlign: 'right' }}>{formatPercent(getWinRate(item.response))}</td>
                      <td style={{ padding: '12px 8px', textAlign: 'right', fontWeight: 600, color: netPnl >= 0 ? 'var(--color-buy)' : 'var(--color-sell)' }}>{formatMoney(netPnl)}</td>
                      <td style={{ padding: '12px 8px', textAlign: 'right' }}>{getProfitFactor(item.response)?.toFixed(2) || 'N/A'}</td>
                      <td style={{ padding: '12px 8px', textAlign: 'right' }}>{formatMoney(getExpectancy(item.response))}</td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        </div>
      )}
    </div>
  );
};
