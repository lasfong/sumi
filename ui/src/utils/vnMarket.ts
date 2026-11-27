import type { IntervalKey, OhlcvBar } from '../types/market'

export type TimeframeVisibilityPreset = 'all' | 'intraday' | 'swing' | 'current'

export const INTRADAY_INTERVALS: IntervalKey[] = ['1m', '5m', '15m', '1h', '4h']
export const SWING_INTERVALS: IntervalKey[] = ['1D', '1W', '1M']

/**
 * Returns true if `ticker` belongs to a Vietnamese exchange (`HOSE`, `HNX`, `UPCOM`).
 */
export function isVietnamTicker(ticker: string): boolean {
  const upper = ticker.trim().toUpperCase()
  return upper.startsWith('HOSE:') || upper.startsWith('HNX:') || upper.startsWith('UPCOM:')
}

/**
 * Returns true if `ticker` is a VN30 Index Futures continuous/expiry contract (`HNX:VN30F1!`).
 */
export function isVn30FuturesTicker(ticker: string): boolean {
  const upper = ticker.trim().toUpperCase()
  return upper.includes('VN30F') || upper.includes('VN301!')
}

/**
 * Snaps an arbitrary dragged or clicked price to the statutory venue tick size:
 * - `HNX:VN30F*`: `0.1` index point
 * - `HOSE` stocks: `< 10,000 => 10 VND`, `10,000..49,950 => 50 VND`, `>= 50,000 => 100 VND`
 * - `HNX` / `UPCOM` stocks: `100 VND`
 * - Crypto / US / FX: rounded cleanly by magnitude
 */
export function snapPriceToVenueTick(ticker: string, rawPrice: number): number {
  if (!Number.isFinite(rawPrice) || rawPrice <= 0) return rawPrice
  const upper = ticker.trim().toUpperCase()
  if (isVn30FuturesTicker(upper)) {
    return Number((Math.round(rawPrice * 10) / 10).toFixed(1))
  }
  if (upper.startsWith('HOSE:')) {
    const step = rawPrice < 10_000 ? 10 : rawPrice < 50_000 ? 50 : 100
    return Math.round(rawPrice / step) * step
  }
  if (upper.startsWith('HNX:') || upper.startsWith('UPCOM:')) {
    return Math.round(rawPrice / 100) * 100
  }
  if (rawPrice >= 10) return Number(rawPrice.toFixed(2))
  if (rawPrice >= 1) return Number(rawPrice.toFixed(3))
  return Number(rawPrice.toFixed(4))
}

/**
 * Evaluates whether a drawing should be visible on `currentInterval` based on its
 * `extendData.visibleTimeframes` metadata (TradingView Visibility by Timeframe, v2.2).
 */
export function isDrawingVisibleOnInterval(
  extendData: unknown,
  currentInterval: IntervalKey,
): boolean {
  if (!extendData || typeof extendData !== 'object') return true
  const raw = (extendData as Record<string, unknown>).visibleTimeframes
  if (!raw || raw === 'all') return true
  if (raw === 'intraday') return INTRADAY_INTERVALS.includes(currentInterval)
  if (raw === 'swing') return SWING_INTERVALS.includes(currentInterval)
  if (Array.isArray(raw)) {
    if (raw.length === 0) return true
    return raw.includes(currentInterval)
  }
  return true
}

export interface VnSessionBreakMarker {
  timestamp: number
  label: string
  kind: 'morning' | 'afternoon' | 'atc'
}

/**
 * Detects Vietnam intraday session transitions (`09:00` Morning Open, `13:00` Afternoon Open,
 * `14:30` ATC) in ICT (`UTC+7`) across visible intraday bars (`1m`, `5m`, `15m`, `1h`).
 */
export function computeVnSessionBreaks(
  bars: Pick<OhlcvBar, 'timestamp'>[],
  interval: IntervalKey,
  maxMarkers = 45,
): VnSessionBreakMarker[] {
  if (!['1m', '5m', '15m', '1h'].includes(interval) || bars.length < 2) {
    return []
  }
  const ICT_OFFSET_MS = 7 * 3_600_000
  const out: VnSessionBreakMarker[] = []
  let prevDay = -1
  let prevSession = ''

  for (const bar of bars) {
    const ictMs = bar.timestamp + ICT_OFFSET_MS
    const day = Math.floor(ictMs / 86_400_000)
    const minuteOfDay = Math.floor((ictMs % 86_400_000 + 86_400_000) % 86_400_000 / 60_000)
    const session =
      minuteOfDay < 12 * 60
        ? 'morning'
        : minuteOfDay >= 14 * 60 + 30
          ? 'atc'
          : 'afternoon'

    if (prevDay !== -1) {
      if (day !== prevDay) {
        const d = new Date(ictMs)
        const dd = String(d.getUTCDate()).padStart(2, '0')
        const mm = String(d.getUTCMonth() + 1).padStart(2, '0')
        out.push({ timestamp: bar.timestamp, label: `${dd}/${mm} 09:00`, kind: 'morning' })
      } else if (session === 'afternoon' && prevSession === 'morning') {
        out.push({ timestamp: bar.timestamp, label: '13:00 Chiều', kind: 'afternoon' })
      } else if (session === 'atc' && prevSession !== 'atc' && interval !== '1h') {
        out.push({ timestamp: bar.timestamp, label: '14:30 ATC', kind: 'atc' })
      }
    }
    prevDay = day
    prevSession = session
  }

  return out.length > maxMarkers ? out.slice(out.length - maxMarkers) : out
}
