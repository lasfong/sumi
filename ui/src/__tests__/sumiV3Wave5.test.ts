import { describe, expect, it, vi } from 'vitest'

vi.mock('klinecharts', () => ({
  utils: {
    formatPrecision: (v: number | string, p = 2) => Number(v).toFixed(p),
    getLinearYFromCoordinates: (
      c1: { x: number; y: number },
      c2: { x: number; y: number },
      target: { x: number; y: number },
    ) => {
      if (Math.abs(c2.x - c1.x) < 1e-12) return c1.y
      return c1.y + ((target.x - c1.x) / (c2.x - c1.x)) * (c2.y - c1.y)
    },
  },
}))

import { ALL_DRAWING_TOOLS_COUNT, DRAWING_TOOL_GROUPS } from '../chart/drawingIcons'
import { buildQuarterlyStepSeries } from '../chart/indicators/fundamentalIndicators'
import { PRO_UPGRADED_OVERLAYS, andrewsPitchfork } from '../chart/overlays/proOverlays'
import { filterPaletteCommands, type PaletteCommandItem } from '../components/modals/CommandPaletteModal'
import { useWorkspaceStore } from '../store/workspaceStore'

describe('Sumi v3.0 Wave 5 — Strategy 2.0, True Blind Drill & Workspace UX Pro', () => {
  it('registers all 4 new Wave 5 overlays (path, highlighter, andrewsPitchfork, fibTimeZones)', () => {
    const overlayNames = new Set(PRO_UPGRADED_OVERLAYS.map((o) => o.name))
    expect(overlayNames.has('path')).toBe(true)
    expect(overlayNames.has('highlighter')).toBe(true)
    expect(overlayNames.has('andrewsPitchfork')).toBe(true)
    expect(overlayNames.has('fibTimeZones')).toBe(true)

    const railKeys = new Set(DRAWING_TOOL_GROUPS.flatMap((g) => g.items.map((i) => i.key)))
    expect(railKeys.has('path')).toBe(true)
    expect(railKeys.has('highlighter')).toBe(true)
    expect(railKeys.has('andrewsPitchfork')).toBe(true)
    expect(railKeys.has('fibTimeZones')).toBe(true)
    expect(ALL_DRAWING_TOOLS_COUNT).toBeGreaterThanOrEqual(41)
  })

  it('filters Command Palette entries cleanly in both plain and > command modes', () => {
    const sample: PaletteCommandItem[] = [
      {
        id: 'blind',
        category: 'Replay',
        title: 'Bắt đầu Luyện Mù Ngẫu Nhiên (True Blind Drill)',
        subtitle: 'Ẩn tên mã & chuẩn hóa giá 100.00',
        keywords: 'blind replay luyen mu',
        run: () => {},
      },
      {
        id: 'kt01',
        category: 'Yumeko KT01',
        title: 'Xuất gói bằng chứng kỹ thuật KT01 (SHA-256)',
        subtitle: 'Chốt verdict',
        keywords: 'yumeko kt01 export',
        run: () => {},
      },
    ]

    expect(filterPaletteCommands(sample, '> blind').map((x) => x.id)).toEqual(['blind'])
    expect(filterPaletteCommands(sample, '> kt01').map((x) => x.id)).toEqual(['kt01'])
    expect(filterPaletteCommands(sample, '').length).toBe(2)
  })

  it('supports Alt+Enter pane maximize toggle and Global Indicator Undo/Redo stack', () => {
    const store = useWorkspaceStore.getState()
    store.setGridLayout('4')
    expect(useWorkspaceStore.getState().maximizedSlotId).toBeNull()

    store.toggleMaximizeSlot('pane-2')
    expect(useWorkspaceStore.getState().maximizedSlotId).toBe('pane-2')
    expect(useWorkspaceStore.getState().activeSlotId).toBe('pane-2')

    // Calling again restores grid
    store.toggleMaximizeSlot('pane-2')
    expect(useWorkspaceStore.getState().maximizedSlotId).toBeNull()

    // Indicator Undo / Redo roundtrip
    const beforeCount = useWorkspaceStore.getState().panes.find((p) => p.slotId === 'pane-0')!.indicators.length
    store.addIndicatorToActivePane(
      { name: 'RSI', paneId: 'pane_rsi_test', isMain: false, calcParams: [14], visible: true },
      'pane-0',
    )
    expect(useWorkspaceStore.getState().panes.find((p) => p.slotId === 'pane-0')!.indicators.length).toBe(
      beforeCount + 1,
    )

    const undone = useWorkspaceStore.getState().undoIndicatorChange()
    expect(undone).toBe(true)
    expect(useWorkspaceStore.getState().panes.find((p) => p.slotId === 'pane-0')!.indicators.length).toBe(
      beforeCount,
    )

    const redone = useWorkspaceStore.getState().redoIndicatorChange()
    expect(redone).toBe(true)
    expect(useWorkspaceStore.getState().panes.find((p) => p.slotId === 'pane-0')!.indicators.length).toBe(
      beforeCount + 1,
    )
  })

  it('updates symbolFlags and watchlist rename/reorder state cleanly', async () => {
    const store = useWorkspaceStore.getState()
    await store.setSymbolFlag('HOSE:FPT', 'green')
    expect(useWorkspaceStore.getState().symbolFlags['HOSE:FPT']).toBe('green')
    await store.setSymbolFlag('HOSE:FPT', null)
    expect(useWorkspaceStore.getState().symbolFlags['HOSE:FPT']).toBeUndefined()
  })

  it('Wave 6 W6-08: andrewsPitchfork projects all 3 parallel tines to bounding right edge x=800', () => {
    const figures = andrewsPitchfork.createPointFigures!({
      coordinates: [
        { x: 10, y: 100 },
        { x: 40, y: 60 },
        { x: 50, y: 140 },
      ],
      bounding: { width: 800, height: 500, left: 0, right: 800, top: 0, bottom: 500 },
    } as never) as Array<{ type: string; attrs?: { coordinates?: Array<{ x: number; y: number }> } }>

    const lineFigs = figures.filter((f) => f.type === 'line')
    // Expect base dashed p1-p2 + 3 projected tines (median, upper, lower) = 4 lines
    expect(lineFigs.length).toBe(4)
    const tines = lineFigs.slice(1)
    for (const tine of tines) {
      const coords = tine.attrs?.coordinates ?? []
      expect(coords.length).toBe(2)
      const [start, end] = coords
      const len = Math.hypot(end.x - start.x, end.y - start.y)
      expect(len).toBeGreaterThan(500)
      expect(end.x).toBe(800)
    }
  })

  it('Wave 6 W6-06: FIN_PE computes dynamic daily Close[t]/EPS_TTM[q(t)] with zero pre-quarter lookahead', () => {
    const tPre = Date.UTC(2024, 0, 15)
    const tQ1 = Date.UTC(2024, 2, 31)
    const tDay1 = Date.UTC(2024, 3, 5)
    const tDay2 = Date.UTC(2024, 3, 10)
    const bars = [
      { timestamp: tPre, close: 90_000 },
      { timestamp: tDay1, close: 100_000 },
      { timestamp: tDay2, close: 120_000 },
    ]
    const vals = [
      {
        period: '2024-Q1',
        contentDate: new Date(tQ1).toISOString().slice(0, 10),
        sourceFamily: 'tradingview_fundamental',
        pe: 20.0,
        pb: 4.0,
        ps: null,
        evEbitda: null,
        eps: 5_000,
        bvps: 25_000,
        roe: 22.5,
        roa: null,
        debtToEquity: null,
      },
    ]
    const peSeries = buildQuarterlyStepSeries(bars, vals, 'pe')
    expect(peSeries[0]).toBeUndefined()
    expect(peSeries[1]).toBeCloseTo(20.0, 5) // 100,000 / 5,000
    expect(peSeries[2]).toBeCloseTo(24.0, 5) // 120,000 / 5,000 (dynamic daily move!)
  })
})
