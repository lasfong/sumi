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
    vi.mocked(backtestApi.getAvailableStrategies).mockResolvedValue([
      { filename: 'trend.yaml', name: 'Trend Strategy', description: 'Trend', config: { name: 'Trend Strategy' } },
      { filename: 'mean.yaml', name: 'Mean Strategy', description: 'Mean', config: { name: 'Mean Strategy' } },
    ]);
  });

  it('runs selected strategies and renders comparison table', async () => {
    vi.mocked(backtestApi.runBacktest)
      .mockResolvedValueOnce({
        status: 'succeeded',
        analytics: {
          total_net_pnl: 1000,
          win_rate: 0.5,
          total_trades: 4,
          average_win: 500,
          average_loss: 250,
          profit_factor: 2,
          expectancy: 125,
        },
      })
      .mockResolvedValueOnce({
        status: 'succeeded',
        analytics: {
          total_net_pnl: 2000,
          win_rate: 0.75,
          total_trades: 6,
          average_win: 700,
          average_loss: 200,
          profit_factor: 3,
          expectancy: 300,
        },
      });

    renderWithClient(<StrategyLabPage />);

    await waitFor(() => {
      expect(screen.getByText('Trend Strategy')).toBeInTheDocument();
      expect(screen.getByText('Mean Strategy')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByLabelText(/Trend Strategy/i));
    fireEvent.click(screen.getByLabelText(/Mean Strategy/i));
    fireEvent.click(screen.getByRole('button', { name: /Compare Strategies/i }));

    await waitFor(() => {
      expect(screen.getByText('Comparison')).toBeInTheDocument();
    });

    expect(backtestApi.runBacktest).toHaveBeenCalledTimes(2);
    expect(screen.getByText('2,000.00')).toBeInTheDocument();
    expect(screen.getByText('Best')).toBeInTheDocument();
  });

  it('runs a parameter sweep and renders sweep results', async () => {
    vi.mocked(strategyLabApi.runParameterSweep).mockResolvedValue({
      status: 'succeeded',
      total_variants: 2,
      truncated: false,
      variants: [
        {
          label: 'indicators[0].length=10',
          parameters: { 'indicators[0].length': 10 },
          response: { status: 'succeeded', analytics: null },
          metrics: {
            status: 'succeeded',
            total_trades: 4,
            win_rate: 0.5,
            net_pnl: 1000,
            profit_factor: 2,
            expectancy: 125,
          },
        },
        {
          label: 'indicators[0].length=20',
          parameters: { 'indicators[0].length': 20 },
          response: { status: 'succeeded', analytics: null },
          metrics: {
            status: 'succeeded',
            total_trades: 5,
            win_rate: 0.6,
            net_pnl: 2000,
            profit_factor: 3,
            expectancy: 300,
          },
        },
      ],
    });

    renderWithClient(<StrategyLabPage />);

    await waitFor(() => {
      expect(screen.getByText('Trend Strategy')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByLabelText(/Trend Strategy/i));
    fireEvent.click(screen.getByRole('button', { name: /Run Sweep/i }));

    await waitFor(() => {
      expect(screen.getByText('Sweep Results')).toBeInTheDocument();
    });

    expect(strategyLabApi.runParameterSweep).toHaveBeenCalledTimes(1);
    expect(screen.getByText('indicators[0].length=20')).toBeInTheDocument();
    expect(screen.getByText('2,000.00')).toBeInTheDocument();
  });
});
