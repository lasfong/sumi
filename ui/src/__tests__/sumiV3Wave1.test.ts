import { describe, expect, it } from 'vitest'
import {
  computeVnSessionBreaks,
  INTRADAY_INTERVALS,
  isDrawingVisibleOnInterval,
} from '../utils/vnMarket'
import { altKeyLabel, formatShortcut, modKeyLabel } from '../utils/platform'
import { useWorkspaceStore } from '../store/workspaceStore'

describe('Sumi v3.0 Wave 1 Regression Guards', () => {
  const ict = (year: number, month: number, day: number, hour: number, minute: number) =>
    Date.UTC(year, month - 1, day, hour - 7, minute, 0, 0)

  it('includes 30m in INTRADAY_INTERVALS and computes VN session breaks on 30m charts (Audit Fix #3)', () => {
    expect(INTRADAY_INTERVALS).toContain('30m')
    expect(isDrawingVisibleOnInterval({ visibleTimeframes: 'intraday' }, '30m')).toBe(true)
    expect(isDrawingVisibleOnInterval({ visibleTimeframes: 'swing' }, '30m')).toBe(false)

    const bars = [
      { timestamp: ict(2026, 4, 20, 9, 0) },
      { timestamp: ict(2026, 4, 20, 11, 0) },
      { timestamp: ict(2026, 4, 20, 13, 0) },
      { timestamp: ict(2026, 4, 20, 14, 30) },
      { timestamp: ict(2026, 4, 21, 9, 0) },
    ]
    const breaks = computeVnSessionBreaks(bars, '30m')
    expect(breaks.map((b) => b.kind)).toEqual(['afternoon', 'atc', 'morning'])
    expect(breaks[0].label).toBe('13:00 Chiều')
    expect(breaks[1].label).toBe('14:30 ATC')
    expect(breaks[2].label).toBe('21/04 09:00')
  })

  it('toggles pane ADJ/RAW adjustment mode per slot cleanly', () => {
    const store = useWorkspaceStore.getState()
    const slot = store.panes[0].slotId
    expect(store.panes[0].adjustment ?? 'splits').toBe('splits')
    store.togglePaneAdjustment(slot)
    expect(useWorkspaceStore.getState().panes[0].adjustment).toBe('unadjusted')
    store.togglePaneAdjustment(slot)
    expect(useWorkspaceStore.getState().panes[0].adjustment).toBe('splits')
  })

  it('formats cross-platform keyboard shortcut helpers deterministically', () => {
    expect(['⌘', 'Ctrl+']).toContain(modKeyLabel())
    expect(['⌥', 'Alt+']).toContain(altKeyLabel())
    const k = formatShortcut('K', { mod: true })
    expect(k.endsWith('K')).toBe(true)
  })
})
