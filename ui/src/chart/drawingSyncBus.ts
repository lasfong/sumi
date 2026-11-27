import type { SavedDrawingRecord } from '../types/market'
import * as api from '../api/tauri'
import { useWorkspaceStore } from '../store/workspaceStore'

export type DrawingSyncEvent =
  | { kind: 'upsert'; sourceSlotId: string; ticker: string; record: SavedDrawingRecord }
  | { kind: 'delete'; sourceSlotId: string; ticker: string; id: string }
  | { kind: 'clear'; sourceSlotId: string; ticker: string }

type Listener = (ev: DrawingSyncEvent) => void

/**
 * Instant Multi-Pane & Multi-Window Drawing Synchronization Bus (`BUG-027` + `BUG-043`).
 * When a trader draws a Support/Resistance level or Trendline on `4h`, every sibling pane
 * (e.g. `1h`, `15m`, `1D`) and detached OS window displaying the same ticker mirrors it immediately!
 */
class DrawingSyncBus {
  private readonly listeners = new Map<string, Listener>()

  subscribe(slotId: string, fn: Listener): () => void {
    this.listeners.set(slotId, fn)
    return () => {
      this.listeners.delete(slotId)
    }
  }

  emit(ev: DrawingSyncEvent, broadcastCrossWindow = true): void {
    if (!useWorkspaceStore.getState().sync.drawings) return
    for (const [slotId, fn] of this.listeners.entries()) {
      if (slotId === ev.sourceSlotId) continue
      fn(ev)
    }
    if (broadcastCrossWindow) {
      void api.broadcastSyncEvent('sumi:sync_drawing', {
        ...ev,
        sourceSlotId: '__remote_window__',
      })
    }
  }
}

export const drawingSyncBus = new DrawingSyncBus()
