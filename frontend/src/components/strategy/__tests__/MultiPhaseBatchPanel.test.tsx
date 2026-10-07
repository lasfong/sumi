import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { MultiPhaseBatchPanel } from '../MultiPhaseBatchPanel';
import * as backtestApi from '../../../api/backtestApi';
import * as universeApi from '../../../api/universeApi';
import type { BatchBacktestResponse } from '../../../api/backtestApi';
import '@testing-library/jest-dom';

vi.mock('../../../api/backtestApi', () => ({
  getAvailableStrategies: vi.fn(),
  runBatchBacktest: vi.fn(),
}));

vi.mock('../../../api/universeApi', () => ({
  listUniverses: vi.fn(),
}));

const mockBatchResponse: BatchBacktestResponse = {
  status: 'succeeded',
  total_symbols: 2,
  total_phases: 2,
  total_runs: 4,
  feature_compute_count: 2,
  simulation_run_count: 4,
  summary: { overall_profit: 25000000 },
  csv_export: 'ticker,phase,return_pct\nFPT,In-Sample,15.5\nSSI,In-Sample,8.2\n',
  metric_matrix: {
    phase_names: ['In-Sample (2020-2022)', 'Out-of-Sample (2023-2024)'],
    symbols: ['FPT', 'SSI'],
    rows_by_symbol: {
      FPT: {
        'In-Sample (2020-2022)': {
          ticker: 'FPT',
          initial_cash: 100000000,
          final_cash: 120000000,
          final_equity: 120000000,
          net_profit: 20000000,
          net_profit_pct: 20.0,
          num_trades: 12,
          win_rate_pct: 60.0,
          max_drawdown: 10.5,
          num_winners: 7,
          num_losers: 5,
          num_breakeven: 0,
          open_position_quantity: 0,
          open_position_value: 0,
        },
        'Out-of-Sample (2023-2024)': {
          ticker: 'FPT',
          initial_cash: 100000000,
          final_cash: 112000000,
          final_equity: 112000000,
          net_profit: 12000000,
          net_profit_pct: 12.0,
          num_trades: 8,
          win_rate_pct: 55.0,
          max_drawdown: 8.0,
          num_winners: 4,
          num_losers: 4,
          num_breakeven: 0,
          open_position_quantity: 0,
          open_position_value: 0,
        },
      },
      SSI: {
        'In-Sample (2020-2022)': {
          ticker: 'SSI',
          initial_cash: 100000000,
          final_cash: 115000000,
          final_equity: 115000000,
          net_profit: 15000000,
          net_profit_pct: 15.0,
          num_trades: 15,
          win_rate_pct: 53.3,
          max_drawdown: 14.2,
          num_winners: 8,
          num_losers: 7,
          num_breakeven: 0,
          open_position_quantity: 0,
          open_position_value: 0,
        },
        'Out-of-Sample (2023-2024)': {
          ticker: 'SSI',
          initial_cash: 100000000,
          final_cash: 105000000,
          final_equity: 105000000,
          net_profit: 5000000,
          net_profit_pct: 5.0,
          num_trades: 6,
          win_rate_pct: 50.0,
          max_drawdown: 12.0,
          num_winners: 3,
          num_losers: 3,
          num_breakeven: 0,
          open_position_quantity: 0,
          open_position_value: 0,
        },
      },
    },
    portfolio_by_phase: {
      'In-Sample (2020-2022)': {
        ticker: 'PORTFOLIO',
        initial_cash: 200000000,
        final_cash: 235000000,
        final_equity: 235000000,
        net_profit: 35000000,
        net_profit_pct: 17.5,
        num_trades: 27,
        win_rate_pct: 56.6,
        max_drawdown: 11.2,
        num_winners: 15,
        num_losers: 12,
        num_breakeven: 0,
        open_position_quantity: 0,
        open_position_value: 0,
      },
      'Out-of-Sample (2023-2024)': {
        ticker: 'PORTFOLIO',
        initial_cash: 200000000,
        final_cash: 217000000,
        final_equity: 217000000,
        net_profit: 17000000,
        net_profit_pct: 8.5,
        num_trades: 14,
        win_rate_pct: 52.5,
        max_drawdown: 9.5,
        num_winners: 7,
        num_losers: 7,
        num_breakeven: 0,
        open_position_quantity: 0,
        open_position_value: 0,
      },
    },
    cross_phase_degradations: [
      {
        ticker: 'FPT',
        base_phase_name: 'In-Sample (2020-2022)',
        target_phase_name: 'Out-of-Sample (2023-2024)',
        net_profit_delta: -8000000,
        return_delta_pct: -8.0,
        win_rate_delta_pct: -5.0,
        profit_ratio: 0.6,
        degradation_pct: 40.0,
        is_degraded: false,
        drawdown_delta: -2.5,
      },
      {
        ticker: 'SSI',
        base_phase_name: 'In-Sample (2020-2022)',
        target_phase_name: 'Out-of-Sample (2023-2024)',
        net_profit_delta: -10000000,
        return_delta_pct: -10.0,
        win_rate_delta_pct: -3.3,
        profit_ratio: 0.33,
        degradation_pct: 66.7,
        is_degraded: true,
        drawdown_delta: -2.2,
      },
    ],
    consistency_score: 78.5,
  },
  phase_results: [],
};

const renderWithClient = (ui: React.ReactElement) => {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(<QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>);
};

describe('MultiPhaseBatchPanel Component (FR-CORE-010, FR-CORE-012, Master F.5)', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(backtestApi.getAvailableStrategies).mockResolvedValue([
      {
        filename: 'trend.yaml',
        name: 'Chiến lược Trend Following',
        description: 'Trend following',
        config: { name: 'Trend Following', indicators: [] },
      },
    ]);
    vi.mocked(universeApi.listUniverses).mockResolvedValue([
      {
        universe_id: 'VN30',
        version: '1.0',
        name: 'VN30 Index',
        status: 'ACTIVE',
        default_mode: 'POINT_IN_TIME',
        selection_purpose: 'Bluechips',
        source_evidence: 'HOSE',
        total_member_count: 30,
        active_member_count: 30,
        content_hash: 'hashvn30',
      },
    ]);
    vi.mocked(backtestApi.runBatchBacktest).mockResolvedValue(mockBatchResponse);
  });

  it('renders phase editor and allows adding new phases (FR-CORE-010)', async () => {
    renderWithClient(<MultiPhaseBatchPanel />);

    await waitFor(() => {
      expect(screen.getByTestId('multi-phase-batch-panel')).toBeInTheDocument();
    });

    expect(screen.getByTestId('phase-row-0')).toBeInTheDocument();
    expect(screen.getByTestId('phase-row-1')).toBeInTheDocument();

    // Click Add Phase
    const addPhaseBtn = screen.getByTestId('add-phase-btn');
    fireEvent.click(addPhaseBtn);

    expect(screen.getByTestId('phase-row-2')).toBeInTheDocument();
  });

  it('selects VN30 universe and auto-populates symbol list', async () => {
    renderWithClient(<MultiPhaseBatchPanel />);

    await waitFor(() => {
      expect(screen.getByTestId('batch-universe-select')).toBeInTheDocument();
    });

    const universeSelect = screen.getByTestId('batch-universe-select');
    fireEvent.change(universeSelect, { target: { value: 'VN30' } });

    const symbolsInput = screen.getByTestId('batch-symbols-input') as HTMLInputElement;
    expect(symbolsInput.value).toContain('ACB');
    expect(symbolsInput.value).toContain('FPT');
    expect(symbolsInput.value).toContain('VNM');
  });

  it('runs batch backtest and displays PhaseMetricMatrix and Degradation Table (FR-CORE-010, FR-CORE-012)', async () => {
    renderWithClient(<MultiPhaseBatchPanel />);

    await waitFor(() => {
      expect(screen.getByText(/Chiến lược Trend Following/i)).toBeInTheDocument();
    });

    const runBtn = screen.getByTestId('run-batch-btn');
    fireEvent.submit(runBtn.closest('form')!);

    await waitFor(() => {
      expect(screen.getByTestId('batch-results-container')).toBeInTheDocument();
      expect(screen.getByTestId('consistency-score')).toHaveTextContent('78.5 / 100');
    });

    // Verify 2D Matrix Table
    const matrixTable = screen.getByTestId('phase-metric-matrix-table');
    expect(matrixTable).toHaveTextContent('DANH MỤC TỔNG (PORTFOLIO)');
    expect(matrixTable).toHaveTextContent('+17.50%');
    expect(matrixTable).toHaveTextContent('+8.50%');
    expect(matrixTable).toHaveTextContent('FPT');
    expect(matrixTable).toHaveTextContent('+20.00%');
    expect(matrixTable).toHaveTextContent('SSI');

    // Verify Cross-Phase Degradation Table
    const degTable = screen.getByTestId('cross-phase-degradation-table');
    expect(degTable).toHaveTextContent('ỔN ĐỊNH ✓');
    expect(degTable).toHaveTextContent('SUY GIẢM MẠNH ⚠️');

    // Verify Master F.5 Warnings & Data Quality panel
    const warningsPanel = screen.getByTestId('batch-warnings-panel');
    expect(warningsPanel).toHaveTextContent('OHLCV_PROXY');
    expect(warningsPanel).toHaveTextContent('T+2.5/T+2');
    expect(warningsPanel).toHaveTextContent('Zero Lookahead');
  });

  it('validates default initial cash (100M VND) without HTML5 stepMismatch error (ST-02)', async () => {
    renderWithClient(<MultiPhaseBatchPanel />);

    await waitFor(() => {
      expect(screen.getByTestId('batch-initial-cash-input')).toBeInTheDocument();
    });

    const cashInput = screen.getByTestId('batch-initial-cash-input') as HTMLInputElement;
    expect(cashInput.value).toBe('100000000');
    expect(cashInput.validity.stepMismatch).toBe(false);
    expect(cashInput.validity.valid).toBe(true);

    const form = cashInput.closest('form');
    expect(form).not.toBeNull();
    expect(form?.checkValidity()).toBe(true);
  });
});
