import React, { useEffect, useMemo, useRef, useState } from 'react'
import { listen } from '@tauri-apps/api/event'
import * as api from './api/tauri'
import { KLineViewport } from './chart/KLineViewport'
import { timeRangeBus } from './chart/crosshairBus'
import { drawingSyncBus, type DrawingSyncEvent } from './chart/drawingSyncBus'
import type { DeepLinkPayload, ReplaySnapshotResponse } from './types/market'
import { BottomTradingDock } from './components/BottomTradingDock'
import { LeftDrawingRail } from './components/LeftDrawingRail'
import { ReplayControlBar } from './components/ReplayControlBar'
import { RightWidgetBar } from './components/RightWidgetBar'
import { TopDownContextStrip } from './components/TopDownContextStrip'
import { TopHeader } from './components/TopHeader'
import { IndicatorsModal } from './components/modals/IndicatorsModal'
import { IndicatorSettingsModal } from './components/modals/IndicatorSettingsModal'
import { SymbolSearchModal } from './components/modals/SymbolSearchModal'
import { useWorkspaceStore } from './store/workspaceStore'

export const App: React.FC = () => {
  const bootstrap = useWorkspaceStore((s) => s.bootstrap)
  const gridLayout = useWorkspaceStore((s) => s.gridLayout)
  const panes = useWorkspaceStore((s) => s.panes)
  const activeSlotId = useWorkspaceStore((s) => s.activeSlotId)
  const rightPanelOpen = useWorkspaceStore((s) => s.rightPanelOpen)
  const rightPanelWidth = useWorkspaceStore((s) => s.rightPanelWidth)
  const bottomDockOpen = useWorkspaceStore((s) => s.bottomDockOpen)
  const bottomDockMaximized = useWorkspaceStore((s) => s.bottomDockMaximized)
  const bottomDockHeight = useWorkspaceStore((s) => s.bottomDockHeight)
  const watchlists = useWorkspaceStore((s) => s.watchlists)
  const activeWatchlistId = useWorkspaceStore((s) => s.activeWatchlistId)
  const dataStatus = useWorkspaceStore((s) => s.dataStatus)
  const toasts = useWorkspaceStore((s) => s.toasts)

  const setActiveSlotId = useWorkspaceStore((s) => s.setActiveSlotId)
  const setRightPanelWidth = useWorkspaceStore((s) => s.setRightPanelWidth)
  const setBottomDockHeight = useWorkspaceStore((s) => s.setBottomDockHeight)
  const applyLiveQuote = useWorkspaceStore((s) => s.applyLiveQuote)
  const openSymbolSearch = useWorkspaceStore((s) => s.openSymbolSearch)
  const setActiveDrawingTool = useWorkspaceStore((s) => s.setActiveDrawingTool)
  const setReplayPlaying = useWorkspaceStore((s) => s.setReplayPlaying)
  const dismissToast = useWorkspaceStore((s) => s.dismissToast)

  const [draggingHandle, setDraggingHandle] = useState<'right' | 'bottom' | null>(null)
  const dragStartRef = useRef<{ x: number; y: number; startSize: number }>({ x: 0, y: 0, startSize: 0 })

  // 1. Bootstrap SQLite workspace state on startup + listen for cross-window sync events (`BUG-043`)
  useEffect(() => {
    void bootstrap()
  }, [bootstrap])

  useEffect(() => {
    if (!api.isTauriRuntime()) return
    const unlisteners: Array<() => void> = []
    let disposed = false

    void Promise.all([
      listen<{ action: 'start' | 'step' | 'jump' | 'stop'; snapshot?: ReplaySnapshotResponse | null }>(
        'sumi:sync_replay',
        (ev) => {
          useWorkspaceStore.getState().applyRemoteReplaySync(ev.payload)
        },
      ),
      listen<DrawingSyncEvent>('sumi:sync_drawing', (ev) => {
        drawingSyncBus.emit(ev.payload, false)
      }),
      listen<{ ticker: string }>('sumi:sync_symbol', (ev) => {
        const st = useWorkspaceStore.getState()
        if (st.sync.symbol && ev.payload?.ticker) {
          const cur = st.panes[0]?.ticker
          if (cur?.toUpperCase() !== ev.payload.ticker.toUpperCase()) {
            const isVn =
              ev.payload.ticker.startsWith('HOSE:') ||
              ev.payload.ticker.startsWith('HNX:') ||
              ev.payload.ticker.startsWith('UPCOM:')
            useWorkspaceStore.setState((s) => ({
              panes: s.panes.map((p) => ({ ...p, ticker: ev.payload.ticker, showVnBands: isVn })),
            }))
          }
        }
      }),
      listen<{ centerTimestamp: number; barSpace?: number | null }>('sumi:sync_range', (ev) => {
        if (typeof ev.payload?.centerTimestamp === 'number') {
          timeRangeBus.applyRemote(ev.payload.centerTimestamp, ev.payload.barSpace ?? null)
        }
      }),
      listen<DeepLinkPayload>('sumi:deeplink', (ev) => {
        if (ev.payload?.ticker) {
          useWorkspaceStore.getState().applyDeepLink(ev.payload)
        }
      }),
    ])
      .then((fns) => {
        if (disposed) {
          for (const fn of fns) fn()
        } else {
          unlisteners.push(...fns)
        }
      })
      .catch(() => {
        // Ignore in single-window or headless harness environments without native event plugin internals
      })

    return () => {
      disposed = true
      for (const fn of unlisteners) fn()
    }
  }, [])

  // 2. Live Quote streaming for active watchlist symbols via Tauri Channel
  const activeWatchlistTickersKey = useMemo(() => {
    const activeWl = watchlists.find((w) => w.id === activeWatchlistId) ?? watchlists[0]
    return (activeWl?.tickers ?? []).join(',')
  }, [watchlists, activeWatchlistId])

  useEffect(() => {
    if (!activeWatchlistTickersKey || dataStatus?.offlineMode) return
    const tickers = activeWatchlistTickersKey.split(',').filter(Boolean)
    if (tickers.length === 0) return

    let active = true
    void api.subscribeQuotes(tickers, (quote) => {
      if (active) {
        applyLiveQuote(quote)
      }
    })
    return () => {
      active = false
      void api.unsubscribeQuotes()
    }
  }, [activeWatchlistTickersKey, dataStatus?.offlineMode, applyLiveQuote])

  // 3. Global TradingView Hotkeys (type any letter -> Symbol Search, Alt+T/H/V/F -> Draw, Space -> Replay)
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement | null
      const inInput =
        target &&
        (target.tagName === 'INPUT' ||
          target.tagName === 'TEXTAREA' ||
          target.tagName === 'SELECT' ||
          target.isContentEditable)
      const state = useWorkspaceStore.getState()
      if (inInput || state.symbolSearchOpen || state.indicatorsModalOpen || state.editingIndicator) return

      // Drawing hotkeys — match `e.code` so macOS Option+T/H/J/V/F (`†/˙/∆/√/ƒ`) work on physical keyboards (`BUG-021`)!
      if (e.altKey && !e.ctrlKey && !e.metaKey) {
        const k = e.key.toLowerCase()
        if (e.code === 'KeyT' || k === 't') {
          e.preventDefault()
          setActiveDrawingTool('segment')
          return
        }
        if (e.code === 'KeyH' || k === 'h') {
          e.preventDefault()
          setActiveDrawingTool('horizontalStraightLine')
          return
        }
        if (e.code === 'KeyJ' || k === 'j') {
          e.preventDefault()
          setActiveDrawingTool('horizontalRayLine')
          return
        }
        if (e.code === 'KeyV' || k === 'v') {
          e.preventDefault()
          setActiveDrawingTool('verticalStraightLine')
          return
        }
        if (e.code === 'KeyF' || k === 'f') {
          e.preventDefault()
          setActiveDrawingTool('fibonacciLine')
          return
        }
      }

      if (e.shiftKey && e.key.toLowerCase() === 'm' && !e.ctrlKey && !e.metaKey) {
        e.preventDefault()
        setActiveDrawingTool('priceDateRange')
        return
      }

      if (e.key === ' ' && state.replay?.active) {
        e.preventDefault()
        setReplayPlaying(!state.replayPlaying)
        return
      }

      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
        e.preventDefault()
        openSymbolSearch('')
        return
      }

      // TradingView signature: typing any alphanumeric character opens Symbol Search immediately!
      if (!e.ctrlKey && !e.metaKey && !e.altKey && /^[a-zA-Z0-9]$/.test(e.key)) {
        e.preventDefault()
        openSymbolSearch(e.key.toUpperCase())
      }
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [openSymbolSearch, setActiveDrawingTool, setReplayPlaying])

  // 4. Smooth pointer-based Resizable Split Panes (Right Widgetbar + Bottom Dock)
  useEffect(() => {
    if (!draggingHandle) return
    const onPointerMove = (e: PointerEvent) => {
      if (draggingHandle === 'right') {
        const delta = dragStartRef.current.x - e.clientX
        setRightPanelWidth(dragStartRef.current.startSize + delta)
      } else if (draggingHandle === 'bottom') {
        const delta = dragStartRef.current.y - e.clientY
        setBottomDockHeight(dragStartRef.current.startSize + delta)
      }
    }
    const onPointerUp = () => setDraggingHandle(null)
    window.addEventListener('pointermove', onPointerMove)
    window.addEventListener('pointerup', onPointerUp)
    return () => {
      window.removeEventListener('pointermove', onPointerMove)
      window.removeEventListener('pointerup', onPointerUp)
    }
  }, [draggingHandle, setRightPanelWidth, setBottomDockHeight])

  const visiblePanes = useMemo(() => {
    if (gridLayout === '1') return panes.slice(0, 1)
    if (gridLayout === '2v' || gridLayout === '2h') return panes.slice(0, 2)
    if (gridLayout === '3') return panes.slice(0, 3)
    return panes.slice(0, 4)
  }, [gridLayout, panes])

  const gridClass =
    gridLayout === '1'
      ? 'grid-cols-1 grid-rows-1'
      : gridLayout === '2v'
        ? 'grid-cols-2 grid-rows-1'
        : gridLayout === '2h'
          ? 'grid-cols-1 grid-rows-2'
          : 'grid-cols-2 grid-rows-2'

  return (
    <div
      className="flex flex-col w-screen h-screen overflow-hidden"
      style={{ backgroundColor: 'var(--tv-bg-chart)', color: 'var(--tv-text-primary)' }}
    >
      {/* 38px Supercharts Top Header */}
      <TopHeader />

      {/* Main Workstation Body: Left Drawing Rail | Center (Charts + Bottom Dock) | Right Widgetbar */}
      <div className="flex flex-1 min-h-0 overflow-hidden">
        {/* 42px Left Drawing Rail (37+ tools) */}
        <LeftDrawingRail />

        {/* Center Workspace Column */}
        <main className="flex flex-col flex-1 min-w-0 min-h-0 overflow-hidden">
          {/* Causal Bar Replay Floating Control Strip (when active) */}
          <ReplayControlBar />

          {/* Yumeko Read-Only Top-Down Context Strip (VM01 -> HP01 -> DN01 -> DM01 + as_of PIT + KT01) */}
          <TopDownContextStrip />

          {/* Multi-Chart Viewport Grid (1 / 2v / 2h / 3 / 4) */}
          <div className={`grid ${gridClass} flex-1 min-w-0 min-h-0 overflow-hidden`}>
            {visiblePanes.map((pane, idx) => (
              <div
                key={pane.slotId}
                className={`min-w-0 min-h-0 overflow-hidden ${
                  gridLayout === '3' && idx === 0 ? 'row-span-2' : ''
                }`}
              >
                <KLineViewport
                  pane={pane}
                  isActive={pane.slotId === activeSlotId}
                  isMultiPane={visiblePanes.length > 1}
                  gridLayout={gridLayout}
                  onActivate={() => setActiveSlotId(pane.slotId)}
                />
              </div>
            ))}
          </div>

          {/* Horizontal Resize Handle for Bottom Trading Dock */}
          {bottomDockOpen && !bottomDockMaximized && (
            <div
              role="separator"
              aria-orientation="horizontal"
              aria-label="Kéo thay đổi chiều cao thanh giao dịch dưới"
              className="split-handle-h"
              data-dragging={draggingHandle === 'bottom'}
              onPointerDown={(e) => {
                dragStartRef.current = { x: e.clientX, y: e.clientY, startSize: bottomDockHeight }
                setDraggingHandle('bottom')
              }}
            />
          )}

          {/* Bottom Trading Dock (Paper Trading VN T+2 / Strategy Tester / Stock Screener 101) */}
          <BottomTradingDock />
        </main>

        {/* Vertical Resize Handle for Right Widgetbar */}
        {rightPanelOpen && (
          <div
            role="separator"
            aria-orientation="vertical"
            aria-label="Kéo thay đổi độ rộng bảng Watchlist"
            className="split-handle-v"
            data-dragging={draggingHandle === 'right'}
            onPointerDown={(e) => {
              dragStartRef.current = { x: e.clientX, y: e.clientY, startSize: rightPanelWidth }
              setDraggingHandle('right')
            }}
          />
        )}

        {/* Right Widgetbar (Watchlist Realtime + VN Symbol Detail + Price Alerts) */}
        {rightPanelOpen && <RightWidgetBar />}
      </div>

      {/* Native <dialog> Modals */}
      <SymbolSearchModal />
      <IndicatorsModal />
      <IndicatorSettingsModal />

      {/* Non-intrusive Toast Stack (top-right below header so bottom dock & order forms stay clear) */}
      {toasts.length > 0 && (
        <div
          aria-live="polite"
          className="fixed top-12 right-4 z-50 flex flex-col gap-1.5 max-w-sm pointer-events-none"
        >
          {toasts.map((t) => (
            <div
              key={t.id}
              className="pointer-events-auto flex items-center justify-between gap-2 px-3 py-2 rounded-md shadow-xl text-[12px]"
              style={{
                backgroundColor: 'var(--tv-bg-elevated)',
                border: `1px solid ${
                  t.kind === 'error'
                    ? '#F23645'
                    : t.kind === 'success'
                      ? '#089981'
                      : t.kind === 'warning'
                        ? '#FBC02D'
                        : 'var(--tv-accent)'
                }`,
                color: 'var(--tv-text-primary)',
              }}
            >
              <span>{t.message}</span>
              <button
                type="button"
                className="border-0 bg-transparent cursor-pointer text-[var(--tv-text-secondary)] hover:text-[var(--tv-text-primary)]"
                onClick={() => dismissToast(t.id)}
              >
                ×
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}

export default App
