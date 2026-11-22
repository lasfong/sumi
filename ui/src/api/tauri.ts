import { Channel, invoke } from '@tauri-apps/api/core'
import type {
  BarsPageResponse,
  BrokerStateSnapshot,
  CafeFImportSummary,
  DataStatusResponse,
  IntervalKey,
  LiveBarPayload,
  LiveQuote,
  MonteCarloResponse,
  PriceAlertRecord,
  ReplaySnapshotResponse,
  ResolvedSymbolResponse,
  SavedDrawingRecord,
  SavedIndicatorSpec,
  SavedLayoutRecord,
  SignalCatalogItemDto,
  StockScreenerInput,
  StockScreenerResponse,
  StrategyBacktestInput,
  StrategyBacktestResponse,
  SubmitPaperOrderInput,
  SymbolSearchResult,
  WalkForwardResponse,
  WatchlistGroup,
  WatchlistQuoteSnapshot,
  WorkspaceBootstrapResponse,
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
  })
}

export async function subscribeBars(
  streamId: string,
  ticker: string,
  interval: IntervalKey,
  onBar: (payload: LiveBarPayload) => void,
): Promise<boolean> {
  if (!isTauriRuntime()) return false
  const channel = new Channel<LiveBarPayload>()
  channel.onmessage = onBar
  return invoke<boolean>('subscribe_bars', { streamId, ticker, interval, onBar: channel })
}

export async function unsubscribeBars(streamId: string): Promise<void> {
  if (!isTauriRuntime()) return
  await invoke<void>('unsubscribe_bars', { streamId })
}

export async function subscribeQuotes(
  tickers: string[],
  onQuote: (quote: LiveQuote) => void,
): Promise<boolean> {
  if (!isTauriRuntime()) return false
  const channel = new Channel<LiveQuote>()
  channel.onmessage = onQuote
  return invoke<boolean>('subscribe_quotes', { tickers, onQuote: channel })
}

export async function unsubscribeQuotes(): Promise<void> {
  if (!isTauriRuntime()) return
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

export async function importCafeFDir(dir: string): Promise<CafeFImportSummary> {
  if (!isTauriRuntime()) {
    return {
      files_processed: [`${dir}/CafeF.SolieuGD.Adjusted.csv`],
      distinct_symbols: 473,
      total_series: 473,
      total_bars_written: 950996,
      quarantined_rows: 0,
      elapsed_ms: 420,
    }
  }
  return invoke<CafeFImportSummary>('import_cafef_dir', { dir })
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
): Promise<PriceAlertRecord> {
  if (!isTauriRuntime()) return webFallbackApi.createAlert(ticker, condition, targetPrice, message)
  return invoke<PriceAlertRecord>('create_alert', { ticker, condition, targetPrice, message })
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

// ─── Causal Bar Replay ───────────────────────────────────────────────────────

export async function replayStart(
  ticker: string,
  interval: IntervalKey,
  cutTimestampMs?: number | null,
): Promise<ReplaySnapshotResponse> {
  if (!isTauriRuntime()) return webFallbackApi.replayStart(ticker, interval, cutTimestampMs)
  return invoke<ReplaySnapshotResponse>('replay_start', {
    ticker,
    interval,
    cutTimestampMs: cutTimestampMs ?? null,
  })
}

export async function replayStep(delta: number): Promise<ReplaySnapshotResponse> {
  if (!isTauriRuntime()) return webFallbackApi.replayStep(delta)
  return invoke<ReplaySnapshotResponse>('replay_step', { delta })
}

export async function replayJump(targetTimestampMs: number): Promise<ReplaySnapshotResponse> {
  if (!isTauriRuntime()) return webFallbackApi.replayStep(0)
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
): Promise<void> {
  if (!isTauriRuntime()) return
  await invoke('broker_update_journal_note', { ticker, tradeId, setupTag, mistakeTag, note })
}

// ─── Strategy Tester (Backtest / WFO / Monte Carlo) ──────────────────────────

export async function runStrategyBacktest(
  input: StrategyBacktestInput,
): Promise<StrategyBacktestResponse> {
  if (!isTauriRuntime()) return webFallbackApi.runBacktest(input)
  return invoke<StrategyBacktestResponse>('run_strategy_backtest', { input })
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
