import React, { useState, useMemo } from 'react';
import { useQuery } from '@tanstack/react-query';
import {
  getAvailableStrategies,
  runBatchBacktest,
  type AvailableStrategy,
  type BatchBacktestRequest,
  type BatchBacktestResponse,
  type PhaseDefinitionRequest,
  type BenchmarkMetricRowResponse,
} from '../../api/backtestApi';
import { listUniverses, type UniverseSummary } from '../../api/universeApi';

export interface MultiPhaseBatchPanelProps {
  className?: string;
}

const DEFAULT_PHASES: PhaseDefinitionRequest[] = [
  {
    name: 'In-Sample (2020-2022)',
    start_date: '2020-01-01',
    end_date: '2022-12-31',
    description: 'Pha tối ưu mẫu ban đầu (sóng tăng mạnh & điều chỉnh lớn)',
  },
  {
    name: 'Out-of-Sample (2023-2024)',
    start_date: '2023-01-01',
    end_date: '2024-12-31',
    description: 'Pha kiểm định ngoài mẫu (thị trường phục hồi phân hóa)',
  },
];

export const MultiPhaseBatchPanel: React.FC<MultiPhaseBatchPanelProps> = ({ className = '' }) => {
  const [symbolsInput, setSymbolsInput] = useState<string>('FPT, SSI, HPG, VCI');
  const [selectedUniverseId, setSelectedUniverseId] = useState<string>('custom');
  const [selectedStrategyFilename, setSelectedStrategyFilename] = useState<string>('');
  const [initialCash, setInitialCash] = useState<number>(100000000);
  const [benchmarkSymbol, setBenchmarkSymbol] = useState<string>('VNINDEX');
  const [phases, setPhases] = useState<PhaseDefinitionRequest[]>(DEFAULT_PHASES);
  const [useCache, setUseCache] = useState<boolean>(true);

  const [isRunning, setIsRunning] = useState<boolean>(false);
  const [batchResponse, setBatchResponse] = useState<BatchBacktestResponse | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  // Queries
  const { data: strategies } = useQuery({
    queryKey: ['strategies'],
    queryFn: getAvailableStrategies,
  });

  const { data: universes } = useQuery({
    queryKey: ['universes-list'],
    queryFn: listUniverses,
  });

  // Effective strategy
  const activeStrategy = useMemo(() => {
    if (!strategies || strategies.length === 0) return null;
    if (selectedStrategyFilename) {
      const match = strategies.find((s) => s.filename === selectedStrategyFilename);
      if (match) return match;
    }
    return strategies[0];
  }, [strategies, selectedStrategyFilename]);

  // Handle Universe selection
  const handleUniverseChange = (universeId: string) => {
    setSelectedUniverseId(universeId);
    if (universeId === 'custom') return;
    if (universeId === 'VN30') {
      setSymbolsInput('ACB, BID, BVH, CTG, FPT, GAS, GVR, HDB, HPG, MBB, MSN, MWG, PLX, POW, SAB, SHB, SSI, STB, TCB, TPB, VCB, VHM, VIB, VIC, VJC, VNM, VPB, VRE');
    } else if (universeId === 'HOSE50') {
      setSymbolsInput('FPT, SSI, HPG, VCI, TCB, MWG, MBB, DGC, VND, KDH, NLG, PVD, PVS, VHC, REE');
    }
  };

  // Phase editing
  const handlePhaseChange = (index: number, field: keyof PhaseDefinitionRequest, val: string) => {
    setPhases((prev) => {
      const next = [...prev];
      next[index] = { ...next[index], [field]: val };
      return next;
    });
  };

  const handleAddPhase = () => {
    setPhases((prev) => [
      ...prev,
      {
        name: `Phase ${prev.length + 1}`,
        start_date: '2025-01-01',
        end_date: '2025-12-31',
        description: '',
      },
    ]);
  };

  const handleRemovePhase = (index: number) => {
    if (phases.length <= 1) return;
    setPhases((prev) => prev.filter((_, i) => i !== index));
  };

  // Execute Batch
  const handleRunBatch = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!activeStrategy) {
      setErrorMessage('Vui lòng chọn một chiến lược cần kiểm định.');
      return;
    }

    const symbols = symbolsInput
      .split(',')
      .map((s) => s.trim().toUpperCase())
      .filter(Boolean);

    if (symbols.length === 0) {
      setErrorMessage('Danh sách mã cổ phiếu không được rỗng.');
      return;
    }

    // Validate phase dates
    for (const p of phases) {
      if (p.start_date >= p.end_date) {
        setErrorMessage(`Pha "${p.name}": Ngày bắt đầu (${p.start_date}) phải trước ngày kết thúc (${p.end_date}).`);
        return;
      }
    }

    setIsRunning(true);
    setErrorMessage(null);

    const req: BatchBacktestRequest = {
      symbols,
      phases,
      strategy: activeStrategy.config,
      initial_cash: initialCash,
      benchmark_symbol: benchmarkSymbol.trim().toUpperCase() || 'VNINDEX',
      execution_profile: 'vietnam_default_conservative',
      exchange: 'HOSE',
      use_cache: useCache,
    };

    try {
      const res = await runBatchBacktest(req);
      setBatchResponse(res);
    } catch (err) {
      setErrorMessage(err instanceof Error ? err.message : 'Batch backtest execution failed');
    } finally {
      setIsRunning(false);
    }
  };

  // CSV Download (FR-CORE-012)
  const handleDownloadCsv = () => {
    if (!batchResponse?.csv_export) return;
    const blob = new Blob([batchResponse.csv_export], { type: 'text/csv;charset=utf-8;' });
    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.setAttribute('download', `sumi_batch_matrix_${Date.now()}.csv`);
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
  };

  const formatPct = (val?: number | null) => {
    if (val === null || val === undefined) return 'N/A';
    const prefix = val > 0 ? '+' : '';
    return `${prefix}${val.toFixed(2)}%`;
  };

  const formatNum = (val?: number | null, decimals = 2) => {
    if (val === null || val === undefined) return 'N/A';
    return val.toFixed(decimals);
  };

  const matrix = batchResponse?.metric_matrix;

  return (
    <div className={`multi-phase-batch-panel ${className}`} data-testid="multi-phase-batch-panel">
      {/* Header */}
      <div style={{ marginBottom: '20px' }}>
        <h3 style={{ margin: '0 0 6px 0', fontSize: '18px', fontWeight: 600 }}>
          📊 Kiểm Định Đa Pha & Ma Trận Xu Hướng Suy Giảm (Multi-Phase Degradation Matrix)
        </h3>
        <p style={{ margin: 0, fontSize: '13px', color: 'var(--text-muted)' }}>
          Thực thi kiểm định ma trận đồng thời trên nhiều pha thị trường (In-Sample, Out-of-Sample, chu kỳ bull/bear) với bộ nhớ đệm tính toán một lần (Compute-Once Cache) và vốn độc lập.
        </p>
      </div>

      {/* Control Form */}
      <div className="glass-panel" style={{ padding: '20px', borderRadius: '8px', marginBottom: '20px' }}>
        <form onSubmit={handleRunBatch}>
          {/* Row 1: Strategy & Universe */}
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(280px, 1fr))', gap: '16px', marginBottom: '16px' }}>
            <div>
              <label htmlFor="batch-strategy-select" style={{ display: 'block', fontSize: '12px', color: 'var(--text-muted)', marginBottom: '4px' }}>
                Chiến lược (Strategy):
              </label>
              <select
                id="batch-strategy-select"
                data-testid="batch-strategy-select"
                value={activeStrategy?.filename || ''}
                onChange={(e) => setSelectedStrategyFilename(e.target.value)}
                style={{
                  width: '100%',
                  padding: '8px 12px',
                  borderRadius: '4px',
                  background: 'rgba(255, 255, 255, 0.05)',
                  border: '1px solid var(--border-color)',
                  color: 'var(--text-main)',
                  fontSize: '13px',
                }}
              >
                {(strategies || []).map((st: AvailableStrategy) => (
                  <option key={st.filename} value={st.filename} style={{ background: '#1c1f26', color: '#fff' }}>
                    {st.name} ({st.filename})
                  </option>
                ))}
              </select>
            </div>

            <div>
              <label htmlFor="batch-universe-select" style={{ display: 'block', fontSize: '12px', color: 'var(--text-muted)', marginBottom: '4px' }}>
                Chọn Vũ Trụ Cổ Phiếu (Universe):
              </label>
              <select
                id="batch-universe-select"
                data-testid="batch-universe-select"
                value={selectedUniverseId}
                onChange={(e) => handleUniverseChange(e.target.value)}
                style={{
                  width: '100%',
                  padding: '8px 12px',
                  borderRadius: '4px',
                  background: 'rgba(255, 255, 255, 0.05)',
                  border: '1px solid var(--border-color)',
                  color: 'var(--text-main)',
                  fontSize: '13px',
                }}
              >
                <option value="custom" style={{ background: '#1c1f26', color: '#fff' }}>Tự nhập danh sách mã</option>
                <option value="VN30" style={{ background: '#1c1f26', color: '#fff' }}>VN30 (30 Cổ phiếu vốn hóa lớn HOSE)</option>
                <option value="HOSE50" style={{ background: '#1c1f26', color: '#fff' }}>HOSE50 (50 Cổ phiếu thanh khoản hàng đầu)</option>
                {(universes || [])
                  .filter((u: UniverseSummary) => !['VN30', 'HOSE50'].includes(u.universe_id))
                  .map((u: UniverseSummary) => (
                    <option key={u.universe_id} value={u.universe_id} style={{ background: '#1c1f26', color: '#fff' }}>
                      {u.name} ({u.active_member_count} mã)
                    </option>
                  ))}
              </select>
            </div>
          </div>

          {/* Row 2: Symbols Input */}
          <div style={{ marginBottom: '16px' }}>
            <label htmlFor="batch-symbols-input" style={{ display: 'block', fontSize: '12px', color: 'var(--text-muted)', marginBottom: '4px' }}>
              Danh sách Ticker (cách nhau bởi dấu phẩy):
            </label>
            <input
              id="batch-symbols-input"
              data-testid="batch-symbols-input"
              type="text"
              value={symbolsInput}
              onChange={(e) => setSymbolsInput(e.target.value.toUpperCase())}
              style={{
                width: '100%',
                padding: '8px 12px',
                borderRadius: '4px',
                background: 'rgba(255, 255, 255, 0.05)',
                border: '1px solid var(--border-color)',
                color: 'var(--text-main)',
                fontSize: '13px',
              }}
              required
            />
          </div>

          {/* Row 3: Phase Definitions Editor (FR-CORE-010) */}
          <div style={{ marginBottom: '16px' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '8px' }}>
              <span style={{ fontSize: '12px', fontWeight: 600, color: 'var(--text-muted)' }}>
                Định Nghĩa Các Pha Kiểm Định (Phases):
              </span>
              <button
                type="button"
                data-testid="add-phase-btn"
                onClick={handleAddPhase}
                style={{
                  padding: '4px 8px',
                  fontSize: '11px',
                  borderRadius: '4px',
                  background: 'rgba(41, 98, 255, 0.2)',
                  color: 'var(--color-primary)',
                  border: '1px solid var(--color-primary)',
                  cursor: 'pointer',
                }}
              >
                + Thêm Pha
              </button>
            </div>

            <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
              {phases.map((ph, idx) => (
                <div
                  key={idx}
                  data-testid={`phase-row-${idx}`}
                  style={{
                    display: 'grid',
                    gridTemplateColumns: '1.5fr 1fr 1fr 32px',
                    gap: '8px',
                    alignItems: 'center',
                    background: 'rgba(255, 255, 255, 0.02)',
                    padding: '8px',
                    borderRadius: '4px',
                    border: '1px solid rgba(255, 255, 255, 0.06)',
                  }}
                >
                  <input
                    type="text"
                    value={ph.name}
                    placeholder="Tên pha (ví dụ: In-Sample 2020-2022)"
                    onChange={(e) => handlePhaseChange(idx, 'name', e.target.value)}
                    style={{
                      padding: '6px 8px',
                      fontSize: '12px',
                      background: 'rgba(255, 255, 255, 0.05)',
                      border: '1px solid var(--border-color)',
                      color: 'var(--text-main)',
                      borderRadius: '4px',
                    }}
                    required
                  />
                  <input
                    type="date"
                    value={ph.start_date}
                    onChange={(e) => handlePhaseChange(idx, 'start_date', e.target.value)}
                    style={{
                      padding: '6px 8px',
                      fontSize: '12px',
                      background: 'rgba(255, 255, 255, 0.05)',
                      border: '1px solid var(--border-color)',
                      color: 'var(--text-main)',
                      borderRadius: '4px',
                    }}
                    required
                  />
                  <input
                    type="date"
                    value={ph.end_date}
                    onChange={(e) => handlePhaseChange(idx, 'end_date', e.target.value)}
                    style={{
                      padding: '6px 8px',
                      fontSize: '12px',
                      background: 'rgba(255, 255, 255, 0.05)',
                      border: '1px solid var(--border-color)',
                      color: 'var(--text-main)',
                      borderRadius: '4px',
                    }}
                    required
                  />
                  <button
                    type="button"
                    disabled={phases.length <= 1}
                    onClick={() => handleRemovePhase(idx)}
                    style={{
                      padding: '6px',
                      background: 'transparent',
                      border: 'none',
                      color: phases.length <= 1 ? 'gray' : 'var(--color-sell)',
                      cursor: phases.length <= 1 ? 'not-allowed' : 'pointer',
                      fontSize: '14px',
                    }}
                    title="Xóa pha"
                  >
                    ✕
                  </button>
                </div>
              ))}
            </div>
          </div>

          {/* Row 4: Financial Controls & Run Button */}
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', flexWrap: 'wrap', gap: '12px' }}>
            <div style={{ display: 'flex', gap: '16px', alignItems: 'center' }}>
              <div>
                <label htmlFor="batch-initial-cash" style={{ fontSize: '11px', color: 'var(--text-muted)', marginRight: '6px' }}>Vốn mỗi mã:</label>
                <input
                  id="batch-initial-cash"
                  type="number"
                  value={initialCash}
                  onChange={(e) => setInitialCash(Number(e.target.value))}
                  step="10000000"
                  min="1000000"
                  style={{
                    padding: '6px 10px',
                    fontSize: '12px',
                    width: '130px',
                    borderRadius: '4px',
                    background: 'rgba(255, 255, 255, 0.05)',
                    border: '1px solid var(--border-color)',
                    color: 'var(--text-main)',
                  }}
                />
              </div>
              <div>
                <label htmlFor="batch-benchmark-symbol" style={{ fontSize: '11px', color: 'var(--text-muted)', marginRight: '6px' }}>Benchmark:</label>
                <input
                  id="batch-benchmark-symbol"
                  type="text"
                  value={benchmarkSymbol}
                  onChange={(e) => setBenchmarkSymbol(e.target.value.toUpperCase())}
                  style={{
                    padding: '6px 10px',
                    fontSize: '12px',
                    width: '100px',
                    borderRadius: '4px',
                    background: 'rgba(255, 255, 255, 0.05)',
                    border: '1px solid var(--border-color)',
                    color: 'var(--text-main)',
                  }}
                />
              </div>
              <label style={{ display: 'flex', alignItems: 'center', gap: '6px', fontSize: '12px', color: 'var(--text-muted)', cursor: 'pointer' }}>
                <input
                  type="checkbox"
                  data-testid="batch-use-cache-toggle"
                  checked={useCache}
                  onChange={(e) => setUseCache(e.target.checked)}
                />
                Dùng Domain Cache
              </label>
            </div>

            <button
              type="submit"
              data-testid="run-batch-btn"
              disabled={isRunning}
              style={{
                padding: '10px 24px',
                fontSize: '14px',
                fontWeight: 600,
                borderRadius: '6px',
                background: 'linear-gradient(135deg, #00E676, #00B0FF)',
                color: '#000',
                border: 'none',
                cursor: isRunning ? 'wait' : 'pointer',
              }}
            >
              {isRunning ? '⏳ Đang Chạy Kiểm Định Đa Pha...' : '🚀 Chạy Kiểm Định Đa Pha (Batch Backtest)'}
            </button>
          </div>
        </form>
      </div>

      {errorMessage && (
        <div
          data-testid="batch-error-message"
          style={{
            padding: '12px 16px',
            background: 'rgba(255, 23, 68, 0.1)',
            border: '1px solid var(--color-sell)',
            borderRadius: '6px',
            color: 'var(--color-sell)',
            fontSize: '13px',
            marginBottom: '20px',
          }}
        >
          {errorMessage}
        </div>
      )}

      {/* Results Section */}
      {batchResponse && (
        <div data-testid="batch-results-container">
          {/* Summary Banner & Consistency Score */}
          <div
            className="glass-panel"
            style={{
              padding: '16px',
              borderRadius: '8px',
              marginBottom: '20px',
              display: 'flex',
              justifyContent: 'space-between',
              alignItems: 'center',
              flexWrap: 'wrap',
              gap: '16px',
            }}
          >
            <div>
              <div style={{ fontSize: '12px', color: 'var(--text-muted)' }}>Tổng hợp kiểm định:</div>
              <div style={{ fontSize: '15px', fontWeight: 600, marginTop: '2px' }}>
                {batchResponse.total_symbols} Mã • {batchResponse.total_phases} Pha • {batchResponse.total_runs} Lượt Chạy • Cache compute: {batchResponse.feature_compute_count} lần
              </div>
              {batchResponse.timing_metrics && (
                <div style={{ display: 'flex', alignItems: 'center', gap: '8px', marginTop: '6px', flexWrap: 'wrap' }}>
                  <span
                    data-testid="batch-timing-badge"
                    style={{
                      display: 'inline-flex',
                      alignItems: 'center',
                      gap: '4px',
                      padding: '3px 8px',
                      borderRadius: '12px',
                      fontSize: '11px',
                      fontWeight: 600,
                      background: 'rgba(0, 230, 118, 0.12)',
                      color: 'var(--color-buy)',
                      border: '1px solid rgba(0, 230, 118, 0.25)',
                    }}
                  >
                    ⚡ {batchResponse.timing_metrics.total_duration_ms.toFixed(0)}ms
                    (Feature: {batchResponse.timing_metrics.feature_compute_ms.toFixed(0)}ms, Sim: {batchResponse.timing_metrics.simulation_ms.toFixed(0)}ms)
                    • Hits: {batchResponse.timing_metrics.cache_hits} | Misses: {batchResponse.timing_metrics.cache_misses}
                  </span>
                </div>
              )}
            </div>

            {matrix && (
              <div style={{ display: 'flex', alignItems: 'center', gap: '16px' }}>
                <div style={{ textAlign: 'right' }}>
                  <div style={{ fontSize: '11px', color: 'var(--text-muted)' }}>Điểm Tính Nhất Quán (Consistency):</div>
                  <div
                    data-testid="consistency-score"
                    style={{
                      fontSize: '20px',
                      fontWeight: 700,
                      color: matrix.consistency_score >= 70 ? 'var(--color-buy)' : matrix.consistency_score >= 40 ? '#FFD166' : 'var(--color-sell)',
                    }}
                  >
                    {matrix.consistency_score.toFixed(1)} / 100
                  </div>
                </div>

                {batchResponse.csv_export && (
                  <button
                    type="button"
                    data-testid="download-csv-btn"
                    onClick={handleDownloadCsv}
                    style={{
                      padding: '8px 14px',
                      fontSize: '12px',
                      borderRadius: '4px',
                      background: 'rgba(255, 255, 255, 0.08)',
                      border: '1px solid var(--border-color)',
                      color: 'var(--text-main)',
                      cursor: 'pointer',
                      display: 'flex',
                      alignItems: 'center',
                      gap: '6px',
                    }}
                  >
                    📥 Tải Xuất Dữ Liệu CSV
                  </button>
                )}
              </div>
            )}
          </div>

          {/* 2D Phase Metric Matrix Table (FR-CORE-010) */}
          {matrix && (
            <div className="glass-panel" style={{ padding: '20px', borderRadius: '8px', marginBottom: '20px' }}>
              <h4 style={{ margin: '0 0 12px 0', fontSize: '15px', fontWeight: 600 }}>
                Ma Trận Chỉ Số Benchmark Theo Từng Pha (Phase Benchmark Matrix)
              </h4>
              <div style={{ overflowX: 'auto' }}>
                <table
                  data-testid="phase-metric-matrix-table"
                  style={{ width: '100%', borderCollapse: 'collapse', fontSize: '12px' }}
                >
                  <thead>
                    <tr style={{ borderBottom: '1px solid rgba(255, 255, 255, 0.1)', color: 'var(--text-muted)', textAlign: 'right' }}>
                      <th style={{ textAlign: 'left', padding: '8px' }}>Mã Ticker</th>
                      {matrix.phase_names.map((pName) => (
                        <th key={pName} colSpan={3} style={{ textAlign: 'center', padding: '8px', borderLeft: '1px solid rgba(255, 255, 255, 0.08)' }}>
                          {pName}
                        </th>
                      ))}
                    </tr>
                    <tr style={{ borderBottom: '1px solid rgba(255, 255, 255, 0.06)', color: 'var(--text-muted)', fontSize: '11px', textAlign: 'right' }}>
                      <th style={{ textAlign: 'left', padding: '6px 8px' }}></th>
                      {matrix.phase_names.map((pName) => (
                        <React.Fragment key={`sub-${pName}`}>
                          <th style={{ padding: '6px 8px', borderLeft: '1px solid rgba(255, 255, 255, 0.08)' }}>Lợi Nhuận %</th>
                          <th style={{ padding: '6px 8px' }}>Win Rate %</th>
                          <th style={{ padding: '6px 8px' }}>Max DD %</th>
                        </React.Fragment>
                      ))}
                    </tr>
                  </thead>
                  <tbody>
                    {/* Portfolio Aggregate Row */}
                    {matrix.portfolio_by_phase && (
                      <tr style={{ borderBottom: '2px solid rgba(88, 166, 255, 0.3)', background: 'rgba(41, 98, 255, 0.08)', fontWeight: 600 }}>
                        <td style={{ padding: '10px 8px', color: '#58A6FF' }}>👑 DANH MỤC TỔNG (PORTFOLIO)</td>
                        {matrix.phase_names.map((pName) => {
                          const m: BenchmarkMetricRowResponse | undefined = matrix.portfolio_by_phase[pName];
                          return (
                            <React.Fragment key={`port-${pName}`}>
                              <td style={{ padding: '10px 8px', textAlign: 'right', borderLeft: '1px solid rgba(255, 255, 255, 0.08)', color: (m?.net_profit_pct ?? 0) >= 0 ? 'var(--color-buy)' : 'var(--color-sell)' }}>
                                {formatPct(m?.net_profit_pct)}
                              </td>
                              <td style={{ padding: '10px 8px', textAlign: 'right' }}>
                                {formatPct(m?.win_rate_pct)}
                              </td>
                              <td style={{ padding: '10px 8px', textAlign: 'right', color: 'var(--color-sell)' }}>
                                -{formatNum(m?.max_drawdown)}%
                              </td>
                            </React.Fragment>
                          );
                        })}
                      </tr>
                    )}

                    {/* Per-Symbol Rows */}
                    {matrix.symbols.map((sym) => (
                      <tr key={sym} style={{ borderBottom: '1px solid rgba(255, 255, 255, 0.04)' }}>
                        <td style={{ padding: '8px', fontWeight: 600 }}>{sym}</td>
                        {matrix.phase_names.map((pName) => {
                          const row: BenchmarkMetricRowResponse | undefined = matrix.rows_by_symbol[sym]?.[pName];
                          return (
                            <React.Fragment key={`${sym}-${pName}`}>
                              <td style={{ padding: '8px', textAlign: 'right', borderLeft: '1px solid rgba(255, 255, 255, 0.08)', color: (row?.net_profit_pct ?? 0) >= 0 ? 'var(--color-buy)' : 'var(--color-sell)' }}>
                                {formatPct(row?.net_profit_pct)}
                              </td>
                              <td style={{ padding: '8px', textAlign: 'right' }}>
                                {formatPct(row?.win_rate_pct)}
                              </td>
                              <td style={{ padding: '8px', textAlign: 'right', color: 'var(--color-sell)' }}>
                                -{formatNum(row?.max_drawdown)}%
                              </td>
                            </React.Fragment>
                          );
                        })}
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          )}

          {/* Cross-Phase Degradation Table (FR-CORE-012) */}
          {matrix?.cross_phase_degradations && matrix.cross_phase_degradations.length > 0 && (
            <div className="glass-panel" style={{ padding: '20px', borderRadius: '8px', marginBottom: '20px' }}>
              <h4 style={{ margin: '0 0 12px 0', fontSize: '15px', fontWeight: 600 }}>
                Kiểm Tra Suy Giảm Hiệu Năng Giữa Các Pha (Cross-Phase Degradation Matrix)
              </h4>
              <div style={{ overflowX: 'auto' }}>
                <table
                  data-testid="cross-phase-degradation-table"
                  style={{ width: '100%', borderCollapse: 'collapse', fontSize: '12px' }}
                >
                  <thead>
                    <tr style={{ borderBottom: '1px solid rgba(255, 255, 255, 0.1)', color: 'var(--text-muted)', textAlign: 'left' }}>
                      <th style={{ padding: '8px' }}>Mã Ticker</th>
                      <th style={{ padding: '8px' }}>Pha Gốc (Base)</th>
                      <th style={{ padding: '8px' }}>Pha Đích (Target)</th>
                      <th style={{ padding: '8px', textAlign: 'right' }}>Delta Lợi Nhuận %</th>
                      <th style={{ padding: '8px', textAlign: 'right' }}>Delta Win Rate %</th>
                      <th style={{ padding: '8px', textAlign: 'right' }}>Tỷ Lệ Suy Giảm %</th>
                      <th style={{ padding: '8px', textAlign: 'center' }}>Đánh Giá Suy Giảm</th>
                    </tr>
                  </thead>
                  <tbody>
                    {matrix.cross_phase_degradations.map((deg, dIdx) => (
                      <tr key={dIdx} style={{ borderBottom: '1px solid rgba(255, 255, 255, 0.04)' }}>
                        <td style={{ padding: '8px', fontWeight: 600 }}>{deg.ticker}</td>
                        <td style={{ padding: '8px', color: 'var(--text-muted)' }}>{deg.base_phase_name}</td>
                        <td style={{ padding: '8px', color: 'var(--text-muted)' }}>{deg.target_phase_name}</td>
                        <td style={{ padding: '8px', textAlign: 'right', color: deg.return_delta_pct >= 0 ? 'var(--color-buy)' : 'var(--color-sell)' }}>
                          {formatPct(deg.return_delta_pct)}
                        </td>
                        <td style={{ padding: '8px', textAlign: 'right' }}>
                          {formatPct(deg.win_rate_delta_pct)}
                        </td>
                        <td style={{ padding: '8px', textAlign: 'right' }}>
                          {deg.degradation_pct !== null && deg.degradation_pct !== undefined ? `${deg.degradation_pct.toFixed(2)}%` : 'N/A'}
                        </td>
                        <td style={{ padding: '8px', textAlign: 'center' }}>
                          {deg.is_degraded ? (
                            <span style={{ padding: '2px 6px', borderRadius: '4px', background: 'rgba(255, 23, 68, 0.2)', color: 'var(--color-sell)', fontWeight: 600 }}>
                              SUY GIẢM MẠNH ⚠️
                            </span>
                          ) : (
                            <span style={{ padding: '2px 6px', borderRadius: '4px', background: 'rgba(0, 230, 118, 0.15)', color: 'var(--color-buy)', fontWeight: 600 }}>
                              ỔN ĐỊNH ✓
                            </span>
                          )}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          )}

          {/* Master Appendix F.5 Warnings & Data-Quality Panel */}
          <div
            className="glass-panel"
            data-testid="batch-warnings-panel"
            style={{
              padding: '16px',
              borderRadius: '8px',
              background: 'rgba(0, 0, 0, 0.3)',
              border: '1px solid rgba(255, 209, 102, 0.3)',
            }}
          >
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px', marginBottom: '8px' }}>
              <span style={{ fontSize: '16px' }}>🛡️</span>
              <h5 style={{ margin: 0, fontSize: '13px', fontWeight: 600, color: '#FFD166' }}>
                Bảng Cảnh Báo & Giả Định Kiểm Định Hệ Thống (Validation & Quality Warnings)
              </h5>
            </div>
            <ul style={{ margin: 0, paddingLeft: '20px', fontSize: '12px', color: 'var(--text-muted)', lineHeight: 1.5 }}>
              <li>
                <strong>Phương pháp dòng tiền BB:</strong> Hệ thống áp dụng phương thức kiểm định <code>OHLCV_PROXY</code> (ước tính biến động giá và khối lượng). Không đại diện cho luồng lệnh khớp thực tế từ sở giao dịch.
              </li>
              <li>
                <strong>Chu kỳ thanh toán:</strong> Mặc định áp dụng thanh toán thận trọng T+2.5/T+2 theo quy định thị trường chứng khoán Việt Nam.
              </li>
              <li>
                <strong>Bảo toàn nhân quả (Zero Lookahead):</strong> Tất cả tín hiệu kỹ thuật và chỉ báo chỉ được đọc đến thời điểm đóng nến (bar close) quan sát, tuyệt đối không tính trước tương lai.
              </li>
              <li>
                <strong>Thời gian khởi động (Warmup):</strong> Các nến trong giai đoạn warmup (thường 20–50 nến đầu tiên) được dùng để làm mượt chỉ báo và không sinh lệnh thực thi.
              </li>
            </ul>
          </div>
        </div>
      )}
    </div>
  );
};
