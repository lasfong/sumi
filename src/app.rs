use std::collections::BTreeMap;

use egui::{Button, Color32, CornerRadius, Frame, Margin, RichText, Stroke, Vec2, Visuals};
use serde::{Deserialize, Serialize};

use crate::chart::{
    ChartDisplayConfig, ChartRenderReport, ChartReplayTradingOverlay, ChartViewportState, palette,
    render_workstation_chart_full,
};
use crate::data::{
    AggregationError, DataFileFormat, DataLoadError, DataLoadOptions, DataLoadReport,
    DataSourcePreset, Timeframe, generate_synthetic_ohlcv, load_ohlcv_bytes, load_ohlcv_csv,
    load_ohlcv_json, load_preset_dataset, remap_timestamp_to_bar_index, resolve_anchor_timestamp,
};
use crate::drawings::{
    DrawingAnchor, DrawingColorPreset, DrawingGeometry, DrawingPaneTarget, DrawingStore,
    DrawingStyle, DrawingTool, FibDirection, LineDashStyle, MagnetMode,
};
use crate::indicators::OscillatorPaneId;
use crate::indicators::{IndicatorCache, IndicatorConfig};
use crate::model::Candle;
use crate::replay::{
    ReplayController, ReplaySpeed, ReplayStatus, ReplayStepUnit, ReplayTransition,
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

/// Primary application state for the Sumi Rust Trading Workstation (`Milestone R07`).
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
    pub drawings: DrawingStore,
    #[serde(default)]
    drawings_by_instrument: BTreeMap<String, DrawingStore>,
    pub replay: ReplayController,
    pub trading: TradingSession,
    #[serde(default)]
    replay_trading_by_instrument: BTreeMap<String, (ReplayController, TradingSession)>,
    pub show_replay_bar: bool,
    pub show_trading_panel: bool,
    pub ticket_kind: TicketOrderKind,
    pub ticket_qty_input: String,
    pub ticket_price_input: String,
    pub ticket_sl_input: String,
    pub ticket_tp_input: String,

    #[serde(skip)]
    pub base_candles: Vec<Candle>,
    #[serde(skip)]
    pub candles: Vec<Candle>,
    #[serde(skip)]
    pub available_symbols: Vec<String>,
    #[serde(skip)]
    in_memory_custom_doc: Option<(DataFileFormat, String)>,
    #[serde(skip)]
    pub show_import_bar: bool,
    #[serde(skip)]
    pub last_load_status: Option<(bool, String)>,
    #[serde(skip)]
    pub last_trade_status: Option<(bool, String)>,
    #[serde(skip)]
    viewport_focus_lock: Option<(f64, f64, f64)>,
    #[serde(skip)]
    pub indicator_cache: IndicatorCache,
    #[serde(skip)]
    dataset_generation: u64,
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
            drawings: DrawingStore::new(),
            drawings_by_instrument: BTreeMap::new(),
            replay: ReplayController::new(),
            trading: TradingSession::default(),
            replay_trading_by_instrument: BTreeMap::new(),
            show_replay_bar: false,
            show_trading_panel: false,
            ticket_kind: TicketOrderKind::Market,
            ticket_qty_input: "10".to_owned(),
            ticket_price_input: String::new(),
            ticket_sl_input: String::new(),
            ticket_tp_input: String::new(),
            base_candles,
            candles,
            available_symbols: vec!["VNINDEX".to_owned()],
            in_memory_custom_doc: None,
            show_import_bar: false,
            last_load_status: None,
            last_trade_status: None,
            viewport_focus_lock: None,
            indicator_cache,
            dataset_generation: 1,
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
        if self.ticket_qty_input.trim().is_empty() || self.ticket_qty_input.len() > 32 {
            self.ticket_qty_input = "10".to_owned();
        }
        self.ticket_price_input.truncate(32);
        self.ticket_sl_input.truncate(32);
        self.ticket_tp_input.truncate(32);
        self.viewport_focus_lock = None;
        self.in_memory_custom_doc = None;
        self.last_trade_status = None;

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
            self.show_replay_bar = false;
        } else {
            self.replay.sanitize_deserialized(&report.candles);
            if self.replay.is_active() {
                self.show_replay_bar = true;
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
        self.indicator_cache
            .sync(&self.candles, self.dataset_generation, &self.indicators);
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

        self.active_timeframe = target_tf;
        self.timeframe = target_tf.label().to_owned();
        self.dataset_generation = self.dataset_generation.wrapping_add(1).max(1);
        self.indicator_cache
            .sync(&self.candles, self.dataset_generation, &self.indicators);

        Ok(())
    }

    fn current_instrument_key(&self) -> String {
        let sym_upper = self.symbol.trim().to_ascii_uppercase();
        match self.data_source {
            DataSourcePreset::CustomFile => {
                if let Some((fmt, ref doc)) = self.in_memory_custom_doc {
                    format!(
                        "CustomMem:{fmt:?}:{:016x}:{sym_upper}",
                        fnv1a_64(doc.as_bytes())
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
        let outgoing_trading = std::mem::take(&mut self.trading);
        if outgoing_replay.is_active() || !outgoing_trading.commands.is_empty() {
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

        match custom_update {
            CustomSourceUpdate::Clear => {
                self.in_memory_custom_doc = None;
            }
            CustomSourceUpdate::KeepExisting => {}
            #[cfg(not(target_arch = "wasm32"))]
            CustomSourceUpdate::FilePath(p) => {
                self.in_memory_custom_doc = None;
                self.import_path_input = p.to_owned();
            }
            CustomSourceUpdate::InMemory(fmt, text) => {
                self.in_memory_custom_doc = Some((fmt, text.to_owned()));
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

        self.dataset_generation = self.dataset_generation.wrapping_add(1).max(1);
        self.viewport.time_scale.set_total_bars(self.candles.len());
        if reset_viewport {
            self.viewport.reset_to_default();
        }
        self.indicator_cache
            .sync(&self.candles, self.dataset_generation, &self.indicators);
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

        self.dataset_generation = self.dataset_generation.wrapping_add(1).max(1);
        self.indicator_cache
            .sync(&self.candles, self.dataset_generation, &self.indicators);
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

        self.dataset_generation = self.dataset_generation.wrapping_add(1).max(1);
        self.indicator_cache
            .sync(&self.candles, self.dataset_generation, &self.indicators);
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

        self.dataset_generation = self.dataset_generation.wrapping_add(1).max(1);
        self.indicator_cache
            .sync(&self.candles, self.dataset_generation, &self.indicators);
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
    pub fn sync_ticket_from_selected_drawing(&mut self) -> bool {
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
        if self.replay.is_playing() {
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

        let was_selecting_cut_bar = self.replay.selecting_cut_bar;

        // 1. Top Workstation Toolbar
        let top_frame = Frame::new()
            .fill(palette::BG_WORKSTATION)
            .inner_margin(Margin::symmetric(12, 7))
            .stroke(Stroke::new(1.0, palette::BORDER_SUBTLE));

        egui::Panel::top("workstation_top_toolbar")
            .frame(top_frame)
            .show(ui, |ui| {
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

                    let _ = indicator_toggle_pill(
                        ui,
                        &mut self.show_trading_panel,
                        "Paper Trade",
                        palette::TRADE_ENTRY_BLUE,
                    );
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
                    }
                });

                // Optional Row 4: Compact Causal Bar Replay Transport Dock (`Milestone R07`)
                if self.show_replay_bar || self.replay.is_active() {
                    ui.add_space(4.0);
                    self.render_replay_dock(ui);
                }
            });

        // Synchronize dirty-tracked indicator cache (0.000 ms on pan/zoom/hover frames!)
        self.indicator_cache
            .sync(&self.candles, self.dataset_generation, &self.indicators);

        // 2. Bottom Telemetry Status Bar
        let bottom_frame = Frame::new()
            .fill(palette::BG_WORKSTATION)
            .inner_margin(Margin::symmetric(12, 5))
            .stroke(Stroke::new(1.0, palette::BORDER_SUBTLE));

        egui::Panel::bottom("workstation_status_bar")
            .frame(bottom_frame)
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    let visible = self.visible_slice();
                    let first_date = visible.first().map_or("-", |c| c.date_label.as_str());
                    let last_date = visible.last().map_or("-", |c| c.date_label.as_str());
                    let scale_mode = self.viewport.main_price_scale.mode().label();
                    let auto_tag = if self.viewport.main_price_scale.is_auto_scale() {
                        "AUTO"
                    } else {
                        "MANUAL"
                    };

                    let mag_tag = match self.drawings.magnet_mode {
                        MagnetMode::Off => "OFF",
                        MagnetMode::Ohlc => "OHLC",
                    };
                    let replay_tag = if self.replay.is_active() {
                        format!(
                            "{} ({}/{})",
                            self.replay.status.short_label(),
                            self.replay
                                .base_cursor_index
                                .saturating_add(1)
                                .min(self.base_candles.len()),
                            self.base_candles.len()
                        )
                    } else if self.replay.selecting_cut_bar {
                        "SELECT CUT".to_owned()
                    } else {
                        "LIVE".to_owned()
                    };
                    let metrics = self.trading.metrics(self.current_mark_price());
                    if let Some(rep) = self.last_report {
                        ui.label(
                            RichText::new(format!(
                                "R07 WORKSTATION   |   FEED: {} ({}->{})   |   REPLAY: {replay_tag}   |   EQUITY: ${:.0} ({:+.0} / {} fills)   |   PANES: {} (OVL: {})   |   DRAW: {} ({mag_tag})   |   BARS: {} drawn / {} ({first_date}..{last_date})   |   CONF: {}x   |   {scale_mode} ({auto_tag})   |   {:.0}×{:.0}px",
                                self.data_source.short_label(),
                                self.base_timeframe,
                                self.active_timeframe,
                                metrics.equity,
                                metrics.net_pnl,
                                metrics.fill_count,
                                rep.pane_count,
                                rep.active_overlay_count,
                                rep.drawing_count,
                                rep.rendered_candle_count,
                                self.candles.len(),
                                rep.conflation_factor,
                                rep.layout.plot_rect.width(),
                                rep.layout.plot_rect.height(),
                            ))
                            .monospace()
                            .size(11.0)
                            .color(palette::TEXT_MUTED),
                        );
                    } else {
                        ui.label(
                            RichText::new(format!(
                                "R07 WORKSTATION   |   REPLAY: {replay_tag}   |   TOTAL BARS: {} ({first_date} .. {last_date})",
                                self.candles.len()
                            ))
                            .monospace()
                            .size(11.0)
                            .color(palette::TEXT_MUTED),
                        );
                    }
                });
            });

        // 3. Optional Right-Side Paper Trading Execution & Blotter Panel (`Milestone R07`)
        if self.show_trading_panel {
            let right_frame = Frame::new()
                .fill(palette::BG_WORKSTATION)
                .inner_margin(Margin::symmetric(10, 8))
                .stroke(Stroke::new(1.0, palette::BORDER_SUBTLE));

            egui::Panel::right("paper_trading_panel")
                .resizable(true)
                .default_size(250.0)
                .size_range(218.0..=304.0)
                .frame(right_frame)
                .show(ui, |ui| {
                    self.render_paper_trading_panel(ui);
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
                let replay_status_label = self
                    .replay
                    .is_active()
                    .then(|| self.replay.status.short_label());
                let has_trading_visuals = self.show_trading_panel
                    || self.trading.open_position().is_some()
                    || !self.trading.fills().is_empty()
                    || self.trading.pending_orders().next().is_some();
                let overlay = ChartReplayTradingOverlay {
                    replay_active: self.replay.is_active(),
                    selecting_cut_bar: self.replay.selecting_cut_bar,
                    replay_status_label,
                    trading_session: has_trading_visuals.then_some(&self.trading),
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
                clicked_cut_idx = self.last_report.and_then(|r| r.clicked_cut_bar_index);
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

        // 5. Non-modal keyboard transport shortcuts (suppressed whenever any text input holds focus)
        if !ui.ctx().egui_wants_keyboard_input() {
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
    /// or `"r07"` offscreen in one headless GPU session (zero windows, zero Dock icons, zero occlusion stalls).
    pub fn run_offscreen_wgpu_capture_batch(batch_name: &str) -> Result<(), String> {
        if !matches!(batch_name, "r04" | "r05" | "r06" | "r07") {
            return Err(format!(
                "unknown SUMI_CAPTURE_BATCH '{batch_name}' (expected 'r04', 'r05', 'r06', or 'r07')"
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
            let json = format!(
                "{{\n  \"symbol\": \"{}\",\n  \"timeframe\": \"{}\",\n  \"base_timeframe\": \"{}\",\n  \"data_source\": \"{}\",\n  \"available_symbols\": [{}],\n  \"base_candles\": {},\n  \"total_candles\": {},\n  \"culled_candles\": {},\n  \"rendered_candles\": {},\n  \"conflation_factor\": {},\n  \"bar_spacing\": {:.6},\n  \"right_offset\": {:.6},\n  \"scale_mode\": \"{}\",\n  \"auto_scale\": {},\n  \"pane_count\": {},\n  \"active_overlay_count\": {},\n  \"drawing_count\": {},\n  \"active_drawing_tool\": \"{}\",\n  \"selected_drawing_id\": {},\n  \"magnet_mode\": \"{}\",\n  \"replay_active\": {},\n  \"replay_status\": \"{}\",\n  \"replay_base_cursor\": {},\n  \"replay_step_unit\": \"{}\",\n  \"show_trading_panel\": {},\n  \"active_price_lines\": {},\n  \"rendered_fill_markers\": {},\n  \"equity\": {:.2},\n  \"net_pnl\": {:.2},\n  \"fill_count\": {},\n  \"closed_trade_count\": {},\n  \"pending_order_count\": {},\n  \"open_position_side\": {},\n  \"stretches\": [{:.3}, {:.3}, {:.3}, {:.3}],\n  \"base_value\": {:.4},\n  \"screenshot_px\": [{}, {}],\n  \"outer_rect\": [{:.1}, {:.1}, {:.1}, {:.1}],\n  \"plot_rect\": [{:.1}, {:.1}, {:.1}, {:.1}],\n  \"price_axis_rect\": [{:.1}, {:.1}, {:.1}, {:.1}],\n  \"time_axis_rect\": [{:.1}, {:.1}, {:.1}, {:.1}],\n  \"raw_price_min\": {:.2},\n  \"raw_price_max\": {:.2}\n}}\n",
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
}
