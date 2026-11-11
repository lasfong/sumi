import React, { useState, useMemo } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useNavigate } from 'react-router-dom';
import { getAvailableStrategies } from '../api/backtestApi';
import { createReplaySessionFromSignal, listScannerRuns, runScanner } from '../api/scannerApi';
import type { ScannerRequest, ScannerResult, ScannerRun } from '../api/scannerApi';
import type { ReplayIntent } from '../types';
import { useReplayStore } from '../store/replayStore';

export const ScannerPage: React.FC = () => {
  const [symbolsInput, setSymbolsInput] = useState('FPT, SSI, VCI');
  const [startDate, setStartDate] = useState('2020-01-01');
  const [endDate, setEndDate] = useState('2023-12-31');
  const [benchmarkSymbol, setBenchmarkSymbol] = useState('VNINDEX');
  const [maxResults, setMaxResults] = useState(200);
  const [selectedStrategyFilename, setSelectedStrategyFilename] = useState('');
  const [replayError, setReplayError] = useState<string | null>(null);
  const [selectedHistoryRun, setSelectedHistoryRun] = useState<ScannerRun | null>(null);

  // SQA-005: Table search, filtering, and sorting state
  const [searchQuery, setSearchQuery] = useState('');
  const [signalFilter, setSignalFilter] = useState('ALL');
  const [regimeFilter, setRegimeFilter] = useState('ALL');
  const [sortField, setSortField] = useState<'symbol' | 'timestamp' | 'signal_type' | 'regime' | 'price'>('timestamp');
  const [sortDirection, setSortDirection] = useState<'asc' | 'desc'>('asc');

  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const setSession = useReplayStore(state => state.setSession);

  const { data: strategies, isLoading: isLoadingStrategies } = useQuery({
    queryKey: ['strategies'],
    queryFn: getAvailableStrategies,
  });

  const mutation = useMutation({
    mutationFn: runScanner,
    onSuccess: () => {
      setSelectedHistoryRun(null);
      queryClient.invalidateQueries({ queryKey: ['scanner-runs'] });
    },
  });

  const replayMutation = useMutation({
    mutationFn: createReplaySessionFromSignal,
  });

  const { data: scannerRuns } = useQuery({
    queryKey: ['scanner-runs'],
    queryFn: () => listScannerRuns(8),
  });

  const effectiveStrategyFilename = selectedStrategyFilename || strategies?.[0]?.filename || '';

  const handleSubmit = (event: React.FormEvent) => {
    event.preventDefault();
    setSelectedHistoryRun(null);
    if (!strategies) return;

    const strategy = strategies.find(item => item.filename === effectiveStrategyFilename);
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
  // SQA-015: When a historical run is selected, display its result_payload rather than current mutation data
  const result = selectedHistoryRun ? selectedHistoryRun.result_payload : mutation.data;

  const availableRegimes = useMemo(() => {
    if (!result?.results) return [];
    return Array.from(new Set(result.results.map((r) => r.regime).filter(Boolean)));
  }, [result]);

  const availableSignalTypes = useMemo(() => {
    if (!result?.results) return [];
    return Array.from(new Set(result.results.map((r) => r.signal_type).filter(Boolean)));
  }, [result]);

  const filteredAndSortedResults = useMemo(() => {
    if (!result?.results) return [];
    let items = [...result.results];

    if (searchQuery.trim()) {
      const q = searchQuery.trim().toUpperCase();
      items = items.filter((item) => item.symbol.toUpperCase().includes(q));
    }

    if (signalFilter !== 'ALL') {
      items = items.filter((item) => item.signal_type.toUpperCase() === signalFilter.toUpperCase());
    }

    if (regimeFilter !== 'ALL') {
      items = items.filter((item) => item.regime === regimeFilter);
    }

    items.sort((a, b) => {
      let comparison = 0;
      if (sortField === 'symbol') {
        comparison = a.symbol.localeCompare(b.symbol);
      } else if (sortField === 'timestamp') {
        comparison = new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime();
      } else if (sortField === 'signal_type') {
        comparison = a.signal_type.localeCompare(b.signal_type);
      } else if (sortField === 'regime') {
        comparison = a.regime.localeCompare(b.regime);
      } else if (sortField === 'price') {
        comparison = a.price - b.price;
      }
      return sortDirection === 'asc' ? comparison : -comparison;
    });

    return items;
  }, [result, searchQuery, signalFilter, regimeFilter, sortField, sortDirection]);

  const handleSort = (field: 'symbol' | 'timestamp' | 'signal_type' | 'regime' | 'price') => {
    if (sortField === field) {
      setSortDirection((prev) => (prev === 'asc' ? 'desc' : 'asc'));
    } else {
      setSortField(field);
      setSortDirection('asc');
    }
  };

  const getSortIcon = (field: string) => {
    if (sortField !== field) return ' ↕';
    return sortDirection === 'asc' ? ' ▲' : ' ▼';
  };

  const handleReplaySignal = async (item: ScannerResult, replayIntent: ReplayIntent = 'blind_practice') => {
    setReplayError(null);
    try {
      const response = await replayMutation.mutateAsync({
        symbol: item.symbol,
        signal_timestamp: item.timestamp,
        signal_type: item.signal_type,
        strategy: item.strategy,
        price: item.price,
        regime: item.regime,
        lookback_days: 120,
        forward_days: 90,
        replay_intent: replayIntent,
      });
      setSession(response.session.id);
      navigate('/replay');
    } catch (error) {
      setReplayError(error instanceof Error ? error.message : 'Could not create replay session.');
    }
  };

  return (
    <div className="animate-fade-in" style={{ maxWidth: '1200px', margin: '0 auto', paddingBottom: '40px' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '2rem' }}>
        <h2 style={{ margin: 0, fontSize: '28px', background: 'linear-gradient(90deg, #fff, #8B949E)', WebkitBackgroundClip: 'text', WebkitTextFillColor: 'transparent' }}>
          Signal Scanner
        </h2>
      </div>

      <div className="glass-panel" style={{ padding: '18px', marginBottom: '24px' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', gap: '12px', alignItems: 'center', marginBottom: '12px' }}>
          <h3 style={{ margin: 0, fontSize: '16px' }}>Scan History</h3>
          <span style={{ color: 'var(--text-muted)', fontSize: '12px' }}>{scannerRuns?.length || 0} saved</span>
        </div>
        {!scannerRuns?.length ? (
          <p style={{ color: 'var(--text-muted)', fontSize: '14px', margin: 0 }}>No saved scan runs yet.</p>
        ) : (
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(220px, 1fr))', gap: '10px' }}>
            {scannerRuns.map((run) => (
              <button
                key={run.id}
                type="button"
                data-testid={`history-run-${run.id}`}
                onClick={() => {
                  setSelectedHistoryRun(run);
                  mutation.reset();
                }}
                style={{
                  textAlign: 'left',
                  padding: '10px',
                  border: selectedHistoryRun?.id === run.id ? '1px solid var(--color-primary)' : '1px solid var(--border-color)',
                  background: selectedHistoryRun?.id === run.id ? 'rgba(41, 98, 255, 0.12)' : 'rgba(255,255,255,0.02)',
                }}
              >
                <div style={{ display: 'flex', justifyContent: 'space-between', gap: '8px', marginBottom: '6px' }}>
                  <strong style={{ fontSize: '13px' }}>{run.label}</strong>
                  <span style={{ color: run.status === 'failed' ? 'var(--color-sell)' : 'var(--color-buy)', fontSize: '11px', textTransform: 'uppercase' }}>{run.status}</span>
                </div>
                <div style={{ color: 'var(--text-muted)', fontSize: '12px' }}>
                  {run.total_results} results - {new Date(run.created_at).toLocaleString()}
                </div>
              </button>
            ))}
          </div>
        )}
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
            <select id="scanner-strategy" value={effectiveStrategyFilename} onChange={event => setSelectedStrategyFilename(event.target.value)} style={{ width: '100%', padding: '10px' }} required>
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

      {replayError && (
        <div className="glass-panel" style={{ borderColor: 'var(--color-sell)', padding: '16px', marginBottom: '24px' }}>
          <p style={{ color: 'var(--color-sell)', margin: 0 }}>{replayError}</p>
        </div>
      )}

      {result?.status === 'succeeded' && (
        <div className="glass-panel" style={{ padding: '20px' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '16px' }}>
            <h3 style={{ margin: 0, fontSize: '16px' }}>
              Signals{selectedHistoryRun ? ` from Run #${selectedHistoryRun.id}` : ''}
            </h3>
            <span style={{ color: 'var(--text-muted)', fontSize: '13px' }}>{result.total_results || 0} results{result.truncated ? ' (truncated)' : ''}</span>
          </div>
          <p data-testid="scanner-ranking-policy" style={{ margin: '0 0 16px', color: 'var(--text-muted)', fontSize: '12px' }}>
            Quantitative ranking: unavailable — scanner signals are shown chronologically until outcomes are validated.
          </p>
          {/* SQA-005: Filter and Search Bar */}
          {result.results.length > 0 && (
            <div
              data-testid="scanner-filter-bar"
              style={{
                display: 'flex',
                gap: '12px',
                alignItems: 'center',
                flexWrap: 'wrap',
                marginBottom: '16px',
                padding: '12px',
                background: 'rgba(255, 255, 255, 0.03)',
                borderRadius: '6px',
                border: '1px solid var(--border-color)',
              }}
            >
              <div style={{ flex: '1 1 200px' }}>
                <input
                  type="text"
                  data-testid="scanner-filter-input"
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  placeholder="Lọc theo mã cổ phiếu (ví dụ: FPT)..."
                  style={{
                    width: '100%',
                    padding: '8px 12px',
                    fontSize: '13px',
                    background: 'rgba(0,0,0,0.2)',
                    border: '1px solid var(--border-color)',
                    color: 'var(--text-main)',
                    borderRadius: '4px',
                  }}
                />
              </div>
              <div>
                <select
                  data-testid="scanner-signal-filter"
                  value={signalFilter}
                  onChange={(e) => setSignalFilter(e.target.value)}
                  style={{
                    padding: '8px 12px',
                    fontSize: '13px',
                    background: '#1c1f26',
                    border: '1px solid var(--border-color)',
                    color: 'var(--text-main)',
                    borderRadius: '4px',
                  }}
                >
                  <option value="ALL">Tất cả tín hiệu</option>
                  {availableSignalTypes.map((sig) => (
                    <option key={sig} value={sig}>
                      {sig.toUpperCase()}
                    </option>
                  ))}
                </select>
              </div>
              <div>
                <select
                  data-testid="scanner-regime-filter"
                  value={regimeFilter}
                  onChange={(e) => setRegimeFilter(e.target.value)}
                  style={{
                    padding: '8px 12px',
                    fontSize: '13px',
                    background: '#1c1f26',
                    border: '1px solid var(--border-color)',
                    color: 'var(--text-main)',
                    borderRadius: '4px',
                  }}
                >
                  <option value="ALL">Tất cả trạng thái (Regime)</option>
                  {availableRegimes.map((reg) => (
                    <option key={reg} value={reg}>
                      {reg}
                    </option>
                  ))}
                </select>
              </div>
              <div
                data-testid="scanner-results-count"
                style={{ fontSize: '12px', color: 'var(--text-muted)', marginLeft: 'auto' }}
              >
                Hiển thị {filteredAndSortedResults.length} / {result.results.length} tín hiệu
              </div>
            </div>
          )}

          {result.results.length > 0 ? (
            filteredAndSortedResults.length > 0 ? (
              <div style={{ overflowX: 'auto' }}>
                <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '13px' }}>
                  <thead>
                    <tr style={{ borderBottom: '1px solid rgba(255,255,255,0.1)', color: 'var(--text-muted)', textAlign: 'left' }}>
                      <th
                        data-testid="th-sort-symbol"
                        onClick={() => handleSort('symbol')}
                        style={{ padding: '8px', fontWeight: 500, cursor: 'pointer', userSelect: 'none' }}
                        title="Sắp xếp theo mã"
                      >
                        Symbol{getSortIcon('symbol')}
                      </th>
                      <th
                        data-testid="th-sort-date"
                        onClick={() => handleSort('timestamp')}
                        style={{ padding: '8px', fontWeight: 500, cursor: 'pointer', userSelect: 'none' }}
                        title="Sắp xếp theo ngày"
                      >
                        Date{getSortIcon('timestamp')}
                      </th>
                      <th
                        data-testid="th-sort-signal"
                        onClick={() => handleSort('signal_type')}
                        style={{ padding: '8px', fontWeight: 500, cursor: 'pointer', userSelect: 'none' }}
                        title="Sắp xếp theo tín hiệu"
                      >
                        Signal{getSortIcon('signal_type')}
                      </th>
                      <th
                        data-testid="th-sort-regime"
                        onClick={() => handleSort('regime')}
                        style={{ padding: '8px', fontWeight: 500, cursor: 'pointer', userSelect: 'none' }}
                        title="Sắp xếp theo trạng thái thị trường"
                      >
                        Regime{getSortIcon('regime')}
                      </th>
                      <th
                        data-testid="th-sort-price"
                        onClick={() => handleSort('price')}
                        style={{ padding: '8px', fontWeight: 500, textAlign: 'right', cursor: 'pointer', userSelect: 'none' }}
                        title="Sắp xếp theo giá"
                      >
                        Price{getSortIcon('price')}
                      </th>
                      <th style={{ padding: '8px', fontWeight: 500 }}>Ranking</th>
                      <th style={{ padding: '8px', fontWeight: 500, textAlign: 'right' }}>Replay</th>
                    </tr>
                  </thead>
                  <tbody>
                    {filteredAndSortedResults.map((item, index) => (
                      <tr key={`${item.symbol}-${item.timestamp}-${index}`} style={{ borderBottom: '1px solid rgba(255,255,255,0.05)' }}>
                        <td style={{ padding: '12px 8px', fontWeight: 600 }}>{item.symbol}</td>
                        <td style={{ padding: '12px 8px', color: 'var(--text-muted)' }}>{new Date(item.timestamp).toLocaleDateString()}</td>
                        <td style={{ padding: '12px 8px', textTransform: 'uppercase' }}>{item.signal_type}</td>
                        <td style={{ padding: '12px 8px' }}>{item.regime}</td>
                        <td style={{ padding: '12px 8px', textAlign: 'right', fontFamily: 'monospace' }}>{formatMoney(item.price)}</td>
                        <td style={{ padding: '12px 8px', color: 'var(--text-muted)' }}>Not rankable</td>
                        <td style={{ padding: '12px 8px', textAlign: 'right' }}>
                          <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '6px' }}>
                            <button
                              type="button"
                              onClick={() => handleReplaySignal(item, 'blind_practice')}
                              disabled={replayMutation.isPending}
                              aria-label={`Start blind practice for ${item.symbol} (recommended)`}
                              style={{ padding: '6px 10px', fontSize: '12px' }}
                            >
                              Blind practice
                            </button>
                            <button
                              type="button"
                              onClick={() => handleReplaySignal(item, 'signal_review')}
                              disabled={replayMutation.isPending}
                              aria-label={`Review signal for ${item.symbol}`}
                              style={{ padding: '6px 10px', fontSize: '12px' }}
                            >
                              Review signal
                            </button>
                          </div>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            ) : (
              <p data-testid="no-filtered-results" style={{ color: 'var(--text-muted)', fontSize: '14px', margin: '16px 0 0' }}>
                Không tìm thấy tín hiệu nào khớp với bộ lọc.
              </p>
            )
          ) : (
            <p style={{ color: 'var(--text-muted)', fontSize: '14px', margin: 0 }}>No signals found.</p>
          )}
        </div>
      )}
    </div>
  );
};
