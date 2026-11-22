import type { Period } from 'klinecharts'
import type { IntervalKey } from '../types/market'

export const ALL_INTERVALS: Array<{ key: IntervalKey; label: string; group: 'Phút' | 'Giờ' | 'Ngày' }> = [
  { key: '1m', label: '1m', group: 'Phút' },
  { key: '5m', label: '5m', group: 'Phút' },
  { key: '15m', label: '15m', group: 'Phút' },
  { key: '30m', label: '30m', group: 'Phút' },
  { key: '1h', label: '1H', group: 'Giờ' },
  { key: '4h', label: '4H', group: 'Giờ' },
  { key: '1D', label: '1D', group: 'Ngày' },
  { key: '1W', label: '1W', group: 'Ngày' },
  { key: '1M', label: '1M', group: 'Ngày' },
]

export const QUICK_INTERVALS: IntervalKey[] = ['1m', '5m', '15m', '1h', '4h', '1D', '1W']

export function intervalToKLinePeriod(interval: IntervalKey): Period {
  switch (interval) {
    case '1m':
      return { span: 1, type: 'minute' }
    case '5m':
      return { span: 5, type: 'minute' }
    case '15m':
      return { span: 15, type: 'minute' }
    case '30m':
      return { span: 30, type: 'minute' }
    case '1h':
      return { span: 1, type: 'hour' }
    case '4h':
      return { span: 4, type: 'hour' }
    case '1D':
      return { span: 1, type: 'day' }
    case '1W':
      return { span: 1, type: 'week' }
    case '1M':
      return { span: 1, type: 'month' }
  }
}

export function intervalToDurationMs(interval: IntervalKey): number {
  switch (interval) {
    case '1m':
      return 60_000
    case '5m':
      return 300_000
    case '15m':
      return 900_000
    case '30m':
      return 1_800_000
    case '1h':
      return 3_600_000
    case '4h':
      return 14_400_000
    case '1D':
      return 86_400_000
    case '1W':
      return 604_800_000
    case '1M':
      return 2_592_000_000
  }
}

export function formatBarCountdown(barTimestampMs: number, interval: IntervalKey): string {
  const duration = intervalToDurationMs(interval)
  const closeAt = barTimestampMs + duration
  const remainingMs = closeAt - Date.now()
  if (!Number.isFinite(remainingMs) || remainingMs <= 0) {
    return ''
  }
  const totalSecs = Math.floor(remainingMs / 1000)
  const days = Math.floor(totalSecs / 86400)
  const hours = Math.floor((totalSecs % 86400) / 3600)
  const mins = Math.floor((totalSecs % 3600) / 60)
  const secs = totalSecs % 60
  const pad = (n: number) => String(n).padStart(2, '0')
  if (days > 0) {
    return `${days}d ${pad(hours)}:${pad(mins)}`
  }
  if (hours > 0) {
    return `${pad(hours)}:${pad(mins)}:${pad(secs)}`
  }
  return `${pad(mins)}:${pad(secs)}`
}

export function formatDurationVietnamese(ms: number): string {
  const abs = Math.abs(ms)
  const mins = Math.floor(abs / 60_000)
  if (mins < 60) return `${mins} phút`
  const hours = Math.floor(mins / 60)
  if (hours < 24) return `${hours}g ${mins % 60}p`
  const days = Math.floor(hours / 24)
  if (days < 30) return `${days} ngày`
  const months = Math.floor(days / 30)
  const remDays = days % 30
  return remDays > 0 ? `${months} tháng ${remDays} ngày` : `${months} tháng`
}
