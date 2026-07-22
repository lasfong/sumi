import { fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import type { PracticeWorkflowSnapshot } from '../../../types';
import { PracticeJournal } from '../PracticeJournal';
import { PracticeRail } from '../PracticeRail';
import { TradeControls } from '../TradeControls';

const snapshot = (patch: Partial<PracticeWorkflowSnapshot> = {}): PracticeWorkflowSnapshot => ({
  session_id: 1, symbol: 'FPT', current_index: 10, visible_bar: 11, total_bars: 100,
  current_date: '2024-01-12T00:00:00', current_price: 100, current_volume: 1234,
  initial_cash: 1_000_000, current_cash: 900_000, available_quantity: 0,
  latest_activity_index: 10, historical: false, can_trade: true,
  decisions: [], orders: [], executions: [], positions: [], trades: [], ...patch,
});

describe('integrated practice workflow UI', () => {
  it('keeps invalid decisions local and prevents duplicate submission', async () => {
    const user = userEvent.setup();
    let resolveSubmit: ((value: { ok: boolean; message: string }) => void) | undefined;
    const onSubmit = vi.fn(() => new Promise<{ ok: boolean; message: string }>(resolve => { resolveSubmit = resolve; }));
    render(<TradeControls snapshot={snapshot()} onSubmitDecision={onSubmit} />);
    await user.click(screen.getByRole('button', { name: 'BUY' }));
    await user.clear(screen.getByLabelText('Quantity'));
    await user.type(screen.getByLabelText('Quantity'), '0');
    await user.click(screen.getByRole('button', { name: 'Submit BUY' }));
    expect(screen.getByRole('alert')).toHaveTextContent('Quantity must be greater than zero');
    expect(onSubmit).not.toHaveBeenCalled();

    await user.clear(screen.getByLabelText('Quantity'));
    await user.type(screen.getByLabelText('Quantity'), '100');
    const submit = screen.getByRole('button', { name: 'Submit BUY' });
    await user.click(submit);
    fireEvent.click(submit);
    expect(onSubmit).toHaveBeenCalledTimes(1);
    expect(screen.getByRole('button', { name: 'Submitting…' })).toBeDisabled();
    resolveSubmit?.({ ok: true, message: 'BUY executed.' });
  });

  it('keeps the modal open with honest backend rejection and blocks historical actions', async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn().mockResolvedValue({ ok: false, message: 'Cannot sell: T+2 constraint.' });
    const { rerender } = render(<TradeControls snapshot={snapshot({ positions: [{ id: 1, symbol: 'FPT', quantity: 100, average_price: 90, total_cost: 9000, current_price: 100, realized_pnl: 0, unrealized_pnl: 1000, available_quantity: 0, opened_at: '2024-01-01' }] })} onSubmitDecision={onSubmit} />);
    await user.click(screen.getByRole('button', { name: 'SELL' }));
    await user.click(screen.getByRole('button', { name: 'Submit SELL' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('T+2 constraint');
    expect(screen.getByRole('dialog')).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: 'Cancel' }));
    rerender(<TradeControls snapshot={snapshot({ can_trade: false, historical: true, latest_activity_index: 15, trade_block_reason: 'Advance to bar #16.' })} onSubmitDecision={onSubmit} />);
    expect(screen.getByTestId('historical-trade-block')).toHaveTextContent('Advance to bar #16');
    expect(screen.getByRole('button', { name: 'BUY' })).toBeDisabled();
  });

  it('serializes a checklist, retains the draft on failure and isolates keyboard events', async () => {
    const user = userEvent.setup();
    const parentKey = vi.fn();
    const onSave = vi.fn().mockResolvedValueOnce({ ok: false, message: 'Transport error' }).mockResolvedValueOnce({ ok: true, message: 'Saved' });
    render(<div onKeyDown={parentKey}><PracticeJournal snapshot={snapshot()} entries={[]} loading={false} loadError={false} onSave={onSave} /></div>);
    await user.click(screen.getByRole('button', { name: 'New observation' }));
    await user.click(screen.getByLabelText('Risk defined'));
    await user.type(screen.getByLabelText('Checklist observation'), 'Defined 2% risk');
    fireEvent.keyDown(screen.getByLabelText('Checklist observation'), { key: 'ArrowLeft' });
    expect(parentKey).not.toHaveBeenCalled();
    await user.click(screen.getByRole('button', { name: 'Save checklist' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Transport error');
    expect(screen.getByLabelText('Checklist observation')).toHaveValue('Defined 2% risk');
    await user.click(screen.getByRole('button', { name: 'Save checklist' }));
    expect(onSave).toHaveBeenCalledTimes(2);
    expect(JSON.parse(onSave.mock.calls[1][0].content).context.candleIndex).toBe(10);
  });

  it('keeps all workflow surfaces in one rail and opens Drawing on selection', async () => {
    const user = userEvent.setup();
    const { rerender } = render(<PracticeRail trade="trade-view" journal="journal-view" decisions="decision-view" drawing="drawing-view" />);
    expect(screen.getByTestId('practice-tab-trade')).toHaveTextContent('trade-view');
    await user.click(screen.getByRole('tab', { name: 'Journal' }));
    expect(screen.getByTestId('practice-tab-journal')).toHaveTextContent('journal-view');
    rerender(<PracticeRail key="drawing-1" trade="trade-view" journal="journal-view" decisions="decision-view" drawing="drawing-view" selectedDrawingId="drawing-1" />);
    expect(await screen.findByTestId('practice-tab-drawing')).toHaveTextContent('drawing-view');
  });

  it('contains trade-dialog focus, closes on Escape and returns focus to the opener', async () => {
    const user = userEvent.setup();
    render(<TradeControls snapshot={snapshot()} onSubmitDecision={vi.fn()} />);
    const opener = screen.getByRole('button', { name: 'BUY' });
    await user.click(opener);
    expect(screen.getByLabelText('Quantity')).toHaveFocus();
    await user.tab({ shift: true });
    expect(screen.getByRole('button', { name: 'Submit BUY' })).toHaveFocus();
    await user.keyboard('{Escape}');
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(opener).toHaveFocus();
  });

  it('contains checklist-dialog focus, closes on Escape and returns focus', async () => {
    const user = userEvent.setup();
    render(<PracticeJournal snapshot={snapshot()} entries={[]} loading={false} loadError={false} onSave={vi.fn()} />);
    const opener = screen.getByRole('button', { name: 'New observation' });
    await user.click(opener);
    expect(screen.getByLabelText('Trend identified')).toHaveFocus();
    await user.tab({ shift: true });
    expect(screen.getByRole('button', { name: 'Save checklist' })).toHaveFocus();
    await user.keyboard('{Escape}');
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(opener).toHaveFocus();
  });

  it('links tabs to the active panel and supports arrow-key navigation', async () => {
    const user = userEvent.setup();
    render(<PracticeRail trade="trade-view" journal="journal-view" decisions="decision-view" drawing="drawing-view" />);
    const trade = screen.getByRole('tab', { name: 'Trade' });
    expect(trade).toHaveAttribute('aria-controls', 'practice-panel-trade');
    expect(screen.getByRole('tabpanel')).toHaveAttribute('aria-labelledby', 'practice-tab-trade');
    trade.focus();
    await user.keyboard('{ArrowRight}');
    expect(screen.getByRole('tab', { name: 'Journal' })).toHaveFocus();
    expect(screen.getByRole('tab', { name: 'Journal' })).toHaveAttribute('aria-selected', 'true');
  });
});
