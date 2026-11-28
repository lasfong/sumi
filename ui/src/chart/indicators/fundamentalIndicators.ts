/**
 * Wave 4 Financial Fundamental Indicators (`FIN_PE`, `FIN_PB`, `FIN_ROE`, `FIN_EPS_YOY`)
 * & Multi-Symbol Causal Rebased Comparison Overlay (`COMPARE_OVERLAY`).
 *
 * Strictly zero runtime `klinecharts` import (`import type` only) so pure calculation helpers
 * remain unit-testable in Node/Vitest without DOM `window`.
 */
import type { IndicatorTemplate, KLineData, TooltipLegend } from 'klinecharts'
import type {
  CompareSeriesResponse,
  SymbolFundamentalsResponse,
  ValuationQuarterDto,
} from '../../types/market'

export const COMPARE_COLORS = [
  '#FF9800',
  '#E040FB',
  '#00BCD4',
  '#FBC02D',
  '#26A69A',
  '#EF5350',
] as const

const fundamentalsBySymbol = new Map<string, SymbolFundamentalsResponse>()
const compareBySlot = new Map<string, CompareSeriesResponse>()

function bareSym(ticker: string): string {
  const up = ticker.trim().toUpperCase()
  const idx = up.lastIndexOf(':')
  return idx >= 0 ? up.slice(idx + 1) : up
}

export function setFundamentalsCache(ticker: string, data: SymbolFundamentalsResponse): void {
  fundamentalsBySymbol.set(bareSym(ticker), data)
}

export function getFundamentalsCache(ticker: string): SymbolFundamentalsResponse | undefined {
  return fundamentalsBySymbol.get(bareSym(ticker))
}

export function clearFundamentalsCache(): void {
  fundamentalsBySymbol.clear()
}

export function setCompareSeriesCache(slotId: string, resp: CompareSeriesResponse | null): void {
  if (!resp || resp.series.length === 0) {
    compareBySlot.delete(slotId)
  } else {
    compareBySlot.set(slotId, resp)
  }
}

export function getCompareSeriesCache(slotId: string): CompareSeriesResponse | undefined {
  return compareBySlot.get(slotId)
}

/**
 * Pure helper: parses a valuation quarter's effective causal timestamp (UTC ms) from `contentDate`
 * (`YYYY-MM-DD`) or fallback `period` (`2025-Q4`, `Q4/2025`, `2025Q4`, `2025`).
 */
export function resolveValuationQuarterTimestampMs(q: ValuationQuarterDto): number | null {
  if (q.contentDate && q.contentDate.trim().length >= 10) {
    const parsed = Date.parse(`${q.contentDate.trim().slice(0, 10)}T00:00:00Z`)
    if (Number.isFinite(parsed)) return parsed
  }
  const p = q.period.trim().toUpperCase()
  const m1 = /^(\d{4})[-/\s]?Q([1-4])$/.exec(p)
  const m2 = /^Q([1-4])[-/\s]?(\d{4})$/.exec(p)
  const year = m1 ? Number(m1[1]) : m2 ? Number(m2[2]) : /^(\d{4})$/.test(p) ? Number(p) : NaN
  const quarter = m1 ? Number(m1[2]) : m2 ? Number(m2[1]) : 4
  if (!Number.isFinite(year) || year < 1990 || year > 2100) return null
  const monthEnd = quarter * 3 // 3, 6, 9, 12
  return Date.UTC(year, monthEnd, 0)
}

/**
 * Strictly causal Point-in-Time quarterly fundamental series builder (`zero future lookahead`, `W6-06`):
 * - NEVER backfills bars prior to the first available quarter with future/current ratios!
 * - Computes **dynamic daily P/E (`Close[t] / EPS_TTM[q(t)]`)** and **dynamic daily P/B
 *   (`Close[t] / BVPS[q(t)]`)** so valuation lines move accurately with every daily price bar
 *   while remaining strictly PIT-locked to the latest published quarter `q(t)`.
 */
export function buildQuarterlyStepSeries(
  bars: Array<{ timestamp: number; close?: number }>,
  valuations: ValuationQuarterDto[],
  metric: 'pe' | 'pb' | 'roe' | 'epsYoy',
): Array<number | undefined> {
  if (bars.length === 0 || valuations.length === 0) {
    return bars.map(() => undefined)
  }

  const stamped = valuations
    .map((q) => ({ q, ts: resolveValuationQuarterTimestampMs(q) }))
    .filter((item): item is { q: ValuationQuarterDto; ts: number } => item.ts !== null)
    .sort((a, b) => a.ts - b.ts)

  if (stamped.length === 0) return bars.map(() => undefined)

  // Locate each quarter's contemporaneous chart close (nearest bar <= quarter end, else first bar >= quarter end)
  // solely for unit normalization (VND vs kVND) or inferring implied EPS/BVPS when only `pe`/`pb` was supplied.
  const findCloseAtOrBefore = (ts: number): number | null => {
    let best: number | null = null
    for (let i = 0; i < bars.length; i++) {
      const c = bars[i].close
      if (typeof c !== 'number' || !Number.isFinite(c) || c <= 0) continue
      if (bars[i].timestamp <= ts) {
        best = c
      } else {
        return best ?? c
      }
    }
    return best
  }

  interface QuarterEntry {
    ts: number
    staticVal?: number
    effectiveEps?: number
    effectiveBvps?: number
  }

  const entries: QuarterEntry[] = []
  for (let i = 0; i < stamped.length; i++) {
    const cur = stamped[i]
    if (metric === 'pe') {
      const refClose = findCloseAtOrBefore(cur.ts)
      let effEps: number | undefined
      if (typeof cur.q.eps === 'number' && Number.isFinite(cur.q.eps) && cur.q.eps > 0) {
        effEps = cur.q.eps
        // Unit guard: if chart is in Raw VND (e.g. 132,500) while EPS is in kVND (e.g. 5.2), scale by 1,000x
        if (
          refClose !== null &&
          typeof cur.q.pe === 'number' &&
          Number.isFinite(cur.q.pe) &&
          cur.q.pe > 0
        ) {
          const rawRatio = refClose / effEps
          const factor = rawRatio / cur.q.pe
          if (factor > 200 && factor < 5000) {
            effEps *= 1000
          } else if (factor > 0.0002 && factor < 0.005) {
            effEps /= 1000
          }
        }
      } else if (
        refClose !== null &&
        typeof cur.q.pe === 'number' &&
        Number.isFinite(cur.q.pe) &&
        cur.q.pe > 0
      ) {
        effEps = refClose / cur.q.pe
      }
      const staticPe = typeof cur.q.pe === 'number' && Number.isFinite(cur.q.pe) ? cur.q.pe : undefined
      if (effEps !== undefined || staticPe !== undefined) {
        entries.push({ ts: cur.ts, effectiveEps: effEps, staticVal: staticPe })
      }
    } else if (metric === 'pb') {
      const refClose = findCloseAtOrBefore(cur.ts)
      let effBvps: number | undefined
      if (typeof cur.q.bvps === 'number' && Number.isFinite(cur.q.bvps) && cur.q.bvps > 0) {
        effBvps = cur.q.bvps
        if (
          refClose !== null &&
          typeof cur.q.pb === 'number' &&
          Number.isFinite(cur.q.pb) &&
          cur.q.pb > 0
        ) {
          const rawRatio = refClose / effBvps
          const factor = rawRatio / cur.q.pb
          if (factor > 200 && factor < 5000) {
            effBvps *= 1000
          } else if (factor > 0.0002 && factor < 0.005) {
            effBvps /= 1000
          }
        }
      } else if (
        refClose !== null &&
        typeof cur.q.pb === 'number' &&
        Number.isFinite(cur.q.pb) &&
        cur.q.pb > 0
      ) {
        effBvps = refClose / cur.q.pb
      }
      const staticPb = typeof cur.q.pb === 'number' && Number.isFinite(cur.q.pb) ? cur.q.pb : undefined
      if (effBvps !== undefined || staticPb !== undefined) {
        entries.push({ ts: cur.ts, effectiveBvps: effBvps, staticVal: staticPb })
      }
    } else if (metric === 'roe') {
      if (typeof cur.q.roe === 'number' && Number.isFinite(cur.q.roe)) {
        entries.push({ ts: cur.ts, staticVal: cur.q.roe })
      }
    } else if (metric === 'epsYoy') {
      const curEps = cur.q.eps
      if (typeof curEps === 'number' && Number.isFinite(curEps)) {
        const targetPrevTs = cur.ts - 365 * 86_400_000
        let bestPrev: { eps: number; diff: number } | null = null
        for (let k = 0; k < i; k++) {
          const prevEps = stamped[k].q.eps
          if (typeof prevEps !== 'number' || !Number.isFinite(prevEps) || Math.abs(prevEps) < 1e-6) continue
          const diff = Math.abs(stamped[k].ts - targetPrevTs)
          if (diff <= 75 * 86_400_000 && (!bestPrev || diff < bestPrev.diff)) {
            bestPrev = { eps: prevEps, diff }
          }
        }
        if (bestPrev) {
          const val = ((curEps - bestPrev.eps) / Math.abs(bestPrev.eps)) * 100
          if (Number.isFinite(val)) {
            entries.push({ ts: cur.ts, staticVal: val })
          }
        }
      }
    }
  }

  if (entries.length === 0) return bars.map(() => undefined)

  const out: Array<number | undefined> = new Array(bars.length)
  let ptr = -1
  for (let i = 0; i < bars.length; i++) {
    const bTs = bars[i].timestamp
    while (ptr + 1 < entries.length && entries[ptr + 1].ts <= bTs) {
      ptr += 1
    }
    if (ptr < 0) {
      out[i] = undefined
      continue
    }
    const active = entries[ptr]
    const barClose = bars[i].close
    if (metric === 'pe' && active.effectiveEps && typeof barClose === 'number' && Number.isFinite(barClose) && barClose > 0) {
      out[i] = barClose / active.effectiveEps
    } else if (metric === 'pb' && active.effectiveBvps && typeof barClose === 'number' && Number.isFinite(barClose) && barClose > 0) {
      out[i] = barClose / active.effectiveBvps
    } else {
      out[i] = active.staticVal
    }
  }
  return out
}

function drawFundaBadge(
  ctx: CanvasRenderingContext2D,
  boundingWidth: number,
  label: string,
  hasData: boolean,
): void {
  ctx.save()
  ctx.font = '600 10px Inter, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif'
  const tw = ctx.measureText(label).width
  const bw = tw + 12
  const bx = Math.max(6, boundingWidth - bw - 8)
  const by = 5
  ctx.fillStyle = hasData ? 'rgba(41, 98, 255, 0.16)' : 'rgba(120, 123, 134, 0.18)'
  ctx.strokeStyle = hasData ? '#2962FF' : '#787B86'
  ctx.lineWidth = 1
  ctx.beginPath()
  ctx.roundRect(bx, by, bw, 17, 3)
  ctx.fill()
  ctx.stroke()
  ctx.fillStyle = hasData ? '#5B8DEF' : '#9CA3AF'
  ctx.textBaseline = 'middle'
  ctx.fillText(label, bx + 6, by + 8.5)
  ctx.restore()
}

interface FinPoint {
  val?: number
  hasData?: number
}

function createFundamentalIndicator(opts: {
  name: string
  shortName: string
  title: string
  metric: 'pe' | 'pb' | 'roe' | 'epsYoy'
  color: string
  suffix: string
  precision: number
}): IndicatorTemplate<FinPoint> {
  return {
    name: opts.name,
    shortName: opts.shortName,
    series: 'normal',
    calcParams: [],
    precision: opts.precision,
    shouldOhlc: false,
    figures: [
      {
        key: 'val',
        title: `${opts.title}: `,
        type: 'line',
        styles: () => ({ color: opts.color, size: 1.8 }),
      },
    ],
    calc: (dataList: KLineData[], indicator) => {
      const symHint = (indicator.extendData as { ticker?: string } | undefined)?.ticker ?? ''
      const cached = symHint ? getFundamentalsCache(symHint) : undefined
      if (!cached || cached.quarterlyValuations.length === 0) {
        // Strict Point-in-Time discipline (`W6-06`): NEVER draw a lookahead horizontal line
        // across past years using today's scalar P/E or P/B when quarterly history is unavailable!
        return dataList.map(() => ({ hasData: 0 }))
      }

      const series = buildQuarterlyStepSeries(dataList, cached.quarterlyValuations, opts.metric)
      const anyVal = series.some((v) => v !== undefined)
      if (!anyVal) return dataList.map(() => ({ hasData: 0 }))
      return series.map((val) => ({ val, hasData: val !== undefined ? 1 : 0 }))
    },
    createTooltipDataSource: ({ indicator, crosshair }) => {
      const idx = crosshair.dataIndex
      const pt = idx !== undefined ? indicator.result[idx] : indicator.result[indicator.result.length - 1]
      const formatted =
        typeof pt?.val === 'number' && Number.isFinite(pt.val)
          ? `${pt.val.toFixed(opts.precision)}${opts.suffix}`
          : '—'
      const legends: TooltipLegend[] = [{ title: `${opts.title}: `, value: formatted }]
      return {
        name: `${opts.shortName} (PIT)`,
        calcParamsText: '',
        legends,
        features: [],
      }
    },
    draw: ({ ctx, bounding, indicator }) => {
      const symHint = (indicator.extendData as { ticker?: string } | undefined)?.ticker ?? ''
      const cached = symHint ? getFundamentalsCache(symHint) : undefined
      const hasData = indicator.result.some((r) => r.hasData === 1 && r.val !== undefined)
      const srcLabel =
        cached?.source === 'yumeko_dn01'
          ? 'Yumeko DN01'
          : cached?.source === 'tradingview_fundamental'
            ? 'TradingView Quý'
            : 'BCTC Quý'
      drawFundaBadge(
        ctx,
        bounding.width,
        hasData ? `✓ ${opts.shortName} · ${srcLabel} (PIT)` : `Chưa có BCTC Quý (${opts.shortName})`,
        hasData,
      )
      return false
    },
  }
}

export const finPeIndicator = createFundamentalIndicator({
  name: 'FIN_PE',
  shortName: 'P/E',
  title: 'P/E',
  metric: 'pe',
  color: '#2962FF',
  suffix: 'x',
  precision: 2,
})

export const finPbIndicator = createFundamentalIndicator({
  name: 'FIN_PB',
  shortName: 'P/B',
  title: 'P/B',
  metric: 'pb',
  color: '#089981',
  suffix: 'x',
  precision: 2,
})

export const finRoeIndicator = createFundamentalIndicator({
  name: 'FIN_ROE',
  shortName: 'ROE',
  title: 'ROE',
  metric: 'roe',
  color: '#FF9800',
  suffix: '%',
  precision: 1,
})

export const finEpsYoyIndicator = createFundamentalIndicator({
  name: 'FIN_EPS_YOY',
  shortName: 'EPS YoY',
  title: 'EPS YoY',
  metric: 'epsYoy',
  color: '#E040FB',
  suffix: '%',
  precision: 1,
})

/**
 * Multi-symbol causal `%`-rebased Comparison Overlay drawn cleanly on `candle_pane`:
 * aligns each compared symbol's percentage trajectory to the left-most visible base candlestick close,
 * preserving TradingView's crisp visual comparison while keeping the main symbol's price axis legible.
 */
export const compareOverlayIndicator: IndicatorTemplate<Record<string, never>> = {
  name: 'COMPARE_OVERLAY',
  shortName: 'So sánh',
  series: 'price',
  calcParams: [],
  shouldOhlc: false,
  figures: [],
  calc: (dataList: KLineData[]) => dataList.map(() => ({})),
  draw: ({ ctx, chart, bounding, xAxis, yAxis, indicator }) => {
    const slotId = (indicator.extendData as { slotId?: string } | undefined)?.slotId
    if (!slotId) return false
    const resp = getCompareSeriesCache(slotId)
    if (!resp || resp.series.length === 0) return false

    const dataList = chart.getDataList()
    if (dataList.length === 0) return false
    const visibleRange = chart.getVisibleRange()
    const fromIdx = Math.max(0, visibleRange.from)
    const toIdx = Math.min(dataList.length - 1, visibleRange.to)
    if (fromIdx > toIdx) return false

    const anchorBar = dataList[fromIdx]
    const anchorBaseClose = anchorBar?.close
    if (!anchorBaseClose || !Number.isFinite(anchorBaseClose) || anchorBaseClose <= 0) return false

    ctx.save()
    ctx.beginPath()
    ctx.rect(bounding.left, bounding.top, bounding.width, bounding.height)
    ctx.clip()

    resp.series.forEach((item, sIdx) => {
      if (item.points.length === 0) return
      const color = COMPARE_COLORS[sIdx % COMPARE_COLORS.length]
      const pctByTs = new Map<number, number>()
      for (const pt of item.points) {
        pctByTs.set(pt.timestamp, pt.pct)
      }

      // Find visible anchor pct so curve starts cleanly at the left edge's base candlestick level!
      let anchorPct: number | null = null
      for (let i = fromIdx; i <= toIdx; i++) {
        const p = pctByTs.get(dataList[i].timestamp)
        if (typeof p === 'number' && Number.isFinite(p)) {
          anchorPct = p
          break
        }
      }
      if (anchorPct === null) return

      ctx.strokeStyle = color
      ctx.lineWidth = 1.7
      ctx.beginPath()
      let started = false
      let lastX = 0
      let lastY = 0
      let lastRelPct = 0

      for (let i = fromIdx; i <= toIdx; i++) {
        const pct = pctByTs.get(dataList[i].timestamp)
        if (typeof pct !== 'number' || !Number.isFinite(pct)) continue
        const relFactor = (100 + pct) / Math.max(1e-6, 100 + anchorPct)
        const mappedPrice = anchorBaseClose * relFactor
        const x = xAxis.convertToPixel(i)
        const y = yAxis.convertToPixel(mappedPrice)
        if (!Number.isFinite(x) || !Number.isFinite(y)) continue
        if (!started) {
          ctx.moveTo(x, y)
          started = true
        } else {
          ctx.lineTo(x, y)
        }
        lastX = x
        lastY = y
        lastRelPct = (relFactor - 1) * 100
      }
      if (started) {
        ctx.stroke()
        // Right-end symbol + % tag pill
        const sign = lastRelPct >= 0 ? '+' : ''
        const tag = `${item.symbol} ${sign}${lastRelPct.toFixed(1)}%`
        ctx.font = '600 10px Inter, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif'
        const tw = ctx.measureText(tag).width
        const bx = Math.min(bounding.left + bounding.width - tw - 14, lastX + 6)
        const by = Math.max(bounding.top + 4, Math.min(bounding.top + bounding.height - 20, lastY - 8))
        ctx.fillStyle = color
        ctx.beginPath()
        ctx.roundRect(bx, by, tw + 10, 16, 3)
        ctx.fill()
        ctx.fillStyle = '#0B0E14'
        ctx.textBaseline = 'middle'
        ctx.fillText(tag, bx + 5, by + 8)
      }
    })

    ctx.restore()
    return false
  },
}

export const WAVE4_FUNDAMENTAL_INDICATORS: IndicatorTemplate[] = [
  finPeIndicator as IndicatorTemplate,
  finPbIndicator as IndicatorTemplate,
  finRoeIndicator as IndicatorTemplate,
  finEpsYoyIndicator as IndicatorTemplate,
  compareOverlayIndicator as IndicatorTemplate,
]
