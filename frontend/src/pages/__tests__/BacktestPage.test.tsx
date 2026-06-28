import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { BacktestPage } from '../BacktestPage';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import * as backtestApi from '../../api/backtestApi';
import '@testing-library/jest-dom';

// Mock the API calls
vi.mock('../../api/backtestApi', () => ({
  getAvailableStrategies: vi.fn(),
  runBacktest: vi.fn()
}));

// Mock EquityChart because lightweight-charts relies on Canvas/DOM elements not fully supported in jsdom
vi.mock('../../components/analytics/EquityChart', () => ({
  EquityChart: () => <div data-testid="mock-equity-chart">Equity Chart</div>
}));

const queryClient = new QueryClient({
  defaultOptions: {
    queries: { retry: false }
  }
});

const renderWithClient = (ui: React.ReactElement) => {
  return render(
    <QueryClientProvider client={queryClient}>
      {ui}
    </QueryClientProvider>
  );
};

describe('BacktestPage UI Test', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    
    // Setup mock data for available strategies
    vi.mocked(backtestApi.getAvailableStrategies).mockResolvedValue([
      { filename: 'strategy1.yaml', name: 'Strategy 1', description: 'Test Strategy', config: {} }
    ]);
  });

  it('renders the form correctly', async () => {
    renderWithClient(<BacktestPage />);
    
    // Wait for strategies to load and populate the select
    await waitFor(() => {
      expect(screen.getByText(/Strategy 1/i)).toBeInTheDocument();
    });

    expect(screen.getByLabelText(/Symbol/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/Start Date/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/End Date/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/Initial Cash/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Run Backtest/i })).toBeInTheDocument();
  });

  it('calls runBacktest on submit and displays analytics', async () => {
    // Setup mock response for runBacktest
    vi.mocked(backtestApi.runBacktest).mockResolvedValue({
      session_id: 123,
      strategy: 'Strategy 1',
      symbol: 'FPT',
      total_candles: 100,
      analytics: {
        total_net_pnl: 50000,
        win_rate: 0.75,
        sharpe_ratio: 1.5,
        max_drawdown: -2000,
        total_trades: 4,
        average_win: 15000,
        average_loss: 5000,
        profit_factor: 3,
        sqn: 2.1,
        equity_curve: []
      }
    });

    renderWithClient(<BacktestPage />);
    
    // Wait for options to load
    await waitFor(() => expect(screen.getByRole('combobox')).toBeInTheDocument());

    // Select the strategy
    fireEvent.change(screen.getByRole('combobox'), { target: { value: 'strategy1.yaml' } });
    
    // Click run
    fireEvent.click(screen.getByRole('button', { name: /Run Backtest/i }));

    // Wait for the analytics to render
    await waitFor(() => {
      expect(screen.getByText(/Results \(Session #123\)/i)).toBeInTheDocument();
    });

    // Check if key metrics are displayed
    expect(screen.getByText('50,000.00')).toBeInTheDocument(); // Net PnL
    expect(screen.getByText('75.00%')).toBeInTheDocument(); // Win rate
    expect(screen.getByText('1.50')).toBeInTheDocument(); // Sharpe
  });
});
