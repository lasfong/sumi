import { describe, it, expect } from 'vitest';
import {
  formatVietnameseDate,
  formatVietnameseDateTime,
  formatVnd,
  formatVietnameseNumber,
  formatVietnamesePercent,
  formatVietnameseVolume,
} from '../formatters';

describe('Vietnamese formatters (PRO-UX-05 / R02-03)', () => {
  it('formats dates in dd/MM/yyyy in Asia/Ho_Chi_Minh timezone', () => {
    expect(formatVietnameseDate(null)).toBe('—');
    expect(formatVietnameseDate(undefined)).toBe('—');
    expect(formatVietnameseDate('')).toBe('—');
    expect(formatVietnameseDate('2026-08-02')).toBe('02/08/2026');
    expect(formatVietnameseDate('2026-01-01')).toBe('01/01/2026');
    expect(formatVietnameseDate('2026-08-02T10:00:00Z')).toBe('02/08/2026');
  });

  it('formats date-time string or fallback gracefully', () => {
    expect(formatVietnameseDateTime(null)).toBe('—');
    expect(formatVietnameseDateTime(undefined)).toBe('—');
    expect(formatVietnameseDateTime('2026-08-02T10:30:00Z')).toContain('2026');
  });

  it('formats VND currency with unambiguous symbol for zero, positive, and negative values', () => {
    expect(formatVnd(null)).toBe('—');
    expect(formatVnd(undefined)).toBe('—');
    expect(formatVnd(0)).toBe('0 ₫');
    expect(formatVnd(100000000)).toBe('100.000.000 ₫');
    expect(formatVnd(-5000000)).toBe('-5.000.000 ₫');
  });

  it('formats numbers and volume with vi-VN separators (dot thousands, comma decimals)', () => {
    expect(formatVietnameseNumber(null)).toBe('—');
    expect(formatVietnameseNumber(undefined)).toBe('—');
    expect(formatVietnameseNumber(0, 2)).toBe('0,00');
    expect(formatVietnameseNumber(1234.56, 2)).toBe('1.234,56');
    expect(formatVietnameseNumber(-9876.5, 2)).toBe('-9.876,50');
    expect(formatVietnameseVolume(0)).toBe('0');
    expect(formatVietnameseVolume(1500250)).toBe('1.500.250');
  });

  it('formats percentages correctly with optional sign', () => {
    expect(formatVietnamesePercent(null)).toBe('—');
    expect(formatVietnamesePercent(undefined)).toBe('—');
    expect(formatVietnamesePercent(0, 2)).toBe('0,00%');
    expect(formatVietnamesePercent(0.125, 2)).toBe('12,50%');
    expect(formatVietnamesePercent(0.125, 2, true)).toBe('+12,50%');
    expect(formatVietnamesePercent(-0.052, 2)).toBe('-5,20%');
  });
});
