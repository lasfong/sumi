export type ThemeMode = 'dark' | 'light'

export type IntervalKey = '1m' | '5m' | '15m' | '30m' | '1h' | '4h' | '1D' | '1W' | '1M'

export type ChartStyleKind = 'candle_solid' | 'candle_stroke' | 'heikin_ashi' | 'ohlc' | 'line' | 'area'

export type PriceScaleKind = 'normal' | 'percentage' | 'logarithm'

export type GridLayoutKind = '1' | '2v' | '2h' | '3' | '4'

export type MagnetMode = 'normal' | 'weak_magnet' | 'strong_magnet'

export type BottomDockTab = 'paper' | 'strategy' | 'screener' | 'breadth'

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
  source:
    | 'cache_hit'
    | 'cache_hit_dnse_fresh'
    | 'cache_with_gap_fill'
    | 'remote_live'
    | 'offline_cache'
    | 'yumeko_dnse_pit'
    | 'yumeko_dnse_pit_stitched'
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
  open?: number | null
  high?: number | null
  low?: number | null
  bid?: number | null
  ask?: number | null
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

export interface VnFreshnessSummary {
  expected_session_ms: number
  expected_session_date: string
  total_vn_d1_series: number
  fresh_series: number
  stale_series: number
  median_latest_ms: number | null
  median_latest_date: string | null
  newest_latest_ms: number | null
  newest_latest_date: string | null
  max_missing_sessions: usizeOrNum
}

type usizeOrNum = number

export interface DataStatusResponse {
  stats: StoreStats
  offlineMode: boolean
  freshness?: VnFreshnessSummary | null
  dataDir?: string
  configDir?: string
  yumekoDbDetected?: boolean
}

export interface CorporateEventMarker {
  timestampMs?: number
  timestamp_ms?: number
  date: string
  kind: 'D' | 'S' | 'E' | 'dividend' | 'split' | 'earnings' | string
  factor?: number | null
  ratio?: number
  label: string
  detail: string
}

export interface SyncProgressPayload {
  scope: string
  completed: number
  total: number
  currentTicker: string
  updatedSymbols: number
  updatedBars: number
  failedSymbols: number
  done: boolean
}

export interface VnUniverseSyncSummary {
  discoveredSymbols: number
  upsertedSymbols: number
  elapsedMs: number
}

export interface DoraemiTransportStatusDto {
  configured: boolean
  sourceFile: string | null
  baseUrl: string | null
  migration0006RequiredNote: string
  readyForCloudPush: boolean
}

export interface SyncStatusDto {
  deviceId: string
  syncFolder: string | null
  syncBundleExists: boolean
  autoSyncOnLaunch: boolean
  lastSyncedAtMs: number | null
  lastSyncedSha256: string | null
  dataDir: string
  configDir: string
  preSyncBackupCount: number
  doraemiTransport: DoraemiTransportStatusDto
}

export interface ExportBundleResponse {
  path: string
  payloadSha256: string
  exportedAtMs: number
  watchlistsCount: number
  drawingsCount: number
  layoutsCount: number
  alertsCount: number
}

export interface WorkspaceMergeReport {
  bundleSha256: string
  sourceDeviceId: string
  backupPath: string | null
  mergedKv: number
  mergedWatchlists: number
  mergedLayouts: number
  mergedPaneIndicators: number
  mergedTemplates: number
  mergedDrawings: number
  mergedAlerts: number
  mergedJournal: number
  mergedReplaySessions: number
  appliedTombstones: number
}

export interface BidirectionalFolderSyncResult {
  bundlePath: string
  pulledAndMerged: boolean
  mergeReport: WorkspaceMergeReport | null
  pushedSha256: string
  syncedAtMs: number
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
  updatedAtMs?: number
}

export interface PriceAlertRecord {
  id: string
  ticker: string
  alertKind?: 'price' | 'drawing' | 'indicator' | string
  condition: 'cross_above' | 'cross_below' | 'cross_any' | string
  targetPrice: number
  interval?: string | null
  paramsJson?: Record<string, unknown> | null
  soundEnabled?: boolean
  message: string
  status: 'armed' | 'triggered' | 'paused' | string
  frequency?: 'once' | 'once_per_bar_close' | string
  cooldownBars?: number
  lastFiredBarTs?: number | null
  createdAtMs: number
  triggeredAtMs: number | null
}

export interface IndicatorTemplateRecord {
  id: string
  name: string
  description: string
  indicators: SavedIndicatorSpec[]
  updatedAtMs: number
}

export interface SavedReplaySessionRecord {
  id: string
  name: string
  ticker: string
  baseInterval: IntervalKey
  trackedIntervals: IntervalKey[]
  cursorTimestampMs: number
  speedMultiplier: number
  blindMode: boolean
  brokerSnapshotJson: Record<string, unknown> | null
  updatedAtMs: number
}

export interface SymbolFlagRecord {
  ticker: string
  color: string
  updatedAtMs: number
}

export interface WorkspaceBootstrapResponse {
  settings: Record<string, unknown> | null
  layouts: SavedLayoutRecord[]
  watchlists: WatchlistGroup[]
  alerts: PriceAlertRecord[]
  indicatorTemplates?: IndicatorTemplateRecord[]
  replaySessions?: SavedReplaySessionRecord[]
  symbolFlags?: SymbolFlagRecord[]
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

export interface ChecklistState {
  trend_aligned: boolean
  level_confirmed: boolean
  catalyst_or_volume_ok: boolean
  stop_and_lot_sized: boolean
  band_and_t2_checked: boolean
  calm_state: boolean
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
  checklistJson?: ChecklistState | null
}

export interface ExecutionFill {
  fill_id: number
  order_id: number
  side: 'Buy' | 'Sell'
  reason: string
  quantity: number
  price: number
  fee: number
  realized_net_pnl: number | null
  base_index: number
  timestamp: number
}

export interface BrokerStateSnapshot {
  ticker: string
  interval: string
  venue: string
  boardLot: number
  contractMultiplier?: number
  allowsShort: boolean
  enforceVnT2LotLock: boolean
  markPrice: number
  markTimestampMs: number
  vnBand: VnBandLevels | null
  config: AccountConfig
  metrics: PortfolioMetrics
  openPosition: Position | null
  orders: EnrichedOrder[]
  fills?: ExecutionFill[]
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

export interface BlindDrillMeta {
  enabled: boolean
  aliasCode: string
  priceScale: number
  revealed: boolean
  realTicker: string
  cutDate: string
}

export interface ReplaySnapshotResponse {
  active: boolean
  status: 'Inactive' | 'Paused' | 'Playing' | 'Completed'
  ticker: string
  interval: string
  trackedIntervals?: string[]
  cursorIndex: number
  totalBars: number
  cutoffTimestampMs: number | null
  speedMultiplier: number
  visibleBars: OhlcvBar[]
  steppedBar: OhlcvBar | null
  steppedBarsByInterval?: Record<string, OhlcvBar>
  cutoffBar?: OhlcvBar | null
  prevCutoffClose?: number | null
  vnBand?: VnBandLevels | null
  triggeredAlerts?: PriceAlertRecord[]
  broker: BrokerStateSnapshot | null
  blindDrill?: BlindDrillMeta | null
}

// ─── Strategy Tester Types ─────────────────────────────────────────────────

export interface CustomClauseInput {
  catalogId: string
  op?: '>' | '>=' | '<' | '<=' | 'cross_above' | 'cross_below'
  threshold?: number
  boolExpected?: boolean
}

export interface RuleGroupInput {
  combinator?: 'and' | 'or'
  clauses: CustomClauseInput[]
}

export interface CustomRulesInput {
  name?: string
  entryCombinator?: 'and' | 'or'
  entryClauses: CustomClauseInput[]
  exitCombinator?: 'and' | 'or'
  exitClauses?: CustomClauseInput[]
  minHealthScore?: number | null
  longEntryInterOp?: 'and' | 'or'
  longEntryGroups?: RuleGroupInput[]
  longExitInterOp?: 'and' | 'or'
  longExitGroups?: RuleGroupInput[]
  shortEntryInterOp?: 'and' | 'or'
  shortEntryGroups?: RuleGroupInput[]
  shortExitInterOp?: 'and' | 'or'
  shortExitGroups?: RuleGroupInput[]
}

export type StrategyPresetSlug =
  | 'qmv_s1'
  | 'qmv_s2'
  | 'qmv_s3'
  | 'qmv_s4'
  | 'qmv_s5'
  | 'qmv_s6'
  | 'sma_ema'
  | 'rsi_mean_rev'
  | 'macd'
  | 'bb_squeeze'
  | 'donchian_atr'
  | 'rsi_bb'
  | 'custom'

export interface StrategyBacktestInput {
  ticker: string
  interval: string
  preset: StrategyPresetSlug
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
  maxLeverage?: number | null
  customRules?: CustomRulesInput | null
  // QMV Strategy Parameters (S1..S6 & QMV Risk Management)
  qmvSupportWindow?: number | null
  qmvRvolConfirm?: number | null
  qmvTrailMode?: number | null
  qmvDivOsc?: number | null
  qmvConfirmMode?: number | null
  qmvIchiVariant?: number | null
  qmvFlowHorizonPair?: number | null
  qmvFlowLowZone?: number | null
  qmvFlowHighZone?: number | null
  qmvUseVnindexGate?: boolean | null
  qmvMinPillars?: number | null
  supportStopLookback?: number | null
  qmvWaveTrailing23?: boolean | null
  // Wave 5 — Strategy Tester 2.0 Risk & Magnifier Controls
  maxPyramiding?: number | null
  scaleOutPct?: number | null
  tp1Pct?: number | null
  trailingStopAtrMult?: number | null
  trailingMaPeriod?: number | null
  trailingKijunPeriod?: number | null
  useBarMagnifier?: boolean | null
}

export interface PortfolioConstituentAttribution {
  symbol: string
  instrument_id: string
  timeframe: string
  final_weight_pct: number
  standalone_return_pct: number
  standalone_max_dd_pct: number
  weighted_return_contribution_pct: number
  num_trades: number
}

export interface PortfolioBacktestInput {
  backtest: StrategyBacktestInput
  tickers: string[]
  qmvGroup?: string | null
  weighting?: 'equal' | 'inverse_vol'
}

export interface PortfolioBacktestResponse {
  presetLabel: string
  interval: string
  weightingLabel: string
  constituentCount: number
  initialCapital: number
  tearsheet: BacktestTearsheet
  avgConstituentMaxDdPct: number
  diversificationDdReductionPct: number
  equityCurve: StrategyEquityPointDto[]
  constituents: PortfolioConstituentAttribution[]
  survivorBiasCaveat: string | null
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

export interface MonthlyReturnCellDto {
  year: number
  month: number
  returnPct: number
  tradesCount: number
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
  monthlyReturns?: MonthlyReturnCellDto[]
  barMagnifierBars?: number
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
  barMagnifierBars?: number
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

// ─── QMV Stock Screener (123 Signals) Types ────────────────────────────────

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
  isQmvCore?: boolean
  isOhlcvProxy?: boolean
}

export type ScreenerExchangeFilter =
  | 'ALL'
  | 'ALL_VN'
  | 'HOSE'
  | 'HNX'
  | 'UPCOM'
  | 'QMV30'
  | 'QMV60'
  | 'DT1'
  | 'DT2'
  | 'DT3'
  | 'WATCHLIST'

export interface StockScreenerInput {
  exchangeFilter: ScreenerExchangeFilter
  tickers?: string[]
  signalId?: string
  signalIds?: string[]
  qmvBasket?: string | null
  sectorGroupFilter?: string | null
  sectorFilter?: string | null
  moneyTierFilter?: string | null
  priceTierFilter?: string | null
  minAvgVol20?: number | null
  categoryFilter?: string
  regimeFilter?: string
  searchQuery?: string
  maxSymbols?: number
  minRsi?: number | null
  maxRsi?: number | null
  minRvol?: number | null
  minChangePct?: number | null
  maxChangePct?: number | null
  macdFilter?: 'all' | 'positive' | 'negative' | null
  minHealthScore?: number | null
  autoRefreshStale?: boolean | null
  // Wave 4 MTF + Fundamentals + Yumeko Blackbox filters
  weeklyRegimeFilter?: string | null
  enableMtf1h?: boolean | null
  maxPe?: number | null
  maxPb?: number | null
  minRoe?: number | null
  minMarketCapBn?: number | null
  onlyMoneyInToday?: boolean | null
  minQmvTmc?: number | null
  minQmvDspi?: number | null
  onlyQmvReversalTPlus?: boolean | null
}

export interface ScreenerRowDto {
  ticker: string
  exchange: string
  symbol: string
  description: string
  lastClose: number
  changePct: number
  lastVolume: number
  avgVol20?: number | null
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
  qmvSectorGroup?: string
  qmvSector?: string
  qmvSectorLabel?: string
  moneyTier?: string
  inQmv30?: boolean
  inQmv60?: boolean
  priceTier?: string
  qmvFlowZ50T05?: number | null
  qmvFlowZ50T20?: number | null
  // Wave 4 MTF + Fundamentals + Yumeko QMV Real columns
  weeklyRegime?: string
  weeklyRegimeShort?: string
  hourlyTiming?: string | null
  mtfAligned?: boolean
  pe?: number | null
  pb?: number | null
  roe?: number | null
  marketCapBn?: number | null
  qmvRealTmc?: number | null
  qmvRealDspi?: number | null
  qmvRealMoneyInToday?: boolean
  qmvRealProvenance?: boolean
}

export interface StockScreenerResponse {
  totalCatalogSignals: number
  scannedSymbols: number
  matchedSymbols: number
  elapsedMs: number
  expectedSessionDate?: string | null
  freshSymbolsCount?: number
  staleSymbolsCount?: number
  medianCachedDate?: string | null
  rows: ScreenerRowDto[]
}

export interface ChartPaneConfig {
  slotId: string
  ticker: string
  interval: IntervalKey
  chartStyle: ChartStyleKind
  priceScale: PriceScaleKind
  adjustment?: 'splits' | 'unadjusted'
  showVnBands: boolean
  showCountdown: boolean
  indicators: SavedIndicatorSpec[]
}

// ─── AFC — Analog Forecast Cone Types (SPEC-SUMI-AFC-V1) ──────────────────

export interface AnalogConeStepDto {
  step: number
  projectedTimestampMs: number
  p10: number
  p25: number
  p50: number
  p75: number
  p90: number
  p10RetPct: number
  p25RetPct: number
  p50RetPct: number
  p75RetPct: number
  p90RetPct: number
  cumulativeFloor: number | null
  cumulativeCeiling: number | null
}

export interface AnalogMatchCaseDto {
  ticker: string
  symbol: string
  entryTimestampMs: number
  entryDate: string
  similarityPct: number
  distance: number
  weight: number
  realizedT5RetPct: number | null
  realizedT10RetPct: number | null
}

export interface AnalogConeForecastDto {
  steps: AnalogConeStepDto[]
  winRatePct: number
  probGainGt3Pct: number
  probTailDrop6Pct: number
  skewRatio: number
  expectedReturnP50Pct: number
  expectedReturnMeanPct: number
  queryYz20Vol: number
  pitCandidateCount: number
  fastCandidateCount: number
  topAnalogs: AnalogMatchCaseDto[]
}

export interface AnalogConeResponseDto {
  ticker: string
  exchange: string
  symbol: string
  venue: string
  interval: string
  supportedInterval: boolean
  replayLocked: boolean
  asOfTimestampMs: number | null
  asOfDate: string | null
  anchorClose: number | null
  horizon: number
  topK: number
  weightProfile: 'MarketAware' | 'EndogenousOnly' | string
  queryLatencyUs: number
  statusMessage: string | null
  forecast: AnalogConeForecastDto | null
}

export interface AnalogConeQueryParams {
  ticker: string
  interval: IntervalKey | string
  horizon?: number
  topK?: number
  useMarketContext?: boolean
  asOfTimestampMs?: number | null
}

// ─── Yumeko v3.0 Read-Only Top-Down Bridge & KT01 Evidence Types (Wave 2) ──

export interface YumekoBridgeStatus {
  available: boolean
  sqlitePath: string
  qmvSharedPath: string | null
  latestAsOfDates: string[]
  qmvTechnicalSymbols: number
  qmvActiveMoneySymbols: number
  qmvForeignFlowSymbols: number
  valuationSymbols: number
  dnseHistorySymbols: number
  macroObservationsCount: number
  portfolioSnapshotsCount: number
  watchlistSymbolsCount: number
  note: string
}

export interface MacroObservationDto {
  indicatorCode: string
  periodDate: string
  releaseDate: string
  value: number | null
  valueText: string | null
  unit: string
  sourceName: string
}

export interface SectorBreadthSnapshotDto {
  icb2Code: string
  icb2Name: string
  changePct: number | null
  advancers: number | null
  decliners: number | null
  unchanged: number | null
  totalStocks: number | null
  advancingRatioPct: number | null
}

export interface Vm01MarketContextDto {
  effectiveAsOf: string | null
  marketSentiment: number | null
  marketBbm05: number | null
  marketBbm20: number | null
  marketBbmz05: number | null
  marketBbmz20: number | null
  marketForeignNetValue: number | null
  topSectors: SectorBreadthSnapshotDto[]
  macroHighlights: MacroObservationDto[]
}

export interface QmvStockBlackboxSnapshotDto {
  contentDate: string
  sourceProvenance: 'qmv_real' | 'local_proxy' | string
  isProviderProprietary: boolean
  closeVnd: number | null
  closeKvnd: number | null
  tmc: number | null
  tma20: number | null
  tma50: number | null
  dspi: number | null
  dspiMa5: number | null
  dspiMa20: number | null
  mpicpAll: number | null
  mpicpZ: number | null
  dm: number | null
  ds: number | null
  tradingScore: number | null
  priceTrend: string | null
  stopLossVnd: number | null
  targetPrice1Vnd: number | null
  targetPrice2Vnd: number | null
  moneyInToday: boolean
  moneyIn2Sessions: boolean
  moneyIn3Sessions: boolean
  reversalUpTPlus: boolean
  reversalUpSwing: boolean
  activeFlags: string[]
  qmvRating: string | null
  tradingSuitability: string | null
  divisionAllocationGroup: string | null
  divisionMoneyFlowLabel: string | null
  inQmv60: boolean
}

export interface ValuationQuarterDto {
  period: string
  contentDate: string | null
  sourceFamily: string
  pe: number | null
  pb: number | null
  ps: number | null
  evEbitda: number | null
  eps: number | null
  bvps: number | null
  roe: number | null
  roa: number | null
  debtToEquity: number | null
}

export interface Dm01HoldingDto {
  asOf: string
  effectiveAsOf: string
  sha256Verified: boolean
  quantity: number
  avgCostVnd: number
  currentPriceVnd: number
  currentValueVnd: number
  unrealizedPnlVnd: number
  unrealizedPnlPct: number
  weightPct: number
  daysHeld: number | null
  sector: string | null
}

export interface Dm01WatchlistEntryDto {
  symbol: string
  strategyTag: string
  note: string
  targetBuyVnd: number | null
  invalidationLevelVnd: number | null
  updatedAt: string
}

export interface EpisodicThesisDto {
  decisionId: string
  asOf: string
  functionName: string
  gateStep: string | null
  stance: string
  confidence: string
  thesisSummary: string
}

export interface TopDownSymbolContextDto {
  symbol: string
  requestedAsOf: string | null
  effectiveAsOf: string | null
  bridgeAvailable: boolean
  flowProvenance: 'qmv_real' | 'local_proxy' | string
  vm01Market: Vm01MarketContextDto
  hp01Blackbox: QmvStockBlackboxSnapshotDto | null
  dn01Valuations: ValuationQuarterDto[]
  dm01RealHolding: Dm01HoldingDto | null
  dm01YumekoWatchlist: Dm01WatchlistEntryDto | null
  episodicTheses: EpisodicThesisDto[]
  dnsePitAvailable: boolean
  gaps: string[]
}

export interface QmvStockBbPointDto {
  date: string
  timestampMs: number
  closeVnd: number | null
  tmc: number | null
  tma20: number | null
  tma50: number | null
  dspi: number | null
  dspiMa5: number | null
  dspiMa20: number | null
  mpicpAll: number | null
  mpicpZ: number | null
  dm: number | null
  ds: number | null
  stopLossVnd: number | null
  tp1Vnd: number | null
  tp2Vnd: number | null
}

export interface QmvActiveMoneyPointDto {
  date: string
  timestampMs: number
  priceVnd: number | null
  shark: number | null
  wolf: number | null
  sheep: number | null
  trendShark: number | null
  trendWolf: number | null
  trendSheep: number | null
  volume: number | null
}

export interface QmvForeignFlowPointDto {
  date: string
  timestampMs: number
  priceVnd: number | null
  buyValue: number | null
  sellValue: number | null
  netValue: number | null
  buyVolume: number | null
  sellVolume: number | null
  netVolume: number | null
}

export interface QmvRealChartSeriesDto {
  symbol: string
  asOf: string | null
  sourceProvenance: 'qmv_real' | 'local_proxy' | string
  isProviderProprietary: boolean
  stockBb: QmvStockBbPointDto[]
  activeMoney: QmvActiveMoneyPointDto[]
  foreignFlow: QmvForeignFlowPointDto[]
}

export type Kt01GateVerdict = 'PASS' | 'WATCH' | 'WAIT_PULLBACK' | 'REJECT'

export interface KT01GateVerdictV1 {
  schemaVersion: string
  verdictId: string
  symbol: string
  ticker: string
  asOf: string
  generatedAtUtc: string
  advisoryOnly: boolean
  flowProvenance: string
  verdict: Kt01GateVerdict
  verdictLabelVi: string
  confidenceScore: number
  mtfAlignment: {
    weeklyTrend: string
    dailyRegime: string
    dailyHealthScore: number | null
    hourlyTiming: string | null
    ma20Vnd: number | null
    ma50Vnd: number | null
    ma200Vnd: number | null
    rsi14: number | null
    rvol20: number | null
    activeSignals: string[]
  }
  executionLevels: {
    lastCloseVnd: number
    lastCloseKvnd: number
    entryZoneLowVnd: number
    entryZoneHighVnd: number
    stopLossVnd: number
    stopLossKvnd: number
    takeProfit1Vnd: number
    takeProfit1Kvnd: number
    takeProfit2Vnd: number
    takeProfit2Kvnd: number
    riskPct: number
    rewardTp1Pct: number
    rrRatioTp1: number
    levelsSource: string
  }
  afcSummary: {
    horizonBars: number
    winRatePct: number
    expectedP50Pct: number
    p10Pct: number
    p90Pct: number
    probTailDrop6Pct: number
  } | null
  dm01IsRealHolding: boolean
  dm01IsYumekoWatchlist: boolean
  invalidationTriggers: string[]
  caveats: string[]
  verdictSha256: string
}

export interface ExportedVerdictReceipt {
  verdict: KT01GateVerdictV1
  jsonPath: string
  markdownPath: string
  yumekoDropzonePath: string | null
  markdownSummary: string
}

export interface ExportedVerdictListItem {
  verdictId: string
  symbol: string
  asOf: string
  verdict: Kt01GateVerdict
  confidenceScore: number
  flowProvenance: string
  jsonPath: string
  markdownPath: string
  generatedAtUtc: string
}

export interface ResearchExportVerificationDto {
  exportDir: string
  manifestFound: boolean
  verifiedFiles: number
  failedFiles: string[]
  asOf: string | null
  valid: boolean
}

export interface DeepLinkPayload {
  rawUrl: string
  action: string
  ticker: string
  symbol: string
  asOf: string | null
  interval: IntervalKey | null
  tab: string | null
}

// ─── Wave 3 Quantitative Analytics, Alerts 2.0, Trade Planner, DOM, Debrief ──

export interface AlertTriggerLogRecord {
  id: string
  alertId: string
  ticker: string
  alertKind: string
  condition: string
  triggerPrice: number
  barTimestampMs: number
  triggeredAtMs: number
  message: string
  deliveredChannels: string
}

export interface AlertDeliveryConfig {
  osNotificationEnabled: boolean
  telegramBotToken: string
  telegramChatId: string
  webhookUrl: string
  watchlistBackgroundScanEnabled: boolean
}

export type RrgQuadrant = 'Leading' | 'Weakening' | 'Lagging' | 'Improving'

export interface SymbolBreadthRotation {
  instrument_id: string
  symbol: string
  preset_label: string
  last_close: number
  return_1bar_pct: number
  traded_value: number
  buy_flow_ratio_pct: number
  above_sma20: boolean | null
  above_ema50: boolean | null
  rs_ratio: number
  rs_momentum: number
  quadrant: RrgQuadrant
  trail: Array<[number, number]>
}

export interface MarketBreadthSnapshot {
  benchmark_symbol: string
  evaluated_timestamp: number | null
  participating_count: number
  advancers: number
  decliners: number
  unchanged: number
  adv_dec_ratio: number
  pct_above_sma20: number
  pct_above_ema50: number
  total_traded_value: number
  aggregate_buy_value: number
  aggregate_sell_value: number
  money_flow_buy_pct: number
  vn30_participating_count: number
  vn30_money_flow_buy_pct: number
  quadrant_counts: [number, number, number, number]
  rotations: SymbolBreadthRotation[]
}

export interface SectorHeatmapTileDto {
  ticker: string
  symbol: string
  sectorCode: string
  sectorLabel: string
  lastClose: number
  changePct: number
  tradedValue: number
  buyFlowRatioPct: number
  rsRatio: number
  rsMomentum: number
  quadrant: RrgQuadrant
  inQmv30: boolean
  inQmv60: boolean
}

export interface SectorHeatmapGroupDto {
  sectorCode: string
  sectorLabel: string
  sectorGroupLabel: string
  stockCount: number
  advancers: number
  decliners: number
  unchanged: number
  avgChangePct: number
  weightedChangePct: number
  totalTradedValue: number
  avgBuyFlowPct: number
  stocks: SectorHeatmapTileDto[]
}

export interface MarketBreadthRrgResponse {
  scope: string
  asOfDate: string | null
  replayLocked: boolean
  snapshot: MarketBreadthSnapshot
  sectors: SectorHeatmapGroupDto[]
  elapsedMs: number
}

export interface TradePlanRequestDto {
  ticker: string
  side: 'LONG' | 'SHORT'
  accountEquity: number
  availableCash: number
  riskPct: number
  maxPositionPct?: number | null
  entryPrice: number
  stopLoss: number
  targetPrice: number
  referencePrice?: number | null
  commissionBps?: number | null
  taxSellBps?: number | null
  slippageBps?: number | null
  entryTimestampMs?: number | null
}

export interface TradePlanResponseDto {
  ticker: string
  venue: string
  side: 'LONG' | 'SHORT' | string
  snappedEntry: number
  snappedStop: number
  snappedTarget: number
  breakevenPrice: number
  maxRiskAmount: number
  plannedQuantity: number
  positionNotional: number
  marginRequired: number
  plannedRiskAmount: number
  plannedRiskPctActual: number
  estimatedNetProfit: number
  estimatedRoundTripFriction: number
  grossRMultiple: number
  netRMultiple: number
  requiredWinRatePct: number
  settlementTimestampMs: number | null
  affordable: boolean
  warnings: string[]
}

export interface DomLadderRow {
  price: number
  tick_size: number
  band_state: 'AboveCeiling' | 'Ceiling' | 'Reference' | 'Floor' | 'BelowFloor' | 'Normal' | string
  is_at_mark: boolean
  is_at_position_entry: boolean
  synth_bid_depth: number
  synth_ask_depth: number
  traded_buy_volume: number
  traded_sell_volume: number
  net_delta: number
  working_buy_qty: number
  working_sell_qty: number
  working_order_ids: number[]
}

export interface DomLadderSnapshot {
  data_fidelity: string
  venue: string
  resolved_unit: string
  price_decimals: number
  mark_price: number
  center_price: number
  daily_band: {
    reference_price: number
    ceiling_price: number
    floor_price: number
    band_pct: number
  } | null
  rows: DomLadderRow[]
  max_row_depth: number
  max_row_volume: number
  cumulative_buy_volume: number
  cumulative_sell_volume: number
  bid_ask_imbalance_pct: number
  poc_price: number | null
  causal_bar_count: number
  last_causal_timestamp: number | null
}

export interface JournalSetupBreakdown {
  setup: string
  trade_count: number
  win_rate_pct: number
  net_pnl: number
  avg_r: number | null
}

export interface JournalAnalytics {
  visible_entry_count: number
  planned_count: number
  closed_count: number
  win_count: number
  loss_count: number
  win_rate_pct: number
  total_net_pnl: number
  profit_factor: number | null
  expectancy_r: number | null
  sqn: number | null
  discipline_score_pct: number
  mistake_trade_count: number
  cost_of_mistakes: number
  avg_entry_drift_pct: number | null
  current_streak: number
  setup_attribution: JournalSetupBreakdown[]
}

export interface JournalEquityPointDto {
  tradeIndex: number
  timestampMs: number
  cumulativePnl: number
  tradePnl: number
}

export interface JournalAnalyticsResponseDto {
  ticker: string
  analytics: JournalAnalytics
  equityPoints: JournalEquityPointDto[]
}

export interface CoachingCallout {
  severity: 'Positive' | 'Neutral' | 'Caution' | 'Critical' | string
  title: string
  detail: string
}

export interface MistakeCostBreakdown {
  mistake: string
  trade_count: number
  decision_note_count: number
  net_pnl_impact: number
  adverse_cost_abs: number
}

export interface SetupEdgeBreakdown {
  setup: string
  trade_count: number
  win_count: number
  win_rate_pct: number
  net_pnl: number
  avg_r: number | null
}

export interface SessionDebriefReport {
  symbol: string
  timeframe: string
  closed_trades: number
  win_count: number
  loss_count: number
  breakeven_count: number
  win_rate_pct: number
  total_net_pnl: number
  total_return_pct: number
  avg_return_pct: number
  profit_factor: number | null
  expectancy_pnl: number | null
  net_r_multiple: number
  avg_r_multiple: number | null
  max_drawdown_pct: number
  total_opened_positions: number
  sl_adherent_positions: number
  sl_adherence_pct: number | null
  mistake_breakdown: MistakeCostBreakdown[]
  setup_breakdown: SetupEdgeBreakdown[]
  callouts: CoachingCallout[]
}

export interface ExportBacktestHtmlResponse {
  filePath: string
  html: string
}

export interface SweepVariantRow {
  rank: number
  label: string
  config: Record<string, unknown>
  tearsheet: BacktestTearsheet
  in_sample_return_pct: number | null
  out_of_sample_return_pct: number | null
  robustness: string | null
}

export interface ParameterSweepResponseDto {
  ticker: string
  interval: string
  presetLabel: string
  rankingLabel: string
  variants: SweepVariantRow[]
}

export interface PresetBattleItemDto {
  rank: number
  presetSlug: StrategyPresetSlug | string
  presetLabel: string
  paramSummary: string
  tearsheet: BacktestTearsheet
  robustness: string | null
}

export interface PresetsBattleResponseDto {
  ticker: string
  interval: string
  rankingLabel: string
  rows: PresetBattleItemDto[]
}

// ─── Wave 4: Fundamentals (DN01/TV), Multi-Symbol Compare & Saved Screens ───

export interface SymbolFundamentalsResponse {
  ticker: string
  symbol: string
  exchange: string
  source: 'yumeko_dn01' | 'tradingview_cache' | 'none' | string
  sector: string | null
  industry: string | null
  marketCapBasic: number | null
  pe: number | null
  pb: number | null
  ps: number | null
  evEbitda: number | null
  roe: number | null
  roa: number | null
  eps: number | null
  bvps: number | null
  debtToEquity: number | null
  totalRevenue: number | null
  totalSharesOutstanding: number | null
  earningsReleaseDate: number | null
  businessDescription: string | null
  quarterlyValuations: ValuationQuarterDto[]
}

export interface ComparePointDto {
  timestamp: number
  pct: number
}

export interface CompareSeriesItemDto {
  ticker: string
  symbol: string
  points: ComparePointDto[]
}

export interface CompareSeriesResponse {
  baseTicker: string
  series: CompareSeriesItemDto[]
}

export interface SavedScreenRecord {
  id: string
  name: string
  filterJson: Record<string, unknown>
  updatedAtMs: number
}
