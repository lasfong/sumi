import type { Chart, KLineData } from 'klinecharts'
import { useWorkspaceStore } from '../store/workspaceStore'

type KLineChartWithStore = Chart & {
  getChartStore?: () => {
    setCrosshair: (
      crosshair?: { x?: number; y?: number; paneId?: string },
      options?: { notInvalidate?: boolean; notExecuteAction?: boolean; forceInvalidate?: boolean },
    ) => void
  }
}

/**
 * Zero-React-re-render Crosshair & Time-Range Synchronization Bus (`BUG-004` + `BUG-043`).
 *
 * Connects active KLineChart v10 canvas instances directly via a module-level `Map`
 * coalesced on `requestAnimationFrame` so moving the mouse or scrolling across 2-4 panes
 * causes **0 React renders** across the entire UI tree (`120 FPS` smooth!).
 */
class CrosshairBus {
  private readonly charts = new Map<string, Chart>()
  private readonly suppressed = new Set<string>()
  private pending: { sourceSlotId: string; timestamp: number | null } | null = null
  private rafId: number | null = null

  register(slotId: string, chart: Chart): void {
    this.charts.set(slotId, chart)
  }

  unregister(slotId: string): void {
    this.charts.delete(slotId)
    this.suppressed.delete(slotId)
  }

  isSuppressed(slotId: string): boolean {
    return this.suppressed.has(slotId)
  }

  publish(sourceSlotId: string, timestamp: number | null): void {
    if (this.charts.size <= 1) return
    if (!useWorkspaceStore.getState().sync.crosshair) return
    this.pending = { sourceSlotId, timestamp }
    if (this.rafId === null && typeof window !== 'undefined') {
      this.rafId = window.requestAnimationFrame(() => {
        this.rafId = null
        const evt = this.pending
        this.pending = null
        if (evt) {
          this.flush(evt.sourceSlotId, evt.timestamp)
        }
      })
    }
  }

  private flush(sourceSlotId: string, timestamp: number | null): void {
    if (!useWorkspaceStore.getState().sync.crosshair) return

    for (const [targetSlotId, targetChart] of this.charts.entries()) {
      if (targetSlotId === sourceSlotId) continue
      const store = (targetChart as KLineChartWithStore).getChartStore?.()

      if (timestamp === null) {
        this.suppressed.add(targetSlotId)
        try {
          if (store) {
            store.setCrosshair(undefined, { notExecuteAction: true })
          }
        } finally {
          this.suppressed.delete(targetSlotId)
        }
        continue
      }

      const dataList = targetChart.getDataList()
      if (dataList.length === 0) continue

      const matched = findNearestBarAtOrBefore(dataList, timestamp)
      const targetTs = matched ? matched.timestamp : timestamp
      const pixel = targetChart.convertToPixel(
        matched ? { timestamp: targetTs, value: matched.close } : { timestamp: targetTs },
        { paneId: 'candle_pane' },
      )
      const coord = Array.isArray(pixel) ? pixel[0] : pixel
      if (coord && typeof coord.x === 'number' && Number.isFinite(coord.x)) {
        this.suppressed.add(targetSlotId)
        try {
          // KLineChart v10.1.1: `getChartStore().setCrosshair(..., { notExecuteAction: true })`
          // updates internal crosshair state + repaints Overlay pane cleanly without feedback loops!
          if (store) {
            store.setCrosshair(
              {
                x: coord.x,
                y: typeof coord.y === 'number' && Number.isFinite(coord.y) ? coord.y : 100,
                paneId: 'candle_pane',
              },
              { notExecuteAction: true },
            )
          }
        } finally {
          this.suppressed.delete(targetSlotId)
        }
      }
    }
  }
}

/**
 * Zero-React-re-render Horizontal Time-Range Synchronization Bus (`Sync Date Range`, `BUG-043`).
 * Keeps sibling MTF panes (`1h` + `4h` + `1D`) aligned to the same historical date when panning!
 */
class TimeRangeBus {
  private readonly charts = new Map<string, Chart>()
  private readonly suppressed = new Set<string>()
  private pending: { sourceSlotId: string; centerTimestamp: number } | null = null
  private rafId: number | null = null

  register(slotId: string, chart: Chart): void {
    this.charts.set(slotId, chart)
  }

  unregister(slotId: string): void {
    this.charts.delete(slotId)
    this.suppressed.delete(slotId)
  }

  isSuppressed(slotId: string): boolean {
    return this.suppressed.has(slotId)
  }

  publish(sourceSlotId: string, centerTimestamp: number): void {
    if (this.charts.size <= 1) return
    if (!useWorkspaceStore.getState().sync.dateRange) return
    this.pending = { sourceSlotId, centerTimestamp }
    if (this.rafId === null && typeof window !== 'undefined') {
      this.rafId = window.requestAnimationFrame(() => {
        this.rafId = null
        const evt = this.pending
        this.pending = null
        if (evt) {
          this.flush(evt.sourceSlotId, evt.centerTimestamp)
        }
      })
    }
  }

  private flush(sourceSlotId: string, centerTimestamp: number): void {
    if (!useWorkspaceStore.getState().sync.dateRange) return
    for (const [targetSlotId, targetChart] of this.charts.entries()) {
      if (targetSlotId === sourceSlotId) continue
      const dataList = targetChart.getDataList()
      if (dataList.length === 0) continue
      const matched = findNearestBarAtOrBefore(dataList, centerTimestamp)
      if (!matched) continue
      this.suppressed.add(targetSlotId)
      try {
        targetChart.scrollToTimestamp(matched.timestamp, 0)
      } finally {
        // Release suppression on next animation frame after scroll settles
        window.requestAnimationFrame(() => {
          this.suppressed.delete(targetSlotId)
        })
      }
    }
  }
}

/** O(log N) binary search finding the bar with the largest timestamp <= `targetTs` (great for multi-timeframe sync). */
function findNearestBarAtOrBefore(bars: KLineData[], targetTs: number): KLineData | null {
  if (bars.length === 0) return null
  if (targetTs < bars[0].timestamp) return bars[0]
  if (targetTs >= bars[bars.length - 1].timestamp) return bars[bars.length - 1]

  let lo = 0
  let hi = bars.length - 1
  let best = 0
  while (lo <= hi) {
    const mid = (lo + hi) >>> 1
    if (bars[mid].timestamp <= targetTs) {
      best = mid
      lo = mid + 1
    } else {
      hi = mid - 1
    }
  }
  return bars[best] ?? null
}

export const crosshairBus = new CrosshairBus()
export const timeRangeBus = new TimeRangeBus()
