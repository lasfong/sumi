/**
 * 4 TradingView-Signature Custom Overlays + Vietnam Price Band Line for KLineChart v10.1.1:
 *  1. longPosition   — Green Target / Red Stop Risk:Reward projection box with % Target, % Stop & R:R
 *  2. shortPosition  — Short Risk:Reward projection box with % Target, % Stop & R:R
 *  3. priceDateRange — Price & Date Range measurement box (ΔPrice, %, Bars, Time span, Sum Volume)
 *  4. brokerOrderLine — Horizontal working order / open position line with right-aligned badge
 *  5. vnPriceBandLine — Vietnam Ceiling (#CE93D8) / Reference (#FBC02D) / Floor (#26C6DA) band marker
 */
import type { OverlayFigure, OverlayTemplate } from 'klinecharts'
import { formatDurationVietnamese } from '../periods'
import { formatChartPrice } from './utils'

function createPositionOverlay(side: 'long' | 'short'): OverlayTemplate {
  const isLong = side === 'long'
  return {
    name: isLong ? 'longPosition' : 'shortPosition',
    totalStep: 4, // 3 control points: [0]=Entry, [1]=Target (TP), [2]=Stop (SL)
    needDefaultPointFigure: true,
    needDefaultXAxisFigure: true,
    needDefaultYAxisFigure: true,
    createPointFigures: ({ chart, coordinates, overlay }) => {
      if (coordinates.length < 2) return []
      const pts = overlay.points
      const entryPt = pts[0]
      const tpPt = pts[1]
      const slPt = pts[2]

      const c0 = coordinates[0]
      const c1 = coordinates[1]
      const c2 = coordinates[2]

      const x1 = Math.min(c0.x, c1.x, c2?.x ?? c1.x)
      const rawX2 = Math.max(c0.x, c1.x, c2?.x ?? c1.x)
      const x2 = Math.max(rawX2, x1 + 96)
      const width = x2 - x1

      const entryVal = typeof entryPt?.value === 'number' ? entryPt.value : 0
      const tpVal = typeof tpPt?.value === 'number' ? tpPt.value : entryVal
      const slVal =
        typeof slPt?.value === 'number'
          ? slPt.value
          : isLong
            ? entryVal - Math.abs(tpVal - entryVal) * 0.5
            : entryVal + Math.abs(entryVal - tpVal) * 0.5

      const entryY = c0.y
      const tpY = c1.y
      const slY = c2 ? c2.y : entryY + (entryY - tpY) * 0.5

      const targetDiff = isLong ? tpVal - entryVal : entryVal - tpVal
      const stopDiff = isLong ? entryVal - slVal : slVal - entryVal
      const targetPct = entryVal > 0 ? (targetDiff / entryVal) * 100 : 0
      const stopPct = entryVal > 0 ? (stopDiff / entryVal) * 100 : 0
      const rr = stopDiff > 1e-9 ? targetDiff / stopDiff : 0

      const tpTop = Math.min(entryY, tpY)
      const tpHeight = Math.max(2, Math.abs(tpY - entryY))
      const slTop = Math.min(entryY, slY)
      const slHeight = Math.max(2, Math.abs(slY - entryY))

      const figures: OverlayFigure[] = [
        // Profit (Target) Zone — Emerald Green
        {
          type: 'rect',
          attrs: { x: x1, y: tpTop, width, height: tpHeight },
          styles: {
            style: 'stroke_fill',
            color: 'rgba(8, 153, 129, 0.22)',
            borderColor: '#089981',
            borderSize: 1,
          },
        },
        // Risk (Stop Loss) Zone — Crimson Red
        {
          type: 'rect',
          attrs: { x: x1, y: slTop, width, height: slHeight },
          styles: {
            style: 'stroke_fill',
            color: 'rgba(242, 54, 69, 0.22)',
            borderColor: '#F23645',
            borderSize: 1,
          },
        },
        // Entry Midline
        {
          type: 'line',
          attrs: {
            coordinates: [
              { x: x1, y: entryY },
              { x: x2, y: entryY },
            ],
          },
          styles: { style: 'solid', color: '#787B86', size: 1.5 },
        },
        // Target Badge
        {
          type: 'text',
          ignoreEvent: true,
          attrs: {
            x: x1 + 6,
            y: isLong ? tpTop - 4 : tpTop + tpHeight + 4,
            text: `TP ${formatChartPrice(chart, tpVal)} (${targetPct >= 0 ? '+' : ''}${targetPct.toFixed(2)}%)`,
            baseline: isLong ? 'bottom' : 'top',
          },
          styles: {
            style: 'fill',
            color: '#FFFFFF',
            backgroundColor: '#089981',
            borderRadius: 3,
            paddingLeft: 6,
            paddingRight: 6,
            paddingTop: 2,
            paddingBottom: 2,
            size: 11,
          },
        },
        // Center R:R Summary Pill
        {
          type: 'text',
          ignoreEvent: true,
          attrs: {
            x: x1 + width / 2,
            y: entryY,
            text: `${isLong ? 'LONG' : 'SHORT'}  R:R ${rr > 0 && Number.isFinite(rr) ? rr.toFixed(2) : '—'}`,
            align: 'center',
            baseline: 'middle',
          },
          styles: {
            style: 'fill',
            color: '#FFFFFF',
            backgroundColor: '#1E222D',
            borderColor: '#2A2E39',
            borderSize: 1,
            borderRadius: 3,
            paddingLeft: 8,
            paddingRight: 8,
            paddingTop: 3,
            paddingBottom: 3,
            size: 11,
            weight: '600',
          },
        },
        // Stop Loss Badge
        {
          type: 'text',
          ignoreEvent: true,
          attrs: {
            x: x1 + 6,
            y: isLong ? slTop + slHeight + 4 : slTop - 4,
            text: `SL ${formatChartPrice(chart, slVal)} (-${Math.abs(stopPct).toFixed(2)}%)`,
            baseline: isLong ? 'top' : 'bottom',
          },
          styles: {
            style: 'fill',
            color: '#FFFFFF',
            backgroundColor: '#F23645',
            borderRadius: 3,
            paddingLeft: 6,
            paddingRight: 6,
            paddingTop: 2,
            paddingBottom: 2,
            size: 11,
          },
        },
      ]
      return figures
    },
  }
}

export const longPosition: OverlayTemplate = createPositionOverlay('long')
export const shortPosition: OverlayTemplate = createPositionOverlay('short')

export const priceDateRange: OverlayTemplate = {
  name: 'priceDateRange',
  totalStep: 3,
  needDefaultPointFigure: true,
  needDefaultXAxisFigure: true,
  needDefaultYAxisFigure: true,
  createPointFigures: ({ chart, coordinates, overlay }) => {
    if (coordinates.length < 2) return []
    const c0 = coordinates[0]
    const c1 = coordinates[1]
    const p0 = overlay.points[0]
    const p1 = overlay.points[1]

    const x = Math.min(c0.x, c1.x)
    const y = Math.min(c0.y, c1.y)
    const width = Math.max(1, Math.abs(c1.x - c0.x))
    const height = Math.max(1, Math.abs(c1.y - c0.y))

    const v0 = typeof p0?.value === 'number' ? p0.value : 0
    const v1 = typeof p1?.value === 'number' ? p1.value : 0
    const diff = v1 - v0
    const pct = v0 > 0 ? (diff / v0) * 100 : 0
    const isUp = diff >= 0

    const idx0 = typeof p0?.dataIndex === 'number' ? p0.dataIndex : 0
    const idx1 = typeof p1?.dataIndex === 'number' ? p1.dataIndex : 0
    const barCount = Math.abs(Math.round(idx1 - idx0))

    const t0 = typeof p0?.timestamp === 'number' ? p0.timestamp : 0
    const t1 = typeof p1?.timestamp === 'number' ? p1.timestamp : 0
    const durationText = t0 > 0 && t1 > 0 ? formatDurationVietnamese(t1 - t0) : `${barCount} nến`

    // Sum volume across measured bar span
    const dataList = chart.getDataList()
    const lo = Math.max(0, Math.min(Math.round(idx0), Math.round(idx1)))
    const hi = Math.min(dataList.length - 1, Math.max(Math.round(idx0), Math.round(idx1)))
    let sumVol = 0
    for (let i = lo; i <= hi; i++) {
      sumVol += dataList[i]?.volume ?? 0
    }
    const volText =
      sumVol >= 1_000_000
        ? `${(sumVol / 1_000_000).toFixed(2)}M`
        : sumVol >= 1_000
          ? `${(sumVol / 1_000).toFixed(1)}K`
          : `${Math.round(sumVol)}`

    const accent = isUp ? '#2962FF' : '#F23645'
    const fill = isUp ? 'rgba(41, 98, 255, 0.16)' : 'rgba(242, 54, 69, 0.16)'
    const midX = x + width / 2
    const midY = y + height / 2
    const sign = diff >= 0 ? '+' : ''
    const label = `${sign}${formatChartPrice(chart, diff)} (${sign}${pct.toFixed(2)}%)  |  ${barCount} nến · ${durationText} · Vol ${volText}`

    return [
      {
        type: 'rect',
        attrs: { x, y, width, height },
        styles: {
          style: 'stroke_fill',
          color: fill,
          borderColor: accent,
          borderSize: 1,
        },
      },
      {
        type: 'line',
        ignoreEvent: true,
        attrs: {
          coordinates: [
            { x, y: midY },
            { x: x + width, y: midY },
          ],
        },
        styles: { style: 'dashed', color: accent, size: 1, dashedValue: [3, 3] },
      },
      {
        type: 'line',
        ignoreEvent: true,
        attrs: {
          coordinates: [
            { x: midX, y },
            { x: midX, y: y + height },
          ],
        },
        styles: { style: 'dashed', color: accent, size: 1, dashedValue: [3, 3] },
      },
      {
        type: 'text',
        ignoreEvent: true,
        attrs: {
          x: midX,
          y: isUp ? y - 6 : y + height + 6,
          text: label,
          align: 'center',
          baseline: isUp ? 'bottom' : 'top',
        },
        styles: {
          style: 'fill',
          color: '#FFFFFF',
          backgroundColor: accent,
          borderRadius: 4,
          paddingLeft: 8,
          paddingRight: 8,
          paddingTop: 4,
          paddingBottom: 4,
          size: 11,
          weight: '600',
        },
      },
    ]
  },
}

export interface BrokerOrderExtendData {
  label: string
  color: string
  dashed?: boolean
  draggable?: boolean
  prefix?: string
  entryPrice?: number
  side?: 'Long' | 'Short'
  qty?: number
  mult?: number
  isVnd?: boolean
}

export const brokerOrderLine: OverlayTemplate<BrokerOrderExtendData> = {
  name: 'brokerOrderLine',
  totalStep: 2,
  needDefaultPointFigure: false,
  needDefaultXAxisFigure: false,
  needDefaultYAxisFigure: true,
  createPointFigures: ({ chart, coordinates, bounding, overlay }) => {
    if (coordinates.length === 0) return []
    const y = coordinates[0].y
    const ext = overlay.extendData
    const color = ext?.color ?? '#2962FF'
    const priceVal = overlay.points[0]?.value

    let label =
      ext?.label ??
      (typeof priceVal === 'number' ? `LỆNH ${formatChartPrice(chart, priceVal)}` : 'LỆNH CHỜ')

    // Dynamically update label with real-time price & projected PnL while dragging SL / TP / LO (v2.2)!
    if (ext?.draggable && typeof priceVal === 'number' && Number.isFinite(priceVal)) {
      const priceText = formatChartPrice(chart, priceVal)
      if (
        typeof ext.entryPrice === 'number' &&
        typeof ext.qty === 'number' &&
        ext.side
      ) {
        const mult = ext.mult ?? 1
        const dir = ext.side === 'Long' ? 1 : -1
        const projPnl = (priceVal - ext.entryPrice) * ext.qty * mult * dir
        const sign = projPnl >= 0 ? '+' : ''
        const pnlStr = ext.isVnd
          ? `${sign}${Math.round(projPnl).toLocaleString('en-US')} đ`
          : `${sign}$${projPnl.toFixed(2)}`
        label = `⋮⋮ ${ext.prefix ?? 'MỨC'} ${priceText} (${pnlStr})`
      } else if (ext.prefix) {
        label = `⋮⋮ ${ext.prefix} @ ${priceText}`
      }
    }

    return [
      {
        type: 'line',
        attrs: {
          coordinates: [
            { x: 0, y },
            { x: bounding.width, y },
          ],
        },
        styles: {
          style: ext?.dashed ? 'dashed' : 'solid',
          color,
          size: ext?.draggable ? 1.8 : 1.5,
          dashedValue: [5, 3],
        },
      },
      {
        type: 'text',
        attrs: {
          x: Math.max(8, bounding.width - 14),
          y,
          text: label,
          align: 'right',
          baseline: 'middle',
        },
        styles: {
          style: 'fill',
          color: '#FFFFFF',
          backgroundColor: color,
          borderRadius: 3,
          paddingLeft: 6,
          paddingRight: 6,
          paddingTop: 2.5,
          paddingBottom: 2.5,
          size: 10.5,
          weight: '600',
        },
      },
    ]
  },
}

export interface VnBandExtendData {
  tag: 'TRẦN' | 'THAM CHIẾU' | 'SÀN'
  color: string
}

export const vnPriceBandLine: OverlayTemplate<VnBandExtendData> = {
  name: 'vnPriceBandLine',
  totalStep: 2,
  lock: true,
  needDefaultPointFigure: false,
  needDefaultXAxisFigure: false,
  needDefaultYAxisFigure: false,
  createPointFigures: ({ chart, coordinates, bounding, overlay }) => {
    if (coordinates.length === 0) return []
    const y = coordinates[0].y
    const ext = overlay.extendData
    const color = ext?.color ?? '#FBC02D'
    const tag = ext?.tag ?? 'THAM CHIẾU'
    const shortTag = tag === 'THAM CHIẾU' ? 'TC' : tag
    const val = overlay.points[0]?.value
    const text = typeof val === 'number' ? `${shortTag} ${formatChartPrice(chart, val)}` : shortTag

    return [
      {
        type: 'line',
        ignoreEvent: true,
        attrs: {
          coordinates: [
            { x: 0, y },
            { x: bounding.width, y },
          ],
        },
        styles: {
          style: 'dashed',
          color,
          size: 1,
          dashedValue: [4, 4],
        },
      },
      {
        type: 'text',
        ignoreEvent: true,
        attrs: {
          x: Math.max(8, bounding.width - 12),
          y: y - 3,
          text,
          align: 'right',
          baseline: 'bottom',
        },
        styles: {
          style: 'fill',
          color: '#131722',
          backgroundColor: color,
          borderRadius: 2,
          paddingLeft: 5,
          paddingRight: 5,
          paddingTop: 1,
          paddingBottom: 1,
          size: 9.5,
          weight: '600',
        },
      },
    ]
  },
}

export interface BacktestTradeMarkerExtendData {
  kind: 'buy' | 'sell'
  label: string
  color: string
}

export const backtestTradeMarker: OverlayTemplate<BacktestTradeMarkerExtendData> = {
  name: 'backtestTradeMarker',
  totalStep: 2,
  lock: true,
  needDefaultPointFigure: false,
  needDefaultXAxisFigure: false,
  needDefaultYAxisFigure: false,
  createPointFigures: ({ coordinates, overlay }) => {
    if (coordinates.length === 0) return []
    const { x, y } = coordinates[0]
    const ext = overlay.extendData
    const isBuy = ext?.kind !== 'sell'
    const color = ext?.color ?? (isBuy ? '#089981' : '#F23645')
    const label = ext?.label ?? (isBuy ? 'MUA' : 'BÁN')

    const arrowTipY = isBuy ? y + 5 : y - 5
    const arrowBaseY = isBuy ? y + 14 : y - 14
    const textY = isBuy ? y + 17 : y - 17

    return [
      {
        type: 'polygon',
        ignoreEvent: true,
        attrs: {
          coordinates: [
            { x, y: arrowTipY },
            { x: x - 5.5, y: arrowBaseY },
            { x: x + 5.5, y: arrowBaseY },
          ],
        },
        styles: {
          style: 'fill',
          color,
        },
      },
      {
        type: 'text',
        ignoreEvent: true,
        attrs: {
          x,
          y: textY,
          text: label,
          align: 'center',
          baseline: isBuy ? 'top' : 'bottom',
        },
        styles: {
          style: 'fill',
          color: '#FFFFFF',
          backgroundColor: color,
          borderRadius: 2,
          paddingLeft: 4,
          paddingRight: 4,
          paddingTop: 1.5,
          paddingBottom: 1.5,
          size: 9.5,
          weight: '600',
        },
      },
    ]
  },
}

export interface VnSessionBreakExtendData {
  label: string
  kind: 'morning' | 'afternoon' | 'atc'
}

export const vnSessionBreakLine: OverlayTemplate<VnSessionBreakExtendData> = {
  name: 'vnSessionBreakLine',
  totalStep: 2,
  lock: true,
  needDefaultPointFigure: false,
  needDefaultXAxisFigure: false,
  needDefaultYAxisFigure: false,
  createPointFigures: ({ coordinates, bounding, overlay }) => {
    if (coordinates.length === 0) return []
    const { x } = coordinates[0]
    const ext = overlay.extendData
    const kind = ext?.kind ?? 'morning'
    const lineColor =
      kind === 'morning'
        ? 'rgba(41, 98, 255, 0.28)'
        : kind === 'afternoon'
          ? 'rgba(251, 192, 45, 0.28)'
          : 'rgba(171, 71, 188, 0.30)'
    const textColor =
      kind === 'morning' ? '#6B9BFF' : kind === 'afternoon' ? '#FBC02D' : '#CE93D8'

    return [
      {
        type: 'line',
        ignoreEvent: true,
        attrs: {
          coordinates: [
            { x, y: 0 },
            { x, y: bounding.height },
          ],
        },
        styles: {
          style: 'dashed',
          color: lineColor,
          size: 1,
          dashedValue: kind === 'morning' ? [4, 4] : [2, 4],
        },
      },
      {
        type: 'text',
        ignoreEvent: true,
        attrs: {
          x: x + 3,
          y: Math.max(20, bounding.height - 8),
          text: ext?.label ?? '',
          align: 'left',
          baseline: 'bottom',
        },
        styles: {
          style: 'fill',
          color: textColor,
          backgroundColor: 'rgba(19, 23, 34, 0.65)',
          borderRadius: 2,
          paddingLeft: 3,
          paddingRight: 3,
          paddingTop: 1,
          paddingBottom: 1,
          size: 9,
          weight: '500',
        },
      },
    ]
  },
}

export const TRADINGVIEW_CUSTOM_OVERLAYS: OverlayTemplate[] = [
  longPosition,
  shortPosition,
  priceDateRange,
  brokerOrderLine as OverlayTemplate,
  vnPriceBandLine as OverlayTemplate,
  backtestTradeMarker as OverlayTemplate,
  vnSessionBreakLine as OverlayTemplate,
]
