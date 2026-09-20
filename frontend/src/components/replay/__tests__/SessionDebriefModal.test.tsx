import { fireEvent, render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { describe, expect, it, vi } from 'vitest';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { SessionDebriefModal } from '../SessionDebriefModal';
import type { PracticeWorkflowSnapshot } from '../../../types';

const mockPracticeData = (patch: Partial<PracticeWorkflowSnapshot> = {}): PracticeWorkflowSnapshot => ({
  session_id: 10,
  symbol: 'FPT',
  current_index: 20,
  visible_bar: 21,
  total_bars: 100,
  current_date: '2024-01-20T00:00:00',
  current_price: 95000,
  current_volume: 500000,
  initial_cash: 100_000_000,
  current_cash: 95_000_000,
  available_quantity: 100,
  latest_activity_index: 20,
  historical: false,
  can_trade: true,
  decisions: [
    {
      id: 1, candle_index: 10, decision_date: '2024-01-10', action: 'BUY', quantity: 100,
      mistake_tag: 'Chased Price',
    },
  ],
  orders: [],
  executions: [],
  positions: [],
  trades: [
    {
      id: 1, symbol: 'FPT', entry_date: '2024-01-10', exit_date: '2024-01-15',
      entry_price: 90000, exit_price: 95000, quantity: 100, net_pnl: 500000, pnl_percent: 5.5,
      status: 'closed', result: 'win', initial_stop_loss: 88000, target_price: 96000,
    },
  ],
  ...patch,
});

const renderWithProviders = (ui: React.ReactElement) => {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={queryClient}>
      <MemoryRouter>{ui}</MemoryRouter>
    </QueryClientProvider>
  );
};

describe('SessionDebriefModal', () => {
  it('renders nothing when closed', () => {
    const { container } = renderWithProviders(
      <SessionDebriefModal isOpen={false} onClose={vi.fn()} sessionId={10} symbolName="FPT" />
    );
    expect(container.firstChild).toBeNull();
  });

  it('renders metrics, discipline breakdown, mistake tags and closes on Escape', () => {
    const onClose = vi.fn();
    renderWithProviders(
      <SessionDebriefModal
        isOpen={true}
        onClose={onClose}
        sessionId={10}
        symbolName="FPT"
        practiceData={mockPracticeData()}
      />
    );

    expect(screen.getByRole('dialog', { name: 'Tổng kết phiên luyện tập' })).toBeInTheDocument();
    expect(screen.getByText(/Tổng kết phiên Replay · FPT/)).toBeInTheDocument();
    expect(screen.getByText('Win Rate (Tỷ lệ thắng)')).toBeInTheDocument();
    expect(screen.getByText('Profit Factor')).toBeInTheDocument();
    expect(screen.getByText(/Tuân thủ Stop Loss/)).toBeInTheDocument();
    expect(screen.getByText(/Chased Price/)).toBeInTheDocument();

    // Escape closes modal
    fireEvent.keyDown(window, { key: 'Escape' });
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
