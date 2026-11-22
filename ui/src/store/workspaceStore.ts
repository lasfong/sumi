import { create } from 'zustand'
import type {
  BottomDockTab,
  BrokerStateSnapshot,
  ChartPaneConfig,
  ChartStyleKind,
  DataStatusResponse,
  GridLayoutKind,
  IntervalKey,
  MagnetMode,
  PriceAlertRecord,
  PriceScaleKind,
  ReplaySnapshotResponse,
  SavedIndicatorSpec,
  SavedLayoutRecord,
  ThemeMode,
  WatchlistGroup,
  WatchlistQuoteSnapshot,
} from '../types/market'
import * as api from '../api/tauri'

export interface SyncOptions {
  symbol: boolean
  interval: boolean
  crosshair: boolean
}

export interface ToastNotice {
  id: string
  kind: 'info' | 'success' | 'warning' | 'error'
  message: string
}

const DEFAULT_PANES: ChartPaneConfig[] = [
  {
    slotId: 'pane-0',
    ticker: 'HOSE:FPT',
    interval: '1D',
    chartStyle: 'candle_solid',
    priceScale: 'normal',
    showVnBands: true,
    showCountdown: true,
    indicators: [
      { id: 'ind_vol_default', name: 'VOL', paneId: 'pane_vol', isMain: false, calcParams: [5, 10, 20], visible: true },
      { id: 'ind_ema_default', name: 'EMA', paneId: 'candle_pane', isMain: true, calcParams: [20, 50], visible: true },
    ],
  },
  {
    slotId: 'pane-1',
    ticker: 'HOSE:VCB',
    interval: '1D',
    chartStyle: 'candle_solid',
    priceScale: 'normal',
    showVnBands: true,
    showCountdown: true,
    indicators: [
      { id: 'ind_vol_1', name: 'VOL', paneId: 'pane_vol', isMain: false, calcParams: [5, 10, 20], visible: true },
    ],
  },
  {
    slotId: 'pane-2',
    ticker: 'HOSE:HPG',
    interval: '1D',
    chartStyle: 'candle_solid',
    priceScale: 'normal',
    showVnBands: true,
    showCountdown: true,
    indicators: [
      { id: 'ind_vol_2', name: 'VOL', paneId: 'pane_vol', isMain: false, calcParams: [5, 10, 20], visible: true },
    ],
  },
  {
    slotId: 'pane-3',
    ticker: 'BINANCE:BTCUSDT',
    interval: '1h',
    chartStyle: 'candle_solid',
    priceScale: 'normal',
    showVnBands: false,
    showCountdown: true,
    indicators: [
      { id: 'ind_vol_3', name: 'VOL', paneId: 'pane_vol', isMain: false, calcParams: [5, 10, 20], visible: true },
    ],
  },
]

export interface WorkspaceState {
  initialized: boolean
  theme: ThemeMode
  gridLayout: GridLayoutKind
  sync: SyncOptions
  activeSlotId: string
  panes: ChartPaneConfig[]

  // Drawing rail state
  activeDrawingTool: string | null
  magnetMode: MagnetMode
  drawingsLocked: boolean
  drawingsHidden: boolean
  clearDrawingsNonce: number
  exportPngNonce: number

  // Crosshair sync broadcast & jump-to-timestamp command
  syncedCrosshair: { sourceSlotId: string; timestamp: number | null; price?: number } | null
  scrollToTimestampCmd: { slotId: string; timestampMs: number; nonce: number } | null

  // Panels layout
  rightPanelOpen: boolean
  rightPanelWidth: number
  bottomDockOpen: boolean
  bottomDockMaximized: boolean
  bottomDockHeight: number
  bottomDockTab: BottomDockTab

  // Modals
  symbolSearchOpen: boolean
  symbolSearchInitialQuery: string
  indicatorsModalOpen: boolean

  // Watchlists, live quotes, alerts, layouts, data status
  watchlists: WatchlistGroup[]
  activeWatchlistId: string
  quotesMap: Record<string, WatchlistQuoteSnapshot>
  quoteFlashMap: Record<string, { dir: 'up' | 'down'; nonce: number }>
  alerts: PriceAlertRecord[]
  layouts: SavedLayoutRecord[]
  dataStatus: DataStatusResponse | null
  importingCafeF: boolean

  // Causal Bar Replay state
  replay: ReplaySnapshotResponse | null
  replaySelectingCut: boolean
  replayPlaying: boolean
  replayResetNonce: number
  replaySteppedBar: { bar: ReplaySnapshotResponse['steppedBar']; nonce: number } | null

  // Paper Broker state (synced with active chart & replay)
  broker: BrokerStateSnapshot | null

  // Toast notifications
  toasts: ToastNotice[]

  // Actions
  bootstrap: () => Promise<void>
  setTheme: (theme: ThemeMode) => void
  toggleTheme: () => void
  setGridLayout: (layout: GridLayoutKind) => void
  setSyncOption: (key: keyof SyncOptions, value: boolean) => void
  setActiveSlotId: (slotId: string) => void

  setActiveTicker: (ticker: string, slotId?: string) => void
  setActiveInterval: (interval: IntervalKey, slotId?: string) => void
  setActiveChartStyle: (style: ChartStyleKind) => void
  setActivePriceScale: (scale: PriceScaleKind) => void
  toggleActiveVnBands: () => void
  toggleActiveCountdown: () => void

  addIndicatorToActivePane: (spec: Omit<SavedIndicatorSpec, 'id'>) => void
  updateIndicatorInActivePane: (id: string, patch: Partial<SavedIndicatorSpec>) => void
  removeIndicatorFromActivePane: (id: string) => void

  setActiveDrawingTool: (tool: string | null) => void
  setMagnetMode: (mode: MagnetMode) => void
  toggleDrawingsLocked: () => void
  toggleDrawingsHidden: () => void
  triggerClearDrawings: () => void
  triggerExportPng: () => void

  publishCrosshairSync: (sourceSlotId: string, timestamp: number | null) => void
  scrollActiveChartToTimestamp: (timestampMs: number) => void

  setRightPanelOpen: (open: boolean) => void
  setRightPanelWidth: (width: number) => void
  setBottomDockOpen: (open: boolean) => void
  setBottomDockMaximized: (max: boolean) => void
  setBottomDockHeight: (height: number) => void
  setBottomDockTab: (tab: BottomDockTab) => void

  openSymbolSearch: (initialQuery?: string) => void
  closeSymbolSearch: () => void
  setIndicatorsModalOpen: (open: boolean) => void

  setActiveWatchlistId: (id: string) => void
  refreshWatchlistQuotes: () => Promise<void>
  applyLiveQuote: (quote: {
    ticker: string
    exchange: string
    symbol: string
    last_price: number | null
    prev_close: number | null
    change: number | null
    change_percent: number | null
    volume: number | null
    timestamp_ms: number
  }) => void
  addSymbolToWatchlist: (ticker: string) => Promise<void>
  removeSymbolFromWatchlist: (ticker: string) => Promise<void>
  createNewWatchlist: (name: string) => Promise<void>

  createNewAlert: (ticker: string, condition: string, targetPrice: number, message: string) => Promise<void>
  toggleAlert: (id: string, status: string) => Promise<void>
  removeAlert: (id: string) => Promise<void>

  saveCurrentLayout: (name: string) => Promise<void>
  applySavedLayout: (layout: SavedLayoutRecord) => void
  deleteSavedLayout: (id: string) => Promise<void>

  toggleOfflineMode: () => Promise<void>
  importCafeFDirectory: (dir: string) => Promise<void>

  // Replay actions
  setReplaySelectingCut: (selecting: boolean) => void
  startReplay: (cutTimestampMs?: number | null) => Promise<void>
  stepReplay: (delta: number) => Promise<void>
  jumpReplay: (targetTimestampMs: number) => Promise<void>
  setReplaySpeed: (mult: number) => Promise<void>
  setReplayPlaying: (playing: boolean) => void
  stopReplay: () => Promise<void>

  // Broker actions
  refreshBroker: () => Promise<void>
  setBrokerSnapshot: (snap: BrokerStateSnapshot) => void

  // Toast helpers
  pushToast: (kind: ToastNotice['kind'], message: string) => void
  dismissToast: (id: string) => void
}

function applyThemeToDocument(theme: ThemeMode): void {
  if (typeof document === 'undefined') return
  document.documentElement.dataset.theme = theme
  const meta = document.querySelector('meta[name="color-scheme"]')
  if (meta) {
    meta.setAttribute('content', theme)
  }
  try {
    localStorage.setItem('sumi-theme', theme)
  } catch {
    // ignore
  }
}

let persistTimer: number | null = null

function schedulePersistSettings(state: WorkspaceState): void {
  if (typeof window === 'undefined') return
  if (persistTimer !== null) {
    window.clearTimeout(persistTimer)
  }
  persistTimer = window.setTimeout(() => {
    const payload = {
      theme: state.theme,
      gridLayout: state.gridLayout,
      sync: state.sync,
      activeSlotId: state.activeSlotId,
      panes: state.panes,
      rightPanelOpen: state.rightPanelOpen,
      rightPanelWidth: state.rightPanelWidth,
      bottomDockOpen: state.bottomDockOpen,
      bottomDockHeight: state.bottomDockHeight,
      bottomDockTab: state.bottomDockTab,
      activeWatchlistId: state.activeWatchlistId,
    }
    void api.saveWorkspaceSettings(payload)
  }, 350)
}

export const useWorkspaceStore = create<WorkspaceState>((set, get) => ({
  initialized: false,
  theme: 'dark',
  gridLayout: '1',
  sync: { symbol: false, interval: true, crosshair: true },
  activeSlotId: 'pane-0',
  panes: DEFAULT_PANES,

  activeDrawingTool: null,
  magnetMode: 'weak_magnet',
  drawingsLocked: false,
  drawingsHidden: false,
  clearDrawingsNonce: 0,
  exportPngNonce: 0,

  syncedCrosshair: null,
  scrollToTimestampCmd: null,

  rightPanelOpen: true,
  rightPanelWidth: 288,
  bottomDockOpen: true,
  bottomDockMaximized: false,
  bottomDockHeight: 276,
  bottomDockTab: 'paper',

  symbolSearchOpen: false,
  symbolSearchInitialQuery: '',
  indicatorsModalOpen: false,

  watchlists: [],
  activeWatchlistId: 'wl_vn30',
  quotesMap: {},
  quoteFlashMap: {},
  alerts: [],
  layouts: [],
  dataStatus: null,
  importingCafeF: false,

  replay: null,
  replaySelectingCut: false,
  replayPlaying: false,
  replayResetNonce: 0,
  replaySteppedBar: null,

  broker: null,
  toasts: [],

  bootstrap: async () => {
    try {
      const [boot, status] = await Promise.all([api.loadWorkspaceBootstrap(), api.getDataStatus()])
      const saved = boot.settings ?? {}
      const theme: ThemeMode = saved.theme === 'light' ? 'light' : 'dark'
      applyThemeToDocument(theme)

      const panes = Array.isArray(saved.panes) && saved.panes.length === 4
        ? (saved.panes as ChartPaneConfig[])
        : DEFAULT_PANES

      const watchlists = boot.watchlists
      const activeWatchlistId =
        typeof saved.activeWatchlistId === 'string' && watchlists.some((w) => w.id === saved.activeWatchlistId)
          ? saved.activeWatchlistId
          : (watchlists[0]?.id ?? 'wl_vn30')

      set({
        initialized: true,
        theme,
        gridLayout: (saved.gridLayout as GridLayoutKind) ?? '1',
        sync: (saved.sync as SyncOptions) ?? { symbol: false, interval: true, crosshair: true },
        activeSlotId: typeof saved.activeSlotId === 'string' ? saved.activeSlotId : 'pane-0',
        panes,
        rightPanelOpen: typeof saved.rightPanelOpen === 'boolean' ? saved.rightPanelOpen : true,
        rightPanelWidth: typeof saved.rightPanelWidth === 'number' ? saved.rightPanelWidth : 288,
        bottomDockOpen: typeof saved.bottomDockOpen === 'boolean' ? saved.bottomDockOpen : true,
        bottomDockHeight: typeof saved.bottomDockHeight === 'number' ? saved.bottomDockHeight : 276,
        bottomDockTab: (saved.bottomDockTab as BottomDockTab) ?? 'paper',
        watchlists,
        activeWatchlistId,
        alerts: boot.alerts,
        layouts: boot.layouts,
        dataStatus: status,
      })

      await Promise.all([get().refreshWatchlistQuotes(), get().refreshBroker()])
    } catch (err) {
      set({ initialized: true })
      get().pushToast('error', `Khởi tạo workspace lỗi: ${String(err)}`)
    }
  },

  setTheme: (theme) => {
    applyThemeToDocument(theme)
    set({ theme })
    schedulePersistSettings(get())
  },

  toggleTheme: () => {
    const next: ThemeMode = get().theme === 'dark' ? 'light' : 'dark'
    get().setTheme(next)
  },

  setGridLayout: (gridLayout) => {
    set({ gridLayout })
    schedulePersistSettings(get())
  },

  setSyncOption: (key, value) => {
    set((s) => ({ sync: { ...s.sync, [key]: value } }))
    schedulePersistSettings(get())
  },

  setActiveSlotId: (activeSlotId) => {
    set({ activeSlotId })
    schedulePersistSettings(get())
    void get().refreshBroker()
  },

  setActiveTicker: (ticker, slotId) => {
    const targetSlot = slotId ?? get().activeSlotId
    const syncSymbol = get().sync.symbol
    const isVn = ticker.startsWith('HOSE:') || ticker.startsWith('HNX:') || ticker.startsWith('UPCOM:')
    set((s) => ({
      panes: s.panes.map((p) =>
        syncSymbol || p.slotId === targetSlot
          ? { ...p, ticker, showVnBands: isVn }
          : p,
      ),
    }))
    schedulePersistSettings(get())
    void get().refreshBroker()
  },

  setActiveInterval: (interval, slotId) => {
    const targetSlot = slotId ?? get().activeSlotId
    const syncIv = get().sync.interval
    set((s) => ({
      panes: s.panes.map((p) => (syncIv || p.slotId === targetSlot ? { ...p, interval } : p)),
    }))
    schedulePersistSettings(get())
    void get().refreshBroker()
  },

  setActiveChartStyle: (chartStyle) => {
    const slot = get().activeSlotId
    set((s) => ({
      panes: s.panes.map((p) => (p.slotId === slot ? { ...p, chartStyle } : p)),
    }))
    schedulePersistSettings(get())
  },

  setActivePriceScale: (priceScale) => {
    const slot = get().activeSlotId
    set((s) => ({
      panes: s.panes.map((p) => (p.slotId === slot ? { ...p, priceScale } : p)),
    }))
    schedulePersistSettings(get())
  },

  toggleActiveVnBands: () => {
    const slot = get().activeSlotId
    set((s) => ({
      panes: s.panes.map((p) => (p.slotId === slot ? { ...p, showVnBands: !p.showVnBands } : p)),
    }))
    schedulePersistSettings(get())
  },

  toggleActiveCountdown: () => {
    const slot = get().activeSlotId
    set((s) => ({
      panes: s.panes.map((p) => (p.slotId === slot ? { ...p, showCountdown: !p.showCountdown } : p)),
    }))
    schedulePersistSettings(get())
  },

  addIndicatorToActivePane: (spec) => {
    const slot = get().activeSlotId
    const newItem: SavedIndicatorSpec = {
      ...spec,
      id: `ind_${spec.name}_${Date.now()}_${Math.floor(Math.random() * 1000)}`,
    }
    set((s) => ({
      panes: s.panes.map((p) =>
        p.slotId === slot ? { ...p, indicators: [...p.indicators, newItem] } : p,
      ),
    }))
    const pane = get().panes.find((p) => p.slotId === slot)
    if (pane) {
      void api.savePaneIndicators(slot, pane.indicators)
    }
    schedulePersistSettings(get())
  },

  updateIndicatorInActivePane: (id, patch) => {
    const slot = get().activeSlotId
    set((s) => ({
      panes: s.panes.map((p) =>
        p.slotId === slot
          ? {
              ...p,
              indicators: p.indicators.map((ind) => (ind.id === id ? { ...ind, ...patch } : ind)),
            }
          : p,
      ),
    }))
    const pane = get().panes.find((p) => p.slotId === slot)
    if (pane) {
      void api.savePaneIndicators(slot, pane.indicators)
    }
    schedulePersistSettings(get())
  },

  removeIndicatorFromActivePane: (id) => {
    const slot = get().activeSlotId
    set((s) => ({
      panes: s.panes.map((p) =>
        p.slotId === slot ? { ...p, indicators: p.indicators.filter((ind) => ind.id !== id) } : p,
      ),
    }))
    const pane = get().panes.find((p) => p.slotId === slot)
    if (pane) {
      void api.savePaneIndicators(slot, pane.indicators)
    }
    schedulePersistSettings(get())
  },

  setActiveDrawingTool: (activeDrawingTool) => set({ activeDrawingTool }),
  setMagnetMode: (magnetMode) => set({ magnetMode }),
  toggleDrawingsLocked: () => set((s) => ({ drawingsLocked: !s.drawingsLocked })),
  toggleDrawingsHidden: () => set((s) => ({ drawingsHidden: !s.drawingsHidden })),
  triggerClearDrawings: () => set((s) => ({ clearDrawingsNonce: s.clearDrawingsNonce + 1 })),
  triggerExportPng: () => set((s) => ({ exportPngNonce: s.exportPngNonce + 1 })),

  publishCrosshairSync: (sourceSlotId, timestamp) => {
    if (!get().sync.crosshair) return
    set({ syncedCrosshair: { sourceSlotId, timestamp } })
  },

  scrollActiveChartToTimestamp: (timestampMs) => {
    const slotId = get().activeSlotId
    set((s) => ({
      scrollToTimestampCmd: {
        slotId,
        timestampMs,
        nonce: (s.scrollToTimestampCmd?.nonce ?? 0) + 1,
      },
    }))
  },

  setRightPanelOpen: (rightPanelOpen) => {
    set({ rightPanelOpen })
    schedulePersistSettings(get())
  },
  setRightPanelWidth: (width) => {
    set({ rightPanelWidth: Math.max(220, Math.min(460, Math.round(width))) })
    schedulePersistSettings(get())
  },
  setBottomDockOpen: (bottomDockOpen) => {
    set({ bottomDockOpen, bottomDockMaximized: bottomDockOpen ? get().bottomDockMaximized : false })
    schedulePersistSettings(get())
  },
  setBottomDockMaximized: (bottomDockMaximized) => {
    set({ bottomDockMaximized, bottomDockOpen: true })
  },
  setBottomDockHeight: (height) => {
    set({ bottomDockHeight: Math.max(150, Math.min(540, Math.round(height))) })
    schedulePersistSettings(get())
  },
  setBottomDockTab: (bottomDockTab) => {
    set({ bottomDockTab, bottomDockOpen: true })
    schedulePersistSettings(get())
  },

  openSymbolSearch: (initialQuery = '') =>
    set({ symbolSearchOpen: true, symbolSearchInitialQuery: initialQuery }),
  closeSymbolSearch: () => set({ symbolSearchOpen: false, symbolSearchInitialQuery: '' }),
  setIndicatorsModalOpen: (indicatorsModalOpen) => set({ indicatorsModalOpen }),

  setActiveWatchlistId: (activeWatchlistId) => {
    set({ activeWatchlistId })
    schedulePersistSettings(get())
    void get().refreshWatchlistQuotes()
  },

  refreshWatchlistQuotes: async () => {
    const state = get()
    const allTickers = Array.from(
      new Set([
        ...state.watchlists.flatMap((w) => w.tickers),
        ...state.panes.map((p) => p.ticker),
      ]),
    )
    if (allTickers.length === 0) return
    try {
      const snaps = await api.getWatchlistSnapshots(allTickers)
      const nextMap: Record<string, WatchlistQuoteSnapshot> = { ...state.quotesMap }
      for (const snap of snaps) {
        nextMap[snap.ticker] = snap
      }
      set({ quotesMap: nextMap })
    } catch {
      // ignore transient error
    }
  },

  applyLiveQuote: (quote) => {
    const prevSnap = get().quotesMap[quote.ticker]
    const prevPrice = prevSnap?.lastPrice
    const nextPrice = quote.last_price ?? prevPrice ?? null
    const flashDir: 'up' | 'down' | null =
      prevPrice !== null && prevPrice !== undefined && nextPrice !== null && nextPrice !== prevPrice
        ? nextPrice > prevPrice
          ? 'up'
          : 'down'
        : null

    set((s) => {
      const cur = s.quotesMap[quote.ticker]
      const updated: WatchlistQuoteSnapshot = {
        ticker: quote.ticker,
        exchange: cur?.exchange ?? quote.exchange,
        symbol: cur?.symbol ?? quote.symbol,
        description: cur?.description ?? quote.symbol,
        lastPrice: nextPrice,
        prevClose: quote.prev_close ?? cur?.prevClose ?? null,
        change: quote.change ?? cur?.change ?? null,
        changePercent: quote.change_percent ?? cur?.changePercent ?? null,
        volume: quote.volume ?? cur?.volume ?? null,
        pricePrecision: cur?.pricePrecision ?? 2,
        vnBand: cur?.vnBand ?? null,
        timestampMs: quote.timestamp_ms,
      }
      const nextFlash = { ...s.quoteFlashMap }
      if (flashDir) {
        nextFlash[quote.ticker] = {
          dir: flashDir,
          nonce: (nextFlash[quote.ticker]?.nonce ?? 0) + 1,
        }
      }
      return {
        quotesMap: { ...s.quotesMap, [quote.ticker]: updated },
        quoteFlashMap: nextFlash,
      }
    })

    // Evaluate armed price alerts when live price ticks across target
    if (prevPrice !== null && prevPrice !== undefined && nextPrice !== null && nextPrice !== prevPrice) {
      void api.checkPriceAlerts(quote.ticker, prevPrice, nextPrice).then((fired) => {
        if (fired.length > 0) {
          set((s) => ({
            alerts: s.alerts.map((a) => fired.find((f) => f.id === a.id) ?? a),
          }))
          for (const f of fired) {
            get().pushToast('warning', `🔔 Cảnh báo ${f.ticker}: ${f.message || `Chạm mốc ${f.targetPrice}`}`)
          }
        }
      })
    }
  },

  addSymbolToWatchlist: async (ticker) => {
    const wlId = get().activeWatchlistId
    await api.addWatchlistSymbol(wlId, ticker)
    set((s) => ({
      watchlists: s.watchlists.map((w) =>
        w.id === wlId && !w.tickers.includes(ticker) ? { ...w, tickers: [...w.tickers, ticker] } : w,
      ),
    }))
    await get().refreshWatchlistQuotes()
  },

  removeSymbolFromWatchlist: async (ticker) => {
    const wlId = get().activeWatchlistId
    await api.removeWatchlistSymbol(wlId, ticker)
    set((s) => ({
      watchlists: s.watchlists.map((w) =>
        w.id === wlId ? { ...w, tickers: w.tickers.filter((t) => t !== ticker) } : w,
      ),
    }))
  },

  createNewWatchlist: async (name) => {
    const trimmed = name.trim()
    if (!trimmed) return
    const id = `wl_${Date.now()}`
    const group = await api.createWatchlist(id, trimmed)
    set((s) => ({
      watchlists: [...s.watchlists, group],
      activeWatchlistId: group.id,
    }))
  },

  createNewAlert: async (ticker, condition, targetPrice, message) => {
    const rec = await api.createAlert(ticker, condition, targetPrice, message)
    set((s) => ({ alerts: [rec, ...s.alerts] }))
    get().pushToast('success', `Đã tạo cảnh báo ${ticker} @ ${targetPrice.toLocaleString('vi-VN')}`)
  },

  toggleAlert: async (id, status) => {
    await api.toggleAlertStatus(id, status)
    set((s) => ({ alerts: s.alerts.map((a) => (a.id === id ? { ...a, status } : a)) }))
  },

  removeAlert: async (id) => {
    await api.deleteAlert(id)
    set((s) => ({ alerts: s.alerts.filter((a) => a.id !== id) }))
  },

  saveCurrentLayout: async (name) => {
    const trimmed = name.trim()
    if (!trimmed) return
    const s = get()
    const id = `layout_${Date.now()}`
    const rec = await api.saveLayout(id, trimmed, {
      gridLayout: s.gridLayout,
      sync: s.sync,
      panes: s.panes,
    })
    set((st) => ({ layouts: [rec, ...st.layouts.filter((l) => l.id !== rec.id)] }))
    get().pushToast('success', `Đã lưu bố cục "${trimmed}" vào SQLite`)
  },

  applySavedLayout: (layout) => {
    const json = layout.layoutJson
    const gridLayout = (json.gridLayout as GridLayoutKind) ?? '1'
    const sync = (json.sync as SyncOptions) ?? get().sync
    const panes = Array.isArray(json.panes) ? (json.panes as ChartPaneConfig[]) : get().panes
    set({ gridLayout, sync, panes })
    schedulePersistSettings(get())
    get().pushToast('info', `Đã nạp bố cục "${layout.name}"`)
  },

  deleteSavedLayout: async (id) => {
    await api.deleteLayout(id)
    set((s) => ({ layouts: s.layouts.filter((l) => l.id !== id) }))
  },

  toggleOfflineMode: async () => {
    const next = !(get().dataStatus?.offlineMode ?? false)
    const status = await api.setOfflineMode(next)
    set({ dataStatus: status })
    get().pushToast('info', next ? 'Đã chuyển sang chế độ Offline (SQLite cục bộ)' : 'Đã bật kết nối Online Realtime')
  },

  importCafeFDirectory: async (dir) => {
    const trimmed = dir.trim()
    if (!trimmed || get().importingCafeF) return
    set({ importingCafeF: true })
    try {
      const summary = await api.importCafeFDir(trimmed)
      const status = await api.getDataStatus()
      set({ dataStatus: status, importingCafeF: false })
      await Promise.all([get().refreshWatchlistQuotes(), get().refreshBroker()])
      get().pushToast(
        'success',
        `Đã nạp ${summary.distinct_symbols.toLocaleString('en-US')} mã (${summary.total_bars_written.toLocaleString('en-US')} nến) vào SQLite trong ${summary.elapsed_ms}ms`,
      )
    } catch (err) {
      set({ importingCafeF: false })
      get().pushToast('error', `Không thể nạp thư mục CafeF: ${String(err)}`)
    }
  },

  // ─── Causal Bar Replay ─────────────────────────────────────────────────────

  setReplaySelectingCut: (replaySelectingCut) => set({ replaySelectingCut }),

  startReplay: async (cutTimestampMs) => {
    const activePane = get().panes.find((p) => p.slotId === get().activeSlotId) ?? get().panes[0]
    try {
      const snap = await api.replayStart(activePane.ticker, activePane.interval, cutTimestampMs)
      set((s) => ({
        replay: snap,
        replaySelectingCut: false,
        replayPlaying: false,
        replayResetNonce: s.replayResetNonce + 1,
        broker: snap.broker ?? s.broker,
      }))
      get().pushToast('info', `Bar Replay nhân quả kích hoạt (${snap.cursorIndex + 1}/${snap.totalBars} nến)`)
    } catch (err) {
      get().pushToast('error', String(err))
    }
  },

  stepReplay: async (delta) => {
    try {
      const snap = await api.replayStep(delta)
      set((s) => ({
        replay: snap,
        replayPlaying: snap.status === 'Completed' ? false : s.replayPlaying,
        broker: snap.broker ?? s.broker,
        replayResetNonce: snap.steppedBar ? s.replayResetNonce : s.replayResetNonce + 1,
        replaySteppedBar: snap.steppedBar
          ? { bar: snap.steppedBar, nonce: (s.replaySteppedBar?.nonce ?? 0) + 1 }
          : s.replaySteppedBar,
      }))
    } catch (err) {
      set({ replayPlaying: false })
      get().pushToast('error', String(err))
    }
  },

  jumpReplay: async (targetTimestampMs) => {
    try {
      const snap = await api.replayJump(targetTimestampMs)
      set((s) => ({
        replay: snap,
        replaySelectingCut: false,
        replayPlaying: false,
        replayResetNonce: s.replayResetNonce + 1,
        broker: snap.broker ?? s.broker,
      }))
    } catch (err) {
      get().pushToast('error', String(err))
    }
  },

  setReplaySpeed: async (mult) => {
    const clamped = await api.replaySetSpeed(mult)
    set((s) => (s.replay ? { replay: { ...s.replay, speedMultiplier: clamped } } : s))
  },

  setReplayPlaying: (replayPlaying) => set({ replayPlaying }),

  stopReplay: async () => {
    await api.replayStop()
    set((s) => ({
      replay: null,
      replaySelectingCut: false,
      replayPlaying: false,
      replayResetNonce: s.replayResetNonce + 1,
    }))
    await get().refreshBroker()
    get().pushToast('info', 'Đã thoát Bar Replay — khôi phục dữ liệu đầy đủ')
  },

  // ─── Paper Broker ──────────────────────────────────────────────────────────

  refreshBroker: async () => {
    const activePane = get().panes.find((p) => p.slotId === get().activeSlotId) ?? get().panes[0]
    try {
      const snap = await api.brokerGetState(activePane.ticker, activePane.interval)
      set({ broker: snap })
    } catch {
      // ignore if chart has no cached bars yet
    }
  },

  setBrokerSnapshot: (broker) => set({ broker }),

  pushToast: (kind, message) => {
    const id = `toast_${Date.now()}_${Math.random().toString(36).slice(2, 6)}`
    set((s) => ({ toasts: [...s.toasts.slice(-3), { id, kind, message }] }))
    if (typeof window !== 'undefined') {
      window.setTimeout(() => get().dismissToast(id), 4200)
    }
  },

  dismissToast: (id) => set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) })),
}))
