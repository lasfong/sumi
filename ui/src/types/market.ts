export type ThemeMode = 'dark' | 'light'

export type IntervalKey = '1m' | '5m' | '15m' | '30m' | '1h' | '4h' | '1D' | '1W' | '1M'

export type ChartStyleKind = 'candle_solid' | 'candle_stroke' | 'heikin_ashi' | 'ohlc' | 'line' | 'area'

export type PriceScaleKind = 'normal' | 'percentage' | 'logarithm'

export type GridLayoutKind = '1' | '2v' | '2h' | '3' | '4'

export type MagnetMode = 'normal' | 'weak_magnet' | 'strong_magnet'

export type BottomDockTab = 'paper' | 'strategy' | 'screener'

export interface OhlcvBar {
  timestamp: number
  open: number
  high: number
  low: number
  close: number
  volume: number
  turnover: number
  [key: string]: unknown
}

export interface VnBandLevels {
  venue: string
  label: string
  reference: number
  ceiling: number
  floor: number
  tickSize: number | null
}

export interface VariableTickTier {
  tick: number
  upper_bound: number | null
}

export interface VariableTickTable {
  raw: string
  tiers: VariableTickTier[]
}

export interface ResolvedSymbol {
  ticker: string
  exchange: string
  listed_exchange: string
  symbol: string
  description: string
  market_type: string
  currency: string
  timezone: string
  session: string
  price_precision: number
  volume_precision: number
  pricescale: number
  minmov: number
  minmove2: number
  tick_size: number | null
  variable_tick_size: string
  variable_tick_table: VariableTickTable | null
  point_value: number
  delay_secs: number
  isin: string
}

export interface SymbolSearchResult {
  ticker: string
  exchange: string
  prefix: string
  symbol: string
  description: string
  market_type: string
  currency_code: string
  country_code: string
  provider_id: string
  type_specs: string[]
}

export interface ResolvedSymbolResponse {
  symbolInfo: ResolvedSymbol
  lastPrice: number | null
  prevClose: number | null
  change: number | null
  changePercent: number | null
  volume: number | null
  vnBand: VnBandLevels | null
}

export interface BarsPageResponse {
  bars: OhlcvBar[]
  hasOlder: boolean
  hasNewer: boolean
  source: 'cache_hit' | 'cache_with_gap_fill' | 'remote_live' | 'offline_cache'
  symbolInfo: ResolvedSymbol | null
  vnBand: VnBandLevels | null
  replayActive: boolean
}

export interface LiveBarPayload {
  ticker: string
  interval: string
  bar: OhlcvBar
  isUpdate: boolean
}

export interface LiveQuote {
  ticker: string
  exchange: string
  symbol: string
  timestamp_ms: number
  last_price: number | null
  bid: number | null
  bid_size: number | null
  ask: number | null
  ask_size: number | null
  open: number | null
  high: number | null
  low: number | null
  prev_close: number | null
  change: number | null
  change_percent: number | null
  volume: number | null
  currency: string | null
}

export interface WatchlistQuoteSnapshot {
  ticker: string
  exchange: string
  symbol: string
  description: string
  lastPrice: number | null
  prevClose: number | null
  change: number | null
  changePercent: number | null
  volume: number | null
  pricePrecision: number
  vnBand: VnBandLevels | null
  timestampMs: number | null
}

export interface StoreStats {
  db_path: string
  total_symbols: number
  total_series: number
  total_bars: number
}

export interface DataStatusResponse {
  stats: StoreStats
  offlineMode: boolean
}

export interface CafeFImportSummary {
  files_processed: string[]
  distinct_symbols: number
  total_series: number
  total_bars_written: number
  quarantined_rows: number
  elapsed_ms: number
}

export interface SavedDrawingRecord {
  id: string
  ticker: string
  name: string
  paneId: string
  groupId: string
  points: Array<{ timestamp?: number; dataIndex?: number; value?: number }>
  styles: Record<string, unknown> | null
  extendData: unknown
  lock: boolean
  visible: boolean
  mode: string
  updatedAtMs: number
}

export interface SavedIndicatorSpec {
  id: string
  name: string
  paneId: string
  isMain: boolean
  calcParams: number[]
  visible: boolean
  styles?: Record<string, unknown> | null
}

export interface SavedLayoutRecord {
  id: string
  name: string
  layoutJson: Record<string, unknown>
  updatedAtMs: number
}

export interface WatchlistGroup {
  id: string
  name: string
  tickers: string[]
  sortOrder: number
}

export interface PriceAlertRecord {
  id: string
  ticker: string
  condition: 'cross_above' | 'cross_below' | 'cross_any' | string
  targetPrice: number
  message: string
  status: 'armed' | 'triggered' | 'paused' | string
  createdAtMs: number
  triggeredAtMs: number | null
}

export interface WorkspaceBootstrapResponse {
  settings: Record<string, unknown> | null
  layouts: SavedLayoutRecord[]
  watchlists: WatchlistGroup[]
  alerts: PriceAlertRecord[]
}

// ─── Paper Broker Types ────────────────────────────────────────────────────

export interface AccountConfig {
  initial_cash: number
  commission_bps: number
  tax_sell_bps: number
  slippage_bps: number
  contract_multiplier: number
  initial_margin_pct: number
  max_leverage: number
  allow_short: boolean
  enforce_vn_t2_lot_lock: boolean
  venue: string
  base_timeframe_secs: number
}

export interface PortfolioMetrics {
  cash_balance: number
  unrealized_pnl: number
  equity: number
  net_pnl: number
  return_pct: number
  max_drawdown_pct: number
  pending_order_count: number
  fill_count: number
  closed_trade_count: number
  win_count: number
  loss_count: number
  breakeven_count: number
  win_rate_pct: number
  avg_return_pct: number
  profit_factor: number | null
  net_r_multiple: number
  avg_r_multiple: number | null
  expectancy_pnl: number | null
}

export interface Position {
  side: 'Long' | 'Short'
  quantity: number
  avg_entry_price: number
  contract_multiplier: number
  opened_timestamp: number
  opened_base_index: number
  stop_loss: number | null
  take_profit: number | null
  initial_stop_loss: number | null
  initial_risk_amount: number | null
  had_initial_sl_at_open: boolean
}

export type OrderKind = 'Market' | { Limit: { limit_price: number } } | { Stop: { stop_price: number } }

export type OrderStatus = 'Pending' | 'Filled' | 'Cancelled' | { Rejected: { reason: string } }

export interface Order {
  id: number
  side: 'Buy' | 'Sell'
  kind: OrderKind
  quantity: number
  stop_loss: number | null
  take_profit: number | null
  submitted_base_index: number
  submitted_timestamp: number
  status: OrderStatus
  filled_price: number | null
  filled_timestamp: number | null
}

export interface EnrichedOrder {
  order: Order
  vnOrderType: string
  submittedTimestampMs: number
  filledTimestampMs: number | null
}

export interface HoldingLot {
  lot_id: number
  buy_fill_id: number
  buy_base_index: number
  buy_timestamp: number
  buy_price: number
  initial_qty: number
  remaining_qty: number
  sellable_at_timestamp: number
}

export interface FifoSettlementSummary {
  venue: string
  as_of_timestamp: number
  lots: HoldingLot[]
  total_held_qty: number
  sellable_qty: number
  locked_t2_qty: number
  earliest_unlock_timestamp: number | null
  unsettled_sell_violation_qty: number
}

export interface ClosedTrade {
  trade_id: number
  side: 'Long' | 'Short'
  quantity: number
  entry_price: number
  exit_price: number
  entry_base_index: number
  exit_base_index: number
  entry_timestamp: number
  exit_timestamp: number
  exit_reason: string
  initial_stop_loss: number | null
  had_initial_sl_at_open: boolean
  gross_pnl: number
  total_fees: number
  net_pnl: number
  return_pct: number
  r_multiple: number | null
}

export interface EnrichedClosedTrade {
  trade: ClosedTrade
  entryTimestampMs: number
  exitTimestampMs: number
  maePct: number
  mfePct: number
  setupTag: string
  mistakeTag: string
  note: string
}

export interface BrokerStateSnapshot {
  ticker: string
  interval: string
  venue: string
  boardLot: number
  allowsShort: boolean
  enforceVnT2LotLock: boolean
  markPrice: number
  markTimestampMs: number
  vnBand: VnBandLevels | null
  config: AccountConfig
  metrics: PortfolioMetrics
  openPosition: Position | null
  orders: EnrichedOrder[]
  fifoSummary: FifoSettlementSummary
  closedTrades: EnrichedClosedTrade[]
  replayLinked: boolean
}

export interface SubmitPaperOrderInput {
  ticker: string
  interval: string
  side: 'BUY' | 'SELL'
  vnOrderType: 'LO' | 'MP' | 'ATO' | 'ATC' | 'STOP'
  quantity: number
  price?: number | null
  stopLoss?: number | null
  takeProfit?: number | null
}

// ─── Bar Replay Types ──────────────────────────────────────────────────────

export interface ReplaySnapshotResponse {
  active: boolean
  status: 'Inactive' | 'Paused' | 'Playing' | 'Completed'
  ticker: string
  interval: string
  cursorIndex: number
  totalBars: number
  cutoffTimestampMs: number | null
  speedMultiplier: number
  visibleBars: OhlcvBar[]
  steppedBar: OhlcvBar | null
  broker: BrokerStateSnapshot | null
}

// ─── Strategy Tester Types ─────────────────────────────────────────────────

export interface StrategyBacktestInput {
  ticker: string
  interval: string
  preset: 'sma_ema' | 'rsi_mean_rev' | 'macd' | 'bb_squeeze' | 'donchian_atr' | 'rsi_bb'
  direction: 'long' | 'short' | 'both'
  initialCash: number
  commissionBps: number
  taxSellBps: number
  slippageBps: number
  lotStep: number
  positionSizePct: number
  stopLossPct?: number | null
  takeProfitPct?: number | null
  minHoldingBars?: number | null
  fastSma?: number | null
  slowEma?: number | null
  rsiPeriod?: number | null
  rsiOversold?: number | null
  rsiOverbought?: number | null
  rsiConfluenceBuy?: number | null
  rsiConfluenceSell?: number | null
  macdFast?: number | null
  macdSlow?: number | null
  macdSignal?: number | null
  macdTrendEma?: number | null
  bbPeriod?: number | null
  bbStdMult?: number | null
  maxSqueezeBwPct?: number | null
  donchianEntry?: number | null
  donchianExit?: number | null
  inSampleSplitPct?: number | null
  maxBars?: number | null
  fromTimestampMs?: number | null
  toTimestampMs?: number | null
}

export interface BacktestChartOverlayState {
  ticker: string
  interval: string
  presetLabel: string
  trades: StrategyTradeDto[]
  visible: boolean
}

export interface BacktestTearsheet {
  initial_cash: number
  final_cash: number
  final_equity: number
  realized_net_profit: number
  total_return_abs: number
  total_return_pct: number
  cagr_pct: number | null
  max_drawdown_abs: number
  max_drawdown_pct: number
  max_drawdown_duration_bars: number
  sharpe_ratio: number | null
  sortino_ratio: number | null
  calmar_ratio: number | null
  num_trades: number
  num_winners: number
  num_losers: number
  num_breakeven: number
  win_rate_pct: number
  profit_factor: number | null
  expectancy_net: number | null
  avg_trade_return_pct: number
  avg_win_pct: number
  avg_loss_pct: number
  avg_bars_held: number
  avg_r_multiple: number | null
  exposure_pct: number
  total_fees_and_tax: number
  low_sample_warning: boolean
}

export interface StrategyEquityPointDto {
  barIndex: number
  timestampMs: number
  equity: number
  underwaterPct: number
}

export interface StrategyTradeDto {
  tradeId: number
  side: string
  quantity: number
  entryBarIndex: number
  exitBarIndex: number
  entryTimestampMs: number
  exitTimestampMs: number
  entryPrice: number
  exitPrice: number
  barsHeld: number
  grossPnl: number
  netPnl: number
  returnPct: number
  rMultiple: number | null
  maePct: number
  mfePct: number
  exitReason: string
}

export interface StrategyBacktestResponse {
  ticker: string
  interval: string
  presetLabel: string
  barCount: number
  tearsheet: BacktestTearsheet
  inSampleTearsheet: BacktestTearsheet | null
  outOfSampleTearsheet: BacktestTearsheet | null
  robustness: string | null
  equityCurve: StrategyEquityPointDto[]
  trades: StrategyTradeDto[]
}

export interface WalkForwardFoldResult {
  fold_index: number
  is_start_bar: number
  is_end_bar: number
  oos_start_bar: number
  oos_end_bar: number
  winning_label: string
  is_tearsheet: BacktestTearsheet
  oos_tearsheet: BacktestTearsheet
}

export interface WalkForwardResponse {
  ticker: string
  interval: string
  totalBars: number
  profitableFolds: number
  foldCount: number
  wfePct: number | null
  paramStabilityPct: number
  verdict: string
  verdictLabel: string
  stitchedOosTearsheet: BacktestTearsheet
  stitchedOosCurve: StrategyEquityPointDto[]
  folds: WalkForwardFoldResult[]
}

export interface QuantileEnvelope5 {
  p5: number
  p25: number
  p50: number
  p75: number
  p95: number
}

export interface MonteCarloRiskMetrics {
  terminal_equity: QuantileEnvelope5
  terminal_return_pct: QuantileEnvelope5
  max_drawdown_p50_pct: number
  max_drawdown_var_95_pct: number
  max_drawdown_cvar_95_pct: number
  max_drawdown_var_99_pct: number
  max_drawdown_cvar_99_pct: number
  prob_of_profit_pct: number
  prob_of_loss_pct: number
  prob_of_ruin_pct: number
}

export interface MonteCarloResponse {
  simulations: number
  seed: number
  sampleCount: number
  horizonSteps: number
  validity: string
  lowSampleWarning: boolean
  statusReason: string
  metrics: MonteCarloRiskMetrics | null
  checkpointEnvelopes: QuantileEnvelope5[]
}

// ─── Stock Screener (101 Signals) Types ────────────────────────────────────

export interface SignalCatalogItemDto {
  id: string
  astAlias: string
  labelEn: string
  labelVi: string
  description: string
  category: string
  categoryLabel: string
  outputType: string
  status: string
  warmupBars: number
}

export interface StockScreenerInput {
  exchangeFilter: 'ALL' | 'HOSE' | 'HNX' | 'UPCOM' | 'WATCHLIST'
  tickers?: string[]
  signalId?: string
  categoryFilter?: string
  regimeFilter?: string
  searchQuery?: string
  maxSymbols?: number
}

export interface ScreenerRowDto {
  ticker: string
  exchange: string
  symbol: string
  description: string
  lastClose: number
  changePct: number
  lastVolume: number
  rvol20: number | null
  rsi14: number | null
  macdHist: number | null
  atrPct: number | null
  regime: string
  regimeShort: string
  healthScore: number | null
  bullishSignals: number
  bearishSignals: number
  matchedSignals: string[]
  selectedSignalValue: string | null
  lastTimestampMs: number
}

export interface StockScreenerResponse {
  totalCatalogSignals: number
  scannedSymbols: number
  matchedSymbols: number
  elapsedMs: number
  rows: ScreenerRowDto[]
}

export interface ChartPaneConfig {
  slotId: string
  ticker: string
  interval: IntervalKey
  chartStyle: ChartStyleKind
  priceScale: PriceScaleKind
  showVnBands: boolean
  showCountdown: boolean
  indicators: SavedIndicatorSpec[]
}
