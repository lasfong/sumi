/**
 * Shared Vietnamese locale and formatting utilities for Sumi.
 * Strict compliance with PRO-UX-05:
 * - vi-VN locale
 * - Asia/Ho_Chi_Minh timezone
 * - dd/MM/yyyy date format
 * - Unambiguous VND currency and percent labels
 */

const VIETNAM_TIMEZONE = 'Asia/Ho_Chi_Minh';
const VIETNAM_LOCALE = 'vi-VN';

/**
 * Format a date string, Date object, or timestamp to dd/MM/yyyy in Asia/Ho_Chi_Minh timezone.
 */
export function formatVietnameseDate(value: string | Date | number | null | undefined): string {
  if (!value) return '—';
  try {
    const date = typeof value === 'string' || typeof value === 'number' ? new Date(value) : value;
    if (isNaN(date.getTime())) {
      // Fallback for simple date keys YYYY-MM-DD
      if (typeof value === 'string' && /^\d{4}-\d{2}-\d{2}$/.test(value)) {
        const [y, m, d] = value.split('-');
        return `${d}/${m}/${y}`;
      }
      return '—';
    }
    const formatter = new Intl.DateTimeFormat(VIETNAM_LOCALE, {
      timeZone: VIETNAM_TIMEZONE,
      day: '2-digit',
      month: '2-digit',
      year: 'numeric',
    });
    return formatter.format(date);
  } catch {
    return '—';
  }
}

/**
 * Format a date-time string, Date object, or timestamp to dd/MM/yyyy HH:mm in Asia/Ho_Chi_Minh.
 */
export function formatVietnameseDateTime(value: string | Date | number | null | undefined): string {
  if (!value) return '—';
  try {
    const date = typeof value === 'string' || typeof value === 'number' ? new Date(value) : value;
    if (isNaN(date.getTime())) return '—';
    const formatter = new Intl.DateTimeFormat(VIETNAM_LOCALE, {
      timeZone: VIETNAM_TIMEZONE,
      day: '2-digit',
      month: '2-digit',
      year: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
      hour12: false,
    });
    return formatter.format(date);
  } catch {
    return '—';
  }
}

/**
 * Format a number as VND currency (e.g., 100.000.000 ₫).
 */
export function formatVnd(value: number | null | undefined): string {
  if (value === null || value === undefined || isNaN(value)) return '—';
  try {
    const formatted = new Intl.NumberFormat(VIETNAM_LOCALE, {
      maximumFractionDigits: 0,
    }).format(value);
    return `${formatted} ₫`;
  } catch {
    return `${value} ₫`;
  }
}

/**
 * Format a number with vi-VN locale separators (dots for thousands, comma for decimals).
 */
export function formatVietnameseNumber(value: number | null | undefined, decimals: number = 2): string {
  if (value === null || value === undefined || isNaN(value)) return '—';
  try {
    return new Intl.NumberFormat(VIETNAM_LOCALE, {
      minimumFractionDigits: decimals,
      maximumFractionDigits: decimals,
    }).format(value);
  } catch {
    return value.toFixed(decimals);
  }
}

/**
 * Format a percentage with vi-VN locale (e.g. +12,50% or -5,20%).
 */
export function formatVietnamesePercent(
  value: number | null | undefined,
  decimals: number = 2,
  includeSign: boolean = false
): string {
  if (value === null || value === undefined || isNaN(value)) return '—';
  try {
    const formatted = new Intl.NumberFormat(VIETNAM_LOCALE, {
      minimumFractionDigits: decimals,
      maximumFractionDigits: decimals,
    }).format(value * 100);
    const sign = includeSign && value > 0 ? '+' : '';
    return `${sign}${formatted}%`;
  } catch {
    return `${value}%`;
  }
}

/**
 * Format integer volume with vi-VN separators.
 */
export function formatVietnameseVolume(value: number | null | undefined): string {
  if (value === null || value === undefined || isNaN(value)) return '—';
  try {
    return new Intl.NumberFormat(VIETNAM_LOCALE, {
      maximumFractionDigits: 0,
    }).format(value);
  } catch {
    return String(value);
  }
}
