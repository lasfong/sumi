import { describe, expect, it, vi } from 'vitest'
import type { KLineData } from 'klinecharts'
import {
  computeVnSessionBreaks,
  isDrawingVisibleOnInterval,
  isVietnamTicker,
  isVn30FuturesTicker,
  snapPriceToVenueTick,
} from '../utils/vnMarket'
import { toHeikinAshi } from '../chart/heikinAshi'
import { drawingSyncBus } from '../chart/drawingSyncBus'
import { formatDurationVietnamese, intervalToDurationMs } from '../chart/periods'
import { useWorkspaceStore } from '../store/workspaceStore'
import type { SavedDrawingRecord } from '../types/market'

describe('Vietnam Market Tick & Venue Rules (v2.2)', () => {
  it('detects Vietnam tickers and VN30 futures accurately', () => {
    expect(isVietnamTicker('HOSE:FPT')).toBe(true)
    expect(isVietnamTicker('hnx:shs')).toBe(true)
    expect(isVietnamTicker('UPCOM:ACV')).toBe(true)
    expect(isVietnamTicker('BINANCE:BTCUSDT')).toBe(false)
    expect(isVietnamTicker('NASDAQ:AAPL')).toBe(false)

    expect(isVn30FuturesTicker('HNX:VN30F1!')).toBe(true)
    expect(isVn30FuturesTicker('HNX:VN301!')).toBe(true)
    expect(isVn30FuturesTicker('HOSE:VN30')).toBe(false)
  })

  it('snaps dragged SL/TP/LO prices to statutory HOSE/HNX/UPCOM/VN30F1! tick sizes', () => {
    // HOSE < 10,000 => 10 VND
    expect(snapPriceToVenueTick('HOSE:HAG', 9_844)).toBe(9_840)
    expect(snapPriceToVenueTick('HOSE:HAG', 9_846)).toBe(9_850)
    // HOSE 10,000..49,950 => 50 VND
    expect(snapPriceToVenueTick('HOSE:HPG', 27_324)).toBe(27_300)
    expect(snapPriceToVenueTick('HOSE:HPG', 27_326)).toBe(27_350)
    // HOSE >= 50,000 => 100 VND
    expect(snapPriceToVenueTick('HOSE:FPT', 131_449)).toBe(131_400)
    expect(snapPriceToVenueTick('HOSE:FPT', 131_451)).toBe(131_500)
    // HNX & UPCOM => 100 VND at all price tiers
    expect(snapPriceToVenueTick('HNX:SHS', 14_360)).toBe(14_400)
    expect(snapPriceToVenueTick('UPCOM:ACV', 98_240)).toBe(98_200)
    // VN30F1! => 0.1 index point
    expect(snapPriceToVenueTick('HNX:VN30F1!', 1324.66)).toBe(1324.7)
    expect(snapPriceToVenueTick('HNX:VN30F1!', 1324.64)).toBe(1324.6)
    // Global stocks / crypto / FX
    expect(snapPriceToVenueTick('BINANCE:BTCUSDT', 91234.567)).toBe(91234.57)
    expect(snapPriceToVenueTick('FX_IDC:EURUSD', 1.0846)).toBe(1.085)
  })
})

describe('Object Tree Visibility by Timeframe (v2.2)', () => {
  it('evaluates all, intraday, swing, and explicit interval arrays cleanly', () => {
    expect(isDrawingVisibleOnInterval(null, '1D')).toBe(true)
    expect(isDrawingVisibleOnInterval({}, '15m')).toBe(true)
    expect(isDrawingVisibleOnInterval({ visibleTimeframes: 'all' }, '1W')).toBe(true)

    // Intraday <= 4h
    expect(isDrawingVisibleOnInterval({ visibleTimeframes: 'intraday' }, '15m')).toBe(true)
    expect(isDrawingVisibleOnInterval({ visibleTimeframes: 'intraday' }, '4h')).toBe(true)
    expect(isDrawingVisibleOnInterval({ visibleTimeframes: 'intraday' }, '1D')).toBe(false)

    // Swing >= 1D
    expect(isDrawingVisibleOnInterval({ visibleTimeframes: 'swing' }, '1D')).toBe(true)
    expect(isDrawingVisibleOnInterval({ visibleTimeframes: 'swing' }, '1W')).toBe(true)
    expect(isDrawingVisibleOnInterval({ visibleTimeframes: 'swing' }, '1h')).toBe(false)

    // Specific intervals
    expect(isDrawingVisibleOnInterval({ visibleTimeframes: ['1h', '4h'] }, '1h')).toBe(true)
    expect(isDrawingVisibleOnInterval({ visibleTimeframes: ['1h', '4h'] }, '15m')).toBe(false)
    expect(isDrawingVisibleOnInterval({ visibleTimeframes: [] }, '1D')).toBe(true)
  })
})

describe('VN Intraday Session Breaks (ICT UTC+7)', () => {
  const ict = (year: number, month: number, day: number, hour: number, minute: number) =>
    Date.UTC(year, month - 1, day, hour - 7, minute, 0, 0)

  it('marks 09:00 morning open, 13:00 afternoon open, and 14:30 ATC on 15m charts', () => {
    const bars = [
      { timestamp: ict(2026, 4, 20, 9, 15) },
      { timestamp: ict(2026, 4, 20, 11, 15) },
      { timestamp: ict(2026, 4, 20, 13, 0) },
      { timestamp: ict(2026, 4, 20, 14, 15) },
      { timestamp: ict(2026, 4, 20, 14, 30) },
      { timestamp: ict(2026, 4, 21, 9, 0) },
      { timestamp: ict(2026, 4, 21, 13, 0) },
    ]
    const markers = computeVnSessionBreaks(bars, '15m')
    expect(markers.map((m) => m.kind)).toEqual(['afternoon', 'atc', 'morning', 'afternoon'])
    expect(markers[0].label).toBe('13:00 Chiều')
    expect(markers[1].label).toBe('14:30 ATC')
    expect(markers[2].label).toBe('21/04 09:00')
  })

  it('suppresses session breaks on daily/weekly/4h timeframes and ATC on 1h', () => {
    const bars = [
      { timestamp: ict(2026, 4, 20, 9, 0) },
      { timestamp: ict(2026, 4, 20, 13, 0) },
      { timestamp: ict(2026, 4, 20, 14, 30) },
    ]
    expect(computeVnSessionBreaks(bars, '1D')).toEqual([])
    expect(computeVnSessionBreaks(bars, '4h')).toEqual([])
    const hourly = computeVnSessionBreaks(bars, '1h')
    expect(hourly.map((m) => m.kind)).toEqual(['afternoon'])
  })
})

describe('Causal Heikin-Ashi Transformation (Zero Drift Invariant)', () => {
  it('computes exact HA candles and avoids intra-bar HA_Open drift over 50 ticks', () => {
    const raw: KLineData[] = [
      { timestamp: 1_000, open: 100, high: 110, low: 95, close: 105, volume: 10 },
      { timestamp: 2_000, open: 105, high: 120, low: 102, close: 115, volume: 20 },
    ]
    const ha = toHeikinAshi(raw)
    expect(ha[0].open).toBeCloseTo(102.5, 6)
    expect(ha[0].close).toBeCloseTo(102.5, 6)
    expect(ha[1].open).toBeCloseTo(102.5, 6)
    expect(ha[1].close).toBeCloseTo((105 + 120 + 102 + 115) / 4, 6)

    // Simulate 50 intra-bar live ticks updating bar[1] using fixed prevHa = ha[0]
    const expectedOpen = (ha[0].open + ha[0].close) / 2
    for (let tick = 0; tick < 50; tick++) {
      const ticked: KLineData = {
        timestamp: 2_000,
        open: 105,
        high: 120 + tick * 0.1,
        low: 102,
        close: 115 + (tick % 5) - 2,
        volume: 20 + tick,
      }
      const [haTick] = toHeikinAshi([ticked], { open: ha[0].open, close: ha[0].close })
      expect(haTick.open).toBeCloseTo(expectedOpen, 8)
    }
  })
})

describe('Multi-Pane Drawing Sync Bus & Period Helpers', () => {
  it('broadcasts drawing updates to sibling slots without echoing to source slot', () => {
    useWorkspaceStore.setState((s) => ({ sync: { ...s.sync, drawings: true } }))
    const slot1 = vi.fn()
    const slot2 = vi.fn()
    const unsub1 = drawingSyncBus.subscribe('slot-1', slot1)
    const unsub2 = drawingSyncBus.subscribe('slot-2', slot2)

    const sampleRecord: SavedDrawingRecord = {
      id: 'drw-1',
      ticker: 'HOSE:FPT',
      name: 'segment',
      paneId: 'candle_pane',
      groupId: 'user_drawings',
      points: [
        { timestamp: 1000, value: 130_000 },
        { timestamp: 2000, value: 135_000 },
      ],
      styles: null,
      extendData: null,
      lock: false,
      visible: true,
      mode: 'normal',
      updatedAtMs: Date.now(),
    }

    drawingSyncBus.emit({ kind: 'upsert', sourceSlotId: 'slot-1', ticker: 'HOSE:FPT', record: sampleRecord }, false)
    expect(slot1).not.toHaveBeenCalled()
    expect(slot2).toHaveBeenCalledTimes(1)

    unsub1()
    unsub2()
  })

  it('formats Vietnamese durations and bar intervals accurately', () => {
    expect(intervalToDurationMs('15m')).toBe(900_000)
    expect(intervalToDurationMs('4h')).toBe(14_400_000)
    expect(formatDurationVietnamese(45 * 60_000)).toBe('45 phút')
    expect(formatDurationVietnamese(150 * 60_000)).toBe('2g 30p')
    expect(formatDurationVietnamese(5 * 86_400_000)).toBe('5 ngày')
  })
})

describe('QMV Streamlined Indicator Catalog & Custom Algorithms', async () => {
  const { INDICATOR_CATALOG, findIndicatorMeta } = await import('../chart/indicators/catalog')
  const { afcIndicator, ichimokuIndicator, qmvFlowIndicator } = await import('../chart/indicators/customIndicators')
  const { webFallbackApi } = await import('../api/webFallback')

  it('retains 11 QMV core indicators (including AFC) and 5 auxiliary quantitative indicators without Tongdaxin clutter', () => {
    const core = INDICATOR_CATALOG.filter((i) => i.tier === 'core')
    const aux = INDICATOR_CATALOG.filter((i) => i.tier === 'aux')
    expect(core).toHaveLength(11)
    expect(aux.length).toBeGreaterThanOrEqual(13)
    expect(findIndicatorMeta('AFC')?.defaultParams).toEqual([10, 100, 1])
    expect(findIndicatorMeta('AFC')?.category).toBe('QMV Cốt lõi')
    expect(findIndicatorMeta('AFC')?.isMain).toBe(true)
    expect(findIndicatorMeta('ICHIMOKU')?.defaultParams).toEqual([6, 18, 36, 18])
    expect(findIndicatorMeta('MA')?.defaultParams).toEqual([20, 50, 100, 200])
    expect(findIndicatorMeta('CCI')?.defaultParams).toEqual([14])
    expect(findIndicatorMeta('RSI')?.defaultParams).toEqual([14])
    expect(findIndicatorMeta('QMV_FLOW')?.isOhlcvProxy).toBe(true)
    expect(findIndicatorMeta('SUPERTREND')?.tier).toBe('aux')
    expect(findIndicatorMeta('STOCHRSI')).toBeUndefined()
    expect(findIndicatorMeta('BIAS')).toBeUndefined()
  })

  it('computes AFC Analog Forecast Cone on 1D with venue bounds, PIT replay lock, and clean non-1D status', () => {
    expect(afcIndicator.name).toBe('AFC')
    expect(afcIndicator.calcParams).toEqual([10, 100, 1])

    const non1D = webFallbackApi.computeAnalogCone({ ticker: 'HOSE:FPT', interval: '1h' })
    expect(non1D.supportedInterval).toBe(false)
    expect(non1D.forecast).toBeNull()
    expect(non1D.statusMessage).toBe('AFC hiệu lực trên khung 1D')

    const d1 = webFallbackApi.computeAnalogCone({
      ticker: 'HOSE:FPT',
      interval: '1D',
      horizon: 10,
      topK: 80,
      useMarketContext: true,
    })
    expect(d1.supportedInterval).toBe(true)
    expect(d1.forecast).not.toBeNull()
    expect(d1.forecast!.steps).toHaveLength(10)
    expect(d1.forecast!.topAnalogs.length).toBeGreaterThanOrEqual(5)
    for (const s of d1.forecast!.steps) {
      expect(s.p10).toBeLessThanOrEqual(s.p25)
      expect(s.p25).toBeLessThanOrEqual(s.p50)
      expect(s.p50).toBeLessThanOrEqual(s.p75)
      expect(s.p75).toBeLessThanOrEqual(s.p90)
      expect(s.p10).toBeGreaterThanOrEqual(s.cumulativeFloor!)
      expect(s.p90).toBeLessThanOrEqual(s.cumulativeCeiling!)
    }

    // Verify Bar Replay strict PIT cursor lock in webFallback
    const snap = webFallbackApi.replayStart('HOSE:FPT', '1D')
    const replayCone = webFallbackApi.computeAnalogCone({
      ticker: 'HOSE:FPT',
      interval: '1D',
      asOfTimestampMs: Number.MAX_SAFE_INTEGER,
    })
    expect(replayCone.replayLocked).toBe(true)
    expect(replayCone.asOfTimestampMs).toBe(snap.cutoffTimestampMs)
    webFallbackApi.replayStop()
  })

  it('computes causal QMV Ichimoku V7 (6,18,36,18) and QMV_FLOW Z-50 bounded in [0, 100]', () => {
    const bars: KLineData[] = Array.from({ length: 80 }, (_, i) => ({
      timestamp: 1_700_000_000_000 + i * 86_400_000,
      open: 50_000 + i * 250,
      high: 50_800 + i * 260,
      low: 49_500 + i * 240,
      close: 50_500 + i * 255 + (i % 5) * 120,
      volume: 800_000 + (i % 7) * 90_000,
    }))
    const ichiOut = ichimokuIndicator.calc(
      bars,
      { calcParams: [6, 18, 36, 18] } as Parameters<typeof ichimokuIndicator.calc>[1],
    ) as Array<{ tenkan?: number; kijun?: number; senkouA?: number; senkouB?: number; rawFutureA?: number }>
    expect(ichiOut).toHaveLength(80)
    expect(ichiOut[79].tenkan).toBeDefined()
    expect(ichiOut[79].kijun).toBeDefined()
    expect(ichiOut[79].senkouA).toBeDefined()
    expect(ichiOut[79].rawFutureA).toBeDefined()

    const flowOut = qmvFlowIndicator.calc(
      bars,
      { calcParams: [3, 5, 10, 20] } as Parameters<typeof qmvFlowIndicator.calc>[1],
    ) as Array<{ f1?: number; f2?: number; f3?: number; f4?: number }>
    expect(flowOut).toHaveLength(80)
    for (const pt of flowOut.slice(25)) {
      expect(pt.f2).toBeGreaterThanOrEqual(0)
      expect(pt.f2).toBeLessThanOrEqual(100)
      expect(pt.f4).toBeGreaterThanOrEqual(0)
      expect(pt.f4).toBeLessThanOrEqual(100)
    }
  })
})
