import { beforeEach, describe, expect, it } from 'vitest'
import type { Indicator, KLineData } from 'klinecharts'
import {
  clearQmvRealSeriesCache,
  qmvActiveMoneyIndicator,
  qmvForeignFlowIndicator,
  qmvStockBbIndicator,
  setQmvRealSeriesCache,
  type QmvActiveMoneyChartPt,
  type QmvForeignFlowChartPt,
  type QmvStockBbChartPt,
} from '../chart/indicators/qmvRealIndicators'
import { INDICATOR_CATALOG } from '../chart/indicators/catalog'
import { useWorkspaceStore } from '../store/workspaceStore'

function makeDailyBars(count: number, startDate = '2026-05-01'): KLineData[] {
  const baseMs = Date.parse(`${startDate}T00:00:00Z`)
  return Array.from({ length: count }, (_, i) => {
    const p = 120_000 + i * 500
    return {
      timestamp: baseMs + i * 86_400_000,
      open: p - 300,
      high: p + 700,
      low: p - 700,
      close: p,
      volume: 1_500_000 + i * 10_000,
    }
  })
}

describe('Sumi v3.0 Wave 2 — Yumeko Read-Only Bridge, QMV Real Indicators & Deep-Link', () => {
  beforeEach(() => {
    clearQmvRealSeriesCache()
  })

  it('registers all 3 authentic Yumeko QMV indicators cleanly in the yumeko tier', () => {
    const yumeko = INDICATOR_CATALOG.filter((item) => item.tier === 'yumeko')
    expect(yumeko.map((x) => x.name)).toEqual(['QMV_STOCK_BB', 'QMV_ACTIVE_MONEY', 'QMV_FOREIGN_FLOW'])
    expect(yumeko.every((x) => !x.isOhlcvProxy && x.isYumekoReal === true)).toBe(true)
  })

  it('QMV_STOCK_BB transparently falls back to OHLCV Proxy (isReal=0) on Clean-Slate machines and switches to qmv_real (isReal=1) when Yumeko cache is hydrated', () => {
    const bars = makeDailyBars(25)
    const mockBbInd = { extendData: { ticker: 'HOSE:FPT' } } as unknown as Indicator<QmvStockBbChartPt>
    const mockAmInd = { extendData: { ticker: 'HOSE:FPT' } } as unknown as Indicator<QmvActiveMoneyChartPt>
    const mockFfInd = { extendData: { ticker: 'HOSE:FPT' } } as unknown as Indicator<QmvForeignFlowChartPt>

    // Clean-Slate (no Yumeko rows cached) -> transparent OHLCV proxy
    const proxyRes = qmvStockBbIndicator.calc(bars, mockBbInd) as Array<{ tmc?: number; isReal?: number }>
    expect(proxyRes).toHaveLength(25)
    expect(proxyRes[24].isReal).toBe(0)
    expect(typeof proxyRes[24].tmc).toBe('number')

    // Hydrate authentic Yumeko QMV Blackbox series
    setQmvRealSeriesCache('HOSE:FPT', {
      symbol: 'FPT',
      asOf: '2026-05-25',
      sourceProvenance: 'qmv_real',
      isProviderProprietary: true,
      stockBb: [
        {
          date: '2026-05-25',
          timestampMs: bars[24].timestamp,
          closeVnd: 132_000,
          tmc: 78.4,
          tma20: 66.2,
          tma50: 54.1,
          dspi: 62.9,
          dspiMa5: 60.0,
          dspiMa20: 55.0,
          mpicpAll: 71.0,
          mpicpZ: 1.4,
          dm: 68.0,
          ds: 32.0,
          stopLossVnd: 124_000,
          tp1Vnd: 145_000,
          tp2Vnd: 156_000,
        },
      ],
      activeMoney: [
        {
          date: '2026-05-25',
          timestampMs: bars[24].timestamp,
          priceVnd: 132_000,
          shark: 64.5,
          wolf: 22.1,
          sheep: 13.4,
          trendShark: 1,
          trendWolf: 1,
          trendSheep: -1,
          volume: 2_500_000,
        },
      ],
      foreignFlow: [
        {
          date: '2026-05-25',
          timestampMs: bars[24].timestamp,
          priceVnd: 132_000,
          buyValue: 185.2,
          sellValue: 92.0,
          netValue: 93.2,
          buyVolume: 1_400_000,
          sellVolume: 700_000,
          netVolume: 700_000,
        },
      ],
    })

    const realRes = qmvStockBbIndicator.calc(bars, mockBbInd) as Array<{
      tmc?: number
      tma20?: number
      dspi?: number
      isReal?: number
    }>
    expect(realRes[24].isReal).toBe(1)
    expect(realRes[24].tmc).toBe(78.4)
    expect(realRes[24].tma20).toBe(66.2)
    expect(realRes[24].dspi).toBe(62.9)

    const activeRes = qmvActiveMoneyIndicator.calc(bars, mockAmInd) as Array<{
      shark?: number
      wolf?: number
      sheep?: number
      isReal?: number
    }>
    expect(activeRes[24].isReal).toBe(1)
    expect(activeRes[24].shark).toBe(64.5)
    expect(activeRes[24].wolf).toBe(22.1)
    expect(activeRes[24].sheep).toBe(13.4)

    const foreignRes = qmvForeignFlowIndicator.calc(bars, mockFfInd) as Array<{
      netValue?: number
      isReal?: number
    }>
    expect(foreignRes[24].isReal).toBe(1)
    expect(foreignRes[24].netValue).toBe(93.2)
  })

  it('applyDeepLink switches ticker, interval, PIT as_of lock, and activates Yumeko panel', () => {
    const store = useWorkspaceStore.getState()
    store.applyDeepLink({
      rawUrl: 'sumi://inspect?symbol=MWG&as_of=2026-11-26&tf=1W&tab=yumeko',
      action: 'inspect',
      ticker: 'HOSE:MWG',
      symbol: 'MWG',
      asOf: '2026-11-26',
      interval: '1W',
      tab: 'yumeko',
    })
    const next = useWorkspaceStore.getState()
    const activePane = next.panes.find((p) => p.slotId === next.activeSlotId) ?? next.panes[0]
    expect(activePane.ticker).toBe('HOSE:MWG')
    expect(activePane.interval).toBe('1W')
    expect(next.yumekoAsOf).toBe('2026-11-26')
    expect(next.rightTab).toBe('yumeko')
    expect(next.rightPanelOpen).toBe(true)
  })
})
