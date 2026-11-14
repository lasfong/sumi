use std::collections::BTreeMap;

use egui::{Button, Color32, CornerRadius, Frame, Margin, RichText, Stroke, Vec2, Visuals};
use serde::{Deserialize, Serialize};

use crate::alerts::{
    AlertCondition, AlertEngine, AlertError, AlertFrequency, AlertId, AlertStatus, CrossDirection,
    truncate_utf8_bytes,
};
use crate::backtest::{
    BacktestConfig, BacktestRunOptions, BacktestRunResult, MIN_STATISTICAL_SAMPLE_TRADES,
    MultiSymbolBattleReport, ParameterSweepReport, PortfolioBacktestReport,
    PortfolioUniverseFilter, PortfolioWeightingMode, PositionSizingMode, PresetBattleRow,
    RobustnessVerdict, StrategyDirectionMode, StrategyPresetKind, SweepRankingMetric,
    WalkForwardConfig, WalkForwardReport, WalkForwardVerdict, is_bucket_incomplete_at_close,
    map_equity_curve_to_candles, run_all_presets_battle, run_backtest_with_options,
    run_multi_symbol_battle_with_close_cutoff, run_parameter_sweep, run_portfolio_backtest,
    run_walk_forward_validation,
};
#[cfg(not(target_arch = "wasm32"))]
use crate::backtest::{StopTakeConfig, StrategyParams};
use crate::chart::{
    ChartDisplayConfig, ChartRenderReport, ChartReplayTradingOverlay, ChartViewportState,
    EquityCurveOverlay, palette, render_workstation_chart_full,
};
use crate::data::{
    AggregationError, DataFileFormat, DataLoadError, DataLoadOptions, DataLoadReport,
    DataSourcePreset, Timeframe, WatchlistFeedEntry, format_utc_date, format_utc_datetime_hm,
    generate_synthetic_ohlcv, load_bundled_watchlist_universe, load_ohlcv_bytes, load_ohlcv_csv,
    load_ohlcv_json, load_preset_dataset, remap_timestamp_to_bar_index, resolve_anchor_timestamp,
    resolve_benchmark_series_for_window,
};
use crate::drawings::{
    DrawingAnchor, DrawingColorPreset, DrawingGeometry, DrawingPaneTarget, DrawingStore,
    DrawingStyle, DrawingTool, FibDirection, LineDashStyle, MagnetMode,
};
use crate::indicators::OscillatorPaneId;
use crate::indicators::{
    IndicatorCache, IndicatorConfig, SeriesBlockIndex, compute_rebased_comparison_series,
};
use crate::journal::{
    ChecklistState, DailyPriceBand, ExchangeVenue, JournalEntrySource, JournalEntryStatus,
    MistakeTag, SetupTag, TradeJournalStore, TradePlanComputed, TradePlanInput, VnPriceUnit,
    compute_daily_price_band, compute_trade_plan,
};
use crate::model::Candle;
use crate::replay::{
    ReplayController, ReplaySpeed, ReplayStatus, ReplayStepUnit, ReplayTransition,
};
use crate::scanner::{
    DEFAULT_BLIND_PRACTICE_LOOKBACK_BARS, HistoricalSignalHit, MarketRegime, ScanSignalKind,
    ScannerFilter, ScannerReplayIntent, ScannerSignalContext, SignalDirection, SymbolScanRow,
    WatchlistSortColumn, evaluate_symbol_scan_and_signals, filter_and_sort_watchlist,
    filter_historical_signals,
};
use crate::trading::{
    OrderKind, OrderRequest, OrderSide, PositionSide, TradingSession, extract_ticket_from_drawing,
};
use crate::viewport::PriceScaleMode;

/// Order-type tab selection in the workstation Paper Trading ticket.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TicketOrderKind {
    #[default]
    Market,
    Limit,
    Stop,
}

impl TicketOrderKind {
    pub const ALL: [Self; 3] = [Self::Market, Self::Limit, Self::Stop];

    #[inline]
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Market => "Market",
            Self::Limit => "Limit",
            Self::Stop => "Stop",
        }
    }
}

/// Active sub-view inside the left Multi-Symbol Watchlist & Technical Scanner dock (`Milestone R08`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ScannerDockTab {
    #[default]
    Watchlist,
    Signals,
}

/// Active sub-view inside the unified right-side Paper Trading, Alerts, Strategy Lab & Journal dock (`Milestones R08–R10`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum RightDockTab {
    #[default]
    Trading,
    Alerts,
    StrategyLab,
    Journal,
}

/// Active sub-view inside the `Strategy Lab` right-side dock (`Milestones R09–R10`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum StrategyLabSubTab {
    #[default]
    Tearsheet,
    Trades,
    Sweep,
    Battle,
    WalkForward,
}

impl StrategyLabSubTab {
    pub const ALL: [Self; 5] = [
        Self::Tearsheet,
        Self::Trades,
        Self::Sweep,
        Self::Battle,
        Self::WalkForward,
    ];

    #[inline]
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::Tearsheet => "Stats",
            Self::Trades => "Trades",
            Self::Sweep => "Sweep",
            Self::Battle => "Battle",
            Self::WalkForward => "WFO",
        }
    }
}

/// Active series plotted inside the synchronized `OscillatorPaneId::Equity` sub-pane (`Milestone R10`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum EquityPaneSource {
    #[default]
    SingleAssetBacktest,
    WalkForwardOos,
    MultiAssetPortfolio,
}

impl EquityPaneSource {
    #[inline]
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::SingleAssetBacktest => "Single BT",
            Self::WalkForwardOos => "WFO OOS",
            Self::MultiAssetPortfolio => "Portfolio",
        }
    }
}

/// Alert condition template selected in the Alerts Manager quick-create form (`Milestone R08`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AlertDraftKind {
    #[default]
    PriceLevel,
    RsiThreshold,
    MacdZeroCross,
    SelectedDrawing,
}

impl AlertDraftKind {
    pub const ALL: [Self; 4] = [
        Self::PriceLevel,
        Self::RsiThreshold,
        Self::MacdZeroCross,
        Self::SelectedDrawing,
    ];

    #[inline]
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::PriceLevel => "Price",
            Self::RsiThreshold => "RSI(14)",
            Self::MacdZeroCross => "MACD 0",
            Self::SelectedDrawing => "Drawing",
        }
    }
}

/// Action category badge displayed in the `Cmd+K` / `Ctrl+K` Command Palette (`Milestone R08`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaletteCategory {
    Symbol,
    Timeframe,
    Indicator,
    Drawing,
    Alert,
    Backtest,
    Replay,
    Trading,
    Scale,
    Workspace,
}

impl PaletteCategory {
    #[inline]
    #[must_use]
    pub const fn badge_label(self) -> &'static str {
        match self {
            Self::Symbol => "SYMBOL",
            Self::Timeframe => "TF",
            Self::Indicator => "INDIC",
            Self::Drawing => "DRAW",
            Self::Alert => "ALERT",
            Self::Backtest => "STRAT",
            Self::Replay => "REPLAY",
            Self::Trading => "TRADE",
            Self::Scale => "SCALE",
            Self::Workspace => "VIEW",
        }
    }

    #[inline]
    #[must_use]
    pub const fn badge_color(self) -> Color32 {
        match self {
            Self::Symbol => Color32::from_rgb(41, 182, 246),
            Self::Timeframe => Color32::from_rgb(77, 208, 225),
            Self::Indicator => Color32::from_rgb(179, 136, 255),
            Self::Drawing => Color32::from_rgb(255, 193, 7),
            Self::Alert => palette::ALERT_ARMED_AMBER,
            Self::Backtest => palette::EQUITY_ABOVE_LINE,
            Self::Replay => palette::REPLAY_CUT_GOLD,
            Self::Trading => palette::TRADE_ENTRY_BLUE,
            Self::Scale => Color32::from_rgb(38, 198, 164),
            Self::Workspace => palette::TEXT_PRIMARY,
        }
    }
}

/// Executable workstation action dispatched from the `Cmd+K` Command Palette (`Milestones R08–R09`).
#[derive(Debug, Clone, PartialEq)]
pub enum PaletteAction {
    SelectWatchlistSymbol {
        preset: DataSourcePreset,
        symbol: String,
    },
    SetTimeframe(Timeframe),
    ToggleIndicator(&'static str),
    ToggleCompareOverlay,
    CycleCompareBenchmark,
    SelectDrawingTool(DrawingTool),
    ToggleMagnet,
    ToggleScannerDock,
    OpenAlertsTab,
    OpenTradingTab,
    OpenJournalTab,
    OpenStrategyLab,
    ToggleEquityPane,
    ToggleBacktestTradesOnChart,
    SelectStrategyPreset(StrategyPresetKind),
    RunParameterSweep,
    RunUniverseStrategyBattle,
    RunWalkForwardValidation,
    RunPortfolioBacktest,
    DrawRiskRewardFromPlanner,
    SyncPaperTradesToJournal,
    ExportJournalCsv,
    QuickPriceAlert {
        price: f64,
        direction: CrossDirection,
    },
    ToggleReplayBar,
    CutReplay70Pct,
    StepReplayForward,
    StepReplayBackward,
    ExitReplay,
    SetScaleMode(PriceScaleMode),
    FitContent,
    ResetView,
}

/// Ranked candidate row inside the `Cmd+K` Command Palette (`Milestone R08`).
#[derive(Debug, Clone)]
pub struct PaletteCandidate {
    pub category: PaletteCategory,
    pub title: String,
    pub subtitle: String,
    pub keywords: String,
    pub score: i32,
    pub action: PaletteAction,
}

/// Transient modal state for the `Cmd+K` / `Ctrl+K` Command Palette (`Milestone R08`).
#[derive(Debug, Clone, Default)]
pub struct CommandPaletteState {
    pub open: bool,
    pub query: String,
    pub selected_index: usize,
    pub request_focus: bool,
}

type BacktestSyncKey = (u64, usize, Option<i64>, Option<i64>, bool, u64);
type SweepSyncKey = (u64, Option<i64>, bool, u64, SweepRankingMetric);
type BattleSyncKey = (String, u64, Timeframe, Option<i64>, u64, SweepRankingMetric);
type WalkForwardSyncKey = (u64, usize, Option<i64>, Option<i64>, bool, u64, u64);
type PortfolioSyncKey = (
    String,
    u64,
    Timeframe,
    Option<i64>,
    u64,
    PortfolioUniverseFilter,
    PortfolioWeightingMode,
);

#[derive(Debug, Clone)]
struct MappedPortfolioCurveCache {
    dataset_generation: u64,
    timeframe: Timeframe,
    replay_close_cutoff: Option<i64>,
    candle_len: usize,
    last_timestamp: Option<i64>,
    title_label: String,
    baseline_cash: f64,
    equity_values: Vec<Option<f64>>,
    equity_idx: SeriesBlockIndex,
    underwater_values: Vec<Option<f64>>,
}

/// Primary application state for the Sumi Rust Trading Workstation (`Milestone R09`).
#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct SumiWorkstationApp {
    pub symbol: String,
    pub timeframe: String,
    pub data_source: DataSourcePreset,
    pub base_timeframe: Timeframe,
    pub active_timeframe: Timeframe,
    pub import_path_input: String,
    pub show_grid: bool,
    pub show_crosshair: bool,
    pub show_volume: bool,
    pub dataset_bar_count: usize,
    pub viewport: ChartViewportState,
    pub indicators: IndicatorConfig,
    #[serde(default)]
    pub compare_benchmark: Option<String>,
    #[serde(default)]
    pub show_compare_overlay: bool,
    pub drawings: DrawingStore,
    #[serde(default)]
    drawings_by_instrument: BTreeMap<String, DrawingStore>,
    pub replay: ReplayController,
    pub trading: TradingSession,
    #[serde(default)]
    replay_trading_by_instrument: BTreeMap<String, (ReplayController, TradingSession)>,
    pub alerts: AlertEngine,
    #[serde(default)]
    alerts_by_instrument: BTreeMap<String, AlertEngine>,
    pub scanner_context: Option<ScannerSignalContext>,
    #[serde(default)]
    scanner_context_by_instrument: BTreeMap<String, ScannerSignalContext>,
    pub show_replay_bar: bool,
    pub show_trading_panel: bool,
    pub show_alerts_panel: bool,
    pub show_strategy_panel: bool,
    pub show_journal_panel: bool,
    pub show_equity_pane: bool,
    pub equity_pane_source: EquityPaneSource,
    pub show_backtest_trades: bool,
    pub right_dock_tab: RightDockTab,
    pub strategy_lab_tab: StrategyLabSubTab,
    pub backtest_config: BacktestConfig,
    pub sweep_ranking: SweepRankingMetric,
    pub wf_config: WalkForwardConfig,
    pub portfolio_universe_filter: PortfolioUniverseFilter,
    pub portfolio_weighting: PortfolioWeightingMode,
    pub risk_venue_override: Option<ExchangeVenue>,
    pub risk_plan_input: TradePlanInput,
    pub journal: TradeJournalStore,
    pub journal_draft_setup: SetupTag,
    pub journal_draft_conviction: u8,
    pub journal_draft_checklist: ChecklistState,
    pub journal_draft_note: String,
    pub show_scanner_dock: bool,
    pub scanner_tab: ScannerDockTab,
    pub scanner_filter: ScannerFilter,
    pub watchlist_sort_col: WatchlistSortColumn,
    pub watchlist_sort_asc: bool,
    pub signal_lookback_bars: usize,
    pub ticket_kind: TicketOrderKind,
    pub ticket_qty_input: String,
    pub ticket_price_input: String,
    pub ticket_sl_input: String,
    pub ticket_tp_input: String,
    pub alert_draft_kind: AlertDraftKind,
    pub alert_direction: CrossDirection,
    pub alert_frequency: AlertFrequency,
    pub alert_value_input: String,
    pub alert_message_input: String,

    #[serde(skip)]
    pub base_candles: Vec<Candle>,
    #[serde(skip)]
    pub candles: Vec<Candle>,
    #[serde(skip)]
    pub available_symbols: Vec<String>,
    #[serde(skip)]
    in_memory_custom_doc: Option<(DataFileFormat, String)>,
    #[serde(skip)]
    in_memory_custom_doc_hash: u64,
    #[serde(skip)]
    pub show_import_bar: bool,
    #[serde(skip)]
    pub last_load_status: Option<(bool, String)>,
    #[serde(skip)]
    pub last_trade_status: Option<(bool, String)>,
    #[serde(skip)]
    pub last_alert_status: Option<(bool, String)>,
    #[serde(skip)]
    pub last_journal_status: Option<(bool, String)>,
    #[serde(skip)]
    pub last_exported_journal_doc: Option<String>,
    #[serde(skip)]
    synced_rr_drawing_signature: Option<(u64, u64, u64, u64)>,
    #[serde(skip)]
    pub command_palette: CommandPaletteState,
    #[serde(skip)]
    watchlist_universe: Vec<WatchlistFeedEntry>,
    #[serde(skip)]
    custom_peer_feeds: Vec<WatchlistFeedEntry>,
    #[serde(skip)]
    raw_watchlist_rows_cache: Vec<SymbolScanRow>,
    #[serde(skip)]
    raw_historical_hits_cache: Vec<HistoricalSignalHit>,
    #[serde(skip)]
    scanner_synced_key: Option<(u64, String, Option<i64>, usize, bool)>,
    #[serde(skip)]
    alerts_synced_key: Option<(u64, u64, Option<i64>, u64)>,
    #[serde(skip)]
    pub backtest_result: Option<BacktestRunResult>,
    #[serde(skip)]
    pub sweep_report: Option<ParameterSweepReport>,
    #[serde(skip)]
    pub battle_report: Option<MultiSymbolBattleReport>,
    #[serde(skip)]
    pub walk_forward_report: Option<WalkForwardReport>,
    #[serde(skip)]
    pub portfolio_report: Option<PortfolioBacktestReport>,
    #[serde(skip)]
    pub preset_battle_rows: Vec<PresetBattleRow>,
    #[serde(skip)]
    pub focused_backtest_trade_id: Option<u64>,
    #[serde(skip)]
    backtest_synced_key: Option<BacktestSyncKey>,
    #[serde(skip)]
    sweep_synced_key: Option<SweepSyncKey>,
    #[serde(skip)]
    battle_synced_key: Option<BattleSyncKey>,
    #[serde(skip)]
    wf_synced_key: Option<WalkForwardSyncKey>,
    #[serde(skip)]
    portfolio_synced_key: Option<PortfolioSyncKey>,
    #[serde(skip)]
    portfolio_mapped_curve: Option<MappedPortfolioCurveCache>,
    #[serde(skip)]
    viewport_focus_lock: Option<(f64, f64, f64)>,
    #[serde(skip)]
    pub indicator_cache: IndicatorCache,
    #[serde(skip)]
    dataset_generation: u64,
    #[serde(skip)]
    resolved_benchmark_cache: Option<(String, i64, i64, u64, Timeframe, Vec<Candle>)>,
    #[serde(skip)]
    benchmark_gen_counter: u64,
    #[serde(skip)]
    compare_cache_key: Option<(u64, Timeframe, String, Option<i64>)>,
    #[serde(skip)]
    compare_series_cached: Vec<Option<f64>>,
    #[serde(skip)]
    compare_series_idx: SeriesBlockIndex,
    #[serde(skip)]
    active_benchmark_label: String,
    #[serde(skip)]
    pub last_report: Option<ChartRenderReport>,
    #[serde(skip)]
    pub frame_count: u64,
    #[serde(skip)]
    pub screenshot_requested: bool,
}

impl Default for SumiWorkstationApp {
    fn default() -> Self {
        let dataset_bar_count = 120;
        let base_candles = generate_synthetic_ohlcv(dataset_bar_count);
        let candles = base_candles.clone();
        let mut viewport = ChartViewportState::new();
        viewport.time_scale.set_total_bars(candles.len());
        let indicators = IndicatorConfig::default();
        let mut indicator_cache = IndicatorCache::new();
        indicator_cache.sync(&candles, 1, &indicators);

        Self {
            symbol: "VNINDEX".to_owned(),
            timeframe: "1D".to_owned(),
            data_source: DataSourcePreset::SyntheticVnindex,
            base_timeframe: Timeframe::D1,
            active_timeframe: Timeframe::D1,
            import_path_input: "ARCHIVE/test-results/p0b/data/CafeF.INDEX.clean.csv".to_owned(),
            show_grid: true,
            show_crosshair: true,
            show_volume: true,
            dataset_bar_count,
            viewport,
            indicators,
            compare_benchmark: None,
            show_compare_overlay: false,
            drawings: DrawingStore::new(),
            drawings_by_instrument: BTreeMap::new(),
            replay: ReplayController::new(),
            trading: TradingSession::default(),
            replay_trading_by_instrument: BTreeMap::new(),
            alerts: AlertEngine::new(),
            alerts_by_instrument: BTreeMap::new(),
            scanner_context: None,
            scanner_context_by_instrument: BTreeMap::new(),
            show_replay_bar: false,
            show_trading_panel: false,
            show_alerts_panel: false,
            show_strategy_panel: false,
            show_journal_panel: false,
            show_equity_pane: false,
            equity_pane_source: EquityPaneSource::SingleAssetBacktest,
            show_backtest_trades: false,
            right_dock_tab: RightDockTab::Trading,
            strategy_lab_tab: StrategyLabSubTab::Tearsheet,
            backtest_config: BacktestConfig::default(),
            sweep_ranking: SweepRankingMetric::SharpeRatio,
            wf_config: WalkForwardConfig::default(),
            portfolio_universe_filter: PortfolioUniverseFilter::Vn30EquitiesOnly,
            portfolio_weighting: PortfolioWeightingMode::EqualWeight,
            risk_venue_override: None,
            risk_plan_input: TradePlanInput::default(),
            journal: TradeJournalStore::new(),
            journal_draft_setup: SetupTag::TrendPullback,
            journal_draft_conviction: 4,
            journal_draft_checklist: ChecklistState::all_checked(),
            journal_draft_note: String::new(),
            show_scanner_dock: false,
            scanner_tab: ScannerDockTab::Watchlist,
            scanner_filter: ScannerFilter::default(),
            watchlist_sort_col: WatchlistSortColumn::Symbol,
            watchlist_sort_asc: true,
            signal_lookback_bars: 80,
            ticket_kind: TicketOrderKind::Market,
            ticket_qty_input: "10".to_owned(),
            ticket_price_input: String::new(),
            ticket_sl_input: String::new(),
            ticket_tp_input: String::new(),
            alert_draft_kind: AlertDraftKind::PriceLevel,
            alert_direction: CrossDirection::CrossAbove,
            alert_frequency: AlertFrequency::Once,
            alert_value_input: String::new(),
            alert_message_input: String::new(),
            base_candles,
            candles,
            available_symbols: vec!["VNINDEX".to_owned()],
            in_memory_custom_doc: None,
            in_memory_custom_doc_hash: 0,
            show_import_bar: false,
            last_load_status: None,
            last_trade_status: None,
            last_alert_status: None,
            last_journal_status: None,
            last_exported_journal_doc: None,
            synced_rr_drawing_signature: None,
            command_palette: CommandPaletteState::default(),
            watchlist_universe: Vec::new(),
            custom_peer_feeds: Vec::new(),
            raw_watchlist_rows_cache: Vec::new(),
            raw_historical_hits_cache: Vec::new(),
            scanner_synced_key: None,
            alerts_synced_key: None,
            backtest_result: None,
            sweep_report: None,
            battle_report: None,
            walk_forward_report: None,
            portfolio_report: None,
            preset_battle_rows: Vec::new(),
            focused_backtest_trade_id: None,
            backtest_synced_key: None,
            sweep_synced_key: None,
            battle_synced_key: None,
            wf_synced_key: None,
            portfolio_synced_key: None,
            portfolio_mapped_curve: None,
            viewport_focus_lock: None,
            indicator_cache,
            dataset_generation: 1,
            resolved_benchmark_cache: None,
            benchmark_gen_counter: 0,
            compare_cache_key: None,
            compare_series_cached: Vec::new(),
            compare_series_idx: SeriesBlockIndex::default(),
            active_benchmark_label: "VNINDEX".to_owned(),
            last_report: None,
            frame_count: 0,
            screenshot_requested: false,
        }
    }
}

impl SumiWorkstationApp {
    /// Initializes the workstation application and applies the dark financial theme.
    #[must_use]
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self::configure_dark_workstation_visuals(&cc.egui_ctx);

        let is_verification_capture = std::env::var("SUMI_SCREENSHOT_PATH")
            .ok()
            .is_some_and(|p| !p.is_empty())
            || std::env::var("SUMI_CAPTURE_BATCH")
                .ok()
                .is_some_and(|b| !b.is_empty());

        let mut app: Self = if !is_verification_capture && let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Self::default()
        };

        if let Ok(bars_str) = std::env::var("SUMI_BAR_COUNT")
            && let Ok(bars) = bars_str.parse::<usize>()
            && bars >= 10
        {
            app.dataset_bar_count = bars;
            app.data_source = DataSourcePreset::SyntheticVnindex;
        }

        #[cfg(not(target_arch = "wasm32"))]
        if let Ok(env_import) = std::env::var("SUMI_IMPORT_PATH")
            && !env_import.trim().is_empty()
        {
            app.viewport.sanitize_deserialized();
            app.indicators.sanitize_deserialized();
            app.drawings = DrawingStore::new();
            app.drawings_by_instrument.clear();
            let _ = app.import_file_from_path(&env_import);
            return app;
        }

        app.restore_persisted_session();
        app
    }

    /// Sanitizes deserialized `eframe` state and restores the exact persisted data source and
    /// `active_timeframe` *before* binding `TimeScale` or `DrawingStore`, preventing `right_offset`
    /// shifts or `base_tf -> active_tf` double-remapping (`CRITICAL-01`).
    pub fn restore_persisted_session(&mut self) {
        self.viewport.sanitize_deserialized();
        self.indicators.sanitize_deserialized();
        self.drawings.sanitize_deserialized();
        while self.drawings_by_instrument.len() > 64 {
            self.drawings_by_instrument.pop_first();
        }
        for store in self.drawings_by_instrument.values_mut() {
            store.sanitize_deserialized();
        }
        while self.replay_trading_by_instrument.len() > 64 {
            self.replay_trading_by_instrument.pop_first();
        }
        self.alerts.sanitize_deserialized();
        while self.alerts_by_instrument.len() > 64 {
            self.alerts_by_instrument.pop_first();
        }
        for engine in self.alerts_by_instrument.values_mut() {
            engine.sanitize_deserialized();
        }
        if let Some(ref mut ctx) = self.scanner_context {
            ctx.sanitize_deserialized();
        }
        while self.scanner_context_by_instrument.len() > 64 {
            self.scanner_context_by_instrument.pop_first();
        }
        for ctx in self.scanner_context_by_instrument.values_mut() {
            ctx.sanitize_deserialized();
        }
        truncate_utf8_bytes(&mut self.scanner_filter.search_query, 64);
        self.signal_lookback_bars = self.signal_lookback_bars.clamp(5, 500);
        if self.ticket_qty_input.trim().is_empty() || self.ticket_qty_input.len() > 32 {
            self.ticket_qty_input = "10".to_owned();
        }
        truncate_utf8_bytes(&mut self.ticket_price_input, 32);
        truncate_utf8_bytes(&mut self.ticket_sl_input, 32);
        truncate_utf8_bytes(&mut self.ticket_tp_input, 32);
        truncate_utf8_bytes(&mut self.alert_value_input, 32);
        truncate_utf8_bytes(&mut self.alert_message_input, 96);
        self.backtest_config.sanitize();
        self.wf_config.sanitize();
        self.journal.sanitize_deserialized();
        self.journal_draft_conviction = self.journal_draft_conviction.clamp(1, 5);
        truncate_utf8_bytes(
            &mut self.journal_draft_note,
            crate::journal::MAX_JOURNAL_NOTE_BYTES,
        );
        sanitize_persisted_trade_plan_input(&mut self.risk_plan_input);
        self.viewport_focus_lock = None;
        if let Some(ref mut bench) = self.compare_benchmark {
            truncate_utf8_bytes(bench, 24);
            let trimmed = bench.trim().to_ascii_uppercase();
            if trimmed.is_empty() {
                self.compare_benchmark = None;
            } else {
                *bench = trimmed;
            }
        }
        self.resolved_benchmark_cache = None;
        self.compare_cache_key = None;
        self.compare_series_cached.clear();
        self.compare_series_idx.clear();
        self.in_memory_custom_doc = None;
        self.in_memory_custom_doc_hash = 0;
        self.custom_peer_feeds.clear();
        self.last_trade_status = None;
        self.last_alert_status = None;
        self.last_journal_status = None;
        self.last_exported_journal_doc = None;
        self.synced_rr_drawing_signature = None;
        self.command_palette = CommandPaletteState::default();
        self.scanner_synced_key = None;
        self.alerts_synced_key = None;
        self.backtest_result = None;
        self.sweep_report = None;
        self.battle_report = None;
        self.walk_forward_report = None;
        self.portfolio_report = None;
        self.preset_battle_rows.clear();
        self.focused_backtest_trade_id = None;
        self.backtest_synced_key = None;
        self.sweep_synced_key = None;
        self.battle_synced_key = None;
        self.wf_synced_key = None;
        self.portfolio_synced_key = None;
        self.portfolio_mapped_curve = None;

        let persisted_tf = Timeframe::parse_label(&self.timeframe).unwrap_or(self.active_timeframe);
        let (report, reset_to_clean_fallback) = if self.data_source == DataSourcePreset::CustomFile
        {
            #[cfg(not(target_arch = "wasm32"))]
            {
                let custom_path = self.import_path_input.trim().to_owned();
                let saved_sym = self.symbol.trim().to_owned();
                let loaded = if custom_path.is_empty() {
                    None
                } else {
                    let opts_with_sym = DataLoadOptions {
                        symbol_filter: (!saved_sym.is_empty()).then(|| saved_sym.clone()),
                        ..DataLoadOptions::default()
                    };
                    crate::data::load_ohlcv_file(&custom_path, &opts_with_sym)
                        .or_else(|_| {
                            crate::data::load_ohlcv_file(&custom_path, &DataLoadOptions::default())
                        })
                        .ok()
                };
                if let Some(rep) = loaded {
                    let sym_changed =
                        !saved_sym.is_empty() && !rep.symbol.eq_ignore_ascii_case(&saved_sym);
                    if sym_changed {
                        self.drawings = DrawingStore::new();
                    }
                    (rep, sym_changed)
                } else {
                    self.data_source = DataSourcePreset::SyntheticVnindex;
                    self.symbol = "VNINDEX".to_owned();
                    self.drawings = DrawingStore::new();
                    (
                        load_preset_dataset(
                            DataSourcePreset::SyntheticVnindex,
                            self.dataset_bar_count.max(10),
                            None,
                        ),
                        true,
                    )
                }
            }
            #[cfg(target_arch = "wasm32")]
            {
                self.data_source = DataSourcePreset::SyntheticVnindex;
                self.symbol = "VNINDEX".to_owned();
                self.drawings = DrawingStore::new();
                (
                    load_preset_dataset(
                        DataSourcePreset::SyntheticVnindex,
                        self.dataset_bar_count.max(10),
                        None,
                    ),
                    true,
                )
            }
        } else {
            let symbol_hint = (!self.symbol.trim().is_empty()).then(|| self.symbol.clone());
            (
                load_preset_dataset(
                    self.data_source,
                    self.dataset_bar_count.max(10),
                    symbol_hint.as_deref(),
                ),
                false,
            )
        };

        if reset_to_clean_fallback {
            self.replay.exit_replay();
            self.trading = TradingSession::default();
            self.alerts = AlertEngine::new();
            self.scanner_context = None;
            self.show_replay_bar = false;
        } else {
            self.replay.sanitize_deserialized(&report.candles);
            if self.replay.is_active() {
                self.show_replay_bar = true;
            } else {
                self.scanner_context = None;
            }
        }

        let (resolved_tf, active_candles) =
            if !reset_to_clean_fallback && report.base_timeframe.can_aggregate_to(persisted_tf) {
                match self.replay.materialize_candles(
                    &report.candles,
                    report.base_timeframe,
                    persisted_tf,
                ) {
                    Ok(agg) => (persisted_tf, agg),
                    Err(_) => {
                        let fallback = self
                            .replay
                            .materialize_candles(
                                &report.candles,
                                report.base_timeframe,
                                report.base_timeframe,
                            )
                            .unwrap_or_else(|_| report.candles.clone());
                        (report.base_timeframe, fallback)
                    }
                }
            } else {
                let fallback = self
                    .replay
                    .materialize_candles(
                        &report.candles,
                        report.base_timeframe,
                        report.base_timeframe,
                    )
                    .unwrap_or_else(|_| report.candles.clone());
                (report.base_timeframe, fallback)
            };

        self.symbol = report.symbol;
        self.available_symbols = report.available_symbols;
        self.base_timeframe = report.base_timeframe;
        self.active_timeframe = resolved_tf;
        self.timeframe = resolved_tf.label().to_owned();
        if self.data_source == DataSourcePreset::SyntheticVnindex {
            self.dataset_bar_count = report.candles.len();
        }
        self.base_candles = report.candles;
        self.candles = active_candles;
        self.rebuild_custom_peer_feeds();

        let cursor_idx = self.current_trading_cursor_base_index();
        self.trading
            .sanitize_and_rebuild(&self.base_candles, cursor_idx);

        // Stamp canonical UTC timestamps onto persisted drawings in-place at `resolved_tf`
        // (`bar_index` remains untouched; `timestamp` is populated for subsequent TF switches).
        self.drawings
            .remap_timeframe(&self.candles, resolved_tf, &self.candles, resolved_tf);
        self.drawings.set_replay_cutoff_timestamp(
            self.replay.cutoff_timestamp(&self.base_candles),
            self.candles.len(),
        );

        self.viewport.time_scale.set_total_bars(self.candles.len());
        if reset_to_clean_fallback {
            self.viewport.reset_to_default();
        }
        self.dataset_generation = self.dataset_generation.wrapping_add(1).max(1);
        self.sync_indicators_and_benchmarks();
        self.reseed_risk_planner_for_active_instrument(reset_to_clean_fallback);
        self.sync_alerts_causal();
    }

    pub fn configure_dark_workstation_visuals(ctx: &egui::Context) {
        let mut visuals = Visuals::dark();
        visuals.panel_fill = palette::BG_WORKSTATION;
        visuals.window_fill = palette::BG_WORKSTATION;
        visuals.extreme_bg_color = palette::BG_CHART_CANVAS;
        visuals.override_text_color = Some(palette::TEXT_PRIMARY);
        visuals.widgets.noninteractive.bg_fill = palette::BG_WORKSTATION;
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, palette::BORDER_SUBTLE);
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, palette::TEXT_PRIMARY);
        visuals.widgets.inactive.bg_fill = Color32::from_rgb(26, 31, 44);
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, palette::BORDER_SUBTLE);
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, palette::TEXT_PRIMARY);
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(38, 45, 64);
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, Color32::WHITE);
        visuals.widgets.active.bg_fill = palette::ACCENT_BLUE;
        visuals.widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);
        ctx.set_visuals(visuals);
    }

    /// Switches the active synthetic benchmark series (`120`, `10_000`, `50_000`, `100_000`) and resets view.
    pub fn set_dataset_size(&mut self, count: usize) {
        let clamped = count.max(10);
        self.dataset_bar_count = clamped;
        let report = load_preset_dataset(DataSourcePreset::SyntheticVnindex, clamped, None);
        self.apply_loaded_report_internal(
            report,
            DataSourcePreset::SyntheticVnindex,
            CustomSourceUpdate::Clear,
            true,
        );
    }

    /// Switches to one of the bundled workstation OHLCV feeds (`VNINDEX Synth`, `VN30F1M 1m`,
    /// `CafeF CSV`, `SPY CSV`, `BTC JSON`).
    pub fn select_data_source(&mut self, preset: DataSourcePreset) {
        let report = load_preset_dataset(preset, self.dataset_bar_count.max(10), None);
        let summary = format!(
            "Loaded {} · {} ({} base {} bars)",
            report.symbol,
            preset.short_label(),
            report.candles.len(),
            report.base_timeframe
        );
        self.apply_loaded_report_internal(report, preset, CustomSourceUpdate::Clear, true);
        self.last_load_status = Some((false, summary));
    }

    /// Switches between tickers discovered in a multi-symbol CSV/JSON source (e.g. `HNX-INDEX` <-> `VNINDEX` in CafeF CSV,
    /// as well as in-memory `load_from_csv_str` / `load_from_json_str` documents on both native and `wasm32`).
    pub fn select_symbol_in_source(&mut self, ticker: &str) {
        let opts = DataLoadOptions {
            symbol_filter: Some(ticker.to_owned()),
            ..DataLoadOptions::default()
        };

        if self.data_source == DataSourcePreset::CustomFile {
            if let Some((fmt, doc)) = self.in_memory_custom_doc.clone() {
                match load_ohlcv_bytes(doc.as_bytes(), fmt, &opts) {
                    Ok(rep) => {
                        let summary = format!(
                            "Selected {} ({} {} bars)",
                            rep.symbol,
                            rep.candles.len(),
                            rep.base_timeframe
                        );
                        self.apply_loaded_report_internal(
                            rep,
                            DataSourcePreset::CustomFile,
                            CustomSourceUpdate::KeepExisting,
                            true,
                        );
                        self.last_load_status = Some((false, summary));
                    }
                    Err(err) => {
                        self.last_load_status = Some((true, err.to_string()));
                    }
                }
                return;
            }

            #[cfg(not(target_arch = "wasm32"))]
            if !self.import_path_input.trim().is_empty() {
                let path_copy = self.import_path_input.trim().to_owned();
                match crate::data::load_ohlcv_file(&path_copy, &opts) {
                    Ok(rep) => {
                        let summary = format!(
                            "Selected {} ({} {} bars)",
                            rep.symbol,
                            rep.candles.len(),
                            rep.base_timeframe
                        );
                        self.apply_loaded_report_internal(
                            rep,
                            DataSourcePreset::CustomFile,
                            CustomSourceUpdate::KeepExisting,
                            true,
                        );
                        self.last_load_status = Some((false, summary));
                    }
                    Err(err) => {
                        self.last_load_status = Some((true, err.to_string()));
                    }
                }
                return;
            }

            self.last_load_status = Some((true, "no custom data source loaded".to_owned()));
            return;
        }

        let rep = load_preset_dataset(
            self.data_source,
            self.dataset_bar_count.max(10),
            Some(ticker),
        );
        let summary = format!(
            "Selected {} ({} {} bars)",
            rep.symbol,
            rep.candles.len(),
            rep.base_timeframe
        );
        self.apply_loaded_report_internal(rep, self.data_source, CustomSourceUpdate::Clear, true);
        self.last_load_status = Some((false, summary));
    }

    /// Loads an in-memory CSV document into the workstation (`wasm32`-compatible).
    pub fn load_from_csv_str(
        &mut self,
        csv_text: &str,
        options: &DataLoadOptions,
    ) -> Result<(), DataLoadError> {
        match load_ohlcv_csv(csv_text, options) {
            Ok(rep) => {
                let msg = format_load_summary("CSV", &rep);
                self.apply_loaded_report_internal(
                    rep,
                    DataSourcePreset::CustomFile,
                    CustomSourceUpdate::InMemory(DataFileFormat::Csv, csv_text),
                    true,
                );
                self.last_load_status = Some((false, msg));
                Ok(())
            }
            Err(err) => {
                self.last_load_status = Some((true, err.to_string()));
                Err(err)
            }
        }
    }

    /// Loads an in-memory JSON document into the workstation (`wasm32`-compatible).
    pub fn load_from_json_str(
        &mut self,
        json_text: &str,
        options: &DataLoadOptions,
    ) -> Result<(), DataLoadError> {
        match load_ohlcv_json(json_text, options) {
            Ok(rep) => {
                let msg = format_load_summary("JSON", &rep);
                self.apply_loaded_report_internal(
                    rep,
                    DataSourcePreset::CustomFile,
                    CustomSourceUpdate::InMemory(DataFileFormat::Json, json_text),
                    true,
                );
                self.last_load_status = Some((false, msg));
                Ok(())
            }
            Err(err) => {
                self.last_load_status = Some((true, err.to_string()));
                Err(err)
            }
        }
    }

    /// Loads a local `.csv`, `.tsv`, `.txt`, or `.json` file path into the workstation (native desktop).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn import_file_from_path(&mut self, path: &str) -> Result<(), DataLoadError> {
        let trimmed = path.trim();
        if trimmed.is_empty() {
            let err = DataLoadError::InvalidConfiguration("file path is empty".to_owned());
            self.last_load_status = Some((true, err.to_string()));
            return Err(err);
        }
        match crate::data::load_ohlcv_file(trimmed, &DataLoadOptions::default()) {
            Ok(rep) => {
                let msg = format_load_summary(trimmed, &rep);
                self.apply_loaded_report_internal(
                    rep,
                    DataSourcePreset::CustomFile,
                    CustomSourceUpdate::FilePath(trimmed),
                    true,
                );
                self.last_load_status = Some((false, msg));
                Ok(())
            }
            Err(err) => {
                self.last_load_status = Some((true, err.to_string()));
                Err(err)
            }
        }
    }

    /// Aggregates `self.base_candles` upward to `target_tf` (`target_tf >= self.base_timeframe`),
    /// remapping all multi-pane drawings + undo/redo stacks via exact UTC timestamps and preserving
    /// historical viewport focus!
    pub fn set_timeframe(&mut self, target_tf: Timeframe) -> Result<(), AggregationError> {
        if !self.base_timeframe.can_aggregate_to(target_tf) {
            let err = AggregationError::CannotDownsample {
                source: self.base_timeframe,
                target: target_tf,
            };
            self.last_load_status = Some((true, err.to_string()));
            return Err(err);
        }
        if target_tf == self.active_timeframe && !self.candles.is_empty() {
            return Ok(());
        }

        let aggregated =
            self.replay
                .materialize_candles(&self.base_candles, self.base_timeframe, target_tf)?;
        let old_tf = self.active_timeframe;
        let old_candles = std::mem::replace(&mut self.candles, aggregated);

        // 1. Remap drawings + undo/redo history across `(old_candles, old_tf) -> (self.candles, target_tf)`
        //    and synchronize causal replay visibility cutoff (`DEF-001`).
        self.drawings
            .remap_timeframe(&old_candles, old_tf, &self.candles, target_tf);
        self.drawings.set_replay_cutoff_timestamp(
            self.replay.cutoff_timestamp(&self.base_candles),
            self.candles.len(),
        );

        // 2. Preserve viewport temporal focus across timeframe switches!
        //    Uses `TimeScale::local_x_to_center_float_index` (`(width - 1.0 - local_x) / bar_spacing`)
        //    for exact round-trip symmetry (`< 1e-12` bar drift), plus `remap_timestamp_to_bar_index`
        //    so coarse `1W` / `1D` bucket-start timestamps snap cleanly to the session's first bar.
        let old_right_offset = self.viewport.time_scale.right_offset();
        let raw_w = self.viewport.time_scale.width();
        let plot_w = if raw_w.is_finite() && raw_w > 0.0 {
            raw_w
        } else {
            1_000.0
        };
        let center_local_x = plot_w * 0.5;
        let bs = self.viewport.time_scale.bar_spacing().max(0.005);
        let delta_from_right = (plot_w - 1.0 - center_local_x) / bs;
        let locked_ts = self
            .viewport_focus_lock
            .and_then(|(ts, saved_ro, saved_bs)| {
                ((old_right_offset - saved_ro).abs() <= 1e-6 && (bs - saved_bs).abs() <= 1e-6)
                    .then_some(ts)
            });
        let historical_center_ts = locked_ts.or_else(|| {
            if old_candles.is_empty() {
                return None;
            }
            let old_center_idx = if raw_w.is_finite() && raw_w > 0.0 {
                self.viewport
                    .time_scale
                    .local_x_to_center_float_index(center_local_x)?
            } else {
                ((old_candles.len() - 1) as f64) + old_right_offset + 0.5 - delta_from_right
            };
            let last_old = (old_candles.len() - 1) as f64;
            let is_sparse_historical_pan = old_center_idx >= -0.5
                && old_center_idx < last_old - 0.25
                && (old_right_offset - 12.0).abs() > 1.0;
            if old_right_offset < -2.0 || is_sparse_historical_pan {
                resolve_anchor_timestamp(old_center_idx, &old_candles, old_tf)
            } else {
                None
            }
        });

        let focus_source_tf = if locked_ts.is_some() {
            self.base_timeframe
        } else {
            old_tf
        };

        self.viewport.kinetic = None;
        self.viewport.time_scale.set_total_bars(self.candles.len());
        if let Some(center_ts) = historical_center_ts
            && let Some((new_center_idx, snapped_ts)) =
                remap_timestamp_to_bar_index(center_ts, focus_source_tf, &self.candles, target_tf)
            && !self.candles.is_empty()
        {
            let new_right_offset =
                new_center_idx + delta_from_right - 0.5 - ((self.candles.len() - 1) as f64);
            self.viewport.time_scale.set_right_offset(new_right_offset);
            let actual_ro = self.viewport.time_scale.right_offset();
            let actual_bs = self.viewport.time_scale.bar_spacing().max(0.005);
            let lock_ts = if locked_ts.is_some() {
                center_ts
            } else {
                snapped_ts
            };
            self.viewport_focus_lock = Some((lock_ts, actual_ro, actual_bs));
        } else {
            self.viewport_focus_lock = None;
            self.viewport
                .time_scale
                .set_right_offset(old_right_offset.min(24.0));
        }

        if self.viewport.main_price_scale.is_auto_scale() {
            self.viewport.main_price_scale.reset_auto_scale();
        }
        self.viewport.rsi_price_scale.reset_auto_scale();
        self.viewport.macd_price_scale.reset_auto_scale();
        self.viewport.atr_price_scale.reset_auto_scale();
        self.viewport.rs_price_scale.reset_auto_scale();
        self.viewport.equity_price_scale.reset_auto_scale();
        self.focused_backtest_trade_id = None;

        self.active_timeframe = target_tf;
        self.timeframe = target_tf.label().to_owned();
        self.dataset_generation = self.dataset_generation.wrapping_add(1).max(1);
        self.sync_indicators_and_benchmarks();
        self.sync_alerts_causal();

        Ok(())
    }

    fn current_instrument_key(&self) -> String {
        let sym_upper = self.symbol.trim().to_ascii_uppercase();
        match self.data_source {
            DataSourcePreset::CustomFile => {
                if let Some((fmt, _)) = self.in_memory_custom_doc {
                    format!(
                        "CustomMem:{fmt:?}:{:016x}:{sym_upper}",
                        self.in_memory_custom_doc_hash
                    )
                } else {
                    format!("CustomFile:{}:{sym_upper}", self.import_path_input.trim())
                }
            }
            DataSourcePreset::SyntheticVnindex => {
                format!(
                    "Preset:{}:{}:{sym_upper}",
                    self.data_source.short_label(),
                    self.base_candles.len()
                )
            }
            _ => format!("Preset:{}:{sym_upper}", self.data_source.short_label()),
        }
    }

    fn apply_loaded_report_internal(
        &mut self,
        report: DataLoadReport,
        source: DataSourcePreset,
        custom_update: CustomSourceUpdate<'_>,
        reset_viewport: bool,
    ) {
        self.drawings.cancel_active_gesture();
        let prev_magnet = self.drawings.magnet_mode;
        let outgoing_key = self.current_instrument_key();

        // Park outgoing instrument's drawings normalized to its `base_timeframe` so switching
        // tickers/sources never cross-contaminates drawings across instruments (`HIGH-03`).
        let mut outgoing_store = std::mem::take(&mut self.drawings);
        outgoing_store.set_replay_cutoff_timestamp(None, self.base_candles.len());
        if !outgoing_store.drawings.is_empty() {
            if !self.candles.is_empty() && !self.base_candles.is_empty() {
                outgoing_store.remap_timeframe(
                    &self.candles,
                    self.active_timeframe,
                    &self.base_candles,
                    self.base_timeframe,
                );
            }
            while self.drawings_by_instrument.len() >= 64
                && !self.drawings_by_instrument.contains_key(&outgoing_key)
            {
                self.drawings_by_instrument.pop_first();
            }
            self.drawings_by_instrument
                .insert(outgoing_key.clone(), outgoing_store);
        } else {
            self.drawings_by_instrument.remove(&outgoing_key);
        }

        // Park outgoing instrument's causal Bar Replay & Paper Trading session so open positions/orders
        // never leak across tickers (`VN30F1M` <-> `BTCUSDT` <-> `VNINDEX`).
        let mut outgoing_replay = std::mem::take(&mut self.replay);
        outgoing_replay.pause();
        outgoing_replay.cancel_cut_selection();
        let outgoing_replay_active = outgoing_replay.is_active();
        let outgoing_trading = std::mem::take(&mut self.trading);
        if outgoing_replay_active || !outgoing_trading.commands.is_empty() {
            while self.replay_trading_by_instrument.len() >= 64
                && !self
                    .replay_trading_by_instrument
                    .contains_key(&outgoing_key)
            {
                self.replay_trading_by_instrument.pop_first();
            }
            self.replay_trading_by_instrument
                .insert(outgoing_key.clone(), (outgoing_replay, outgoing_trading));
        } else {
            self.replay_trading_by_instrument.remove(&outgoing_key);
        }

        // Park outgoing instrument's AlertEngine & ScannerSignalContext cleanly per instrument (`Milestone R08`).
        let outgoing_alerts = std::mem::take(&mut self.alerts);
        if !outgoing_alerts.rules.is_empty() {
            while self.alerts_by_instrument.len() >= 64
                && !self.alerts_by_instrument.contains_key(&outgoing_key)
            {
                self.alerts_by_instrument.pop_first();
            }
            self.alerts_by_instrument
                .insert(outgoing_key.clone(), outgoing_alerts);
        } else {
            self.alerts_by_instrument.remove(&outgoing_key);
        }

        if let Some(ctx) = self.scanner_context.take()
            && outgoing_replay_active
        {
            while self.scanner_context_by_instrument.len() >= 64
                && !self
                    .scanner_context_by_instrument
                    .contains_key(&outgoing_key)
            {
                self.scanner_context_by_instrument.pop_first();
            }
            self.scanner_context_by_instrument
                .insert(outgoing_key.clone(), ctx);
        } else {
            self.scanner_context_by_instrument.remove(&outgoing_key);
        }

        let mut refresh_custom_peers = false;
        match custom_update {
            CustomSourceUpdate::Clear => {
                self.in_memory_custom_doc = None;
                self.in_memory_custom_doc_hash = 0;
                self.custom_peer_feeds.clear();
            }
            CustomSourceUpdate::KeepExisting => {}
            #[cfg(not(target_arch = "wasm32"))]
            CustomSourceUpdate::FilePath(p) => {
                self.in_memory_custom_doc = None;
                self.in_memory_custom_doc_hash = 0;
                self.import_path_input = p.to_owned();
                refresh_custom_peers = true;
            }
            CustomSourceUpdate::InMemory(fmt, text) => {
                self.in_memory_custom_doc_hash = fnv1a_64(text.as_bytes());
                self.in_memory_custom_doc = Some((fmt, text.to_owned()));
                refresh_custom_peers = true;
            }
        }

        self.viewport_focus_lock = None;
        self.data_source = source;
        self.symbol = report.symbol;
        self.available_symbols = report.available_symbols;
        self.base_timeframe = report.base_timeframe;
        self.active_timeframe = report.base_timeframe;
        self.timeframe = report.base_timeframe.label().to_owned();
        if source == DataSourcePreset::SyntheticVnindex {
            self.dataset_bar_count = report.candles.len();
        }
        self.base_candles = report.candles;
        if refresh_custom_peers {
            self.rebuild_custom_peer_feeds();
        }

        let incoming_key = self.current_instrument_key();
        let (mut incoming_replay, mut incoming_trading) = self
            .replay_trading_by_instrument
            .remove(&incoming_key)
            .unwrap_or_default();
        incoming_replay.sanitize_deserialized(&self.base_candles);
        self.candles = incoming_replay
            .materialize_candles(
                &self.base_candles,
                self.base_timeframe,
                self.active_timeframe,
            )
            .unwrap_or_else(|_| self.base_candles.clone());
        self.replay = incoming_replay;

        let cursor_idx = self.current_trading_cursor_base_index();
        incoming_trading.sanitize_and_rebuild(&self.base_candles, cursor_idx);
        let has_existing_trades = !incoming_trading.commands.is_empty();
        self.trading = incoming_trading;
        self.show_replay_bar = self.replay.is_active();
        self.last_trade_status = None;
        if !has_existing_trades {
            self.ticket_qty_input = default_ticket_qty_string(
                self.current_mark_price(),
                self.trading.config.initial_cash,
            );
            self.ticket_price_input.clear();
            self.ticket_sl_input.clear();
            self.ticket_tp_input.clear();
        }

        let mut incoming_store = self
            .drawings_by_instrument
            .remove(&incoming_key)
            .unwrap_or_default();
        incoming_store.magnet_mode = prev_magnet;
        if !incoming_store.drawings.is_empty() && !self.candles.is_empty() {
            incoming_store.remap_timeframe(
                &self.base_candles,
                self.base_timeframe,
                &self.candles,
                self.active_timeframe,
            );
        }
        incoming_store.set_replay_cutoff_timestamp(
            self.replay.cutoff_timestamp(&self.base_candles),
            self.candles.len(),
        );
        self.drawings = incoming_store;

        let mut incoming_alerts = self
            .alerts_by_instrument
            .remove(&incoming_key)
            .unwrap_or_default();
        incoming_alerts.sanitize_deserialized();
        self.alerts = incoming_alerts;
        self.last_alert_status = None;
        self.alerts_synced_key = None;
        if outgoing_key != incoming_key {
            self.alert_value_input.clear();
            self.alert_message_input.clear();
        }

        self.scanner_context = self
            .scanner_context_by_instrument
            .remove(&incoming_key)
            .filter(|_| self.replay.is_active());

        self.dataset_generation = self.dataset_generation.wrapping_add(1).max(1);
        self.resolved_benchmark_cache = None;
        self.compare_cache_key = None;
        self.focused_backtest_trade_id = None;
        self.synced_rr_drawing_signature = None;
        self.viewport.rs_price_scale.reset_auto_scale();
        self.viewport.equity_price_scale.reset_auto_scale();
        self.viewport.time_scale.set_total_bars(self.candles.len());
        if reset_viewport {
            self.viewport.reset_to_default();
        }
        self.sync_indicators_and_benchmarks();
        if outgoing_key != incoming_key {
            self.risk_venue_override = None;
            self.reseed_risk_planner_for_active_instrument(true);
        } else {
            self.reseed_risk_planner_for_active_instrument(false);
        }
        self.sync_alerts_causal();
    }

    /// Switches vertical price transformation mode (`Normal`, `Logarithmic`, `Percentage`, `IndexedTo100`).
    pub fn set_scale_mode(&mut self, mode: PriceScaleMode) {
        let base_value = self.visible_slice().first().map(|c| c.close);
        self.viewport
            .main_price_scale
            .set_mode_with_base(mode, base_value);
    }

    /// Returns the slice of candles currently inside `TimeScale::strict_visible_data_range()`.
    #[must_use]
    pub fn visible_slice(&self) -> &[Candle] {
        if self.candles.is_empty() {
            return &[];
        }
        let range = self.viewport.time_scale.strict_visible_data_range();
        if range.is_empty() {
            let start = self.candles.len().saturating_sub(80);
            &self.candles[start..]
        } else {
            &self.candles[range]
        }
    }

    pub fn zoom_in(&mut self) {
        self.viewport_focus_lock = None;
        if self.viewport.time_scale.width() <= 0.0 {
            self.viewport.time_scale.set_width(1000.0);
        }
        self.viewport.zoom_around_center(2.5);
    }

    pub fn zoom_out(&mut self) {
        self.viewport_focus_lock = None;
        if self.viewport.time_scale.width() <= 0.0 {
            self.viewport.time_scale.set_width(1000.0);
        }
        self.viewport.zoom_around_center(-2.5);
    }

    pub fn fit_content(&mut self) {
        self.viewport_focus_lock = None;
        if self.viewport.time_scale.width() <= 0.0 {
            self.viewport.time_scale.set_width(1000.0);
        }
        self.viewport.fit_all();
    }

    pub fn reset_view(&mut self) {
        self.viewport_focus_lock = None;
        self.viewport.reset_to_default();
        self.viewport.reset_pane_stretches();
    }

    /// Populates a deterministic, data-anchored set of multi-pane annotations (`HorizontalLine`,
    /// `TrendLine`, $360^\circ$ `Ray`, `Rectangle`, `FibonacciRetracement` + `RSI` / `MACD` / `ATR`
    /// sub-pane annotations) scaled cleanly to the currently visible candle range as a single
    /// undoable `DrawingCommand::ReplaceAll` step preserving `magnet_mode` and `locked` history!
    pub fn populate_showcase_drawings(&mut self) {
        if self.candles.len() < 2 {
            return;
        }
        let last_avail = self.candles.len() - 1;
        let vis_range = self.viewport.time_scale.strict_visible_data_range();
        let (left_i, right_i) =
            if vis_range.len() >= 12 || (vis_range.len() >= 2 && self.candles.len() < 12) {
                (vis_range.start, vis_range.end - 1)
            } else {
                let start = self.candles.len().saturating_sub(120);
                (start, last_avail)
            };
        let span = right_i.saturating_sub(left_i).max(1);
        let idx_at = |frac: f64| -> usize {
            (left_i + ((span as f64) * frac).round() as usize).min(last_avail)
        };
        let distinct_pair = |f0: f64, f1: f64| -> (usize, usize) {
            let i0 = idx_at(f0);
            let mut i1 = idx_at(f1);
            if i0 == i1 {
                if i1 < last_avail {
                    i1 += 1;
                } else {
                    return (i0.saturating_sub(1), i1);
                }
            }
            (i0, i1)
        };

        let mut slice_low = (usize::MAX, f64::INFINITY);
        let mut slice_high = (0_usize, f64::NEG_INFINITY);
        for (offset, c) in self.candles[left_i..=right_i].iter().enumerate() {
            let idx = left_i + offset;
            if c.low < slice_low.1 {
                slice_low = (idx, c.low);
            }
            if c.high > slice_high.1 {
                slice_high = (idx, c.high);
            }
        }
        let price_span = (slice_high.1 - slice_low.1).max(1.0);

        let mut showcase = Vec::with_capacity(8);
        let cutoff_ts = self.drawings.replay_cutoff_timestamp();
        let make_drawing = |pane: DrawingPaneTarget,
                            geom: DrawingGeometry,
                            style: Option<DrawingStyle>,
                            locked: bool| {
            let resolved_style =
                style.unwrap_or_else(|| DrawingStyle::default_for_tool(geom.tool_kind()));
            let mut d = crate::drawings::Drawing::new(0, pane, geom, resolved_style);
            d.locked = locked;
            d.created_at_timestamp = cutoff_ts;
            d
        };

        // 1. Main Pane: Locked Horizontal Support Line at visible swing low
        showcase.push(make_drawing(
            DrawingPaneTarget::Main,
            DrawingGeometry::HorizontalLine {
                anchor: DrawingAnchor::new(slice_low.0 as f64, slice_low.1),
            },
            Some(DrawingStyle {
                color: DrawingColorPreset::AmberGold,
                line_width: 1.5,
                dash: LineDashStyle::Dashed,
                fill_opacity: 0.0,
            }),
            true,
        ));

        // 2. Main Pane: Structural TrendLine connecting early and late swing points
        let (tl_i0, tl_i1) = distinct_pair(0.08, 0.54);
        showcase.push(make_drawing(
            DrawingPaneTarget::Main,
            DrawingGeometry::TrendLine {
                start: DrawingAnchor::new(tl_i0 as f64, self.candles[tl_i0].low),
                end: DrawingAnchor::new(tl_i1 as f64, self.candles[tl_i1].close),
            },
            None,
            false,
        ));

        // 3. Main Pane: Directional 360° Ray projecting into future `right_offset` whitespace
        let (ray_i0, ray_i1) = distinct_pair(0.60, 0.88);
        showcase.push(make_drawing(
            DrawingPaneTarget::Main,
            DrawingGeometry::Ray {
                origin: DrawingAnchor::new(ray_i0 as f64, self.candles[ray_i0].high),
                through: DrawingAnchor::new(
                    ray_i1 as f64,
                    self.candles[ray_i1].close + price_span * 0.04,
                ),
            },
            None,
            false,
        ));

        // 4. Main Pane: Translucent Supply/Demand Consolidation Rectangle
        let (rect_i0, rect_i1) = distinct_pair(0.18, 0.42);
        let rect_mid = (self.candles[rect_i0].close + self.candles[rect_i1].close) * 0.5;
        showcase.push(make_drawing(
            DrawingPaneTarget::Main,
            DrawingGeometry::Rectangle {
                corner_a: DrawingAnchor::new(rect_i0 as f64, rect_mid - price_span * 0.11),
                corner_b: DrawingAnchor::new(rect_i1 as f64, rect_mid + price_span * 0.11),
            },
            None,
            false,
        ));

        // 5. Main Pane: Selected Fibonacci Retracement across the primary impulse leg
        let (fib_i0, fib_i1) = distinct_pair(0.46, 0.92);
        let fib_entry_index = showcase.len();
        showcase.push(make_drawing(
            DrawingPaneTarget::Main,
            DrawingGeometry::FibonacciRetracement {
                start: DrawingAnchor::new(fib_i0 as f64, slice_low.1 + price_span * 0.08),
                end: DrawingAnchor::new(fib_i1 as f64, slice_high.1 - price_span * 0.04),
                direction: FibDirection::StartToEnd,
                extend_right: true,
            },
            None,
            false,
        ));

        // 6. RSI Sub-Pane: Momentum TrendLine (`32 -> 68`)
        if self.indicators.show_rsi {
            let (r0, r1) = distinct_pair(0.15, 0.82);
            let v0 = self
                .indicator_cache
                .rsi
                .get(r0)
                .copied()
                .flatten()
                .filter(|v| v.is_finite())
                .unwrap_or(36.0);
            let v1 = self
                .indicator_cache
                .rsi
                .get(r1)
                .copied()
                .flatten()
                .filter(|v| v.is_finite())
                .unwrap_or(66.0);
            showcase.push(make_drawing(
                DrawingPaneTarget::Oscillator(OscillatorPaneId::Rsi),
                DrawingGeometry::TrendLine {
                    start: DrawingAnchor::new(r0 as f64, v0),
                    end: DrawingAnchor::new(r1 as f64, v1),
                },
                Some(DrawingStyle {
                    color: DrawingColorPreset::Cyan,
                    line_width: 1.75,
                    dash: LineDashStyle::Solid,
                    fill_opacity: 0.0,
                }),
                false,
            ));
        }

        // 7. MACD Sub-Pane: Negative-to-positive accumulation Rectangle
        if self.indicators.show_macd {
            let (m0, m1) = distinct_pair(0.25, 0.75);
            let mv0 = self
                .indicator_cache
                .macd
                .macd_line
                .get(m0)
                .copied()
                .flatten()
                .filter(|v| v.is_finite())
                .unwrap_or(-4.0);
            let mv1 = self
                .indicator_cache
                .macd
                .signal_line
                .get(m1)
                .copied()
                .flatten()
                .filter(|v| v.is_finite())
                .unwrap_or(4.0);
            let pad = ((mv0 - mv1).abs() * 0.25).max(0.5);
            let lo = mv0.min(mv1) - pad;
            let hi = mv0.max(mv1) + pad;
            showcase.push(make_drawing(
                DrawingPaneTarget::Oscillator(OscillatorPaneId::Macd),
                DrawingGeometry::Rectangle {
                    corner_a: DrawingAnchor::new(m0 as f64, lo),
                    corner_b: DrawingAnchor::new(m1 as f64, hi),
                },
                Some(DrawingStyle {
                    color: DrawingColorPreset::ElectricBlue,
                    line_width: 1.5,
                    dash: LineDashStyle::Dashed,
                    fill_opacity: 0.14,
                }),
                false,
            ));
        }

        // 8. ATR Sub-Pane: Volatility threshold HorizontalLine
        if self.indicators.show_atr {
            let a_idx = idx_at(0.70);
            let atr_val = self
                .indicator_cache
                .atr
                .get(a_idx)
                .copied()
                .flatten()
                .filter(|v| v.is_finite())
                .unwrap_or(18.0);
            showcase.push(make_drawing(
                DrawingPaneTarget::Oscillator(OscillatorPaneId::Atr),
                DrawingGeometry::HorizontalLine {
                    anchor: DrawingAnchor::new(a_idx as f64, atr_val),
                },
                Some(DrawingStyle {
                    color: DrawingColorPreset::Magenta,
                    line_width: 1.4,
                    dash: LineDashStyle::Dashed,
                    fill_opacity: 0.0,
                }),
                false,
            ));
        }

        let _ = self.drawings.replace_all(showcase, Some(fib_entry_index));
    }

    /// Returns the causal `base_candles` index governing order submission and mark-to-market PnL.
    #[inline]
    #[must_use]
    pub fn current_trading_cursor_base_index(&self) -> usize {
        if self.base_candles.is_empty() {
            0
        } else if self.replay.is_active() {
            self.replay
                .base_cursor_index
                .min(self.base_candles.len() - 1)
        } else {
            self.base_candles.len() - 1
        }
    }

    /// Returns the causal close price at [`Self::current_trading_cursor_base_index`].
    #[inline]
    #[must_use]
    pub fn current_mark_price(&self) -> f64 {
        let idx = self.current_trading_cursor_base_index();
        self.base_candles
            .get(idx)
            .or_else(|| self.candles.last())
            .map_or(0.0, |c| c.close)
    }

    /// Enters interactive on-chart `Jump to Bar` scrubber selection (`✂ Cut Bar`), pausing playback
    /// while preserving any active causal replay cutoff (`DEF-001`).
    pub fn enter_replay_cut_selection(&mut self) {
        if self.base_candles.is_empty() {
            return;
        }
        self.drawings.cancel_active_gesture();
        self.drawings.set_tool(DrawingTool::Select);
        self.show_replay_bar = true;
        self.replay.enter_cut_selection();
    }

    /// Cancels interactive `Jump to Bar` scrubber selection without altering the replay cursor.
    pub fn cancel_replay_cut_selection(&mut self) {
        self.replay.cancel_cut_selection();
    }

    /// Handles a primary click on `clicked_active_idx` while `self.replay.selecting_cut_bar` is active.
    pub fn handle_chart_cut_bar_click(&mut self, clicked_active_idx: usize) -> bool {
        let max_allowed = self
            .replay
            .is_active()
            .then_some(self.replay.base_cursor_index);
        let Some(base_idx) = ReplayController::resolve_cut_base_index(
            &self.base_candles,
            &self.candles,
            self.active_timeframe,
            clicked_active_idx,
            max_allowed,
        ) else {
            self.replay.cancel_cut_selection();
            return false;
        };

        if self.replay.is_active() {
            if let Some(tr) = self.replay.jump_to_base_index(&self.base_candles, base_idx) {
                self.apply_replay_transition(tr);
            } else {
                self.replay.cancel_cut_selection();
            }
            true
        } else {
            self.start_replay_at_base_index(base_idx)
        }
    }

    /// Starts (or re-anchors) causal Bar Replay at `base_index` on `self.base_candles`, slicing all
    /// higher-timeframe OHLCV series, indicators, drawings, and paper-trading state to `<= cutoff`.
    pub fn start_replay_at_base_index(&mut self, base_index: usize) -> bool {
        if self.base_candles.is_empty() {
            return false;
        }
        self.drawings.cancel_active_gesture();
        if !self.candles.is_empty() {
            // Stamp canonical UTC timestamps onto all pre-existing drawings before slicing candles.
            self.drawings.remap_timeframe(
                &self.candles,
                self.active_timeframe,
                &self.candles,
                self.active_timeframe,
            );
        }

        let clamped_idx = base_index.min(self.base_candles.len() - 1);
        let Some(cutoff_ts) = self
            .replay
            .start_at_base_index(&self.base_candles, clamped_idx)
        else {
            return false;
        };
        self.show_replay_bar = true;
        self.scanner_context = None;
        self.last_trade_status = None;

        self.trading.rewind_to(
            &self.base_candles,
            self.replay.start_base_index,
            clamped_idx,
        );

        self.candles = self
            .replay
            .materialize_candles(
                &self.base_candles,
                self.base_timeframe,
                self.active_timeframe,
            )
            .unwrap_or_else(|_| self.base_candles[..=clamped_idx].to_vec());

        self.drawings
            .set_replay_cutoff_timestamp(Some(cutoff_ts), self.candles.len());

        self.viewport.kinetic = None;
        self.viewport_focus_lock = None;
        self.viewport.time_scale.set_total_bars(self.candles.len());
        self.viewport.time_scale.set_right_offset(16.0);
        if self.viewport.main_price_scale.is_auto_scale() {
            self.viewport.main_price_scale.reset_auto_scale();
        }
        self.viewport.volume_price_scale.reset_auto_scale();
        self.viewport.rsi_price_scale.reset_auto_scale();
        self.viewport.macd_price_scale.reset_auto_scale();
        self.viewport.atr_price_scale.reset_auto_scale();
        self.viewport.rs_price_scale.reset_auto_scale();
        self.viewport.equity_price_scale.reset_auto_scale();
        self.focused_backtest_trade_id = None;

        self.dataset_generation = self.dataset_generation.wrapping_add(1).max(1);
        self.sync_indicators_and_benchmarks();
        self.sync_alerts_causal();
        self.invalidate_or_sync_backtest_after_mutation();
        true
    }

    /// Synchronizes trading execution/rewind, causal OHLCV materialization, drawing timestamps,
    /// viewport camera tracking, and indicator cache after a [`ReplayTransition`].
    pub fn apply_replay_transition(&mut self, tr: ReplayTransition) {
        if self.base_candles.is_empty() {
            return;
        }
        if tr.new_base_index > tr.prev_base_index {
            self.trading.advance_through_bars(
                &self.base_candles,
                tr.prev_base_index.saturating_add(1),
                tr.new_base_index,
            );
        } else if tr.new_base_index < tr.prev_base_index {
            self.last_trade_status = None;
            if !self.candles.is_empty() {
                self.drawings.remap_timeframe(
                    &self.candles,
                    self.active_timeframe,
                    &self.candles,
                    self.active_timeframe,
                );
            }
            self.trading.rewind_to(
                &self.base_candles,
                self.replay.start_base_index,
                tr.new_base_index,
            );
            if self.journal.entries.iter().any(|e| {
                e.symbol.eq_ignore_ascii_case(&self.symbol)
                    && matches!(e.source, JournalEntrySource::PaperTrade { .. })
            }) {
                let venue = self.active_exchange_venue();
                let _ = self.journal.sync_paper_closed_trades(
                    &self.symbol,
                    venue,
                    self.trading.closed_trades(),
                );
            }
        }

        let old_len = self.candles.len();
        let old_ro = self.viewport.time_scale.right_offset();
        let old_last_visible = if old_len > 0 {
            let old_b = (old_len - 1) as i64;
            self.viewport
                .time_scale
                .visible_strict_range()
                .is_none_or(|strict| old_b >= strict.left && old_b <= strict.right)
        } else {
            true
        };

        self.candles = self
            .replay
            .materialize_candles(
                &self.base_candles,
                self.base_timeframe,
                self.active_timeframe,
            )
            .unwrap_or_else(|_| {
                let end = tr.new_base_index.min(self.base_candles.len() - 1);
                self.base_candles[..=end].to_vec()
            });
        let new_len = self.candles.len();

        self.drawings.refresh_materialized_anchor_timestamps(
            old_len,
            &self.candles,
            self.active_timeframe,
        );
        self.drawings
            .set_replay_cutoff_timestamp(Some(tr.cutoff_timestamp), new_len);

        self.viewport.kinetic = None;
        self.viewport_focus_lock = None;
        self.viewport.time_scale.set_total_bars(new_len);
        if new_len < old_len {
            let removed = (old_len - new_len) as f64;
            if old_last_visible {
                self.viewport
                    .time_scale
                    .set_right_offset(old_ro.clamp(8.0, 24.0));
            } else {
                self.viewport.time_scale.set_right_offset(old_ro + removed);
            }
        }

        if self.viewport.main_price_scale.is_auto_scale() {
            self.viewport.main_price_scale.reset_auto_scale();
        }
        self.viewport.volume_price_scale.reset_auto_scale();
        self.viewport.rsi_price_scale.reset_auto_scale();
        self.viewport.macd_price_scale.reset_auto_scale();
        self.viewport.atr_price_scale.reset_auto_scale();
        self.viewport.rs_price_scale.reset_auto_scale();
        self.viewport.equity_price_scale.reset_auto_scale();

        self.dataset_generation = self.dataset_generation.wrapping_add(1).max(1);
        self.sync_indicators_and_benchmarks();
        self.sync_alerts_causal();
        self.invalidate_or_sync_backtest_after_mutation();
    }

    /// Steps causal Bar Replay forward by `steps` units (`ActiveBar` or `BaseBar`).
    pub fn step_replay_forward(&mut self, steps: usize) -> bool {
        if let Some(tr) = self
            .replay
            .step_forward(&self.base_candles, self.active_timeframe, steps)
        {
            self.apply_replay_transition(tr);
            true
        } else {
            false
        }
    }

    /// Steps causal Bar Replay backward by `steps` units, rewinding paper-trading state bit-exactly.
    pub fn step_replay_backward(&mut self, steps: usize) -> bool {
        if let Some(tr) =
            self.replay
                .step_backward(&self.base_candles, self.active_timeframe, steps)
        {
            self.apply_replay_transition(tr);
            true
        } else {
            false
        }
    }

    /// Jumps causal Bar Replay back to `start_base_index`, rewinding paper-trading state bit-exactly.
    pub fn jump_replay_to_start(&mut self) -> bool {
        if let Some(tr) = self.replay.jump_to_start(&self.base_candles) {
            self.apply_replay_transition(tr);
            true
        } else {
            false
        }
    }

    /// Toggles auto-play (`Playing <-> Paused`) when Bar Replay is active.
    pub fn toggle_replay_play_pause(&mut self) -> bool {
        self.replay.toggle_play_pause(&self.base_candles)
    }

    /// Exits causal Bar Replay and restores full unmasked chart visibility while advancing any
    /// working orders/brackets causally to the live close so runtime state matches persisted restore.
    pub fn exit_replay(&mut self) {
        let was_active = self.replay.is_active();
        let prev_cursor = self.replay.base_cursor_index;
        self.replay.exit_replay();
        self.scanner_context = None;
        if !was_active || self.base_candles.is_empty() {
            return;
        }

        let last_base = self.base_candles.len() - 1;
        if prev_cursor < last_base
            && (!self.trading.commands.is_empty() || self.trading.open_position().is_some())
        {
            self.trading.advance_through_bars(
                &self.base_candles,
                prev_cursor.saturating_add(1),
                last_base,
            );
        }

        let old_len = self.candles.len();
        self.candles = self
            .replay
            .materialize_candles(
                &self.base_candles,
                self.base_timeframe,
                self.active_timeframe,
            )
            .unwrap_or_else(|_| self.base_candles.clone());
        self.drawings.refresh_materialized_anchor_timestamps(
            old_len,
            &self.candles,
            self.active_timeframe,
        );
        self.drawings
            .set_replay_cutoff_timestamp(None, self.candles.len());

        self.viewport.kinetic = None;
        self.viewport_focus_lock = None;
        self.viewport.time_scale.set_total_bars(self.candles.len());
        if self.viewport.main_price_scale.is_auto_scale() {
            self.viewport.main_price_scale.reset_auto_scale();
        }
        self.viewport.volume_price_scale.reset_auto_scale();
        self.viewport.rsi_price_scale.reset_auto_scale();
        self.viewport.macd_price_scale.reset_auto_scale();
        self.viewport.atr_price_scale.reset_auto_scale();
        self.viewport.rs_price_scale.reset_auto_scale();
        self.viewport.equity_price_scale.reset_auto_scale();

        self.dataset_generation = self.dataset_generation.wrapping_add(1).max(1);
        self.sync_indicators_and_benchmarks();
        self.sync_alerts_causal();
        self.invalidate_or_sync_backtest_after_mutation();
    }

    /// Parses the workstation Order Ticket fields and submits `side` (`Buy` or `Sell`) at the
    /// current causal replay/live cursor.
    pub fn submit_ticket_order(
        &mut self,
        side: OrderSide,
    ) -> Result<u64, crate::trading::TradingError> {
        let qty = parse_ticket_qty(&self.ticket_qty_input).inspect_err(|err| {
            self.last_trade_status = Some((true, err.to_string()));
        })?;
        let kind = match self.ticket_kind {
            TicketOrderKind::Market => OrderKind::Market,
            TicketOrderKind::Limit => {
                let limit_price = parse_ticket_required_price(&self.ticket_price_input)
                    .inspect_err(|err| {
                        self.last_trade_status = Some((true, err.to_string()));
                    })?;
                OrderKind::Limit { limit_price }
            }
            TicketOrderKind::Stop => {
                let stop_price = parse_ticket_required_price(&self.ticket_price_input)
                    .inspect_err(|err| {
                        self.last_trade_status = Some((true, err.to_string()));
                    })?;
                OrderKind::Stop { stop_price }
            }
        };
        let stop_loss = parse_ticket_optional_price(&self.ticket_sl_input).inspect_err(|err| {
            self.last_trade_status = Some((true, err.to_string()));
        })?;
        let take_profit =
            parse_ticket_optional_price(&self.ticket_tp_input).inspect_err(|err| {
                self.last_trade_status = Some((true, err.to_string()));
            })?;

        let cursor_idx = self.current_trading_cursor_base_index();
        let req = OrderRequest {
            side,
            kind,
            quantity: qty,
            stop_loss,
            take_profit,
        };
        match self
            .trading
            .submit_order(req, &self.base_candles, cursor_idx)
        {
            Ok(order_id) => {
                let msg = match kind {
                    OrderKind::Market => format!(
                        "Filled #{} {} {:.4} MKT @ {:.2}",
                        order_id,
                        side.short_label(),
                        qty,
                        self.current_mark_price()
                    ),
                    OrderKind::Limit { limit_price } => format!(
                        "Working #{} {} {:.4} LMT @ {:.2}",
                        order_id,
                        side.short_label(),
                        qty,
                        limit_price
                    ),
                    OrderKind::Stop { stop_price } => format!(
                        "Working #{} {} {:.4} STP @ {:.2}",
                        order_id,
                        side.short_label(),
                        qty,
                        stop_price
                    ),
                };
                self.last_trade_status = Some((false, msg));
                Ok(order_id)
            }
            Err(err) => {
                self.last_trade_status = Some((true, err.to_string()));
                Err(err)
            }
        }
    }

    /// Populates the Paper Trading Order Ticket from the currently selected main-pane [`crate::drawings::Drawing`].
    /// When the selected drawing is a [`DrawingGeometry::RiskRewardBox`], also computes tick-snapped brackets
    /// and board-lot-compliant position quantity via [`compute_trade_plan`].
    pub fn sync_ticket_from_selected_drawing(&mut self) -> bool {
        if self
            .drawings
            .selected_drawing()
            .is_some_and(|d| matches!(d.geometry, DrawingGeometry::RiskRewardBox { .. }))
        {
            let _ = self.sync_planner_from_selected_rr_box();
            if self.sync_ticket_from_risk_planner() {
                return true;
            }
        }
        let Some(sync) = self
            .drawings
            .selected_drawing()
            .and_then(extract_ticket_from_drawing)
        else {
            self.last_trade_status = Some((
                true,
                "select a Main-pane drawing to sync Entry / SL / TP".to_owned(),
            ));
            return false;
        };
        let mark = self.current_mark_price();
        self.ticket_kind = match sync.side {
            OrderSide::Buy if mark > 0.0 && sync.entry_price > mark => TicketOrderKind::Stop,
            OrderSide::Sell if mark > 0.0 && sync.entry_price < mark => TicketOrderKind::Stop,
            _ => TicketOrderKind::Limit,
        };
        self.ticket_price_input = format!("{:.2}", sync.entry_price);
        self.ticket_sl_input = sync
            .stop_loss
            .map_or_else(String::new, |v| format!("{v:.2}"));
        self.ticket_tp_input = sync
            .take_profit
            .map_or_else(String::new, |v| format!("{v:.2}"));
        self.show_trading_panel = true;
        self.right_dock_tab = RightDockTab::Trading;
        self.last_trade_status = Some((
            false,
            format!(
                "Synced {} {} @ {:.2}",
                sync.side.short_label(),
                self.ticket_kind.label(),
                sync.entry_price
            ),
        ));
        true
    }

    /// Applies the ticket `SL` / `TP` inputs to the currently open position.
    pub fn apply_ticket_brackets_to_position(
        &mut self,
    ) -> Result<(), crate::trading::TradingError> {
        let sl = parse_ticket_optional_price(&self.ticket_sl_input).inspect_err(|err| {
            self.last_trade_status = Some((true, err.to_string()));
        })?;
        let tp = parse_ticket_optional_price(&self.ticket_tp_input).inspect_err(|err| {
            self.last_trade_status = Some((true, err.to_string()));
        })?;
        let cursor_idx = self.current_trading_cursor_base_index();
        match self
            .trading
            .update_position_brackets(sl, tp, &self.base_candles, cursor_idx)
        {
            Ok(()) => {
                self.last_trade_status = Some((false, "Updated position SL / TP".to_owned()));
                Ok(())
            }
            Err(err) => {
                self.last_trade_status = Some((true, err.to_string()));
                Err(err)
            }
        }
    }

    /// Closes the entire open position at the current causal bar close (`Flatten`).
    pub fn flatten_open_position(&mut self) -> Result<(), crate::trading::TradingError> {
        let cursor_idx = self.current_trading_cursor_base_index();
        match self
            .trading
            .flatten_position(&self.base_candles, cursor_idx)
        {
            Ok(fill) => {
                self.last_trade_status = Some((
                    false,
                    format!(
                        "Flattened {:.4} @ {:.2} (PnL {:+.2})",
                        fill.quantity,
                        fill.price,
                        fill.realized_net_pnl.unwrap_or(0.0)
                    ),
                ));
                Ok(())
            }
            Err(err) => {
                self.last_trade_status = Some((true, err.to_string()));
                Err(err)
            }
        }
    }

    /// Cancels a `Pending` working order by ID.
    pub fn cancel_working_order(
        &mut self,
        order_id: u64,
    ) -> Result<(), crate::trading::TradingError> {
        let cursor_idx = self.current_trading_cursor_base_index();
        match self
            .trading
            .cancel_order(order_id, &self.base_candles, cursor_idx)
        {
            Ok(()) => {
                self.last_trade_status = Some((false, format!("Cancelled order #{order_id}")));
                Ok(())
            }
            Err(err) => {
                self.last_trade_status = Some((true, err.to_string()));
                Err(err)
            }
        }
    }

    /// Resets the Paper Trading account cleanly at the start of the active replay session (or current bar).
    pub fn reset_trading_session(&mut self) {
        let start_idx = if self.replay.is_active() {
            self.replay.start_base_index
        } else {
            self.current_trading_cursor_base_index()
        };
        let cursor_idx = self.current_trading_cursor_base_index();
        self.trading.reset_at(&self.base_candles, start_idx);
        if cursor_idx > start_idx {
            self.trading.advance_through_bars(
                &self.base_candles,
                start_idx.saturating_add(1),
                cursor_idx,
            );
        }
        self.last_trade_status = Some((
            false,
            format!("Reset account (${:.0})", self.trading.config.initial_cash),
        ));
    }

    // =========================================================================
    // Milestone R08 — Watchlist & Scanner, Causal Alerts & Command Palette API
    // =========================================================================

    /// Lazily initializes the bundled `13`-instrument multi-asset Watchlist universe once per session.
    pub fn ensure_watchlist_universe_loaded(&mut self) {
        if self.watchlist_universe.is_empty() {
            self.watchlist_universe = load_bundled_watchlist_universe();
        }
    }

    fn rebuild_custom_peer_feeds(&mut self) {
        self.custom_peer_feeds.clear();
        if self.data_source != DataSourcePreset::CustomFile || self.available_symbols.len() <= 1 {
            return;
        }
        for sym in self.available_symbols.clone().into_iter().take(32) {
            let sym_upper = sym.trim().to_ascii_uppercase();
            let instrument_id = if let Some((fmt, _)) = self.in_memory_custom_doc {
                format!(
                    "CustomMem:{fmt:?}:{:016x}:{sym_upper}",
                    self.in_memory_custom_doc_hash
                )
            } else {
                format!("CustomFile:{}:{sym_upper}", self.import_path_input.trim())
            };
            if sym.eq_ignore_ascii_case(&self.symbol) {
                self.custom_peer_feeds.push(WatchlistFeedEntry {
                    instrument_id,
                    preset: DataSourcePreset::CustomFile,
                    symbol: sym,
                    base_timeframe: self.base_timeframe,
                    candles: self.base_candles.clone(),
                });
                continue;
            }
            let opts = DataLoadOptions {
                symbol_filter: Some(sym.clone()),
                ..DataLoadOptions::default()
            };
            let loaded = if let Some((fmt, ref doc)) = self.in_memory_custom_doc {
                load_ohlcv_bytes(doc.as_bytes(), fmt, &opts).ok()
            } else {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let p = self.import_path_input.trim();
                    (!p.is_empty())
                        .then(|| crate::data::load_ohlcv_file(p, &opts).ok())
                        .flatten()
                }
                #[cfg(target_arch = "wasm32")]
                {
                    None
                }
            };
            if let Some(rep) = loaded {
                self.custom_peer_feeds.push(WatchlistFeedEntry {
                    instrument_id,
                    preset: DataSourcePreset::CustomFile,
                    symbol: rep.symbol,
                    base_timeframe: rep.base_timeframe,
                    candles: rep.candles,
                });
            }
        }
    }

    /// Recomputes raw Watchlist snapshot rows and Historical Signal hits only when dataset generation,
    /// active instrument key, causal Replay cutoff (`DEF-001`), or lookback window changes (`0.000 ms` on hover/pan/zoom).
    pub fn sync_scanner_caches(&mut self) {
        self.ensure_watchlist_universe_loaded();
        let active_key = self.current_instrument_key();
        let replay_cutoff = self.replay.cutoff_timestamp(&self.base_candles);
        let lookback = self.signal_lookback_bars.clamp(5, 500);
        let incomplete_active_tail = self.replay.is_active()
            && self.active_timeframe > self.base_timeframe
            && !self.base_candles.is_empty()
            && {
                let idx = self
                    .replay
                    .base_cursor_index
                    .min(self.base_candles.len() - 1);
                idx + 1 < self.base_candles.len()
                    && self
                        .active_timeframe
                        .bucket_start_timestamp(self.base_candles[idx].timestamp)
                        == self
                            .active_timeframe
                            .bucket_start_timestamp(self.base_candles[idx + 1].timestamp)
            };
        let cache_key = (
            self.dataset_generation,
            active_key.clone(),
            replay_cutoff,
            lookback,
            incomplete_active_tail,
        );
        if self.scanner_synced_key.as_ref() == Some(&cache_key) {
            return;
        }

        let mut rows =
            Vec::with_capacity(self.watchlist_universe.len() + self.custom_peer_feeds.len() + 1);
        let mut hits = Vec::new();

        let active_in_universe = self
            .watchlist_universe
            .iter()
            .any(|e| e.instrument_id == active_key)
            || (self.data_source == DataSourcePreset::CustomFile
                && self
                    .custom_peer_feeds
                    .iter()
                    .any(|e| e.instrument_id == active_key));

        if !active_in_universe && !self.candles.is_empty() {
            // `self.candles` is already causally materialized to `<= replay_cutoff`!
            let (row_opt, entry_hits) = evaluate_symbol_scan_and_signals(
                &active_key,
                &self.symbol,
                self.data_source.short_label(),
                self.active_timeframe,
                &self.candles,
                replay_cutoff,
                lookback,
                incomplete_active_tail,
            );
            if let Some(row) = row_opt {
                rows.push(row);
            }
            hits.extend(entry_hits);
        }

        if self.data_source == DataSourcePreset::CustomFile && !self.custom_peer_feeds.is_empty() {
            for entry in &self.custom_peer_feeds {
                if entry.instrument_id == active_key {
                    let (row_opt, entry_hits) = evaluate_symbol_scan_and_signals(
                        &entry.instrument_id,
                        &entry.symbol,
                        entry.preset.short_label(),
                        self.active_timeframe,
                        &self.candles,
                        replay_cutoff,
                        lookback,
                        incomplete_active_tail,
                    );
                    if let Some(row) = row_opt {
                        rows.push(row);
                    }
                    hits.extend(entry_hits);
                } else {
                    let (row_opt, entry_hits) = evaluate_symbol_scan_and_signals(
                        &entry.instrument_id,
                        &entry.symbol,
                        entry.preset.short_label(),
                        entry.base_timeframe,
                        &entry.candles,
                        replay_cutoff,
                        lookback,
                        false,
                    );
                    if let Some(row) = row_opt {
                        rows.push(row);
                    }
                    hits.extend(entry_hits);
                }
            }
        }

        for entry in &self.watchlist_universe {
            if entry.instrument_id == active_key {
                let (row_opt, entry_hits) = evaluate_symbol_scan_and_signals(
                    &entry.instrument_id,
                    &entry.symbol,
                    entry.preset.short_label(),
                    self.active_timeframe,
                    &self.candles,
                    replay_cutoff,
                    lookback,
                    incomplete_active_tail,
                );
                if let Some(row) = row_opt {
                    rows.push(row);
                }
                hits.extend(entry_hits);
            } else {
                // Strict cross-sectional `DEF-001` causal gate: pass `replay_cutoff` directly so
                // future-only peer feeds (`first.timestamp > cut`) are withheld (`0` future leak!)
                // while overlapping & historical peer feeds slice cleanly to `<= cut`.
                let (row_opt, entry_hits) = evaluate_symbol_scan_and_signals(
                    &entry.instrument_id,
                    &entry.symbol,
                    entry.preset.short_label(),
                    entry.base_timeframe,
                    &entry.candles,
                    replay_cutoff,
                    lookback,
                    false,
                );
                if let Some(row) = row_opt {
                    rows.push(row);
                }
                hits.extend(entry_hits);
            }
        }

        self.raw_watchlist_rows_cache = rows;
        self.raw_historical_hits_cache = hits;
        self.scanner_synced_key = Some(cache_key);
    }

    /// Returns filtered & deterministically sorted Watchlist rows (`None`/`NaN` metrics always last).
    #[must_use]
    pub fn filtered_watchlist_rows(&mut self) -> Vec<SymbolScanRow> {
        self.sync_scanner_caches();
        filter_and_sort_watchlist(
            &self.raw_watchlist_rows_cache,
            &self.scanner_filter,
            self.watchlist_sort_col,
            self.watchlist_sort_asc,
        )
    }

    /// Returns filtered & chronologically newest-first Historical Signal hits across the universe.
    #[must_use]
    pub fn filtered_historical_signals(&mut self) -> Vec<HistoricalSignalHit> {
        self.sync_scanner_caches();
        filter_historical_signals(&self.raw_historical_hits_cache, &self.scanner_filter)
    }

    /// Switches to `(preset, symbol)` in one click while parking outgoing drawings, replay, trading, and alerts.
    pub fn select_watchlist_instrument(&mut self, preset: DataSourcePreset, symbol: &str) {
        if preset == DataSourcePreset::CustomFile {
            if !self.symbol.eq_ignore_ascii_case(symbol) {
                self.select_symbol_in_source(symbol);
            }
            return;
        }
        if preset == DataSourcePreset::SyntheticVnindex {
            if self.data_source != DataSourcePreset::SyntheticVnindex {
                self.set_dataset_size(self.dataset_bar_count.max(10));
            }
            return;
        }
        if self.data_source == preset && self.symbol.eq_ignore_ascii_case(symbol) {
            return;
        }
        let rep = load_preset_dataset(preset, self.dataset_bar_count.max(10), Some(symbol));
        let summary = format!(
            "Loaded {} · {} ({} {} bars)",
            rep.symbol,
            preset.short_label(),
            rep.candles.len(),
            rep.base_timeframe
        );
        self.apply_loaded_report_internal(rep, preset, CustomSourceUpdate::Clear, true);
        self.last_load_status = Some((false, summary));
    }

    /// Launches a causal Bar Replay session directly from a [`HistoricalSignalHit`] in either
    /// [`ScannerReplayIntent::BlindPractice`] (cuts `15` bars prior & masks signal details until reached)
    /// or [`ScannerReplayIntent::SignalReview`] (jumps directly onto the signal bar revealed).
    pub fn start_scanner_signal_replay(
        &mut self,
        hit: HistoricalSignalHit,
        intent: ScannerReplayIntent,
    ) -> bool {
        self.ensure_watchlist_universe_loaded();
        let active_key = self.current_instrument_key();
        if hit.instrument_id != active_key {
            if let Some((preset, sym)) = self
                .watchlist_universe
                .iter()
                .find(|e| e.instrument_id == hit.instrument_id)
                .map(|e| (e.preset, e.symbol.clone()))
            {
                self.select_watchlist_instrument(preset, &sym);
            } else if let Some(sym) = self
                .custom_peer_feeds
                .iter()
                .find(|e| e.instrument_id == hit.instrument_id)
                .map(|e| e.symbol.clone())
            {
                self.select_watchlist_instrument(DataSourcePreset::CustomFile, &sym);
            } else if hit.instrument_id.starts_with("Preset:Synth:") {
                self.select_watchlist_instrument(DataSourcePreset::SyntheticVnindex, &hit.symbol);
            }
        }
        if self.current_instrument_key() != hit.instrument_id {
            return false;
        }

        if self.base_candles.is_empty() {
            return false;
        }
        if self.replay.is_active() {
            self.exit_replay();
        }
        if self.base_timeframe.can_aggregate_to(hit.timeframe)
            && self.active_timeframe != hit.timeframe
        {
            let _ = self.set_timeframe(hit.timeframe);
        }
        if self.candles.is_empty() {
            return false;
        }

        let sig_active_idx = self
            .candles
            .partition_point(|c| c.timestamp <= hit.timestamp)
            .saturating_sub(1)
            .min(self.candles.len() - 1);

        let Some(sig_close_base_idx) = ReplayController::resolve_cut_base_index(
            &self.base_candles,
            &self.candles,
            self.active_timeframe,
            sig_active_idx,
            None,
        ) else {
            return false;
        };
        let reveal_ts = self.base_candles[sig_close_base_idx].timestamp;

        let target_active_idx = match intent {
            ScannerReplayIntent::SignalReview => sig_active_idx,
            ScannerReplayIntent::BlindPractice => {
                if sig_active_idx == 0 {
                    0
                } else {
                    sig_active_idx
                        .saturating_sub(DEFAULT_BLIND_PRACTICE_LOOKBACK_BARS)
                        .min(sig_active_idx - 1)
                }
            }
        };

        let Some(cut_base_idx) = ReplayController::resolve_cut_base_index(
            &self.base_candles,
            &self.candles,
            self.active_timeframe,
            target_active_idx,
            None,
        ) else {
            return false;
        };

        let started = self.start_replay_at_base_index(cut_base_idx);
        if started {
            self.scanner_context = Some(ScannerSignalContext::with_reveal_timestamp(
                intent, hit, reveal_ts,
            ));
            self.scanner_synced_key = None;
        }
        started
    }

    /// Evaluates causal alerts whenever `dataset_generation`, main-pane drawing geometry, causal
    /// Replay cutoff, or alert rule definitions change (`0.000 ms` on idle hover/pan/zoom frames).
    pub fn sync_alerts_causal(&mut self) {
        let cutoff = self.replay.cutoff_timestamp(&self.base_candles);
        let draw_digest = compute_drawings_alert_digest(&self.drawings);
        let pre_rules_digest = compute_alert_rules_digest(&self.alerts);
        let check_key = (
            self.dataset_generation,
            draw_digest,
            cutoff,
            pre_rules_digest,
        );
        if self.alerts_synced_key == Some(check_key) {
            return;
        }
        if self.alerts.rules.is_empty() {
            self.alerts.triggers.clear();
            self.alerts_synced_key = Some(check_key);
            return;
        }
        let cursor_idx = self.current_trading_cursor_base_index();
        self.alerts.evaluate_causal(
            &self.base_candles,
            self.base_timeframe,
            &self.candles,
            self.active_timeframe,
            cursor_idx,
            cutoff,
            &self.drawings,
        );
        let post_rules_digest = compute_alert_rules_digest(&self.alerts);
        self.alerts_synced_key = Some((
            self.dataset_generation,
            draw_digest,
            cutoff,
            post_rules_digest,
        ));
    }

    // =========================================================================
    // Milestone R09 — Deterministic Strategy Backtest Lab & Equity Pane API
    // =========================================================================

    /// Returns the inclusive end-of-bar causal timestamp (`cursor_open_ts + base_timeframe.seconds() - 1`)
    /// when Bar Replay is active (`[CRITICAL-01]` cross-timeframe `DEF-001` gate).
    #[inline]
    #[must_use]
    pub fn replay_causal_close_timestamp(&self) -> Option<i64> {
        let open_ts = self.replay.cutoff_timestamp(&self.base_candles)?;
        let span = self.base_timeframe.seconds().max(1).saturating_sub(1);
        Some(open_ts.saturating_add(span))
    }

    /// Returns `true` when causal Bar Replay is active on an aggregated higher timeframe and the
    /// current `base_cursor_index` has not yet closed the active higher-timeframe bucket (`DEF-001`,
    /// `[MAJOR-04]`: pure prefix check without inspecting any future `base_candles[idx + 1]`!).
    #[must_use]
    pub fn is_active_tail_incomplete(&self) -> bool {
        let Some(close_ts) = self.replay_causal_close_timestamp() else {
            return false;
        };
        let Some(last_active) = self.candles.last() else {
            return false;
        };
        is_bucket_incomplete_at_close(
            last_active.timestamp,
            close_ts,
            self.base_timeframe,
            self.active_timeframe,
        )
    }

    fn invalidate_or_sync_backtest_after_mutation(&mut self) {
        if self.show_strategy_panel || self.show_equity_pane || self.show_backtest_trades {
            self.sync_backtest_causal();
        } else {
            self.backtest_result = None;
            self.backtest_synced_key = None;
            self.sweep_report = None;
            self.preset_battle_rows.clear();
            self.sweep_synced_key = None;
            self.battle_report = None;
            self.battle_synced_key = None;
            self.walk_forward_report = None;
            self.wf_synced_key = None;
            self.portfolio_report = None;
            self.portfolio_synced_key = None;
            self.portfolio_mapped_curve = None;
        }
    }

    /// Evaluates the causal strategy backtest (and refreshes/invalidates `Sweep`, `Battle`,
    /// `Walk-Forward` & `Portfolio` caches across Bar Replay cutoffs per `DEF-001`) only when active
    /// and dirty (`0.000 ms` on hover/pan/zoom).
    pub fn sync_backtest_causal(&mut self) {
        let any_active =
            self.show_strategy_panel || self.show_equity_pane || self.show_backtest_trades;
        if !any_active {
            return;
        }
        self.backtest_config.sanitize();
        self.wf_config.sanitize();
        let replay_close_cutoff = self.replay_causal_close_timestamp();
        let incomplete_tail = self.is_active_tail_incomplete();
        let cfg_digest = compute_backtest_config_digest(&self.backtest_config);
        let wf_digest = compute_walk_forward_config_digest(&self.wf_config);
        let last_ts = self.candles.last().map(|c| c.timestamp);
        let check_key = (
            self.dataset_generation,
            self.candles.len(),
            last_ts,
            replay_close_cutoff,
            incomplete_tail,
            cfg_digest,
        );

        if self.backtest_synced_key != Some(check_key) {
            let opts = BacktestRunOptions {
                replay_cutoff_timestamp: replay_close_cutoff,
                incomplete_tail_bar: incomplete_tail,
            };
            let res = run_backtest_with_options(
                &self.symbol,
                self.active_timeframe,
                &self.candles,
                &self.backtest_config,
                opts,
            );
            if let Some(tid) = self.focused_backtest_trade_id
                && !res.trades.iter().any(|t| t.trade_id == tid)
            {
                self.focused_backtest_trade_id = None;
            }
            self.backtest_result = Some(res);
            self.backtest_synced_key = Some(check_key);
            if self.viewport.equity_price_scale.is_auto_scale() {
                self.viewport.equity_price_scale.reset_auto_scale();
            }
        }

        // Guard `Sweep` & `Battle` caches against stale full-history numbers during Bar Replay (`DEF-001`).
        let sweep_key = (
            self.dataset_generation,
            replay_close_cutoff,
            incomplete_tail,
            cfg_digest,
            self.sweep_ranking,
        );
        if self.sweep_synced_key != Some(sweep_key) {
            if self.show_strategy_panel
                && self.right_dock_tab == RightDockTab::StrategyLab
                && self.strategy_lab_tab == StrategyLabSubTab::Sweep
            {
                self.run_active_parameter_sweep();
            } else {
                self.sweep_report = None;
                self.preset_battle_rows.clear();
                self.sweep_synced_key = None;
            }
        }

        let battle_key = (
            self.current_instrument_key(),
            self.dataset_generation,
            self.active_timeframe,
            replay_close_cutoff,
            cfg_digest,
            self.sweep_ranking,
        );
        if self.battle_synced_key.as_ref() != Some(&battle_key) {
            if self.show_strategy_panel
                && self.right_dock_tab == RightDockTab::StrategyLab
                && self.strategy_lab_tab == StrategyLabSubTab::Battle
            {
                self.run_universe_strategy_battle();
            } else {
                self.battle_report = None;
                self.battle_synced_key = None;
            }
        }

        // Guard `Walk-Forward` & `Portfolio` caches across Bar Replay cutoffs (`DEF-001`).
        let wf_key = (
            self.dataset_generation,
            self.candles.len(),
            last_ts,
            replay_close_cutoff,
            incomplete_tail,
            cfg_digest,
            wf_digest,
        );
        let wf_needed = (self.show_strategy_panel
            && self.right_dock_tab == RightDockTab::StrategyLab
            && self.strategy_lab_tab == StrategyLabSubTab::WalkForward)
            || (self.show_equity_pane
                && self.equity_pane_source == EquityPaneSource::WalkForwardOos);
        if self.wf_synced_key != Some(wf_key) {
            if wf_needed {
                self.run_active_walk_forward();
            } else {
                self.walk_forward_report = None;
                self.wf_synced_key = None;
            }
        }

        let port_key = (
            self.current_instrument_key(),
            self.dataset_generation,
            self.active_timeframe,
            replay_close_cutoff,
            cfg_digest,
            self.portfolio_universe_filter,
            self.portfolio_weighting,
        );
        let port_needed = (self.show_strategy_panel
            && self.right_dock_tab == RightDockTab::StrategyLab
            && self.strategy_lab_tab == StrategyLabSubTab::WalkForward)
            || (self.show_equity_pane
                && self.equity_pane_source == EquityPaneSource::MultiAssetPortfolio);
        if self.portfolio_synced_key.as_ref() != Some(&port_key) {
            if port_needed {
                self.run_active_portfolio_backtest();
            } else {
                self.portfolio_report = None;
                self.portfolio_synced_key = None;
                self.portfolio_mapped_curve = None;
            }
        } else if self.show_equity_pane
            && self.equity_pane_source == EquityPaneSource::MultiAssetPortfolio
        {
            self.refresh_portfolio_mapped_curve();
        }
    }

    /// Opens the right-dock Strategy Lab, activates the synchronized `Equity / Drawdown` sub-pane
    /// and chart trade connectors, and computes the deterministic causal backtest immediately.
    pub fn open_strategy_lab(&mut self) {
        self.show_strategy_panel = true;
        self.show_equity_pane = true;
        self.show_backtest_trades = true;
        self.right_dock_tab = RightDockTab::StrategyLab;
        self.viewport.equity_price_scale.reset_auto_scale();
        self.sync_backtest_causal();
    }

    /// Runs the deterministic 12-variant parameter grid sweep + 6-preset comparison on the active symbol.
    pub fn run_active_parameter_sweep(&mut self) {
        self.backtest_config.sanitize();
        let replay_close_cutoff = self.replay_causal_close_timestamp();
        let incomplete_tail = self.is_active_tail_incomplete();
        let cfg_digest = compute_backtest_config_digest(&self.backtest_config);
        let opts = BacktestRunOptions {
            replay_cutoff_timestamp: replay_close_cutoff,
            incomplete_tail_bar: incomplete_tail,
        };
        self.sweep_report = Some(run_parameter_sweep(
            &self.symbol,
            self.active_timeframe,
            &self.candles,
            &self.backtest_config,
            self.sweep_ranking,
            opts,
        ));
        self.preset_battle_rows = run_all_presets_battle(
            &self.symbol,
            self.active_timeframe,
            &self.candles,
            &self.backtest_config,
            self.sweep_ranking,
            opts,
        );
        self.sweep_synced_key = Some((
            self.dataset_generation,
            replay_close_cutoff,
            incomplete_tail,
            cfg_digest,
            self.sweep_ranking,
        ));
    }

    /// Runs the active `BacktestConfig` causally across the 13-instrument Watchlist universe (plus 6-preset comparison).
    pub fn run_universe_strategy_battle(&mut self) {
        self.ensure_watchlist_universe_loaded();
        self.backtest_config.sanitize();
        let replay_close_cutoff = self.replay_causal_close_timestamp();
        let incomplete_tail = self.is_active_tail_incomplete();
        let cfg_digest = compute_backtest_config_digest(&self.backtest_config);
        let opts = BacktestRunOptions {
            replay_cutoff_timestamp: replay_close_cutoff,
            incomplete_tail_bar: incomplete_tail,
        };
        self.preset_battle_rows = run_all_presets_battle(
            &self.symbol,
            self.active_timeframe,
            &self.candles,
            &self.backtest_config,
            self.sweep_ranking,
            opts,
        );

        let feeds: Vec<WatchlistFeedEntry> = if self.data_source == DataSourcePreset::CustomFile
            && !self.custom_peer_feeds.is_empty()
        {
            self.custom_peer_feeds.clone()
        } else {
            let mut list = self.watchlist_universe.clone();
            let active_key = self.current_instrument_key();
            if !list.iter().any(|e| e.instrument_id == active_key) && !self.base_candles.is_empty()
            {
                list.push(WatchlistFeedEntry {
                    instrument_id: active_key,
                    preset: self.data_source,
                    symbol: self.symbol.clone(),
                    base_timeframe: self.base_timeframe,
                    candles: self.base_candles.clone(),
                });
            }
            list
        };

        self.battle_report = Some(run_multi_symbol_battle_with_close_cutoff(
            &feeds,
            self.active_timeframe,
            &self.backtest_config,
            self.sweep_ranking,
            replay_close_cutoff,
        ));
        self.battle_synced_key = Some((
            self.current_instrument_key(),
            self.dataset_generation,
            self.active_timeframe,
            replay_close_cutoff,
            cfg_digest,
            self.sweep_ranking,
        ));
    }

    /// Centers the shared `TimeScale` camera onto `trade_id` and highlights its entry/exit halo on the chart.
    pub fn focus_backtest_trade(&mut self, trade_id: u64) -> bool {
        self.show_backtest_trades = true;
        self.sync_backtest_causal();
        let Some((entry_idx, exit_idx)) = self
            .backtest_result
            .as_ref()
            .and_then(|res| res.trades.iter().find(|t| t.trade_id == trade_id))
            .map(|t| (t.entry_bar_index, t.exit_bar_index))
        else {
            return false;
        };
        if self.candles.is_empty() {
            return false;
        }
        self.focused_backtest_trade_id = Some(trade_id);
        let plot_w = self.viewport.time_scale.width().max(320.0);
        let span_bars = (exit_idx.saturating_sub(entry_idx) + 1).max(6) as f64;
        let cur_bs = self.viewport.time_scale.bar_spacing();
        let target_bs = if span_bars * cur_bs > plot_w * 0.68 {
            (plot_w * 0.58 / span_bars).clamp(2.5, 16.0)
        } else {
            cur_bs.clamp(6.0, 16.0)
        };
        self.viewport.time_scale.set_bar_spacing(target_bs);
        let mid_bar = (entry_idx + exit_idx) as f64 * 0.5;
        let last_idx = (self.candles.len() - 1) as f64;
        let right_offset = mid_bar + 0.5 * (plot_w / target_bs) - last_idx;
        self.viewport.time_scale.set_right_offset(right_offset);
        if self.viewport.main_price_scale.is_auto_scale() {
            self.viewport.main_price_scale.reset_auto_scale();
        }
        self.viewport.equity_price_scale.reset_auto_scale();
        true
    }

    /// Launches a causal Bar Replay session `5` bars before `trade_id`'s entry bar so the trader can
    /// step bar-by-bar through signal confirmation, `Open[T+1]` fill, and SL/TP/trailing management.
    pub fn replay_backtest_trade(&mut self, trade_id: u64) -> bool {
        self.show_backtest_trades = true;
        self.sync_backtest_causal();
        let Some(entry_idx) = self
            .backtest_result
            .as_ref()
            .and_then(|res| res.trades.iter().find(|t| t.trade_id == trade_id))
            .map(|t| t.entry_bar_index)
        else {
            return false;
        };
        if self.base_candles.is_empty() || self.candles.is_empty() {
            return false;
        }
        if self.replay.is_active() {
            self.exit_replay();
        }
        let target_active_idx = entry_idx.saturating_sub(5).min(self.candles.len() - 1);
        let Some(cut_base_idx) = ReplayController::resolve_cut_base_index(
            &self.base_candles,
            &self.candles,
            self.active_timeframe,
            target_active_idx,
            None,
        ) else {
            return false;
        };
        self.start_replay_at_base_index(cut_base_idx)
    }

    // =========================================================================
    // Milestone R11 — Volume Profile (VRVP/FRVP), Session/Anchored VWAP & Causal RS
    // =========================================================================

    /// Resolves the active comparison/RS benchmark symbol (defaults to `VNINDEX` for Vietnam symbols,
    /// `FPT` when viewing `VNINDEX`, `SPY` for US/Crypto).
    #[must_use]
    pub fn effective_benchmark_symbol(&self) -> &str {
        if let Some(ref b) = self.compare_benchmark {
            let trimmed = b.trim();
            if !trimmed.is_empty() && !trimmed.eq_ignore_ascii_case(&self.symbol) {
                return trimmed;
            }
        }
        let upper = self.symbol.trim().to_ascii_uppercase();
        if self.active_exchange_venue().utc_offset_seconds() == 7 * 3_600 {
            if upper == "VNINDEX" { "FPT" } else { "VNINDEX" }
        } else if upper == "SPY" {
            "VNINDEX"
        } else {
            "SPY"
        }
    }

    /// Updates the comparison/RS benchmark override and refreshes benchmark caches cleanly.
    pub fn set_compare_benchmark(&mut self, symbol: Option<String>) {
        self.compare_benchmark = symbol
            .map(|s| s.trim().to_ascii_uppercase())
            .filter(|s| !s.is_empty());
        self.resolved_benchmark_cache = None;
        self.compare_cache_key = None;
        self.sync_indicators_and_benchmarks();
    }

    /// Cycles the active benchmark across bundled Vietnamese & global reference indices (`VNINDEX`,
    /// `HNX-INDEX`, `FPT`, `VCB`, `HPG`, `SPY`, `BTCUSDT`), skipping the currently displayed symbol.
    pub fn cycle_compare_benchmark(&mut self) {
        const BENCHMARKS: [&str; 7] = [
            "VNINDEX",
            "HNX-INDEX",
            "FPT",
            "VCB",
            "HPG",
            "SPY",
            "BTCUSDT",
        ];
        let current = self.effective_benchmark_symbol().to_ascii_uppercase();
        let start_idx = BENCHMARKS.iter().position(|b| *b == current).unwrap_or(0);
        for offset in 1..=BENCHMARKS.len() {
            let cand = BENCHMARKS[(start_idx + offset) % BENCHMARKS.len()];
            if !cand.eq_ignore_ascii_case(&self.symbol) {
                self.set_compare_benchmark(Some(cand.to_owned()));
                return;
            }
        }
    }

    /// Synchronizes session-aware indicators (`Session VWAP` UTC+7 / US / UTC) and, when `RS` or
    /// `Compare` is active, lazily resolves the benchmark feed and recomputes causal `RS` + rebased curve.
    pub fn sync_indicators_and_benchmarks(&mut self) {
        let tz_offset = self.active_exchange_venue().utc_offset_seconds();
        self.indicator_cache.sync_with_session_context(
            &self.candles,
            self.dataset_generation,
            &self.indicators,
            self.active_timeframe,
            tz_offset,
        );

        let need_bench = self.indicators.show_rs || self.show_compare_overlay;
        if !need_bench || self.candles.is_empty() {
            self.indicator_cache.clear_relative_strength();
            self.compare_cache_key = None;
            if !self.compare_series_cached.is_empty() {
                self.compare_series_cached.clear();
                self.compare_series_idx.clear();
            }
            return;
        }

        let target_bench = self.effective_benchmark_symbol().to_ascii_uppercase();
        let ref_first_ts = self
            .base_candles
            .first()
            .or_else(|| self.candles.first())
            .map_or(0, |c| c.timestamp);
        let ref_last_ts = self
            .base_candles
            .last()
            .or_else(|| self.candles.last())
            .map_or(0, |c| c.timestamp);

        let cache_hit =
            self.resolved_benchmark_cache
                .as_ref()
                .is_some_and(|(sym_k, f_ts, l_ts, _, _, _)| {
                    sym_k == &target_bench && *f_ts == ref_first_ts && *l_ts == ref_last_ts
                });
        if !cache_hit {
            self.ensure_watchlist_universe_loaded();
            let resolved = if !self.custom_peer_feeds.is_empty() {
                let mut combined = Vec::with_capacity(
                    self.custom_peer_feeds.len() + self.watchlist_universe.len(),
                );
                combined.extend(self.custom_peer_feeds.iter().cloned());
                combined.extend(self.watchlist_universe.iter().cloned());
                resolve_benchmark_series_for_window(
                    &target_bench,
                    ref_first_ts,
                    ref_last_ts,
                    &combined,
                )
            } else {
                resolve_benchmark_series_for_window(
                    &target_bench,
                    ref_first_ts,
                    ref_last_ts,
                    &self.watchlist_universe,
                )
            };
            if let Some((resolved_sym, bench_tf, bench_candles)) = resolved {
                self.benchmark_gen_counter = self.benchmark_gen_counter.wrapping_add(1).max(1);
                self.active_benchmark_label = resolved_sym;
                self.resolved_benchmark_cache = Some((
                    target_bench.clone(),
                    ref_first_ts,
                    ref_last_ts,
                    self.benchmark_gen_counter,
                    bench_tf,
                    bench_candles,
                ));
            } else {
                self.resolved_benchmark_cache = None;
                self.active_benchmark_label = target_bench;
            }
        }

        let causal_close_ts = self.replay_causal_close_timestamp();
        if let Some((_, _, _, bench_gen, bench_tf, ref bench_candles)) =
            self.resolved_benchmark_cache
        {
            self.indicator_cache.sync_relative_strength(
                &self.candles,
                self.dataset_generation,
                self.active_timeframe,
                &self.active_benchmark_label,
                bench_gen,
                bench_candles,
                bench_tf,
                self.indicators.rs_sma_period,
                causal_close_ts,
            );
        } else {
            self.indicator_cache.clear_relative_strength();
        }

        if self.show_compare_overlay {
            let total = self.candles.len();
            let cmp_key = (
                self.dataset_generation,
                self.active_timeframe,
                self.active_benchmark_label.clone(),
                causal_close_ts,
            );
            if self.compare_cache_key.as_ref() != Some(&cmp_key) {
                self.compare_series_cached = compute_rebased_comparison_series(
                    &self.candles,
                    &self.indicator_cache.rs.aligned_benchmark_close,
                    0..total,
                );
                self.compare_series_idx = SeriesBlockIndex::build(&self.compare_series_cached);
                self.compare_cache_key = Some(cmp_key);
            }
        } else {
            self.compare_cache_key = None;
            if !self.compare_series_cached.is_empty() {
                self.compare_series_cached.clear();
                self.compare_series_idx.clear();
            }
        }
    }

    /// Programmatically inserts a Fixed Range Volume Profile (`FRVP`) drawing across `[start_bar..=end_bar]`.
    pub fn add_frvp_drawing(
        &mut self,
        start_bar: usize,
        end_bar: usize,
        row_count: u16,
    ) -> Option<crate::drawings::DrawingId> {
        if self.candles.is_empty() {
            return None;
        }
        let max_idx = self.candles.len() - 1;
        let i0 = start_bar.min(max_idx);
        let i1 = end_bar.min(max_idx).max(i0);
        let slice = &self.candles[i0..=i1];
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for c in slice {
            lo = lo.min(c.low);
            hi = hi.max(c.high);
        }
        if !lo.is_finite() || !hi.is_finite() {
            lo = self.candles[i0].low;
            hi = self.candles[i0].high;
        }
        let geom = DrawingGeometry::FixedRangeVolumeProfile {
            start: DrawingAnchor::with_timestamp(i0 as f64, lo, self.candles[i0].timestamp as f64),
            end: DrawingAnchor::with_timestamp(i1 as f64, hi, self.candles[i1].timestamp as f64),
            row_count: row_count.clamp(8, 200),
        };
        self.drawings
            .add_drawing(DrawingPaneTarget::Main, geom, None)
    }

    /// Programmatically inserts an Anchored VWAP (`AVWAP`) drawing pinned at `anchor_bar`.
    pub fn add_anchored_vwap_drawing(
        &mut self,
        anchor_bar: usize,
        show_bands: bool,
    ) -> Option<crate::drawings::DrawingId> {
        if self.candles.is_empty() {
            return None;
        }
        let idx = anchor_bar.min(self.candles.len() - 1);
        let price = self.candles[idx].low;
        let geom = DrawingGeometry::AnchoredVwap {
            anchor: DrawingAnchor::with_timestamp(
                idx as f64,
                price,
                self.candles[idx].timestamp as f64,
            ),
            show_bands,
        };
        self.drawings
            .add_drawing(DrawingPaneTarget::Main, geom, None)
    }

    // =========================================================================
    // Milestone R10 — Vietnam Risk Planner, R:R Box Sync, Journal & WFO/Portfolio
    // =========================================================================

    /// Resolves the active [`ExchangeVenue`] (`risk_venue_override` or auto-inferred from `symbol` + `data_source`).
    #[inline]
    #[must_use]
    pub fn active_exchange_venue(&self) -> ExchangeVenue {
        self.risk_venue_override.unwrap_or_else(|| {
            ExchangeVenue::infer_from_symbol_and_preset(&self.symbol, self.data_source)
        })
    }

    /// Computes the strictly causal reference price (`<=` current replay/live frontier) for daily price bands (`DEF-001` & `M-09`).
    ///
    /// Evaluates against `base_candles[..=cursor]` using the active venue's exchange-local civil day (`UTC+7` ICT on VN venues,
    /// `UTC-5` EST on US Equities) so viewing `1W` on `FPT` (`base = D1`) still uses yesterday's `D1` close rather than last week's close.
    #[must_use]
    pub fn causal_reference_price(&self) -> Option<f64> {
        if self.base_candles.is_empty() {
            let last = self.candles.last()?;
            return (last.open.is_finite() && last.open > 0.0).then_some(last.open);
        }
        let cursor = self
            .current_trading_cursor_base_index()
            .min(self.base_candles.len() - 1);
        let base_slice = &self.base_candles[..=cursor];
        let last = base_slice.last()?;
        if base_slice.len() < 2 {
            return (last.open.is_finite() && last.open > 0.0).then_some(last.open);
        }
        if self.base_timeframe.seconds() >= 86_400 {
            let prev = &base_slice[base_slice.len() - 2];
            return (prev.close.is_finite() && prev.close > 0.0).then_some(prev.close);
        }
        let tz_offset = self.active_exchange_venue().utc_offset_seconds();
        let cur_local_day = last.timestamp.saturating_add(tz_offset).div_euclid(86_400);
        if let Some(prior_day_bar) = base_slice.iter().rev().find(|c| {
            c.timestamp.saturating_add(tz_offset).div_euclid(86_400) < cur_local_day
                && c.close.is_finite()
                && c.close > 0.0
        }) {
            Some(prior_day_bar.close)
        } else {
            let first = &base_slice[0];
            (first.open.is_finite() && first.open > 0.0).then_some(first.open)
        }
    }

    /// Computes today's statutory [`DailyPriceBand`] (`HOSE ±7%`, `HNX ±10%`, `UPCOM ±15%`, `VN30F ±7%`) at the causal frontier.
    #[must_use]
    pub fn active_daily_price_band(&self) -> Option<DailyPriceBand> {
        let venue = self.active_exchange_venue();
        let ref_px = self.causal_reference_price()?;
        compute_daily_price_band(venue, ref_px, self.risk_plan_input.price_unit)
    }

    /// Re-seeds or refreshes [`SumiWorkstationApp::risk_plan_input`] for the active instrument without lookahead.
    pub fn reseed_risk_planner_for_active_instrument(&mut self, reset_levels: bool) {
        let venue = self.active_exchange_venue();
        self.risk_plan_input.venue = venue;
        self.risk_plan_input.price_unit = VnPriceUnit::Auto;
        self.risk_plan_input.reference_price = self.causal_reference_price();
        self.risk_plan_input.entry_timestamp = self.candles.last().map(|c| c.timestamp);

        if reset_levels {
            self.risk_plan_input.commission_bps = venue.default_commission_bps();
            self.risk_plan_input.tax_sell_bps = venue.default_tax_sell_bps();
            self.risk_plan_input.contract_multiplier = 1.0;
            self.risk_plan_input.initial_margin_pct = venue.default_initial_margin_pct();
            self.risk_plan_input.slippage_bps = match venue {
                ExchangeVenue::Vn30Futures
                | ExchangeVenue::UsEquities
                | ExchangeVenue::VnIndexBenchmark => 3.0,
                ExchangeVenue::CryptoSpotPerp => 4.0,
                ExchangeVenue::Hose | ExchangeVenue::Hnx | ExchangeVenue::Upcom => 5.0,
            };
            if !venue.allows_short() {
                self.risk_plan_input.side = PositionSide::Long;
            }
            if self.sync_planner_from_selected_rr_box() {
                return;
            }
        }

        let mark = self.current_mark_price();
        if !mark.is_finite() || mark <= 0.0 {
            return;
        }
        let scale_mismatch = !self.risk_plan_input.entry_price.is_finite()
            || self.risk_plan_input.entry_price <= 0.0
            || self.risk_plan_input.entry_price > mark * 4.0
            || self.risk_plan_input.entry_price < mark * 0.25;

        if reset_levels || scale_mismatch {
            self.seed_brackets_around_mark(venue, mark);
        }
    }

    /// Explicit `"Reseed"` action (`M-08`): forces fresh brackets around `current_mark_price()` for the active venue
    /// AND updates any currently selected on-chart [`DrawingGeometry::RiskRewardBox`] so it never stays stale.
    pub fn reseed_risk_planner_from_live_mark(&mut self) {
        let venue = self.active_exchange_venue();
        self.risk_plan_input.venue = venue;
        self.risk_plan_input.price_unit = VnPriceUnit::Auto;
        self.risk_plan_input.reference_price = self.causal_reference_price();
        self.risk_plan_input.entry_timestamp = self.candles.last().map(|c| c.timestamp);
        self.risk_plan_input.commission_bps = venue.default_commission_bps();
        self.risk_plan_input.tax_sell_bps = venue.default_tax_sell_bps();
        self.risk_plan_input.contract_multiplier = 1.0;
        self.risk_plan_input.initial_margin_pct = venue.default_initial_margin_pct();
        if !venue.allows_short() {
            self.risk_plan_input.side = PositionSide::Long;
        }
        let mark = self.current_mark_price();
        if !mark.is_finite() || mark <= 0.0 {
            return;
        }
        self.seed_brackets_around_mark(venue, mark);
        if let Ok(plan) = self.compute_active_trade_plan()
            && self.drawings.update_selected_risk_reward_levels(
                plan.snapped_entry,
                plan.snapped_stop,
                plan.snapped_target,
            )
            && let Some(sel_id) = self.drawings.selected_id
        {
            self.synced_rr_drawing_signature = Some((
                sel_id,
                plan.snapped_entry.to_bits(),
                plan.snapped_stop.to_bits(),
                plan.snapped_target.to_bits(),
            ));
        }
        self.last_journal_status = Some((
            false,
            format!(
                "Reseeded {} @ {}",
                venue.short_label(),
                format_venue_price(mark, venue)
            ),
        ));
    }

    fn seed_brackets_around_mark(&mut self, venue: ExchangeVenue, mark: f64) {
        let risk_frac = match venue {
            ExchangeVenue::Vn30Futures => 0.010,
            ExchangeVenue::VnIndexBenchmark => 0.020,
            ExchangeVenue::Hose => 0.035,
            ExchangeVenue::Hnx | ExchangeVenue::Upcom => 0.045,
            ExchangeVenue::UsEquities => 0.025,
            ExchangeVenue::CryptoSpotPerp => 0.030,
        };
        let reward_frac = risk_frac * 2.4;
        self.risk_plan_input.entry_price = mark;
        match self.risk_plan_input.side {
            PositionSide::Long => {
                self.risk_plan_input.stop_loss = (mark * (1.0 - risk_frac)).max(mark * 0.2);
                self.risk_plan_input.target_price = mark * (1.0 + reward_frac);
            }
            PositionSide::Short => {
                self.risk_plan_input.stop_loss = mark * (1.0 + risk_frac);
                self.risk_plan_input.target_price = (mark * (1.0 - reward_frac)).max(mark * 0.2);
            }
        }
        if venue.is_vn_cash_equity()
            && mark >= 5_000.0
            && self.risk_plan_input.account_equity < 10_000_000.0
        {
            self.risk_plan_input.account_equity = 100_000_000.0;
            self.risk_plan_input.available_cash = 100_000_000.0;
        } else if venue == ExchangeVenue::Vn30Futures
            && mark < 5_000.0
            && self.risk_plan_input.account_equity < 50_000.0
        {
            self.risk_plan_input.account_equity = 150_000.0;
            self.risk_plan_input.available_cash = 150_000.0;
        }
    }

    /// Builds the causally anchored [`TradePlanInput`] for the current bar/replay frontier.
    #[must_use]
    pub fn effective_trade_plan_input(&self) -> TradePlanInput {
        let mut inp = self.risk_plan_input.clone();
        inp.venue = self.active_exchange_venue();
        inp.reference_price = self.causal_reference_price();
        inp.entry_timestamp = self.candles.last().map(|c| c.timestamp);
        if !inp.available_cash.is_finite() || inp.available_cash <= 0.0 {
            inp.available_cash = inp.account_equity;
        }
        inp
    }

    /// Evaluates [`compute_trade_plan`] against [`SumiWorkstationApp::effective_trade_plan_input`].
    pub fn compute_active_trade_plan(
        &self,
    ) -> Result<TradePlanComputed, crate::journal::TradePlanError> {
        compute_trade_plan(&self.effective_trade_plan_input())
    }

    /// Pulls `side`, `entry_price`, `stop_loss`, and `target_price` from the selected on-chart [`DrawingGeometry::RiskRewardBox`].
    pub fn sync_planner_from_selected_rr_box(&mut self) -> bool {
        let Some((id, entry_px, stop_px, target_px)) =
            self.drawings
                .selected_drawing()
                .and_then(|d| match d.geometry {
                    DrawingGeometry::RiskRewardBox {
                        start,
                        end,
                        stop_price,
                    } => Some((d.id, start.value, stop_price, end.value)),
                    _ => None,
                })
        else {
            return false;
        };
        if !entry_px.is_finite()
            || !stop_px.is_finite()
            || !target_px.is_finite()
            || entry_px <= 0.0
            || stop_px <= 0.0
            || target_px <= 0.0
        {
            return false;
        }
        self.risk_plan_input.side = if target_px >= entry_px {
            PositionSide::Long
        } else {
            PositionSide::Short
        };
        self.risk_plan_input.entry_price = entry_px;
        self.risk_plan_input.stop_loss = stop_px;
        self.risk_plan_input.target_price = target_px;
        self.synced_rr_drawing_signature = Some((
            id,
            entry_px.to_bits(),
            stop_px.to_bits(),
            target_px.to_bits(),
        ));
        true
    }

    fn auto_sync_planner_if_rr_box_changed(&mut self) {
        let Some((id, entry_px, stop_px, target_px)) =
            self.drawings
                .selected_drawing()
                .and_then(|d| match d.geometry {
                    DrawingGeometry::RiskRewardBox {
                        start,
                        end,
                        stop_price,
                    } => Some((d.id, start.value, stop_price, end.value)),
                    _ => None,
                })
        else {
            self.synced_rr_drawing_signature = None;
            return;
        };
        let sig = (
            id,
            entry_px.to_bits(),
            stop_px.to_bits(),
            target_px.to_bits(),
        );
        if self.synced_rr_drawing_signature != Some(sig) {
            let _ = self.sync_planner_from_selected_rr_box();
        }
    }

    /// Snaps the Risk Planner's `Entry / Stop / Target` to valid exchange ticks and updates any selected `RiskRewardBox`.
    pub fn snap_planner_ticks_and_sync_rr_box(&mut self) -> bool {
        let mut inp = self.effective_trade_plan_input();
        inp.snap_to_ticks = true;
        let Ok(plan) = compute_trade_plan(&inp) else {
            self.last_journal_status = Some((
                true,
                "invalid Entry / Stop / Target brackets for active side".to_owned(),
            ));
            return false;
        };
        self.risk_plan_input.entry_price = plan.snapped_entry;
        self.risk_plan_input.stop_loss = plan.snapped_stop;
        self.risk_plan_input.target_price = plan.snapped_target;
        if self.drawings.update_selected_risk_reward_levels(
            plan.snapped_entry,
            plan.snapped_stop,
            plan.snapped_target,
        ) && let Some(sel) = self.drawings.selected_drawing()
        {
            self.synced_rr_drawing_signature = Some((
                sel.id,
                plan.snapped_entry.to_bits(),
                plan.snapped_stop.to_bits(),
                plan.snapped_target.to_bits(),
            ));
        }
        self.last_journal_status = Some((
            false,
            format!(
                "Snapped {} ticks: E {:.2} · SL {:.2} · TP {:.2}",
                plan.venue.short_label(),
                plan.snapped_entry,
                plan.snapped_stop,
                plan.snapped_target
            ),
        ));
        true
    }

    /// Draws a new on-chart [`DrawingGeometry::RiskRewardBox`] (or updates the selected one) from the active Risk Planner.
    pub fn draw_or_update_rr_box_from_planner(&mut self) -> Option<u64> {
        let Ok(plan) = self.compute_active_trade_plan() else {
            self.last_journal_status = Some((
                true,
                "cannot draw R:R Box: fix invalid Entry / Stop / Target levels".to_owned(),
            ));
            return None;
        };
        if self.drawings.update_selected_risk_reward_levels(
            plan.snapped_entry,
            plan.snapped_stop,
            plan.snapped_target,
        ) && let Some(sel_id) = self.drawings.selected_id
        {
            self.synced_rr_drawing_signature = Some((
                sel_id,
                plan.snapped_entry.to_bits(),
                plan.snapped_stop.to_bits(),
                plan.snapped_target.to_bits(),
            ));
            self.last_journal_status = Some((
                false,
                format!(
                    "Updated R:R Box #{sel_id} ({:.2}R net)",
                    plan.net_r_multiple
                ),
            ));
            return Some(sel_id);
        }

        if self.candles.is_empty() {
            return None;
        }
        let vis_range = self.viewport.time_scale.strict_visible_data_range();
        let end_idx = if vis_range.is_empty() {
            self.candles.len() - 1
        } else {
            vis_range.end.saturating_sub(1).min(self.candles.len() - 1)
        };
        let vis_len = if vis_range.is_empty() {
            40
        } else {
            vis_range.len().max(8)
        };
        let box_span = (vis_len / 4).clamp(5, 28);
        let start_idx = end_idx.saturating_sub(box_span);
        let start_anchor = match resolve_anchor_timestamp(
            start_idx as f64,
            &self.candles,
            self.active_timeframe,
        ) {
            Some(ts) => DrawingAnchor::with_timestamp(start_idx as f64, plan.snapped_entry, ts),
            None => DrawingAnchor::new(start_idx as f64, plan.snapped_entry),
        };
        let end_anchor =
            match resolve_anchor_timestamp(end_idx as f64, &self.candles, self.active_timeframe) {
                Some(ts) => DrawingAnchor::with_timestamp(end_idx as f64, plan.snapped_target, ts),
                None => DrawingAnchor::new(end_idx as f64, plan.snapped_target),
            };

        let geom = DrawingGeometry::RiskRewardBox {
            start: start_anchor,
            end: end_anchor,
            stop_price: plan.snapped_stop,
        };
        let color = if plan.side == PositionSide::Long {
            DrawingColorPreset::Emerald
        } else {
            DrawingColorPreset::Crimson
        };
        let style = DrawingStyle {
            color,
            line_width: 1.8,
            dash: LineDashStyle::Solid,
            fill_opacity: 0.16,
        };
        let id = self
            .drawings
            .add_drawing(DrawingPaneTarget::Main, geom, Some(style))?;
        self.drawings.selected_id = Some(id);
        self.drawings.active_tool = DrawingTool::Select;
        self.synced_rr_drawing_signature = Some((
            id,
            plan.snapped_entry.to_bits(),
            plan.snapped_stop.to_bits(),
            plan.snapped_target.to_bits(),
        ));
        self.last_journal_status = Some((
            false,
            format!(
                "Created R:R Box #{id} ({} {:.2}R net)",
                plan.side.short_label(),
                plan.net_r_multiple
            ),
        ));
        Some(id)
    }

    /// Populates the Paper Trading Order Ticket (`qty`, `entry`, `SL`, `TP`, `Limit/Stop`) from the active Risk Planner.
    ///
    /// Rejects unaffordable or `0`-lot plans (`C-04`) instead of silently substituting `board_lot()`.
    pub fn sync_ticket_from_risk_planner(&mut self) -> bool {
        let plan = match self.compute_active_trade_plan() {
            Ok(p) => p,
            Err(err) => {
                self.last_journal_status = Some((true, err.message().to_owned()));
                return false;
            }
        };
        if !plan.affordable || plan.planned_quantity <= 0.0 {
            let reason = plan.warnings.first().map_or_else(
                || "Risk Plan sizes to 0 lots under current risk/margin budget".to_owned(),
                |w| w.description(),
            );
            let msg = format!("Cannot send 0-lot plan to Ticket: {reason}");
            self.last_journal_status = Some((true, msg.clone()));
            self.last_trade_status = Some((true, msg));
            return false;
        }
        let mark = self.current_mark_price();
        self.ticket_kind = match plan.side {
            PositionSide::Long if mark > 0.0 && plan.snapped_entry > mark => TicketOrderKind::Stop,
            PositionSide::Short if mark > 0.0 && plan.snapped_entry < mark => TicketOrderKind::Stop,
            _ => TicketOrderKind::Limit,
        };
        self.ticket_qty_input = format_clean_qty(plan.planned_quantity);
        self.ticket_price_input = format_venue_price(plan.snapped_entry, plan.venue);
        self.ticket_sl_input = format_venue_price(plan.snapped_stop, plan.venue);
        self.ticket_tp_input = format_venue_price(plan.snapped_target, plan.venue);
        self.show_trading_panel = true;
        self.right_dock_tab = RightDockTab::Trading;
        let msg = format!(
            "Planner -> Ticket: {} {} @ {} (SL {} / TP {})",
            plan.side.short_label(),
            self.ticket_qty_input,
            self.ticket_price_input,
            self.ticket_sl_input,
            self.ticket_tp_input
        );
        self.last_trade_status = Some((false, msg.clone()));
        self.last_journal_status = Some((false, msg));
        true
    }

    /// Logs the currently computed Risk Plan into [`SumiWorkstationApp::journal`] at the causal frontier timestamp.
    pub fn log_active_plan_to_journal(&mut self) -> Option<u64> {
        let plan = match self.compute_active_trade_plan() {
            Ok(p) => p,
            Err(err) => {
                self.last_journal_status = Some((true, err.message().to_owned()));
                return None;
            }
        };
        let causal_ts = self
            .replay_causal_close_timestamp()
            .or_else(|| self.candles.last().map(|c| c.timestamp))
            .unwrap_or(0);
        let id = self.journal.record_trade_plan(
            &self.symbol,
            &plan,
            causal_ts,
            self.journal_draft_setup,
            self.journal_draft_conviction,
            self.journal_draft_checklist,
            &self.journal_draft_note,
        );
        self.last_journal_status = Some((
            false,
            format!(
                "Logged Plan #{id} ({} {} · {:.2}R net)",
                self.symbol,
                plan.side.short_label(),
                plan.net_r_multiple
            ),
        ));
        Some(id)
    }

    /// Idempotently synchronizes closed Paper Trading trades for the active symbol into the Journal.
    pub fn sync_paper_trades_to_journal(&mut self) -> usize {
        let venue = self.active_exchange_venue();
        let added = self.journal.sync_paper_closed_trades(
            &self.symbol,
            venue,
            self.trading.closed_trades(),
        );
        self.last_journal_status = Some((
            false,
            format!(
                "Synced Paper Trades ({added} new, {} total closed)",
                self.trading.closed_trades().len()
            ),
        ));
        added
    }

    /// Idempotently imports up to `max_import` causal Strategy Backtest trades into the Journal.
    pub fn import_backtest_trades_to_journal(&mut self, max_import: usize) -> usize {
        self.backtest_config.sanitize();
        self.sync_backtest_causal();
        let on_demand_bt;
        let bt = if let Some(ref cached) = self.backtest_result {
            cached
        } else {
            let opts = BacktestRunOptions {
                replay_cutoff_timestamp: self.replay_causal_close_timestamp(),
                incomplete_tail_bar: self.is_active_tail_incomplete(),
            };
            on_demand_bt = crate::backtest::run_backtest_with_options(
                &self.symbol,
                self.active_timeframe,
                &self.candles,
                &self.backtest_config,
                opts,
            );
            &on_demand_bt
        };
        let venue = self.active_exchange_venue();
        let setup_hint = match bt.config.preset {
            StrategyPresetKind::SmaEmaTrendCross => SetupTag::TrendPullback,
            StrategyPresetKind::RsiMeanReversion => SetupTag::MeanReversion,
            StrategyPresetKind::MacdMomentum => SetupTag::MomentumContinuation,
            StrategyPresetKind::BollingerSqueezeBreakout
            | StrategyPresetKind::DonchianAtrBreakout => SetupTag::SqueezeBreakout,
            StrategyPresetKind::RsiBollingerConfluence => SetupTag::SupportBounce,
        };
        let added = self.journal.import_backtest_trades(
            &self.symbol,
            venue,
            bt.config.preset.short_label(),
            setup_hint,
            &bt.trades,
            max_import,
        );
        self.last_journal_status = Some((
            false,
            format!("Imported {added} causal backtest trades into Journal"),
        ));
        added
    }

    /// Generates and caches the causally gated RFC-4180 CSV export of [`SumiWorkstationApp::journal`].
    pub fn export_journal_csv_causal(&mut self) -> String {
        let cutoff = self.replay_causal_close_timestamp();
        let csv = self.journal.export_csv(cutoff);
        let rows = self.journal.visible_entries(cutoff).len();
        self.last_exported_journal_doc = Some(csv.clone());
        self.last_journal_status = Some((
            false,
            format!("Copied {rows} causal journal entries as CSV"),
        ));
        csv
    }

    /// Generates and caches the causally gated Markdown debrief report of [`SumiWorkstationApp::journal`].
    pub fn export_journal_markdown_causal(&mut self) -> String {
        let cutoff = self.replay_causal_close_timestamp();
        let md = self.journal.export_markdown(cutoff);
        let rows = self.journal.visible_entries(cutoff).len();
        self.last_exported_journal_doc = Some(md.clone());
        self.last_journal_status = Some((
            false,
            format!("Copied {rows} causal journal entries as Markdown"),
        ));
        md
    }

    /// Opens the right-dock `Risk Planner & Journal` tab and syncs any selected `RiskRewardBox`.
    pub fn open_journal_tab(&mut self) {
        self.show_journal_panel = true;
        self.right_dock_tab = RightDockTab::Journal;
        let _ = self.sync_planner_from_selected_rr_box();
    }

    /// Runs `K`-fold Rolling or Anchored Walk-Forward Optimization causally on the active series (`Milestone R10`).
    pub fn run_active_walk_forward(&mut self) {
        self.backtest_config.sanitize();
        self.wf_config.sanitize();
        let replay_close_cutoff = self.replay_causal_close_timestamp();
        let incomplete_tail = self.is_active_tail_incomplete();
        let cfg_digest = compute_backtest_config_digest(&self.backtest_config);
        let wf_digest = compute_walk_forward_config_digest(&self.wf_config);
        let last_ts = self.candles.last().map(|c| c.timestamp);
        let opts = BacktestRunOptions {
            replay_cutoff_timestamp: replay_close_cutoff,
            incomplete_tail_bar: incomplete_tail,
        };
        let rep = run_walk_forward_validation(
            &self.symbol,
            self.active_timeframe,
            &self.candles,
            &self.backtest_config,
            self.wf_config,
            opts,
        );
        self.walk_forward_report = Some(rep);
        self.wf_synced_key = Some((
            self.dataset_generation,
            self.candles.len(),
            last_ts,
            replay_close_cutoff,
            incomplete_tail,
            cfg_digest,
            wf_digest,
        ));
        if self.equity_pane_source == EquityPaneSource::WalkForwardOos {
            self.viewport.equity_price_scale.reset_auto_scale();
        }
    }

    /// Runs causal Multi-Asset Portfolio Simulation across `VN30 (8)` or `All 13` feeds (`Milestone R10`).
    pub fn run_active_portfolio_backtest(&mut self) {
        self.ensure_watchlist_universe_loaded();
        self.backtest_config.sanitize();
        let replay_close_cutoff = self.replay_causal_close_timestamp();
        let cfg_digest = compute_backtest_config_digest(&self.backtest_config);

        let feeds: Vec<WatchlistFeedEntry> = if self.data_source == DataSourcePreset::CustomFile
            && !self.custom_peer_feeds.is_empty()
        {
            self.custom_peer_feeds.clone()
        } else {
            let mut list = self.watchlist_universe.clone();
            let active_key = self.current_instrument_key();
            if !list.iter().any(|e| e.instrument_id == active_key) && !self.base_candles.is_empty()
            {
                list.push(WatchlistFeedEntry {
                    instrument_id: active_key,
                    preset: self.data_source,
                    symbol: self.symbol.clone(),
                    base_timeframe: self.base_timeframe,
                    candles: self.base_candles.clone(),
                });
            }
            list
        };

        let effective_filter = if self.data_source == DataSourcePreset::CustomFile
            && !self.custom_peer_feeds.is_empty()
        {
            PortfolioUniverseFilter::AllUniverseFeeds
        } else {
            self.portfolio_universe_filter
        };

        let rep = run_portfolio_backtest(
            &feeds,
            self.active_timeframe,
            &self.backtest_config,
            effective_filter,
            self.portfolio_weighting,
            replay_close_cutoff,
        );
        self.portfolio_report = Some(rep);
        self.portfolio_synced_key = Some((
            self.current_instrument_key(),
            self.dataset_generation,
            self.active_timeframe,
            replay_close_cutoff,
            cfg_digest,
            self.portfolio_universe_filter,
            self.portfolio_weighting,
        ));
        self.portfolio_mapped_curve = None;
        self.refresh_portfolio_mapped_curve();
        if self.equity_pane_source == EquityPaneSource::MultiAssetPortfolio {
            self.viewport.equity_price_scale.reset_auto_scale();
        }
    }

    fn refresh_portfolio_mapped_curve(&mut self) {
        let Some(ref port) = self.portfolio_report else {
            self.portfolio_mapped_curve = None;
            return;
        };
        let last_ts = self.candles.last().map(|c| c.timestamp);
        let replay_close_cutoff = self.replay_causal_close_timestamp();
        if let Some(ref cached) = self.portfolio_mapped_curve
            && cached.dataset_generation == self.dataset_generation
            && cached.timeframe == self.active_timeframe
            && cached.replay_close_cutoff == replay_close_cutoff
            && cached.candle_len == self.candles.len()
            && cached.last_timestamp == last_ts
        {
            return;
        }
        let (eq_vals, eq_idx, uw_vals, _) = map_equity_curve_to_candles(
            &port.portfolio_equity_curve,
            &self.candles,
            self.active_timeframe,
            replay_close_cutoff,
        );
        let title = format!(
            "PORT {} · {}",
            port.universe_filter.short_label(),
            port.weighting.short_label()
        );
        self.portfolio_mapped_curve = Some(MappedPortfolioCurveCache {
            dataset_generation: self.dataset_generation,
            timeframe: self.active_timeframe,
            replay_close_cutoff,
            candle_len: self.candles.len(),
            last_timestamp: last_ts,
            title_label: title,
            baseline_cash: port.initial_capital,
            equity_values: eq_vals,
            equity_idx: eq_idx,
            underwater_values: uw_vals,
        });
    }

    /// Arms a horizontal [`AlertCondition::PriceLevel`] alert at the current causal frontier.
    pub fn add_quick_price_alert(
        &mut self,
        price: f64,
        direction: CrossDirection,
    ) -> Result<AlertId, AlertError> {
        let cursor_idx = self.current_trading_cursor_base_index();
        let cutoff = self.replay.cutoff_timestamp(&self.base_candles);
        let res = self.alerts.add_alert(
            &self.symbol,
            self.active_timeframe,
            AlertCondition::PriceLevel { price, direction },
            self.alert_frequency,
            None,
            &self.base_candles,
            cursor_idx,
            cutoff,
            Some(&self.drawings),
        );
        match res {
            Ok(id) => {
                self.alerts_synced_key = None;
                self.sync_alerts_causal();
                self.show_alerts_panel = true;
                self.right_dock_tab = RightDockTab::Alerts;
                self.last_alert_status = Some((
                    false,
                    format!(
                        "Armed #{} {} {} {:.2}",
                        id,
                        self.symbol,
                        direction.short_label(),
                        price
                    ),
                ));
                Ok(id)
            }
            Err(err) => {
                self.last_alert_status = Some((true, err.to_string()));
                Err(err)
            }
        }
    }

    /// Arms a [`AlertCondition::DrawingLine`] alert on the currently selected Main-pane drawing
    /// (`HorizontalLine`, finite `TrendLine`, or $360^\circ$ `Ray`).
    pub fn add_alert_on_selected_drawing(
        &mut self,
        direction: CrossDirection,
    ) -> Result<AlertId, AlertError> {
        let Some(drawing_id) = self.drawings.selected_id else {
            let err = AlertError::DrawingNotFound(0);
            self.last_alert_status = Some((
                true,
                "select a Main-pane HorizontalLine, TrendLine, or Ray first".to_owned(),
            ));
            return Err(err);
        };
        let cursor_idx = self.current_trading_cursor_base_index();
        let cutoff = self.replay.cutoff_timestamp(&self.base_candles);
        let res = self.alerts.add_alert(
            &self.symbol,
            self.active_timeframe,
            AlertCondition::DrawingLine {
                drawing_id,
                direction,
            },
            self.alert_frequency,
            None,
            &self.base_candles,
            cursor_idx,
            cutoff,
            Some(&self.drawings),
        );
        match res {
            Ok(id) => {
                self.alerts_synced_key = None;
                self.sync_alerts_causal();
                self.show_alerts_panel = true;
                self.right_dock_tab = RightDockTab::Alerts;
                self.last_alert_status = Some((
                    false,
                    format!(
                        "Armed #{} on Drawing #{} ({})",
                        id,
                        drawing_id,
                        direction.short_label()
                    ),
                ));
                Ok(id)
            }
            Err(err) => {
                self.last_alert_status = Some((true, err.to_string()));
                Err(err)
            }
        }
    }

    /// Submits the draft alert rule configured in the right-side Alerts Manager panel.
    pub fn submit_draft_alert(&mut self) -> Result<AlertId, AlertError> {
        let condition = match self.alert_draft_kind {
            AlertDraftKind::PriceLevel => {
                let cleaned = self.alert_value_input.trim().replace(',', "");
                let price = cleaned
                    .parse::<f64>()
                    .map_err(|_| AlertError::InvalidPriceLevel)
                    .inspect_err(|err| {
                        self.last_alert_status = Some((true, err.to_string()));
                    })?;
                AlertCondition::PriceLevel {
                    price,
                    direction: self.alert_direction,
                }
            }
            AlertDraftKind::RsiThreshold => {
                let cleaned = self.alert_value_input.trim().replace(',', "");
                let level = if cleaned.is_empty() {
                    30.0
                } else {
                    cleaned
                        .parse::<f64>()
                        .map_err(|_| AlertError::InvalidIndicatorParams)
                        .inspect_err(|err| {
                            self.last_alert_status = Some((true, err.to_string()));
                        })?
                };
                AlertCondition::RsiThreshold {
                    period: 14,
                    level,
                    direction: self.alert_direction,
                }
            }
            AlertDraftKind::MacdZeroCross => AlertCondition::MacdHistogramZeroCross {
                fast: 12,
                slow: 26,
                signal: 9,
                direction: self.alert_direction,
            },
            AlertDraftKind::SelectedDrawing => {
                let Some(drawing_id) = self.drawings.selected_id else {
                    let err = AlertError::DrawingNotFound(0);
                    self.last_alert_status = Some((
                        true,
                        "select a Main-pane HorizontalLine, TrendLine, or Ray".to_owned(),
                    ));
                    return Err(err);
                };
                AlertCondition::DrawingLine {
                    drawing_id,
                    direction: self.alert_direction,
                }
            }
        };

        let cursor_idx = self.current_trading_cursor_base_index();
        let cutoff = self.replay.cutoff_timestamp(&self.base_candles);
        let custom_msg = (!self.alert_message_input.trim().is_empty())
            .then(|| self.alert_message_input.trim().to_owned());
        match self.alerts.add_alert(
            &self.symbol,
            self.active_timeframe,
            condition,
            self.alert_frequency,
            custom_msg,
            &self.base_candles,
            cursor_idx,
            cutoff,
            Some(&self.drawings),
        ) {
            Ok(id) => {
                self.alert_message_input.clear();
                self.alerts_synced_key = None;
                self.sync_alerts_causal();
                self.last_alert_status = Some((false, format!("Armed alert #{id}")));
                Ok(id)
            }
            Err(err) => {
                self.last_alert_status = Some((true, err.to_string()));
                Err(err)
            }
        }
    }

    /// Opens the keyboard-first `Cmd+K` / `Ctrl+K` Command Palette modal and requests input focus.
    pub fn open_command_palette(&mut self) {
        self.drawings.cancel_active_gesture();
        self.viewport.cancel_active_interactions();
        self.ensure_watchlist_universe_loaded();
        self.command_palette.open = true;
        self.command_palette.selected_index = 0;
        self.command_palette.request_focus = true;
    }

    /// Closes the `Cmd+K` Command Palette modal cleanly.
    pub fn close_command_palette(&mut self) {
        self.command_palette.open = false;
        self.command_palette.request_focus = false;
    }

    /// Toggles the `Cmd+K` Command Palette modal (`open <-> closed`).
    pub fn toggle_command_palette(&mut self) {
        if self.command_palette.open {
            self.close_command_palette();
        } else {
            self.open_command_palette();
        }
    }

    /// Builds and ranks all eligible `Cmd+K` Command Palette candidates for `self.command_palette.query`.
    pub fn build_command_palette_candidates(&mut self) -> Vec<PaletteCandidate> {
        self.sync_scanner_caches();
        let q = self.command_palette.query.trim().to_owned();
        let mut out = Vec::with_capacity(48);

        // 1. Dynamic `alert <price>` synthesis (`"alert 1295"`, `"alert > 1295"`, `"alert < 68500"`)
        let parsed_quick_alert = parse_quick_alert_query(&q, self.current_mark_price());
        if let Some((price, dir)) = parsed_quick_alert {
            out.push(PaletteCandidate {
                category: PaletteCategory::Alert,
                title: format!(
                    "Create Price Alert: {} {} {:.2}",
                    self.symbol,
                    dir.short_label(),
                    price
                ),
                subtitle: format!("Arm on {} ({})", self.symbol, self.timeframe),
                keywords: "alert price cross level".to_owned(),
                score: 10_000,
                action: PaletteAction::QuickPriceAlert {
                    price,
                    direction: dir,
                },
            });
        }

        let effective_match_q = if parsed_quick_alert.is_some() {
            "alert"
        } else {
            q.as_str()
        };

        let mut push_scored = |category: PaletteCategory,
                               title: String,
                               subtitle: String,
                               keywords: String,
                               action: PaletteAction| {
            if let Some(score) = score_palette_match(effective_match_q, &title, &keywords) {
                out.push(PaletteCandidate {
                    category,
                    title,
                    subtitle,
                    keywords,
                    score,
                    action,
                });
            }
        };

        // 2. Multi-Asset Watchlist Symbols (`13` bundled feeds + any multi-symbol custom feeds)
        let mut universe_snapshot: Vec<(String, DataSourcePreset, String, Timeframe)> = self
            .watchlist_universe
            .iter()
            .map(|e| {
                (
                    e.instrument_id.clone(),
                    e.preset,
                    e.symbol.clone(),
                    e.base_timeframe,
                )
            })
            .collect();
        if self.data_source == DataSourcePreset::CustomFile {
            for e in &self.custom_peer_feeds {
                universe_snapshot.push((
                    e.instrument_id.clone(),
                    e.preset,
                    e.symbol.clone(),
                    e.base_timeframe,
                ));
            }
        }
        for (inst_id, preset, sym, base_tf) in universe_snapshot {
            let row_info = self
                .raw_watchlist_rows_cache
                .iter()
                .find(|r| r.instrument_id == inst_id)
                .map_or_else(
                    || format!("{} · base {base_tf}", preset.short_label()),
                    |r| {
                        format!(
                            "{} · {:.2} ({:+.2}%) · {}",
                            preset.short_label(),
                            r.last_close,
                            r.change_pct,
                            r.regime.full_label()
                        )
                    },
                );
            push_scored(
                PaletteCategory::Symbol,
                format!("{sym} — Switch Symbol"),
                row_info,
                format!("symbol ticker switch {} {}", sym, preset.short_label()),
                PaletteAction::SelectWatchlistSymbol {
                    preset,
                    symbol: sym,
                },
            );
        }

        // 3. Multi-Timeframe Lattice (`1m..1W` allowed from `self.base_timeframe`)
        for tf in Timeframe::ALL {
            if self.base_timeframe.can_aggregate_to(tf) {
                let active_tag = if self.active_timeframe == tf {
                    "Current active timeframe"
                } else {
                    "Aggregate causal OHLCV & remap drawings"
                };
                push_scored(
                    PaletteCategory::Timeframe,
                    format!("Timeframe: {}", tf.label()),
                    active_tag.to_owned(),
                    format!("tf timeframe interval {}", tf.label()),
                    PaletteAction::SetTimeframe(tf),
                );
            }
        }

        // 4. Technical Indicators (`SMA`, `EMA`, `BB`, `VWAP`, `VRVP`, `RSI`, `MACD`, `ATR`, `RS`)
        for (key, name, on) in [
            ("SMA", "SMA(20) Overlay", self.indicators.show_sma),
            ("EMA", "EMA(50) Overlay", self.indicators.show_ema),
            (
                "BB",
                "Bollinger Bands(20, 2.0)",
                self.indicators.show_bbands,
            ),
            ("VWAP", "Session VWAP ±1σ/±2σ", self.indicators.show_vwap),
            (
                "VRVP",
                "Visible Range Volume Profile (POC/VAH/VAL)",
                self.indicators.show_vrvp,
            ),
            ("RSI", "RSI(14) Sub-Pane", self.indicators.show_rsi),
            (
                "MACD",
                "MACD(12, 26, 9) Sub-Pane",
                self.indicators.show_macd,
            ),
            ("ATR", "ATR(14) Sub-Pane", self.indicators.show_atr),
            (
                "RS",
                "Relative Strength vs Benchmark Sub-Pane",
                self.indicators.show_rs,
            ),
        ] {
            push_scored(
                PaletteCategory::Indicator,
                format!("Toggle Indicator: {name}"),
                if on { "Currently ON" } else { "Currently OFF" }.to_owned(),
                format!("indicator overlay oscillator {key} {name}"),
                PaletteAction::ToggleIndicator(key),
            );
        }
        let bench_label = self.effective_benchmark_symbol().to_owned();
        push_scored(
            PaletteCategory::Indicator,
            format!("Toggle Compare Overlay (vs {bench_label})"),
            if self.show_compare_overlay {
                "Currently ON · Rebased benchmark comparison"
            } else {
                "Currently OFF · Plot rebased benchmark curve"
            }
            .to_owned(),
            format!("compare overlay benchmark vnindex spy rs {bench_label}"),
            PaletteAction::ToggleCompareOverlay,
        );
        push_scored(
            PaletteCategory::Indicator,
            format!("Cycle Compare/RS Benchmark (active: {bench_label})"),
            "Switch between VNINDEX, HNX-INDEX, FPT, VCB, HPG, SPY, BTCUSDT".to_owned(),
            "cycle switch benchmark compare rs vnindex spy".to_owned(),
            PaletteAction::CycleCompareBenchmark,
        );

        // 5. Drawing Tools & Magnet
        for tool in [
            DrawingTool::Select,
            DrawingTool::HorizontalLine,
            DrawingTool::TrendLine,
            DrawingTool::Ray,
            DrawingTool::Rectangle,
            DrawingTool::FibonacciRetracement,
            DrawingTool::RiskRewardBox,
            DrawingTool::FixedRangeVolumeProfile,
            DrawingTool::AnchoredVwap,
        ] {
            push_scored(
                PaletteCategory::Drawing,
                format!("Drawing Tool: {}", tool.short_label()),
                "Activate chart annotation tool".to_owned(),
                format!("draw tool annotation {}", tool.short_label()),
                PaletteAction::SelectDrawingTool(tool),
            );
        }
        push_scored(
            PaletteCategory::Drawing,
            "Toggle OHLC Magnet Mode".to_owned(),
            format!("Current: {:?}", self.drawings.magnet_mode),
            "magnet snap ohlc drawing".to_owned(),
            PaletteAction::ToggleMagnet,
        );

        // 6. Workspace Docks, Alerts, Trading, Replay & Scale Modes
        push_scored(
            PaletteCategory::Workspace,
            "Toggle Watchlist & Technical Scanner Dock".to_owned(),
            if self.show_scanner_dock {
                "Hide left scanner dock".to_owned()
            } else {
                "Open left multi-symbol scanner & signals dock".to_owned()
            },
            "watchlist scanner signals screener dock left".to_owned(),
            PaletteAction::ToggleScannerDock,
        );
        let pal_cutoff = self.replay.cutoff_timestamp(&self.base_candles);
        let pal_vis_rules = self
            .alerts
            .rules
            .iter()
            .filter(|r| r.is_causally_visible(pal_cutoff))
            .count();
        let pal_vis_trigs = self
            .alerts
            .triggers
            .iter()
            .filter(|ev| pal_cutoff.is_none_or(|cut| alert_event_eval_timestamp(ev) <= cut))
            .count();
        push_scored(
            PaletteCategory::Alert,
            "Open Alerts Manager & Trigger Log".to_owned(),
            format!("{pal_vis_rules} rules · {pal_vis_trigs} causal triggers"),
            "alerts alarm trigger log right panel".to_owned(),
            PaletteAction::OpenAlertsTab,
        );
        if self.current_mark_price() > 0.0 {
            let mark = self.current_mark_price();
            let up_px = ((mark * 1.01) * 100.0).round() / 100.0;
            push_scored(
                PaletteCategory::Alert,
                format!("Quick Alert +1% Above Mark ({up_px:.2})"),
                format!("Arm Once alert on {} @ {up_px:.2}", self.symbol),
                "quick alert price above mark".to_owned(),
                PaletteAction::QuickPriceAlert {
                    price: up_px,
                    direction: CrossDirection::CrossAbove,
                },
            );
        }
        push_scored(
            PaletteCategory::Trading,
            "Open Paper Trading Ticket & Blotter".to_owned(),
            "Inspect equity, submit Market/Limit/Stop orders".to_owned(),
            "paper trade ticket order blotter position equity".to_owned(),
            PaletteAction::OpenTradingTab,
        );
        let j_vis = self
            .journal
            .visible_entries(self.replay_causal_close_timestamp())
            .len();
        push_scored(
            PaletteCategory::Trading,
            "Open Risk Planner & Structured Trade Journal".to_owned(),
            format!(
                "{} microstructure · {j_vis} causal journal entries",
                self.active_exchange_venue().short_label()
            ),
            "risk planner position sizing hose lot t+2 ceiling floor journal sqn mistake"
                .to_owned(),
            PaletteAction::OpenJournalTab,
        );
        push_scored(
            PaletteCategory::Drawing,
            "Draw On-Chart R:R Position Box from Risk Planner".to_owned(),
            format!(
                "Create or update {} Risk-Reward Box on Main pane",
                self.risk_plan_input.side.short_label()
            ),
            "draw risk reward box rr planner entry stop target".to_owned(),
            PaletteAction::DrawRiskRewardFromPlanner,
        );
        push_scored(
            PaletteCategory::Trading,
            "Journal: Sync Closed Paper Trades".to_owned(),
            format!(
                "Import {} closed paper trades idempotently",
                self.trading.closed_trades().len()
            ),
            "journal sync import paper trades closed".to_owned(),
            PaletteAction::SyncPaperTradesToJournal,
        );
        push_scored(
            PaletteCategory::Trading,
            "Journal: Copy Causal CSV Export".to_owned(),
            format!("Export {j_vis} causally visible entries (RFC-4180 formula-safe)"),
            "journal export copy csv spreadsheet".to_owned(),
            PaletteAction::ExportJournalCsv,
        );
        push_scored(
            PaletteCategory::Backtest,
            "Open Strategy Backtest Lab & Tearsheet".to_owned(),
            format!(
                "Active preset: {} · Synchronized Equity/Drawdown pane",
                self.backtest_config.preset.short_label()
            ),
            "backtest strategy lab tearsheet sharpe sortino calmar equity drawdown".to_owned(),
            PaletteAction::OpenStrategyLab,
        );
        push_scored(
            PaletteCategory::Backtest,
            "Toggle Synchronized Equity / Drawdown Sub-Pane".to_owned(),
            if self.show_equity_pane {
                "Currently ON · Hide baseline equity sub-pane".to_owned()
            } else {
                "Currently OFF · Show baseline equity & underwater DD pane".to_owned()
            },
            "backtest equity curve drawdown baseline pane".to_owned(),
            PaletteAction::ToggleEquityPane,
        );
        push_scored(
            PaletteCategory::Backtest,
            "Toggle Backtest Entry/Exit Trade Links on Chart".to_owned(),
            if self.show_backtest_trades {
                "Currently ON · Hide entry/exit arrows & connectors".to_owned()
            } else {
                "Currently OFF · Show causal fill arrows & PnL connectors".to_owned()
            },
            "backtest trades markers links arrows entry exit chart".to_owned(),
            PaletteAction::ToggleBacktestTradesOnChart,
        );
        push_scored(
            PaletteCategory::Backtest,
            format!(
                "Run 12-Variant Parameter Sweep ({})",
                self.backtest_config.preset.short_label()
            ),
            format!(
                "Rank grid by {} with IS/OOS robustness check",
                self.sweep_ranking.short_label()
            ),
            "backtest sweep optimize parameter grid sharpe oos".to_owned(),
            PaletteAction::RunParameterSweep,
        );
        push_scored(
            PaletteCategory::Backtest,
            "Run 13-Symbol Universe Strategy Battle".to_owned(),
            format!(
                "Compare {} across VN30, HNX/UPCOM, Crypto & VN30F1M",
                self.backtest_config.preset.short_label()
            ),
            "backtest universe battle multi symbol compare watchlist".to_owned(),
            PaletteAction::RunUniverseStrategyBattle,
        );
        push_scored(
            PaletteCategory::Backtest,
            format!(
                "Run {}-Fold Walk-Forward Validation ({})",
                self.wf_config.fold_count,
                if self.wf_config.anchored {
                    "Anchored"
                } else {
                    "Rolling"
                }
            ),
            "Causal IS->OOS stitched equity curve, Pardo WFE & parameter stability".to_owned(),
            "walk forward wfo oos rolling anchored efficiency robustness".to_owned(),
            PaletteAction::RunWalkForwardValidation,
        );
        push_scored(
            PaletteCategory::Backtest,
            format!(
                "Run Multi-Asset Portfolio Simulation ({})",
                self.portfolio_universe_filter.short_label()
            ),
            format!(
                "{} allocation · Plot combined portfolio equity curve",
                self.portfolio_weighting.short_label()
            ),
            "portfolio multi asset vn30 equal weight inverse volatility diversification".to_owned(),
            PaletteAction::RunPortfolioBacktest,
        );
        for preset in StrategyPresetKind::ALL {
            push_scored(
                PaletteCategory::Backtest,
                format!("Strategy Preset: {}", preset.full_label()),
                preset.summary_rule().to_owned(),
                format!(
                    "backtest strategy preset {} {}",
                    preset.short_label(),
                    preset.full_label()
                ),
                PaletteAction::SelectStrategyPreset(preset),
            );
        }
        push_scored(
            PaletteCategory::Replay,
            "Toggle Bar Replay Transport Dock".to_owned(),
            "Show/hide causal replay controls".to_owned(),
            "bar replay transport dock".to_owned(),
            PaletteAction::ToggleReplayBar,
        );
        if !self.replay.is_active() && self.base_candles.len() >= 4 {
            push_scored(
                PaletteCategory::Replay,
                "Start Causal Bar Replay at 70% Cut".to_owned(),
                "Slice chart at 70% history with zero lookahead".to_owned(),
                "replay start cut 70% practice".to_owned(),
                PaletteAction::CutReplay70Pct,
            );
        }
        if self.replay.is_active() {
            push_scored(
                PaletteCategory::Replay,
                "Replay: Step Forward +1 Bar".to_owned(),
                "Advance causal replay frontier by 1 step".to_owned(),
                "replay step forward next bar".to_owned(),
                PaletteAction::StepReplayForward,
            );
            push_scored(
                PaletteCategory::Replay,
                "Replay: Step Backward -1 Bar".to_owned(),
                "Rewind causal replay frontier by 1 step".to_owned(),
                "replay step backward rewind prev".to_owned(),
                PaletteAction::StepReplayBackward,
            );
            push_scored(
                PaletteCategory::Replay,
                "Replay: Exit to Live Full Series".to_owned(),
                "Restore full unmasked OHLCV series".to_owned(),
                "replay exit stop live".to_owned(),
                PaletteAction::ExitReplay,
            );
        }
        for mode in [
            PriceScaleMode::Normal,
            PriceScaleMode::Logarithmic,
            PriceScaleMode::Percentage,
            PriceScaleMode::IndexedTo100,
        ] {
            push_scored(
                PaletteCategory::Scale,
                format!("Price Scale: {}", mode.label()),
                "Switch vertical price axis transformation".to_owned(),
                format!("scale axis price mode {}", mode.label()),
                PaletteAction::SetScaleMode(mode),
            );
        }
        push_scored(
            PaletteCategory::Workspace,
            "Fit Entire Series to Viewport".to_owned(),
            "Fit [0..N) bars & reset all pane autoscales".to_owned(),
            "fit all content zoom full".to_owned(),
            PaletteAction::FitContent,
        );
        push_scored(
            PaletteCategory::Workspace,
            "Reset Chart Viewport to Default".to_owned(),
            "Restore 8px bar spacing & autoscale".to_owned(),
            "reset default view camera".to_owned(),
            PaletteAction::ResetView,
        );

        if !q.is_empty() {
            out.sort_by(|a, b| {
                b.score
                    .cmp(&a.score)
                    .then_with(|| a.title.len().cmp(&b.title.len()))
                    .then_with(|| a.title.cmp(&b.title))
            });
        }
        out
    }

    /// Dispatches a selected [`PaletteAction`] and closes the `Cmd+K` Command Palette.
    pub fn execute_palette_action(&mut self, action: PaletteAction) {
        self.close_command_palette();
        match action {
            PaletteAction::SelectWatchlistSymbol { preset, symbol } => {
                self.select_watchlist_instrument(preset, &symbol);
            }
            PaletteAction::SetTimeframe(tf) => {
                let _ = self.set_timeframe(tf);
            }
            PaletteAction::ToggleIndicator(key) => {
                match key {
                    "SMA" => self.indicators.show_sma = !self.indicators.show_sma,
                    "EMA" => self.indicators.show_ema = !self.indicators.show_ema,
                    "BB" => self.indicators.show_bbands = !self.indicators.show_bbands,
                    "VWAP" => self.indicators.show_vwap = !self.indicators.show_vwap,
                    "VRVP" => self.indicators.show_vrvp = !self.indicators.show_vrvp,
                    "RSI" => {
                        self.indicators.show_rsi = !self.indicators.show_rsi;
                        if self.indicators.show_rsi {
                            self.viewport.rsi_price_scale.reset_auto_scale();
                        }
                    }
                    "MACD" => {
                        self.indicators.show_macd = !self.indicators.show_macd;
                        if self.indicators.show_macd {
                            self.viewport.macd_price_scale.reset_auto_scale();
                        }
                    }
                    "ATR" => {
                        self.indicators.show_atr = !self.indicators.show_atr;
                        if self.indicators.show_atr {
                            self.viewport.atr_price_scale.reset_auto_scale();
                        }
                    }
                    "RS" => {
                        self.indicators.show_rs = !self.indicators.show_rs;
                        if self.indicators.show_rs {
                            self.viewport.rs_price_scale.reset_auto_scale();
                        }
                    }
                    _ => {}
                }
                self.sync_indicators_and_benchmarks();
            }
            PaletteAction::ToggleCompareOverlay => {
                self.show_compare_overlay = !self.show_compare_overlay;
                self.viewport.main_price_scale.reset_auto_scale();
                self.sync_indicators_and_benchmarks();
            }
            PaletteAction::CycleCompareBenchmark => {
                self.cycle_compare_benchmark();
            }
            PaletteAction::SelectDrawingTool(tool) => {
                if tool != DrawingTool::Select {
                    self.cancel_replay_cut_selection();
                }
                self.drawings.set_tool(tool);
            }
            PaletteAction::ToggleMagnet => {
                self.drawings.magnet_mode = match self.drawings.magnet_mode {
                    MagnetMode::Off => MagnetMode::Ohlc,
                    MagnetMode::Ohlc => MagnetMode::Off,
                };
            }
            PaletteAction::ToggleScannerDock => {
                self.show_scanner_dock = !self.show_scanner_dock;
            }
            PaletteAction::OpenAlertsTab => {
                self.show_alerts_panel = true;
                self.right_dock_tab = RightDockTab::Alerts;
            }
            PaletteAction::OpenTradingTab => {
                self.show_trading_panel = true;
                self.right_dock_tab = RightDockTab::Trading;
            }
            PaletteAction::OpenJournalTab => {
                self.open_journal_tab();
            }
            PaletteAction::OpenStrategyLab => {
                self.open_strategy_lab();
            }
            PaletteAction::ToggleEquityPane => {
                self.show_equity_pane = !self.show_equity_pane;
                if self.show_equity_pane {
                    self.viewport.equity_price_scale.reset_auto_scale();
                    self.sync_backtest_causal();
                }
            }
            PaletteAction::ToggleBacktestTradesOnChart => {
                self.show_backtest_trades = !self.show_backtest_trades;
                if self.show_backtest_trades {
                    self.sync_backtest_causal();
                }
            }
            PaletteAction::SelectStrategyPreset(preset) => {
                self.backtest_config.preset = preset;
                self.open_strategy_lab();
            }
            PaletteAction::RunParameterSweep => {
                self.open_strategy_lab();
                self.strategy_lab_tab = StrategyLabSubTab::Sweep;
                self.run_active_parameter_sweep();
            }
            PaletteAction::RunUniverseStrategyBattle => {
                self.open_strategy_lab();
                self.strategy_lab_tab = StrategyLabSubTab::Battle;
                self.run_universe_strategy_battle();
            }
            PaletteAction::RunWalkForwardValidation => {
                self.open_strategy_lab();
                self.strategy_lab_tab = StrategyLabSubTab::WalkForward;
                self.equity_pane_source = EquityPaneSource::WalkForwardOos;
                self.run_active_walk_forward();
                self.viewport.equity_price_scale.reset_auto_scale();
            }
            PaletteAction::RunPortfolioBacktest => {
                self.open_strategy_lab();
                self.strategy_lab_tab = StrategyLabSubTab::WalkForward;
                self.equity_pane_source = EquityPaneSource::MultiAssetPortfolio;
                self.run_active_portfolio_backtest();
                self.viewport.equity_price_scale.reset_auto_scale();
            }
            PaletteAction::DrawRiskRewardFromPlanner => {
                let _ = self.draw_or_update_rr_box_from_planner();
            }
            PaletteAction::SyncPaperTradesToJournal => {
                self.open_journal_tab();
                let _ = self.sync_paper_trades_to_journal();
            }
            PaletteAction::ExportJournalCsv => {
                let _ = self.export_journal_csv_causal();
            }
            PaletteAction::QuickPriceAlert { price, direction } => {
                let _ = self.add_quick_price_alert(price, direction);
            }
            PaletteAction::ToggleReplayBar => {
                if self.show_replay_bar && !self.replay.is_active() {
                    self.show_replay_bar = false;
                    self.replay.cancel_cut_selection();
                } else {
                    self.show_replay_bar = true;
                }
            }
            PaletteAction::CutReplay70Pct => {
                if self.base_candles.len() >= 4 {
                    let cut = (self.base_candles.len() * 70) / 100;
                    let _ = self.start_replay_at_base_index(cut);
                }
            }
            PaletteAction::StepReplayForward => {
                let _ = self.step_replay_forward(1);
            }
            PaletteAction::StepReplayBackward => {
                let _ = self.step_replay_backward(1);
            }
            PaletteAction::ExitReplay => {
                self.exit_replay();
                self.show_replay_bar = false;
            }
            PaletteAction::SetScaleMode(mode) => {
                self.set_scale_mode(mode);
            }
            PaletteAction::FitContent => {
                self.fit_content();
            }
            PaletteAction::ResetView => {
                self.reset_view();
            }
        }
    }
}

/// Pure deterministic fuzzy/substring scorer for the `Cmd+K` Command Palette (`Milestone R08`).
#[must_use]
pub fn score_palette_match(query: &str, title: &str, keywords: &str) -> Option<i32> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Some(0);
    }
    let title_l = title.to_ascii_lowercase();
    let kw_l = keywords.to_ascii_lowercase();
    let mut total_score = 0_i32;

    for token in trimmed.split_whitespace() {
        let tok = token.to_ascii_lowercase();
        if tok.is_empty() {
            continue;
        }
        if title_l == tok {
            total_score += 600;
        } else if title_l.starts_with(&tok) {
            total_score += 360;
        } else if title_l
            .split(|c: char| !c.is_ascii_alphanumeric())
            .any(|w| w.starts_with(&tok))
        {
            total_score += 240;
        } else if title_l.contains(&tok) {
            total_score += 140;
        } else if kw_l.contains(&tok) {
            total_score += 90;
        } else {
            let subseq_bonus = ordered_subsequence_score(&tok, &title_l)?;
            total_score += 40 + subseq_bonus;
        }
    }

    Some(total_score - (title.len().min(80) as i32))
}

fn ordered_subsequence_score(needle: &str, haystack: &str) -> Option<i32> {
    if needle.len() < 2 {
        return None;
    }
    let mut hay_chars = haystack.char_indices();
    let mut prev_pos: Option<usize> = None;
    let mut consecutive_bonus = 0_i32;
    for nc in needle.chars() {
        let (pos, _) = hay_chars.find(|&(_, hc)| hc == nc)?;
        if let Some(p) = prev_pos
            && pos == p + 1
        {
            consecutive_bonus += 12;
        }
        prev_pos = Some(pos);
    }
    Some(consecutive_bonus)
}

fn parse_quick_alert_query(query: &str, mark_price: f64) -> Option<(f64, CrossDirection)> {
    let lower = query.trim().to_ascii_lowercase();
    let rest = lower
        .strip_prefix("alert ")
        .or_else(|| lower.strip_prefix("alt "))?;
    let trimmed = rest.trim();
    let (explicit_dir, num_part) = if let Some(s) = trimmed.strip_prefix(">=") {
        (Some(CrossDirection::CrossAbove), s.trim())
    } else if let Some(s) = trimmed.strip_prefix('>') {
        (Some(CrossDirection::CrossAbove), s.trim())
    } else if let Some(s) = trimmed.strip_prefix("<=") {
        (Some(CrossDirection::CrossBelow), s.trim())
    } else if let Some(s) = trimmed.strip_prefix('<') {
        (Some(CrossDirection::CrossBelow), s.trim())
    } else if let Some(s) = trimmed.strip_prefix("above ") {
        (Some(CrossDirection::CrossAbove), s.trim())
    } else if let Some(s) = trimmed.strip_prefix("below ") {
        (Some(CrossDirection::CrossBelow), s.trim())
    } else {
        (None, trimmed)
    };
    let price_tok = num_part.split_whitespace().next()?.replace(',', "");
    let price = price_tok.parse::<f64>().ok()?;
    if !price.is_finite() || price <= 0.0 || price > crate::data::MAX_ABS_PRICE {
        return None;
    }
    let dir = explicit_dir.unwrap_or(if mark_price > 0.0 && price < mark_price {
        CrossDirection::CrossBelow
    } else {
        CrossDirection::CrossAbove
    });
    Some((price, dir))
}

fn compute_drawings_alert_digest(store: &DrawingStore) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325_u64;
    let mix = |h: &mut u64, v: u64| {
        *h ^= v;
        *h = h.wrapping_mul(0x0100_0000_01b3);
    };
    for d in &store.drawings {
        if d.pane != DrawingPaneTarget::Main {
            continue;
        }
        mix(&mut h, d.id);
        mix(&mut h, u64::from(d.visible));
        mix(&mut h, d.created_at_timestamp.unwrap_or(-1) as u64);
        match d.geometry {
            DrawingGeometry::HorizontalLine { anchor } => {
                mix(&mut h, 1);
                mix(&mut h, anchor.bar_index.to_bits());
                mix(&mut h, anchor.value.to_bits());
                mix(&mut h, anchor.timestamp.map_or(u64::MAX, f64::to_bits));
            }
            DrawingGeometry::TrendLine { start, end } => {
                mix(&mut h, 2);
                mix(&mut h, start.bar_index.to_bits());
                mix(&mut h, start.value.to_bits());
                mix(&mut h, start.timestamp.map_or(u64::MAX, f64::to_bits));
                mix(&mut h, end.bar_index.to_bits());
                mix(&mut h, end.value.to_bits());
                mix(&mut h, end.timestamp.map_or(u64::MAX, f64::to_bits));
            }
            DrawingGeometry::Ray { origin, through } => {
                mix(&mut h, 3);
                mix(&mut h, origin.bar_index.to_bits());
                mix(&mut h, origin.value.to_bits());
                mix(&mut h, origin.timestamp.map_or(u64::MAX, f64::to_bits));
                mix(&mut h, through.bar_index.to_bits());
                mix(&mut h, through.value.to_bits());
                mix(&mut h, through.timestamp.map_or(u64::MAX, f64::to_bits));
            }
            DrawingGeometry::Rectangle { .. }
            | DrawingGeometry::FibonacciRetracement { .. }
            | DrawingGeometry::RiskRewardBox { .. } => {}
            DrawingGeometry::FixedRangeVolumeProfile {
                start,
                end,
                row_count,
            } => {
                mix(&mut h, 7);
                mix(&mut h, start.bar_index.to_bits());
                mix(&mut h, start.value.to_bits());
                mix(&mut h, start.timestamp.map_or(u64::MAX, f64::to_bits));
                mix(&mut h, end.bar_index.to_bits());
                mix(&mut h, end.value.to_bits());
                mix(&mut h, end.timestamp.map_or(u64::MAX, f64::to_bits));
                mix(&mut h, u64::from(row_count));
            }
            DrawingGeometry::AnchoredVwap { anchor, show_bands } => {
                mix(&mut h, 8);
                mix(&mut h, anchor.bar_index.to_bits());
                mix(&mut h, anchor.value.to_bits());
                mix(&mut h, anchor.timestamp.map_or(u64::MAX, f64::to_bits));
                mix(&mut h, u64::from(show_bands));
            }
        }
    }
    h
}

fn compute_alert_rules_digest(engine: &AlertEngine) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325_u64;
    let mix = |h: &mut u64, v: u64| {
        *h ^= v;
        *h = h.wrapping_mul(0x0100_0000_01b3);
    };
    mix(&mut h, engine.rules.len() as u64);
    for r in &engine.rules {
        mix(&mut h, r.id);
        mix(&mut h, r.status as u64);
        mix(&mut h, r.frequency as u64);
        mix(&mut h, r.timeframe.seconds() as u64);
        mix(&mut h, r.armed_at_timestamp as u64);
        match r.condition {
            AlertCondition::PriceLevel { price, direction } => {
                mix(&mut h, 11);
                mix(&mut h, price.to_bits());
                mix(&mut h, direction as u64);
            }
            AlertCondition::RsiThreshold {
                period,
                level,
                direction,
            } => {
                mix(&mut h, 12);
                mix(&mut h, period as u64);
                mix(&mut h, level.to_bits());
                mix(&mut h, direction as u64);
            }
            AlertCondition::MacdHistogramZeroCross {
                fast,
                slow,
                signal,
                direction,
            } => {
                mix(&mut h, 13);
                mix(&mut h, fast as u64);
                mix(&mut h, slow as u64);
                mix(&mut h, signal as u64);
                mix(&mut h, direction as u64);
            }
            AlertCondition::DrawingLine {
                drawing_id,
                direction,
            } => {
                mix(&mut h, 14);
                mix(&mut h, drawing_id);
                mix(&mut h, direction as u64);
            }
        }
    }
    h
}

fn compute_backtest_config_digest(cfg: &BacktestConfig) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325_u64;
    let mix = |acc: &mut u64, val: u64| {
        for b in val.to_le_bytes() {
            *acc ^= u64::from(b);
            *acc = acc.wrapping_mul(0x0100_0000_01b3);
        }
    };
    let mix_opt = |acc: &mut u64, val: Option<f64>| match val {
        None => mix(acc, 0),
        Some(v) => {
            mix(acc, 1);
            mix(acc, v.to_bits());
        }
    };
    mix(&mut h, cfg.preset as u64);
    mix(&mut h, cfg.direction_mode as u64);
    match cfg.sizing {
        PositionSizingMode::PercentEquity(v) => {
            mix(&mut h, 1);
            mix(&mut h, v.to_bits());
        }
        PositionSizingMode::RiskPercentPerTrade(v) => {
            mix(&mut h, 2);
            mix(&mut h, v.to_bits());
        }
        PositionSizingMode::FixedNotional(v) => {
            mix(&mut h, 3);
            mix(&mut h, v.to_bits());
        }
    }
    mix(&mut h, cfg.params.fast_sma as u64);
    mix(&mut h, cfg.params.slow_ema as u64);
    mix(&mut h, cfg.params.rsi_period as u64);
    mix(&mut h, cfg.params.rsi_oversold.to_bits());
    mix(&mut h, cfg.params.rsi_overbought.to_bits());
    mix(&mut h, cfg.params.rsi_exit_midline.to_bits());
    mix(&mut h, cfg.params.rsi_confluence_buy.to_bits());
    mix(&mut h, cfg.params.rsi_confluence_sell.to_bits());
    mix(&mut h, cfg.params.macd_fast as u64);
    mix(&mut h, cfg.params.macd_slow as u64);
    mix(&mut h, cfg.params.macd_signal as u64);
    mix(&mut h, cfg.params.macd_trend_ema as u64);
    mix(&mut h, cfg.params.bb_period as u64);
    mix(&mut h, cfg.params.bb_std_mult.to_bits());
    mix(&mut h, cfg.params.max_squeeze_bw_pct.to_bits());
    mix(&mut h, cfg.params.donchian_entry_period as u64);
    mix(&mut h, cfg.params.donchian_exit_period as u64);
    mix(&mut h, cfg.params.atr_period as u64);

    mix_opt(&mut h, cfg.risk.stop_loss_pct);
    mix_opt(&mut h, cfg.risk.stop_loss_atr_mult);
    mix_opt(&mut h, cfg.risk.take_profit_pct);
    mix_opt(&mut h, cfg.risk.take_profit_rr_mult);
    mix_opt(&mut h, cfg.risk.trailing_stop_atr_mult);
    match cfg.risk.max_holding_bars {
        None => mix(&mut h, 0),
        Some(n) => {
            mix(&mut h, 1);
            mix(&mut h, n as u64);
        }
    }
    mix(&mut h, cfg.risk.min_holding_bars as u64);

    mix(&mut h, cfg.initial_cash.to_bits());
    mix(&mut h, cfg.commission_bps.to_bits());
    mix(&mut h, cfg.tax_sell_bps.to_bits());
    mix(&mut h, cfg.slippage_bps.to_bits());
    mix(&mut h, cfg.max_leverage.to_bits());
    mix(&mut h, cfg.lot_step.to_bits());
    mix_opt(&mut h, cfg.in_sample_split_pct);
    h
}

fn compute_walk_forward_config_digest(wf: &WalkForwardConfig) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325_u64;
    let mix = |acc: &mut u64, val: u64| {
        for b in val.to_le_bytes() {
            *acc ^= u64::from(b);
            *acc = acc.wrapping_mul(0x0100_0000_01b3);
        }
    };
    mix(&mut h, wf.fold_count as u64);
    mix(&mut h, wf.in_sample_ratio.to_bits());
    mix(&mut h, u64::from(wf.anchored));
    mix(&mut h, wf.ranking_metric as u64);
    h
}

fn sanitize_persisted_trade_plan_input(inp: &mut TradePlanInput) {
    if !inp.venue.allows_short() {
        inp.side = PositionSide::Long;
    }
    if !inp.account_equity.is_finite() || inp.account_equity <= 0.0 {
        inp.account_equity = 100_000.0;
    } else {
        inp.account_equity = inp.account_equity.clamp(10.0, 1.0e12);
    }
    if !inp.available_cash.is_finite() || inp.available_cash <= 0.0 {
        inp.available_cash = inp.account_equity;
    } else {
        inp.available_cash = inp.available_cash.clamp(1.0, 1.0e12);
    }
    if !inp.leverage.is_finite() || inp.leverage <= 0.0 {
        inp.leverage = 1.0;
    } else {
        inp.leverage = inp.leverage.clamp(0.1, 20.0);
    }
    if !inp.risk_pct.is_finite() || inp.risk_pct <= 0.0 {
        inp.risk_pct = 1.0;
    } else {
        inp.risk_pct = inp.risk_pct.clamp(0.05, 50.0);
    }
    if !inp.entry_price.is_finite() || !(1.0e-8..=1.0e12).contains(&inp.entry_price) {
        inp.entry_price = 100.0;
    }
    if !inp.stop_loss.is_finite() || !(1.0e-8..=1.0e12).contains(&inp.stop_loss) {
        inp.stop_loss = match inp.side {
            PositionSide::Long => (inp.entry_price * 0.96).max(1.0e-8),
            PositionSide::Short => (inp.entry_price * 1.04).min(1.0e12),
        };
    }
    if !inp.target_price.is_finite() || !(1.0e-8..=1.0e12).contains(&inp.target_price) {
        inp.target_price = match inp.side {
            PositionSide::Long => (inp.entry_price * 1.08).min(1.0e12),
            PositionSide::Short => (inp.entry_price * 0.92).max(1.0e-8),
        };
    }
    match inp.side {
        PositionSide::Long => {
            if inp.stop_loss >= inp.entry_price {
                inp.stop_loss = (inp.entry_price * 0.96).max(1.0e-8);
            }
            if inp.target_price <= inp.entry_price {
                inp.target_price = (inp.entry_price * 1.08).min(1.0e12);
            }
        }
        PositionSide::Short => {
            if inp.stop_loss <= inp.entry_price {
                inp.stop_loss = (inp.entry_price * 1.04).min(1.0e12);
            }
            if inp.target_price >= inp.entry_price {
                inp.target_price = (inp.entry_price * 0.92).max(1.0e-8);
            }
        }
    }
    inp.reference_price = inp
        .reference_price
        .filter(|v| v.is_finite() && (1.0e-8..=1.0e12).contains(v));
    if !inp.contract_multiplier.is_finite() || inp.contract_multiplier <= 0.0 {
        inp.contract_multiplier = 1.0;
    } else {
        inp.contract_multiplier = inp.contract_multiplier.clamp(1.0e-4, 1.0e9);
    }
    if !inp.initial_margin_pct.is_finite() || inp.initial_margin_pct <= 0.0 {
        inp.initial_margin_pct = inp.venue.default_initial_margin_pct();
    } else {
        inp.initial_margin_pct = inp.initial_margin_pct.clamp(1.0, 100.0);
    }
    if !inp.max_position_pct.is_finite() || inp.max_position_pct <= 0.0 {
        inp.max_position_pct = 100.0;
    } else {
        inp.max_position_pct = inp.max_position_pct.clamp(1.0, 1_000.0);
    }
    inp.entry_timestamp = inp
        .entry_timestamp
        .filter(|t| (-4_102_444_800..=32_503_680_000).contains(t));
    if !inp.commission_bps.is_finite() || !(0.0..1_000.0).contains(&inp.commission_bps) {
        inp.commission_bps = inp.venue.default_commission_bps();
    }
    if !inp.tax_sell_bps.is_finite() || !(0.0..1_000.0).contains(&inp.tax_sell_bps) {
        inp.tax_sell_bps = inp.venue.default_tax_sell_bps();
    }
    if !inp.slippage_bps.is_finite() || !(0.0..1_000.0).contains(&inp.slippage_bps) {
        inp.slippage_bps = 5.0;
    }
}

fn format_clean_qty(qty: f64) -> String {
    if !qty.is_finite() || qty <= 0.0 {
        return "0".to_owned();
    }
    if (qty - qty.round()).abs() < 1.0e-6 {
        format!("{:.0}", qty.round())
    } else {
        let s = format!("{qty:.4}");
        s.trim_end_matches('0').trim_end_matches('.').to_owned()
    }
}

fn format_venue_price(price: f64, venue: ExchangeVenue) -> String {
    if !price.is_finite() {
        return "0.00".to_owned();
    }
    match venue {
        ExchangeVenue::Hose | ExchangeVenue::Hnx | ExchangeVenue::Upcom => {
            if price >= 1_000.0 {
                format!("{price:.0}")
            } else {
                format!("{price:.2}")
            }
        }
        ExchangeVenue::Vn30Futures => format!("{price:.1}"),
        ExchangeVenue::VnIndexBenchmark | ExchangeVenue::UsEquities => format!("{price:.2}"),
        ExchangeVenue::CryptoSpotPerp => {
            if price >= 100.0 {
                format!("{price:.2}")
            } else if price >= 1.0 {
                format!("{price:.4}")
            } else if price >= 0.01 {
                format!("{price:.6}")
            } else {
                format!("{price:.8}")
            }
        }
    }
}

fn default_ticket_qty_string(mark_price: f64, initial_cash: f64) -> String {
    if !mark_price.is_finite() || mark_price <= 0.0 {
        return "1".to_owned();
    }
    let target_notional = (initial_cash * 0.12).max(100.0);
    let raw = target_notional / mark_price;
    if raw >= 10.0 {
        format!("{:.0}", ((raw / 5.0).round().max(1.0)) * 5.0)
    } else if raw >= 1.0 {
        format!("{:.0}", raw.round().max(1.0))
    } else if raw >= 0.1 {
        format!("{:.2}", ((raw * 10.0).round().max(1.0)) / 10.0)
    } else {
        format!("{:.4}", raw.max(0.0001))
    }
}

fn parse_ticket_qty(raw: &str) -> Result<f64, crate::trading::TradingError> {
    let cleaned = raw.trim().replace(',', "");
    let qty = cleaned
        .parse::<f64>()
        .map_err(|_| crate::trading::TradingError::InvalidQuantity(0.0))?;
    Ok(qty)
}

fn parse_ticket_required_price(raw: &str) -> Result<f64, crate::trading::TradingError> {
    let cleaned = raw.trim().replace(',', "");
    let price = cleaned
        .parse::<f64>()
        .map_err(|_| crate::trading::TradingError::InvalidPrice(0.0))?;
    Ok(price)
}

fn parse_ticket_optional_price(raw: &str) -> Result<Option<f64>, crate::trading::TradingError> {
    let cleaned = raw.trim().replace(',', "");
    if cleaned.is_empty() {
        return Ok(None);
    }
    let price = cleaned
        .parse::<f64>()
        .map_err(|_| crate::trading::TradingError::InvalidPrice(0.0))?;
    Ok(Some(price))
}

#[derive(Clone, Copy)]
enum CustomSourceUpdate<'a> {
    Clear,
    KeepExisting,
    #[cfg(not(target_arch = "wasm32"))]
    FilePath(&'a str),
    InMemory(DataFileFormat, &'a str),
}

fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for &b in bytes {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

fn format_load_summary(source_tag: &str, rep: &DataLoadReport) -> String {
    format!(
        "Loaded {source_tag}: {} · base {} · {} bars (dedup {}, filtered {})",
        rep.symbol,
        rep.base_timeframe,
        rep.candles.len(),
        rep.deduplicated_count,
        rep.filtered_other_symbol_rows
    )
}

fn indicator_toggle_pill(
    ui: &mut egui::Ui,
    active: &mut bool,
    label: &str,
    accent: Color32,
) -> bool {
    let fill = if *active {
        accent.gamma_multiply(0.24)
    } else {
        Color32::from_rgb(24, 29, 40)
    };
    let stroke = if *active {
        Stroke::new(1.0, accent)
    } else {
        Stroke::new(1.0, palette::BORDER_SUBTLE)
    };
    let text_color = if *active { accent } else { palette::TEXT_MUTED };
    let btn = Button::new(
        RichText::new(label)
            .monospace()
            .size(11.0)
            .color(text_color),
    )
    .fill(fill)
    .stroke(stroke)
    .corner_radius(CornerRadius::same(4));
    let clicked = ui.add(btn).clicked();
    if clicked {
        *active = !*active;
    }
    clicked
}

impl eframe::App for SumiWorkstationApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.render_workstation_ui(ui);
    }
}

impl SumiWorkstationApp {
    /// Renders one complete workstation UI pass (toolbar, optional Replay Dock, optional Paper
    /// Trading panel, status bar, and multi-pane chart canvas). Shared identically between
    /// interactive `eframe` sessions and offscreen Metal/`wgpu` Retina verification captures.
    pub fn render_workstation_ui(&mut self, ui: &mut egui::Ui) {
        // Global keyboard-first `Cmd+K` / `Ctrl+K` Command Palette toggle (`Milestone R08`)
        let cmd_k_pressed = ui.input_mut(|i| {
            i.consume_key(egui::Modifiers::COMMAND, egui::Key::K)
                || i.consume_key(egui::Modifiers::CTRL, egui::Key::K)
        });
        if cmd_k_pressed {
            self.toggle_command_palette();
        }

        if self.command_palette.open {
            let vp_rect = ui.input(|i| i.viewport_rect());
            let modal_est = command_palette_modal_rect_estimate(vp_rect);
            ui.input_mut(|i| {
                let _ = i.consume_key(egui::Modifiers::NONE, egui::Key::Tab);
                let _ = i.consume_key(egui::Modifiers::SHIFT, egui::Key::Tab);
                if i.pointer
                    .hover_pos()
                    .is_none_or(|pos| !modal_est.contains(pos))
                {
                    i.smooth_scroll_delta = Vec2::ZERO;
                    i.raw
                        .events
                        .retain(|ev| !matches!(ev, egui::Event::MouseWheel { .. }));
                }
            });
        }

        if self.replay.is_playing() && !self.command_palette.open {
            let dt = f64::from(ui.input(|i| i.stable_dt).max(0.0));
            if dt > 0.0
                && let Some(tr) =
                    self.replay
                        .advance_playback_dt(dt, &self.base_candles, self.active_timeframe)
            {
                self.apply_replay_transition(tr);
            }
            if self.replay.is_playing() {
                let wait_ms = (self.replay.speed.interval_secs() * 500.0).clamp(16.0, 250.0) as u64;
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(wait_ms));
            }
        }

        // Dirty-tracked causal Alert evaluation (`0.000 ms` on idle hover/pan/zoom frames!)
        self.sync_alerts_causal();

        let was_selecting_cut_bar = self.replay.selecting_cut_bar;

        // 1. Top Workstation Toolbar
        let top_frame = Frame::new()
            .fill(palette::BG_WORKSTATION)
            .inner_margin(Margin::symmetric(12, 7))
            .stroke(Stroke::new(1.0, palette::BORDER_SUBTLE));

        egui::Panel::top("workstation_top_toolbar")
            .frame(top_frame)
            .show(ui, |ui| {
                if self.command_palette.open {
                    ui.visuals_mut().disabled_alpha = 1.0;
                    ui.disable();
                }
                // Row 1: Identity, Price Ticker, Data Feeds, Multi-Timeframe Selector (`1m..1W`) & Replay/Trade Toggles
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(5.0, 4.0);

                    Frame::new()
                        .fill(palette::ACCENT_BLUE)
                        .corner_radius(CornerRadius::same(4))
                        .inner_margin(Margin::symmetric(8, 3))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new("SUMI WORKSTATION")
                                    .strong()
                                    .size(12.0)
                                    .color(Color32::WHITE),
                            );
                        });

                    Frame::new()
                        .fill(Color32::from_rgb(26, 31, 44))
                        .corner_radius(CornerRadius::same(4))
                        .stroke(Stroke::new(1.0, palette::BORDER_SUBTLE))
                        .inner_margin(Margin::symmetric(8, 3))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(format!("{} · {}", self.symbol, self.timeframe))
                                    .monospace()
                                    .strong()
                                    .size(12.0)
                                    .color(palette::TEXT_PRIMARY),
                            );
                        });

                    if let Some(last) = self.candles.last() {
                        let color = if last.is_bullish() {
                            palette::BULLISH
                        } else {
                            palette::BEARISH
                        };
                        let sign = if last.change() >= 0.0 { "+" } else { "" };
                        ui.label(
                            RichText::new(format!(
                                "{:.2} ({sign}{:.2}%)",
                                last.close,
                                last.change_pct()
                            ))
                            .monospace()
                            .size(12.0)
                            .color(color),
                        );
                    }

                    ui.separator();

                    // Data Feed Presets + CSV/JSON File Import Toggle
                    let mut chosen_preset = None;
                    for preset in DataSourcePreset::TOOLBAR_PRESETS {
                        if ui
                            .selectable_label(self.data_source == preset, preset.short_label())
                            .clicked()
                        {
                            chosen_preset = Some(preset);
                        }
                    }
                    if let Some(p) = chosen_preset {
                        self.select_data_source(p);
                    }

                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        let _ = indicator_toggle_pill(
                            ui,
                            &mut self.show_import_bar,
                            "Import CSV/JSON",
                            Color32::from_rgb(38, 198, 164),
                        );
                    }

                    if self.data_source == DataSourcePreset::SyntheticVnindex {
                        ui.separator();
                        let mut requested_size = None;
                        for (count, label) in [
                            (120_usize, "120B"),
                            (10_000_usize, "10K"),
                            (50_000_usize, "50K"),
                            (100_000_usize, "100K"),
                        ] {
                            if ui
                                .selectable_label(self.dataset_bar_count == count, label)
                                .clicked()
                            {
                                requested_size = Some(count);
                            }
                        }
                        if let Some(new_count) = requested_size {
                            self.set_dataset_size(new_count);
                        }
                    } else if self.available_symbols.len() > 1 {
                        ui.separator();
                        let mut picked_sym: Option<String> = None;
                        if self.available_symbols.len() <= 6 {
                            for sym in &self.available_symbols {
                                if ui
                                    .selectable_label(self.symbol.eq_ignore_ascii_case(sym), sym)
                                    .clicked()
                                {
                                    picked_sym = Some(sym.clone());
                                }
                            }
                        } else {
                            egui::ComboBox::from_id_salt("multi_symbol_picker")
                                .selected_text(RichText::new(&self.symbol).monospace().size(11.0))
                                .width(116.0)
                                .show_ui(ui, |ui| {
                                    for sym in &self.available_symbols {
                                        let is_cur = self.symbol.eq_ignore_ascii_case(sym);
                                        if ui.selectable_label(is_cur, sym).clicked() {
                                            picked_sym = Some(sym.clone());
                                        }
                                    }
                                });
                        }
                        if let Some(sym) = picked_sym {
                            self.select_symbol_in_source(&sym);
                        }
                    }

                    ui.separator();

                    // Multi-Timeframe Selector (`1m`, `5m`, `15m`, `1h`, `4h`, `1D`, `1W`)
                    ui.label(
                        RichText::new("TF:")
                            .monospace()
                            .strong()
                            .size(11.0)
                            .color(palette::TEXT_MUTED),
                    );
                    let mut requested_tf = None;
                    for tf in Timeframe::ALL {
                        let allowed = self.base_timeframe.can_aggregate_to(tf);
                        let is_cur = self.active_timeframe == tf;
                        if ui
                            .add_enabled(allowed, Button::selectable(is_cur, tf.label()))
                            .clicked()
                        {
                            requested_tf = Some(tf);
                        }
                    }
                    if let Some(tf) = requested_tf {
                        let _ = self.set_timeframe(tf);
                    }

                    ui.separator();

                    let _ = indicator_toggle_pill(
                        ui,
                        &mut self.show_scanner_dock,
                        "Watchlist / Scan",
                        Color32::from_rgb(41, 182, 246),
                    );

                    let mut replay_dock_open = self.show_replay_bar || self.replay.is_active();
                    if indicator_toggle_pill(
                        ui,
                        &mut replay_dock_open,
                        "<< Bar Replay",
                        palette::REPLAY_CUT_GOLD,
                    ) {
                        if replay_dock_open {
                            self.show_replay_bar = true;
                        } else if !self.replay.is_active() {
                            self.show_replay_bar = false;
                            self.replay.cancel_cut_selection();
                        }
                    }

                    if indicator_toggle_pill(
                        ui,
                        &mut self.show_trading_panel,
                        "Paper Trade",
                        palette::TRADE_ENTRY_BLUE,
                    ) && self.show_trading_panel
                    {
                        self.right_dock_tab = RightDockTab::Trading;
                    }

                    let tb_cutoff = self.replay.cutoff_timestamp(&self.base_candles);
                    let vis_rules = self
                        .alerts
                        .rules
                        .iter()
                        .filter(|r| r.is_causally_visible(tb_cutoff))
                        .count();
                    let alerts_lbl = if vis_rules == 0 {
                        "Alerts".to_owned()
                    } else {
                        format!(
                            "Alerts ({}/{vis_rules})",
                            self.alerts.armed_count(tb_cutoff)
                        )
                    };
                    if indicator_toggle_pill(
                        ui,
                        &mut self.show_alerts_panel,
                        &alerts_lbl,
                        palette::ALERT_ARMED_AMBER,
                    ) && self.show_alerts_panel
                    {
                        self.right_dock_tab = RightDockTab::Alerts;
                    }

                    if indicator_toggle_pill(
                        ui,
                        &mut self.show_strategy_panel,
                        "Strategy Lab",
                        palette::EQUITY_ABOVE_LINE,
                    ) && self.show_strategy_panel
                    {
                        self.right_dock_tab = RightDockTab::StrategyLab;
                        self.show_equity_pane = true;
                        self.show_backtest_trades = true;
                        self.viewport.equity_price_scale.reset_auto_scale();
                    }

                    let j_vis_top = self
                        .journal
                        .visible_entries(self.replay_causal_close_timestamp())
                        .len();
                    let j_lbl = if j_vis_top == 0 {
                        "Risk & Journal".to_owned()
                    } else {
                        format!("Journal ({j_vis_top})")
                    };
                    if indicator_toggle_pill(
                        ui,
                        &mut self.show_journal_panel,
                        &j_lbl,
                        Color32::from_rgb(38, 198, 218),
                    ) && self.show_journal_panel
                    {
                        self.open_journal_tab();
                    }

                    let mut palette_open = self.command_palette.open;
                    if indicator_toggle_pill(
                        ui,
                        &mut palette_open,
                        "Cmd+K",
                        Color32::from_rgb(179, 136, 255),
                    ) {
                        self.toggle_command_palette();
                    }
                });

                // Optional Non-Modal CSV/JSON File Import Bar
                #[cfg(not(target_arch = "wasm32"))]
                if self.show_import_bar {
                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(6.0, 4.0);
                        ui.label(
                            RichText::new("PATH (.csv/.json):")
                                .monospace()
                                .strong()
                                .size(11.0)
                                .color(palette::TEXT_MUTED),
                        );
                        let edit_resp = ui.add(
                            egui::TextEdit::singleline(&mut self.import_path_input)
                                .desired_width(360.0)
                                .font(egui::FontId::monospace(11.0)),
                        );
                        let enter_pressed =
                            edit_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                        if ui.button("Load File").clicked() || enter_pressed {
                            let path_copy = self.import_path_input.clone();
                            let _ = self.import_file_from_path(&path_copy);
                        }
                        if let Some((is_err, ref status_msg)) = self.last_load_status {
                            let st_color = if is_err {
                                palette::BEARISH
                            } else {
                                palette::BULLISH
                            };
                            ui.label(
                                RichText::new(status_msg)
                                    .monospace()
                                    .size(11.0)
                                    .color(st_color),
                            );
                        }
                    });
                }

                // Row 2: Indicators, Scale Modes, Viewport Controls & Drawing Toolbar
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(5.0, 4.0);

                    let _ = indicator_toggle_pill(
                        ui,
                        &mut self.indicators.show_sma,
                        "SMA",
                        palette::IND_SMA,
                    );
                    let _ = indicator_toggle_pill(
                        ui,
                        &mut self.indicators.show_ema,
                        "EMA",
                        palette::IND_EMA,
                    );
                    let _ = indicator_toggle_pill(
                        ui,
                        &mut self.indicators.show_bbands,
                        "BB",
                        palette::IND_BB_BORDER,
                    );
                    let _ = indicator_toggle_pill(
                        ui,
                        &mut self.indicators.show_vwap,
                        "VWAP",
                        palette::IND_VWAP_LINE,
                    );
                    let _ = indicator_toggle_pill(
                        ui,
                        &mut self.indicators.show_vrvp,
                        "VRVP",
                        palette::VRVP_POC_GOLD,
                    );
                    if indicator_toggle_pill(
                        ui,
                        &mut self.indicators.show_rsi,
                        "RSI",
                        palette::IND_RSI,
                    ) && self.indicators.show_rsi
                    {
                        self.viewport.rsi_price_scale.reset_auto_scale();
                    }
                    if indicator_toggle_pill(
                        ui,
                        &mut self.indicators.show_macd,
                        "MACD",
                        palette::IND_MACD,
                    ) && self.indicators.show_macd
                    {
                        self.viewport.macd_price_scale.reset_auto_scale();
                    }
                    if indicator_toggle_pill(
                        ui,
                        &mut self.indicators.show_atr,
                        "ATR",
                        palette::IND_ATR,
                    ) && self.indicators.show_atr
                    {
                        self.viewport.atr_price_scale.reset_auto_scale();
                    }
                    if indicator_toggle_pill(
                        ui,
                        &mut self.indicators.show_rs,
                        "RS",
                        palette::IND_RS_LINE,
                    ) && self.indicators.show_rs
                    {
                        self.viewport.rs_price_scale.reset_auto_scale();
                    }
                    if indicator_toggle_pill(
                        ui,
                        &mut self.show_equity_pane,
                        "EQ",
                        palette::EQUITY_ABOVE_LINE,
                    ) && self.show_equity_pane
                    {
                        self.viewport.equity_price_scale.reset_auto_scale();
                    }
                    let cmp_lbl = format!("Cmp:{}", self.effective_benchmark_symbol());
                    if indicator_toggle_pill(
                        ui,
                        &mut self.show_compare_overlay,
                        &cmp_lbl,
                        palette::COMPARE_BENCH_LINE,
                    ) {
                        self.viewport.main_price_scale.reset_auto_scale();
                    }
                    if ui
                        .small_button("v")
                        .on_hover_text("Cycle RS / Compare Benchmark")
                        .clicked()
                    {
                        self.cycle_compare_benchmark();
                    }

                    ui.separator();

                    let active_mode = self.viewport.main_price_scale.mode();
                    for mode in [
                        PriceScaleMode::Normal,
                        PriceScaleMode::Logarithmic,
                        PriceScaleMode::Percentage,
                        PriceScaleMode::IndexedTo100,
                    ] {
                        if ui
                            .selectable_label(active_mode == mode, mode.label())
                            .clicked()
                        {
                            self.set_scale_mode(mode);
                        }
                    }
                    let is_auto = self.viewport.main_price_scale.is_auto_scale();
                    if ui.selectable_label(is_auto, "Auto").clicked() {
                        self.viewport.main_price_scale.set_auto_scale(!is_auto);
                    }
                    let is_inv = self.viewport.main_price_scale.is_inverted();
                    if ui.selectable_label(is_inv, "Inv").clicked() {
                        self.viewport.main_price_scale.set_inverted(!is_inv);
                    }

                    ui.separator();

                    if ui.button("Zoom +").clicked() {
                        self.zoom_in();
                    }
                    if ui.button("Zoom −").clicked() {
                        self.zoom_out();
                    }
                    if ui.button("Fit").clicked() {
                        self.fit_content();
                    }
                    if ui.button("Reset").clicked() {
                        self.reset_view();
                    }

                    ui.separator();

                    ui.checkbox(&mut self.show_grid, "Grid");
                    ui.checkbox(&mut self.show_crosshair, "Crosshair");
                    ui.checkbox(&mut self.show_volume, "Volume");
                });

                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(5.0, 4.0);

                    ui.label(
                        RichText::new("DRAW:")
                            .monospace()
                            .strong()
                            .size(11.0)
                            .color(palette::TEXT_MUTED),
                    );

                    for tool in [
                        DrawingTool::Select,
                        DrawingTool::HorizontalLine,
                        DrawingTool::TrendLine,
                        DrawingTool::Ray,
                        DrawingTool::Rectangle,
                        DrawingTool::FibonacciRetracement,
                        DrawingTool::RiskRewardBox,
                        DrawingTool::FixedRangeVolumeProfile,
                        DrawingTool::AnchoredVwap,
                    ] {
                        let is_active = self.drawings.active_tool == tool;
                        if ui.selectable_label(is_active, tool.short_label()).clicked() {
                            if tool != DrawingTool::Select {
                                self.cancel_replay_cut_selection();
                            }
                            self.drawings.set_tool(tool);
                        }
                    }

                    ui.separator();

                    let mut magnet_on = self.drawings.magnet_mode == MagnetMode::Ohlc;
                    if indicator_toggle_pill(
                        ui,
                        &mut magnet_on,
                        "Magnet OHLC",
                        Color32::from_rgb(255, 193, 7),
                    ) {
                        self.drawings.magnet_mode = if magnet_on {
                            MagnetMode::Ohlc
                        } else {
                            MagnetMode::Off
                        };
                    }

                    if ui
                        .add_enabled(self.drawings.can_undo(), Button::new("Undo"))
                        .clicked()
                    {
                        let _ = self.drawings.undo();
                    }
                    if ui
                        .add_enabled(self.drawings.can_redo(), Button::new("Redo"))
                        .clicked()
                    {
                        let _ = self.drawings.redo();
                    }
                    if ui
                        .add_enabled(self.candles.len() >= 2, Button::new("Demo Set"))
                        .clicked()
                    {
                        self.populate_showcase_drawings();
                    }
                    if ui
                        .add_enabled(self.drawings.has_unlocked_drawings(), Button::new("Clear"))
                        .clicked()
                    {
                        let _ = self.drawings.clear_unlocked();
                    }

                    if let Some(sel) = self.drawings.selected_drawing().cloned() {
                        ui.separator();
                        let [cr, cg, cb] = sel.style.color.rgb();
                        ui.label(
                            RichText::new(format!(
                                "#{} [{}·{}]",
                                sel.id,
                                sel.pane.short_label(),
                                sel.tool_kind().short_label()
                            ))
                            .monospace()
                            .size(11.0)
                            .color(Color32::from_rgb(cr, cg, cb)),
                        );
                        if ui
                            .add_enabled(
                                !sel.locked,
                                Button::new(format!("Color: {}", sel.style.color.short_label())),
                            )
                            .clicked()
                        {
                            let _ = self.drawings.cycle_selected_color();
                        }
                        if ui
                            .add_enabled(
                                !sel.locked,
                                Button::new(format!("Dash: {}", sel.style.dash.label())),
                            )
                            .clicked()
                        {
                            let _ = self.drawings.cycle_selected_dash();
                        }
                        if let DrawingGeometry::FibonacciRetracement { extend_right, .. } =
                            sel.geometry
                        {
                            if ui
                                .add_enabled(!sel.locked, Button::new("Rev 0↔1"))
                                .clicked()
                            {
                                let _ = self.drawings.reverse_selected_fibonacci();
                            }
                            let ext_label = if extend_right {
                                "Ext >>: ON"
                            } else {
                                "Ext >>: OFF"
                            };
                            if ui
                                .add_enabled(!sel.locked, Button::new(ext_label))
                                .clicked()
                            {
                                let _ = self.drawings.toggle_selected_fib_extend();
                            }
                        }
                        if matches!(sel.geometry, DrawingGeometry::RiskRewardBox { .. }) {
                            if ui
                                .add_enabled(!sel.locked, Button::new("Flip L/S"))
                                .clicked()
                            {
                                let _ = self.drawings.reverse_selected_risk_reward();
                                let _ = self.sync_planner_from_selected_rr_box();
                            }
                            if ui.button("-> Planner").clicked() {
                                self.open_journal_tab();
                            }
                        }
                        if let DrawingGeometry::FixedRangeVolumeProfile { row_count, .. } =
                            sel.geometry
                            && ui
                                .add_enabled(!sel.locked, Button::new(format!("Rows: {row_count}")))
                                .clicked()
                        {
                            let _ = self.drawings.cycle_selected_frvp_rows();
                        }
                        if let DrawingGeometry::AnchoredVwap { show_bands, .. } = sel.geometry {
                            let bands_lbl = if show_bands {
                                "Bands: ON"
                            } else {
                                "Bands: OFF"
                            };
                            if ui
                                .add_enabled(!sel.locked, Button::new(bands_lbl))
                                .clicked()
                            {
                                let _ = self.drawings.toggle_selected_avwap_bands();
                            }
                        }
                        let lock_label = if sel.locked { "Unlock" } else { "Lock" };
                        if ui.button(lock_label).clicked() {
                            let _ = self.drawings.toggle_selected_lock();
                        }
                        if ui.add_enabled(!sel.locked, Button::new("Delete")).clicked() {
                            let _ = self.drawings.delete_selected();
                        }
                        if sel.pane == DrawingPaneTarget::Main
                            && ui.button("-> Order Ticket").clicked()
                        {
                            let _ = self.sync_ticket_from_selected_drawing();
                        }
                        if sel.pane == DrawingPaneTarget::Main
                            && matches!(
                                sel.geometry,
                                DrawingGeometry::HorizontalLine { .. }
                                    | DrawingGeometry::TrendLine { .. }
                                    | DrawingGeometry::Ray { .. }
                            )
                            && ui.button("-> Alert").clicked()
                        {
                            let _ = self.add_alert_on_selected_drawing(self.alert_direction);
                        }
                    }
                });

                // Optional Row 4: Compact Causal Bar Replay Transport Dock (`Milestone R07`)
                if self.show_replay_bar || self.replay.is_active() {
                    ui.add_space(4.0);
                    self.render_replay_dock(ui);
                }
            });

        // Synchronize dirty-tracked indicator, session VWAP, causal RS & backtest caches
        self.sync_indicators_and_benchmarks();
        self.sync_backtest_causal();

        // 2. Bottom Telemetry Status Bar
        let bottom_frame = Frame::new()
            .fill(palette::BG_WORKSTATION)
            .inner_margin(Margin::symmetric(12, 5))
            .stroke(Stroke::new(1.0, palette::BORDER_SUBTLE));

        egui::Panel::bottom("workstation_status_bar")
            .frame(bottom_frame)
            .show(ui, |ui| {
                if self.command_palette.open {
                    ui.visuals_mut().disabled_alpha = 1.0;
                    ui.disable();
                }
                ui.horizontal(|ui| {
                    let visible = self.visible_slice();
                    let first_date = visible.first().map_or("-", |c| c.date_label.as_str());
                    let last_date = visible.last().map_or("-", |c| c.date_label.as_str());
                    let scale_mode = self.viewport.main_price_scale.mode().label();
                    let auto_tag = if self.viewport.main_price_scale.is_auto_scale() {
                        "AUTO"
                    } else {
                        "MAN"
                    };
                    let replay_tag = if self.replay.is_active() {
                        format!(
                            "{} {}/{}",
                            self.replay.status.short_label(),
                            self.replay
                                .base_cursor_index
                                .saturating_add(1)
                                .min(self.base_candles.len()),
                            self.base_candles.len()
                        )
                    } else if self.replay.selecting_cut_bar {
                        "CUT".to_owned()
                    } else {
                        "LIVE".to_owned()
                    };
                    let metrics = self.trading.metrics(self.current_mark_price());
                    let sb_cutoff = self.replay.cutoff_timestamp(&self.base_candles);
                    let armed_alerts = self.alerts.armed_count(sb_cutoff);
                    let trig_events = self
                        .alerts
                        .triggers
                        .iter()
                        .filter(|ev| {
                            sb_cutoff.is_none_or(|cut| alert_event_eval_timestamp(ev) <= cut)
                        })
                        .count();
                    let bt_tag = if (self.show_strategy_panel
                        || self.show_equity_pane
                        || self.show_backtest_trades)
                        && let Some(ref bt) = self.backtest_result
                    {
                        format!(
                            "  |  BT: {:+.1}% ({}T)",
                            bt.tearsheet.total_return_pct, bt.tearsheet.num_trades
                        )
                    } else {
                        String::new()
                    };
                    if let Some(rep) = self.last_report {
                        ui.add(
                            egui::Label::new(
                                RichText::new(format!(
                                    "R11  |  {} ({}->{})  |  RPL: {replay_tag}  |  ALT: {armed_alerts}A/{trig_events}T  |  EQ: ${:.0} ({:+.0}){bt_tag}  |  {}P/{}D  |  BARS: {}/{} ({first_date}..{last_date}) {}x  |  {scale_mode}({auto_tag})  |  {:.0}×{:.0}",
                                    self.data_source.short_label(),
                                    self.base_timeframe,
                                    self.active_timeframe,
                                    metrics.equity,
                                    metrics.net_pnl,
                                    rep.pane_count,
                                    rep.drawing_count,
                                    rep.rendered_candle_count,
                                    self.candles.len(),
                                    rep.conflation_factor,
                                    rep.layout.plot_rect.width(),
                                    rep.layout.plot_rect.height(),
                                ))
                                .monospace()
                                .size(10.2)
                                .color(palette::TEXT_MUTED),
                            )
                            .truncate(),
                        );
                    } else {
                        ui.add(
                            egui::Label::new(
                                RichText::new(format!(
                                    "R11  |  RPL: {replay_tag}  |  ALT: {armed_alerts} armed{bt_tag}  |  BARS: {} ({first_date}..{last_date})",
                                    self.candles.len()
                                ))
                                .monospace()
                                .size(10.2)
                                .color(palette::TEXT_MUTED),
                            )
                            .truncate(),
                        );
                    }
                });
            });

        // 2b. Optional Left-Side Multi-Symbol Watchlist & Technical Scanner Dock (`Milestone R08`)
        if self.show_scanner_dock {
            let left_frame = Frame::new()
                .fill(palette::BG_WORKSTATION)
                .inner_margin(Margin::symmetric(8, 8))
                .stroke(Stroke::new(1.0, palette::BORDER_SUBTLE));

            egui::Panel::left("scanner_watchlist_dock")
                .resizable(true)
                .default_size(276.0)
                .size_range(236.0..=360.0)
                .frame(left_frame)
                .show(ui, |ui| {
                    if self.command_palette.open {
                        ui.visuals_mut().disabled_alpha = 1.0;
                        ui.disable();
                    }
                    self.render_scanner_dock(ui);
                });
        }

        // 3. Optional Unified Right-Side Paper Trading, Alerts, Strategy Lab & Journal Dock (`Milestones R07–R10`)
        if self.show_trading_panel
            || self.show_alerts_panel
            || self.show_strategy_panel
            || self.show_journal_panel
        {
            let first_enabled_tab = if self.show_journal_panel {
                RightDockTab::Journal
            } else if self.show_strategy_panel {
                RightDockTab::StrategyLab
            } else if self.show_trading_panel {
                RightDockTab::Trading
            } else {
                RightDockTab::Alerts
            };
            match self.right_dock_tab {
                RightDockTab::Trading if !self.show_trading_panel => {
                    self.right_dock_tab = first_enabled_tab;
                }
                RightDockTab::Alerts if !self.show_alerts_panel => {
                    self.right_dock_tab = first_enabled_tab;
                }
                RightDockTab::StrategyLab if !self.show_strategy_panel => {
                    self.right_dock_tab = first_enabled_tab;
                }
                RightDockTab::Journal if !self.show_journal_panel => {
                    self.right_dock_tab = first_enabled_tab;
                }
                _ => {}
            }
            let right_frame = Frame::new()
                .fill(palette::BG_WORKSTATION)
                .inner_margin(Margin::symmetric(10, 8))
                .stroke(Stroke::new(1.0, palette::BORDER_SUBTLE));

            let win_w = ui.ctx().content_rect().width();
            let (panel_id, def_w, min_w, max_w) = match self.right_dock_tab {
                RightDockTab::StrategyLab | RightDockTab::Journal => {
                    let def = if win_w < 1060.0 { 282.0 } else { 312.0 };
                    ("strategy_lab_panel", def, 248.0, 386.0)
                }
                RightDockTab::Trading | RightDockTab::Alerts => {
                    ("paper_trading_panel", 258.0, 222.0, 320.0)
                }
            };

            egui::Panel::right(panel_id)
                .resizable(true)
                .default_size(def_w)
                .size_range(min_w..=max_w)
                .frame(right_frame)
                .show(ui, |ui| {
                    if self.command_palette.open {
                        ui.visuals_mut().disabled_alpha = 1.0;
                        ui.disable();
                    }
                    self.render_right_dock(ui);
                });
        }

        // 4. Central Multi-Pane Workstation Chart Canvas
        let central_frame = Frame::new()
            .fill(palette::BG_WORKSTATION)
            .inner_margin(Margin::symmetric(8, 8));

        let mut clicked_cut_idx: Option<usize> = None;
        egui::CentralPanel::default()
            .frame(central_frame)
            .show(ui, |ui| {
                let pre_ro = self.viewport.time_scale.right_offset();
                let pre_bs = self.viewport.time_scale.bar_spacing();
                let pre_draw_digest = compute_drawings_alert_digest(&self.drawings);
                let replay_status_label = self
                    .replay
                    .is_active()
                    .then(|| self.replay.status.short_label());
                let replay_cutoff_timestamp = self.replay.cutoff_timestamp(&self.base_candles);
                let frontier_base_timestamp = self
                    .base_candles
                    .get(self.current_trading_cursor_base_index())
                    .map(|c| c.timestamp);
                let has_trading_visuals = self.show_trading_panel
                    || self.trading.open_position().is_some()
                    || !self.trading.fills().is_empty()
                    || self.trading.pending_orders().next().is_some();
                let has_alert_visuals =
                    !self.alerts.rules.is_empty() || !self.alerts.triggers.is_empty();
                let has_backtest_visuals =
                    self.show_strategy_panel || self.show_equity_pane || self.show_backtest_trades;
                let wf_title_str = self.walk_forward_report.as_ref().map(|wf| {
                    format!(
                        "WFO {}F {} · {}",
                        wf.wf_config.fold_count,
                        if wf.wf_config.anchored { "ANC" } else { "ROL" },
                        wf.preset.short_label()
                    )
                });
                let equity_override = if !self.show_equity_pane && !self.show_backtest_trades {
                    None
                } else {
                    match self.equity_pane_source {
                        EquityPaneSource::SingleAssetBacktest => None,
                        EquityPaneSource::WalkForwardOos => self
                            .walk_forward_report
                            .as_ref()
                            .zip(wf_title_str.as_deref())
                            .map(|(wf, title)| EquityCurveOverlay {
                                title_label: title,
                                baseline_cash: self.backtest_config.initial_cash,
                                equity_values: &wf.stitched_equity_values,
                                equity_idx: &wf.stitched_equity_idx,
                                underwater_values: &wf.stitched_underwater_values,
                                bar_count: wf.total_bars,
                                split_bar_index: wf.first_oos_bar_index,
                                trades: Some(&wf.stitched_oos_trades),
                            }),
                        EquityPaneSource::MultiAssetPortfolio => self
                            .portfolio_mapped_curve
                            .as_ref()
                            .map(|mc| EquityCurveOverlay {
                                title_label: &mc.title_label,
                                baseline_cash: mc.baseline_cash,
                                equity_values: &mc.equity_values,
                                equity_idx: &mc.equity_idx,
                                underwater_values: &mc.underwater_values,
                                bar_count: mc.candle_len,
                                split_bar_index: None,
                                trades: Some(&[]),
                            }),
                    }
                };
                let compare_overlay = (self.show_compare_overlay
                    && !self.compare_series_cached.is_empty())
                .then_some((
                    self.active_benchmark_label.as_str(),
                    self.compare_series_cached.as_slice(),
                    &self.compare_series_idx,
                ));
                let overlay = ChartReplayTradingOverlay {
                    replay_active: self.replay.is_active(),
                    selecting_cut_bar: self.replay.selecting_cut_bar,
                    replay_status_label,
                    trading_session: has_trading_visuals.then_some(&self.trading),
                    alert_engine: has_alert_visuals.then_some(&self.alerts),
                    backtest_result: has_backtest_visuals
                        .then_some(self.backtest_result.as_ref())
                        .flatten(),
                    equity_override,
                    show_equity_pane: self.show_equity_pane,
                    show_backtest_trades_on_chart: self.show_backtest_trades,
                    focused_backtest_trade_id: self.focused_backtest_trade_id,
                    replay_cutoff_timestamp,
                    frontier_base_timestamp,
                    compare_overlay,
                    rs_benchmark_label: (!self.active_benchmark_label.is_empty())
                        .then_some(self.active_benchmark_label.as_str()),
                    modal_open: self.command_palette.open,
                };
                self.last_report = render_workstation_chart_full(
                    ui,
                    &self.candles,
                    &mut self.viewport,
                    ChartDisplayConfig {
                        symbol: &self.symbol,
                        timeframe: &self.timeframe,
                        show_grid: self.show_grid,
                        show_crosshair: self.show_crosshair,
                        show_volume: self.show_volume,
                    },
                    &self.indicators,
                    &self.indicator_cache,
                    &mut self.drawings,
                    overlay,
                );
                self.auto_sync_planner_if_rr_box_changed();
                clicked_cut_idx = self.last_report.and_then(|r| r.clicked_cut_bar_index);
                if compute_drawings_alert_digest(&self.drawings) != pre_draw_digest {
                    self.sync_alerts_causal();
                }
                if (self.viewport.time_scale.right_offset() - pre_ro).abs() > 1e-6
                    || (self.viewport.time_scale.bar_spacing() - pre_bs).abs() > 1e-6
                {
                    self.viewport_focus_lock = None;
                }
            });

        if let Some(active_idx) = clicked_cut_idx {
            let _ = self.handle_chart_cut_bar_click(active_idx);
            ui.ctx().request_repaint();
        }

        // 4b. Floating `Cmd+K` / `Ctrl+K` Command Palette Modal (`Milestone R08`)
        if self.command_palette.open {
            self.render_command_palette_modal(ui);
        }

        // 5. Non-modal keyboard transport shortcuts (suppressed whenever modal or text input holds focus)
        if !self.command_palette.open && !ui.ctx().egui_wants_keyboard_input() {
            let (esc_pressed, space_pressed, step_fwd, step_back) = ui.input(|i| {
                let plain = !i.modifiers.command && !i.modifiers.ctrl && !i.modifiers.alt;
                (
                    i.key_pressed(egui::Key::Escape),
                    plain && i.key_pressed(egui::Key::Space),
                    plain && i.key_pressed(egui::Key::Period),
                    plain && i.key_pressed(egui::Key::Comma),
                )
            });
            let mut changed = false;
            if was_selecting_cut_bar && esc_pressed {
                self.cancel_replay_cut_selection();
                changed = true;
            } else if self.replay.is_active() {
                if space_pressed {
                    let _ = self.toggle_replay_play_pause();
                    changed = true;
                } else if step_fwd {
                    changed = self.step_replay_forward(1);
                } else if step_back {
                    changed = self.step_replay_backward(1);
                }
            }
            if changed {
                ui.ctx().request_repaint();
            }
        }

        self.frame_count = self.frame_count.saturating_add(1);
    }

    fn render_replay_dock(&mut self, ui: &mut egui::Ui) {
        Frame::new()
            .fill(Color32::from_rgb(20, 25, 36))
            .corner_radius(CornerRadius::same(4))
            .stroke(Stroke::new(
                1.0,
                if self.replay.is_active() || self.replay.selecting_cut_bar {
                    palette::REPLAY_CUT_GOLD.gamma_multiply(0.65)
                } else {
                    palette::BORDER_SUBTLE
                },
            ))
            .inner_margin(Margin::symmetric(8, 4))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(5.0, 4.0);

                    let status_text = if self.replay.selecting_cut_bar {
                        "SELECT CUT"
                    } else {
                        self.replay.status.short_label()
                    };
                    let badge_color = match self.replay.status {
                        ReplayStatus::Playing => palette::BULLISH,
                        ReplayStatus::Paused | ReplayStatus::Completed => palette::REPLAY_CUT_GOLD,
                        ReplayStatus::Inactive => {
                            if self.replay.selecting_cut_bar {
                                palette::REPLAY_CUT_GOLD
                            } else {
                                palette::TEXT_MUTED
                            }
                        }
                    };
                    ui.label(
                        RichText::new(format!("REPLAY · {status_text}"))
                            .monospace()
                            .strong()
                            .size(11.0)
                            .color(badge_color),
                    );

                    let mut cut_mode = self.replay.selecting_cut_bar;
                    if indicator_toggle_pill(
                        ui,
                        &mut cut_mode,
                        "<< Cut Bar",
                        palette::REPLAY_CUT_GOLD,
                    ) {
                        if cut_mode {
                            self.enter_replay_cut_selection();
                        } else {
                            self.cancel_replay_cut_selection();
                        }
                    }

                    if !self.replay.is_active()
                        && self.base_candles.len() >= 4
                        && ui.button("Cut 70%").clicked()
                    {
                        let cut_idx = (self.base_candles.len() * 70) / 100;
                        let _ = self.start_replay_at_base_index(cut_idx);
                    }

                    ui.separator();

                    let active = self.replay.is_active() && !self.base_candles.is_empty();
                    let can_back = active && self.replay.base_cursor_index > 0;
                    let can_fwd =
                        active && self.replay.base_cursor_index + 1 < self.base_candles.len();

                    if ui
                        .add_enabled(
                            active && self.replay.base_cursor_index > self.replay.start_base_index,
                            Button::new("|<< Start"),
                        )
                        .clicked()
                    {
                        let _ = self.jump_replay_to_start();
                    }
                    if ui.add_enabled(can_back, Button::new("<< -1")).clicked() {
                        let _ = self.step_replay_backward(1);
                    }
                    let play_label = if self.replay.is_playing() {
                        "Pause"
                    } else {
                        "Play >"
                    };
                    let play_btn = Button::new(
                        RichText::new(play_label)
                            .monospace()
                            .strong()
                            .size(11.0)
                            .color(if self.replay.is_playing() {
                                palette::BULLISH
                            } else {
                                palette::TEXT_PRIMARY
                            }),
                    );
                    if ui
                        .add_enabled(can_fwd || self.replay.is_playing(), play_btn)
                        .clicked()
                    {
                        let _ = self.toggle_replay_play_pause();
                    }
                    if ui.add_enabled(can_fwd, Button::new("+1 >>")).clicked() {
                        let _ = self.step_replay_forward(1);
                    }

                    ui.separator();

                    ui.label(
                        RichText::new("Speed:")
                            .monospace()
                            .size(11.0)
                            .color(palette::TEXT_MUTED),
                    );
                    for sp in ReplaySpeed::ALL {
                        if ui
                            .selectable_label(self.replay.speed == sp, sp.short_label())
                            .clicked()
                        {
                            self.replay.speed = sp;
                        }
                    }

                    if self.active_timeframe != self.base_timeframe {
                        ui.separator();
                        ui.label(
                            RichText::new("Step:")
                                .monospace()
                                .size(11.0)
                                .color(palette::TEXT_MUTED),
                        );
                        for unit in ReplayStepUnit::ALL {
                            if ui
                                .selectable_label(self.replay.step_unit == unit, unit.short_label())
                                .clicked()
                            {
                                self.replay.step_unit = unit;
                            }
                        }
                    }

                    ui.separator();

                    let total_base = self.base_candles.len();
                    let cur_base = if self.replay.is_active() && total_base > 0 {
                        self.replay.base_cursor_index.min(total_base - 1) + 1
                    } else {
                        total_base
                    };
                    let cutoff_lbl = self.candles.last().map_or("-", |c| c.date_label.as_str());
                    ui.label(
                        RichText::new(format!(
                            "Base {cur_base}/{total_base} · Active {} · {cutoff_lbl}",
                            self.candles.len()
                        ))
                        .monospace()
                        .size(11.0)
                        .color(palette::TEXT_PRIMARY),
                    );

                    if let Some(ref ctx) = self.scanner_context {
                        ui.separator();
                        let cutoff = self.replay.cutoff_timestamp(&self.base_candles);
                        if let Some(sig) = ctx.revealed_signal(cutoff) {
                            let sig_color = match sig.direction {
                                SignalDirection::Bullish => palette::BULLISH,
                                SignalDirection::Bearish => palette::BEARISH,
                            };
                            Frame::new()
                                .fill(sig_color.gamma_multiply(0.18))
                                .stroke(Stroke::new(1.0, sig_color))
                                .corner_radius(CornerRadius::same(3))
                                .inner_margin(Margin::symmetric(6, 2))
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new(format!(
                                            "{} · {} ({})",
                                            ctx.intent().short_label(),
                                            sig.signal.short_label(),
                                            sig.direction.short_label()
                                        ))
                                        .monospace()
                                        .strong()
                                        .size(10.5)
                                        .color(sig_color),
                                    );
                                });
                        } else {
                            Frame::new()
                                .fill(palette::ALERT_ARMED_AMBER.gamma_multiply(0.18))
                                .stroke(Stroke::new(1.0, palette::ALERT_ARMED_AMBER))
                                .corner_radius(CornerRadius::same(3))
                                .inner_margin(Margin::symmetric(6, 2))
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new(
                                            "BLIND PRACTICE · Signal hidden until reached",
                                        )
                                        .monospace()
                                        .strong()
                                        .size(10.5)
                                        .color(palette::ALERT_ARMED_AMBER),
                                    );
                                });
                        }
                    }

                    if ui.button("Exit ×").clicked() {
                        self.exit_replay();
                        self.show_replay_bar = false;
                    }
                });
            });
    }

    fn render_paper_trading_panel(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(5.0, 6.0);

                // Header Row
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("PAPER TRADING")
                            .monospace()
                            .strong()
                            .size(12.0)
                            .color(palette::TEXT_PRIMARY),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button(RichText::new("Reset").monospace().size(10.5))
                            .clicked()
                        {
                            self.reset_trading_session();
                        }
                    });
                });

                let mark = self.current_mark_price();
                let metrics = self.trading.metrics(mark);
                let pnl_color = if metrics.net_pnl > 1e-6 {
                    palette::BULLISH
                } else if metrics.net_pnl < -1e-6 {
                    palette::BEARISH
                } else {
                    palette::TEXT_PRIMARY
                };

                // 1. Account Scoreboard Card
                Frame::new()
                    .fill(Color32::from_rgb(19, 24, 35))
                    .corner_radius(CornerRadius::same(4))
                    .stroke(Stroke::new(1.0, palette::BORDER_SUBTLE))
                    .inner_margin(Margin::symmetric(8, 6))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("Equity:")
                                    .monospace()
                                    .size(11.0)
                                    .color(palette::TEXT_MUTED),
                            );
                            ui.label(
                                RichText::new(format!("${:.2}", metrics.equity))
                                    .monospace()
                                    .strong()
                                    .size(11.5)
                                    .color(palette::TEXT_PRIMARY),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("Net PnL:")
                                    .monospace()
                                    .size(11.0)
                                    .color(palette::TEXT_MUTED),
                            );
                            ui.label(
                                RichText::new(format!(
                                    "{:+.2} ({:+.2}%)",
                                    metrics.net_pnl, metrics.return_pct
                                ))
                                .monospace()
                                .strong()
                                .size(11.0)
                                .color(pnl_color),
                            );
                        });
                        ui.label(
                            RichText::new(format!(
                                "Cash ${:.0} · MaxDD {:.2}%",
                                metrics.cash_balance, metrics.max_drawdown_pct
                            ))
                            .monospace()
                            .size(10.5)
                            .color(palette::TEXT_MUTED),
                        );
                        let pf_str = metrics
                            .profit_factor
                            .map_or_else(|| "-".to_owned(), |pf| format!("{pf:.2}"));
                        let r_str = metrics
                            .avg_r_multiple
                            .map_or_else(|| "-".to_owned(), |r| format!("{r:+.2}R"));
                        ui.label(
                            RichText::new(format!(
                                "Trades {} (Win {:.0}%) · PF {pf_str} · {r_str}",
                                metrics.closed_trade_count, metrics.win_rate_pct
                            ))
                            .monospace()
                            .size(10.5)
                            .color(palette::TEXT_MUTED),
                        );
                    });

                // 2. Order Ticket Card
                Frame::new()
                    .fill(Color32::from_rgb(19, 24, 35))
                    .corner_radius(CornerRadius::same(4))
                    .stroke(Stroke::new(1.0, palette::BORDER_SUBTLE))
                    .inner_margin(Margin::symmetric(8, 6))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            for kind in TicketOrderKind::ALL {
                                if ui
                                    .selectable_label(self.ticket_kind == kind, kind.label())
                                    .clicked()
                                {
                                    self.ticket_kind = kind;
                                    if kind != TicketOrderKind::Market
                                        && self.ticket_price_input.trim().is_empty()
                                        && mark > 0.0
                                    {
                                        self.ticket_price_input = format!("{mark:.2}");
                                    }
                                }
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("Qty:")
                                    .monospace()
                                    .size(10.5)
                                    .color(palette::TEXT_MUTED),
                            );
                            ui.add(
                                egui::TextEdit::singleline(&mut self.ticket_qty_input)
                                    .desired_width(68.0)
                                    .font(egui::FontId::monospace(11.0)),
                            );
                            ui.label(
                                RichText::new(format!("Mark {mark:.2}"))
                                    .monospace()
                                    .size(10.5)
                                    .color(palette::TEXT_MUTED),
                            );
                        });

                        if self.ticket_kind != TicketOrderKind::Market {
                            ui.horizontal(|ui| {
                                let lbl = match self.ticket_kind {
                                    TicketOrderKind::Limit => "Lmt Px:",
                                    TicketOrderKind::Stop => "Stp Px:",
                                    TicketOrderKind::Market => "Px:",
                                };
                                ui.label(
                                    RichText::new(lbl)
                                        .monospace()
                                        .size(10.5)
                                        .color(palette::TEXT_MUTED),
                                );
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.ticket_price_input)
                                        .desired_width(88.0)
                                        .font(egui::FontId::monospace(11.0)),
                                );
                                if ui
                                    .button(RichText::new("@Mark").monospace().size(10.0))
                                    .clicked()
                                    && mark > 0.0
                                {
                                    self.ticket_price_input = format!("{mark:.2}");
                                }
                            });
                        }

                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("SL:")
                                    .monospace()
                                    .size(10.5)
                                    .color(palette::TRADE_STOP_LOSS),
                            );
                            ui.add(
                                egui::TextEdit::singleline(&mut self.ticket_sl_input)
                                    .desired_width(68.0)
                                    .font(egui::FontId::monospace(11.0)),
                            );
                            ui.label(
                                RichText::new("TP:")
                                    .monospace()
                                    .size(10.5)
                                    .color(palette::TRADE_TAKE_PROFIT),
                            );
                            ui.add(
                                egui::TextEdit::singleline(&mut self.ticket_tp_input)
                                    .desired_width(68.0)
                                    .font(egui::FontId::monospace(11.0)),
                            );
                        });

                        let can_sync = self
                            .drawings
                            .selected_drawing()
                            .is_some_and(|d| d.pane == DrawingPaneTarget::Main);
                        if ui
                            .add_enabled(
                                can_sync,
                                Button::new(
                                    RichText::new("Sync Selected Drawing")
                                        .monospace()
                                        .size(10.5),
                                ),
                            )
                            .clicked()
                        {
                            let _ = self.sync_ticket_from_selected_drawing();
                        }

                        ui.horizontal(|ui| {
                            let half_w = ((ui.available_width() - 6.0) * 0.5).max(72.0);
                            let buy_btn = Button::new(
                                RichText::new(format!("BUY {}", self.ticket_kind.label()))
                                    .monospace()
                                    .strong()
                                    .size(11.0)
                                    .color(Color32::WHITE),
                            )
                            .fill(Color32::from_rgb(16, 148, 108))
                            .min_size(Vec2::new(half_w, 26.0));

                            let sell_btn = Button::new(
                                RichText::new(format!("SELL {}", self.ticket_kind.label()))
                                    .monospace()
                                    .strong()
                                    .size(11.0)
                                    .color(Color32::WHITE),
                            )
                            .fill(Color32::from_rgb(196, 52, 68))
                            .min_size(Vec2::new(half_w, 26.0));

                            if ui.add(buy_btn).clicked() {
                                let _ = self.submit_ticket_order(OrderSide::Buy);
                            }
                            if ui.add(sell_btn).clicked() {
                                let _ = self.submit_ticket_order(OrderSide::Sell);
                            }
                        });

                        if let Some((is_err, ref msg)) = self.last_trade_status {
                            ui.label(RichText::new(msg).monospace().size(10.0).color(if is_err {
                                palette::BEARISH
                            } else {
                                palette::BULLISH
                            }));
                        }
                    });

                // 3. Open Position Card
                Frame::new()
                    .fill(Color32::from_rgb(19, 24, 35))
                    .corner_radius(CornerRadius::same(4))
                    .stroke(Stroke::new(1.0, palette::BORDER_SUBTLE))
                    .inner_margin(Margin::symmetric(8, 6))
                    .show(ui, |ui| {
                        if let Some(pos) = self.trading.open_position().cloned() {
                            let side_color = match pos.side {
                                PositionSide::Long => palette::BULLISH,
                                PositionSide::Short => palette::BEARISH,
                            };
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(format!(
                                        "{} {:.4} @ {:.2}",
                                        pos.side.short_label(),
                                        pos.quantity,
                                        pos.avg_entry_price
                                    ))
                                    .monospace()
                                    .strong()
                                    .size(11.0)
                                    .color(side_color),
                                );
                            });
                            let upnl = pos.unrealized_pnl(mark);
                            let upct = pos.unrealized_return_pct(mark);
                            let u_color = if upnl >= 0.0 {
                                palette::BULLISH
                            } else {
                                palette::BEARISH
                            };
                            ui.label(
                                RichText::new(format!("uPnL: {upnl:+.2} ({upct:+.2}%)"))
                                    .monospace()
                                    .strong()
                                    .size(11.0)
                                    .color(u_color),
                            );
                            let sl_lbl = pos
                                .stop_loss
                                .map_or_else(|| "-".to_owned(), |v| format!("{v:.2}"));
                            let tp_lbl = pos
                                .take_profit
                                .map_or_else(|| "-".to_owned(), |v| format!("{v:.2}"));
                            ui.label(
                                RichText::new(format!("SL {sl_lbl} · TP {tp_lbl}"))
                                    .monospace()
                                    .size(10.5)
                                    .color(palette::TEXT_MUTED),
                            );
                            ui.horizontal(|ui| {
                                if ui
                                    .button(RichText::new("Apply SL/TP").monospace().size(10.5))
                                    .clicked()
                                {
                                    let _ = self.apply_ticket_brackets_to_position();
                                }
                                if ui
                                    .button(
                                        RichText::new("Flatten")
                                            .monospace()
                                            .strong()
                                            .size(10.5)
                                            .color(palette::BEARISH),
                                    )
                                    .clicked()
                                {
                                    let _ = self.flatten_open_position();
                                }
                            });
                        } else {
                            ui.label(
                                RichText::new("Position: FLAT")
                                    .monospace()
                                    .size(10.5)
                                    .color(palette::TEXT_MUTED),
                            );
                        }
                    });

                // 4. Working Orders (`Pending`)
                let pending_snapshot: Vec<_> = self.trading.pending_orders().cloned().collect();
                if !pending_snapshot.is_empty() {
                    let mut cancel_req: Option<u64> = None;
                    Frame::new()
                        .fill(Color32::from_rgb(19, 24, 35))
                        .corner_radius(CornerRadius::same(4))
                        .stroke(Stroke::new(1.0, palette::BORDER_SUBTLE))
                        .inner_margin(Margin::symmetric(8, 6))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(format!(
                                    "WORKING ORDERS ({})",
                                    pending_snapshot.len()
                                ))
                                .monospace()
                                .strong()
                                .size(10.5)
                                .color(palette::REPLAY_CUT_GOLD),
                            );
                            for ord in &pending_snapshot {
                                ui.horizontal(|ui| {
                                    let px = ord.kind.trigger_price().unwrap_or(0.0);
                                    ui.label(
                                        RichText::new(format!(
                                            "#{} {} {} {:.2} @ {:.2}",
                                            ord.id,
                                            ord.side.short_label(),
                                            ord.kind.short_label(),
                                            ord.quantity,
                                            px
                                        ))
                                        .monospace()
                                        .size(10.0)
                                        .color(palette::TEXT_PRIMARY),
                                    );
                                    if ui
                                        .button(RichText::new("×").monospace().size(10.0))
                                        .clicked()
                                    {
                                        cancel_req = Some(ord.id);
                                    }
                                });
                            }
                        });
                    if let Some(id) = cancel_req {
                        let _ = self.cancel_working_order(id);
                    }
                }

                // 5. Execution Fill & Closed Trade Blotter
                if !self.trading.fills().is_empty() {
                    Frame::new()
                        .fill(Color32::from_rgb(19, 24, 35))
                        .corner_radius(CornerRadius::same(4))
                        .stroke(Stroke::new(1.0, palette::BORDER_SUBTLE))
                        .inner_margin(Margin::symmetric(8, 6))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(format!(
                                    "EXECUTION BLOTTER ({} fills)",
                                    self.trading.fills().len()
                                ))
                                .monospace()
                                .strong()
                                .size(10.5)
                                .color(palette::TEXT_MUTED),
                            );
                            for fill in self.trading.fills().iter().rev().take(6) {
                                let c = match fill.side {
                                    OrderSide::Buy => palette::BULLISH,
                                    OrderSide::Sell => palette::BEARISH,
                                };
                                let pnl_suffix = fill
                                    .realized_net_pnl
                                    .map_or_else(String::new, |p| format!(" ({p:+.1})"));
                                ui.label(
                                    RichText::new(format!(
                                        "#{} {} {} {:.2}@{:.2}{pnl_suffix}",
                                        fill.fill_id,
                                        fill.side.short_label(),
                                        fill.reason.short_label(),
                                        fill.quantity,
                                        fill.price
                                    ))
                                    .monospace()
                                    .size(9.8)
                                    .color(c),
                                );
                            }
                        });
                }
            });
    }

    fn render_scanner_dock(&mut self, ui: &mut egui::Ui) {
        self.sync_scanner_caches();
        let active_key = self.current_instrument_key();
        let rows = filter_and_sort_watchlist(
            &self.raw_watchlist_rows_cache,
            &self.scanner_filter,
            self.watchlist_sort_col,
            self.watchlist_sort_asc,
        );
        let hits = filter_historical_signals(&self.raw_historical_hits_cache, &self.scanner_filter);

        ui.spacing_mut().item_spacing = Vec2::new(5.0, 6.0);

        // 1. Header & Sub-Tab Switcher
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("WATCHLIST & SCANNER")
                    .monospace()
                    .strong()
                    .size(11.5)
                    .color(palette::TEXT_PRIMARY),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(RichText::new("×").monospace().size(11.0))
                    .clicked()
                {
                    self.show_scanner_dock = false;
                }
            });
        });

        ui.horizontal(|ui| {
            let sp_x = ui.spacing().item_spacing.x;
            let half_w = ((ui.available_width() - sp_x - 2.0) * 0.5).max(68.0);
            for (tab, lbl) in [
                (
                    ScannerDockTab::Watchlist,
                    format!("Watchlist ({})", rows.len()),
                ),
                (ScannerDockTab::Signals, format!("Signals ({})", hits.len())),
            ] {
                let selected = self.scanner_tab == tab;
                let fill = if selected {
                    palette::ACCENT_BLUE.gamma_multiply(0.26)
                } else {
                    Color32::from_rgb(22, 27, 38)
                };
                let stroke = if selected {
                    Stroke::new(1.0, palette::ACCENT_BLUE)
                } else {
                    Stroke::new(1.0, palette::BORDER_SUBTLE)
                };
                let btn = Button::new(RichText::new(lbl).monospace().strong().size(10.5).color(
                    if selected {
                        Color32::WHITE
                    } else {
                        palette::TEXT_MUTED
                    },
                ))
                .fill(fill)
                .stroke(stroke)
                .min_size(Vec2::new(half_w, 22.0));
                if ui.add(btn).clicked() {
                    self.scanner_tab = tab;
                }
            }
        });

        // 2. Shared Search & Filter Card
        Frame::new()
            .fill(Color32::from_rgb(19, 24, 35))
            .corner_radius(CornerRadius::same(4))
            .stroke(Stroke::new(1.0, palette::BORDER_SUBTLE))
            .inner_margin(Margin::symmetric(6, 5))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(4.0, 4.0);
                ui.horizontal(|ui| {
                    let has_filter = !self.scanner_filter.search_query.is_empty()
                        || self.scanner_filter.signal_filter.is_some()
                        || self.scanner_filter.regime_filter.is_some()
                        || self.scanner_filter.direction_filter.is_some()
                        || self.scanner_filter.only_active_signals;
                    let edit_w =
                        (ui.available_width() - if has_filter { 38.0 } else { 4.0 }).max(90.0);
                    ui.add(
                        egui::TextEdit::singleline(&mut self.scanner_filter.search_query)
                            .desired_width(edit_w)
                            .hint_text("Search symbol / regime / sig")
                            .font(egui::FontId::monospace(10.5)),
                    );
                    truncate_utf8_bytes(&mut self.scanner_filter.search_query, 64);
                    if has_filter
                        && ui
                            .button(RichText::new("Clr").monospace().size(9.5))
                            .clicked()
                    {
                        self.scanner_filter = ScannerFilter::default();
                    }
                });

                ui.horizontal_wrapped(|ui| {
                    if self.scanner_tab == ScannerDockTab::Watchlist {
                        let _ = indicator_toggle_pill(
                            ui,
                            &mut self.scanner_filter.only_active_signals,
                            "Sig Only",
                            palette::ALERT_ARMED_AMBER,
                        );
                    }
                    for (dir_opt, lbl) in [
                        (None, "All"),
                        (Some(SignalDirection::Bullish), "Bull"),
                        (Some(SignalDirection::Bearish), "Bear"),
                    ] {
                        if ui
                            .selectable_label(
                                self.scanner_filter.direction_filter == dir_opt,
                                RichText::new(lbl).monospace().size(10.0),
                            )
                            .clicked()
                        {
                            self.scanner_filter.direction_filter = dir_opt;
                        }
                    }

                    if self.scanner_tab == ScannerDockTab::Signals {
                        ui.separator();
                        for lb in [30_usize, 80, 180] {
                            if ui
                                .selectable_label(
                                    self.signal_lookback_bars == lb,
                                    RichText::new(format!("{lb}B")).monospace().size(10.0),
                                )
                                .clicked()
                            {
                                self.signal_lookback_bars = lb;
                            }
                        }
                    }
                });

                ui.horizontal(|ui| {
                    let reg_lbl = self
                        .scanner_filter
                        .regime_filter
                        .map_or("Regime: All", MarketRegime::short_label);
                    egui::ComboBox::from_id_salt("scanner_regime_combo")
                        .width(102.0)
                        .selected_text(RichText::new(reg_lbl).monospace().size(10.0))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.scanner_filter.regime_filter,
                                None,
                                "All Regimes",
                            );
                            for reg in MarketRegime::ALL_FILTERABLE {
                                ui.selectable_value(
                                    &mut self.scanner_filter.regime_filter,
                                    Some(reg),
                                    format!("{} ({})", reg.full_label(), reg.short_label()),
                                );
                            }
                        });

                    let sig_lbl = self
                        .scanner_filter
                        .signal_filter
                        .map_or("Sig: All", ScanSignalKind::short_label);
                    egui::ComboBox::from_id_salt("scanner_sig_combo")
                        .width(102.0)
                        .selected_text(RichText::new(sig_lbl).monospace().size(10.0))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.scanner_filter.signal_filter,
                                None,
                                "All Signals",
                            );
                            for sig in ScanSignalKind::ALL {
                                ui.selectable_value(
                                    &mut self.scanner_filter.signal_filter,
                                    Some(sig),
                                    format!("{} — {}", sig.short_label(), sig.full_label()),
                                );
                            }
                        });
                });
            });

        // 3. Sub-Tab Body
        match self.scanner_tab {
            ScannerDockTab::Watchlist => {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(3.0, 2.0);
                    for col in WatchlistSortColumn::ALL {
                        let is_col = self.watchlist_sort_col == col;
                        let arrow = if !is_col {
                            ""
                        } else if self.watchlist_sort_asc {
                            "▲"
                        } else {
                            "▼"
                        };
                        let text = format!("{}{arrow}", col.short_label());
                        if ui
                            .selectable_label(
                                is_col,
                                RichText::new(text).monospace().size(9.8).color(if is_col {
                                    Color32::WHITE
                                } else {
                                    palette::TEXT_MUTED
                                }),
                            )
                            .clicked()
                        {
                            if is_col {
                                self.watchlist_sort_asc = !self.watchlist_sort_asc;
                            } else {
                                self.watchlist_sort_col = col;
                                self.watchlist_sort_asc = col == WatchlistSortColumn::Symbol;
                            }
                        }
                    }
                });

                let mut clicked_instrument: Option<(DataSourcePreset, String)> = None;
                egui::ScrollArea::vertical()
                    .id_salt("sumi_watchlist_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(4.0, 4.0);
                        for row in &rows {
                            let is_active = row.instrument_id == active_key;
                            let card_fill = if is_active {
                                Color32::from_rgb(25, 36, 56)
                            } else {
                                Color32::from_rgb(18, 22, 32)
                            };
                            let card_stroke = if is_active {
                                Stroke::new(1.0, palette::ACCENT_BLUE)
                            } else {
                                Stroke::new(1.0, palette::BORDER_SUBTLE)
                            };

                            let inner = Frame::new()
                                .fill(card_fill)
                                .corner_radius(CornerRadius::same(4))
                                .stroke(card_stroke)
                                .inner_margin(Margin::symmetric(7, 5))
                                .show(ui, |ui| {
                                    ui.set_min_width(ui.available_width());
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(&row.symbol)
                                                .monospace()
                                                .strong()
                                                .size(11.0)
                                                .color(if is_active {
                                                    Color32::from_rgb(100, 198, 255)
                                                } else {
                                                    palette::TEXT_PRIMARY
                                                }),
                                        );
                                        ui.label(
                                            RichText::new(row.timeframe.label())
                                                .monospace()
                                                .size(9.5)
                                                .color(palette::TEXT_MUTED),
                                        );
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                let chg_color = if row.change_pct > 1e-6 {
                                                    palette::BULLISH
                                                } else if row.change_pct < -1e-6 {
                                                    palette::BEARISH
                                                } else {
                                                    palette::TEXT_MUTED
                                                };
                                                ui.label(
                                                    RichText::new(format!(
                                                        "{:+.2}%",
                                                        row.change_pct
                                                    ))
                                                    .monospace()
                                                    .strong()
                                                    .size(10.2)
                                                    .color(chg_color),
                                                );
                                                ui.label(
                                                    RichText::new(format_scanner_price(
                                                        row.last_close,
                                                    ))
                                                    .monospace()
                                                    .strong()
                                                    .size(10.5)
                                                    .color(palette::TEXT_PRIMARY),
                                                );
                                            },
                                        );
                                    });

                                    ui.horizontal(|ui| {
                                        let reg_c = regime_badge_color(row.regime);
                                        ui.label(
                                            RichText::new(row.regime.short_label())
                                                .monospace()
                                                .strong()
                                                .size(9.5)
                                                .color(reg_c),
                                        );
                                        let rvol_s = row.rvol_20.map_or_else(
                                            || "RV -".to_owned(),
                                            |v| format!("RV {v:.1}x"),
                                        );
                                        let rsi_s = row.rsi_14.map_or_else(
                                            || "RSI -".to_owned(),
                                            |v| format!("RSI {v:.0}"),
                                        );
                                        ui.label(
                                            RichText::new(format!("{rvol_s} · {rsi_s}"))
                                                .monospace()
                                                .size(9.5)
                                                .color(palette::TEXT_MUTED),
                                        );
                                        if let Some(first_sig) = row.active_signals.first().copied()
                                        {
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    let sig_c = match first_sig.direction() {
                                                        SignalDirection::Bullish => {
                                                            palette::BULLISH
                                                        }
                                                        SignalDirection::Bearish => {
                                                            palette::BEARISH
                                                        }
                                                    };
                                                    let extra = if row.active_signals.len() > 1 {
                                                        format!(
                                                            "{} +{}",
                                                            first_sig.short_label(),
                                                            row.active_signals.len() - 1
                                                        )
                                                    } else {
                                                        first_sig.short_label().to_owned()
                                                    };
                                                    ui.label(
                                                        RichText::new(extra)
                                                            .monospace()
                                                            .strong()
                                                            .size(9.5)
                                                            .color(sig_c),
                                                    );
                                                },
                                            );
                                        }
                                    });
                                });

                            let click_resp = ui.interact(
                                inner.response.rect,
                                ui.make_persistent_id(("wl_row_click", &row.instrument_id)),
                                egui::Sense::click(),
                            );
                            if click_resp.hovered() {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            }
                            if click_resp.clicked() {
                                if let Some(entry) = self
                                    .watchlist_universe
                                    .iter()
                                    .find(|e| e.instrument_id == row.instrument_id)
                                {
                                    clicked_instrument = Some((entry.preset, entry.symbol.clone()));
                                } else if let Some(entry) = self
                                    .custom_peer_feeds
                                    .iter()
                                    .find(|e| e.instrument_id == row.instrument_id)
                                {
                                    clicked_instrument =
                                        Some((DataSourcePreset::CustomFile, entry.symbol.clone()));
                                } else if row.instrument_id.starts_with("Preset:Synth:") {
                                    clicked_instrument = Some((
                                        DataSourcePreset::SyntheticVnindex,
                                        row.symbol.clone(),
                                    ));
                                }
                            }
                        }
                    });

                if let Some((preset, sym)) = clicked_instrument {
                    self.select_watchlist_instrument(preset, &sym);
                }
            }
            ScannerDockTab::Signals => {
                let mut launch_req: Option<(HistoricalSignalHit, ScannerReplayIntent)> = None;
                egui::ScrollArea::vertical()
                    .id_salt("sumi_signals_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(4.0, 5.0);
                        if hits.is_empty() {
                            ui.label(
                                RichText::new("No causal signals match current filter.")
                                    .monospace()
                                    .size(10.0)
                                    .color(palette::TEXT_MUTED),
                            );
                        }
                        for hit in hits.iter().take(64) {
                            let sig_color = match hit.direction {
                                SignalDirection::Bullish => palette::BULLISH,
                                SignalDirection::Bearish => palette::BEARISH,
                            };
                            Frame::new()
                                .fill(Color32::from_rgb(18, 22, 32))
                                .corner_radius(CornerRadius::same(4))
                                .stroke(Stroke::new(1.0, palette::BORDER_SUBTLE))
                                .inner_margin(Margin::symmetric(7, 5))
                                .show(ui, |ui| {
                                    ui.set_min_width(ui.available_width());
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(format!(
                                                "{} · {}",
                                                hit.symbol,
                                                hit.timeframe.label()
                                            ))
                                            .monospace()
                                            .strong()
                                            .size(10.8)
                                            .color(palette::TEXT_PRIMARY),
                                        );
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                ui.label(
                                                    RichText::new(format!(
                                                        "{} · {}",
                                                        hit.signal.short_label(),
                                                        hit.regime.short_label()
                                                    ))
                                                    .monospace()
                                                    .strong()
                                                    .size(10.0)
                                                    .color(sig_color),
                                                );
                                            },
                                        );
                                    });

                                    let ts_lbl = if hit.timeframe.is_intraday() {
                                        format_utc_datetime_hm(hit.timestamp)
                                    } else {
                                        format_utc_date(hit.timestamp)
                                    };
                                    ui.label(
                                        RichText::new(format!(
                                            "@ {} · {ts_lbl}",
                                            format_scanner_price(hit.price)
                                        ))
                                        .monospace()
                                        .size(9.6)
                                        .color(palette::TEXT_MUTED),
                                    );

                                    ui.horizontal(|ui| {
                                        let sp_x = ui.spacing().item_spacing.x;
                                        let btn_w =
                                            ((ui.available_width() - sp_x - 2.0) * 0.5).max(60.0);
                                        let blind_btn = Button::new(
                                            RichText::new("Blind Practice")
                                                .monospace()
                                                .strong()
                                                .size(9.8)
                                                .color(palette::ALERT_ARMED_AMBER),
                                        )
                                        .fill(palette::ALERT_ARMED_AMBER.gamma_multiply(0.16))
                                        .stroke(Stroke::new(
                                            1.0,
                                            palette::ALERT_ARMED_AMBER.gamma_multiply(0.75),
                                        ))
                                        .min_size(Vec2::new(btn_w, 20.0));

                                        let review_btn = Button::new(
                                            RichText::new("Signal Review")
                                                .monospace()
                                                .strong()
                                                .size(9.8)
                                                .color(Color32::from_rgb(120, 200, 255)),
                                        )
                                        .fill(palette::ACCENT_BLUE.gamma_multiply(0.18))
                                        .stroke(Stroke::new(
                                            1.0,
                                            palette::ACCENT_BLUE.gamma_multiply(0.75),
                                        ))
                                        .min_size(Vec2::new(btn_w, 20.0));

                                        if ui.add(blind_btn).clicked() {
                                            launch_req = Some((
                                                hit.clone(),
                                                ScannerReplayIntent::BlindPractice,
                                            ));
                                        }
                                        if ui.add(review_btn).clicked() {
                                            launch_req = Some((
                                                hit.clone(),
                                                ScannerReplayIntent::SignalReview,
                                            ));
                                        }
                                    });
                                });
                        }
                    });

                if let Some((hit, intent)) = launch_req {
                    let _ = self.start_scanner_signal_replay(hit, intent);
                }
            }
        }
    }

    fn render_right_dock(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing = Vec2::new(5.0, 6.0);
        let cutoff = self.replay.cutoff_timestamp(&self.base_candles);
        let armed_n = self.alerts.armed_count(cutoff);
        let vis_rules = self
            .alerts
            .rules
            .iter()
            .filter(|r| r.is_causally_visible(cutoff))
            .count();
        let j_vis = self
            .journal
            .visible_entries(self.replay_causal_close_timestamp())
            .len();

        ui.horizontal(|ui| {
            let sp_x = ui.spacing().item_spacing.x;
            let tab_w = ((ui.available_width() - 3.0 * sp_x - 2.0) / 4.0).max(46.0);
            for (tab, lbl, accent) in [
                (
                    RightDockTab::Trading,
                    "Trade".to_owned(),
                    palette::TRADE_ENTRY_BLUE,
                ),
                (
                    RightDockTab::Alerts,
                    if vis_rules == 0 {
                        "Alerts".to_owned()
                    } else {
                        format!("Alt({armed_n}/{vis_rules})")
                    },
                    palette::ALERT_ARMED_AMBER,
                ),
                (
                    RightDockTab::StrategyLab,
                    "Strat".to_owned(),
                    palette::EQUITY_ABOVE_LINE,
                ),
                (
                    RightDockTab::Journal,
                    if j_vis == 0 {
                        "Risk/Jrn".to_owned()
                    } else {
                        format!("Jrn({j_vis})")
                    },
                    Color32::from_rgb(16, 185, 129),
                ),
            ] {
                let selected = self.right_dock_tab == tab;
                let btn = Button::new(RichText::new(lbl).monospace().strong().size(9.8).color(
                    if selected {
                        Color32::WHITE
                    } else {
                        palette::TEXT_MUTED
                    },
                ))
                .fill(if selected {
                    accent.gamma_multiply(0.24)
                } else {
                    Color32::from_rgb(22, 27, 38)
                })
                .stroke(Stroke::new(
                    1.0,
                    if selected {
                        accent
                    } else {
                        palette::BORDER_SUBTLE
                    },
                ))
                .min_size(Vec2::new(tab_w, 22.0));

                if ui.add(btn).clicked() {
                    self.right_dock_tab = tab;
                    match tab {
                        RightDockTab::Trading => self.show_trading_panel = true,
                        RightDockTab::Alerts => self.show_alerts_panel = true,
                        RightDockTab::StrategyLab => {
                            self.show_strategy_panel = true;
                            self.sync_backtest_causal();
                        }
                        RightDockTab::Journal => {
                            self.open_journal_tab();
                        }
                    }
                }
            }
        });
        ui.separator();

        match self.right_dock_tab {
            RightDockTab::Trading => self.render_paper_trading_panel(ui),
            RightDockTab::Alerts => self.render_alerts_panel(ui),
            RightDockTab::StrategyLab => self.render_strategy_lab_panel(ui),
            RightDockTab::Journal => self.render_journal_panel(ui),
        }
    }

    fn render_strategy_lab_panel(&mut self, ui: &mut egui::Ui) {
        self.sync_backtest_causal();
        let pre_cfg_digest = compute_backtest_config_digest(&self.backtest_config);
        let pre_wf_digest = compute_walk_forward_config_digest(&self.wf_config);
        let pre_ranking = self.sweep_ranking;
        let pre_sub_tab = self.strategy_lab_tab;
        let pre_port_filter = self.portfolio_universe_filter;
        let pre_port_wt = self.portfolio_weighting;
        let pre_eq_src = self.equity_pane_source;

        let mut focus_trade_req: Option<u64> = None;
        let mut replay_trade_req: Option<u64> = None;
        let mut apply_sweep_cfg: Option<BacktestConfig> = None;
        let mut apply_preset_req: Option<StrategyPresetKind> = None;
        let mut select_symbol_req: Option<(DataSourcePreset, String)> = None;
        let mut force_sweep = false;
        let mut force_battle = false;
        let mut force_wfo = false;
        let mut force_portfolio = false;

        egui::ScrollArea::vertical()
            .id_salt("sumi_strategy_lab_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(4.0, 5.0);

                // Header Row + Causal Mode Badge + Chart Sync Toggles
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("STRATEGY LAB")
                            .monospace()
                            .strong()
                            .size(11.5)
                            .color(palette::TEXT_PRIMARY),
                    );
                    let (mode_lbl, mode_col) = if self.replay.is_active() {
                        ("RPL CAUSAL", palette::REPLAY_CUT_GOLD)
                    } else {
                        ("LIVE CAUSAL", palette::EQUITY_ABOVE_LINE)
                    };
                    ui.label(
                        RichText::new(mode_lbl)
                            .monospace()
                            .strong()
                            .size(9.5)
                            .color(mode_col),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let _ = indicator_toggle_pill(
                            ui,
                            &mut self.show_backtest_trades,
                            "Links",
                            palette::TRADE_LINK_WIN,
                        );
                        if indicator_toggle_pill(
                            ui,
                            &mut self.show_equity_pane,
                            "EQ",
                            palette::EQUITY_ABOVE_LINE,
                        ) && self.show_equity_pane
                        {
                            self.viewport.equity_price_scale.reset_auto_scale();
                        }
                    });
                });

                // Strategy Configuration Card
                Frame::new()
                    .fill(Color32::from_rgb(17, 21, 30))
                    .stroke(Stroke::new(1.0, palette::BORDER_SUBTLE))
                    .corner_radius(4.0)
                    .inner_margin(Margin::symmetric(7, 6))
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(4.0, 4.0);

                        egui::ComboBox::from_id_salt("strategy_preset_combo")
                            .selected_text(
                                RichText::new(self.backtest_config.preset.full_label())
                                    .monospace()
                                    .strong()
                                    .size(10.5)
                                    .color(palette::TEXT_PRIMARY),
                            )
                            .width((ui.available_width() - 4.0).max(160.0))
                            .show_ui(ui, |ui| {
                                for preset in StrategyPresetKind::ALL {
                                    let is_sel = self.backtest_config.preset == preset;
                                    if ui.selectable_label(is_sel, preset.full_label()).clicked() {
                                        self.backtest_config.preset = preset;
                                    }
                                }
                            });

                        ui.label(
                            RichText::new(self.backtest_config.preset.summary_rule())
                                .monospace()
                                .size(9.2)
                                .color(palette::TEXT_MUTED),
                        );

                        // Preset-specific compact parameters
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(4.0, 3.0);
                            match self.backtest_config.preset {
                                StrategyPresetKind::SmaEmaTrendCross => {
                                    ui.label(RichText::new("SMA:").monospace().size(9.8));
                                    ui.add(
                                        egui::DragValue::new(
                                            &mut self.backtest_config.params.fast_sma,
                                        )
                                        .range(2..=199)
                                        .speed(1),
                                    );
                                    ui.label(RichText::new("EMA:").monospace().size(9.8));
                                    ui.add(
                                        egui::DragValue::new(
                                            &mut self.backtest_config.params.slow_ema,
                                        )
                                        .range(3..=250)
                                        .speed(1),
                                    );
                                }
                                StrategyPresetKind::RsiMeanReversion => {
                                    ui.label(RichText::new("P:").monospace().size(9.8));
                                    ui.add(
                                        egui::DragValue::new(
                                            &mut self.backtest_config.params.rsi_period,
                                        )
                                        .range(2..=100),
                                    );
                                    ui.label(RichText::new("OS:").monospace().size(9.8));
                                    ui.add(
                                        egui::DragValue::new(
                                            &mut self.backtest_config.params.rsi_oversold,
                                        )
                                        .range(5.0..=48.0)
                                        .speed(0.5),
                                    );
                                    ui.label(RichText::new("OB:").monospace().size(9.8));
                                    ui.add(
                                        egui::DragValue::new(
                                            &mut self.backtest_config.params.rsi_overbought,
                                        )
                                        .range(52.0..=95.0)
                                        .speed(0.5),
                                    );
                                }
                                StrategyPresetKind::MacdMomentum => {
                                    ui.label(RichText::new("F/S/Sg:").monospace().size(9.6));
                                    ui.add(
                                        egui::DragValue::new(
                                            &mut self.backtest_config.params.macd_fast,
                                        )
                                        .range(2..=80),
                                    );
                                    ui.add(
                                        egui::DragValue::new(
                                            &mut self.backtest_config.params.macd_slow,
                                        )
                                        .range(3..=160),
                                    );
                                    ui.add(
                                        egui::DragValue::new(
                                            &mut self.backtest_config.params.macd_signal,
                                        )
                                        .range(2..=60),
                                    );
                                    ui.label(RichText::new("EMA:").monospace().size(9.6));
                                    ui.add(
                                        egui::DragValue::new(
                                            &mut self.backtest_config.params.macd_trend_ema,
                                        )
                                        .range(0..=200),
                                    );
                                }
                                StrategyPresetKind::BollingerSqueezeBreakout => {
                                    ui.label(RichText::new("BB:").monospace().size(9.8));
                                    ui.add(
                                        egui::DragValue::new(
                                            &mut self.backtest_config.params.bb_period,
                                        )
                                        .range(5..=100),
                                    );
                                    ui.label(RichText::new("SD:").monospace().size(9.8));
                                    ui.add(
                                        egui::DragValue::new(
                                            &mut self.backtest_config.params.bb_std_mult,
                                        )
                                        .range(0.5..=4.0)
                                        .speed(0.1),
                                    );
                                    ui.label(RichText::new("Sqz%:").monospace().size(9.8));
                                    ui.add(
                                        egui::DragValue::new(
                                            &mut self.backtest_config.params.max_squeeze_bw_pct,
                                        )
                                        .range(1.0..=60.0)
                                        .speed(0.5),
                                    );
                                }
                                StrategyPresetKind::DonchianAtrBreakout => {
                                    ui.label(RichText::new("In:").monospace().size(9.8));
                                    ui.add(
                                        egui::DragValue::new(
                                            &mut self.backtest_config.params.donchian_entry_period,
                                        )
                                        .range(3..=160),
                                    );
                                    ui.label(RichText::new("Out:").monospace().size(9.8));
                                    ui.add(
                                        egui::DragValue::new(
                                            &mut self.backtest_config.params.donchian_exit_period,
                                        )
                                        .range(2..=120),
                                    );
                                    ui.label(RichText::new("ATR:").monospace().size(9.8));
                                    ui.add(
                                        egui::DragValue::new(
                                            &mut self.backtest_config.params.atr_period,
                                        )
                                        .range(2..=80),
                                    );
                                }
                                StrategyPresetKind::RsiBollingerConfluence => {
                                    ui.label(RichText::new("RSI:").monospace().size(9.8));
                                    ui.add(
                                        egui::DragValue::new(
                                            &mut self.backtest_config.params.rsi_period,
                                        )
                                        .range(2..=80),
                                    );
                                    ui.label(RichText::new("B/S:").monospace().size(9.8));
                                    ui.add(
                                        egui::DragValue::new(
                                            &mut self.backtest_config.params.rsi_confluence_buy,
                                        )
                                        .range(5.0..=49.0)
                                        .speed(0.5),
                                    );
                                    ui.add(
                                        egui::DragValue::new(
                                            &mut self.backtest_config.params.rsi_confluence_sell,
                                        )
                                        .range(51.0..=95.0)
                                        .speed(0.5),
                                    );
                                }
                            }
                        });

                        // Direction + Sizing + VN T+2 Spot Preset
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(4.0, 3.0);
                            for dir in StrategyDirectionMode::ALL {
                                let sel = self.backtest_config.direction_mode == dir;
                                if ui
                                    .selectable_label(
                                        sel,
                                        RichText::new(dir.short_label()).monospace().size(9.8),
                                    )
                                    .clicked()
                                {
                                    self.backtest_config.direction_mode = dir;
                                }
                            }
                            ui.separator();
                            let is_pct_eq = matches!(
                                self.backtest_config.sizing,
                                PositionSizingMode::PercentEquity(_)
                            );
                            let is_risk_sz = matches!(
                                self.backtest_config.sizing,
                                PositionSizingMode::RiskPercentPerTrade(_)
                            );
                            let pct_eq_lbl = match self.backtest_config.sizing {
                                PositionSizingMode::PercentEquity(p)
                                    if (p - 95.0).abs() > 0.05 =>
                                {
                                    format!("{p:.0}%Eq")
                                }
                                PositionSizingMode::FixedNotional(n) => {
                                    if n >= 1000.0 {
                                        format!("${:.0}k", n / 1000.0)
                                    } else {
                                        format!("${n:.0}")
                                    }
                                }
                                _ => "95%Eq".to_owned(),
                            };
                            if ui
                                .selectable_label(
                                    is_pct_eq,
                                    RichText::new(pct_eq_lbl).monospace().size(9.6),
                                )
                                .clicked()
                            {
                                self.backtest_config.sizing =
                                    PositionSizingMode::PercentEquity(95.0);
                            }
                            if ui
                                .selectable_label(
                                    is_risk_sz,
                                    RichText::new("1.5%R").monospace().size(9.6),
                                )
                                .clicked()
                            {
                                self.backtest_config.sizing =
                                    PositionSizingMode::RiskPercentPerTrade(1.5);
                            }
                            let mut vn_spot = self.backtest_config.risk.min_holding_bars >= 2
                                && self.backtest_config.tax_sell_bps >= 10.0;
                            if indicator_toggle_pill(
                                ui,
                                &mut vn_spot,
                                "VN T+2",
                                Color32::from_rgb(255, 183, 77),
                            ) {
                                if vn_spot {
                                    self.backtest_config.direction_mode =
                                        StrategyDirectionMode::LongOnly;
                                    self.backtest_config.risk.min_holding_bars = 2;
                                    self.backtest_config.tax_sell_bps = 10.0;
                                    self.backtest_config.lot_step = 100.0;
                                } else {
                                    self.backtest_config.risk.min_holding_bars = 0;
                                    self.backtest_config.tax_sell_bps = 0.0;
                                    self.backtest_config.lot_step = 0.0;
                                }
                            }
                        });

                        // SL / TP / ATR Trail / OOS Split Row
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(4.0, 3.0);

                            let mut sl_on = self.backtest_config.risk.stop_loss_pct.is_some();
                            if ui
                                .checkbox(
                                    &mut sl_on,
                                    RichText::new("SL%").monospace().size(9.5),
                                )
                                .changed()
                            {
                                self.backtest_config.risk.stop_loss_pct =
                                    sl_on.then_some(5.0);
                            }
                            if let Some(ref mut sl_v) = self.backtest_config.risk.stop_loss_pct {
                                ui.add(
                                    egui::DragValue::new(sl_v)
                                        .range(0.2..=50.0)
                                        .speed(0.2)
                                        .fixed_decimals(1),
                                );
                            }

                            let mut tp_on = self.backtest_config.risk.take_profit_pct.is_some();
                            if ui
                                .checkbox(
                                    &mut tp_on,
                                    RichText::new("TP%").monospace().size(9.5),
                                )
                                .changed()
                            {
                                self.backtest_config.risk.take_profit_pct =
                                    tp_on.then_some(12.0);
                            }
                            if let Some(ref mut tp_v) = self.backtest_config.risk.take_profit_pct {
                                ui.add(
                                    egui::DragValue::new(tp_v)
                                        .range(0.5..=200.0)
                                        .speed(0.5)
                                        .fixed_decimals(1),
                                );
                            }

                            let mut tr_on =
                                self.backtest_config.risk.trailing_stop_atr_mult.is_some();
                            if ui
                                .checkbox(
                                    &mut tr_on,
                                    RichText::new("TrATR").monospace().size(9.5),
                                )
                                .changed()
                            {
                                self.backtest_config.risk.trailing_stop_atr_mult =
                                    tr_on.then_some(2.5);
                            }
                            if let Some(ref mut tr_v) =
                                self.backtest_config.risk.trailing_stop_atr_mult
                            {
                                ui.add(
                                    egui::DragValue::new(tr_v)
                                        .range(0.5..=10.0)
                                        .speed(0.1)
                                        .fixed_decimals(1),
                                );
                            }

                            let mut oos_on = self.backtest_config.in_sample_split_pct.is_some();
                            if ui
                                .checkbox(
                                    &mut oos_on,
                                    RichText::new("OOS 70/30").monospace().size(9.5),
                                )
                                .changed()
                            {
                                self.backtest_config.in_sample_split_pct =
                                    oos_on.then_some(0.70);
                            }
                        });

                        // Fee & Slippage bps compact row
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(4.0, 2.0);
                            ui.label(
                                RichText::new("Fee(bp):")
                                    .monospace()
                                    .size(9.4)
                                    .color(palette::TEXT_MUTED),
                            );
                            ui.add(
                                egui::DragValue::new(&mut self.backtest_config.commission_bps)
                                    .range(0.0..=150.0)
                                    .speed(0.5)
                                    .fixed_decimals(1),
                            );
                            ui.label(
                                RichText::new("Slip(bp):")
                                    .monospace()
                                    .size(9.4)
                                    .color(palette::TEXT_MUTED),
                            );
                            ui.add(
                                egui::DragValue::new(&mut self.backtest_config.slippage_bps)
                                    .range(0.0..=150.0)
                                    .speed(0.5)
                                    .fixed_decimals(1),
                            );
                            ui.label(
                                RichText::new(format!(
                                    "Cash ${:.0}k",
                                    self.backtest_config.initial_cash / 1000.0
                                ))
                                .monospace()
                                .size(9.4)
                                .color(palette::TEXT_MUTED),
                            );
                        });
                    });

                // 5-Way Sub-Tab Switcher (`Stats` | `Trd(N)` | `Sweep` | `Battle` | `WFO`)
                let trade_n = self
                    .backtest_result
                    .as_ref()
                    .map_or(0, |r| r.trades.len());
                ui.horizontal(|ui| {
                    let sp_x = ui.spacing().item_spacing.x;
                    let w5 = ((ui.available_width() - 4.0 * sp_x - 2.0) / 5.0).max(38.0);
                    for sub in StrategyLabSubTab::ALL {
                        let lbl = match sub {
                            StrategyLabSubTab::Tearsheet => "Stats".to_owned(),
                            StrategyLabSubTab::Trades => format!("Trd({trade_n})"),
                            StrategyLabSubTab::Sweep => "Sweep".to_owned(),
                            StrategyLabSubTab::Battle => "Battle".to_owned(),
                            StrategyLabSubTab::WalkForward => "WFO".to_owned(),
                        };
                        let sel = self.strategy_lab_tab == sub;
                        let btn = Button::new(
                            RichText::new(lbl)
                                .monospace()
                                .strong()
                                .size(9.5)
                                .color(if sel {
                                    Color32::WHITE
                                } else {
                                    palette::TEXT_MUTED
                                }),
                        )
                        .fill(if sel {
                            palette::EQUITY_ABOVE_LINE.gamma_multiply(0.24)
                        } else {
                            Color32::from_rgb(20, 24, 34)
                        })
                        .stroke(Stroke::new(
                            1.0,
                            if sel {
                                palette::EQUITY_ABOVE_LINE
                            } else {
                                palette::BORDER_SUBTLE
                            },
                        ))
                        .min_size(Vec2::new(w5, 21.0));
                        if ui.add(btn).clicked() {
                            self.strategy_lab_tab = sub;
                        }
                    }
                });

                if self.show_equity_pane {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(4.0, 2.0);
                        ui.label(
                            RichText::new("EQ PANE:")
                                .monospace()
                                .strong()
                                .size(9.2)
                                .color(palette::TEXT_MUTED),
                        );
                        for src in [
                            EquityPaneSource::SingleAssetBacktest,
                            EquityPaneSource::WalkForwardOos,
                            EquityPaneSource::MultiAssetPortfolio,
                        ] {
                            let is_sel = self.equity_pane_source == src;
                            if ui
                                .selectable_label(
                                    is_sel,
                                    RichText::new(src.short_label()).monospace().size(9.4),
                                )
                                .clicked()
                            {
                                self.equity_pane_source = src;
                                self.viewport.equity_price_scale.reset_auto_scale();
                            }
                        }
                    });
                }

                match self.strategy_lab_tab {
                    StrategyLabSubTab::Tearsheet => {
                        if let Some(ref bt) = self.backtest_result {
                            let ts = &bt.tearsheet;
                            let ret_col = if ts.total_return_pct >= 0.0 {
                                palette::EQUITY_ABOVE_LINE
                            } else {
                                palette::EQUITY_BELOW_LINE
                            };

                            // Hero Equity + Robustness Card
                            Frame::new()
                                .fill(Color32::from_rgb(18, 23, 33))
                                .stroke(Stroke::new(1.0, ret_col.gamma_multiply(0.55)))
                                .corner_radius(4.0)
                                .inner_margin(Margin::symmetric(8, 6))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(format!("${:.0}", ts.final_equity))
                                                .monospace()
                                                .strong()
                                                .size(13.5)
                                                .color(palette::TEXT_PRIMARY),
                                        );
                                        ui.label(
                                            RichText::new(format!(
                                                "{:+.2}% ({:+.0})",
                                                ts.total_return_pct, ts.total_return_abs
                                            ))
                                            .monospace()
                                            .strong()
                                            .size(11.5)
                                            .color(ret_col),
                                        );
                                        if let Some(verdict) = bt.robustness {
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    let vc = robustness_badge_color(verdict);
                                                    ui.label(
                                                        RichText::new(verdict.label())
                                                            .monospace()
                                                            .strong()
                                                            .size(9.5)
                                                            .color(vc),
                                                    );
                                                },
                                            );
                                        }
                                    });
                                    if ts.low_sample_warning {
                                        ui.label(
                                            RichText::new(format!(
                                                "LOW SAMPLE: N={} < {} closed trades (statistical caution)",
                                                ts.num_trades, MIN_STATISTICAL_SAMPLE_TRADES
                                            ))
                                            .monospace()
                                            .size(9.2)
                                            .color(palette::ALERT_ARMED_AMBER),
                                        );
                                    }
                                });

                            // Quantitative Performance Tearsheet Grid
                            Frame::new()
                                .fill(Color32::from_rgb(16, 20, 29))
                                .stroke(Stroke::new(1.0, palette::BORDER_SUBTLE))
                                .corner_radius(4.0)
                                .inner_margin(Margin::symmetric(7, 6))
                                .show(ui, |ui| {
                                    let cagr_s = ts.cagr_pct.map_or_else(
                                        || "N/A(<7d)".to_owned(),
                                        |v| format!("{v:+.2}%"),
                                    );
                                    let sharpe_s = ts
                                        .sharpe_ratio
                                        .map_or_else(|| "—".to_owned(), |v| format!("{v:.2}"));
                                    let sortino_s = ts
                                        .sortino_ratio
                                        .map_or_else(|| "—".to_owned(), |v| format!("{v:.2}"));
                                    let calmar_s = ts
                                        .calmar_ratio
                                        .map_or_else(|| "—".to_owned(), |v| format!("{v:.2}"));
                                    let pf_s = if let Some(pf) = ts.profit_factor {
                                        format!("{pf:.2}")
                                    } else if ts.num_winners > 0 && ts.num_losers == 0 {
                                        "∞".to_owned()
                                    } else {
                                        "—".to_owned()
                                    };
                                    let exp_s = ts
                                        .expectancy_net
                                        .map_or_else(|| "—".to_owned(), |v| format!("{v:+.0}"));
                                    let r_s = ts
                                        .avg_r_multiple
                                        .map_or_else(|| "—".to_owned(), |v| format!("{v:+.2}R"));

                                    egui::Grid::new("strategy_tearsheet_grid")
                                        .num_columns(2)
                                        .spacing([8.0, 3.5])
                                        .striped(true)
                                        .show(ui, |ui| {
                                            let row = |ui: &mut egui::Ui,
                                                       k: &str,
                                                       v: String,
                                                       col: Color32| {
                                                ui.label(
                                                    RichText::new(k)
                                                        .monospace()
                                                        .size(9.6)
                                                        .color(palette::TEXT_MUTED),
                                                );
                                                ui.label(
                                                    RichText::new(v)
                                                        .monospace()
                                                        .strong()
                                                        .size(9.8)
                                                        .color(col),
                                                );
                                                ui.end_row();
                                            };

                                            row(
                                                ui,
                                                "CAGR (Ann.)",
                                                cagr_s,
                                                palette::TEXT_PRIMARY,
                                            );
                                            row(
                                                ui,
                                                "Realized PnL",
                                                format!("${:+.0}", ts.realized_net_profit),
                                                if ts.realized_net_profit >= 0.0 {
                                                    palette::BULLISH
                                                } else {
                                                    palette::BEARISH
                                                },
                                            );
                                            row(
                                                ui,
                                                "Max Drawdown",
                                                format!(
                                                    "-{:.2}% ({}b)",
                                                    ts.max_drawdown_pct,
                                                    ts.max_drawdown_duration_bars
                                                ),
                                                palette::EQUITY_BELOW_LINE,
                                            );
                                            row(
                                                ui,
                                                "Sharpe / Sortino",
                                                format!("{sharpe_s} / {sortino_s}"),
                                                palette::TEXT_PRIMARY,
                                            );
                                            row(ui, "Calmar Ratio", calmar_s, palette::TEXT_PRIMARY);
                                            row(
                                                ui,
                                                "Win Rate",
                                                format!(
                                                    "{:.1}% ({}W/{}L)",
                                                    ts.win_rate_pct, ts.num_winners, ts.num_losers
                                                ),
                                                palette::TEXT_PRIMARY,
                                            );
                                            row(
                                                ui,
                                                "Profit Factor",
                                                pf_s,
                                                palette::TEXT_PRIMARY,
                                            );
                                            row(
                                                ui,
                                                "Expectancy / R",
                                                format!("${exp_s} · {r_s}"),
                                                palette::TEXT_PRIMARY,
                                            );
                                            row(
                                                ui,
                                                "Avg Win / Loss",
                                                format!(
                                                    "{:+.2}% / {:.2}%",
                                                    ts.avg_win_pct, ts.avg_loss_pct
                                                ),
                                                palette::TEXT_PRIMARY,
                                            );
                                            row(
                                                ui,
                                                "Exposure / Hold",
                                                format!(
                                                    "{:.1}% · {:.1}b",
                                                    ts.exposure_pct, ts.avg_bars_held
                                                ),
                                                palette::TEXT_PRIMARY,
                                            );
                                            row(
                                                ui,
                                                "Fees+Tax / Unfill",
                                                format!(
                                                    "${:.0} · {} cut",
                                                    ts.total_fees_and_tax,
                                                    bt.unexecuted_signals.len()
                                                ),
                                                palette::TEXT_MUTED,
                                            );
                                        });
                                });

                            // In-Sample vs Out-of-Sample Split Card
                            if let (Some(is_s), Some(oos_s), Some(split_i)) = (
                                bt.in_sample_tearsheet.as_ref(),
                                bt.out_of_sample_tearsheet.as_ref(),
                                bt.split_bar_index,
                            ) {
                                Frame::new()
                                    .fill(Color32::from_rgb(17, 22, 32))
                                    .stroke(Stroke::new(
                                        1.0,
                                        palette::BACKTEST_OOS_SPLIT.gamma_multiply(0.65),
                                    ))
                                    .corner_radius(4.0)
                                    .inner_margin(Margin::symmetric(7, 5))
                                    .show(ui, |ui| {
                                        ui.label(
                                            RichText::new(format!(
                                                "IS (0..{split_i}) vs OOS ({split_i}..{})",
                                                bt.bar_count
                                            ))
                                            .monospace()
                                            .strong()
                                            .size(9.8)
                                            .color(palette::BACKTEST_OOS_SPLIT),
                                        );
                                        egui::Grid::new("is_oos_split_grid")
                                            .num_columns(3)
                                            .spacing([10.0, 2.5])
                                            .show(ui, |ui| {
                                                ui.label(
                                                    RichText::new("Metric")
                                                        .monospace()
                                                        .size(9.2)
                                                        .color(palette::TEXT_MUTED),
                                                );
                                                ui.label(
                                                    RichText::new("IS (70%)")
                                                        .monospace()
                                                        .strong()
                                                        .size(9.2)
                                                        .color(palette::TEXT_PRIMARY),
                                                );
                                                ui.label(
                                                    RichText::new("OOS (30%)")
                                                        .monospace()
                                                        .strong()
                                                        .size(9.2)
                                                        .color(palette::BACKTEST_OOS_SPLIT),
                                                );
                                                ui.end_row();

                                                let cmp_row = |ui: &mut egui::Ui,
                                                               m: &str,
                                                               v1: String,
                                                               v2: String| {
                                                    ui.label(
                                                        RichText::new(m)
                                                            .monospace()
                                                            .size(9.4)
                                                            .color(palette::TEXT_MUTED),
                                                    );
                                                    ui.label(
                                                        RichText::new(v1)
                                                            .monospace()
                                                            .size(9.5)
                                                            .color(palette::TEXT_PRIMARY),
                                                    );
                                                    ui.label(
                                                        RichText::new(v2)
                                                            .monospace()
                                                            .strong()
                                                            .size(9.5)
                                                            .color(palette::TEXT_PRIMARY),
                                                    );
                                                    ui.end_row();
                                                };

                                                cmp_row(
                                                    ui,
                                                    "Return",
                                                    format!("{:+.2}%", is_s.total_return_pct),
                                                    format!("{:+.2}%", oos_s.total_return_pct),
                                                );
                                                cmp_row(
                                                    ui,
                                                    "MaxDD",
                                                    format!("-{:.2}%", is_s.max_drawdown_pct),
                                                    format!("-{:.2}%", oos_s.max_drawdown_pct),
                                                );
                                                cmp_row(
                                                    ui,
                                                    "Sharpe",
                                                    is_s.sharpe_ratio.map_or_else(
                                                        || "—".to_owned(),
                                                        |v| format!("{v:.2}"),
                                                    ),
                                                    oos_s.sharpe_ratio.map_or_else(
                                                        || "—".to_owned(),
                                                        |v| format!("{v:.2}"),
                                                    ),
                                                );
                                                cmp_row(
                                                    ui,
                                                    "Win / Trd",
                                                    format!(
                                                        "{:.0}% ({})",
                                                        is_s.win_rate_pct, is_s.num_trades
                                                    ),
                                                    format!(
                                                        "{:.0}% ({})",
                                                        oos_s.win_rate_pct, oos_s.num_trades
                                                    ),
                                                );
                                            });
                                    });
                            }

                            if let Some(ref open_pos) = bt.open_position {
                                let p_col = if open_pos.unrealized_pnl >= 0.0 {
                                    palette::BULLISH
                                } else {
                                    palette::BEARISH
                                };
                                Frame::new()
                                    .fill(Color32::from_rgb(19, 25, 36))
                                    .stroke(Stroke::new(1.0, p_col.gamma_multiply(0.6)))
                                    .corner_radius(4.0)
                                    .inner_margin(Margin::symmetric(7, 5))
                                    .show(ui, |ui| {
                                        ui.label(
                                            RichText::new(format!(
                                                "OPEN {} {:.2} @ {:.2}  |  uPnL {:+.0} ({:+.2}%)",
                                                open_pos.side.short_label(),
                                                open_pos.quantity,
                                                open_pos.entry_price,
                                                open_pos.unrealized_pnl,
                                                open_pos.unrealized_return_pct
                                            ))
                                            .monospace()
                                            .strong()
                                            .size(9.5)
                                            .color(p_col),
                                        );
                                    });
                            }
                        }
                    }

                    StrategyLabSubTab::Trades => {
                        if let Some(ref bt) = self.backtest_result {
                            ui.label(
                                RichText::new(format!(
                                    "{} closed trades (newest first · causal Open[T+1] fills)",
                                    bt.trades.len()
                                ))
                                .monospace()
                                .size(9.4)
                                .color(palette::TEXT_MUTED),
                            );
                            if bt.trades.is_empty() {
                                ui.label(
                                    RichText::new("No closed round-trip trades in causal window.")
                                        .monospace()
                                        .size(10.0)
                                        .color(palette::TEXT_MUTED),
                                );
                            }
                            for tr in bt.trades.iter().rev().take(80) {
                                let is_win = tr.net_pnl > 0.0;
                                let t_col = if is_win {
                                    palette::TRADE_LINK_WIN
                                } else {
                                    palette::TRADE_LINK_LOSS
                                };
                                let is_focused =
                                    self.focused_backtest_trade_id == Some(tr.trade_id);
                                Frame::new()
                                    .fill(if is_focused {
                                        Color32::from_rgb(28, 33, 46)
                                    } else {
                                        Color32::from_rgb(17, 21, 30)
                                    })
                                    .stroke(Stroke::new(
                                        if is_focused { 1.4 } else { 1.0 },
                                        if is_focused {
                                            palette::TRADE_FOCUS_HALO
                                        } else {
                                            palette::BORDER_SUBTLE
                                        },
                                    ))
                                    .corner_radius(4.0)
                                    .inner_margin(Margin::symmetric(6, 5))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            let r_tag = tr.r_multiple.map_or_else(
                                                String::new,
                                                |r| format!(" · {r:+.2}R"),
                                            );
                                            ui.label(
                                                RichText::new(format!(
                                                    "#{} {} {:+.2}% ({:+.0}){r_tag} [{}]",
                                                    tr.trade_id,
                                                    tr.side.short_label(),
                                                    tr.return_pct,
                                                    tr.net_pnl,
                                                    tr.exit_reason.short_label()
                                                ))
                                                .monospace()
                                                .strong()
                                                .size(9.6)
                                                .color(t_col),
                                            );
                                        });
                                        ui.label(
                                            RichText::new(format!(
                                                "{:.2} -> {:.2} ({}b) · MAE {:.1}% / MFE {:+.1}%",
                                                tr.entry_price,
                                                tr.exit_price,
                                                tr.bars_held,
                                                tr.mae_pct,
                                                tr.mfe_pct
                                            ))
                                            .monospace()
                                            .size(9.1)
                                            .color(palette::TEXT_MUTED),
                                        );
                                        ui.horizontal(|ui| {
                                            if ui
                                                .small_button(
                                                    RichText::new(if is_focused {
                                                        "Focused ★"
                                                    } else {
                                                        "Focus Chart"
                                                    })
                                                    .monospace()
                                                    .size(9.2),
                                                )
                                                .clicked()
                                            {
                                                focus_trade_req = Some(tr.trade_id);
                                            }
                                            if ui
                                                .small_button(
                                                    RichText::new("Replay -5b")
                                                        .monospace()
                                                        .size(9.2),
                                                )
                                                .clicked()
                                            {
                                                replay_trade_req = Some(tr.trade_id);
                                            }
                                        });
                                    });
                            }
                        }
                    }

                    StrategyLabSubTab::Sweep => {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("Rank:")
                                    .monospace()
                                    .size(9.5)
                                    .color(palette::TEXT_MUTED),
                            );
                            for m in SweepRankingMetric::ALL {
                                if ui
                                    .selectable_label(
                                        self.sweep_ranking == m,
                                        RichText::new(m.short_label()).monospace().size(9.4),
                                    )
                                    .clicked()
                                {
                                    self.sweep_ranking = m;
                                }
                            }
                            if ui
                                .small_button(RichText::new("Run").monospace().size(9.4))
                                .clicked()
                            {
                                force_sweep = true;
                            }
                        });

                        if let Some(ref sweep) = self.sweep_report {
                            Frame::new()
                                .fill(Color32::from_rgb(16, 20, 29))
                                .stroke(Stroke::new(1.0, palette::BORDER_SUBTLE))
                                .corner_radius(4.0)
                                .inner_margin(Margin::symmetric(6, 5))
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new(format!(
                                            "12-GRID SWEEP · {} {}",
                                            sweep.symbol, sweep.timeframe
                                        ))
                                        .monospace()
                                        .strong()
                                        .size(9.6)
                                        .color(palette::TEXT_PRIMARY),
                                    );
                                    egui::Grid::new("strategy_sweep_grid")
                                        .num_columns(5)
                                        .spacing([5.0, 2.5])
                                        .striped(true)
                                        .show(ui, |ui| {
                                            for hdr in ["#", "Variant", "Ret%", "Shp", "OOS%"] {
                                                ui.label(
                                                    RichText::new(hdr)
                                                        .monospace()
                                                        .strong()
                                                        .size(9.0)
                                                        .color(palette::TEXT_MUTED),
                                                );
                                            }
                                            ui.end_row();
                                            for row in &sweep.variants {
                                                let col = if row.tearsheet.total_return_pct >= 0.0 {
                                                    palette::BULLISH
                                                } else {
                                                    palette::BEARISH
                                                };
                                                ui.label(
                                                    RichText::new(format!("{}", row.rank))
                                                        .monospace()
                                                        .size(9.1)
                                                        .color(palette::TEXT_MUTED),
                                                );
                                                if ui
                                                    .link(
                                                        RichText::new(&row.label)
                                                            .monospace()
                                                            .size(9.1)
                                                            .color(palette::TEXT_PRIMARY),
                                                    )
                                                    .clicked()
                                                {
                                                    apply_sweep_cfg = Some(row.config.clone());
                                                }
                                                ui.label(
                                                    RichText::new(format!(
                                                        "{:+.1}%",
                                                        row.tearsheet.total_return_pct
                                                    ))
                                                    .monospace()
                                                    .strong()
                                                    .size(9.1)
                                                    .color(col),
                                                );
                                                ui.label(
                                                    RichText::new(
                                                        row.tearsheet.sharpe_ratio.map_or_else(
                                                            || "—".to_owned(),
                                                            |v| format!("{v:.2}"),
                                                        ),
                                                    )
                                                    .monospace()
                                                    .size(9.1),
                                                );
                                                ui.label(
                                                    RichText::new(
                                                        row.out_of_sample_return_pct.map_or_else(
                                                            || "—".to_owned(),
                                                            |v| format!("{v:+.1}%"),
                                                        ),
                                                    )
                                                    .monospace()
                                                    .size(9.1),
                                                );
                                                ui.end_row();
                                            }
                                        });
                                });
                        }

                        if !self.preset_battle_rows.is_empty() {
                            Frame::new()
                                .fill(Color32::from_rgb(16, 20, 29))
                                .stroke(Stroke::new(1.0, palette::BORDER_SUBTLE))
                                .corner_radius(4.0)
                                .inner_margin(Margin::symmetric(6, 5))
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new(format!(
                                            "6-PRESET BATTLE ON {}",
                                            self.symbol
                                        ))
                                        .monospace()
                                        .strong()
                                        .size(9.6)
                                        .color(palette::BACKTEST_OOS_SPLIT),
                                    );
                                    egui::Grid::new("preset_battle_grid_sweep")
                                        .num_columns(4)
                                        .spacing([6.0, 2.5])
                                        .striped(true)
                                        .show(ui, |ui| {
                                            for hdr in ["Preset", "Ret%", "MaxDD", "Shp"] {
                                                ui.label(
                                                    RichText::new(hdr)
                                                        .monospace()
                                                        .strong()
                                                        .size(9.0)
                                                        .color(palette::TEXT_MUTED),
                                                );
                                            }
                                            ui.end_row();
                                            for pr in &self.preset_battle_rows {
                                                let col = if pr.tearsheet.total_return_pct >= 0.0 {
                                                    palette::BULLISH
                                                } else {
                                                    palette::BEARISH
                                                };
                                                if ui
                                                    .link(
                                                        RichText::new(pr.preset.short_label())
                                                            .monospace()
                                                            .size(9.2)
                                                            .color(palette::TEXT_PRIMARY),
                                                    )
                                                    .clicked()
                                                {
                                                    apply_preset_req = Some(pr.preset);
                                                }
                                                ui.label(
                                                    RichText::new(format!(
                                                        "{:+.1}%",
                                                        pr.tearsheet.total_return_pct
                                                    ))
                                                    .monospace()
                                                    .strong()
                                                    .size(9.2)
                                                    .color(col),
                                                );
                                                ui.label(
                                                    RichText::new(format!(
                                                        "-{:.1}%",
                                                        pr.tearsheet.max_drawdown_pct
                                                    ))
                                                    .monospace()
                                                    .size(9.1)
                                                    .color(palette::EQUITY_BELOW_LINE),
                                                );
                                                ui.label(
                                                    RichText::new(
                                                        pr.tearsheet.sharpe_ratio.map_or_else(
                                                            || "—".to_owned(),
                                                            |v| format!("{v:.2}"),
                                                        ),
                                                    )
                                                    .monospace()
                                                    .size(9.1),
                                                );
                                                ui.end_row();
                                            }
                                        });
                                });
                        }
                    }

                    StrategyLabSubTab::Battle => {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("Rank:")
                                    .monospace()
                                    .size(9.5)
                                    .color(palette::TEXT_MUTED),
                            );
                            for m in SweepRankingMetric::ALL {
                                if ui
                                    .selectable_label(
                                        self.sweep_ranking == m,
                                        RichText::new(m.short_label()).monospace().size(9.4),
                                    )
                                    .clicked()
                                {
                                    self.sweep_ranking = m;
                                }
                            }
                            if ui
                                .small_button(RichText::new("Run").monospace().size(9.4))
                                .clicked()
                            {
                                force_battle = true;
                            }
                        });

                        if let Some(ref battle) = self.battle_report {
                            Frame::new()
                                .fill(Color32::from_rgb(16, 20, 29))
                                .stroke(Stroke::new(1.0, palette::BORDER_SUBTLE))
                                .corner_radius(4.0)
                                .inner_margin(Margin::symmetric(6, 5))
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new(format!(
                                            "UNIVERSE BATTLE ({}/{}) · Avg {:+.1}%",
                                            battle.profitable_symbols,
                                            battle.rows.len(),
                                            battle.average_return_pct
                                        ))
                                        .monospace()
                                        .strong()
                                        .size(9.6)
                                        .color(palette::EQUITY_ABOVE_LINE),
                                    );
                                    egui::Grid::new("universe_battle_grid")
                                        .num_columns(5)
                                        .spacing([6.0, 2.5])
                                        .striped(true)
                                        .show(ui, |ui| {
                                            for hdr in ["#", "Symbol", "Ret%", "DD%", "Shp"] {
                                                ui.label(
                                                    RichText::new(hdr)
                                                        .monospace()
                                                        .strong()
                                                        .size(9.0)
                                                        .color(palette::TEXT_MUTED),
                                                );
                                            }
                                            ui.end_row();
                                            for row in &battle.rows {
                                                let col = if row.tearsheet.total_return_pct >= 0.0 {
                                                    palette::BULLISH
                                                } else {
                                                    palette::BEARISH
                                                };
                                                ui.label(
                                                    RichText::new(format!("{}", row.rank))
                                                        .monospace()
                                                        .size(9.1)
                                                        .color(palette::TEXT_MUTED),
                                                );
                                                let sym_lbl = format!(
                                                    "{}·{}",
                                                    row.symbol, row.timeframe
                                                );
                                                if ui
                                                    .link(
                                                        RichText::new(sym_lbl)
                                                            .monospace()
                                                            .strong()
                                                            .size(9.1)
                                                            .color(palette::TEXT_PRIMARY),
                                                    )
                                                    .clicked()
                                                {
                                                    if let Some(preset) = self
                                                        .watchlist_universe
                                                        .iter()
                                                        .find(|e| {
                                                            e.instrument_id == row.instrument_id
                                                        })
                                                        .map(|e| e.preset)
                                                    {
                                                        select_symbol_req =
                                                            Some((preset, row.symbol.clone()));
                                                    } else if self
                                                        .custom_peer_feeds
                                                        .iter()
                                                        .any(|e| {
                                                            e.instrument_id == row.instrument_id
                                                        })
                                                    {
                                                        select_symbol_req = Some((
                                                            DataSourcePreset::CustomFile,
                                                            row.symbol.clone(),
                                                        ));
                                                    }
                                                }
                                                ui.label(
                                                    RichText::new(format!(
                                                        "{:+.1}%",
                                                        row.tearsheet.total_return_pct
                                                    ))
                                                    .monospace()
                                                    .strong()
                                                    .size(9.1)
                                                    .color(col),
                                                );
                                                ui.label(
                                                    RichText::new(format!(
                                                        "-{:.1}%",
                                                        row.tearsheet.max_drawdown_pct
                                                    ))
                                                    .monospace()
                                                    .size(9.1)
                                                    .color(palette::EQUITY_BELOW_LINE),
                                                );
                                                ui.label(
                                                    RichText::new(
                                                        row.tearsheet.sharpe_ratio.map_or_else(
                                                            || "—".to_owned(),
                                                            |v| format!("{v:.2}"),
                                                        ),
                                                    )
                                                    .monospace()
                                                    .size(9.1),
                                                );
                                                ui.end_row();
                                            }
                                        });
                                });
                        }
                    }
                    StrategyLabSubTab::WalkForward => {
                        // Card A: Rolling / Anchored Walk-Forward Optimization (`IS -> OOS`)
                        Frame::new()
                            .fill(Color32::from_rgb(17, 22, 32))
                            .stroke(Stroke::new(1.0, palette::BORDER_SUBTLE))
                            .corner_radius(4.0)
                            .inner_margin(Margin::symmetric(7, 6))
                            .show(ui, |ui| {
                                ui.spacing_mut().item_spacing = Vec2::new(4.0, 4.0);
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new("WALK-FORWARD (IS -> OOS)")
                                            .monospace()
                                            .strong()
                                            .size(10.0)
                                            .color(palette::TEXT_PRIMARY),
                                    );
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if ui
                                                .button(
                                                    RichText::new("Plot OOS EQ")
                                                        .monospace()
                                                        .strong()
                                                        .size(9.3)
                                                        .color(palette::EQUITY_ABOVE_LINE),
                                                )
                                                .clicked()
                                            {
                                                self.show_equity_pane = true;
                                                self.equity_pane_source =
                                                    EquityPaneSource::WalkForwardOos;
                                                self.viewport
                                                    .equity_price_scale
                                                    .reset_auto_scale();
                                                force_wfo = true;
                                            }
                                            if ui
                                                .button(
                                                    RichText::new("Run WFO")
                                                        .monospace()
                                                        .size(9.3),
                                                )
                                                .clicked()
                                            {
                                                force_wfo = true;
                                            }
                                        },
                                    );
                                });

                                ui.horizontal_wrapped(|ui| {
                                    ui.spacing_mut().item_spacing = Vec2::new(4.0, 3.0);
                                    ui.label(RichText::new("Folds:").monospace().size(9.4));
                                    ui.add(
                                        egui::DragValue::new(&mut self.wf_config.fold_count)
                                            .range(2..=8)
                                            .speed(0.2),
                                    );
                                    let mut is_pct = (self.wf_config.in_sample_ratio * 100.0)
                                        .round()
                                        .clamp(50.0, 85.0);
                                    ui.label(RichText::new("IS%:").monospace().size(9.4));
                                    if ui
                                        .add(
                                            egui::DragValue::new(&mut is_pct)
                                                .range(50.0..=85.0)
                                                .speed(1.0)
                                                .suffix("%"),
                                        )
                                        .changed()
                                    {
                                        self.wf_config.in_sample_ratio =
                                            (is_pct / 100.0).clamp(0.50, 0.85);
                                    }
                                    for (anchored, lbl) in [(false, "Rolling"), (true, "Anchored")]
                                    {
                                        if ui
                                            .selectable_label(
                                                self.wf_config.anchored == anchored,
                                                RichText::new(lbl).monospace().size(9.3),
                                            )
                                            .clicked()
                                        {
                                            self.wf_config.anchored = anchored;
                                        }
                                    }
                                });

                                if let Some(ref wf) = self.walk_forward_report {
                                    let v_col = match wf.verdict {
                                        WalkForwardVerdict::RobustOutOfSample => palette::BULLISH,
                                        WalkForwardVerdict::ModerateDecay => {
                                            palette::ALERT_ARMED_AMBER
                                        }
                                        WalkForwardVerdict::OverfitRegimeDecay
                                        | WalkForwardVerdict::InsufficientData => palette::BEARISH,
                                    };
                                    let wfe_str = wf
                                        .wfe_pct
                                        .map_or_else(|| "-".to_owned(), |w| format!("{w:.0}%"));
                                    ui.horizontal_wrapped(|ui| {
                                        ui.label(
                                            RichText::new(wf.verdict.label())
                                                .monospace()
                                                .strong()
                                                .size(9.8)
                                                .color(v_col),
                                        );
                                        ui.label(
                                            RichText::new(format!(
                                                "· WFE {wfe_str} · Stab {:.0}% · Prof {}/{}",
                                                wf.param_stability_pct,
                                                wf.profitable_folds,
                                                wf.folds.len()
                                            ))
                                            .monospace()
                                            .size(9.3)
                                            .color(palette::TEXT_MUTED),
                                        );
                                    });

                                    let oos_ts = &wf.stitched_oos_tearsheet;
                                    let oos_col = if oos_ts.total_return_pct >= 0.0 {
                                        palette::EQUITY_ABOVE_LINE
                                    } else {
                                        palette::EQUITY_BELOW_LINE
                                    };
                                    let sh_str = oos_ts
                                        .sharpe_ratio
                                        .map_or_else(|| "-".to_owned(), |v| format!("{v:.2}"));
                                    ui.label(
                                        RichText::new(format!(
                                            "Stitched OOS {:+.2}% · DD -{:.2}% · Sh {sh_str} · {} trd",
                                            oos_ts.total_return_pct,
                                            oos_ts.max_drawdown_pct,
                                            wf.stitched_oos_trades.len()
                                        ))
                                        .monospace()
                                        .strong()
                                        .size(9.5)
                                        .color(oos_col),
                                    );

                                    if !wf.folds.is_empty() {
                                        ui.separator();
                                        egui::Grid::new("sumi_wfo_folds_grid")
                                            .striped(true)
                                            .spacing([6.0, 2.5])
                                            .show(ui, |ui| {
                                                for hdr in ["Fold", "OOS", "Winner", "IS%", "OOS%"]
                                                {
                                                    ui.label(
                                                        RichText::new(hdr)
                                                            .monospace()
                                                            .strong()
                                                            .size(8.9)
                                                            .color(palette::TEXT_MUTED),
                                                    );
                                                }
                                                ui.end_row();
                                                for fold in &wf.folds {
                                                    ui.label(
                                                        RichText::new(format!(
                                                            "F{}",
                                                            fold.fold_index
                                                        ))
                                                        .monospace()
                                                        .size(9.0),
                                                    );
                                                    ui.label(
                                                        RichText::new(format!(
                                                            "{}..{}",
                                                            fold.oos_start_bar, fold.oos_end_bar
                                                        ))
                                                        .monospace()
                                                        .size(8.9)
                                                        .color(palette::TEXT_MUTED),
                                                    );
                                                    if ui
                                                        .small_button(
                                                            RichText::new(&fold.winning_label)
                                                                .monospace()
                                                                .size(8.9),
                                                        )
                                                        .on_hover_text(
                                                            "Apply fold winning config to Strategy Lab",
                                                        )
                                                        .clicked()
                                                    {
                                                        apply_sweep_cfg =
                                                            Some(fold.winning_config.clone());
                                                    }
                                                    ui.label(
                                                        RichText::new(format!(
                                                            "{:+.1}%",
                                                            fold.is_tearsheet.total_return_pct
                                                        ))
                                                        .monospace()
                                                        .size(9.0)
                                                        .color(palette::TEXT_PRIMARY),
                                                    );
                                                    let f_col =
                                                        if fold.oos_tearsheet.total_return_pct >= 0.0
                                                        {
                                                            palette::BULLISH
                                                        } else {
                                                            palette::BEARISH
                                                        };
                                                    ui.label(
                                                        RichText::new(format!(
                                                            "{:+.1}%",
                                                            fold.oos_tearsheet.total_return_pct
                                                        ))
                                                        .monospace()
                                                        .strong()
                                                        .size(9.0)
                                                        .color(f_col),
                                                    );
                                                    ui.end_row();
                                                }
                                            });
                                    }
                                }
                            });

                        // Card B: Multi-Asset Portfolio Simulation (`VN30 Basket` / `All 13 Feeds`)
                        Frame::new()
                            .fill(Color32::from_rgb(17, 22, 32))
                            .stroke(Stroke::new(1.0, palette::BORDER_SUBTLE))
                            .corner_radius(4.0)
                            .inner_margin(Margin::symmetric(7, 6))
                            .show(ui, |ui| {
                                ui.spacing_mut().item_spacing = Vec2::new(4.0, 4.0);
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new("MULTI-ASSET PORTFOLIO")
                                            .monospace()
                                            .strong()
                                            .size(10.0)
                                            .color(palette::TEXT_PRIMARY),
                                    );
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if ui
                                                .button(
                                                    RichText::new("Plot Port EQ")
                                                        .monospace()
                                                        .strong()
                                                        .size(9.3)
                                                        .color(palette::TRADE_ENTRY_BLUE),
                                                )
                                                .clicked()
                                            {
                                                self.show_equity_pane = true;
                                                self.equity_pane_source =
                                                    EquityPaneSource::MultiAssetPortfolio;
                                                self.viewport
                                                    .equity_price_scale
                                                    .reset_auto_scale();
                                                force_portfolio = true;
                                            }
                                            if ui
                                                .button(
                                                    RichText::new("Run Port")
                                                        .monospace()
                                                        .size(9.3),
                                                )
                                                .clicked()
                                            {
                                                force_portfolio = true;
                                            }
                                        },
                                    );
                                });

                                ui.horizontal_wrapped(|ui| {
                                    ui.spacing_mut().item_spacing = Vec2::new(4.0, 3.0);
                                    for filt in [
                                        PortfolioUniverseFilter::Vn30EquitiesOnly,
                                        PortfolioUniverseFilter::AllUniverseFeeds,
                                    ] {
                                        if ui
                                            .selectable_label(
                                                self.portfolio_universe_filter == filt,
                                                RichText::new(filt.short_label())
                                                    .monospace()
                                                    .size(9.3),
                                            )
                                            .clicked()
                                        {
                                            self.portfolio_universe_filter = filt;
                                        }
                                    }
                                    ui.separator();
                                    let is_eq = matches!(
                                        self.portfolio_weighting,
                                        PortfolioWeightingMode::EqualWeight
                                    );
                                    if ui
                                        .selectable_label(
                                            is_eq,
                                            RichText::new("1/N Equal").monospace().size(9.3),
                                        )
                                        .clicked()
                                    {
                                        self.portfolio_weighting =
                                            PortfolioWeightingMode::EqualWeight;
                                    }
                                    if ui
                                        .selectable_label(
                                            !is_eq,
                                            RichText::new("Inv-Vol(20)").monospace().size(9.3),
                                        )
                                        .clicked()
                                    {
                                        self.portfolio_weighting =
                                            PortfolioWeightingMode::CausalInverseVolatility {
                                                lookback_bars: 20,
                                            };
                                    }
                                });

                                if let Some(ref port) = self.portfolio_report {
                                    let pts = &port.portfolio_tearsheet;
                                    let p_col = if pts.total_return_pct >= 0.0 {
                                        palette::EQUITY_ABOVE_LINE
                                    } else {
                                        palette::EQUITY_BELOW_LINE
                                    };
                                    let p_sh = pts
                                        .sharpe_ratio
                                        .map_or_else(|| "-".to_owned(), |v| format!("{v:.2}"));
                                    ui.label(
                                        RichText::new(format!(
                                            "Port {:+.2}% · DD -{:.2}% · Sh {p_sh} ({} assets)",
                                            pts.total_return_pct,
                                            pts.max_drawdown_pct,
                                            port.constituents.len()
                                        ))
                                        .monospace()
                                        .strong()
                                        .size(9.5)
                                        .color(p_col),
                                    );
                                    ui.label(
                                        RichText::new(format!(
                                            "Avg Asset DD -{:.2}% -> Div DD Red {:+.1}%",
                                            port.avg_constituent_max_dd_pct,
                                            port.diversification_dd_reduction_pct
                                        ))
                                        .monospace()
                                        .size(9.1)
                                        .color(palette::TEXT_MUTED),
                                    );

                                    if !port.constituents.is_empty() {
                                        ui.separator();
                                        egui::Grid::new("sumi_portfolio_attrib_grid")
                                            .striped(true)
                                            .spacing([6.0, 2.5])
                                            .show(ui, |ui| {
                                                for hdr in ["Sym", "Wt%", "Ret%", "DD%", "Ctrb%"] {
                                                    ui.label(
                                                        RichText::new(hdr)
                                                            .monospace()
                                                            .strong()
                                                            .size(8.9)
                                                            .color(palette::TEXT_MUTED),
                                                    );
                                                }
                                                ui.end_row();
                                                for row in &port.constituents {
                                                    ui.label(
                                                        RichText::new(&row.symbol)
                                                            .monospace()
                                                            .strong()
                                                            .size(9.0),
                                                    );
                                                    ui.label(
                                                        RichText::new(format!(
                                                            "{:.1}%",
                                                            row.final_weight_pct
                                                        ))
                                                        .monospace()
                                                        .size(9.0)
                                                        .color(palette::TEXT_MUTED),
                                                    );
                                                    let r_c = if row.standalone_return_pct >= 0.0 {
                                                        palette::BULLISH
                                                    } else {
                                                        palette::BEARISH
                                                    };
                                                    ui.label(
                                                        RichText::new(format!(
                                                            "{:+.1}%",
                                                            row.standalone_return_pct
                                                        ))
                                                        .monospace()
                                                        .size(9.0)
                                                        .color(r_c),
                                                    );
                                                    ui.label(
                                                        RichText::new(format!(
                                                            "-{:.1}%",
                                                            row.standalone_max_dd_pct
                                                        ))
                                                        .monospace()
                                                        .size(9.0)
                                                        .color(palette::TEXT_MUTED),
                                                    );
                                                    let c_col = if row
                                                        .weighted_return_contribution_pct
                                                        >= 0.0
                                                    {
                                                        palette::EQUITY_ABOVE_LINE
                                                    } else {
                                                        palette::EQUITY_BELOW_LINE
                                                    };
                                                    ui.label(
                                                        RichText::new(format!(
                                                            "{:+.2}%",
                                                            row.weighted_return_contribution_pct
                                                        ))
                                                        .monospace()
                                                        .strong()
                                                        .size(9.0)
                                                        .color(c_col),
                                                    );
                                                    ui.end_row();
                                                }
                                            });
                                    }
                                }
                            });
                    }
                }
            });

        if let Some(cfg) = apply_sweep_cfg {
            self.backtest_config = cfg;
        }
        if let Some(preset) = apply_preset_req {
            self.backtest_config.preset = preset;
        }
        if let Some((preset, sym)) = select_symbol_req {
            self.select_watchlist_instrument(preset, &sym);
            self.show_strategy_panel = true;
            self.right_dock_tab = RightDockTab::StrategyLab;
        }
        if force_sweep {
            self.run_active_parameter_sweep();
        }
        if force_battle {
            self.run_universe_strategy_battle();
        }
        if force_wfo {
            self.run_active_walk_forward();
        }
        if force_portfolio {
            self.run_active_portfolio_backtest();
        }
        if let Some(tid) = focus_trade_req {
            let _ = self.focus_backtest_trade(tid);
        }
        if let Some(tid) = replay_trade_req {
            let _ = self.replay_backtest_trade(tid);
        }

        self.backtest_config.sanitize();
        self.wf_config.sanitize();
        if compute_backtest_config_digest(&self.backtest_config) != pre_cfg_digest
            || compute_walk_forward_config_digest(&self.wf_config) != pre_wf_digest
            || self.sweep_ranking != pre_ranking
            || self.strategy_lab_tab != pre_sub_tab
            || self.portfolio_universe_filter != pre_port_filter
            || self.portfolio_weighting != pre_port_wt
            || self.equity_pane_source != pre_eq_src
        {
            self.sync_backtest_causal();
        }
    }

    fn render_journal_panel(&mut self, ui: &mut egui::Ui) {
        self.auto_sync_planner_if_rr_box_changed();
        let replay_cutoff = self.replay_causal_close_timestamp();
        let active_venue = self.active_exchange_venue();
        let inferred_venue =
            ExchangeVenue::infer_from_symbol_and_preset(&self.symbol, self.data_source);
        let daily_band = self.active_daily_price_band();

        let mut action_from_box = false;
        let mut action_snap_ticks = false;
        let mut action_draw_rr = false;
        let mut action_to_ticket = false;
        let mut action_log_plan = false;
        let mut action_sync_paper = false;
        let mut action_import_bt = false;
        let mut action_copy_csv = false;
        let mut action_copy_md = false;
        let mut delete_entry_req: Option<u64> = None;

        egui::ScrollArea::vertical()
            .id_salt("sumi_journal_panel_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(4.0, 3.5);

                // Header Row + Causal Mode Badge
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("VN RISK & JOURNAL")
                            .monospace()
                            .strong()
                            .size(11.5)
                            .color(palette::TEXT_PRIMARY),
                    );
                    let (m_lbl, m_col) = if self.replay.is_active() {
                        ("RPL CAUSAL", palette::REPLAY_CUT_GOLD)
                    } else {
                        ("LIVE", Color32::from_rgb(16, 185, 129))
                    };
                    ui.label(
                        RichText::new(m_lbl)
                            .monospace()
                            .strong()
                            .size(9.4)
                            .color(m_col),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .small_button(RichText::new("Reseed").monospace().size(9.3))
                            .on_hover_text("Reseed Entry/Stop/Target from current mark & venue")
                            .clicked()
                        {
                            self.reseed_risk_planner_from_live_mark();
                        }
                    });
                });

                if let Some((is_err, ref msg)) = self.last_journal_status {
                    let s_col = if is_err {
                        palette::BEARISH
                    } else {
                        Color32::from_rgb(16, 185, 129)
                    };
                    Frame::new()
                        .fill(s_col.gamma_multiply(0.14))
                        .stroke(Stroke::new(1.0, s_col.gamma_multiply(0.75)))
                        .corner_radius(3.0)
                        .inner_margin(Margin::symmetric(5, 2))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(msg)
                                    .monospace()
                                    .size(9.0)
                                    .color(s_col),
                            );
                        });
                }

                // Card 1: Vietnam & Multi-Asset Microstructure Risk Planner
                Frame::new()
                    .fill(Color32::from_rgb(17, 22, 32))
                    .stroke(Stroke::new(1.0, palette::BORDER_SUBTLE))
                    .corner_radius(4.0)
                    .inner_margin(Margin::symmetric(6, 4))
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(4.0, 2.8);

                        // Venue Selector + Unit Selector
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("Venue:")
                                    .monospace()
                                    .size(9.4)
                                    .color(palette::TEXT_MUTED),
                            );
                            let selected_lbl = match self.risk_venue_override {
                                None => format!("Auto ({})", inferred_venue.short_label()),
                                Some(v) => v.short_label().to_owned(),
                            };
                            let prev_override = self.risk_venue_override;
                            egui::ComboBox::from_id_salt("sumi_risk_venue_combo")
                                .selected_text(
                                    RichText::new(selected_lbl)
                                        .monospace()
                                        .strong()
                                        .size(9.4)
                                        .color(palette::TEXT_PRIMARY),
                                )
                                .width(118.0)
                                .show_ui(ui, |ui| {
                                    if ui
                                        .selectable_label(
                                            self.risk_venue_override.is_none(),
                                            format!("Auto ({})", inferred_venue.full_label()),
                                        )
                                        .clicked()
                                    {
                                        self.risk_venue_override = None;
                                    }
                                    for v in ExchangeVenue::ALL {
                                        if ui
                                            .selectable_label(
                                                self.risk_venue_override == Some(v),
                                                v.full_label(),
                                            )
                                            .clicked()
                                        {
                                            self.risk_venue_override = Some(v);
                                        }
                                    }
                                });
                            if self.risk_venue_override != prev_override {
                                let new_v = self.active_exchange_venue();
                                self.risk_plan_input.commission_bps =
                                    new_v.default_commission_bps();
                                self.risk_plan_input.tax_sell_bps = new_v.default_tax_sell_bps();
                                self.risk_plan_input.contract_multiplier = 1.0;
                                self.risk_plan_input.initial_margin_pct =
                                    new_v.default_initial_margin_pct();
                                if !new_v.allows_short() {
                                    self.risk_plan_input.side = PositionSide::Long;
                                }
                            }
                            if active_venue.is_vn_cash_equity()
                                || active_venue == ExchangeVenue::Vn30Futures
                            {
                                let unit_lbl = match self.risk_plan_input.price_unit {
                                    VnPriceUnit::Auto => "U:Auto",
                                    VnPriceUnit::ThousandVnd => "U:1k",
                                    VnPriceUnit::RawVnd => "U:VND",
                                };
                                egui::ComboBox::from_id_salt("sumi_risk_price_unit_combo")
                                    .selected_text(
                                        RichText::new(unit_lbl)
                                            .monospace()
                                            .size(9.1)
                                            .color(palette::TEXT_MUTED),
                                    )
                                    .width(64.0)
                                    .show_ui(ui, |ui| {
                                        for (u, lbl) in [
                                            (VnPriceUnit::Auto, "Auto (1k / Raw VND)"),
                                            (VnPriceUnit::ThousandVnd, "Thousand VND (1k)"),
                                            (VnPriceUnit::RawVnd, "Raw VND"),
                                        ] {
                                            ui.selectable_value(
                                                &mut self.risk_plan_input.price_unit,
                                                u,
                                                lbl,
                                            );
                                        }
                                    });
                            }
                        });

                        if let Some(band) = daily_band {
                            let locked_state = self.candles.last().and_then(|bar| {
                                if band.is_locked_ceiling(bar.close, bar.high) {
                                    Some((
                                        "LOCKED CEILING (Trắng bên bán)",
                                        Color32::from_rgb(192, 132, 252),
                                    ))
                                } else if band.is_locked_floor(bar.close, bar.low) {
                                    Some((
                                        "LOCKED FLOOR (Trắng bên mua)",
                                        Color32::from_rgb(56, 189, 248),
                                    ))
                                } else {
                                    None
                                }
                            });
                            ui.label(
                                RichText::new(format!(
                                    "Band ±{:.0}% · Flr {} · Ref {} · Ceil {} · Lot {:.0} · T+{}",
                                    band.band_pct * 100.0,
                                    format_venue_price(band.floor_price, active_venue),
                                    format_venue_price(band.reference_price, active_venue),
                                    format_venue_price(band.ceiling_price, active_venue),
                                    active_venue.board_lot(),
                                    active_venue.settlement_business_days()
                                ))
                                .monospace()
                                .size(9.1)
                                .color(Color32::from_rgb(125, 211, 252)),
                            );
                            if let Some((lock_txt, lock_col)) = locked_state {
                                ui.label(
                                    RichText::new(lock_txt)
                                        .monospace()
                                        .strong()
                                        .size(9.3)
                                        .color(lock_col),
                                );
                            }
                        } else {
                            ui.label(
                                RichText::new(format!(
                                    "No statutory price band · Lot {} · T+{}",
                                    format_clean_qty(active_venue.board_lot()),
                                    active_venue.settlement_business_days()
                                ))
                                .monospace()
                                .size(9.1)
                                .color(palette::TEXT_MUTED),
                            );
                        }

                        // Side + Capital + Risk %
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(4.0, 3.0);
                            for side in [PositionSide::Long, PositionSide::Short] {
                                let is_sel = self.risk_plan_input.side == side;
                                let s_col = match side {
                                    PositionSide::Long => palette::BULLISH,
                                    PositionSide::Short => palette::BEARISH,
                                };
                                if ui
                                    .selectable_label(
                                        is_sel,
                                        RichText::new(side.short_label())
                                            .monospace()
                                            .strong()
                                            .size(9.6)
                                            .color(if is_sel {
                                                s_col
                                            } else {
                                                palette::TEXT_MUTED
                                            }),
                                    )
                                    .clicked()
                                    && self.risk_plan_input.side != side
                                {
                                    self.risk_plan_input.side = side;
                                    let entry = self.risk_plan_input.entry_price;
                                    let dist_sl =
                                        (entry - self.risk_plan_input.stop_loss).abs();
                                    let dist_tp =
                                        (self.risk_plan_input.target_price - entry).abs();
                                    if entry > 0.0 && dist_sl > 0.0 && dist_tp > 0.0 {
                                        match side {
                                            PositionSide::Long => {
                                                self.risk_plan_input.stop_loss =
                                                    (entry - dist_sl).max(entry * 0.05);
                                                self.risk_plan_input.target_price = entry + dist_tp;
                                            }
                                            PositionSide::Short => {
                                                self.risk_plan_input.stop_loss = entry + dist_sl;
                                                self.risk_plan_input.target_price =
                                                    (entry - dist_tp).max(entry * 0.05);
                                            }
                                        }
                                    }
                                }
                            }
                            ui.label(RichText::new("Cap:").monospace().size(9.4));
                            let cap_step = if self.risk_plan_input.account_equity >= 1_000_000.0 {
                                1_000_000.0
                            } else {
                                500.0
                            };
                            if ui
                                .add(
                                    egui::DragValue::new(&mut self.risk_plan_input.account_equity)
                                        .range(100.0..=1.0e12)
                                        .speed(cap_step),
                                )
                                .changed()
                            {
                                self.risk_plan_input.available_cash =
                                    self.risk_plan_input.account_equity;
                            }
                            ui.label(RichText::new("Risk%:").monospace().size(9.4));
                            ui.add(
                                egui::DragValue::new(&mut self.risk_plan_input.risk_pct)
                                    .range(0.05..=25.0)
                                    .speed(0.05)
                                    .fixed_decimals(2),
                            );
                        });

                        // Entry / Stop / Target + Tick Snap
                        let px_speed = if self.risk_plan_input.entry_price >= 1_000.0 {
                            50.0
                        } else {
                            0.05
                        };
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(4.0, 3.0);
                            ui.label(RichText::new("E:").monospace().size(9.4));
                            ui.add(
                                egui::DragValue::new(&mut self.risk_plan_input.entry_price)
                                    .range(0.01..=1.0e9)
                                    .speed(px_speed),
                            );
                            ui.label(
                                RichText::new("SL:")
                                    .monospace()
                                    .size(9.4)
                                    .color(palette::BEARISH),
                            );
                            ui.add(
                                egui::DragValue::new(&mut self.risk_plan_input.stop_loss)
                                    .range(0.01..=1.0e9)
                                    .speed(px_speed),
                            );
                            ui.label(
                                RichText::new("TP:")
                                    .monospace()
                                    .size(9.4)
                                    .color(palette::BULLISH),
                            );
                            ui.add(
                                egui::DragValue::new(&mut self.risk_plan_input.target_price)
                                    .range(0.01..=1.0e9)
                                    .speed(px_speed),
                            );
                        });

                        // Friction row (`Comm / Tax / Slip` + `Snap`)
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(4.0, 3.0);
                            ui.label(
                                RichText::new("Fee/Tax/Slp(bp):")
                                    .monospace()
                                    .size(9.1)
                                    .color(palette::TEXT_MUTED),
                            );
                            ui.add(
                                egui::DragValue::new(&mut self.risk_plan_input.commission_bps)
                                    .range(0.0..=500.0)
                                    .speed(0.5),
                            );
                            ui.add(
                                egui::DragValue::new(&mut self.risk_plan_input.tax_sell_bps)
                                    .range(0.0..=500.0)
                                    .speed(0.5),
                            );
                            ui.add(
                                egui::DragValue::new(&mut self.risk_plan_input.slippage_bps)
                                    .range(0.0..=500.0)
                                    .speed(0.5),
                            );
                            ui.checkbox(
                                &mut self.risk_plan_input.snap_to_ticks,
                                RichText::new("Tick").monospace().size(9.1),
                            );
                        });

                        // Action Bar (`From Box | Snap | Draw R:R | -> Ticket | + Log Plan`)
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(3.5, 3.0);
                            if ui
                                .small_button(RichText::new("From Box").monospace().size(9.2))
                                .clicked()
                            {
                                action_from_box = true;
                            }
                            if ui
                                .small_button(RichText::new("Snap Ticks").monospace().size(9.2))
                                .clicked()
                            {
                                action_snap_ticks = true;
                            }
                            if ui
                                .small_button(
                                    RichText::new("Draw R:R")
                                        .monospace()
                                        .strong()
                                        .size(9.2)
                                        .color(Color32::from_rgb(16, 185, 129)),
                                )
                                .clicked()
                            {
                                action_draw_rr = true;
                            }
                            if ui
                                .small_button(
                                    RichText::new("-> Ticket")
                                        .monospace()
                                        .strong()
                                        .size(9.2)
                                        .color(palette::TRADE_ENTRY_BLUE),
                                )
                                .clicked()
                            {
                                action_to_ticket = true;
                            }
                            if ui
                                .small_button(
                                    RichText::new("+ Log Plan")
                                        .monospace()
                                        .strong()
                                        .size(9.2)
                                        .color(palette::ALERT_ARMED_AMBER),
                                )
                                .clicked()
                            {
                                action_log_plan = true;
                            }
                        });

                        ui.separator();

                        // Computed Sizing Readout + Warnings
                        match self.compute_active_trade_plan() {
                            Ok(plan) => {
                                let r_col = if plan.net_r_multiple >= 2.0 {
                                    palette::BULLISH
                                } else if plan.net_r_multiple >= 1.0 {
                                    palette::ALERT_ARMED_AMBER
                                } else {
                                    palette::BEARISH
                                };
                                ui.horizontal_wrapped(|ui| {
                                    let mult_suffix = if plan.contract_multiplier > 1.0 + 1.0e-6 {
                                        format!(
                                            " (x{:.0} · Mgn {:.0})",
                                            plan.contract_multiplier, plan.margin_required
                                        )
                                    } else {
                                        String::new()
                                    };
                                    ui.label(
                                        RichText::new(format!(
                                            "Qty {} · Notional {:.0}{mult_suffix}",
                                            format_clean_qty(plan.planned_quantity),
                                            plan.position_notional
                                        ))
                                        .monospace()
                                        .strong()
                                        .size(9.5)
                                        .color(palette::TEXT_PRIMARY),
                                    );
                                    ui.label(
                                        RichText::new(format!(
                                            "· Net {:.2}R (Gross {:.2}R)",
                                            plan.net_r_multiple, plan.gross_r_multiple
                                        ))
                                        .monospace()
                                        .strong()
                                        .size(9.4)
                                        .color(r_col),
                                    );
                                });
                                let settle_lbl = plan.settlement_timestamp.map_or_else(
                                    || format!("T+{}", plan.venue.settlement_business_days()),
                                    |ts| {
                                        let dt = crate::data::format_utc_datetime_hm(ts);
                                        format!(
                                            "T+{} ({})",
                                            plan.venue.settlement_business_days(),
                                            dt.get(..10).unwrap_or(&dt)
                                        )
                                    },
                                );
                                ui.label(
                                    RichText::new(format!(
                                        "All-In Risk {:.0} ({:.2}%) · BE {} · Win>= {:.1}% · {settle_lbl}",
                                        plan.planned_risk_amount,
                                        plan.planned_risk_pct_actual,
                                        format_venue_price(plan.breakeven_price, plan.venue),
                                        plan.required_win_rate_pct
                                    ))
                                    .monospace()
                                    .size(9.1)
                                    .color(palette::TEXT_MUTED),
                                );
                                for warn in &plan.warnings {
                                    ui.label(
                                        RichText::new(format!("! {}", warn.description()))
                                            .monospace()
                                            .size(9.0)
                                            .color(palette::ALERT_ARMED_AMBER),
                                    );
                                }
                            }
                            Err(err) => {
                                ui.label(
                                    RichText::new(err.message())
                                        .monospace()
                                        .size(9.2)
                                        .color(palette::BEARISH),
                                );
                            }
                        }
                    });

                // Card 2: Structured Trade Journal & Discipline Analytics (`DEF-001` Causal)
                Frame::new()
                    .fill(Color32::from_rgb(17, 22, 32))
                    .stroke(Stroke::new(1.0, palette::BORDER_SUBTLE))
                    .corner_radius(4.0)
                    .inner_margin(Margin::symmetric(6, 4))
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(4.0, 2.8);

                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(3.5, 2.5);
                            ui.label(
                                RichText::new("TRADE JOURNAL")
                                    .monospace()
                                    .strong()
                                    .size(9.8)
                                    .color(palette::TEXT_PRIMARY),
                            );
                            if ui
                                .small_button(RichText::new("Sync Paper").monospace().size(9.0))
                                .clicked()
                            {
                                action_sync_paper = true;
                            }
                            if ui
                                .small_button(RichText::new("Import BT").monospace().size(9.0))
                                .clicked()
                            {
                                action_import_bt = true;
                            }
                            if ui
                                .small_button(RichText::new("Copy CSV").monospace().size(9.0))
                                .clicked()
                            {
                                action_copy_csv = true;
                            }
                            if ui
                                .small_button(RichText::new("Copy MD").monospace().size(9.0))
                                .clicked()
                            {
                                action_copy_md = true;
                            }
                        });

                        let stats = self.journal.compute_analytics(replay_cutoff);
                        let pnl_c = if stats.total_net_pnl >= 0.0 {
                            palette::BULLISH
                        } else {
                            palette::BEARISH
                        };
                        let exp_s = stats
                            .expectancy_r
                            .map_or_else(|| "-".to_owned(), |v| format!("{v:+.2}R"));
                        let pf_s = if let Some(pf) = stats.profit_factor {
                            format!("{pf:.2}")
                        } else if stats.win_count > 0 && stats.loss_count == 0 {
                            "∞".to_owned()
                        } else {
                            "-".to_owned()
                        };
                        let sqn_s = stats
                            .sqn
                            .map_or_else(|| "-".to_owned(), |v| format!("{v:.2}"));
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                RichText::new(format!(
                                    "Vis {} ({}C/{}P) · Win {:.0}% · PF {pf_s} · Net {:+.1}",
                                    stats.visible_entry_count,
                                    stats.closed_count,
                                    stats.planned_count,
                                    stats.win_rate_pct,
                                    stats.total_net_pnl
                                ))
                                .monospace()
                                .strong()
                                .size(9.2)
                                .color(pnl_c),
                            );
                        });
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                RichText::new(format!(
                                    "Exp {exp_s} · SQN {sqn_s} · Disc {:.0}% · Mistake Cost -{:.1}",
                                    stats.discipline_score_pct, stats.cost_of_mistakes
                                ))
                                .monospace()
                                .size(9.1)
                                .color(palette::TEXT_MUTED),
                            );
                        });

                        if !stats.setup_attribution.is_empty() {
                            let top_summary = stats
                                .setup_attribution
                                .iter()
                                .take(3)
                                .map(|b| {
                                    format!(
                                        "{}:{}({:+.0})",
                                        b.setup.short_label(),
                                        b.trade_count,
                                        b.net_pnl
                                    )
                                })
                                .collect::<Vec<_>>()
                                .join(" · ");
                            ui.label(
                                RichText::new(format!("Setups: {top_summary}"))
                                    .monospace()
                                    .size(8.9)
                                    .color(Color32::from_rgb(148, 163, 184)),
                            );
                        }

                        // Visible Entry List (strictly `DEF-001` causal)
                        struct VisibleJournalRow {
                            id: u64,
                            sym: String,
                            side: PositionSide,
                            status: JournalEntryStatus,
                            pnl: f64,
                            r_val: Option<f64>,
                            setup: SetupTag,
                            mistake: MistakeTag,
                        }
                        let visible_snapshots: Vec<VisibleJournalRow> = self
                            .journal
                            .visible_entries(replay_cutoff)
                            .into_iter()
                            .rev()
                            .take(24)
                            .map(|e| VisibleJournalRow {
                                id: e.id,
                                sym: e.symbol.clone(),
                                side: e.side,
                                status: e.status,
                                pnl: e.net_pnl,
                                r_val: e.realized_r.or(e.planned_net_r),
                                setup: e.setup,
                                mistake: e.mistake,
                            })
                            .collect();

                        let selected_visible_id = self
                            .journal
                            .selected_entry_id
                            .filter(|id| visible_snapshots.iter().any(|r| r.id == *id))
                            .or_else(|| visible_snapshots.first().map(|r| r.id));

                        if !visible_snapshots.is_empty() {
                            ui.separator();
                            egui::ScrollArea::vertical()
                                .id_salt("sumi_journal_entries_list")
                                .max_height(115.0)
                                .show(ui, |ui| {
                                    for row in &visible_snapshots {
                                        let is_sel = selected_visible_id == Some(row.id);
                                        let r_txt = row
                                            .r_val
                                            .map_or_else(|| "-".to_owned(), |r| format!("{r:+.2}R"));
                                        let row_txt = match row.status {
                                            JournalEntryStatus::Planned => format!(
                                                "#{} {} {} PLAN {r_txt} [{}]",
                                                row.id,
                                                row.sym,
                                                row.side.short_label(),
                                                row.setup.short_label()
                                            ),
                                            JournalEntryStatus::Closed => {
                                                let m_flag = if row.mistake != MistakeTag::None {
                                                    " !"
                                                } else {
                                                    ""
                                                };
                                                format!(
                                                    "#{} {} {} {:+.1} ({r_txt}) [{}]{m_flag}",
                                                    row.id,
                                                    row.sym,
                                                    row.side.short_label(),
                                                    row.pnl,
                                                    row.setup.short_label()
                                                )
                                            }
                                        };
                                        let row_col = match row.status {
                                            JournalEntryStatus::Planned => {
                                                palette::ALERT_ARMED_AMBER
                                            }
                                            JournalEntryStatus::Closed if row.pnl >= 0.0 => {
                                                palette::BULLISH
                                            }
                                            JournalEntryStatus::Closed => palette::BEARISH,
                                        };
                                        if ui
                                            .selectable_label(
                                                is_sel,
                                                RichText::new(row_txt)
                                                    .monospace()
                                                    .size(9.0)
                                                    .color(row_col),
                                            )
                                            .clicked()
                                        {
                                            self.journal.selected_entry_id = Some(row.id);
                                        }
                                    }
                                });
                        }

                        ui.separator();

                        // Inline Debrief & Tagging Editor (edits selected visible entry, or draft plan tags)
                        if let Some(sel_id) = selected_visible_id
                            && let Some(entry) = self.journal.entry_mut(sel_id)
                        {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(format!(
                                        "Edit #{sel_id} ({} · {})",
                                        entry.symbol,
                                        entry.source.kind_label()
                                    ))
                                    .monospace()
                                    .strong()
                                    .size(9.3)
                                    .color(palette::TEXT_PRIMARY),
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui
                                            .small_button(
                                                RichText::new("Del")
                                                    .monospace()
                                                    .size(9.0)
                                                    .color(palette::BEARISH),
                                            )
                                            .clicked()
                                        {
                                            delete_entry_req = Some(sel_id);
                                        }
                                    },
                                );
                            });

                            ui.horizontal_wrapped(|ui| {
                                ui.spacing_mut().item_spacing = Vec2::new(4.0, 3.0);
                                egui::ComboBox::from_id_salt("sumi_jrn_setup_combo")
                                    .selected_text(
                                        RichText::new(entry.setup.short_label())
                                            .monospace()
                                            .size(9.2),
                                    )
                                    .width(105.0)
                                    .show_ui(ui, |ui| {
                                        for tag in SetupTag::ALL {
                                            if ui
                                                .selectable_label(
                                                    entry.setup == tag,
                                                    tag.short_label(),
                                                )
                                                .clicked()
                                            {
                                                entry.setup = tag;
                                            }
                                        }
                                    });
                                egui::ComboBox::from_id_salt("sumi_jrn_mistake_combo")
                                    .selected_text(
                                        RichText::new(entry.mistake.short_label())
                                            .monospace()
                                            .size(9.2),
                                    )
                                    .width(115.0)
                                    .show_ui(ui, |ui| {
                                        for m in MistakeTag::ALL {
                                            if ui
                                                .selectable_label(
                                                    entry.mistake == m,
                                                    m.short_label(),
                                                )
                                                .clicked()
                                            {
                                                entry.mistake = m;
                                            }
                                        }
                                    });
                                ui.label(RichText::new("Conv:").monospace().size(9.0));
                                for c in 1..=5_u8 {
                                    if ui
                                        .selectable_label(
                                            entry.conviction == c,
                                            RichText::new(format!("{c}")).monospace().size(9.0),
                                        )
                                        .clicked()
                                    {
                                        entry.conviction = c;
                                    }
                                }
                            });

                            ui.horizontal_wrapped(|ui| {
                                ui.spacing_mut().item_spacing = Vec2::new(3.5, 2.0);
                                ui.checkbox(
                                    &mut entry.checklist.trend_aligned,
                                    RichText::new("Trd").monospace().size(8.9),
                                );
                                ui.checkbox(
                                    &mut entry.checklist.level_confirmed,
                                    RichText::new("Lvl").monospace().size(8.9),
                                );
                                ui.checkbox(
                                    &mut entry.checklist.catalyst_or_volume_ok,
                                    RichText::new("Vol").monospace().size(8.9),
                                );
                                ui.checkbox(
                                    &mut entry.checklist.stop_and_lot_sized,
                                    RichText::new("Lot").monospace().size(8.9),
                                );
                                ui.checkbox(
                                    &mut entry.checklist.band_and_t2_checked,
                                    RichText::new("T+2").monospace().size(8.9),
                                );
                                ui.checkbox(
                                    &mut entry.checklist.calm_state,
                                    RichText::new("Calm").monospace().size(8.9),
                                );
                            });

                            if ui
                                .add(
                                    egui::TextEdit::singleline(&mut entry.notes)
                                        .hint_text("Post-trade debrief note...")
                                        .desired_width((ui.available_width() - 4.0).max(120.0))
                                        .font(egui::FontId::monospace(9.2)),
                                )
                                .changed()
                            {
                                truncate_utf8_bytes(
                                    &mut entry.notes,
                                    crate::journal::MAX_JOURNAL_NOTE_BYTES,
                                );
                            }
                        } else {
                            ui.label(
                                RichText::new("Draft Plan Checklist & Setup:")
                                    .monospace()
                                    .size(9.2)
                                    .color(palette::TEXT_MUTED),
                            );
                            ui.horizontal_wrapped(|ui| {
                                egui::ComboBox::from_id_salt("sumi_jrn_draft_setup")
                                    .selected_text(
                                        RichText::new(self.journal_draft_setup.short_label())
                                            .monospace()
                                            .size(9.2),
                                    )
                                    .width(110.0)
                                    .show_ui(ui, |ui| {
                                        for tag in SetupTag::ALL {
                                            if ui
                                                .selectable_label(
                                                    self.journal_draft_setup == tag,
                                                    tag.short_label(),
                                                )
                                                .clicked()
                                            {
                                                self.journal_draft_setup = tag;
                                            }
                                        }
                                    });
                                ui.label(RichText::new("Conv:").monospace().size(9.0));
                                for c in 1..=5_u8 {
                                    if ui
                                        .selectable_label(
                                            self.journal_draft_conviction == c,
                                            RichText::new(format!("{c}")).monospace().size(9.0),
                                        )
                                        .clicked()
                                    {
                                        self.journal_draft_conviction = c;
                                    }
                                }
                            });
                        }
                    });
            });

        if let Some(del_id) = delete_entry_req {
            let _ = self.journal.delete_entry(del_id);
        }
        if action_from_box && !self.sync_planner_from_selected_rr_box() {
            self.last_journal_status = Some((
                true,
                "select an on-chart Pos R:R Box first to pull Entry / SL / TP".to_owned(),
            ));
        }
        if action_snap_ticks {
            let _ = self.snap_planner_ticks_and_sync_rr_box();
        }
        if action_draw_rr {
            let _ = self.draw_or_update_rr_box_from_planner();
        }
        if action_to_ticket {
            let _ = self.sync_ticket_from_risk_planner();
        }
        if action_log_plan {
            let _ = self.log_active_plan_to_journal();
        }
        if action_sync_paper {
            let _ = self.sync_paper_trades_to_journal();
        }
        if action_import_bt {
            let _ = self.import_backtest_trades_to_journal(12);
        }
        if action_copy_csv {
            let csv = self.export_journal_csv_causal();
            ui.ctx().copy_text(csv);
        }
        if action_copy_md {
            let md = self.export_journal_markdown_causal();
            ui.ctx().copy_text(md);
        }
    }

    fn render_alerts_panel(&mut self, ui: &mut egui::Ui) {
        self.sync_alerts_causal();
        let cutoff = self.replay.cutoff_timestamp(&self.base_candles);
        let mark = self.current_mark_price();

        egui::ScrollArea::vertical()
            .id_salt("sumi_alerts_panel_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(5.0, 6.0);

                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("CAUSAL ALERTS")
                            .monospace()
                            .strong()
                            .size(12.0)
                            .color(palette::TEXT_PRIMARY),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if !self.alerts.is_empty()
                            && ui
                                .button(RichText::new("Clear All").monospace().size(10.0))
                                .clicked()
                        {
                            self.alerts.clear_all();
                            self.alerts_synced_key = None;
                            self.sync_alerts_causal();
                        }
                    });
                });

                // 1. Quick-Arm Alert Builder Card
                Frame::new()
                    .fill(Color32::from_rgb(19, 24, 35))
                    .corner_radius(CornerRadius::same(4))
                    .stroke(Stroke::new(1.0, palette::BORDER_SUBTLE))
                    .inner_margin(Margin::symmetric(8, 6))
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            for kind in AlertDraftKind::ALL {
                                if ui
                                    .selectable_label(
                                        self.alert_draft_kind == kind,
                                        RichText::new(kind.short_label()).monospace().size(10.2),
                                    )
                                    .clicked()
                                {
                                    self.alert_draft_kind = kind;
                                    if kind == AlertDraftKind::PriceLevel
                                        && self.alert_value_input.trim().is_empty()
                                        && mark > 0.0
                                    {
                                        self.alert_value_input = format!("{mark:.2}");
                                    } else if kind == AlertDraftKind::RsiThreshold
                                        && self.alert_value_input.trim().is_empty()
                                    {
                                        self.alert_value_input = "30.0".to_owned();
                                    }
                                }
                            }
                        });

                        ui.horizontal_wrapped(|ui| {
                            for dir in CrossDirection::ALL {
                                if ui
                                    .selectable_label(
                                        self.alert_direction == dir,
                                        RichText::new(dir.short_label()).monospace().size(10.0),
                                    )
                                    .clicked()
                                {
                                    self.alert_direction = dir;
                                }
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("Freq:")
                                    .monospace()
                                    .size(10.0)
                                    .color(palette::TEXT_MUTED),
                            );
                            for freq in AlertFrequency::ALL {
                                if ui
                                    .selectable_label(
                                        self.alert_frequency == freq,
                                        RichText::new(freq.short_label()).monospace().size(10.0),
                                    )
                                    .clicked()
                                {
                                    self.alert_frequency = freq;
                                }
                            }
                        });

                        match self.alert_draft_kind {
                            AlertDraftKind::PriceLevel | AlertDraftKind::RsiThreshold => {
                                ui.horizontal(|ui| {
                                    let lbl = if self.alert_draft_kind == AlertDraftKind::PriceLevel
                                    {
                                        "Level:"
                                    } else {
                                        "RSI:"
                                    };
                                    ui.label(
                                        RichText::new(lbl)
                                            .monospace()
                                            .size(10.2)
                                            .color(palette::TEXT_MUTED),
                                    );
                                    ui.add(
                                        egui::TextEdit::singleline(&mut self.alert_value_input)
                                            .desired_width(92.0)
                                            .font(egui::FontId::monospace(10.8)),
                                    );
                                    if ui
                                        .button(RichText::new("@Mark").monospace().size(9.8))
                                        .clicked()
                                    {
                                        if self.alert_draft_kind == AlertDraftKind::PriceLevel
                                            && mark > 0.0
                                        {
                                            self.alert_value_input = format!("{mark:.2}");
                                        } else if self.alert_draft_kind
                                            == AlertDraftKind::RsiThreshold
                                        {
                                            self.alert_value_input = match self.alert_direction {
                                                CrossDirection::CrossBelow => "70.0".to_owned(),
                                                _ => "30.0".to_owned(),
                                            };
                                        }
                                    }
                                });
                            }
                            AlertDraftKind::MacdZeroCross => {
                                ui.label(
                                    RichText::new("MACD(12, 26, 9) Histogram 0-Cross")
                                        .monospace()
                                        .size(10.0)
                                        .color(palette::TEXT_MUTED),
                                );
                            }
                            AlertDraftKind::SelectedDrawing => {
                                let status = match self.drawings.selected_drawing() {
                                    Some(d) if d.pane == DrawingPaneTarget::Main => format!(
                                        "Selected: #{} ({})",
                                        d.id,
                                        d.geometry.tool_kind().short_label()
                                    ),
                                    _ => "Select a Main-pane Line or Ray".to_owned(),
                                };
                                ui.label(
                                    RichText::new(status)
                                        .monospace()
                                        .size(10.0)
                                        .color(palette::ALERT_ARMED_AMBER),
                                );
                            }
                        }

                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("Note:")
                                    .monospace()
                                    .size(10.0)
                                    .color(palette::TEXT_MUTED),
                            );
                            ui.add(
                                egui::TextEdit::singleline(&mut self.alert_message_input)
                                    .desired_width((ui.available_width() - 4.0).max(60.0))
                                    .hint_text("optional alert note")
                                    .font(egui::FontId::monospace(10.2)),
                            );
                        });

                        let arm_btn = Button::new(
                            RichText::new("+ Arm Causal Alert")
                                .monospace()
                                .strong()
                                .size(10.8)
                                .color(palette::ALERT_AXIS_TEXT_DARK),
                        )
                        .fill(palette::ALERT_ARMED_AMBER)
                        .min_size(Vec2::new((ui.available_width() - 2.0).max(80.0), 25.0));
                        if ui.add(arm_btn).clicked() {
                            let _ = self.submit_draft_alert();
                        }

                        if let Some((is_err, ref msg)) = self.last_alert_status {
                            ui.label(RichText::new(msg).monospace().size(9.8).color(if is_err {
                                palette::BEARISH
                            } else {
                                palette::BULLISH
                            }));
                        }
                    });

                // 2. Active, Triggered & Paused Rules List
                let visible_rules: Vec<_> = self
                    .alerts
                    .rules
                    .iter()
                    .filter(|r| r.is_causally_visible(cutoff))
                    .cloned()
                    .collect();

                let mut toggle_id: Option<AlertId> = None;
                let mut remove_id: Option<AlertId> = None;
                Frame::new()
                    .fill(Color32::from_rgb(19, 24, 35))
                    .corner_radius(CornerRadius::same(4))
                    .stroke(Stroke::new(1.0, palette::BORDER_SUBTLE))
                    .inner_margin(Margin::symmetric(8, 6))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(format!("ALERT RULES ({})", visible_rules.len()))
                                .monospace()
                                .strong()
                                .size(10.5)
                                .color(palette::TEXT_MUTED),
                        );
                        if visible_rules.is_empty() {
                            ui.label(
                                RichText::new("No causal alert rules armed.")
                                    .monospace()
                                    .size(9.8)
                                    .color(palette::TEXT_MUTED),
                            );
                        }
                        for rule in &visible_rules {
                            let badge_color = match rule.status {
                                AlertStatus::Armed => palette::ALERT_ARMED_AMBER,
                                AlertStatus::Triggered => palette::ALERT_TRIGGERED_GOLD,
                                AlertStatus::Paused => palette::TEXT_MUTED,
                            };
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(format!(
                                        "#{} {}",
                                        rule.id,
                                        rule.status.short_label()
                                    ))
                                    .monospace()
                                    .strong()
                                    .size(9.8)
                                    .color(badge_color),
                                );
                                ui.label(
                                    RichText::new(format!(
                                        "{} · ×{}",
                                        rule.frequency.short_label(),
                                        rule.trigger_count
                                    ))
                                    .monospace()
                                    .size(9.5)
                                    .color(palette::TEXT_MUTED),
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui
                                            .button(RichText::new("×").monospace().size(9.8))
                                            .clicked()
                                        {
                                            remove_id = Some(rule.id);
                                        }
                                        let pause_lbl = if rule.status == AlertStatus::Paused {
                                            "Arm"
                                        } else {
                                            "Pause"
                                        };
                                        if ui
                                            .button(RichText::new(pause_lbl).monospace().size(9.5))
                                            .clicked()
                                        {
                                            toggle_id = Some(rule.id);
                                        }
                                    },
                                );
                            });
                            ui.label(
                                RichText::new(format!(
                                    "{} ({}) · {}",
                                    rule.symbol,
                                    rule.timeframe.label(),
                                    rule.condition.summary()
                                ))
                                .monospace()
                                .size(9.8)
                                .color(palette::TEXT_PRIMARY),
                            );
                            ui.separator();
                        }
                    });

                if let Some(id) = toggle_id {
                    self.alerts.toggle_pause(id);
                    self.alerts_synced_key = None;
                    self.sync_alerts_causal();
                }
                if let Some(id) = remove_id {
                    self.alerts.remove_alert(id);
                    self.alerts_synced_key = None;
                    self.sync_alerts_causal();
                }

                // 3. Causal Trigger Log (newest-first)
                let visible_triggers: Vec<_> = self
                    .alerts
                    .triggers
                    .iter()
                    .filter(|ev| cutoff.is_none_or(|cut| alert_event_eval_timestamp(ev) <= cut))
                    .collect();
                Frame::new()
                    .fill(Color32::from_rgb(19, 24, 35))
                    .corner_radius(CornerRadius::same(4))
                    .stroke(Stroke::new(1.0, palette::BORDER_SUBTLE))
                    .inner_margin(Margin::symmetric(8, 6))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(format!(
                                "CAUSAL TRIGGER LOG ({})",
                                visible_triggers.len()
                            ))
                            .monospace()
                            .strong()
                            .size(10.5)
                            .color(palette::ALERT_TRIGGERED_GOLD),
                        );
                        if visible_triggers.is_empty() {
                            ui.label(
                                RichText::new("Step Replay forward to fire armed alerts.")
                                    .monospace()
                                    .size(9.8)
                                    .color(palette::TEXT_MUTED),
                            );
                        }
                        for ev in visible_triggers.into_iter().rev().take(10) {
                            let ev_ts = alert_event_eval_timestamp(ev);
                            let ts_lbl = if ev.timeframe.is_intraday() {
                                format_utc_datetime_hm(ev_ts)
                            } else {
                                format_utc_date(ev_ts)
                            };
                            ui.label(
                                RichText::new(format!(
                                    "⚡ #{} {} · {}",
                                    ev.alert_id, ev.symbol, ev.condition_summary
                                ))
                                .monospace()
                                .strong()
                                .size(9.8)
                                .color(palette::ALERT_TRIGGERED_GOLD),
                            );
                            ui.label(
                                RichText::new(format!("Obs {:.2} @ {ts_lbl}", ev.observed_value))
                                    .monospace()
                                    .size(9.4)
                                    .color(palette::TEXT_MUTED),
                            );
                        }
                    });
            });
    }

    fn render_command_palette_modal(&mut self, ui: &mut egui::Ui) {
        let mut candidates = self.build_command_palette_candidates();
        if candidates.is_empty() {
            self.command_palette.selected_index = 0;
        } else if self.command_palette.selected_index >= candidates.len() {
            self.command_palette.selected_index = candidates.len() - 1;
        }

        // Consume modal navigation & execution keys BEFORE `TextEdit` processes input!
        let (esc, nav_down, nav_up, enter) = ui.input_mut(|i| {
            (
                i.consume_key(egui::Modifiers::NONE, egui::Key::Escape),
                i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown),
                i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp),
                i.consume_key(egui::Modifiers::NONE, egui::Key::Enter),
            )
        });

        if esc {
            self.close_command_palette();
            ui.ctx().request_repaint();
            return;
        }
        let mut nav_changed = false;
        if !candidates.is_empty() {
            if nav_down {
                self.command_palette.selected_index =
                    (self.command_palette.selected_index + 1) % candidates.len();
                nav_changed = true;
            } else if nav_up {
                self.command_palette.selected_index = if self.command_palette.selected_index == 0 {
                    candidates.len() - 1
                } else {
                    self.command_palette.selected_index - 1
                };
                nav_changed = true;
            }
        }
        if enter {
            if let Some(action) = candidates
                .get(self.command_palette.selected_index)
                .map(|c| c.action.clone())
            {
                self.execute_palette_action(action);
            } else {
                self.close_command_palette();
            }
            ui.ctx().request_repaint();
            return;
        }

        let vp_rect = ui.input(|i| i.viewport_rect());

        // 1. Translucent dark backdrop scrim (`Order::Middle`)
        let scrim_resp = egui::Area::new(egui::Id::new("sumi_cmd_k_scrim"))
            .order(egui::Order::Middle)
            .fixed_pos(vp_rect.min)
            .show(ui.ctx(), |ui| {
                let (rect, resp) =
                    ui.allocate_exact_size(vp_rect.size(), egui::Sense::click_and_drag());
                ui.painter()
                    .rect_filled(rect, 0.0, Color32::from_rgba_unmultiplied(6, 9, 15, 182));
                resp
            })
            .inner;

        // 2. Centered floating Command Palette card (`Order::Foreground`)
        let modal_w = (vp_rect.width() - 48.0).clamp(320.0, 640.0);
        let modal_pos = egui::Pos2::new(
            vp_rect.center().x - modal_w * 0.5,
            vp_rect.top() + (vp_rect.height() * 0.12).clamp(32.0, 96.0),
        );

        let mut clicked_action: Option<PaletteAction> = None;
        let modal_area = egui::Area::new(egui::Id::new("sumi_cmd_k_modal"))
            .order(egui::Order::Foreground)
            .fixed_pos(modal_pos)
            .show(ui.ctx(), |ui| {
                Frame::new()
                    .fill(Color32::from_rgb(17, 22, 33))
                    .corner_radius(CornerRadius::same(8))
                    .stroke(Stroke::new(1.5, palette::ACCENT_BLUE))
                    .inner_margin(Margin::symmetric(12, 10))
                    .show(ui, |ui| {
                        ui.set_width(modal_w - 24.0);
                        ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);

                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("CMD+K COMMAND PALETTE")
                                    .monospace()
                                    .strong()
                                    .size(11.5)
                                    .color(Color32::from_rgb(120, 200, 255)),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        RichText::new("↑↓ Select · Enter Run · Esc Close")
                                            .monospace()
                                            .size(9.8)
                                            .color(palette::TEXT_MUTED),
                                    );
                                },
                            );
                        });

                        let prev_q = self.command_palette.query.clone();
                        let edit_resp = ui.add(
                            egui::TextEdit::singleline(&mut self.command_palette.query)
                                .desired_width(ui.available_width())
                                .hint_text(
                                    "Search symbol (FPT, BTC), timeframe (15m, 4h), tool, or 'alert 68500'...",
                                )
                                .font(egui::FontId::monospace(12.0)),
                        );
                        truncate_utf8_bytes(&mut self.command_palette.query, 128);
                        if self.command_palette.request_focus || !edit_resp.has_focus() {
                            edit_resp.request_focus();
                            self.command_palette.request_focus = false;
                        }
                        if self.command_palette.query != prev_q {
                            self.command_palette.selected_index = 0;
                            candidates = self.build_command_palette_candidates();
                        }

                        ui.separator();

                        egui::ScrollArea::vertical()
                            .id_salt("sumi_cmd_k_results_scroll")
                            .max_height(300.0)
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                ui.spacing_mut().item_spacing = Vec2::new(4.0, 3.0);
                                if candidates.is_empty() {
                                    ui.label(
                                        RichText::new("No matching workstation commands or symbols.")
                                            .monospace()
                                            .size(10.5)
                                            .color(palette::TEXT_MUTED),
                                    );
                                }
                                for (idx, cand) in candidates.iter().enumerate() {
                                    let is_sel = idx == self.command_palette.selected_index;
                                    let row_fill = if is_sel {
                                        palette::ACCENT_BLUE.gamma_multiply(0.24)
                                    } else {
                                        Color32::from_rgb(22, 27, 39)
                                    };
                                    let row_stroke = if is_sel {
                                        Stroke::new(1.0, palette::ACCENT_BLUE)
                                    } else {
                                        Stroke::new(1.0, Color32::TRANSPARENT)
                                    };
                                    let row_frame = Frame::new()
                                        .fill(row_fill)
                                        .corner_radius(CornerRadius::same(4))
                                        .stroke(row_stroke)
                                        .inner_margin(Margin::symmetric(8, 5))
                                        .show(ui, |ui| {
                                            ui.set_min_width(ui.available_width());
                                            ui.horizontal(|ui| {
                                                let cat_c = cand.category.badge_color();
                                                Frame::new()
                                                    .fill(cat_c.gamma_multiply(0.18))
                                                    .stroke(Stroke::new(
                                                        1.0,
                                                        cat_c.gamma_multiply(0.75),
                                                    ))
                                                    .corner_radius(CornerRadius::same(3))
                                                    .inner_margin(Margin::symmetric(5, 1))
                                                    .show(ui, |ui| {
                                                        ui.label(
                                                            RichText::new(
                                                                cand.category.badge_label(),
                                                            )
                                                            .monospace()
                                                            .strong()
                                                            .size(9.2)
                                                            .color(cat_c),
                                                        );
                                                    });

                                                ui.label(
                                                    RichText::new(&cand.title)
                                                        .monospace()
                                                        .strong()
                                                        .size(11.0)
                                                        .color(if is_sel {
                                                            Color32::WHITE
                                                        } else {
                                                            palette::TEXT_PRIMARY
                                                        }),
                                                );

                                                ui.with_layout(
                                                    egui::Layout::right_to_left(
                                                        egui::Align::Center,
                                                    ),
                                                    |ui| {
                                                        ui.label(
                                                            RichText::new(&cand.subtitle)
                                                                .monospace()
                                                                .size(9.6)
                                                                .color(palette::TEXT_MUTED),
                                                        );
                                                    },
                                                );
                                            });
                                        });

                                    if is_sel && nav_changed {
                                        row_frame.response.scroll_to_me(Some(egui::Align::Center));
                                    }
                                    let row_click = ui.interact(
                                        row_frame.response.rect,
                                        ui.make_persistent_id(("cmd_k_row", idx)),
                                        egui::Sense::click(),
                                    );
                                    if row_click.hovered() {
                                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                    }
                                    if row_click.clicked() {
                                        clicked_action = Some(cand.action.clone());
                                    }
                                }
                            });
                    })
            });

        if let Some(action) = clicked_action {
            self.execute_palette_action(action);
            ui.ctx().request_repaint();
        } else if scrim_resp.clicked()
            && ui.input(|i| {
                i.pointer
                    .interact_pos()
                    .is_none_or(|pos| !modal_area.response.rect.contains(pos))
            })
        {
            self.close_command_palette();
            ui.ctx().request_repaint();
        }
    }
}

#[inline]
fn alert_event_eval_timestamp(ev: &crate::alerts::AlertTriggerEvent) -> i64 {
    if ev.eval_base_timestamp != 0 {
        ev.eval_base_timestamp
    } else {
        ev.bar_timestamp
    }
}

#[inline]
fn command_palette_modal_rect_estimate(vp_rect: egui::Rect) -> egui::Rect {
    let modal_w = (vp_rect.width() - 48.0).clamp(320.0, 640.0);
    let modal_pos = egui::Pos2::new(
        vp_rect.center().x - modal_w * 0.5,
        vp_rect.top() + (vp_rect.height() * 0.12).clamp(32.0, 96.0),
    );
    egui::Rect::from_min_size(modal_pos, Vec2::new(modal_w, 410.0))
}

fn regime_badge_color(regime: MarketRegime) -> Color32 {
    match regime {
        MarketRegime::Uptrend => palette::BULLISH,
        MarketRegime::Downtrend => palette::BEARISH,
        MarketRegime::Pullback => palette::ALERT_ARMED_AMBER,
        MarketRegime::Recovery => Color32::from_rgb(77, 208, 225),
        MarketRegime::SidewaysSqueeze => Color32::from_rgb(179, 136, 255),
        MarketRegime::Neutral | MarketRegime::InsufficientHistory => palette::TEXT_MUTED,
    }
}

fn robustness_badge_color(verdict: RobustnessVerdict) -> Color32 {
    match verdict {
        RobustnessVerdict::Robust => palette::EQUITY_ABOVE_LINE,
        RobustnessVerdict::Stable => Color32::from_rgb(77, 208, 225),
        RobustnessVerdict::Degraded | RobustnessVerdict::LowSample => palette::ALERT_ARMED_AMBER,
        RobustnessVerdict::Overfitted | RobustnessVerdict::Unprofitable => {
            palette::EQUITY_BELOW_LINE
        }
    }
}

fn format_scanner_price(price: f64) -> String {
    if !price.is_finite() {
        "-".to_owned()
    } else if price.abs() >= 1_000.0 {
        format!("{price:.1}")
    } else {
        format!("{price:.2}")
    }
}

#[cfg(not(target_arch = "wasm32"))]
struct ThreadWaker(std::thread::Thread);

#[cfg(not(target_arch = "wasm32"))]
impl std::task::Wake for ThreadWaker {
    fn wake(self: std::sync::Arc<Self>) {
        self.0.unpark();
    }

    fn wake_by_ref(self: &std::sync::Arc<Self>) {
        self.0.unpark();
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn block_on_std<F: std::future::Future>(fut: F) -> F::Output {
    let waker = std::task::Waker::from(std::sync::Arc::new(ThreadWaker(std::thread::current())));
    let mut cx = std::task::Context::from_waker(&waker);
    let mut fut = std::pin::pin!(fut);
    loop {
        match fut.as_mut().poll(&mut cx) {
            std::task::Poll::Ready(val) => return val,
            std::task::Poll::Pending => std::thread::park(),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
struct OffscreenWgpuHarness {
    render_state: eframe::egui_wgpu::RenderState,
    ctx: egui::Context,
}

#[cfg(not(target_arch = "wasm32"))]
impl OffscreenWgpuHarness {
    fn new() -> Result<Self, String> {
        use eframe::egui_wgpu::{self, RendererOptions, WgpuConfiguration};

        let wgpu_config = WgpuConfiguration::default();
        let instance = block_on_std(wgpu_config.wgpu_setup.new_instance());
        let render_state = block_on_std(egui_wgpu::RenderState::create(
            &wgpu_config,
            &instance,
            None,
            RendererOptions::default(),
        ))
        .map_err(|err| format!("failed to create offscreen wgpu RenderState: {err}"))?;

        let info = render_state.adapter.get_info();
        log::info!(
            "Initialized offscreen GPU verification renderer on '{}' ({:?}, target_format={:?})",
            info.name,
            info.backend,
            render_state.target_format
        );

        let ctx = egui::Context::default();
        SumiWorkstationApp::configure_dark_workstation_visuals(&ctx);
        ctx.set_pixels_per_point(2.0);
        Ok(Self { render_state, ctx })
    }

    fn capture_app_frame(
        &self,
        app: &mut SumiWorkstationApp,
        logical_width: u32,
        logical_height: u32,
        settle_passes: usize,
    ) -> Result<egui::ColorImage, String> {
        use eframe::egui_wgpu::{ScreenDescriptor, wgpu};

        let ppp = 2.0_f32;
        self.ctx.set_pixels_per_point(ppp);
        let screen_rect = Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            Vec2::new(logical_width as f32, logical_height as f32),
        ));
        let width_px = logical_width.saturating_mul(2).max(2);
        let height_px = logical_height.saturating_mul(2).max(2);
        let total_passes = settle_passes.max(2);

        let device = &self.render_state.device;
        let queue = &self.render_state.queue;

        let mut final_output = None;
        for pass_idx in 0..total_passes {
            let raw_input = egui::RawInput {
                screen_rect,
                time: Some((app.frame_count as f64) * (1.0 / 60.0)),
                ..Default::default()
            };
            let full_out = self.ctx.run_ui(raw_input, |ui| {
                app.render_workstation_ui(ui);
            });

            {
                let mut renderer = self.render_state.renderer.write();
                for (id, deltas) in &full_out.textures_delta.set {
                    for delta in deltas {
                        renderer.update_texture(device, queue, *id, delta);
                    }
                }
            }

            if pass_idx + 1 == total_passes {
                final_output = Some(full_out);
            } else {
                let mut renderer = self.render_state.renderer.write();
                for id in &full_out.textures_delta.free {
                    renderer.free_texture(id);
                }
            }
        }

        let full_out = final_output.ok_or_else(|| "missing final egui output".to_owned())?;
        let primitives = self
            .ctx
            .tessellate(full_out.shapes, full_out.pixels_per_point);
        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [width_px, height_px],
            pixels_per_point: ppp,
        };

        let target_format = self.render_state.target_format;
        let target_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("sumi_offscreen_capture_texture"),
            size: wgpu::Extent3d {
                width: width_px,
                height: height_px,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: target_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let target_view = target_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let unpadded_bytes_per_row = width_px * 4;
        let padded_bytes_per_row =
            wgpu::util::align_to(unpadded_bytes_per_row, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sumi_offscreen_readback_buffer"),
            size: u64::from(padded_bytes_per_row) * u64::from(height_px),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("sumi_offscreen_capture_encoder"),
        });

        let user_cmd_bufs = {
            let mut renderer = self.render_state.renderer.write();
            renderer.update_buffers(device, queue, &mut encoder, &primitives, &screen_descriptor)
        };

        {
            let clear = palette::BG_WORKSTATION.to_normalized_gamma_f32();
            let renderer = self.render_state.renderer.read();
            let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("sumi_offscreen_egui_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: f64::from(clear[0]),
                            g: f64::from(clear[1]),
                            b: f64::from(clear[2]),
                            a: f64::from(clear[3]),
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            renderer.render(
                &mut render_pass.forget_lifetime(),
                &primitives,
                &screen_descriptor,
            );
        }

        encoder.copy_texture_to_buffer(
            target_texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(height_px),
                },
            },
            wgpu::Extent3d {
                width: width_px,
                height: height_px,
                depth_or_array_layers: 1,
            },
        );

        queue.submit(core::iter::chain(user_cmd_bufs, [encoder.finish()]));

        {
            let mut renderer = self.render_state.renderer.write();
            for id in &full_out.textures_delta.free {
                renderer.free_texture(id);
            }
        }

        let buffer_slice = readback_buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |res| {
            let _ = tx.send(res);
        });
        device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(15)),
            })
            .map_err(|err| format!("offscreen GPU readback poll failed: {err}"))?;
        rx.recv()
            .map_err(|err| format!("offscreen GPU map_async channel dropped: {err}"))?
            .map_err(|err| format!("offscreen GPU buffer map_async failed: {err}"))?;

        let to_rgba = match target_format {
            wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Rgba8UnormSrgb => [0, 1, 2, 3],
            wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb => [2, 1, 0, 3],
            other => {
                return Err(format!(
                    "unsupported offscreen capture format: {other:?} (expected Rgba8Unorm/Bgra8Unorm)"
                ));
            }
        };

        let mapped_range = buffer_slice
            .get_mapped_range()
            .map_err(|err| format!("failed to read mapped GPU capture buffer: {err}"))?;
        let mut pixels = Vec::with_capacity((width_px * height_px) as usize);
        for padded_row in mapped_range
            .chunks(padded_bytes_per_row as usize)
            .take(height_px as usize)
        {
            let row = &padded_row[..unpadded_bytes_per_row as usize];
            for color in row.as_chunks::<4>().0 {
                pixels.push(Color32::from_rgba_premultiplied(
                    color[to_rgba[0]],
                    color[to_rgba[1]],
                    color[to_rgba[2]],
                    color[to_rgba[3]],
                ));
            }
        }
        drop(mapped_range);
        readback_buffer.unmap();
        target_texture.destroy();

        Ok(egui::ColorImage::new(
            [width_px as usize, height_px as usize],
            pixels,
        ))
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl SumiWorkstationApp {
    fn new_for_offscreen_capture() -> Self {
        let mut app = Self::default();
        if let Ok(bars_str) = std::env::var("SUMI_BAR_COUNT")
            && let Ok(bars) = bars_str.parse::<usize>()
            && bars >= 10
        {
            app.dataset_bar_count = bars;
            app.data_source = DataSourcePreset::SyntheticVnindex;
        }
        if let Ok(env_import) = std::env::var("SUMI_IMPORT_PATH")
            && !env_import.trim().is_empty()
        {
            let _ = app.import_file_from_path(&env_import);
            return app;
        }
        app.restore_persisted_session();
        app
    }

    /// Executes a single offscreen Retina (`2.0x`) Metal/`wgpu` workstation capture without
    /// opening a window, bouncing the macOS Dock, stealing focus, or stalling on display sleep.
    pub fn run_offscreen_wgpu_single_capture(
        screenshot_path: &str,
        telemetry_path: Option<&str>,
        scenario: Option<&str>,
        logical_width: u32,
        logical_height: u32,
    ) -> Result<(), String> {
        let harness = OffscreenWgpuHarness::new()?;
        let mut app = Self::new_for_offscreen_capture();
        let _ = harness.capture_app_frame(&mut app, logical_width, logical_height, 2)?;

        if let Some(sc) = scenario {
            match sc {
                "zoom_cursor" => {
                    let w = app.viewport.time_scale.width();
                    app.viewport.time_scale.zoom(w * 0.65, 8.0);
                }
                "pan_history" => {
                    app.viewport.time_scale.set_right_offset(-35.0);
                }
                "log_100k_fit" => {
                    app.set_scale_mode(PriceScaleMode::Logarithmic);
                    app.viewport.fit_all();
                }
                "pct_mode" => {
                    app.set_scale_mode(PriceScaleMode::Percentage);
                }
                _ => {}
            }
        }

        let image = harness.capture_app_frame(&mut app, logical_width, logical_height, 4)?;
        if let Some(parent) = std::path::Path::new(screenshot_path).parent()
            && !parent.as_os_str().is_empty()
        {
            let _ = std::fs::create_dir_all(parent);
        }
        app.persist_capture_artifacts(screenshot_path, telemetry_path, &image);
        Ok(())
    }

    /// Executes all 3 Retina (`2.0x`) Metal/`wgpu` verification scenarios for `"r04"`, `"r05"`, `"r06"`,
    /// `"r07"`, `"r08"`, `"r09"`, `"r10"`, or `"r11"` offscreen in one headless GPU session (zero windows, zero Dock icons, zero occlusion stalls).
    pub fn run_offscreen_wgpu_capture_batch(batch_name: &str) -> Result<(), String> {
        if !matches!(
            batch_name,
            "r04" | "r05" | "r06" | "r07" | "r08" | "r09" | "r10" | "r11"
        ) {
            return Err(format!(
                "unknown SUMI_CAPTURE_BATCH '{batch_name}' (expected 'r04', 'r05', 'r06', 'r07', 'r08', 'r09', 'r10', or 'r11')"
            ));
        }
        std::fs::create_dir_all("docs/evidence")
            .map_err(|err| format!("failed to create docs/evidence: {err}"))?;

        let harness = OffscreenWgpuHarness::new()?;
        let mut app = Self::new_for_offscreen_capture();

        for step in 0..3 {
            let (logical_w, logical_h, png_path, json_path) = match batch_name {
                "r04" => app.apply_r04_batch_step_state(step),
                "r05" => app.apply_r05_batch_step_state(step),
                "r06" => app.apply_r06_batch_step_state(step),
                "r07" => app.apply_r07_batch_step_state(step),
                "r08" => app.apply_r08_batch_step_state(step),
                "r09" => app.apply_r09_batch_step_state(step),
                "r10" => app.apply_r10_batch_step_state(step),
                "r11" => app.apply_r11_batch_step_state(step),
                _ => unreachable!(),
            };
            let image = harness.capture_app_frame(&mut app, logical_w, logical_h, 6)?;
            app.persist_capture_artifacts(png_path, Some(json_path), &image);
        }
        Ok(())
    }

    fn apply_r04_batch_step_state(
        &mut self,
        step: usize,
    ) -> (u32, u32, &'static str, &'static str) {
        match step {
            0 => {
                self.set_dataset_size(500);
                self.set_scale_mode(PriceScaleMode::Normal);
                self.viewport.reset_pane_stretches();
                self.indicators = IndicatorConfig {
                    show_sma: true,
                    show_ema: true,
                    show_bbands: true,
                    show_rsi: true,
                    show_macd: true,
                    show_atr: false,
                    ..IndicatorConfig::default()
                };
                self.indicator_cache
                    .sync(&self.candles, self.dataset_generation, &self.indicators);
                (
                    1280,
                    800,
                    "docs/evidence/r04_multipane_default_1280x800.png",
                    "docs/evidence/r04_multipane_default_1280x800.json",
                )
            }
            1 => {
                self.set_dataset_size(100_000);
                self.indicators = IndicatorConfig {
                    show_sma: true,
                    show_ema: true,
                    show_bbands: true,
                    show_rsi: true,
                    show_macd: true,
                    show_atr: true,
                    ..IndicatorConfig::default()
                };
                self.indicator_cache
                    .sync(&self.candles, self.dataset_generation, &self.indicators);
                self.set_scale_mode(PriceScaleMode::Logarithmic);
                self.viewport.reset_pane_stretches();
                self.viewport.fit_all();
                (
                    1280,
                    800,
                    "docs/evidence/r04_all_indicators_100k_fit_1280x800.png",
                    "docs/evidence/r04_all_indicators_100k_fit_1280x800.json",
                )
            }
            _ => {
                self.set_dataset_size(300);
                self.set_scale_mode(PriceScaleMode::Normal);
                self.indicators = IndicatorConfig {
                    show_sma: true,
                    show_ema: false,
                    show_bbands: true,
                    show_rsi: true,
                    show_macd: true,
                    show_atr: true,
                    ..IndicatorConfig::default()
                };
                self.indicator_cache
                    .sync(&self.candles, self.dataset_generation, &self.indicators);
                self.viewport.reset_pane_stretches();
                self.viewport
                    .apply_separator_drag_delta(None, OscillatorPaneId::Rsi, 400.0, -58.0);
                (
                    960,
                    640,
                    "docs/evidence/r04_resized_separator_960x640.png",
                    "docs/evidence/r04_resized_separator_960x640.json",
                )
            }
        }
    }

    fn apply_r05_batch_step_state(
        &mut self,
        step: usize,
    ) -> (u32, u32, &'static str, &'static str) {
        match step {
            0 => {
                self.set_dataset_size(240);
                self.viewport.time_scale.set_width(1178.0);
                self.set_scale_mode(PriceScaleMode::Normal);
                self.viewport.reset_pane_stretches();
                self.indicators = IndicatorConfig {
                    show_sma: true,
                    show_ema: true,
                    show_bbands: true,
                    show_rsi: true,
                    show_macd: true,
                    show_atr: false,
                    ..IndicatorConfig::default()
                };
                self.indicator_cache
                    .sync(&self.candles, self.dataset_generation, &self.indicators);
                self.populate_showcase_drawings();
                (
                    1280,
                    800,
                    "docs/evidence/r05_drawings_showcase_1280x800.png",
                    "docs/evidence/r05_drawings_showcase_1280x800.json",
                )
            }
            1 => {
                self.set_dataset_size(100_000);
                self.viewport.time_scale.set_width(1178.0);
                self.indicators = IndicatorConfig {
                    show_sma: true,
                    show_ema: true,
                    show_bbands: true,
                    show_rsi: true,
                    show_macd: true,
                    show_atr: true,
                    ..IndicatorConfig::default()
                };
                self.indicator_cache
                    .sync(&self.candles, self.dataset_generation, &self.indicators);
                self.set_scale_mode(PriceScaleMode::Logarithmic);
                self.viewport.reset_pane_stretches();
                self.viewport.fit_all();
                self.populate_showcase_drawings();
                (
                    1280,
                    800,
                    "docs/evidence/r05_drawings_log_100k_fit_1280x800.png",
                    "docs/evidence/r05_drawings_log_100k_fit_1280x800.json",
                )
            }
            _ => {
                self.set_dataset_size(180);
                self.viewport.time_scale.set_width(858.0);
                self.set_scale_mode(PriceScaleMode::Normal);
                self.indicators = IndicatorConfig {
                    show_sma: true,
                    show_ema: false,
                    show_bbands: true,
                    show_rsi: true,
                    show_macd: true,
                    show_atr: false,
                    ..IndicatorConfig::default()
                };
                self.indicator_cache
                    .sync(&self.candles, self.dataset_generation, &self.indicators);
                self.viewport.reset_pane_stretches();
                self.viewport
                    .apply_separator_drag_delta(None, OscillatorPaneId::Rsi, 360.0, -36.0);
                self.populate_showcase_drawings();
                self.drawings.magnet_mode = MagnetMode::Ohlc;

                let left_origin = 162_usize.min(self.candles.len() - 1);
                let left_through = 128_usize.min(self.candles.len() - 1);
                let _ = self.drawings.add_drawing(
                    DrawingPaneTarget::Main,
                    DrawingGeometry::Ray {
                        origin: DrawingAnchor::new(
                            left_origin as f64,
                            self.candles[left_origin].high,
                        ),
                        through: DrawingAnchor::new(
                            left_through as f64,
                            self.candles[left_through].high + 6.0,
                        ),
                    },
                    Some(DrawingStyle {
                        color: DrawingColorPreset::Crimson,
                        line_width: 1.8,
                        dash: LineDashStyle::Solid,
                        fill_opacity: 0.0,
                    }),
                );

                let mut tmp_ps = self.viewport.main_price_scale.clone();
                tmp_ps.set_height(320.0);
                let vis = self.visible_slice();
                if let Some(bounds) = crate::model::PriceBounds::from_candles(vis, 0.08) {
                    let bv = vis.first().map_or(1200.0, |c| c.close);
                    tmp_ps.autoscale_raw_bounds(bounds.raw_min, bounds.raw_max, bv);
                    let proj = crate::drawings::PaneDrawingProjection::new(
                        &self.viewport.time_scale,
                        &tmp_ps,
                        858.0,
                        320.0,
                        bv,
                    );
                    let i0 = 115_usize.min(self.candles.len() - 1);
                    let i1 = 168_usize.min(self.candles.len() - 1);
                    if let (Some(lp0), Some(lp1)) = (
                        proj.anchor_to_local(DrawingAnchor::new(i0 as f64, self.candles[i0].low)),
                        proj.anchor_to_local(DrawingAnchor::new(i1 as f64, self.candles[i1].close)),
                    ) {
                        self.drawings.set_tool(DrawingTool::TrendLine);
                        let _ = self.drawings.pointer_down(
                            DrawingPaneTarget::Main,
                            lp0,
                            false,
                            &proj,
                            &self.candles,
                        );
                        let _ = self.drawings.pointer_up(
                            DrawingPaneTarget::Main,
                            Some(lp0),
                            false,
                            &proj,
                            &self.candles,
                        );
                        self.drawings.pointer_move(
                            DrawingPaneTarget::Main,
                            lp1,
                            false,
                            &proj,
                            &self.candles,
                        );
                    }
                }
                (
                    960,
                    640,
                    "docs/evidence/r05_drawings_interaction_resized_960x640.png",
                    "docs/evidence/r05_drawings_interaction_resized_960x640.json",
                )
            }
        }
    }

    fn apply_r06_batch_step_state(
        &mut self,
        step: usize,
    ) -> (u32, u32, &'static str, &'static str) {
        match step {
            0 => {
                // Scenario 0: 1280x800, Intraday `VN30F1M` 1m -> 5m drawings -> remapped to `15m` (`360` bars),
                // demonstrating adaptive `YYYY-MM-DD HH:MM` / `MM-DD HH:MM` / `HH:MM` time axis!
                self.show_import_bar = false;
                self.select_data_source(DataSourcePreset::IntradayVn30F1m);
                self.viewport.time_scale.set_width(1178.0);
                self.set_scale_mode(PriceScaleMode::Normal);
                self.viewport.reset_pane_stretches();
                self.indicators = IndicatorConfig {
                    show_sma: true,
                    show_ema: true,
                    show_bbands: true,
                    show_rsi: true,
                    show_macd: true,
                    show_atr: false,
                    ..IndicatorConfig::default()
                };
                let _ = self.set_timeframe(Timeframe::M5);
                self.populate_showcase_drawings();
                let _ = self.set_timeframe(Timeframe::M15);
                (
                    1280,
                    800,
                    "docs/evidence/r06_intraday_15m_vn30f_1280x800.png",
                    "docs/evidence/r06_intraday_15m_vn30f_1280x800.json",
                )
            }
            1 => {
                // Scenario 1: 1280x800, Real multi-symbol `CafeF.INDEX.clean.csv` (`VNINDEX` isolated from `HNX-INDEX`),
                // aggregated `1D -> 1W` (ISO-Monday weekly bars), Logarithmic full-fit across 4 panes + drawings!
                self.show_import_bar = false;
                self.select_data_source(DataSourcePreset::CafeFIndexCsv);
                self.select_symbol_in_source("VNINDEX");
                self.viewport.time_scale.set_width(1178.0);
                self.indicators = IndicatorConfig {
                    show_sma: true,
                    show_ema: true,
                    show_bbands: true,
                    show_rsi: true,
                    show_macd: true,
                    show_atr: true,
                    ..IndicatorConfig::default()
                };
                let _ = self.set_timeframe(Timeframe::W1);
                self.set_scale_mode(PriceScaleMode::Logarithmic);
                self.viewport.reset_pane_stretches();
                self.viewport.fit_all();
                self.populate_showcase_drawings();
                (
                    1280,
                    800,
                    "docs/evidence/r06_cafef_vnindex_weekly_1280x800.png",
                    "docs/evidence/r06_cafef_vnindex_weekly_1280x800.json",
                )
            }
            _ => {
                // Scenario 2: Resized 960x640 compact layout, 24/7 `BTCUSDT` `15m` JSON klines aggregated to `4h` (`84` bars),
                // non-modal Import CSV/JSON bar open with status summary + multi-pane drawings!
                self.select_data_source(DataSourcePreset::BtcUsdt15mJson);
                self.viewport.time_scale.set_width(858.0);
                self.set_scale_mode(PriceScaleMode::Normal);
                self.indicators = IndicatorConfig {
                    show_sma: true,
                    show_ema: false,
                    show_bbands: true,
                    show_rsi: true,
                    show_macd: true,
                    show_atr: false,
                    ..IndicatorConfig::default()
                };
                let _ = self.set_timeframe(Timeframe::H4);
                self.viewport.reset_pane_stretches();
                self.viewport.fit_all();
                self.populate_showcase_drawings();
                self.show_import_bar = true;
                (
                    960,
                    640,
                    "docs/evidence/r06_btc_json_4h_resized_960x640.png",
                    "docs/evidence/r06_btc_json_4h_resized_960x640.json",
                )
            }
        }
    }

    fn apply_r07_batch_step_state(
        &mut self,
        step: usize,
    ) -> (u32, u32, &'static str, &'static str) {
        match step {
            0 => {
                // Scenario 0: 1280x800, Intraday `VN30F1M` (`1m` base -> `15m` causal aggregation),
                // Bar Replay cut mid-bucket (`base_index = 3_607`) with historical round-trip fill arrows,
                // active bracketed LONG (`Entry` + `SL` + `TP`), working `BUY LIMIT` order, Replay Dock,
                // and right-side Paper Trading Panel!
                self.show_import_bar = false;
                self.select_data_source(DataSourcePreset::IntradayVn30F1m);
                self.viewport.time_scale.set_width(930.0);
                self.set_scale_mode(PriceScaleMode::Normal);
                self.viewport.reset_pane_stretches();
                self.indicators = IndicatorConfig {
                    show_sma: true,
                    show_ema: true,
                    show_bbands: true,
                    show_rsi: true,
                    show_macd: true,
                    show_atr: false,
                    ..IndicatorConfig::default()
                };
                let _ = self.set_timeframe(Timeframe::M15);
                let _ = self.start_replay_at_base_index(3_150);
                self.replay.speed = ReplaySpeed::X5;
                self.replay.step_unit = ReplayStepUnit::ActiveBar;

                // Execute historical round-trip #1 (`3_150 -> 3_420`) so Buy & Sell arrows render in view!
                let _ = self.trading.submit_order(
                    OrderRequest {
                        side: OrderSide::Buy,
                        kind: OrderKind::Market,
                        quantity: 20.0,
                        stop_loss: None,
                        take_profit: None,
                    },
                    &self.base_candles,
                    3_150,
                );
                if let Some(tr) = self.replay.jump_to_base_index(&self.base_candles, 3_420) {
                    self.apply_replay_transition(tr);
                }
                let _ = self.flatten_open_position();

                // Advance to causal mid-15m cut `3_607` (`DEF-001` partial active bar) and open bracketed LONG + working Limit
                if let Some(tr) = self.replay.jump_to_base_index(&self.base_candles, 3_607) {
                    self.apply_replay_transition(tr);
                }
                self.viewport.time_scale.set_bar_spacing(11.0);
                self.viewport.time_scale.set_right_offset(16.0);
                let mark = self.current_mark_price();
                let (vis_lo, vis_hi) =
                    crate::model::PriceBounds::from_candles(self.visible_slice(), 0.0)
                        .map_or((mark - 10.0, mark + 10.0), |b| (b.raw_min, b.raw_max));
                let span = (vis_hi - vis_lo).max(4.0);
                let sl_px = (mark - span * 0.20)
                    .max(vis_lo - span * 0.03)
                    .min(mark - span * 0.03);
                let tp_px = (mark + span * 0.20)
                    .min(vis_hi + span * 0.04)
                    .max(mark + span * 0.03);
                let lmt_px = 0.5 * (mark + sl_px);

                self.ticket_kind = TicketOrderKind::Limit;
                self.ticket_qty_input = "20".to_owned();
                self.ticket_price_input = format!("{lmt_px:.2}");
                self.ticket_sl_input = format!("{sl_px:.2}");
                self.ticket_tp_input = format!("{tp_px:.2}");
                let _ = self.trading.submit_order(
                    OrderRequest {
                        side: OrderSide::Buy,
                        kind: OrderKind::Market,
                        quantity: 20.0,
                        stop_loss: Some(sl_px),
                        take_profit: Some(tp_px),
                    },
                    &self.base_candles,
                    3_607,
                );
                let _ = self.trading.submit_order(
                    OrderRequest {
                        side: OrderSide::Buy,
                        kind: OrderKind::Limit {
                            limit_price: lmt_px,
                        },
                        quantity: 10.0,
                        stop_loss: None,
                        take_profit: None,
                    },
                    &self.base_candles,
                    3_607,
                );

                self.populate_showcase_drawings();
                self.show_replay_bar = true;
                self.show_trading_panel = true;
                (
                    1280,
                    800,
                    "docs/evidence/r07_replay_cut_intraday_15m_vn30f_1280x800.png",
                    "docs/evidence/r07_replay_cut_intraday_15m_vn30f_1280x800.json",
                )
            }
            1 => {
                // Scenario 1: 1280x800, 24/7 `BTCUSDT` (`15m` base -> `4h` causal aggregation, 4 panes),
                // multi-trade campaign + active bracketed SHORT position (`Entry`, `SL`, `TP`) + working `BUY LIMIT` TP!
                self.show_import_bar = false;
                self.select_data_source(DataSourcePreset::BtcUsdt15mJson);
                self.viewport.time_scale.set_width(930.0);
                self.set_scale_mode(PriceScaleMode::Normal);
                self.viewport.reset_pane_stretches();
                self.indicators = IndicatorConfig {
                    show_sma: true,
                    show_ema: true,
                    show_bbands: true,
                    show_rsi: true,
                    show_macd: true,
                    show_atr: true,
                    ..IndicatorConfig::default()
                };
                let _ = self.set_timeframe(Timeframe::H4);
                let _ = self.start_replay_at_base_index(640);
                self.replay.speed = ReplaySpeed::X10;
                self.replay.step_unit = ReplayStepUnit::ActiveBar;

                // Trade #1 (`640 -> 880` Long round-trip)
                let _ = self.trading.submit_order(
                    OrderRequest {
                        side: OrderSide::Buy,
                        kind: OrderKind::Market,
                        quantity: 0.50,
                        stop_loss: None,
                        take_profit: None,
                    },
                    &self.base_candles,
                    640,
                );
                if let Some(tr) = self.replay.jump_to_base_index(&self.base_candles, 880) {
                    self.apply_replay_transition(tr);
                }
                let _ = self.flatten_open_position();

                // Advance to `1_088` (`68` 4h bars) and open bracketed SHORT + working Buy Limit partial
                if let Some(tr) = self.replay.jump_to_base_index(&self.base_candles, 1_088) {
                    self.apply_replay_transition(tr);
                }
                self.viewport.fit_all();
                self.viewport.time_scale.set_right_offset(14.0);
                let mark = self.current_mark_price();
                let (vis_lo, vis_hi) =
                    crate::model::PriceBounds::from_candles(self.visible_slice(), 0.0)
                        .map_or((mark - 2_000.0, mark + 2_000.0), |b| (b.raw_min, b.raw_max));
                let span = (vis_hi - vis_lo).max(200.0);
                let sl = (mark + span * 0.18)
                    .min(vis_hi + span * 0.04)
                    .max(mark + span * 0.03);
                let tp = (mark - span * 0.22)
                    .max(vis_lo - span * 0.03)
                    .min(mark - span * 0.03)
                    .max(1.0);
                let lmt_px = 0.5 * (mark + tp);
                let _ = self.trading.submit_order(
                    OrderRequest {
                        side: OrderSide::Sell,
                        kind: OrderKind::Market,
                        quantity: 0.45,
                        stop_loss: Some(sl),
                        take_profit: Some(tp),
                    },
                    &self.base_candles,
                    1_088,
                );
                let _ = self.trading.submit_order(
                    OrderRequest {
                        side: OrderSide::Buy,
                        kind: OrderKind::Limit {
                            limit_price: lmt_px,
                        },
                        quantity: 0.20,
                        stop_loss: None,
                        take_profit: None,
                    },
                    &self.base_candles,
                    1_088,
                );

                self.ticket_kind = TicketOrderKind::Stop;
                self.ticket_qty_input = "0.45".to_owned();
                self.ticket_price_input = format!("{lmt_px:.2}");
                self.ticket_sl_input = format!("{sl:.2}");
                self.ticket_tp_input = format!("{tp:.2}");

                self.show_replay_bar = true;
                self.show_trading_panel = true;
                (
                    1280,
                    800,
                    "docs/evidence/r07_paper_trading_short_btc_4h_1280x800.png",
                    "docs/evidence/r07_paper_trading_short_btc_4h_1280x800.json",
                )
            }
            _ => {
                // Scenario 2: Compact resized `960x640`, real `CafeF.INDEX.clean.csv` (`VNINDEX` `1D` -> `1W`),
                // Logarithmic scale, `BaseBar` (`Tick`) intra-week causal replay + bracketed LONG + Demo drawings!
                self.show_import_bar = false;
                self.select_data_source(DataSourcePreset::CafeFIndexCsv);
                self.select_symbol_in_source("VNINDEX");
                self.viewport.time_scale.set_width(612.0);
                self.indicators = IndicatorConfig {
                    show_sma: true,
                    show_ema: false,
                    show_bbands: true,
                    show_rsi: true,
                    show_macd: true,
                    show_atr: false,
                    ..IndicatorConfig::default()
                };
                let _ = self.set_timeframe(Timeframe::W1);
                self.set_scale_mode(PriceScaleMode::Logarithmic);
                self.viewport.reset_pane_stretches();

                let cut_idx = ((self.base_candles.len() * 82) / 100).max(10);
                let prior_idx = cut_idx.saturating_sub(45);
                let _ = self.start_replay_at_base_index(prior_idx);
                self.replay.step_unit = ReplayStepUnit::BaseBar;
                self.replay.speed = ReplaySpeed::X2;

                let _ = self.trading.submit_order(
                    OrderRequest {
                        side: OrderSide::Buy,
                        kind: OrderKind::Market,
                        quantity: 15.0,
                        stop_loss: None,
                        take_profit: None,
                    },
                    &self.base_candles,
                    prior_idx,
                );
                if let Some(tr) = self.replay.jump_to_base_index(&self.base_candles, cut_idx) {
                    self.apply_replay_transition(tr);
                }
                self.viewport.time_scale.set_bar_spacing(8.5);
                self.viewport.time_scale.set_right_offset(14.0);
                let mark = self.current_mark_price();
                let (vis_lo, vis_hi) =
                    crate::model::PriceBounds::from_candles(self.visible_slice(), 0.0)
                        .map_or((mark * 0.92, mark * 1.08), |b| (b.raw_min, b.raw_max));
                let span = (vis_hi - vis_lo).max(10.0);
                let entry_ref = self
                    .trading
                    .open_position()
                    .map_or(mark, |p| p.avg_entry_price.min(mark));
                let sl = (entry_ref - span * 0.16)
                    .max(vis_lo + span * 0.05)
                    .min(mark - span * 0.05)
                    .max(1.0);
                let tp = (mark + span * 0.18)
                    .min(vis_hi + span * 0.05)
                    .max(mark + span * 0.03);
                let _ = self.trading.update_position_brackets(
                    Some(sl),
                    Some(tp),
                    &self.base_candles,
                    cut_idx,
                );

                self.ticket_kind = TicketOrderKind::Market;
                self.ticket_qty_input = "15".to_owned();
                self.ticket_sl_input = format!("{sl:.2}");
                self.ticket_tp_input = format!("{tp:.2}");

                self.populate_showcase_drawings();
                self.show_replay_bar = true;
                self.show_trading_panel = true;
                (
                    960,
                    640,
                    "docs/evidence/r07_vnindex_weekly_causal_replay_960x640.png",
                    "docs/evidence/r07_vnindex_weekly_causal_replay_960x640.json",
                )
            }
        }
    }

    fn apply_r08_batch_step_state(
        &mut self,
        step: usize,
    ) -> (u32, u32, &'static str, &'static str) {
        self.close_command_palette();
        self.show_import_bar = false;
        self.scanner_filter = ScannerFilter::default();

        match step {
            0 => {
                // Scenario 0: 1280x800, Multi-Symbol Watchlist & Technical Scanner Dock (Left, 13 symbols)
                // + Causal Alerts Manager & Trigger Log (Right) + On-Chart Armed/Triggered Price & Ray
                // Alerts + Trigger Diamonds + Frontier `ALERT FIRED` Toast on `FPT` (`VN30 Basket` `1D`)!
                self.exit_replay();
                self.select_watchlist_instrument(DataSourcePreset::Vn30Basket, "FPT");
                self.alerts.clear_all();
                let _ = self.drawings.clear_unlocked();
                self.set_scale_mode(PriceScaleMode::Normal);
                self.viewport.reset_pane_stretches();
                self.indicators = IndicatorConfig {
                    show_sma: true,
                    show_ema: true,
                    show_bbands: true,
                    show_rsi: true,
                    show_macd: true,
                    show_atr: false,
                    ..IndicatorConfig::default()
                };
                self.viewport.time_scale.set_width(700.0);

                let end_idx = self.base_candles.len().saturating_sub(42).max(60);
                let arm_idx = end_idx.saturating_sub(32);
                let _ = self.start_replay_at_base_index(arm_idx);
                self.replay.speed = ReplaySpeed::X5;

                // 1) Sloped `Ray` drawing + `DrawingLine` alert
                let r0 = arm_idx.saturating_sub(18);
                let r1 = arm_idx.saturating_sub(2);
                let ray_id = self
                    .drawings
                    .add_drawing(
                        DrawingPaneTarget::Main,
                        DrawingGeometry::Ray {
                            origin: DrawingAnchor::with_timestamp(
                                r0 as f64,
                                self.base_candles[r0].low,
                                self.base_candles[r0].timestamp as f64,
                            ),
                            through: DrawingAnchor::with_timestamp(
                                r1 as f64,
                                self.base_candles[r1].close,
                                self.base_candles[r1].timestamp as f64,
                            ),
                        },
                        Some(DrawingStyle {
                            color: DrawingColorPreset::Cyan,
                            line_width: 1.75,
                            dash: LineDashStyle::Solid,
                            fill_opacity: 0.0,
                        }),
                    )
                    .unwrap_or(1);
                self.drawings.selected_id = Some(ray_id);
                let _ = self.add_alert_on_selected_drawing(CrossDirection::CrossAny);

                // 2) Historical `Once` PriceLevel alert that triggers midway through `(arm_idx..end_idx)`
                let arm_close = self.base_candles[arm_idx].close;
                let mut mid_hi = arm_close;
                let mut mid_lo = arm_close;
                for c in &self.base_candles[(arm_idx + 1)..end_idx] {
                    mid_hi = mid_hi.max(c.high);
                    mid_lo = mid_lo.min(c.low);
                }
                self.alert_frequency = AlertFrequency::Once;
                if mid_hi - arm_close >= arm_close - mid_lo && mid_hi > arm_close {
                    let lvl = 0.5 * (arm_close + mid_hi);
                    let _ = self.add_quick_price_alert(lvl, CrossDirection::CrossAbove);
                } else if arm_close > mid_lo {
                    let lvl = 0.5 * (arm_close + mid_lo);
                    let _ = self.add_quick_price_alert(lvl, CrossDirection::CrossBelow);
                }

                // Advance to `end_idx - 1` and arm a frontier alert that fires right on `end_idx`!
                if let Some(tr) = self
                    .replay
                    .jump_to_base_index(&self.base_candles, end_idx.saturating_sub(1))
                {
                    self.apply_replay_transition(tr);
                }
                let prev_c = self.base_candles[end_idx.saturating_sub(1)].close;
                let (end_hi, end_lo) = (
                    self.base_candles[end_idx].high,
                    self.base_candles[end_idx].low,
                );
                if end_hi - prev_c >= prev_c - end_lo && end_hi > prev_c {
                    let frontier_lvl = 0.5 * (prev_c + end_hi);
                    let _ = self.add_quick_price_alert(frontier_lvl, CrossDirection::CrossAbove);
                } else {
                    let frontier_lvl = 0.5 * (prev_c + end_lo.min(prev_c * 0.999));
                    let _ = self.add_quick_price_alert(frontier_lvl, CrossDirection::CrossBelow);
                }

                // Also arm an active `ARMED` resistance alert slightly above visible mark
                let armed_res = end_hi * 1.022;
                let _ = self.add_quick_price_alert(armed_res, CrossDirection::CrossAbove);
                self.alert_value_input = format!("{armed_res:.2}");

                if let Some(tr) = self.replay.jump_to_base_index(&self.base_candles, end_idx) {
                    self.apply_replay_transition(tr);
                }

                self.viewport.time_scale.set_bar_spacing(9.5);
                self.viewport.time_scale.set_right_offset(14.0);
                self.watchlist_sort_col = WatchlistSortColumn::ChangePct;
                self.watchlist_sort_asc = false;
                self.scanner_tab = ScannerDockTab::Watchlist;
                self.show_scanner_dock = true;
                self.show_replay_bar = true;
                self.show_trading_panel = true;
                self.show_alerts_panel = true;
                self.right_dock_tab = RightDockTab::Alerts;

                (
                    1280,
                    800,
                    "docs/evidence/r08_watchlist_scanner_and_alerts_1280x800.png",
                    "docs/evidence/r08_watchlist_scanner_and_alerts_1280x800.json",
                )
            }
            1 => {
                // Scenario 1: 1280x800, 4-Pane `BTCUSDT` (`15m -> 4h`) + Active Alerts + Floating `Cmd+K`
                // Command Palette Modal synthesizing dynamic `alert 68500` quick-action!
                self.exit_replay();
                self.select_data_source(DataSourcePreset::BtcUsdt15mJson);
                self.alerts.clear_all();
                self.set_scale_mode(PriceScaleMode::Normal);
                self.viewport.reset_pane_stretches();
                self.indicators = IndicatorConfig {
                    show_sma: true,
                    show_ema: true,
                    show_bbands: true,
                    show_rsi: true,
                    show_macd: true,
                    show_atr: true,
                    ..IndicatorConfig::default()
                };
                let _ = self.set_timeframe(Timeframe::H4);
                self.viewport.time_scale.set_width(940.0);
                self.viewport.fit_all();
                self.viewport.time_scale.set_right_offset(12.0);

                let mark = self.current_mark_price();
                let upper_lvl = ((mark * 1.018) / 10.0).round() * 10.0;
                let lower_lvl = ((mark * 0.982) / 10.0).round() * 10.0;
                self.alert_frequency = AlertFrequency::Once;
                let _ = self.add_quick_price_alert(upper_lvl, CrossDirection::CrossAbove);
                let _ = self.add_quick_price_alert(lower_lvl, CrossDirection::CrossBelow);

                self.show_scanner_dock = false;
                self.show_replay_bar = false;
                self.show_trading_panel = false;
                self.show_alerts_panel = true;
                self.right_dock_tab = RightDockTab::Alerts;

                self.open_command_palette();
                let query_px = (((mark * 1.008) / 50.0).round() * 50.0) as i64;
                self.command_palette.query = format!("alert {query_px}");
                self.command_palette.selected_index = 0;

                (
                    1280,
                    800,
                    "docs/evidence/r08_command_palette_cmd_k_btc_1280x800.png",
                    "docs/evidence/r08_command_palette_cmd_k_btc_1280x800.json",
                )
            }
            _ => {
                // Scenario 2: Compact resized `960x640`, Left Scanner `Signals` tab + Causal
                // `BlindPractice` Replay (`BLIND PRACTICE · Signal hidden until reached`)!
                self.exit_replay();
                self.select_watchlist_instrument(DataSourcePreset::Vn30Basket, "FPT");
                self.signal_lookback_bars = 80;
                self.scanner_filter = ScannerFilter::default();
                let candidate_hit = self
                    .filtered_historical_signals()
                    .into_iter()
                    .find(|h| h.bar_index >= 45 && h.instrument_id.starts_with("Preset:VN30"))
                    .or_else(|| self.filtered_historical_signals().into_iter().next());

                self.indicators = IndicatorConfig {
                    show_sma: true,
                    show_ema: true,
                    show_bbands: false,
                    show_rsi: true,
                    show_macd: true,
                    show_atr: false,
                    ..IndicatorConfig::default()
                };
                self.set_scale_mode(PriceScaleMode::Normal);
                self.viewport.reset_pane_stretches();
                self.viewport.time_scale.set_width(640.0);

                if let Some(hit) = candidate_hit {
                    let _ =
                        self.start_scanner_signal_replay(hit, ScannerReplayIntent::BlindPractice);
                    let _ = self.step_replay_forward(4);
                }

                let mark = self.current_mark_price();
                if mark > 0.0 {
                    self.alerts.clear_all();
                    let _ = self.add_quick_price_alert(mark * 1.015, CrossDirection::CrossAbove);
                }
                self.viewport.time_scale.set_bar_spacing(9.0);
                self.viewport.time_scale.set_right_offset(14.0);

                self.show_scanner_dock = true;
                self.scanner_tab = ScannerDockTab::Signals;
                self.show_replay_bar = true;
                self.show_trading_panel = false;
                self.show_alerts_panel = false;

                (
                    960,
                    640,
                    "docs/evidence/r08_blind_practice_signal_replay_960x640.png",
                    "docs/evidence/r08_blind_practice_signal_replay_960x640.json",
                )
            }
        }
    }

    fn apply_r09_batch_step_state(
        &mut self,
        step: usize,
    ) -> (u32, u32, &'static str, &'static str) {
        self.close_command_palette();
        self.show_import_bar = false;
        self.alerts.clear_all();
        self.drawings = DrawingStore::new();
        self.trading = TradingSession::default();

        match step {
            0 => {
                // Scenario 0: `1280x800` (`2560x1600` Retina), `FPT` (`VN30` `1D`), Strategy Lab Tearsheet +
                // IS/OOS Split (`70%/30%`) + Synchronized `Equity / Drawdown` Baseline Sub-Pane + Focused Trade Links!
                self.exit_replay();
                self.select_watchlist_instrument(DataSourcePreset::Vn30Basket, "FPT");
                self.indicators = IndicatorConfig {
                    show_sma: true,
                    show_ema: true,
                    show_bbands: false,
                    show_rsi: true,
                    show_macd: false,
                    show_atr: false,
                    sma_period: 15,
                    ema_period: 40,
                    ..IndicatorConfig::default()
                };
                self.set_scale_mode(PriceScaleMode::Normal);
                self.viewport.reset_pane_stretches();
                self.viewport.equity_stretch = 1.45;
                self.viewport.rsi_stretch = 0.85;

                self.backtest_config = BacktestConfig {
                    preset: StrategyPresetKind::MacdMomentum,
                    params: StrategyParams {
                        macd_fast: 6,
                        macd_slow: 16,
                        macd_signal: 5,
                        macd_trend_ema: 30,
                        ..StrategyParams::default()
                    },
                    direction_mode: StrategyDirectionMode::LongOnly,
                    sizing: PositionSizingMode::PercentEquity(95.0),
                    risk: StopTakeConfig {
                        stop_loss_pct: Some(3.2),
                        take_profit_pct: Some(8.5),
                        trailing_stop_atr_mult: Some(2.2),
                        min_holding_bars: 2,
                        ..StopTakeConfig::default()
                    },
                    initial_cash: 100_000.0,
                    commission_bps: 15.0,
                    tax_sell_bps: 10.0,
                    slippage_bps: 5.0,
                    max_leverage: 1.0,
                    lot_step: 100.0,
                    in_sample_split_pct: Some(0.70),
                };

                self.show_scanner_dock = false;
                self.show_replay_bar = false;
                self.show_trading_panel = false;
                self.show_alerts_panel = false;
                self.strategy_lab_tab = StrategyLabSubTab::Tearsheet;
                self.open_strategy_lab();

                // Fit full series so both In-Sample (`70%`) and Out-of-Sample (`30%`) + `IS | OOS` split marker
                // + both above-baseline emerald growth and below-baseline crimson dips are clearly visible!
                self.viewport.time_scale.set_width(880.0);
                self.viewport.fit_all();
                if let Some(last_tid) = self
                    .backtest_result
                    .as_ref()
                    .and_then(|r| r.trades.last().map(|t| t.trade_id))
                {
                    self.focused_backtest_trade_id = Some(last_tid);
                }

                (
                    1280,
                    800,
                    "docs/evidence/r09_strategy_backtest_equity_pane_fpt_1280x800.png",
                    "docs/evidence/r09_strategy_backtest_equity_pane_fpt_1280x800.json",
                )
            }
            1 => {
                // Scenario 1: `1280x800` (`2560x1600` Retina), `BTCUSDT` `1h -> 4h`, `LongShort` Strategy,
                // 12-Variant Parameter Sweep Grid + 6-Preset Strategy Battle + Synchronized `IS | OOS` Equity Curve!
                self.exit_replay();
                self.select_watchlist_instrument(DataSourcePreset::BtcUsdt15mJson, "BTCUSDT");
                let _ = self.set_timeframe(Timeframe::H4);
                self.indicators = IndicatorConfig {
                    show_sma: false,
                    show_ema: true,
                    show_bbands: true,
                    show_rsi: false,
                    show_macd: true,
                    show_atr: false,
                    ..IndicatorConfig::default()
                };
                self.set_scale_mode(PriceScaleMode::Normal);
                self.viewport.reset_pane_stretches();
                self.viewport.equity_stretch = 1.45;
                self.viewport.macd_stretch = 0.85;

                self.backtest_config = BacktestConfig {
                    preset: StrategyPresetKind::MacdMomentum,
                    params: StrategyParams {
                        macd_fast: 12,
                        macd_slow: 26,
                        macd_signal: 9,
                        macd_trend_ema: 34,
                        ..StrategyParams::default()
                    },
                    direction_mode: StrategyDirectionMode::LongShort,
                    sizing: PositionSizingMode::PercentEquity(95.0),
                    risk: StopTakeConfig {
                        stop_loss_pct: Some(3.5),
                        take_profit_pct: Some(9.0),
                        trailing_stop_atr_mult: Some(2.4),
                        min_holding_bars: 0,
                        ..StopTakeConfig::default()
                    },
                    initial_cash: 100_000.0,
                    commission_bps: 8.0,
                    tax_sell_bps: 0.0,
                    slippage_bps: 4.0,
                    max_leverage: 1.0,
                    lot_step: 0.0,
                    in_sample_split_pct: Some(0.70),
                };
                self.sweep_ranking = SweepRankingMetric::SharpeRatio;

                self.show_scanner_dock = false;
                self.show_replay_bar = false;
                self.show_trading_panel = false;
                self.show_alerts_panel = false;
                self.open_strategy_lab();
                self.strategy_lab_tab = StrategyLabSubTab::Sweep;
                self.run_active_parameter_sweep();
                self.run_universe_strategy_battle();
                // Apply rank #1 sweep variant so chart displays the top-ranked configuration!
                if let Some(best_cfg) = self
                    .sweep_report
                    .as_ref()
                    .and_then(|s| s.variants.first().map(|v| v.config.clone()))
                {
                    self.backtest_config = best_cfg;
                    self.sync_backtest_causal();
                }
                self.viewport.time_scale.set_width(880.0);
                self.viewport.fit_all();

                (
                    1280,
                    800,
                    "docs/evidence/r09_param_sweep_and_battle_btc_4h_1280x800.png",
                    "docs/evidence/r09_param_sweep_and_battle_btc_4h_1280x800.json",
                )
            }
            _ => {
                // Scenario 2: Compact `960x640` (`1920x1280` Retina), Intraday `VN30F1M` `1m -> 5m` under
                // active Causal Bar Replay (`DEF-001` zero-lookahead cutoff) + `Trades` Log + Focused Trade Halo!
                self.exit_replay();
                self.select_watchlist_instrument(DataSourcePreset::IntradayVn30F1m, "VN30F1M");
                let _ = self.set_timeframe(Timeframe::M5);
                self.indicators = IndicatorConfig {
                    show_sma: true,
                    show_ema: true,
                    show_bbands: false,
                    show_rsi: false,
                    show_macd: false,
                    show_atr: true,
                    sma_period: 12,
                    ema_period: 30,
                    ..IndicatorConfig::default()
                };
                self.set_scale_mode(PriceScaleMode::Normal);
                self.viewport.reset_pane_stretches();
                self.viewport.equity_stretch = 1.45;
                self.viewport.atr_stretch = 0.80;

                if self.base_candles.len() >= 20 {
                    let cut_base = (self.base_candles.len() * 82) / 100;
                    let _ = self.start_replay_at_base_index(cut_base);
                }

                self.backtest_config = BacktestConfig {
                    preset: StrategyPresetKind::DonchianAtrBreakout,
                    params: StrategyParams {
                        donchian_entry_period: 15,
                        donchian_exit_period: 8,
                        atr_period: 14,
                        ..StrategyParams::default()
                    },
                    direction_mode: StrategyDirectionMode::LongShort,
                    sizing: PositionSizingMode::RiskPercentPerTrade(1.5),
                    risk: StopTakeConfig {
                        stop_loss_pct: Some(1.2),
                        take_profit_pct: Some(2.8),
                        trailing_stop_atr_mult: Some(2.0),
                        min_holding_bars: 0,
                        ..StopTakeConfig::default()
                    },
                    initial_cash: 100_000.0,
                    commission_bps: 3.0,
                    tax_sell_bps: 0.0,
                    slippage_bps: 2.0,
                    max_leverage: 2.0,
                    lot_step: 1.0,
                    in_sample_split_pct: Some(0.70),
                };

                self.show_scanner_dock = false;
                self.show_replay_bar = true;
                self.show_trading_panel = false;
                self.show_alerts_panel = false;
                self.open_strategy_lab();
                self.strategy_lab_tab = StrategyLabSubTab::Trades;
                self.viewport.time_scale.set_width(590.0);
                if let Some(focus_tid) = self
                    .backtest_result
                    .as_ref()
                    .and_then(|r| r.trades.last().map(|t| t.trade_id))
                {
                    let _ = self.focus_backtest_trade(focus_tid);
                }

                (
                    960,
                    640,
                    "docs/evidence/r09_intraday_vn30f_trade_inspect_960x640.png",
                    "docs/evidence/r09_intraday_vn30f_trade_inspect_960x640.json",
                )
            }
        }
    }

    fn apply_r10_batch_step_state(
        &mut self,
        step: usize,
    ) -> (u32, u32, &'static str, &'static str) {
        self.close_command_palette();
        self.show_import_bar = false;
        self.alerts.clear_all();
        self.drawings = DrawingStore::new();
        self.trading = TradingSession::default();
        self.risk_venue_override = None;

        match step {
            0 => {
                // Scenario 0: `1280x800` (`2560x1600` Retina), `FPT` (`VN30` `1D` `HOSE`),
                // Vietnam Microstructure Risk Planner (`±7%` band, lot `100`, `T+2`, `0.10%` sell tax),
                // selected dual-zone `RiskRewardBox` on chart + Structured Trade Journal & Discipline KPIs!
                self.exit_replay();
                self.select_watchlist_instrument(DataSourcePreset::Vn30Basket, "FPT");
                self.indicators = IndicatorConfig {
                    show_sma: true,
                    show_ema: true,
                    show_bbands: true,
                    show_rsi: true,
                    show_macd: false,
                    show_atr: false,
                    sma_period: 15,
                    ema_period: 40,
                    ..IndicatorConfig::default()
                };
                self.set_scale_mode(PriceScaleMode::Normal);
                self.viewport.reset_pane_stretches();
                self.viewport.rsi_stretch = 0.85;
                self.viewport.time_scale.set_width(880.0);
                self.viewport.time_scale.set_bar_spacing(10.0);
                self.viewport.time_scale.set_right_offset(10.0);

                self.show_scanner_dock = false;
                self.show_replay_bar = false;
                self.show_trading_panel = false;
                self.show_alerts_panel = false;
                self.show_strategy_panel = false;
                self.show_equity_pane = false;
                self.show_backtest_trades = false;

                // Seed journal from causal backtest trades + realistic discipline debrief annotations
                self.journal = TradeJournalStore::new();
                self.backtest_config = BacktestConfig {
                    preset: StrategyPresetKind::MacdMomentum,
                    params: StrategyParams {
                        macd_fast: 6,
                        macd_slow: 16,
                        macd_signal: 5,
                        macd_trend_ema: 0,
                        ..StrategyParams::default()
                    },
                    risk: StopTakeConfig {
                        stop_loss_pct: Some(3.2),
                        take_profit_pct: Some(8.5),
                        trailing_stop_atr_mult: Some(2.2),
                        min_holding_bars: 2,
                        ..StopTakeConfig::default()
                    },
                    lot_step: 100.0,
                    commission_bps: 15.0,
                    tax_sell_bps: 10.0,
                    ..BacktestConfig::default()
                };
                let _ = self.import_backtest_trades_to_journal(10);
                for (i, e) in self.journal.entries.iter_mut().enumerate() {
                    e.checklist = ChecklistState::all_checked();
                    e.conviction = if e.net_pnl >= 0.0 { 4 } else { 3 };
                    if i % 2 == 1 {
                        e.setup = SetupTag::TrendPullback;
                    }
                }
                if let Some(losing_entry) =
                    self.journal.entries.iter_mut().find(|e| e.net_pnl < 0.0)
                {
                    losing_entry.mistake = MistakeTag::FomoChase;
                    losing_entry.checklist.calm_state = false;
                    losing_entry.checklist.level_confirmed = false;
                    losing_entry.conviction = 2;
                    losing_entry.notes =
                        "Chased breakout above intraday VWAP without volume confirmation"
                            .to_owned();
                }

                // Configure HOSE Risk Planner + draw dual-zone `RiskRewardBox` on `Main` pane
                let mark = self.current_mark_price().max(10.0);
                let cap = if mark >= 1_000.0 {
                    250_000_000.0
                } else {
                    250_000.0
                };
                self.risk_plan_input = TradePlanInput {
                    venue: ExchangeVenue::Hose,
                    price_unit: VnPriceUnit::Auto,
                    side: PositionSide::Long,
                    account_equity: cap,
                    available_cash: cap,
                    leverage: 1.0,
                    risk_pct: 1.0,
                    entry_price: mark * 0.973,
                    stop_loss: mark * 0.938,
                    target_price: mark * 1.062,
                    reference_price: self.causal_reference_price(),
                    commission_bps: 15.0,
                    tax_sell_bps: 10.0,
                    slippage_bps: 5.0,
                    snap_to_ticks: true,
                    entry_timestamp: self.candles.last().map(|c| c.timestamp),
                    ..TradePlanInput::default()
                };
                let _ = self.snap_planner_ticks_and_sync_rr_box();
                let _ = self.draw_or_update_rr_box_from_planner();
                self.journal_draft_setup = SetupTag::TrendPullback;
                self.journal_draft_conviction = 5;
                self.journal_draft_note =
                    "FPT pullback to SMA15 support · HOSE T+2 swing plan".to_owned();
                let _ = self.log_active_plan_to_journal();
                self.open_journal_tab();

                (
                    1280,
                    800,
                    "docs/evidence/r10_vn_hose_risk_planner_and_rr_box_fpt_1280x800.png",
                    "docs/evidence/r10_vn_hose_risk_planner_and_rr_box_fpt_1280x800.json",
                )
            }
            1 => {
                // Scenario 1: `1280x800` (`2560x1600` Retina), `FPT` (`VN30` `1D`),
                // Strategy Lab -> `WFO` Sub-Tab: `4`-Fold Rolling Walk-Forward (`Pardo WFE` + Stitched OOS Equity)
                // + `VN30 Basket (8)` Causal Inv-Vol Multi-Asset Portfolio Simulation!
                self.exit_replay();
                self.select_watchlist_instrument(DataSourcePreset::Vn30Basket, "FPT");
                self.indicators = IndicatorConfig {
                    show_sma: true,
                    show_ema: true,
                    show_bbands: false,
                    show_rsi: true,
                    show_macd: false,
                    show_atr: false,
                    sma_period: 15,
                    ema_period: 40,
                    ..IndicatorConfig::default()
                };
                self.set_scale_mode(PriceScaleMode::Normal);
                self.viewport.reset_pane_stretches();
                self.viewport.equity_stretch = 1.45;
                self.viewport.rsi_stretch = 0.80;

                self.backtest_config = BacktestConfig {
                    preset: StrategyPresetKind::MacdMomentum,
                    direction_mode: StrategyDirectionMode::LongOnly,
                    sizing: PositionSizingMode::PercentEquity(95.0),
                    risk: StopTakeConfig {
                        stop_loss_pct: Some(3.2),
                        take_profit_pct: Some(8.5),
                        trailing_stop_atr_mult: Some(2.2),
                        min_holding_bars: 2,
                        ..StopTakeConfig::default()
                    },
                    initial_cash: 100_000.0,
                    commission_bps: 15.0,
                    tax_sell_bps: 10.0,
                    slippage_bps: 5.0,
                    max_leverage: 1.0,
                    lot_step: 100.0,
                    in_sample_split_pct: Some(0.70),
                    ..BacktestConfig::default()
                };
                self.wf_config = WalkForwardConfig {
                    fold_count: 4,
                    in_sample_ratio: 0.70,
                    anchored: false,
                    ranking_metric: SweepRankingMetric::SharpeRatio,
                };
                self.portfolio_universe_filter = PortfolioUniverseFilter::Vn30EquitiesOnly;
                self.portfolio_weighting =
                    PortfolioWeightingMode::CausalInverseVolatility { lookback_bars: 20 };

                self.show_scanner_dock = false;
                self.show_replay_bar = false;
                self.show_trading_panel = false;
                self.show_alerts_panel = false;
                self.show_journal_panel = false;
                self.open_strategy_lab();
                self.strategy_lab_tab = StrategyLabSubTab::WalkForward;
                self.show_equity_pane = true;
                self.show_backtest_trades = false;
                self.equity_pane_source = EquityPaneSource::WalkForwardOos;
                self.run_active_walk_forward();
                self.run_active_portfolio_backtest();

                self.viewport.time_scale.set_width(880.0);
                self.viewport.fit_all();

                (
                    1280,
                    800,
                    "docs/evidence/r10_walk_forward_and_portfolio_equity_1280x800.png",
                    "docs/evidence/r10_walk_forward_and_portfolio_equity_1280x800.json",
                )
            }
            _ => {
                // Scenario 2: Compact `960x640` (`1920x1280` Retina), Intraday `VN30F1M` `1m -> 15m`
                // under active Causal Bar Replay (`DEF-001` future-entry masking), `SHORT` `RiskRewardBox`,
                // and synced Paper Trading round-trips in `VN RISK & JOURNAL`!
                self.exit_replay();
                self.select_watchlist_instrument(DataSourcePreset::IntradayVn30F1m, "VN30F1M");
                let _ = self.set_timeframe(Timeframe::M15);
                self.indicators = IndicatorConfig {
                    show_sma: true,
                    show_ema: true,
                    show_bbands: false,
                    show_rsi: false,
                    show_macd: true,
                    show_atr: false,
                    sma_period: 12,
                    ema_period: 34,
                    ..IndicatorConfig::default()
                };
                self.set_scale_mode(PriceScaleMode::Normal);
                self.viewport.reset_pane_stretches();
                self.viewport.macd_stretch = 0.82;
                self.journal = TradeJournalStore::new();

                let n_base = self.base_candles.len();
                let t1_idx = ((n_base * 52) / 100).max(20);
                let t1_exit = ((n_base * 60) / 100).max(t1_idx + 5);
                let t2_exit = ((n_base * 68) / 100).max(t1_exit + 5);
                let replay_cut = ((n_base * 72) / 100).max(t2_exit + 3);
                let future_idx = ((n_base * 84) / 100)
                    .max(replay_cut + 10)
                    .min(n_base.saturating_sub(2));

                let _ = self.start_replay_at_base_index(t1_idx);
                let _ = self.trading.submit_order(
                    OrderRequest {
                        side: OrderSide::Buy,
                        kind: OrderKind::Market,
                        quantity: 5.0,
                        stop_loss: None,
                        take_profit: None,
                    },
                    &self.base_candles,
                    t1_idx,
                );
                if let Some(tr) = self.replay.jump_to_base_index(&self.base_candles, t1_exit) {
                    self.apply_replay_transition(tr);
                }
                let _ = self.flatten_open_position();

                let _ = self.trading.submit_order(
                    OrderRequest {
                        side: OrderSide::Sell,
                        kind: OrderKind::Market,
                        quantity: 4.0,
                        stop_loss: None,
                        take_profit: None,
                    },
                    &self.base_candles,
                    t1_exit,
                );
                if let Some(tr) = self.replay.jump_to_base_index(&self.base_candles, t2_exit) {
                    self.apply_replay_transition(tr);
                }
                let _ = self.flatten_open_position();
                let _ = self.sync_paper_trades_to_journal();

                // Annotate journal setups & record a future planned entry at `future_idx`, then rewind to `replay_cut`
                // so `DEF-001` causal masking actively hides future entries while preserving past round-trips!
                for (i, entry) in self.journal.entries.iter_mut().enumerate() {
                    entry.setup = if i % 2 == 0 {
                        SetupTag::MomentumContinuation
                    } else {
                        SetupTag::LiquiditySweep
                    };
                    entry.conviction = 4;
                    if entry.net_pnl < 0.0 {
                        entry.mistake = MistakeTag::CounterTrendImpulse;
                        entry.checklist.trend_aligned = false;
                    }
                }
                if future_idx < self.base_candles.len()
                    && let Ok(plan) = self.compute_active_trade_plan()
                {
                    let fut_ts = self.base_candles[future_idx].timestamp + 60;
                    let _ = self.journal.record_trade_plan(
                        "VN30F1M",
                        &plan,
                        fut_ts,
                        SetupTag::SqueezeBreakout,
                        4,
                        ChecklistState::all_checked(),
                        "Future plan masked at 72% replay cut",
                    );
                }
                if let Some(tr) = self
                    .replay
                    .jump_to_base_index(&self.base_candles, replay_cut)
                {
                    self.apply_replay_transition(tr);
                }

                self.viewport.time_scale.set_width(590.0);
                self.viewport.time_scale.set_bar_spacing(9.0);
                self.viewport.time_scale.set_right_offset(10.0);

                // Draw `SHORT` `RiskRewardBox` on `VN30F1M` (`T+0`, lot `1`, tick `0.1`, `100 kVND/pt`, `17%` margin)
                let mark = self.current_mark_price().max(500.0);
                self.risk_plan_input.side = PositionSide::Short;
                self.risk_plan_input.account_equity = 150_000.0;
                self.risk_plan_input.available_cash = 150_000.0;
                self.risk_plan_input.risk_pct = 1.50;
                self.risk_plan_input.entry_price = mark * 1.003;
                self.risk_plan_input.stop_loss = mark * 1.011;
                self.risk_plan_input.target_price = mark * 0.979;
                let _ = self.snap_planner_ticks_and_sync_rr_box();
                let _ = self.draw_or_update_rr_box_from_planner();
                self.last_journal_status = None;

                self.show_scanner_dock = false;
                self.show_replay_bar = true;
                self.show_trading_panel = false;
                self.show_alerts_panel = false;
                self.show_strategy_panel = false;
                self.show_equity_pane = false;
                self.show_backtest_trades = false;
                self.open_journal_tab();

                (
                    960,
                    640,
                    "docs/evidence/r10_trade_journal_discipline_replay_960x640.png",
                    "docs/evidence/r10_trade_journal_discipline_replay_960x640.json",
                )
            }
        }
    }

    fn apply_r11_batch_step_state(
        &mut self,
        step: usize,
    ) -> (u32, u32, &'static str, &'static str) {
        self.close_command_palette();
        self.show_import_bar = false;
        self.alerts.clear_all();
        self.drawings = DrawingStore::new();
        self.trading = TradingSession::default();
        self.risk_venue_override = None;
        self.show_scanner_dock = false;
        self.show_trading_panel = false;
        self.show_alerts_panel = false;
        self.show_strategy_panel = false;
        self.show_journal_panel = false;
        self.show_equity_pane = false;
        self.show_backtest_trades = false;
        self.show_compare_overlay = false;
        self.compare_benchmark = None;

        match step {
            0 => {
                // Scenario 0: `1280x800` (`2560x1600` Retina), `FPT` (`VN30` `1D` `HOSE`),
                // Order-Flow `VRVP` + Swing `FRVP` + Monthly `VWAP (±1σ, ±2σ)` + Rebased Compare `VNINDEX` + `RS vs VNINDEX` sub-pane.
                self.exit_replay();
                self.select_watchlist_instrument(DataSourcePreset::Vn30Basket, "FPT");
                let _ = self.set_timeframe(Timeframe::D1);
                self.indicators = IndicatorConfig {
                    show_sma: true,
                    show_ema: true,
                    show_bbands: false,
                    show_vwap: true,
                    show_vrvp: true,
                    vrvp_rows: 48,
                    show_rsi: false,
                    show_macd: false,
                    show_atr: false,
                    show_rs: true,
                    rs_sma_period: 20,
                    sma_period: 20,
                    ema_period: 50,
                    ..IndicatorConfig::default()
                };
                self.show_compare_overlay = true;
                self.set_compare_benchmark(Some("VNINDEX".to_owned()));
                self.set_scale_mode(PriceScaleMode::Normal);
                self.viewport.reset_pane_stretches();
                self.viewport.rs_stretch = 0.85;
                self.viewport.time_scale.set_width(1190.0);
                self.viewport.time_scale.set_bar_spacing(9.0);
                self.viewport.time_scale.set_right_offset(18.0);
                self.show_replay_bar = false;

                let n = self.candles.len();
                if n >= 20 {
                    let i0 = (n * 42) / 100;
                    let i1 = ((n * 62) / 100).max(i0 + 8).min(n - 1);
                    let _ = self.add_frvp_drawing(i0, i1, 32);
                    let _ = self.add_anchored_vwap_drawing((n * 45) / 100, false);
                }
                self.sync_indicators_and_benchmarks();

                (
                    1280,
                    800,
                    "docs/evidence/r11_vrvp_frvp_vwap_and_rs_vnindex_fpt_1280x800.png",
                    "docs/evidence/r11_vrvp_frvp_vwap_and_rs_vnindex_fpt_1280x800.json",
                )
            }
            1 => {
                // Scenario 1: `1280x800` (`2560x1600` Retina), `VN30F1M` (`15m` Intraday `UTC+7`),
                // Intraday `Session VWAP` resetting cleanly across `UTC+7` day boundaries + `Anchored VWAP (±1σ)` + `VRVP` + `FRVP` + `RS vs VNINDEX`.
                self.exit_replay();
                self.select_watchlist_instrument(DataSourcePreset::IntradayVn30F1m, "VN30F1M");
                let _ = self.set_timeframe(Timeframe::M15);
                self.indicators = IndicatorConfig {
                    show_sma: false,
                    show_ema: true,
                    show_bbands: false,
                    show_vwap: true,
                    show_vrvp: true,
                    vrvp_rows: 64,
                    show_rsi: false,
                    show_macd: false,
                    show_atr: false,
                    show_rs: true,
                    rs_sma_period: 20,
                    ema_period: 34,
                    ..IndicatorConfig::default()
                };
                self.set_scale_mode(PriceScaleMode::Normal);
                self.viewport.reset_pane_stretches();
                self.viewport.rs_stretch = 0.85;
                self.viewport.time_scale.set_width(1190.0);
                self.viewport.time_scale.set_bar_spacing(8.5);
                self.viewport.time_scale.set_right_offset(16.0);
                self.show_replay_bar = false;

                let n = self.candles.len();
                if n >= 24 {
                    let avwap_bar = (n * 72) / 100;
                    let frvp_start = (n * 76) / 100;
                    let frvp_end = ((n * 92) / 100).max(frvp_start + 6).min(n - 1);
                    let _ = self.add_anchored_vwap_drawing(avwap_bar, true);
                    let _ = self.add_frvp_drawing(frvp_start, frvp_end, 36);
                }
                self.sync_indicators_and_benchmarks();

                (
                    1280,
                    800,
                    "docs/evidence/r11_intraday_session_vwap_avwap_vn30f_1280x800.png",
                    "docs/evidence/r11_intraday_session_vwap_avwap_vn30f_1280x800.json",
                )
            }
            _ => {
                // Scenario 2: `960x640` (`1920x1280` Retina), `FPT` (`1D`) in Causal Bar Replay (`DEF-001`),
                // Future `AVWAP` masked cleanly behind replay frontier while causal `VRVP`, `FRVP`, `Session VWAP`, `Compare` and `RS vs VNINDEX` remain active.
                self.exit_replay();
                self.select_watchlist_instrument(DataSourcePreset::Vn30Basket, "FPT");
                let _ = self.set_timeframe(Timeframe::D1);
                self.indicators = IndicatorConfig {
                    show_sma: true,
                    show_ema: false,
                    show_bbands: false,
                    show_vwap: true,
                    show_vrvp: true,
                    vrvp_rows: 48,
                    show_rsi: false,
                    show_macd: false,
                    show_atr: false,
                    show_rs: true,
                    rs_sma_period: 20,
                    sma_period: 20,
                    ..IndicatorConfig::default()
                };
                self.show_compare_overlay = true;
                self.set_compare_benchmark(Some("VNINDEX".to_owned()));
                self.set_scale_mode(PriceScaleMode::Normal);
                self.viewport.reset_pane_stretches();
                self.viewport.rs_stretch = 0.85;

                let n_base = self.base_candles.len();
                let replay_cut = ((n_base * 68) / 100).max(20).min(n_base.saturating_sub(8));
                let future_idx = ((n_base * 88) / 100)
                    .max(replay_cut + 5)
                    .min(n_base.saturating_sub(1));

                // Create a future AVWAP at `future_idx`, then rewind to `replay_cut` so `DEF-001` masks it.
                let _ = self.start_replay_at_base_index(future_idx);
                let _ = self.add_anchored_vwap_drawing(self.candles.len().saturating_sub(2), true);
                if let Some(tr) = self
                    .replay
                    .jump_to_base_index(&self.base_candles, replay_cut)
                {
                    self.apply_replay_transition(tr);
                }

                let n = self.candles.len();
                if n >= 16 {
                    let i0 = (n * 45) / 100;
                    let i1 = ((n * 72) / 100).max(i0 + 6).min(n - 1);
                    let _ = self.add_frvp_drawing(i0, i1, 28);
                    let _ = self.add_anchored_vwap_drawing((n * 48) / 100, true);
                }

                self.viewport.time_scale.set_width(880.0);
                self.viewport.time_scale.set_bar_spacing(8.5);
                self.viewport.time_scale.set_right_offset(14.0);
                self.show_replay_bar = true;
                self.sync_indicators_and_benchmarks();

                (
                    960,
                    640,
                    "docs/evidence/r11_causal_replay_avwap_frvp_rs_960x640.png",
                    "docs/evidence/r11_causal_replay_avwap_frvp_rs_960x640.json",
                )
            }
        }
    }

    fn persist_capture_artifacts(
        &self,
        screenshot_path: &str,
        telemetry_path: Option<&str>,
        image: &egui::ColorImage,
    ) {
        if let Err(err) = Self::write_color_image_png(screenshot_path, image) {
            log::error!("Failed to save verification screenshot to {screenshot_path}: {err}");
        } else {
            log::info!(
                "Saved native GPU verification screenshot ({}x{}) to {screenshot_path}",
                image.width(),
                image.height()
            );
        }

        if let Some(t_path) = telemetry_path.filter(|p| !p.is_empty())
            && let Some(rep) = self.last_report
        {
            let selected_json = rep
                .selected_drawing_id
                .map_or_else(|| "null".to_owned(), |id| id.to_string());
            let magnet_str = match rep.magnet_mode {
                MagnetMode::Off => "Off",
                MagnetMode::Ohlc => "Ohlc",
            };
            let avail_syms_json: Vec<String> = self
                .available_symbols
                .iter()
                .map(|s| format!("\"{}\"", s.replace('"', "\\\"")))
                .collect();
            let metrics = self.trading.metrics(self.current_mark_price());
            let pos_side_json = self.trading.open_position().map_or_else(
                || "null".to_owned(),
                |p| format!("\"{}\"", p.side.short_label()),
            );
            let replay_cutoff = self.replay.cutoff_timestamp(&self.base_candles);
            let scanner_sig_revealed = self.scanner_context.as_ref().map_or_else(
                || "null".to_owned(),
                |ctx| ctx.is_revealed(replay_cutoff).to_string(),
            );
            let bt_active =
                self.show_strategy_panel || self.show_equity_pane || self.show_backtest_trades;
            let bt_ref = bt_active.then_some(self.backtest_result.as_ref()).flatten();
            let bt_preset_json = bt_ref.map_or_else(
                || "null".to_owned(),
                |b| format!("\"{}\"", b.config.preset.short_label()),
            );
            let bt_trades_count = bt_ref.map_or(0, |b| b.tearsheet.num_trades);
            let bt_ret_pct = bt_ref.map_or(0.0, |b| b.tearsheet.total_return_pct);
            let bt_mdd_pct = bt_ref.map_or(0.0, |b| b.tearsheet.max_drawdown_pct);
            let bt_sharpe_json = bt_ref
                .and_then(|b| b.tearsheet.sharpe_ratio)
                .map_or_else(|| "null".to_owned(), |v| format!("{v:.4}"));
            let bt_rob_json = bt_ref
                .and_then(|b| b.robustness)
                .map_or_else(|| "null".to_owned(), |v| format!("\"{}\"", v.label()));
            let j_stats = self
                .journal
                .compute_analytics(self.replay_causal_close_timestamp());
            let wfo_verdict_json = self.walk_forward_report.as_ref().map_or_else(
                || "null".to_owned(),
                |w| format!("\"{}\"", w.verdict.label()),
            );
            let wfo_wfe_json = self
                .walk_forward_report
                .as_ref()
                .and_then(|w| w.wfe_pct)
                .map_or_else(|| "null".to_owned(), |v| format!("{v:.4}"));
            let port_ret_pct = self
                .portfolio_report
                .as_ref()
                .map_or(0.0, |p| p.portfolio_tearsheet.total_return_pct);
            let port_dd_red_pct = self
                .portfolio_report
                .as_ref()
                .map_or(0.0, |p| p.diversification_dd_reduction_pct);
            let vrvp_poc_json = rep
                .vrvp_poc_price
                .map_or_else(|| "null".to_owned(), |v| format!("{v:.4}"));
            let compare_bench_json = self.compare_benchmark.as_deref().map_or_else(
                || "null".to_owned(),
                |b| format!("\"{}\"", b.replace('"', "\\\"")),
            );
            let json = format!(
                "{{\n  \"symbol\": \"{}\",\n  \"timeframe\": \"{}\",\n  \"base_timeframe\": \"{}\",\n  \"data_source\": \"{}\",\n  \"available_symbols\": [{}],\n  \"base_candles\": {},\n  \"total_candles\": {},\n  \"culled_candles\": {},\n  \"rendered_candles\": {},\n  \"conflation_factor\": {},\n  \"bar_spacing\": {:.6},\n  \"right_offset\": {:.6},\n  \"scale_mode\": \"{}\",\n  \"auto_scale\": {},\n  \"pane_count\": {},\n  \"active_overlay_count\": {},\n  \"drawing_count\": {},\n  \"active_drawing_tool\": \"{}\",\n  \"selected_drawing_id\": {},\n  \"magnet_mode\": \"{}\",\n  \"replay_active\": {},\n  \"replay_status\": \"{}\",\n  \"replay_base_cursor\": {},\n  \"replay_step_unit\": \"{}\",\n  \"show_trading_panel\": {},\n  \"show_scanner_dock\": {},\n  \"show_alerts_panel\": {},\n  \"show_strategy_panel\": {},\n  \"show_journal_panel\": {},\n  \"show_equity_pane\": {},\n  \"equity_pane_source\": \"{}\",\n  \"equity_pane_active\": {},\n  \"show_vwap\": {},\n  \"show_vrvp\": {},\n  \"show_rs\": {},\n  \"show_compare_overlay\": {},\n  \"compare_benchmark\": {},\n  \"vrvp_poc_price\": {},\n  \"active_frvp_count\": {},\n  \"rs_stretch\": {:.3},\n  \"active_venue\": \"{}\",\n  \"journal_total_entries\": {},\n  \"journal_visible_entries\": {},\n  \"journal_discipline_pct\": {:.2},\n  \"wfo_verdict\": {},\n  \"wfo_wfe_pct\": {},\n  \"portfolio_return_pct\": {:.4},\n  \"portfolio_dd_reduction_pct\": {:.4},\n  \"show_backtest_trades\": {},\n  \"rendered_backtest_trades\": {},\n  \"backtest_preset\": {},\n  \"backtest_trade_count\": {},\n  \"backtest_return_pct\": {:.4},\n  \"backtest_max_dd_pct\": {:.4},\n  \"backtest_sharpe\": {},\n  \"backtest_robustness\": {},\n  \"command_palette_open\": {},\n  \"alert_rule_count\": {},\n  \"alert_trigger_count\": {},\n  \"active_alert_lines\": {},\n  \"rendered_alert_markers\": {},\n  \"scanner_signal_revealed\": {},\n  \"active_price_lines\": {},\n  \"rendered_fill_markers\": {},\n  \"equity\": {:.2},\n  \"net_pnl\": {:.2},\n  \"fill_count\": {},\n  \"closed_trade_count\": {},\n  \"pending_order_count\": {},\n  \"open_position_side\": {},\n  \"stretches\": [{:.3}, {:.3}, {:.3}, {:.3}, {:.3}],\n  \"base_value\": {:.4},\n  \"screenshot_px\": [{}, {}],\n  \"outer_rect\": [{:.1}, {:.1}, {:.1}, {:.1}],\n  \"plot_rect\": [{:.1}, {:.1}, {:.1}, {:.1}],\n  \"price_axis_rect\": [{:.1}, {:.1}, {:.1}, {:.1}],\n  \"time_axis_rect\": [{:.1}, {:.1}, {:.1}, {:.1}],\n  \"raw_price_min\": {:.2},\n  \"raw_price_max\": {:.2}\n}}\n",
                self.symbol,
                self.timeframe,
                self.base_timeframe,
                self.data_source.short_label(),
                avail_syms_json.join(", "),
                self.base_candles.len(),
                self.candles.len(),
                rep.culled_bar_count,
                rep.rendered_candle_count,
                rep.conflation_factor,
                rep.bar_spacing,
                rep.right_offset,
                self.viewport.main_price_scale.mode().label(),
                self.viewport.main_price_scale.is_auto_scale(),
                rep.pane_count,
                rep.active_overlay_count,
                rep.drawing_count,
                rep.active_drawing_tool.short_label(),
                selected_json,
                magnet_str,
                self.replay.is_active(),
                self.replay.status.short_label(),
                self.current_trading_cursor_base_index(),
                self.replay.step_unit.short_label(),
                self.show_trading_panel,
                self.show_scanner_dock,
                self.show_alerts_panel,
                self.show_strategy_panel,
                self.show_journal_panel,
                self.show_equity_pane,
                self.equity_pane_source.short_label(),
                rep.equity_pane_active,
                self.indicators.show_vwap,
                self.indicators.show_vrvp,
                self.indicators.show_rs,
                self.show_compare_overlay,
                compare_bench_json,
                vrvp_poc_json,
                rep.active_frvp_count,
                self.viewport.rs_stretch,
                self.active_exchange_venue().short_label(),
                self.journal.entries.len(),
                j_stats.visible_entry_count,
                j_stats.discipline_score_pct,
                wfo_verdict_json,
                wfo_wfe_json,
                port_ret_pct,
                port_dd_red_pct,
                self.show_backtest_trades,
                rep.rendered_backtest_trade_count,
                bt_preset_json,
                bt_trades_count,
                bt_ret_pct,
                bt_mdd_pct,
                bt_sharpe_json,
                bt_rob_json,
                self.command_palette.open,
                self.alerts.rules.len(),
                self.alerts.triggers.len(),
                rep.active_alert_line_count,
                rep.rendered_alert_marker_count,
                scanner_sig_revealed,
                rep.active_price_line_count,
                rep.rendered_fill_marker_count,
                metrics.equity,
                metrics.net_pnl,
                metrics.fill_count,
                metrics.closed_trade_count,
                metrics.pending_order_count,
                pos_side_json,
                self.viewport.main_stretch,
                self.viewport.rsi_stretch,
                self.viewport.macd_stretch,
                self.viewport.atr_stretch,
                self.viewport.equity_stretch,
                rep.base_value,
                image.width(),
                image.height(),
                rep.layout.outer_rect.left(),
                rep.layout.outer_rect.top(),
                rep.layout.outer_rect.width(),
                rep.layout.outer_rect.height(),
                rep.layout.plot_rect.left(),
                rep.layout.plot_rect.top(),
                rep.layout.plot_rect.width(),
                rep.layout.plot_rect.height(),
                rep.layout.price_axis_rect.left(),
                rep.layout.price_axis_rect.top(),
                rep.layout.price_axis_rect.width(),
                rep.layout.price_axis_rect.height(),
                rep.layout.time_axis_rect.left(),
                rep.layout.time_axis_rect.top(),
                rep.layout.time_axis_rect.width(),
                rep.layout.time_axis_rect.height(),
                rep.bounds.raw_min,
                rep.bounds.raw_max,
            );
            let _ = std::fs::write(t_path, json);
        }
    }

    fn write_color_image_png(path: &str, image: &egui::ColorImage) -> std::io::Result<()> {
        let file = std::fs::File::create(path)?;
        let writer = std::io::BufWriter::new(file);
        let mut encoder = png::Encoder::new(writer, image.width() as u32, image.height() as u32);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut png_writer = encoder
            .write_header()
            .map_err(|e| std::io::Error::other(e.to_string()))?;

        let mut rgba_bytes = Vec::with_capacity(image.pixels.len() * 4);
        for pixel in &image.pixels {
            rgba_bytes.extend_from_slice(&pixel.to_array());
        }
        png_writer
            .write_image_data(&rgba_bytes)
            .map_err(|e| std::io::Error::other(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_state_zoom_fit_scale_modes_indicators_and_dataset_switching_stay_within_bounds() {
        let mut app = SumiWorkstationApp::default();
        assert_eq!(app.candles.len(), 120);
        assert_eq!(app.indicator_cache.sma.len(), 120);
        assert_eq!(app.indicator_cache.rsi.len(), 120);
        app.viewport.time_scale.set_width(1000.0);

        let initial_bs = app.viewport.time_scale.bar_spacing();
        app.zoom_in();
        assert!(app.viewport.time_scale.bar_spacing() > initial_bs);

        app.zoom_out();
        app.zoom_out();
        assert!(app.viewport.time_scale.bar_spacing() < initial_bs);

        for mode in [
            PriceScaleMode::Logarithmic,
            PriceScaleMode::Percentage,
            PriceScaleMode::IndexedTo100,
            PriceScaleMode::Normal,
        ] {
            app.set_scale_mode(mode);
            assert_eq!(app.viewport.main_price_scale.mode(), mode);
        }

        app.indicators.show_bbands = true;
        app.indicators.show_macd = true;
        app.indicators.show_atr = true;
        app.set_dataset_size(10_000);
        assert_eq!(app.candles.len(), 10_000);
        assert_eq!(app.viewport.time_scale.total_bars(), 10_000);
        assert_eq!(app.indicator_cache.bbands.upper.len(), 10_000);
        assert_eq!(app.indicator_cache.macd.histogram.len(), 10_000);
        assert_eq!(app.indicator_cache.atr.len(), 10_000);

        app.fit_content();
        assert_eq!(app.visible_slice().len(), 10_000);

        app.reset_view();
        assert!((app.viewport.time_scale.right_offset() - 12.0).abs() < 1e-6);
        assert!((app.viewport.time_scale.bar_spacing() - 8.0).abs() < 1e-6);

        // Verify Critic R05 atomic `populate_showcase_drawings` preserves `magnet_mode`, prior user drawings
        // on 1-step Undo, `locked = true` across Undo -> Redo, and `has_unlocked_drawings` state!
        app.drawings.magnet_mode = MagnetMode::Ohlc;
        let user_id = app
            .drawings
            .add_drawing(
                DrawingPaneTarget::Main,
                DrawingGeometry::HorizontalLine {
                    anchor: DrawingAnchor::new(9950.0, 1200.0),
                },
                None,
            )
            .unwrap();
        app.drawings.selected_id = Some(user_id);

        app.populate_showcase_drawings();
        assert_eq!(app.drawings.magnet_mode, MagnetMode::Ohlc);
        assert_eq!(app.drawings.drawings.len(), 8);
        assert!(app.drawings.drawings[0].locked);
        assert!(app.drawings.selected_id.is_some());
        assert!(app.drawings.has_unlocked_drawings());

        assert!(app.drawings.undo());
        assert_eq!(app.drawings.drawings.len(), 1);
        assert_eq!(app.drawings.drawings[0].id, user_id);
        assert_eq!(app.drawings.selected_id, Some(user_id));

        assert!(app.drawings.redo());
        assert_eq!(app.drawings.drawings.len(), 8);
        assert!(
            app.drawings.drawings[0].locked,
            "locked support line must remain locked after Undo -> Redo"
        );

        let unlocked_cleared = app.drawings.clear_unlocked();
        assert_eq!(unlocked_cleared, 7);
        assert_eq!(
            app.drawings.drawings.len(),
            1,
            "locked support line stays preserved"
        );
        assert!(
            !app.drawings.has_unlocked_drawings(),
            "Clear button must disable once only locked drawings remain"
        );
    }

    #[test]
    fn r06_workstation_presets_csv_json_import_and_multi_timeframe_switching_preserve_focus_and_drawings()
     {
        let mut app = SumiWorkstationApp::default();
        app.viewport.time_scale.set_width(1000.0);

        // 1. Daily source rejects downsampling to `5m` cleanly while supporting `1W`
        assert!(app.set_timeframe(Timeframe::M5).is_err());
        assert!(app.set_timeframe(Timeframe::W1).is_ok());
        assert_eq!(app.active_timeframe, Timeframe::W1);
        assert_eq!(app.timeframe, "1W");
        assert!(app.candles.len() < app.base_candles.len());

        // 2. Switch to Intraday `VN30F1M` (`1m` base, `5,400` bars) and test full `1m..1W` traversal!
        app.select_data_source(DataSourcePreset::IntradayVn30F1m);
        assert_eq!(app.base_timeframe, Timeframe::M1);
        assert_eq!(app.active_timeframe, Timeframe::M1);
        assert_eq!(app.candles.len(), 5_400);

        // Pan deep into historical sessions (`right_offset = -320`) and place an annotation on `5m`
        app.viewport.time_scale.set_width(1_000.0);
        app.set_timeframe(Timeframe::M5).unwrap();
        app.viewport.time_scale.set_right_offset(-320.0);
        let center_5m_idx = app
            .viewport
            .time_scale
            .local_x_to_center_float_index(500.0)
            .unwrap();
        let expected_center_ts =
            resolve_anchor_timestamp(center_5m_idx, &app.candles, Timeframe::M5).unwrap();

        let d_id = app
            .drawings
            .add_drawing(
                DrawingPaneTarget::Main,
                DrawingGeometry::TrendLine {
                    start: DrawingAnchor::new(220.5, 1280.0),
                    end: DrawingAnchor::new(410.25, 1295.0),
                },
                None,
            )
            .unwrap();

        for tf in [
            Timeframe::M15,
            Timeframe::H1,
            Timeframe::H4,
            Timeframe::D1,
            Timeframe::W1,
        ] {
            app.set_timeframe(tf).unwrap();
            assert_eq!(app.indicator_cache.sma.len(), app.candles.len());
        }

        // Return to `5m` and verify both drawing anchor and historical viewport focus match!
        app.set_timeframe(Timeframe::M5).unwrap();
        let restored = app.drawings.drawings.iter().find(|d| d.id == d_id).unwrap();
        let [a0, a1] = restored.geometry.anchors();
        assert!((a0.bar_index - 220.5).abs() < 1e-9);
        assert!((a1.bar_index - 410.25).abs() < 1e-9);

        let restored_center_idx = app
            .viewport
            .time_scale
            .local_x_to_center_float_index(500.0)
            .unwrap();
        let restored_center_ts =
            resolve_anchor_timestamp(restored_center_idx, &app.candles, Timeframe::M5).unwrap();
        assert!(
            (restored_center_ts - expected_center_ts).abs() < 1e-3,
            "viewport center timestamp must stay locked across 5m -> 1W -> 5m: got {restored_center_ts} vs {expected_center_ts}"
        );

        // 3. CafeF multi-symbol switcher (`HNX-INDEX` <-> `VNINDEX`) & JSON string loader
        app.select_data_source(DataSourcePreset::CafeFIndexCsv);
        assert!(app.available_symbols.len() >= 2);
        app.select_symbol_in_source("HNX-INDEX");
        assert_eq!(app.symbol, "HNX-INDEX");
        app.select_symbol_in_source("VNINDEX");
        assert_eq!(app.symbol, "VNINDEX");

        let custom_json = crate::data::build_btcusdt_15m_json_document(96);
        app.load_from_json_str(&custom_json, &DataLoadOptions::default())
            .unwrap();
        assert_eq!(app.symbol, "BTCUSDT");
        assert_eq!(app.base_timeframe, Timeframe::M15);
        assert_eq!(app.candles.len(), 96);
    }

    #[test]
    fn critic_r06_regression_persisted_session_restore_preserves_4h_timeframe_drawings_and_missing_file_fallback()
     {
        // 1. Configure workstation on `BTCUSDT` (`15m` base -> `4h` active = `84` bars), pan into history,
        //    and add a `4h` TrendLine, then simulate `eframe` JSON persistence round-trip (`CRITICAL-01`).
        let mut app = SumiWorkstationApp::default();
        app.viewport.time_scale.set_width(1_000.0);
        app.select_data_source(DataSourcePreset::BtcUsdt15mJson);
        app.set_timeframe(Timeframe::H4).unwrap();
        assert_eq!(app.candles.len(), 84);
        app.viewport.time_scale.set_right_offset(-18.5);
        let saved_ro = app.viewport.time_scale.right_offset();

        let d_id = app
            .drawings
            .add_drawing(
                DrawingPaneTarget::Main,
                DrawingGeometry::TrendLine {
                    start: DrawingAnchor::new(14.0, 63_800.0),
                    end: DrawingAnchor::new(52.0, 65_200.0),
                },
                None,
            )
            .unwrap();

        #[derive(Default)]
        struct MemStorage(BTreeMap<String, String>);
        impl eframe::Storage for MemStorage {
            fn get_string(&self, key: &str) -> Option<String> {
                self.0.get(key).cloned()
            }
            fn set_string(&mut self, key: &str, value: String) {
                self.0.insert(key.to_owned(), value);
            }
            fn remove_string(&mut self, key: &str) {
                self.0.remove(key);
            }
            fn flush(&mut self) {}
        }

        let mut storage = MemStorage::default();
        eframe::App::save(&mut app, &mut storage);
        let mut restored: SumiWorkstationApp =
            eframe::get_value(&storage, eframe::APP_KEY).unwrap();
        restored.restore_persisted_session();

        assert_eq!(restored.data_source, DataSourcePreset::BtcUsdt15mJson);
        assert_eq!(restored.base_timeframe, Timeframe::M15);
        assert_eq!(restored.active_timeframe, Timeframe::H4);
        assert_eq!(restored.timeframe, "4h");
        assert_eq!(restored.candles.len(), 84);
        assert!(
            (restored.viewport.time_scale.right_offset() - saved_ro).abs() < 1e-9,
            "persisted right_offset must not shift by base_len - active_len on restore"
        );
        let d_restored = restored
            .drawings
            .drawings
            .iter()
            .find(|d| d.id == d_id)
            .unwrap();
        let [a0, a1] = d_restored.geometry.anchors();
        assert!((a0.bar_index - 14.0).abs() < 1e-9);
        assert!((a1.bar_index - 52.0).abs() < 1e-9);
        assert!(a0.timestamp.is_some() && a1.timestamp.is_some());

        // Switching `4h -> 15m -> 4h` after session restore must preserve exact `4h` indices!
        restored.set_timeframe(Timeframe::M15).unwrap();
        restored.set_timeframe(Timeframe::H4).unwrap();
        let d_after = restored
            .drawings
            .drawings
            .iter()
            .find(|d| d.id == d_id)
            .unwrap();
        let [b0, b1] = d_after.geometry.anchors();
        assert!((b0.bar_index - 14.0).abs() < 1e-9);
        assert!((b1.bar_index - 52.0).abs() < 1e-9);

        // 2. Missing `CustomFile` path falls back cleanly to `SyntheticVnindex` without empty state
        let mut missing = SumiWorkstationApp {
            data_source: DataSourcePreset::CustomFile,
            import_path_input: "/nonexistent/sumi_deleted_r06_fixture.csv".to_owned(),
            ..SumiWorkstationApp::default()
        };
        let mut missing_storage = MemStorage::default();
        eframe::App::save(&mut missing, &mut missing_storage);
        missing = eframe::get_value(&missing_storage, eframe::APP_KEY).unwrap();
        missing.restore_persisted_session();
        assert_eq!(missing.data_source, DataSourcePreset::SyntheticVnindex);
        assert!(!missing.candles.is_empty());
    }

    #[test]
    fn critic_r06_regression_per_instrument_drawing_isolation_in_memory_multisymbol_and_cursor_pill_multiplier()
     {
        let mut app = SumiWorkstationApp::default();
        app.viewport.time_scale.set_width(1_000.0);

        // 1. In-memory multi-symbol CSV switching (`HIGH-03`) + per-instrument `DrawingStore` isolation!
        let multi_csv = concat!(
            "symbol,date,open,high,low,close,volume\n",
            "AAA,2026-10-05,10,12,9,11,1000\n",
            "AAA,2026-10-06,11,13,10,12,1100\n",
            "AAA,2026-10-07,12,14,11,13,1200\n",
            "BBB,2026-10-05,500,520,490,510,5000\n",
            "BBB,2026-10-06,510,530,500,525,5200\n",
            "BBB,2026-10-07,525,540,515,535,5400\n",
        );
        app.load_from_csv_str(multi_csv, &DataLoadOptions::default())
            .unwrap();
        assert_eq!(app.symbol, "AAA");
        assert_eq!(app.available_symbols, vec!["AAA", "BBB"]);

        let aaa_id = app
            .drawings
            .add_drawing(
                DrawingPaneTarget::Main,
                DrawingGeometry::HorizontalLine {
                    anchor: DrawingAnchor::new(1.0, 11.5),
                },
                None,
            )
            .unwrap();
        assert_eq!(app.drawings.drawings.len(), 1);

        // Switch to `BBB` inside the in-memory CSV -> `AAA` drawing is parked, `BBB` starts clean!
        app.select_symbol_in_source("BBB");
        assert_eq!(app.symbol, "BBB");
        assert_eq!(app.drawings.drawings.len(), 0);

        let _ = app.drawings.add_drawing(
            DrawingPaneTarget::Main,
            DrawingGeometry::HorizontalLine {
                anchor: DrawingAnchor::new(0.0, 505.0),
            },
            None,
        );
        let _ = app.drawings.add_drawing(
            DrawingPaneTarget::Main,
            DrawingGeometry::HorizontalLine {
                anchor: DrawingAnchor::new(2.0, 530.0),
            },
            None,
        );
        assert_eq!(app.drawings.drawings.len(), 2);

        // Switch back to `AAA` -> `AAA`'s exact 1 drawing is restored; `BBB`'s 2 drawings stay parked!
        app.select_symbol_in_source("AAA");
        assert_eq!(app.symbol, "AAA");
        assert_eq!(app.drawings.drawings.len(), 1);
        assert_eq!(app.drawings.drawings[0].id, aaa_id);

        // Non-existent symbol sets an explicit error in `last_load_status` (`HIGH-03`)
        app.select_symbol_in_source("NO_SUCH_TICKER");
        assert!(app.last_load_status.as_ref().is_some_and(|(err, _)| *err));

        // 2. Sparse `1W` (`5` bars) historical pan -> `5m` centers week 1 + `populate_showcase_drawings` on <10 bars
        app.select_data_source(DataSourcePreset::IntradayVn30F1m);
        app.indicators.show_rsi = true;
        app.indicators.show_macd = true;
        app.indicators.show_atr = true;
        app.set_timeframe(Timeframe::W1).unwrap();
        assert!(app.candles.len() >= 4 && app.candles.len() < 10);
        app.populate_showcase_drawings();
        assert_eq!(
            app.drawings.drawings.len(),
            8,
            "Demo Set must work cleanly on <10 weekly bars"
        );
        for d in &app.drawings.drawings {
            assert!(d.is_valid(), "all 8 Demo Set drawings on 1W must be valid");
        }

        // Center bar `1` on sparse `1W` (`right_offset > 0` because 5 bars < 500px half-screen)
        let bs = app.viewport.time_scale.bar_spacing();
        let w1_last = (app.candles.len() - 1) as f64;
        let target_w1_ro = 1.0 + (1_000.0 - 1.0 - 500.0) / bs - 0.5 - w1_last;
        app.viewport.time_scale.set_right_offset(target_w1_ro);
        let w1_week1_ts = app.candles[1].timestamp;
        app.set_timeframe(Timeframe::M5).unwrap();
        let m5_center_idx = app
            .viewport
            .time_scale
            .local_x_to_center_float_index(500.0)
            .unwrap();
        let m5_center_ts =
            resolve_anchor_timestamp(m5_center_idx, &app.candles, Timeframe::M5).unwrap();
        assert!(
            m5_center_ts >= w1_week1_ts as f64 && m5_center_ts <= (w1_week1_ts + 12 * 3_600) as f64,
            "switching from centered week 1 on sparse 1W to 5m must land on week 1 Monday open, got {m5_center_ts}"
        );

        // Calling `zoom_in()` invalidates `viewport_focus_lock` (`HIGH-02`)
        assert!(app.viewport_focus_lock.is_some());
        app.zoom_in();
        assert!(app.viewport_focus_lock.is_none());

        // 3. Whitespace cursor pill multiplies bar delta by `Timeframe::unit_multiplier` (`HIGH-04`)
        let pill_15m = crate::chart::format_time_axis_cursor_pill_label(
            &app.candles,
            "15m",
            (app.candles.len() as i64 - 1) + 4,
        );
        assert_eq!(pill_15m, "+60m");
        let pill_4h = crate::chart::format_time_axis_cursor_pill_label(
            &app.candles,
            "4h",
            (app.candles.len() as i64 - 1) + 3,
        );
        assert_eq!(pill_4h, "+12h");
        let pill_5m_behind =
            crate::chart::format_time_axis_cursor_pill_label(&app.candles, "5m", -6);
        assert_eq!(pill_5m_behind, "-30m");
    }

    #[test]
    fn r07_workstation_causal_replay_mtf_zero_lookahead_ticket_sync_rewind_and_instrument_isolation()
     {
        let mut app = SumiWorkstationApp::default();
        app.viewport.time_scale.set_width(1_000.0);
        app.select_data_source(DataSourcePreset::IntradayVn30F1m);
        app.set_timeframe(Timeframe::M15).unwrap();
        assert_eq!(app.base_candles.len(), 5_400);
        assert_eq!(app.candles.len(), 360);

        // 1. Place a pre-replay drawing in the future half (`15m` bar `300`) and one in the past (`bar 120`).
        let past_draw_id = app
            .drawings
            .add_drawing(
                DrawingPaneTarget::Main,
                DrawingGeometry::HorizontalLine {
                    anchor: DrawingAnchor::new(120.0, app.candles[120].close),
                },
                None,
            )
            .unwrap();
        let future_draw_id = app
            .drawings
            .add_drawing(
                DrawingPaneTarget::Main,
                DrawingGeometry::HorizontalLine {
                    anchor: DrawingAnchor::new(300.0, app.candles[300].close),
                },
                None,
            )
            .unwrap();

        // 2. Start replay mid-15m bucket at `base_index = 2_707` (`DEF-001` zero-lookahead check!).
        assert!(app.start_replay_at_base_index(2_707));
        assert!(app.replay.is_active());
        let cutoff_ts = app.replay.cutoff_timestamp(&app.base_candles).unwrap();
        assert_eq!(cutoff_ts, app.base_candles[2_707].timestamp);

        // Verify `self.candles` matches `aggregate_ohlcv(&base_candles[..=2707])` bit-for-bit (`DEF-001`)!
        let expected_15m = crate::data::aggregate_ohlcv(
            &app.base_candles[..=2_707],
            Timeframe::M1,
            Timeframe::M15,
        )
        .unwrap();
        assert_eq!(app.candles, expected_15m);
        assert_eq!(app.indicator_cache.sma.len(), expected_15m.len());

        // Switching `15m -> 1h -> 15m` during replay NEVER leaks `base_candles[2708..]`!
        app.set_timeframe(Timeframe::H1).unwrap();
        let expected_1h =
            crate::data::aggregate_ohlcv(&app.base_candles[..=2_707], Timeframe::M1, Timeframe::H1)
                .unwrap();
        assert_eq!(app.candles, expected_1h);
        app.set_timeframe(Timeframe::M15).unwrap();
        assert_eq!(app.candles, expected_15m);

        // Past drawing (`120`) is causally visible; future drawing (`300`) is causally hidden!
        let vis_cutoff = app.drawings.replay_cutoff_timestamp();
        let past_d = app
            .drawings
            .drawings
            .iter()
            .find(|d| d.id == past_draw_id)
            .unwrap();
        let fut_d = app
            .drawings
            .drawings
            .iter()
            .find(|d| d.id == future_draw_id)
            .unwrap();
        assert!(past_d.is_causally_visible(vis_cutoff, app.candles.len()));
        assert!(!fut_d.is_causally_visible(vis_cutoff, app.candles.len()));

        // 3. Select a Fibonacci drawing, sync Order Ticket, submit Market Buy, step forward, then rewind!
        let mark_0 = app.current_mark_price();
        let fib_id = app
            .drawings
            .add_drawing(
                DrawingPaneTarget::Main,
                DrawingGeometry::FibonacciRetracement {
                    start: DrawingAnchor::new(150.0, mark_0 - 14.0),
                    end: DrawingAnchor::new(175.0, mark_0 + 18.0),
                    direction: FibDirection::StartToEnd,
                    extend_right: true,
                },
                None,
            )
            .unwrap();
        app.drawings.selected_id = Some(fib_id);
        assert!(app.sync_ticket_from_selected_drawing());
        assert!(!app.ticket_price_input.is_empty());
        assert!(!app.ticket_sl_input.is_empty());
        assert!(!app.ticket_tp_input.is_empty());

        let (vis_lo, vis_hi) = crate::model::PriceBounds::from_candles(app.visible_slice(), 0.0)
            .map_or((mark_0 - 5.0, mark_0 + 5.0), |b| (b.raw_min, b.raw_max));
        let vis_span = (vis_hi - vis_lo).max(4.0);
        let sl_test = (mark_0 - vis_span * 0.18)
            .max(vis_lo - vis_span * 0.03)
            .min(mark_0 - vis_span * 0.03);
        let tp_test = (mark_0 + vis_span * 0.18)
            .min(vis_hi + vis_span * 0.04)
            .max(mark_0 + vis_span * 0.03);
        app.ticket_kind = TicketOrderKind::Market;
        app.ticket_qty_input = "10".to_owned();
        app.ticket_sl_input = format!("{sl_test:.2}");
        app.ticket_tp_input = format!("{tp_test:.2}");
        let ord_id = app.submit_ticket_order(OrderSide::Buy).unwrap();
        assert_eq!(ord_id, 1);
        assert!(app.trading.open_position().is_some());

        let snapshot_equity_at_entry = app.trading.equity_at_price(app.current_mark_price());
        assert!(app.step_replay_forward(3));
        assert!(app.replay.base_cursor_index > 2_707);

        // Step backward -> jump back to `2_707` (`start`)!
        assert!(app.jump_replay_to_start());
        assert_eq!(app.replay.base_cursor_index, 2_707);
        assert!(
            (app.trading.equity_at_price(app.current_mark_price()) - snapshot_equity_at_entry)
                .abs()
                < 1e-9,
            "rewinding to start_base_index must restore exact entry-bar equity"
        );

        // 4. Per-instrument `(ReplayController, TradingSession)` isolation (`VN30F1M` <-> `BTCUSDT`)!
        app.select_data_source(DataSourcePreset::BtcUsdt15mJson);
        assert!(!app.replay.is_active());
        assert!(app.trading.open_position().is_none());
        assert_eq!(app.trading.fills().len(), 0);

        // Return to `VN30F1M` -> paused replay at `2_707` and open `LONG 10` position are restored!
        app.select_data_source(DataSourcePreset::IntradayVn30F1m);
        app.set_timeframe(Timeframe::M15).unwrap();
        assert!(app.replay.is_active());
        assert_eq!(app.replay.base_cursor_index, 2_707);
        assert_eq!(app.trading.open_position().unwrap().quantity, 10.0);

        // 5. Headless `egui::Context` full workstation UI pass renders Replay Dock + Paper Trading Panel + overlays!
        app.show_replay_bar = true;
        app.show_trading_panel = true;
        let ctx = egui::Context::default();
        SumiWorkstationApp::configure_dark_workstation_visuals(&ctx);
        for pass in 0..2 {
            let raw = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    Vec2::new(1280.0, 800.0),
                )),
                time: Some(f64::from(pass) * 0.016),
                ..Default::default()
            };
            let mut out = ctx.run_ui(raw, |ui| {
                app.render_workstation_ui(ui);
            });
            out.textures_delta.clear();
        }
        let rep = app
            .last_report
            .expect("headless R07 UI pass must emit ChartRenderReport");
        assert!(
            rep.active_price_line_count >= 3,
            "Entry + SL + TP price lines must render"
        );
        assert!(
            rep.rendered_fill_marker_count >= 1,
            "Market Buy fill arrow must render"
        );
    }

    #[test]
    fn r07_persisted_session_restore_pauses_playing_replay_and_rebuilds_trading_ledger_deterministically()
     {
        #[derive(Default)]
        struct MemStorage(BTreeMap<String, String>);
        impl eframe::Storage for MemStorage {
            fn get_string(&self, key: &str) -> Option<String> {
                self.0.get(key).cloned()
            }
            fn set_string(&mut self, key: &str, value: String) {
                self.0.insert(key.to_owned(), value);
            }
            fn remove_string(&mut self, key: &str) {
                self.0.remove(key);
            }
            fn flush(&mut self) {}
        }

        let mut app = SumiWorkstationApp::default();
        app.viewport.time_scale.set_width(1_000.0);
        app.select_data_source(DataSourcePreset::BtcUsdt15mJson);
        app.set_timeframe(Timeframe::H4).unwrap();
        assert!(app.start_replay_at_base_index(600));
        assert!(app.toggle_replay_play_pause());
        assert!(app.replay.is_playing());

        app.ticket_kind = TicketOrderKind::Market;
        app.ticket_qty_input = "0.40".to_owned();
        let mark = app.current_mark_price();
        app.ticket_sl_input = format!("{:.2}", (mark - 1_500.0).max(100.0));
        app.ticket_tp_input = format!("{:.2}", mark + 2_200.0);
        app.submit_ticket_order(OrderSide::Buy).unwrap();
        assert!(app.step_replay_forward(2));
        let expected_cursor = app.replay.base_cursor_index;
        let expected_candles = app.candles.clone();
        let expected_metrics = app.trading.metrics(app.current_mark_price());

        let mut storage = MemStorage::default();
        eframe::App::save(&mut app, &mut storage);
        let mut restored: SumiWorkstationApp =
            eframe::get_value(&storage, eframe::APP_KEY).unwrap();
        restored.restore_persisted_session();

        assert!(restored.replay.is_active());
        assert_eq!(
            restored.replay.status,
            ReplayStatus::Paused,
            "restored replay session must pause automatically on startup"
        );
        assert_eq!(restored.replay.base_cursor_index, expected_cursor);
        assert_eq!(restored.candles, expected_candles);
        let restored_metrics = restored.trading.metrics(restored.current_mark_price());
        assert!((restored_metrics.equity - expected_metrics.equity).abs() < 1e-9);
        assert_eq!(restored_metrics.fill_count, expected_metrics.fill_count);
    }

    #[test]
    fn critic_r07_regression_conflated_boundary_fill_markers_empty_click_deselect_and_cut_tool_cancel()
     {
        let ctx = egui::Context::default();
        SumiWorkstationApp::configure_dark_workstation_visuals(&ctx);
        let screen_rect = Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            Vec2::new(1280.0, 800.0),
        ));
        ctx.set_pixels_per_point(2.0);

        // 1. `[MEDIUM-02]`: Conflated `50_000`-bar chart (`cf > 1`) with an execution fill at bar `2`
        //    while `culling_range.start > 2` inside the same conflated bucket `[0..cf)` must still render its marker!
        let mut app = SumiWorkstationApp::default();
        app.set_dataset_size(50_000);
        app.trading.reset_at(&app.base_candles, 0);
        app.trading
            .submit_order(
                OrderRequest {
                    side: OrderSide::Buy,
                    kind: OrderKind::Market,
                    quantity: 5.0,
                    stop_loss: None,
                    take_profit: None,
                },
                &app.base_candles,
                2,
            )
            .unwrap();
        app.show_trading_panel = true;

        for _ in 0..2 {
            let mut out0 = ctx.run_ui(
                egui::RawInput {
                    screen_rect,
                    ..Default::default()
                },
                |ui| app.render_workstation_ui(ui),
            );
            out0.textures_delta.clear();
        }

        app.viewport.time_scale.set_bar_spacing(0.03);
        let bs = app.viewport.time_scale.bar_spacing();
        // Place bar `10` near the left edge of `main_plot_rect` so `culling_range.start > 2` while `< cf`!
        let plot_w = app.last_report.unwrap().layout.plot_rect.width() as f64;
        let visible_bars = plot_w / bs;
        let target_right_index = 10.0 + visible_bars;
        let ro = target_right_index - ((app.candles.len() - 1) as f64);
        app.viewport.time_scale.set_right_offset(ro);

        let mut out1 = ctx.run_ui(
            egui::RawInput {
                screen_rect,
                ..Default::default()
            },
            |ui| app.render_workstation_ui(ui),
        );
        out1.textures_delta.clear();
        let rep = app.last_report.expect("conflated report");
        assert!(rep.conflation_factor >= 8);
        assert!(
            rep.rendered_fill_marker_count >= 1,
            "fill at bar 2 inside visible conflated bucket [0..{}) must render without boundary drop",
            rep.conflation_factor
        );

        // 2. `[LOW-02]` & `[LOW-03]`: Entering cut selection and switching to a drawing tool cancels cut mode,
        //    and `start_replay_at_base_index` / `step_replay_backward` clears stale `last_trade_status`.
        app.last_trade_status = Some((true, "Stale fill status".to_owned()));
        assert!(app.start_replay_at_base_index(5_000));
        assert!(app.last_trade_status.is_none());
        app.enter_replay_cut_selection();
        assert!(app.replay.selecting_cut_bar);
        app.cancel_replay_cut_selection();
        assert!(!app.replay.selecting_cut_bar);

        // 3. `[HIGH-02]`: Clicking empty canvas over a hidden future Rectangle during Replay deselects cleanly!
        app.exit_replay();
        app.set_dataset_size(120);
        let vis_id = app
            .drawings
            .add_drawing(
                DrawingPaneTarget::Main,
                crate::drawings::DrawingGeometry::HorizontalLine {
                    anchor: crate::drawings::DrawingAnchor::with_timestamp(
                        10.0,
                        app.candles[10].close,
                        app.candles[10].timestamp as f64,
                    ),
                },
                None,
            )
            .unwrap();
        let _hidden_rect_id = app
            .drawings
            .add_drawing(
                DrawingPaneTarget::Main,
                crate::drawings::DrawingGeometry::Rectangle {
                    corner_a: crate::drawings::DrawingAnchor::with_timestamp(
                        90.0,
                        0.0,
                        app.candles[90].timestamp as f64,
                    ),
                    corner_b: crate::drawings::DrawingAnchor::with_timestamp(
                        115.0,
                        1_000_000.0,
                        app.candles[115].timestamp as f64,
                    ),
                },
                None,
            )
            .unwrap();
        assert!(app.start_replay_at_base_index(50));
        app.drawings.selected_id = Some(vis_id);
        assert!(app.drawings.selected_drawing().is_some());

        // Render once at replay 50, then click right-offset whitespace where the future Rectangle would sit if unmasked:
        let mut r0 = ctx.run_ui(
            egui::RawInput {
                screen_rect,
                ..Default::default()
            },
            |ui| app.render_workstation_ui(ui),
        );
        r0.textures_delta.clear();
        let plot_rect = app.last_report.unwrap().layout.plot_rect;
        let click_pos = egui::Pos2::new(plot_rect.right() - 12.0, plot_rect.top() + 24.0);
        let mut r_press = ctx.run_ui(
            egui::RawInput {
                screen_rect,
                events: vec![
                    egui::Event::PointerMoved(click_pos),
                    egui::Event::PointerButton {
                        pos: click_pos,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                ..Default::default()
            },
            |ui| app.render_workstation_ui(ui),
        );
        r_press.textures_delta.clear();
        let mut r_rel = ctx.run_ui(
            egui::RawInput {
                screen_rect,
                events: vec![egui::Event::PointerButton {
                    pos: click_pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                }],
                ..Default::default()
            },
            |ui| app.render_workstation_ui(ui),
        );
        r_rel.textures_delta.clear();
        assert_eq!(
            app.drawings.selected_id, None,
            "empty canvas click must deselect even when a hidden future Rectangle spans that coordinate"
        );
    }

    #[test]
    fn r08_watchlist_scanner_blind_practice_alerts_and_cmd_k_palette_e2e() {
        let mut app = SumiWorkstationApp::default();
        app.viewport.time_scale.set_width(960.0);

        // 1. Multi-Symbol Watchlist Universe (`13` bundled feeds + active SyntheticVNINDEX = `14` rows initially,
        //    then `13` rows once switched onto `Vn30Basket::FPT`)
        let initial_rows = app.filtered_watchlist_rows();
        assert_eq!(initial_rows.len(), 14);

        app.select_watchlist_instrument(DataSourcePreset::Vn30Basket, "FPT");
        assert_eq!(app.symbol, "FPT");
        assert_eq!(app.data_source, DataSourcePreset::Vn30Basket);
        let basket_rows = app.filtered_watchlist_rows();
        assert_eq!(basket_rows.len(), 13);

        // 2. Historical Signal -> `BlindPractice` Replay launch (`15` bars pre-signal, masked until reached,
        //    and re-masked on causal `step_replay_backward`!)
        let hits = app.filtered_historical_signals();
        assert!(
            !hits.is_empty(),
            "bundled 13-symbol universe must emit historical signals"
        );
        let target_hit = hits
            .iter()
            .find(|h| h.symbol == "FPT" && h.bar_index >= 40)
            .or_else(|| hits.iter().find(|h| h.bar_index >= 40))
            .expect("signal with sufficient pre-signal history")
            .clone();

        assert!(
            app.start_scanner_signal_replay(target_hit.clone(), ScannerReplayIntent::BlindPractice)
        );
        assert!(app.replay.is_active());
        let initial_cutoff = app.replay.cutoff_timestamp(&app.base_candles);
        assert!(initial_cutoff.unwrap() < target_hit.timestamp);
        assert_eq!(
            app.scanner_context
                .as_ref()
                .unwrap()
                .revealed_signal(initial_cutoff),
            None,
            "BlindPractice must hide signal details prior to reveal_at_timestamp"
        );

        // Also verify scanner hits are strictly gated by `DEF-001` cutoff on the active instrument!
        for h in app.filtered_historical_signals() {
            if h.instrument_id == target_hit.instrument_id {
                assert!(
                    h.timestamp <= initial_cutoff.unwrap(),
                    "active-instrument historical scan must never leak future signals past replay cutoff"
                );
            }
        }

        while app.replay.cutoff_timestamp(&app.base_candles).unwrap() < target_hit.timestamp {
            assert!(app.step_replay_forward(1));
        }
        let reveal_cutoff = app.replay.cutoff_timestamp(&app.base_candles);
        assert_eq!(reveal_cutoff, Some(target_hit.timestamp));
        assert!(
            app.scanner_context
                .as_ref()
                .unwrap()
                .revealed_signal(reveal_cutoff)
                .is_some(),
            "signal must reveal once replay reaches signal timestamp"
        );

        assert!(app.step_replay_backward(1));
        let rewound_cutoff = app.replay.cutoff_timestamp(&app.base_candles);
        assert!(
            app.scanner_context
                .as_ref()
                .unwrap()
                .revealed_signal(rewound_cutoff)
                .is_none(),
            "stepping backward before signal bar must immediately re-mask BlindPractice context"
        );
        app.exit_replay();

        // 3. Per-Instrument Alert Isolation (`FPT <-> BTCUSDT <-> FPT`) & Real-Time Drawing Mutation Re-Sync
        app.select_watchlist_instrument(DataSourcePreset::Vn30Basket, "FPT");
        app.alerts.clear_all();
        assert!(app.start_replay_at_base_index(120));
        let c120 = app.base_candles[120].close;
        let line_id = app
            .drawings
            .add_drawing(
                DrawingPaneTarget::Main,
                DrawingGeometry::HorizontalLine {
                    anchor: DrawingAnchor::with_timestamp(
                        115.0,
                        c120 * 1.50,
                        app.base_candles[115].timestamp as f64,
                    ),
                },
                None,
            )
            .unwrap();
        app.drawings.selected_id = Some(line_id);
        let draw_alert_id = app
            .add_alert_on_selected_drawing(CrossDirection::CrossAny)
            .unwrap();
        assert_eq!(app.alerts.rules.len(), 1);
        assert_eq!(app.alerts.triggers.len(), 0);

        assert!(app.step_replay_forward(5));
        assert_eq!(
            app.alerts.triggers.len(),
            0,
            "far-away horizontal line must not fire"
        );

        // Mutate the drawing line between `base_candles[120].close` and `base_candles[125].close` (or across bar 121's range)
        let b121 = &app.base_candles[121];
        let mid_121 = 0.5 * (b121.low + b121.high);
        app.drawings
            .drawings
            .iter_mut()
            .find(|d| d.id == line_id)
            .unwrap()
            .geometry = DrawingGeometry::HorizontalLine {
            anchor: DrawingAnchor::with_timestamp(
                115.0,
                mid_121,
                app.base_candles[115].timestamp as f64,
            ),
        };
        app.sync_alerts_causal();
        assert!(
            app.alerts
                .triggers
                .iter()
                .any(|ev| ev.alert_id == draw_alert_id),
            "dragging/modifying a DrawingLine must immediately re-evaluate causal alert crossings"
        );

        // Switch to `BTCUSDT` -> alerts must start empty, then switch back to `FPT` -> rule & trigger restored!
        app.select_watchlist_instrument(DataSourcePreset::BtcUsdt15mJson, "BTCUSDT");
        assert!(app.alerts.rules.is_empty());
        app.select_watchlist_instrument(DataSourcePreset::Vn30Basket, "FPT");
        assert_eq!(app.alerts.rules.len(), 1);
        assert!(!app.alerts.triggers.is_empty());
        app.exit_replay();
        app.alerts.clear_all();

        // 4. Headless `egui::Context` UI Pass: `Cmd+K` Modal Focus Isolation + Dynamic `alert <px>` Dispatch + On-Chart Line
        let ctx = egui::Context::default();
        SumiWorkstationApp::configure_dark_workstation_visuals(&ctx);
        let screen_rect = Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::Vec2::new(1280.0, 800.0),
        ));
        app.show_scanner_dock = true;
        app.show_alerts_panel = true;
        app.right_dock_tab = RightDockTab::Alerts;

        // Open palette via `Cmd+K` key press while a drawing stays selected (verifying `Esc` closes modal, NOT drawing selection!)
        app.drawings.selected_id = Some(line_id);
        let mut out_open = ctx.run_ui(
            egui::RawInput {
                screen_rect,
                events: vec![egui::Event::Key {
                    key: egui::Key::K,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers {
                        command: true,
                        ..Default::default()
                    },
                }],
                ..Default::default()
            },
            |ui| app.render_workstation_ui(ui),
        );
        out_open.textures_delta.clear();
        assert!(app.command_palette.open);

        // Pressing `Escape` while modal is open must close modal while keeping `selected_id == Some(line_id)`!
        let mut out_esc = ctx.run_ui(
            egui::RawInput {
                screen_rect,
                events: vec![egui::Event::Key {
                    key: egui::Key::Escape,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
                ..Default::default()
            },
            |ui| app.render_workstation_ui(ui),
        );
        out_esc.textures_delta.clear();
        assert!(!app.command_palette.open);
        assert_eq!(
            app.drawings.selected_id,
            Some(line_id),
            "Escape inside Cmd+K modal must not leak through to deselect chart drawings"
        );

        // Re-open palette, type dynamic `alert <mark * 1.01>`, press `Enter`, and verify alert rule + on-chart line render!
        app.open_command_palette();
        let quick_px = ((app.current_mark_price() * 1.01) * 100.0).round() / 100.0;
        app.command_palette.query = format!("alert {quick_px:.2}");
        let mut out_enter = ctx.run_ui(
            egui::RawInput {
                screen_rect,
                events: vec![egui::Event::Key {
                    key: egui::Key::Enter,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
                ..Default::default()
            },
            |ui| app.render_workstation_ui(ui),
        );
        out_enter.textures_delta.clear();
        assert!(!app.command_palette.open);
        assert_eq!(app.alerts.rules.len(), 1);

        let mut out_after = ctx.run_ui(
            egui::RawInput {
                screen_rect,
                ..Default::default()
            },
            |ui| app.render_workstation_ui(ui),
        );
        out_after.textures_delta.clear();
        let rep = app.last_report.expect("chart render report");
        assert!(
            rep.active_alert_line_count >= 1,
            "armed PriceLevel alert must render an on-chart horizontal line & price-axis pill"
        );
    }

    #[test]
    fn critic_r08_regression_utf8_truncation_mtf_blind_practice_and_peer_future_lock() {
        let mut app = SumiWorkstationApp::default();

        // 1. Multi-byte Vietnamese + emoji UTF-8 truncation (`CRITICAL-01`):
        //    `restore_persisted_session` must never slice mid-codepoint or panic!
        let long_vn = "Cảnh báo phá vỡ kháng cự đỉnh cũ 🔥 ".repeat(16);
        app.scanner_filter.search_query = long_vn.clone();
        app.alert_message_input = long_vn.clone();
        app.alert_value_input = long_vn.clone();
        app.ticket_price_input = long_vn.clone();
        app.command_palette.query = long_vn.clone();
        let _ =
            app.add_quick_price_alert(app.current_mark_price() * 1.02, CrossDirection::CrossAbove);
        app.alerts.rules[0].message = long_vn.clone();
        app.restore_persisted_session();
        assert!(app.scanner_filter.search_query.len() <= 64);
        assert!(app.alert_message_input.len() <= 160);
        assert!(app.alerts.rules[0].message.len() <= crate::alerts::MAX_MESSAGE_BYTES);
        app.scanner_filter.search_query.clear();
        app.command_palette.query.clear();

        // 2. Multi-symbol Custom CSV (`AAA` + `BBB`) Watchlist, `Cmd+K` switching, and bogus ID guard (`MEDIUM-02`)
        let mut csv_lines = vec!["symbol,timestamp,open,high,low,close,volume".to_owned()];
        let base_ts = 1_750_000_000_i64;
        for i in 0..45_i64 {
            let ts = base_ts + i * 86_400;
            let px_a = 100.0 + (i as f64) * 1.2 + if i % 4 == 0 { -2.5 } else { 0.8 };
            let px_b = 200.0 + (i as f64) * 0.9 + if i % 5 == 0 { -3.0 } else { 1.1 };
            csv_lines.push(format!(
                "AAA,{ts},{:.2},{:.2},{:.2},{:.2},12000",
                px_a - 0.5,
                px_a + 1.8,
                px_a - 1.6,
                px_a
            ));
            csv_lines.push(format!(
                "BBB,{ts},{:.2},{:.2},{:.2},{:.2},18000",
                px_b - 0.6,
                px_b + 2.0,
                px_b - 1.9,
                px_b
            ));
        }
        app.load_from_csv_str(&csv_lines.join("\n"), &DataLoadOptions::default())
            .expect("multi-symbol CSV must load");
        let custom_rows = app.filtered_watchlist_rows();
        assert!(custom_rows.iter().any(|r| r.symbol == "AAA"));
        assert!(custom_rows.iter().any(|r| r.symbol == "BBB"));
        app.command_palette.query = "BBB".to_owned();
        let cands = app.build_command_palette_candidates();
        assert!(
            cands
                .iter()
                .any(|c| matches!(&c.action, PaletteAction::SelectWatchlistSymbol { preset: DataSourcePreset::CustomFile, symbol } if symbol == "BBB")),
            "Cmd+K palette must list peer symbol BBB from multi-symbol CustomFile"
        );
        app.command_palette.query.clear();

        let mut bogus_hit = app.filtered_historical_signals()[0].clone();
        bogus_hit.instrument_id = "Bogus:NonExistent:XYZ".to_owned();
        bogus_hit.symbol = "XYZ".to_owned();
        assert!(
            !app.start_scanner_signal_replay(bogus_hit, ScannerReplayIntent::SignalReview),
            "unknown instrument_id must return false instead of silently replaying the active symbol"
        );

        // 3. Cross-instrument causal Replay lock (`CRITICAL-02` Part A):
        //    Replaying `FPT` (`Vn30Basket`, Sep 2026) must withhold future Oct 2026 `BTCUSDT`!
        app.select_watchlist_instrument(DataSourcePreset::Vn30Basket, "FPT");
        assert!(app.start_replay_at_base_index(120));
        let fpt_cutoff = app.replay.cutoff_timestamp(&app.base_candles).unwrap();
        let replay_rows = app.filtered_watchlist_rows();
        assert!(
            !replay_rows.iter().any(|r| r.symbol == "BTCUSDT"),
            "future-only peer feed (Oct 2026 BTCUSDT) must not leak into Watchlist during Sep 2026 replay"
        );
        for h in app.filtered_historical_signals() {
            assert!(
                h.timestamp <= fpt_cutoff,
                "every peer historical signal during Replay must obey <= replay_cutoff"
            );
        }
        app.exit_replay();

        // 4. Higher-TF `15m` `BlindPractice` on `1m` `VN30F1M` (`CRITICAL-02` Part B):
        //    Signal must stay masked across `09:00..09:13` (`BaseBar` ticks) and only reveal at `15m` bucket close!
        app.select_watchlist_instrument(DataSourcePreset::IntradayVn30F1m, "VN30F1M");
        app.set_timeframe(Timeframe::M15).unwrap();
        let hit_15m = app
            .filtered_historical_signals()
            .into_iter()
            .find(|h| h.symbol == "VN30F1M" && h.timeframe == Timeframe::M15 && h.bar_index >= 25)
            .expect("15m VN30F1M historical signal");

        assert!(
            app.start_scanner_signal_replay(hit_15m.clone(), ScannerReplayIntent::BlindPractice)
        );
        let reveal_ts = app.scanner_context.as_ref().unwrap().reveal_at_timestamp();
        assert!(
            reveal_ts > hit_15m.timestamp,
            "15m signal on 1m base feed must set reveal_at_timestamp to bucket close (> bucket open)"
        );

        app.replay.step_unit = ReplayStepUnit::BaseBar;
        while app.replay.cutoff_timestamp(&app.base_candles).unwrap() < hit_15m.timestamp {
            assert!(app.step_replay_forward(1));
        }
        let open_cutoff = app.replay.cutoff_timestamp(&app.base_candles).unwrap();
        assert_eq!(open_cutoff, hit_15m.timestamp);
        assert!(
            app.scanner_context
                .as_ref()
                .unwrap()
                .revealed_signal(Some(open_cutoff))
                .is_none(),
            "BlindPractice must remain masked on the first 1m tick of the 15m signal bucket"
        );
        assert!(
            !app.filtered_historical_signals().iter().any(|h| {
                h.instrument_id == hit_15m.instrument_id && h.timestamp == hit_15m.timestamp
            }),
            "unclosed active 15m tail bucket must withhold intrabar scanner signals until bucket close"
        );

        while app.replay.cutoff_timestamp(&app.base_candles).unwrap() < reveal_ts {
            assert!(app.step_replay_forward(1));
        }
        let close_cutoff = app.replay.cutoff_timestamp(&app.base_candles).unwrap();
        assert_eq!(close_cutoff, reveal_ts);
        assert!(
            app.scanner_context
                .as_ref()
                .unwrap()
                .revealed_signal(Some(close_cutoff))
                .is_some(),
            "BlindPractice must reveal cleanly once the 15m bucket closes"
        );
    }

    #[test]
    fn critic_r08_regression_cmd_k_modal_tab_delete_gesture_and_scroll_isolation() {
        let mut app = SumiWorkstationApp::default();
        let ctx = egui::Context::default();
        SumiWorkstationApp::configure_dark_workstation_visuals(&ctx);
        let screen_rect = Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::Vec2::new(1280.0, 800.0),
        ));

        // Add an unlocked selected drawing so `Delete`/`Backspace` would delete it if leaked.
        let keep_id = app
            .drawings
            .add_drawing(
                DrawingPaneTarget::Main,
                DrawingGeometry::HorizontalLine {
                    anchor: DrawingAnchor::new(80.0, app.current_mark_price()),
                },
                None,
            )
            .unwrap();
        app.drawings.selected_id = Some(keep_id);

        // Warm up layout once, then initiate a 2-anchor TrendLine gesture + kinetic scroll before `Cmd+K`.
        let mut warm = ctx.run_ui(
            egui::RawInput {
                screen_rect,
                ..Default::default()
            },
            |ui| app.render_workstation_ui(ui),
        );
        warm.textures_delta.clear();

        app.drawings.active_tool = DrawingTool::TrendLine;
        let click_pos = egui::Pos2::new(620.0, 320.0);
        let mut start_draw = ctx.run_ui(
            egui::RawInput {
                screen_rect,
                events: vec![
                    egui::Event::PointerMoved(click_pos),
                    egui::Event::PointerButton {
                        pos: click_pos,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::NONE,
                    },
                    egui::Event::PointerButton {
                        pos: click_pos,
                        button: egui::PointerButton::Primary,
                        pressed: false,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                ..Default::default()
            },
            |ui| app.render_workstation_ui(ui),
        );
        start_draw.textures_delta.clear();
        assert!(
            !matches!(
                app.drawings.phase(),
                crate::drawings::DrawingInteractionPhase::Idle
            ),
            "1st TrendLine click must arm PlacingTwoAnchor before Cmd+K"
        );
        app.viewport.kinetic = Some(crate::viewport::KineticAnimation::new(
            0.01, 5.0, 0.997, 1.0, 50.0, 0.01,
        ));

        // Open `Cmd+K` -> must cancel active drawing placement AND kinetic scroll immediately (`HIGH-04`).
        app.open_command_palette();
        app.command_palette.query.clear();
        app.drawings.selected_id = Some(keep_id);
        assert!(matches!(
            app.drawings.phase(),
            crate::drawings::DrawingInteractionPhase::Idle
        ));
        assert!(app.viewport.kinetic.is_none());

        let ro_before = app.viewport.time_scale.right_offset();
        let bs_before = app.viewport.time_scale.bar_spacing();

        // While `Cmd+K` is open with an EMPTY query, fire `Tab`, `Shift+Tab`, `Delete`, `Backspace`,
        // and outside-modal `MouseWheel` scroll -> none may leak to background panels, drawings, or chart!
        let mut modal_turn = ctx.run_ui(
            egui::RawInput {
                screen_rect,
                events: vec![
                    egui::Event::PointerMoved(egui::Pos2::new(120.0, 260.0)),
                    egui::Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Point,
                        delta: egui::Vec2::new(48.0, -36.0),
                        phase: egui::TouchPhase::Move,
                        modifiers: egui::Modifiers::NONE,
                    },
                    egui::Event::Key {
                        key: egui::Key::Tab,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers::NONE,
                    },
                    egui::Event::Key {
                        key: egui::Key::Tab,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers {
                            shift: true,
                            ..Default::default()
                        },
                    },
                    egui::Event::Key {
                        key: egui::Key::Delete,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers::NONE,
                    },
                    egui::Event::Key {
                        key: egui::Key::Backspace,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                ..Default::default()
            },
            |ui| app.render_workstation_ui(ui),
        );
        modal_turn.textures_delta.clear();

        assert!(app.command_palette.open);
        assert_eq!(
            app.drawings.drawings.len(),
            1,
            "Delete/Backspace while Cmd+K is open with empty query must never delete selected chart drawings"
        );
        assert_eq!(app.drawings.selected_id, Some(keep_id));
        assert!(
            (app.viewport.time_scale.right_offset() - ro_before).abs() < 1e-9
                && (app.viewport.time_scale.bar_spacing() - bs_before).abs() < 1e-9,
            "outside-modal scroll wheel must not pan or zoom the chart while Cmd+K is open"
        );
    }

    #[test]
    fn strategy_lab_causal_sync_across_bar_replay_and_sweep_invalidation() {
        let mut app = SumiWorkstationApp::default();
        app.select_watchlist_instrument(DataSourcePreset::Vn30Basket, "FPT");
        app.open_strategy_lab();
        app.strategy_lab_tab = StrategyLabSubTab::Tearsheet;
        app.run_active_parameter_sweep();
        app.run_universe_strategy_battle();

        let full_bars = app.candles.len();
        assert!(full_bars >= 120);
        assert_eq!(
            app.backtest_result.as_ref().expect("full bt").bar_count,
            full_bars
        );
        assert!(app.sweep_report.is_some());
        assert!(app.battle_report.is_some());

        // Cut causal Bar Replay at bar `90` (`DEF-001`) -> `sync_backtest_causal` MUST slice `backtest_result`
        // to `91` bars AND invalidate stale full-history `sweep_report` & `battle_report`!
        assert!(app.start_replay_at_base_index(90));
        app.sync_backtest_causal();
        let cut_bt = app.backtest_result.as_ref().expect("causal replay bt");
        assert_eq!(cut_bt.bar_count, 91);
        assert_eq!(cut_bt.last_timestamp, Some(app.base_candles[90].timestamp));
        let prev_sweep_ts = cut_bt.last_timestamp;
        assert!(
            app.sweep_report.is_none() && app.battle_report.is_none(),
            "stale full-history sweep/battle reports must be invalidated when entering Bar Replay"
        );

        // Switching to `Sweep` tab during Replay computes a strictly causal `91`-bar sweep!
        app.strategy_lab_tab = StrategyLabSubTab::Sweep;
        app.sync_backtest_causal();
        assert!(app.sweep_report.is_some());

        // Stepping Replay forward by 5 bars updates `backtest_result` (`96` bars) and refreshes `Sweep`!
        assert!(app.step_replay_forward(5));
        app.sync_backtest_causal();
        let stepped_bt = app.backtest_result.as_ref().expect("stepped bt");
        assert_eq!(stepped_bt.bar_count, 96);
        assert!(stepped_bt.last_timestamp > prev_sweep_ts);
    }

    #[test]
    fn command_palette_strategy_lab_actions_trade_focus_and_trade_replay() {
        let mut app = SumiWorkstationApp::default();
        app.select_watchlist_instrument(DataSourcePreset::BtcUsdt15mJson, "BTCUSDT");
        let _ = app.set_timeframe(Timeframe::H1);

        // 1. `Cmd+K` query `"backtest"` surfaces Strategy Lab actions cleanly!
        app.open_command_palette();
        app.command_palette.query = "backtest sweep".to_owned();
        let cands = app.build_command_palette_candidates();
        assert!(
            cands
                .iter()
                .any(|c| matches!(c.action, PaletteAction::RunParameterSweep)),
            "Cmd+K query 'backtest sweep' must rank RunParameterSweep"
        );
        app.execute_palette_action(PaletteAction::SelectStrategyPreset(
            StrategyPresetKind::MacdMomentum,
        ));
        assert!(app.show_strategy_panel);
        assert!(app.show_equity_pane);
        assert!(app.show_backtest_trades);
        assert_eq!(app.backtest_config.preset, StrategyPresetKind::MacdMomentum);

        // 2. Focus a closed trade -> centers `TimeScale` around the trade & sets `focused_backtest_trade_id`!
        let first_trade = app
            .backtest_result
            .as_ref()
            .and_then(|r| r.trades.first().cloned())
            .expect("at least one trade on BTCUSDT 1h MacdMomentum");
        assert!(app.focus_backtest_trade(first_trade.trade_id));
        assert_eq!(app.focused_backtest_trade_id, Some(first_trade.trade_id));

        // 3. Replay a closed trade -> starts causal Bar Replay 5 bars prior to `entry_bar_index`!
        assert!(app.replay_backtest_trade(first_trade.trade_id));
        assert!(app.replay.is_active());
        assert!(app.candles.len() <= first_trade.entry_bar_index + 1);
        assert_eq!(
            app.backtest_result
                .as_ref()
                .expect("synced replay bt")
                .bar_count,
            app.candles.len(),
            "MAJOR-03: replay_backtest_trade must synchronize backtest_result immediately without manual sync call"
        );
    }

    #[test]
    fn critic_r09_regression_auto_sync_on_replay_transitions_and_headless_trade_replay() {
        let mut app = SumiWorkstationApp::default();
        app.select_watchlist_instrument(DataSourcePreset::IntradayVn30F1m, "VN30F1M");
        let _ = app.set_timeframe(Timeframe::M15);
        app.open_strategy_lab();

        let full_active_len = app.candles.len();
        assert_eq!(
            app.backtest_result.as_ref().expect("full bt").bar_count,
            full_active_len
        );

        // 1. Cut Replay on an interior `1m` sub-bar (`idx = 152`, `2` minutes into a `15m` bucket)
        // WITHOUT calling `sync_backtest_causal()` manually!
        assert!(app.start_replay_at_base_index(152));
        assert!(
            app.is_active_tail_incomplete(),
            "MAJOR-04: interior 1m sub-bar inside 15m bucket must report incomplete_tail_bar = true"
        );
        assert_eq!(
            app.backtest_result
                .as_ref()
                .expect("auto-synced cut bt")
                .bar_count,
            app.candles.len()
        );

        // 2. Step forward & backward WITHOUT manual `sync_backtest_causal()` -> always stays 1:1 synced!
        assert!(app.step_replay_forward(3));
        assert_eq!(
            app.backtest_result
                .as_ref()
                .expect("stepped fwd bt")
                .bar_count,
            app.candles.len()
        );
        assert!(app.step_replay_backward(1));
        assert_eq!(
            app.backtest_result
                .as_ref()
                .expect("stepped back bt")
                .bar_count,
            app.candles.len()
        );

        // 3. Exit replay -> immediately restores full causal backtest!
        app.exit_replay();
        assert_eq!(
            app.backtest_result.as_ref().expect("restored bt").bar_count,
            full_active_len
        );

        // 4. Turn ALL 3 Strategy UI flags off (`false`) and call `replay_backtest_trade(tid)` headlessly!
        let sample_tid = app
            .backtest_result
            .as_ref()
            .and_then(|r| r.trades.last().map(|t| t.trade_id))
            .expect("at least one trade on VN30F1M 15m");
        app.show_strategy_panel = false;
        app.show_equity_pane = false;
        app.show_backtest_trades = false;
        app.backtest_result = None;
        app.backtest_synced_key = None;

        assert!(
            app.replay_backtest_trade(sample_tid),
            "MAJOR-03: headless replay_backtest_trade must succeed even when UI flags started false"
        );
        assert!(app.show_backtest_trades);
        assert!(app.replay.is_active());
        assert_eq!(
            app.backtest_result
                .as_ref()
                .expect("headless trade replay bt")
                .bar_count,
            app.candles.len()
        );
    }

    #[test]
    fn r10_workstation_risk_planner_rr_box_journal_walk_forward_and_portfolio_e2e() {
        let mut app = SumiWorkstationApp::default();
        app.select_watchlist_instrument(DataSourcePreset::Vn30Basket, "FPT");
        assert_eq!(app.active_exchange_venue(), ExchangeVenue::Hose);
        assert!(app.active_daily_price_band().is_some());

        // 1. Configure HOSE Risk Planner -> Snap Ticks -> Draw `RiskRewardBox` -> Sync Order Ticket
        let mark = app.current_mark_price();
        app.risk_plan_input.account_equity = 250_000_000.0;
        app.risk_plan_input.available_cash = 250_000_000.0;
        app.risk_plan_input.risk_pct = 1.0;
        app.risk_plan_input.side = PositionSide::Long;
        app.risk_plan_input.entry_price = mark * 0.99;
        app.risk_plan_input.stop_loss = mark * 0.95;
        app.risk_plan_input.target_price = mark * 1.08;

        assert!(app.snap_planner_ticks_and_sync_rr_box());
        let rr_id = app
            .draw_or_update_rr_box_from_planner()
            .expect("create RiskRewardBox from planner");
        assert_eq!(app.drawings.selected_id, Some(rr_id));

        // Flip L/S on the selected `RiskRewardBox` -> `auto_sync_planner_if_rr_box_changed` flips Planner to `Short`!
        assert!(app.drawings.reverse_selected_risk_reward());
        app.auto_sync_planner_if_rr_box_changed();
        assert_eq!(app.risk_plan_input.side, PositionSide::Short);

        // Flip back to `Long` and push to Paper Trading Order Ticket (`Lot 100` multiple!)
        assert!(app.drawings.reverse_selected_risk_reward());
        app.auto_sync_planner_if_rr_box_changed();
        assert_eq!(app.risk_plan_input.side, PositionSide::Long);
        assert!(app.sync_ticket_from_risk_planner());
        let parsed_qty: f64 = app.ticket_qty_input.parse().expect("valid ticket qty");
        assert!(
            parsed_qty >= 100.0 && (parsed_qty % 100.0).abs() < 1.0e-6,
            "HOSE ticket quantity must be floored to 100-share board lot, got {parsed_qty}"
        );

        // 2. Log active plan + import causal backtest trades + verify `DEF-001` causal replay masking on Journal & CSV/MD
        let logged_id = app.log_active_plan_to_journal().expect("logged plan");
        assert!(logged_id >= 1);
        app.backtest_config.preset = StrategyPresetKind::MacdMomentum;
        app.backtest_config.params.macd_fast = 6;
        app.backtest_config.params.macd_slow = 16;
        app.backtest_config.params.macd_signal = 5;
        app.backtest_config.lot_step = 100.0;
        let imported_bt = app.import_backtest_trades_to_journal(10);
        assert!(imported_bt >= 1);
        let full_visible = app.journal.visible_entries(None).len();

        let cut_idx = app.base_candles.len() / 2;
        assert!(app.start_replay_at_base_index(cut_idx));
        let cut_cutoff = app.replay_causal_close_timestamp();
        let replay_visible = app.journal.visible_entries(cut_cutoff).len();
        assert!(
            replay_visible < full_visible,
            "DEF-001: future journal entries must be causally masked at 50% replay cutoff ({replay_visible} vs {full_visible})"
        );
        let csv_doc = app.export_journal_csv_causal();
        assert_eq!(csv_doc.lines().count(), replay_visible + 1);
        let md_doc = app.export_journal_markdown_causal();
        assert!(md_doc.contains("# Sumi Structured Trade Journal & Discipline Report"));
        app.exit_replay();

        // 3. `Cmd+K` Command Palette actions (`RunWalkForwardValidation` + `RunPortfolioBacktest`) + Headless UI Frames!
        app.open_command_palette();
        app.command_palette.query = "walk forward".to_owned();
        let cands = app.build_command_palette_candidates();
        assert!(
            cands
                .iter()
                .any(|c| matches!(c.action, PaletteAction::RunWalkForwardValidation))
        );
        app.execute_palette_action(PaletteAction::RunWalkForwardValidation);
        assert!(app.walk_forward_report.is_some());
        assert_eq!(app.equity_pane_source, EquityPaneSource::WalkForwardOos);

        app.execute_palette_action(PaletteAction::RunPortfolioBacktest);
        assert!(app.portfolio_report.is_some());
        assert_eq!(
            app.portfolio_report
                .as_ref()
                .expect("port report")
                .constituents
                .len(),
            8
        );
        assert_eq!(
            app.equity_pane_source,
            EquityPaneSource::MultiAssetPortfolio
        );

        let ctx = egui::Context::default();
        SumiWorkstationApp::configure_dark_workstation_visuals(&ctx);
        let screen_rect = Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            Vec2::new(1280.0, 800.0),
        ));
        for dock_tab in [RightDockTab::StrategyLab, RightDockTab::Journal] {
            app.right_dock_tab = dock_tab;
            if dock_tab == RightDockTab::Journal {
                app.open_journal_tab();
            }
            let mut full_output = ctx.run_ui(
                egui::RawInput {
                    screen_rect,
                    ..Default::default()
                },
                |ui| app.render_workstation_ui(ui),
            );
            full_output.textures_delta.clear();
            let rep = app.last_report.expect("chart render report");
            assert!(rep.equity_pane_active);
            assert!(rep.drawing_count >= 1);
        }
    }

    #[test]
    fn critic_r10_regression_zero_lot_ticket_guard_reseed_venue_reset_and_1w_ref_price() {
        let mut app = SumiWorkstationApp::default();
        app.select_watchlist_instrument(DataSourcePreset::Vn30Basket, "FPT");

        // 1. M-09: `FPT` reference price on `1W` must equal yesterday's `D1` close (never last week's `1W` close!)
        let d1_ref = app.causal_reference_price().expect("D1 ref price");
        app.set_timeframe(Timeframe::W1).expect("W1 rollup");
        let w1_ref = app.causal_reference_price().expect("W1 ref price");
        assert!(
            (d1_ref - w1_ref).abs() < 1e-9,
            "M-09: switching to 1W must preserve D1 statutory reference price ({d1_ref} vs {w1_ref})"
        );
        app.set_timeframe(Timeframe::D1).expect("back to D1");

        // 2. C-04: Unaffordable `0`-lot plan MUST NOT populate Order Ticket with `100` shares!
        app.ticket_qty_input = "777".to_owned();
        app.risk_plan_input.account_equity = 500.0;
        app.risk_plan_input.available_cash = 500.0;
        app.risk_plan_input.risk_pct = 0.5;
        let zero_plan = app.compute_active_trade_plan().expect("valid 0-lot plan");
        assert_eq!(zero_plan.planned_quantity, 0.0);
        assert!(
            !app.sync_ticket_from_risk_planner(),
            "C-04: 0-lot plan must reject ticket sync instead of substituting 100-share board lot"
        );
        assert_eq!(app.ticket_qty_input, "777");

        // 3. M-08: `Reseed` updates selected `RiskRewardBox`, and switching tickers resets `risk_venue_override`!
        app.risk_plan_input.account_equity = 250_000_000.0;
        app.risk_plan_input.available_cash = 250_000_000.0;
        app.reseed_risk_planner_from_live_mark();
        let rr_id = app
            .draw_or_update_rr_box_from_planner()
            .expect("drawn R:R box");
        app.risk_plan_input.entry_price *= 0.85;
        app.reseed_risk_planner_from_live_mark();
        let (_, box_entry, _, _, _) = app
            .drawings
            .drawings
            .iter()
            .find(|d| d.id == rr_id)
            .unwrap()
            .geometry
            .risk_reward_metrics()
            .unwrap();
        assert!(
            (box_entry - app.current_mark_price()).abs() < 1.0,
            "M-08: Reseed must update selected RiskRewardBox to fresh live mark"
        );

        app.risk_venue_override = Some(ExchangeVenue::UsEquities);
        assert_eq!(app.active_exchange_venue(), ExchangeVenue::UsEquities);
        app.select_watchlist_instrument(DataSourcePreset::IntradayVn30F1m, "VN30F1M");
        assert_eq!(
            app.risk_venue_override, None,
            "M-08: switching instruments must reset risk_venue_override"
        );
        assert_eq!(app.active_exchange_venue(), ExchangeVenue::Vn30Futures);
    }

    #[test]
    fn r11_workstation_vrvp_frvp_vwap_avwap_and_rs_vnindex_e2e() {
        let mut app = SumiWorkstationApp::default();
        app.select_watchlist_instrument(DataSourcePreset::Vn30Basket, "FPT");
        app.set_timeframe(Timeframe::D1).expect("D1 timeframe");

        app.indicators.show_vwap = true;
        app.indicators.show_vrvp = true;
        app.indicators.vrvp_rows = 48;
        app.indicators.show_rs = true;
        app.indicators.show_rsi = false;
        app.indicators.show_macd = false;
        app.indicators.show_atr = false;
        app.execute_palette_action(PaletteAction::ToggleCompareOverlay);
        assert!(app.show_compare_overlay);
        assert!(!app.compare_series_cached.is_empty());

        let n = app.candles.len();
        assert!(n >= 40);
        let frvp_id = app
            .add_frvp_drawing((n * 40) / 100, (n * 65) / 100, 32)
            .expect("FRVP drawing added");
        let _avwap_id = app
            .add_anchored_vwap_drawing((n * 45) / 100, true)
            .expect("AVWAP drawing added");
        assert!(
            app.drawings.drawings.iter().any(|d| d.id == frvp_id),
            "FRVP must be stored in DrawingStore"
        );

        app.viewport.time_scale.set_width(1190.0);
        app.viewport.time_scale.set_bar_spacing(8.5);
        app.viewport.time_scale.set_right_offset(14.0);
        app.sync_indicators_and_benchmarks();

        let ctx = egui::Context::default();
        SumiWorkstationApp::configure_dark_workstation_visuals(&ctx);
        let screen_rect = Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            Vec2::new(1280.0, 800.0),
        ));
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect,
                ..Default::default()
            },
            |ui| app.render_workstation_ui(ui),
        );
        out.textures_delta.clear();

        let rep = app.last_report.expect("R11 chart render report");
        assert!(
            rep.vrvp_poc_price.is_some(),
            "VRVP POC must be computed and rendered"
        );
        assert!(
            rep.active_frvp_count >= 1,
            "Active FRVP count must reflect drawn FRVP"
        );
        assert!(
            rep.pane_count >= 2,
            "Relative Strength sub-pane must produce >= 2 panes"
        );
        assert_eq!(app.indicator_cache.vwap.vwap.len(), app.candles.len());
        assert_eq!(app.indicator_cache.rs.rs_line.len(), app.candles.len());

        // Cycle compare benchmark -> switches from VNINDEX -> next benchmark in cycle cleanly.
        let prev_bench = app.active_benchmark_label.clone();
        app.execute_palette_action(PaletteAction::CycleCompareBenchmark);
        assert_ne!(app.active_benchmark_label, prev_bench);

        // DEF-001 Causal Bar Replay: future AVWAP created at future_idx must be masked on rewind,
        // and RS / Compare / VWAP vectors must truncate cleanly to the replay candle count.
        let n_base = app.base_candles.len();
        let replay_cut = (n_base * 65) / 100;
        let future_idx = (n_base * 88) / 100;
        assert!(app.start_replay_at_base_index(future_idx));
        let future_avwap_id = app
            .add_anchored_vwap_drawing(app.candles.len().saturating_sub(2), true)
            .expect("future AVWAP");
        if let Some(tr) = app.replay.jump_to_base_index(&app.base_candles, replay_cut) {
            app.apply_replay_transition(tr);
        }
        app.sync_indicators_and_benchmarks();

        let vis_cutoff = app.drawings.replay_cutoff_timestamp();
        let vis_len = app.candles.len();
        let future_d = app
            .drawings
            .drawings
            .iter()
            .find(|d| d.id == future_avwap_id)
            .expect("future AVWAP exists in store");
        assert!(
            !future_d.is_causally_visible(vis_cutoff, vis_len),
            "DEF-001: future AVWAP created after replay cutoff must be causally hidden"
        );
        assert_eq!(app.indicator_cache.rs.rs_line.len(), vis_len);
        assert_eq!(app.indicator_cache.vwap.vwap.len(), vis_len);
        assert_eq!(app.compare_series_cached.len(), vis_len);

        let mut replay_out = ctx.run_ui(
            egui::RawInput {
                screen_rect,
                ..Default::default()
            },
            |ui| app.render_workstation_ui(ui),
        );
        replay_out.textures_delta.clear();
        let replay_rep = app.last_report.expect("replay report");
        assert_eq!(app.viewport.time_scale.total_bars(), vis_len);
        assert!(replay_rep.culled_bar_count <= vis_len);

        // Switch to intraday VN30F1M 15m and verify UTC+7 daily Session VWAP resets.
        app.exit_replay();
        app.select_watchlist_instrument(DataSourcePreset::IntradayVn30F1m, "VN30F1M");
        app.set_timeframe(Timeframe::M15).expect("15m timeframe");
        app.sync_indicators_and_benchmarks();
        assert!(
            app.indicator_cache.vwap.session_starts.len() > 1,
            "Intraday 15m VN30F1M must split across multiple UTC+7 trading sessions"
        );
    }

    #[test]
    fn critic_r11_regression_benchmark_cache_invalidation_and_compare_pan_o1() {
        // M3 (a): Horizontal panning across compare overlay must NEVER invalidate or rebuild `compare_series_cached`
        // (live left-edge rebasing runs O(1) in renderer via `compute_compare_viewport_scale`).
        let mut app = SumiWorkstationApp::default();
        app.select_watchlist_instrument(DataSourcePreset::Vn30Basket, "FPT");
        app.show_compare_overlay = true;
        app.set_compare_benchmark(Some("VNINDEX".to_owned()));
        app.viewport.time_scale.set_width(1000.0);
        app.viewport.time_scale.set_bar_spacing(10.0);
        app.sync_indicators_and_benchmarks();

        let key_initial = app.compare_cache_key.clone().expect("compare key built");
        let ptr_initial = app.compare_series_cached.as_ptr();
        let len_initial = app.compare_series_cached.len();
        assert!(len_initial > 20);

        for offset in [-40.0_f64, -80.0, 15.0] {
            app.viewport.time_scale.scroll_by_bars(offset);
            app.sync_indicators_and_benchmarks();
            assert_eq!(app.compare_cache_key.as_ref(), Some(&key_initial));
            assert_eq!(app.compare_series_cached.as_ptr(), ptr_initial);
        }

        // M3 (b): Loading a new report via CSV must clear and rebuild `resolved_benchmark_cache` and
        // `compare_cache_key` for the new symbol/window (never reusing stale FPT benchmark state).
        let prev_bench_meta = app
            .resolved_benchmark_cache
            .as_ref()
            .map(|c| (c.0.clone(), c.1, c.2, c.3));
        let mut csv = String::from("time,open,high,low,close,volume\n");
        let t_start = 1_735_689_600_i64; // 2025-01-01
        for i in 0..40 {
            let p = 50.0 + (i as f64) * 0.4;
            let _ = std::fmt::Write::write_fmt(
                &mut csv,
                format_args!(
                    "{},{p:.2},{:.2},{:.2},{p:.2},12000\n",
                    t_start + (i as i64) * 86_400,
                    p + 0.8,
                    p - 0.6
                ),
            );
        }
        app.load_from_csv_str(&csv, &DataLoadOptions::default())
            .expect("load custom benchmark-compatible csv");
        let after_bench_meta = app
            .resolved_benchmark_cache
            .as_ref()
            .map(|c| (c.0.clone(), c.1, c.2, c.3));
        assert_ne!(after_bench_meta, prev_bench_meta);
        assert_ne!(app.compare_cache_key.as_ref(), Some(&key_initial));
        assert_eq!(app.compare_series_cached.len(), app.candles.len());
    }
}
