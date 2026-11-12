use egui::epaint::Mesh;
use egui::text::LayoutJob;
use egui::{
    Align2, Color32, CursorIcon, FontId, Key, Pos2, Rect, Sense, Shape, Stroke, StrokeKind,
    TextFormat, Ui, Vec2,
};
use serde::{Deserialize, Serialize};

use crate::drawings::{
    Drawing, DrawingColorPreset, DrawingGeometry, DrawingId, DrawingInteractionPhase,
    DrawingPaneTarget, DrawingStore, DrawingTool, HandlePart, HitTarget, LineDashStyle, LocalPoint,
    MagnetMode, PaneDrawingProjection, PointerCaptureOutcome, ProjectedDrawingShape,
};
use crate::indicators::{
    BucketLineEnvelope, IndicatorCache, IndicatorConfig, MACD_REFERENCE_LEVELS, OscillatorPaneId,
    RSI_REFERENCE_LEVELS, SeriesBlockIndex, bucket_line_envelope_indexed,
    visit_conflated_histogram_bars_indexed,
};
use crate::model::{Candle, PriceBounds};
use crate::trading::{ExecutionFill, OrderSide, PositionSide, TradingSession};
use crate::viewport::{
    CandlestickBarMetrics, HorzScaleOptions, KineticAnimation, KineticScrollConfig, PaneViewport,
    PlotRect, PriceScale, PriceScaleOptions, TimeScale, conflated_candle_at,
    iter_conflated_candles,
};

/// TradingView-inspired dark workstation & indicator palette.
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

    // Milestone R04 — Indicator & Multi-Pane Separator Palette
    pub const IND_SMA: Color32 = Color32::from_rgb(255, 183, 77); // #FFB74D Amber
    pub const IND_EMA: Color32 = Color32::from_rgb(41, 182, 246); // #29B6F6 Sky Blue
    pub const IND_BB_BORDER: Color32 = Color32::from_rgb(77, 208, 225); // #4DD0E1 Cyan
    pub const IND_BB_MIDDLE: Color32 = Color32::from_rgb(0, 172, 193); // #00ACC1 Teal Basis
    pub const IND_BB_FILL: Color32 = Color32::from_rgba_premultiplied(8, 22, 24, 26);
    pub const IND_RSI: Color32 = Color32::from_rgb(179, 136, 255); // #B388FF Violet
    pub const IND_RSI_BAND_FILL: Color32 = Color32::from_rgba_premultiplied(18, 14, 26, 26);
    pub const IND_MACD: Color32 = Color32::from_rgb(88, 166, 255); // #58A6FF Blue
    pub const IND_MACD_SIGNAL: Color32 = Color32::from_rgb(255, 138, 0); // #FF8A00 Orange
    pub const IND_MACD_HIST_POS_UP: Color32 = Color32::from_rgb(38, 166, 154);
    pub const IND_MACD_HIST_POS_DOWN: Color32 = Color32::from_rgb(24, 98, 92);
    pub const IND_MACD_HIST_NEG_DOWN: Color32 = Color32::from_rgb(239, 83, 80);
    pub const IND_MACD_HIST_NEG_UP: Color32 = Color32::from_rgb(132, 42, 44);
    pub const IND_ATR: Color32 = Color32::from_rgb(224, 64, 251); // #E040FB Magenta
    pub const SEPARATOR_BG: Color32 = Color32::from_rgb(26, 31, 44);
    pub const SEPARATOR_ACTIVE: Color32 = Color32::from_rgb(66, 118, 255);

    // Milestone R07 — Causal Bar Replay & Paper Trading Overlay Palette
    pub const REPLAY_CUT_GOLD: Color32 = Color32::from_rgb(255, 179, 0); // #FFB300
    pub const REPLAY_MASK_CURTAIN: Color32 = Color32::from_rgba_premultiplied(5, 7, 11, 152);
    pub const TRADE_ENTRY_BLUE: Color32 = Color32::from_rgb(88, 166, 255); // #58A6FF
    pub const TRADE_STOP_LOSS: Color32 = Color32::from_rgb(255, 23, 68); // #FF1744
    pub const TRADE_TAKE_PROFIT: Color32 = Color32::from_rgb(0, 230, 118); // #00E676
    pub const MARKER_BUY_ARROW: Color32 = Color32::from_rgb(0, 230, 118); // #00E676
    pub const MARKER_SELL_ARROW: Color32 = Color32::from_rgb(255, 82, 82); // #FF5252
}

/// Layout regions computed dynamically from the available chart area on every frame (single-pane view).
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

        let split_x = (outer_rect.right() - price_axis_width)
            .max(outer_rect.left() + 40.0)
            .min(outer_rect.right());
        let split_y = (outer_rect.bottom() - time_axis_height)
            .max(outer_rect.top() + 40.0)
            .min(outer_rect.bottom());

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

/// Geometry of a single stacked oscillator sub-pane (`RSI`, `MACD`, or `ATR`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SubPaneLayout {
    pub id: OscillatorPaneId,
    pub plot_rect: Rect,
    pub price_axis_rect: Rect,
}

/// Interactive horizontal resize handle between two vertically adjacent panes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaneSeparatorLayout {
    pub separator_index: usize,
    pub top_pane: Option<OscillatorPaneId>,
    pub bottom_pane: OscillatorPaneId,
    pub rect: Rect,
}

/// Stacked multi-pane workstation layout sharing a single horizontal `TimeScale` (`chart-widget.ts:507–556`).
#[derive(Debug, Clone, PartialEq)]
pub struct MultiPaneLayout {
    pub outer_rect: Rect,
    pub main_plot_rect: Rect,
    pub main_price_axis_rect: Rect,
    pub oscillators: Vec<SubPaneLayout>,
    pub separators: Vec<PaneSeparatorLayout>,
    pub time_axis_rect: Rect,
    pub corner_rect: Rect,
}

impl MultiPaneLayout {
    pub const SEPARATOR_HEIGHT: f32 = 5.0;
    pub const MIN_PANE_HEIGHT: f32 = 44.0;

    /// Computes proportional vertical heights from `stretch_factor` values with iterative
    /// minimum-height clamping (`LWC _adjustSizeImpl`) so panes never overlap or leave seam gaps.
    #[must_use]
    pub fn compute(
        outer_rect: Rect,
        main_stretch: f32,
        active_oscillators: &[(OscillatorPaneId, f32)],
    ) -> Self {
        if active_oscillators.is_empty() {
            let single = ChartLayout::from_outer_rect(outer_rect);
            return Self {
                outer_rect,
                main_plot_rect: single.plot_rect,
                main_price_axis_rect: single.price_axis_rect,
                oscillators: Vec::new(),
                separators: Vec::new(),
                time_axis_rect: single.time_axis_rect,
                corner_rect: single.corner_rect,
            };
        }

        let price_axis_width =
            ChartLayout::PRICE_AXIS_WIDTH.min((outer_rect.width() * 0.25).max(52.0));
        let time_axis_height =
            ChartLayout::TIME_AXIS_HEIGHT.min((outer_rect.height() * 0.18).max(20.0));
        let split_x = (outer_rect.right() - price_axis_width)
            .max(outer_rect.left() + 40.0)
            .min(outer_rect.right());
        let bottom_y = (outer_rect.bottom() - time_axis_height)
            .max(outer_rect.top() + 60.0)
            .min(outer_rect.bottom());

        let total_panes = 1 + active_oscillators.len();
        let total_avail_y = (bottom_y - outer_rect.top()).max(0.0);
        let sep_h = Self::SEPARATOR_HEIGHT
            .min((total_avail_y * 0.25 / (active_oscillators.len() as f32)).max(0.0));
        let total_sep_h = (active_oscillators.len() as f32) * sep_h;
        let available_pane_h = (total_avail_y - total_sep_h).max(0.0);
        let effective_min_h =
            Self::MIN_PANE_HEIGHT.min((available_pane_h / (total_panes as f32)).floor().max(0.0));

        let mut stretches = Vec::with_capacity(total_panes);
        stretches.push(sanitize_stretch(main_stretch, 4.0));
        for &(_, s) in active_oscillators {
            stretches.push(sanitize_stretch(s, 1.0));
        }

        let mut heights = vec![0.0_f32; total_panes];
        let mut locked = vec![false; total_panes];
        let mut remaining_h = available_pane_h;

        for _ in 0..total_panes {
            let free_stretch: f32 = stretches
                .iter()
                .enumerate()
                .filter(|(i, _)| !locked[*i])
                .map(|(_, &s)| s)
                .sum();
            if free_stretch <= 0.0 {
                break;
            }

            // Snapshot `pass_remaining_h` before locking panes in this pass so `proposed` shares use
            // the same numerator & denominator (`free_stretch`), preserving stretch monotonicity.
            let pass_remaining_h = remaining_h;
            let mut clamped_any = false;
            for i in 0..total_panes {
                if !locked[i] {
                    let proposed = pass_remaining_h * (stretches[i] / free_stretch);
                    if proposed < effective_min_h {
                        heights[i] = effective_min_h;
                        locked[i] = true;
                        remaining_h = (remaining_h - effective_min_h).max(0.0);
                        clamped_any = true;
                    } else {
                        heights[i] = proposed;
                    }
                }
            }
            if !clamped_any {
                break;
            }
        }

        let mut y_cursor = outer_rect.top();
        let main_bottom = (y_cursor + heights[0]).min(bottom_y);
        let main_plot_rect = Rect::from_min_max(
            Pos2::new(outer_rect.left(), y_cursor),
            Pos2::new(split_x, main_bottom),
        );
        let main_price_axis_rect = Rect::from_min_max(
            Pos2::new(split_x, y_cursor),
            Pos2::new(outer_rect.right(), main_bottom),
        );
        y_cursor = main_bottom;

        let mut oscillators = Vec::with_capacity(active_oscillators.len());
        let mut separators = Vec::with_capacity(active_oscillators.len());

        for (osc_idx, &(osc_id, _)) in active_oscillators.iter().enumerate() {
            let top_pane = if osc_idx == 0 {
                None
            } else {
                Some(active_oscillators[osc_idx - 1].0)
            };
            let sep_bottom = (y_cursor + sep_h).min(bottom_y);
            let sep_rect = Rect::from_min_max(
                Pos2::new(outer_rect.left(), y_cursor),
                Pos2::new(outer_rect.right(), sep_bottom),
            );
            separators.push(PaneSeparatorLayout {
                separator_index: osc_idx,
                top_pane,
                bottom_pane: osc_id,
                rect: sep_rect,
            });
            y_cursor = sep_bottom;

            let pane_bottom = if osc_idx + 1 == active_oscillators.len() {
                bottom_y
            } else {
                (y_cursor + heights[osc_idx + 1]).min(bottom_y)
            };
            let plot_rect = Rect::from_min_max(
                Pos2::new(outer_rect.left(), y_cursor),
                Pos2::new(split_x, pane_bottom),
            );
            let price_axis_rect = Rect::from_min_max(
                Pos2::new(split_x, y_cursor),
                Pos2::new(outer_rect.right(), pane_bottom),
            );
            oscillators.push(SubPaneLayout {
                id: osc_id,
                plot_rect,
                price_axis_rect,
            });
            y_cursor = pane_bottom;
        }

        let time_axis_rect = Rect::from_min_max(
            Pos2::new(outer_rect.left(), bottom_y),
            Pos2::new(split_x, outer_rect.bottom()),
        );
        let corner_rect = Rect::from_min_max(Pos2::new(split_x, bottom_y), outer_rect.max);

        Self {
            outer_rect,
            main_plot_rect,
            main_price_axis_rect,
            oscillators,
            separators,
            time_axis_rect,
            corner_rect,
        }
    }

    /// Projects the primary pane + shared bottom time axis into a backward-compatible `ChartLayout`.
    #[must_use]
    pub const fn as_primary_chart_layout(&self) -> ChartLayout {
        ChartLayout {
            outer_rect: self.outer_rect,
            plot_rect: self.main_plot_rect,
            price_axis_rect: self.main_price_axis_rect,
            time_axis_rect: self.time_axis_rect,
            corner_rect: self.corner_rect,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct SeparatorDragState {
    separator_index: usize,
    top_pane: Option<OscillatorPaneId>,
    bottom_pane: OscillatorPaneId,
    start_pointer_y: f32,
    start_top_stretch: f32,
    start_bottom_stretch: f32,
    combined_height_px: f32,
}

fn default_rsi_price_scale() -> PriceScale {
    PriceScale::new("rsi-right", PriceScaleOptions::sumi_oscillator())
}

fn default_macd_price_scale() -> PriceScale {
    PriceScale::new("macd-right", PriceScaleOptions::sumi_oscillator())
}

fn default_atr_price_scale() -> PriceScale {
    PriceScale::new("atr-right", PriceScaleOptions::sumi_oscillator())
}

const fn default_main_stretch() -> f32 {
    4.0
}

const fn default_sub_stretch() -> f32 {
    1.0
}

#[inline]
fn sanitize_stretch(val: f32, fallback: f32) -> f32 {
    if val.is_finite() && val >= 0.05 {
        val.min(50.0)
    } else {
        fallback
    }
}

/// Modular viewport state coordinating the shared horizontal `TimeScale` and independent per-pane
/// `PriceScale` instances (`main-right`, `volume-overlay`, `rsi-right`, `macd-right`, `atr-right`),
/// proportional multi-pane stretch ratios, and inertial kinetic scrolling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartViewportState {
    pub time_scale: TimeScale,
    pub main_price_scale: PriceScale,
    pub volume_price_scale: PriceScale,

    #[serde(default = "default_rsi_price_scale")]
    pub rsi_price_scale: PriceScale,
    #[serde(default = "default_macd_price_scale")]
    pub macd_price_scale: PriceScale,
    #[serde(default = "default_atr_price_scale")]
    pub atr_price_scale: PriceScale,

    #[serde(default = "default_main_stretch")]
    pub main_stretch: f32,
    #[serde(default = "default_sub_stretch")]
    pub rsi_stretch: f32,
    #[serde(default = "default_sub_stretch")]
    pub macd_stretch: f32,
    #[serde(default = "default_sub_stretch")]
    pub atr_stretch: f32,

    #[serde(skip)]
    pub kinetic: Option<KineticAnimation>,
    #[serde(skip)]
    separator_drag: Option<SeparatorDragState>,
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
            rsi_price_scale: default_rsi_price_scale(),
            macd_price_scale: default_macd_price_scale(),
            atr_price_scale: default_atr_price_scale(),
            main_stretch: default_main_stretch(),
            rsi_stretch: default_sub_stretch(),
            macd_stretch: default_sub_stretch(),
            atr_stretch: default_sub_stretch(),
            kinetic: None,
            separator_drag: None,
        }
    }

    /// Sanitizes state restored from persistent `serde` storage against NaN/Infinity or mode mismatches.
    pub fn sanitize_deserialized(&mut self) {
        self.kinetic = None;
        self.separator_drag = None;
        self.time_scale.sanitize_deserialized();
        self.main_price_scale.sanitize_deserialized();
        self.volume_price_scale.sanitize_deserialized();
        self.rsi_price_scale.sanitize_deserialized();
        self.macd_price_scale.sanitize_deserialized();
        self.atr_price_scale.sanitize_deserialized();
        // Oscillator sub-panes always operate in Linear vertical space (`RSI`, `MACD`, `ATR`).
        self.rsi_price_scale
            .set_mode(crate::viewport::PriceScaleMode::Normal);
        self.macd_price_scale
            .set_mode(crate::viewport::PriceScaleMode::Normal);
        self.atr_price_scale
            .set_mode(crate::viewport::PriceScaleMode::Normal);
        self.main_stretch = sanitize_stretch(self.main_stretch, default_main_stretch());
        self.rsi_stretch = sanitize_stretch(self.rsi_stretch, default_sub_stretch());
        self.macd_stretch = sanitize_stretch(self.macd_stretch, default_sub_stretch());
        self.atr_stretch = sanitize_stretch(self.atr_stretch, default_sub_stretch());
    }

    #[must_use]
    pub const fn oscillator_price_scale(&self, id: OscillatorPaneId) -> &PriceScale {
        match id {
            OscillatorPaneId::Rsi => &self.rsi_price_scale,
            OscillatorPaneId::Macd => &self.macd_price_scale,
            OscillatorPaneId::Atr => &self.atr_price_scale,
        }
    }

    pub fn oscillator_price_scale_mut(&mut self, id: OscillatorPaneId) -> &mut PriceScale {
        match id {
            OscillatorPaneId::Rsi => &mut self.rsi_price_scale,
            OscillatorPaneId::Macd => &mut self.macd_price_scale,
            OscillatorPaneId::Atr => &mut self.atr_price_scale,
        }
    }

    #[must_use]
    pub const fn oscillator_stretch(&self, id: OscillatorPaneId) -> f32 {
        match id {
            OscillatorPaneId::Rsi => self.rsi_stretch,
            OscillatorPaneId::Macd => self.macd_stretch,
            OscillatorPaneId::Atr => self.atr_stretch,
        }
    }

    pub fn set_oscillator_stretch(&mut self, id: OscillatorPaneId, stretch: f32) {
        let s = sanitize_stretch(stretch, 1.0);
        match id {
            OscillatorPaneId::Rsi => self.rsi_stretch = s,
            OscillatorPaneId::Macd => self.macd_stretch = s,
            OscillatorPaneId::Atr => self.atr_stretch = s,
        }
    }

    #[must_use]
    fn pane_stretch(&self, pane: Option<OscillatorPaneId>) -> f32 {
        match pane {
            None => self.main_stretch,
            Some(id) => self.oscillator_stretch(id),
        }
    }

    fn set_pane_stretch(&mut self, pane: Option<OscillatorPaneId>, stretch: f32) {
        match pane {
            None => self.main_stretch = sanitize_stretch(stretch, 4.0),
            Some(id) => self.set_oscillator_stretch(id, stretch),
        }
    }

    /// Resets proportional pane stretch ratios to `Main = 4.0`, `Oscillator = 1.0` (`PaneManager.ts:76–77`).
    pub fn reset_pane_stretches(&mut self) {
        self.separator_drag = None;
        self.main_stretch = default_main_stretch();
        self.rsi_stretch = default_sub_stretch();
        self.macd_stretch = default_sub_stretch();
        self.atr_stretch = default_sub_stretch();
    }

    /// Applies an exact LWC-style adjacent-pair stretch transfer (`pane-separator.ts:181–220`)
    /// so uninvolved panes retain 100% invariant heights and `MIN_PANE_HEIGHT` is enforced.
    pub fn apply_separator_drag_delta(
        &mut self,
        top_pane: Option<OscillatorPaneId>,
        bottom_pane: OscillatorPaneId,
        combined_height_px: f32,
        delta_y: f32,
    ) {
        let start_top = self.pane_stretch(top_pane);
        let start_bottom = self.oscillator_stretch(bottom_pane);
        let (new_top, new_bottom) =
            compute_adjacent_stretch_transfer(start_top, start_bottom, combined_height_px, delta_y);
        self.set_pane_stretch(top_pane, new_top);
        self.set_oscillator_stretch(bottom_pane, new_bottom);
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

    /// Fits the entire series `[0..total_bars)` plus default future `right_offset` and resets all price autoscales.
    pub fn fit_all(&mut self) {
        self.kinetic = None;
        self.time_scale.fit_content();
        self.main_price_scale.reset_auto_scale();
        self.volume_price_scale.reset_auto_scale();
        self.rsi_price_scale.reset_auto_scale();
        self.macd_price_scale.reset_auto_scale();
        self.atr_price_scale.reset_auto_scale();
    }

    /// Restores default `bar_spacing` (`8.0`), default future `right_offset` (`12.0`), and all price autoscales.
    pub fn reset_to_default(&mut self) {
        self.kinetic = None;
        self.separator_drag = None;
        self.time_scale.restore_default();
        self.main_price_scale.reset_auto_scale();
        self.volume_price_scale.reset_auto_scale();
        self.rsi_price_scale.reset_auto_scale();
        self.macd_price_scale.reset_auto_scale();
        self.atr_price_scale.reset_auto_scale();
    }
}

#[inline]
fn compute_adjacent_stretch_transfer(
    start_top_stretch: f32,
    start_bottom_stretch: f32,
    combined_height_px: f32,
    delta_y: f32,
) -> (f32, f32) {
    let pair_stretch = (start_top_stretch + start_bottom_stretch).max(0.10);
    let safe_h = combined_height_px.max(24.0);
    let px_to_stretch = pair_stretch / safe_h;
    let effective_min_h = MultiPaneLayout::MIN_PANE_HEIGHT.min(safe_h * 0.45);
    let min_stretch = effective_min_h * px_to_stretch;
    let max_stretch = (pair_stretch - min_stretch).max(min_stretch);
    let new_top = (start_top_stretch + delta_y * px_to_stretch).clamp(min_stretch, max_stretch);
    let new_bottom = (pair_stretch - new_top).clamp(min_stretch, max_stretch);
    (new_top, new_bottom)
}

/// Visual configuration passed to `render_candlestick_chart` and `render_workstation_chart`.
#[derive(Debug, Clone, Copy)]
pub struct ChartDisplayConfig<'a> {
    pub symbol: &'a str,
    pub timeframe: &'a str,
    pub show_grid: bool,
    pub show_crosshair: bool,
    pub show_volume: bool,
}

/// Optional Bar Replay & Paper Trading visual overlay state passed to [`render_workstation_chart_full`].
#[derive(Debug, Clone, Copy, Default)]
pub struct ChartReplayTradingOverlay<'a> {
    pub replay_active: bool,
    pub selecting_cut_bar: bool,
    pub replay_status_label: Option<&'a str>,
    pub trading_session: Option<&'a TradingSession>,
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
    pub pane_count: usize,
    pub active_overlay_count: usize,
    pub hovered_oscillator: Option<OscillatorPaneId>,
    pub hovered_oscillator_value: Option<f64>,
    pub first_oscillator_plot_rect: Option<Rect>,
    pub first_separator_rect: Option<Rect>,
    pub drawing_count: usize,
    pub active_drawing_tool: DrawingTool,
    pub selected_drawing_id: Option<DrawingId>,
    pub magnet_mode: MagnetMode,
    pub clicked_cut_bar_index: Option<usize>,
    pub rendered_fill_marker_count: usize,
    pub active_price_line_count: usize,
}

/// Renders the native candlestick chart, volume histogram, price/time axes, crosshair, and OHLCV overlay
/// without indicator overlays or sub-panes (backward-compatible R03 entrypoint).
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

/// Flagship Milestone R04 workstation renderer: renders the main candlestick + volume pane,
/// price overlays (`SMA`, `EMA`, `BollingerBands`), `0..=3` stacked oscillator sub-panes (`RSI`,
/// `MACD`, `ATR`) with interactive splitter resize handles, independent vertical `PriceScale` axes,
/// shared horizontal `TimeScale` navigation, and synchronized cross-pane crosshair & HUD readouts.
pub fn render_workstation_chart(
    ui: &mut Ui,
    candles: &[Candle],
    viewport: &mut ChartViewportState,
    config: ChartDisplayConfig<'_>,
    indicators: &IndicatorConfig,
    indicator_cache: &IndicatorCache,
) -> Option<ChartRenderReport> {
    render_workstation_chart_inner(
        ui,
        candles,
        viewport,
        config,
        indicators,
        indicator_cache,
        None,
        ChartReplayTradingOverlay::default(),
    )
}

/// Flagship Milestone R05 workstation renderer: extends [`render_workstation_chart`] with full
/// multi-pane interactive drawing & annotation creation, Liang-Barsky clipping, OHLC magnet
/// snapping, handle/body manipulation, rubber-band previews, and keyboard shortcuts.
pub fn render_workstation_chart_with_drawings(
    ui: &mut Ui,
    candles: &[Candle],
    viewport: &mut ChartViewportState,
    config: ChartDisplayConfig<'_>,
    indicators: &IndicatorConfig,
    indicator_cache: &IndicatorCache,
    drawings: &mut DrawingStore,
) -> Option<ChartRenderReport> {
    render_workstation_chart_inner(
        ui,
        candles,
        viewport,
        config,
        indicators,
        indicator_cache,
        Some(drawings),
        ChartReplayTradingOverlay::default(),
    )
}

/// Flagship Milestone R07 workstation renderer: extends [`render_workstation_chart_with_drawings`]
/// with interactive Bar Replay cut-scrubber selection, causal frontier rendering, Position & Working
/// Order `PriceLine`s + right-axis pills, and `O(log F + V)` LWC-style execution fill arrows.
#[allow(clippy::too_many_arguments)]
pub fn render_workstation_chart_full(
    ui: &mut Ui,
    candles: &[Candle],
    viewport: &mut ChartViewportState,
    config: ChartDisplayConfig<'_>,
    indicators: &IndicatorConfig,
    indicator_cache: &IndicatorCache,
    drawings: &mut DrawingStore,
    replay_trading: ChartReplayTradingOverlay<'_>,
) -> Option<ChartRenderReport> {
    render_workstation_chart_inner(
        ui,
        candles,
        viewport,
        config,
        indicators,
        indicator_cache,
        Some(drawings),
        replay_trading,
    )
}

#[allow(clippy::too_many_arguments)]
fn render_workstation_chart_inner(
    ui: &mut Ui,
    candles: &[Candle],
    viewport: &mut ChartViewportState,
    config: ChartDisplayConfig<'_>,
    indicators: &IndicatorConfig,
    indicator_cache: &IndicatorCache,
    mut drawings: Option<&mut DrawingStore>,
    replay_trading: ChartReplayTradingOverlay<'_>,
) -> Option<ChartRenderReport> {
    if candles.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.label("No OHLCV data loaded.");
        });
        return None;
    }

    let available_size = ui.available_size().max(Vec2::new(200.0, 140.0));
    let (outer_rect, _) = ui.allocate_exact_size(available_size, Sense::hover());

    let mut fallback_clean_indicators = indicators.clone();
    fallback_clean_indicators.sanitize_deserialized();
    let indicators = indicator_cache
        .applied_config()
        .unwrap_or(&fallback_clean_indicators);

    let active_osc_ids = indicators.active_oscillator_panes();
    let active_osc_specs: Vec<(OscillatorPaneId, f32)> = active_osc_ids
        .iter()
        .map(|&id| (id, viewport.oscillator_stretch(id)))
        .collect();

    // 1. Pre-compute initial MultiPaneLayout so separator drags can resolve current heights cleanly
    let pre_layout = MultiPaneLayout::compute(outer_rect, viewport.main_stretch, &active_osc_specs);

    let mut sep_responses = Vec::with_capacity(pre_layout.separators.len());
    for sep in &pre_layout.separators {
        let sep_id = ui
            .id()
            .with(("sumi_pane_separator", sep.top_pane, sep.bottom_pane));
        sep_responses.push(ui.interact(sep.rect, sep_id, Sense::click_and_drag()));
    }

    // Handle interactive pane separator dragging (`LWC pane-separator.ts:181–220`)
    let mut separator_active_idx: Option<usize> = None;
    let mut separator_stretched_this_frame = false;
    for (idx, (sep, resp)) in pre_layout
        .separators
        .iter()
        .zip(sep_responses.iter())
        .enumerate()
    {
        if resp.hovered() || resp.dragged() {
            ui.ctx().set_cursor_icon(CursorIcon::ResizeVertical);
            separator_active_idx = Some(idx);
        }
        if resp.double_clicked() {
            viewport.kinetic = None;
            viewport.reset_pane_stretches();
            separator_stretched_this_frame = true;
            continue;
        }
        if resp.drag_started()
            && let Some(origin) = ui
                .input(|i| i.pointer.press_origin())
                .or_else(|| resp.interact_pointer_pos())
        {
            viewport.kinetic = None;
            // Sync stored stretches to effective rendered height shares (`pre_layout`) before starting
            // drag so a previously `MIN_PANE_HEIGHT`-clamped pane never causes an initial drag jump!
            let total_actual_h: f32 = pre_layout.main_plot_rect.height()
                + pre_layout
                    .oscillators
                    .iter()
                    .map(|o| o.plot_rect.height())
                    .sum::<f32>();
            if total_actual_h > 1.0 {
                let total_stretch_sum: f32 = sanitize_stretch(viewport.main_stretch, 4.0)
                    + active_osc_ids
                        .iter()
                        .map(|&id| sanitize_stretch(viewport.oscillator_stretch(id), 1.0))
                        .sum::<f32>();
                viewport.main_stretch = sanitize_stretch(
                    (pre_layout.main_plot_rect.height() / total_actual_h) * total_stretch_sum,
                    4.0,
                );
                for osc in &pre_layout.oscillators {
                    let eff_s = (osc.plot_rect.height() / total_actual_h) * total_stretch_sum;
                    viewport.set_oscillator_stretch(osc.id, eff_s);
                }
            }

            let top_h = match sep.top_pane {
                None => pre_layout.main_plot_rect.height(),
                Some(top_id) => pre_layout
                    .oscillators
                    .iter()
                    .find(|o| o.id == top_id)
                    .map_or(80.0, |o| o.plot_rect.height()),
            };
            let bot_h = pre_layout
                .oscillators
                .iter()
                .find(|o| o.id == sep.bottom_pane)
                .map_or(80.0, |o| o.plot_rect.height());
            viewport.separator_drag = Some(SeparatorDragState {
                separator_index: idx,
                top_pane: sep.top_pane,
                bottom_pane: sep.bottom_pane,
                start_pointer_y: origin.y,
                start_top_stretch: viewport.pane_stretch(sep.top_pane),
                start_bottom_stretch: viewport.oscillator_stretch(sep.bottom_pane),
                combined_height_px: top_h + bot_h,
            });
        }
        if resp.dragged()
            && let Some(pos) = resp.interact_pointer_pos()
            && let Some(drag) = viewport.separator_drag
            && drag.separator_index == idx
            && drag.top_pane == sep.top_pane
            && drag.bottom_pane == sep.bottom_pane
        {
            let (new_top, new_bot) = compute_adjacent_stretch_transfer(
                drag.start_top_stretch,
                drag.start_bottom_stretch,
                drag.combined_height_px,
                pos.y - drag.start_pointer_y,
            );
            viewport.set_pane_stretch(drag.top_pane, new_top);
            viewport.set_oscillator_stretch(drag.bottom_pane, new_bot);
            separator_stretched_this_frame = true;
        }
        if resp.drag_stopped() {
            viewport.separator_drag = None;
        }
    }

    let multi_layout = if separator_stretched_this_frame {
        let updated_specs: Vec<(OscillatorPaneId, f32)> = active_osc_ids
            .iter()
            .map(|&id| (id, viewport.oscillator_stretch(id)))
            .collect();
        MultiPaneLayout::compute(outer_rect, viewport.main_stretch, &updated_specs)
    } else {
        pre_layout
    };

    let painter = ui.painter_at(outer_rect);

    // 2. Build interactive responses for Main Pane, Oscillator Sub-Panes, and shared Time Axis
    let main_plot_resp = ui.interact(
        multi_layout.main_plot_rect,
        ui.id().with("sumi_plot_canvas"),
        Sense::click_and_drag(),
    );
    let main_price_resp = ui.interact(
        multi_layout.main_price_axis_rect,
        ui.id().with("sumi_price_axis"),
        Sense::click_and_drag(),
    );
    let time_resp = ui.interact(
        multi_layout.time_axis_rect,
        ui.id().with("sumi_time_axis"),
        Sense::click_and_drag(),
    );

    let mut osc_responses = Vec::with_capacity(multi_layout.oscillators.len());
    for osc in &multi_layout.oscillators {
        let p_resp = ui.interact(
            osc.plot_rect,
            ui.id().with(("sumi_osc_plot", osc.id.key())),
            Sense::click_and_drag(),
        );
        let ax_resp = ui.interact(
            osc.price_axis_rect,
            ui.id().with(("sumi_osc_axis", osc.id.key())),
            Sense::click_and_drag(),
        );
        osc_responses.push((osc.id, osc.plot_rect, osc.price_axis_rect, p_resp, ax_resp));
    }

    // 3. Synchronize shared TimeScale width & per-pane PriceScale heights
    let ppp = f64::from(ui.ctx().pixels_per_point()).max(1.0);
    let main_plot_f64 = PlotRect::from_egui(multi_layout.main_plot_rect);
    viewport.time_scale.set_total_bars(candles.len());
    viewport.time_scale.set_width(main_plot_f64.width);
    viewport.main_price_scale.set_height(main_plot_f64.height);
    viewport.volume_price_scale.set_height(main_plot_f64.height);

    for osc in &multi_layout.oscillators {
        viewport
            .oscillator_price_scale_mut(osc.id)
            .set_height(f64::from(osc.plot_rect.height()));
    }

    let desired_bottom_margin = if config.show_volume { 0.20 } else { 0.10 };
    if (viewport.main_price_scale.options().scale_margin_bottom - desired_bottom_margin).abs()
        > 1e-6
    {
        let top_m = viewport.main_price_scale.options().scale_margin_top;
        viewport
            .main_price_scale
            .set_margins(top_m, desired_bottom_margin);
    }

    // 4. Advance shared kinetic inertia once per frame BEFORE pre-input autoscale & drawing projection
    // so inertial scrolling and `PaneDrawingProjection` stay 100% same-frame synchronized!
    let (smooth_scroll, time_ms) = ui.input(|i| (i.smooth_scroll_delta, i.time * 1000.0));
    let any_pointer_down = main_plot_resp.is_pointer_button_down_on()
        || main_price_resp.is_pointer_button_down_on()
        || time_resp.is_pointer_button_down_on()
        || sep_responses
            .iter()
            .any(egui::Response::is_pointer_button_down_on)
        || osc_responses.iter().any(|(_, _, _, pr, ar)| {
            pr.is_pointer_button_down_on() || ar.is_pointer_button_down_on()
        });
    let any_plot_dragged =
        main_plot_resp.dragged() || osc_responses.iter().any(|(_, _, _, pr, _)| pr.dragged());

    if any_pointer_down {
        viewport.kinetic = None;
    } else if !any_plot_dragged {
        step_shared_kinetic_inertia(ui, time_ms, &mut viewport.time_scale, &mut viewport.kinetic);
    }

    // Ensure current price ranges & `pre_base_value` are up to date BEFORE drawing input arbitrage
    // so `PaneDrawingProjection` maps pointer clicks/drags accurately even right after toolbar or kinetic changes.
    let init_cf = viewport.time_scale.conflation_factor(ppp);
    let init_strict = non_empty_candle_range(
        viewport.time_scale.strict_visible_data_range(),
        candles.len(),
    );
    let pre_base_value = if viewport.main_price_scale.price_range().is_none()
        || viewport.main_price_scale.is_auto_scale()
    {
        refresh_main_pane_autoscale_and_bounds(
            candles,
            &viewport.time_scale,
            &mut viewport.main_price_scale,
            &mut viewport.volume_price_scale,
            config.show_volume,
            init_cf,
            Some((indicators, indicator_cache)),
        )
        .map_or_else(|| candles[init_strict.start].close, |(bv, _)| bv)
    } else {
        candles[init_strict.start].close
    };
    for osc in &multi_layout.oscillators {
        if viewport
            .oscillator_price_scale(osc.id)
            .price_range()
            .is_none()
            || viewport.oscillator_price_scale(osc.id).is_auto_scale()
        {
            refresh_oscillator_autoscale(
                osc.id,
                viewport.oscillator_price_scale_mut(osc.id),
                indicator_cache,
                init_strict.clone(),
            );
        }
    }

    // 4b. Process Drawing Engine keyboard shortcuts & per-pane pointer arbitrage (`Milestone R05`)
    let separator_busy = separator_active_idx.is_some_and(|idx| {
        sep_responses[idx].dragged() || sep_responses[idx].is_pointer_button_down_on()
    });
    let mut suppress_main_drag_pan = false;
    let mut suppress_osc_drag_pan = vec![false; osc_responses.len()];
    let mut drawing_blocked_double_click = false;

    if let Some(store) = drawings.as_deref_mut() {
        sanitize_hidden_oscillator_drawing_state(store, &active_osc_ids);
        if replay_trading.selecting_cut_bar {
            store.clear_hover_for_pane(DrawingPaneTarget::Main);
            for &id in &active_osc_ids {
                store.clear_hover_for_pane(DrawingPaneTarget::Oscillator(id));
            }
            drawing_blocked_double_click = true;
        } else {
            handle_drawing_keyboard_shortcuts(ui, store);

            let ptr = sample_drawing_pointer_input(ui);
            drawing_blocked_double_click = store.active_tool != DrawingTool::Select
                || !matches!(store.phase(), DrawingInteractionPhase::Idle);

            if !separator_busy {
                let main_proj = PaneDrawingProjection::new(
                    &viewport.time_scale,
                    &viewport.main_price_scale,
                    main_plot_f64.width,
                    main_plot_f64.height,
                    pre_base_value,
                );
                suppress_main_drag_pan = drive_pane_drawing_input(
                    store,
                    DrawingPaneTarget::Main,
                    multi_layout.main_plot_rect,
                    main_plot_f64,
                    &main_plot_resp,
                    ptr,
                    &main_proj,
                    candles,
                );
                if suppress_main_drag_pan || store.hovered_hit().is_some() {
                    drawing_blocked_double_click = true;
                }
                if suppress_main_drag_pan {
                    viewport.kinetic = None;
                    viewport.time_scale.end_scroll();
                    viewport.main_price_scale.end_scroll();
                }

                for (osc_idx, &(osc_id, osc_plot_rect, _, ref osc_plot_resp, _)) in
                    osc_responses.iter().enumerate()
                {
                    let osc_plot_f64 = PlotRect::from_egui(osc_plot_rect);
                    let osc_proj = PaneDrawingProjection::new(
                        &viewport.time_scale,
                        viewport.oscillator_price_scale(osc_id),
                        osc_plot_f64.width,
                        osc_plot_f64.height,
                        1.0,
                    );
                    let captured = drive_pane_drawing_input(
                        store,
                        DrawingPaneTarget::Oscillator(osc_id),
                        osc_plot_rect,
                        osc_plot_f64,
                        osc_plot_resp,
                        ptr,
                        &osc_proj,
                        candles,
                    );
                    suppress_osc_drag_pan[osc_idx] = captured;
                    if captured || store.hovered_hit().is_some() {
                        drawing_blocked_double_click = true;
                    }
                    if captured {
                        viewport.kinetic = None;
                        viewport.time_scale.end_scroll();
                        viewport.oscillator_price_scale_mut(osc_id).end_scroll();
                    }
                }
            }
        }
    } else if replay_trading.selecting_cut_bar {
        drawing_blocked_double_click = true;
    }

    // 5. Process Main Pane, Oscillator Sub-Panes, Right Price Axes, and Shared Time Axis gestures
    // Double-clicking ANY plot canvas resets `time_scale` AND all pane `PriceScale`s synchronously
    // (unless a drawing placement, drag, or hover hit claimed the gesture).
    let any_plot_double_clicked = !drawing_blocked_double_click
        && (main_plot_resp.double_clicked()
            || osc_responses
                .iter()
                .any(|(_, _, _, pr, _)| pr.double_clicked()));
    if any_plot_double_clicked {
        viewport.reset_to_default();
    }

    handle_plot_interactions(
        ui,
        &main_plot_resp,
        main_plot_f64,
        smooth_scroll,
        time_ms,
        &mut viewport.time_scale,
        &mut viewport.kinetic,
        &mut viewport.main_price_scale,
        Some(&mut viewport.volume_price_scale),
        false,
        suppress_main_drag_pan,
    );
    handle_price_axis_interactions(
        ui,
        &main_price_resp,
        main_plot_f64,
        &mut viewport.main_price_scale,
    );

    for (osc_idx, &(osc_id, osc_plot_rect, _, ref osc_plot_resp, ref osc_ax_resp)) in
        osc_responses.iter().enumerate()
    {
        let osc_plot_f64 = PlotRect::from_egui(osc_plot_rect);
        {
            let ChartViewportState {
                time_scale,
                kinetic,
                rsi_price_scale,
                macd_price_scale,
                atr_price_scale,
                ..
            } = viewport;
            let osc_ps = match osc_id {
                OscillatorPaneId::Rsi => rsi_price_scale,
                OscillatorPaneId::Macd => macd_price_scale,
                OscillatorPaneId::Atr => atr_price_scale,
            };
            handle_plot_interactions(
                ui,
                osc_plot_resp,
                osc_plot_f64,
                smooth_scroll,
                time_ms,
                time_scale,
                kinetic,
                osc_ps,
                None,
                false,
                suppress_osc_drag_pan[osc_idx],
            );
            handle_price_axis_interactions(ui, osc_ax_resp, osc_plot_f64, osc_ps);
        }
    }

    if let Some(store) = drawings.as_deref()
        && !separator_busy
    {
        apply_drawing_cursor_feedback(ui, store);
    }

    handle_time_axis_interactions(
        ui,
        &time_resp,
        main_plot_f64,
        &mut viewport.time_scale,
        &mut viewport.kinetic,
    );

    // 6. Refresh visible range, dynamic `base_value`, main autoscale + overlay bounds, and oscillator autoscales
    let conflation_factor = viewport.time_scale.conflation_factor(ppp);
    let (base_value, bounds) = refresh_main_pane_autoscale_and_bounds(
        candles,
        &viewport.time_scale,
        &mut viewport.main_price_scale,
        &mut viewport.volume_price_scale,
        config.show_volume,
        conflation_factor,
        Some((indicators, indicator_cache)),
    )?;
    let strict_range = non_empty_candle_range(
        viewport.time_scale.strict_visible_data_range(),
        candles.len(),
    );
    for osc in &multi_layout.oscillators {
        refresh_oscillator_autoscale(
            osc.id,
            viewport.oscillator_price_scale_mut(osc.id),
            indicator_cache,
            strict_range.clone(),
        );
    }
    let culling_range =
        non_empty_candle_range(viewport.time_scale.culling_data_range(), candles.len());

    // 7. Paint backgrounds across all panes, axes, and separators
    painter.rect_filled(multi_layout.main_plot_rect, 0.0, palette::BG_CHART_CANVAS);
    painter.rect_filled(
        multi_layout.main_price_axis_rect,
        0.0,
        palette::BG_AXIS_GUTTER,
    );
    for osc in &multi_layout.oscillators {
        painter.rect_filled(osc.plot_rect, 0.0, palette::BG_CHART_CANVAS);
        painter.rect_filled(osc.price_axis_rect, 0.0, palette::BG_AXIS_GUTTER);
    }
    painter.rect_filled(multi_layout.time_axis_rect, 0.0, palette::BG_AXIS_GUTTER);
    painter.rect_filled(multi_layout.corner_rect, 0.0, palette::BG_WORKSTATION);

    // 8. Compute vertical time grid X coordinates once and share across all panes + bottom time axis
    let main_vp = PaneViewport::new(
        &viewport.time_scale,
        &viewport.main_price_scale,
        main_plot_f64,
        base_value,
    );
    let vol_vp = PaneViewport::new(
        &viewport.time_scale,
        &viewport.volume_price_scale,
        main_plot_f64,
        1.0,
    );

    let time_grid_ticks = collect_time_grid_ticks(
        candles,
        config.timeframe,
        &viewport.time_scale,
        &main_vp,
        multi_layout.main_plot_rect,
        multi_layout.time_axis_rect,
    );

    // 9. Paint Main Pane horizontal price grid + vertical time grid + candlesticks + volume + overlays
    paint_horizontal_price_grid_and_ticks(
        &painter,
        multi_layout.main_plot_rect,
        multi_layout.main_price_axis_rect,
        &viewport.main_price_scale,
        &main_vp,
        base_value,
        config.show_grid,
        54.0,
        4,
        12,
    );
    if config.show_grid {
        for tick in &time_grid_ticks {
            painter.line_segment(
                [
                    Pos2::new(tick.x, multi_layout.main_plot_rect.top()),
                    Pos2::new(tick.x, multi_layout.main_plot_rect.bottom()),
                ],
                Stroke::new(1.0, palette::GRID_LINE),
            );
        }
    }
    paint_bottom_time_axis_labels(&painter, multi_layout.time_axis_rect, &time_grid_ticks);

    let effective_bar_spacing = viewport.time_scale.bar_spacing() * (conflation_factor as f64);
    let bar_metrics = CandlestickBarMetrics::compute(effective_bar_spacing, ppp);
    let rendered_candle_count = paint_candlesticks_and_volume(
        &painter,
        multi_layout.main_plot_rect,
        candles,
        culling_range.clone(),
        conflation_factor,
        &main_vp,
        &vol_vp,
        bar_metrics,
        ppp,
        config.show_volume,
    );

    // Main-Pane Technical Overlays (`BollingerBands` ribbon + `SMA` + `EMA`) aligned to exact `conflation_factor`
    let main_plot_painter = painter.with_clip_rect(multi_layout.main_plot_rect);
    let mut active_overlay_count = 0_usize;
    if indicators.show_bbands {
        active_overlay_count += 1;
        paint_bollinger_bands(
            &main_plot_painter,
            indicator_cache,
            culling_range.clone(),
            conflation_factor,
            &main_vp,
        );
    }
    if indicators.show_sma {
        active_overlay_count += 1;
        paint_conflated_line_series(
            &main_plot_painter,
            &indicator_cache.sma,
            Some(&indicator_cache.sma_idx),
            culling_range.clone(),
            conflation_factor,
            &main_vp,
            Stroke::new(1.6, palette::IND_SMA),
        );
    }
    if indicators.show_ema {
        active_overlay_count += 1;
        paint_conflated_line_series(
            &main_plot_painter,
            &indicator_cache.ema,
            Some(&indicator_cache.ema_idx),
            culling_range.clone(),
            conflation_factor,
            &main_vp,
            Stroke::new(1.6, palette::IND_EMA),
        );
    }

    paint_latest_close_line_and_pill(
        &painter,
        &main_plot_painter,
        multi_layout.main_plot_rect,
        multi_layout.main_price_axis_rect,
        candles,
        &viewport.main_price_scale,
        &main_vp,
        base_value,
    );

    if let Some(store) = drawings.as_deref() {
        let main_draw_proj = PaneDrawingProjection::new(
            &viewport.time_scale,
            &viewport.main_price_scale,
            main_plot_f64.width,
            main_plot_f64.height,
            base_value,
        );
        paint_pane_drawings(
            &painter,
            &main_plot_painter,
            multi_layout.main_plot_rect,
            multi_layout.main_price_axis_rect,
            DrawingPaneTarget::Main,
            store,
            &main_draw_proj,
            &viewport.main_price_scale,
            base_value,
            candles.len(),
        );
    }

    let (rendered_fill_marker_count, active_price_line_count) =
        if let Some(session) = replay_trading.trading_session {
            paint_trading_overlays(
                &painter,
                &main_plot_painter,
                multi_layout.main_plot_rect,
                multi_layout.main_price_axis_rect,
                candles,
                culling_range.clone(),
                conflation_factor,
                &viewport.main_price_scale,
                &main_vp,
                base_value,
                session,
            )
        } else {
            (0, 0)
        };

    if replay_trading.replay_active && !replay_trading.selecting_cut_bar {
        paint_replay_frontier(
            &main_plot_painter,
            &painter,
            &multi_layout,
            candles,
            &main_vp,
            viewport.time_scale.bar_spacing(),
            replay_trading.replay_status_label,
        );
    }

    // 10. Paint Stacked Oscillator Sub-Panes (`RSI`, `MACD`, `ATR`)
    for osc in &multi_layout.oscillators {
        let osc_ps = viewport.oscillator_price_scale(osc.id);
        let osc_plot_f64 = PlotRect::from_egui(osc.plot_rect);
        let osc_vp = PaneViewport::new(&viewport.time_scale, osc_ps, osc_plot_f64, 1.0);

        paint_horizontal_price_grid_and_ticks(
            &painter,
            osc.plot_rect,
            osc.price_axis_rect,
            osc_ps,
            &osc_vp,
            1.0,
            config.show_grid,
            42.0,
            2,
            5,
        );

        if config.show_grid {
            for tick in &time_grid_ticks {
                painter.line_segment(
                    [
                        Pos2::new(tick.x, osc.plot_rect.top()),
                        Pos2::new(tick.x, osc.plot_rect.bottom()),
                    ],
                    Stroke::new(1.0, palette::GRID_LINE),
                );
            }
        }

        let osc_painter = painter.with_clip_rect(osc.plot_rect);
        match osc.id {
            OscillatorPaneId::Rsi => {
                paint_rsi_pane(
                    &osc_painter,
                    osc.plot_rect,
                    &indicator_cache.rsi,
                    Some(&indicator_cache.rsi_idx),
                    culling_range.clone(),
                    conflation_factor,
                    &osc_vp,
                );
            }
            OscillatorPaneId::Macd => {
                paint_macd_pane(
                    &osc_painter,
                    osc.plot_rect,
                    indicator_cache,
                    culling_range.clone(),
                    conflation_factor,
                    &osc_vp,
                    bar_metrics,
                );
            }
            OscillatorPaneId::Atr => {
                paint_conflated_line_series(
                    &osc_painter,
                    &indicator_cache.atr,
                    Some(&indicator_cache.atr_idx),
                    culling_range.clone(),
                    conflation_factor,
                    &osc_vp,
                    Stroke::new(1.5, palette::IND_ATR),
                );
            }
        }

        if let Some(store) = drawings.as_deref() {
            let osc_draw_proj = PaneDrawingProjection::new(
                &viewport.time_scale,
                osc_ps,
                osc_plot_f64.width,
                osc_plot_f64.height,
                1.0,
            );
            paint_pane_drawings(
                &painter,
                &osc_painter,
                osc.plot_rect,
                osc.price_axis_rect,
                DrawingPaneTarget::Oscillator(osc.id),
                store,
                &osc_draw_proj,
                osc_ps,
                1.0,
                candles.len(),
            );
        }
    }

    // 11. Resolve Unified Synchronized Crosshair across Main + Oscillator Sub-Panes
    #[derive(Clone, Copy)]
    enum HoveredPane {
        Main(Pos2),
        Oscillator(OscillatorPaneId, Rect, Rect, Pos2),
    }

    let hovered_pane = if separator_active_idx.is_some_and(|idx| sep_responses[idx].dragged()) {
        None
    } else if let Some(pos) = main_plot_resp
        .hover_pos()
        .filter(|p| multi_layout.main_plot_rect.contains(*p))
    {
        Some(HoveredPane::Main(pos))
    } else {
        osc_responses
            .iter()
            .find_map(|&(id, p_rect, ax_rect, ref pr, _)| {
                pr.hover_pos()
                    .filter(|p| p_rect.contains(*p))
                    .map(|pos| HoveredPane::Oscillator(id, p_rect, ax_rect, pos))
            })
    };

    let hover_x = hovered_pane.map(|hp| match hp {
        HoveredPane::Main(p) | HoveredPane::Oscillator(_, _, _, p) => p.x,
    });
    let hovered_slot_index = hover_x.and_then(|hx| main_vp.screen_x_to_index(f64::from(hx)));
    let default_inspected_idx = culling_range
        .end
        .checked_sub(1)
        .unwrap_or(candles.len() - 1);
    let inspected_data_index = hover_x
        .and_then(|hx| main_vp.screen_x_to_clamped_data_index(f64::from(hx)))
        .unwrap_or(default_inspected_idx);

    let mut pointer_price = None;
    let mut hovered_oscillator = None;
    let mut hovered_oscillator_value = None;

    if let Some(hp) = hovered_pane {
        match hp {
            HoveredPane::Main(pos) => {
                pointer_price = main_vp.screen_y_to_price(f64::from(pos.y));
            }
            HoveredPane::Oscillator(id, p_rect, _, pos) => {
                hovered_oscillator = Some(id);
                let osc_vp = PaneViewport::new(
                    &viewport.time_scale,
                    viewport.oscillator_price_scale(id),
                    PlotRect::from_egui(p_rect),
                    1.0,
                );
                hovered_oscillator_value = osc_vp.screen_y_to_price(f64::from(pos.y));
            }
        }
    }

    if config.show_crosshair
        && let Some(hp) = hovered_pane
    {
        let raw_x = match hp {
            HoveredPane::Main(p) | HoveredPane::Oscillator(_, _, _, p) => p.x,
        };
        let slot_idx = hovered_slot_index.unwrap_or(inspected_data_index as i64);
        let snapped_screen_x =
            if conflation_factor > 1 && slot_idx >= 0 && (slot_idx as usize) < candles.len() {
                conflated_candle_at(candles, slot_idx as usize, conflation_factor)
                    .and_then(|b| main_vp.float_index_to_screen_x(b.center_logical_index))
                    .or_else(|| main_vp.index_to_screen_x(slot_idx))
            } else {
                main_vp.index_to_screen_x(slot_idx)
            };
        let cross_x = snapped_screen_x.map_or(raw_x, |sx| sx as f32).clamp(
            multi_layout.main_plot_rect.left(),
            multi_layout.main_plot_rect.right(),
        );
        let cross_stroke = Stroke::new(1.0, palette::CROSSHAIR.gamma_multiply(0.50));

        // Synchronized vertical crosshair across Main Pane + ALL active Oscillator Sub-Panes!
        main_plot_painter.line_segment(
            [
                Pos2::new(cross_x, multi_layout.main_plot_rect.top()),
                Pos2::new(cross_x, multi_layout.main_plot_rect.bottom()),
            ],
            cross_stroke,
        );
        for osc in &multi_layout.oscillators {
            painter.with_clip_rect(osc.plot_rect).line_segment(
                [
                    Pos2::new(cross_x, osc.plot_rect.top()),
                    Pos2::new(cross_x, osc.plot_rect.bottom()),
                ],
                cross_stroke,
            );
        }

        // Horizontal crosshair + right-axis value pill in the specific hovered pane
        match hp {
            HoveredPane::Main(pos) => {
                main_plot_painter.line_segment(
                    [
                        Pos2::new(multi_layout.main_plot_rect.left(), pos.y),
                        Pos2::new(multi_layout.main_plot_rect.right(), pos.y),
                    ],
                    cross_stroke,
                );
                if let Some(pv) = pointer_price {
                    paint_axis_cursor_pill(
                        &painter,
                        multi_layout.main_price_axis_rect,
                        pos.y,
                        &viewport.main_price_scale.format_price(pv, base_value),
                    );
                }
            }
            HoveredPane::Oscillator(id, p_rect, ax_rect, pos) => {
                painter.with_clip_rect(p_rect).line_segment(
                    [
                        Pos2::new(p_rect.left(), pos.y),
                        Pos2::new(p_rect.right(), pos.y),
                    ],
                    cross_stroke,
                );
                if let Some(ov) = hovered_oscillator_value {
                    paint_axis_cursor_pill(
                        &painter,
                        ax_rect,
                        pos.y,
                        &viewport.oscillator_price_scale(id).format_price(ov, 1.0),
                    );
                }
            }
        }

        paint_time_axis_cursor_pill(
            &painter,
            multi_layout.time_axis_rect,
            candles,
            config.timeframe,
            slot_idx,
            cross_x,
        );
    }

    // 11b. Interactive Bar Replay Cut Scrubber (`selecting_cut_bar == true`)
    let mut clicked_cut_bar_index = None;
    if replay_trading.selecting_cut_bar {
        if hovered_pane.is_some() {
            ui.ctx().set_cursor_icon(CursorIcon::Crosshair);
        }
        let scrubber_idx = hover_x
            .and_then(|hx| main_vp.screen_x_to_clamped_data_index(f64::from(hx)))
            .unwrap_or_else(|| ((candles.len() * 2) / 3).min(candles.len().saturating_sub(1)));
        paint_replay_cut_scrubber(
            &painter,
            &main_plot_painter,
            &multi_layout,
            candles,
            scrubber_idx,
            &main_vp,
            viewport.time_scale.bar_spacing(),
        );

        let clicked_pos = if main_plot_resp.clicked() {
            main_plot_resp
                .interact_pointer_pos()
                .or_else(|| ui.input(|i| i.pointer.interact_pos()))
        } else {
            osc_responses.iter().find_map(|(_, _, _, pr, _)| {
                pr.clicked()
                    .then(|| {
                        pr.interact_pointer_pos()
                            .or_else(|| ui.input(|i| i.pointer.interact_pos()))
                    })
                    .flatten()
            })
        };
        if let Some(pos) = clicked_pos {
            clicked_cut_bar_index = main_vp.screen_x_to_clamped_data_index(f64::from(pos.x));
        }
    }

    // 12. Paint interactive pane separators, axis gutters, and outer border
    for (idx, sep) in multi_layout.separators.iter().enumerate() {
        let is_hot = separator_active_idx == Some(idx);
        painter.rect_filled(sep.rect, 0.0, palette::SEPARATOR_BG);
        let line_color = if is_hot {
            palette::SEPARATOR_ACTIVE
        } else {
            palette::BORDER_SUBTLE
        };
        let line_w = if is_hot { 1.5 } else { 1.0 };
        painter.line_segment(
            [
                Pos2::new(sep.rect.left(), sep.rect.center().y),
                Pos2::new(sep.rect.right(), sep.rect.center().y),
            ],
            Stroke::new(line_w, line_color),
        );
        // Subtle center grip pill so discoverability of draggable splitters is immediate
        let grip_cx = multi_layout.main_plot_rect.center().x;
        painter.line_segment(
            [
                Pos2::new(grip_cx - 14.0, sep.rect.center().y),
                Pos2::new(grip_cx + 14.0, sep.rect.center().y),
            ],
            Stroke::new(
                2.0,
                if is_hot {
                    Color32::WHITE
                } else {
                    palette::TEXT_MUTED
                },
            ),
        );
    }

    painter.line_segment(
        [
            Pos2::new(multi_layout.main_plot_rect.right(), outer_rect.top()),
            Pos2::new(multi_layout.main_plot_rect.right(), outer_rect.bottom()),
        ],
        Stroke::new(1.0, palette::BORDER_SUBTLE),
    );
    painter.line_segment(
        [
            Pos2::new(outer_rect.left(), multi_layout.time_axis_rect.top()),
            Pos2::new(outer_rect.right(), multi_layout.time_axis_rect.top()),
        ],
        Stroke::new(1.0, palette::BORDER_SUBTLE),
    );
    painter.rect_stroke(
        outer_rect,
        0.0,
        Stroke::new(1.0, palette::BORDER_SUBTLE),
        StrokeKind::Inside,
    );

    // 13. Top-left Main OHLCV HUD + Overlay Legend HUD + Per-Oscillator Sub-Pane Legend HUDs
    // When conflated (`conflation_factor > 1`), inspect `end_index` of the hovered bucket so series
    // HUD readouts correspond to the closing bar of the inspected conflated candlestick!
    let hud_series_index = conflated_candle_at(candles, inspected_data_index, conflation_factor)
        .map_or(inspected_data_index, |b| b.end_index);
    paint_main_ohlcv_hud(
        &painter,
        multi_layout.main_plot_rect,
        candles,
        config,
        inspected_data_index,
        conflation_factor,
    );
    if active_overlay_count > 0 {
        paint_main_overlay_hud(
            &painter,
            multi_layout.main_plot_rect,
            indicators,
            indicator_cache,
            hud_series_index,
        );
    }
    for osc in &multi_layout.oscillators {
        paint_oscillator_hud(
            &painter,
            osc.plot_rect,
            osc.id,
            indicators,
            indicator_cache,
            hud_series_index,
        );
    }

    let (drawing_count, active_drawing_tool, selected_drawing_id, magnet_mode) =
        if let Some(store) = drawings.as_deref() {
            let cutoff = store.replay_cutoff_timestamp();
            let visible_count = store
                .drawings
                .iter()
                .filter(|d| {
                    d.is_causally_visible(cutoff, candles.len())
                        && match d.pane {
                            DrawingPaneTarget::Main => true,
                            DrawingPaneTarget::Oscillator(id) => active_osc_ids.contains(&id),
                        }
                })
                .count();
            (
                visible_count,
                store.active_tool,
                store.selected_id,
                store.magnet_mode,
            )
        } else {
            (0, DrawingTool::Select, None, MagnetMode::Off)
        };

    Some(ChartRenderReport {
        layout: multi_layout.as_primary_chart_layout(),
        bounds,
        inspected_data_index,
        pointer_price,
        bar_spacing: viewport.time_scale.bar_spacing(),
        right_offset: viewport.time_scale.right_offset(),
        conflation_factor,
        culled_bar_count: culling_range.len(),
        rendered_candle_count,
        base_value,
        kinetic_active: viewport
            .kinetic
            .as_ref()
            .is_some_and(KineticAnimation::is_active),
        pane_count: 1 + multi_layout.oscillators.len(),
        active_overlay_count,
        hovered_oscillator,
        hovered_oscillator_value,
        first_oscillator_plot_rect: multi_layout.oscillators.first().map(|o| o.plot_rect),
        first_separator_rect: multi_layout.separators.first().map(|s| s.rect),
        drawing_count,
        active_drawing_tool,
        selected_drawing_id,
        magnet_mode,
        clicked_cut_bar_index,
        rendered_fill_marker_count,
        active_price_line_count,
    })
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

    if main_price_scale.price_range().is_none() {
        let init_cf = shared_time_scale.conflation_factor(ppp);
        let _ = refresh_main_pane_autoscale_and_bounds(
            candles,
            shared_time_scale,
            main_price_scale,
            volume_price_scale,
            config.show_volume,
            init_cf,
            None,
        );
    }

    let (smooth_scroll, time_ms) = ui.input(|i| (i.smooth_scroll_delta, i.time * 1000.0));
    if plot_resp.is_pointer_button_down_on()
        || price_resp.is_pointer_button_down_on()
        || time_resp.is_pointer_button_down_on()
    {
        *shared_kinetic = None;
    } else if sync_shared_time_scale && !plot_resp.dragged() {
        step_shared_kinetic_inertia(ui, time_ms, shared_time_scale, shared_kinetic);
    }

    handle_plot_interactions(
        ui,
        &plot_resp,
        plot_rect_f64,
        smooth_scroll,
        time_ms,
        shared_time_scale,
        shared_kinetic,
        main_price_scale,
        Some(volume_price_scale),
        true,
        false,
    );
    handle_price_axis_interactions(ui, &price_resp, plot_rect_f64, main_price_scale);
    handle_time_axis_interactions(
        ui,
        &time_resp,
        plot_rect_f64,
        shared_time_scale,
        shared_kinetic,
    );

    let conflation_factor = shared_time_scale.conflation_factor(ppp);
    let (base_value, bounds) = refresh_main_pane_autoscale_and_bounds(
        candles,
        shared_time_scale,
        main_price_scale,
        volume_price_scale,
        config.show_volume,
        conflation_factor,
        None,
    )?;
    let culling_range =
        non_empty_candle_range(shared_time_scale.culling_data_range(), candles.len());

    painter.rect_filled(layout.plot_rect, 0.0, palette::BG_CHART_CANVAS);
    painter.rect_filled(layout.price_axis_rect, 0.0, palette::BG_AXIS_GUTTER);
    painter.rect_filled(layout.time_axis_rect, 0.0, palette::BG_AXIS_GUTTER);
    painter.rect_filled(layout.corner_rect, 0.0, palette::BG_WORKSTATION);

    let main_vp = PaneViewport::new(
        shared_time_scale,
        main_price_scale,
        plot_rect_f64,
        base_value,
    );
    let vol_vp = PaneViewport::new(shared_time_scale, volume_price_scale, plot_rect_f64, 1.0);

    paint_horizontal_price_grid_and_ticks(
        &painter,
        layout.plot_rect,
        layout.price_axis_rect,
        main_price_scale,
        &main_vp,
        base_value,
        config.show_grid,
        54.0,
        4,
        12,
    );

    let time_grid_ticks = collect_time_grid_ticks(
        candles,
        config.timeframe,
        shared_time_scale,
        &main_vp,
        layout.plot_rect,
        layout.time_axis_rect,
    );
    if config.show_grid {
        for tick in &time_grid_ticks {
            painter.line_segment(
                [
                    Pos2::new(tick.x, layout.plot_rect.top()),
                    Pos2::new(tick.x, layout.plot_rect.bottom()),
                ],
                Stroke::new(1.0, palette::GRID_LINE),
            );
        }
    }
    paint_bottom_time_axis_labels(&painter, layout.time_axis_rect, &time_grid_ticks);

    let effective_bar_spacing = shared_time_scale.bar_spacing() * (conflation_factor as f64);
    let bar_metrics = CandlestickBarMetrics::compute(effective_bar_spacing, ppp);
    let rendered_candle_count = paint_candlesticks_and_volume(
        &painter,
        layout.plot_rect,
        candles,
        culling_range.clone(),
        conflation_factor,
        &main_vp,
        &vol_vp,
        bar_metrics,
        ppp,
        config.show_volume,
    );

    let plot_painter = painter.with_clip_rect(layout.plot_rect);
    paint_latest_close_line_and_pill(
        &painter,
        &plot_painter,
        layout.plot_rect,
        layout.price_axis_rect,
        candles,
        main_price_scale,
        &main_vp,
        base_value,
    );

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
        let snapped_screen_x =
            if conflation_factor > 1 && slot_idx >= 0 && (slot_idx as usize) < candles.len() {
                conflated_candle_at(candles, slot_idx as usize, conflation_factor)
                    .and_then(|b| main_vp.float_index_to_screen_x(b.center_logical_index))
                    .or_else(|| main_vp.index_to_screen_x(slot_idx))
            } else {
                main_vp.index_to_screen_x(slot_idx)
            };
        let cross_x = snapped_screen_x
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

        if let Some(price_at_cursor) = pointer_price {
            paint_axis_cursor_pill(
                &painter,
                layout.price_axis_rect,
                cross_y,
                &main_price_scale.format_price(price_at_cursor, base_value),
            );
        }
        paint_time_axis_cursor_pill(
            &painter,
            layout.time_axis_rect,
            candles,
            config.timeframe,
            slot_idx,
            cross_x,
        );
    }

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

    paint_main_ohlcv_hud(
        &painter,
        layout.plot_rect,
        candles,
        config,
        inspected_data_index,
        conflation_factor,
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
        pane_count: 1,
        active_overlay_count: 0,
        hovered_oscillator: None,
        hovered_oscillator_value: None,
        first_oscillator_plot_rect: None,
        first_separator_rect: None,
        drawing_count: 0,
        active_drawing_tool: DrawingTool::Select,
        selected_drawing_id: None,
        magnet_mode: MagnetMode::Off,
        clicked_cut_bar_index: None,
        rendered_fill_marker_count: 0,
        active_price_line_count: 0,
    })
}

fn step_shared_kinetic_inertia(
    ui: &Ui,
    time_ms: f64,
    shared_time_scale: &mut TimeScale,
    shared_kinetic: &mut Option<KineticAnimation>,
) {
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

#[allow(clippy::too_many_arguments)]
fn paint_horizontal_price_grid_and_ticks(
    painter: &egui::Painter,
    plot_rect: Rect,
    price_axis_rect: Rect,
    price_scale: &PriceScale,
    vp: &PaneViewport<'_>,
    base_value: f64,
    show_grid: bool,
    px_per_tick: f32,
    min_ticks: usize,
    max_ticks: usize,
) {
    let target_ticks =
        ((plot_rect.height() / px_per_tick).floor() as usize).clamp(min_ticks, max_ticks);
    let ticks = price_scale.nice_ticks(target_ticks, base_value);
    for tick in &ticks {
        let y = vp.local_y_to_screen_y(tick.local_y) as f32;
        if y < plot_rect.top() || y > plot_rect.bottom() {
            continue;
        }
        if show_grid {
            painter.line_segment(
                [
                    Pos2::new(plot_rect.left(), y),
                    Pos2::new(plot_rect.right(), y),
                ],
                Stroke::new(1.0, palette::GRID_LINE),
            );
        }
        painter.line_segment(
            [
                Pos2::new(price_axis_rect.left(), y),
                Pos2::new(price_axis_rect.left() + 5.0, y),
            ],
            Stroke::new(1.0, palette::BORDER_SUBTLE),
        );
        let label_y = y.clamp(price_axis_rect.top() + 8.0, price_axis_rect.bottom() - 8.0);
        painter.text(
            Pos2::new(price_axis_rect.left() + 8.0, label_y),
            Align2::LEFT_CENTER,
            &tick.label,
            FontId::monospace(11.0),
            palette::TEXT_MUTED,
        );
    }
}

struct TimeGridTick {
    x: f32,
    label: Option<String>,
}

fn collect_time_grid_ticks(
    candles: &[Candle],
    timeframe: &str,
    time_scale: &TimeScale,
    main_vp: &PaneViewport<'_>,
    plot_rect: Rect,
    time_axis_rect: Rect,
) -> Vec<TimeGridTick> {
    let mut out = Vec::new();
    let Some(strict) = time_scale.visible_strict_range() else {
        return out;
    };
    let is_intraday = crate::data::Timeframe::parse_label(timeframe)
        .is_some_and(crate::data::Timeframe::is_intraday)
        || candles.first().is_some_and(|c| c.date_label.len() > 10);

    let px_per_label = if is_intraday { 138.0 } else { 130.0 };
    let target_time_labels = ((plot_rect.width() / px_per_label).floor() as i64).clamp(3, 10);
    let raw_stride = (strict.count().max(1) / target_time_labels).max(1);
    let stride = nice_bar_stride(raw_stride);
    let first_idx = strict.left.max(0).div_euclid(stride) * stride;
    let max_data_idx = (candles.len() as i64).saturating_sub(1);

    let mut prev_labeled_ts: Option<i64> = None;
    let mut last_label_right_x = f32::NEG_INFINITY;

    let mut idx = first_idx;
    while idx <= strict.right.min(max_data_idx) {
        if idx >= 0
            && let Some(sx) = main_vp.index_to_screen_x(idx)
        {
            let x = sx as f32;
            if x >= plot_rect.left() && x <= plot_rect.right() {
                let c = &candles[idx as usize];
                let candidate = if is_intraday {
                    crate::data::format_time_axis_tick(c.timestamp, prev_labeled_ts, true)
                } else {
                    c.date_label.clone()
                };
                let half_w = ((candidate.chars().count() as f32) * 3.45 + 6.0).max(34.0);
                let fits_inside_axis = x - half_w >= time_axis_rect.left() + 4.0
                    && x + half_w <= time_axis_rect.right() - 4.0;
                let non_overlapping = x - half_w >= last_label_right_x + 10.0;

                let label = if fits_inside_axis && non_overlapping {
                    prev_labeled_ts = Some(c.timestamp);
                    last_label_right_x = x + half_w;
                    Some(candidate)
                } else {
                    None
                };
                out.push(TimeGridTick { x, label });
            }
        }
        idx = idx.saturating_add(stride);
    }
    out
}

fn paint_bottom_time_axis_labels(
    painter: &egui::Painter,
    time_axis_rect: Rect,
    ticks: &[TimeGridTick],
) {
    for tick in ticks {
        if let Some(label) = tick.label.as_deref() {
            painter.line_segment(
                [
                    Pos2::new(tick.x, time_axis_rect.top()),
                    Pos2::new(tick.x, time_axis_rect.top() + 5.0),
                ],
                Stroke::new(1.0, palette::BORDER_SUBTLE),
            );
            painter.text(
                Pos2::new(tick.x, time_axis_rect.center().y),
                Align2::CENTER_CENTER,
                label,
                FontId::monospace(11.0),
                palette::TEXT_MUTED,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_candlesticks_and_volume(
    painter: &egui::Painter,
    plot_rect: Rect,
    candles: &[Candle],
    culling_range: std::ops::Range<usize>,
    conflation_factor: usize,
    main_vp: &PaneViewport<'_>,
    vol_vp: &PaneViewport<'_>,
    bar_metrics: CandlestickBarMetrics,
    ppp: f64,
    show_volume: bool,
) -> usize {
    let half_body_w = (bar_metrics.body_width_logical as f32) * 0.5;
    let wick_stroke_w = bar_metrics.wick_width_logical as f32;
    let min_body_h = (1.0 / (ppp as f32)).max(0.25);

    let plot_painter = painter.with_clip_rect(plot_rect);
    let mut rendered_candle_count = 0_usize;
    let plot_bottom = plot_rect.bottom();
    let plot_top = plot_rect.top();

    let conflated_iter = iter_conflated_candles(candles, culling_range, conflation_factor);
    let est_bars = conflated_iter.size_hint().0;
    let vol_layer_slot = show_volume.then(|| plot_painter.add(Shape::Noop));
    let mut vol_mesh = Mesh::default();
    if show_volume {
        vol_mesh.vertices.reserve(est_bars * 4);
        vol_mesh.indices.reserve(est_bars * 6);
    }
    let mut candle_mesh = Mesh::default();
    candle_mesh.vertices.reserve(est_bars * 12);
    candle_mesh.indices.reserve(est_bars * 18);
    let half_wick_w = (wick_stroke_w * 0.5).max(0.45);

    for bar in conflated_iter {
        let Some(sx) = main_vp.float_index_to_screen_x(bar.center_logical_index) else {
            continue;
        };
        let cx = sx as f32;
        if cx + half_body_w < plot_rect.left() || cx - half_body_w > plot_rect.right() {
            continue;
        };

        if show_volume {
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
            vol_mesh.add_colored_rect(
                Rect::from_min_max(
                    Pos2::new(cx - half_body_w, vol_top_y),
                    Pos2::new(cx + half_body_w, plot_bottom),
                ),
                fill,
            );
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
        let wick_top_y = high_y.min(low_y);
        let wick_bottom_y = high_y.max(low_y);

        if wick_top_y < top_y {
            candle_mesh.add_colored_rect(
                Rect::from_min_max(
                    Pos2::new(cx - half_wick_w, wick_top_y),
                    Pos2::new(cx + half_wick_w, top_y),
                ),
                color,
            );
        }
        if wick_bottom_y > bottom_y {
            candle_mesh.add_colored_rect(
                Rect::from_min_max(
                    Pos2::new(cx - half_wick_w, bottom_y),
                    Pos2::new(cx + half_wick_w, wick_bottom_y),
                ),
                color,
            );
        }

        let body_rect = Rect::from_min_max(
            Pos2::new(cx - half_body_w, top_y),
            Pos2::new(cx + half_body_w, bottom_y),
        );
        candle_mesh.add_colored_rect(body_rect, color);
    }

    if let Some(slot) = vol_layer_slot
        && !vol_mesh.indices.is_empty()
    {
        plot_painter.set(slot, Shape::mesh(vol_mesh));
    }
    if !candle_mesh.indices.is_empty() {
        plot_painter.add(Shape::mesh(candle_mesh));
    }

    rendered_candle_count
}

#[inline]
fn resolve_bucket_screen_ys(
    env: &BucketLineEnvelope,
    vp: &PaneViewport<'_>,
) -> Option<(f32, f32, bool)> {
    let sy0 = vp.price_to_screen_y(env.entry_val)? as f32;
    if env.is_single {
        return Some((sy0, sy0, true));
    }
    let sy1 = vp.price_to_screen_y(env.exit_val)? as f32;
    if env.has_internal_gap {
        return Some((sy0, sy1, false));
    }
    // Collapse degenerate sub-pixel vertical jogs (`< 0.65px`) inside smooth buckets while
    // preserving every genuine intra-bucket M4 vertical spike (`>= 0.65px`).
    if (sy1 - sy0).abs() < 0.65 {
        let mid = 0.5 * (sy0 + sy1);
        Some((mid, mid, true))
    } else {
        Some((sy0, sy1, false))
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_conflated_line_series(
    painter: &egui::Painter,
    series: &[Option<f64>],
    block_index: Option<&SeriesBlockIndex>,
    culling_range: std::ops::Range<usize>,
    conflation_factor: usize,
    vp: &PaneViewport<'_>,
    stroke: Stroke,
) {
    let step = conflation_factor.max(1);
    let total = series.len();
    let clamped_start = culling_range.start.min(total);
    let clamped_end = culling_range.end.min(total);
    if clamped_start >= clamped_end {
        return;
    }

    let mut current_pts: Vec<Pos2> = Vec::with_capacity(256);
    let flush_segment = |painter: &egui::Painter, pts: &mut Vec<Pos2>| {
        if pts.len() >= 2 {
            painter.add(Shape::line(std::mem::take(pts), stroke));
        } else if let Some(&single_pt) = pts.first() {
            painter.circle_filled(single_pt, (stroke.width * 0.9).max(1.2), stroke.color);
            pts.clear();
        }
    };

    if step == 1 {
        for (idx, item) in series
            .iter()
            .enumerate()
            .take(clamped_end)
            .skip(clamped_start)
        {
            if let Some(v) = item.filter(|v| v.is_finite())
                && let (Some(sx), Some(sy)) = (
                    vp.float_index_to_screen_x(idx as f64),
                    vp.price_to_screen_y(v),
                )
            {
                current_pts.push(Pos2::new(sx as f32, sy as f32));
            } else {
                flush_segment(painter, &mut current_pts);
            }
        }
        flush_segment(painter, &mut current_pts);
        return;
    }

    let aligned_start = (clamped_start / step) * step;
    let aligned_end = clamped_end.div_ceil(step).saturating_mul(step).min(total);
    let mut b_start = aligned_start;

    while b_start < aligned_end {
        let b_end = (b_start + step).min(total);
        if let Some(env) = bucket_line_envelope_indexed(series, block_index, b_start, b_end)
            && let (Some(sx), Some((y_entry, y_exit, single))) = (
                vp.float_index_to_screen_x(env.center_logical_index),
                resolve_bucket_screen_ys(&env, vp),
            )
        {
            let x = sx as f32;
            if env.has_leading_gap {
                flush_segment(painter, &mut current_pts);
            }
            if env.has_internal_gap {
                current_pts.push(Pos2::new(x, y_entry));
                flush_segment(painter, &mut current_pts);
                current_pts.push(Pos2::new(x, y_exit));
            } else if single {
                current_pts.push(Pos2::new(x, y_entry));
            } else {
                current_pts.push(Pos2::new(x, y_entry));
                current_pts.push(Pos2::new(x, y_exit));
            }
            if env.has_trailing_gap {
                flush_segment(painter, &mut current_pts);
            }
        } else {
            flush_segment(painter, &mut current_pts);
        }
        b_start = b_end;
    }
    flush_segment(painter, &mut current_pts);
}

fn paint_bollinger_bands(
    painter: &egui::Painter,
    cache: &IndicatorCache,
    culling_range: std::ops::Range<usize>,
    conflation_factor: usize,
    vp: &PaneViewport<'_>,
) {
    let bbands = &cache.bbands;
    let step = conflation_factor.max(1);
    let total = bbands.middle.len();
    let clamped_start = culling_range.start.min(total);
    let clamped_end = culling_range.end.min(total);
    if clamped_start < clamped_end {
        let mut mesh = Mesh::default();
        // `(prev_x, prev_u_exit_y, prev_l_exit_y)` matching the exact exit coordinates of the
        // `bbands.upper` and `bbands.lower` border polylines so the filled ribbon can NEVER bleed!
        let mut prev_exit: Option<(f32, f32, f32)> = None;

        let aligned_start = (clamped_start / step) * step;
        let aligned_end = clamped_end.div_ceil(step).saturating_mul(step).min(total);
        let mut b_start = aligned_start;

        while b_start < aligned_end {
            let b_end = (b_start + step).min(total);
            let u_env_opt = bucket_line_envelope_indexed(
                &bbands.upper,
                Some(&cache.bb_upper_idx),
                b_start,
                b_end,
            );
            let l_env_opt = bucket_line_envelope_indexed(
                &bbands.lower,
                Some(&cache.bb_lower_idx),
                b_start,
                b_end,
            );

            if let (Some(u_env), Some(l_env)) = (u_env_opt, l_env_opt)
                && let (Some(sx), Some((u_in, u_out, _)), Some((l_in, l_out, _))) = (
                    vp.float_index_to_screen_x(u_env.center_logical_index),
                    resolve_bucket_screen_ys(&u_env, vp),
                    resolve_bucket_screen_ys(&l_env, vp),
                )
            {
                let x = sx as f32;
                if u_env.has_leading_gap || l_env.has_leading_gap {
                    prev_exit = None;
                }
                if let Some((px, pu_out, pl_out)) = prev_exit {
                    let tl = pu_out.min(pl_out);
                    let bl = pu_out.max(pl_out);
                    let tr = u_in.min(l_in);
                    let br = u_in.max(l_in);
                    let base_v = mesh.vertices.len() as u32;
                    mesh.colored_vertex(Pos2::new(px, tl), palette::IND_BB_FILL);
                    mesh.colored_vertex(Pos2::new(px, bl), palette::IND_BB_FILL);
                    mesh.colored_vertex(Pos2::new(x, tr), palette::IND_BB_FILL);
                    mesh.colored_vertex(Pos2::new(x, br), palette::IND_BB_FILL);
                    mesh.add_triangle(base_v, base_v + 1, base_v + 2);
                    mesh.add_triangle(base_v + 1, base_v + 3, base_v + 2);
                }
                if u_env.has_trailing_gap || l_env.has_trailing_gap {
                    prev_exit = None;
                } else {
                    prev_exit = Some((x, u_out, l_out));
                }
            } else {
                prev_exit = None;
            }

            b_start = b_end;
        }

        if !mesh.indices.is_empty() {
            painter.add(Shape::mesh(mesh));
        }
    }

    let edge_stroke = Stroke::new(1.2, palette::IND_BB_BORDER);
    let mid_stroke = Stroke::new(1.2, palette::IND_BB_MIDDLE);
    paint_conflated_line_series(
        painter,
        &bbands.upper,
        Some(&cache.bb_upper_idx),
        culling_range.clone(),
        conflation_factor,
        vp,
        edge_stroke,
    );
    paint_conflated_line_series(
        painter,
        &bbands.middle,
        Some(&cache.bb_middle_idx),
        culling_range.clone(),
        conflation_factor,
        vp,
        mid_stroke,
    );
    paint_conflated_line_series(
        painter,
        &bbands.lower,
        Some(&cache.bb_lower_idx),
        culling_range,
        conflation_factor,
        vp,
        edge_stroke,
    );
}

#[allow(clippy::too_many_arguments)]
fn paint_rsi_pane(
    painter: &egui::Painter,
    plot_rect: Rect,
    rsi_series: &[Option<f64>],
    rsi_index: Option<&SeriesBlockIndex>,
    culling_range: std::ops::Range<usize>,
    conflation_factor: usize,
    vp: &PaneViewport<'_>,
) {
    let upper_ref = RSI_REFERENCE_LEVELS[0].value;
    let mid_ref = RSI_REFERENCE_LEVELS[1].value;
    let lower_ref = RSI_REFERENCE_LEVELS[2].value;

    if let (Some(y_hi), Some(y_lo)) = (
        vp.price_to_screen_y(upper_ref),
        vp.price_to_screen_y(lower_ref),
    ) {
        let top_y = (y_hi as f32)
            .min(y_lo as f32)
            .clamp(plot_rect.top(), plot_rect.bottom());
        let bot_y = (y_hi as f32)
            .max(y_lo as f32)
            .clamp(plot_rect.top(), plot_rect.bottom());
        if bot_y > top_y {
            painter.rect_filled(
                Rect::from_min_max(
                    Pos2::new(plot_rect.left(), top_y),
                    Pos2::new(plot_rect.right(), bot_y),
                ),
                0.0,
                palette::IND_RSI_BAND_FILL,
            );
        }
    }

    for (lvl, alpha) in [(upper_ref, 0.55), (mid_ref, 0.30), (lower_ref, 0.55)] {
        if let Some(sy) = vp.price_to_screen_y(lvl) {
            let y = sy as f32;
            if y >= plot_rect.top() && y <= plot_rect.bottom() {
                painter.line_segment(
                    [
                        Pos2::new(plot_rect.left(), y),
                        Pos2::new(plot_rect.right(), y),
                    ],
                    Stroke::new(1.0, palette::IND_RSI.gamma_multiply(alpha)),
                );
            }
        }
    }

    paint_conflated_line_series(
        painter,
        rsi_series,
        rsi_index,
        culling_range,
        conflation_factor,
        vp,
        Stroke::new(1.5, palette::IND_RSI),
    );
}

#[inline]
fn resolve_macd_histogram_bar_ys(
    zero_y: f32,
    val_y: f32,
    value: f64,
    plot_top: f32,
    plot_bottom: f32,
) -> Option<(f32, f32)> {
    let (raw_top, raw_bot) = if value.abs() <= 1e-15 {
        (zero_y - 0.5, zero_y + 0.5)
    } else if val_y >= zero_y {
        (zero_y, val_y.max(zero_y + 0.75))
    } else {
        (val_y.min(zero_y - 0.75), zero_y)
    };
    let top_y = raw_top.max(plot_top);
    let bot_y = raw_bot.min(plot_bottom);
    (bot_y > top_y).then_some((top_y, bot_y))
}

fn paint_macd_pane(
    painter: &egui::Painter,
    plot_rect: Rect,
    cache: &IndicatorCache,
    culling_range: std::ops::Range<usize>,
    conflation_factor: usize,
    vp: &PaneViewport<'_>,
    bar_metrics: CandlestickBarMetrics,
) {
    let macd = &cache.macd;
    let zero_level = MACD_REFERENCE_LEVELS[0].value;
    let zero_y_opt = vp.price_to_screen_y(zero_level).map(|sy| sy as f32);

    if let Some(zy) = zero_y_opt
        && zy >= plot_rect.top()
        && zy <= plot_rect.bottom()
    {
        painter.line_segment(
            [
                Pos2::new(plot_rect.left(), zy),
                Pos2::new(plot_rect.right(), zy),
            ],
            Stroke::new(1.0, palette::BORDER_SUBTLE),
        );
    }

    if let Some(zero_y) = zero_y_opt {
        let half_w = ((bar_metrics.body_width_logical as f32) * 0.5).max(0.5);
        let mut hist_mesh = Mesh::default();
        visit_conflated_histogram_bars_indexed(
            &macd.histogram,
            Some(&cache.macd_hist_idx),
            culling_range.clone(),
            conflation_factor,
            |bar| {
                let Some(sx) = vp.float_index_to_screen_x(bar.center_logical_index) else {
                    return;
                };
                let cx = sx as f32;
                if cx + half_w < plot_rect.left() || cx - half_w > plot_rect.right() {
                    return;
                }
                let Some(val_sy) = vp.price_to_screen_y(bar.value) else {
                    return;
                };
                let Some((top_y, bot_y)) = resolve_macd_histogram_bar_ys(
                    zero_y,
                    val_sy as f32,
                    bar.value,
                    plot_rect.top(),
                    plot_rect.bottom(),
                ) else {
                    return;
                };

                let rising = bar.prev_value.is_none_or(|pv| bar.value >= pv);
                let color = if bar.value >= 0.0 {
                    if rising {
                        palette::IND_MACD_HIST_POS_UP
                    } else {
                        palette::IND_MACD_HIST_POS_DOWN
                    }
                } else if rising {
                    palette::IND_MACD_HIST_NEG_UP
                } else {
                    palette::IND_MACD_HIST_NEG_DOWN
                };

                hist_mesh.add_colored_rect(
                    Rect::from_min_max(
                        Pos2::new(cx - half_w, top_y),
                        Pos2::new(cx + half_w, bot_y),
                    ),
                    color,
                );
            },
        );
        if !hist_mesh.indices.is_empty() {
            painter.add(Shape::mesh(hist_mesh));
        }
    }

    paint_conflated_line_series(
        painter,
        &macd.macd_line,
        Some(&cache.macd_line_idx),
        culling_range.clone(),
        conflation_factor,
        vp,
        Stroke::new(1.4, palette::IND_MACD),
    );
    paint_conflated_line_series(
        painter,
        &macd.signal_line,
        Some(&cache.macd_signal_idx),
        culling_range,
        conflation_factor,
        vp,
        Stroke::new(1.4, palette::IND_MACD_SIGNAL),
    );
}

#[allow(clippy::too_many_arguments)]
fn paint_latest_close_line_and_pill(
    painter: &egui::Painter,
    plot_painter: &egui::Painter,
    plot_rect: Rect,
    price_axis_rect: Rect,
    candles: &[Candle],
    main_price_scale: &PriceScale,
    main_vp: &PaneViewport<'_>,
    base_value: f64,
) {
    if let Some(last_candle) = candles.last()
        && let Some(last_sy) = main_vp.price_to_screen_y(last_candle.close)
    {
        let last_y = last_sy as f32;
        if last_y >= plot_rect.top() && last_y <= plot_rect.bottom() {
            let last_color = if last_candle.is_bullish() {
                palette::BULLISH
            } else {
                palette::BEARISH
            };
            plot_painter.line_segment(
                [
                    Pos2::new(plot_rect.left(), last_y),
                    Pos2::new(plot_rect.right(), last_y),
                ],
                Stroke::new(1.0, last_color.gamma_multiply(0.55)),
            );

            let pill_y = last_y.clamp(
                price_axis_rect.top() + 10.0,
                price_axis_rect.bottom() - 10.0,
            );
            let pill_rect = Rect::from_min_max(
                Pos2::new(price_axis_rect.left() + 2.0, pill_y - 10.0),
                Pos2::new(price_axis_rect.right() - 4.0, pill_y + 10.0),
            );
            painter.rect_filled(pill_rect, 3.0, last_color);
            painter.text(
                Pos2::new(price_axis_rect.left() + 7.0, pill_y),
                Align2::LEFT_CENTER,
                main_price_scale.format_price(last_candle.close, base_value),
                FontId::monospace(11.0),
                Color32::WHITE,
            );
        }
    }
}

fn paint_axis_cursor_pill(
    painter: &egui::Painter,
    axis_rect: Rect,
    cross_y: f32,
    formatted_label: &str,
) {
    let pill_y = cross_y.clamp(axis_rect.top() + 10.0, axis_rect.bottom() - 10.0);
    let cursor_pill = Rect::from_min_max(
        Pos2::new(axis_rect.left() + 2.0, pill_y - 9.5),
        Pos2::new(axis_rect.right() - 4.0, pill_y + 9.5),
    );
    painter.rect_filled(cursor_pill, 3.0, palette::ACCENT_BLUE);
    painter.text(
        Pos2::new(axis_rect.left() + 7.0, pill_y),
        Align2::LEFT_CENTER,
        formatted_label,
        FontId::monospace(11.0),
        Color32::WHITE,
    );
}

pub(crate) fn format_time_axis_cursor_pill_label(
    candles: &[Candle],
    timeframe: &str,
    slot_idx: i64,
) -> String {
    let parsed_tf = crate::data::Timeframe::parse_label(timeframe);
    let unit_suffix = parsed_tf.map_or("b", crate::data::Timeframe::unit_suffix);
    let unit_mult = parsed_tf.map_or(1_i64, crate::data::Timeframe::unit_multiplier);
    if slot_idx >= 0 && (slot_idx as usize) < candles.len() {
        candles[slot_idx as usize].date_label.clone()
    } else if slot_idx >= candles.len() as i64 {
        let ahead = (slot_idx - (candles.len() as i64 - 1)).saturating_mul(unit_mult);
        format!("+{ahead}{unit_suffix}")
    } else {
        let behind = slot_idx.saturating_mul(unit_mult);
        format!("{behind}{unit_suffix}")
    }
}

fn paint_time_axis_cursor_pill(
    painter: &egui::Painter,
    time_axis_rect: Rect,
    candles: &[Candle],
    timeframe: &str,
    slot_idx: i64,
    cross_x: f32,
) {
    let time_label = format_time_axis_cursor_pill_label(candles, timeframe, slot_idx);
    let max_half_w = (time_axis_rect.width() * 0.48).max(42.0);
    let pill_half_w = ((time_label.chars().count() as f32) * 3.45 + 9.0)
        .max(42.0)
        .min(max_half_w);
    let pill_cx = cross_x.clamp(
        time_axis_rect.left() + pill_half_w + 2.0,
        (time_axis_rect.right() - pill_half_w - 2.0).max(time_axis_rect.left() + pill_half_w + 2.0),
    );
    let time_pill = Rect::from_min_max(
        Pos2::new(pill_cx - pill_half_w, time_axis_rect.top() + 3.0),
        Pos2::new(pill_cx + pill_half_w, time_axis_rect.bottom() - 3.0),
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

fn paint_main_ohlcv_hud(
    painter: &egui::Painter,
    plot_rect: Rect,
    candles: &[Candle],
    config: ChartDisplayConfig<'_>,
    inspected_data_index: usize,
    conflation_factor: usize,
) {
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
        Pos2::new(plot_rect.left() + 12.0, plot_rect.top() + 8.0),
        Vec2::new(
            desired_hud_w.min((plot_rect.width() - 24.0).max(120.0)),
            26.0,
        ),
    );
    painter.rect_filled(hud_rect, 4.0, palette::BG_FLOATING_HUD);
    painter.rect_stroke(
        hud_rect,
        4.0,
        Stroke::new(1.0, palette::BORDER_SUBTLE),
        StrokeKind::Inside,
    );
    painter.with_clip_rect(hud_rect).galley(
        Pos2::new(
            hud_rect.left() + 10.0,
            hud_rect.center().y - hud_galley.size().y * 0.5,
        ),
        hud_galley,
        value_color,
    );
}

fn paint_main_overlay_hud(
    painter: &egui::Painter,
    plot_rect: Rect,
    indicators: &IndicatorConfig,
    cache: &IndicatorCache,
    idx: usize,
) {
    if plot_rect.height() < 64.0 {
        return;
    }
    let mut job = LayoutJob::default();
    let font = FontId::monospace(11.0);
    let mut has_item = false;

    let append_sep = |job: &mut LayoutJob, has_item: &mut bool| {
        if *has_item {
            job.append(
                "   ·   ",
                0.0,
                TextFormat {
                    font_id: font.clone(),
                    color: palette::TEXT_MUTED,
                    ..Default::default()
                },
            );
        }
        *has_item = true;
    };

    if indicators.show_sma {
        append_sep(&mut job, &mut has_item);
        let val = cache
            .sma
            .get(idx)
            .copied()
            .flatten()
            .map_or_else(|| "—".to_owned(), |v| format!("{v:.2}"));
        job.append(
            &format!("SMA({}) {val}", indicators.sma_period),
            0.0,
            TextFormat {
                font_id: font.clone(),
                color: palette::IND_SMA,
                ..Default::default()
            },
        );
    }
    if indicators.show_ema {
        append_sep(&mut job, &mut has_item);
        let val = cache
            .ema
            .get(idx)
            .copied()
            .flatten()
            .map_or_else(|| "—".to_owned(), |v| format!("{v:.2}"));
        job.append(
            &format!("EMA({}) {val}", indicators.ema_period),
            0.0,
            TextFormat {
                font_id: font.clone(),
                color: palette::IND_EMA,
                ..Default::default()
            },
        );
    }
    if indicators.show_bbands {
        append_sep(&mut job, &mut has_item);
        let fmt_opt = |s: &[Option<f64>]| {
            s.get(idx)
                .copied()
                .flatten()
                .map_or_else(|| "—".to_owned(), |v| format!("{v:.2}"))
        };
        job.append(
            &format!(
                "BB({}, {:.1}) {} / {} / {}",
                indicators.bb_period,
                indicators.bb_std_mult,
                fmt_opt(&cache.bbands.upper),
                fmt_opt(&cache.bbands.middle),
                fmt_opt(&cache.bbands.lower),
            ),
            0.0,
            TextFormat {
                font_id: font,
                color: palette::IND_BB_BORDER,
                ..Default::default()
            },
        );
    }

    paint_layout_job_hud(
        painter,
        Pos2::new(plot_rect.left() + 12.0, plot_rect.top() + 38.0),
        (plot_rect.width() - 24.0).max(100.0),
        22.0,
        job,
    );
}

fn paint_oscillator_hud(
    painter: &egui::Painter,
    plot_rect: Rect,
    id: OscillatorPaneId,
    indicators: &IndicatorConfig,
    cache: &IndicatorCache,
    idx: usize,
) {
    let mut job = LayoutJob::default();
    let font = FontId::monospace(10.8);

    match id {
        OscillatorPaneId::Rsi => {
            let val = cache
                .rsi
                .get(idx)
                .copied()
                .flatten()
                .map_or_else(|| "—".to_owned(), |v| format!("{v:.2}"));
            job.append(
                &format!("RSI({})  {val}", indicators.rsi_period),
                0.0,
                TextFormat {
                    font_id: font,
                    color: palette::IND_RSI,
                    ..Default::default()
                },
            );
        }
        OscillatorPaneId::Macd => {
            let fmt_signed = |s: &[Option<f64>]| {
                s.get(idx)
                    .copied()
                    .flatten()
                    .map_or_else(|| "—".to_owned(), |v| format!("{v:+.2}"))
            };
            let hist_val = cache.macd.histogram.get(idx).copied().flatten();
            let hist_color = match hist_val {
                Some(v) if v >= 0.0 => palette::BULLISH,
                Some(_) => palette::BEARISH,
                None => palette::TEXT_MUTED,
            };
            job.append(
                &format!(
                    "MACD({}, {}, {})  ",
                    indicators.macd_fast, indicators.macd_slow, indicators.macd_signal
                ),
                0.0,
                TextFormat {
                    font_id: font.clone(),
                    color: palette::TEXT_PRIMARY,
                    ..Default::default()
                },
            );
            job.append(
                &format!("MACD {}   ", fmt_signed(&cache.macd.macd_line)),
                0.0,
                TextFormat {
                    font_id: font.clone(),
                    color: palette::IND_MACD,
                    ..Default::default()
                },
            );
            job.append(
                &format!("Sig {}   ", fmt_signed(&cache.macd.signal_line)),
                0.0,
                TextFormat {
                    font_id: font.clone(),
                    color: palette::IND_MACD_SIGNAL,
                    ..Default::default()
                },
            );
            job.append(
                &format!("Hist {}", fmt_signed(&cache.macd.histogram)),
                0.0,
                TextFormat {
                    font_id: font,
                    color: hist_color,
                    ..Default::default()
                },
            );
        }
        OscillatorPaneId::Atr => {
            let val = cache
                .atr
                .get(idx)
                .copied()
                .flatten()
                .map_or_else(|| "—".to_owned(), |v| format!("{v:.2}"));
            job.append(
                &format!("ATR({})  {val}", indicators.atr_period),
                0.0,
                TextFormat {
                    font_id: font,
                    color: palette::IND_ATR,
                    ..Default::default()
                },
            );
        }
    }

    paint_layout_job_hud(
        painter,
        Pos2::new(plot_rect.left() + 10.0, plot_rect.top() + 6.0),
        (plot_rect.width() - 20.0).max(90.0),
        21.0,
        job,
    );
}

fn paint_layout_job_hud(
    painter: &egui::Painter,
    pos: Pos2,
    max_width: f32,
    height: f32,
    mut job: LayoutJob,
) {
    job.wrap.max_width = f32::INFINITY;
    let galley = painter.layout_job(job);
    let desired_w = (galley.size().x + 18.0).clamp(90.0, max_width);
    let hud_rect = Rect::from_min_size(pos, Vec2::new(desired_w, height));
    painter.rect_filled(hud_rect, 4.0, palette::BG_FLOATING_HUD);
    painter.rect_stroke(
        hud_rect,
        4.0,
        Stroke::new(1.0, palette::BORDER_SUBTLE),
        StrokeKind::Inside,
    );
    painter.with_clip_rect(hud_rect).galley(
        Pos2::new(
            hud_rect.left() + 8.0,
            hud_rect.center().y - galley.size().y * 0.5,
        ),
        galley,
        Color32::WHITE,
    );
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
    pane_price_scale: &mut PriceScale,
    volume_price_scale: Option<&mut PriceScale>,
    allow_double_click_reset: bool,
    suppress_drag_pan: bool,
) {
    if plot_resp.hovered() && !plot_resp.dragged() {
        ui.ctx().set_cursor_icon(CursorIcon::Crosshair);

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

    if suppress_drag_pan {
        return;
    }

    if allow_double_click_reset && plot_resp.double_clicked() {
        *kinetic = None;
        time_scale.restore_default();
        pane_price_scale.reset_auto_scale();
        if let Some(vps) = volume_price_scale {
            vps.reset_auto_scale();
        }
        return;
    }

    if plot_resp.drag_started() {
        *kinetic = None;
        if let Some(pos) = ui
            .input(|i| i.pointer.press_origin())
            .or_else(|| plot_resp.interact_pointer_pos())
        {
            let local_x = f64::from(pos.x) - plot_rect.left;
            let local_y = f64::from(pos.y) - plot_rect.top;
            time_scale.start_scroll(local_x);

            let mut ka = KineticAnimation::for_bar_spacing(
                time_scale.bar_spacing(),
                KineticScrollConfig::default(),
            );
            ka.add_position(time_scale.right_offset(), time_ms);
            *kinetic = Some(ka);

            if !pane_price_scale.is_auto_scale() {
                pane_price_scale.start_scroll(local_y);
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
            if !pane_price_scale.is_auto_scale() {
                pane_price_scale.scroll_to(local_y);
            }
        }
    }

    if plot_resp.drag_stopped() {
        time_scale.end_scroll();
        pane_price_scale.end_scroll();
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
    price_scale: &mut PriceScale,
) {
    if price_resp.hovered() || price_resp.dragged() {
        ui.ctx().set_cursor_icon(CursorIcon::ResizeVertical);
    }

    if price_resp.double_clicked() {
        price_scale.reset_auto_scale();
        return;
    }

    if price_resp.drag_started()
        && let Some(pos) = ui
            .input(|i| i.pointer.press_origin())
            .or_else(|| price_resp.interact_pointer_pos())
    {
        price_scale.start_scale(f64::from(pos.y) - plot_rect.top);
    }

    if price_resp.dragged()
        && let Some(pos) = price_resp.interact_pointer_pos()
    {
        price_scale.scale_to(f64::from(pos.y) - plot_rect.top);
    }

    if price_resp.drag_stopped() {
        price_scale.end_scale();
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
        if let Some(pos) = ui
            .input(|i| i.pointer.press_origin())
            .or_else(|| time_resp.interact_pointer_pos())
        {
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

fn refresh_main_pane_autoscale_and_bounds(
    candles: &[Candle],
    time_scale: &TimeScale,
    main_price_scale: &mut PriceScale,
    volume_price_scale: &mut PriceScale,
    show_volume: bool,
    conflation_factor: usize,
    overlays: Option<(&IndicatorConfig, &IndicatorCache)>,
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

    for bar in iter_conflated_candles(candles, strict_range.clone(), conflation_factor) {
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

    // Include visible main-pane overlays (`SMA`, `EMA`, `BollingerBands`) in `main_price_scale` autoscale!
    if main_price_scale.is_auto_scale()
        && let Some((cfg, cache)) = overlays
    {
        let mut merge_ov = |bounds_opt: Option<(f64, f64)>| {
            if let Some((ov_min, ov_max)) = bounds_opt {
                if !main_price_scale.is_log() || ov_min > 0.0 {
                    raw_min = raw_min.min(ov_min);
                }
                if !main_price_scale.is_log() || ov_max > 0.0 {
                    raw_max = raw_max.max(ov_max);
                }
            }
        };
        if cfg.show_sma {
            merge_ov(cache.sma_idx.range_bounds(&cache.sma, strict_range.clone()));
        }
        if cfg.show_ema {
            merge_ov(cache.ema_idx.range_bounds(&cache.ema, strict_range.clone()));
        }
        if cfg.show_bbands {
            merge_ov(
                cache
                    .bb_upper_idx
                    .range_bounds(&cache.bbands.upper, strict_range.clone()),
            );
            merge_ov(
                cache
                    .bb_lower_idx
                    .range_bounds(&cache.bbands.lower, strict_range),
            );
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

fn refresh_oscillator_autoscale(
    id: OscillatorPaneId,
    price_scale: &mut PriceScale,
    cache: &IndicatorCache,
    strict_range: std::ops::Range<usize>,
) {
    if !price_scale.is_auto_scale() {
        return;
    }
    match id {
        OscillatorPaneId::Rsi => {
            let (min_v, max_v) = cache
                .rsi_idx
                .range_bounds(&cache.rsi, strict_range)
                .unwrap_or((0.0, 100.0));
            price_scale.autoscale_raw_bounds(min_v.min(0.0), max_v.max(100.0), 50.0);
        }
        OscillatorPaneId::Macd => {
            let mut min_v = 0.0_f64;
            let mut max_v = 0.0_f64;
            for b in [
                cache
                    .macd_line_idx
                    .range_bounds(&cache.macd.macd_line, strict_range.clone()),
                cache
                    .macd_signal_idx
                    .range_bounds(&cache.macd.signal_line, strict_range.clone()),
                cache
                    .macd_hist_idx
                    .range_bounds(&cache.macd.histogram, strict_range),
            ]
            .into_iter()
            .flatten()
            {
                min_v = min_v.min(b.0);
                max_v = max_v.max(b.1);
            }
            price_scale.autoscale_raw_bounds(min_v, max_v, 1.0);
        }
        OscillatorPaneId::Atr => {
            let (min_v, max_v) = cache
                .atr_idx
                .range_bounds(&cache.atr, strict_range)
                .unwrap_or((0.0, 1.0));
            price_scale.autoscale_raw_bounds(min_v, max_v, 1.0);
        }
    }
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

#[derive(Debug, Clone, Copy)]
struct DrawingPointerInput {
    primary_pressed: bool,
    primary_down: bool,
    primary_released: bool,
    press_origin: Option<Pos2>,
    latest_pos: Option<Pos2>,
    snap_modifier: bool,
}

fn sample_drawing_pointer_input(ui: &Ui) -> DrawingPointerInput {
    ui.input(|i| DrawingPointerInput {
        primary_pressed: i.pointer.primary_pressed(),
        primary_down: i.pointer.primary_down(),
        primary_released: i.pointer.primary_released(),
        press_origin: i.pointer.press_origin(),
        latest_pos: i.pointer.latest_pos().or_else(|| i.pointer.interact_pos()),
        snap_modifier: i.modifiers.command || i.modifiers.ctrl,
    })
}

fn sanitize_hidden_oscillator_drawing_state(
    store: &mut DrawingStore,
    active_osc_ids: &[OscillatorPaneId],
) {
    let pane_is_active = |pane: DrawingPaneTarget| match pane {
        DrawingPaneTarget::Main => true,
        DrawingPaneTarget::Oscillator(id) => active_osc_ids.contains(&id),
    };
    let cancel_phase = match store.phase() {
        DrawingInteractionPhase::Idle => false,
        DrawingInteractionPhase::PlacingSingle { pane, .. }
        | DrawingInteractionPhase::PlacingTwoAnchor { pane, .. }
        | DrawingInteractionPhase::DraggingHandle { pane, .. }
        | DrawingInteractionPhase::DraggingBody { pane, .. } => !pane_is_active(*pane),
    };
    if cancel_phase {
        store.cancel_active_gesture();
    }
    if let Some((hp, _)) = store.hovered_hit()
        && !pane_is_active(hp)
    {
        store.clear_hover_for_pane(hp);
    }
    if let Some(sel_pane) = store.selected_drawing().map(|d| d.pane)
        && !pane_is_active(sel_pane)
    {
        store.selected_id = None;
    }
}

fn handle_drawing_keyboard_shortcuts(ui: &Ui, store: &mut DrawingStore) {
    if ui.ctx().egui_wants_keyboard_input() {
        return;
    }
    let (esc, del, undo, redo) = ui.input(|i| {
        let cmd_or_ctrl = i.modifiers.command || i.modifiers.ctrl;
        let esc = i.key_pressed(Key::Escape);
        let del = i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace);
        let undo = cmd_or_ctrl && !i.modifiers.shift && i.key_pressed(Key::Z);
        let redo = (cmd_or_ctrl && i.modifiers.shift && i.key_pressed(Key::Z))
            || (i.modifiers.ctrl && i.key_pressed(Key::Y));
        (esc, del, undo, redo)
    });

    if esc {
        let _ = store.cancel_or_deselect();
    } else if del {
        let _ = store.delete_selected();
    } else if redo {
        let _ = store.redo();
    } else if undo {
        let _ = store.undo();
    }
}

#[allow(clippy::too_many_arguments)]
fn drive_pane_drawing_input(
    store: &mut DrawingStore,
    pane: DrawingPaneTarget,
    plot_rect: Rect,
    plot_rect_f64: PlotRect,
    plot_resp: &egui::Response,
    ptr: DrawingPointerInput,
    proj: &PaneDrawingProjection<'_>,
    candles: &[Candle],
) -> bool {
    let to_local = |p: Pos2| {
        LocalPoint::new(
            f64::from(p.x) - plot_rect_f64.left,
            f64::from(p.y) - plot_rect_f64.top,
        )
    };

    let was_capturing_before = store.is_capturing_pane_gesture(pane);
    let mut claimed_press = false;
    let mut pressed_pos_this_frame: Option<Pos2> = None;

    if ptr.primary_pressed
        && let Some(pp) = ptr.press_origin.or(ptr.latest_pos)
        && plot_rect.contains(pp)
    {
        pressed_pos_this_frame = Some(pp);
        let outcome = store.pointer_down(pane, to_local(pp), ptr.snap_modifier, proj, candles);
        claimed_press = outcome == PointerCaptureOutcome::CapturedByDrawing;
    }

    let phase_on_pane = match store.phase() {
        DrawingInteractionPhase::Idle => false,
        DrawingInteractionPhase::PlacingSingle { pane: p, .. }
        | DrawingInteractionPhase::PlacingTwoAnchor { pane: p, .. }
        | DrawingInteractionPhase::DraggingHandle { pane: p, .. }
        | DrawingInteractionPhase::DraggingBody { pane: p, .. } => *p == pane,
    };
    let is_edit_drag_phase = matches!(
        store.phase(),
        DrawingInteractionPhase::DraggingHandle { .. }
            | DrawingInteractionPhase::DraggingBody { .. }
    );

    if let Some(lp) = ptr.latest_pos {
        let same_as_press = pressed_pos_this_frame.is_some_and(|pp| (lp - pp).length_sq() <= 1e-6);
        if plot_rect.contains(lp) || (phase_on_pane && ptr.primary_down) {
            if !(same_as_press && is_edit_drag_phase) {
                store.pointer_move(pane, to_local(lp), ptr.snap_modifier, proj, candles);
            }
        } else if !phase_on_pane {
            store.clear_hover_for_pane(pane);
        }
    } else if !phase_on_pane {
        store.clear_hover_for_pane(pane);
    }

    let should_release_on_pane = match store.phase() {
        DrawingInteractionPhase::PlacingSingle { pane: p, .. }
        | DrawingInteractionPhase::DraggingHandle { pane: p, .. }
        | DrawingInteractionPhase::DraggingBody { pane: p, .. } => *p == pane,
        DrawingInteractionPhase::PlacingTwoAnchor {
            pane: p,
            wait_for_second_press,
            second_press_active,
            ..
        } => *p == pane && (!*wait_for_second_press || *second_press_active),
        DrawingInteractionPhase::Idle => false,
    };

    if ptr.primary_released && should_release_on_pane {
        let _ = store.pointer_up(
            pane,
            ptr.latest_pos.map(to_local),
            ptr.snap_modifier,
            proj,
            candles,
        );
    } else if !claimed_press
        && !was_capturing_before
        && plot_resp.clicked()
        && store.active_tool == DrawingTool::Select
        && matches!(store.phase(), DrawingInteractionPhase::Idle)
    {
        let hit_any = ptr.latest_pos.is_some_and(|lp| {
            crate::drawings::hit_test_pane_drawings_causal(
                &store.drawings,
                pane,
                proj,
                to_local(lp),
                store.selected_id,
                store.replay_cutoff_timestamp(),
                candles.len(),
            )
            .is_some()
        });
        if !hit_any {
            store.on_empty_canvas_click();
        }
    }

    was_capturing_before || claimed_press || store.is_capturing_pane_gesture(pane)
}

fn apply_drawing_cursor_feedback(ui: &Ui, store: &DrawingStore) {
    match store.phase() {
        DrawingInteractionPhase::PlacingSingle { .. }
        | DrawingInteractionPhase::PlacingTwoAnchor { .. } => {
            ui.ctx().set_cursor_icon(CursorIcon::Crosshair);
        }
        DrawingInteractionPhase::DraggingHandle { .. }
        | DrawingInteractionPhase::DraggingBody { .. } => {
            ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
        }
        DrawingInteractionPhase::Idle => {
            if store.active_tool != DrawingTool::Select {
                ui.ctx().set_cursor_icon(CursorIcon::Crosshair);
            } else if let Some((_, hit)) = store.hovered_hit() {
                let icon = if hit.locked {
                    CursorIcon::PointingHand
                } else {
                    match hit.target {
                        HitTarget::Handle(_) => CursorIcon::PointingHand,
                        HitTarget::Body => CursorIcon::Grab,
                    }
                };
                ui.ctx().set_cursor_icon(icon);
            }
        }
    }
}

#[inline]
fn drawing_color32(preset: DrawingColorPreset) -> Color32 {
    let [r, g, b] = preset.rgb();
    Color32::from_rgb(r, g, b)
}

#[inline]
fn drawing_premultiplied_fill(preset: DrawingColorPreset, opacity: f32) -> Color32 {
    let alpha = opacity.clamp(0.0, 1.0);
    let [r, g, b] = preset.rgb();
    Color32::from_rgba_premultiplied(
        (f32::from(r) * alpha).round() as u8,
        (f32::from(g) * alpha).round() as u8,
        (f32::from(b) * alpha).round() as u8,
        (255.0 * alpha).round() as u8,
    )
}

#[inline]
fn local_to_screen_pos(plot_rect: Rect, pt: LocalPoint) -> Pos2 {
    Pos2::new(
        plot_rect.left() + (pt.x as f32),
        plot_rect.top() + (pt.y as f32),
    )
}

fn paint_styled_segment_phased(
    painter: &egui::Painter,
    p0: Pos2,
    p1: Pos2,
    phase_origin: Pos2,
    stroke: Stroke,
    dash: LineDashStyle,
) {
    match dash {
        LineDashStyle::Solid => {
            painter.line_segment([p0, p1], stroke);
        }
        LineDashStyle::Dashed | LineDashStyle::Dotted => {
            let delta = p1 - p0;
            let total_len = delta.length();
            if !total_len.is_finite() || total_len <= 0.5 {
                return;
            }
            let (dash_len, gap_len) = match dash {
                LineDashStyle::Dashed => (7.5_f32, 4.5_f32),
                LineDashStyle::Dotted => (2.5_f32, 4.5_f32),
                LineDashStyle::Solid => unreachable!(),
            };
            let dir = delta / total_len;
            let step = (dash_len + gap_len).max(1.0);
            let max_dashes = 1_024_usize;

            // Anchor dash phase to `phase_origin` so clipped lines/rays never shimmer ("marching ants") while panning!
            let s0 = if phase_origin.is_finite() {
                (p0 - phase_origin).dot(dir)
            } else {
                0.0
            };
            let phase = if s0.is_finite() {
                s0.rem_euclid(step)
            } else {
                0.0
            };

            let mut count = 0_usize;
            let is_horizontal = delta.y.abs() <= 1e-4;
            let is_vertical = delta.x.abs() <= 1e-4;

            if is_horizontal || is_vertical {
                let half_w = (stroke.width * 0.5).max(0.5);
                let est_dashes = ((total_len / step).ceil() as usize + 2).min(max_dashes);
                let mut mesh = Mesh::with_texture(egui::TextureId::default());
                mesh.vertices.reserve(est_dashes * 4);
                mesh.indices.reserve(est_dashes * 6);

                let mut emit_axis_dash = |d0: f32, d1: f32| {
                    let a = p0 + dir * d0;
                    let b = p0 + dir * d1;
                    let rect = if is_horizontal {
                        Rect::from_min_max(
                            Pos2::new(a.x.min(b.x), p0.y - half_w),
                            Pos2::new(a.x.max(b.x), p0.y + half_w),
                        )
                    } else {
                        Rect::from_min_max(
                            Pos2::new(p0.x - half_w, a.y.min(b.y)),
                            Pos2::new(p0.x + half_w, a.y.max(b.y)),
                        )
                    };
                    mesh.add_colored_rect(rect, stroke.color);
                };

                let mut dist = if phase < dash_len {
                    let first_end = (dash_len - phase).min(total_len);
                    if first_end > 0.15 {
                        emit_axis_dash(0.0, first_end);
                        count += 1;
                    }
                    step - phase
                } else {
                    step - phase
                };

                while dist < total_len && count < max_dashes {
                    let end_d = (dist + dash_len).min(total_len);
                    emit_axis_dash(dist, end_d);
                    dist += step;
                    count += 1;
                }

                if !mesh.is_empty() {
                    painter.add(Shape::mesh(mesh));
                }
            } else {
                let mut dist = if phase < dash_len {
                    let first_end = (dash_len - phase).min(total_len);
                    if first_end > 0.15 {
                        painter.line_segment([p0, p0 + dir * first_end], stroke);
                        count += 1;
                    }
                    step - phase
                } else {
                    step - phase
                };

                while dist < total_len && count < max_dashes {
                    let end_d = (dist + dash_len).min(total_len);
                    painter.line_segment([p0 + dir * dist, p0 + dir * end_d], stroke);
                    dist += step;
                    count += 1;
                }
            }
        }
    }
}

fn format_compact_qty(qty: f64) -> String {
    if (qty - qty.round()).abs() < 1e-6 {
        format!("{:.0}", qty.round())
    } else if (qty * 100.0 - (qty * 100.0).round()).abs() < 1e-4 {
        format!("{qty:.2}")
    } else {
        format!("{qty:.4}")
    }
}

fn paint_replay_frontier(
    main_plot_painter: &egui::Painter,
    painter: &egui::Painter,
    multi_layout: &MultiPaneLayout,
    candles: &[Candle],
    main_vp: &PaneViewport<'_>,
    bar_spacing: f64,
    status_label: Option<&str>,
) {
    if candles.is_empty() {
        return;
    }
    let last_idx = candles.len() - 1;
    if let Some(last_cx) = main_vp.index_to_screen_x(last_idx as i64) {
        let frontier_x = (last_cx + bar_spacing * 0.5) as f32;
        if frontier_x > multi_layout.main_plot_rect.left()
            && frontier_x < multi_layout.main_plot_rect.right()
        {
            let stroke = Stroke::new(1.25, palette::REPLAY_CUT_GOLD.gamma_multiply(0.65));
            let p0 = Pos2::new(frontier_x, multi_layout.main_plot_rect.top());
            let p1 = Pos2::new(frontier_x, multi_layout.main_plot_rect.bottom());
            paint_styled_segment_phased(
                main_plot_painter,
                p0,
                p1,
                p0,
                stroke,
                LineDashStyle::Dashed,
            );
            for osc in &multi_layout.oscillators {
                let osc_p = painter.with_clip_rect(osc.plot_rect);
                let op0 = Pos2::new(frontier_x, osc.plot_rect.top());
                let op1 = Pos2::new(frontier_x, osc.plot_rect.bottom());
                paint_styled_segment_phased(&osc_p, op0, op1, op0, stroke, LineDashStyle::Dashed);
            }
        }
    }

    let badge_text = format!(
        "REPLAY · {} · {}",
        status_label.unwrap_or("ACTIVE"),
        candles[last_idx].date_label
    );
    let font = FontId::monospace(10.5);
    let galley = painter.layout_no_wrap(badge_text, font, palette::REPLAY_CUT_GOLD);
    let pad = Vec2::new(7.0, 3.5);
    let rect = Rect::from_min_size(
        Pos2::new(
            (multi_layout.main_plot_rect.right() - galley.size().x - pad.x * 2.0 - 8.0)
                .max(multi_layout.main_plot_rect.left() + 8.0),
            (multi_layout.main_plot_rect.bottom() - galley.size().y - pad.y * 2.0 - 8.0)
                .max(multi_layout.main_plot_rect.top() + 8.0),
        ),
        galley.size() + pad * 2.0,
    );
    main_plot_painter.rect_filled(rect, 4.0, palette::BG_FLOATING_HUD);
    main_plot_painter.rect_stroke(
        rect,
        4.0,
        Stroke::new(1.0, palette::REPLAY_CUT_GOLD.gamma_multiply(0.75)),
        StrokeKind::Inside,
    );
    main_plot_painter.galley(rect.min + pad, galley, palette::REPLAY_CUT_GOLD);
}

fn paint_replay_cut_scrubber(
    painter: &egui::Painter,
    main_plot_painter: &egui::Painter,
    multi_layout: &MultiPaneLayout,
    candles: &[Candle],
    scrubber_idx: usize,
    main_vp: &PaneViewport<'_>,
    bar_spacing: f64,
) {
    if candles.is_empty() {
        return;
    }
    let idx = scrubber_idx.min(candles.len() - 1);
    let Some(cx) = main_vp.index_to_screen_x(idx as i64) else {
        return;
    };
    let cut_x = ((cx + bar_spacing * 0.5) as f32).clamp(
        multi_layout.main_plot_rect.left(),
        multi_layout.main_plot_rect.right(),
    );

    if cut_x < multi_layout.main_plot_rect.right() {
        let mask_rect = Rect::from_min_max(
            Pos2::new(cut_x, multi_layout.main_plot_rect.top()),
            Pos2::new(
                multi_layout.main_plot_rect.right(),
                multi_layout.main_plot_rect.bottom(),
            ),
        );
        main_plot_painter.rect_filled(mask_rect, 0.0, palette::REPLAY_MASK_CURTAIN);
    }
    main_plot_painter.line_segment(
        [
            Pos2::new(cut_x, multi_layout.main_plot_rect.top()),
            Pos2::new(cut_x, multi_layout.main_plot_rect.bottom()),
        ],
        Stroke::new(2.0, palette::REPLAY_CUT_GOLD),
    );

    for osc in &multi_layout.oscillators {
        let osc_p = painter.with_clip_rect(osc.plot_rect);
        if cut_x < osc.plot_rect.right() {
            let osc_mask = Rect::from_min_max(
                Pos2::new(cut_x, osc.plot_rect.top()),
                Pos2::new(osc.plot_rect.right(), osc.plot_rect.bottom()),
            );
            osc_p.rect_filled(osc_mask, 0.0, palette::REPLAY_MASK_CURTAIN);
        }
        osc_p.line_segment(
            [
                Pos2::new(cut_x, osc.plot_rect.top()),
                Pos2::new(cut_x, osc.plot_rect.bottom()),
            ],
            Stroke::new(1.8, palette::REPLAY_CUT_GOLD),
        );
    }

    let label = format!("<< CUT REPLAY HERE · {}", candles[idx].date_label);
    let font = FontId::monospace(11.0);
    let galley = painter.layout_no_wrap(label, font, Color32::from_rgb(18, 20, 26));
    let pad = Vec2::new(8.0, 4.0);
    let pill_w = galley.size().x + pad.x * 2.0;
    let pill_h = galley.size().y + pad.y * 2.0;
    let min_x = (cut_x - pill_w * 0.5).clamp(
        multi_layout.main_plot_rect.left() + 6.0,
        (multi_layout.main_plot_rect.right() - pill_w - 6.0)
            .max(multi_layout.main_plot_rect.left() + 6.0),
    );
    let pill_rect = Rect::from_min_size(
        Pos2::new(min_x, multi_layout.main_plot_rect.top() + 10.0),
        Vec2::new(pill_w, pill_h),
    );
    main_plot_painter.rect_filled(pill_rect, 4.0, palette::REPLAY_CUT_GOLD);
    main_plot_painter.galley(pill_rect.min + pad, galley, Color32::from_rgb(18, 20, 26));
}

#[allow(clippy::too_many_arguments)]
fn paint_trading_price_line_and_pill(
    axis_painter: &egui::Painter,
    plot_painter: &egui::Painter,
    plot_rect: Rect,
    price_axis_rect: Rect,
    price: f64,
    color: Color32,
    dash: LineDashStyle,
    stroke_width: f32,
    inline_badge: &str,
    price_scale: &PriceScale,
    main_vp: &PaneViewport<'_>,
    base_value: f64,
) -> bool {
    let Some(sy) = main_vp.price_to_screen_y(price) else {
        return false;
    };
    let y = sy as f32;
    if y < plot_rect.top() || y > plot_rect.bottom() {
        return false;
    }

    let p0 = Pos2::new(plot_rect.left(), y);
    let p1 = Pos2::new(plot_rect.right(), y);
    paint_styled_segment_phased(
        plot_painter,
        p0,
        p1,
        p0,
        Stroke::new(stroke_width, color.gamma_multiply(0.90)),
        dash,
    );

    if !inline_badge.is_empty() && plot_rect.width() > 160.0 {
        let font = FontId::monospace(10.0);
        let galley = axis_painter.layout_no_wrap(inline_badge.to_owned(), font, Color32::WHITE);
        let pad = Vec2::new(5.5, 2.5);
        let badge_w = galley.size().x + pad.x * 2.0;
        let badge_h = galley.size().y + pad.y * 2.0;
        let badge_x = (plot_rect.right() - badge_w - 10.0).max(plot_rect.left() + 8.0);
        let badge_y = (y - badge_h * 0.5).clamp(
            plot_rect.top() + 2.0,
            (plot_rect.bottom() - badge_h - 2.0).max(plot_rect.top() + 2.0),
        );
        let badge_rect =
            Rect::from_min_size(Pos2::new(badge_x, badge_y), Vec2::new(badge_w, badge_h));
        plot_painter.rect_filled(
            badge_rect,
            3.0,
            Color32::from_rgba_unmultiplied(15, 19, 28, 232),
        );
        plot_painter.rect_stroke(
            badge_rect,
            3.0,
            Stroke::new(1.0, color.gamma_multiply(0.85)),
            StrokeKind::Inside,
        );
        plot_painter.galley(badge_rect.min + pad, galley, Color32::WHITE);
    }

    let pill_y = y.clamp(price_axis_rect.top() + 9.5, price_axis_rect.bottom() - 9.5);
    let pill_rect = Rect::from_min_max(
        Pos2::new(price_axis_rect.left() + 2.0, pill_y - 9.0),
        Pos2::new(price_axis_rect.right() - 4.0, pill_y + 9.0),
    );
    axis_painter.rect_filled(pill_rect, 3.0, color);
    axis_painter.text(
        Pos2::new(price_axis_rect.left() + 6.0, pill_y),
        Align2::LEFT_CENTER,
        price_scale.format_price(price, base_value),
        FontId::monospace(10.5),
        Color32::WHITE,
    );
    true
}

#[allow(clippy::too_many_arguments)]
fn paint_execution_fill_markers(
    plot_painter: &egui::Painter,
    plot_rect: Rect,
    candles: &[Candle],
    culling_range: std::ops::Range<usize>,
    conflation_factor: usize,
    inverted_scale: bool,
    bar_spacing: f64,
    main_vp: &PaneViewport<'_>,
    fills: &[ExecutionFill],
) -> usize {
    if fills.is_empty() || candles.is_empty() || culling_range.is_empty() {
        return 0;
    }
    let cf = conflation_factor.max(1);
    let aligned_start = if cf > 1 {
        (culling_range.start / cf) * cf
    } else {
        culling_range.start
    }
    .min(candles.len());
    let aligned_end = if cf > 1 {
        culling_range
            .end
            .div_ceil(cf)
            .saturating_mul(cf)
            .min(candles.len())
    } else {
        culling_range.end.min(candles.len())
    };
    if aligned_start >= aligned_end {
        return 0;
    }

    let min_ts = candles[aligned_start].timestamp;
    let max_excl_ts = (aligned_end < candles.len()).then(|| candles[aligned_end].timestamp);
    let fill_start = fills.partition_point(|f| f.timestamp < min_ts);
    let fill_end = match max_excl_ts {
        Some(t) => fills.partition_point(|f| f.timestamp < t),
        None => fills.len(),
    };
    if fill_start >= fill_end {
        return 0;
    }

    let mut candle_cursor = aligned_start;
    let mut rendered_markers = 0_usize;
    let arrow_cap = (fill_end - fill_start).min(256);
    let mut arrow_mesh = Mesh::with_texture(egui::TextureId::default());
    arrow_mesh.vertices.reserve(arrow_cap * 6);
    arrow_mesh.indices.reserve(arrow_cap * 6);
    let border_col = Color32::from_rgba_unmultiplied(10, 14, 20, 220);

    let emit_arrow_triangle = |mesh: &mut Mesh,
                               tip: Pos2,
                               b_left: Pos2,
                               b_right: Pos2,
                               outward: f32,
                               fill_col: Color32| {
        // Crisp 1px dark halo triangle behind the solid fill triangle (zero per-arrow Vec alloc!)
        let o_tip = Pos2::new(tip.x, tip.y - outward * 1.0);
        let o_bl = Pos2::new(b_left.x - 0.9, b_left.y + outward * 0.8);
        let o_br = Pos2::new(b_right.x + 0.9, b_right.y + outward * 0.8);
        let base = mesh.vertices.len() as u32;
        mesh.colored_vertex(o_tip, border_col);
        mesh.colored_vertex(o_bl, border_col);
        mesh.colored_vertex(o_br, border_col);
        mesh.add_triangle(base, base + 1, base + 2);

        let ibase = mesh.vertices.len() as u32;
        mesh.colored_vertex(tip, fill_col);
        mesh.colored_vertex(b_left, fill_col);
        mesh.colored_vertex(b_right, fill_col);
        mesh.add_triangle(ibase, ibase + 1, ibase + 2);
    };

    let mut cur_bucket_id: Option<usize> = None;
    let mut buy_count_in_bucket = 0_usize;
    let mut sell_count_in_bucket = 0_usize;
    let mut last_bucket_geom: Option<(f32, f32, f32)> = None; // (cx, low_y, high_y)

    let flush_overflow = |painter: &egui::Painter,
                          geom: Option<(f32, f32, f32)>,
                          buy_cnt: usize,
                          sell_cnt: usize| {
        let Some((cx, low_y, high_y)) = geom else {
            return;
        };
        if buy_cnt > 3 {
            let dir = if inverted_scale { -1.0_f32 } else { 1.0_f32 };
            let oy = (low_y + dir * (10.0 + 3.0 * 11.0))
                .clamp(plot_rect.top() + 8.0, plot_rect.bottom() - 8.0);
            painter.text(
                Pos2::new(cx, oy),
                Align2::CENTER_CENTER,
                format!("+{}", buy_cnt - 3),
                FontId::monospace(9.5),
                palette::MARKER_BUY_ARROW,
            );
        }
        if sell_cnt > 3 {
            let dir = if inverted_scale { 1.0_f32 } else { -1.0_f32 };
            let oy = (high_y + dir * (10.0 + 3.0 * 11.0))
                .clamp(plot_rect.top() + 8.0, plot_rect.bottom() - 8.0);
            painter.text(
                Pos2::new(cx, oy),
                Align2::CENTER_CENTER,
                format!("+{}", sell_cnt - 3),
                FontId::monospace(9.5),
                palette::MARKER_SELL_ARROW,
            );
        }
    };

    for fill in &fills[fill_start..fill_end] {
        let step = candles[candle_cursor..aligned_end]
            .partition_point(|c| c.timestamp <= fill.timestamp)
            .saturating_sub(1);
        candle_cursor += step;
        let bucket_id = candle_cursor / cf;

        if cur_bucket_id != Some(bucket_id) {
            flush_overflow(
                plot_painter,
                last_bucket_geom,
                buy_count_in_bucket,
                sell_count_in_bucket,
            );
            cur_bucket_id = Some(bucket_id);
            buy_count_in_bucket = 0;
            sell_count_in_bucket = 0;

            let (logical_x, bar_high, bar_low) = if cf > 1 {
                if let Some(b) = conflated_candle_at(candles, candle_cursor, cf) {
                    (b.center_logical_index, b.high, b.low)
                } else {
                    let c = &candles[candle_cursor];
                    (candle_cursor as f64, c.high, c.low)
                }
            } else {
                let c = &candles[candle_cursor];
                (candle_cursor as f64, c.high, c.low)
            };

            let sx = main_vp.float_index_to_screen_x(logical_x);
            let hy = main_vp.price_to_screen_y(bar_high);
            let ly = main_vp.price_to_screen_y(bar_low);
            last_bucket_geom = match (sx, ly, hy) {
                (Some(x), Some(l), Some(h)) => Some((x as f32, l as f32, h as f32)),
                _ => None,
            };
        }

        let Some((cx, low_y, high_y)) = last_bucket_geom else {
            continue;
        };
        if cx < plot_rect.left() - 8.0 || cx > plot_rect.right() + 8.0 {
            continue;
        }

        let half_w = (bar_spacing as f32 * 0.42).clamp(4.2, 6.8);
        let arrow_h = (half_w * 1.35).clamp(5.8, 9.0);

        match fill.side {
            OrderSide::Buy => {
                let slot = buy_count_in_bucket;
                buy_count_in_bucket += 1;
                if slot < 3 {
                    // `belowBar` (`low` side), pointing inward toward the candle!
                    let outward = if inverted_scale { -1.0_f32 } else { 1.0_f32 };
                    let tip_y = (low_y + outward * (5.5 + (slot as f32) * 11.5))
                        .clamp(plot_rect.top() + 10.0, plot_rect.bottom() - 10.0);
                    let base_y = tip_y + outward * arrow_h;
                    emit_arrow_triangle(
                        &mut arrow_mesh,
                        Pos2::new(cx, tip_y),
                        Pos2::new(cx - half_w, base_y),
                        Pos2::new(cx + half_w, base_y),
                        outward,
                        palette::MARKER_BUY_ARROW,
                    );
                    if bar_spacing >= 12.0 && slot == 0 {
                        plot_painter.text(
                            Pos2::new(cx, base_y + outward * 6.5),
                            Align2::CENTER_CENTER,
                            fill.reason.short_label(),
                            FontId::monospace(9.0),
                            palette::MARKER_BUY_ARROW,
                        );
                    }
                    rendered_markers += 1;
                }
            }
            OrderSide::Sell => {
                let slot = sell_count_in_bucket;
                sell_count_in_bucket += 1;
                if slot < 3 {
                    // `aboveBar` (`high` side), pointing inward toward the candle!
                    let outward = if inverted_scale { 1.0_f32 } else { -1.0_f32 };
                    let tip_y = (high_y + outward * (5.5 + (slot as f32) * 11.5))
                        .clamp(plot_rect.top() + 10.0, plot_rect.bottom() - 10.0);
                    let base_y = tip_y + outward * arrow_h;
                    emit_arrow_triangle(
                        &mut arrow_mesh,
                        Pos2::new(cx, tip_y),
                        Pos2::new(cx - half_w, base_y),
                        Pos2::new(cx + half_w, base_y),
                        outward,
                        palette::MARKER_SELL_ARROW,
                    );
                    if bar_spacing >= 12.0 && slot == 0 {
                        plot_painter.text(
                            Pos2::new(cx, base_y + outward * 6.5),
                            Align2::CENTER_CENTER,
                            fill.reason.short_label(),
                            FontId::monospace(9.0),
                            palette::MARKER_SELL_ARROW,
                        );
                    }
                    rendered_markers += 1;
                }
            }
        }
    }

    if !arrow_mesh.is_empty() {
        plot_painter.add(Shape::mesh(arrow_mesh));
    }

    flush_overflow(
        plot_painter,
        last_bucket_geom,
        buy_count_in_bucket,
        sell_count_in_bucket,
    );
    rendered_markers
}

#[allow(clippy::too_many_arguments)]
fn paint_trading_overlays(
    axis_painter: &egui::Painter,
    plot_painter: &egui::Painter,
    plot_rect: Rect,
    price_axis_rect: Rect,
    candles: &[Candle],
    culling_range: std::ops::Range<usize>,
    conflation_factor: usize,
    price_scale: &PriceScale,
    main_vp: &PaneViewport<'_>,
    base_value: f64,
    session: &TradingSession,
) -> (usize, usize) {
    let mut price_lines = 0_usize;

    // 1. Working (`Pending`) Limit & Stop orders
    for order in session.pending_orders() {
        if let Some(trig_price) = order.kind.trigger_price() {
            let color = match order.side {
                OrderSide::Buy => palette::BULLISH,
                OrderSide::Sell => palette::BEARISH,
            };
            let badge = format!(
                "{} {} {}",
                order.side.short_label(),
                order.kind.short_label(),
                format_compact_qty(order.quantity)
            );
            if paint_trading_price_line_and_pill(
                axis_painter,
                plot_painter,
                plot_rect,
                price_axis_rect,
                trig_price,
                color,
                LineDashStyle::Dotted,
                1.2,
                &badge,
                price_scale,
                main_vp,
                base_value,
            ) {
                price_lines += 1;
            }
        }
    }

    // 2. Active Position Entry, Stop Loss, and Take Profit lines
    if let Some(pos) = session.open_position() {
        let mark_price = candles.last().map_or(pos.avg_entry_price, |c| c.close);
        let unreal_pnl = pos.unrealized_pnl(mark_price);
        let unreal_pct = pos.unrealized_return_pct(mark_price);
        let side_tag = match pos.side {
            PositionSide::Long => "LONG",
            PositionSide::Short => "SHORT",
        };
        let entry_badge = format!(
            "{} {}  {:+.2} ({:+.2}%)",
            side_tag,
            format_compact_qty(pos.quantity),
            unreal_pnl,
            unreal_pct
        );
        if paint_trading_price_line_and_pill(
            axis_painter,
            plot_painter,
            plot_rect,
            price_axis_rect,
            pos.avg_entry_price,
            palette::TRADE_ENTRY_BLUE,
            LineDashStyle::Dashed,
            1.55,
            &entry_badge,
            price_scale,
            main_vp,
            base_value,
        ) {
            price_lines += 1;
        }

        if let Some(sl) = pos.stop_loss {
            let sl_pnl = pos.unrealized_pnl(sl);
            let sl_badge = format!("SL  {:+.2}", sl_pnl);
            if paint_trading_price_line_and_pill(
                axis_painter,
                plot_painter,
                plot_rect,
                price_axis_rect,
                sl,
                palette::TRADE_STOP_LOSS,
                LineDashStyle::Dashed,
                1.35,
                &sl_badge,
                price_scale,
                main_vp,
                base_value,
            ) {
                price_lines += 1;
            }
        }

        if let Some(tp) = pos.take_profit {
            let tp_pnl = pos.unrealized_pnl(tp);
            let tp_badge = format!("TP  {:+.2}", tp_pnl);
            if paint_trading_price_line_and_pill(
                axis_painter,
                plot_painter,
                plot_rect,
                price_axis_rect,
                tp,
                palette::TRADE_TAKE_PROFIT,
                LineDashStyle::Dashed,
                1.35,
                &tp_badge,
                price_scale,
                main_vp,
                base_value,
            ) {
                price_lines += 1;
            }
        }
    }

    // 3. Execution Fill Markers (`arrowUp` / `arrowDown`)
    let marker_count = paint_execution_fill_markers(
        plot_painter,
        plot_rect,
        candles,
        culling_range,
        conflation_factor,
        price_scale.is_inverted(),
        main_vp.time_scale.bar_spacing(),
        main_vp,
        session.fills(),
    );

    (marker_count, price_lines)
}

#[allow(clippy::too_many_arguments)]
fn paint_pane_drawings(
    axis_painter: &egui::Painter,
    plot_painter: &egui::Painter,
    plot_rect: Rect,
    price_axis_rect: Rect,
    pane: DrawingPaneTarget,
    store: &DrawingStore,
    proj: &PaneDrawingProjection<'_>,
    price_scale: &PriceScale,
    base_value: f64,
    visible_candle_count: usize,
) {
    let cutoff = store.replay_cutoff_timestamp();
    for drawing in &store.drawings {
        if !drawing.is_causally_visible(cutoff, visible_candle_count) || drawing.pane != pane {
            continue;
        }
        let is_selected = store.selected_id == Some(drawing.id);
        let is_hovered = store
            .hovered_hit()
            .is_some_and(|(hp, hm)| hp == pane && hm.drawing_id == drawing.id);
        paint_single_drawing(
            axis_painter,
            plot_painter,
            plot_rect,
            price_axis_rect,
            drawing,
            proj,
            price_scale,
            base_value,
            is_selected,
            is_hovered,
            false,
        );
    }

    if let Some(preview) = store.preview_drawing()
        && preview.pane == pane
    {
        paint_single_drawing(
            axis_painter,
            plot_painter,
            plot_rect,
            price_axis_rect,
            &preview,
            proj,
            price_scale,
            base_value,
            true,
            false,
            true,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_single_drawing(
    axis_painter: &egui::Painter,
    plot_painter: &egui::Painter,
    plot_rect: Rect,
    price_axis_rect: Rect,
    drawing: &Drawing,
    proj: &PaneDrawingProjection<'_>,
    price_scale: &PriceScale,
    base_value: f64,
    is_selected: bool,
    is_hovered: bool,
    is_preview: bool,
) {
    let Some(projected) = proj.project_drawing(drawing) else {
        return;
    };
    let base_color = drawing_color32(drawing.style.color);
    let stroke_w = if is_selected {
        (drawing.style.line_width + 0.45).min(8.5)
    } else {
        drawing.style.line_width
    };
    let dash = if is_preview && drawing.style.dash == LineDashStyle::Solid {
        LineDashStyle::Dashed
    } else {
        drawing.style.dash
    };
    let primary_stroke = Stroke::new(stroke_w, base_color);
    let highlight_glow = (is_selected || is_hovered).then(|| {
        Stroke::new(
            stroke_w + 3.2,
            drawing_premultiplied_fill(drawing.style.color, 0.22),
        )
    });

    let bar0_screen_x = proj
        .time_scale
        .index_to_local_x(0)
        .filter(|x| x.is_finite() && x.abs() <= 1.0e6)
        .map_or(plot_rect.left(), |x| plot_rect.left() + (x as f32));

    match &projected.shape {
        ProjectedDrawingShape::HorizontalLine {
            y,
            value,
            visible_segment,
        } => {
            if let Some(seg) = visible_segment {
                let p0 = local_to_screen_pos(plot_rect, seg.start);
                let p1 = local_to_screen_pos(plot_rect, seg.end);
                if let Some(glow) = highlight_glow {
                    plot_painter.line_segment([p0, p1], glow);
                }
                paint_styled_segment_phased(
                    plot_painter,
                    p0,
                    p1,
                    Pos2::new(bar0_screen_x, p0.y),
                    primary_stroke,
                    dash,
                );

                let formatted = price_scale.format_price(*value, base_value);
                let label_text = if drawing.locked {
                    format!("LOCK {formatted}")
                } else {
                    formatted.clone()
                };
                let label_y = (plot_rect.top() + (*y as f32) - 4.0)
                    .clamp(plot_rect.top() + 10.0, plot_rect.bottom() - 4.0);
                plot_painter.text(
                    Pos2::new(plot_rect.right() - 8.0, label_y),
                    Align2::RIGHT_BOTTOM,
                    label_text,
                    FontId::monospace(10.5),
                    base_color,
                );

                if is_selected {
                    let sy = (plot_rect.top() + (*y as f32))
                        .clamp(price_axis_rect.top() + 9.5, price_axis_rect.bottom() - 9.5);
                    let badge_rect = Rect::from_min_max(
                        Pos2::new(price_axis_rect.left() + 2.0, sy - 8.5),
                        Pos2::new(price_axis_rect.right() - 4.0, sy + 8.5),
                    );
                    axis_painter.rect_filled(badge_rect, 3.0, base_color);
                    axis_painter.text(
                        Pos2::new(price_axis_rect.left() + 6.0, sy),
                        Align2::LEFT_CENTER,
                        formatted,
                        FontId::monospace(10.5),
                        palette::BG_WORKSTATION,
                    );
                }
            }
        }
        ProjectedDrawingShape::Segment {
            raw_start, clipped, ..
        } => {
            if let Some(seg) = clipped {
                let p0 = local_to_screen_pos(plot_rect, seg.start);
                let p1 = local_to_screen_pos(plot_rect, seg.end);
                let raw_origin_screen = local_to_screen_pos(plot_rect, *raw_start);
                if let Some(glow) = highlight_glow {
                    plot_painter.line_segment([p0, p1], glow);
                }
                paint_styled_segment_phased(
                    plot_painter,
                    p0,
                    p1,
                    raw_origin_screen,
                    primary_stroke,
                    dash,
                );

                // Subtle origin ring on `Ray` so directional rays are visually distinct from finite `TrendLine`s.
                if matches!(drawing.geometry, DrawingGeometry::Ray { .. })
                    && plot_rect.expand(4.0).contains(raw_origin_screen)
                {
                    plot_painter.circle_stroke(
                        raw_origin_screen,
                        3.5,
                        Stroke::new(1.4, base_color),
                    );
                }
            }
        }
        ProjectedDrawingShape::Rectangle {
            raw_rect,
            clipped_fill,
            clipped_edges,
        } => {
            if let Some(fill_rect) = clipped_fill
                && drawing.style.fill_opacity > 0.001
            {
                let screen_fill = Rect::from_min_max(
                    local_to_screen_pos(plot_rect, LocalPoint::new(fill_rect.left, fill_rect.top)),
                    local_to_screen_pos(
                        plot_rect,
                        LocalPoint::new(fill_rect.right, fill_rect.bottom),
                    ),
                );
                let mut mesh = Mesh::default();
                mesh.add_colored_rect(
                    screen_fill,
                    drawing_premultiplied_fill(drawing.style.color, drawing.style.fill_opacity),
                );
                plot_painter.add(Shape::mesh(mesh));
            }

            let raw_tl =
                local_to_screen_pos(plot_rect, LocalPoint::new(raw_rect.left, raw_rect.top));
            for edge in clipped_edges.iter().flatten() {
                let p0 = local_to_screen_pos(plot_rect, edge.start);
                let p1 = local_to_screen_pos(plot_rect, edge.end);
                if let Some(glow) = highlight_glow {
                    plot_painter.line_segment([p0, p1], glow);
                }
                paint_styled_segment_phased(plot_painter, p0, p1, raw_tl, primary_stroke, dash);
            }
        }
        ProjectedDrawingShape::Fibonacci {
            control_segment,
            levels,
            bands,
        } => {
            if !bands.is_empty() && drawing.style.fill_opacity > 0.001 {
                let mut band_mesh = Mesh::default();
                for band in bands {
                    let opacity = if band.band_index.is_multiple_of(2) {
                        drawing.style.fill_opacity
                    } else {
                        drawing.style.fill_opacity * 0.50
                    };
                    let screen_band = Rect::from_min_max(
                        local_to_screen_pos(
                            plot_rect,
                            LocalPoint::new(band.rect.left, band.rect.top),
                        ),
                        local_to_screen_pos(
                            plot_rect,
                            LocalPoint::new(band.rect.right, band.rect.bottom),
                        ),
                    );
                    band_mesh.add_colored_rect(
                        screen_band,
                        drawing_premultiplied_fill(drawing.style.color, opacity),
                    );
                }
                plot_painter.add(Shape::mesh(band_mesh));
            }

            if let Some(ctrl) = control_segment {
                let c0 = local_to_screen_pos(plot_rect, ctrl.start);
                let c1 = local_to_screen_pos(plot_rect, ctrl.end);
                let ctrl_origin = projected
                    .anchors_local
                    .first()
                    .map_or(c0, |pt| local_to_screen_pos(plot_rect, *pt));
                paint_styled_segment_phased(
                    plot_painter,
                    c0,
                    c1,
                    ctrl_origin,
                    Stroke::new(1.15, base_color.gamma_multiply(0.75)),
                    LineDashStyle::Dashed,
                );
            }

            for lvl in levels {
                if !lvl.visible_in_pane {
                    continue;
                }
                let lp0 = local_to_screen_pos(plot_rect, LocalPoint::new(lvl.x_left, lvl.y));
                let lp1 = local_to_screen_pos(plot_rect, LocalPoint::new(lvl.x_right, lvl.y));
                let is_key_ratio = (lvl.ratio - 0.0).abs() < 1e-6
                    || (lvl.ratio - 0.5).abs() < 1e-6
                    || (lvl.ratio - 0.618).abs() < 1e-6
                    || (lvl.ratio - 1.0).abs() < 1e-6;
                let lvl_w = if is_key_ratio {
                    stroke_w
                } else {
                    (stroke_w * 0.85).max(1.0)
                };
                if let Some(glow) = highlight_glow {
                    plot_painter.line_segment([lp0, lp1], glow);
                }
                paint_styled_segment_phased(
                    plot_painter,
                    lp0,
                    lp1,
                    Pos2::new(bar0_screen_x, lp0.y),
                    Stroke::new(lvl_w, base_color),
                    drawing.style.dash,
                );
            }

            // Collision-free Fibonacci level labels prioritized by structural significance (`0.0, 1.0, 0.5, 0.618, ...`)
            // so compressed or zero-height retracements never pile overlapping text on the same pixel row!
            const LABEL_PRIORITY_ORDER: [usize; 7] = [0, 6, 3, 4, 2, 1, 5];
            let mut drawn_label_ys = [0.0_f32; 7];
            let mut drawn_count = 0_usize;
            for &lvl_idx in &LABEL_PRIORITY_ORDER {
                let Some(lvl) = levels.get(lvl_idx) else {
                    continue;
                };
                if !lvl.visible_in_pane {
                    continue;
                }
                let lp0 = local_to_screen_pos(plot_rect, LocalPoint::new(lvl.x_left, lvl.y));
                let lp1 = local_to_screen_pos(plot_rect, LocalPoint::new(lvl.x_right, lvl.y));
                if (lp1.x - lp0.x) < 54.0 {
                    continue;
                }
                let ty = (lp0.y - 2.0).clamp(plot_rect.top() + 9.0, plot_rect.bottom() - 2.0);
                if drawn_label_ys[..drawn_count]
                    .iter()
                    .any(|&py| (ty - py).abs() < 11.5)
                {
                    continue;
                }
                drawn_label_ys[drawn_count] = ty;
                drawn_count += 1;

                let ratio_str = if (lvl.ratio - 0.0).abs() < 1e-6
                    || (lvl.ratio - 0.5).abs() < 1e-6
                    || (lvl.ratio - 1.0).abs() < 1e-6
                {
                    format!("{:.1}", lvl.ratio)
                } else {
                    format!("{:.3}", lvl.ratio)
                };
                let txt = format!(
                    "{ratio_str} ({})",
                    price_scale.format_price(lvl.value, base_value)
                );
                plot_painter.text(
                    Pos2::new(lp0.x + 5.0, ty),
                    Align2::LEFT_BOTTOM,
                    txt,
                    FontId::monospace(10.0),
                    base_color,
                );
            }
        }
    }

    // Paint crisp anchor / corner handles when selected, hovered, or actively placing a preview!
    if (is_selected || is_hovered || is_preview) && !projected.locked {
        for handle in &projected.handles {
            let hp = local_to_screen_pos(plot_rect, handle.pos);
            if !plot_rect.expand(6.0).contains(hp) {
                continue;
            }
            match handle.part {
                HandlePart::Anchor(_) => {
                    plot_painter.circle_filled(hp, 4.6, palette::BG_CHART_CANVAS);
                    plot_painter.circle_stroke(hp, 4.6, Stroke::new(2.0, base_color));
                    if is_selected || is_preview {
                        plot_painter.circle_filled(hp, 1.8, Color32::WHITE);
                    }
                }
                HandlePart::RectCorner(_) => {
                    let sq = Rect::from_center_size(hp, Vec2::splat(8.4));
                    plot_painter.rect_filled(sq, 1.5, palette::BG_CHART_CANVAS);
                    plot_painter.rect_stroke(
                        sq,
                        1.5,
                        Stroke::new(1.8, base_color),
                        StrokeKind::Inside,
                    );
                }
            }
        }
    } else if is_selected
        && projected.locked
        && let Some(&first_pt) = projected.anchors_local.first()
    {
        let lp = local_to_screen_pos(plot_rect, first_pt);
        let badge_pos = Pos2::new(
            lp.x.clamp(plot_rect.left() + 28.0, plot_rect.right() - 28.0),
            (lp.y - 10.0).clamp(plot_rect.top() + 12.0, plot_rect.bottom() - 12.0),
        );
        plot_painter.text(
            badge_pos,
            Align2::CENTER_CENTER,
            "[LOCKED]",
            FontId::monospace(10.0),
            base_color,
        );
    }
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
    fn multipane_layout_and_separator_stretch_transfer_preserve_uninvolved_panes() {
        let outer = Rect::from_min_size(Pos2::new(0.0, 40.0), Vec2::new(1280.0, 760.0));
        let mut vp = ChartViewportState::new();
        let active = [
            (OscillatorPaneId::Rsi, vp.rsi_stretch),
            (OscillatorPaneId::Macd, vp.macd_stretch),
            (OscillatorPaneId::Atr, vp.atr_stretch),
        ];
        let before = MultiPaneLayout::compute(outer, vp.main_stretch, &active);
        assert_eq!(before.oscillators.len(), 3);
        assert_eq!(before.separators.len(), 3);
        assert!(
            (before.oscillators[2].plot_rect.bottom() - before.time_axis_rect.top()).abs() < 1e-4
        );

        // Drag separator 0 (between Main and RSI) DOWN by +60px -> Main grows, RSI shrinks,
        // while MACD and ATR heights stay 100% unchanged!
        let combined_h = before.main_plot_rect.height() + before.oscillators[0].plot_rect.height();
        vp.apply_separator_drag_delta(None, OscillatorPaneId::Rsi, combined_h, 60.0);
        let after_specs = [
            (OscillatorPaneId::Rsi, vp.rsi_stretch),
            (OscillatorPaneId::Macd, vp.macd_stretch),
            (OscillatorPaneId::Atr, vp.atr_stretch),
        ];
        let after = MultiPaneLayout::compute(outer, vp.main_stretch, &after_specs);

        assert!(after.main_plot_rect.height() > before.main_plot_rect.height() + 50.0);
        assert!(after.oscillators[0].plot_rect.height() >= MultiPaneLayout::MIN_PANE_HEIGHT - 0.1);
        assert!(
            (after.oscillators[1].plot_rect.height() - before.oscillators[1].plot_rect.height())
                .abs()
                < 1e-3,
            "uninvolved MACD pane height must remain invariant when resizing Main/RSI separator"
        );
        assert!(
            (after.oscillators[2].plot_rect.height() - before.oscillators[2].plot_rect.height())
                .abs()
                < 1e-3,
            "uninvolved ATR pane height must remain invariant when resizing Main/RSI separator"
        );

        // Extreme tiny outer rect (200x140 with 4 panes) must never produce negative or overlapping rects
        let tiny = MultiPaneLayout::compute(
            Rect::from_min_size(Pos2::ZERO, Vec2::new(200.0, 140.0)),
            4.0,
            &after_specs,
        );
        assert!(tiny.main_plot_rect.height() >= 12.0);
        for osc in &tiny.oscillators {
            assert!(osc.plot_rect.height() >= 10.0);
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
        assert!(r2.pointer_price.is_some());
        assert!(r2.bar_spacing > r1.bar_spacing);
        let after_float_idx = viewport
            .time_scale
            .local_x_to_float_index(anchor_local_x)
            .unwrap();
        assert!((after_float_idx - before_float_idx).abs() <= 2e-5);

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
        assert!(viewport.time_scale.right_offset() < ro_before_drag);
    }

    #[test]
    fn headless_multipane_crosshair_oscillator_zoom_and_separator_drag() {
        let candles = generate_synthetic_ohlcv(400);
        let mut viewport = ChartViewportState::new();
        let ind_cfg = IndicatorConfig {
            show_sma: true,
            show_ema: true,
            show_bbands: true,
            show_rsi: true,
            show_macd: true,
            show_atr: true,
            ..IndicatorConfig::default()
        };
        let mut cache = IndicatorCache::new();
        cache.sync(&candles, 1, &ind_cfg);

        let ctx = egui::Context::default();
        let screen_rect = Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 900.0)));
        let cfg = ChartDisplayConfig {
            symbol: "VNINDEX",
            timeframe: "1D",
            show_grid: true,
            show_crosshair: true,
            show_volume: true,
        };

        // Frame 1: Establish 4-pane layout (Main + RSI + MACD + ATR)
        let mut rep1 = None;
        let mut out1 = ctx.run_ui(
            egui::RawInput {
                screen_rect,
                time: Some(1.0),
                ..Default::default()
            },
            |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    rep1 = render_workstation_chart(
                        ui,
                        &candles,
                        &mut viewport,
                        cfg,
                        &ind_cfg,
                        &cache,
                    );
                });
            },
        );
        out1.textures_delta.clear();
        let r1 = rep1.expect("workstation report");
        assert_eq!(r1.pane_count, 4);
        assert_eq!(r1.active_overlay_count, 3);

        // Frame 2: Hover + wheel-zoom inside the RSI oscillator sub-pane -> must zoom shared TimeScale
        // and resolve RSI cursor value + identical inspected_data_index as hovering Main at the same X!
        let rsi_rect = r1.first_oscillator_plot_rect.expect("RSI plot rect");
        let osc_cursor = Pos2::new(
            rsi_rect.left() + rsi_rect.width() * 0.55,
            rsi_rect.center().y,
        );
        let mut rep2 = None;
        let mut out2 = ctx.run_ui(
            egui::RawInput {
                screen_rect,
                time: Some(1.016),
                events: vec![
                    egui::Event::PointerMoved(osc_cursor),
                    egui::Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Point,
                        delta: Vec2::new(0.0, 54.0),
                        phase: egui::TouchPhase::Move,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                ..Default::default()
            },
            |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    rep2 = render_workstation_chart(
                        ui,
                        &candles,
                        &mut viewport,
                        cfg,
                        &ind_cfg,
                        &cache,
                    );
                });
            },
        );
        out2.textures_delta.clear();
        let r2 = rep2.unwrap();
        assert_eq!(r2.hovered_oscillator, Some(OscillatorPaneId::Rsi));
        assert!(r2.hovered_oscillator_value.is_some());
        assert!(
            r2.bar_spacing > r1.bar_spacing,
            "wheel scroll inside RSI sub-pane must zoom shared TimeScale"
        );

        // Frames 3..5: Drag separator 0 downward by +50px -> Main pane plot height increases!
        let sep_rect = r2.first_separator_rect.expect("separator 0 rect");
        let sep_start = sep_rect.center();
        let sep_end = Pos2::new(sep_start.x, sep_start.y + 50.0);
        let main_h_before = r2.layout.plot_rect.height();
        let mut rep_after_sep = None;

        for (t, events) in [
            (
                1.032,
                vec![
                    egui::Event::PointerMoved(sep_start),
                    egui::Event::PointerButton {
                        pos: sep_start,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            ),
            (1.048, vec![egui::Event::PointerMoved(sep_end)]),
            (
                1.064,
                vec![egui::Event::PointerButton {
                    pos: sep_end,
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
                        rep_after_sep = render_workstation_chart(
                            ui,
                            &candles,
                            &mut viewport,
                            cfg,
                            &ind_cfg,
                            &cache,
                        );
                    });
                },
            );
            out.textures_delta.clear();
        }
        let r_sep = rep_after_sep.unwrap();
        assert!(
            r_sep.layout.plot_rect.height() > main_h_before + 35.0,
            "dragging separator down must expand Main pane height"
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
        assert_eq!(shape_counts[0], shape_counts[1]);
    }

    #[test]
    fn critic_r04_regression_multipane_waterfill_monotonicity_and_tiny_rect() {
        // 1. Stretches `[0.75, 0.75, 1.45, 0.75]` when 3 smaller panes clamp to `MIN_PANE_HEIGHT = 44`:
        // `MACD(1.45)` must receive the larger remaining share (`~68px`), NEVER get falsely locked at `44px`!
        let outer = Rect::from_min_size(Pos2::ZERO, Vec2::new(900.0, 243.0));
        let layout = MultiPaneLayout::compute(
            outer,
            0.75,
            &[
                (OscillatorPaneId::Rsi, 0.75),
                (OscillatorPaneId::Macd, 1.45),
                (OscillatorPaneId::Atr, 0.75),
            ],
        );
        let main_h = layout.main_plot_rect.height();
        let rsi_h = layout.oscillators[0].plot_rect.height();
        let macd_h = layout.oscillators[1].plot_rect.height();
        let atr_h = layout.oscillators[2].plot_rect.height();
        assert!((main_h - 44.0).abs() < 1e-3);
        assert!((rsi_h - 44.0).abs() < 1e-3);
        assert!((atr_h - 44.0).abs() < 1e-3);
        assert!(
            macd_h > 65.0,
            "larger stretch pane MACD(1.45) must receive remaining height (~68px), got {macd_h}"
        );

        // 2. Pathologically tiny outer_rect (`30x35`) must never invert or exceed `outer_rect`
        let tiny = Rect::from_min_size(Pos2::new(10.0, 10.0), Vec2::new(30.0, 35.0));
        let single = ChartLayout::from_outer_rect(tiny);
        assert!(single.plot_rect.right() <= tiny.right() + 1e-4);
        assert!(single.plot_rect.bottom() <= tiny.bottom() + 1e-4);

        let multi_tiny = MultiPaneLayout::compute(
            tiny,
            4.0,
            &[
                (OscillatorPaneId::Rsi, 1.0),
                (OscillatorPaneId::Macd, 1.0),
                (OscillatorPaneId::Atr, 1.0),
            ],
        );
        assert!(multi_tiny.main_plot_rect.right() <= tiny.right() + 1e-4);
        assert!(multi_tiny.time_axis_rect.bottom() <= tiny.bottom() + 1e-4);
        for osc in &multi_tiny.oscillators {
            assert!(osc.plot_rect.height() >= 0.0);
            assert!(osc.plot_rect.bottom() <= tiny.bottom() + 1e-4);
        }
    }

    #[test]
    fn critic_r04_regression_macd_zero_bar_symmetry_and_offscreen_clipping() {
        // Exact `0.0` value is symmetric around `zero_y`
        let (t0, b0) = resolve_macd_histogram_bar_ys(150.0, 150.0, 0.0, 100.0, 200.0).unwrap();
        assert!((t0 - 149.5).abs() < 1e-5 && (b0 - 150.5).abs() < 1e-5);

        // Small positive bar (`val_y < zero_y` on normal scale) extends upward from `zero_y`, never downward
        let (tp, bp) = resolve_macd_histogram_bar_ys(150.0, 149.9, 0.01, 100.0, 200.0).unwrap();
        assert!((tp - 149.25).abs() < 1e-5 && (bp - 150.0).abs() < 1e-5);

        // Off-screen `zero_y = 260.0` (`> plot_bottom = 200.0`) with `0.0` value clips away completely (`None`)
        assert!(resolve_macd_histogram_bar_ys(260.0, 260.0, 0.0, 100.0, 200.0).is_none());
    }

    #[test]
    fn critic_r04_regression_cross_pane_double_click_and_clamped_separator_drag() {
        let candles = generate_synthetic_ohlcv(200);
        let ind_cfg = IndicatorConfig {
            show_sma: true,
            show_ema: false,
            show_bbands: true,
            show_rsi: true,
            show_macd: true,
            show_atr: true,
            ..IndicatorConfig::default()
        };
        let mut cache = IndicatorCache::new();
        cache.sync(&candles, 1, &ind_cfg);
        let cfg = ChartDisplayConfig {
            symbol: "VNINDEX",
            timeframe: "1D",
            show_grid: true,
            show_crosshair: false,
            show_volume: true,
        };
        let ctx = egui::Context::default();
        let screen_rect = Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 700.0)));
        let mut viewport = ChartViewportState::new();
        // Force `RSI` stretch below `MIN_PANE_HEIGHT` share AND disable `main_price_scale` autoscale
        viewport.rsi_stretch = 0.05;

        let run_frame = |t: f64, events: Vec<egui::Event>, vp: &mut ChartViewportState| {
            let mut rep = None;
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect,
                    time: Some(t),
                    events,
                    ..Default::default()
                },
                |ui| {
                    egui::CentralPanel::default().show(ui, |ui| {
                        rep = render_workstation_chart(ui, &candles, vp, cfg, &ind_cfg, &cache);
                    });
                },
            );
            out.textures_delta.clear();
            rep.unwrap()
        };

        let r0 = run_frame(1.0, Vec::new(), &mut viewport);
        viewport.main_price_scale.start_scale(50.0);
        viewport.main_price_scale.scale_to(120.0);
        viewport.main_price_scale.end_scale();
        assert!(!viewport.main_price_scale.is_auto_scale());

        // Drag separator 0 by +4px while RSI was previously min-clamped -> MACD & ATR stretches stay invariant!
        let sep0 = r0.first_separator_rect.unwrap().center();
        let before_layout = MultiPaneLayout::compute(
            r0.layout.outer_rect,
            viewport.main_stretch,
            &[
                (OscillatorPaneId::Rsi, viewport.rsi_stretch),
                (OscillatorPaneId::Macd, viewport.macd_stretch),
                (OscillatorPaneId::Atr, viewport.atr_stretch),
            ],
        );
        let macd_h0 = before_layout.oscillators[1].plot_rect.height();
        let atr_h0 = before_layout.oscillators[2].plot_rect.height();

        let _ = run_frame(
            1.02,
            vec![
                egui::Event::PointerMoved(sep0),
                egui::Event::PointerButton {
                    pos: sep0,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            &mut viewport,
        );
        let r_drag = run_frame(
            1.04,
            vec![egui::Event::PointerMoved(Pos2::new(sep0.x, sep0.y - 8.0))],
            &mut viewport,
        );
        let _ = run_frame(
            1.06,
            vec![egui::Event::PointerButton {
                pos: Pos2::new(sep0.x, sep0.y - 8.0),
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
            &mut viewport,
        );
        let after_layout = MultiPaneLayout::compute(
            r_drag.layout.outer_rect,
            viewport.main_stretch,
            &[
                (OscillatorPaneId::Rsi, viewport.rsi_stretch),
                (OscillatorPaneId::Macd, viewport.macd_stretch),
                (OscillatorPaneId::Atr, viewport.atr_stretch),
            ],
        );
        assert!((after_layout.oscillators[1].plot_rect.height() - macd_h0).abs() < 0.5);
        assert!((after_layout.oscillators[2].plot_rect.height() - atr_h0).abs() < 0.5);

        // Now double-click inside the RSI oscillator plot -> must reset `main_price_scale` autoscale too!
        let rsi_center = r_drag.first_oscillator_plot_rect.unwrap().center();
        for (t, pressed) in [(2.00, true), (2.02, false), (2.08, true), (2.10, false)] {
            let _ = run_frame(
                t,
                vec![
                    egui::Event::PointerMoved(rsi_center),
                    egui::Event::PointerButton {
                        pos: rsi_center,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                &mut viewport,
            );
        }
        assert!(
            viewport.main_price_scale.is_auto_scale(),
            "double-clicking RSI plot must reset main_price_scale autoscale synchronously"
        );
    }

    #[test]
    fn multipane_drawing_arbitrage_creates_drags_selects_locked_and_preserves_pan() {
        let candles = generate_synthetic_ohlcv(240);
        let mut viewport = ChartViewportState::new();
        let indicators = IndicatorConfig {
            show_rsi: true,
            show_macd: true,
            ..Default::default()
        };
        let mut cache = IndicatorCache::new();
        cache.sync(&candles, 1, &indicators);
        let mut drawings = DrawingStore::new();
        let ctx = egui::Context::default();

        let run_frame = |time: f64,
                         events: Vec<egui::Event>,
                         vp: &mut ChartViewportState,
                         ds: &mut DrawingStore|
         -> ChartRenderReport {
            let raw = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 800.0))),
                time: Some(time),
                events,
                ..Default::default()
            };
            let mut rep = None;
            let mut out = ctx.run_ui(raw, |ui| {
                rep = render_workstation_chart_with_drawings(
                    ui,
                    &candles,
                    vp,
                    ChartDisplayConfig {
                        symbol: "VNINDEX",
                        timeframe: "1D",
                        show_grid: true,
                        show_crosshair: true,
                        show_volume: true,
                    },
                    &indicators,
                    &cache,
                    ds,
                );
            });
            let _ = ctx.tessellate(out.shapes, out.pixels_per_point);
            out.textures_delta.clear();
            rep.expect("chart renders")
        };

        let r0 = run_frame(1.0, Vec::new(), &mut viewport, &mut drawings);
        let ro_initial = r0.right_offset;

        // 1. Place a 2-click TrendLine on the Main pane without panning `time_scale`.
        drawings.set_tool(DrawingTool::TrendLine);
        let main_rect = r0.layout.plot_rect;
        let p0 = Pos2::new(main_rect.left() + 300.0, main_rect.top() + 180.0);
        let p1 = Pos2::new(main_rect.left() + 520.0, main_rect.top() + 110.0);

        let _ = run_frame(
            1.02,
            vec![
                egui::Event::PointerMoved(p0),
                egui::Event::PointerButton {
                    pos: p0,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            &mut viewport,
            &mut drawings,
        );
        let _ = run_frame(
            1.04,
            vec![egui::Event::PointerButton {
                pos: p0,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
            &mut viewport,
            &mut drawings,
        );
        let _ = run_frame(
            1.06,
            vec![egui::Event::PointerMoved(p1)],
            &mut viewport,
            &mut drawings,
        );
        let _ = run_frame(
            1.08,
            vec![egui::Event::PointerButton {
                pos: p1,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            }],
            &mut viewport,
            &mut drawings,
        );
        let r_after_create = run_frame(
            1.10,
            vec![egui::Event::PointerButton {
                pos: p1,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
            &mut viewport,
            &mut drawings,
        );
        assert_eq!(r_after_create.drawing_count, 1);
        assert_eq!(drawings.active_tool, DrawingTool::Select);
        assert!((r_after_create.right_offset - ro_initial).abs() < 1e-6);

        // 2. Place a HorizontalLine on the RSI sub-pane.
        drawings.set_tool(DrawingTool::HorizontalLine);
        let rsi_center = r_after_create.first_oscillator_plot_rect.unwrap().center();
        let _ = run_frame(
            1.12,
            vec![
                egui::Event::PointerMoved(rsi_center),
                egui::Event::PointerButton {
                    pos: rsi_center,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            &mut viewport,
            &mut drawings,
        );
        let r_after_rsi = run_frame(
            1.14,
            vec![egui::Event::PointerButton {
                pos: rsi_center,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
            &mut viewport,
            &mut drawings,
        );
        assert_eq!(r_after_rsi.drawing_count, 2);
        let rsi_id = drawings.selected_id.expect("RSI horizontal line selected");

        // 3. Lock the RSI HorizontalLine, deselect on empty Main canvas, then click RSI line again -> stays selected!
        assert!(drawings.toggle_selected_lock());
        let empty_pt = Pos2::new(main_rect.left() + 80.0, main_rect.top() + 40.0);
        let _ = run_frame(
            1.16,
            vec![
                egui::Event::PointerMoved(empty_pt),
                egui::Event::PointerButton {
                    pos: empty_pt,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            &mut viewport,
            &mut drawings,
        );
        let _ = run_frame(
            1.18,
            vec![egui::Event::PointerButton {
                pos: empty_pt,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
            &mut viewport,
            &mut drawings,
        );
        assert_eq!(drawings.selected_id, None);

        let _ = run_frame(
            1.20,
            vec![
                egui::Event::PointerMoved(rsi_center),
                egui::Event::PointerButton {
                    pos: rsi_center,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            &mut viewport,
            &mut drawings,
        );
        let _ = run_frame(
            1.22,
            vec![egui::Event::PointerButton {
                pos: rsi_center,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
            &mut viewport,
            &mut drawings,
        );
        assert_eq!(
            drawings.selected_id,
            Some(rsi_id),
            "two-frame click on locked drawing must remain selected on release"
        );

        // 4. Starting a drag directly on a locked drawing passes through to `TimeScale` pan smoothly!
        let ro_before_locked_pan = viewport.time_scale.right_offset();
        let drag_to = Pos2::new(rsi_center.x - 80.0, rsi_center.y);
        let _ = run_frame(
            1.24,
            vec![
                egui::Event::PointerMoved(rsi_center),
                egui::Event::PointerButton {
                    pos: rsi_center,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            &mut viewport,
            &mut drawings,
        );
        let _ = run_frame(
            1.26,
            vec![egui::Event::PointerMoved(drag_to)],
            &mut viewport,
            &mut drawings,
        );
        let r_after_locked_pan = run_frame(
            1.28,
            vec![egui::Event::PointerButton {
                pos: drag_to,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
            &mut viewport,
            &mut drawings,
        );
        assert!(
            (r_after_locked_pan.right_offset - ro_before_locked_pan).abs() > 1.0,
            "dragging over a locked drawing must pan shared TimeScale unimpeded"
        );
    }
}
