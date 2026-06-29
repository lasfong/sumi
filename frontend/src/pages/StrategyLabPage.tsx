import React, { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { getAvailableStrategies, runBacktest } from '../api/backtestApi';
import type { BacktestRequest, BacktestResponse, AvailableStrategy } from '../api/backtestApi';
import { deleteStrategyLabRun, listStrategyLabRuns, runParameterSweep, saveStrategyLabRun } from '../api/strategyLabApi';
import type { StrategyLabRun, SweepVariant } from '../api/strategyLabApi';

interface LabResult {
  filename: string;
  name: string;
  response: BacktestResponse;
}

interface LabHistoryEntry {
  id: string;
  type: 'comparison' | 'sweep';
  createdAt: string;
  label: string;
  results?: LabResult[];
  sweepResults?: SweepVariant[];
}

const HISTORY_KEY = 'sumi.strategyLab.history.v1';

const loadHistory = (): LabHistoryEntry[] => {
  if (typeof window === 'undefined') return [];
  try {
    const raw = window.localStorage.getItem(HISTORY_KEY);
    return raw ? JSON.parse(raw) : [];
  } catch {
    return [];
  }
};

const saveHistory = (entries: LabHistoryEntry[]) => {
  if (typeof window !== 'undefined') {
    window.localStorage.setItem(HISTORY_KEY, JSON.stringify(entries.slice(0, 20)));
  }
};

export const StrategyLabPage: React.FC = () => {
  const [symbolsInput, setSymbolsInput] = useState('FPT, SSI, VCI');
  const [startDate, setStartDate] = useState('2020-01-01');
  const [endDate, setEndDate] = useState('2023-12-31');
  const [initialCash, setInitialCash] = useState(100000000);
  const [benchmarkSymbol, setBenchmarkSymbol] = useState('VNINDEX');
  const [selectedFilenames, setSelectedFilenames] = useState<string[]>([]);
  const [isRunning, setIsRunning] = useState(false);
  const [isSweeping, setIsSweeping] = useState(false);
  const [results, setResults] = useState<LabResult[]>([]);
  const [sweepPath, setSweepPath] = useState('indicators[0].length');
  const [sweepValues, setSweepValues] = useState('5, 10, 20');
  const [maxVariants, setMaxVariants] = useState(12);
  const [sweepResults, setSweepResults] = useState<SweepVariant[]>([]);
  const [history, setHistory] = useState<LabHistoryEntry[]>(loadHistory);
  const [hiddenSavedRunIds, setHiddenSavedRunIds] = useState<number[]>([]);
  const [error, setError] = useState<string | null>(null);

  const { data: strategies, isLoading: isLoadingStrategies } = useQuery({
    queryKey: ['strategies'],
    queryFn: getAvailableStrategies,
  });

  const { data: savedRuns } = useQuery({
    queryKey: ['strategy-lab-runs'],
    queryFn: () => listStrategyLabRuns(50),
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
      addHistory({
        type: 'comparison',
        label: `${selected.length} strategy comparison`,
        results: nextResults,
      }, {
        symbols,
        start_date: startDate,
        end_date: endDate,
        initial_cash: initialCash,
        benchmark_symbol: benchmarkSymbol,
      });
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Strategy comparison failed.');
    } finally {
      setIsRunning(false);
    }
  };

  const handleSweep = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!strategies) return;

    const strategy = strategies.find(item => item.filename === selectedFilenames[0]);
    if (!strategy) {
      setError('Select one strategy as the sweep base.');
      return;
    }

    const symbols = symbolsInput.split(',').map(item => item.trim().toUpperCase()).filter(Boolean);
    const values = sweepValues.split(',').map(item => parseSweepValue(item.trim())).filter(item => item !== '');
    if (symbols.length === 0 || values.length === 0) {
      setError('Enter symbols and sweep values.');
      return;
    }

    setIsSweeping(true);
    setError(null);
    try {
      const request = {
        start_date: startDate,
        end_date: endDate,
        initial_cash: initialCash,
        benchmark_symbol: benchmarkSymbol.trim().toUpperCase() || undefined,
        strategy: strategy.config,
        sweep: [{ path: sweepPath.trim(), values }],
        max_variants: maxVariants,
      };
      const response = await runParameterSweep(symbols.length > 1 ? { ...request, symbols } : { ...request, symbol: symbols[0] });
      if (response.status === 'failed') {
        setError(response.message || 'Parameter sweep failed.');
        setSweepResults([]);
      } else {
        setSweepResults(response.variants);
        addHistory({
          type: 'sweep',
          label: `${strategy.name} sweep: ${sweepPath.trim()}`,
          sweepResults: response.variants,
        }, {
          symbols,
          start_date: startDate,
          end_date: endDate,
          initial_cash: initialCash,
          benchmark_symbol: benchmarkSymbol,
          sweep_path: sweepPath.trim(),
          sweep_values: values,
        });
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Parameter sweep failed.');
    } finally {
      setIsSweeping(false);
    }
  };

  const selectAll = () => {
    setSelectedFilenames((strategies || []).map(strategy => strategy.filename));
  };

  const clearSelection = () => {
    setSelectedFilenames([]);
  };

  const addHistory = (entry: Omit<LabHistoryEntry, 'id' | 'createdAt'>, requestConfig: Record<string, unknown> = {}) => {
    setHistory((current) => {
      const next = [{
        ...entry,
        id: `${Date.now()}-${Math.random().toString(16).slice(2)}`,
        createdAt: new Date().toISOString(),
      }, ...current].slice(0, 20);
      saveHistory(next);
      return next;
    });
    void saveStrategyLabRun({
      run_type: entry.type,
      label: entry.label,
      request_config: requestConfig,
      result_payload: {
        results: entry.results,
        sweepResults: entry.sweepResults,
      },
      metrics: {},
    });
  };

  const restoreHistory = (entry: LabHistoryEntry) => {
    if (entry.results) setResults(entry.results);
    if (entry.sweepResults) setSweepResults(entry.sweepResults);
    setError(null);
  };

  const clearHistory = () => {
    setHistory([]);
    saveHistory([]);
    setHiddenSavedRunIds((savedRuns || []).map(run => run.id));
    void Promise.all((savedRuns || []).map(run => deleteStrategyLabRun(run.id)));
  };

  const getTrades = (response: BacktestResponse) => response.summary?.total_trades ?? response.analytics?.total_trades ?? 0;
  const getWinRate = (response: BacktestResponse) => response.summary?.win_rate ?? response.analytics?.win_rate ?? 0;
  const getNetPnl = (response: BacktestResponse) => response.summary?.total_net_pnl ?? response.analytics?.total_net_pnl ?? 0;
  const getProfitFactor = (response: BacktestResponse) => response.analytics?.profit_factor ?? null;
  const getExpectancy = (response: BacktestResponse) => response.analytics?.expectancy ?? null;
  const formatMoney = (value?: number | null) => value ? value.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 }) : '0.00';
  const formatPercent = (value?: number) => value ? `${(value * 100).toFixed(2)}%` : '0.00%';
  const parseSweepValue = (value: string) => {
    if (value === '') return '';
    if (value.toLowerCase() === 'true') return true;
    if (value.toLowerCase() === 'false') return false;
    const numeric = Number(value);
    return Number.isNaN(numeric) ? value : numeric;
  };

  const sortedResults = [...results].sort((left, right) => getNetPnl(right.response) - getNetPnl(left.response));
  const bestFilename = sortedResults[0]?.filename;
  const savedHistory = (savedRuns || [])
    .filter(run => !hiddenSavedRunIds.includes(run.id))
    .map((run: StrategyLabRun): LabHistoryEntry => ({
      id: `server-${run.id}`,
      type: run.run_type,
      createdAt: run.created_at,
      label: run.label,
      results: run.result_payload.results as LabResult[] | undefined,
      sweepResults: run.result_payload.sweepResults as SweepVariant[] | undefined,
    }));
  const displayHistory = [...history, ...savedHistory];

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

      {displayHistory.length > 0 && (
        <div className="glass-panel" style={{ padding: '20px', marginBottom: '24px' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '16px' }}>
            <h3 style={{ margin: 0, fontSize: '16px' }}>Run History</h3>
            <button type="button" onClick={clearHistory} style={{ padding: '6px 10px', fontSize: '12px' }}>Clear History</button>
          </div>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(240px, 1fr))', gap: '12px' }}>
            {displayHistory.map((entry) => (
              <button key={entry.id} type="button" onClick={() => restoreHistory(entry)} className="glass-panel" style={{ padding: '12px', textAlign: 'left', cursor: 'pointer' }}>
                <strong style={{ display: 'block', marginBottom: '4px' }}>{entry.label}</strong>
                <span style={{ color: 'var(--text-muted)', fontSize: '12px' }}>
                  {entry.type} - {new Date(entry.createdAt).toLocaleString()}
                </span>
              </button>
            ))}
          </div>
        </div>
      )}

      <div className="glass-panel" style={{ padding: '20px', marginBottom: '24px' }}>
        <h3 style={{ marginTop: 0, marginBottom: '16px', fontSize: '16px' }}>Parameter Sweep</h3>
        <form onSubmit={handleSweep} style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))', gap: '16px', alignItems: 'end' }}>
          <div>
            <label htmlFor="sweep-path" style={{ display: 'block', marginBottom: '8px', color: 'var(--text-muted)', fontSize: '14px' }}>Parameter Path</label>
            <input id="sweep-path" type="text" value={sweepPath} onChange={event => setSweepPath(event.target.value)} style={{ width: '100%', padding: '10px' }} required />
          </div>
          <div>
            <label htmlFor="sweep-values" style={{ display: 'block', marginBottom: '8px', color: 'var(--text-muted)', fontSize: '14px' }}>Values</label>
            <input id="sweep-values" type="text" value={sweepValues} onChange={event => setSweepValues(event.target.value)} style={{ width: '100%', padding: '10px' }} required />
          </div>
          <div>
            <label htmlFor="sweep-max" style={{ display: 'block', marginBottom: '8px', color: 'var(--text-muted)', fontSize: '14px' }}>Max Variants</label>
            <input id="sweep-max" type="number" min={1} max={30} value={maxVariants} onChange={event => setMaxVariants(Number(event.target.value))} style={{ width: '100%', padding: '10px' }} required />
          </div>
          <button type="submit" className="btn-primary" style={{ height: '42px' }} disabled={isSweeping || isLoadingStrategies}>
            {isSweeping ? 'Sweeping...' : 'Run Sweep'}
          </button>
        </form>
      </div>

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

      {sweepResults.length > 0 && (
        <div className="glass-panel" style={{ padding: '20px', marginTop: '24px' }}>
          <h3 style={{ marginTop: 0, marginBottom: '16px', fontSize: '16px' }}>Sweep Results</h3>
          <div style={{ overflowX: 'auto' }}>
            <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '13px' }}>
              <thead>
                <tr style={{ borderBottom: '1px solid rgba(255,255,255,0.1)', color: 'var(--text-muted)', textAlign: 'left' }}>
                  <th style={{ padding: '8px', fontWeight: 500 }}>Variant</th>
                  <th style={{ padding: '8px', fontWeight: 500 }}>Status</th>
                  <th style={{ padding: '8px', fontWeight: 500, textAlign: 'right' }}>Trades</th>
                  <th style={{ padding: '8px', fontWeight: 500, textAlign: 'right' }}>Win %</th>
                  <th style={{ padding: '8px', fontWeight: 500, textAlign: 'right' }}>Net PnL</th>
                  <th style={{ padding: '8px', fontWeight: 500, textAlign: 'right' }}>Profit Factor</th>
                  <th style={{ padding: '8px', fontWeight: 500, textAlign: 'right' }}>Expectancy</th>
                </tr>
              </thead>
              <tbody>
                {sweepResults.map((item) => (
                  <tr key={item.label} style={{ borderBottom: '1px solid rgba(255,255,255,0.05)' }}>
                    <td style={{ padding: '12px 8px', fontWeight: 600 }}>{item.label}</td>
                    <td style={{ padding: '12px 8px', textTransform: 'uppercase' }}>{item.metrics.status}</td>
                    <td style={{ padding: '12px 8px', textAlign: 'right' }}>{item.metrics.total_trades}</td>
                    <td style={{ padding: '12px 8px', textAlign: 'right' }}>{formatPercent(item.metrics.win_rate)}</td>
                    <td style={{ padding: '12px 8px', textAlign: 'right', color: item.metrics.net_pnl >= 0 ? 'var(--color-buy)' : 'var(--color-sell)', fontWeight: 600 }}>{formatMoney(item.metrics.net_pnl)}</td>
                    <td style={{ padding: '12px 8px', textAlign: 'right' }}>{item.metrics.profit_factor?.toFixed(2) || 'N/A'}</td>
                    <td style={{ padding: '12px 8px', textAlign: 'right' }}>{formatMoney(item.metrics.expectancy)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}
    </div>
  );
};
