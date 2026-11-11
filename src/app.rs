use egui::{Button, Color32, CornerRadius, Frame, Margin, RichText, Stroke, Vec2, Visuals};
use serde::{Deserialize, Serialize};

use crate::chart::{
    ChartDisplayConfig, ChartRenderReport, ChartViewportState, palette,
    render_workstation_chart_with_drawings,
};
use crate::data::generate_synthetic_ohlcv;
use crate::drawings::{
    DrawingAnchor, DrawingColorPreset, DrawingGeometry, DrawingPaneTarget, DrawingStore,
    DrawingStyle, DrawingTool, FibDirection, LineDashStyle, MagnetMode,
};
use crate::indicators::OscillatorPaneId;
use crate::indicators::{IndicatorCache, IndicatorConfig};
use crate::model::Candle;
use crate::viewport::PriceScaleMode;

/// Primary application state for the Sumi Rust Trading Workstation (`Milestone R05`).
#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct SumiWorkstationApp {
    pub symbol: String,
    pub timeframe: String,
    pub show_grid: bool,
    pub show_crosshair: bool,
    pub show_volume: bool,
    pub dataset_bar_count: usize,
    pub viewport: ChartViewportState,
    pub indicators: IndicatorConfig,
    pub drawings: DrawingStore,

    #[serde(skip)]
    pub candles: Vec<Candle>,
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
    #[serde(skip)]
    scenario_applied: bool,
    #[serde(skip)]
    batch_step: usize,
    #[serde(skip)]
    batch_step_start_frame: u64,
}

impl Default for SumiWorkstationApp {
    fn default() -> Self {
        let dataset_bar_count = 120;
        let candles = generate_synthetic_ohlcv(dataset_bar_count);
        let mut viewport = ChartViewportState::new();
        viewport.time_scale.set_total_bars(candles.len());
        let indicators = IndicatorConfig::default();
        let mut indicator_cache = IndicatorCache::new();
        indicator_cache.sync(&candles, 1, &indicators);

        Self {
            symbol: "VNINDEX".to_owned(),
            timeframe: "1D".to_owned(),
            show_grid: true,
            show_crosshair: true,
            show_volume: true,
            dataset_bar_count,
            viewport,
            indicators,
            drawings: DrawingStore::new(),
            candles,
            indicator_cache,
            dataset_generation: 1,
            last_report: None,
            frame_count: 0,
            screenshot_requested: false,
            scenario_applied: false,
            batch_step: 0,
            batch_step_start_frame: 0,
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
        app.viewport.sanitize_deserialized();
        app.indicators.sanitize_deserialized();
        app.drawings.sanitize_deserialized();

        if let Ok(bars_str) = std::env::var("SUMI_BAR_COUNT")
            && let Ok(bars) = bars_str.parse::<usize>()
            && bars >= 10
        {
            app.dataset_bar_count = bars;
            app.candles.clear();
        }

        let target_count = app.dataset_bar_count.max(10);
        if app.candles.len() != target_count {
            app.candles = generate_synthetic_ohlcv(target_count);
            app.dataset_bar_count = app.candles.len();
        }
        app.dataset_generation = app.dataset_generation.max(1);
        app.viewport.time_scale.set_total_bars(app.candles.len());
        app.indicator_cache
            .sync(&app.candles, app.dataset_generation, &app.indicators);
        app
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
        self.drawings.cancel_active_gesture();
        self.candles = generate_synthetic_ohlcv(clamped);
        self.dataset_bar_count = self.candles.len();
        self.dataset_generation = self.dataset_generation.wrapping_add(1).max(1);
        self.viewport.time_scale.set_total_bars(self.candles.len());
        self.viewport.reset_to_default();
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
        if self.viewport.time_scale.width() <= 0.0 {
            self.viewport.time_scale.set_width(1000.0);
        }
        self.viewport.zoom_around_center(2.5);
    }

    pub fn zoom_out(&mut self) {
        if self.viewport.time_scale.width() <= 0.0 {
            self.viewport.time_scale.set_width(1000.0);
        }
        self.viewport.zoom_around_center(-2.5);
    }

    pub fn fit_content(&mut self) {
        if self.viewport.time_scale.width() <= 0.0 {
            self.viewport.time_scale.set_width(1000.0);
        }
        self.viewport.fit_all();
    }

    pub fn reset_view(&mut self) {
        self.viewport.reset_to_default();
        self.viewport.reset_pane_stretches();
    }

    /// Populates a deterministic, data-anchored set of multi-pane annotations (`HorizontalLine`,
    /// `TrendLine`, $360^\circ$ `Ray`, `Rectangle`, `FibonacciRetracement` + `RSI` / `MACD` / `ATR`
    /// sub-pane annotations) scaled cleanly to the currently visible candle range as a single
    /// undoable `DrawingCommand::ReplaceAll` step preserving `magnet_mode` and `locked` history!
    pub fn populate_showcase_drawings(&mut self) {
        if self.candles.len() < 10 {
            return;
        }
        let vis_range = self.viewport.time_scale.strict_visible_data_range();
        let (left_i, right_i) = if vis_range.len() >= 12 {
            (vis_range.start, vis_range.end - 1)
        } else {
            let start = self.candles.len().saturating_sub(120);
            (start, self.candles.len() - 1)
        };
        let span = (right_i - left_i).max(10);
        let idx_at = |frac: f64| -> usize {
            (left_i + ((span as f64) * frac).round() as usize).min(self.candles.len() - 1)
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
        let price_span = (slice_high.1 - slice_low.1).max(5.0);

        let mut showcase = Vec::with_capacity(8);
        let make_drawing = |pane: DrawingPaneTarget,
                            geom: DrawingGeometry,
                            style: Option<DrawingStyle>,
                            locked: bool| {
            let resolved_style =
                style.unwrap_or_else(|| DrawingStyle::default_for_tool(geom.tool_kind()));
            let mut d = crate::drawings::Drawing::new(0, pane, geom, resolved_style);
            d.locked = locked;
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
        let tl_i0 = idx_at(0.08);
        let tl_i1 = idx_at(0.54);
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
        let ray_i0 = idx_at(0.60);
        let ray_i1 = idx_at(0.88);
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
        let rect_i0 = idx_at(0.18);
        let rect_i1 = idx_at(0.42);
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
        let fib_i0 = idx_at(0.46);
        let fib_i1 = idx_at(0.92);
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
            let r0 = idx_at(0.15);
            let r1 = idx_at(0.82);
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

        // 7. MACD Sub-Pane: Negative-to-positive accumulation Rectangle (proving `value <= 0` support)
        if self.indicators.show_macd {
            let m0 = idx_at(0.25);
            let m1 = idx_at(0.75);
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
            let lo = mv0.min(mv1) - 1.5;
            let hi = mv0.max(mv1) + 1.5;
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
        // 1. Top Workstation Toolbar
        let top_frame = Frame::new()
            .fill(palette::BG_WORKSTATION)
            .inner_margin(Margin::symmetric(12, 8))
            .stroke(Stroke::new(1.0, palette::BORDER_SUBTLE));

        egui::Panel::top("workstation_top_toolbar")
            .frame(top_frame)
            .show(ui, |ui| {
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

                    ui.separator();

                    // Technical Indicator Toggles (`SMA`, `EMA`, `BB` Overlays + `RSI`, `MACD`, `ATR` Panes)
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
                    if ui.button("Demo Set").clicked() {
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
                                "Ext →: ON"
                            } else {
                                "Ext →: OFF"
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
                    }
                });
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
                    if let Some(rep) = self.last_report {
                        ui.label(
                            RichText::new(format!(
                                "R05 WORKSTATION   |   PANES: {} (OVL: {})   |   DRAWINGS: {} (TOOL: {}, MAG: {mag_tag})   |   BARS: {} culled / {} drawn / {} total ({first_date} → {last_date})   |   CONFLATION: {}x   |   SPACING: {:.3}px   |   SCALE: {scale_mode} ({auto_tag})   |   PLOT: {:.0}×{:.0}px",
                                rep.pane_count,
                                rep.active_overlay_count,
                                rep.drawing_count,
                                rep.active_drawing_tool.short_label(),
                                rep.culled_bar_count,
                                rep.rendered_candle_count,
                                self.candles.len(),
                                rep.conflation_factor,
                                rep.bar_spacing,
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
                                "R05 WORKSTATION   |   TOTAL BARS: {} ({first_date} → {last_date})",
                                self.candles.len()
                            ))
                            .monospace()
                            .size(11.0)
                            .color(palette::TEXT_MUTED),
                        );
                    }
                });
            });

        // 3. Central Multi-Pane Workstation Chart Canvas
        let central_frame = Frame::new()
            .fill(palette::BG_WORKSTATION)
            .inner_margin(Margin::symmetric(8, 8));

        egui::CentralPanel::default()
            .frame(central_frame)
            .show(ui, |ui| {
                self.last_report = render_workstation_chart_with_drawings(
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
                );
            });

        self.frame_count = self.frame_count.saturating_add(1);

        #[cfg(not(target_arch = "wasm32"))]
        self.handle_automated_verification_capture(ui);
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl SumiWorkstationApp {
    fn handle_automated_verification_capture(&mut self, ui: &mut egui::Ui) {
        if let Ok(batch_name) = std::env::var("SUMI_CAPTURE_BATCH") {
            match batch_name.as_str() {
                "r04" => {
                    self.handle_r04_single_session_batch_capture(ui);
                    return;
                }
                "r05" => {
                    self.handle_r05_single_session_batch_capture(ui);
                    return;
                }
                _ => {}
            }
        }

        let Ok(screenshot_path) = std::env::var("SUMI_SCREENSHOT_PATH") else {
            return;
        };
        if screenshot_path.is_empty() {
            return;
        }

        ui.ctx().request_repaint();

        if self.frame_count == 1
            && let (Ok(w_str), Ok(h_str)) = (
                std::env::var("SUMI_WINDOW_WIDTH"),
                std::env::var("SUMI_WINDOW_HEIGHT"),
            )
            && let (Ok(w), Ok(h)) = (w_str.parse::<f32>(), h_str.parse::<f32>())
        {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(w, h)));
        }

        if self.frame_count == 2 && !self.scenario_applied {
            self.scenario_applied = true;
            if let Ok(scenario) = std::env::var("SUMI_SCENARIO") {
                match scenario.as_str() {
                    "zoom_cursor" => {
                        let w = self.viewport.time_scale.width();
                        self.viewport.time_scale.zoom(w * 0.65, 8.0);
                    }
                    "pan_history" => {
                        self.viewport.time_scale.set_right_offset(-35.0);
                    }
                    "log_100k_fit" => {
                        self.set_scale_mode(PriceScaleMode::Logarithmic);
                        self.viewport.fit_all();
                    }
                    "pct_mode" => {
                        self.set_scale_mode(PriceScaleMode::Percentage);
                    }
                    _ => {}
                }
            }
        }

        if self.frame_count >= 6 && !self.screenshot_requested {
            self.screenshot_requested = true;
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }

        if let Some(image) = Self::take_screenshot_event(ui) {
            self.persist_capture_artifacts(
                &screenshot_path,
                std::env::var("SUMI_TELEMETRY_PATH").ok().as_deref(),
                &image,
            );
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    /// Captures all 3 Milestone R04 Retina GPU verification scenarios inside ONE single background
    /// window session (zero window flashing, zero focus stealing).
    fn handle_r04_single_session_batch_capture(&mut self, ui: &mut egui::Ui) {
        ui.ctx().request_repaint();

        if self.batch_step_start_frame == 0 {
            let _ = std::fs::create_dir_all("docs/evidence");
            self.batch_step = 0;
            self.batch_step_start_frame = self.frame_count;
            self.apply_r04_batch_step(ui, 0);
            return;
        }

        let elapsed_frames = self.frame_count.saturating_sub(self.batch_step_start_frame);
        let settle_frames = if self.batch_step == 2 { 8 } else { 6 };

        if elapsed_frames >= settle_frames && !self.screenshot_requested {
            self.screenshot_requested = true;
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }

        if let Some(image) = Self::take_screenshot_event(ui) {
            let (png_path, json_path) = match self.batch_step {
                0 => (
                    "docs/evidence/r04_multipane_default_1280x800.png",
                    "docs/evidence/r04_multipane_default_1280x800.json",
                ),
                1 => (
                    "docs/evidence/r04_all_indicators_100k_fit_1280x800.png",
                    "docs/evidence/r04_all_indicators_100k_fit_1280x800.json",
                ),
                _ => (
                    "docs/evidence/r04_resized_separator_960x640.png",
                    "docs/evidence/r04_resized_separator_960x640.json",
                ),
            };
            self.persist_capture_artifacts(png_path, Some(json_path), &image);

            self.batch_step += 1;
            self.screenshot_requested = false;
            if self.batch_step >= 3 {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            } else {
                self.batch_step_start_frame = self.frame_count;
                self.apply_r04_batch_step(ui, self.batch_step);
            }
        }
    }

    fn apply_r04_batch_step(&mut self, ui: &mut egui::Ui, step: usize) {
        match step {
            0 => {
                // Scenario 0: 1280x800, 500 bars, SMA+EMA+BB overlays + RSI + MACD sub-panes (3 panes)
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(1280.0, 800.0)));
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
            }
            1 => {
                // Scenario 1: 1280x800, 100,000 bars full-fit Logarithmic + ALL 6 indicators (4 stacked panes, 32x conflation)
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
            }
            _ => {
                // Scenario 2: Resized 960x640 compact layout + separator resized (expanded RSI pane)
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(960.0, 640.0)));
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
            }
        }
    }

    /// Captures all 3 Milestone R05 Retina GPU drawing verification scenarios inside ONE single
    /// background window session (`with_active(false)`, zero window flashing, zero focus stealing).
    fn handle_r05_single_session_batch_capture(&mut self, ui: &mut egui::Ui) {
        ui.ctx().request_repaint();

        if self.batch_step_start_frame == 0 {
            let _ = std::fs::create_dir_all("docs/evidence");
            self.batch_step = 0;
            self.batch_step_start_frame = self.frame_count;
            self.apply_r05_batch_step(ui, 0);
            return;
        }

        let elapsed_frames = self.frame_count.saturating_sub(self.batch_step_start_frame);
        let settle_frames = if self.batch_step == 2 { 8 } else { 6 };

        if elapsed_frames >= settle_frames && !self.screenshot_requested {
            self.screenshot_requested = true;
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }

        if let Some(image) = Self::take_screenshot_event(ui) {
            let (png_path, json_path) = match self.batch_step {
                0 => (
                    "docs/evidence/r05_drawings_showcase_1280x800.png",
                    "docs/evidence/r05_drawings_showcase_1280x800.json",
                ),
                1 => (
                    "docs/evidence/r05_drawings_log_100k_fit_1280x800.png",
                    "docs/evidence/r05_drawings_log_100k_fit_1280x800.json",
                ),
                _ => (
                    "docs/evidence/r05_drawings_interaction_resized_960x640.png",
                    "docs/evidence/r05_drawings_interaction_resized_960x640.json",
                ),
            };
            self.persist_capture_artifacts(png_path, Some(json_path), &image);

            self.batch_step += 1;
            self.screenshot_requested = false;
            if self.batch_step >= 3 {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            } else {
                self.batch_step_start_frame = self.frame_count;
                self.apply_r05_batch_step(ui, self.batch_step);
            }
        }
    }

    fn apply_r05_batch_step(&mut self, ui: &mut egui::Ui, step: usize) {
        match step {
            0 => {
                // Scenario 0: 1280x800, 240 bars, 3 panes (Main + RSI + MACD), all 5 primitives + selected Fib with handles
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(1280.0, 800.0)));
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
            }
            1 => {
                // Scenario 1: 1280x800, 100,000 bars full-fit Logarithmic + ALL 6 indicators (4 panes, 32x conflation) + 8 multi-pane annotations
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
            }
            _ => {
                // Scenario 2: Resized 960x640 compact layout + OHLC Magnet ON + leftward 360° Ray + live rubber-band TrendLine preview
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(960.0, 640.0)));
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

                // Add an explicit leftward 360° Ray (`dx < 0`) to visually demonstrate full directional Liang-Barsky clipping!
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

                // Seed a live 2-click rubber-band TrendLine preview using `PaneDrawingProjection` + OHLC Magnet!
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
            }
        }
    }

    fn take_screenshot_event(ui: &egui::Ui) -> Option<std::sync::Arc<egui::ColorImage>> {
        ui.input(|i| {
            for event in &i.events {
                if let egui::Event::Screenshot { image, .. } = event {
                    return Some(std::sync::Arc::clone(image));
                }
            }
            None
        })
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
            let json = format!(
                "{{\n  \"symbol\": \"{}\",\n  \"timeframe\": \"{}\",\n  \"total_candles\": {},\n  \"culled_candles\": {},\n  \"rendered_candles\": {},\n  \"conflation_factor\": {},\n  \"bar_spacing\": {:.6},\n  \"right_offset\": {:.6},\n  \"scale_mode\": \"{}\",\n  \"auto_scale\": {},\n  \"pane_count\": {},\n  \"active_overlay_count\": {},\n  \"drawing_count\": {},\n  \"active_drawing_tool\": \"{}\",\n  \"selected_drawing_id\": {},\n  \"magnet_mode\": \"{}\",\n  \"stretches\": [{:.3}, {:.3}, {:.3}, {:.3}],\n  \"base_value\": {:.4},\n  \"screenshot_px\": [{}, {}],\n  \"outer_rect\": [{:.1}, {:.1}, {:.1}, {:.1}],\n  \"plot_rect\": [{:.1}, {:.1}, {:.1}, {:.1}],\n  \"price_axis_rect\": [{:.1}, {:.1}, {:.1}, {:.1}],\n  \"time_axis_rect\": [{:.1}, {:.1}, {:.1}, {:.1}],\n  \"raw_price_min\": {:.2},\n  \"raw_price_max\": {:.2}\n}}\n",
                self.symbol,
                self.timeframe,
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

        // Single Undo restores user's previous drawing & selection!
        assert!(app.drawings.undo());
        assert_eq!(app.drawings.drawings.len(), 1);
        assert_eq!(app.drawings.drawings[0].id, user_id);
        assert_eq!(app.drawings.selected_id, Some(user_id));

        // Single Redo restores all 8 showcase drawings WITH `locked == true` preserved on item 0!
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
}
