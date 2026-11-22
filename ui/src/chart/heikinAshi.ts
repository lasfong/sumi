import type { KLineData } from 'klinecharts'

/**
 * Pure causal Heikin-Ashi transformation for KLineChart v10 bars.
 * Preserves timestamp, volume, and turnover while computing smoothed HA OHLC.
 */
export function toHeikinAshi(bars: KLineData[], prevHa?: { open: number; close: number }): KLineData[] {
  if (bars.length === 0) return []
  const out: KLineData[] = new Array(bars.length)
  let prevOpen = prevHa?.open ?? (bars[0].open + bars[0].close) / 2
  let prevClose = prevHa?.close ?? (bars[0].open + bars[0].high + bars[0].low + bars[0].close) / 4

  for (let i = 0; i < bars.length; i++) {
    const b = bars[i]
    const haClose = (b.open + b.high + b.low + b.close) / 4
    const haOpen = i === 0 && !prevHa ? (b.open + b.close) / 2 : (prevOpen + prevClose) / 2
    const haHigh = Math.max(b.high, haOpen, haClose)
    const haLow = Math.min(b.low, haOpen, haClose)
    out[i] = {
      timestamp: b.timestamp,
      open: haOpen,
      high: haHigh,
      low: haLow,
      close: haClose,
      volume: b.volume,
      turnover: b.turnover,
    }
    prevOpen = haOpen
    prevClose = haClose
  }
  return out
}
