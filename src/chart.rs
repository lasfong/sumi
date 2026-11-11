use egui::{Align2, Color32, CursorIcon, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};
use serde::{Deserialize, Serialize};

use crate::model::{Candle, PriceBounds};
use crate::viewport::{
    CandlestickBarMetrics, HorzScaleOptions, KineticAnimation, KineticScrollConfig, PaneViewport,
    PlotRect, PriceScale, PriceScaleOptions, TimeScale, conflated_candle_at,
    iter_conflated_candles,
};

/// TradingView-inspired dark workstation palette.
pub mod palette {
    use egui::Color32;

    pub const BG_WORKSTATION: Color32 = Color32::from_rgb(15, 19, 26); // #0F131A
    pub const BG_CHART_CANVAS: Color32 = Color32::from_rgb(19, 23, 34); // #131722
    pub const BG_AXIS_GUTTER: Color32 = Color32::from_rgb(22, 27, 38); // #161B26
    pub const BG_FLOATING_HUD: Color32 = Color32::from_rgba_premultiplied(19, 24, 34, 225);
    pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(42, 46, 57); // #2A2E39
    pub const GRID_LINE: Color32 = Color32::from_rgb(31, 36, 48); // #1F2430
    pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(209, 212, 220); // #D1D4DC
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(120, 123, 134); // #787B86
    pub const BULLISH: Color32 = Color32::from_rgb(38, 166, 154); // #26A69A
    pub const BEARISH: Color32 = Color32::from_rgb(239, 83, 80); // #EF5350
    // Properly premultiplied alpha (`rgb = round(base_rgb * 72 / 255) <= 72`) so dense volume bars never over-brighten!
    pub const BULLISH_VOL: Color32 = Color32::from_rgba_premultiplied(11, 47, 43, 72);
    pub const BEARISH_VOL: Color32 = Color32::from_rgba_premultiplied(67, 23, 23, 72);
    pub const CROSSHAIR: Color32 = Color32::from_rgb(149, 152, 161); // #9598A1
    pub const ACCENT_BLUE: Color32 = Color32::from_rgb(41, 98, 255); // #2962FF
}

/// Layout regions computed dynamically from the available chart area on every frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChartLayout {
    pub outer_rect: Rect,
    pub plot_rect: Rect,
    pub price_axis_rect: Rect,
    pub time_axis_rect: Rect,
    pub corner_rect: Rect,
}

impl ChartLayout {
    pub const PRICE_AXIS_WIDTH: f32 = 86.0;
    pub const TIME_AXIS_HEIGHT: f32 = 28.0;

    /// Computes a resize-aware layout splitting plot canvas, right price axis, and bottom time axis.
    #[must_use]
    pub fn from_outer_rect(outer_rect: Rect) -> Self {
        let price_axis_width = Self::PRICE_AXIS_WIDTH.min((outer_rect.width() * 0.25).max(52.0));
        let time_axis_height = Self::TIME_AXIS_HEIGHT.min((outer_rect.height() * 0.20).max(20.0));

        let split_x = (outer_rect.right() - price_axis_width).max(outer_rect.left() + 40.0);
        let split_y = (outer_rect.bottom() - time_axis_height).max(outer_rect.top() + 40.0);

        let plot_rect = Rect::from_min_max(outer_rect.min, Pos2::new(split_x, split_y));
        let price_axis_rect = Rect::from_min_max(
            Pos2::new(split_x, outer_rect.top()),
            Pos2::new(outer_rect.right(), split_y),
        );
        let time_axis_rect = Rect::from_min_max(
            Pos2::new(outer_rect.left(), split_y),
            Pos2::new(split_x, outer_rect.bottom()),
        );
        let corner_rect = Rect::from_min_max(Pos2::new(split_x, split_y), outer_rect.max);

        Self {
            outer_rect,
            plot_rect,
            price_axis_rect,
            time_axis_rect,
            corner_rect,
        }
    }
}

/// Modular viewport state coordinating the shared horizontal `TimeScale` and independent per-axis
/// `PriceScale` instances (`main-right` + `volume-overlay`), plus inertial kinetic scrolling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartViewportState {
    pub time_scale: TimeScale,
    pub main_price_scale: PriceScale,
    pub volume_price_scale: PriceScale,

    #[serde(skip)]
    pub kinetic: Option<KineticAnimation>,
}

impl Default for ChartViewportState {
    fn default() -> Self {
        Self::new()
    }
}

impl ChartViewportState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            time_scale: TimeScale::new(HorzScaleOptions::sumi_default()),
            main_price_scale: PriceScale::new("main-right", PriceScaleOptions::sumi_main_price()),
            volume_price_scale: PriceScale::new(
                "volume-overlay",
                PriceScaleOptions::sumi_volume_overlay(),
            ),
            kinetic: None,
        }
    }

    /// Sanitizes state restored from persistent `serde` storage against NaN/Infinity or mode mismatches.
    pub fn sanitize_deserialized(&mut self) {
        self.kinetic = None;
        self.time_scale.sanitize_deserialized();
        self.main_price_scale.sanitize_deserialized();
        self.volume_price_scale.sanitize_deserialized();
    }

    /// Enables or disables automatic sub-pixel OHLCV conflation on `time_scale`.
    pub fn set_conflation_enabled(&mut self, enabled: bool) {
        let mut opts = self.time_scale.options().clone();
        opts.enable_conflation = enabled;
        self.time_scale.apply_options(opts);
    }

    /// Zooms around the horizontal center of the plot area (`scale > 0` zooms in, `scale < 0` zooms out).
    pub fn zoom_around_center(&mut self, scale: f64) {
        self.kinetic = None;
        let center_x = (self.time_scale.width() * 0.5).max(1.0);
        self.time_scale.zoom(center_x, scale);
    }

    /// Fits the entire series `[0..total_bars)` plus default future `right_offset` and resets price autoscale.
    pub fn fit_all(&mut self) {
        self.kinetic = None;
        self.time_scale.fit_content();
        self.main_price_scale.reset_auto_scale();
        self.volume_price_scale.reset_auto_scale();
    }

    /// Restores default `bar_spacing` (`8.0`), default future `right_offset` (`12.0`), and price autoscale.
    pub fn reset_to_default(&mut self) {
        self.kinetic = None;
        self.time_scale.restore_default();
        self.main_price_scale.reset_auto_scale();
        self.volume_price_scale.reset_auto_scale();
    }
}

/// Visual configuration passed to `render_candlestick_chart`.
#[derive(Debug, Clone, Copy)]
pub struct ChartDisplayConfig<'a> {
    pub symbol: &'a str,
    pub timeframe: &'a str,
    pub show_grid: bool,
    pub show_crosshair: bool,
    pub show_volume: bool,
}

/// Telemetry returned by a chart render pass for HUD and verification readouts.
#[derive(Debug, Clone, Copy)]
pub struct ChartRenderReport {
    pub layout: ChartLayout,
    pub bounds: PriceBounds,
    pub inspected_data_index: usize,
    pub pointer_price: Option<f64>,
    pub bar_spacing: f64,
    pub right_offset: f64,
    pub conflation_factor: usize,
    pub culled_bar_count: usize,
    pub rendered_candle_count: usize,
    pub base_value: f64,
    pub kinetic_active: bool,
}

/// Renders the native candlestick chart, volume histogram, price/time axes, crosshair, and OHLCV overlay
/// powered by `crate::viewport::{TimeScale, PriceScale, PaneViewport, KineticAnimation}`.
pub fn render_candlestick_chart(
    ui: &mut Ui,
    candles: &[Candle],
    viewport: &mut ChartViewportState,
    config: ChartDisplayConfig<'_>,
) -> Option<ChartRenderReport> {
    render_shared_pane_candlestick_chart(
        ui,
        candles,
        &mut viewport.time_scale,
        &mut viewport.kinetic,
        &mut viewport.main_price_scale,
        &mut viewport.volume_price_scale,
        true,
        config,
    )
}

/// Multi-pane-capable chart renderer accepting a shared `&mut TimeScale` (and shared kinetic state)
/// paired with pane-local `&mut PriceScale` instances (`main_price_scale` + `volume_price_scale`).
///
/// Set `sync_shared_time_scale = true` on the primary/first pane in a frame so `set_total_bars`,
/// `set_width`, and kinetic inertia step once per frame across all stacked panes.
#[allow(clippy::too_many_arguments)]
pub fn render_shared_pane_candlestick_chart(
    ui: &mut Ui,
    candles: &[Candle],
    shared_time_scale: &mut TimeScale,
    shared_kinetic: &mut Option<KineticAnimation>,
    main_price_scale: &mut PriceScale,
    volume_price_scale: &mut PriceScale,
    sync_shared_time_scale: bool,
    config: ChartDisplayConfig<'_>,
) -> Option<ChartRenderReport> {
    if candles.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.label("No OHLCV data loaded.");
        });
        return None;
    }

    let available_size = ui.available_size().max(Vec2::new(200.0, 140.0));
    let (outer_rect, _) = ui.allocate_exact_size(available_size, Sense::hover());
    let layout = ChartLayout::from_outer_rect(outer_rect);
    let painter = ui.painter_at(outer_rect);

    let plot_id = ui.id().with("sumi_plot_canvas");
    let price_id = ui.id().with("sumi_price_axis");
    let time_id = ui.id().with("sumi_time_axis");

    let plot_resp = ui.interact(layout.plot_rect, plot_id, Sense::click_and_drag());
    let price_resp = ui.interact(layout.price_axis_rect, price_id, Sense::click_and_drag());
    let time_resp = ui.interact(layout.time_axis_rect, time_id, Sense::click_and_drag());

    // 1. Synchronize viewport dimensions & bar count
    let ppp = f64::from(ui.ctx().pixels_per_point()).max(1.0);
    let plot_rect_f64 = PlotRect::from_egui(layout.plot_rect);
    if sync_shared_time_scale {
        shared_time_scale.set_total_bars(candles.len());
        shared_time_scale.set_width(plot_rect_f64.width);
    }
    main_price_scale.set_height(plot_rect_f64.height);
    volume_price_scale.set_height(plot_rect_f64.height);

    let desired_bottom_margin = if config.show_volume { 0.20 } else { 0.10 };
    if (main_price_scale.options().scale_margin_bottom - desired_bottom_margin).abs() > 1e-6 {
        let top_m = main_price_scale.options().scale_margin_top;
        main_price_scale.set_margins(top_m, desired_bottom_margin);
    }

    // Ensure initial price range exists before first-frame axis drag gestures
    if main_price_scale.price_range().is_none() {
        let init_cf = shared_time_scale.conflation_factor(ppp);
        let _ = refresh_pane_autoscale_and_bounds(
            candles,
            shared_time_scale,
            main_price_scale,
            volume_price_scale,
            config.show_volume,
            init_cf,
        );
    }

    // 2. Advance active kinetic inertia on `TimeScale::right_offset` (or cancel immediately on pointer press)
    let (smooth_scroll, time_ms) = ui.input(|i| (i.smooth_scroll_delta, i.time * 1000.0));
    if plot_resp.is_pointer_button_down_on()
        || price_resp.is_pointer_button_down_on()
        || time_resp.is_pointer_button_down_on()
    {
        *shared_kinetic = None;
    } else if sync_shared_time_scale && !plot_resp.dragged() {
        let mut stop_kinetic = false;
        if let Some(ka) = shared_kinetic.as_ref()
            && ka.is_active()
        {
            if let Some(next_ro) = ka.position_at(time_ms) {
                let prev_ro = shared_time_scale.right_offset();
                shared_time_scale.set_right_offset(next_ro);
                let clamped_hit_wall = (shared_time_scale.right_offset() - next_ro).abs() > 1e-6
                    && (shared_time_scale.right_offset() - prev_ro).abs() < 1e-9;
                if ka.is_finished(time_ms) || clamped_hit_wall {
                    stop_kinetic = true;
                } else {
                    ui.ctx().request_repaint();
                }
            } else {
                stop_kinetic = true;
            }
        }
        if stop_kinetic {
            *shared_kinetic = None;
        }
    }

    // 3. Process interactions across Plot Canvas, Right Price Axis, and Bottom Time Axis
    handle_plot_interactions(
        ui,
        &plot_resp,
        plot_rect_f64,
        smooth_scroll,
        time_ms,
        shared_time_scale,
        shared_kinetic,
        main_price_scale,
        volume_price_scale,
    );
    handle_price_axis_interactions(ui, &price_resp, plot_rect_f64, main_price_scale);
    handle_time_axis_interactions(
        ui,
        &time_resp,
        plot_rect_f64,
        shared_time_scale,
        shared_kinetic,
    );

    // 4. Refresh visible slice, dynamic `base_value` (`firstValue`), and single-pass autoscale + bounds
    let conflation_factor = shared_time_scale.conflation_factor(ppp);
    let (base_value, bounds) = refresh_pane_autoscale_and_bounds(
        candles,
        shared_time_scale,
        main_price_scale,
        volume_price_scale,
        config.show_volume,
        conflation_factor,
    )?;
    let culling_range =
        non_empty_candle_range(shared_time_scale.culling_data_range(), candles.len());

    // 5. Paint backgrounds
    painter.rect_filled(layout.plot_rect, 0.0, palette::BG_CHART_CANVAS);
    painter.rect_filled(layout.price_axis_rect, 0.0, palette::BG_AXIS_GUTTER);
    painter.rect_filled(layout.time_axis_rect, 0.0, palette::BG_AXIS_GUTTER);
    painter.rect_filled(layout.corner_rect, 0.0, palette::BG_WORKSTATION);

    // Construct non-monolithic `PaneViewport` coordinate bridges (`Logical <-> PaneLocal <-> Screen`)
    let main_vp = PaneViewport::new(
        shared_time_scale,
        main_price_scale,
        plot_rect_f64,
        base_value,
    );
    let vol_vp = PaneViewport::new(shared_time_scale, volume_price_scale, plot_rect_f64, 1.0);

    // 6. Horizontal price grid & right price axis ticks (`Linear` / `Log` / `%` / `Idx100`)
    let target_price_ticks = ((layout.plot_rect.height() / 54.0).floor() as usize).clamp(4, 12);
    let price_ticks = main_price_scale.nice_ticks(target_price_ticks, base_value);
    for tick in &price_ticks {
        let y = main_vp.local_y_to_screen_y(tick.local_y) as f32;
        if config.show_grid {
            painter.line_segment(
                [
                    Pos2::new(layout.plot_rect.left(), y),
                    Pos2::new(layout.plot_rect.right(), y),
                ],
                Stroke::new(1.0, palette::GRID_LINE),
            );
        }

        painter.line_segment(
            [
                Pos2::new(layout.price_axis_rect.left(), y),
                Pos2::new(layout.price_axis_rect.left() + 5.0, y),
            ],
            Stroke::new(1.0, palette::BORDER_SUBTLE),
        );

        let label_y = y.clamp(
            layout.price_axis_rect.top() + 8.0,
            layout.price_axis_rect.bottom() - 8.0,
        );
        painter.text(
            Pos2::new(layout.price_axis_rect.left() + 8.0, label_y),
            Align2::LEFT_CENTER,
            &tick.label,
            FontId::monospace(11.0),
            palette::TEXT_MUTED,
        );
    }

    // 7. Vertical time grid & bottom time axis ticks ($O(\text{ticks})$ data-aligned stride)
    if let Some(strict) = shared_time_scale.visible_strict_range() {
        let target_time_labels = ((layout.plot_rect.width() / 130.0).floor() as i64).clamp(3, 10);
        let raw_stride = (strict.count().max(1) / target_time_labels).max(1);
        let stride = nice_bar_stride(raw_stride);
        let first_idx = strict.left.max(0).div_euclid(stride) * stride;
        let max_data_idx = (candles.len() as i64).saturating_sub(1);

        let mut idx = first_idx;
        while idx <= strict.right.min(max_data_idx) {
            if idx >= 0
                && let Some(sx) = main_vp.index_to_screen_x(idx)
            {
                let x = sx as f32;
                if x >= layout.plot_rect.left() && x <= layout.plot_rect.right() {
                    if config.show_grid {
                        painter.line_segment(
                            [
                                Pos2::new(x, layout.plot_rect.top()),
                                Pos2::new(x, layout.plot_rect.bottom()),
                            ],
                            Stroke::new(1.0, palette::GRID_LINE),
                        );
                    }

                    if x >= layout.time_axis_rect.left() + 34.0
                        && x <= layout.time_axis_rect.right() - 34.0
                    {
                        painter.line_segment(
                            [
                                Pos2::new(x, layout.time_axis_rect.top()),
                                Pos2::new(x, layout.time_axis_rect.top() + 5.0),
                            ],
                            Stroke::new(1.0, palette::BORDER_SUBTLE),
                        );
                        painter.text(
                            Pos2::new(x, layout.time_axis_rect.center().y),
                            Align2::CENTER_CENTER,
                            &candles[idx as usize].date_label,
                            FontId::monospace(11.0),
                            palette::TEXT_MUTED,
                        );
                    }
                }
            }
            idx = idx.saturating_add(stride);
        }
    }

    // 8. Single-pass Candlestick + Volume rendering with O(1) slice culling & DPR-aware OHLCV conflation
    let effective_bar_spacing = shared_time_scale.bar_spacing() * (conflation_factor as f64);
    let bar_metrics = CandlestickBarMetrics::compute(effective_bar_spacing, ppp);
    let half_body_w = (bar_metrics.body_width_logical as f32) * 0.5;
    let wick_stroke_w = bar_metrics.wick_width_logical as f32;
    let min_body_h = (1.0 / (ppp as f32)).max(0.25);

    let plot_painter = painter.with_clip_rect(layout.plot_rect);
    let mut rendered_candle_count = 0_usize;
    let plot_bottom = layout.plot_rect.bottom();
    let plot_top = layout.plot_rect.top();

    let conflated_iter = iter_conflated_candles(candles, culling_range.clone(), conflation_factor);
    let vol_layer_slot = config
        .show_volume
        .then(|| plot_painter.add(egui::Shape::Noop));
    let mut vol_shapes = if config.show_volume {
        Vec::with_capacity(conflated_iter.size_hint().0)
    } else {
        Vec::new()
    };

    for bar in conflated_iter {
        let Some(sx) = main_vp.float_index_to_screen_x(bar.center_logical_index) else {
            continue;
        };
        let cx = sx as f32;
        if cx + half_body_w < layout.plot_rect.left() || cx - half_body_w > layout.plot_rect.right()
        {
            continue;
        }

        if config.show_volume {
            let vol_top_y = vol_vp
                .price_to_screen_y(bar.volume)
                .map_or(plot_bottom - 1.0, |sy| {
                    (sy as f32).clamp(plot_top, plot_bottom - 1.0)
                });
            let fill = if bar.is_bullish() {
                palette::BULLISH_VOL
            } else {
                palette::BEARISH_VOL
            };
            vol_shapes.push(egui::Shape::rect_filled(
                Rect::from_min_max(
                    Pos2::new(cx - half_body_w, vol_top_y),
                    Pos2::new(cx + half_body_w, plot_bottom),
                ),
                0.0,
                fill,
            ));
        }

        let (Some(high_sy), Some(low_sy), Some(open_sy), Some(close_sy)) = (
            main_vp.price_to_screen_y(bar.high),
            main_vp.price_to_screen_y(bar.low),
            main_vp.price_to_screen_y(bar.open),
            main_vp.price_to_screen_y(bar.close),
        ) else {
            continue;
        };

        rendered_candle_count += 1;

        let high_y = high_sy as f32;
        let low_y = low_sy as f32;
        let open_y = open_sy as f32;
        let close_y = close_sy as f32;

        let color = if bar.is_bullish() {
            palette::BULLISH
        } else {
            palette::BEARISH
        };

        let top_y = open_y.min(close_y);
        let bottom_y = open_y.max(close_y).max(top_y + min_body_h);
        // Orientation-agnostic wick extremes so wicks remain 100% intact when `invert_scale == true`!
        let wick_top_y = high_y.min(low_y);
        let wick_bottom_y = high_y.max(low_y);

        if wick_top_y < top_y {
            plot_painter.line_segment(
                [Pos2::new(cx, wick_top_y), Pos2::new(cx, top_y)],
                Stroke::new(wick_stroke_w, color),
            );
        }
        if wick_bottom_y > bottom_y {
            plot_painter.line_segment(
                [Pos2::new(cx, bottom_y), Pos2::new(cx, wick_bottom_y)],
                Stroke::new(wick_stroke_w, color),
            );
        }

        let body_rect = Rect::from_min_max(
            Pos2::new(cx - half_body_w, top_y),
            Pos2::new(cx + half_body_w, bottom_y),
        );
        plot_painter.rect_filled(body_rect, 0.0, color);
    }

    if let Some(slot) = vol_layer_slot {
        plot_painter.set(slot, egui::Shape::Vec(vol_shapes));
    }

    // 9. Latest series close reference line & right price axis pill
    if let Some(last_candle) = candles.last()
        && let Some(last_sy) = main_vp.price_to_screen_y(last_candle.close)
    {
        let last_y = last_sy as f32;
        if last_y >= layout.plot_rect.top() && last_y <= layout.plot_rect.bottom() {
            let last_color = if last_candle.is_bullish() {
                palette::BULLISH
            } else {
                palette::BEARISH
            };
            plot_painter.line_segment(
                [
                    Pos2::new(layout.plot_rect.left(), last_y),
                    Pos2::new(layout.plot_rect.right(), last_y),
                ],
                Stroke::new(1.0, last_color.gamma_multiply(0.55)),
            );

            let pill_y = last_y.clamp(
                layout.price_axis_rect.top() + 10.0,
                layout.price_axis_rect.bottom() - 10.0,
            );
            let pill_rect = Rect::from_min_max(
                Pos2::new(layout.price_axis_rect.left() + 2.0, pill_y - 10.0),
                Pos2::new(layout.price_axis_rect.right() - 4.0, pill_y + 10.0),
            );
            painter.rect_filled(pill_rect, 3.0, last_color);
            painter.text(
                Pos2::new(layout.price_axis_rect.left() + 7.0, pill_y),
                Align2::LEFT_CENTER,
                main_price_scale.format_price(last_candle.close, base_value),
                FontId::monospace(11.0),
                Color32::WHITE,
            );
        }
    }

    // 10. Hovered/inspected candle & two-axis crosshair badges
    let hover_pos = plot_resp
        .hover_pos()
        .filter(|pos| layout.plot_rect.contains(*pos));
    let hovered_slot_index = hover_pos.and_then(|pos| main_vp.screen_x_to_index(f64::from(pos.x)));
    let default_inspected_idx = culling_range
        .end
        .checked_sub(1)
        .unwrap_or(candles.len() - 1);
    let inspected_data_index = hover_pos
        .and_then(|pos| main_vp.screen_x_to_clamped_data_index(f64::from(pos.x)))
        .unwrap_or(default_inspected_idx);
    let pointer_price = hover_pos.and_then(|pos| main_vp.screen_y_to_price(f64::from(pos.y)));

    if config.show_crosshair
        && let Some(pos) = hover_pos
    {
        let slot_idx = hovered_slot_index.unwrap_or(inspected_data_index as i64);
        let cross_x = main_vp
            .index_to_screen_x(slot_idx)
            .map_or(pos.x, |sx| sx as f32)
            .clamp(layout.plot_rect.left(), layout.plot_rect.right());
        let cross_y = pos.y;

        plot_painter.line_segment(
            [
                Pos2::new(cross_x, layout.plot_rect.top()),
                Pos2::new(cross_x, layout.plot_rect.bottom()),
            ],
            Stroke::new(1.0, palette::CROSSHAIR.gamma_multiply(0.50)),
        );
        plot_painter.line_segment(
            [
                Pos2::new(layout.plot_rect.left(), cross_y),
                Pos2::new(layout.plot_rect.right(), cross_y),
            ],
            Stroke::new(1.0, palette::CROSSHAIR.gamma_multiply(0.50)),
        );

        // Right Price Axis crosshair pill
        if let Some(price_at_cursor) = pointer_price {
            let pill_y = cross_y.clamp(
                layout.price_axis_rect.top() + 10.0,
                layout.price_axis_rect.bottom() - 10.0,
            );
            let cursor_pill = Rect::from_min_max(
                Pos2::new(layout.price_axis_rect.left() + 2.0, pill_y - 9.5),
                Pos2::new(layout.price_axis_rect.right() - 4.0, pill_y + 9.5),
            );
            painter.rect_filled(cursor_pill, 3.0, palette::ACCENT_BLUE);
            painter.text(
                Pos2::new(layout.price_axis_rect.left() + 7.0, pill_y),
                Align2::LEFT_CENTER,
                main_price_scale.format_price(price_at_cursor, base_value),
                FontId::monospace(11.0),
                Color32::WHITE,
            );
        }

        // Bottom Time Axis crosshair pill (supports both historical bars and future whitespace!)
        let unit_suffix = if config.timeframe.eq_ignore_ascii_case("1D") {
            "d"
        } else {
            "b"
        };
        let time_label = if slot_idx >= 0 && (slot_idx as usize) < candles.len() {
            candles[slot_idx as usize].date_label.clone()
        } else if slot_idx >= candles.len() as i64 {
            let ahead = slot_idx - (candles.len() as i64 - 1);
            format!("+{ahead}{unit_suffix}")
        } else {
            format!("{slot_idx}{unit_suffix}")
        };
        let pill_half_w = 42.0_f32;
        let pill_cx = cross_x.clamp(
            layout.time_axis_rect.left() + pill_half_w + 2.0,
            layout.time_axis_rect.right() - pill_half_w - 2.0,
        );
        let time_pill = Rect::from_min_max(
            Pos2::new(pill_cx - pill_half_w, layout.time_axis_rect.top() + 3.0),
            Pos2::new(pill_cx + pill_half_w, layout.time_axis_rect.bottom() - 3.0),
        );
        painter.rect_filled(time_pill, 3.0, palette::ACCENT_BLUE);
        painter.text(
            time_pill.center(),
            Align2::CENTER_CENTER,
            time_label,
            FontId::monospace(11.0),
            Color32::WHITE,
        );
    }

    // 11. Axis separator lines & outer border
    painter.line_segment(
        [
            Pos2::new(layout.plot_rect.right(), layout.outer_rect.top()),
            Pos2::new(layout.plot_rect.right(), layout.outer_rect.bottom()),
        ],
        Stroke::new(1.0, palette::BORDER_SUBTLE),
    );
    painter.line_segment(
        [
            Pos2::new(layout.outer_rect.left(), layout.plot_rect.bottom()),
            Pos2::new(layout.outer_rect.right(), layout.plot_rect.bottom()),
        ],
        Stroke::new(1.0, palette::BORDER_SUBTLE),
    );
    painter.rect_stroke(
        layout.outer_rect,
        0.0,
        Stroke::new(1.0, palette::BORDER_SUBTLE),
        StrokeKind::Inside,
    );

    // 12. Top-left TradingView-style OHLCV Legend HUD inside plot canvas
    // When `conflation_factor > 1`, displays the exact conflated OHLCV bucket rendered under cursor!
    let inspected_bucket = conflated_candle_at(candles, inspected_data_index, conflation_factor)
        .unwrap_or_else(|| {
            let c = &candles[inspected_data_index];
            crate::viewport::ConflatedCandle {
                start_index: inspected_data_index,
                end_index: inspected_data_index,
                center_logical_index: inspected_data_index as f64,
                timestamp: c.timestamp,
                date_label: &c.date_label,
                open: c.open,
                high: c.high,
                low: c.low,
                close: c.close,
                volume: c.volume,
            }
        });
    let sign = if inspected_bucket.change() >= 0.0 {
        "+"
    } else {
        ""
    };
    let confl_badge = if conflation_factor > 1 {
        format!(" [{conflation_factor}x]")
    } else {
        String::new()
    };
    let vol_str = if inspected_bucket.volume >= 1_000_000.0 {
        format!("{:.2}M", inspected_bucket.volume / 1_000_000.0)
    } else {
        format!("{:.0}K", inspected_bucket.volume / 1_000.0)
    };
    let hud_text = format!(
        "{} · {}{confl_badge} · {}   O {:.2}   H {:.2}   L {:.2}   C {:.2}   {sign}{:.2} ({sign}{:.2}%)   Vol {vol_str}",
        config.symbol,
        config.timeframe,
        inspected_bucket.date_label,
        inspected_bucket.open,
        inspected_bucket.high,
        inspected_bucket.low,
        inspected_bucket.close,
        inspected_bucket.change(),
        inspected_bucket.change_pct(),
    );
    let value_color = if inspected_bucket.is_bullish() {
        palette::BULLISH
    } else {
        palette::BEARISH
    };
    let hud_galley = painter.layout_no_wrap(hud_text, FontId::monospace(11.5), value_color);
    let desired_hud_w = (hud_galley.size().x + 22.0).max(160.0);
    let hud_rect = Rect::from_min_size(
        Pos2::new(
            layout.plot_rect.left() + 12.0,
            layout.plot_rect.top() + 10.0,
        ),
        Vec2::new(
            desired_hud_w.min((layout.plot_rect.width() - 24.0).max(120.0)),
            28.0,
        ),
    );
    painter.rect_filled(hud_rect, 4.0, palette::BG_FLOATING_HUD);
    painter.rect_stroke(
        hud_rect,
        4.0,
        Stroke::new(1.0, palette::BORDER_SUBTLE),
        StrokeKind::Inside,
    );
    let hud_painter = painter.with_clip_rect(hud_rect);
    hud_painter.galley(
        Pos2::new(
            hud_rect.left() + 10.0,
            hud_rect.center().y - hud_galley.size().y * 0.5,
        ),
        hud_galley,
        value_color,
    );

    Some(ChartRenderReport {
        layout,
        bounds,
        inspected_data_index,
        pointer_price,
        bar_spacing: shared_time_scale.bar_spacing(),
        right_offset: shared_time_scale.right_offset(),
        conflation_factor,
        culled_bar_count: culling_range.len(),
        rendered_candle_count,
        base_value,
        kinetic_active: shared_kinetic
            .as_ref()
            .is_some_and(KineticAnimation::is_active),
    })
}

#[allow(clippy::too_many_arguments)]
fn handle_plot_interactions(
    ui: &Ui,
    plot_resp: &egui::Response,
    plot_rect: PlotRect,
    smooth_scroll: Vec2,
    time_ms: f64,
    time_scale: &mut TimeScale,
    kinetic: &mut Option<KineticAnimation>,
    main_price_scale: &mut PriceScale,
    volume_price_scale: &mut PriceScale,
) {
    if plot_resp.hovered() && !plot_resp.dragged() {
        ui.ctx().set_cursor_icon(CursorIcon::Crosshair);

        // Horizontal trackpad two-finger pan vs cursor-anchored wheel zoom
        if smooth_scroll.x.abs() > 0.05 && smooth_scroll.x.abs() > smooth_scroll.y.abs() * 1.15 {
            *kinetic = None;
            time_scale.scroll_by_pixels(f64::from(smooth_scroll.x));
        } else if smooth_scroll.y.abs() > 0.05 {
            *kinetic = None;
            let anchor_local_x = plot_resp.hover_pos().map_or(plot_rect.width * 0.5, |pos| {
                f64::from(pos.x) - plot_rect.left
            });
            let scale = (f64::from(smooth_scroll.y) / 36.0).clamp(-3.0, 3.0);
            time_scale.zoom(anchor_local_x, scale);
        }
    }

    if plot_resp.double_clicked() {
        *kinetic = None;
        time_scale.restore_default();
        main_price_scale.reset_auto_scale();
        volume_price_scale.reset_auto_scale();
        return;
    }

    if plot_resp.drag_started() {
        *kinetic = None;
        if let Some(pos) = plot_resp.interact_pointer_pos() {
            let local_x = f64::from(pos.x) - plot_rect.left;
            let local_y = f64::from(pos.y) - plot_rect.top;
            time_scale.start_scroll(local_x);

            let mut ka = KineticAnimation::for_bar_spacing(
                time_scale.bar_spacing(),
                KineticScrollConfig::default(),
            );
            ka.add_position(time_scale.right_offset(), time_ms);
            *kinetic = Some(ka);

            if !main_price_scale.is_auto_scale() {
                main_price_scale.start_scroll(local_y);
            }
        }
    }

    if plot_resp.dragged() {
        ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
        if let Some(pos) = plot_resp.interact_pointer_pos() {
            let local_x = f64::from(pos.x) - plot_rect.left;
            let local_y = f64::from(pos.y) - plot_rect.top;
            time_scale.scroll_to(local_x);

            if let Some(ka) = kinetic.as_mut() {
                ka.add_position(time_scale.right_offset(), time_ms);
            }
            if !main_price_scale.is_auto_scale() {
                main_price_scale.scroll_to(local_y);
            }
        }
    }

    if plot_resp.drag_stopped() {
        time_scale.end_scroll();
        main_price_scale.end_scroll();
        if let Some(ka) = kinetic.as_mut() {
            if ka.start(time_scale.right_offset(), time_ms) {
                ui.ctx().request_repaint();
            } else {
                *kinetic = None;
            }
        }
    }
}

fn handle_price_axis_interactions(
    ui: &Ui,
    price_resp: &egui::Response,
    plot_rect: PlotRect,
    main_price_scale: &mut PriceScale,
) {
    if price_resp.hovered() || price_resp.dragged() {
        ui.ctx().set_cursor_icon(CursorIcon::ResizeVertical);
    }

    if price_resp.double_clicked() {
        main_price_scale.reset_auto_scale();
        return;
    }

    if price_resp.drag_started()
        && let Some(pos) = price_resp.interact_pointer_pos()
    {
        main_price_scale.start_scale(f64::from(pos.y) - plot_rect.top);
    }

    if price_resp.dragged()
        && let Some(pos) = price_resp.interact_pointer_pos()
    {
        main_price_scale.scale_to(f64::from(pos.y) - plot_rect.top);
    }

    if price_resp.drag_stopped() {
        main_price_scale.end_scale();
    }
}

fn handle_time_axis_interactions(
    ui: &Ui,
    time_resp: &egui::Response,
    plot_rect: PlotRect,
    time_scale: &mut TimeScale,
    kinetic: &mut Option<KineticAnimation>,
) {
    if time_resp.hovered() || time_resp.dragged() {
        ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
    }

    if time_resp.double_clicked() {
        *kinetic = None;
        time_scale.restore_default();
        return;
    }

    if time_resp.drag_started() {
        *kinetic = None;
        if let Some(pos) = time_resp.interact_pointer_pos() {
            time_scale.start_scale(f64::from(pos.x) - plot_rect.left);
        }
    }

    if time_resp.dragged()
        && let Some(pos) = time_resp.interact_pointer_pos()
    {
        time_scale.scale_to(f64::from(pos.x) - plot_rect.left);
    }

    if time_resp.drag_stopped() {
        time_scale.end_scale();
    }
}

fn refresh_pane_autoscale_and_bounds(
    candles: &[Candle],
    time_scale: &TimeScale,
    main_price_scale: &mut PriceScale,
    volume_price_scale: &mut PriceScale,
    show_volume: bool,
    conflation_factor: usize,
) -> Option<(f64, PriceBounds)> {
    let strict_range =
        non_empty_candle_range(time_scale.strict_visible_data_range(), candles.len());
    let visible_slice = &candles[strict_range.clone()];
    let base_value = visible_slice
        .first()
        .map_or_else(|| candles[0].close, |c| c.close);

    let mut raw_min = f64::INFINITY;
    let mut raw_max = f64::NEG_INFINITY;
    let mut max_vol = 0.0_f64;

    // Single pass across the rendered conflated (or 1:1) visible candles:
    // - Eliminates redundant O(N) scans at 100K bars
    // - Keeps volume autoscale 100% consistent with conflated volume bar heights!
    for bar in iter_conflated_candles(candles, strict_range, conflation_factor) {
        if bar.low.is_finite() {
            raw_min = raw_min.min(bar.low);
        }
        if bar.high.is_finite() {
            raw_max = raw_max.max(bar.high);
        }
        if bar.volume.is_finite() {
            max_vol = max_vol.max(bar.volume);
        }
    }

    main_price_scale.autoscale_raw_bounds(raw_min, raw_max, base_value);
    if show_volume {
        volume_price_scale.autoscale_raw_bounds(0.0, max_vol, 1.0);
    }

    let bounds = PriceBounds::from_raw_extents(raw_min, raw_max, max_vol, 0.08)
        .or_else(|| PriceBounds::from_candles(&candles[candles.len() - 1..], 0.08))?;
    Some((base_value, bounds))
}

fn non_empty_candle_range(range: std::ops::Range<usize>, total: usize) -> std::ops::Range<usize> {
    if total == 0 {
        0..0
    } else if range.is_empty() {
        (total - 1)..total
    } else {
        range
    }
}

fn nice_bar_stride(raw_stride: i64) -> i64 {
    const STEPS: [i64; 18] = [
        1, 2, 5, 10, 15, 20, 25, 50, 100, 200, 250, 500, 1_000, 2_000, 2_500, 5_000, 10_000, 20_000,
    ];
    for &s in &STEPS {
        if s >= raw_stride {
            return s;
        }
    }
    raw_stride.max(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::generate_synthetic_ohlcv;

    #[test]
    fn chart_layout_resizes_cleanly_and_keeps_axes_aligned() {
        for (w, h) in [(1280.0, 800.0), (1600.0, 960.0), (640.0, 420.0)] {
            let outer = Rect::from_min_size(Pos2::new(0.0, 36.0), Vec2::new(w, h));
            let layout = ChartLayout::from_outer_rect(outer);

            assert!((layout.plot_rect.right() - layout.price_axis_rect.left()).abs() < 1e-4);
            assert!((layout.plot_rect.bottom() - layout.time_axis_rect.top()).abs() < 1e-4);
            assert!((layout.time_axis_rect.right() - layout.price_axis_rect.left()).abs() < 1e-4);
            assert!(layout.plot_rect.width() > 100.0);
            assert!(layout.plot_rect.height() > 100.0);
        }
    }

    #[test]
    fn headless_chart_render_pass_produces_valid_telemetry_at_120_and_100k_bars() {
        for count in [120_usize, 100_000_usize] {
            let candles = generate_synthetic_ohlcv(count);
            let mut viewport = ChartViewportState::new();
            let ctx = egui::Context::default();
            let mut report_out = None;

            let raw_input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 800.0))),
                ..Default::default()
            };

            let mut out = ctx.run_ui(raw_input, |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    report_out = render_candlestick_chart(
                        ui,
                        &candles,
                        &mut viewport,
                        ChartDisplayConfig {
                            symbol: "VNINDEX",
                            timeframe: "1D",
                            show_grid: true,
                            show_crosshair: true,
                            show_volume: true,
                        },
                    );
                });
            });
            out.textures_delta.clear();

            let rep = report_out.expect("chart render report must be produced");
            assert!(rep.culled_bar_count > 0 && rep.culled_bar_count <= 200);
            assert!(rep.rendered_candle_count > 0);
            assert!((rep.right_offset - 12.0).abs() < 1e-6);
            assert!((rep.bar_spacing - 8.0).abs() < 1e-6);
        }
    }

    #[test]
    fn headless_ui_hover_wheel_zoom_and_drag_pan_interactions() {
        let candles = generate_synthetic_ohlcv(500);
        let mut viewport = ChartViewportState::new();
        let ctx = egui::Context::default();
        let screen_rect = Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 800.0)));
        let cfg = ChartDisplayConfig {
            symbol: "VNINDEX",
            timeframe: "1D",
            show_grid: true,
            show_crosshair: true,
            show_volume: true,
        };

        // Frame 1: Establish widget rects
        let mut rep1 = None;
        let mut out1 = ctx.run_ui(
            egui::RawInput {
                screen_rect,
                time: Some(1.0),
                ..Default::default()
            },
            |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    rep1 = render_candlestick_chart(ui, &candles, &mut viewport, cfg);
                });
            },
        );
        out1.textures_delta.clear();
        let r1 = rep1.unwrap();

        // Frame 2: Hover pointer inside plot canvas + wheel zoom in around cursor
        let cursor_pos = Pos2::new(
            r1.layout.plot_rect.left() + r1.layout.plot_rect.width() * 0.60,
            r1.layout.plot_rect.center().y,
        );
        let anchor_local_x = f64::from(cursor_pos.x - r1.layout.plot_rect.left());
        let before_float_idx = viewport
            .time_scale
            .local_x_to_float_index(anchor_local_x)
            .unwrap();

        let mut rep2 = None;
        let mut out2 = ctx.run_ui(
            egui::RawInput {
                screen_rect,
                time: Some(1.016),
                events: vec![
                    egui::Event::PointerMoved(cursor_pos),
                    egui::Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Point,
                        delta: Vec2::new(0.0, 72.0),
                        phase: egui::TouchPhase::Move,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                ..Default::default()
            },
            |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    rep2 = render_candlestick_chart(ui, &candles, &mut viewport, cfg);
                });
            },
        );
        out2.textures_delta.clear();
        let r2 = rep2.unwrap();
        assert!(
            r2.pointer_price.is_some(),
            "crosshair price must resolve under pointer"
        );
        assert!(r2.bar_spacing > r1.bar_spacing, "wheel scroll must zoom in");
        let after_float_idx = viewport
            .time_scale
            .local_x_to_float_index(anchor_local_x)
            .unwrap();
        assert!(
            (after_float_idx - before_float_idx).abs() <= 2e-5,
            "UI wheel zoom must preserve exact bar under cursor"
        );

        // Frames 3..5: Press, drag right by 120px, release -> right_offset shifts into history
        let ro_before_drag = viewport.time_scale.right_offset();
        let drag_end = Pos2::new(cursor_pos.x + 120.0, cursor_pos.y);

        for (t, events) in [
            (
                1.032,
                vec![
                    egui::Event::PointerMoved(cursor_pos),
                    egui::Event::PointerButton {
                        pos: cursor_pos,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            ),
            (1.048, vec![egui::Event::PointerMoved(drag_end)]),
            (
                1.064,
                vec![egui::Event::PointerButton {
                    pos: drag_end,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            ),
        ] {
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect,
                    time: Some(t),
                    events,
                    ..Default::default()
                },
                |ui| {
                    egui::CentralPanel::default().show(ui, |ui| {
                        let _ = render_candlestick_chart(ui, &candles, &mut viewport, cfg);
                    });
                },
            );
            out.textures_delta.clear();
        }
        assert!(
            viewport.time_scale.right_offset() < ro_before_drag,
            "dragging right must scroll right_offset backward into history"
        );
    }

    #[test]
    fn headless_inverted_scale_preserves_candlestick_wick_segments() {
        let candles = vec![Candle {
            timestamp: 1_750_000_000,
            date_label: "2026-06-15".to_owned(),
            open: 1200.0,
            high: 1300.0,
            low: 1100.0,
            close: 1220.0,
            volume: 50_000.0,
        }];
        let cfg = ChartDisplayConfig {
            symbol: "VNINDEX",
            timeframe: "1D",
            show_grid: false,
            show_crosshair: false,
            show_volume: false,
        };
        let screen_rect = Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0)));

        let mut shape_counts = [0_usize; 2];
        for (idx, inverted) in [false, true].into_iter().enumerate() {
            let ctx = egui::Context::default();
            let mut vp = ChartViewportState::new();
            vp.main_price_scale.set_inverted(inverted);
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect,
                    ..Default::default()
                },
                |ui| {
                    egui::CentralPanel::default().show(ui, |ui| {
                        let _ = render_candlestick_chart(ui, &candles, &mut vp, cfg);
                    });
                },
            );
            shape_counts[idx] = out.shapes.len();
            out.textures_delta.clear();
        }
        assert_eq!(
            shape_counts[0], shape_counts[1],
            "inverted scale must emit the exact same upper & lower wick shapes as normal scale"
        );
    }
}
