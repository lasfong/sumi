import { create } from 'zustand'
import type {
  BacktestChartOverlayState,
  BottomDockTab,
  BrokerStateSnapshot,
  ChartPaneConfig,
  ChartStyleKind,
  DataStatusResponse,
  DeepLinkPayload,
  ExportedVerdictListItem,
  ExportedVerdictReceipt,
  GridLayoutKind,
  IndicatorTemplateRecord,
  IntervalKey,
  MagnetMode,
  OhlcvBar,
  PriceAlertRecord,
  PriceScaleKind,
  QmvRealChartSeriesDto,
  ReplaySnapshotResponse,
  SavedIndicatorSpec,
  SavedLayoutRecord,
  SavedReplaySessionRecord,
  SyncProgressPayload,
  SyncStatusDto,
  ThemeMode,
  TopDownSymbolContextDto,
  WatchlistGroup,
  WatchlistQuoteSnapshot,
  YumekoBridgeStatus,
} from '../types/market'
import * as api from '../api/tauri'
import { setQmvRealSeriesCache } from '../chart/indicators/qmvRealIndicators'
import { notifyAlertTriggered } from '../utils/alertSound'
import { snapPriceToVenueTick } from '../utils/vnMarket'

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

export type TimezoneMode = 'auto' | 'Asia/Ho_Chi_Minh' | 'Etc/UTC'

export interface SyncOptions {
  symbol: boolean
  interval: boolean
  crosshair: boolean
  dateRange: boolean
  drawings: boolean
}

export interface ToastNotice {
  id: string
  kind: 'info' | 'success' | 'warning' | 'error'
  message: string
}

export interface TradePlannerDraft {
  ticker?: string
  side?: 'LONG' | 'SHORT'
  entryPrice?: number
  stopLoss?: number
  targetPrice?: number
}

export const DEFAULT_SYNC: SyncOptions = {
  symbol: true,
  interval: false,
  crosshair: true,
  dateRange: false,
  drawings: true,
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
      { id: 'ind_vol_0', name: 'VOL', paneId: 'pane_vol_0', isMain: false, calcParams: [5, 20], visible: true },
      {
        id: 'ind_ma_qmv_0',
        name: 'MA',
        paneId: 'candle_pane',
        isMain: true,
        calcParams: [20, 50, 200],
        visible: true,
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
      { id: 'ind_vol_1', name: 'VOL', paneId: 'pane_vol_1', isMain: false, calcParams: [5, 20], visible: true },
      {
        id: 'ind_ma_qmv_1',
        name: 'MA',
        paneId: 'candle_pane',
        isMain: true,
        calcParams: [20, 50],
        visible: true,
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
      { id: 'ind_vol_2', name: 'VOL', paneId: 'pane_vol_2', isMain: false, calcParams: [5, 20], visible: true },
      {
        id: 'ind_ma_qmv_2',
        name: 'MA',
        paneId: 'candle_pane',
        isMain: true,
        calcParams: [20, 50],
        visible: true,
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
      { id: 'ind_vol_3', name: 'VOL', paneId: 'pane_vol_3', isMain: false, calcParams: [5, 20], visible: true },
      {
        id: 'ind_ma_qmv_3',
        name: 'MA',
        paneId: 'candle_pane',
        isMain: true,
        calcParams: [20, 50],
        visible: true,
      },
    ],
  },
]

export interface WorkspaceState {
  initialized: boolean
  theme: ThemeMode
  timezoneMode: TimezoneMode
  gridLayout: GridLayoutKind
  sync: SyncOptions
  activeSlotId: string
  panes: ChartPaneConfig[]

  // Drawing rail state
  activeDrawingTool: string | null
  magnetMode: MagnetMode
  drawingsLocked: boolean
  drawingsHidden: boolean
  showVnSessionBreaks: boolean
  drawingsVersion: number
  clearDrawingsNonce: number
  exportPngNonce: number

  // Jump-to-timestamp command & Strategy Backtest chart overlay
  scrollToTimestampCmd: { slotId: string; timestampMs: number; nonce: number } | null
  backtestOverlay: BacktestChartOverlayState | null

  // Panels layout
  rightPanelOpen: boolean
  rightPanelWidth: number
  rightTab: 'watchlist' | 'alerts' | 'objects' | 'yumeko'
  bottomDockOpen: boolean
  bottomDockMaximized: boolean
  bottomDockHeight: number
  bottomDockTab: BottomDockTab

  // Yumeko v3.0 Read-Only Top-Down Bridge & KT01 Evidence (Wave 2)
  yumekoStatus: YumekoBridgeStatus | null
  yumekoAsOf: string | null
  yumekoContext: TopDownSymbolContextDto | null
  yumekoQmvSeries: QmvRealChartSeriesDto | null
  yumekoSeriesVersion: number
  yumekoLoading: boolean
  exportingKt01: boolean
  exportedVerdicts: ExportedVerdictListItem[]
  lastExportedReceipt: ExportedVerdictReceipt | null

  // Modals
  symbolSearchOpen: boolean
  symbolSearchInitialQuery: string
  indicatorsModalOpen: boolean
  editingIndicator: { slotId: string; indicatorId: string } | null
  tradePlannerOpen: boolean
  tradePlannerDraft: TradePlannerDraft | null
  debriefModalOpen: boolean

  // Watchlists, live quotes, alerts, layouts, data status
  watchlists: WatchlistGroup[]
  activeWatchlistId: string
  quotesMap: Record<string, WatchlistQuoteSnapshot>
  quoteFlashMap: Record<string, { dir: 'up' | 'down'; nonce: number }>
  alerts: PriceAlertRecord[]
  layouts: SavedLayoutRecord[]
  indicatorTemplates: IndicatorTemplateRecord[]
  replaySessions: SavedReplaySessionRecord[]
  dataStatus: DataStatusResponse | null
  syncStatus: SyncStatusDto | null
  importingCafeF: boolean
  syncingCatalog: boolean
  syncingEod: boolean
  eodSyncProgress: SyncProgressPayload | null

  // Causal Bar Replay state
  replay: ReplaySnapshotResponse | null
  replaySelectingCut: boolean
  replayPlaying: boolean
  replayBlindMode: boolean
  replayResetNonce: number
  replaySteppedBar: { bar: ReplaySnapshotResponse['steppedBar']; nonce: number } | null
  replayStepEvent: { ticker: string; barsByInterval: Record<string, OhlcvBar>; nonce: number } | null

  // Paper Broker state (synced with active chart & replay)
  broker: BrokerStateSnapshot | null

  // Toast notifications
  toasts: ToastNotice[]

  // Actions
  bootstrap: () => Promise<void>
  setTheme: (theme: ThemeMode) => void
  toggleTheme: () => void
  setTimezoneMode: (tz: TimezoneMode) => void
  setGridLayout: (layout: GridLayoutKind) => void
  setSyncOption: (key: keyof SyncOptions, value: boolean) => void
  setActiveSlotId: (slotId: string) => void

  setActiveTicker: (ticker: string, slotId?: string) => void
  setActiveInterval: (interval: IntervalKey, slotId?: string) => void
  setActiveChartStyle: (style: ChartStyleKind) => void
  setActivePriceScale: (scale: PriceScaleKind) => void
  togglePaneAdjustment: (slotId?: string) => void
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
  toggleVnSessionBreaks: () => void
  bumpDrawingsVersion: () => void
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
  submitQuickBracket: (kind: 'sl' | 'tp', rawPrice: number) => Promise<void>

  setRightPanelOpen: (open: boolean) => void
  setRightPanelWidth: (width: number) => void
  setRightTab: (tab: 'watchlist' | 'alerts' | 'objects' | 'yumeko') => void
  setBottomDockOpen: (open: boolean) => void
  setBottomDockMaximized: (max: boolean) => void
  setBottomDockHeight: (height: number) => void
  setBottomDockTab: (tab: BottomDockTab) => void

  // Yumeko Top-Down Bridge & Deep-Link actions (Wave 2)
  refreshYumekoStatus: () => Promise<void>
  setYumekoAsOf: (asOf: string | null) => void
  loadYumekoForActiveTicker: () => Promise<void>
  exportActiveKt01Verdict: (copyToYumekoDropzone?: boolean) => Promise<ExportedVerdictReceipt | null>
  refreshExportedVerdicts: () => Promise<void>
  applyDeepLink: (payload: DeepLinkPayload) => void

  openSymbolSearch: (initialQuery?: string) => void
  closeSymbolSearch: () => void
  setIndicatorsModalOpen: (open: boolean) => void
  openTradePlanner: (draft?: TradePlannerDraft) => void
  closeTradePlanner: () => void
  setDebriefModalOpen: (open: boolean) => void

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
    open?: number | null
    high?: number | null
    low?: number | null
    bid?: number | null
    ask?: number | null
    timestamp_ms: number
  }) => void
  addSymbolToWatchlist: (ticker: string) => Promise<void>
  removeSymbolFromWatchlist: (ticker: string) => Promise<void>
  createNewWatchlist: (name: string) => Promise<void>

  createNewAlert: (
    ticker: string,
    condition: string,
    targetPrice: number,
    message: string,
    opts?: {
      alertKind?: 'price' | 'drawing' | 'indicator'
      interval?: IntervalKey | null
      paramsJson?: Record<string, unknown> | null
      soundEnabled?: boolean
      frequency?: 'once' | 'once_per_bar_close'
      cooldownBars?: number
    },
  ) => Promise<void>
  updateExistingAlert: (params: {
    id: string
    condition?: string
    targetPrice?: number
    message?: string
    frequency?: 'once' | 'once_per_bar_close'
    cooldownBars?: number
    rearm?: boolean
  }) => Promise<void>
  toggleAlert: (id: string, status: string) => Promise<void>
  removeAlert: (id: string) => Promise<void>
  checkLiveBarAlerts: (ticker: string, interval: IntervalKey) => Promise<void>
  scanAllWatchlistAlerts: (silent?: boolean) => Promise<PriceAlertRecord[]>

  saveCurrentIndicatorTemplate: (name: string, description?: string) => Promise<void>
  applyIndicatorTemplate: (tpl: IndicatorTemplateRecord) => void
  deleteIndicatorTemplate: (id: string) => Promise<void>

  saveCurrentLayout: (name: string) => Promise<void>
  applySavedLayout: (layout: SavedLayoutRecord) => void
  deleteSavedLayout: (id: string) => Promise<void>

  toggleOfflineMode: () => Promise<void>
  importCafeFDirectory: (dir: string, includeRaw?: boolean) => Promise<void>
  refreshSyncStatus: () => Promise<void>
  syncVnUniverseCatalog: () => Promise<void>
  syncUniverseEod: (scope?: string, maxSymbols?: number) => Promise<void>

  // Replay actions
  setReplaySelectingCut: (selecting: boolean) => void
  setReplayBlindMode: (blind: boolean) => void
  startReplay: (cutTimestampMs?: number | null) => Promise<void>
  stepReplay: (delta: number) => Promise<void>
  jumpReplay: (targetTimestampMs: number) => Promise<void>
  setReplaySpeed: (mult: number) => Promise<void>
  setReplayPlaying: (playing: boolean) => void
  stopReplay: () => Promise<void>
  saveCurrentReplaySession: (name: string) => Promise<void>
  resumeSavedReplaySession: (id: string) => Promise<void>
  deleteSavedReplaySession: (id: string) => Promise<void>
  applyRemoteReplaySync: (payload: {
    action: 'start' | 'step' | 'jump' | 'stop'
    snapshot?: ReplaySnapshotResponse | null
  }) => void

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
const lastFlashTsByTicker = new Map<string, number>()

function schedulePersistSettings(state: WorkspaceState): void {
  if (typeof window === 'undefined') return
  // Detached auxiliary monitor windows don't overwrite the main workstation layout
  if (window.location.search.includes('detached=1')) return
  if (persistTimer !== null) {
    window.clearTimeout(persistTimer)
  }
  persistTimer = window.setTimeout(() => {
    const payload = {
      settingsVersion: SETTINGS_SCHEMA_VERSION,
      theme: state.theme,
      timezoneMode: state.timezoneMode,
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
      showVnSessionBreaks: state.showVnSessionBreaks,
    }
    void api.saveWorkspaceSettings(payload)
  }, 350)
}

let inFlightBrokerKey: string | null = null
let inFlightBrokerPromise: Promise<void> | null = null

export const useWorkspaceStore = create<WorkspaceState>((set, get) => ({
  initialized: false,
  theme: 'dark',
  timezoneMode: 'auto',
  gridLayout: '1',
  sync: DEFAULT_SYNC,
  activeSlotId: 'pane-0',
  panes: DEFAULT_PANES,

  activeDrawingTool: null,
  magnetMode: 'weak_magnet',
  drawingsLocked: false,
  drawingsHidden: false,
  showVnSessionBreaks: true,
  drawingsVersion: 0,
  clearDrawingsNonce: 0,
  exportPngNonce: 0,

  scrollToTimestampCmd: null,
  backtestOverlay: null,

  rightPanelOpen: true,
  rightPanelWidth: 288,
  rightTab: 'watchlist',
  bottomDockOpen: true,
  bottomDockMaximized: false,
  bottomDockHeight: 276,
  bottomDockTab: 'paper',

  yumekoStatus: null,
  yumekoAsOf: null,
  yumekoContext: null,
  yumekoQmvSeries: null,
  yumekoSeriesVersion: 0,
  yumekoLoading: false,
  exportingKt01: false,
  exportedVerdicts: [],
  lastExportedReceipt: null,

  symbolSearchOpen: false,
  symbolSearchInitialQuery: '',
  indicatorsModalOpen: false,
  editingIndicator: null,
  tradePlannerOpen: false,
  tradePlannerDraft: null,
  debriefModalOpen: false,

  watchlists: [],
  activeWatchlistId: 'wl_qmv30',
  quotesMap: {},
  quoteFlashMap: {},
  alerts: [],
  layouts: [],
  indicatorTemplates: [],
  replaySessions: [],
  dataStatus: null,
  syncStatus: null,
  importingCafeF: false,
  syncingCatalog: false,
  syncingEod: false,
  eodSyncProgress: null,

  replay: null,
  replaySelectingCut: false,
  replayPlaying: false,
  replayBlindMode: true,
  replayResetNonce: 0,
  replaySteppedBar: null,
  replayStepEvent: null,

  broker: null,
  toasts: [],

  bootstrap: async () => {
    try {
      const [boot, status, activeReplay, syncStatus] = await Promise.all([
        api.loadWorkspaceBootstrap(),
        api.getDataStatus(),
        api.replayGetState().catch(() => null),
        api.syncGetStatus().catch(() => null),
      ])
      const saved = boot.settings ?? {}
      const theme: ThemeMode = saved.theme === 'light' ? 'light' : 'dark'
      applyThemeToDocument(theme)
      const timezoneMode: TimezoneMode =
        saved.timezoneMode === 'Asia/Ho_Chi_Minh' || saved.timezoneMode === 'Etc/UTC'
          ? saved.timezoneMode
          : 'auto'

      const savedVersion = typeof saved.settingsVersion === 'number' ? saved.settingsVersion : 1
      let panes: ChartPaneConfig[] =
        Array.isArray(saved.panes) && saved.panes.length === 4
          ? (saved.panes as ChartPaneConfig[])
          : DEFAULT_PANES

      // Hydrate per-pane indicators from dedicated `pane_indicators` table if saved (`BUG-038`)
      const hydratedInds = await Promise.all(
        panes.map((p) => api.getPaneIndicators(p.slotId).catch(() => [])),
      )
      panes = panes.map((p, idx) =>
        hydratedInds[idx] && hydratedInds[idx].length > 0 ? { ...p, indicators: hydratedInds[idx] } : p,
      )

      // Migrate v2.0.0 workspace settings to v2.1.0 Multi-Timeframe TradingView defaults:
      // sync.symbol = true, sync.interval = false, and multi-timeframe intervals (1D / 4h / 1h / 15m).
      let sync: SyncOptions =
        savedVersion >= SETTINGS_SCHEMA_VERSION && saved.sync
          ? { ...DEFAULT_SYNC, ...(saved.sync as Partial<SyncOptions>) }
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

      // Support detached auxiliary OS window (`?detached=1&ticker=...&interval=...`, `BUG-043`)
      let gridLayout: GridLayoutKind = (saved.gridLayout as GridLayoutKind) ?? '1'
      let rightPanelOpen = typeof saved.rightPanelOpen === 'boolean' ? saved.rightPanelOpen : true
      let bottomDockOpen = typeof saved.bottomDockOpen === 'boolean' ? saved.bottomDockOpen : true
      if (typeof window !== 'undefined' && window.location.search.includes('detached=1')) {
        const params = new URLSearchParams(window.location.search)
        const dTicker = params.get('ticker')
        const dInterval = params.get('interval') as IntervalKey | null
        gridLayout = '1'
        rightPanelOpen = false
        bottomDockOpen = false
        if (dTicker || dInterval) {
          const t = dTicker || panes[0].ticker
          const isVn = t.startsWith('HOSE:') || t.startsWith('HNX:') || t.startsWith('UPCOM:')
          panes = panes.map((p, idx) =>
            idx === 0
              ? { ...p, ticker: t, interval: dInterval || p.interval, showVnBands: isVn }
              : p,
          )
        }
      }

      const watchlists = boot.watchlists
      const activeWatchlistId =
        typeof saved.activeWatchlistId === 'string' && watchlists.some((w) => w.id === saved.activeWatchlistId)
          ? saved.activeWatchlistId
          : (watchlists[0]?.id ?? 'wl_vn30')

      set({
        initialized: true,
        theme,
        timezoneMode,
        gridLayout,
        sync,
        activeSlotId: typeof saved.activeSlotId === 'string' ? saved.activeSlotId : 'pane-0',
        panes,
        rightPanelOpen,
        rightPanelWidth: typeof saved.rightPanelWidth === 'number' ? saved.rightPanelWidth : 288,
        bottomDockOpen,
        bottomDockHeight: typeof saved.bottomDockHeight === 'number' ? saved.bottomDockHeight : 276,
        bottomDockTab: (saved.bottomDockTab as BottomDockTab) ?? 'paper',
        watchlists,
        activeWatchlistId,
        alerts: boot.alerts,
        layouts: boot.layouts,
        indicatorTemplates: boot.indicatorTemplates ?? [],
        replaySessions: boot.replaySessions ?? [],
        showVnSessionBreaks:
          typeof saved.showVnSessionBreaks === 'boolean' ? saved.showVnSessionBreaks : true,
        dataStatus: status,
        syncStatus,
        replay: activeReplay?.active ? activeReplay : null,
      })

      if (savedVersion < SETTINGS_SCHEMA_VERSION) {
        schedulePersistSettings(get())
      }

      await Promise.all([
        get().refreshWatchlistQuotes(),
        get().refreshBroker(),
        get().refreshYumekoStatus(),
        get().loadYumekoForActiveTicker(),
        get().refreshExportedVerdicts(),
      ])
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

  setTimezoneMode: (timezoneMode) => {
    set({ timezoneMode })
    schedulePersistSettings(get())
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
    const prev = get().activeSlotId
    if (prev === activeSlotId) return
    set({ activeSlotId })
    schedulePersistSettings(get())
    const nextPane = get().panes.find((p) => p.slotId === activeSlotId)
    const activeRep = get().replay
    // During Multi-Timeframe Replay on the same ticker, clicking Pane 2 (e.g. `4h` while replaying `1h`)
    // MUST NOT clobber or desync the isolated Replay Broker (`BUG-040`)!
    if (activeRep?.active && nextPane && nextPane.ticker.toUpperCase() === activeRep.ticker.toUpperCase()) {
      return
    }
    void get().refreshBroker()
    void get().loadYumekoForActiveTicker()
  },

  setActiveTicker: (ticker, slotId) => {
    const targetSlot = slotId ?? get().activeSlotId
    const syncSymbol = get().sync.symbol
    const isVn = ticker.startsWith('HOSE:') || ticker.startsWith('HNX:') || ticker.startsWith('UPCOM:')
    const activeRep = get().replay
    if (activeRep?.active && activeRep.ticker.toUpperCase() !== ticker.toUpperCase()) {
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
    if (syncSymbol) {
      void api.broadcastSyncEvent('sumi:sync_symbol', { ticker })
    }
    void get().refreshBroker()
    void get().loadYumekoForActiveTicker()
  },

  setActiveInterval: (interval, slotId) => {
    const targetSlot = slotId ?? get().activeSlotId
    const syncIv = get().sync.interval
    const activeRep = get().replay
    const targetPane = get().panes.find((p) => p.slotId === targetSlot)
    // Multi-Timeframe Bar Replay (`BUG-040`): changing timeframe on a pane sharing `replay.ticker`
    // MUST keep Bar Replay active and causally aggregate via `get_bars` instead of stopping Replay!
    const keepReplayAlive = Boolean(
      activeRep?.active &&
        targetPane &&
        targetPane.ticker.toUpperCase() === activeRep.ticker.toUpperCase(),
    )
    set((s) => ({
      replaySelectingCut: false,
      panes: s.panes.map((p) => (syncIv || p.slotId === targetSlot ? { ...p, interval } : p)),
    }))
    schedulePersistSettings(get())
    if (!keepReplayAlive) {
      void get().refreshBroker()
    }
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

  togglePaneAdjustment: (slotId) => {
    const slot = slotId ?? get().activeSlotId
    set((s) => ({
      panes: s.panes.map((p) =>
        p.slotId === slot
          ? { ...p, adjustment: (p.adjustment ?? 'splits') === 'splits' ? 'unadjusted' : 'splits' }
          : p,
      ),
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
  toggleVnSessionBreaks: () => {
    set((s) => ({ showVnSessionBreaks: !s.showVnSessionBreaks }))
    schedulePersistSettings(get())
  },
  bumpDrawingsVersion: () => set((s) => ({ drawingsVersion: s.drawingsVersion + 1 })),
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
    const upper = ticker.toUpperCase()
    const isVn30f = upper.includes('VN30F') || upper.includes('VN301!')
    const isHose = upper.startsWith('HOSE:')
    const isHnxOrUpcom = (upper.startsWith('HNX:') || upper.startsWith('UPCOM:')) && !isVn30f
    const isCrypto = upper.startsWith('BINANCE:') || upper.startsWith('OKX:') || upper.startsWith('BYBIT:')

    // Exact per-venue statutory tick & default order quantity (`BUG-008`)!
    let tick = 0.01
    let qty = 1
    let decimals = 2
    if (isVn30f) {
      tick = 0.1
      qty = 1
      decimals = 1
    } else if (isHnxOrUpcom) {
      tick = 100
      qty = 100
      decimals = 0
    } else if (isHose) {
      tick = price >= 50000 ? 100 : price >= 10000 ? 50 : 10
      qty = 100
      decimals = 0
    } else if (isCrypto) {
      tick = price >= 100 ? 0.01 : 0.0001
      qty = price >= 1000 ? 0.01 : 1
      decimals = price >= 100 ? 2 : 4
    }

    const snapped = Number((Math.round(price / tick) * tick).toFixed(decimals))
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

  submitQuickBracket: async (kind, rawPrice) => {
    const state = get()
    const broker = state.broker
    const activePane = state.panes.find((p) => p.slotId === state.activeSlotId) ?? state.panes[0]
    if (!broker?.openPosition) {
      state.pushToast('warning', 'Chưa có vị thế mở để đặt Cắt lỗ (SL) / Chốt lời (TP)')
      return
    }
    const snapped = snapPriceToVenueTick(broker.ticker, rawPrice)
    const nextSl = kind === 'sl' ? snapped : (broker.openPosition.stop_loss ?? null)
    const nextTp = kind === 'tp' ? snapped : (broker.openPosition.take_profit ?? null)
    try {
      const updated = await api.brokerUpdateBrackets(
        broker.ticker,
        (broker.interval as IntervalKey) || activePane.interval,
        nextSl,
        nextTp,
      )
      set({ broker: updated })
      state.pushToast(
        'success',
        `Đã cập nhật ${kind === 'sl' ? 'Cắt lỗ (SL)' : 'Chốt lời (TP)'} @ ${snapped.toLocaleString('en-US')}`,
      )
    } catch (err) {
      state.pushToast('error', String(err))
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
  setRightTab: (rightTab) => set({ rightTab, rightPanelOpen: true }),

  refreshYumekoStatus: async () => {
    try {
      const yumekoStatus = await api.yumekoGetBridgeStatus()
      set({ yumekoStatus })
    } catch {
      // ignore on Clean-Slate machines
    }
  },

  setYumekoAsOf: (asOf) => {
    const clean = asOf && asOf.trim().length === 10 ? asOf.trim() : null
    set({ yumekoAsOf: clean })
    void get().loadYumekoForActiveTicker()
    if (clean) {
      get().pushToast('info', `Đã khóa mốc Point-in-Time as_of = ${clean}`)
    } else {
      get().pushToast('info', 'Đã mở khóa mốc thời gian — hiển thị dữ liệu mới nhất')
    }
  },

  loadYumekoForActiveTicker: async () => {
    const state = get()
    const activePane = state.panes.find((p) => p.slotId === state.activeSlotId) ?? state.panes[0]
    if (!activePane) return
    const ticker = activePane.ticker
    const asOf = state.yumekoAsOf
    set({ yumekoLoading: true })
    try {
      const [ctx, series] = await Promise.all([
        api.yumekoGetTopdownContext(ticker, asOf),
        api.yumekoGetQmvSeries(ticker, asOf),
      ])
      setQmvRealSeriesCache(ticker, series)
      set((s) => ({
        yumekoContext: ctx,
        yumekoQmvSeries: series,
        yumekoSeriesVersion: s.yumekoSeriesVersion + 1,
        yumekoLoading: false,
      }))
    } catch {
      set({ yumekoLoading: false })
    }
  },

  exportActiveKt01Verdict: async (copyToYumekoDropzone = true) => {
    if (get().exportingKt01) return null
    const state = get()
    const activePane = state.panes.find((p) => p.slotId === state.activeSlotId) ?? state.panes[0]
    if (!activePane) return null
    set({ exportingKt01: true })
    try {
      const receipt = await api.yumekoExportKt01Verdict(
        activePane.ticker,
        state.yumekoAsOf,
        copyToYumekoDropzone,
      )
      set({
        exportingKt01: false,
        lastExportedReceipt: receipt,
        rightPanelOpen: true,
        rightTab: 'yumeko',
      })
      void get().refreshExportedVerdicts()
      get().pushToast(
        'success',
        `Đã xuất KT01 (${receipt.verdict.verdict} · ${receipt.verdict.confidenceScore.toFixed(0)}/100) cho ${receipt.verdict.symbol} [SHA-256 ${receipt.verdict.verdictSha256.slice(0, 8)}]`,
      )
      return receipt
    } catch (err) {
      set({ exportingKt01: false })
      get().pushToast('error', `Không thể xuất KT01: ${String(err)}`)
      return null
    }
  },

  refreshExportedVerdicts: async () => {
    try {
      const exportedVerdicts = await api.yumekoListExportedVerdicts(20)
      set({ exportedVerdicts })
    } catch {
      // ignore
    }
  },

  applyDeepLink: (payload) => {
    if (!payload?.ticker) return
    const curSlot = get().activeSlotId
    get().setActiveTicker(payload.ticker, curSlot)
    if (payload.interval) {
      get().setActiveInterval(payload.interval, curSlot)
    }
    if (payload.asOf !== undefined) {
      set({ yumekoAsOf: payload.asOf })
      void get().loadYumekoForActiveTicker()
    }
    if (payload.tab === 'yumeko' || payload.tab === 'kt01') {
      set({ rightPanelOpen: true, rightTab: 'yumeko' })
    } else if (payload.tab === 'screener') {
      set({ bottomDockOpen: true, bottomDockTab: 'screener' })
    } else if (payload.tab === 'backtest' || payload.tab === 'strategy') {
      set({ bottomDockOpen: true, bottomDockTab: 'strategy' })
    } else if (payload.tab === 'paper') {
      set({ bottomDockOpen: true, bottomDockTab: 'paper' })
    } else {
      set({ rightPanelOpen: true, rightTab: 'yumeko' })
    }
    get().pushToast(
      'info',
      `Yumeko Deep-Link -> ${payload.ticker}${payload.interval ? ` (${payload.interval})` : ''}${payload.asOf ? ` · as_of=${payload.asOf}` : ''}`,
    )
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
  openTradePlanner: (draft) => set({ tradePlannerOpen: true, tradePlannerDraft: draft ?? null }),
  closeTradePlanner: () => set({ tradePlannerOpen: false, tradePlannerDraft: null }),
  setDebriefModalOpen: (debriefModalOpen) => set({ debriefModalOpen }),

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
        open: quote.open ?? cur?.open ?? null,
        high: quote.high ?? cur?.high ?? null,
        low: quote.low ?? cur?.low ?? null,
        bid: quote.bid ?? cur?.bid ?? null,
        ask: quote.ask ?? cur?.ask ?? null,
        pricePrecision: cur?.pricePrecision ?? 2,
        vnBand: cur?.vnBand ?? null,
        timestampMs: quote.timestamp_ms,
      }
      const nextFlash = { ...s.quoteFlashMap }
      const nowMs = Date.now()
      const lastFlash = lastFlashTsByTicker.get(quote.ticker) ?? 0
      // Throttle flash triggers to >= 400ms per symbol (<= 2.5Hz, WCAG 2.3.1 safe, `BUG-030`).
      if (flashDir && nowMs - lastFlash >= 400) {
        lastFlashTsByTicker.set(quote.ticker, nowMs)
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
            notifyAlertTriggered(f)
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

  createNewAlert: async (ticker, condition, targetPrice, message, opts) => {
    const rec = await api.createAlert(ticker, condition, targetPrice, message, opts)
    set((s) => ({ alerts: [rec, ...s.alerts] }))
    const kindLabel =
      opts?.alertKind === 'drawing'
        ? 'đường vẽ'
        : opts?.alertKind === 'indicator'
          ? 'chỉ báo / tín hiệu'
          : `@ ${targetPrice.toLocaleString('en-US')}`
    get().pushToast('success', `Đã tạo cảnh báo ${ticker} (${kindLabel})`)
  },

  updateExistingAlert: async (params) => {
    const updated = await api.updateAlert(params)
    set((s) => ({ alerts: s.alerts.map((a) => (a.id === updated.id ? updated : a)) }))
    get().pushToast('success', `Đã cập nhật cảnh báo ${updated.ticker}`)
  },

  toggleAlert: async (id, status) => {
    await api.toggleAlertStatus(id, status)
    set((s) => ({ alerts: s.alerts.map((a) => (a.id === id ? { ...a, status } : a)) }))
  },

  removeAlert: async (id) => {
    await api.deleteAlert(id)
    set((s) => ({ alerts: s.alerts.filter((a) => a.id !== id) }))
  },

  scanAllWatchlistAlerts: async (silent = false) => {
    try {
      const fired = await api.scanWatchlistAlerts()
      if (fired.length > 0) {
        set((s) => ({
          alerts: s.alerts.map((a) => fired.find((f) => f.id === a.id) ?? a),
        }))
        for (const f of fired) {
          notifyAlertTriggered(f)
          get().pushToast('warning', `🔔 Watchlist Alert ${f.ticker}: ${f.message || `Khớp mốc ${f.targetPrice}`}`)
        }
      } else if (!silent) {
        get().pushToast('info', 'Quét xong toàn bộ cảnh báo đang chờ — chưa có mã nào kích hoạt')
      }
      return fired
    } catch (err) {
      if (!silent) get().pushToast('error', `Lỗi quét cảnh báo: ${String(err)}`)
      return []
    }
  },

  checkLiveBarAlerts: async (ticker, interval) => {
    const hasArmed = get().alerts.some(
      (a) =>
        a.status === 'armed' &&
        a.ticker.toUpperCase() === ticker.toUpperCase() &&
        (a.alertKind === 'drawing' || a.alertKind === 'indicator'),
    )
    if (!hasArmed) return
    try {
      const fired = await api.checkBarAlerts(ticker, interval)
      if (fired.length > 0) {
        set((s) => ({
          alerts: s.alerts.map((a) => fired.find((f) => f.id === a.id) ?? a),
        }))
        for (const f of fired) {
          notifyAlertTriggered(f)
          get().pushToast('warning', `🔔 Cảnh báo ${f.ticker}: ${f.message || f.condition}`)
        }
      }
    } catch {
      // ignore transient eval failures
    }
  },

  saveCurrentIndicatorTemplate: async (name, description) => {
    const activePane = get().panes.find((p) => p.slotId === get().activeSlotId) ?? get().panes[0]
    try {
      const saved = await api.saveIndicatorTemplate(name, activePane.indicators, description)
      set((s) => ({
        indicatorTemplates: [saved, ...s.indicatorTemplates.filter((t) => t.id !== saved.id)],
      }))
      get().pushToast('success', `Đã lưu mẫu chỉ báo "${saved.name}"`)
    } catch (err) {
      get().pushToast('error', String(err))
    }
  },


  applyIndicatorTemplate: (tpl) => {
    const slot = get().activeSlotId
    const freshIndicators = tpl.indicators.map((ind, idx) => ({
      ...ind,
      id: `${ind.name.toLowerCase()}_${Date.now()}_${idx}`,
      paneId: ind.isMain ? 'candle_pane' : `pane_${ind.name.toLowerCase()}_${Date.now()}_${idx}`,
    }))
    set((s) => ({
      panes: s.panes.map((p) => (p.slotId === slot ? { ...p, indicators: freshIndicators } : p)),
    }))
    void api.savePaneIndicators(slot, freshIndicators)
    schedulePersistSettings(get())
    get().pushToast('info', `Đã áp dụng mẫu chỉ báo "${tpl.name}" (${freshIndicators.length} chỉ báo)`)
  },

  deleteIndicatorTemplate: async (id) => {
    await api.deleteIndicatorTemplate(id)
    set((s) => ({ indicatorTemplates: s.indicatorTemplates.filter((t) => t.id !== id) }))
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

  importCafeFDirectory: async (dir, includeRaw = true) => {
    const trimmed = dir.trim()
    if (!trimmed || get().importingCafeF) return
    set({ importingCafeF: true })
    try {
      const summary = await api.importCafeFDir(trimmed, includeRaw)
      const status = await api.getDataStatus()
      set({ dataStatus: status, importingCafeF: false })
      await Promise.all([
        get().refreshWatchlistQuotes(),
        get().refreshBroker(),
        get().refreshYumekoStatus(),
        get().loadYumekoForActiveTicker(),
        get().refreshExportedVerdicts(),
      ])
      get().pushToast(
        'success',
        `Đã nạp ${summary.distinct_symbols.toLocaleString('en-US')} mã (${summary.total_bars_written.toLocaleString('en-US')} nến${includeRaw ? ', kèm RAW' : ''}) vào SQLite trong ${summary.elapsed_ms}ms`,
      )
    } catch (err) {
      set({ importingCafeF: false })
      get().pushToast('error', `Không thể nạp thư mục CafeF: ${String(err)}`)
    }
  },

  refreshSyncStatus: async () => {
    try {
      const syncStatus = await api.syncGetStatus()
      set({ syncStatus })
    } catch {
      // ignore
    }
  },

  syncVnUniverseCatalog: async () => {
    if (get().syncingCatalog) return
    set({ syncingCatalog: true })
    try {
      const res = await api.syncVnUniverseCatalog()
      const status = await api.getDataStatus()
      set({ dataStatus: status, syncingCatalog: false })
      get().pushToast(
        'success',
        `Đã đồng bộ danh mục ${res.discoveredSymbols.toLocaleString('en-US')} mã VN (cập nhật ${res.upsertedSymbols.toLocaleString('en-US')} mã trong ${res.elapsedMs}ms)`,
      )
    } catch (err) {
      set({ syncingCatalog: false })
      get().pushToast('error', `Lỗi đồng bộ danh mục VN: ${String(err)}`)
    }
  },

  syncUniverseEod: async (scope = 'ALL_VN', maxSymbols = 150) => {
    if (get().syncingEod) return
    set({
      syncingEod: true,
      eodSyncProgress: {
        scope,
        completed: 0,
        total: 0,
        currentTicker: '',
        updatedSymbols: 0,
        updatedBars: 0,
        failedSymbols: 0,
        done: false,
      },
    })
    try {
      const wl = get().watchlists.find((w) => w.id === get().activeWatchlistId)
      const summary = await api.syncUniverseEod(
        {
          scope,
          tickers: scope === 'WATCHLIST' ? (wl?.tickers ?? []) : undefined,
          maxSymbols,
        },
        (progress) => set({ eodSyncProgress: progress }),
      )
      const status = await api.getDataStatus()
      set({ dataStatus: status, syncingEod: false, eodSyncProgress: summary })
      await Promise.all([
        get().refreshWatchlistQuotes(),
        get().refreshBroker(),
        get().refreshYumekoStatus(),
        get().loadYumekoForActiveTicker(),
        get().refreshExportedVerdicts(),
      ])
      const stale = status.freshness?.stale_series ?? 0
      get().pushToast(
        'success',
        stale === 0
          ? `Đồng bộ EOD xong (+${summary.updatedBars.toLocaleString('en-US')} nến / ${summary.updatedSymbols} mã) — đã cập nhật tới ${status.freshness?.expected_session_date ?? 'phiên mới nhất'}`
          : `Đồng bộ EOD xong (+${summary.updatedBars.toLocaleString('en-US')} nến / ${summary.updatedSymbols} mã) — còn ${stale} mã cũ hơn ${status.freshness?.expected_session_date ?? ''}`,
      )
    } catch (err) {
      set({ syncingEod: false })
      get().pushToast('error', `Lỗi đồng bộ EOD: ${String(err)}`)
    }
  },

  // ─── Causal Multi-Timeframe Bar Replay ─────────────────────────────────────

  setReplaySelectingCut: (replaySelectingCut) => set({ replaySelectingCut }),
  setReplayBlindMode: (replayBlindMode) => set({ replayBlindMode }),

  startReplay: async (cutTimestampMs) => {
    const state = get()
    const activePane = state.panes.find((p) => p.slotId === state.activeSlotId) ?? state.panes[0]
    const visibleCount =
      state.gridLayout === '1' ? 1 : state.gridLayout === '2v' || state.gridLayout === '2h' ? 2 : state.gridLayout === '3' ? 3 : 4
    // Collect ALL open visible panes sharing `activePane.ticker` so backend picks the finest TF (e.g. `1h` for `1h`+`4h` MTF Replay, `BUG-040`)!
    const siblingIntervals = Array.from(
      new Set(
        state.panes
          .slice(0, visibleCount)
          .filter((p) => p.ticker.toUpperCase() === activePane.ticker.toUpperCase())
          .map((p) => p.interval),
      ),
    )
    if (!siblingIntervals.includes(activePane.interval)) {
      siblingIntervals.push(activePane.interval)
    }

    try {
      const snap = await api.replayStart(activePane.ticker, activePane.interval, cutTimestampMs, siblingIntervals)
      set((s) => ({
        replay: snap,
        replaySelectingCut: false,
        replayPlaying: false,
        replaySteppedBar: null,
        replayStepEvent: null,
        replayResetNonce: s.replayResetNonce + 1,
        broker: snap.broker ?? s.broker,
      }))
      void api.broadcastSyncEvent('sumi:sync_replay', { action: 'start', snapshot: snap })
      const tfList = (snap.trackedIntervals ?? [snap.interval]).join(' + ')
      get().pushToast(
        'info',
        `Bar Replay nhân quả kích hoạt (${tfList} · nến ${snap.cursorIndex + 1}/${snap.totalBars})`,
      )
    } catch (err) {
      get().pushToast('error', String(err))
    }
  },


  stepReplay: async (delta) => {
    try {
      const snap = await api.replayStep(delta)
      const barsMap: Record<string, OhlcvBar> = { ...(snap.steppedBarsByInterval ?? {}) }
      if (snap.steppedBar && !barsMap[snap.interval]) {
        barsMap[snap.interval] = snap.steppedBar
      }
      const hasSteppedMap = Object.keys(barsMap).length > 0
      const firedAlerts = snap.triggeredAlerts ?? []

      const prevStatus = get().replay?.status
      set((s) => ({
        replay: snap,
        replayPlaying: snap.status === 'Completed' ? false : s.replayPlaying,
        debriefModalOpen:
          snap.status === 'Completed' && prevStatus !== 'Completed' ? true : s.debriefModalOpen,
        broker: snap.broker ?? s.broker,
        alerts:
          firedAlerts.length > 0
            ? s.alerts.map((a) => firedAlerts.find((f) => f.id === a.id) ?? a)
            : s.alerts,
        replayResetNonce: hasSteppedMap ? s.replayResetNonce : s.replayResetNonce + 1,
        replaySteppedBar: snap.steppedBar
          ? { bar: snap.steppedBar, nonce: (s.replaySteppedBar?.nonce ?? 0) + 1 }
          : null,
        replayStepEvent: hasSteppedMap
          ? {
              ticker: snap.ticker,
              barsByInterval: barsMap,
              nonce: (s.replayStepEvent?.nonce ?? 0) + 1,
            }
          : null,
      }))

      for (const f of firedAlerts) {
        notifyAlertTriggered(f)
        get().pushToast('warning', `🔔 Cảnh báo Replay ${f.ticker}: ${f.message || `Chạm mốc ${f.targetPrice}`}`)
      }
      void api.broadcastSyncEvent('sumi:sync_replay', { action: 'step', snapshot: snap })
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
        replaySteppedBar: null,
        replayStepEvent: null,
        replayResetNonce: s.replayResetNonce + 1,
        broker: snap.broker ?? s.broker,
      }))
      void api.broadcastSyncEvent('sumi:sync_replay', { action: 'jump', snapshot: snap })
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
      replaySteppedBar: null,
      replayStepEvent: null,
      replayResetNonce: s.replayResetNonce + 1,
    }))
    void api.broadcastSyncEvent('sumi:sync_replay', { action: 'stop', snapshot: null })
    await get().refreshBroker()
    get().pushToast('info', 'Đã thoát Bar Replay — khôi phục dữ liệu đầy đủ & sổ lệnh Live nguyên vẹn')
  },

  saveCurrentReplaySession: async (name) => {
    const trimmed = name.trim()
    if (!trimmed) return
    try {
      const saved = await api.saveReplaySession(trimmed, get().replayBlindMode)
      set((s) => ({
        replaySessions: [saved, ...s.replaySessions.filter((r) => r.id !== saved.id)],
      }))
      get().pushToast('success', `Đã lưu phiên Bar Replay "${saved.name}" (kèm sổ lệnh luyện tập)`)
    } catch (err) {
      get().pushToast('error', String(err))
    }
  },


  resumeSavedReplaySession: async (id) => {
    try {
      const savedMeta = get().replaySessions.find((r) => r.id === id)
      const snap = await api.resumeReplaySession(id)
      const tracked = (snap.trackedIntervals ?? [snap.interval]) as IntervalKey[]
      const isVn =
        snap.ticker.startsWith('HOSE:') || snap.ticker.startsWith('HNX:') || snap.ticker.startsWith('UPCOM:')
      set((s) => {
        // Auto-configure 2v layout if the saved session tracked 2+ timeframes (e.g. `1h + 4h`)!
        let nextLayout = s.gridLayout
        let nextPanes = s.panes
        if (tracked.length >= 2) {
          nextLayout = s.gridLayout === '1' ? '2v' : s.gridLayout
          nextPanes = s.panes.map((p, idx) =>
            idx < tracked.length
              ? { ...p, ticker: snap.ticker, interval: tracked[idx], showVnBands: isVn }
              : p,
          )
        } else {
          nextPanes = s.panes.map((p) =>
            p.slotId === s.activeSlotId
              ? { ...p, ticker: snap.ticker, interval: snap.interval as IntervalKey, showVnBands: isVn }
              : p,
          )
        }
        return {
          gridLayout: nextLayout,
          panes: nextPanes,
          replay: snap,
          replayBlindMode: savedMeta?.blindMode ?? true,
          replaySelectingCut: false,
          replayPlaying: false,
          replaySteppedBar: null,
          replayStepEvent: null,
          replayResetNonce: s.replayResetNonce + 1,
          broker: snap.broker ?? s.broker,
        }
      })
      void api.broadcastSyncEvent('sumi:sync_replay', { action: 'start', snapshot: snap })
      get().pushToast(
        'success',
        `Đã khôi phục phiên "${savedMeta?.name ?? snap.ticker}" (${tracked.join(' + ')} · nến ${snap.cursorIndex + 1}/${snap.totalBars})`,
      )
    } catch (err) {
      get().pushToast('error', String(err))
    }
  },


  deleteSavedReplaySession: async (id) => {
    await api.deleteReplaySession(id)
    set((s) => ({ replaySessions: s.replaySessions.filter((r) => r.id !== id) }))
  },

  applyRemoteReplaySync: (payload) => {
    if (payload.action === 'stop' || !payload.snapshot) {
      set((s) => ({
        replay: null,
        replaySelectingCut: false,
        replayPlaying: false,
        replaySteppedBar: null,
        replayStepEvent: null,
        replayResetNonce: s.replayResetNonce + 1,
      }))
      return
    }
    const snap = payload.snapshot
    const barsMap: Record<string, OhlcvBar> = { ...(snap.steppedBarsByInterval ?? {}) }
    if (snap.steppedBar && !barsMap[snap.interval]) {
      barsMap[snap.interval] = snap.steppedBar
    }
    const isStep = payload.action === 'step' && Object.keys(barsMap).length > 0
    set((s) => ({
      replay: snap,
      replaySelectingCut: false,
      replayResetNonce: isStep ? s.replayResetNonce : s.replayResetNonce + 1,
      replayStepEvent: isStep
        ? { ticker: snap.ticker, barsByInterval: barsMap, nonce: (s.replayStepEvent?.nonce ?? 0) + 1 }
        : null,
      broker: snap.broker ?? s.broker,
    }))
  },

  // ─── Paper Broker ──────────────────────────────────────────────────────────

  refreshBroker: async () => {
    const activePane = get().panes.find((p) => p.slotId === get().activeSlotId) ?? get().panes[0]
    const key = `${activePane.ticker.toUpperCase()}|${activePane.interval}`
    if (inFlightBrokerKey === key && inFlightBrokerPromise) {
      return inFlightBrokerPromise
    }
    inFlightBrokerKey = key
    const p = (async () => {
      try {
        const snap = await api.brokerGetState(activePane.ticker, activePane.interval)
        set({ broker: snap })
      } catch {
        // ignore if chart has no cached bars yet
      } finally {
        if (inFlightBrokerKey === key) {
          inFlightBrokerKey = null
          inFlightBrokerPromise = null
        }
      }
    })()
    inFlightBrokerPromise = p
    return p
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
