import { create } from 'zustand'
import type {
  BacktestChartOverlayState,
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

const SETTINGS_SCHEMA_VERSION = 2

export const INDICATOR_COLOR_PALETTE = [
  '#2962FF',
  '#FF9800',
  '#E040FB',
  '#00BCD4',
  '#FBC02D',
  '#26A69A',
  '#EF5350',
  '#AB47BC',
] as const

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

export const DEFAULT_SYNC: SyncOptions = {
  symbol: true,
  interval: false,
  crosshair: true,
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
      { id: 'ind_vol_0', name: 'VOL', paneId: 'pane_vol_0', isMain: false, calcParams: [5, 10, 20], visible: true },
      {
        id: 'ind_ema20_0',
        name: 'EMA',
        paneId: 'candle_pane',
        isMain: true,
        calcParams: [20],
        visible: true,
        styles: { lines: [{ color: '#2962FF', size: 1.6 }] },
      },
      {
        id: 'ind_ema50_0',
        name: 'EMA',
        paneId: 'candle_pane',
        isMain: true,
        calcParams: [50],
        visible: true,
        styles: { lines: [{ color: '#FF9800', size: 1.6 }] },
      },
    ],
  },
  {
    slotId: 'pane-1',
    ticker: 'HOSE:FPT',
    interval: '4h',
    chartStyle: 'candle_solid',
    priceScale: 'normal',
    showVnBands: true,
    showCountdown: true,
    indicators: [
      { id: 'ind_vol_1', name: 'VOL', paneId: 'pane_vol_1', isMain: false, calcParams: [5, 10, 20], visible: true },
      {
        id: 'ind_ema20_1',
        name: 'EMA',
        paneId: 'candle_pane',
        isMain: true,
        calcParams: [20],
        visible: true,
        styles: { lines: [{ color: '#2962FF', size: 1.5 }] },
      },
    ],
  },
  {
    slotId: 'pane-2',
    ticker: 'HOSE:FPT',
    interval: '1h',
    chartStyle: 'candle_solid',
    priceScale: 'normal',
    showVnBands: true,
    showCountdown: true,
    indicators: [
      { id: 'ind_vol_2', name: 'VOL', paneId: 'pane_vol_2', isMain: false, calcParams: [5, 10, 20], visible: true },
      {
        id: 'ind_ema20_2',
        name: 'EMA',
        paneId: 'candle_pane',
        isMain: true,
        calcParams: [20],
        visible: true,
        styles: { lines: [{ color: '#2962FF', size: 1.5 }] },
      },
    ],
  },
  {
    slotId: 'pane-3',
    ticker: 'HOSE:FPT',
    interval: '15m',
    chartStyle: 'candle_solid',
    priceScale: 'normal',
    showVnBands: true,
    showCountdown: true,
    indicators: [
      { id: 'ind_vol_3', name: 'VOL', paneId: 'pane_vol_3', isMain: false, calcParams: [5, 10, 20], visible: true },
      {
        id: 'ind_ema20_3',
        name: 'EMA',
        paneId: 'candle_pane',
        isMain: true,
        calcParams: [20],
        visible: true,
        styles: { lines: [{ color: '#2962FF', size: 1.5 }] },
      },
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

  // Jump-to-timestamp command & Strategy Backtest chart overlay
  scrollToTimestampCmd: { slotId: string; timestampMs: number; nonce: number } | null
  backtestOverlay: BacktestChartOverlayState | null

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
  editingIndicator: { slotId: string; indicatorId: string } | null

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

  addIndicatorToActivePane: (spec: Omit<SavedIndicatorSpec, 'id'>, slotId?: string) => void
  updateIndicatorInActivePane: (id: string, patch: Partial<SavedIndicatorSpec>, slotId?: string) => void
  removeIndicatorFromActivePane: (id: string, slotId?: string) => void
  openIndicatorSettings: (slotId: string, indicatorId: string) => void
  closeIndicatorSettings: () => void

  setActiveDrawingTool: (tool: string | null) => void
  setMagnetMode: (mode: MagnetMode) => void
  toggleDrawingsLocked: () => void
  toggleDrawingsHidden: () => void
  triggerClearDrawings: () => void
  triggerExportPng: () => void

  scrollActiveChartToTimestamp: (timestampMs: number) => void
  setBacktestOverlay: (overlay: BacktestChartOverlayState | null) => void
  toggleBacktestOverlayVisible: () => void
  submitQuickLimitOrder: (
    ticker: string,
    interval: IntervalKey,
    side: 'BUY' | 'SELL',
    price: number,
  ) => Promise<void>

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
      settingsVersion: SETTINGS_SCHEMA_VERSION,
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
  sync: DEFAULT_SYNC,
  activeSlotId: 'pane-0',
  panes: DEFAULT_PANES,

  activeDrawingTool: null,
  magnetMode: 'weak_magnet',
  drawingsLocked: false,
  drawingsHidden: false,
  clearDrawingsNonce: 0,
  exportPngNonce: 0,

  scrollToTimestampCmd: null,
  backtestOverlay: null,

  rightPanelOpen: true,
  rightPanelWidth: 288,
  bottomDockOpen: true,
  bottomDockMaximized: false,
  bottomDockHeight: 276,
  bottomDockTab: 'paper',

  symbolSearchOpen: false,
  symbolSearchInitialQuery: '',
  indicatorsModalOpen: false,
  editingIndicator: null,

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

      const savedVersion = typeof saved.settingsVersion === 'number' ? saved.settingsVersion : 1
      let panes: ChartPaneConfig[] =
        Array.isArray(saved.panes) && saved.panes.length === 4
          ? (saved.panes as ChartPaneConfig[])
          : DEFAULT_PANES

      // Migrate v2.0.0 workspace settings to v2.1.0 Multi-Timeframe TradingView defaults:
      // sync.symbol = true, sync.interval = false, and multi-timeframe intervals (1D / 4h / 1h / 15m).
      let sync: SyncOptions =
        savedVersion >= SETTINGS_SCHEMA_VERSION && saved.sync
          ? (saved.sync as SyncOptions)
          : DEFAULT_SYNC

      if (savedVersion < SETTINGS_SCHEMA_VERSION) {
        const primaryTicker = panes[0]?.ticker ?? 'HOSE:FPT'
        const isVnPrimary =
          primaryTicker.startsWith('HOSE:') ||
          primaryTicker.startsWith('HNX:') ||
          primaryTicker.startsWith('UPCOM:')
        const mtfIntervals: IntervalKey[] = ['1D', '4h', '1h', '15m']
        panes = panes.map((p, idx) => ({
          ...p,
          ticker: primaryTicker,
          interval: mtfIntervals[idx] ?? p.interval,
          showVnBands: isVnPrimary,
        }))
        sync = DEFAULT_SYNC
      }

      const watchlists = boot.watchlists
      const activeWatchlistId =
        typeof saved.activeWatchlistId === 'string' && watchlists.some((w) => w.id === saved.activeWatchlistId)
          ? saved.activeWatchlistId
          : (watchlists[0]?.id ?? 'wl_vn30')

      set({
        initialized: true,
        theme,
        gridLayout: (saved.gridLayout as GridLayoutKind) ?? '1',
        sync,
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

      if (savedVersion < SETTINGS_SCHEMA_VERSION) {
        schedulePersistSettings(get())
      }

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
    const visibleCount =
      gridLayout === '1' ? 1 : gridLayout === '2v' || gridLayout === '2h' ? 2 : gridLayout === '3' ? 3 : 4
    const curActive = get().activeSlotId
    const visibleIds = get().panes.slice(0, visibleCount).map((p) => p.slotId)
    const nextActive = visibleIds.includes(curActive) ? curActive : (visibleIds[0] ?? 'pane-0')
    set({ gridLayout, activeSlotId: nextActive })
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
    if (get().replay?.active && targetSlot === get().activeSlotId) {
      void get().stopReplay()
    }
    set((s) => ({
      replaySelectingCut: false,
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
    if (get().replay?.active && targetSlot === get().activeSlotId) {
      void get().stopReplay()
    }
    set((s) => ({
      replaySelectingCut: false,
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

  addIndicatorToActivePane: (spec, slotId) => {
    const slot = slotId ?? get().activeSlotId
    const targetPane = get().panes.find((p) => p.slotId === slot)
    const sameCount = targetPane ? targetPane.indicators.filter((i) => i.name === spec.name).length : 0
    const nextColor = INDICATOR_COLOR_PALETTE[sameCount % INDICATOR_COLOR_PALETTE.length]

    // Auto-assign distinct line colors & sensible next period when adding another MA/EMA/SMA instance!
    let calcParams = [...spec.calcParams]
    if ((spec.name === 'EMA' || spec.name === 'MA') && sameCount > 0) {
      const presetPeriods = [20, 50, 200, 10, 100]
      calcParams = [presetPeriods[sameCount % presetPeriods.length]]
    }

    const styles =
      spec.styles ??
      (spec.isMain
        ? {
            lines: (calcParams.length > 0 ? calcParams : [1]).map((_, idx) => ({
              color: INDICATOR_COLOR_PALETTE[(sameCount + idx) % INDICATOR_COLOR_PALETTE.length],
              size: 1.6,
            })),
          }
        : {
            lines: [
              { color: nextColor, size: 1.5 },
              { color: INDICATOR_COLOR_PALETTE[(sameCount + 1) % INDICATOR_COLOR_PALETTE.length], size: 1.5 },
              { color: INDICATOR_COLOR_PALETTE[(sameCount + 2) % INDICATOR_COLOR_PALETTE.length], size: 1.5 },
            ],
          })

    const newItem: SavedIndicatorSpec = {
      ...spec,
      calcParams,
      styles,
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

  updateIndicatorInActivePane: (id, patch, slotId) => {
    const slot = slotId ?? get().activeSlotId
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

  removeIndicatorFromActivePane: (id, slotId) => {
    const slot = slotId ?? get().activeSlotId
    set((s) => ({
      panes: s.panes.map((p) =>
        p.slotId === slot ? { ...p, indicators: p.indicators.filter((ind) => ind.id !== id) } : p,
      ),
      editingIndicator:
        s.editingIndicator?.slotId === slot && s.editingIndicator?.indicatorId === id
          ? null
          : s.editingIndicator,
    }))
    const pane = get().panes.find((p) => p.slotId === slot)
    if (pane) {
      void api.savePaneIndicators(slot, pane.indicators)
    }
    schedulePersistSettings(get())
  },

  openIndicatorSettings: (slotId, indicatorId) =>
    set({ editingIndicator: { slotId, indicatorId } }),
  closeIndicatorSettings: () => set({ editingIndicator: null }),

  setActiveDrawingTool: (activeDrawingTool) => set({ activeDrawingTool }),
  setMagnetMode: (magnetMode) => set({ magnetMode }),
  toggleDrawingsLocked: () => set((s) => ({ drawingsLocked: !s.drawingsLocked })),
  toggleDrawingsHidden: () => set((s) => ({ drawingsHidden: !s.drawingsHidden })),
  triggerClearDrawings: () => set((s) => ({ clearDrawingsNonce: s.clearDrawingsNonce + 1 })),
  triggerExportPng: () => set((s) => ({ exportPngNonce: s.exportPngNonce + 1 })),

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

  setBacktestOverlay: (backtestOverlay) => set({ backtestOverlay }),
  toggleBacktestOverlayVisible: () =>
    set((s) =>
      s.backtestOverlay
        ? { backtestOverlay: { ...s.backtestOverlay, visible: !s.backtestOverlay.visible } }
        : s,
    ),

  submitQuickLimitOrder: async (ticker, interval, side, price) => {
    const isVn = ticker.startsWith('HOSE:') || ticker.startsWith('HNX:') || ticker.startsWith('UPCOM:')
    const tick = isVn ? (price >= 50000 ? 100 : price >= 10000 ? 50 : 10) : 0.01
    const snapped = isVn ? Math.round(price / tick) * tick : Number(price.toFixed(2))
    const qty = isVn ? 1000 : 1
    try {
      const snap = await api.brokerSubmitOrder({
        ticker,
        interval,
        side,
        vnOrderType: 'LO',
        quantity: qty,
        price: snapped,
      })
      set({ broker: snap, bottomDockOpen: true, bottomDockTab: 'paper' })
      get().pushToast(
        'success',
        `Đã đặt lệnh LO ${side === 'BUY' ? 'MUA' : 'BÁN'} ${qty.toLocaleString('en-US')} ${ticker} @ ${snapped.toLocaleString('en-US')}`,
      )
    } catch (err) {
      get().pushToast('error', String(err))
    }
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
