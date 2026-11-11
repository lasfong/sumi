import { describe, expect, it } from 'vitest';
import type { PracticeWorkflowSnapshot } from '../../../types';
import { validateDecision } from '../practiceDomain';
import { formatVietnamesePercent, formatVietnameseNumber } from '../../../utils/formatters';

const baseSnapshot = (): PracticeWorkflowSnapshot => ({
  session_id: 1,
  symbol: 'FPT',
  current_index: 5,
  visible_bar: 6,
  total_bars: 100,
  current_date: '2024-01-12T00:00:00',
  current_price: 90_000,
  current_volume: 1_000_000,
  initial_cash: 100_000_000,
  current_cash: 50_000_000,
  available_quantity: 0,
  latest_activity_index: 5,
  historical: false,
  can_trade: true,
  decisions: [],
  orders: [],
  executions: [],
  positions: [],
  trades: [],
});

describe('SQA-017 & SQA-018 Frontend Adversarial Challenge', () => {
  it('validates early cash affordability for BUY and ADD orders', () => {
    const snap = baseSnapshot(); // cash = 50,000,000 VND

    // 1. Affordable BUY: 500 shares @ 90,000 VND = 45,000,000 * 1.0015 = 45,067,500 <= 50M
    expect(validateDecision({ action: 'BUY', quantity: 500, price: 90_000 }, snap)).toBeNull();

    // 2. Unaffordable BUY: 1,000 shares @ 90,000 VND = 90,000,000 * 1.0015 = 90,135,000 > 50M
    const errMkt = validateDecision({ action: 'BUY', quantity: 1000, price: 90_000 }, snap);
    expect(errMkt).toContain('Không đủ tiền mặt');
    expect(errMkt).toContain((90_135_000).toLocaleString());
    expect(errMkt).toContain((50_000_000).toLocaleString());

    // 3. Fallback to current_price when price is undefined
    const errNoPrice = validateDecision({ action: 'BUY', quantity: 1000 }, snap);
    expect(errNoPrice).toContain('Không đủ tiền mặt');

    // 4. ADD action also validates cash
    const errAdd = validateDecision({ action: 'ADD', quantity: 1000, price: 90_000 }, snap);
    expect(errAdd).toContain('Không đủ tiền mặt');

    // 5. Null current_cash does not block submission
    const snapNullCash = { ...snap, current_cash: undefined as unknown as number };
    expect(validateDecision({ action: 'BUY', quantity: 1000, price: 90_000 }, snapNullCash)).toBeNull();

    // 6. SELL orders do not trigger cash check
    const snapWithPos = {
      ...snap,
      current_cash: 0,
      positions: [{ id: 1, symbol: 'FPT', quantity: 500, average_price: 90_000, current_price: 90_000, unrealized_pnl: 0, realized_pnl: 0, total_cost: 45_000_000, available_quantity: 500, opened_at: '2024-01-10T00:00:00' }],
    };
    expect(validateDecision({ action: 'SELL', quantity: 200, price: 90_000 }, snapWithPos as unknown as PracticeWorkflowSnapshot)).toBeNull();
  });

  it('verifies SQA-018 percentage formatting contracts without double scaling', () => {
    // Contract A: formatVietnamesePercent is for 0..1 fractions (multiplies by 100)
    expect(formatVietnamesePercent(0.50, 2)).toBe('50,00%');
    expect(formatVietnamesePercent(0.25, 2)).toBe('25,00%');
    expect(formatVietnamesePercent(0.00, 2)).toBe('0,00%');
    expect(formatVietnamesePercent(1.00, 2)).toBe('100,00%');

    // Contract B: formatPctValue in AnalyticsPage uses formatVietnameseNumber(val, 2) + '%'
    // for values already on 0..100 percentage scale (e.g. -2.45 for -2.45%)
    const formatPctValue = (val?: number | null) => (val !== undefined && val !== null ? `${formatVietnameseNumber(val, 2)}%` : '—');

    expect(formatPctValue(-2.45)).toBe('-2,45%');
    expect(formatPctValue(15.25)).toBe('15,25%');
    expect(formatPctValue(0.00)).toBe('0,00%');
    expect(formatPctValue(-99.95)).toBe('-99,95%');
    expect(formatPctValue(500.00)).toBe('500,00%');
    expect(formatPctValue(null)).toBe('—');
    expect(formatPctValue(undefined)).toBe('—');

    // Confirm that passing -2.45 to formatPctValue NEVER yields double-scaled -245%
    expect(formatPctValue(-2.45)).not.toContain('245');
    expect(formatPctValue(-2.45)).not.toContain('244');
  });
});
