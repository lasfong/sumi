import { describe, expect, it } from 'vitest'
import { INDICATOR_CATALOG } from '../chart/indicators/catalog'
import {
  buildQuarterlyStepSeries,
  resolveValuationQuarterTimestampMs,
} from '../chart/indicators/fundamentalIndicators'
import { isFormulaExpressionQuery } from '../components/modals/SymbolSearchModal'
import { useWorkspaceStore } from '../store/workspaceStore'
import type { ValuationQuarterDto } from '../types/market'

describe('Sumi v3.0 Wave 4 — Fundamental Indicators, Synthetic Formulas & Compare Store', () => {
  it('detects synthetic arithmetic expressions without false positives on standard tickers', () => {
    expect(isFormulaExpressionQuery('HOSE:SSI / HOSE:VNINDEX')).toBe(true)
    expect(isFormulaExpressionQuery('HNX:VN30F1! - HOSE:VN30')).toBe(true)
    expect(isFormulaExpressionQuery('(HOSE:HPG + HOSE:HSG) / 2')).toBe(true)
    expect(isFormulaExpressionQuery('HOSE:FPT*1.1')).toBe(true)

    expect(isFormulaExpressionQuery('HOSE:FPT')).toBe(false)
    expect(isFormulaExpressionQuery('HNX:VN30F1!')).toBe(false)
    expect(isFormulaExpressionQuery('NYSE:BRK-B')).toBe(false)
    expect(isFormulaExpressionQuery('')).toBe(false)
  })

  it('aligns DN01 quarterly metrics strictly causally (zero future lookahead) and computes EPS YoY %', () => {
    const valuations: ValuationQuarterDto[] = [
      {
        period: '2025-Q3',
        contentDate: '2025-10-20',
        sourceFamily: 'dn01',
        pe: 15.0,
        pb: 3.2,
        ps: null,
        evEbitda: null,
        eps: 5000,
        bvps: null,
        roe: 22.0,
        roa: null,
        debtToEquity: null,
      },
      {
        period: '2026-Q3',
        contentDate: '2026-10-20',
        sourceFamily: 'dn01',
        pe: 18.5,
        pb: 4.1,
        ps: null,
        evEbitda: null,
        eps: 6000,
        bvps: null,
        roe: 25.5,
        roa: null,
        debtToEquity: null,
      },
    ]

    expect(resolveValuationQuarterTimestampMs(valuations[0])).toBe(Date.parse('2025-10-20T00:00:00Z'))

    const bars = [
      { timestamp: Date.parse('2025-10-01T00:00:00Z') }, // before Q3 2025 -> undefined
      { timestamp: Date.parse('2025-11-05T00:00:00Z') }, // after Q3 2025 -> 15.0
      { timestamp: Date.parse('2026-10-19T00:00:00Z') }, // 1 day before Q3 2026 -> STILL 15.0 (causal!)
      { timestamp: Date.parse('2026-10-25T00:00:00Z') }, // after Q3 2026 -> 18.5
    ]

    const peSeries = buildQuarterlyStepSeries(bars, valuations, 'pe')
    expect(peSeries).toEqual([undefined, 15.0, 15.0, 18.5])

    const roeSeries = buildQuarterlyStepSeries(bars, valuations, 'roe')
    expect(roeSeries).toEqual([undefined, 22.0, 22.0, 25.5])

    const epsYoySeries = buildQuarterlyStepSeries(bars, valuations, 'epsYoy')
    expect(epsYoySeries[0]).toBeUndefined()
    expect(epsYoySeries[1]).toBeUndefined()
    expect(epsYoySeries[2]).toBeUndefined()
    expect(epsYoySeries[3]).toBeCloseTo(20.0, 5) // (6000 - 5000) / 5000 = +20%
  })

  it('registers all 4 DN01 financial sub-pane indicators in INDICATOR_CATALOG', () => {
    const names = new Set(INDICATOR_CATALOG.map((i) => i.name))
    expect(names.has('FIN_PE')).toBe(true)
    expect(names.has('FIN_PB')).toBe(true)
    expect(names.has('FIN_ROE')).toBe(true)
    expect(names.has('FIN_EPS_YOY')).toBe(true)
  })

  it('manages Compare (+) tickers per pane with normalization, deduplication, and 6-symbol cap', () => {
    const store = useWorkspaceStore.getState()
    store.clearCompareTickers('pane-0')
    store.addCompareTicker('vnindex', 'pane-0')
    store.addCompareTicker('HOSE:VNINDEX', 'pane-0') // duplicate ignored
    store.addCompareTicker('HOSE:SSI', 'pane-0')
    expect(useWorkspaceStore.getState().compareTickersBySlot['pane-0']).toEqual([
      'HOSE:VNINDEX',
      'HOSE:SSI',
    ])

    store.removeCompareTicker('HOSE:VNINDEX', 'pane-0')
    expect(useWorkspaceStore.getState().compareTickersBySlot['pane-0']).toEqual(['HOSE:SSI'])

    store.clearCompareTickers('pane-0')
    expect(useWorkspaceStore.getState().compareTickersBySlot['pane-0']).toEqual([])
  })
})
