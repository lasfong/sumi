import { describe, expect, it } from 'vitest';
import type { JournalEntry, PracticeWorkflowSnapshot } from '../../../types';
import { createChecklistEntry, emptyChecklistChecks, parseChecklist, validateDecision } from '../practiceDomain';

const snapshot = (): PracticeWorkflowSnapshot => ({
  session_id: 7, symbol: 'FPT', current_index: 10, visible_bar: 11, total_bars: 100,
  current_date: '2024-01-12T00:00:00', current_price: 100, current_volume: 1000,
  initial_cash: 1_000_000, current_cash: 900_000, available_quantity: 0,
  latest_activity_index: 10, historical: false, can_trade: true,
  decisions: [], orders: [], executions: [], positions: [], trades: [],
});

describe('practice domain', () => {
  it('serializes and parses the versioned checklist with exact context', () => {
    const entry = createChecklistEntry(snapshot(), { ...emptyChecklistChecks(), riskDefined: true }, '  planned risk  ');
    const parsed = parseChecklist({ id: 1, session_id: 7, note_type: entry.note_type, content: entry.content, created_at: '', updated_at: '' } as JournalEntry);
    expect(parsed?.context).toEqual({ sessionId: 7, symbol: 'FPT', candleIndex: 10, date: '2024-01-12' });
    expect(parsed?.checks.riskDefined).toBe(true);
    expect(parsed?.observation).toBe('planned risk');
  });

  it('validates trade inputs and historical state before submission', () => {
    expect(validateDecision({ action: 'BUY', quantity: 0 }, snapshot())).toBe('Quantity must be greater than zero.');
    expect(validateDecision({ action: 'BUY', quantity: 100, order_type: 'LIMIT' }, snapshot())).toBe('Limit price must be greater than zero.');
    expect(validateDecision({ action: 'BUY', quantity: 100, stop_loss: 110 }, snapshot())).toContain('below');
    expect(validateDecision({ action: 'HOLD' }, snapshot())).toBeNull();
    const historical = { ...snapshot(), can_trade: false, historical: true, trade_block_reason: 'Advance first.' };
    expect(validateDecision({ action: 'BUY', quantity: 100 }, historical)).toBe('Advance first.');
  });
});
