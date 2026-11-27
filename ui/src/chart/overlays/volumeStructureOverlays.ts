/**
 * Wave 3 Institutional Volume Structure Drawing Overlays matching `sumi-core`:
 *   1. `anchoredVwap`            (1-click Anchor -> Causal Welford AVWAP + ±1σ / ±2σ bands)
 *   2. `fixedRangeVolumeProfile` (2-click Range -> Overlap-conserving Buy/Sell Volume Profile + POC + 70% VAH/VAL)
 */
import type { Chart, OverlayFigure, OverlayTemplate } from 'klinecharts'

function formatChartPrice(chart: Chart, value: number): string {
  if (!Number.isFinite(value)) return '—'
  const rawPrec = chart.getSymbol()?.pricePrecision ?? (Math.abs(value) >= 1000 ? 0 : 2)
  const p = Math.max(0, Math.min(8, Math.round(rawPrec)))
  return value.toLocaleString('en-US', {
    minimumFractionDigits: p,
    maximumFractionDigits: p,
  })
}

export const anchoredVwapOverlay: OverlayTemplate = {
  name: 'anchoredVwap',
  totalStep: 2, // 1 anchor click
  needDefaultPointFigure: true,
  needDefaultXAxisFigure: true,
  needDefaultYAxisFigure: true,
  createPointFigures: ({ chart, coordinates, overlay }) => {
    if (coordinates.length === 0) return []
    const dataList = chart.getDataList()
    if (dataList.length === 0) return []

    const pt0 = overlay.points[0]
    let anchorIdx = typeof pt0?.dataIndex === 'number' ? Math.round(pt0.dataIndex) : -1
    if (anchorIdx < 0 && typeof pt0?.timestamp === 'number') {
      anchorIdx = dataList.findIndex((b) => b.timestamp >= pt0.timestamp!)
    }
    if (anchorIdx < 0 || anchorIdx >= dataList.length) return []

    const range = chart.getVisibleRange()
    const visEnd = Math.min(dataList.length - 1, Math.max(anchorIdx, range.to + 2))

    // Exact volume-weighted Welford variance matching `sumi-core/src/indicators/vwap.rs`
    let sumW = 0
    let mean = 0
    let m2 = 0
    const ptsVwap: Array<{ x: number; y: number }> = []
    const ptsU1: Array<{ x: number; y: number }> = []
    const ptsL1: Array<{ x: number; y: number }> = []
    const ptsU2: Array<{ x: number; y: number }> = []
    const ptsL2: Array<{ x: number; y: number }> = []
    let lastVwap = 0

    for (let i = anchorIdx; i <= visEnd; i++) {
      const b = dataList[i]
      const tp = (b.high + b.low + b.close) / 3
      const rawVol = b.volume ?? 0
      const w = Number.isFinite(rawVol) && rawVol > 0 ? rawVol : 1
      const nextW = sumW + w
      const delta = tp - mean
      const nextMean = mean + (w / nextW) * delta
      m2 += w * delta * (tp - nextMean)
      sumW = nextW
      mean = nextMean
      const std = sumW > 0 ? Math.sqrt(Math.max(0, m2 / sumW)) : 0
      lastVwap = mean

      if (i >= Math.max(anchorIdx, range.from - 2)) {
        const conv = chart.convertToPixel(
          [
            { dataIndex: i, value: mean },
            { dataIndex: i, value: mean + std },
            { dataIndex: i, value: mean - std },
            { dataIndex: i, value: mean + 2 * std },
            { dataIndex: i, value: mean - 2 * std },
          ],
          { paneId: 'candle_pane' },
        )
        const arr = Array.isArray(conv) ? conv : [conv]
        if (arr[0]?.x !== undefined && arr[0]?.y !== undefined) {
          ptsVwap.push({ x: arr[0].x, y: arr[0].y })
          if (arr[1]?.y !== undefined) ptsU1.push({ x: arr[0].x, y: arr[1].y })
          if (arr[2]?.y !== undefined) ptsL1.push({ x: arr[0].x, y: arr[2].y })
          if (arr[3]?.y !== undefined) ptsU2.push({ x: arr[0].x, y: arr[3].y })
          if (arr[4]?.y !== undefined) ptsL2.push({ x: arr[0].x, y: arr[4].y })
        }
      }
    }

    if (ptsVwap.length === 0) return []
    const lastPt = ptsVwap[ptsVwap.length - 1]
    const anchorColor =
      (overlay.styles as { line?: { color?: string } } | null)?.line?.color ?? '#FF9800'

    const figures: OverlayFigure[] = [
      {
        type: 'line',
        attrs: { coordinates: ptsVwap },
        styles: { style: 'solid', color: anchorColor, size: 2 },
      },
    ]
    if (ptsU1.length >= 2) {
      figures.push(
        {
          type: 'line',
          ignoreEvent: true,
          attrs: { coordinates: ptsU1 },
          styles: { style: 'dashed', color: 'rgba(41, 98, 255, 0.72)', size: 1, dashedValue: [4, 3] },
        },
        {
          type: 'line',
          ignoreEvent: true,
          attrs: { coordinates: ptsL1 },
          styles: { style: 'dashed', color: 'rgba(41, 98, 255, 0.72)', size: 1, dashedValue: [4, 3] },
        },
      )
    }
    if (ptsU2.length >= 2) {
      figures.push(
        {
          type: 'line',
          ignoreEvent: true,
          attrs: { coordinates: ptsU2 },
          styles: { style: 'dashed', color: 'rgba(171, 71, 188, 0.55)', size: 1, dashedValue: [2, 3] },
        },
        {
          type: 'line',
          ignoreEvent: true,
          attrs: { coordinates: ptsL2 },
          styles: { style: 'dashed', color: 'rgba(171, 71, 188, 0.55)', size: 1, dashedValue: [2, 3] },
        },
      )
    }
    figures.push({
      type: 'text',
      ignoreEvent: true,
      attrs: {
        x: lastPt.x + 6,
        y: lastPt.y,
        text: `AVWAP ${formatChartPrice(chart, lastVwap)}`,
        align: 'left',
        baseline: 'middle',
      },
      styles: {
        style: 'fill',
        color: '#FFFFFF',
        backgroundColor: anchorColor,
        borderRadius: 3,
        paddingLeft: 5,
        paddingRight: 5,
        paddingTop: 2,
        paddingBottom: 2,
        size: 10,
        weight: '600',
      },
    })
    return figures
  },
}

export const fixedRangeVolumeProfileOverlay: OverlayTemplate = {
  name: 'fixedRangeVolumeProfile',
  totalStep: 3, // 2 points: range start & range end
  needDefaultPointFigure: true,
  needDefaultXAxisFigure: true,
  needDefaultYAxisFigure: true,
  createPointFigures: ({ chart, coordinates, overlay }) => {
    if (coordinates.length < 2) return []
    const dataList = chart.getDataList()
    if (dataList.length === 0) return []

    const resolveIdx = (pt: { dataIndex?: number; timestamp?: number } | undefined, fallback: number): number => {
      if (typeof pt?.dataIndex === 'number' && Number.isFinite(pt.dataIndex)) {
        return Math.max(0, Math.min(dataList.length - 1, Math.round(pt.dataIndex)))
      }
      if (typeof pt?.timestamp === 'number') {
        const found = dataList.findIndex((b) => b.timestamp >= pt.timestamp!)
        if (found >= 0) return found
      }
      return fallback
    }

    const i0 = resolveIdx(overlay.points[0], 0)
    const i1 = resolveIdx(overlay.points[1], dataList.length - 1)
    const loIdx = Math.min(i0, i1)
    const hiIdx = Math.max(i0, i1)
    if (hiIdx < loIdx) return []

    let minLow = Infinity
    let maxHigh = -Infinity
    let hasPosVol = false
    for (let i = loIdx; i <= hiIdx; i++) {
      const b = dataList[i]
      if (b.low < minLow) minLow = b.low
      if (b.high > maxHigh) maxHigh = b.high
      if ((b.volume ?? 0) > 0) hasPosVol = true
    }
    if (!Number.isFinite(minLow) || !Number.isFinite(maxHigh) || maxHigh <= minLow) return []

    const numBins = 28
    const step = (maxHigh - minLow) / numBins
    const buyBins = new Array<number>(numBins).fill(0)
    const sellBins = new Array<number>(numBins).fill(0)

    // Exact overlap-conserving attribution matching `sumi-core/src/indicators/volume_profile.rs`
    for (let i = loIdx; i <= hiIdx; i++) {
      const b = dataList[i]
      const vol = hasPosVol ? Math.max(0, b.volume ?? 0) : 1
      if (vol <= 0) continue
      const rng = b.high - b.low
      const buyFrac =
        rng > 1e-12
          ? 0.5 * (1 + Math.max(-1, Math.min(1, ((b.close - b.low) + (b.close - b.high)) / rng)))
          : b.close >= b.open
            ? 1
            : 0
      const bLo = Math.max(0, Math.min(numBins - 1, Math.floor((b.low - minLow) / step)))
      const bHi = Math.max(0, Math.min(numBins - 1, Math.floor((b.high - minLow) / step)))
      if (rng <= 1e-12 || bLo === bHi) {
        buyBins[bLo] += vol * buyFrac
        sellBins[bLo] += vol * (1 - buyFrac)
      } else {
        for (let bi = bLo; bi <= bHi; bi++) {
          const binLo = minLow + bi * step
          const binHi = binLo + step
          const overlap = Math.max(0, Math.min(b.high, binHi) - Math.max(b.low, binLo))
          const share = overlap / rng
          buyBins[bi] += vol * share * buyFrac
          sellBins[bi] += vol * share * (1 - buyFrac)
        }
      }
    }

    let pocIdx = 0
    let maxBinVol = 0
    let totalVol = 0
    for (let i = 0; i < numBins; i++) {
      const tot = buyBins[i] + sellBins[i]
      totalVol += tot
      if (tot > maxBinVol) {
        maxBinVol = tot
        pocIdx = i
      }
    }
    if (maxBinVol <= 0) return []

    // CME Dual-Row 70% Value Area expansion around POC
    const targetVa = totalVol * 0.7
    let vaVol = buyBins[pocIdx] + sellBins[pocIdx]
    let vaLo = pocIdx
    let vaHi = pocIdx
    while (vaVol < targetVa && (vaLo > 0 || vaHi < numBins - 1)) {
      const upVol = vaHi + 1 < numBins ? buyBins[vaHi + 1] + sellBins[vaHi + 1] : -1
      const dnVol = vaLo > 0 ? buyBins[vaLo - 1] + sellBins[vaLo - 1] : -1
      if (upVol >= dnVol && upVol >= 0) {
        vaHi += 1
        vaVol += upVol
      } else if (dnVol >= 0) {
        vaLo -= 1
        vaVol += dnVol
      } else {
        break
      }
    }

    const pocPrice = minLow + (pocIdx + 0.5) * step
    const vahPrice = minLow + (vaHi + 1) * step
    const valPrice = minLow + vaLo * step

    const xLeft = Math.min(coordinates[0].x, coordinates[1].x)
    const xRight = Math.max(coordinates[0].x, coordinates[1].x)
    const boxW = Math.max(24, xRight - xLeft)
    const maxBarW = Math.max(16, Math.min(boxW * 0.65, 220))

    const keyPx = chart.convertToPixel(
      [{ value: maxHigh }, { value: minLow }, { value: pocPrice }, { value: vahPrice }, { value: valPrice }],
      { paneId: 'candle_pane' },
    )
    const arr = Array.isArray(keyPx) ? keyPx : [keyPx]
    const yTop = arr[0]?.y ?? 0
    const yBot = arr[1]?.y ?? 100
    const yPoc = arr[2]?.y ?? (yTop + yBot) / 2
    const yVah = arr[3]?.y ?? yTop
    const yVal = arr[4]?.y ?? yBot

    const figures: OverlayFigure[] = [
      // Outer fixed-range selection frame
      {
        type: 'rect',
        attrs: { x: xLeft, y: Math.min(yTop, yBot), width: boxW, height: Math.max(4, Math.abs(yBot - yTop)) },
        styles: {
          style: 'stroke_fill',
          color: 'rgba(41, 98, 255, 0.05)',
          borderColor: 'rgba(41, 98, 255, 0.45)',
          borderSize: 1,
          borderStyle: 'dashed',
          borderDashedValue: [4, 3],
        },
      },
      // 70% Value Area highlight band
      {
        type: 'rect',
        ignoreEvent: true,
        attrs: {
          x: xLeft,
          y: Math.min(yVah, yVal),
          width: boxW,
          height: Math.max(2, Math.abs(yVal - yVah)),
        },
        styles: {
          style: 'fill',
          color: 'rgba(41, 98, 255, 0.08)',
        },
      },
    ]

    // Buy / Sell split horizontal volume histogram bins anchored on left of range
    for (let bi = 0; bi < numBins; bi++) {
      const tot = buyBins[bi] + sellBins[bi]
      if (tot <= 0) continue
      const pLo = minLow + bi * step
      const pHi = pLo + step
      const px = chart.convertToPixel([{ value: pHi }, { value: pLo }], { paneId: 'candle_pane' })
      const pArr = Array.isArray(px) ? px : [px]
      const y1 = pArr[0]?.y
      const y2 = pArr[1]?.y
      if (y1 === undefined || y2 === undefined) continue
      const h = Math.max(1.5, Math.abs(y2 - y1) - 1)
      const yRow = Math.min(y1, y2) + 0.5
      const inVa = bi >= vaLo && bi <= vaHi
      const totalW = (tot / maxBinVol) * maxBarW
      const buyW = totalW * (buyBins[bi] / tot)
      const sellW = totalW - buyW

      if (buyW > 0.5) {
        figures.push({
          type: 'rect',
          ignoreEvent: true,
          attrs: { x: xLeft + 1, y: yRow, width: buyW, height: h },
          styles: {
            style: 'fill',
            color: inVa ? 'rgba(8, 153, 129, 0.55)' : 'rgba(8, 153, 129, 0.24)',
          },
        })
      }
      if (sellW > 0.5) {
        figures.push({
          type: 'rect',
          ignoreEvent: true,
          attrs: { x: xLeft + 1 + buyW, y: yRow, width: sellW, height: h },
          styles: {
            style: 'fill',
            color: inVa ? 'rgba(242, 54, 69, 0.52)' : 'rgba(242, 54, 69, 0.22)',
          },
        })
      }
    }

    // POC, VAH, VAL lines
    figures.push(
      {
        type: 'line',
        ignoreEvent: true,
        attrs: { coordinates: [{ x: xLeft, y: yPoc }, { x: xRight, y: yPoc }] },
        styles: { style: 'solid', color: '#FBC02D', size: 1.8 },
      },
      {
        type: 'line',
        ignoreEvent: true,
        attrs: { coordinates: [{ x: xLeft, y: yVah }, { x: xRight, y: yVah }] },
        styles: { style: 'dashed', color: '#2962FF', size: 1, dashedValue: [4, 3] },
      },
      {
        type: 'line',
        ignoreEvent: true,
        attrs: { coordinates: [{ x: xLeft, y: yVal }, { x: xRight, y: yVal }] },
        styles: { style: 'dashed', color: '#2962FF', size: 1, dashedValue: [4, 3] },
      },
      {
        type: 'text',
        ignoreEvent: true,
        attrs: {
          x: xRight - 4,
          y: yPoc - 4,
          text: `FRVP POC ${formatChartPrice(chart, pocPrice)} · VA [${formatChartPrice(chart, valPrice)}–${formatChartPrice(chart, vahPrice)}]`,
          align: 'right',
          baseline: 'bottom',
        },
        styles: {
          style: 'fill',
          color: '#131722',
          backgroundColor: '#FBC02D',
          borderRadius: 2,
          paddingLeft: 5,
          paddingRight: 5,
          paddingTop: 1.5,
          paddingBottom: 1.5,
          size: 9.5,
          weight: '700',
        },
      },
    )

    return figures
  },
}

export const VOLUME_STRUCTURE_OVERLAYS: OverlayTemplate[] = [
  anchoredVwapOverlay,
  fixedRangeVolumeProfileOverlay,
]
