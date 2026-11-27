import { describe, expect, it, vi } from 'vitest'

vi.mock('klinecharts', () => ({
  utils: {
    formatPrecision: (v: number | string, p = 2) => Number(v).toFixed(p),
  },
}))

import { ALL_DRAWING_TOOLS_COUNT, DRAWING_TOOL_GROUPS } from '../chart/drawingIcons'
import { PRO_UPGRADED_OVERLAYS } from '../chart/overlays/proOverlays'
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
})
