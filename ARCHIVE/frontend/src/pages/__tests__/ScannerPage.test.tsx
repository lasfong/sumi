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
  listScannerRuns: vi.fn(),
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
    vi.mocked(scannerApi.listScannerRuns).mockResolvedValue([]);
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
        source_context: {
          schema_version: 1,
          source_type: 'scanner_signal',
          replay_intent: 'blind_practice',
          reveal_at_index: 8,
          revealed: false,
          signal: null,
        },
        created_at: '2024-01-01T00:00:00',
        updated_at: '2024-01-01T00:00:00',
      },
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

    expect(screen.getByRole('button', { name: /Start blind practice for FPT \(recommended\)/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Review signal for FPT/i })).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /Start blind practice for FPT \(recommended\)/i }));

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
        replay_intent: 'blind_practice',
      });
    });
  });

  it('sends the explicit signal-review intent', async () => {
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
        id: 43,
        symbol: 'FPT',
        timeframe: '1D',
        adjustment_type: 'unadjusted',
        start_date: '2024-01-01',
        end_date: '2024-04-01',
        current_index: 8,
        initial_cash: 100000000,
        current_cash: 100000000,
        status: 'active',
        mode: 'normal',
        hide_symbol: 0,
        hide_date: 0,
        source_context: {
          schema_version: 1,
          source_type: 'scanner_signal',
          replay_intent: 'signal_review',
          reveal_at_index: 8,
          revealed: true,
          signal: {
            timestamp: '2024-01-10T00:00:00',
            type: 'entry',
            strategy: 'Scanner Strategy',
            price: 100,
            regime: 'bullish',
          },
        },
        created_at: '2024-01-01T00:00:00',
        updated_at: '2024-01-01T00:00:00',
      },
      window_start: '2024-01-01',
      window_end: '2024-04-01',
    });
    renderWithClient(<ScannerPage />);
    await screen.findByText(/Scanner Strategy/i);
    fireEvent.change(screen.getByRole('combobox'), { target: { value: 'strategy.yaml' } });
    fireEvent.click(screen.getByRole('button', { name: /Run Scanner/i }));
    await screen.findByText('FPT');
    fireEvent.click(screen.getByRole('button', { name: /Review signal for FPT/i }));
    await waitFor(() => expect(
      vi.mocked(scannerApi.createReplaySessionFromSignal).mock.calls[0][0],
    ).toEqual(expect.objectContaining({ replay_intent: 'signal_review' })));
  });

  it('displays historical run results when clicking saved run even after a new scan (SQA-015)', async () => {
    const historicalRun: scannerApi.ScannerRun = {
      id: 4,
      label: 'Historical Run #4',
      status: 'succeeded',
      total_results: 1,
      created_at: '2024-01-01T10:00:00Z',
      request_config: { symbols: ['VNM'] },
      result_payload: {
        status: 'succeeded',
        total_results: 1,
        truncated: false,
        warnings: [],
        results: [
          {
            symbol: 'VNM',
            timestamp: '2024-01-05T00:00:00',
            signal_type: 'entry',
            strategy: 'Old Strategy',
            price: 70,
            regime: 'bullish',
          },
        ],
      },
    };

    vi.mocked(scannerApi.listScannerRuns).mockResolvedValue([historicalRun]);

    vi.mocked(scannerApi.runScanner).mockResolvedValue({
      status: 'succeeded',
      total_results: 1,
      truncated: false,
      warnings: [],
      results: [
        {
          symbol: 'FPT',
          timestamp: '2024-01-10T00:00:00',
          signal_type: 'entry',
          strategy: 'Scanner Strategy',
          price: 100,
          regime: 'bullish',
        },
      ],
    });

    renderWithClient(<ScannerPage />);
    await screen.findByText(/Scanner Strategy/i);

    // 1. Run a new scan
    fireEvent.click(screen.getByRole('button', { name: /Run Scanner/i }));
    await screen.findByText('FPT');
    expect(screen.queryByText('VNM')).not.toBeInTheDocument();

    // 2. Click the historical run #4
    const historyRunBtn = await screen.findByTestId('history-run-4');
    fireEvent.click(historyRunBtn);

    // 3. The historical run results (VNM) must be displayed, not the stale FPT results
    await screen.findByText('VNM');
    expect(screen.queryByText('FPT')).not.toBeInTheDocument();
    expect(screen.getByText(/Signals from Run #4/i)).toBeInTheDocument();
  });

  it('filters and sorts scanner results table by symbol, signal, and price (SQA-005)', async () => {
    vi.mocked(scannerApi.runScanner).mockResolvedValue({
      status: 'succeeded',
      total_results: 3,
      truncated: false,
      warnings: [],
      results: [
        { symbol: 'FPT', timestamp: '2024-01-10T00:00:00', signal_type: 'entry', strategy: 'Strat', price: 100, regime: 'bullish' },
        { symbol: 'SSI', timestamp: '2024-01-12T00:00:00', signal_type: 'exit', strategy: 'Strat', price: 35, regime: 'bearish' },
        { symbol: 'HPG', timestamp: '2024-01-15T00:00:00', signal_type: 'entry', strategy: 'Strat', price: 25, regime: 'bullish' },
      ],
    });

    renderWithClient(<ScannerPage />);
    await screen.findByText(/Scanner Strategy/i);
    fireEvent.click(screen.getByRole('button', { name: /Run Scanner/i }));

    await screen.findByText('FPT');
    expect(screen.getByText('SSI')).toBeInTheDocument();
    expect(screen.getByText('HPG')).toBeInTheDocument();

    // Verify results count badge
    expect(screen.getByTestId('scanner-results-count')).toHaveTextContent('Hiển thị 3 / 3 tín hiệu');

    // 1. Search/Filter by symbol 'FP'
    const filterInput = screen.getByTestId('scanner-filter-input');
    fireEvent.change(filterInput, { target: { value: 'FP' } });
    expect(screen.getByText('FPT')).toBeInTheDocument();
    expect(screen.queryByText('SSI')).not.toBeInTheDocument();
    expect(screen.queryByText('HPG')).not.toBeInTheDocument();
    expect(screen.getByTestId('scanner-results-count')).toHaveTextContent('Hiển thị 1 / 3 tín hiệu');

    // Clear search filter
    fireEvent.change(filterInput, { target: { value: '' } });
    expect(screen.getByText('SSI')).toBeInTheDocument();

    // 2. Filter by signal dropdown 'exit'
    const signalSelect = screen.getByTestId('scanner-signal-filter');
    fireEvent.change(signalSelect, { target: { value: 'exit' } });
    expect(screen.getByText('SSI')).toBeInTheDocument();
    expect(screen.queryByText('FPT')).not.toBeInTheDocument();
    expect(screen.queryByText('HPG')).not.toBeInTheDocument();

    // Reset signal filter
    fireEvent.change(signalSelect, { target: { value: 'ALL' } });

    // 3. Test sort by Price (click th-sort-price)
    const priceTh = screen.getByTestId('th-sort-price');
    fireEvent.click(priceTh); // ascending: 25 (HPG), 35 (SSI), 100 (FPT)
    const rowsAsc = screen.getAllByRole('row');
    // First data row (index 1) should be HPG (price 25)
    expect(rowsAsc[1]).toHaveTextContent('HPG');

    fireEvent.click(priceTh); // descending: 100 (FPT), 35 (SSI), 25 (HPG)
    const rowsDesc = screen.getAllByRole('row');
    expect(rowsDesc[1]).toHaveTextContent('FPT');
  });
});
