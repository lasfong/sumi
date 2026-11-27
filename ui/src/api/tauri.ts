import { Channel, invoke } from '@tauri-apps/api/core'
import type {
  AlertDeliveryConfig,
  AlertTriggerLogRecord,
  AnalogConeQueryParams,
  AnalogConeResponseDto,
  BarsPageResponse,
  BidirectionalFolderSyncResult,
  BrokerStateSnapshot,
  CafeFImportSummary,
  ChecklistState,
  CompareSeriesResponse,
  CorporateEventMarker,
  DataStatusResponse,
  DomLadderSnapshot,
  DoraemiTransportStatusDto,
  ExportBacktestHtmlResponse,
  ExportBundleResponse,
  ExportedVerdictListItem,
  ExportedVerdictReceipt,
  IndicatorTemplateRecord,
  IntervalKey,
  JournalAnalyticsResponseDto,
  LiveBarPayload,
  LiveQuote,
  MarketBreadthRrgResponse,
  MonteCarloResponse,
  ParameterSweepResponseDto,
  PortfolioBacktestInput,
  PortfolioBacktestResponse,
  PresetsBattleResponseDto,
  PriceAlertRecord,
  QmvRealChartSeriesDto,
  ReplaySnapshotResponse,
  ResearchExportVerificationDto,
  ResolvedSymbolResponse,
  SavedDrawingRecord,
  SavedIndicatorSpec,
  SavedLayoutRecord,
  SavedReplaySessionRecord,
  SavedScreenRecord,
  SessionDebriefReport,
  SignalCatalogItemDto,
  StockScreenerInput,
  StockScreenerResponse,
  StrategyBacktestInput,
  StrategyBacktestResponse,
  SubmitPaperOrderInput,
  SymbolFundamentalsResponse,
  SymbolSearchResult,
  SyncProgressPayload,
  SyncStatusDto,
  TopDownSymbolContextDto,
  TradePlanRequestDto,
  TradePlanResponseDto,
  VnUniverseSyncSummary,
  WalkForwardResponse,
  WatchlistGroup,
  WatchlistQuoteSnapshot,
  WorkspaceBootstrapResponse,
  WorkspaceMergeReport,
  YumekoBridgeStatus,
} from '../types/market'
import { webFallbackApi } from './webFallback'

export function isTauriRuntime(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
}

// ─── Market Data & Real-Time Channels ────────────────────────────────────────

export async function searchSymbols(
  query: string,
  category?: string,
  limit = 40,
): Promise<SymbolSearchResult[]> {
  if (!isTauriRuntime()) return webFallbackApi.searchSymbols(query, category)
  return invoke<SymbolSearchResult[]>('search_symbols', { query, category: category ?? null, limit })
}

export async function resolveSymbol(ticker: string): Promise<ResolvedSymbolResponse> {
  if (!isTauriRuntime()) return webFallbackApi.resolveSymbol(ticker)
  return invoke<ResolvedSymbolResponse>('resolve_symbol', { ticker })
}

export async function getBars(params: {
  ticker: string
  interval: IntervalKey
  mode: 'init' | 'forward' | 'backward'
  timestamp?: number | null
  limit?: number
  adjustment?: string
  asOfDate?: string | null
}): Promise<BarsPageResponse> {
  if (!isTauriRuntime()) {
    return webFallbackApi.getBars(params.ticker, params.interval, params.mode, params.timestamp, params.limit)
  }
  return invoke<BarsPageResponse>('get_bars', {
    ticker: params.ticker,
    interval: params.interval,
    mode: params.mode,
    timestamp: params.timestamp ?? null,
    limit: params.limit ?? 300,
    adjustment: params.adjustment ?? 'splits',
    asOfDate: params.asOfDate ?? null,
  })
}

// Track active Tauri Channel instances per stream so callbacks can be detached cleanly on unsubscribe (`BUG-012`).
const activeBarChannels = new Map<string, Channel<LiveBarPayload>>()
let activeQuoteChannel: Channel<LiveQuote> | null = null

function cleanupTauriChannelCallback(ch: { id?: number }): void {
  const internals = (
    window as unknown as {
      __TAURI_INTERNALS__?: { unregisterCallback?: (id: number) => void }
    }
  ).__TAURI_INTERNALS__
  if (typeof ch.id === 'number' && typeof internals?.unregisterCallback === 'function') {
    try {
      internals.unregisterCallback(ch.id)
    } catch {
      // ignore if already released
    }
  }
}

const barStreamClientGens = new Map<string, number>()

export async function subscribeBars(
  streamId: string,
  ticker: string,
  interval: IntervalKey,
  onBar: (payload: LiveBarPayload) => void,
  adjustment?: string,
): Promise<boolean> {
  if (!isTauriRuntime()) return false
  const myGen = (barStreamClientGens.get(streamId) ?? 0) + 1
  barStreamClientGens.set(streamId, myGen)
  const prev = activeBarChannels.get(streamId)
  if (prev) {
    prev.onmessage = () => {}
    cleanupTauriChannelCallback(prev as unknown as { id?: number })
    activeBarChannels.delete(streamId)
  }
  const channel = new Channel<LiveBarPayload>()
  channel.onmessage = onBar
  activeBarChannels.set(streamId, channel)
  const ok = await invoke<boolean>('subscribe_bars', {
    streamId,
    ticker,
    interval,
    adjustment: adjustment ?? 'splits',
    onBar: channel,
  })
  // If `unsubscribeBars` or a newer `subscribeBars` ran while this invoke was in flight, tear down immediately (`BUG-012`)!
  if (barStreamClientGens.get(streamId) !== myGen) {
    channel.onmessage = () => {}
    cleanupTauriChannelCallback(channel as unknown as { id?: number })
    if (activeBarChannels.get(streamId) === channel) {
      activeBarChannels.delete(streamId)
    }
    if (barStreamClientGens.get(streamId) === myGen + 1 && !activeBarChannels.has(streamId)) {
      void invoke<void>('unsubscribe_bars', { streamId })
    }
    return false
  }
  return ok
}

export async function unsubscribeBars(streamId: string): Promise<void> {
  if (!isTauriRuntime()) return
  barStreamClientGens.set(streamId, (barStreamClientGens.get(streamId) ?? 0) + 1)
  const prev = activeBarChannels.get(streamId)
  if (prev) {
    prev.onmessage = () => {}
    cleanupTauriChannelCallback(prev as unknown as { id?: number })
    activeBarChannels.delete(streamId)
  }
  await invoke<void>('unsubscribe_bars', { streamId })
}

export async function subscribeQuotes(
  tickers: string[],
  onQuote: (quote: LiveQuote) => void,
): Promise<boolean> {
  if (!isTauriRuntime()) return false
  if (activeQuoteChannel) {
    activeQuoteChannel.onmessage = () => {}
    cleanupTauriChannelCallback(activeQuoteChannel as unknown as { id?: number })
    activeQuoteChannel = null
  }
  const channel = new Channel<LiveQuote>()
  channel.onmessage = onQuote
  activeQuoteChannel = channel
  return invoke<boolean>('subscribe_quotes', { tickers, onQuote: channel })
}

export async function unsubscribeQuotes(): Promise<void> {
  if (!isTauriRuntime()) return
  if (activeQuoteChannel) {
    activeQuoteChannel.onmessage = () => {}
    cleanupTauriChannelCallback(activeQuoteChannel as unknown as { id?: number })
    activeQuoteChannel = null
  }
  await invoke<void>('unsubscribe_quotes')
}

export async function getWatchlistSnapshots(tickers: string[]): Promise<WatchlistQuoteSnapshot[]> {
  if (!isTauriRuntime()) return webFallbackApi.getWatchlistSnapshots(tickers)
  return invoke<WatchlistQuoteSnapshot[]>('get_watchlist_snapshots', { tickers })
}

export async function getDataStatus(): Promise<DataStatusResponse> {
  if (!isTauriRuntime()) return webFallbackApi.getDataStatus()
  return invoke<DataStatusResponse>('get_data_status')
}

export async function setOfflineMode(offline: boolean): Promise<DataStatusResponse> {
  if (!isTauriRuntime()) {
    const s = webFallbackApi.getDataStatus()
    return { ...s, offlineMode: offline }
  }
  return invoke<DataStatusResponse>('set_offline_mode', { offline })
}

export async function importCafeFDir(dir: string, includeRaw = true): Promise<CafeFImportSummary> {
  if (!isTauriRuntime()) {
    return {
      files_processed: [`${dir}/CafeF.SolieuGD.Adjusted.csv`],
      distinct_symbols: 473,
      total_series: includeRaw ? 946 : 473,
      total_bars_written: 950996,
      quarantined_rows: 0,
      elapsed_ms: 420,
    }
  }
  return invoke<CafeFImportSummary>('import_cafef_dir', { dir, includeRaw })
}

export async function syncVnUniverseCatalog(): Promise<VnUniverseSyncSummary> {
  if (!isTauriRuntime()) {
    return { discoveredSymbols: 1620, upsertedSymbols: 1620, elapsedMs: 120 }
  }
  return invoke<VnUniverseSyncSummary>('sync_vn_universe_catalog')
}

export async function syncUniverseEod(
  opts: { scope?: string; tickers?: string[]; maxSymbols?: number },
  onProgress: (payload: SyncProgressPayload) => void,
): Promise<SyncProgressPayload> {
  if (!isTauriRuntime()) {
    const done: SyncProgressPayload = {
      scope: opts.scope ?? 'ALL_VN',
      completed: 10,
      total: 10,
      currentTicker: '',
      updatedSymbols: 10,
      updatedBars: 100,
      failedSymbols: 0,
      done: true,
    }
    onProgress(done)
    return done
  }
  const channel = new Channel<SyncProgressPayload>()
  channel.onmessage = onProgress
  try {
    return await invoke<SyncProgressPayload>('sync_universe_eod', {
      scope: opts.scope ?? 'ALL_VN',
      tickers: opts.tickers ?? null,
      maxSymbols: opts.maxSymbols ?? null,
      onProgress: channel,
    })
  } finally {
    channel.onmessage = () => {}
    cleanupTauriChannelCallback(channel as unknown as { id?: number })
  }
}

export async function getCorporateEvents(ticker: string): Promise<CorporateEventMarker[]> {
  if (!isTauriRuntime()) return webFallbackApi.getCorporateEvents(ticker)
  return invoke<CorporateEventMarker[]>('get_corporate_events', { ticker })
}

// ─── Cross-Device Workspace Sync v1 ──────────────────────────────────────────

export async function syncGetStatus(): Promise<SyncStatusDto> {
  if (!isTauriRuntime()) return webFallbackApi.syncGetStatus()
  return invoke<SyncStatusDto>('sync_get_status')
}

export async function syncExportBundle(outPath?: string | null): Promise<ExportBundleResponse> {
  if (!isTauriRuntime()) {
    throw new Error('Xuất gói đồng bộ .sumi.json yêu cầu Sumi Desktop runtime')
  }
  return invoke<ExportBundleResponse>('sync_export_bundle', { outPath: outPath ?? null })
}

export async function syncImportBundle(bundlePath: string): Promise<WorkspaceMergeReport> {
  if (!isTauriRuntime()) {
    throw new Error('Nhập gói đồng bộ .sumi.json yêu cầu Sumi Desktop runtime')
  }
  return invoke<WorkspaceMergeReport>('sync_import_bundle', { bundlePath })
}

export async function syncConfigureFolder(folderPath: string | null): Promise<SyncStatusDto> {
  if (!isTauriRuntime()) return webFallbackApi.syncGetStatus()
  return invoke<SyncStatusDto>('sync_configure_folder', {
    folderPath: folderPath ?? '',
    deviceId: null,
    autoSyncOnLaunch: null,
  })
}

export async function syncFolderNow(folderPath?: string | null): Promise<BidirectionalFolderSyncResult> {
  if (!isTauriRuntime()) {
    throw new Error('Đồng bộ thư mục 2 chiều yêu cầu Sumi Desktop runtime')
  }
  return invoke<BidirectionalFolderSyncResult>('sync_folder_now', { folderPath: folderPath ?? null })
}

export async function syncGetDoraemiStatus(): Promise<DoraemiTransportStatusDto> {
  if (!isTauriRuntime()) return webFallbackApi.syncGetStatus().doraemiTransport
  return invoke<DoraemiTransportStatusDto>('sync_get_doraemi_status')
}

// ─── Workspace, Drawings, Indicators, Watchlists, Alerts ─────────────────────

export async function loadWorkspaceBootstrap(): Promise<WorkspaceBootstrapResponse> {
  if (!isTauriRuntime()) return webFallbackApi.loadBootstrap()
  return invoke<WorkspaceBootstrapResponse>('load_workspace_bootstrap')
}

export async function saveWorkspaceSettings(settings: Record<string, unknown>): Promise<void> {
  if (!isTauriRuntime()) {
    webFallbackApi.saveSettings(settings)
    return
  }
  await invoke<void>('save_workspace_settings', { settings })
}

export async function getDrawings(ticker: string): Promise<SavedDrawingRecord[]> {
  if (!isTauriRuntime()) return webFallbackApi.getDrawings(ticker)
  return invoke<SavedDrawingRecord[]>('get_drawings', { ticker })
}

export async function upsertDrawing(drawing: SavedDrawingRecord): Promise<void> {
  if (!isTauriRuntime()) {
    webFallbackApi.upsertDrawing(drawing)
    return
  }
  await invoke<void>('upsert_drawing', { drawing })
}

export async function deleteDrawing(ticker: string, id: string): Promise<void> {
  if (!isTauriRuntime()) {
    webFallbackApi.deleteDrawing(ticker, id)
    return
  }
  await invoke<void>('delete_drawing', { ticker, id })
}

export async function clearDrawings(ticker: string): Promise<void> {
  if (!isTauriRuntime()) {
    webFallbackApi.clearDrawings(ticker)
    return
  }
  await invoke<void>('clear_drawings', { ticker })
}

export async function getPaneIndicators(slotId: string): Promise<SavedIndicatorSpec[]> {
  if (!isTauriRuntime()) return webFallbackApi.getPaneIndicators(slotId)
  return invoke<SavedIndicatorSpec[]>('get_pane_indicators', { slotId })
}

export async function savePaneIndicators(slotId: string, indicators: SavedIndicatorSpec[]): Promise<void> {
  if (!isTauriRuntime()) {
    webFallbackApi.savePaneIndicators(slotId, indicators)
    return
  }
  await invoke<void>('save_pane_indicators', { slotId, indicators })
}

export async function saveLayout(
  id: string,
  name: string,
  layoutJson: Record<string, unknown>,
): Promise<SavedLayoutRecord> {
  if (!isTauriRuntime()) return webFallbackApi.saveLayout(id, name, layoutJson)
  return invoke<SavedLayoutRecord>('save_layout', { id, name, layoutJson })
}

export async function deleteLayout(id: string): Promise<void> {
  if (!isTauriRuntime()) {
    webFallbackApi.deleteLayout(id)
    return
  }
  await invoke<void>('delete_layout', { id })
}

export async function createWatchlist(id: string, name: string): Promise<WatchlistGroup> {
  if (!isTauriRuntime()) return webFallbackApi.createWatchlist(id, name)
  return invoke<WatchlistGroup>('create_watchlist', { id, name })
}

export async function deleteWatchlist(id: string): Promise<void> {
  if (!isTauriRuntime()) {
    webFallbackApi.deleteWatchlist(id)
    return
  }
  await invoke<void>('delete_watchlist', { id })
}

export async function addWatchlistSymbol(watchlistId: string, ticker: string): Promise<void> {
  if (!isTauriRuntime()) {
    webFallbackApi.addWatchlistSymbol(watchlistId, ticker)
    return
  }
  await invoke<void>('add_watchlist_symbol', { watchlistId, ticker })
}

export async function removeWatchlistSymbol(watchlistId: string, ticker: string): Promise<void> {
  if (!isTauriRuntime()) {
    webFallbackApi.removeWatchlistSymbol(watchlistId, ticker)
    return
  }
  await invoke<void>('remove_watchlist_symbol', { watchlistId, ticker })
}

export async function createAlert(
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
): Promise<PriceAlertRecord> {
  if (!isTauriRuntime()) return webFallbackApi.createAlert(ticker, condition, targetPrice, message)
  return invoke<PriceAlertRecord>('create_alert', {
    ticker,
    condition,
    targetPrice,
    message,
    alertKind: opts?.alertKind ?? null,
    interval: opts?.interval ?? null,
    paramsJson: opts?.paramsJson ?? null,
    soundEnabled: opts?.soundEnabled ?? true,
    frequency: opts?.frequency ?? 'once',
    cooldownBars: opts?.cooldownBars ?? 0,
  })
}

export async function updateAlert(params: {
  id: string
  condition?: string
  targetPrice?: number
  message?: string
  interval?: IntervalKey | null
  paramsJson?: Record<string, unknown> | null
  soundEnabled?: boolean
  frequency?: 'once' | 'once_per_bar_close'
  cooldownBars?: number
  rearm?: boolean
}): Promise<PriceAlertRecord> {
  if (!isTauriRuntime()) {
    return webFallbackApi.updateAlert(params)
  }
  return invoke<PriceAlertRecord>('update_alert', {
    id: params.id,
    condition: params.condition ?? null,
    targetPrice: params.targetPrice ?? null,
    message: params.message ?? null,
    interval: params.interval ?? null,
    paramsJson: params.paramsJson ?? null,
    soundEnabled: params.soundEnabled ?? null,
    frequency: params.frequency ?? null,
    cooldownBars: params.cooldownBars ?? null,
    rearm: params.rearm ?? false,
  })
}

export async function listAlertLogs(limit = 100): Promise<AlertTriggerLogRecord[]> {
  if (!isTauriRuntime()) return []
  return invoke<AlertTriggerLogRecord[]>('list_alert_logs', { limit })
}

export async function clearAlertLogs(): Promise<void> {
  if (!isTauriRuntime()) return
  await invoke<void>('clear_alert_logs')
}

export async function scanWatchlistAlerts(): Promise<PriceAlertRecord[]> {
  if (!isTauriRuntime()) return []
  return invoke<PriceAlertRecord[]>('scan_watchlist_alerts')
}

export async function getAlertDeliveryConfig(): Promise<AlertDeliveryConfig> {
  if (!isTauriRuntime()) {
    return {
      osNotificationEnabled: true,
      telegramBotToken: '',
      telegramChatId: '',
      webhookUrl: '',
      watchlistBackgroundScanEnabled: true,
    }
  }
  return invoke<AlertDeliveryConfig>('get_alert_delivery_config')
}

export async function saveAlertDeliveryConfig(config: AlertDeliveryConfig): Promise<AlertDeliveryConfig> {
  if (!isTauriRuntime()) return config
  return invoke<AlertDeliveryConfig>('save_alert_delivery_config', { config })
}

export async function testAlertDelivery(): Promise<string[]> {
  if (!isTauriRuntime()) return ['in_app']
  return invoke<string[]>('test_alert_delivery')
}

export async function toggleAlertStatus(id: string, status: string): Promise<void> {
  if (!isTauriRuntime()) {
    webFallbackApi.toggleAlert(id, status)
    return
  }
  await invoke<void>('toggle_alert_status', { id, status })
}

export async function deleteAlert(id: string): Promise<void> {
  if (!isTauriRuntime()) {
    webFallbackApi.deleteAlert(id)
    return
  }
  await invoke<void>('delete_alert', { id })
}

export async function checkPriceAlerts(
  ticker: string,
  prevPrice: number,
  currentPrice: number,
): Promise<PriceAlertRecord[]> {
  if (!isTauriRuntime()) return []
  return invoke<PriceAlertRecord[]>('check_price_alerts', { ticker, prevPrice, currentPrice })
}

export async function checkBarAlerts(
  ticker: string,
  interval: IntervalKey,
): Promise<PriceAlertRecord[]> {
  if (!isTauriRuntime()) return []
  return invoke<PriceAlertRecord[]>('check_bar_alerts', { ticker, interval })
}

// ─── Indicator Templates (v2.2) ──────────────────────────────────────────────

export async function listIndicatorTemplates(): Promise<IndicatorTemplateRecord[]> {
  if (!isTauriRuntime()) return []
  return invoke<IndicatorTemplateRecord[]>('list_indicator_templates')
}

export async function saveIndicatorTemplate(
  name: string,
  indicators: SavedIndicatorSpec[],
  description?: string,
  id?: string,
): Promise<IndicatorTemplateRecord> {
  if (!isTauriRuntime()) {
    return { id: id ?? `tpl_${Date.now()}`, name, description: description ?? '', indicators, updatedAtMs: Date.now() }
  }
  return invoke<IndicatorTemplateRecord>('save_indicator_template', {
    id: id ?? null,
    name,
    description: description ?? null,
    indicators,
  })
}

export async function deleteIndicatorTemplate(id: string): Promise<void> {
  if (!isTauriRuntime()) return
  await invoke<void>('delete_indicator_template', { id })
}

export async function openDetachedWindow(ticker: string, interval: IntervalKey): Promise<string | null> {
  if (!isTauriRuntime()) return null
  return invoke<string>('open_detached_window', { ticker, interval })
}

export async function broadcastSyncEvent(
  eventName: 'sumi:sync_replay' | 'sumi:sync_drawing' | 'sumi:sync_symbol' | 'sumi:sync_range',
  payload: Record<string, unknown>,
): Promise<void> {
  if (!isTauriRuntime()) return
  try {
    await invoke<void>('broadcast_sync_event', { eventName, payload })
  } catch {
    // ignore if running outside multi-window Tauri host
  }
}

// ─── Causal Bar Replay ───────────────────────────────────────────────────────

export async function replayStart(
  ticker: string,
  interval: IntervalKey,
  cutTimestampMs?: number | null,
  activeIntervals?: IntervalKey[],
): Promise<ReplaySnapshotResponse> {
  if (!isTauriRuntime()) return webFallbackApi.replayStart(ticker, interval, cutTimestampMs)
  return invoke<ReplaySnapshotResponse>('replay_start', {
    ticker,
    interval,
    cutTimestampMs: cutTimestampMs ?? null,
    activeIntervals: activeIntervals && activeIntervals.length > 0 ? activeIntervals : null,
  })
}

export async function replayGetState(): Promise<ReplaySnapshotResponse | null> {
  if (!isTauriRuntime()) return null
  return invoke<ReplaySnapshotResponse>('replay_get_state')
}

export async function replayStep(delta: number): Promise<ReplaySnapshotResponse> {
  if (!isTauriRuntime()) return webFallbackApi.replayStep(delta)
  return invoke<ReplaySnapshotResponse>('replay_step', { delta })
}

export async function replayJump(targetTimestampMs: number): Promise<ReplaySnapshotResponse> {
  if (!isTauriRuntime()) return webFallbackApi.replayJump(targetTimestampMs)
  return invoke<ReplaySnapshotResponse>('replay_jump', { targetTimestampMs })
}

export async function replaySetSpeed(speedMultiplier: number): Promise<number> {
  if (!isTauriRuntime()) return speedMultiplier
  return invoke<number>('replay_set_speed', { speedMultiplier })
}

export async function replayStop(): Promise<void> {
  if (!isTauriRuntime()) {
    webFallbackApi.replayStop()
    return
  }
  await invoke<void>('replay_stop')
}

export async function listReplaySessions(): Promise<SavedReplaySessionRecord[]> {
  if (!isTauriRuntime()) return []
  return invoke<SavedReplaySessionRecord[]>('list_replay_sessions')
}

export async function saveReplaySession(
  name: string,
  blindMode = true,
  id?: string,
): Promise<SavedReplaySessionRecord> {
  if (!isTauriRuntime()) {
    throw new Error('Lưu phiên Bar Replay yêu cầu Sumi Desktop runtime')
  }
  return invoke<SavedReplaySessionRecord>('save_replay_session', {
    id: id ?? null,
    name,
    blindMode,
  })
}

export async function resumeReplaySession(id: string): Promise<ReplaySnapshotResponse> {
  if (!isTauriRuntime()) {
    throw new Error('Khôi phục phiên Bar Replay yêu cầu Sumi Desktop runtime')
  }
  return invoke<ReplaySnapshotResponse>('resume_replay_session', { id })
}

export async function deleteReplaySession(id: string): Promise<void> {
  if (!isTauriRuntime()) return
  await invoke<void>('delete_replay_session', { id })
}

// ─── Paper Broker (VN VSD T+2 FIFO) ──────────────────────────────────────────

export async function brokerGetState(ticker: string, interval: IntervalKey): Promise<BrokerStateSnapshot> {
  if (!isTauriRuntime()) return webFallbackApi.brokerState(ticker, interval)
  return invoke<BrokerStateSnapshot>('broker_get_state', { ticker, interval })
}

export async function brokerSubmitOrder(input: SubmitPaperOrderInput): Promise<BrokerStateSnapshot> {
  if (!isTauriRuntime()) return webFallbackApi.submitPaperOrder(input)
  return invoke<BrokerStateSnapshot>('broker_submit_order', { input })
}

export async function brokerCancelOrder(
  ticker: string,
  interval: IntervalKey,
  orderId: number,
): Promise<BrokerStateSnapshot> {
  if (!isTauriRuntime()) return webFallbackApi.cancelPaperOrder(ticker, interval, orderId)
  return invoke<BrokerStateSnapshot>('broker_cancel_order', { ticker, interval, orderId })
}

export async function brokerModifyOrder(
  ticker: string,
  interval: IntervalKey,
  orderId: number,
  newPrice: number,
): Promise<BrokerStateSnapshot> {
  if (!isTauriRuntime()) return webFallbackApi.modifyPaperOrder(ticker, interval, orderId, newPrice)
  return invoke<BrokerStateSnapshot>('broker_modify_order', { ticker, interval, orderId, newPrice })
}

export async function brokerUpdateBrackets(
  ticker: string,
  interval: IntervalKey,
  stopLoss: number | null,
  takeProfit: number | null,
): Promise<BrokerStateSnapshot> {
  if (!isTauriRuntime()) {
    return webFallbackApi.updatePositionBrackets(ticker, interval, stopLoss, takeProfit)
  }
  return invoke<BrokerStateSnapshot>('broker_update_brackets', { ticker, interval, stopLoss, takeProfit })
}

export async function brokerFlattenPosition(
  ticker: string,
  interval: IntervalKey,
): Promise<BrokerStateSnapshot> {
  if (!isTauriRuntime()) return webFallbackApi.flattenPaperPosition(ticker, interval)
  return invoke<BrokerStateSnapshot>('broker_flatten_position', { ticker, interval })
}

export async function brokerResetAccount(
  ticker: string,
  interval: IntervalKey,
  initialCash?: number,
  enforceT2?: boolean,
): Promise<BrokerStateSnapshot> {
  if (!isTauriRuntime()) return webFallbackApi.resetPaperAccount(ticker, interval, initialCash)
  return invoke<BrokerStateSnapshot>('broker_reset_account', {
    ticker,
    interval,
    initialCash: initialCash ?? null,
    enforceT2: enforceT2 ?? null,
  })
}

export async function brokerUpdateJournalNote(
  ticker: string,
  tradeId: number,
  setupTag: string,
  mistakeTag: string,
  note: string,
  checklistJson?: ChecklistState | null,
): Promise<void> {
  if (!isTauriRuntime()) return
  await invoke('broker_update_journal_note', {
    ticker,
    tradeId,
    setupTag,
    mistakeTag,
    note,
    checklistJson: checklistJson ?? null,
  })
}

export async function brokerGetJournalAnalytics(
  ticker: string,
  interval: IntervalKey,
): Promise<JournalAnalyticsResponseDto> {
  if (!isTauriRuntime()) return webFallbackApi.brokerGetJournalAnalytics(ticker)
  return invoke<JournalAnalyticsResponseDto>('broker_get_journal_analytics', { ticker, interval })
}

export async function computeDomLadder(
  ticker: string,
  interval: IntervalKey,
  halfRows = 8,
  lookbackBars = 100,
): Promise<DomLadderSnapshot> {
  if (!isTauriRuntime()) return webFallbackApi.computeDomLadder(ticker)
  return invoke<DomLadderSnapshot>('compute_dom_ladder_cmd', { ticker, interval, halfRows, lookbackBars })
}

export async function computeTradePlan(input: TradePlanRequestDto): Promise<TradePlanResponseDto> {
  if (!isTauriRuntime()) return webFallbackApi.computeTradePlan(input)
  return invoke<TradePlanResponseDto>('compute_trade_plan_cmd', { input })
}

export async function computeReplayDebrief(
  ticker: string,
  interval: IntervalKey,
): Promise<SessionDebriefReport> {
  if (!isTauriRuntime()) return webFallbackApi.computeReplayDebrief(ticker, interval)
  return invoke<SessionDebriefReport>('compute_replay_debrief_cmd', { ticker, interval })
}

// ─── Strategy Tester (Backtest / WFO / Monte Carlo) ──────────────────────────

export async function runStrategyBacktest(
  input: StrategyBacktestInput,
): Promise<StrategyBacktestResponse> {
  if (!isTauriRuntime()) return webFallbackApi.runBacktest(input)
  return invoke<StrategyBacktestResponse>('run_strategy_backtest', { input })
}

export async function runStrategyPortfolio(
  input: PortfolioBacktestInput,
): Promise<PortfolioBacktestResponse> {
  if (!isTauriRuntime()) {
    throw new Error('Portfolio Backtest yêu cầu lõi Rust Sumi Desktop')
  }
  return invoke<PortfolioBacktestResponse>('run_strategy_portfolio', { input })
}

export async function runStrategyWfo(
  backtest: StrategyBacktestInput,
  foldCount: number,
  inSampleRatio: number,
  anchored: boolean,
): Promise<WalkForwardResponse> {
  if (!isTauriRuntime()) return webFallbackApi.runWfo(backtest)
  return invoke<WalkForwardResponse>('run_strategy_wfo', {
    input: { backtest, foldCount, inSampleRatio, anchored },
  })
}

export async function runStrategyMonteCarlo(
  backtest: StrategyBacktestInput,
  simulations: number,
  ruinDrawdownPct: number,
  seed?: number,
): Promise<MonteCarloResponse> {
  if (!isTauriRuntime()) return webFallbackApi.runMonteCarlo(backtest)
  return invoke<MonteCarloResponse>('run_strategy_monte_carlo', {
    input: { backtest, simulations, seed: seed ?? null, ruinDrawdownPct },
  })
}

// ─── Stock Screener (101 Signals) ────────────────────────────────────────────

export async function getSignalCatalog(): Promise<SignalCatalogItemDto[]> {
  if (!isTauriRuntime()) return webFallbackApi.getSignalCatalog()
  return invoke<SignalCatalogItemDto[]>('get_signal_catalog')
}

export async function runStockScreener(input: StockScreenerInput): Promise<StockScreenerResponse> {
  if (!isTauriRuntime()) return webFallbackApi.runScreener(input)
  return invoke<StockScreenerResponse>('run_stock_screener', { input })
}

// ─── AFC — Analog Forecast Cone (Dual-Horizon PIT Probability Fan) ───────────

export async function computeAnalogCone(
  params: AnalogConeQueryParams,
): Promise<AnalogConeResponseDto> {
  if (!isTauriRuntime()) return webFallbackApi.computeAnalogCone(params)
  return invoke<AnalogConeResponseDto>('compute_analog_cone', {
    ticker: params.ticker,
    interval: params.interval,
    horizon: params.horizon ?? null,
    topK: params.topK ?? null,
    useMarketContext: params.useMarketContext ?? null,
    asOfTimestampMs: params.asOfTimestampMs ?? null,
  })
}

// ─── Yumeko v3.0 Read-Only Top-Down Bridge & KT01 Evidence (Wave 2) ──────────

export async function yumekoGetBridgeStatus(): Promise<YumekoBridgeStatus> {
  if (!isTauriRuntime()) return webFallbackApi.yumekoGetBridgeStatus()
  return invoke<YumekoBridgeStatus>('yumeko_get_bridge_status')
}

export async function yumekoGetTopdownContext(
  ticker: string,
  asOf?: string | null,
): Promise<TopDownSymbolContextDto> {
  if (!isTauriRuntime()) return webFallbackApi.yumekoGetTopdownContext(ticker, asOf ?? null)
  return invoke<TopDownSymbolContextDto>('yumeko_get_topdown_context', {
    ticker,
    asOf: asOf ?? null,
  })
}

export async function yumekoGetQmvSeries(
  ticker: string,
  asOf?: string | null,
): Promise<QmvRealChartSeriesDto> {
  if (!isTauriRuntime()) return webFallbackApi.yumekoGetQmvSeries(ticker, asOf ?? null)
  return invoke<QmvRealChartSeriesDto>('yumeko_get_qmv_series', {
    ticker,
    asOf: asOf ?? null,
  })
}

export async function yumekoExportKt01Verdict(
  ticker: string,
  asOf?: string | null,
  copyToYumekoDropzone = true,
): Promise<ExportedVerdictReceipt> {
  if (!isTauriRuntime()) {
    throw new Error('Xuất gói bằng chứng KT01 (SHA-256) yêu cầu Sumi Desktop runtime')
  }
  return invoke<ExportedVerdictReceipt>('yumeko_export_kt01_verdict', {
    ticker,
    asOf: asOf ?? null,
    copyToYumekoDropzone,
  })
}

export async function yumekoListExportedVerdicts(limit = 20): Promise<ExportedVerdictListItem[]> {
  if (!isTauriRuntime()) return []
  return invoke<ExportedVerdictListItem[]>('yumeko_list_exported_verdicts', { limit })
}

export async function yumekoVerifyResearchExport(dir: string): Promise<ResearchExportVerificationDto> {
  if (!isTauriRuntime()) {
    return {
      exportDir: dir,
      manifestFound: false,
      verifiedFiles: 0,
      failedFiles: ['Yêu cầu Sumi Desktop runtime'],
      asOf: null,
      valid: false,
    }
  }
  return invoke<ResearchExportVerificationDto>('yumeko_verify_research_export', { dir })
}

// ─── Wave 3: Market Breadth/RRG, Backtest Export, Sweep & Presets Battle ────

export async function getMarketBreadthAndRrg(
  scope = 'ALL_VN',
  asOfDate?: string | null,
): Promise<MarketBreadthRrgResponse> {
  if (!isTauriRuntime()) return webFallbackApi.getMarketBreadthAndRrg(scope, asOfDate ?? null)
  return invoke<MarketBreadthRrgResponse>('get_market_breadth_and_rrg', {
    scope,
    asOfDate: asOfDate ?? null,
  })
}

export async function exportBacktestReportHtml(
  input: StrategyBacktestInput,
): Promise<ExportBacktestHtmlResponse> {
  if (!isTauriRuntime()) {
    throw new Error('Xuất báo cáo Backtest HTML/PDF yêu cầu Sumi Desktop runtime')
  }
  return invoke<ExportBacktestHtmlResponse>('export_backtest_report_html', { input })
}

export async function runStrategyParameterSweep(
  input: StrategyBacktestInput,
  rankingMetric = 'sharpe',
): Promise<ParameterSweepResponseDto> {
  if (!isTauriRuntime()) {
    throw new Error('Tối ưu hóa tham số (Parameter Sweep) yêu cầu Sumi Desktop runtime')
  }
  return invoke<ParameterSweepResponseDto>('run_strategy_parameter_sweep', {
    input,
    rankingMetric,
  })
}

export async function runStrategyPresetsBattle(
  input: StrategyBacktestInput,
  rankingMetric = 'sharpe',
): Promise<PresetsBattleResponseDto> {
  if (!isTauriRuntime()) {
    throw new Error('Đấu trường Chiến lược (All-Presets Battle) yêu cầu Sumi Desktop runtime')
  }
  return invoke<PresetsBattleResponseDto>('run_strategy_presets_battle', {
    input,
    rankingMetric,
  })
}

// ─── Wave 4: Fundamentals (DN01/TV), Multi-Symbol Compare & Saved Screens ───

export async function getSymbolFundamentals(
  ticker: string,
  asOf?: string | null,
): Promise<SymbolFundamentalsResponse> {
  if (!isTauriRuntime()) return webFallbackApi.getSymbolFundamentals(ticker)
  return invoke<SymbolFundamentalsResponse>('get_symbol_fundamentals', {
    ticker,
    asOf: asOf ?? null,
  })
}

export async function getCompareSeries(params: {
  baseTicker: string
  compareTickers: string[]
  interval: IntervalKey
  limit?: number
  asOfDate?: string | null
}): Promise<CompareSeriesResponse> {
  if (!isTauriRuntime()) {
    return webFallbackApi.getCompareSeries(params.baseTicker, params.compareTickers, params.interval)
  }
  return invoke<CompareSeriesResponse>('get_compare_series', {
    baseTicker: params.baseTicker,
    compareTickers: params.compareTickers,
    interval: params.interval,
    limit: params.limit ?? 400,
    asOfDate: params.asOfDate ?? null,
  })
}

export async function listSavedScreens(): Promise<SavedScreenRecord[]> {
  if (!isTauriRuntime()) return webFallbackApi.listSavedScreens()
  return invoke<SavedScreenRecord[]>('list_saved_screens')
}

export async function saveScreen(
  name: string,
  filterJson: Record<string, unknown>,
  id?: string | null,
): Promise<SavedScreenRecord> {
  if (!isTauriRuntime()) return webFallbackApi.saveScreen(name, filterJson, id ?? undefined)
  return invoke<SavedScreenRecord>('save_screen', {
    id: id ?? null,
    name,
    filterJson,
  })
}

export async function deleteScreen(id: string): Promise<void> {
  if (!isTauriRuntime()) {
    webFallbackApi.deleteScreen(id)
    return
  }
  await invoke<void>('delete_screen', { id })
}
