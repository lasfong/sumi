/**
 * Wave 3 Unlocked `sumi-core` Indicators for KLineChart v10.1.1:
 *   - `TPO_PROFILE`     (Session TPO / Market Profile: POC, 70% VAH/VAL, IBH/IBL, causal Naked POC rays)
 *   - `CANDLE_PATTERNS` (12 Japanese Candlestick Patterns matching `sumi-core/src/indicators/patterns.rs`)
 *   - `SUPERTREND`      (Wilder ATR SuperTrend trailing regime line)
 *   - `PSAR`            (Parabolic SAR dots)
 *   - `KELTNER`         (EMA + ATR Keltner Channels)
 *   - `STOCH`           (Slow Stochastic %K / %D + 20/80 guides)
 *   - `MFI`             (Money Flow Index + 20/80 guides)
 *   - `CVD`             (Cumulative Volume Delta [OHLCV Proxy] with transparent proxy badge!)
 */
import type { IndicatorTemplate, KLineData } from 'klinecharts'

function wilderAtr(dataList: KLineData[], period: number): Array<number | undefined> {
  const n = dataList.length
  const p = Math.max(1, Math.round(period))
  const out: Array<number | undefined> = new Array(n).fill(undefined)
  if (n < p) return out
  const tr = new Array<number>(n)
  for (let i = 0; i < n; i++) {
    const c = dataList[i]
    tr[i] =
      i === 0
        ? c.high - c.low
        : Math.max(c.high - c.low, Math.abs(c.high - dataList[i - 1].close), Math.abs(c.low - dataList[i - 1].close))
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

function emaArray(values: number[], period: number): Array<number | undefined> {
  const n = values.length
  const p = Math.max(1, Math.round(period))
  const out: Array<number | undefined> = new Array(n).fill(undefined)
  if (n < p) return out
  let sum = 0
  for (let i = 0; i < p; i++) sum += values[i]
  let prev = sum / p
  out[p - 1] = prev
  const k = 2 / (p + 1)
  for (let i = p; i < n; i++) {
    prev = values[i] * k + prev * (1 - k)
    out[i] = prev
  }
  return out
}

// ─── 1. SUPERTREND ───────────────────────────────────────────────────────────

export interface SuperTrendPoint {
  up?: number
  down?: number
}

export const superTrendIndicator: IndicatorTemplate<SuperTrendPoint> = {
  name: 'SUPERTREND',
  shortName: 'SuperTrend',
  series: 'price',
  calcParams: [10, 3],
  shouldOhlc: true,
  figures: [
    { key: 'up', title: 'ST Tăng: ', type: 'line', styles: () => ({ color: '#089981', size: 1.8 }) },
    { key: 'down', title: 'ST Giảm: ', type: 'line', styles: () => ({ color: '#F23645', size: 1.8 }) },
  ],
  calc: (dataList, indicator) => {
    const period = Math.max(2, Math.round(Number(indicator.calcParams[0] ?? 10)))
    const mult = Math.max(0.5, Number(indicator.calcParams[1] ?? 3))
    const atr = wilderAtr(dataList, period)
    const out: SuperTrendPoint[] = new Array(dataList.length).fill(null).map(() => ({}))
    let finalUpper = 0
    let finalLower = 0
    let bull = true

    for (let i = 0; i < dataList.length; i++) {
      const a = atr[i]
      if (a === undefined) continue
      const c = dataList[i]
      const hl2 = (c.high + c.low) / 2
      const basicUpper = hl2 + mult * a
      const basicLower = hl2 - mult * a
      if (i === period - 1 || atr[i - 1] === undefined) {
        finalUpper = basicUpper
        finalLower = basicLower
        bull = c.close >= hl2
      } else {
        const prevClose = dataList[i - 1].close
        finalUpper = basicUpper < finalUpper || prevClose > finalUpper ? basicUpper : finalUpper
        finalLower = basicLower > finalLower || prevClose < finalLower ? basicLower : finalLower
        if (bull && c.close < finalLower) bull = false
        else if (!bull && c.close > finalUpper) bull = true
      }
      out[i] = bull ? { up: finalLower } : { down: finalUpper }
    }
    return out
  },
}

// ─── 2. PARABOLIC SAR (PSAR) ─────────────────────────────────────────────────

export interface PsarPoint {
  sar?: number
  bull?: number
}

export const psarIndicator: IndicatorTemplate<PsarPoint> = {
  name: 'PSAR',
  shortName: 'PSAR',
  series: 'price',
  calcParams: [0.02, 0.2],
  shouldOhlc: true,
  figures: [
    {
      key: 'sar',
      title: 'PSAR: ',
      type: 'circle',
      styles: ({ data }) => ({
        style: 'fill',
        color: data.current?.bull === 1 ? '#089981' : '#F23645',
      }),
    },
  ],
  calc: (dataList, indicator) => {
    const step = Math.max(0.005, Number(indicator.calcParams[0] ?? 0.02))
    const maxAf = Math.max(step, Number(indicator.calcParams[1] ?? 0.2))
    const n = dataList.length
    const out: PsarPoint[] = new Array(n).fill(null).map(() => ({}))
    if (n < 2) return out

    let bull = dataList[1].close >= dataList[0].close
    let af = step
    let ep = bull ? dataList[1].high : dataList[1].low
    let sar = bull ? dataList[0].low : dataList[0].high
    out[1] = { sar, bull: bull ? 1 : 0 }

    for (let i = 2; i < n; i++) {
      const prev = dataList[i - 1]
      const prev2 = dataList[i - 2]
      const cur = dataList[i]
      sar = sar + af * (ep - sar)
      if (bull) {
        sar = Math.min(sar, prev.low, prev2.low)
        if (cur.low < sar) {
          bull = false
          sar = ep
          ep = cur.low
          af = step
        } else if (cur.high > ep) {
          ep = cur.high
          af = Math.min(maxAf, af + step)
        }
      } else {
        sar = Math.max(sar, prev.high, prev2.high)
        if (cur.high > sar) {
          bull = true
          sar = ep
          ep = cur.high
          af = step
        } else if (cur.low < ep) {
          ep = cur.low
          af = Math.min(maxAf, af + step)
        }
      }
      out[i] = { sar, bull: bull ? 1 : 0 }
    }
    return out
  },
}

// ─── 3. KELTNER CHANNELS ─────────────────────────────────────────────────────

export interface KeltnerPoint {
  upper?: number
  mid?: number
  lower?: number
}

export const keltnerIndicator: IndicatorTemplate<KeltnerPoint> = {
  name: 'KELTNER',
  shortName: 'Keltner',
  series: 'price',
  calcParams: [20, 10, 2],
  shouldOhlc: true,
  figures: [
    { key: 'upper', title: 'KC Trên: ', type: 'line', styles: () => ({ color: '#2962FF', size: 1.2, style: 'dashed' }) },
    { key: 'mid', title: 'KC Giữa: ', type: 'line', styles: () => ({ color: '#FF9800', size: 1.4 }) },
    { key: 'lower', title: 'KC Dưới: ', type: 'line', styles: () => ({ color: '#2962FF', size: 1.2, style: 'dashed' }) },
  ],
  calc: (dataList, indicator) => {
    const emaP = Math.max(2, Math.round(Number(indicator.calcParams[0] ?? 20)))
    const atrP = Math.max(2, Math.round(Number(indicator.calcParams[1] ?? 10)))
    const mult = Math.max(0.5, Number(indicator.calcParams[2] ?? 2))
    const mid = emaArray(dataList.map((b) => b.close), emaP)
    const atr = wilderAtr(dataList, atrP)
    return dataList.map((_, i) => {
      const m = mid[i]
      const a = atr[i]
      if (m === undefined || a === undefined) return {}
      return { upper: m + mult * a, mid: m, lower: m - mult * a }
    })
  },
}

// ─── 4. STOCHASTIC OSCILLATOR ────────────────────────────────────────────────

export interface StochPoint {
  k?: number
  d?: number
}

export const stochIndicator: IndicatorTemplate<StochPoint> = {
  name: 'STOCH',
  shortName: 'Stoch',
  series: 'normal',
  calcParams: [14, 3, 3],
  precision: 1,
  minValue: 0,
  maxValue: 100,
  figures: [
    { key: 'k', title: '%K: ', type: 'line', styles: () => ({ color: '#2962FF', size: 1.5 }) },
    { key: 'd', title: '%D: ', type: 'line', styles: () => ({ color: '#FF9800', size: 1.4 }) },
  ],
  calc: (dataList, indicator) => {
    const kP = Math.max(2, Math.round(Number(indicator.calcParams[0] ?? 14)))
    const smoothK = Math.max(1, Math.round(Number(indicator.calcParams[1] ?? 3)))
    const dP = Math.max(1, Math.round(Number(indicator.calcParams[2] ?? 3)))
    const n = dataList.length
    const rawK: Array<number | undefined> = new Array(n).fill(undefined)
    for (let i = kP - 1; i < n; i++) {
      let hh = -Infinity
      let ll = Infinity
      for (let j = i - kP + 1; j <= i; j++) {
        if (dataList[j].high > hh) hh = dataList[j].high
        if (dataList[j].low < ll) ll = dataList[j].low
      }
      rawK[i] = hh - ll > 1e-12 ? ((dataList[i].close - ll) / (hh - ll)) * 100 : 50
    }
    const smaOpt = (arr: Array<number | undefined>, w: number) =>
      arr.map((_, i) => {
        if (i < w - 1) return undefined
        let sum = 0
        for (let j = i - w + 1; j <= i; j++) {
          if (arr[j] === undefined) return undefined
          sum += arr[j]!
        }
        return sum / w
      })
    const kLine = smaOpt(rawK, smoothK)
    const dLine = smaOpt(kLine, dP)
    return dataList.map((_, i) => ({ k: kLine[i], d: dLine[i] }))
  },
  draw: ({ ctx, bounding, yAxis }) => {
    ctx.save()
    ctx.setLineDash([4, 4])
    ctx.lineWidth = 1
    for (const [lv, col] of [
      [80, 'rgba(242, 54, 69, 0.45)'],
      [20, 'rgba(8, 153, 129, 0.45)'],
    ] as const) {
      const y = yAxis.convertToPixel(lv)
      if (!Number.isFinite(y)) continue
      ctx.beginPath()
      ctx.strokeStyle = col
      ctx.moveTo(0, y)
      ctx.lineTo(bounding.width, y)
      ctx.stroke()
    }
    ctx.restore()
    return false
  },
}

// ─── 5. MONEY FLOW INDEX (MFI) ───────────────────────────────────────────────

export interface MfiPoint {
  mfi?: number
}

export const mfiIndicator: IndicatorTemplate<MfiPoint> = {
  name: 'MFI',
  shortName: 'MFI',
  series: 'normal',
  calcParams: [14],
  precision: 1,
  minValue: 0,
  maxValue: 100,
  figures: [{ key: 'mfi', title: 'MFI: ', type: 'line', styles: () => ({ color: '#AB47BC', size: 1.6 }) }],
  calc: (dataList, indicator) => {
    const p = Math.max(2, Math.round(Number(indicator.calcParams[0] ?? 14)))
    const n = dataList.length
    const pos = new Array<number>(n).fill(0)
    const neg = new Array<number>(n).fill(0)
    for (let i = 1; i < n; i++) {
      const tp = (dataList[i].high + dataList[i].low + dataList[i].close) / 3
      const prevTp = (dataList[i - 1].high + dataList[i - 1].low + dataList[i - 1].close) / 3
      const mf = tp * Math.max(0, dataList[i].volume ?? 0)
      if (tp > prevTp) pos[i] = mf
      else if (tp < prevTp) neg[i] = mf
    }
    return dataList.map((_, i) => {
      if (i < p) return {}
      let sumPos = 0
      let sumNeg = 0
      for (let j = i - p + 1; j <= i; j++) {
        sumPos += pos[j]
        sumNeg += neg[j]
      }
      if (sumPos + sumNeg <= 1e-12) return { mfi: 50 }
      if (sumNeg <= 1e-12) return { mfi: 100 }
      const ratio = sumPos / sumNeg
      return { mfi: 100 - 100 / (1 + ratio) }
    })
  },
  draw: ({ ctx, bounding, yAxis }) => {
    ctx.save()
    ctx.setLineDash([4, 4])
    ctx.lineWidth = 1
    for (const [lv, col] of [
      [80, 'rgba(242, 54, 69, 0.45)'],
      [20, 'rgba(8, 153, 129, 0.45)'],
    ] as const) {
      const y = yAxis.convertToPixel(lv)
      if (!Number.isFinite(y)) continue
      ctx.beginPath()
      ctx.strokeStyle = col
      ctx.moveTo(0, y)
      ctx.lineTo(bounding.width, y)
      ctx.stroke()
    }
    ctx.restore()
    return false
  },
}

// ─── 6. CUMULATIVE VOLUME DELTA (CVD — Honest OHLCV Proxy) ───────────────────

export interface CvdPoint {
  cvd?: number
  delta?: number
}

export const cvdIndicator: IndicatorTemplate<CvdPoint> = {
  name: 'CVD',
  shortName: 'CVD [Proxy]',
  series: 'normal',
  calcParams: [1],
  precision: 0,
  figures: [
    {
      key: 'delta',
      title: 'Delta: ',
      type: 'bar',
      baseValue: 0,
      styles: ({ data }) => ({
        style: 'fill',
        color: (data.current?.delta ?? 0) >= 0 ? 'rgba(8, 153, 129, 0.45)' : 'rgba(242, 54, 69, 0.45)',
      }),
    },
    { key: 'cvd', title: 'CVD: ', type: 'line', styles: () => ({ color: '#00BCD4', size: 1.7 }) },
  ],
  calc: (dataList, indicator) => {
    // `calcParams[0]`: `1` = continuous cumulative CVD (default), `0` = daily ICT session reset, `> 1` = rolling N-bar window (`W6-08`)
    const modeOrWindow = Math.max(0, Math.round(Number(indicator.calcParams[0] ?? 1)))
    let cum = 0
    let prevDayBucket = -1
    const deltas: number[] = []
    let rollingSum = 0
    return dataList.map((b, idx) => {
      const vol = Math.max(0, b.volume ?? 0)
      const rng = b.high - b.low
      const pressure =
        rng > 1e-12
          ? Math.max(-1, Math.min(1, ((b.close - b.low) + (b.close - b.high)) / rng))
          : b.close > b.open
            ? 1
            : b.close < b.open
              ? -1
              : 0
      const delta = vol * pressure
      deltas.push(delta)

      if (modeOrWindow === 0) {
        const dayBucket = Math.floor((b.timestamp + 7 * 3_600_000) / 86_400_000)
        if (dayBucket !== prevDayBucket) {
          cum = 0
          prevDayBucket = dayBucket
        }
        cum += delta
        return { delta, cvd: cum }
      }
      if (modeOrWindow > 1) {
        rollingSum += delta
        if (idx >= modeOrWindow) {
          rollingSum -= deltas[idx - modeOrWindow]
        }
        return { delta, cvd: rollingSum }
      }
      cum += delta
      return { delta, cvd: cum }
    })
  },
  draw: ({ ctx, bounding }) => {
    const label = '⚠ OHLCV Proxy (CLV Delta)'
    ctx.save()
    ctx.font = '600 9.5px Inter, -apple-system, sans-serif'
    const bw = ctx.measureText(label).width + 10
    const bx = Math.max(6, bounding.width - bw - 8)
    ctx.fillStyle = 'rgba(251, 192, 45, 0.16)'
    ctx.strokeStyle = '#FBC02D'
    ctx.lineWidth = 1
    ctx.beginPath()
    ctx.roundRect(bx, 5, bw, 16, 3)
    ctx.fill()
    ctx.stroke()
    ctx.fillStyle = '#FBC02D'
    ctx.textBaseline = 'middle'
    ctx.fillText(label, bx + 5, 13)
    ctx.restore()
    return false
  },
}

// ─── 7. SESSION TPO / MARKET PROFILE OVERLAY (`market_profile.rs` parity) ───

export interface TpoOverlayPoint {
  poc?: number
}

export const tpoMarketProfileIndicator: IndicatorTemplate<TpoOverlayPoint> = {
  name: 'TPO_PROFILE',
  shortName: 'TPO Profile',
  series: 'price',
  calcParams: [20, 6],
  shouldOhlc: true,
  figures: [{ key: 'poc', title: 'TPO POC: ', type: 'line', styles: () => ({ color: '#FBC02D', size: 1, style: 'dashed' }) }],
  calc: (dataList) => dataList.map(() => ({})),
  draw: ({ ctx, chart, bounding, xAxis, yAxis, indicator }) => {
    const dataList = chart.getDataList()
    if (dataList.length < 10) return false
    const numBins = Math.max(8, Math.min(48, Math.round(Number(indicator.calcParams[0] ?? 20))))
    const maxSessions = Math.max(1, Math.min(16, Math.round(Number(indicator.calcParams[1] ?? 6))))
    const range = chart.getVisibleRange()
    const from = Math.max(0, range.from)
    const to = Math.min(dataList.length - 1, range.to)

    // Group visible bars into sessions (ISO week for >=1D, ICT day for intraday)
    const period = chart.getPeriod()
    const isDailyPlus = period?.type === 'day' || period?.type === 'week' || period?.type === 'month'
    const bucketKey = (ts: number) => {
      const days = Math.floor((ts + 7 * 3_600_000) / 86_400_000)
      if (!isDailyPlus) return days
      const dow = ((days + 3) % 7 + 7) % 7
      return days - dow // ISO Monday
    }

    const sessions: Array<{ start: number; end: number }> = []
    let curKey: number | null = null
    let curStart = from
    for (let i = from; i <= to; i++) {
      const k = bucketKey(dataList[i].timestamp)
      if (curKey !== k) {
        if (curKey !== null) sessions.push({ start: curStart, end: i - 1 })
        curKey = k
        curStart = i
      }
    }
    if (curKey !== null) sessions.push({ start: curStart, end: to })
    const recent = sessions.slice(-maxSessions)

    ctx.save()
    for (let sIdx = 0; sIdx < recent.length; sIdx++) {
      const sess = recent[sIdx]
      let lo = Infinity
      let hi = -Infinity
      for (let i = sess.start; i <= sess.end; i++) {
        if (dataList[i].low < lo) lo = dataList[i].low
        if (dataList[i].high > hi) hi = dataList[i].high
      }
      if (!Number.isFinite(lo) || !Number.isFinite(hi) || hi <= lo) continue
      const step = (hi - lo) / numBins
      const counts = new Array<number>(numBins).fill(0)
      for (let i = sess.start; i <= sess.end; i++) {
        const b = dataList[i]
        const bLo = Math.max(0, Math.min(numBins - 1, Math.floor((b.low - lo) / step)))
        const bHi = Math.max(0, Math.min(numBins - 1, Math.floor((b.high - lo) / step)))
        for (let bi = bLo; bi <= bHi; bi++) counts[bi] += 1
      }
      let pocIdx = 0
      let maxTpo = 0
      let totalTpo = 0
      for (let bi = 0; bi < numBins; bi++) {
        totalTpo += counts[bi]
        if (counts[bi] > maxTpo) {
          maxTpo = counts[bi]
          pocIdx = bi
        }
      }
      if (maxTpo === 0) continue

      // 70% Value Area
      let va = counts[pocIdx]
      let vaLo = pocIdx
      let vaHi = pocIdx
      while (va < totalTpo * 0.7 && (vaLo > 0 || vaHi < numBins - 1)) {
        const up = vaHi + 1 < numBins ? counts[vaHi + 1] : -1
        const dn = vaLo > 0 ? counts[vaLo - 1] : -1
        if (up >= dn && up >= 0) {
          vaHi += 1
          va += up
        } else if (dn >= 0) {
          vaLo -= 1
          va += dn
        } else break
      }

      const xStart = xAxis.convertToPixel(sess.start)
      const xEnd = xAxis.convertToPixel(sess.end)
      const sessW = Math.max(12, xEnd - xStart)
      const maxBarW = Math.min(sessW * 0.75, 96)

      for (let bi = 0; bi < numBins; bi++) {
        if (counts[bi] === 0) continue
        const y1 = yAxis.convertToPixel(lo + (bi + 1) * step)
        const y2 = yAxis.convertToPixel(lo + bi * step)
        const h = Math.max(1.5, Math.abs(y2 - y1) - 1)
        const w = (counts[bi] / maxTpo) * maxBarW
        const inVa = bi >= vaLo && bi <= vaHi
        ctx.fillStyle =
          bi === pocIdx
            ? 'rgba(251, 192, 45, 0.78)'
            : inVa
              ? 'rgba(0, 188, 212, 0.34)'
              : 'rgba(120, 123, 134, 0.22)'
        ctx.fillRect(xStart, Math.min(y1, y2) + 0.5, w, h)
      }

      // Causal Naked POC check across subsequent sessions
      const pocPrice = lo + (pocIdx + 0.5) * step
      const yPoc = yAxis.convertToPixel(pocPrice)
      let mitigatedIdx: number | null = null
      for (let k = sess.end + 1; k <= to; k++) {
        if (dataList[k].low <= pocPrice && dataList[k].high >= pocPrice) {
          mitigatedIdx = k
          break
        }
      }
      if (sIdx < recent.length - 1) {
        const rayEndX = mitigatedIdx !== null ? xAxis.convertToPixel(mitigatedIdx) : bounding.width
        ctx.beginPath()
        ctx.setLineDash(mitigatedIdx === null ? [5, 3] : [2, 4])
        ctx.strokeStyle = mitigatedIdx === null ? 'rgba(251, 192, 45, 0.85)' : 'rgba(251, 192, 45, 0.3)'
        ctx.lineWidth = mitigatedIdx === null ? 1.4 : 1
        ctx.moveTo(xStart, yPoc)
        ctx.lineTo(rayEndX, yPoc)
        ctx.stroke()
        ctx.setLineDash([])
      }
    }
    ctx.restore()
    return false
  },
}

// ─── 8. 12 JAPANESE CANDLESTICK PATTERNS OVERLAY (`patterns.rs` parity) ──────

export interface CandlePatternPoint {
  badge?: string
  bull?: number
}

export const candlePatternsIndicator: IndicatorTemplate<CandlePatternPoint> = {
  name: 'CANDLE_PATTERNS',
  shortName: 'Mô Hình Nến (12)',
  series: 'price',
  calcParams: [14],
  shouldOhlc: true,
  figures: [],
  calc: (dataList, indicator) => {
    const n = dataList.length
    const atrPeriod = Math.max(2, Math.min(100, Math.round(Number(indicator.calcParams[0] ?? 14))))
    const atr = wilderAtr(dataList, atrPeriod)
    const out: CandlePatternPoint[] = new Array(n).fill(null).map(() => ({}))

    for (let i = 0; i < n; i++) {
      const c = dataList[i]
      const rng = c.high - c.low
      if (rng <= 1e-12) continue
      const body = Math.abs(c.close - c.open)
      const bodyHi = Math.max(c.open, c.close)
      const bodyLo = Math.min(c.open, c.close)
      const upWick = c.high - bodyHi
      const loWick = bodyLo - c.low
      const isBull = c.close > c.open
      const isBear = c.close < c.open

      // Single-bar patterns: Hammer / Shooting Star
      if (body / rng <= 0.35 && loWick >= body * 2 && upWick <= rng * 0.18 && (c.close - c.low) / rng >= 0.65) {
        out[i] = { badge: 'HMR', bull: 1 }
      } else if (body / rng <= 0.35 && upWick >= body * 2 && loWick <= rng * 0.18 && (c.close - c.low) / rng <= 0.35) {
        out[i] = { badge: 'SSR', bull: 0 }
      }

      if (i >= 1) {
        const p = dataList[i - 1]
        const pBody = Math.abs(p.close - p.open)
        const pMid = (p.open + p.close) / 2
        const a = atr[i] ?? rng
        if (p.close < p.open && isBull && c.open <= p.close && c.close >= p.open && body > pBody) {
          out[i] = { badge: 'BE+', bull: 1 }
        } else if (p.close > p.open && isBear && c.open >= p.close && c.close <= p.open && body > pBody) {
          out[i] = { badge: 'BE-', bull: 0 }
        } else if (p.close < p.open && isBull && c.open < p.low && c.close > pMid && c.close < p.open) {
          out[i] = { badge: 'PRC', bull: 1 }
        } else if (p.close > p.open && isBear && c.open > p.high && c.close < pMid && c.close > p.open) {
          out[i] = { badge: 'DCC', bull: 0 }
        } else if (Math.abs(c.low - p.low) <= 0.12 * a && p.close < p.open && isBull) {
          out[i] = { badge: 'TWB', bull: 1 }
        } else if (Math.abs(c.high - p.high) <= 0.12 * a && p.close > p.open && isBear) {
          out[i] = { badge: 'TWT', bull: 0 }
        }
      }

      if (i >= 2) {
        const p2 = dataList[i - 2]
        const p1 = dataList[i - 1]
        const p2Body = Math.abs(p2.close - p2.open)
        const p1Body = Math.abs(p1.close - p1.open)
        const p2Mid = (p2.open + p2.close) / 2
        if (p2.close < p2.open && p1Body <= p2Body * 0.45 && isBull && c.close > p2Mid) {
          out[i] = { badge: 'MST', bull: 1 }
        } else if (p2.close > p2.open && p1Body <= p2Body * 0.45 && isBear && c.close < p2Mid) {
          out[i] = { badge: 'EST', bull: 0 }
        } else if (p1.high <= p2.high && p1.low >= p2.low && c.close > p2.high) {
          out[i] = { badge: 'IBU', bull: 1 }
        } else if (p1.high <= p2.high && p1.low >= p2.low && c.close < p2.low) {
          out[i] = { badge: 'IBD', bull: 0 }
        }
      }
    }
    return out
  },
  draw: ({ ctx, chart, xAxis, yAxis, indicator }) => {
    const dataList = chart.getDataList()
    const res = indicator.result
    const range = chart.getVisibleRange()
    const from = Math.max(0, range.from)
    const to = Math.min(dataList.length - 1, range.to)

    ctx.save()
    ctx.font = '700 9px Inter, -apple-system, sans-serif'
    ctx.textAlign = 'center'
    for (let i = from; i <= to; i++) {
      const pt = res[i]
      if (!pt?.badge) continue
      const bar = dataList[i]
      const x = xAxis.convertToPixel(i)
      const isBull = pt.bull === 1
      const yAnchor = yAxis.convertToPixel(isBull ? bar.low : bar.high)
      const y = isBull ? yAnchor + 14 : yAnchor - 14
      const col = isBull ? '#089981' : '#F23645'
      ctx.fillStyle = col
      ctx.textBaseline = 'middle'
      ctx.fillText(`${isBull ? '▲' : '▼'}${pt.badge}`, x, y)
    }
    ctx.restore()
    return false
  },
}

export const WAVE3_INDICATORS = [
  superTrendIndicator,
  psarIndicator,
  keltnerIndicator,
  stochIndicator,
  mfiIndicator,
  cvdIndicator,
  tpoMarketProfileIndicator,
  candlePatternsIndicator,
] as const
