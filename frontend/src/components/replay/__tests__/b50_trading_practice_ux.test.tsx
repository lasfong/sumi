import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { PracticeWorkflowSnapshot } from '../../../types';
import { TradeControls } from '../TradeControls';
import { PositionPanel } from '../PositionPanel';
import { PendingOrdersPanel } from '../PendingOrdersPanel';

const makeSnapshot = (overrides: Partial<PracticeWorkflowSnapshot> = {}): PracticeWorkflowSnapshot => ({
  session_id: 42,
  symbol: 'FPT',
  current_index: 5,
  visible_bar: 6,
  total_bars: 50,
  current_date: '2025-03-10T00:00:00',
  current_price: 100000,
  current_volume: 1200000,
  initial_cash: 100000000,
  current_cash: 85000000,
  available_quantity: 0,
  blocked_quantity: 200,
  earliest_release_date: '2025-03-12T00:00:00',
  settlement_label: 'T+1 · về sau 1 phiên',
  latest_activity_index: 5,
  historical: false,
  can_trade: true,
  decisions: [],
  orders: [],
  executions: [],
  positions: [],
  trades: [],
  ...overrides,
});

describe('B50 Trading Practice UX (GAP-U-01, GAP-U-02, GAP-U-04)', () => {
  it('renders inline T+1.5 settlement notice and Available vs Unsettled shares without hover (GAP-U-01)', () => {
    const snapshot = makeSnapshot({
      positions: [
        {
          id: 1,
          symbol: 'FPT',
          quantity: 200,
          average_price: 98000,
          total_cost: 19600000,
          current_price: 100000,
          realized_pnl: 0,
          unrealized_pnl: 400000,
          available_quantity: 0,
          blocked_quantity: 200,
          earliest_release_date: '2025-03-12T00:00:00',
          opened_at: '2025-03-10T00:00:00',
        },
      ],
    });

    render(
      <>
        <TradeControls snapshot={snapshot} onSubmitDecision={vi.fn()} />
        <PositionPanel positions={snapshot.positions} snapshot={snapshot} />
      </>
    );

    const inlineNotice = screen.getByTestId('settlement-inline-notice');
    expect(inlineNotice.textContent).toContain('T+1 · về sau 1 phiên');
    expect(inlineNotice.textContent).toContain('200');
    expect(inlineNotice.textContent).toContain('2025-03-12');

    expect(screen.getByTestId('position-available-qty').textContent).toBe('0');
    expect(screen.getByTestId('position-unsettled-qty').textContent).toBe('200');
  });

  it('shows inline Vietnamese SL/TP validation alert while keeping sizing panel visible (GAP-U-02)', () => {
    const snapshot = makeSnapshot();
    render(<TradeControls snapshot={snapshot} onSubmitDecision={vi.fn()} />);

    fireEvent.click(screen.getByText('BUY'));
    expect(screen.getByTestId('position-sizing-panel')).toBeTruthy();

    // Enter invalid SL >= Entry (105000 >= 100000) for LONG
    const slInput = screen.getByLabelText('Stop loss');
    fireEvent.change(slInput, { target: { value: '105000' } });

    const alert = screen.getByTestId('sl-tp-validation-alert');
    expect(alert.textContent).toContain('Giá cắt lỗ (SL) phải thấp hơn giá mua');
    // Sizing panel remains visible (never disappears)
    expect(screen.getByTestId('position-sizing-panel')).toBeTruthy();
  });

  it('computes symmetric SHORT PnL/R in PositionPanel and syncs Risk-Reward drawing in both directions (GAP-U-02)', () => {
    // Mount drawing state element with a SHORT Risk-Reward drawing (Stop 105000 > Entry 100000 > Target 90000)
    const drawingHolder = document.createElement('div');
    drawingHolder.setAttribute(
      'data-drawing-interaction-state',
      JSON.stringify({
        drawings: [
          {
            tool: 'risk-reward',
            anchors: [{ price: 100000 }, { price: 105000 }, { price: 90000 }],
          },
        ],
      }),
    );
    document.body.appendChild(drawingHolder);

    const shortSnapshot = makeSnapshot({
      available_quantity: 300,
      positions: [
        {
          id: 2,
          symbol: 'FPT',
          quantity: -300,
          average_price: 100000,
          total_cost: 30000000,
          current_price: 95000,
          realized_pnl: 0,
          unrealized_pnl: 1500000,
          available_quantity: 300,
          blocked_quantity: 0,
          opened_at: '2025-03-05T00:00:00',
        },
      ],
      trades: [
        {
          id: 10,
          symbol: 'FPT',
          entry_date: '2025-03-05T00:00:00',
          entry_price: 100000,
          quantity: -300,
          initial_stop_loss: 105000,
          target_price: 90000,
          planned_r: 2.0,
          status: 'open',
          result: 'open',
        },
      ],
    });

    const settledSellableSnapshot = makeSnapshot({
      available_quantity: 300,
      positions: [
        {
          ...shortSnapshot.positions[0],
          quantity: 300,
          available_quantity: 300,
        },
      ],
    });

    render(
      <>
        <PositionPanel positions={shortSnapshot.positions} snapshot={shortSnapshot} />
        <TradeControls snapshot={settledSellableSnapshot} onSubmitDecision={vi.fn()} />
      </>
    );

    // For SHORT at 100k when price falls to 95k, unrealized PnL is +5.00% (+1.00R)
    const pnlEl = screen.getByTestId('position-unrealized-pnl');
    expect(pnlEl.textContent).toContain('+5.00%');
    expect(pnlEl.textContent).toContain('+1.00R');

    // Open SELL dialog and verify Sync from Drawing is available and populates SL/TP for SHORT
    fireEvent.click(screen.getByText('SELL'));
    const syncBtn = screen.getByTestId('sync-from-drawing');
    fireEvent.click(syncBtn);

    expect((screen.getByLabelText('Stop loss') as HTMLInputElement).value).toBe('105000');
    expect((screen.getByLabelText('Target price') as HTMLInputElement).value).toBe('90000');
    expect(screen.getByTestId('position-sizing-panel').textContent).toContain('1 : 2.0');

    document.body.removeChild(drawingHolder);
  });

  it('renders Hủy lệnh button for pending LIMIT orders and invokes onCancelOrder (GAP-U-04)', async () => {
    const onCancelOrder = vi.fn().mockResolvedValue(undefined);
    render(
      <PendingOrdersPanel
        orders={[
          {
            id: 77,
            decision_id: 12,
            side: 'BUY',
            order_type: 'LIMIT',
            requested_price: 98500,
            quantity: 200,
            status: 'pending',
            decision_index: 4,
            explanation: 'Pending until a future visible candle range reaches the requested price.',
          },
        ]}
        onCancelOrder={onCancelOrder}
      />
    );

    const cancelBtn = screen.getByTestId('cancel-order-77');
    expect(cancelBtn.textContent).toBe('Hủy lệnh');
    fireEvent.click(cancelBtn);

    await waitFor(() => {
      expect(onCancelOrder).toHaveBeenCalledWith(77);
    });
  });
});
