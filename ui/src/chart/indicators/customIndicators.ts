/**
 * QMV Workstation Quantitative Custom Indicators matching `sumi-core` exact algorithms for KLineChart v10.1.1:
 *  - ICHIMOKU   (QMV V7 default: Tenkan 6, Kijun 18, Senkou B 36, Displacement 18 + 18-bar Future Kumo projection!)
 *  - QMV_FLOW   (Causal OHLCV Money-Flow Cycle Proxy `Z-50` [0..100] across T+ / Swing horizons `[3, 5, 10, 20]` + 20/30/50/70/80 guides)
 *  - RS_VNINDEX (Causal Relative Strength vs `HOSE:VNINDEX` normalized to 100 + SMA(20) signal line)
 *  - VRVP       (Visible-Range Volume Profile horizontal histogram + Point of Control `POC` line on price pane)
 *  - RSI        (Single-line Wilder `RSI(14)` + 30/50/70 reference lines)
 *  - CCI        (Lambert `CCI(14)` + -100/0/+100 reference lines)
 *  - MACD       (TradingView / QMV standard: `Histogram = DIF - DEA`, NOT Tongdaxin `*2`!)
 *  - VWAP       (Median-Δt Intraday/Session VWAP + ±1σ / ±2σ weighted Welford variance bands)
 *  - ATR        (Wilder Average True Range, period 14)
 */
import type {
  Chart,
  Indicator,
  IndicatorFigureStyle,
  IndicatorTemplate,
  KLineData,
  TooltipFeatureStyle,
  TooltipLegend,
} from 'klinecharts'
import { computeAnalogCone } from '../../api/tauri'
import type { AnalogConeResponseDto } from '../../types/market'

export interface IchimokuPoint {
  tenkan?: number
  kijun?: number
  senkouA?: number
  senkouB?: number
  chikou?: number
  rawFutureA?: number
  rawFutureB?: number
}

export interface QmvFlowPoint {
  f1?: number
  f2?: number
  f3?: number
  f4?: number
}

export interface RsVnindexPoint {
  rs?: number
  rsMa?: number
}

export interface VrvpPoint {
  poc?: number
}

export interface RsiSinglePoint {
  rsi1?: number
  rsi2?: number
  rsi3?: number
}

export interface CciPoint {
  cci?: number
}

export interface VwapPoint {
  vwap?: number
  upper1?: number
  lower1?: number
  upper2?: number
  lower2?: number
}

export interface AtrPoint {
  atr?: number
}

export interface MacdPoint {
  dif?: number
  dea?: number
  macd?: number
}

export interface AfcPoint {
  anchor?: number
}

// ─── AFC Forecast Cache & Non-Blocking Request Deduplicator ───────────────────
const afcForecastCache = new Map<string, AnalogConeResponseDto>()
const afcInFlight = new Map<string, Promise<void>>()

export function clearAfcForecastCache(): void {
  afcForecastCache.clear()
  afcInFlight.clear()
}

function normalizeAfcParams(calcParams: ReadonlyArray<unknown>): {
  horizon: number
  topK: number
  useMarketContext: boolean
} {
  const rawH = Number(calcParams[0] ?? 10)
  const rawK = Number(calcParams[1] ?? 100)
  const rawMkt = Number(calcParams[2] ?? 1)
  const horizon = Math.max(1, Math.min(20, Math.round(Number.isFinite(rawH) ? rawH : 10)))
  const topK = Math.max(10, Math.min(250, Math.round(Number.isFinite(rawK) ? rawK : 100)))
  const useMarketContext = Math.round(Number.isFinite(rawMkt) ? rawMkt : 1) > 0
  return { horizon, topK, useMarketContext }
}

function resolveChartIntervalKey(chart: Chart): string {
  const p = chart.getPeriod()
  if (!p) return '1D'
  if (p.type === 'day' && p.span === 1) return '1D'
  if (p.type === 'week') return `${p.span}W`
  if (p.type === 'month') return `${p.span}M`
  if (p.type === 'hour') return `${p.span}h`
  if (p.type === 'minute') return `${p.span}m`
  return `${p.span}${p.type}`
}

function ensureAfcForecast(
  chart: Chart,
  paneId: string,
  ticker: string,
  interval: string,
  asOfTimestampMs: number,
  horizon: number,
  topK: number,
  useMarketContext: boolean,
): AnalogConeResponseDto | undefined {
  const key = `${ticker.toUpperCase()}|${interval}|${asOfTimestampMs}|${horizon}|${topK}|${useMarketContext ? 1 : 0}`
  const hit = afcForecastCache.get(key)
  if (hit) return hit

  if (!afcInFlight.has(key)) {
    const req = computeAnalogCone({
      ticker,
      interval,
      horizon,
      topK,
      useMarketContext,
      asOfTimestampMs,
    })
      .then((res) => {
        if (afcForecastCache.size >= 120) {
          const oldest = afcForecastCache.keys().next().value
          if (oldest !== undefined) afcForecastCache.delete(oldest)
        }
        afcForecastCache.set(key, res)
        const updatable = chart as Chart & { updatePane?: (level?: number, id?: string) => void }
        if (typeof updatable.updatePane === 'function') {
          // `UpdateLevel.Drawer = 3` repaints both main indicator canvas and study legend immediately!
          updatable.updatePane(3, paneId)
        } else {
          chart.resize()
        }
      })
      .catch(() => {})
      .finally(() => {
        afcInFlight.delete(key)
      })
    afcInFlight.set(key, req)
  }
  return undefined
}

function formatSignedPct(val: number, digits = 1): string {
  const sign = val >= 0 ? '+' : ''
  return `${sign}${val.toFixed(digits)}%`
}

function formatIsoToViDate(iso: string): string {
  const parts = iso.split('-')
  if (parts.length === 3) {
    return `${parts[2]}/${parts[1]}/${parts[0]}`
  }
  return iso
}

function extractTooltipFeatures<D>(indicator: Indicator<D>): TooltipFeatureStyle[] {
  const styles = indicator.styles as { tooltip?: { features?: TooltipFeatureStyle[] } } | null
  return styles?.tooltip?.features ?? []
}

// Module-level VNINDEX benchmark daily close cache (keyed by ICT trading day index `floor((ts + 7h)/86400s)`)
const vnindexCloseByIctDay = new Map<number, number>()
const vnindexSortedDays: Array<{ dayKey: number; close: number }> = []

function ictDayKey(timestampMs: number): number {
  return Math.floor((timestampMs + 7 * 3_600_000) / 86_400_000)
}

export function setVnindexBenchmarkBars(bars: KLineData[]): void {
  if (!Array.isArray(bars) || bars.length === 0) return
  vnindexCloseByIctDay.clear()
  vnindexSortedDays.length = 0
  for (const b of bars) {
    if (Number.isFinite(b.close) && b.close > 0 && Number.isFinite(b.timestamp)) {
      const dk = ictDayKey(b.timestamp)
      vnindexCloseByIctDay.set(dk, b.close)
    }
  }
  const keys = Array.from(vnindexCloseByIctDay.keys()).sort((a, b) => a - b)
  for (const dk of keys) {
    vnindexSortedDays.push({ dayKey: dk, close: vnindexCloseByIctDay.get(dk)! })
  }
}

export function hasVnindexBenchmarkBars(): boolean {
  return vnindexSortedDays.length >= 20
}

/** Causal point-in-time lookup: exact ICT day match, or most recent prior VNINDEX session close (never future!). */
function lookupCausalVnindexClose(timestampMs: number): number | undefined {
  const dk = ictDayKey(timestampMs)
  const exact = vnindexCloseByIctDay.get(dk)
  if (exact !== undefined) return exact
  if (vnindexSortedDays.length === 0) return undefined
  let lo = 0
  let hi = vnindexSortedDays.length - 1
  let best: number | undefined
  while (lo <= hi) {
    const mid = (lo + hi) >> 1
    if (vnindexSortedDays[mid].dayKey <= dk) {
      best = vnindexSortedDays[mid].close
      lo = mid + 1
    } else {
      hi = mid - 1
    }
  }
  return best
}

function userLineStyle<D, C = unknown>(
  indicator: Indicator<D, C>,
  idx: number,
  fallbackColor: string,
  fallbackSize: number,
  fallbackStyle: 'solid' | 'dashed' = 'solid',
): IndicatorFigureStyle {
  const lines = (indicator.styles as { lines?: Array<{ color?: string; size?: number; style?: string }> } | null)?.lines
  const ln = lines?.[idx]
  return {
    color: ln?.color || fallbackColor,
    size: typeof ln?.size === 'number' && Number.isFinite(ln.size) ? ln.size : fallbackSize,
    style: (ln?.style === 'dashed' || ln?.style === 'solid' ? ln.style : fallbackStyle) as 'solid' | 'dashed',
  }
}

function drawDashedGuideLines(
  ctx: CanvasRenderingContext2D,
  yAxis: { convertToPixel: (val: number) => number },
  paneWidth: number,
  levels: Array<{ value: number; color: string; width?: number }>,
): void {
  ctx.save()
  ctx.setLineDash([4, 4])
  for (const lv of levels) {
    const y = yAxis.convertToPixel(lv.value)
    if (!Number.isFinite(y)) continue
    ctx.beginPath()
    ctx.strokeStyle = lv.color
    ctx.lineWidth = lv.width ?? 1
    ctx.moveTo(0, y)
    ctx.lineTo(paneWidth, y)
    ctx.stroke()
  }
  ctx.restore()
}

function donchianMid(dataList: KLineData[], endIdx: number, period: number): number | undefined {
  const p = Math.max(1, Math.round(period))
  if (endIdx < p - 1) return undefined
  let hi = -Infinity
  let lo = Infinity
  for (let j = endIdx - p + 1; j <= endIdx; j++) {
    if (dataList[j].high > hi) hi = dataList[j].high
    if (dataList[j].low < lo) lo = dataList[j].low
  }
  return (hi + lo) / 2
}

function computeWilderAtrArray(dataList: KLineData[], period: number): Array<number | undefined> {
  const n = dataList.length
  const p = Math.max(1, Math.round(period))
  const out: Array<number | undefined> = new Array(n).fill(undefined)
  if (n < p) return out

  const tr: number[] = new Array(n)
  for (let i = 0; i < n; i++) {
    const c = dataList[i]
    if (i === 0) {
      tr[i] = c.high - c.low
    } else {
      const prevClose = dataList[i - 1].close
      tr[i] = Math.max(c.high - c.low, Math.abs(c.high - prevClose), Math.abs(c.low - prevClose))
    }
  }

  let sum = 0
  for (let i = 0; i < p; i++) sum += tr[i]
  let atr = sum / p
  out[p - 1] = atr
  for (let i = p; i < n; i++) {
    atr = (atr * (p - 1) + tr[i]) / p
    out[i] = atr
  }
  return out
}

function computeWilderRsiArray(dataList: KLineData[], period: number): Array<number | undefined> {
  const n = dataList.length
  const p = Math.max(2, Math.round(period))
  const out: Array<number | undefined> = new Array(n).fill(undefined)
  if (n <= p) return out

  let gainSum = 0
  let lossSum = 0
  for (let i = 1; i <= p; i++) {
    const diff = dataList[i].close - dataList[i - 1].close
    if (diff > 0) gainSum += diff
    else lossSum += -diff
  }
  let avgGain = gainSum / p
  let avgLoss = lossSum / p
  out[p] = avgLoss === 0 ? 100 : 100 - 100 / (1 + avgGain / avgLoss)

  for (let i = p + 1; i < n; i++) {
    const diff = dataList[i].close - dataList[i - 1].close
    const gain = diff > 0 ? diff : 0
    const loss = diff < 0 ? -diff : 0
    avgGain = (avgGain * (p - 1) + gain) / p
    avgLoss = (avgLoss * (p - 1) + loss) / p
    out[i] = avgLoss === 0 ? 100 : 100 - 100 / (1 + avgGain / avgLoss)
  }
  return out
}

/** Robust median bar spacing detector immune to overnight/weekend gaps on 1h/4h equity charts (`BUG-020`). */
function detectIntradaySeries(dataList: KLineData[]): boolean {
  const n = dataList.length
  if (n < 2) return false
  const diffs: number[] = []
  const start = Math.max(1, n - 40)
  for (let i = start; i < n; i++) {
    const dt = dataList[i].timestamp - dataList[i - 1].timestamp
    if (dt > 0) diffs.push(dt)
  }
  if (diffs.length === 0) return false
  diffs.sort((a, b) => a - b)
  const q1 = diffs[Math.floor(diffs.length * 0.25)]
  return q1 < 43_200_000
}

/**
 * QMV V7 Ichimoku (`6, 18, 36, disp=18` default; also supports `3, 6, 18, 18` or `9, 26, 52, 26`)
 * WITH full 18-bar Future Kumo Cloud + Span A/B lines projected into right-hand chart whitespace!
 */
export const ichimokuIndicator: IndicatorTemplate<IchimokuPoint, number> = {
  name: 'ICHIMOKU',
  shortName: 'Ichimoku QMV',
  series: 'price',
  calcParams: [6, 18, 36, 18],
  figures: [
    { key: 'tenkan', title: 'Tenkan: ', type: 'line', styles: ({ indicator }) => userLineStyle(indicator, 0, '#2962FF', 1.35) },
    { key: 'kijun', title: 'Kijun: ', type: 'line', styles: ({ indicator }) => userLineStyle(indicator, 1, '#E91E63', 1.35) },
    { key: 'senkouA', title: 'SpanA: ', type: 'line', styles: ({ indicator }) => userLineStyle(indicator, 2, 'rgba(8, 153, 129, 0.88)', 1.05) },
    { key: 'senkouB', title: 'SpanB: ', type: 'line', styles: ({ indicator }) => userLineStyle(indicator, 3, 'rgba(242, 54, 69, 0.88)', 1.05) },
    { key: 'chikou', title: 'Chikou: ', type: 'line', styles: ({ indicator }) => userLineStyle(indicator, 4, '#9C27B0', 1, 'dashed') },
  ],
  calc: (dataList, indicator) => {
    const [tenkanP = 6, kijunP = 18, senkouBP = 36, dispParam] = indicator.calcParams
    const displacement = Math.max(1, Math.round(dispParam ?? kijunP))
    const n = dataList.length
    const result: IchimokuPoint[] = Array.from({ length: n }, () => ({}))

    const rawA: Array<number | undefined> = new Array(n).fill(undefined)
    const rawB: Array<number | undefined> = new Array(n).fill(undefined)

    for (let i = 0; i < n; i++) {
      const t = donchianMid(dataList, i, tenkanP)
      const k = donchianMid(dataList, i, kijunP)
      result[i].tenkan = t
      result[i].kijun = k
      if (t !== undefined && k !== undefined) {
        rawA[i] = (t + k) / 2
      }
      rawB[i] = donchianMid(dataList, i, senkouBP)
      result[i].rawFutureA = rawA[i]
      result[i].rawFutureB = rawB[i]
      if (i >= displacement) {
        result[i - displacement].chikou = dataList[i].close
      }
    }

    for (let i = 0; i < n; i++) {
      const src = i - displacement
      if (src >= 0) {
        result[i].senkouA = rawA[src]
        result[i].senkouB = rawB[src]
      }
    }
    return result
  },
  draw: ({ ctx, chart, indicator, xAxis, yAxis }) => {
    const result = indicator.result
    const n = result.length
    if (n === 0) return false
    const range = chart.getVisibleRange()
    const from = Math.max(0, range.from - 1)
    const to = Math.min(n - 1, range.to + 1)

    // 1. Historical Kumo cloud up to bar `n - 1`
    for (let i = from; i < to; i++) {
      const curr = result[i]
      const next = result[i + 1]
      if (
        curr?.senkouA !== undefined &&
        curr?.senkouB !== undefined &&
        next?.senkouA !== undefined &&
        next?.senkouB !== undefined
      ) {
        const x1 = xAxis.convertToPixel(i)
        const x2 = xAxis.convertToPixel(i + 1)
        const y1a = yAxis.convertToPixel(curr.senkouA)
        const y1b = yAxis.convertToPixel(curr.senkouB)
        const y2a = yAxis.convertToPixel(next.senkouA)
        const y2b = yAxis.convertToPixel(next.senkouB)
        const bullish = curr.senkouA >= curr.senkouB

        ctx.beginPath()
        ctx.moveTo(x1, y1a)
        ctx.lineTo(x2, y2a)
        ctx.lineTo(x2, y2b)
        ctx.lineTo(x1, y1b)
        ctx.closePath()
        ctx.fillStyle = bullish ? 'rgba(8, 153, 129, 0.14)' : 'rgba(242, 54, 69, 0.14)'
        ctx.fill()
      }
    }

    // 2. Future Kumo Cloud & Span A/B projection (`n - 1 .. n - 1 + displacement`) in right-hand whitespace
    const [, kijunP = 18, , dispParam] = indicator.calcParams
    const displacement = Math.max(1, Math.round(dispParam ?? kijunP))
    if (range.to >= n - 2 && n > displacement) {
      const spanAStyle = userLineStyle(indicator, 2, 'rgba(8, 153, 129, 0.88)', 1.05)
      const spanBStyle = userLineStyle(indicator, 3, 'rgba(242, 54, 69, 0.88)', 1.05)

      for (let k = 0; k < displacement; k++) {
        const src1 = n - 1 - displacement + k
        const src2 = src1 + 1
        if (src1 < 0 || src2 >= n) continue
        const a1 = result[src1]?.rawFutureA
        const b1 = result[src1]?.rawFutureB
        const a2 = result[src2]?.rawFutureA
        const b2 = result[src2]?.rawFutureB
        if (a1 === undefined || b1 === undefined || a2 === undefined || b2 === undefined) continue

        const x1 = xAxis.convertToPixel(n - 1 + k)
        const x2 = xAxis.convertToPixel(n - 1 + k + 1)
        const y1a = yAxis.convertToPixel(a1)
        const y1b = yAxis.convertToPixel(b1)
        const y2a = yAxis.convertToPixel(a2)
        const y2b = yAxis.convertToPixel(b2)
        const bullish = a1 >= b1

        ctx.beginPath()
        ctx.moveTo(x1, y1a)
        ctx.lineTo(x2, y2a)
        ctx.lineTo(x2, y2b)
        ctx.lineTo(x1, y1b)
        ctx.closePath()
        ctx.fillStyle = bullish ? 'rgba(8, 153, 129, 0.16)' : 'rgba(242, 54, 69, 0.16)'
        ctx.fill()

        ctx.save()
        ctx.beginPath()
        ctx.strokeStyle = spanAStyle.color ?? 'rgba(8, 153, 129, 0.88)'
        ctx.lineWidth = spanAStyle.size ?? 1.05
        ctx.moveTo(x1, y1a)
        ctx.lineTo(x2, y2a)
        ctx.stroke()

        ctx.beginPath()
        ctx.strokeStyle = spanBStyle.color ?? 'rgba(242, 54, 69, 0.88)'
        ctx.lineWidth = spanBStyle.size ?? 1.05
        ctx.moveTo(x1, y1b)
        ctx.lineTo(x2, y2b)
        ctx.stroke()
        ctx.restore()
      }
    }
    return false
  },
}

/**
 * QMV Multi-Horizon Money-Flow Cycle Proxy (`QMV_FLOW`, strictly causal `bb_v1_ohlcv_proxy` + 50-bar `Z-50` `[0..100]`).
 * Matches `sumi_core::indicators::proxy_bb::ProxyBBSeriesOutput::z50_horizons` 1-to-1!
 * Explicitly labeled `OHLCV Proxy` (never claimed as proprietary QMV Webapp Blackbox).
 */
export const qmvFlowIndicator: IndicatorTemplate<QmvFlowPoint, number> = {
  name: 'QMV_FLOW',
  shortName: 'QMV Flow (OHLCV Proxy)',
  series: 'normal',
  precision: 1,
  minValue: 0,
  maxValue: 100,
  calcParams: [3, 5, 10, 20],
  figures: [
    { key: 'f1', title: 'T+ (3): ', type: 'line', styles: ({ indicator }) => userLineStyle(indicator, 0, '#00BCD4', 1.2) },
    { key: 'f2', title: 'T+2 (5): ', type: 'line', styles: ({ indicator }) => userLineStyle(indicator, 1, '#2962FF', 1.55) },
    { key: 'f3', title: 'Ngắn (10): ', type: 'line', styles: ({ indicator }) => userLineStyle(indicator, 2, '#FF9800', 1.35) },
    { key: 'f4', title: 'Sóng (20): ', type: 'line', styles: ({ indicator }) => userLineStyle(indicator, 3, '#E91E63', 1.7) },
  ],
  calc: (dataList, indicator) => {
    const [p1 = 3, p2 = 5, p3 = 10, p4 = 20] = indicator.calcParams
    const horizons = [p1, p2, p3, p4].map((p) => Math.max(2, Math.round(p)))
    const n = dataList.length
    const out: QmvFlowPoint[] = Array.from({ length: n }, () => ({}))
    if (n === 0) return out

    // Step 1: Decompose true-range order imbalance pressure (`AT15` exact match with `sumi-core`)
    const spArr = new Float64Array(n)
    const tvArr = new Float64Array(n)
    let prevValidClose: number | undefined
    for (let i = 0; i < n; i++) {
      const b = dataList[i]
      const trueHigh = prevValidClose !== undefined ? Math.max(b.high, prevValidClose) : b.high
      const trueLow = prevValidClose !== undefined ? Math.min(b.low, prevValidClose) : b.low
      const trSpan = trueHigh - trueLow
      const eps = 1e-12 * Math.max(Math.abs(trueHigh) + Math.abs(trueLow), 1e-30)
      const pressure = trSpan <= eps ? 0 : Math.max(-1, Math.min(1, (2 * b.close - trueHigh - trueLow) / trSpan))
      const tp = Math.max(0, (b.high + b.low + b.close) / 3)
      const vol = Number.isFinite(b.volume) && (b.volume ?? 0) > 0 ? b.volume! : 0
      const tv = tp > 0 && vol > 0 ? tp * vol : 0
      spArr[i] = pressure * tv
      tvArr[i] = tv
      if (Number.isFinite(b.close)) prevValidClose = b.close
    }

    // Step 2: Rolling `bb_val = 50 * (1 + spSum / tvSum)` + causal 50-bar `Z-50` min-max `[0..100]`
    const Z50_WIN = 50
    const Z50_MIN_VALID = 15
    const zSeries: Array<Array<number | undefined>> = horizons.map((lookback) => {
      const rawBb: Array<number | undefined> = new Array(n).fill(undefined)
      for (let t = lookback - 1; t < n; t++) {
        let spSum = 0
        let tvSum = 0
        for (let j = t + 1 - lookback; j <= t; j++) {
          spSum += spArr[j]
          tvSum += tvArr[j]
        }
        if (tvSum > 0) {
          const oib = Math.max(-1, Math.min(1, spSum / tvSum))
          rawBb[t] = Math.max(0, Math.min(100, 50 * (1 + oib)))
        }
      }

      const zOut: Array<number | undefined> = new Array(n).fill(undefined)
      for (let t = 0; t < n; t++) {
        const curr = rawBb[t]
        if (curr === undefined) continue
        const wStart = Math.max(0, t + 1 - Z50_WIN)
        let lo = Infinity
        let hi = -Infinity
        let validCnt = 0
        for (let k = wStart; k <= t; k++) {
          const v = rawBb[k]
          if (v !== undefined) {
            if (v < lo) lo = v
            if (v > hi) hi = v
            validCnt++
          }
        }
        if (validCnt >= Z50_MIN_VALID) {
          const span = hi - lo
          zOut[t] = span > 1e-6 ? Math.max(0, Math.min(100, ((curr - lo) / span) * 100)) : curr
        } else {
          zOut[t] = curr
        }
      }
      return zOut
    })

    for (let i = 0; i < n; i++) {
      out[i] = {
        f1: zSeries[0][i],
        f2: zSeries[1][i],
        f3: zSeries[2][i],
        f4: zSeries[3][i],
      }
    }
    return out
  },
  draw: ({ ctx, bounding, yAxis }) => {
    const y20 = yAxis.convertToPixel(20)
    const y30 = yAxis.convertToPixel(30)
    const y70 = yAxis.convertToPixel(70)
    const y80 = yAxis.convertToPixel(80)
    ctx.save()
    ctx.fillStyle = 'rgba(8, 153, 129, 0.07)'
    ctx.fillRect(0, Math.min(y20, y30), bounding.width, Math.abs(y20 - y30))
    ctx.fillStyle = 'rgba(242, 54, 69, 0.07)'
    ctx.fillRect(0, Math.min(y70, y80), bounding.width, Math.abs(y70 - y80))
    ctx.restore()
    drawDashedGuideLines(ctx, yAxis, bounding.width, [
      { value: 20, color: 'rgba(8, 153, 129, 0.45)' },
      { value: 30, color: 'rgba(8, 153, 129, 0.65)' },
      { value: 50, color: 'rgba(148, 163, 184, 0.35)' },
      { value: 70, color: 'rgba(242, 54, 69, 0.65)' },
      { value: 80, color: 'rgba(242, 54, 69, 0.45)' },
    ])
    return false
  },
}

/**
 * Causal Relative Strength vs `HOSE:VNINDEX` (`RS_VNINDEX`, normalized to `100` at series start + `SMA(20)`).
 * Rising `RS > MA(RS)` indicates the stock is outperforming `VNINDEX` (`Cổ phiếu khỏe hơn thị trường chung`).
 */
export const rsVnindexIndicator: IndicatorTemplate<RsVnindexPoint, number> = {
  name: 'RS_VNINDEX',
  shortName: 'RS vs VNINDEX',
  series: 'normal',
  precision: 2,
  calcParams: [20],
  figures: [
    { key: 'rs', title: 'RS: ', type: 'line', styles: ({ indicator }) => userLineStyle(indicator, 0, '#2962FF', 1.65) },
    { key: 'rsMa', title: 'MA: ', type: 'line', styles: ({ indicator }) => userLineStyle(indicator, 1, '#FF9800', 1.25, 'dashed') },
  ],
  calc: (dataList, indicator) => {
    const [maP = 20] = indicator.calcParams
    const win = Math.max(2, Math.round(maP))
    const n = dataList.length
    const out: RsVnindexPoint[] = Array.from({ length: n }, () => ({}))
    if (n === 0) return out

    const rawRatio: Array<number | undefined> = new Array(n).fill(undefined)
    let baseRatio: number | undefined
    const hasBench = hasVnindexBenchmarkBars()

    for (let i = 0; i < n; i++) {
      const c = dataList[i].close
      if (!Number.isFinite(c) || c <= 0) continue
      if (hasBench) {
        const bench = lookupCausalVnindexClose(dataList[i].timestamp)
        if (bench !== undefined && bench > 0) {
          const r = c / bench
          if (baseRatio === undefined) baseRatio = r
          rawRatio[i] = (r / baseRatio) * 100
          continue
        }
      }
      // Fallback before async VNINDEX cache finishes: causal price vs rolling win-bar anchor
      const anchorIdx = Math.max(0, i - win)
      const anchor = dataList[anchorIdx].close
      if (anchor > 0) {
        rawRatio[i] = (c / anchor) * 100
      }
    }

    for (let i = 0; i < n; i++) {
      const rs = rawRatio[i]
      let rsMa: number | undefined
      if (i >= win - 1) {
        let sum = 0
        let cnt = 0
        for (let j = i - win + 1; j <= i; j++) {
          if (rawRatio[j] !== undefined) {
            sum += rawRatio[j]!
            cnt++
          }
        }
        if (cnt === win) rsMa = sum / win
      }
      out[i] = { rs, rsMa }
    }
    return out
  },
}

/**
 * Visible-Range Volume Profile (`VRVP`) + Point of Control (`POC`) overlaid on the main price pane.
 * Helps QMV traders immediately identify hard support/resistance congestion zones (`KT01`).
 */
export const vrvpIndicator: IndicatorTemplate<VrvpPoint, number> = {
  name: 'VRVP',
  shortName: 'VRVP + POC',
  series: 'price',
  calcParams: [24],
  figures: [],
  calc: (dataList) => dataList.map(() => ({})),
  draw: ({ ctx, chart, indicator, bounding, yAxis }) => {
    const dataList = chart.getDataList()
    const n = dataList.length
    if (n === 0) return false
    const range = chart.getVisibleRange()
    const from = Math.max(0, range.from)
    const to = Math.min(n - 1, range.to)
    if (to < from) return false

    const bins = Math.max(8, Math.min(64, Math.round(indicator.calcParams[0] ?? 24)))
    let lo = Infinity
    let hi = -Infinity
    for (let i = from; i <= to; i++) {
      const b = dataList[i]
      if (b.low < lo) lo = b.low
      if (b.high > hi) hi = b.high
    }
    const span = hi - lo
    if (!Number.isFinite(span) || span <= 0) return false

    const step = span / bins
    const upVol = new Float64Array(bins)
    const dnVol = new Float64Array(bins)

    for (let i = from; i <= to; i++) {
      const b = dataList[i]
      const vol = Number.isFinite(b.volume) && (b.volume ?? 0) > 0 ? b.volume! : 0
      if (vol <= 0) continue
      const isUp = b.close >= b.open
      const startBin = Math.max(0, Math.min(bins - 1, Math.floor((b.low - lo) / step)))
      const endBin = Math.max(startBin, Math.min(bins - 1, Math.floor((b.high - lo) / step)))
      const share = vol / (endBin - startBin + 1)
      for (let k = startBin; k <= endBin; k++) {
        if (isUp) upVol[k] += share
        else dnVol[k] += share
      }
    }

    let maxBinVol = 0
    let pocBin = 0
    for (let k = 0; k < bins; k++) {
      const tot = upVol[k] + dnVol[k]
      if (tot > maxBinVol) {
        maxBinVol = tot
        pocBin = k
      }
    }
    if (maxBinVol <= 0) return false

    const maxBarWidth = Math.min(132, bounding.width * 0.2)
    ctx.save()
    for (let k = 0; k < bins; k++) {
      const pLow = lo + k * step
      const pHigh = pLow + step
      const yTop = yAxis.convertToPixel(pHigh)
      const yBot = yAxis.convertToPixel(pLow)
      const h = Math.max(1.5, Math.abs(yBot - yTop) - 1)
      const y = Math.min(yTop, yBot)
      const tot = upVol[k] + dnVol[k]
      if (tot <= 0) continue
      const totalW = (tot / maxBinVol) * maxBarWidth
      const upW = (upVol[k] / tot) * totalW
      const dnW = totalW - upW
      const xRight = bounding.width

      ctx.fillStyle = k === pocBin ? 'rgba(8, 153, 129, 0.36)' : 'rgba(8, 153, 129, 0.18)'
      ctx.fillRect(xRight - totalW, y, upW, h)
      ctx.fillStyle = k === pocBin ? 'rgba(242, 54, 69, 0.36)' : 'rgba(242, 54, 69, 0.18)'
      ctx.fillRect(xRight - dnW, y, dnW, h)
    }

    // Point of Control (POC) horizontal reference line
    const pocPrice = lo + (pocBin + 0.5) * step
    const yPoc = yAxis.convertToPixel(pocPrice)
    ctx.beginPath()
    ctx.setLineDash([5, 4])
    ctx.strokeStyle = 'rgba(251, 192, 45, 0.85)'
    ctx.lineWidth = 1.25
    ctx.moveTo(0, yPoc)
    ctx.lineTo(bounding.width, yPoc)
    ctx.stroke()
    ctx.restore()
    return false
  },
}

/**
 * Single-line Wilder `RSI(14)` (`[14]` default instead of KLineChart's cluttered 3-line `[6, 14, 24]`)
 * with `30 / 50 / 70` dashed reference guides. Supports up to 3 custom periods if added by user.
 */
export const qmvRsiIndicator: IndicatorTemplate<RsiSinglePoint, number> = {
  name: 'RSI',
  shortName: 'RSI',
  series: 'normal',
  precision: 1,
  minValue: 0,
  maxValue: 100,
  calcParams: [14],
  figures: [
    { key: 'rsi1', title: 'RSI: ', type: 'line', styles: ({ indicator }) => userLineStyle(indicator, 0, '#9C27B0', 1.6) },
    { key: 'rsi2', title: 'RSI2: ', type: 'line', styles: ({ indicator }) => userLineStyle(indicator, 1, '#2962FF', 1.3) },
    { key: 'rsi3', title: 'RSI3: ', type: 'line', styles: ({ indicator }) => userLineStyle(indicator, 2, '#FF9800', 1.3) },
  ],
  calc: (dataList, indicator) => {
    const params = indicator.calcParams.length > 0 ? indicator.calcParams : [14]
    const r1 = params[0] !== undefined ? computeWilderRsiArray(dataList, params[0]) : undefined
    const r2 = params[1] !== undefined ? computeWilderRsiArray(dataList, params[1]) : undefined
    const r3 = params[2] !== undefined ? computeWilderRsiArray(dataList, params[2]) : undefined
    return dataList.map((_, i) => ({
      rsi1: r1?.[i],
      rsi2: r2?.[i],
      rsi3: r3?.[i],
    }))
  },
  draw: ({ ctx, bounding, yAxis }) => {
    drawDashedGuideLines(ctx, yAxis, bounding.width, [
      { value: 30, color: 'rgba(8, 153, 129, 0.55)' },
      { value: 50, color: 'rgba(148, 163, 184, 0.30)' },
      { value: 70, color: 'rgba(242, 54, 69, 0.55)' },
    ])
    return false
  },
}

/**
 * QMV Standard `CCI(14)` (`(TP - SMA(TP, 14)) / (0.015 * MeanDev)`) with `-100 / 0 / +100` guides.
 */
export const qmvCciIndicator: IndicatorTemplate<CciPoint, number> = {
  name: 'CCI',
  shortName: 'CCI',
  series: 'normal',
  precision: 1,
  calcParams: [14],
  figures: [
    { key: 'cci', title: 'CCI: ', type: 'line', styles: ({ indicator }) => userLineStyle(indicator, 0, '#00BCD4', 1.55) },
  ],
  calc: (dataList, indicator) => {
    const [period = 14] = indicator.calcParams
    const p = Math.max(2, Math.round(period))
    const n = dataList.length
    const out: CciPoint[] = Array.from({ length: n }, () => ({}))
    if (n < p) return out

    const tp = new Float64Array(n)
    for (let i = 0; i < n; i++) {
      tp[i] = (dataList[i].high + dataList[i].low + dataList[i].close) / 3
    }
    for (let i = p - 1; i < n; i++) {
      let sum = 0
      for (let j = i - p + 1; j <= i; j++) sum += tp[j]
      const sma = sum / p
      let mdSum = 0
      for (let j = i - p + 1; j <= i; j++) mdSum += Math.abs(tp[j] - sma)
      const md = mdSum / p
      out[i] = { cci: md > 1e-12 ? (tp[i] - sma) / (0.015 * md) : 0 }
    }
    return out
  },
  draw: ({ ctx, bounding, yAxis }) => {
    drawDashedGuideLines(ctx, yAxis, bounding.width, [
      { value: -100, color: 'rgba(8, 153, 129, 0.55)' },
      { value: 0, color: 'rgba(148, 163, 184, 0.30)' },
      { value: 100, color: 'rgba(242, 54, 69, 0.55)' },
    ])
    return false
  },
}

export const vwapIndicator: IndicatorTemplate<VwapPoint, number> = {
  name: 'VWAP',
  shortName: 'VWAP',
  series: 'price',
  calcParams: [1, 2],
  figures: [
    { key: 'vwap', title: 'VWAP: ', type: 'line', styles: ({ indicator }) => userLineStyle(indicator, 0, '#FBC02D', 1.8) },
    { key: 'upper1', title: '+1σ: ', type: 'line', styles: ({ indicator }) => userLineStyle(indicator, 1, 'rgba(251, 192, 45, 0.55)', 1, 'dashed') },
    { key: 'lower1', title: '-1σ: ', type: 'line', styles: ({ indicator }) => userLineStyle(indicator, 2, 'rgba(251, 192, 45, 0.55)', 1, 'dashed') },
    { key: 'upper2', title: '+2σ: ', type: 'line', styles: ({ indicator }) => userLineStyle(indicator, 3, 'rgba(251, 192, 45, 0.35)', 1, 'dashed') },
    { key: 'lower2', title: '-2σ: ', type: 'line', styles: ({ indicator }) => userLineStyle(indicator, 4, 'rgba(251, 192, 45, 0.35)', 1, 'dashed') },
  ],
  calc: (dataList, indicator) => {
    const [band1 = 1, band2 = 2] = indicator.calcParams
    const n = dataList.length
    const result: VwapPoint[] = Array.from({ length: n }, () => ({}))
    const isIntraday = detectIntradaySeries(dataList)

    let cumW = 0
    let mean = 0
    let m2 = 0
    let lastDayKey = -1

    for (let i = 0; i < n; i++) {
      const bar = dataList[i]
      if (isIntraday) {
        const dayKey = ictDayKey(bar.timestamp)
        if (dayKey !== lastDayKey) {
          cumW = 0
          mean = 0
          m2 = 0
          lastDayKey = dayKey
        }
      }
      const tp = (bar.high + bar.low + bar.close) / 3
      const vol = Number.isFinite(bar.volume) && (bar.volume ?? 0) > 0 ? bar.volume! : 1
      const prevMean = mean
      cumW += vol
      const delta = tp - prevMean
      mean = prevMean + (vol / cumW) * delta
      const delta2 = tp - mean
      m2 += vol * delta * delta2

      const variance = cumW > 0 ? Math.max(0, m2 / cumW) : 0
      const sd = Math.sqrt(variance)

      result[i] = {
        vwap: mean,
        upper1: mean + sd * band1,
        lower1: mean - sd * band1,
        upper2: mean + sd * band2,
        lower2: mean - sd * band2,
      }
    }
    return result
  },
}

export const atrIndicator: IndicatorTemplate<AtrPoint, number> = {
  name: 'ATR',
  shortName: 'ATR',
  series: 'normal',
  precision: 2,
  calcParams: [14],
  figures: [
    { key: 'atr', title: 'ATR: ', type: 'line', styles: ({ indicator }) => userLineStyle(indicator, 0, '#FF9800', 1.5) },
  ],
  calc: (dataList, indicator) => {
    const [period = 14] = indicator.calcParams
    const arr = computeWilderAtrArray(dataList, period)
    return arr.map((v) => (v === undefined ? {} : { atr: v }))
  },
}

/**
 * TradingView & `sumi-core` standard MACD (`Histogram = DIF - DEA`, WITHOUT Tongdaxin's `*2` factor — `BUG-010`)!
 * Overrides KLineChart's built-in `MACD` so Chart === Screener === Strategy Tester 1:1.
 */
export const tradingViewMacdIndicator: IndicatorTemplate<MacdPoint, number> = {
  name: 'MACD',
  shortName: 'MACD',
  calcParams: [12, 26, 9],
  figures: [
    { key: 'dif', title: 'DIF: ', type: 'line', styles: ({ indicator }) => userLineStyle(indicator, 0, '#2962FF', 1.5) },
    { key: 'dea', title: 'DEA: ', type: 'line', styles: ({ indicator }) => userLineStyle(indicator, 1, '#FF6D00', 1.5) },
    {
      key: 'macd',
      title: 'HIST: ',
      type: 'bar',
      baseValue: 0,
      styles: ({ data, defaultStyles }) => {
        const cur = data.current?.macd ?? 0
        const prev = data.prev?.macd ?? 0
        const upColor = defaultStyles?.bars?.[0]?.upColor ?? '#089981'
        const downColor = defaultStyles?.bars?.[0]?.downColor ?? '#F23645'
        const isUp = cur >= 0
        const strong = Math.abs(cur) >= Math.abs(prev)
        return {
          style: strong ? 'fill' : 'stroke',
          color: isUp ? upColor : downColor,
          borderColor: isUp ? upColor : downColor,
        }
      },
    },
  ],
  calc: (dataList, indicator) => {
    const [fastP = 12, slowP = 26, signalP = 9] = indicator.calcParams
    const fast = Math.max(1, Math.round(fastP))
    const slow = Math.max(fast + 1, Math.round(slowP))
    const sig = Math.max(1, Math.round(signalP))
    const n = dataList.length
    const out: MacdPoint[] = Array.from({ length: n }, () => ({}))
    if (n === 0) return out

    const kFast = 2 / (fast + 1)
    const kSlow = 2 / (slow + 1)
    const kSig = 2 / (sig + 1)

    let emaFast = dataList[0].close
    let emaSlow = dataList[0].close
    let dea = 0

    for (let i = 0; i < n; i++) {
      const c = dataList[i].close
      if (i > 0) {
        emaFast = c * kFast + emaFast * (1 - kFast)
        emaSlow = c * kSlow + emaSlow * (1 - kSlow)
      }
      const dif = emaFast - emaSlow
      dea = i === 0 ? dif : dif * kSig + dea * (1 - kSig)
      out[i] = {
        dif,
        dea,
        macd: dif - dea,
      }
    }
    return out
  },
}

export function indexIndicatorByTimestamp<T>(dataList: KLineData[], values: T[]): Record<number, T> {
  const map: Record<number, T> = {}
  for (let i = 0; i < Math.min(dataList.length, values.length); i++) {
    map[dataList[i].timestamp] = values[i]
  }
  return map
}

/**
 * `AFC — Analog Forecast Cone` (`SPEC-SUMI-AFC-V1`):
 * Projects the Dual-Horizon strictly Point-in-Time (`DEF-001`) `[P10, P25, P50, P75, P90]` probability cone
 * `1..=H` trading sessions into right-hand whitespace from the last visible `1D` candle (`n - 1`),
 * snapped to statutory Vietnam exchange price bands (`HOSE ±7%`, `HNX ±10%`, `UPCOM ±15%`) and ticks,
 * and surfaces Top-4..5 matched historical analogs directly on the indicator info line.
 */
export const afcIndicator: IndicatorTemplate<AfcPoint, number> = {
  name: 'AFC',
  shortName: 'AFC',
  series: 'price',
  calcParams: [10, 100, 1],
  figures: [],
  calc: (dataList) => dataList.map((b) => ({ anchor: b.close })),
  createTooltipDataSource: ({ chart, indicator }) => {
    const { horizon, topK, useMarketContext } = normalizeAfcParams(indicator.calcParams)
    const calcParamsText = `(${horizon}, ${topK}, ${useMarketContext ? 1 : 0})`
    const features = extractTooltipFeatures(indicator)
    const interval = resolveChartIntervalKey(chart)

    if (interval !== '1D') {
      return {
        name: 'AFC',
        calcParamsText,
        features,
        legends: [
          {
            title: { text: 'Trạng thái: ', color: '#94A3B8' },
            value: { text: 'AFC hiệu lực trên khung 1D', color: '#FBC02D' },
          },
        ],
      }
    }

    const dataList = chart.getDataList()
    if (dataList.length === 0) {
      return { name: 'AFC', calcParamsText, features, legends: [] }
    }
    const ticker = chart.getSymbol()?.ticker ?? 'HOSE:FPT'
    const lastBar = dataList[dataList.length - 1]
    const resp = ensureAfcForecast(
      chart,
      indicator.paneId,
      ticker,
      interval,
      lastBar.timestamp,
      horizon,
      topK,
      useMarketContext,
    )

    if (!resp) {
      return {
        name: 'AFC',
        calcParamsText,
        features,
        legends: [
          {
            title: { text: 'PIT: ', color: '#94A3B8' },
            value: { text: 'Đang tổng hợp nón tương đồng...', color: '#94A3B8' },
          },
        ],
      }
    }

    if (!resp.forecast) {
      return {
        name: 'AFC',
        calcParamsText,
        features,
        legends: [
          {
            title: { text: 'AFC: ', color: '#94A3B8' },
            value: { text: resp.statusMessage ?? 'Chưa đủ mẫu lịch sử PIT', color: '#FBC02D' },
          },
        ],
      }
    }

    const fc = resp.forecast
    const isBull = fc.expectedReturnP50Pct >= 0
    const mainColor = isBull ? '#089981' : '#F23645'
    const legends: TooltipLegend[] = [
      {
        title: { text: `T+${fc.steps.length} P50: `, color: '#94A3B8' },
        value: {
          text: `${formatSignedPct(fc.expectedReturnP50Pct, 2)} (Win ${Math.round(fc.winRatePct)}% · TailRisk ${Math.round(fc.probTailDrop6Pct)}% · R:R ${fc.skewRatio.toFixed(1)}x)`,
          color: mainColor,
        },
      },
    ]

    // Surface Top-4 closest historical analog cases directly in the indicator info line!
    for (let i = 0; i < Math.min(4, fc.topAnalogs.length); i++) {
      const a = fc.topAnalogs[i]
      const histRet = horizon <= 5 ? (a.realizedT5RetPct ?? a.realizedT10RetPct) : (a.realizedT10RetPct ?? a.realizedT5RetPct)
      const retLabel = histRet !== null && Number.isFinite(histRet) ? formatSignedPct(histRet, 1) : '—'
      const retColor = histRet !== null && histRet < 0 ? '#F23645' : '#089981'
      const prefix = i === 0 ? 'Khớp: ' : '· '
      legends.push({
        title: {
          text: `${prefix}${a.symbol} ${formatIsoToViDate(a.entryDate)} (${Math.round(a.similarityPct)}%, `,
          color: '#787B86',
        },
        value: {
          text: `${retLabel})`,
          color: retColor,
        },
      })
    }

    return {
      name: 'AFC',
      calcParamsText,
      features,
      legends,
    }
  },
  draw: ({ ctx, chart, indicator, bounding, xAxis, yAxis }) => {
    if (!indicator.visible) return false
    const dataList = chart.getDataList()
    const n = dataList.length
    if (n === 0) return false

    const interval = resolveChartIntervalKey(chart)
    if (interval !== '1D') {
      // Compact non-blocking status badge when viewing sub-daily / weekly timeframes
      ctx.save()
      ctx.font = '600 10.5px -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif'
      const msg = 'AFC hiệu lực trên khung 1D'
      const padX = 7
      const tw = ctx.measureText(msg).width
      const bw = tw + padX * 2
      const bh = 20
      const bx = Math.max(8, bounding.width - bw - 10)
      const by = 28
      ctx.fillStyle = 'rgba(19, 23, 34, 0.84)'
      ctx.strokeStyle = 'rgba(251, 192, 45, 0.55)'
      ctx.lineWidth = 1
      ctx.beginPath()
      ctx.roundRect(bx, by, bw, bh, 4)
      ctx.fill()
      ctx.stroke()
      ctx.fillStyle = '#FBC02D'
      ctx.textBaseline = 'middle'
      ctx.fillText(msg, bx + padX, by + bh / 2)
      ctx.restore()
      return false
    }

    const range = chart.getVisibleRange()
    if (range.to < n - 4) return false

    const { horizon, topK, useMarketContext } = normalizeAfcParams(indicator.calcParams)
    const ticker = chart.getSymbol()?.ticker ?? 'HOSE:FPT'
    const lastBar = dataList[n - 1]
    const resp = ensureAfcForecast(
      chart,
      indicator.paneId,
      ticker,
      interval,
      lastBar.timestamp,
      horizon,
      topK,
      useMarketContext,
    )
    const fc = resp?.forecast
    if (!fc || fc.steps.length === 0) return false

    const anchorClose = resp?.anchorClose ?? lastBar.close
    const x0 = xAxis.convertToPixel(n - 1)
    const y0 = yAxis.convertToPixel(anchorClose)
    if (!Number.isFinite(x0) || !Number.isFinite(y0)) return false

    const coords = [
      { x: x0, p10: y0, p25: y0, p50: y0, p75: y0, p90: y0 },
      ...fc.steps.map((s) => ({
        x: xAxis.convertToPixel(n - 1 + s.step),
        p10: yAxis.convertToPixel(s.p10),
        p25: yAxis.convertToPixel(s.p25),
        p50: yAxis.convertToPixel(s.p50),
        p75: yAxis.convertToPixel(s.p75),
        p90: yAxis.convertToPixel(s.p90),
      })),
    ]

    const isBull = fc.expectedReturnP50Pct >= 0
    const baseRgb = isBull ? '8, 153, 129' : '242, 54, 69'
    const dynamicP50Color = isBull ? '#089981' : '#F23645'
    const p50LineStyle = userLineStyle(indicator, 0, dynamicP50Color, 1.85)
    // Dynamic bullish/bearish color on default palette (`#2962FF` default swatch -> use dynamic green/red!)
    const effectiveP50Color =
      !p50LineStyle.color || p50LineStyle.color.toUpperCase() === '#2962FF'
        ? dynamicP50Color
        : p50LineStyle.color

    ctx.save()

    // 1. Outer [P10, P90] probability fan (`alpha = 0.10`) + dashed boundary lines
    ctx.beginPath()
    ctx.moveTo(coords[0].x, coords[0].p90)
    for (let i = 1; i < coords.length; i++) ctx.lineTo(coords[i].x, coords[i].p90)
    for (let i = coords.length - 1; i >= 0; i--) ctx.lineTo(coords[i].x, coords[i].p10)
    ctx.closePath()
    ctx.fillStyle = `rgba(${baseRgb}, 0.10)`
    ctx.fill()

    ctx.save()
    ctx.setLineDash([4, 3])
    ctx.strokeStyle = `rgba(${baseRgb}, 0.48)`
    ctx.lineWidth = 1
    ctx.beginPath()
    ctx.moveTo(coords[0].x, coords[0].p90)
    for (let i = 1; i < coords.length; i++) ctx.lineTo(coords[i].x, coords[i].p90)
    ctx.stroke()

    ctx.beginPath()
    ctx.moveTo(coords[0].x, coords[0].p10)
    for (let i = 1; i < coords.length; i++) ctx.lineTo(coords[i].x, coords[i].p10)
    ctx.stroke()
    ctx.restore()

    // 2. Core IQR [P25, P75] probability fan (`alpha = 0.22`)
    ctx.beginPath()
    ctx.moveTo(coords[0].x, coords[0].p75)
    for (let i = 1; i < coords.length; i++) ctx.lineTo(coords[i].x, coords[i].p75)
    for (let i = coords.length - 1; i >= 0; i--) ctx.lineTo(coords[i].x, coords[i].p25)
    ctx.closePath()
    ctx.fillStyle = `rgba(${baseRgb}, 0.22)`
    ctx.fill()

    ctx.strokeStyle = `rgba(${baseRgb}, 0.42)`
    ctx.lineWidth = 0.9
    ctx.beginPath()
    ctx.moveTo(coords[0].x, coords[0].p75)
    for (let i = 1; i < coords.length; i++) ctx.lineTo(coords[i].x, coords[i].p75)
    ctx.stroke()
    ctx.beginPath()
    ctx.moveTo(coords[0].x, coords[0].p25)
    for (let i = 1; i < coords.length; i++) ctx.lineTo(coords[i].x, coords[i].p25)
    ctx.stroke()

    // 3. Median `P50` trajectory (`1.85px` solid) + terminal anchor dot
    ctx.strokeStyle = effectiveP50Color
    ctx.lineWidth = p50LineStyle.size ?? 1.85
    ctx.beginPath()
    ctx.moveTo(coords[0].x, coords[0].p50)
    for (let i = 1; i < coords.length; i++) ctx.lineTo(coords[i].x, coords[i].p50)
    ctx.stroke()

    const endPt = coords[coords.length - 1]
    ctx.beginPath()
    ctx.arc(endPt.x, endPt.p50, 2.8, 0, Math.PI * 2)
    ctx.fillStyle = effectiveP50Color
    ctx.fill()

    // 4. Compact right-edge summary badge (`P50 % | Win % | TailRisk %`)
    const summaryText = `T+${fc.steps.length}: P50 ${formatSignedPct(fc.expectedReturnP50Pct, 1)} | Win ${Math.round(fc.winRatePct)}% | TailRisk ${Math.round(fc.probTailDrop6Pct)}%`
    ctx.font = '600 10px -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif'
    const padX = 6
    const tw = ctx.measureText(summaryText).width
    const bw = tw + padX * 2
    const bh = 18
    const bx = Math.max(6, Math.min(endPt.x + 6, bounding.width - bw - 4))
    const by = Math.max(22, Math.min(endPt.p50 - bh / 2, bounding.height - bh - 6))

    ctx.fillStyle = 'rgba(19, 23, 34, 0.88)'
    ctx.strokeStyle = effectiveP50Color
    ctx.lineWidth = 1
    ctx.beginPath()
    ctx.roundRect(bx, by, bw, bh, 3.5)
    ctx.fill()
    ctx.stroke()

    ctx.fillStyle = '#F8FAFC'
    ctx.textBaseline = 'middle'
    ctx.fillText(summaryText, bx + padX, by + bh / 2)

    ctx.restore()
    return false
  },
}

export const SUMI_CUSTOM_INDICATORS = [
  afcIndicator,
  ichimokuIndicator,
  qmvFlowIndicator,
  rsVnindexIndicator,
  vrvpIndicator,
  vwapIndicator,
  atrIndicator,
] as const

export const SUMI_BUILTIN_OVERRIDES = [
  tradingViewMacdIndicator,
  qmvRsiIndicator,
  qmvCciIndicator,
] as const
