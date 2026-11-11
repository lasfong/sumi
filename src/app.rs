use egui::{Color32, CornerRadius, Frame, Margin, RichText, Stroke, Vec2, Visuals};
use serde::{Deserialize, Serialize};

use crate::chart::{
    ChartDisplayConfig, ChartRenderReport, ChartViewportState, palette, render_candlestick_chart,
};
use crate::data::generate_synthetic_ohlcv;
use crate::model::Candle;
use crate::viewport::PriceScaleMode;

/// Primary application state for the Sumi Rust Trading Workstation (`Milestone R03`).
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

    #[serde(skip)]
    pub candles: Vec<Candle>,
    #[serde(skip)]
    pub last_report: Option<ChartRenderReport>,
    #[serde(skip)]
    pub frame_count: u64,
    #[serde(skip)]
    pub screenshot_requested: bool,
    #[serde(skip)]
    scenario_applied: bool,
}

impl Default for SumiWorkstationApp {
    fn default() -> Self {
        let dataset_bar_count = 120;
        let candles = generate_synthetic_ohlcv(dataset_bar_count);
        let mut viewport = ChartViewportState::new();
        viewport.time_scale.set_total_bars(candles.len());

        Self {
            symbol: "VNINDEX".to_owned(),
            timeframe: "1D".to_owned(),
            show_grid: true,
            show_crosshair: true,
            show_volume: true,
            dataset_bar_count,
            viewport,
            candles,
            last_report: None,
            frame_count: 0,
            screenshot_requested: false,
            scenario_applied: false,
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
            .is_some_and(|p| !p.is_empty());

        let mut app: Self = if !is_verification_capture && let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Self::default()
        };
        app.viewport.sanitize_deserialized();

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
        app.viewport.time_scale.set_total_bars(app.candles.len());
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
        self.candles = generate_synthetic_ohlcv(clamped);
        self.dataset_bar_count = self.candles.len();
        self.viewport.time_scale.set_total_bars(self.candles.len());
        self.viewport.reset_to_default();
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
    }
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
                    ui.spacing_mut().item_spacing = Vec2::new(6.0, 4.0);

                    // Brand pill
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

                    // Symbol & Timeframe badge
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

                    // Latest series bar summary
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

                    // Dataset scale presets (120, 10K, 50K, 100K)
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

                    // PriceScaleMode selector (`Linear` | `Log` | `%` | `Idx100` + `Auto` + `Inv`)
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

                    // Viewport navigation controls
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
            });

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

                    if let Some(rep) = self.last_report {
                        ui.label(
                            RichText::new(format!(
                                "R03 VIEWPORT   |   BARS: {} culled / {} drawn / {} total ({first_date} → {last_date})   |   CONFLATION: {}x   |   SPACING: {:.3}px   |   RIGHT OFFSET: {:+.1}   |   SCALE: {scale_mode} ({auto_tag})   |   PLOT: {:.0}×{:.0}px",
                                rep.culled_bar_count,
                                rep.rendered_candle_count,
                                self.candles.len(),
                                rep.conflation_factor,
                                rep.bar_spacing,
                                rep.right_offset,
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
                                "R03 VIEWPORT   |   TOTAL BARS: {} ({first_date} → {last_date})",
                                self.candles.len()
                            ))
                            .monospace()
                            .size(11.0)
                            .color(palette::TEXT_MUTED),
                        );
                    }
                });
            });

        // 3. Central Candlestick Chart Canvas
        let central_frame = Frame::new()
            .fill(palette::BG_WORKSTATION)
            .inner_margin(Margin::symmetric(8, 8));

        egui::CentralPanel::default()
            .frame(central_frame)
            .show(ui, |ui| {
                self.last_report = render_candlestick_chart(
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

        // Apply deterministic verification scenario once plot_rect width/height are established
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

        let screenshot_image = ui.input(|i| {
            for event in &i.events {
                if let egui::Event::Screenshot { image, .. } = event {
                    return Some(std::sync::Arc::clone(image));
                }
            }
            None
        });

        if let Some(image) = screenshot_image {
            if let Err(err) = Self::write_color_image_png(&screenshot_path, &image) {
                log::error!("Failed to save verification screenshot to {screenshot_path}: {err}");
            } else {
                log::info!(
                    "Saved native GPU verification screenshot ({}x{}) to {screenshot_path}",
                    image.width(),
                    image.height()
                );
            }

            if let Ok(telemetry_path) = std::env::var("SUMI_TELEMETRY_PATH")
                && !telemetry_path.is_empty()
                && let Some(rep) = self.last_report
            {
                let json = format!(
                    "{{\n  \"symbol\": \"{}\",\n  \"timeframe\": \"{}\",\n  \"total_candles\": {},\n  \"culled_candles\": {},\n  \"rendered_candles\": {},\n  \"conflation_factor\": {},\n  \"bar_spacing\": {:.6},\n  \"right_offset\": {:.6},\n  \"scale_mode\": \"{}\",\n  \"auto_scale\": {},\n  \"base_value\": {:.4},\n  \"screenshot_px\": [{}, {}],\n  \"outer_rect\": [{:.1}, {:.1}, {:.1}, {:.1}],\n  \"plot_rect\": [{:.1}, {:.1}, {:.1}, {:.1}],\n  \"price_axis_rect\": [{:.1}, {:.1}, {:.1}, {:.1}],\n  \"time_axis_rect\": [{:.1}, {:.1}, {:.1}, {:.1}],\n  \"raw_price_min\": {:.2},\n  \"raw_price_max\": {:.2}\n}}\n",
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
                let _ = std::fs::write(&telemetry_path, json);
            }

            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
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
    fn app_state_zoom_fit_scale_modes_and_dataset_switching_stay_within_bounds() {
        let mut app = SumiWorkstationApp::default();
        assert_eq!(app.candles.len(), 120);
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

        app.set_dataset_size(10_000);
        assert_eq!(app.candles.len(), 10_000);
        assert_eq!(app.viewport.time_scale.total_bars(), 10_000);

        app.fit_content();
        assert!(app.visible_slice().len() == 10_000);

        app.reset_view();
        assert!((app.viewport.time_scale.right_offset() - 12.0).abs() < 1e-6);
        assert!((app.viewport.time_scale.bar_spacing() - 8.0).abs() < 1e-6);
    }
}
