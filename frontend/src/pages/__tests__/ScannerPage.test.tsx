import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { MemoryRouter } from 'react-router-dom';
import { ScannerPage } from '../ScannerPage';
import * as backtestApi from '../../api/backtestApi';
import * as scannerApi from '../../api/scannerApi';
import '@testing-library/jest-dom';

vi.mock('../../api/backtestApi', () => ({
  getAvailableStrategies: vi.fn(),
}));

vi.mock('../../api/scannerApi', () => ({
  runScanner: vi.fn(),
  createReplaySessionFromSignal: vi.fn(),
}));

const renderWithClient = (ui: React.ReactElement) => {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
    },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <MemoryRouter>
        {ui}
      </MemoryRouter>
    </QueryClientProvider>
  );
};

describe('ScannerPage', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(backtestApi.getAvailableStrategies).mockResolvedValue([
      { filename: 'strategy.yaml', name: 'Scanner Strategy', description: 'Scanner', config: { name: 'Scanner Strategy' } },
    ]);
  });

  it('creates replay session from a scanner signal', async () => {
    vi.mocked(scannerApi.runScanner).mockResolvedValue({
      status: 'succeeded',
      total_results: 1,
      truncated: false,
      warnings: [],
      results: [{
        symbol: 'FPT',
        timestamp: '2024-01-10T00:00:00',
        signal_type: 'entry',
        strategy: 'Scanner Strategy',
        price: 100,
        regime: 'bullish',
      }],
    });
    vi.mocked(scannerApi.createReplaySessionFromSignal).mockResolvedValue({
      session: {
        id: 42,
        symbol: 'FPT',
        timeframe: '1D',
        adjustment_type: 'unadjusted',
        start_date: '2024-01-01',
        end_date: '2024-04-01',
        current_index: 0,
        initial_cash: 100000000,
        current_cash: 100000000,
        status: 'active',
        mode: 'normal',
        hide_symbol: 0,
        hide_date: 0,
        created_at: '2024-01-01T00:00:00',
        updated_at: '2024-01-01T00:00:00',
      },
      signal_timestamp: '2024-01-10T00:00:00',
      window_start: '2024-01-01',
      window_end: '2024-04-01',
    });

    renderWithClient(<ScannerPage />);

    await waitFor(() => {
      expect(screen.getByText(/Scanner Strategy/i)).toBeInTheDocument();
    });

    fireEvent.change(screen.getByRole('combobox'), { target: { value: 'strategy.yaml' } });
    fireEvent.click(screen.getByRole('button', { name: /Run Scanner/i }));

    await waitFor(() => {
      expect(screen.getByText('FPT')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole('button', { name: /Replay/i }));

    await waitFor(() => {
      expect(scannerApi.createReplaySessionFromSignal).toHaveBeenCalled();
      expect(vi.mocked(scannerApi.createReplaySessionFromSignal).mock.calls[0][0]).toEqual({
        symbol: 'FPT',
        signal_timestamp: '2024-01-10T00:00:00',
        signal_type: 'entry',
        strategy: 'Scanner Strategy',
        price: 100,
        regime: 'bullish',
        lookback_days: 120,
        forward_days: 90,
      });
    });
  });
});
