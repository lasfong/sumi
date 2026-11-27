/**
 * Authentic Yumeko QMV Blackbox Real-Data Indicators (`Wave 2`):
 *   - `QMV_STOCK_BB`     (`TMC`, `TMA20`, `TMA50`, `DSPI` — green `QMV Blackbox (Thật)` badge,
 *                         transparent amber `Fallback: OHLCV Proxy` badge when Yumeko DB lacks rows)
 *   - `QMV_ACTIVE_MONEY` (Real *Cáo (Shark)*, *Sói (Wolf)*, *Thỏ (Sheep)* active money flow)
 *   - `QMV_FOREIGN_FLOW` (Real foreign institutional net value histogram `BuyValue - SellValue`)
 */
import type { IndicatorTemplate, KLineData, TooltipLegend } from 'klinecharts'
import type { QmvRealChartSeriesDto } from '../../types/market'

const qmvSeriesBySymbol = new Map<string, QmvRealChartSeriesDto>()

function bareSym(ticker: string): string {
  const up = ticker.trim().toUpperCase()
  const idx = up.lastIndexOf(':')
  return idx >= 0 ? up.slice(idx + 1) : up
}

function barDateKeyUtc(timestampMs: number): string {
  const d = new Date(timestampMs + 7 * 3_600_000)
  const y = d.getUTCFullYear()
  const m = String(d.getUTCMonth() + 1).padStart(2, '0')
  const day = String(d.getUTCDate()).padStart(2, '0')
  return `${y}-${m}-${day}`
}

export function setQmvRealSeriesCache(ticker: string, series: QmvRealChartSeriesDto): void {
  qmvSeriesBySymbol.set(bareSym(ticker), series)
}

export function getQmvRealSeriesCache(ticker: string): QmvRealChartSeriesDto | undefined {
  return qmvSeriesBySymbol.get(bareSym(ticker))
}

export function clearQmvRealSeriesCache(): void {
  qmvSeriesBySymbol.clear()
}

function drawProvenanceBadge(
  ctx: CanvasRenderingContext2D,
  boundingWidth: number,
  label: string,
  isReal: boolean,
): void {
  ctx.save()
  ctx.font = '600 10px Inter, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif'
  const tw = ctx.measureText(label).width
  const bw = tw + 12
  const bx = Math.max(6, boundingWidth - bw - 8)
  const by = 5
  ctx.fillStyle = isReal ? 'rgba(8, 153, 129, 0.18)' : 'rgba(251, 192, 45, 0.18)'
  ctx.strokeStyle = isReal ? '#089981' : '#FBC02D'
  ctx.lineWidth = 1
  ctx.beginPath()
  ctx.roundRect(bx, by, bw, 17, 3)
  ctx.fill()
  ctx.stroke()
  ctx.fillStyle = isReal ? '#089981' : '#FBC02D'
  ctx.textBaseline = 'middle'
  ctx.fillText(label, bx + 6, by + 8.5)
  ctx.restore()
}

export interface QmvStockBbChartPt {
  tmc?: number
  tma20?: number
  tma50?: number
  dspi?: number
  isReal?: number
}

export const qmvStockBbIndicator: IndicatorTemplate<QmvStockBbChartPt> = {
  name: 'QMV_STOCK_BB',
  shortName: 'QMV BB',
  series: 'normal',
  calcParams: [],
  precision: 2,
  shouldOhlc: false,
  figures: [
    { key: 'tmc', title: 'TMC: ', type: 'line', styles: () => ({ color: '#089981', size: 1.8 }) },
    { key: 'tma20', title: 'TMA20: ', type: 'line', styles: () => ({ color: '#2962FF', size: 1.4 }) },
    { key: 'tma50', title: 'TMA50: ', type: 'line', styles: () => ({ color: '#FF9800', size: 1.3 }) },
    { key: 'dspi', title: 'DSPI: ', type: 'line', styles: () => ({ color: '#E040FB', size: 1.3, style: 'dashed' }) },
  ],
  calc: (dataList: KLineData[], indicator) => {
    const symHint = (indicator.extendData as { ticker?: string } | undefined)?.ticker ?? ''
    const cached = symHint ? getQmvRealSeriesCache(symHint) : undefined
    if (cached && cached.isProviderProprietary && cached.stockBb.length > 0) {
      const byDate = new Map(cached.stockBb.map((p) => [p.date, p]))
      return dataList.map((bar) => {
        const pt = byDate.get(barDateKeyUtc(bar.timestamp))
        if (!pt) return { isReal: 1 }
        return {
          tmc: pt.tmc ?? undefined,
          tma20: pt.tma20 ?? undefined,
          tma50: pt.tma50 ?? undefined,
          dspi: pt.dspi ?? undefined,
          isReal: 1,
        }
      })
    }
    // Transparent OHLCV Proxy fallback (never claims `qmv_real`!)
    const rawFlow: number[] = dataList.map((b, i) => {
      const rng = Math.max(1e-9, b.high - b.low)
      const clv = ((b.close - b.low) - (b.high - b.close)) / rng
      const ret = i > 0 && dataList[i - 1].close > 0 ? (b.close - dataList[i - 1].close) / dataList[i - 1].close : 0
      return 50 + clv * 25 + Math.max(-1, Math.min(1, ret * 14)) * 25
    })
    return dataList.map((_, i) => {
      const avg = (w: number) => {
        const s = Math.max(0, i - w + 1)
        let sum = 0
        for (let j = s; j <= i; j++) sum += rawFlow[j]
        return sum / (i - s + 1)
      }
      return {
        tmc: avg(5),
        tma20: avg(20),
        tma50: avg(50),
        dspi: avg(10),
        isReal: 0,
      }
    })
  },
  createTooltipDataSource: ({ indicator, crosshair }) => {
    const idx = crosshair.dataIndex
    const pt = idx !== undefined ? indicator.result[idx] : indicator.result[indicator.result.length - 1]
    const isReal = pt?.isReal === 1
    const fmt = (v?: number) => (typeof v === 'number' && Number.isFinite(v) ? v.toFixed(1) : '—')
    const legends: TooltipLegend[] = [
      { title: 'TMC: ', value: fmt(pt?.tmc) },
      { title: 'TMA20: ', value: fmt(pt?.tma20) },
      { title: 'TMA50: ', value: fmt(pt?.tma50) },
      { title: 'DSPI: ', value: fmt(pt?.dspi) },
    ]
    return {
      name: isReal ? 'QMV Blackbox (Thật)' : 'QMV BB [Fallback: OHLCV Proxy]',
      calcParamsText: '',
      legends,
      features: (indicator.styles as { tooltip?: { features?: never[] } } | null)?.tooltip?.features ?? [],
    }
  },
  draw: ({ ctx, bounding, indicator }) => {
    const last = indicator.result[indicator.result.length - 1]
    const isReal = last?.isReal === 1
    drawProvenanceBadge(
      ctx,
      bounding.width,
      isReal ? '✓ QMV Blackbox (Thật)' : '⚠ Fallback: OHLCV Proxy',
      isReal,
    )
    return false
  },
}

export interface QmvActiveMoneyChartPt {
  shark?: number
  wolf?: number
  sheep?: number
  isReal?: number
}

export const qmvActiveMoneyIndicator: IndicatorTemplate<QmvActiveMoneyChartPt> = {
  name: 'QMV_ACTIVE_MONEY',
  shortName: 'Cáo-Sói-Thỏ',
  series: 'normal',
  calcParams: [],
  precision: 1,
  shouldOhlc: false,
  figures: [
    { key: 'shark', title: 'Cáo (Shark): ', type: 'line', styles: () => ({ color: '#089981', size: 1.8 }) },
    { key: 'wolf', title: 'Sói (Wolf): ', type: 'line', styles: () => ({ color: '#2962FF', size: 1.5 }) },
    { key: 'sheep', title: 'Thỏ (Sheep): ', type: 'line', styles: () => ({ color: '#F23645', size: 1.3 }) },
  ],
  calc: (dataList: KLineData[], indicator) => {
    const symHint = (indicator.extendData as { ticker?: string } | undefined)?.ticker ?? ''
    const cached = symHint ? getQmvRealSeriesCache(symHint) : undefined
    if (!cached || cached.activeMoney.length === 0) {
      return dataList.map(() => ({ isReal: 0 }))
    }
    const byDate = new Map(cached.activeMoney.map((p) => [p.date, p]))
    return dataList.map((bar) => {
      const pt = byDate.get(barDateKeyUtc(bar.timestamp))
      if (!pt) return { isReal: 1 }
      return {
        shark: pt.shark ?? undefined,
        wolf: pt.wolf ?? undefined,
        sheep: pt.sheep ?? undefined,
        isReal: 1,
      }
    })
  },
  draw: ({ ctx, bounding, indicator }) => {
    const hasReal = indicator.result.some((r) => r.isReal === 1 && (r.shark !== undefined || r.wolf !== undefined))
    drawProvenanceBadge(
      ctx,
      bounding.width,
      hasReal ? '✓ QMV Cáo–Sói–Thỏ (Thật)' : 'Chưa có dữ liệu Active Money từ Yumeko',
      hasReal,
    )
    return false
  },
}

export interface QmvForeignFlowChartPt {
  netValue?: number
  isReal?: number
}

export const qmvForeignFlowIndicator: IndicatorTemplate<QmvForeignFlowChartPt> = {
  name: 'QMV_FOREIGN_FLOW',
  shortName: 'Khối Ngoại',
  series: 'normal',
  calcParams: [],
  precision: 2,
  shouldOhlc: false,
  figures: [
    {
      key: 'netValue',
      title: 'Khối ngoại ròng: ',
      type: 'bar',
      baseValue: 0,
      styles: ({ data }) => {
        const v = data.current?.netValue ?? 0
        return {
          style: 'fill',
          color: v >= 0 ? 'rgba(8, 153, 129, 0.85)' : 'rgba(242, 54, 69, 0.85)',
        }
      },
    },
  ],
  calc: (dataList: KLineData[], indicator) => {
    const symHint = (indicator.extendData as { ticker?: string } | undefined)?.ticker ?? ''
    const cached = symHint ? getQmvRealSeriesCache(symHint) : undefined
    if (!cached || cached.foreignFlow.length === 0) {
      return dataList.map(() => ({ isReal: 0 }))
    }
    const byDate = new Map(cached.foreignFlow.map((p) => [p.date, p]))
    return dataList.map((bar) => {
      const pt = byDate.get(barDateKeyUtc(bar.timestamp))
      if (!pt) return { isReal: 1 }
      return {
        netValue: pt.netValue ?? undefined,
        isReal: 1,
      }
    })
  },
  draw: ({ ctx, bounding, indicator }) => {
    const hasReal = indicator.result.some((r) => r.isReal === 1 && r.netValue !== undefined)
    drawProvenanceBadge(
      ctx,
      bounding.width,
      hasReal ? '✓ QMV Khối ngoại (Thật)' : 'Chưa có dữ liệu Khối ngoại từ Yumeko',
      hasReal,
    )
    return false
  },
}

export const YUMEKO_REAL_INDICATORS = [
  qmvStockBbIndicator,
  qmvActiveMoneyIndicator,
  qmvForeignFlowIndicator,
] as const
