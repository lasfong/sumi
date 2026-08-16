import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { StrategyLabPage } from '../StrategyLabPage';
import * as backtestApi from '../../api/backtestApi';
import * as strategyLabApi from '../../api/strategyLabApi';
import '@testing-library/jest-dom';

vi.mock('../../api/backtestApi', () => ({
  getAvailableStrategies: vi.fn(),
  runBacktest: vi.fn(),
}));

vi.mock('../../api/strategyLabApi', () => ({
  runParameterSweep: vi.fn(),
  cancelParameterSweep: vi.fn(),
  listStrategyLabRuns: vi.fn(),
  saveStrategyLabRun: vi.fn(),
  deleteStrategyLabRun: vi.fn(),
}));

const renderWithClient = (ui: React.ReactElement) => {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
    },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      {ui}
    </QueryClientProvider>
  );
};

describe('StrategyLabPage', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    window.localStorage.clear();
    vi.mocked(strategyLabApi.listStrategyLabRuns).mockResolvedValue([]);
    vi.mocked(strategyLabApi.saveStrategyLabRun).mockResolvedValue({
      id: 1,
      run_type: 'comparison',
      label: 'Saved',
      request_config: {},
      result_payload: {},
      metrics: {},
      created_at: new Date().toISOString(),
    });
    vi.mocked(strategyLabApi.deleteStrategyLabRun).mockResolvedValue({ status: 'succeeded', deleted: true });
    vi.mocked(strategyLabApi.cancelParameterSweep).mockResolvedValue({ status: 'succeeded', message: 'Cancelled', sweep_id: 'test' });
    vi.mocked(backtestApi.getAvailableStrategies).mockResolvedValue([
      {
        filename: 'trend.yaml',
        name: 'Trend Strategy',
        description: 'Trend',
        config: {
          name: 'Trend Strategy',
          version: '1.0',
          indicators: [
            { name: 'sma_fast', type: 'sma', length: 5 },
            { name: 'sma_slow', type: 'sma', length: 20 },
          ],
          entry_rules: [],
          exit_rules: [],
          position_sizing: { method: 'fixed_quantity', quantity: 100 },
        },
      },
      {
        filename: 'mean.yaml',
        name: 'Mean Strategy',
        description: 'Mean',
        config: {
          name: 'Mean Strategy',
          version: '1.0',
          indicators: [{ name: 'rsi', type: 'rsi', length: 14 }],
          entry_rules: [],
          exit_rules: [],
          position_sizing: { method: 'fixed_quantity', quantity: 100 },
        },
      },
    ]);
  });

  it('runs selected strategies and renders comparison table with ranking eligibility checks', async () => {
    // 4 trades (< 5) is not ranking eligible
    vi.mocked(backtestApi.runBacktest)
      .mockResolvedValueOnce({
        status: 'succeeded',
        analytics: {
          total_net_pnl: 1000,
          win_rate: 0.5,
          total_trades: 4,
          profit_factor: 2,
          expectancy: 125,
          average_win: 500,
          average_loss: 250,
          metrics: {
            win_rate: { value: 0.5, status: 'valid', sample_size: 4 },
            profit_factor: { value: 2, status: 'valid', sample_size: 4 },
            total_net_pnl: { value: 1000, status: 'valid', sample_size: 4 },
          },
        },
      })
      // 6 trades (>= 5) is ranking eligible
      .mockResolvedValueOnce({
        status: 'succeeded',
        analytics: {
          total_net_pnl: 2000,
          win_rate: 0.75,
          total_trades: 6,
          profit_factor: 3,
          expectancy: 300,
          average_win: 700,
          average_loss: 200,
          metrics: {
            win_rate: { value: 0.75, status: 'valid', sample_size: 6 },
            profit_factor: { value: 3, status: 'valid', sample_size: 6 },
            total_net_pnl: { value: 2000, status: 'valid', sample_size: 6 },
          },
        },
      });

    renderWithClient(<StrategyLabPage />);

    await waitFor(() => {
      expect(screen.getAllByText('Trend Strategy').length).toBeGreaterThan(0);
      expect(screen.getAllByText('Mean Strategy').length).toBeGreaterThan(0);
    });

    const checkboxes = screen.getAllByRole('checkbox');
    fireEvent.click(checkboxes[0]); // First strategy checkbox
    fireEvent.click(checkboxes[1]); // Second strategy checkbox
    fireEvent.click(screen.getByRole('button', { name: /Compare Strategies/i }));

    await waitFor(() => {
      expect(screen.getByText('Comparison')).toBeInTheDocument();
    });

    expect(backtestApi.runBacktest).toHaveBeenCalledTimes(2);
    expect(screen.getByText('2,000.00')).toBeInTheDocument();
    expect(screen.getByText('Best eligible')).toBeInTheDocument();
    expect(screen.getByText('Not rankable (Low sample)')).toBeInTheDocument();
    expect(screen.getByText('Run History')).toBeInTheDocument();
    expect(screen.getByText('2 strategy comparison')).toBeInTheDocument();
    expect(strategyLabApi.saveStrategyLabRun).toHaveBeenCalledTimes(1);
  });

  it('validates non-overlapping In-Sample and Out-of-Sample date ranges', async () => {
    renderWithClient(<StrategyLabPage />);

    await waitFor(() => {
      expect(screen.getAllByText('Trend Strategy').length).toBeGreaterThan(0);
    });

    // Enable OOS
    fireEvent.click(screen.getByLabelText(/Enable Out-of-Sample/i));

    // Set overlapping dates (IS: 2020-01-01 to 2023-06-01, OOS: 2023-01-01 to 2023-12-31)
    const isEndInput = screen.getByLabelText(/In-Sample End Date/i);
    fireEvent.change(isEndInput, { target: { value: '2023-06-01' } });

    const checkboxes = screen.getAllByRole('checkbox');
    fireEvent.click(checkboxes[0]); // First strategy checkbox
    fireEvent.click(screen.getByRole('button', { name: /Compare Strategies/i }));

    await waitFor(() => {
      expect(screen.getByText(/must not overlap/i)).toBeInTheDocument();
    });

    expect(backtestApi.runBacktest).not.toHaveBeenCalled();
  });

  it('runs a parameter sweep with typed parameter selection and renders robustness metrics', async () => {
    vi.mocked(strategyLabApi.runParameterSweep).mockResolvedValue({
      status: 'succeeded',
      total_variants: 2,
      truncated: false,
      variants: [
        {
          label: 'sma_fast.length=5',
          parameters: { 'indicators[0].length': 5 },
          response: { status: 'succeeded', analytics: null },
          metrics: {
            status: 'succeeded',
            total_trades: 8,
            win_rate: 0.6,
            net_pnl: 2500,
            profit_factor: 2.5,
            expectancy: 312.5,
            ranking_eligible: true,
            robustness_badge: 'Robust',
            robustness_score: 85.0,
            robustness: {
              badge: 'Robust',
              score: 85.0,
              sample_size_is: 8,
            },
          },
        },
        {
          label: 'sma_fast.length=10',
          parameters: { 'indicators[0].length': 10 },
          response: { status: 'succeeded', analytics: null },
          metrics: {
            status: 'succeeded',
            total_trades: 3,
            win_rate: 0.33,
            net_pnl: -200,
            profit_factor: 0.5,
            expectancy: -66.6,
            ranking_eligible: false,
            ranking_reason: 'Insufficient sample size (3 trades < 5 required).',
            robustness_badge: 'Low Sample',
            robustness_score: 0.0,
            robustness: {
              badge: 'Low Sample',
              score: 0.0,
              sample_size_is: 3,
            },
          },
        },
      ],
    });

    renderWithClient(<StrategyLabPage />);

    await waitFor(() => {
      expect(screen.getAllByText('Trend Strategy').length).toBeGreaterThan(0);
    });

    // Pick target component and parameter dropdowns
    expect(screen.getByLabelText(/Target Component/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/Parameter/i)).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /Run Sweep/i }));

    await waitFor(() => {
      expect(screen.getByText('Sweep Results')).toBeInTheDocument();
    });

    expect(strategyLabApi.runParameterSweep).toHaveBeenCalledTimes(1);
    expect(screen.getByText('sma_fast.length=5')).toBeInTheDocument();
    expect(screen.getByText('Robust (85)')).toBeInTheDocument();
    expect(screen.getByText('Low Sample (0)')).toBeInTheDocument();
    expect(screen.getByText('Not rankable (Low sample)')).toBeInTheDocument();
  });
});
