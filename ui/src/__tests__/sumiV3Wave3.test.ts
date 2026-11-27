import { describe, expect, it } from 'vitest'
import type { IndicatorTemplate, KLineData } from 'klinecharts'
import { INDICATOR_CATALOG, findIndicatorMeta } from '../chart/indicators/catalog'
import {
  WAVE3_INDICATORS,
  candlePatternsIndicator,
  cvdIndicator,
  keltnerIndicator,
  mfiIndicator,
  psarIndicator,
  stochIndicator,
  superTrendIndicator,
} from '../chart/indicators/wave3Indicators'
import { DRAWING_TOOL_GROUPS } from '../chart/drawingIcons'
import { VOLUME_STRUCTURE_OVERLAYS } from '../chart/overlays/volumeStructureOverlays'
import { useWorkspaceStore } from '../store/workspaceStore'

function makeSampleCandles(count = 60): KLineData[] {
  const out: KLineData[] = []
  let price = 100_000
  const baseTs = Date.UTC(2025, 0, 2)
  for (let i = 0; i < count; i++) {
    const wave = Math.sin(i / 5) * 2_200 + i * 240
    const open = Math.round(price)
    const close = Math.round(100_000 + wave)
    const high = Math.max(open, close) + 900
    const low = Math.min(open, close) - 800
    out.push({
      timestamp: baseTs + i * 86_400_000,
      open,
      high,
      low,
      close,
      volume: 1_000_000 + (i % 7) * 250_000,
    })
    price = close
  }
  return out
}

function runCalc<T>(tpl: IndicatorTemplate<T>, bars: KLineData[], calcParams: number[]): T[] {
  return tpl.calc(bars, { calcParams } as Parameters<typeof tpl.calc>[1]) as T[]
}

describe('Sumi v3.0 Wave 3 — Volume Structure, Quantitative Indicators & Store Contracts', () => {
  it('registers all 8 Wave 3 indicators and exposes honest OHLCV Proxy badge on CVD', () => {
    expect(WAVE3_INDICATORS.length).toBe(8)
    const names = WAVE3_INDICATORS.map((x) => x.name)
    expect(names).toEqual(
      expect.arrayContaining([
        'SUPERTREND',
        'PSAR',
        'KELTNER',
        'STOCH',
        'MFI',
        'CVD',
        'TPO_PROFILE',
        'CANDLE_PATTERNS',
      ]),
    )
    const cvdMeta = findIndicatorMeta('CVD')
    expect(cvdMeta?.isOhlcvProxy).toBe(true)
    expect(INDICATOR_CATALOG.filter((i) => i.tier === 'aux').length).toBe(13)
  })

  it('exposes Anchored VWAP and Fixed Range Volume Profile overlays in drawing rail', () => {
    const ovNames = VOLUME_STRUCTURE_OVERLAYS.map((o) => o.name)
    expect(ovNames).toContain('anchoredVwap')
    expect(ovNames).toContain('fixedRangeVolumeProfile')
    const allToolKeys = DRAWING_TOOL_GROUPS.flatMap((g) => g.items.map((it) => it.key))
    expect(allToolKeys).toContain('anchoredVwap')
    expect(allToolKeys).toContain('fixedRangeVolumeProfile')
  })

  it('computes causal values for SuperTrend, PSAR, Keltner, Stoch, MFI, CVD and Candlestick Patterns', () => {
    const bars = makeSampleCandles(60)
    const stRes = runCalc(superTrendIndicator, bars, [10, 3])
    expect(stRes.length).toBe(bars.length)
    expect(stRes[stRes.length - 1].up !== undefined || stRes[stRes.length - 1].down !== undefined).toBe(true)

    const psarRes = runCalc(psarIndicator, bars, [0.02, 0.2])
    expect(psarRes[psarRes.length - 1].sar).toBeTypeOf('number')

    const kcRes = runCalc(keltnerIndicator, bars, [20, 10, 2])
    const kcLast = kcRes[kcRes.length - 1]
    expect(kcLast.upper!).toBeGreaterThan(kcLast.mid!)
    expect(kcLast.mid!).toBeGreaterThan(kcLast.lower!)

    const stochRes = runCalc(stochIndicator, bars, [14, 3, 3])
    const sLast = stochRes[stochRes.length - 1]
    expect(sLast.k!).toBeGreaterThanOrEqual(0)
    expect(sLast.k!).toBeLessThanOrEqual(100)

    const mfiRes = runCalc(mfiIndicator, bars, [14])
    const mLast = mfiRes[mfiRes.length - 1]
    expect(mLast.mfi!).toBeGreaterThanOrEqual(0)
    expect(mLast.mfi!).toBeLessThanOrEqual(100)

    const cvdRes = runCalc(cvdIndicator, bars, [1])
    expect(Number.isFinite(cvdRes[cvdRes.length - 1].cvd)).toBe(true)

    const patRes = runCalc(candlePatternsIndicator, bars, [14])
    expect(patRes.length).toBe(bars.length)
  })

  it('wires Wave 3 Trade Planner, Debrief Modal and Market Breadth dock tab in workspaceStore', () => {
    const st = useWorkspaceStore.getState()
    st.openTradePlanner({ ticker: 'HOSE:FPT', side: 'LONG', entryPrice: 130_000, stopLoss: 124_000, targetPrice: 145_000 })
    expect(useWorkspaceStore.getState().tradePlannerOpen).toBe(true)
    expect(useWorkspaceStore.getState().tradePlannerDraft?.entryPrice).toBe(130_000)
    st.closeTradePlanner()
    expect(useWorkspaceStore.getState().tradePlannerOpen).toBe(false)

    st.setDebriefModalOpen(true)
    expect(useWorkspaceStore.getState().debriefModalOpen).toBe(true)
    st.setDebriefModalOpen(false)

    st.setBottomDockTab('breadth')
    expect(useWorkspaceStore.getState().bottomDockTab).toBe('breadth')
  })
})
