/**
 * 5 Quantitative Custom Indicators matching `sumi-core` exact algorithms for KLineChart v10.1.1:
 *  - ICHIMOKU   (Tenkan 9, Kijun 26, Senkou B 52 + shaded Kumo cloud)
 *  - SUPERTREND (ATR period 10, Multiplier 3.0 — green bullish / red bearish)
 *  - VWAP       (Cumulative/Session VWAP + ±1σ / ±2σ Welford bands)
 *  - ATR        (Wilder Average True Range, period 14)
 *  - STOCHRSI   (Wilder RSI 14 -> Stochastic 14 -> K 3 / D 3)
 *
 * Also provides timestamp-keyed map helpers (`Record<number, D>`) alongside KLineChart v10.1.1's
 * bar-aligned `D[]` return required by `IndicatorCalcCallback`.
 */
import type { IndicatorTemplate, KLineData } from 'klinecharts'

export interface IchimokuPoint {
  tenkan?: number
  kijun?: number
  senkouA?: number
  senkouB?: number
  chikou?: number
}

export interface SuperTrendPoint {
  supertrend?: number
  direction?: number
  up?: number
  down?: number
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

export interface StochRsiPoint {
  k?: number
  d?: number
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

export const ichimokuIndicator: IndicatorTemplate<IchimokuPoint, number> = {
  name: 'ICHIMOKU',
  shortName: 'Ichimoku',
  series: 'price',
  calcParams: [9, 26, 52],
  figures: [
    { key: 'tenkan', title: 'Tenkan: ', type: 'line', styles: () => ({ color: '#2962FF', size: 1.3 }) },
    { key: 'kijun', title: 'Kijun: ', type: 'line', styles: () => ({ color: '#E91E63', size: 1.3 }) },
    { key: 'senkouA', title: 'SpanA: ', type: 'line', styles: () => ({ color: 'rgba(8, 153, 129, 0.85)', size: 1 }) },
    { key: 'senkouB', title: 'SpanB: ', type: 'line', styles: () => ({ color: 'rgba(242, 54, 69, 0.85)', size: 1 }) },
    { key: 'chikou', title: 'Chikou: ', type: 'line', styles: () => ({ color: '#9C27B0', size: 1, style: 'dashed' }) },
  ],
  calc: (dataList, indicator) => {
    const [tenkanP = 9, kijunP = 26, senkouBP = 52] = indicator.calcParams
    const displacement = Math.max(1, Math.round(kijunP))
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
      // Causal Chikou plotted back by displacement
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
    const range = chart.getVisibleRange()
    const from = Math.max(0, range.from - 1)
    const to = Math.min(result.length - 1, range.to + 1)

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
    return false
  },
}

export const superTrendIndicator: IndicatorTemplate<SuperTrendPoint, number> = {
  name: 'SUPERTREND',
  shortName: 'SuperTrend',
  series: 'price',
  calcParams: [10, 3],
  figures: [
    { key: 'up', title: 'Bull: ', type: 'line', styles: () => ({ color: '#089981', size: 1.8 }) },
    { key: 'down', title: 'Bear: ', type: 'line', styles: () => ({ color: '#F23645', size: 1.8 }) },
  ],
  calc: (dataList, indicator) => {
    const [period = 10, mult = 3] = indicator.calcParams
    const n = dataList.length
    const atr = computeWilderAtrArray(dataList, period)
    const result: SuperTrendPoint[] = Array.from({ length: n }, () => ({}))

    let finalUpper = 0
    let finalLower = 0
    let dir = 1
    let initialized = false

    for (let i = 0; i < n; i++) {
      const a = atr[i]
      if (a === undefined) continue
      const bar = dataList[i]
      const hl2 = (bar.high + bar.low) / 2
      const basicUpper = hl2 + mult * a
      const basicLower = hl2 - mult * a

      if (!initialized) {
        finalUpper = basicUpper
        finalLower = basicLower
        dir = bar.close >= hl2 ? 1 : -1
        initialized = true
      } else {
        const prevClose = dataList[i - 1].close
        finalUpper = basicUpper < finalUpper || prevClose > finalUpper ? basicUpper : finalUpper
        finalLower = basicLower > finalLower || prevClose < finalLower ? basicLower : finalLower

        if (dir === 1 && bar.close < finalLower) {
          dir = -1
        } else if (dir === -1 && bar.close > finalUpper) {
          dir = 1
        }
      }

      const val = dir === 1 ? finalLower : finalUpper
      result[i] = {
        supertrend: val,
        direction: dir,
        up: dir === 1 ? val : undefined,
        down: dir === -1 ? val : undefined,
      }
    }
    return result
  },
}

export const vwapIndicator: IndicatorTemplate<VwapPoint, number> = {
  name: 'VWAP',
  shortName: 'VWAP',
  series: 'price',
  calcParams: [1, 2],
  figures: [
    { key: 'vwap', title: 'VWAP: ', type: 'line', styles: () => ({ color: '#FBC02D', size: 1.8 }) },
    { key: 'upper1', title: '+1σ: ', type: 'line', styles: () => ({ color: 'rgba(251, 192, 45, 0.55)', size: 1, style: 'dashed' }) },
    { key: 'lower1', title: '-1σ: ', type: 'line', styles: () => ({ color: 'rgba(251, 192, 45, 0.55)', size: 1, style: 'dashed' }) },
    { key: 'upper2', title: '+2σ: ', type: 'line', styles: () => ({ color: 'rgba(251, 192, 45, 0.35)', size: 1, style: 'dashed' }) },
    { key: 'lower2', title: '-2σ: ', type: 'line', styles: () => ({ color: 'rgba(251, 192, 45, 0.35)', size: 1, style: 'dashed' }) },
  ],
  calc: (dataList, indicator) => {
    const [band1 = 1, band2 = 2] = indicator.calcParams
    const n = dataList.length
    const result: VwapPoint[] = Array.from({ length: n }, () => ({}))
    let cumVol = 0
    let cumPv = 0
    let cumPv2 = 0
    let lastDayKey = -1

    // Detect if intraday (< 12h spacing) so VWAP resets daily on session boundary; daily+ uses rolling cumulative
    const isIntraday = n >= 2 && dataList[n - 1].timestamp - dataList[n - 2].timestamp < 43_200_000

    for (let i = 0; i < n; i++) {
      const bar = dataList[i]
      if (isIntraday) {
        const dayKey = Math.floor((bar.timestamp + 7 * 3600_000) / 86_400_000)
        if (dayKey !== lastDayKey) {
          cumVol = 0
          cumPv = 0
          cumPv2 = 0
          lastDayKey = dayKey
        }
      }
      const tp = (bar.high + bar.low + bar.close) / 3
      const vol = Math.max(bar.volume ?? 0, 1e-6)
      cumVol += vol
      cumPv += tp * vol
      cumPv2 += tp * tp * vol
      const vwap = cumPv / cumVol
      const variance = Math.max(0, cumPv2 / cumVol - vwap * vwap)
      const sd = Math.sqrt(variance)

      result[i] = {
        vwap,
        upper1: vwap + sd * band1,
        lower1: vwap - sd * band1,
        upper2: vwap + sd * band2,
        lower2: vwap - sd * band2,
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
  figures: [{ key: 'atr', title: 'ATR: ', type: 'line', styles: () => ({ color: '#FF9800', size: 1.5 }) }],
  calc: (dataList, indicator) => {
    const [period = 14] = indicator.calcParams
    const arr = computeWilderAtrArray(dataList, period)
    return arr.map((v) => (v === undefined ? {} : { atr: v }))
  },
}

export const stochRsiIndicator: IndicatorTemplate<StochRsiPoint, number> = {
  name: 'STOCHRSI',
  shortName: 'StochRSI',
  series: 'normal',
  precision: 2,
  minValue: 0,
  maxValue: 100,
  calcParams: [14, 14, 3, 3],
  figures: [
    { key: 'k', title: '%K: ', type: 'line', styles: () => ({ color: '#2962FF', size: 1.5 }) },
    { key: 'd', title: '%D: ', type: 'line', styles: () => ({ color: '#FF6D00', size: 1.5 }) },
  ],
  calc: (dataList, indicator) => {
    const [rsiP = 14, stochP = 14, kSmooth = 3, dSmooth = 3] = indicator.calcParams
    const n = dataList.length
    const rsi = computeWilderRsiArray(dataList, rsiP)
    const rawStoch: Array<number | undefined> = new Array(n).fill(undefined)
    const sp = Math.max(2, Math.round(stochP))

    for (let i = 0; i < n; i++) {
      if (i < sp - 1 || rsi[i] === undefined) continue
      let lo = Infinity
      let hi = -Infinity
      let valid = true
      for (let j = i - sp + 1; j <= i; j++) {
        const rv = rsi[j]
        if (rv === undefined) {
          valid = false
          break
        }
        if (rv < lo) lo = rv
        if (rv > hi) hi = rv
      }
      if (!valid) continue
      const denom = hi - lo
      rawStoch[i] = denom > 1e-9 ? ((rsi[i]! - lo) / denom) * 100 : 50
    }

    const smaOver = (src: Array<number | undefined>, win: number): Array<number | undefined> => {
      const w = Math.max(1, Math.round(win))
      const res: Array<number | undefined> = new Array(n).fill(undefined)
      for (let i = 0; i < n; i++) {
        if (i < w - 1) continue
        let sum = 0
        let ok = true
        for (let j = i - w + 1; j <= i; j++) {
          if (src[j] === undefined) {
            ok = false
            break
          }
          sum += src[j]!
        }
        if (ok) res[i] = sum / w
      }
      return res
    }

    const kArr = smaOver(rawStoch, kSmooth)
    const dArr = smaOver(kArr, dSmooth)
    return dataList.map((_, i) => ({
      k: kArr[i],
      d: dArr[i],
    }))
  },
}

/**
 * Helper converting any indicator array output into a timestamp-keyed Record<timestamp, value>
 * matching Sumi's quantitative signal bridge.
 */
export function indexIndicatorByTimestamp<T>(dataList: KLineData[], values: T[]): Record<number, T> {
  const map: Record<number, T> = {}
  for (let i = 0; i < Math.min(dataList.length, values.length); i++) {
    map[dataList[i].timestamp] = values[i]
  }
  return map
}

export const SUMI_CUSTOM_INDICATORS = [
  ichimokuIndicator,
  superTrendIndicator,
  vwapIndicator,
  atrIndicator,
  stochRsiIndicator,
] as const
