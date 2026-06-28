import React, { useState } from 'react';
import { useMutation, useQuery } from '@tanstack/react-query';
import { getAvailableStrategies } from '../api/backtestApi';
import { runScanner } from '../api/scannerApi';
import type { ScannerRequest } from '../api/scannerApi';

export const ScannerPage: React.FC = () => {
  const [symbolsInput, setSymbolsInput] = useState('FPT, SSI, VCI');
  const [startDate, setStartDate] = useState('2020-01-01');
  const [endDate, setEndDate] = useState('2023-12-31');
  const [benchmarkSymbol, setBenchmarkSymbol] = useState('VNINDEX');
  const [maxResults, setMaxResults] = useState(200);
  const [selectedStrategyFilename, setSelectedStrategyFilename] = useState('');

  const { data: strategies, isLoading: isLoadingStrategies } = useQuery({
    queryKey: ['strategies'],
    queryFn: getAvailableStrategies,
  });

  const mutation = useMutation({
    mutationFn: runScanner,
  });

  const handleSubmit = (event: React.FormEvent) => {
    event.preventDefault();
    if (!strategies) return;

    const strategy = strategies.find(item => item.filename === selectedStrategyFilename);
    if (!strategy) {
      alert('Please select a strategy.');
      return;
    }

    const symbols = symbolsInput.split(',').map(item => item.trim().toUpperCase()).filter(Boolean);
    const request: ScannerRequest = {
      symbols,
      start_date: startDate,
      end_date: endDate,
      strategy: strategy.config,
      benchmark_symbol: benchmarkSymbol.trim().toUpperCase() || undefined,
      max_results: maxResults,
    };

    mutation.mutate(request);
  };

  const formatMoney = (value?: number) => value ? value.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 }) : '0.00';
  const result = mutation.data;

  return (
    <div className="animate-fade-in" style={{ maxWidth: '1200px', margin: '0 auto', paddingBottom: '40px' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '2rem' }}>
        <h2 style={{ margin: 0, fontSize: '28px', background: 'linear-gradient(90deg, #fff, #8B949E)', WebkitBackgroundClip: 'text', WebkitTextFillColor: 'transparent' }}>
          Signal Scanner
        </h2>
      </div>

      <div className="glass-panel" style={{ padding: '24px', marginBottom: '32px' }}>
        <form onSubmit={handleSubmit} style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))', gap: '20px', alignItems: 'end' }}>
          <div>
            <label htmlFor="scanner-symbols" style={{ display: 'block', marginBottom: '8px', color: 'var(--text-muted)', fontSize: '14px' }}>Symbols</label>
            <input id="scanner-symbols" type="text" value={symbolsInput} onChange={event => setSymbolsInput(event.target.value.toUpperCase())} style={{ width: '100%', padding: '10px' }} required />
          </div>
          <div>
            <label htmlFor="scanner-start" style={{ display: 'block', marginBottom: '8px', color: 'var(--text-muted)', fontSize: '14px' }}>Start Date</label>
            <input id="scanner-start" type="date" value={startDate} onChange={event => setStartDate(event.target.value)} style={{ width: '100%', padding: '10px' }} required />
          </div>
          <div>
            <label htmlFor="scanner-end" style={{ display: 'block', marginBottom: '8px', color: 'var(--text-muted)', fontSize: '14px' }}>End Date</label>
            <input id="scanner-end" type="date" value={endDate} onChange={event => setEndDate(event.target.value)} style={{ width: '100%', padding: '10px' }} required />
          </div>
          <div>
            <label htmlFor="scanner-benchmark" style={{ display: 'block', marginBottom: '8px', color: 'var(--text-muted)', fontSize: '14px' }}>Benchmark</label>
            <input id="scanner-benchmark" type="text" value={benchmarkSymbol} onChange={event => setBenchmarkSymbol(event.target.value.toUpperCase())} style={{ width: '100%', padding: '10px' }} />
          </div>
          <div>
            <label htmlFor="scanner-max" style={{ display: 'block', marginBottom: '8px', color: 'var(--text-muted)', fontSize: '14px' }}>Max Results</label>
            <input id="scanner-max" type="number" min={1} max={1000} value={maxResults} onChange={event => setMaxResults(Number(event.target.value))} style={{ width: '100%', padding: '10px' }} required />
          </div>
          <div>
            <label htmlFor="scanner-strategy" style={{ display: 'block', marginBottom: '8px', color: 'var(--text-muted)', fontSize: '14px' }}>Strategy</label>
            <select id="scanner-strategy" value={selectedStrategyFilename} onChange={event => setSelectedStrategyFilename(event.target.value)} style={{ width: '100%', padding: '10px' }} required>
              <option value="" disabled>Select a strategy</option>
              {strategies?.map(strategy => (
                <option key={strategy.filename} value={strategy.filename}>{strategy.name} ({strategy.filename})</option>
              ))}
            </select>
          </div>
          <div>
            <button type="submit" className="btn-primary" style={{ width: '100%', height: '42px', display: 'flex', alignItems: 'center', justifyContent: 'center' }} disabled={mutation.isPending || isLoadingStrategies}>
              {mutation.isPending ? 'Scanning...' : 'Run Scanner'}
            </button>
          </div>
        </form>
      </div>

      {result?.status === 'failed' && (
        <div className="glass-panel" style={{ borderColor: 'var(--color-sell)', padding: '16px' }}>
          <p style={{ color: 'var(--color-sell)', margin: 0 }}>{result.message || 'Scanner failed.'}</p>
        </div>
      )}

      {result?.status === 'succeeded' && (
        <div className="glass-panel" style={{ padding: '20px' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '16px' }}>
            <h3 style={{ margin: 0, fontSize: '16px' }}>Signals</h3>
            <span style={{ color: 'var(--text-muted)', fontSize: '13px' }}>{result.total_results || 0} results{result.truncated ? ' (truncated)' : ''}</span>
          </div>
          {result.results.length > 0 ? (
            <div style={{ overflowX: 'auto' }}>
              <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '13px' }}>
                <thead>
                  <tr style={{ borderBottom: '1px solid rgba(255,255,255,0.1)', color: 'var(--text-muted)', textAlign: 'left' }}>
                    <th style={{ padding: '8px', fontWeight: 500 }}>Symbol</th>
                    <th style={{ padding: '8px', fontWeight: 500 }}>Date</th>
                    <th style={{ padding: '8px', fontWeight: 500 }}>Signal</th>
                    <th style={{ padding: '8px', fontWeight: 500 }}>Regime</th>
                    <th style={{ padding: '8px', fontWeight: 500, textAlign: 'right' }}>Price</th>
                  </tr>
                </thead>
                <tbody>
                  {result.results.map((item, index) => (
                    <tr key={`${item.symbol}-${item.timestamp}-${index}`} style={{ borderBottom: '1px solid rgba(255,255,255,0.05)' }}>
                      <td style={{ padding: '12px 8px', fontWeight: 600 }}>{item.symbol}</td>
                      <td style={{ padding: '12px 8px', color: 'var(--text-muted)' }}>{new Date(item.timestamp).toLocaleDateString()}</td>
                      <td style={{ padding: '12px 8px', textTransform: 'uppercase' }}>{item.signal_type}</td>
                      <td style={{ padding: '12px 8px' }}>{item.regime}</td>
                      <td style={{ padding: '12px 8px', textAlign: 'right', fontFamily: 'monospace' }}>{formatMoney(item.price)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          ) : (
            <p style={{ color: 'var(--text-muted)', fontSize: '14px', margin: 0 }}>No signals found.</p>
          )}
        </div>
      )}
    </div>
  );
};
