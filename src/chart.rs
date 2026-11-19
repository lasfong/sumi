use egui::epaint::Mesh;
use egui::text::LayoutJob;
use egui::{
    Align2, Color32, CursorIcon, FontId, Key, Pos2, Rect, Sense, Shape, Stroke, StrokeKind,
    TextFormat, Ui, Vec2,
};
use serde::{Deserialize, Serialize};

use crate::alerts::{
    AlertCondition, AlertEngine, AlertStatus, AlertTriggerEvent, CrossDirection,
    drawing_line_value_at_bar,
};
use crate::backtest::{
    BacktestExitReason, BacktestRunResult, BacktestTrade, ComparisonCurveKind, MonteCarloFanSeries,
    MultiStrategyComparisonReport, StrategyPresetKind,
};
use crate::data::Timeframe;
use crate::drawings::{
    Drawing, DrawingColorPreset, DrawingGeometry, DrawingId, DrawingInteractionPhase,
    DrawingPaneTarget, DrawingStore, DrawingTool, HandlePart, HitTarget, LineDashStyle, LocalPoint,
    MagnetMode, PaneDrawingProjection, PointerCaptureOutcome, ProjectedDrawingShape,
};
use crate::indicators::{
    ADX_REFERENCE_LEVELS, BBHorizon, BucketLineEnvelope, CCI_REFERENCE_LEVELS,
    CVD_REFERENCE_LEVELS, CandleBlockIndex, CvdResetMode, IndicatorCache, IndicatorConfig,
    MACD_REFERENCE_LEVELS, MFI_REFERENCE_LEVELS, OscillatorPaneId, PROXY_BB_REFERENCE_LEVELS,
    RS_REFERENCE_LEVELS, RSI_REFERENCE_LEVELS, STOCH_REFERENCE_LEVELS, SeriesBlockIndex,
    VolumeProfileConfig, VolumeProfileResult, VwapBandsOutput, bucket_line_envelope_indexed,
    compute_anchored_vwap, compute_volume_profile, visit_conflated_histogram_bars_indexed,
};
use crate::model::{Candle, PriceBounds};
use crate::trading::{ExecutionFill, OrderSide, PositionSide, TradingSession};
use crate::viewport::{
    CandlestickBarMetrics, HorzScaleOptions, KineticAnimation, KineticScrollConfig, PaneViewport,
    PlotRect, PriceScale, PriceScaleOptions, TimeScale, conflated_candle_at,
    iter_conflated_candles, resolve_linked_crosshair_bar,
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

    // Milestone R08 — Causal Alert Engine Overlay Palette
    pub const ALERT_ARMED_AMBER: Color32 = Color32::from_rgb(255, 167, 38); // #FFA726
    pub const ALERT_TRIGGERED_GOLD: Color32 = Color32::from_rgb(255, 213, 79); // #FFD54F
    pub const ALERT_AXIS_TEXT_DARK: Color32 = Color32::from_rgb(18, 20, 26); // #12141A

    // Milestone R09 — Strategy Backtest Lab & Baseline Equity Curve Palette
    pub const EQUITY_ABOVE_LINE: Color32 = Color32::from_rgb(38, 166, 154); // #26A69A Emerald
    pub const EQUITY_ABOVE_FILL: Color32 = Color32::from_rgba_premultiplied(7, 29, 27, 44);
    pub const EQUITY_BELOW_LINE: Color32 = Color32::from_rgb(239, 83, 80); // #EF5350 Crimson
    pub const EQUITY_BELOW_FILL: Color32 = Color32::from_rgba_premultiplied(45, 16, 15, 48);
    pub const EQUITY_BASELINE_REF: Color32 = Color32::from_rgb(132, 142, 156);
    pub const BACKTEST_OOS_SPLIT: Color32 = Color32::from_rgb(255, 183, 77); // #FFB74D
    pub const TRADE_LINK_WIN: Color32 = Color32::from_rgb(0, 230, 118); // #00E676
    pub const TRADE_LINK_LOSS: Color32 = Color32::from_rgb(255, 82, 82); // #FF5252
    pub const TRADE_FOCUS_HALO: Color32 = Color32::from_rgb(255, 213, 79); // #FFD54F

    // Milestone R11 — Volume Profile (VRVP/FRVP), Session/Anchored VWAP & Relative Strength Palette
    pub const IND_VWAP_LINE: Color32 = Color32::from_rgb(206, 147, 216); // #CE93D8 Crisp Lavender-Violet
    pub const IND_VWAP_BAND_1: Color32 = Color32::from_rgb(171, 71, 188); // #AB47BC Medium Orchid
    pub const IND_VWAP_BAND_2: Color32 = Color32::from_rgb(126, 87, 194); // #7E57C2 Deep Indigo-Violet
    pub const IND_VWAP_FILL_1: Color32 = Color32::from_rgba_premultiplied(19, 8, 21, 28);
    pub const IND_VWAP_FILL_2: Color32 = Color32::from_rgba_premultiplied(9, 6, 14, 18);
    pub const VRVP_BUY_VA: Color32 = Color32::from_rgba_premultiplied(14, 62, 58, 96);
    pub const VRVP_SELL_VA: Color32 = Color32::from_rgba_premultiplied(90, 31, 30, 96);
    pub const VRVP_BUY_OUT: Color32 = Color32::from_rgba_premultiplied(7, 30, 28, 46);
    pub const VRVP_SELL_OUT: Color32 = Color32::from_rgba_premultiplied(43, 15, 14, 46);
    pub const VRVP_POC_GOLD: Color32 = Color32::from_rgb(255, 213, 79); // #FFD54F
    pub const VRVP_VA_LINE: Color32 = Color32::from_rgb(79, 195, 247); // #4FC3F7
    pub const COMPARE_BENCH_LINE: Color32 = Color32::from_rgb(255, 167, 38); // #FFA726 Vivid Orange
    pub const IND_RS_LINE: Color32 = Color32::from_rgb(38, 198, 218); // #26C6DA Teal-Cyan
    pub const IND_RS_SMA: Color32 = Color32::from_rgb(255, 202, 40); // #FFCA28 Amber
    pub const IND_RS_BASELINE: Color32 = Color32::from_rgb(120, 123, 134);
    pub const IND_ICHIMOKU_TENKAN: Color32 = Color32::from_rgb(41, 98, 255); // #2962FF
    pub const IND_ICHIMOKU_KIJUN: Color32 = Color32::from_rgb(183, 28, 28); // #B71C1C
    pub const IND_ICHIMOKU_SENKOU_A: Color32 = Color32::from_rgb(67, 160, 71); // #43A047
    pub const IND_ICHIMOKU_SENKOU_B: Color32 = Color32::from_rgb(244, 67, 54); // #F44336
    pub const IND_ICHIMOKU_CHIKOU: Color32 = Color32::from_rgb(77, 208, 225); // #4DD0E1
    pub const IND_KUMO_BULL_FILL: Color32 = Color32::from_rgba_premultiplied(12, 29, 13, 46);
    pub const IND_KUMO_BEAR_FILL: Color32 = Color32::from_rgba_premultiplied(44, 12, 10, 46);
    pub const IND_CVD_LINE: Color32 = Color32::from_rgb(38, 198, 218); // #26C6DA
    pub const IND_CVD_SMA: Color32 = Color32::from_rgb(255, 202, 40); // #FFCA28
    pub const LINKED_CROSSHAIR_LINE: Color32 = Color32::from_rgba_premultiplied(94, 101, 112, 155);
    pub const MP_VA_FILL: Color32 = Color32::from_rgba_premultiplied(10, 22, 36, 44);
    pub const MP_TPO_VA: Color32 = Color32::from_rgba_premultiplied(24, 62, 92, 96);
    pub const MP_TPO_OUT: Color32 = Color32::from_rgba_premultiplied(38, 44, 56, 64);
    pub const MP_SINGLE_PRINT: Color32 = Color32::from_rgba_premultiplied(72, 40, 12, 78);
    pub const MP_POC_LINE: Color32 = Color32::from_rgb(255, 202, 40);
    pub const MP_NAKED_POC_LINE: Color32 = Color32::from_rgba_premultiplied(175, 114, 26, 180);
    pub const MP_IB_BRACKET: Color32 = Color32::from_rgba_premultiplied(28, 144, 160, 190);
    pub const MP_VA_BORDER: Color32 = Color32::from_rgba_premultiplied(60, 108, 148, 145);

    // Milestone R18 — Extended Technical Overlays, Candlestick Badges & ProxyBB Flow Sub-Pane
    pub const IND_SUPERTREND_BULL: Color32 = Color32::from_rgb(0, 230, 118); // #00E676 Emerald
    pub const IND_SUPERTREND_BEAR: Color32 = Color32::from_rgb(255, 82, 82); // #FF5252 Crimson
    pub const IND_PSAR_BULL: Color32 = Color32::from_rgb(38, 198, 218); // #26C6DA Cyan-Teal
    pub const IND_PSAR_BEAR: Color32 = Color32::from_rgb(255, 171, 64); // #FFAB40 Warm Amber
    pub const IND_KELTNER_BORDER: Color32 = Color32::from_rgb(129, 199, 132); // #81C784 Sage-Teal
    pub const IND_KELTNER_MIDDLE: Color32 = Color32::from_rgb(102, 187, 106); // #66BB6A
    pub const IND_KELTNER_FILL: Color32 = Color32::from_rgba_premultiplied(9, 21, 14, 24);
    pub const IND_VOLUME_SMA: Color32 = Color32::from_rgb(255, 213, 79); // #FFD54F Crisp Gold
    pub const IND_ADX_LINE: Color32 = Color32::from_rgb(255, 202, 40); // #FFCA28 Gold
    pub const IND_PLUS_DI: Color32 = Color32::from_rgb(38, 166, 154); // #26A69A Emerald
    pub const IND_MINUS_DI: Color32 = Color32::from_rgb(239, 83, 80); // #EF5350 Crimson
    pub const IND_STOCH_K: Color32 = Color32::from_rgb(41, 182, 246); // #29B6F6 Sky Blue
    pub const IND_STOCH_D: Color32 = Color32::from_rgb(255, 138, 0); // #FF8A00 Orange
    pub const IND_CCI: Color32 = Color32::from_rgb(38, 198, 218); // #26C6DA Cyan
    pub const IND_MFI: Color32 = Color32::from_rgb(240, 98, 146); // #F06292 Rose-Pink
    pub const IND_PROXY_BB_T05: Color32 = Color32::from_rgb(128, 222, 234); // #80DEEA Fast T05
    pub const IND_PROXY_BB_T20: Color32 = Color32::from_rgb(38, 166, 154); // #26A69A Baseline T20
    pub const IND_PROXY_BB_T50: Color32 = Color32::from_rgb(255, 183, 77); // #FFB74D Structural T50
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

fn default_equity_price_scale() -> PriceScale {
    PriceScale::new("equity-right", PriceScaleOptions::sumi_oscillator())
}

fn default_rs_price_scale() -> PriceScale {
    PriceScale::new("rs-right", PriceScaleOptions::sumi_oscillator())
}

fn default_cvd_price_scale() -> PriceScale {
    PriceScale::new("cvd-right", PriceScaleOptions::sumi_cvd_oscillator())
}

fn default_adx_price_scale() -> PriceScale {
    PriceScale::new("adx-right", PriceScaleOptions::sumi_oscillator())
}

fn default_stoch_price_scale() -> PriceScale {
    PriceScale::new("stoch-right", PriceScaleOptions::sumi_oscillator())
}

fn default_cci_price_scale() -> PriceScale {
    PriceScale::new("cci-right", PriceScaleOptions::sumi_oscillator())
}

fn default_mfi_price_scale() -> PriceScale {
    PriceScale::new("mfi-right", PriceScaleOptions::sumi_oscillator())
}

fn default_proxy_bb_price_scale() -> PriceScale {
    PriceScale::new("proxy-bb-right", PriceScaleOptions::sumi_oscillator())
}

/// Heap-boxed `PriceScale` bundle for Milestone R18 oscillator sub-panes (`ADX`, `Stochastic`,
/// `CCI`, `MFI`, `ProxyBB`) keeping [`ChartViewportState`] stack frames compact during `serde` restore.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtendedOscillatorScales {
    #[serde(default = "default_adx_price_scale")]
    pub adx_price_scale: PriceScale,
    #[serde(default = "default_stoch_price_scale")]
    pub stoch_price_scale: PriceScale,
    #[serde(default = "default_cci_price_scale")]
    pub cci_price_scale: PriceScale,
    #[serde(default = "default_mfi_price_scale")]
    pub mfi_price_scale: PriceScale,
    #[serde(default = "default_proxy_bb_price_scale")]
    pub proxy_bb_price_scale: PriceScale,
}

impl Default for ExtendedOscillatorScales {
    fn default() -> Self {
        Self {
            adx_price_scale: default_adx_price_scale(),
            stoch_price_scale: default_stoch_price_scale(),
            cci_price_scale: default_cci_price_scale(),
            mfi_price_scale: default_mfi_price_scale(),
            proxy_bb_price_scale: default_proxy_bb_price_scale(),
        }
    }
}

const fn default_main_stretch() -> f32 {
    4.0
}

const fn default_sub_stretch() -> f32 {
    1.0
}

const fn default_equity_stretch() -> f32 {
    1.25
}

#[inline]
fn sanitize_stretch(val: f32, fallback: f32) -> f32 {
    if val.is_finite() && val >= 0.05 {
        val.min(50.0)
    } else {
        fallback
    }
}

#[inline]
fn hash_candle_into(h: &mut u64, c: &Candle) {
    for bits in [
        c.timestamp as u64,
        c.open.to_bits(),
        c.high.to_bits(),
        c.low.to_bits(),
        c.close.to_bits(),
        c.volume.to_bits(),
    ] {
        *h ^= bits;
        *h = h.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        *h ^= *h >> 30;
    }
}

/// Computes a fast deterministic seeded content fingerprint for `candles` used by [`R11RenderMemo`].
/// Hashes every bar for `<= 2048` bars and samples 65 evenly spaced bars plus `n - 2` (`O(1)`) above `2048`.
pub(crate) fn candles_fast_fingerprint_seeded(candles: &[Candle], seed: u64) -> u64 {
    let n = candles.len();
    if n == 0 {
        return seed;
    }
    let mut h: u64 = seed ^ 0x517C_C1B7_2722_0A95 ^ (n as u64);
    if n <= 2048 {
        for c in candles {
            hash_candle_into(&mut h, c);
        }
        return h;
    }
    let mut prev = usize::MAX;
    for step in 0..=64_usize {
        let idx = (step * (n - 1)) / 64;
        if idx != prev {
            if idx == n - 1 && prev != n - 2 {
                hash_candle_into(&mut h, &candles[n - 2]);
            }
            hash_candle_into(&mut h, &candles[idx]);
            prev = idx;
        }
    }
    h
}

/// Computes a fast deterministic content fingerprint for `candles` used by [`R11RenderMemo`].
pub(crate) fn candles_fast_fingerprint(candles: &[Candle]) -> u64 {
    candles_fast_fingerprint_seeded(candles, 0)
}

#[inline]
fn r11_subslice_seed(candles_fp: u64) -> u64 {
    candles_fp
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .rotate_left(13)
        ^ 0xBF58_476D_1CE4_E5B9
}

const MAX_R11_DRAWING_MEMO_ENTRIES: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
struct VrvpMemoKey {
    candles_fp: u64,
    start: usize,
    end: usize,
    row_count: usize,
    va_pct_bits: u64,
}

#[derive(Debug, Clone)]
struct VrvpMemoEntry {
    key: VrvpMemoKey,
    result: Option<std::sync::Arc<VolumeProfileResult>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FrvpMemoKey {
    candles_fp: u64,
    start: usize,
    end_inclusive: usize,
    row_count: usize,
    va_pct_bits: u64,
}

#[derive(Debug, Clone)]
struct FrvpMemoEntry {
    key: FrvpMemoKey,
    result: Option<std::sync::Arc<VolumeProfileResult>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AvwapMemoKey {
    candles_fp: u64,
    anchor_index: usize,
    causal_end: usize,
    mult_1_bits: u64,
    mult_2_bits: u64,
}

#[derive(Debug, Clone)]
struct AvwapMemoEntry {
    key: AvwapMemoKey,
    result: std::sync::Arc<VwapBandsOutput>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PriceScaleMemoKey {
    mode: crate::viewport::PriceScaleMode,
    format_mode: crate::viewport::PriceAxisFormatMode,
    inverted: bool,
    height_bits: u64,
    top_margin_bits: u64,
    bottom_margin_bits: u64,
    range_bits: Option<(u64, u64)>,
    log_logical_bits: u64,
    log_coord_bits: u64,
}

impl PriceScaleMemoKey {
    fn from_price_scale(ps: &PriceScale) -> Self {
        let log_f = ps.log_formula();
        Self {
            mode: ps.mode(),
            format_mode: ps.format_mode(),
            inverted: ps.is_inverted(),
            height_bits: ps.height().to_bits(),
            top_margin_bits: ps.top_margin_px().to_bits(),
            bottom_margin_bits: ps.bottom_margin_px().to_bits(),
            range_bits: ps.price_range().map(|r| (r.min.to_bits(), r.max.to_bits())),
            log_logical_bits: log_f.logical_offset.to_bits(),
            log_coord_bits: log_f.coord_offset.to_bits(),
        }
    }
}

#[inline]
fn rect_to_bits(r: Rect) -> [u32; 4] {
    [
        r.min.x.to_bits(),
        r.min.y.to_bits(),
        r.max.x.to_bits(),
        r.max.y.to_bits(),
    ]
}

#[inline]
fn opt_f64_bits(v: Option<f64>) -> u64 {
    match v {
        Some(x) if x.is_finite() => x.to_bits() ^ 0xA5A5_5A5A_1234_5678,
        Some(x) => x.to_bits(),
        None => 0xDEAD_BEEF_CAFE_BABE,
    }
}

#[inline]
fn series_edge_stamp(series: &[Option<f64>]) -> (usize, u64, u64) {
    let len = series.len();
    if len == 0 {
        return (0, 0, 0);
    }
    let first = opt_f64_bits(series[0]);
    let mid = opt_f64_bits(series[len / 2]).rotate_left(19);
    let last = opt_f64_bits(series[len - 1]);
    let q3 = opt_f64_bits(series[(len * 3) / 4]).rotate_left(11);
    (len, first ^ mid, last ^ q3)
}

#[derive(Debug, Clone, PartialEq)]
struct PlotDataLayerKey {
    layer_id: egui::LayerId,
    parent_clip_bits: [u32; 4],
    opacity_bits: u32,
    ppp_bits: u64,
    candles_fp: u64,
    candles_len: usize,
    dataset_gen: u64,
    main_plot_bits: [u32; 4],
    culling_start: usize,
    culling_end: usize,
    conflation_factor: usize,
    ov_step: usize,
    time_width_bits: u64,
    bar_spacing_bits: u64,
    right_offset_bits: u64,
    base_index: Option<i64>,
    base_value_bits: u64,
    show_volume: bool,
    main_ps: PriceScaleMemoKey,
    vol_ps: PriceScaleMemoKey,
    indicators: IndicatorConfig,
    vwap_stamp: (usize, (usize, u64, u64)),
    mp_stamp: Option<(usize, usize, u64)>,
    cvd_stamp: (usize, (usize, u64, u64)),
    rs_stamp: (usize, u64, u64),
    compare_stamp: Option<(String, (usize, u64, u64), u64)>,
    osc_panes: Vec<(OscillatorPaneId, [u32; 4], PriceScaleMemoKey)>,
}

#[derive(Debug, Clone)]
pub(crate) struct PlotDataLayerMemo {
    key: PlotDataLayerKey,
    main_shapes: Vec<egui::epaint::ClippedShape>,
    osc_shapes: Vec<(OscillatorPaneId, Vec<egui::epaint::ClippedShape>)>,
    rendered_candle_count: usize,
    static_overlay_count: usize,
}

fn painter_next_shape_idx(painter: &egui::Painter) -> usize {
    painter
        .ctx()
        .graphics(|g| g.get(painter.layer_id()).map_or(0, |l| l.next_idx().0))
}

fn capture_painter_shapes_since(
    painter: &egui::Painter,
    start_idx: usize,
) -> Vec<egui::epaint::ClippedShape> {
    painter.ctx().graphics(|g| {
        g.get(painter.layer_id())
            .map(|l| l.all_entries().skip(start_idx).cloned().collect())
            .unwrap_or_default()
    })
}

fn replay_cached_shapes(painter: &egui::Painter, shapes: &[egui::epaint::ClippedShape]) {
    painter.ctx().graphics_mut(|g| {
        let list = g.entry(painter.layer_id());
        for cs in shapes {
            list.add(cs.clip_rect, cs.shape.clone());
        }
    });
}

/// Frame-to-frame viewport memoization cache for R11 order-flow computations (`VRVP`, `FRVP`, `AVWAP`)
/// and conflated static `Arc<Mesh>` chart plot layers.
#[derive(Debug, Clone, Default)]
pub(crate) struct R11RenderMemo {
    vrvp: Option<VrvpMemoEntry>,
    frvp: Vec<FrvpMemoEntry>,
    avwap: Vec<AvwapMemoEntry>,
    plot_data: Option<PlotDataLayerMemo>,
}

impl R11RenderMemo {
    pub(crate) fn clear(&mut self) {
        self.vrvp = None;
        self.frvp.clear();
        self.avwap.clear();
        self.plot_data = None;
    }

    pub(crate) fn get_or_compute_vrvp(
        &mut self,
        candles: &[Candle],
        candles_fp: u64,
        range: std::ops::Range<usize>,
        cfg: VolumeProfileConfig,
    ) -> Option<std::sync::Arc<VolumeProfileResult>> {
        if range.start >= range.end || range.end > candles.len() {
            return None;
        }
        let slice_fp =
            candles_fast_fingerprint_seeded(&candles[range.clone()], r11_subslice_seed(candles_fp));
        let key = VrvpMemoKey {
            candles_fp: slice_fp,
            start: range.start,
            end: range.end,
            row_count: cfg.row_count,
            va_pct_bits: cfg.value_area_pct.to_bits(),
        };
        if let Some(entry) = &self.vrvp
            && entry.key == key
        {
            return entry.result.clone();
        }
        let computed = compute_volume_profile(&candles[range], cfg).map(std::sync::Arc::new);
        self.vrvp = Some(VrvpMemoEntry {
            key,
            result: computed.clone(),
        });
        computed
    }

    pub(crate) fn get_or_compute_frvp(
        &mut self,
        candles: &[Candle],
        candles_fp: u64,
        start: usize,
        end_inclusive: usize,
        cfg: VolumeProfileConfig,
    ) -> Option<std::sync::Arc<VolumeProfileResult>> {
        if start > end_inclusive || end_inclusive >= candles.len() {
            return None;
        }
        let span = &candles[start..=end_inclusive];
        let slice_fp = candles_fast_fingerprint_seeded(span, r11_subslice_seed(candles_fp));
        let key = FrvpMemoKey {
            candles_fp: slice_fp,
            start,
            end_inclusive,
            row_count: cfg.row_count,
            va_pct_bits: cfg.value_area_pct.to_bits(),
        };
        if let Some(idx) = self.frvp.iter().position(|e| e.key == key) {
            if idx + 1 < self.frvp.len() {
                let entry = self.frvp.remove(idx);
                let res = entry.result.clone();
                self.frvp.push(entry);
                return res;
            }
            return self.frvp[idx].result.clone();
        }
        let computed = compute_volume_profile(span, cfg).map(std::sync::Arc::new);
        if self.frvp.len() >= MAX_R11_DRAWING_MEMO_ENTRIES {
            self.frvp.remove(0);
        }
        self.frvp.push(FrvpMemoEntry {
            key,
            result: computed.clone(),
        });
        computed
    }

    pub(crate) fn get_or_compute_avwap(
        &mut self,
        candles: &[Candle],
        candles_fp: u64,
        anchor_index: usize,
        causal_end: usize,
        mult_1: f64,
        mult_2: f64,
    ) -> std::sync::Arc<VwapBandsOutput> {
        if anchor_index >= causal_end || causal_end > candles.len() {
            return std::sync::Arc::new(VwapBandsOutput::default());
        }
        let tail = &candles[anchor_index..causal_end];
        let slice_fp = candles_fast_fingerprint_seeded(tail, r11_subslice_seed(candles_fp));
        let key = AvwapMemoKey {
            candles_fp: slice_fp,
            anchor_index,
            causal_end,
            mult_1_bits: mult_1.to_bits(),
            mult_2_bits: mult_2.to_bits(),
        };
        if let Some(idx) = self.avwap.iter().position(|e| e.key == key) {
            if idx + 1 < self.avwap.len() {
                let entry = self.avwap.remove(idx);
                let res = entry.result.clone();
                self.avwap.push(entry);
                return res;
            }
            return self.avwap[idx].result.clone();
        }
        let computed = std::sync::Arc::new(compute_anchored_vwap(tail, 0, mult_1, mult_2));
        if self.avwap.len() >= MAX_R11_DRAWING_MEMO_ENTRIES {
            self.avwap.remove(0);
        }
        self.avwap.push(AvwapMemoEntry {
            key,
            result: computed.clone(),
        });
        computed
    }
}

/// Modular viewport state coordinating the shared horizontal `TimeScale` and independent per-pane
/// `PriceScale` instances (`main-right`, `volume-overlay`, `rsi-right`, `macd-right`, `atr-right`,
/// `rs-right`, `equity-right`), proportional multi-pane stretch ratios, and inertial kinetic scrolling.
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
    #[serde(default = "default_rs_price_scale")]
    pub rs_price_scale: PriceScale,
    #[serde(default = "default_cvd_price_scale")]
    pub cvd_price_scale: PriceScale,
    #[serde(default = "default_equity_price_scale")]
    pub equity_price_scale: PriceScale,
    #[serde(default)]
    pub ext_scales: Box<ExtendedOscillatorScales>,

    #[serde(default = "default_main_stretch")]
    pub main_stretch: f32,
    #[serde(default = "default_sub_stretch")]
    pub rsi_stretch: f32,
    #[serde(default = "default_sub_stretch")]
    pub macd_stretch: f32,
    #[serde(default = "default_sub_stretch")]
    pub atr_stretch: f32,
    #[serde(default = "default_sub_stretch")]
    pub rs_stretch: f32,
    #[serde(default = "default_sub_stretch")]
    pub cvd_stretch: f32,
    #[serde(default = "default_equity_stretch")]
    pub equity_stretch: f32,
    #[serde(default = "default_sub_stretch")]
    pub adx_stretch: f32,
    #[serde(default = "default_sub_stretch")]
    pub stoch_stretch: f32,
    #[serde(default = "default_sub_stretch")]
    pub cci_stretch: f32,
    #[serde(default = "default_sub_stretch")]
    pub mfi_stretch: f32,
    #[serde(default = "default_sub_stretch")]
    pub proxy_bb_stretch: f32,

    #[serde(skip)]
    pub kinetic: Option<KineticAnimation>,
    #[serde(skip)]
    separator_drag: Option<SeparatorDragState>,
    #[serde(skip)]
    pub(crate) r11_memo: R11RenderMemo,
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
            rs_price_scale: default_rs_price_scale(),
            cvd_price_scale: default_cvd_price_scale(),
            equity_price_scale: default_equity_price_scale(),
            ext_scales: Box::default(),
            main_stretch: default_main_stretch(),
            rsi_stretch: default_sub_stretch(),
            macd_stretch: default_sub_stretch(),
            atr_stretch: default_sub_stretch(),
            rs_stretch: default_sub_stretch(),
            cvd_stretch: default_sub_stretch(),
            equity_stretch: default_equity_stretch(),
            adx_stretch: default_sub_stretch(),
            stoch_stretch: default_sub_stretch(),
            cci_stretch: default_sub_stretch(),
            mfi_stretch: default_sub_stretch(),
            proxy_bb_stretch: default_sub_stretch(),
            kinetic: None,
            separator_drag: None,
            r11_memo: R11RenderMemo::default(),
        }
    }

    /// Sanitizes state restored from persistent `serde` storage against NaN/Infinity or mode mismatches.
    pub fn sanitize_deserialized(&mut self) {
        self.kinetic = None;
        self.separator_drag = None;
        self.r11_memo.clear();
        self.time_scale.sanitize_deserialized();
        self.main_price_scale.sanitize_deserialized();
        self.volume_price_scale.sanitize_deserialized();
        self.rsi_price_scale.sanitize_deserialized();
        self.macd_price_scale.sanitize_deserialized();
        self.atr_price_scale.sanitize_deserialized();
        self.rs_price_scale.sanitize_deserialized();
        self.cvd_price_scale.sanitize_deserialized();
        self.equity_price_scale.sanitize_deserialized();
        self.ext_scales.adx_price_scale.sanitize_deserialized();
        self.ext_scales.stoch_price_scale.sanitize_deserialized();
        self.ext_scales.cci_price_scale.sanitize_deserialized();
        self.ext_scales.mfi_price_scale.sanitize_deserialized();
        self.ext_scales.proxy_bb_price_scale.sanitize_deserialized();
        // Oscillator / Equity sub-panes always operate in Linear vertical space.
        self.rsi_price_scale
            .set_mode(crate::viewport::PriceScaleMode::Normal);
        self.macd_price_scale
            .set_mode(crate::viewport::PriceScaleMode::Normal);
        self.atr_price_scale
            .set_mode(crate::viewport::PriceScaleMode::Normal);
        self.rs_price_scale
            .set_mode(crate::viewport::PriceScaleMode::Normal);
        self.cvd_price_scale
            .set_mode(crate::viewport::PriceScaleMode::Normal);
        self.cvd_price_scale
            .set_format_mode(crate::viewport::PriceAxisFormatMode::CompactVolume);
        self.equity_price_scale
            .set_mode(crate::viewport::PriceScaleMode::Normal);
        self.ext_scales
            .adx_price_scale
            .set_mode(crate::viewport::PriceScaleMode::Normal);
        self.ext_scales
            .stoch_price_scale
            .set_mode(crate::viewport::PriceScaleMode::Normal);
        self.ext_scales
            .cci_price_scale
            .set_mode(crate::viewport::PriceScaleMode::Normal);
        self.ext_scales
            .mfi_price_scale
            .set_mode(crate::viewport::PriceScaleMode::Normal);
        self.ext_scales
            .proxy_bb_price_scale
            .set_mode(crate::viewport::PriceScaleMode::Normal);
        self.main_stretch = sanitize_stretch(self.main_stretch, default_main_stretch());
        self.rsi_stretch = sanitize_stretch(self.rsi_stretch, default_sub_stretch());
        self.macd_stretch = sanitize_stretch(self.macd_stretch, default_sub_stretch());
        self.atr_stretch = sanitize_stretch(self.atr_stretch, default_sub_stretch());
        self.rs_stretch = sanitize_stretch(self.rs_stretch, default_sub_stretch());
        self.cvd_stretch = sanitize_stretch(self.cvd_stretch, default_sub_stretch());
        self.equity_stretch = sanitize_stretch(self.equity_stretch, default_equity_stretch());
        self.adx_stretch = sanitize_stretch(self.adx_stretch, default_sub_stretch());
        self.stoch_stretch = sanitize_stretch(self.stoch_stretch, default_sub_stretch());
        self.cci_stretch = sanitize_stretch(self.cci_stretch, default_sub_stretch());
        self.mfi_stretch = sanitize_stretch(self.mfi_stretch, default_sub_stretch());
        self.proxy_bb_stretch = sanitize_stretch(self.proxy_bb_stretch, default_sub_stretch());
    }

    #[must_use]
    pub fn oscillator_price_scale(&self, id: OscillatorPaneId) -> &PriceScale {
        match id {
            OscillatorPaneId::Rsi => &self.rsi_price_scale,
            OscillatorPaneId::Macd => &self.macd_price_scale,
            OscillatorPaneId::Atr => &self.atr_price_scale,
            OscillatorPaneId::Cvd => &self.cvd_price_scale,
            OscillatorPaneId::RelativeStrength => &self.rs_price_scale,
            OscillatorPaneId::Equity => &self.equity_price_scale,
            OscillatorPaneId::Adx => &self.ext_scales.adx_price_scale,
            OscillatorPaneId::Stochastic => &self.ext_scales.stoch_price_scale,
            OscillatorPaneId::Cci => &self.ext_scales.cci_price_scale,
            OscillatorPaneId::Mfi => &self.ext_scales.mfi_price_scale,
            OscillatorPaneId::ProxyBb => &self.ext_scales.proxy_bb_price_scale,
        }
    }

    pub fn oscillator_price_scale_mut(&mut self, id: OscillatorPaneId) -> &mut PriceScale {
        match id {
            OscillatorPaneId::Rsi => &mut self.rsi_price_scale,
            OscillatorPaneId::Macd => &mut self.macd_price_scale,
            OscillatorPaneId::Atr => &mut self.atr_price_scale,
            OscillatorPaneId::Cvd => &mut self.cvd_price_scale,
            OscillatorPaneId::RelativeStrength => &mut self.rs_price_scale,
            OscillatorPaneId::Equity => &mut self.equity_price_scale,
            OscillatorPaneId::Adx => &mut self.ext_scales.adx_price_scale,
            OscillatorPaneId::Stochastic => &mut self.ext_scales.stoch_price_scale,
            OscillatorPaneId::Cci => &mut self.ext_scales.cci_price_scale,
            OscillatorPaneId::Mfi => &mut self.ext_scales.mfi_price_scale,
            OscillatorPaneId::ProxyBb => &mut self.ext_scales.proxy_bb_price_scale,
        }
    }

    #[must_use]
    pub const fn oscillator_stretch(&self, id: OscillatorPaneId) -> f32 {
        match id {
            OscillatorPaneId::Rsi => self.rsi_stretch,
            OscillatorPaneId::Macd => self.macd_stretch,
            OscillatorPaneId::Atr => self.atr_stretch,
            OscillatorPaneId::Cvd => self.cvd_stretch,
            OscillatorPaneId::RelativeStrength => self.rs_stretch,
            OscillatorPaneId::Equity => self.equity_stretch,
            OscillatorPaneId::Adx => self.adx_stretch,
            OscillatorPaneId::Stochastic => self.stoch_stretch,
            OscillatorPaneId::Cci => self.cci_stretch,
            OscillatorPaneId::Mfi => self.mfi_stretch,
            OscillatorPaneId::ProxyBb => self.proxy_bb_stretch,
        }
    }

    pub fn set_oscillator_stretch(&mut self, id: OscillatorPaneId, stretch: f32) {
        let s = sanitize_stretch(stretch, 1.0);
        match id {
            OscillatorPaneId::Rsi => self.rsi_stretch = s,
            OscillatorPaneId::Macd => self.macd_stretch = s,
            OscillatorPaneId::Atr => self.atr_stretch = s,
            OscillatorPaneId::Cvd => self.cvd_stretch = s,
            OscillatorPaneId::RelativeStrength => self.rs_stretch = s,
            OscillatorPaneId::Equity => self.equity_stretch = s,
            OscillatorPaneId::Adx => self.adx_stretch = s,
            OscillatorPaneId::Stochastic => self.stoch_stretch = s,
            OscillatorPaneId::Cci => self.cci_stretch = s,
            OscillatorPaneId::Mfi => self.mfi_stretch = s,
            OscillatorPaneId::ProxyBb => self.proxy_bb_stretch = s,
        }
    }

    /// Resets `auto_scale` across every oscillator / equity sub-pane `PriceScale`.
    pub fn reset_all_oscillator_auto_scales(&mut self) {
        self.rsi_price_scale.reset_auto_scale();
        self.macd_price_scale.reset_auto_scale();
        self.atr_price_scale.reset_auto_scale();
        self.rs_price_scale.reset_auto_scale();
        self.cvd_price_scale.reset_auto_scale();
        self.equity_price_scale.reset_auto_scale();
        self.ext_scales.adx_price_scale.reset_auto_scale();
        self.ext_scales.stoch_price_scale.reset_auto_scale();
        self.ext_scales.cci_price_scale.reset_auto_scale();
        self.ext_scales.mfi_price_scale.reset_auto_scale();
        self.ext_scales.proxy_bb_price_scale.reset_auto_scale();
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

    /// Resets proportional pane stretch ratios to `Main = 4.0`, `Oscillator = 1.0`, `Equity = 1.25` (`PaneManager.ts:76–77`).
    pub fn reset_pane_stretches(&mut self) {
        self.separator_drag = None;
        self.main_stretch = default_main_stretch();
        self.rsi_stretch = default_sub_stretch();
        self.macd_stretch = default_sub_stretch();
        self.atr_stretch = default_sub_stretch();
        self.rs_stretch = default_sub_stretch();
        self.cvd_stretch = default_sub_stretch();
        self.equity_stretch = default_equity_stretch();
        self.adx_stretch = default_sub_stretch();
        self.stoch_stretch = default_sub_stretch();
        self.cci_stretch = default_sub_stretch();
        self.mfi_stretch = default_sub_stretch();
        self.proxy_bb_stretch = default_sub_stretch();
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
        self.reset_all_oscillator_auto_scales();
    }

    /// Restores default `bar_spacing` (`8.0`), default future `right_offset` (`12.0`), and all price autoscales.
    pub fn reset_to_default(&mut self) {
        self.kinetic = None;
        self.separator_drag = None;
        self.r11_memo.clear();
        self.time_scale.restore_default();
        self.main_price_scale.reset_auto_scale();
        self.volume_price_scale.reset_auto_scale();
        self.reset_all_oscillator_auto_scales();
    }

    /// Cancels transient kinetic scrolling and pane-splitter drag gestures (e.g. when a modal opens).
    pub fn cancel_active_interactions(&mut self) {
        self.kinetic = None;
        self.separator_drag = None;
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

#[inline]
pub(crate) fn safe_clamp_f32(val: f32, lo: f32, hi: f32) -> f32 {
    if !lo.is_finite() || !hi.is_finite() {
        return if val.is_finite() { val } else { 0.0 };
    }
    if !val.is_finite() || lo > hi {
        return 0.5 * (lo + hi);
    }
    val.clamp(lo, hi)
}

#[inline]
pub(crate) fn clamp_axis_pill_center_y(y: f32, axis_rect: Rect, desired_pad: f32) -> Option<f32> {
    if !y.is_finite()
        || !axis_rect.min.x.is_finite()
        || !axis_rect.min.y.is_finite()
        || !axis_rect.max.x.is_finite()
        || !axis_rect.max.y.is_finite()
        || axis_rect.height() < 14.0
        || axis_rect.width() < 4.0
    {
        return None;
    }
    let eff_pad = desired_pad
        .max(0.0)
        .min((0.5 * axis_rect.height() - 0.5).max(0.0));
    Some(safe_clamp_f32(
        y,
        axis_rect.top() + eff_pad,
        axis_rect.bottom() - eff_pad,
    ))
}

pub(crate) fn resolve_non_overlapping_axis_pill_y(
    raw_y: f32,
    axis_rect: Rect,
    desired_pad: f32,
    placed_ys: &mut Vec<f32>,
) -> Option<f32> {
    let base_y = clamp_axis_pill_center_y(raw_y, axis_rect, desired_pad)?;
    let eff_pad = desired_pad
        .max(0.0)
        .min((0.5 * axis_rect.height() - 0.5).max(0.0));
    let min_y = axis_rect.top() + eff_pad;
    let max_y = axis_rect.bottom() - eff_pad;
    const MIN_SEP: f32 = 15.0;
    const STEP: f32 = 15.5;

    let is_clear = |cand: f32, ys: &[f32]| ys.iter().all(|&p| (cand - p).abs() >= MIN_SEP);
    if is_clear(base_y, placed_ys) {
        placed_ys.push(base_y);
        return Some(base_y);
    }
    for k in 1..=6 {
        let off = (k as f32) * STEP;
        for cand in [base_y - off, base_y + off] {
            if cand >= min_y - 0.5 && cand <= max_y + 0.5 {
                let clamped = safe_clamp_f32(cand, min_y, max_y);
                if is_clear(clamped, placed_ys) {
                    placed_ys.push(clamped);
                    return Some(clamped);
                }
            }
        }
    }
    None
}

pub(crate) fn place_non_overlapping_order_badge(
    plot_rect: Rect,
    y: f32,
    badge_size: Vec2,
    placed: &mut Vec<Rect>,
) -> Rect {
    let min_x = plot_rect.left() + 8.0;
    let max_x = (plot_rect.right() - badge_size.x - 10.0).max(min_x);
    let min_y = plot_rect.top() + 2.0;
    let max_y = (plot_rect.bottom() - badge_size.y - 2.0).max(min_y);

    let mut x = max_x;
    let mut top_y = safe_clamp_f32(y - badge_size.y * 0.5, min_y, max_y);
    let base_top_y = top_y;
    let mut row_step = 1_usize;

    for _ in 0..16 {
        let cand = Rect::from_min_size(Pos2::new(x, top_y), badge_size);
        if placed.iter().all(|r| !r.intersects(cand.expand(1.5))) {
            placed.push(cand);
            return cand;
        }
        x -= badge_size.x + 6.0;
        if x < min_x {
            x = max_x;
            let shift = ((row_step.div_ceil(2)) as f32) * (badge_size.y + 3.0);
            let next_y = if row_step % 2 == 1 {
                base_top_y - shift
            } else {
                base_top_y + shift
            };
            top_y = safe_clamp_f32(next_y, min_y, max_y);
            row_step += 1;
        }
    }

    let fallback = Rect::from_min_size(Pos2::new(x.max(min_x), top_y), badge_size);
    placed.push(fallback);
    fallback
}

#[inline]
pub(crate) fn time_tick_label_occluded(
    tick_x: f32,
    label: &str,
    occluding_x_span: Option<(f32, f32)>,
) -> bool {
    let Some((occ_l, occ_r)) = occluding_x_span else {
        return false;
    };
    let half_w = ((label.chars().count() as f32) * 3.45 + 6.0).max(34.0);
    let lbl_l = tick_x - half_w;
    let lbl_r = tick_x + half_w;
    lbl_r >= occ_l - 3.0 && lbl_l <= occ_r + 3.0
}

pub(crate) fn compute_time_axis_cursor_pill_rect(
    time_axis_rect: Rect,
    time_label: &str,
    cross_x: f32,
) -> Option<Rect> {
    if !cross_x.is_finite()
        || !time_axis_rect.min.x.is_finite()
        || !time_axis_rect.min.y.is_finite()
        || !time_axis_rect.max.x.is_finite()
        || !time_axis_rect.max.y.is_finite()
        || time_axis_rect.width() < 24.0
        || time_axis_rect.height() < 10.0
    {
        return None;
    }
    let max_half_w = (time_axis_rect.width() * 0.48).max(12.0);
    let pill_half_w = ((time_label.chars().count() as f32) * 3.45 + 9.0)
        .max(42.0)
        .min(max_half_w);
    let lo_cx = time_axis_rect.left() + pill_half_w + 2.0;
    let hi_cx = (time_axis_rect.right() - pill_half_w - 2.0).max(lo_cx);
    let pill_cx = safe_clamp_f32(cross_x, lo_cx, hi_cx);
    Some(Rect::from_min_max(
        Pos2::new(pill_cx - pill_half_w, time_axis_rect.top() + 3.0),
        Pos2::new(pill_cx + pill_half_w, time_axis_rect.bottom() - 3.0),
    ))
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

/// Custom synchronized equity curve series (`Walk-Forward Stitched OOS` or `Multi-Asset Portfolio`)
/// overriding the default single-asset [`BacktestRunResult`] curve inside [`OscillatorPaneId::Equity`].
#[derive(Debug, Clone, Copy)]
pub struct EquityCurveOverlay<'a> {
    pub title_label: &'a str,
    pub baseline_cash: f64,
    pub equity_values: &'a [Option<f64>],
    pub equity_idx: &'a SeriesBlockIndex,
    pub underwater_values: &'a [Option<f64>],
    pub bar_count: usize,
    pub split_bar_index: Option<usize>,
    pub trades: Option<&'a [BacktestTrade]>,
}

/// Optional Bar Replay, Paper Trading, Alert & Strategy Backtest Lab visual overlay state passed to [`render_workstation_chart_full`].
#[derive(Debug, Clone, Copy, Default)]
pub struct ChartReplayTradingOverlay<'a> {
    pub replay_active: bool,
    pub selecting_cut_bar: bool,
    pub replay_status_label: Option<&'a str>,
    pub trading_session: Option<&'a TradingSession>,
    pub alert_engine: Option<&'a AlertEngine>,
    pub backtest_result: Option<&'a BacktestRunResult>,
    pub equity_override: Option<EquityCurveOverlay<'a>>,
    pub multi_strategy_overlay: Option<&'a MultiStrategyComparisonReport>,
    pub monte_carlo_fan: Option<&'a MonteCarloFanSeries>,
    pub show_equity_pane: bool,
    pub show_backtest_trades_on_chart: bool,
    pub focused_backtest_trade_id: Option<u64>,
    pub replay_cutoff_timestamp: Option<i64>,
    pub causal_close_cap: Option<i64>,
    pub frontier_base_timestamp: Option<i64>,
    pub compare_overlay: Option<(&'a str, &'a [Option<f64>], &'a SeriesBlockIndex)>,
    pub rs_benchmark_label: Option<&'a str>,
    pub external_crosshair_timestamp: Option<i64>,
    pub modal_open: bool,
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
    pub active_alert_line_count: usize,
    pub rendered_alert_marker_count: usize,
    pub equity_pane_active: bool,
    pub rendered_backtest_trade_count: usize,
    pub vrvp_poc_price: Option<f64>,
    pub active_frvp_count: usize,
    pub hovered_bar_timestamp: Option<i64>,
    pub ichimoku_active: bool,
    pub market_profile_active: bool,
    pub cvd_pane_active: bool,
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

    let mut active_osc_ids = indicators.active_oscillator_panes();
    let equity_pane_active = replay_trading.show_equity_pane
        && (replay_trading.multi_strategy_overlay.is_some()
            || replay_trading.equity_override.is_some()
            || replay_trading.backtest_result.is_some());
    if equity_pane_active && !active_osc_ids.contains(&OscillatorPaneId::Equity) {
        active_osc_ids.push(OscillatorPaneId::Equity);
    }
    let active_osc_specs: Vec<(OscillatorPaneId, f32)> = active_osc_ids
        .iter()
        .map(|&id| (id, viewport.oscillator_stretch(id)))
        .collect();

    // 1. Pre-compute initial MultiPaneLayout so separator drags can resolve current heights cleanly
    let pre_layout = MultiPaneLayout::compute(outer_rect, viewport.main_stretch, &active_osc_specs);

    let chart_sense = if replay_trading.modal_open {
        Sense::hover()
    } else {
        Sense::click_and_drag()
    };

    let mut sep_responses = Vec::with_capacity(pre_layout.separators.len());
    for sep in &pre_layout.separators {
        let sep_id = ui
            .id()
            .with(("sumi_pane_separator", sep.top_pane, sep.bottom_pane));
        sep_responses.push(ui.interact(sep.rect, sep_id, chart_sense));
    }

    if replay_trading.modal_open {
        viewport.kinetic = None;
        viewport.separator_drag = None;
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
        if replay_trading.modal_open {
            break;
        }
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
        chart_sense,
    );
    let main_price_resp = ui.interact(
        multi_layout.main_price_axis_rect,
        ui.id().with("sumi_price_axis"),
        chart_sense,
    );
    let time_resp = ui.interact(
        multi_layout.time_axis_rect,
        ui.id().with("sumi_time_axis"),
        chart_sense,
    );

    let mut osc_responses = Vec::with_capacity(multi_layout.oscillators.len());
    for osc in &multi_layout.oscillators {
        let p_resp = ui.interact(
            osc.plot_rect,
            ui.id().with(("sumi_osc_plot", osc.id.key())),
            chart_sense,
        );
        let ax_resp = ui.interact(
            osc.price_axis_rect,
            ui.id().with(("sumi_osc_axis", osc.id.key())),
            chart_sense,
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
    let (raw_smooth_scroll, time_ms) = ui.input(|i| (i.smooth_scroll_delta, i.time * 1000.0));
    let smooth_scroll = if replay_trading.modal_open {
        Vec2::ZERO
    } else {
        raw_smooth_scroll
    };
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
            replay_trading.compare_overlay,
        )
        .map_or_else(|| candles[init_strict.start].close, |(bv, _)| bv)
    } else {
        candles[init_strict.start].close
    };
    for osc in &multi_layout.oscillators {
        let ps = viewport.oscillator_price_scale_mut(osc.id);
        if osc.id == OscillatorPaneId::Equity {
            if replay_trading.multi_strategy_overlay.is_some() {
                if ps.is_log() {
                    ps.set_mode(crate::viewport::PriceScaleMode::Normal);
                    ps.reset_auto_scale();
                }
                ps.set_format_mode(crate::viewport::PriceAxisFormatMode::SignedPercentage);
            } else {
                ps.set_format_mode(crate::viewport::PriceAxisFormatMode::Standard);
            }
        }
        if ps.price_range().is_none() || ps.is_auto_scale() {
            refresh_oscillator_autoscale(
                osc.id,
                ps,
                indicator_cache,
                replay_trading.backtest_result,
                replay_trading.equity_override,
                replay_trading.multi_strategy_overlay,
                replay_trading.monte_carlo_fan,
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
        if replay_trading.selecting_cut_bar || replay_trading.modal_open {
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
                )
                .with_candles(candles);
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
                    )
                    .with_candles(candles);
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
    } else if replay_trading.selecting_cut_bar || replay_trading.modal_open {
        drawing_blocked_double_click = true;
    }

    // 5. Process Main Pane, Oscillator Sub-Panes, Right Price Axes, and Shared Time Axis gestures
    // Double-clicking ANY plot canvas resets `time_scale` AND all pane `PriceScale`s synchronously
    // (unless a drawing placement, drag, hover hit, or active modal claimed the gesture).
    if !replay_trading.modal_open {
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
                    cvd_price_scale,
                    rs_price_scale,
                    equity_price_scale,
                    ext_scales,
                    ..
                } = viewport;
                let osc_ps = match osc_id {
                    OscillatorPaneId::Rsi => rsi_price_scale,
                    OscillatorPaneId::Macd => macd_price_scale,
                    OscillatorPaneId::Atr => atr_price_scale,
                    OscillatorPaneId::Cvd => cvd_price_scale,
                    OscillatorPaneId::RelativeStrength => rs_price_scale,
                    OscillatorPaneId::Equity => equity_price_scale,
                    OscillatorPaneId::Adx => &mut ext_scales.adx_price_scale,
                    OscillatorPaneId::Stochastic => &mut ext_scales.stoch_price_scale,
                    OscillatorPaneId::Cci => &mut ext_scales.cci_price_scale,
                    OscillatorPaneId::Mfi => &mut ext_scales.mfi_price_scale,
                    OscillatorPaneId::ProxyBb => &mut ext_scales.proxy_bb_price_scale,
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
    }

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
        replay_trading.compare_overlay,
    )?;
    let strict_range = non_empty_candle_range(
        viewport.time_scale.strict_visible_data_range(),
        candles.len(),
    );
    let compare_scale = replay_trading.compare_overlay.map_or(1.0, |(_, s, idx)| {
        compute_compare_viewport_scale(candles, s, idx, strict_range.clone())
    });
    for osc in &multi_layout.oscillators {
        let ps = viewport.oscillator_price_scale_mut(osc.id);
        if osc.id == OscillatorPaneId::Equity {
            if replay_trading.multi_strategy_overlay.is_some() {
                if ps.is_log() {
                    ps.set_mode(crate::viewport::PriceScaleMode::Normal);
                    ps.reset_auto_scale();
                }
                ps.set_format_mode(crate::viewport::PriceAxisFormatMode::SignedPercentage);
            } else {
                ps.set_format_mode(crate::viewport::PriceAxisFormatMode::Standard);
            }
        }
        refresh_oscillator_autoscale(
            osc.id,
            ps,
            indicator_cache,
            replay_trading.backtest_result,
            replay_trading.equity_override,
            replay_trading.multi_strategy_overlay,
            replay_trading.monte_carlo_fan,
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

    // 8. Compute vertical time grid X coordinates & crosshair occlusion state once before painting panes
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

    let equity_base_val = if replay_trading.multi_strategy_overlay.is_some() {
        1.0
    } else {
        replay_trading
            .equity_override
            .map(|ov| ov.baseline_cash.max(1.0))
            .or_else(|| {
                replay_trading
                    .backtest_result
                    .map(|bt| bt.config.initial_cash.max(1.0))
            })
            .unwrap_or(100_000.0)
    };

    #[derive(Clone, Copy)]
    enum HoveredPane {
        Main(Pos2),
        Oscillator(OscillatorPaneId, Rect, Rect, Pos2),
    }

    let hovered_pane = if replay_trading.modal_open
        || separator_active_idx.is_some_and(|idx| sep_responses[idx].dragged())
    {
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
    let tf_secs = timeframe_str_to_seconds(config.timeframe);
    let linked_bar_idx = if hovered_pane.is_none()
        && config.show_crosshair
        && let Some(ext_ts) = replay_trading.external_crosshair_timestamp
    {
        resolve_linked_crosshair_bar(candles, tf_secs, ext_ts, replay_trading.causal_close_cap)
    } else {
        None
    };
    let default_inspected_idx = culling_range
        .end
        .checked_sub(1)
        .unwrap_or(candles.len() - 1);
    let inspected_data_index = hover_x
        .and_then(|hx| main_vp.screen_x_to_clamped_data_index(f64::from(hx)))
        .or(linked_bar_idx)
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
                let osc_bv = match id {
                    OscillatorPaneId::Equity => equity_base_val,
                    OscillatorPaneId::RelativeStrength => 100.0,
                    OscillatorPaneId::Rsi
                    | OscillatorPaneId::Macd
                    | OscillatorPaneId::Atr
                    | OscillatorPaneId::Cvd
                    | OscillatorPaneId::Adx
                    | OscillatorPaneId::Stochastic
                    | OscillatorPaneId::Cci
                    | OscillatorPaneId::Mfi
                    | OscillatorPaneId::ProxyBb => 1.0,
                };
                let osc_vp = PaneViewport::new(
                    &viewport.time_scale,
                    viewport.oscillator_price_scale(id),
                    PlotRect::from_egui(p_rect),
                    osc_bv,
                );
                hovered_oscillator_value = osc_vp.screen_y_to_price(f64::from(pos.y));
            }
        }
    }

    let snap_screen_x_for_slot = |slot_idx: i64| -> Option<f64> {
        if conflation_factor > 1 && slot_idx >= 0 && (slot_idx as usize) < candles.len() {
            conflated_candle_at(candles, slot_idx as usize, conflation_factor)
                .and_then(|b| main_vp.float_index_to_screen_x(b.center_logical_index))
                .or_else(|| main_vp.index_to_screen_x(slot_idx))
        } else {
            main_vp.index_to_screen_x(slot_idx)
        }
    };

    let mut active_cross_state: Option<(HoveredPane, i64, f32)> = None;
    let mut main_cursor_occluding_y: Option<f32> = None;
    let mut osc_cursor_occluding: Option<(OscillatorPaneId, f32)> = None;
    let mut time_cursor_occluding_span: Option<(f32, f32)> = None;

    if config.show_crosshair
        && let Some(hp) = hovered_pane
    {
        let raw_x = match hp {
            HoveredPane::Main(p) | HoveredPane::Oscillator(_, _, _, p) => p.x,
        };
        let slot_idx = hovered_slot_index.unwrap_or(inspected_data_index as i64);
        let cross_x = safe_clamp_f32(
            snap_screen_x_for_slot(slot_idx).map_or(raw_x, |sx| sx as f32),
            multi_layout.main_plot_rect.left(),
            multi_layout.main_plot_rect.right(),
        );
        active_cross_state = Some((hp, slot_idx, cross_x));
        match hp {
            HoveredPane::Main(pos) => {
                if pointer_price.is_some() {
                    main_cursor_occluding_y =
                        clamp_axis_pill_center_y(pos.y, multi_layout.main_price_axis_rect, 10.0);
                }
            }
            HoveredPane::Oscillator(id, _, ax_rect, pos) => {
                if hovered_oscillator_value.is_some()
                    && let Some(py) = clamp_axis_pill_center_y(pos.y, ax_rect, 10.0)
                {
                    osc_cursor_occluding = Some((id, py));
                }
            }
        }
        let time_lbl = format_time_axis_cursor_pill_label(candles, config.timeframe, slot_idx);
        time_cursor_occluding_span =
            compute_time_axis_cursor_pill_rect(multi_layout.time_axis_rect, &time_lbl, cross_x)
                .map(|r| (r.left(), r.right()));
    } else if config.show_crosshair
        && let Some(l_idx) = linked_bar_idx
        && l_idx < candles.len()
        && let Some(sx) = snap_screen_x_for_slot(l_idx as i64)
    {
        let cross_x = sx as f32;
        if cross_x >= multi_layout.main_plot_rect.left()
            && cross_x <= multi_layout.main_plot_rect.right()
        {
            let close_price = candles[l_idx].close;
            if close_price.is_finite()
                && let Some(sy) = main_vp.price_to_screen_y(close_price)
            {
                let cy = sy as f32;
                if cy >= multi_layout.main_plot_rect.top()
                    && cy <= multi_layout.main_plot_rect.bottom()
                {
                    main_cursor_occluding_y =
                        clamp_axis_pill_center_y(cy, multi_layout.main_price_axis_rect, 10.0);
                }
            }
            let time_lbl =
                format_time_axis_cursor_pill_label(candles, config.timeframe, l_idx as i64);
            time_cursor_occluding_span =
                compute_time_axis_cursor_pill_rect(multi_layout.time_axis_rect, &time_lbl, cross_x)
                    .map(|r| (r.left(), r.right()));
        }
    }

    let time_grid_ticks = collect_time_grid_ticks(
        candles,
        config.timeframe,
        &viewport.time_scale,
        &main_vp,
        multi_layout.main_plot_rect,
        multi_layout.time_axis_rect,
    );

    // 9. Paint Main Pane horizontal price grid + vertical time grid + candlesticks + volume + overlays
    let main_close_pill_y = latest_close_pill_y(
        candles,
        &main_vp,
        multi_layout.main_plot_rect,
        multi_layout.main_price_axis_rect,
    );
    let mut main_seed_pill_ys = [0.0_f32; 2];
    let mut main_seed_pill_len = 0_usize;
    if let Some(py) = main_close_pill_y {
        main_seed_pill_ys[main_seed_pill_len] = py;
        main_seed_pill_len += 1;
    }
    if let Some(cy) = main_cursor_occluding_y {
        main_seed_pill_ys[main_seed_pill_len] = cy;
        main_seed_pill_len += 1;
    }
    let main_seed_slice = &main_seed_pill_ys[..main_seed_pill_len];
    let mut main_occluding_ys = if let Some(session) = replay_trading.trading_session {
        collect_trading_axis_pill_ys(
            session,
            &main_vp,
            multi_layout.main_plot_rect,
            multi_layout.main_price_axis_rect,
            main_seed_slice,
        )
    } else {
        main_seed_slice.to_vec()
    };
    if let Some(alert_eng) = replay_trading.alert_engine {
        append_alert_axis_pill_ys(
            alert_eng,
            drawings.as_deref(),
            candles.len(),
            replay_trading.replay_cutoff_timestamp,
            &main_vp,
            multi_layout.main_plot_rect,
            multi_layout.main_price_axis_rect,
            &mut main_occluding_ys,
        );
    }
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
        &main_occluding_ys,
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
    paint_bottom_time_axis_labels(
        &painter,
        multi_layout.time_axis_rect,
        &time_grid_ticks,
        time_cursor_occluding_span,
    );

    let main_plot_painter = painter.with_clip_rect(multi_layout.main_plot_rect);
    let mut active_overlay_count = 0_usize;
    let candles_fp = if conflation_factor >= 2 || indicators.show_vrvp || drawings.is_some() {
        candles_fast_fingerprint(candles) ^ indicator_cache.dataset_generation().rotate_left(17)
    } else {
        0
    };
    let vrvp_result = if indicators.show_vrvp {
        let vp_cfg = VolumeProfileConfig {
            row_count: indicators.vrvp_rows,
            value_area_pct: indicators.vrvp_value_area_pct,
        };
        let res = viewport.r11_memo.get_or_compute_vrvp(
            candles,
            candles_fp,
            strict_range.clone(),
            vp_cfg,
        );
        if res.is_some() {
            active_overlay_count += 1;
        }
        res
    } else {
        None
    };
    let vrvp_poc_price = vrvp_result.as_deref().map(|r| r.poc_price);
    if let Some(vrvp) = vrvp_result.as_deref() {
        let right_edge = multi_layout.main_plot_rect.right() - 2.0;
        let max_profile_w = (multi_layout.main_plot_rect.width() * 0.22).clamp(48.0, 180.0);
        paint_volume_profile_bars(
            &main_plot_painter,
            &main_vp,
            vrvp,
            right_edge,
            max_profile_w,
            true,
            (
                (right_edge - max_profile_w).max(multi_layout.main_plot_rect.left()),
                multi_layout.main_plot_rect.right(),
            ),
            palette::VRVP_POC_GOLD,
            palette::VRVP_VA_LINE,
            false,
            false,
            None,
        );
    }

    let ov_step = overlay_conflation_step(conflation_factor, viewport.time_scale.bar_spacing());
    let ichimoku_active = indicators.show_ichimoku && !indicator_cache.ichimoku.senkou_a.is_empty();
    let market_profile_active =
        indicators.show_market_profile && !indicator_cache.market_profile.profiles.is_empty();
    let effective_bar_spacing = viewport.time_scale.bar_spacing() * (conflation_factor as f64);
    let bar_metrics = CandlestickBarMetrics::compute(effective_bar_spacing, ppp);

    let plot_memo_key = (conflation_factor >= 2 && painter.is_visible()).then(|| {
        let osc_panes = multi_layout
            .oscillators
            .iter()
            .filter(|osc| osc.id != OscillatorPaneId::Equity)
            .map(|osc| {
                (
                    osc.id,
                    rect_to_bits(osc.plot_rect),
                    PriceScaleMemoKey::from_price_scale(viewport.oscillator_price_scale(osc.id)),
                )
            })
            .collect();
        PlotDataLayerKey {
            layer_id: painter.layer_id(),
            parent_clip_bits: rect_to_bits(painter.clip_rect()),
            opacity_bits: painter.opacity().to_bits(),
            ppp_bits: ppp.to_bits(),
            candles_fp,
            candles_len: candles.len(),
            dataset_gen: indicator_cache.dataset_generation(),
            main_plot_bits: rect_to_bits(multi_layout.main_plot_rect),
            culling_start: culling_range.start,
            culling_end: culling_range.end,
            conflation_factor,
            ov_step,
            time_width_bits: viewport.time_scale.width().to_bits(),
            bar_spacing_bits: viewport.time_scale.bar_spacing().to_bits(),
            right_offset_bits: viewport.time_scale.right_offset().to_bits(),
            base_index: viewport.time_scale.base_index(),
            base_value_bits: base_value.to_bits(),
            show_volume: config.show_volume,
            main_ps: PriceScaleMemoKey::from_price_scale(&viewport.main_price_scale),
            vol_ps: PriceScaleMemoKey::from_price_scale(&viewport.volume_price_scale),
            indicators: indicators.clone(),
            vwap_stamp: (
                indicator_cache.vwap.session_starts.len(),
                series_edge_stamp(&indicator_cache.vwap.vwap),
            ),
            mp_stamp: indicators.show_market_profile.then(|| {
                let ps = &indicator_cache.market_profile.profiles;
                let last_end = ps.last().map_or(0, |s| s.naked_poc_end_index);
                let poc_bits = ps.last().map_or(0, |s| s.poc_price.to_bits());
                (ps.len(), last_end, poc_bits)
            }),
            cvd_stamp: (
                indicator_cache.cvd.session_starts.len(),
                series_edge_stamp(&indicator_cache.cvd.cvd),
            ),
            rs_stamp: series_edge_stamp(&indicator_cache.rs.rs_line),
            compare_stamp: replay_trading.compare_overlay.map(|(sym, s, _)| {
                (
                    sym.to_owned(),
                    series_edge_stamp(s),
                    compare_scale.to_bits(),
                )
            }),
            osc_panes,
        }
    });
    if plot_memo_key.is_none() {
        viewport.r11_memo.plot_data = None;
    }
    let plot_cache_hit = plot_memo_key.as_ref().is_some_and(|k| {
        viewport
            .r11_memo
            .plot_data
            .as_ref()
            .is_some_and(|m| &m.key == k)
    });

    let mut captured_main_shapes = Vec::new();
    let mut captured_static_overlay_count = 0_usize;
    let rendered_candle_count =
        if plot_cache_hit && let Some(memo) = viewport.r11_memo.plot_data.as_ref() {
            replay_cached_shapes(&painter, &memo.main_shapes);
            active_overlay_count += memo.static_overlay_count;
            memo.rendered_candle_count
        } else {
            let main_shape_start = if plot_memo_key.is_some() {
                painter_next_shape_idx(&painter)
            } else {
                0
            };
            let pre_static_overlays = active_overlay_count;

            if market_profile_active {
                active_overlay_count += 1;
                paint_session_market_profiles(
                    &main_plot_painter,
                    multi_layout.main_plot_rect,
                    &viewport.time_scale,
                    indicators,
                    indicator_cache,
                    culling_range.clone(),
                    &main_vp,
                );
            }
            if ichimoku_active {
                paint_ichimoku_kumo_fill(
                    &main_plot_painter,
                    multi_layout.main_plot_rect,
                    &viewport.time_scale,
                    indicator_cache,
                    ov_step,
                    &main_vp,
                );
            }

            let count = paint_candlesticks_and_volume(
                &painter,
                multi_layout.main_plot_rect,
                candles,
                Some(&indicator_cache.candle_idx),
                culling_range.clone(),
                conflation_factor,
                &main_vp,
                &vol_vp,
                bar_metrics,
                ppp,
                config.show_volume,
            );

            // Main-Pane Technical Overlays (`Ichimoku Cloud`, `BollingerBands`, `SMA`, `EMA`, `Session VWAP`, `Compare Benchmark`)
            if ichimoku_active {
                active_overlay_count += 1;
                paint_ichimoku_lines(
                    &main_plot_painter,
                    &viewport.time_scale,
                    indicators,
                    indicator_cache,
                    culling_range.clone(),
                    ov_step,
                    &main_vp,
                );
            }
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
            if indicators.show_keltner {
                active_overlay_count += 1;
                paint_keltner_channels(
                    &main_plot_painter,
                    indicator_cache,
                    culling_range.clone(),
                    conflation_factor,
                    &main_vp,
                );
            }
            if indicators.show_supertrend {
                active_overlay_count += 1;
                paint_supertrend_overlay(
                    &main_plot_painter,
                    indicator_cache,
                    culling_range.clone(),
                    ov_step,
                    &main_vp,
                );
            }
            if indicators.show_psar {
                active_overlay_count += 1;
                paint_psar_overlay(
                    &main_plot_painter,
                    indicator_cache,
                    culling_range.clone(),
                    ov_step,
                    &main_vp,
                    bar_metrics,
                );
            }
            if config.show_volume && indicators.show_volume_sma {
                active_overlay_count += 1;
                paint_conflated_line_series(
                    &main_plot_painter,
                    &indicator_cache.volume_sma,
                    Some(&indicator_cache.volume_sma_idx),
                    culling_range.clone(),
                    ov_step,
                    &vol_vp,
                    Stroke::new(1.35, palette::IND_VOLUME_SMA),
                );
            }
            if indicators.show_candle_patterns {
                active_overlay_count += 1;
                paint_candlestick_pattern_badges(
                    &main_plot_painter,
                    multi_layout.main_plot_rect,
                    candles,
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
                    ov_step,
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
                    ov_step,
                    &main_vp,
                    Stroke::new(1.6, palette::IND_EMA),
                );
            }
            if indicators.show_vwap {
                active_overlay_count += 1;
                paint_vwap_output(
                    &main_plot_painter,
                    &indicator_cache.vwap,
                    Some((
                        &indicator_cache.vwap_idx,
                        &indicator_cache.vwap_u1_idx,
                        &indicator_cache.vwap_l1_idx,
                        &indicator_cache.vwap_u2_idx,
                        &indicator_cache.vwap_l2_idx,
                    )),
                    culling_range.clone(),
                    ov_step,
                    &main_vp,
                    indicators.vwap_band_1 > 0.0,
                    indicators.vwap_band_2 > 0.0,
                    palette::IND_VWAP_LINE,
                );
            }
            if let Some((_, cmp_series, cmp_idx)) = replay_trading.compare_overlay {
                active_overlay_count += 1;
                paint_conflated_line_series_scaled(
                    &main_plot_painter,
                    cmp_series,
                    Some(cmp_idx),
                    culling_range.clone(),
                    ov_step,
                    &main_vp,
                    Stroke::new(1.5, palette::COMPARE_BENCH_LINE),
                    compare_scale,
                );
            }

            if plot_memo_key.is_some() {
                captured_main_shapes = capture_painter_shapes_since(&painter, main_shape_start);
                captured_static_overlay_count =
                    active_overlay_count.saturating_sub(pre_static_overlays);
            }
            count
        };

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
        )
        .with_candles(candles);
        paint_pane_drawings(
            &painter.with_clip_rect(multi_layout.main_price_axis_rect),
            &main_plot_painter,
            multi_layout.main_plot_rect,
            multi_layout.main_price_axis_rect,
            DrawingPaneTarget::Main,
            store,
            &main_draw_proj,
            &viewport.main_price_scale,
            base_value,
            candles,
            culling_range.clone(),
            conflation_factor,
            &main_vp,
            Some(&mut viewport.r11_memo),
            candles_fp,
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
                main_seed_slice,
            )
        } else {
            (0, 0)
        };

    let (active_alert_line_count, rendered_alert_marker_count) =
        if let Some(alerts) = replay_trading.alert_engine {
            paint_alert_overlays(
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
                alerts,
                drawings.as_deref(),
                replay_trading.replay_cutoff_timestamp,
                replay_trading.frontier_base_timestamp,
            )
        } else {
            (0, 0)
        };

    if equity_pane_active
        && let Some(eq_ov) = replay_trading.equity_override
        && let Some(split_idx) = eq_ov.split_bar_index
    {
        paint_oos_split_marker(
            &main_plot_painter,
            multi_layout.main_plot_rect,
            eq_ov.bar_count,
            candles.len(),
            split_idx,
            &main_vp,
            false,
        );
    }

    if (replay_trading.show_backtest_trades_on_chart || equity_pane_active)
        && replay_trading.equity_override.is_none()
        && let Some(bt) = replay_trading.backtest_result
        && let Some(split_idx) = bt.split_bar_index
    {
        paint_oos_split_marker(
            &main_plot_painter,
            multi_layout.main_plot_rect,
            bt.bar_count,
            candles.len(),
            split_idx,
            &main_vp,
            false,
        );
    }

    let rendered_backtest_trade_count = if replay_trading.show_backtest_trades_on_chart {
        let active_trade_source = replay_trading
            .equity_override
            .and_then(|ov| ov.trades.map(|tr| (tr, ov.bar_count)))
            .or_else(|| {
                replay_trading
                    .backtest_result
                    .map(|bt| (bt.trades.as_slice(), bt.bar_count))
            });
        if let Some((trades_slice, src_bar_count)) = active_trade_source {
            paint_backtest_trade_links(
                &main_plot_painter,
                multi_layout.main_plot_rect,
                candles,
                culling_range.clone(),
                conflation_factor,
                &main_vp,
                trades_slice,
                src_bar_count,
                replay_trading.focused_backtest_trade_id,
                replay_trading.replay_cutoff_timestamp,
            )
        } else {
            0
        }
    } else {
        0
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

    // 10. Paint Stacked Oscillator / Strategy Sub-Panes (`RSI`, `MACD`, `ATR`, `Equity`)
    let mut captured_osc_shapes: Vec<(OscillatorPaneId, Vec<egui::epaint::ClippedShape>)> =
        Vec::with_capacity(multi_layout.oscillators.len());
    for osc in &multi_layout.oscillators {
        let osc_ps = viewport.oscillator_price_scale(osc.id);
        let osc_plot_f64 = PlotRect::from_egui(osc.plot_rect);
        let osc_bv = match osc.id {
            OscillatorPaneId::Equity => equity_base_val,
            OscillatorPaneId::RelativeStrength => 100.0,
            OscillatorPaneId::Rsi
            | OscillatorPaneId::Macd
            | OscillatorPaneId::Atr
            | OscillatorPaneId::Cvd
            | OscillatorPaneId::Adx
            | OscillatorPaneId::Stochastic
            | OscillatorPaneId::Cci
            | OscillatorPaneId::Mfi
            | OscillatorPaneId::ProxyBb => 1.0,
        };
        let osc_vp = PaneViewport::new(&viewport.time_scale, osc_ps, osc_plot_f64, osc_bv);

        let mut osc_occluding_ys = [0.0_f32; 2];
        let mut osc_occluding_len = 0_usize;
        if osc.id == OscillatorPaneId::Equity {
            let baseline = if replay_trading.multi_strategy_overlay.is_some() {
                Some(0.0)
            } else {
                replay_trading
                    .equity_override
                    .map(|ov| ov.baseline_cash.max(1.0))
                    .or_else(|| {
                        replay_trading
                            .backtest_result
                            .map(|bt| bt.config.initial_cash.max(1.0))
                    })
            };
            if let Some(bl) = baseline
                && let Some(base_sy) = osc_vp.price_to_screen_y(bl)
            {
                let base_y = base_sy as f32;
                if base_y >= osc.plot_rect.top()
                    && base_y <= osc.plot_rect.bottom()
                    && let Some(py) = clamp_axis_pill_center_y(base_y, osc.price_axis_rect, 8.0)
                {
                    osc_occluding_ys[osc_occluding_len] = py;
                    osc_occluding_len += 1;
                }
            }
        }
        if let Some((hid, hy)) = osc_cursor_occluding
            && hid == osc.id
        {
            osc_occluding_ys[osc_occluding_len] = hy;
            osc_occluding_len += 1;
        }

        paint_horizontal_price_grid_and_ticks(
            &painter,
            osc.plot_rect,
            osc.price_axis_rect,
            osc_ps,
            &osc_vp,
            osc_bv,
            config.show_grid,
            24.0,
            3,
            6,
            &osc_occluding_ys[..osc_occluding_len],
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
        if osc.id == OscillatorPaneId::Equity {
            if let Some(ms) = replay_trading.multi_strategy_overlay {
                let ms_crosshair_mark: Option<(usize, f32)> = if let Some((_, slot_idx, cross_x)) =
                    active_cross_state
                {
                    if slot_idx >= 0 && (slot_idx as usize) < candles.len() {
                        let idx = slot_idx as usize;
                        let hud_idx = conflated_candle_at(candles, idx, conflation_factor)
                            .map_or(idx, |b| b.end_index);
                        Some((hud_idx, cross_x))
                    } else {
                        None
                    }
                } else if config.show_crosshair
                    && let Some(l_idx) = linked_bar_idx
                    && l_idx < candles.len()
                {
                    let snapped_sx = if conflation_factor > 1 {
                        conflated_candle_at(candles, l_idx, conflation_factor)
                            .and_then(|b| osc_vp.float_index_to_screen_x(b.center_logical_index))
                            .or_else(|| osc_vp.index_to_screen_x(l_idx as i64))
                    } else {
                        osc_vp.index_to_screen_x(l_idx as i64)
                    };
                    let hud_idx = conflated_candle_at(candles, l_idx, conflation_factor)
                        .map_or(l_idx, |b| b.end_index);
                    snapped_sx
                        .map(|sx| sx as f32)
                        .filter(|&cx| cx >= osc.plot_rect.left() && cx <= osc.plot_rect.right())
                        .map(|cx| (hud_idx, cx))
                } else {
                    None
                };
                paint_multi_strategy_equity_pane(
                    &painter,
                    &osc_painter,
                    osc.plot_rect,
                    osc.price_axis_rect,
                    ms,
                    candles.len(),
                    culling_range.clone(),
                    conflation_factor,
                    &osc_vp,
                    ms_crosshair_mark,
                );
            } else if let Some(eq_ov) = replay_trading.equity_override {
                paint_equity_baseline_pane(
                    &painter,
                    &osc_painter,
                    osc.plot_rect,
                    osc.price_axis_rect,
                    eq_ov,
                    replay_trading.monte_carlo_fan,
                    candles.len(),
                    culling_range.clone(),
                    conflation_factor,
                    osc_ps,
                    &osc_vp,
                );
            } else if let Some(bt) = replay_trading.backtest_result {
                let eq_ov = EquityCurveOverlay {
                    title_label: bt.config.preset.short_label(),
                    baseline_cash: bt.config.initial_cash,
                    equity_values: &bt.equity_values,
                    equity_idx: &bt.equity_idx,
                    underwater_values: &bt.underwater_values,
                    bar_count: bt.bar_count,
                    split_bar_index: bt.split_bar_index,
                    trades: None,
                };
                paint_equity_baseline_pane(
                    &painter,
                    &osc_painter,
                    osc.plot_rect,
                    osc.price_axis_rect,
                    eq_ov,
                    replay_trading.monte_carlo_fan,
                    candles.len(),
                    culling_range.clone(),
                    conflation_factor,
                    osc_ps,
                    &osc_vp,
                );
            }
        } else if plot_cache_hit
            && let Some(memo) = viewport.r11_memo.plot_data.as_ref()
            && let Some((_, shapes)) = memo.osc_shapes.iter().find(|(id, _)| *id == osc.id)
        {
            replay_cached_shapes(&painter, shapes);
        } else {
            let osc_shape_start = if plot_memo_key.is_some() {
                painter_next_shape_idx(&painter)
            } else {
                0
            };
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
                OscillatorPaneId::Cvd => {
                    paint_cvd_pane(
                        &osc_painter,
                        osc.plot_rect,
                        indicator_cache,
                        culling_range.clone(),
                        ov_step,
                        &osc_vp,
                        bar_metrics,
                    );
                }
                OscillatorPaneId::RelativeStrength => {
                    paint_relative_strength_pane(
                        &osc_painter,
                        osc.plot_rect,
                        indicator_cache,
                        culling_range.clone(),
                        conflation_factor,
                        &osc_vp,
                    );
                }
                OscillatorPaneId::Adx => {
                    paint_adx_pane(
                        &osc_painter,
                        osc.plot_rect,
                        indicator_cache,
                        culling_range.clone(),
                        conflation_factor,
                        &osc_vp,
                    );
                }
                OscillatorPaneId::Stochastic => {
                    paint_stochastic_pane(
                        &osc_painter,
                        osc.plot_rect,
                        indicator_cache,
                        culling_range.clone(),
                        conflation_factor,
                        &osc_vp,
                    );
                }
                OscillatorPaneId::Cci => {
                    paint_cci_pane(
                        &osc_painter,
                        osc.plot_rect,
                        indicator_cache,
                        culling_range.clone(),
                        conflation_factor,
                        &osc_vp,
                    );
                }
                OscillatorPaneId::Mfi => {
                    paint_mfi_pane(
                        &osc_painter,
                        osc.plot_rect,
                        indicator_cache,
                        culling_range.clone(),
                        conflation_factor,
                        &osc_vp,
                    );
                }
                OscillatorPaneId::ProxyBb => {
                    paint_proxy_bb_pane(
                        &osc_painter,
                        osc.plot_rect,
                        indicator_cache,
                        culling_range.clone(),
                        conflation_factor,
                        &osc_vp,
                    );
                }
                OscillatorPaneId::Equity => {}
            }
            if plot_memo_key.is_some() {
                captured_osc_shapes.push((
                    osc.id,
                    capture_painter_shapes_since(&painter, osc_shape_start),
                ));
            }
        }

        if let Some(store) = drawings.as_deref() {
            let osc_draw_proj = PaneDrawingProjection::new(
                &viewport.time_scale,
                osc_ps,
                osc_plot_f64.width,
                osc_plot_f64.height,
                osc_bv,
            )
            .with_candles(candles);
            paint_pane_drawings(
                &painter.with_clip_rect(osc.price_axis_rect),
                &osc_painter,
                osc.plot_rect,
                osc.price_axis_rect,
                DrawingPaneTarget::Oscillator(osc.id),
                store,
                &osc_draw_proj,
                osc_ps,
                osc_bv,
                candles,
                culling_range.clone(),
                conflation_factor,
                &osc_vp,
                None,
                candles_fp,
            );
        }
    }

    if !plot_cache_hit && let Some(key) = plot_memo_key {
        viewport.r11_memo.plot_data = Some(PlotDataLayerMemo {
            key,
            main_shapes: captured_main_shapes,
            osc_shapes: captured_osc_shapes,
            rendered_candle_count,
            static_overlay_count: captured_static_overlay_count,
        });
    }

    // 11. Paint Unified Synchronized Crosshair across Main + Oscillator Sub-Panes
    if let Some((hp, slot_idx, cross_x)) = active_cross_state {
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
                    let osc_bv = match id {
                        OscillatorPaneId::Equity => equity_base_val,
                        OscillatorPaneId::RelativeStrength => 100.0,
                        OscillatorPaneId::Rsi
                        | OscillatorPaneId::Macd
                        | OscillatorPaneId::Atr
                        | OscillatorPaneId::Cvd
                        | OscillatorPaneId::Adx
                        | OscillatorPaneId::Stochastic
                        | OscillatorPaneId::Cci
                        | OscillatorPaneId::Mfi
                        | OscillatorPaneId::ProxyBb => 1.0,
                    };
                    paint_axis_cursor_pill(
                        &painter,
                        ax_rect,
                        pos.y,
                        &viewport
                            .oscillator_price_scale(id)
                            .format_price_for_tick_density(ov, osc_bv, 24.0, 3, 6),
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
    } else if config.show_crosshair
        && let Some(l_idx) = linked_bar_idx
    {
        paint_linked_crosshair(
            &painter,
            &main_plot_painter,
            &multi_layout,
            candles,
            config.timeframe,
            &viewport.main_price_scale,
            &main_vp,
            base_value,
            conflation_factor,
            l_idx,
        );
    }

    // 11b. Interactive Bar Replay Cut Scrubber (`selecting_cut_bar == true`)
    let mut clicked_cut_bar_index = None;
    if replay_trading.selecting_cut_bar && !replay_trading.modal_open {
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
            vrvp_result.as_deref(),
            replay_trading
                .compare_overlay
                .map(|(sym, series, _)| (sym, series, compare_scale)),
            &viewport.main_price_scale,
            base_value,
        );
    }
    for osc in &multi_layout.oscillators {
        paint_oscillator_hud(
            &painter,
            osc.plot_rect,
            osc.id,
            indicators,
            indicator_cache,
            replay_trading.backtest_result,
            replay_trading.equity_override,
            replay_trading.multi_strategy_overlay,
            replay_trading.monte_carlo_fan,
            replay_trading.rs_benchmark_label,
            hud_series_index,
        );
    }

    let (drawing_count, active_frvp_count, active_drawing_tool, selected_drawing_id, magnet_mode) =
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
            let frvp_count = store
                .drawings
                .iter()
                .filter(|d| {
                    d.is_causally_visible(cutoff, candles.len())
                        && matches!(d.geometry, DrawingGeometry::FixedRangeVolumeProfile { .. })
                })
                .count();
            (
                visible_count,
                frvp_count,
                store.active_tool,
                store.selected_id,
                store.magnet_mode,
            )
        } else {
            (0, 0, DrawingTool::Select, None, MagnetMode::Off)
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
        active_alert_line_count,
        rendered_alert_marker_count,
        equity_pane_active,
        rendered_backtest_trade_count,
        vrvp_poc_price,
        active_frvp_count,
        hovered_bar_timestamp: hovered_slot_index
            .filter(|&bi| bi >= 0 && (bi as usize) < candles.len())
            .map(|bi| candles[bi as usize].timestamp),
        ichimoku_active,
        market_profile_active,
        cvd_pane_active: active_osc_ids.contains(&OscillatorPaneId::Cvd),
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

    let single_cross_state = if config.show_crosshair
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
        let cross_x = safe_clamp_f32(
            snapped_screen_x.map_or(pos.x, |sx| sx as f32),
            layout.plot_rect.left(),
            layout.plot_rect.right(),
        );
        Some((pos, slot_idx, cross_x))
    } else {
        None
    };

    let single_close_pill_y =
        latest_close_pill_y(candles, &main_vp, layout.plot_rect, layout.price_axis_rect);
    let mut single_occluding_ys = Vec::with_capacity(8);
    if let Some(py) = single_close_pill_y {
        single_occluding_ys.push(py);
    }
    if let Some((pos, _, _)) = single_cross_state
        && pointer_price.is_some()
        && let Some(py) = clamp_axis_pill_center_y(pos.y, layout.price_axis_rect, 10.0)
    {
        single_occluding_ys.push(py);
    }

    let single_time_occluding_span = single_cross_state.and_then(|(_, slot_idx, cross_x)| {
        let time_label = format_time_axis_cursor_pill_label(candles, config.timeframe, slot_idx);
        compute_time_axis_cursor_pill_rect(layout.time_axis_rect, &time_label, cross_x)
            .map(|r| (r.left(), r.right()))
    });

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
        &single_occluding_ys,
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
    paint_bottom_time_axis_labels(
        &painter,
        layout.time_axis_rect,
        &time_grid_ticks,
        single_time_occluding_span,
    );

    let effective_bar_spacing = shared_time_scale.bar_spacing() * (conflation_factor as f64);
    let bar_metrics = CandlestickBarMetrics::compute(effective_bar_spacing, ppp);
    let rendered_candle_count = paint_candlesticks_and_volume(
        &painter,
        layout.plot_rect,
        candles,
        None,
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

    if let Some((pos, slot_idx, cross_x)) = single_cross_state {
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
        active_alert_line_count: 0,
        rendered_alert_marker_count: 0,
        equity_pane_active: false,
        rendered_backtest_trade_count: 0,
        vrvp_poc_price: None,
        active_frvp_count: 0,
        hovered_bar_timestamp: hover_pos
            .is_some()
            .then(|| candles.get(inspected_data_index).map(|c| c.timestamp))
            .flatten(),
        ichimoku_active: false,
        market_profile_active: false,
        cvd_pane_active: false,
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

fn latest_close_pill_y(
    candles: &[Candle],
    vp: &PaneViewport<'_>,
    plot_rect: Rect,
    price_axis_rect: Rect,
) -> Option<f32> {
    candles
        .last()
        .and_then(|c| vp.price_to_screen_y(c.close))
        .map(|sy| sy as f32)
        .filter(|&y| y >= plot_rect.top() && y <= plot_rect.bottom())
        .and_then(|y| clamp_axis_pill_center_y(y, price_axis_rect, 10.0))
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
    occluding_pill_ys: &[f32],
) {
    let target_ticks =
        ((plot_rect.height().max(0.0) / px_per_tick).floor() as usize).clamp(min_ticks, max_ticks);
    let ticks = price_scale.nice_ticks(target_ticks, base_value);
    let axis_painter = painter.with_clip_rect(price_axis_rect);
    for tick in &ticks {
        let y = vp.local_y_to_screen_y(tick.local_y) as f32;
        if !y.is_finite() || y < plot_rect.top() || y > plot_rect.bottom() {
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
        let axis_h = price_axis_rect.height();
        if axis_h < 10.0 || price_axis_rect.width() < 4.0 {
            continue;
        }
        axis_painter.line_segment(
            [
                Pos2::new(price_axis_rect.left(), y),
                Pos2::new(price_axis_rect.left() + 5.0, y),
            ],
            Stroke::new(1.0, palette::BORDER_SUBTLE),
        );
        let pad_y = crate::viewport::axis_edge_pad_px(f64::from(axis_h)) as f32;
        let label_y = safe_clamp_f32(
            y,
            price_axis_rect.top() + pad_y,
            price_axis_rect.bottom() - pad_y,
        );
        // Suppress grid tick text occluded by active right-axis price pills (keeps gridline & tick).
        if occluding_pill_ys
            .iter()
            .any(|&py| (label_y - py).abs() < 16.0)
        {
            continue;
        }
        axis_painter.text(
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
    occluding_x_span: Option<(f32, f32)>,
) {
    if time_axis_rect.height() < 6.0 || time_axis_rect.width() < 12.0 {
        return;
    }
    let time_axis_painter = painter.with_clip_rect(time_axis_rect);
    for tick in ticks {
        if let Some(label) = tick.label.as_deref() {
            time_axis_painter.line_segment(
                [
                    Pos2::new(tick.x, time_axis_rect.top()),
                    Pos2::new(tick.x, time_axis_rect.top() + 5.0),
                ],
                Stroke::new(1.0, palette::BORDER_SUBTLE),
            );
            if time_tick_label_occluded(tick.x, label, occluding_x_span) {
                continue;
            }
            time_axis_painter.text(
                Pos2::new(tick.x, time_axis_rect.center().y),
                Align2::CENTER_CENTER,
                label,
                FontId::monospace(11.0),
                palette::TEXT_MUTED,
            );
        }
    }
}

#[inline]
fn overlay_conflation_step(conflation_factor: usize, bar_spacing: f64) -> usize {
    let mut step = conflation_factor.max(1);
    if step >= 16 && bar_spacing.is_finite() && bar_spacing > 0.0 {
        while step < 512 && bar_spacing * (step as f64) < 1.25 {
            step *= 2;
        }
    }
    step.min(512)
}

#[allow(clippy::too_many_arguments)]
fn paint_candlesticks_and_volume(
    painter: &egui::Painter,
    plot_rect: Rect,
    candles: &[Candle],
    candle_idx: Option<&CandleBlockIndex>,
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

    let step = conflation_factor.max(1);
    let clamped_len = culling_range
        .end
        .min(candles.len())
        .saturating_sub(culling_range.start.min(candles.len()));
    let est_bars = clamped_len.div_ceil(step).saturating_add(2);
    let half_wick_w = (wick_stroke_w * 0.5).max(0.45);
    let single_rect_mode = half_body_w <= half_wick_w + 0.08;

    let vol_layer_slot = show_volume.then(|| plot_painter.add(Shape::Noop));
    let mut vol_mesh = Mesh::default();
    if show_volume {
        vol_mesh.vertices.reserve(est_bars * 4);
        vol_mesh.indices.reserve(est_bars * 6);
    }
    let mut candle_mesh = Mesh::default();
    candle_mesh
        .vertices
        .reserve(est_bars * if single_rect_mode { 4 } else { 12 });
    candle_mesh
        .indices
        .reserve(est_bars * if single_rect_mode { 6 } else { 18 });

    let mut push_bar = |center_logical_index: f64,
                        open: f64,
                        high: f64,
                        low: f64,
                        close: f64,
                        volume: f64,
                        is_bullish: bool| {
        let Some(sx) = main_vp.float_index_to_screen_x(center_logical_index) else {
            return;
        };
        let cx = sx as f32;
        if cx + half_body_w < plot_rect.left() || cx - half_body_w > plot_rect.right() {
            return;
        }

        if show_volume {
            let safe_vol_hi = (plot_bottom - 1.0).max(plot_top);
            let vol_top_y = vol_vp.price_to_screen_y(volume).map_or(safe_vol_hi, |sy| {
                safe_clamp_f32(sy as f32, plot_top, safe_vol_hi)
            });
            let fill = if is_bullish {
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
            main_vp.price_to_screen_y(high),
            main_vp.price_to_screen_y(low),
            main_vp.price_to_screen_y(open),
            main_vp.price_to_screen_y(close),
        ) else {
            return;
        };

        rendered_candle_count += 1;

        let high_y = high_sy as f32;
        let low_y = low_sy as f32;
        let open_y = open_sy as f32;
        let close_y = close_sy as f32;

        let color = if is_bullish {
            palette::BULLISH
        } else {
            palette::BEARISH
        };

        let top_y = open_y.min(close_y);
        let bottom_y = open_y.max(close_y).max(top_y + min_body_h);
        let wick_top_y = high_y.min(low_y);
        let wick_bottom_y = high_y.max(low_y);

        if single_rect_mode {
            let half_w = half_wick_w.max(half_body_w);
            candle_mesh.add_colored_rect(
                Rect::from_min_max(
                    Pos2::new(cx - half_w, wick_top_y.min(top_y)),
                    Pos2::new(cx + half_w, wick_bottom_y.max(bottom_y)),
                ),
                color,
            );
            return;
        }

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
    };

    if let Some(idx) = candle_idx {
        idx.for_each_conflated_bucket(candles, culling_range, conflation_factor, push_bar);
    } else {
        for bar in iter_conflated_candles(candles, culling_range, conflation_factor) {
            push_bar(
                bar.center_logical_index,
                bar.open,
                bar.high,
                bar.low,
                bar.close,
                bar.volume,
                bar.is_bullish(),
            );
        }
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
    resolve_bucket_screen_ys_scaled(env, vp, 1.0)
}

#[inline]
fn resolve_bucket_screen_ys_scaled(
    env: &BucketLineEnvelope,
    vp: &PaneViewport<'_>,
    value_scale: f64,
) -> Option<(f32, f32, bool)> {
    let v0 = env.entry_val * value_scale;
    if !v0.is_finite() {
        return None;
    }
    let sy0 = vp.price_to_screen_y(v0)? as f32;
    if env.is_single {
        return Some((sy0, sy0, true));
    }
    let v1 = env.exit_val * value_scale;
    if !v1.is_finite() {
        return None;
    }
    let sy1 = vp.price_to_screen_y(v1)? as f32;
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

#[inline]
fn next_distinct_polyline_pt(pts: &[Pos2], after: usize) -> Option<usize> {
    let anchor = pts[after];
    let mut j = after + 1;
    while j < pts.len() {
        let dx = pts[j].x - anchor.x;
        let dy = pts[j].y - anchor.y;
        if dx * dx + dy * dy > 1e-4 {
            return Some(j);
        }
        j += 1;
    }
    None
}

#[inline]
fn polyline_unit_normal(p0: Pos2, p1: Pos2) -> Vec2 {
    let dx = p1.x - p0.x;
    let dy = p1.y - p0.y;
    if dx.abs() <= 1e-4 {
        Vec2::new(if dy > 0.0 { -1.0 } else { 1.0 }, 0.0)
    } else {
        let inv = (dx * dx + dy * dy).sqrt().recip();
        Vec2::new(-dy * inv, dx * inv)
    }
}

/// Appends a connected `2 * N`-vertex triangle strip along `pts` into `mesh` to avoid per-segment stroke feathering.
fn append_polyline_quad_strip(mesh: &mut Mesh, pts: &[Pos2], stroke: Stroke) {
    if pts.is_empty() {
        return;
    }
    let Some(mut i1) = next_distinct_polyline_pt(pts, 0) else {
        let r = (stroke.width * 0.9).max(1.2);
        mesh.add_colored_rect(
            Rect::from_center_size(pts[0], Vec2::splat(r * 2.0)),
            stroke.color,
        );
        return;
    };

    let half_w = (stroke.width * 0.5).max(0.55);
    let mut n_prev = polyline_unit_normal(pts[0], pts[i1]);
    let mut prev_b = mesh.vertices.len() as u32;
    mesh.colored_vertex(pts[0] + n_prev * half_w, stroke.color);
    mesh.colored_vertex(pts[0] - n_prev * half_w, stroke.color);

    while let Some(i2) = next_distinct_polyline_pt(pts, i1) {
        let n_next = polyline_unit_normal(pts[i1], pts[i2]);
        let sum = n_prev + n_next;
        let sum_sq = sum.length_sq();
        let miter = if sum_sq <= 1e-4 {
            n_prev * half_w
        } else {
            let miter_n = sum * sum_sq.sqrt().recip();
            let cos_half = miter_n.dot(n_prev);
            if cos_half > 1e-3 {
                miter_n * (half_w / cos_half).clamp(half_w, half_w * 1.35)
            } else {
                n_prev * half_w
            }
        };
        let b = mesh.vertices.len() as u32;
        mesh.colored_vertex(pts[i1] + miter, stroke.color);
        mesh.colored_vertex(pts[i1] - miter, stroke.color);
        mesh.add_triangle(prev_b, prev_b + 1, b);
        mesh.add_triangle(prev_b + 1, b + 1, b);
        prev_b = b;
        i1 = i2;
        n_prev = n_next;
    }

    let b = mesh.vertices.len() as u32;
    mesh.colored_vertex(pts[i1] + n_prev * half_w, stroke.color);
    mesh.colored_vertex(pts[i1] - n_prev * half_w, stroke.color);
    mesh.add_triangle(prev_b, prev_b + 1, b);
    mesh.add_triangle(prev_b + 1, b + 1, b);
}

#[allow(clippy::too_many_arguments)]
fn append_conflated_line_segment(
    painter: &egui::Painter,
    dense_mesh: &mut Option<Mesh>,
    pts_scratch: &mut Vec<Pos2>,
    series: &[Option<f64>],
    block_index: Option<&SeriesBlockIndex>,
    global_offset: usize,
    hard_range: std::ops::Range<usize>,
    visible_range: std::ops::Range<usize>,
    conflation_factor: usize,
    vp: &PaneViewport<'_>,
    stroke: Stroke,
    value_scale: f64,
) {
    let step = conflation_factor.max(1);
    let total = series.len();
    let hard_start = hard_range.start.min(total);
    let hard_end = hard_range.end.min(total);
    let vis_start = visible_range.start.max(hard_start).min(hard_end);
    let vis_end = visible_range.end.max(vis_start).min(hard_end);
    if vis_start >= vis_end {
        return;
    }

    pts_scratch.clear();
    let flush_segment =
        |painter: &egui::Painter, dense_mesh: &mut Option<Mesh>, pts: &mut Vec<Pos2>| {
            if let Some(mesh) = dense_mesh.as_mut() {
                append_polyline_quad_strip(mesh, pts, stroke);
                pts.clear();
            } else if pts.len() >= 2 {
                painter.add(Shape::line(std::mem::take(pts), stroke));
            } else if let Some(&single_pt) = pts.first() {
                painter.circle_filled(single_pt, (stroke.width * 0.9).max(1.2), stroke.color);
                pts.clear();
            }
        };

    if step == 1 {
        for (idx, item) in series.iter().enumerate().take(vis_end).skip(vis_start) {
            if let Some(v) = item.map(|raw| raw * value_scale).filter(|v| v.is_finite())
                && let (Some(sx), Some(sy)) = (
                    vp.float_index_to_screen_x((global_offset + idx) as f64),
                    vp.price_to_screen_y(v),
                )
            {
                pts_scratch.push(Pos2::new(sx as f32, sy as f32));
            } else {
                flush_segment(painter, dense_mesh, pts_scratch);
            }
        }
        flush_segment(painter, dense_mesh, pts_scratch);
        return;
    }

    let g_hard_start = global_offset + hard_start;
    let g_hard_end = global_offset + hard_end;
    let aligned_global_start = ((global_offset + vis_start) / step) * step;
    let aligned_global_end = (global_offset + vis_end)
        .div_ceil(step)
        .saturating_mul(step)
        .min(g_hard_end);
    let mut g_start = aligned_global_start;

    while g_start < aligned_global_end {
        let g_end = (g_start + step).min(g_hard_end);
        let b_start = g_start.max(g_hard_start).saturating_sub(global_offset);
        let b_end = g_end.saturating_sub(global_offset).min(hard_end);
        if b_start < b_end
            && let Some(env) = bucket_line_envelope_indexed(series, block_index, b_start, b_end)
            && let (Some(sx), Some((y_entry, y_exit, single))) = (
                vp.float_index_to_screen_x(global_offset as f64 + env.center_logical_index),
                resolve_bucket_screen_ys_scaled(&env, vp, value_scale),
            )
        {
            let x = sx as f32;
            if env.has_leading_gap {
                flush_segment(painter, dense_mesh, pts_scratch);
            }
            if env.has_internal_gap {
                pts_scratch.push(Pos2::new(x, y_entry));
                flush_segment(painter, dense_mesh, pts_scratch);
                pts_scratch.push(Pos2::new(x, y_exit));
            } else if single {
                pts_scratch.push(Pos2::new(x, y_entry));
            } else {
                pts_scratch.push(Pos2::new(x, y_entry));
                pts_scratch.push(Pos2::new(x, y_exit));
            }
            if env.has_trailing_gap {
                flush_segment(painter, dense_mesh, pts_scratch);
            }
        } else {
            flush_segment(painter, dense_mesh, pts_scratch);
        }
        g_start = g_start.saturating_add(step);
    }
    flush_segment(painter, dense_mesh, pts_scratch);
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
    paint_conflated_line_series_scaled(
        painter,
        series,
        block_index,
        culling_range,
        conflation_factor,
        vp,
        stroke,
        1.0,
    );
}

#[allow(clippy::too_many_arguments)]
fn paint_conflated_line_series_scaled(
    painter: &egui::Painter,
    series: &[Option<f64>],
    block_index: Option<&SeriesBlockIndex>,
    culling_range: std::ops::Range<usize>,
    conflation_factor: usize,
    vp: &PaneViewport<'_>,
    stroke: Stroke,
    value_scale: f64,
) {
    let step = conflation_factor.max(1);
    let total = series.len();
    let mut dense_mesh = (step >= 8).then(|| Mesh::with_texture(egui::TextureId::default()));
    let mut pts_scratch = Vec::with_capacity(256);
    append_conflated_line_segment(
        painter,
        &mut dense_mesh,
        &mut pts_scratch,
        series,
        block_index,
        0,
        0..total,
        culling_range,
        step,
        vp,
        stroke,
        value_scale,
    );
    if let Some(mesh) = dense_mesh
        && !mesh.is_empty()
    {
        painter.add(Shape::mesh(mesh));
    }
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

fn paint_keltner_channels(
    painter: &egui::Painter,
    cache: &IndicatorCache,
    culling_range: std::ops::Range<usize>,
    conflation_factor: usize,
    vp: &PaneViewport<'_>,
) {
    let kc = &cache.keltner;
    let step = conflation_factor.max(1);
    let total = kc.middle.len();
    let clamped_start = culling_range.start.min(total);
    let clamped_end = culling_range.end.min(total);
    if clamped_start < clamped_end {
        let mut mesh = Mesh::default();
        let mut prev_exit: Option<(f32, f32, f32)> = None;
        let aligned_start = (clamped_start / step) * step;
        let aligned_end = clamped_end.div_ceil(step).saturating_mul(step).min(total);
        let mut b_start = aligned_start;

        while b_start < aligned_end {
            let b_end = (b_start + step).min(total);
            let u_env_opt =
                bucket_line_envelope_indexed(&kc.upper, Some(&cache.kc_upper_idx), b_start, b_end);
            let l_env_opt =
                bucket_line_envelope_indexed(&kc.lower, Some(&cache.kc_lower_idx), b_start, b_end);

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
                    mesh.colored_vertex(Pos2::new(px, tl), palette::IND_KELTNER_FILL);
                    mesh.colored_vertex(Pos2::new(px, bl), palette::IND_KELTNER_FILL);
                    mesh.colored_vertex(Pos2::new(x, tr), palette::IND_KELTNER_FILL);
                    mesh.colored_vertex(Pos2::new(x, br), palette::IND_KELTNER_FILL);
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

    let edge_stroke = Stroke::new(1.2, palette::IND_KELTNER_BORDER);
    let mid_stroke = Stroke::new(1.2, palette::IND_KELTNER_MIDDLE);
    paint_conflated_line_series(
        painter,
        &kc.upper,
        Some(&cache.kc_upper_idx),
        culling_range.clone(),
        conflation_factor,
        vp,
        edge_stroke,
    );
    paint_conflated_line_series(
        painter,
        &kc.middle,
        Some(&cache.kc_middle_idx),
        culling_range.clone(),
        conflation_factor,
        vp,
        mid_stroke,
    );
    paint_conflated_line_series(
        painter,
        &kc.lower,
        Some(&cache.kc_lower_idx),
        culling_range,
        conflation_factor,
        vp,
        edge_stroke,
    );
}

fn paint_supertrend_overlay(
    painter: &egui::Painter,
    cache: &IndicatorCache,
    culling_range: std::ops::Range<usize>,
    ov_step: usize,
    vp: &PaneViewport<'_>,
) {
    let st = &cache.supertrend;
    let total = st.trend.len().min(st.direction.len());
    let start = culling_range.start.min(total);
    let end = culling_range.end.min(total);
    if start >= end {
        return;
    }
    let step = ov_step.max(1);
    let mut run_dir: Option<i8> = None;
    let mut run_pts: Vec<Pos2> = Vec::new();
    let flush_run = |painter: &egui::Painter, dir: Option<i8>, pts: &mut Vec<Pos2>| {
        if let Some(d) = dir {
            let col = if d >= 0 {
                palette::IND_SUPERTREND_BULL
            } else {
                palette::IND_SUPERTREND_BEAR
            };
            if pts.len() >= 2 {
                painter.add(Shape::line(std::mem::take(pts), Stroke::new(1.8, col)));
            } else if let Some(&p0) = pts.first() {
                painter.circle_filled(p0, 2.0, col);
                pts.clear();
            }
        } else {
            pts.clear();
        }
    };

    let mut i = start;
    while i < end {
        match (st.trend[i], st.direction[i]) {
            (Some(px), Some(dir)) if px.is_finite() => {
                if let (Some(sx), Some(sy)) =
                    (vp.index_to_screen_x(i as i64), vp.price_to_screen_y(px))
                {
                    let pt = Pos2::new(sx as f32, sy as f32);
                    if run_dir != Some(dir) {
                        flush_run(painter, run_dir, &mut run_pts);
                        run_dir = Some(dir);
                    }
                    run_pts.push(pt);
                } else {
                    flush_run(painter, run_dir, &mut run_pts);
                    run_dir = None;
                }
            }
            _ => {
                flush_run(painter, run_dir, &mut run_pts);
                run_dir = None;
            }
        }
        i = i.saturating_add(step);
    }
    flush_run(painter, run_dir, &mut run_pts);
}

fn paint_psar_overlay(
    painter: &egui::Painter,
    cache: &IndicatorCache,
    culling_range: std::ops::Range<usize>,
    ov_step: usize,
    vp: &PaneViewport<'_>,
    bar_metrics: CandlestickBarMetrics,
) {
    let psar = &cache.psar;
    let total = psar.sar.len().min(psar.is_long.len());
    let start = culling_range.start.min(total);
    let end = culling_range.end.min(total);
    if start >= end {
        return;
    }
    let step = ov_step.max(1);
    let radius = ((bar_metrics.body_width_logical as f32) * 0.28).clamp(1.6, 3.4);
    let mut i = start;
    while i < end {
        if let (Some(px), Some(is_long)) = (psar.sar[i], psar.is_long[i])
            && px.is_finite()
            && let (Some(sx), Some(sy)) = (vp.index_to_screen_x(i as i64), vp.price_to_screen_y(px))
        {
            let col = if is_long {
                palette::IND_PSAR_BULL
            } else {
                palette::IND_PSAR_BEAR
            };
            painter.circle_filled(Pos2::new(sx as f32, sy as f32), radius, col);
        }
        i = i.saturating_add(step);
    }
}

fn paint_candlestick_pattern_badges(
    painter: &egui::Painter,
    plot_rect: Rect,
    candles: &[Candle],
    cache: &IndicatorCache,
    culling_range: std::ops::Range<usize>,
    conflation_factor: usize,
    vp: &PaneViewport<'_>,
) {
    if conflation_factor > 4 || cache.candle_patterns.is_empty() {
        return;
    }
    let total = candles.len().min(cache.candle_patterns.len());
    let start = culling_range.start.min(total);
    let end = culling_range.end.min(total);
    if start >= end {
        return;
    }

    let font = FontId::monospace(8.8);
    let mut last_top_x = f32::NEG_INFINITY;
    let mut last_bot_x = f32::NEG_INFINITY;

    for (i, (candle, &summary)) in candles
        .iter()
        .zip(cache.candle_patterns.iter())
        .enumerate()
        .take(end)
        .skip(start)
    {
        let Some(kind) = summary.primary_badge else {
            continue;
        };
        let Some(sx) = vp.index_to_screen_x(i as i64) else {
            continue;
        };
        let cx = sx as f32;
        if cx < plot_rect.left() + 10.0 || cx > plot_rect.right() - 10.0 {
            continue;
        }
        let is_bull = kind.is_bullish();
        let is_bear = kind.is_bearish();
        let out_sign = if vp.price_scale.is_inverted() {
            -1.0_f32
        } else {
            1.0_f32
        };
        if is_bull {
            if cx - last_bot_x < 24.0 {
                continue;
            }
            let Some(sy_low) = vp.price_to_screen_y(candle.low) else {
                continue;
            };
            let cy = (sy_low as f32 + 11.0 * out_sign)
                .clamp(plot_rect.top() + 9.0, plot_rect.bottom() - 9.0);
            let pill = Rect::from_center_size(Pos2::new(cx, cy), Vec2::new(22.0, 13.0));
            painter.rect_filled(pill, 2.5, palette::BG_FLOATING_HUD);
            painter.rect_stroke(
                pill,
                2.5,
                Stroke::new(1.0, palette::BULLISH),
                StrokeKind::Inside,
            );
            painter.text(
                pill.center(),
                Align2::CENTER_CENTER,
                kind.short_badge(),
                font.clone(),
                palette::BULLISH,
            );
            last_bot_x = cx;
        } else {
            if cx - last_top_x < 24.0 {
                continue;
            }
            let Some(sy_high) = vp.price_to_screen_y(candle.high) else {
                continue;
            };
            let accent = if is_bear {
                palette::BEARISH
            } else {
                palette::IND_SMA
            };
            let cy = (sy_high as f32 - 11.0 * out_sign)
                .clamp(plot_rect.top() + 9.0, plot_rect.bottom() - 9.0);
            let pill = Rect::from_center_size(Pos2::new(cx, cy), Vec2::new(22.0, 13.0));
            painter.rect_filled(pill, 2.5, palette::BG_FLOATING_HUD);
            painter.rect_stroke(pill, 2.5, Stroke::new(1.0, accent), StrokeKind::Inside);
            painter.text(
                pill.center(),
                Align2::CENTER_CENTER,
                kind.short_badge(),
                font.clone(),
                accent,
            );
            last_top_x = cx;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn append_band_ribbon_into_mesh(
    mesh: &mut Mesh,
    upper: &[Option<f64>],
    upper_idx: Option<&SeriesBlockIndex>,
    lower: &[Option<f64>],
    lower_idx: Option<&SeriesBlockIndex>,
    global_offset: usize,
    hard_range: std::ops::Range<usize>,
    visible_range: std::ops::Range<usize>,
    conflation_factor: usize,
    vp: &PaneViewport<'_>,
    fill_color: Color32,
) {
    let step = conflation_factor.max(1);
    let total = upper.len().min(lower.len());
    let hard_start = hard_range.start.min(total);
    let hard_end = hard_range.end.min(total);
    let vis_start = visible_range.start.max(hard_start).min(hard_end);
    let vis_end = visible_range.end.max(vis_start).min(hard_end);
    if vis_start >= vis_end {
        return;
    }

    type RibbonPrevExit = Option<(f32, f32, f32, Option<(u32, u32)>)>;
    let mut prev_exit: RibbonPrevExit = None;
    #[allow(clippy::too_many_arguments)]
    let push_pair = |mesh: &mut Mesh,
                     prev_exit: &mut RibbonPrevExit,
                     x: f32,
                     u_in: f32,
                     u_out: f32,
                     l_in: f32,
                     l_out: f32,
                     lead_gap: bool,
                     trail_gap: bool| {
        if lead_gap {
            *prev_exit = None;
        }
        let mut next_shared: Option<(u32, u32)> = None;
        if let Some((px, pu_out, pl_out, prev_verts)) = *prev_exit {
            let (l_top_idx, l_bot_idx) = if let Some(pair) = prev_verts {
                pair
            } else {
                let tl = pu_out.min(pl_out);
                let bl = pu_out.max(pl_out);
                let base_v = mesh.vertices.len() as u32;
                mesh.colored_vertex(Pos2::new(px, tl), fill_color);
                mesh.colored_vertex(Pos2::new(px, bl), fill_color);
                (base_v, base_v + 1)
            };
            let tr = u_in.min(l_in);
            let br = u_in.max(l_in);
            let right_v = mesh.vertices.len() as u32;
            mesh.colored_vertex(Pos2::new(x, tr), fill_color);
            mesh.colored_vertex(Pos2::new(x, br), fill_color);
            mesh.add_triangle(l_top_idx, l_bot_idx, right_v);
            mesh.add_triangle(l_bot_idx, right_v + 1, right_v);
            if !trail_gap && (u_in - u_out).abs() < 0.65 && (l_in - l_out).abs() < 0.65 {
                let tr_out = u_out.min(l_out);
                let br_out = u_out.max(l_out);
                if (tr - tr_out).abs() < 0.65 && (br - br_out).abs() < 0.65 {
                    next_shared = Some((right_v, right_v + 1));
                }
            }
        }
        if trail_gap {
            *prev_exit = None;
        } else {
            *prev_exit = Some((x, u_out, l_out, next_shared));
        }
    };

    if step == 1 {
        for idx in vis_start..vis_end {
            if let (Some(uv), Some(lv)) = (
                upper[idx].filter(|v| v.is_finite()),
                lower[idx].filter(|v| v.is_finite()),
            ) && let (Some(sx), Some(uy), Some(ly)) = (
                vp.float_index_to_screen_x((global_offset + idx) as f64),
                vp.price_to_screen_y(uv),
                vp.price_to_screen_y(lv),
            ) {
                let uyf = uy as f32;
                let lyf = ly as f32;
                push_pair(
                    mesh,
                    &mut prev_exit,
                    sx as f32,
                    uyf,
                    uyf,
                    lyf,
                    lyf,
                    false,
                    false,
                );
            } else {
                prev_exit = None;
            }
        }
        return;
    }

    let g_hard_start = global_offset + hard_start;
    let g_hard_end = global_offset + hard_end;
    let aligned_global_start = ((global_offset + vis_start) / step) * step;
    let aligned_global_end = (global_offset + vis_end)
        .div_ceil(step)
        .saturating_mul(step)
        .min(g_hard_end);
    let mut g_start = aligned_global_start;

    while g_start < aligned_global_end {
        let g_end = (g_start + step).min(g_hard_end);
        let b_start = g_start.max(g_hard_start).saturating_sub(global_offset);
        let b_end = g_end.saturating_sub(global_offset).min(hard_end);
        let u_env_opt = (b_start < b_end)
            .then(|| bucket_line_envelope_indexed(upper, upper_idx, b_start, b_end))
            .flatten();
        let l_env_opt = (b_start < b_end)
            .then(|| bucket_line_envelope_indexed(lower, lower_idx, b_start, b_end))
            .flatten();

        if let (Some(u_env), Some(l_env)) = (u_env_opt, l_env_opt)
            && let (Some(sx), Some((u_in, u_out, _)), Some((l_in, l_out, _))) = (
                vp.float_index_to_screen_x(global_offset as f64 + u_env.center_logical_index),
                resolve_bucket_screen_ys(&u_env, vp),
                resolve_bucket_screen_ys(&l_env, vp),
            )
        {
            push_pair(
                mesh,
                &mut prev_exit,
                sx as f32,
                u_in,
                u_out,
                l_in,
                l_out,
                u_env.has_leading_gap || l_env.has_leading_gap,
                u_env.has_trailing_gap || l_env.has_trailing_gap,
            );
        } else {
            prev_exit = None;
        }
        g_start = g_start.saturating_add(step);
    }
}

#[inline]
fn premultiplied_tint(color: Color32, alpha: u8) -> Color32 {
    let a = u16::from(alpha);
    Color32::from_rgba_premultiplied(
        ((u16::from(color.r()) * a) / 255) as u8,
        ((u16::from(color.g()) * a) / 255) as u8,
        ((u16::from(color.b()) * a) / 255) as u8,
        alpha,
    )
}

/// Pan-invariant sub-pixel session interval merger: coalesces consecutive sub-pixel sessions
/// within the same global `env_step`-aligned horizontal bin without dropping extrema and without
/// bridging vertically disjoint session resets.
struct WorldBinIntervalAccumulator {
    env_step: usize,
    half_w: f32,
    cur_bin: Option<(usize, f32)>,
    intervals: Vec<(f32, f32)>,
}

impl WorldBinIntervalAccumulator {
    fn new(bar_spacing: f64, base_step: usize, stroke_w: f32) -> Self {
        let min_px_bars = (1.0 / bar_spacing.max(1.0e-6)).floor().max(1.0) as usize;
        let env_step = base_step.max(min_px_bars).max(1);
        let half_w = ((env_step as f64 * bar_spacing * 0.5) as f32)
            .max(stroke_w * 0.5)
            .max(0.55);
        Self {
            env_step,
            half_w,
            cur_bin: None,
            intervals: Vec::with_capacity(4),
        }
    }

    #[inline]
    fn bin_for_and_cx(&self, bin_anchor_idx: usize, vp: &PaneViewport<'_>) -> Option<(usize, f32)> {
        let bin = bin_anchor_idx / self.env_step;
        let start_idx = bin.saturating_mul(self.env_step);
        let end_idx = start_idx.saturating_add(self.env_step).saturating_sub(1);
        let center_idx = (start_idx as f64 + end_idx as f64) * 0.5;
        vp.float_index_to_screen_x(center_idx)
            .map(|sx| (bin, sx as f32))
    }

    fn push_raw(
        &mut self,
        bin: usize,
        bin_cx: f32,
        y_a: f32,
        y_b: f32,
        mesh_opt: &mut Option<Mesh>,
        color: Color32,
    ) {
        if self.cur_bin.is_some_and(|(b, _)| b != bin) {
            self.flush(mesh_opt, color);
        }
        if self.cur_bin.is_none() {
            self.cur_bin = Some((bin, bin_cx));
        }
        let lo = y_a.min(y_b);
        let hi = y_a.max(y_b);
        for (e_lo, e_hi) in &mut self.intervals {
            if lo <= *e_hi + 1.0 && hi >= *e_lo - 1.0 {
                *e_lo = (*e_lo).min(lo);
                *e_hi = (*e_hi).max(hi);
                return;
            }
        }
        self.intervals.push((lo, hi));
    }

    #[allow(clippy::too_many_arguments)]
    fn push_series_bounds(
        &mut self,
        bin_anchor_idx: usize,
        vis_range: std::ops::Range<usize>,
        series: &[Option<f64>],
        block_idx: Option<&SeriesBlockIndex>,
        vp: &PaneViewport<'_>,
        y_pad: f32,
        mesh_opt: &mut Option<Mesh>,
        color: Color32,
    ) {
        let bounds = match block_idx {
            Some(ix) => ix.range_bounds(series, vis_range.clone()),
            None => crate::indicators::visible_series_bounds(&[series], vis_range),
        };
        if let Some((min_v, max_v)) = bounds
            && min_v.is_finite()
            && max_v.is_finite()
            && let (Some((bin, cx)), Some(sy_min), Some(sy_max)) = (
                self.bin_for_and_cx(bin_anchor_idx, vp),
                vp.price_to_screen_y(min_v),
                vp.price_to_screen_y(max_v),
            )
        {
            let top_y = (sy_min as f32).min(sy_max as f32) - y_pad;
            let bot_y = (sy_min as f32).max(sy_max as f32) + y_pad;
            self.push_raw(bin, cx, top_y, bot_y, mesh_opt, color);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn push_band_bounds(
        &mut self,
        bin_anchor_idx: usize,
        vis_range: std::ops::Range<usize>,
        upper: &[Option<f64>],
        upper_idx: Option<&SeriesBlockIndex>,
        lower: &[Option<f64>],
        lower_idx: Option<&SeriesBlockIndex>,
        vp: &PaneViewport<'_>,
        mesh_opt: &mut Option<Mesh>,
        fill_color: Color32,
    ) {
        let u_bounds = match upper_idx {
            Some(ix) => ix.range_bounds(upper, vis_range.clone()),
            None => crate::indicators::visible_series_bounds(&[upper], vis_range.clone()),
        };
        let l_bounds = match lower_idx {
            Some(ix) => ix.range_bounds(lower, vis_range.clone()),
            None => crate::indicators::visible_series_bounds(&[lower], vis_range),
        };
        if let (Some((u_min, u_max)), Some((l_min, l_max))) = (u_bounds, l_bounds)
            && u_max.is_finite()
            && l_min.is_finite()
            && let (Some((bin, cx)), Some(sy_hi), Some(sy_lo)) = (
                self.bin_for_and_cx(bin_anchor_idx, vp),
                vp.price_to_screen_y(u_max.max(l_max)),
                vp.price_to_screen_y(l_min.min(u_min)),
            )
        {
            self.push_raw(bin, cx, sy_hi as f32, sy_lo as f32, mesh_opt, fill_color);
        }
    }

    fn flush(&mut self, mesh_opt: &mut Option<Mesh>, color: Color32) {
        if let Some((_, cx)) = self.cur_bin.take() {
            if self.intervals.is_empty() {
                return;
            }
            let mesh =
                mesh_opt.get_or_insert_with(|| Mesh::with_texture(egui::TextureId::default()));
            for (lo, hi) in self.intervals.drain(..) {
                mesh.add_colored_rect(
                    Rect::from_min_max(
                        Pos2::new(cx - self.half_w, lo),
                        Pos2::new(cx + self.half_w, hi),
                    ),
                    color,
                );
            }
        } else {
            self.intervals.clear();
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_vwap_output(
    painter: &egui::Painter,
    vwap: &VwapBandsOutput,
    indices: Option<(
        &SeriesBlockIndex,
        &SeriesBlockIndex,
        &SeriesBlockIndex,
        &SeriesBlockIndex,
        &SeriesBlockIndex,
    )>,
    culling_range: std::ops::Range<usize>,
    conflation_factor: usize,
    vp: &PaneViewport<'_>,
    show_band_1: bool,
    show_band_2: bool,
    line_color: Color32,
) {
    let total = vwap.vwap.len();
    let start = culling_range.start.min(total);
    let end = culling_range.end.min(total);
    if start >= end {
        return;
    }

    let band1_stroke = Stroke::new(1.0, line_color.gamma_multiply(0.60));
    let band2_stroke = Stroke::new(1.0, line_color.gamma_multiply(0.38));
    let band1_fill = if line_color == palette::IND_VWAP_LINE {
        palette::IND_VWAP_FILL_1
    } else {
        premultiplied_tint(line_color, 26)
    };
    let band2_fill = if line_color == palette::IND_VWAP_LINE {
        palette::IND_VWAP_FILL_2
    } else {
        premultiplied_tint(line_color, 16)
    };
    let main_stroke = Stroke::new(1.7, line_color);

    let starts = &vwap.session_starts;
    let first_pos = starts.partition_point(|&s| s <= start).saturating_sub(1);
    let last_pos = starts.partition_point(|&s| s < end);
    let visible_sessions = if starts.is_empty() {
        1
    } else {
        last_pos.saturating_sub(first_pos).max(1)
    };
    let step = conflation_factor.max(1);
    let use_line_mesh = step >= 8 || visible_sessions > 128;

    let (idx_v, idx_u1, idx_l1, idx_u2, idx_l2) = match indices {
        Some((v, u1, l1, u2, l2)) => (Some(v), Some(u1), Some(l1), Some(u2), Some(l2)),
        None => (None, None, None, None, None),
    };

    let mut band2_mesh = show_band_2.then(Mesh::default);
    let mut band1_mesh = show_band_1.then(Mesh::default);
    let mut band_border_mesh = ((show_band_2 || show_band_1) && step < 8 && visible_sessions > 128)
        .then(|| Mesh::with_texture(egui::TextureId::default()));
    let mut main_line_mesh = use_line_mesh.then(|| Mesh::with_texture(egui::TextureId::default()));
    let mut pts_scratch = Vec::with_capacity(256);

    // Always segment strictly per session boundary across EVERY conflation zoom level (`1x`, `4x`, `16x`, ...),
    // accumulating ribbons and dense strokes into shared meshes while never bridging across resets.
    let bar_spacing = vp.time_scale.bar_spacing();
    let mut render_session_slice =
        |hard_range: std::ops::Range<usize>,
         vis_range: std::ops::Range<usize>,
         skip_subpixel_bands: bool,
         main_line_mesh: &mut Option<Mesh>,
         band1_mesh: &mut Option<Mesh>,
         band2_mesh: &mut Option<Mesh>| {
            if show_band_2 && !skip_subpixel_bands {
                if let Some(mesh) = band2_mesh.as_mut() {
                    append_band_ribbon_into_mesh(
                        mesh,
                        &vwap.upper_2,
                        idx_u2,
                        &vwap.lower_2,
                        idx_l2,
                        0,
                        hard_range.clone(),
                        vis_range.clone(),
                        step,
                        vp,
                        band2_fill,
                    );
                }
                if step < 8 {
                    append_conflated_line_segment(
                        painter,
                        &mut band_border_mesh,
                        &mut pts_scratch,
                        &vwap.upper_2,
                        idx_u2,
                        0,
                        hard_range.clone(),
                        vis_range.clone(),
                        step,
                        vp,
                        band2_stroke,
                        1.0,
                    );
                    append_conflated_line_segment(
                        painter,
                        &mut band_border_mesh,
                        &mut pts_scratch,
                        &vwap.lower_2,
                        idx_l2,
                        0,
                        hard_range.clone(),
                        vis_range.clone(),
                        step,
                        vp,
                        band2_stroke,
                        1.0,
                    );
                }
            }
            if show_band_1 && !skip_subpixel_bands {
                if let Some(mesh) = band1_mesh.as_mut() {
                    append_band_ribbon_into_mesh(
                        mesh,
                        &vwap.upper_1,
                        idx_u1,
                        &vwap.lower_1,
                        idx_l1,
                        0,
                        hard_range.clone(),
                        vis_range.clone(),
                        step,
                        vp,
                        band1_fill,
                    );
                }
                if step < 8 {
                    append_conflated_line_segment(
                        painter,
                        &mut band_border_mesh,
                        &mut pts_scratch,
                        &vwap.upper_1,
                        idx_u1,
                        0,
                        hard_range.clone(),
                        vis_range.clone(),
                        step,
                        vp,
                        band1_stroke,
                        1.0,
                    );
                    append_conflated_line_segment(
                        painter,
                        &mut band_border_mesh,
                        &mut pts_scratch,
                        &vwap.lower_1,
                        idx_l1,
                        0,
                        hard_range.clone(),
                        vis_range.clone(),
                        step,
                        vp,
                        band1_stroke,
                        1.0,
                    );
                }
            }
            append_conflated_line_segment(
                painter,
                main_line_mesh,
                &mut pts_scratch,
                &vwap.vwap,
                idx_v,
                0,
                hard_range,
                vis_range,
                step,
                vp,
                main_stroke,
                1.0,
            );
        };

    if starts.is_empty() {
        render_session_slice(
            0..total,
            start..end,
            false,
            &mut main_line_mesh,
            &mut band1_mesh,
            &mut band2_mesh,
        );
    } else {
        let mut main_acc = WorldBinIntervalAccumulator::new(bar_spacing, step, main_stroke.width);
        let mut b1_acc =
            show_band_1.then(|| WorldBinIntervalAccumulator::new(bar_spacing, step, 1.0));
        let mut b2_acc =
            show_band_2.then(|| WorldBinIntervalAccumulator::new(bar_spacing, step, 1.0));
        let flush_accs = |main_acc: &mut WorldBinIntervalAccumulator,
                          b1_acc: &mut Option<WorldBinIntervalAccumulator>,
                          b2_acc: &mut Option<WorldBinIntervalAccumulator>,
                          main_mesh: &mut Option<Mesh>,
                          b1_mesh: &mut Option<Mesh>,
                          b2_mesh: &mut Option<Mesh>| {
            main_acc.flush(main_mesh, main_stroke.color);
            if let Some(acc) = b1_acc.as_mut() {
                acc.flush(b1_mesh, band1_fill);
            }
            if let Some(acc) = b2_acc.as_mut() {
                acc.flush(b2_mesh, band2_fill);
            }
        };

        for si in first_pos..last_pos {
            let hard_start = starts[si].min(total);
            let hard_end = starts.get(si + 1).copied().unwrap_or(total).min(total);
            let vis_start = hard_start.max(start);
            let vis_end = hard_end.min(end);
            if vis_start >= vis_end {
                continue;
            }
            let hard_px = ((hard_end - hard_start) as f64) * bar_spacing;
            let is_subpixel_session = hard_px < 1.25
                || ((vis_end - vis_start) <= step && (hard_end - hard_start) <= step);
            if is_subpixel_session {
                main_acc.push_series_bounds(
                    hard_start,
                    vis_start..vis_end,
                    &vwap.vwap,
                    idx_v,
                    vp,
                    0.75,
                    &mut main_line_mesh,
                    main_stroke.color,
                );
                if let Some(acc) = b1_acc.as_mut() {
                    acc.push_band_bounds(
                        hard_start,
                        vis_start..vis_end,
                        &vwap.upper_1,
                        idx_u1,
                        &vwap.lower_1,
                        idx_l1,
                        vp,
                        &mut band1_mesh,
                        band1_fill,
                    );
                }
                if let Some(acc) = b2_acc.as_mut() {
                    acc.push_band_bounds(
                        hard_start,
                        vis_start..vis_end,
                        &vwap.upper_2,
                        idx_u2,
                        &vwap.lower_2,
                        idx_l2,
                        vp,
                        &mut band2_mesh,
                        band2_fill,
                    );
                }
            } else {
                flush_accs(
                    &mut main_acc,
                    &mut b1_acc,
                    &mut b2_acc,
                    &mut main_line_mesh,
                    &mut band1_mesh,
                    &mut band2_mesh,
                );
                render_session_slice(
                    hard_start..hard_end,
                    vis_start..vis_end,
                    false,
                    &mut main_line_mesh,
                    &mut band1_mesh,
                    &mut band2_mesh,
                );
            }
        }
        flush_accs(
            &mut main_acc,
            &mut b1_acc,
            &mut b2_acc,
            &mut main_line_mesh,
            &mut band1_mesh,
            &mut band2_mesh,
        );
    }

    if let Some(mesh) = band2_mesh
        && !mesh.indices.is_empty()
    {
        painter.add(Shape::mesh(mesh));
    }
    if let Some(mesh) = band1_mesh
        && !mesh.indices.is_empty()
    {
        painter.add(Shape::mesh(mesh));
    }
    if let Some(mesh) = band_border_mesh
        && !mesh.is_empty()
    {
        painter.add(Shape::mesh(mesh));
    }
    if let Some(mesh) = main_line_mesh
        && !mesh.is_empty()
    {
        painter.add(Shape::mesh(mesh));
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_anchored_vwap_tail(
    painter: &egui::Painter,
    vwap: &VwapBandsOutput,
    global_offset: usize,
    local_range: std::ops::Range<usize>,
    conflation_factor: usize,
    vp: &PaneViewport<'_>,
    show_bands: bool,
    line_color: Color32,
) {
    let total = vwap.vwap.len();
    let s = local_range.start.min(total);
    let e = local_range.end.min(total);
    if s >= e {
        return;
    }

    let step = conflation_factor.max(1);
    let band_fill = premultiplied_tint(line_color, 24);
    let band_stroke = Stroke::new(1.0, line_color.gamma_multiply(0.55));
    let main_stroke = Stroke::new(1.7, line_color);

    let mut pts_scratch = Vec::with_capacity(256);
    if show_bands {
        let mut band_mesh = Mesh::default();
        append_band_ribbon_into_mesh(
            &mut band_mesh,
            &vwap.upper_1,
            None,
            &vwap.lower_1,
            None,
            global_offset,
            0..total,
            s..e,
            step,
            vp,
            band_fill,
        );
        if !band_mesh.indices.is_empty() {
            painter.add(Shape::mesh(band_mesh));
        }
        if step < 8 {
            let mut border_mesh = None;
            append_conflated_line_segment(
                painter,
                &mut border_mesh,
                &mut pts_scratch,
                &vwap.upper_1,
                None,
                global_offset,
                0..total,
                s..e,
                step,
                vp,
                band_stroke,
                1.0,
            );
            append_conflated_line_segment(
                painter,
                &mut border_mesh,
                &mut pts_scratch,
                &vwap.lower_1,
                None,
                global_offset,
                0..total,
                s..e,
                step,
                vp,
                band_stroke,
                1.0,
            );
        }
    }

    let mut line_mesh = (step >= 8).then(|| Mesh::with_texture(egui::TextureId::default()));
    append_conflated_line_segment(
        painter,
        &mut line_mesh,
        &mut pts_scratch,
        &vwap.vwap,
        None,
        global_offset,
        0..total,
        s..e,
        step,
        vp,
        main_stroke,
        1.0,
    );
    if let Some(mesh) = line_mesh
        && !mesh.is_empty()
    {
        painter.add(Shape::mesh(mesh));
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_session_market_profiles(
    painter: &egui::Painter,
    plot_rect: Rect,
    time_scale: &TimeScale,
    indicators: &IndicatorConfig,
    cache: &IndicatorCache,
    culling_range: std::ops::Range<usize>,
    vp: &PaneViewport<'_>,
) {
    let profiles = &cache.market_profile.profiles;
    let historical_naked = &cache.market_profile.historical_naked_pocs;
    if profiles.is_empty() && historical_naked.is_empty() {
        return;
    }

    let mut mesh = Mesh::with_texture(egui::TextureId::default());
    mesh.vertices.reserve(profiles.len() * 64);
    mesh.indices.reserve(profiles.len() * 96);

    let mut poc_segments: Vec<[Pos2; 2]> = Vec::with_capacity(profiles.len());
    let mut va_segments: Vec<[Pos2; 2]> = Vec::with_capacity(profiles.len() * 2);
    let mut ib_segments: Vec<[Pos2; 2]> = Vec::with_capacity(profiles.len());
    let mut naked_segments: Vec<[Pos2; 2]> =
        Vec::with_capacity(profiles.len() + historical_naked.len());

    let half_bar_w = (time_scale.bar_spacing() * 0.42).max(1.5) as f32;
    let y_min_clamp = plot_rect.top() - 4.0;
    let y_max_clamp = plot_rect.bottom() + 4.0;

    for hnp in historical_naked {
        if !indicators.mp_extend_naked_poc
            || hnp.naked_poc_end_index < culling_range.start
            || hnp.session_end_index >= culling_range.end
            || hnp.session_end_index >= hnp.naked_poc_end_index
        {
            continue;
        }
        if let (Some(xr0), Some(xr1), Some(y_poc)) = (
            vp.index_to_screen_x(hnp.session_end_index as i64),
            vp.index_to_screen_x(hnp.naked_poc_end_index as i64),
            vp.price_to_screen_y(hnp.poc_price),
        ) {
            let yp = (y_poc as f32).clamp(y_min_clamp, y_max_clamp);
            let x_start = (xr0 as f32) + half_bar_w;
            let x_end = (xr1 as f32) + half_bar_w;
            if x_end > x_start {
                naked_segments.push([Pos2::new(x_start, yp), Pos2::new(x_end, yp)]);
            }
        }
    }

    for s in profiles {
        if s.naked_poc_end_index < culling_range.start || s.start_index >= culling_range.end {
            continue;
        }
        let body_visible = s.end_index >= culling_range.start && s.start_index < culling_range.end;
        if body_visible
            && let (Some(x0_raw), Some(x1_raw)) = (
                vp.index_to_screen_x(s.start_index as i64),
                vp.index_to_screen_x(s.end_index as i64),
            )
        {
            let x0 = (x0_raw as f32) - half_bar_w;
            let x1 = ((x1_raw as f32) + half_bar_w).max(x0 + 2.0);
            let session_w = x1 - x0;

            if session_w >= 3.0
                && let (Some(yv_hi), Some(yv_lo)) = (
                    vp.price_to_screen_y(s.vah_price),
                    vp.price_to_screen_y(s.val_price),
                )
            {
                let va_top = (yv_hi.min(yv_lo) as f32).clamp(y_min_clamp, y_max_clamp);
                let va_bot = (yv_hi.max(yv_lo) as f32).clamp(y_min_clamp, y_max_clamp);
                if va_bot > va_top + 0.5 && session_w >= 4.0 {
                    mesh.add_colored_rect(
                        Rect::from_min_max(Pos2::new(x0, va_top), Pos2::new(x1, va_bot)),
                        palette::MP_VA_FILL,
                    );
                }
                va_segments.push([Pos2::new(x0, va_top), Pos2::new(x1, va_top)]);
                va_segments.push([Pos2::new(x0, va_bot), Pos2::new(x1, va_bot)]);
            }

            if session_w >= 14.0 && s.max_row_tpo > 0 && s.bin_height > 0.0 {
                let max_tpo_w = (session_w * 0.55).max(2.0);
                for row in &s.rows {
                    if row.tpo_count == 0 {
                        continue;
                    }
                    let (Some(yr_lo), Some(yr_hi)) = (
                        vp.price_to_screen_y(row.price_low),
                        vp.price_to_screen_y(row.price_high),
                    ) else {
                        continue;
                    };
                    let top = (yr_lo.min(yr_hi) as f32).clamp(y_min_clamp, y_max_clamp);
                    let bot = (yr_lo.max(yr_hi) as f32).clamp(y_min_clamp, y_max_clamp);
                    let h = (bot - top).max(1.2);
                    let gap = if h >= 3.0 { 0.4_f32 } else { 0.0 };
                    let y0 = top + gap * 0.5;
                    let y1 = (top + h - gap * 0.5).max(y0 + 0.8);
                    let bar_w = ((row.tpo_count as f32 / s.max_row_tpo as f32) * max_tpo_w)
                        .clamp(1.5, max_tpo_w);
                    let col = if row.is_single_print {
                        palette::MP_SINGLE_PRINT
                    } else if row.in_value_area {
                        palette::MP_TPO_VA
                    } else {
                        palette::MP_TPO_OUT
                    };
                    mesh.add_colored_rect(
                        Rect::from_min_max(Pos2::new(x0, y0), Pos2::new(x0 + bar_w, y1)),
                        col,
                    );
                }
            }

            if session_w >= 3.0
                && let (Some(y_ib_hi), Some(y_ib_lo)) = (
                    vp.price_to_screen_y(s.ib_high),
                    vp.price_to_screen_y(s.ib_low),
                )
            {
                let x_ib = x0 - 2.0;
                if x_ib >= plot_rect.left() {
                    let ib_top = (y_ib_hi.min(y_ib_lo) as f32).clamp(y_min_clamp, y_max_clamp);
                    let ib_bot = (y_ib_hi.max(y_ib_lo) as f32).clamp(y_min_clamp, y_max_clamp);
                    ib_segments.push([Pos2::new(x_ib, ib_top), Pos2::new(x_ib, ib_bot)]);
                }
            }

            if let Some(y_poc) = vp.price_to_screen_y(s.poc_price) {
                let yp = (y_poc as f32).clamp(y_min_clamp, y_max_clamp);
                poc_segments.push([Pos2::new(x0, yp), Pos2::new(x1, yp)]);
            }
        }

        if indicators.mp_extend_naked_poc
            && s.end_index < s.naked_poc_end_index
            && let (Some(xr0), Some(xr1), Some(y_poc)) = (
                vp.index_to_screen_x(s.end_index as i64),
                vp.index_to_screen_x(s.naked_poc_end_index as i64),
                vp.price_to_screen_y(s.poc_price),
            )
        {
            let yp = (y_poc as f32).clamp(y_min_clamp, y_max_clamp);
            let x_start = (xr0 as f32) + half_bar_w;
            let x_end = (xr1 as f32) + half_bar_w;
            if x_end > x_start {
                naked_segments.push([Pos2::new(x_start, yp), Pos2::new(x_end, yp)]);
            }
        }
    }

    if !mesh.is_empty() {
        painter.add(Shape::mesh(mesh));
    }
    let va_stroke = Stroke::new(1.0, palette::MP_VA_BORDER);
    for seg in va_segments {
        painter.line_segment(seg, va_stroke);
    }
    let ib_stroke = Stroke::new(2.0, palette::MP_IB_BRACKET);
    for seg in ib_segments {
        painter.line_segment(seg, ib_stroke);
    }
    let poc_stroke = Stroke::new(1.5, palette::MP_POC_LINE);
    for seg in poc_segments {
        painter.line_segment(seg, poc_stroke);
    }
    let naked_stroke = Stroke::new(1.15, palette::MP_NAKED_POC_LINE);
    for seg in naked_segments {
        painter.add(Shape::dashed_line(&seg, naked_stroke, 5.0, 4.0));
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_volume_profile_bars(
    painter: &egui::Painter,
    vp: &PaneViewport<'_>,
    profile: &VolumeProfileResult,
    x_anchor: f32,
    max_bar_width: f32,
    align_right: bool,
    line_span_x: (f32, f32),
    poc_color: Color32,
    va_line_color: Color32,
    is_selected: bool,
    show_poc_tag: bool,
    poc_label: Option<&str>,
) {
    if profile.max_bin_volume <= 0.0 || max_bar_width <= 1.0 {
        return;
    }

    let mut mesh = Mesh::with_texture(egui::TextureId::default());
    mesh.vertices.reserve(profile.bins.len() * 8);
    mesh.indices.reserve(profile.bins.len() * 12);

    let buy_va = if is_selected {
        Color32::from_rgba_premultiplied(18, 78, 72, 122)
    } else {
        palette::VRVP_BUY_VA
    };
    let sell_va = if is_selected {
        Color32::from_rgba_premultiplied(114, 39, 38, 122)
    } else {
        palette::VRVP_SELL_VA
    };
    let buy_out = if is_selected {
        Color32::from_rgba_premultiplied(9, 40, 37, 64)
    } else {
        palette::VRVP_BUY_OUT
    };
    let sell_out = if is_selected {
        Color32::from_rgba_premultiplied(57, 20, 19, 64)
    } else {
        palette::VRVP_SELL_OUT
    };

    for bin in &profile.bins {
        if bin.total_volume <= 0.0 {
            continue;
        }
        let (Some(y_lo), Some(y_hi)) = (
            vp.price_to_screen_y(bin.price_low),
            vp.price_to_screen_y(bin.price_high),
        ) else {
            continue;
        };
        let raw_top = (y_hi.min(y_lo)) as f32;
        let raw_bot = (y_hi.max(y_lo)) as f32;
        if !raw_top.is_finite() || !raw_bot.is_finite() {
            continue;
        }
        let h = (raw_bot - raw_top).max(1.2);
        let gap = if h >= 3.5 { 0.45_f32 } else { 0.0 };
        let y0 = raw_top + gap * 0.5;
        let y1 = (raw_top + h - gap * 0.5).max(y0 + 0.8);

        let total_w = ((bin.total_volume / profile.max_bin_volume) as f32 * max_bar_width)
            .clamp(1.2, max_bar_width);
        let buy_frac = (bin.buy_volume / bin.total_volume).clamp(0.0, 1.0) as f32;
        let buy_w = total_w * buy_frac;
        let sell_w = (total_w - buy_w).max(0.0);

        let (buy_col, sell_col) = if bin.in_value_area {
            (buy_va, sell_va)
        } else {
            (buy_out, sell_out)
        };

        if align_right {
            let x_right = x_anchor;
            let x_mid = x_right - sell_w;
            let x_left = x_right - total_w;
            if buy_w > 0.25 {
                mesh.add_colored_rect(
                    Rect::from_min_max(Pos2::new(x_left, y0), Pos2::new(x_mid, y1)),
                    buy_col,
                );
            }
            if sell_w > 0.25 {
                mesh.add_colored_rect(
                    Rect::from_min_max(Pos2::new(x_mid, y0), Pos2::new(x_right, y1)),
                    sell_col,
                );
            }
        } else {
            let x_left = x_anchor;
            let x_mid = x_left + buy_w;
            let x_right = x_left + total_w;
            if buy_w > 0.25 {
                mesh.add_colored_rect(
                    Rect::from_min_max(Pos2::new(x_left, y0), Pos2::new(x_mid, y1)),
                    buy_col,
                );
            }
            if sell_w > 0.25 {
                mesh.add_colored_rect(
                    Rect::from_min_max(Pos2::new(x_mid, y0), Pos2::new(x_right, y1)),
                    sell_col,
                );
            }
        }
    }

    if !mesh.is_empty() {
        painter.add(Shape::mesh(mesh));
    }

    let x0 = line_span_x.0;
    let x1 = line_span_x.1;
    if x1 - x0 > 2.0 {
        for level_price in [profile.vah_price, profile.val_price] {
            if let Some(sy) = vp.price_to_screen_y(level_price) {
                let y = sy as f32;
                let p0 = Pos2::new(x0, y);
                let p1 = Pos2::new(x1, y);
                paint_styled_segment_phased(
                    painter,
                    p0,
                    p1,
                    p0,
                    Stroke::new(1.05, va_line_color.gamma_multiply(0.82)),
                    LineDashStyle::Dashed,
                );
            }
        }
        if let Some(py) = vp.price_to_screen_y(profile.poc_price) {
            let y = py as f32;
            painter.line_segment(
                [Pos2::new(x0, y), Pos2::new(x1, y)],
                Stroke::new(2.0, poc_color),
            );
            if show_poc_tag
                && let Some(lbl) = poc_label
                && !lbl.is_empty()
            {
                let galley =
                    painter.layout_no_wrap(lbl.to_owned(), FontId::monospace(10.0), poc_color);
                let pill_pos = Pos2::new(x0 + 5.0, y - galley.size().y - 4.0);
                let bg_rect =
                    Rect::from_min_size(pill_pos, galley.size()).expand2(Vec2::new(4.0, 2.0));
                painter.rect_filled(
                    bg_rect,
                    3.0,
                    Color32::from_rgba_unmultiplied(13, 17, 26, 215),
                );
                painter.galley(pill_pos, galley, Color32::WHITE);
            }
        }
    }
}

#[inline]
fn timeframe_str_to_seconds(tf_label: &str) -> i64 {
    Timeframe::ALL
        .iter()
        .find(|t| t.label().eq_ignore_ascii_case(tf_label.trim()))
        .map_or(86_400, |t| t.seconds())
}

#[inline]
fn format_compact_signed(v: f64) -> String {
    let av = v.abs();
    if av >= 1_000_000_000.0 {
        format!("{:+.2}B", v / 1_000_000_000.0)
    } else if av >= 1_000_000.0 {
        format!("{:+.2}M", v / 1_000_000.0)
    } else if av >= 10_000.0 {
        format!("{:+.1}K", v / 1_000.0)
    } else {
        format!("{v:+.0}")
    }
}

#[inline]
fn ichimoku_visible_index_range(
    time_scale: &TimeScale,
    series_len: usize,
    conflation_factor: usize,
) -> std::ops::Range<usize> {
    if series_len == 0 {
        return 0..0;
    }
    let pad = (conflation_factor.max(1) as isize) + 1;
    let Some(lr) = time_scale.visible_logical_range() else {
        return 0..series_len;
    };
    let start = ((lr.left.floor() as isize) - pad).clamp(0, series_len as isize) as usize;
    let end =
        ((lr.right.ceil() as isize) + pad + 1).clamp(start as isize, series_len as isize) as usize;
    start..end
}

/// Computes `0.5 * (a + b)` without overflowing `f64::MAX` on large finite operands
/// while keeping small/subnormal magnitudes monotonic and exact.
#[inline]
pub(crate) fn safe_midpoint_f64(a: f64, b: f64) -> f64 {
    if !a.is_finite() || !b.is_finite() {
        return 0.5 * (a + b);
    }
    if a.abs() > (f64::MAX * 0.25) || b.abs() > (f64::MAX * 0.25) {
        return (a * 0.5) + (b * 0.5);
    }
    0.5 * (a + b)
}

/// Emits non-crossing Kumo fill triangles into `mesh` for a single step `(x0..x1)` between
/// `Senkou A` (`ya`) and `Senkou B` (`yb`), splitting linearly at exact Kumo Twist crossovers
/// so Bullish (`A >= B`) and Bearish (`A < B`) clouds never emit bowtie quads.
#[allow(clippy::too_many_arguments)]
pub(crate) fn emit_ichimoku_kumo_segment_triangles(
    mesh: &mut Mesh,
    x0: f32,
    ya0: f32,
    yb0: f32,
    a0: f64,
    b0: f64,
    x1: f32,
    ya1: f32,
    yb1: f32,
    a1: f64,
    b1: f64,
    y_clamp: (f32, f32),
) {
    let Ok(base_idx) = u32::try_from(mesh.vertices.len()) else {
        return;
    };
    if base_idx > u32::MAX - 8 {
        return;
    }

    let clamp_pos = |x: f32, y: f32| Pos2::new(x, y.clamp(y_clamp.0, y_clamp.1));

    let bull0 = a0 >= b0;
    let bull1 = a1 >= b1;

    let push_tri = |mesh: &mut Mesh, p0: Pos2, p1: Pos2, p2: Pos2, col: Color32| {
        let Ok(idx) = u32::try_from(mesh.vertices.len()) else {
            return;
        };
        if idx > u32::MAX - 3 {
            return;
        }
        mesh.colored_vertex(p0, col);
        mesh.colored_vertex(p1, col);
        mesh.colored_vertex(p2, col);
        mesh.add_triangle(idx, idx + 1, idx + 2);
    };

    let col_for = |bull: bool| {
        if bull {
            palette::IND_KUMO_BULL_FILL
        } else {
            palette::IND_KUMO_BEAR_FILL
        }
    };

    if bull0 != bull1 {
        let dy0 = f64::from(ya0) - f64::from(yb0);
        let dy1 = f64::from(ya1) - f64::from(yb1);
        let denom_s = dy0 - dy1;
        let t_f64 = if denom_s.abs() > 1.0e-9
            && (dy0.signum() != dy1.signum() || dy0 == 0.0 || dy1 == 0.0)
        {
            (dy0 / denom_s).clamp(0.0, 1.0)
        } else {
            let dp0 = a0 - b0;
            let dp1 = a1 - b1;
            let denom_p = dp0 - dp1;
            if denom_p.abs() > 1.0e-15 {
                (dp0 / denom_p).clamp(0.0, 1.0)
            } else {
                0.5
            }
        };
        let t = t_f64 as f32;
        let xm = x0 + t * (x1 - x0);
        let ym = ya0 + t * (ya1 - ya0);
        let p_mid = clamp_pos(xm, ym);
        push_tri(
            mesh,
            clamp_pos(x0, ya0),
            clamp_pos(x0, yb0),
            p_mid,
            col_for(bull0),
        );
        push_tri(
            mesh,
            p_mid,
            clamp_pos(x1, ya1),
            clamp_pos(x1, yb1),
            col_for(bull1),
        );
    } else {
        let col = col_for(safe_midpoint_f64(a0, a1) >= safe_midpoint_f64(b0, b1));
        let p_a0 = clamp_pos(x0, ya0);
        let p_b0 = clamp_pos(x0, yb0);
        let p_a1 = clamp_pos(x1, ya1);
        let p_b1 = clamp_pos(x1, yb1);
        push_tri(mesh, p_a0, p_b0, p_b1, col);
        push_tri(mesh, p_a0, p_b1, p_a1, col);
    }
}

fn paint_ichimoku_kumo_fill(
    painter: &egui::Painter,
    plot_rect: Rect,
    time_scale: &TimeScale,
    cache: &IndicatorCache,
    conflation_factor: usize,
    vp: &PaneViewport<'_>,
) {
    let senkou_a = &cache.ichimoku.senkou_a;
    let senkou_b = &cache.ichimoku.senkou_b;
    let total = senkou_a.len().min(senkou_b.len());
    if total == 0 {
        return;
    }

    let step = conflation_factor.max(1);
    let vis_range = ichimoku_visible_index_range(time_scale, total, step);
    if vis_range.start >= vis_range.end {
        return;
    }

    let y_clamp = (plot_rect.top() - 4096.0, plot_rect.bottom() + 4096.0);
    let mut mesh = Mesh::default();
    let est_steps = ((vis_range.end - vis_range.start) / step + 2).min(2048);
    mesh.vertices.reserve(est_steps * 4);
    mesh.indices.reserve(est_steps * 6);

    let mut prev: Option<(f32, f32, f32, f64, f64)> = None;

    if step == 1 {
        for idx in vis_range {
            if let (Some(a), Some(b)) = (senkou_a[idx], senkou_b[idx])
                && a.is_finite()
                && b.is_finite()
                && let (Some(sx), Some(sya), Some(syb)) = (
                    vp.float_index_to_screen_x(idx as f64),
                    vp.price_to_screen_y(a),
                    vp.price_to_screen_y(b),
                )
            {
                let x = sx as f32;
                let ya = sya as f32;
                let yb = syb as f32;
                if let Some((x0, ya0, yb0, a0, b0)) = prev {
                    emit_ichimoku_kumo_segment_triangles(
                        &mut mesh, x0, ya0, yb0, a0, b0, x, ya, yb, a, b, y_clamp,
                    );
                }
                prev = Some((x, ya, yb, a, b));
            } else {
                prev = None;
            }
        }
    } else {
        let mut b_start = (vis_range.start / step) * step;
        let b_limit = vis_range.end.min(total);
        while b_start < b_limit {
            let b_end = (b_start + step).min(b_limit);
            if let (Some(ea), Some(eb)) = (
                bucket_line_envelope_indexed(
                    senkou_a,
                    Some(&cache.ichimoku_senkou_a_idx),
                    b_start,
                    b_end,
                ),
                bucket_line_envelope_indexed(
                    senkou_b,
                    Some(&cache.ichimoku_senkou_b_idx),
                    b_start,
                    b_end,
                ),
            ) {
                let has_lead_gap = ea.has_leading_gap || eb.has_leading_gap;
                let has_int_gap = ea.has_internal_gap || eb.has_internal_gap;
                let has_trail_gap = ea.has_trailing_gap || eb.has_trailing_gap;
                if has_lead_gap {
                    prev = None;
                }
                let center = safe_midpoint_f64(ea.center_logical_index, eb.center_logical_index);
                if has_int_gap {
                    let entry_a = ea.entry_val;
                    let entry_b = eb.entry_val;
                    if let Some((x0, ya0, yb0, a0, b0)) = prev
                        && entry_a.is_finite()
                        && entry_b.is_finite()
                        && let (Some(sx), Some(sya), Some(syb)) = (
                            vp.float_index_to_screen_x(center),
                            vp.price_to_screen_y(entry_a),
                            vp.price_to_screen_y(entry_b),
                        )
                    {
                        emit_ichimoku_kumo_segment_triangles(
                            &mut mesh, x0, ya0, yb0, a0, b0, sx as f32, sya as f32, syb as f32,
                            entry_a, entry_b, y_clamp,
                        );
                    }
                    prev = None;
                    let exit_a = ea.exit_val;
                    let exit_b = eb.exit_val;
                    if !has_trail_gap
                        && exit_a.is_finite()
                        && exit_b.is_finite()
                        && let (Some(sx), Some(sya), Some(syb)) = (
                            vp.float_index_to_screen_x(center),
                            vp.price_to_screen_y(exit_a),
                            vp.price_to_screen_y(exit_b),
                        )
                    {
                        prev = Some((sx as f32, sya as f32, syb as f32, exit_a, exit_b));
                    }
                } else {
                    let a = safe_midpoint_f64(ea.entry_val, ea.exit_val);
                    let b = safe_midpoint_f64(eb.entry_val, eb.exit_val);
                    if a.is_finite()
                        && b.is_finite()
                        && let (Some(sx), Some(sya), Some(syb)) = (
                            vp.float_index_to_screen_x(center),
                            vp.price_to_screen_y(a),
                            vp.price_to_screen_y(b),
                        )
                    {
                        let x = sx as f32;
                        let ya = sya as f32;
                        let yb = syb as f32;
                        if let Some((x0, ya0, yb0, a0, b0)) = prev {
                            emit_ichimoku_kumo_segment_triangles(
                                &mut mesh, x0, ya0, yb0, a0, b0, x, ya, yb, a, b, y_clamp,
                            );
                        }
                        prev = (!has_trail_gap).then_some((x, ya, yb, a, b));
                    } else {
                        prev = None;
                    }
                }
            } else {
                prev = None;
            }
            b_start = b_start.saturating_add(step);
        }
    }

    if !mesh.indices.is_empty() {
        painter.add(Shape::mesh(mesh));
    }
}

fn paint_ichimoku_lines(
    painter: &egui::Painter,
    time_scale: &TimeScale,
    indicators: &IndicatorConfig,
    cache: &IndicatorCache,
    culling_range: std::ops::Range<usize>,
    conflation_factor: usize,
    vp: &PaneViewport<'_>,
) {
    let senkou_len = cache
        .ichimoku
        .senkou_a
        .len()
        .min(cache.ichimoku.senkou_b.len());
    let senkou_vis = ichimoku_visible_index_range(time_scale, senkou_len, conflation_factor);
    if senkou_vis.start < senkou_vis.end {
        let use_mesh = conflation_factor.max(1) >= 8;
        let mut sa_mesh = use_mesh.then(|| Mesh::with_texture(egui::TextureId::default()));
        let mut sb_mesh = use_mesh.then(|| Mesh::with_texture(egui::TextureId::default()));
        let mut pts_scratch = Vec::with_capacity(256);
        append_conflated_line_segment(
            painter,
            &mut sa_mesh,
            &mut pts_scratch,
            &cache.ichimoku.senkou_a,
            Some(&cache.ichimoku_senkou_a_idx),
            0,
            0..senkou_len,
            senkou_vis.clone(),
            conflation_factor,
            vp,
            Stroke::new(1.25, palette::IND_ICHIMOKU_SENKOU_A),
            1.0,
        );
        append_conflated_line_segment(
            painter,
            &mut sb_mesh,
            &mut pts_scratch,
            &cache.ichimoku.senkou_b,
            Some(&cache.ichimoku_senkou_b_idx),
            0,
            0..senkou_len,
            senkou_vis,
            conflation_factor,
            vp,
            Stroke::new(1.25, palette::IND_ICHIMOKU_SENKOU_B),
            1.0,
        );
        if let Some(m) = sa_mesh
            && !m.is_empty()
        {
            painter.add(Shape::mesh(m));
        }
        if let Some(m) = sb_mesh
            && !m.is_empty()
        {
            painter.add(Shape::mesh(m));
        }
    }

    paint_conflated_line_series(
        painter,
        &cache.ichimoku.tenkan,
        Some(&cache.ichimoku_tenkan_idx),
        culling_range.clone(),
        conflation_factor,
        vp,
        Stroke::new(1.45, palette::IND_ICHIMOKU_TENKAN),
    );
    paint_conflated_line_series(
        painter,
        &cache.ichimoku.kijun,
        Some(&cache.ichimoku_kijun_idx),
        culling_range.clone(),
        conflation_factor,
        vp,
        Stroke::new(1.45, palette::IND_ICHIMOKU_KIJUN),
    );
    if indicators.show_ichimoku_chikou {
        paint_conflated_line_series(
            painter,
            &cache.ichimoku.chikou,
            Some(&cache.ichimoku_chikou_idx),
            culling_range,
            conflation_factor,
            vp,
            Stroke::new(1.25, palette::IND_ICHIMOKU_CHIKOU),
        );
    }
}

fn paint_cvd_pane(
    painter: &egui::Painter,
    plot_rect: Rect,
    cache: &IndicatorCache,
    culling_range: std::ops::Range<usize>,
    conflation_factor: usize,
    vp: &PaneViewport<'_>,
    bar_metrics: CandlestickBarMetrics,
) {
    let cvd_out = &cache.cvd;
    let zero_level = CVD_REFERENCE_LEVELS[0].value;
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

    if let Some(zero_y) = zero_y_opt
        && !cvd_out.delta.is_empty()
    {
        let step = conflation_factor.max(1);
        let half_w = ((bar_metrics.body_width_logical as f32) * 0.45).max(0.5);
        let mut hist_mesh = Mesh::default();
        let vis_start = culling_range.start.min(cvd_out.delta.len());
        let vis_end = culling_range.end.min(cvd_out.delta.len());

        let mut push_delta_bar = |center_idx: f64, delta_val: f64, bar_hw: f32| {
            let Some(sx) = vp.float_index_to_screen_x(center_idx) else {
                return;
            };
            let cx = sx as f32;
            if cx + bar_hw < plot_rect.left() || cx - bar_hw > plot_rect.right() {
                return;
            }
            let Some(val_sy) = vp.price_to_screen_y(delta_val) else {
                return;
            };
            let Some((top_y, bot_y)) = resolve_macd_histogram_bar_ys(
                zero_y,
                val_sy as f32,
                delta_val,
                plot_rect.top(),
                plot_rect.bottom(),
            ) else {
                return;
            };
            let col = if delta_val >= 0.0 {
                palette::BULLISH.gamma_multiply(0.45)
            } else {
                palette::BEARISH.gamma_multiply(0.45)
            };
            hist_mesh.add_colored_rect(
                Rect::from_min_max(Pos2::new(cx - bar_hw, top_y), Pos2::new(cx + bar_hw, bot_y)),
                col,
            );
        };

        if step <= 1 {
            for idx in vis_start..vis_end {
                if let Some(d) = cvd_out.delta[idx].filter(|v| v.is_finite()) {
                    push_delta_bar(idx as f64, d, half_w);
                }
            }
        } else {
            let bar_spacing = vp.time_scale.bar_spacing().max(1.0e-6);
            let min_px_bars = (1.0 / bar_spacing).floor().max(1.0) as usize;
            let hist_step = step.max(min_px_bars);
            let conflated_hw = ((hist_step as f64 * bar_spacing * 0.46) as f32)
                .max(half_w)
                .max(0.55);
            let mut b_start = (vis_start / hist_step) * hist_step;
            while b_start < vis_end {
                let b_end = (b_start + hist_step).min(vis_end);
                if b_start < b_end
                    && let Some((min_d, max_d)) = cache
                        .cvd_delta_idx
                        .range_bounds(&cvd_out.delta, b_start..b_end)
                {
                    let center = (b_start + b_end - 1) as f64 * 0.5;
                    if max_d > 0.0 {
                        push_delta_bar(center, max_d, conflated_hw);
                    }
                    if min_d < 0.0 {
                        push_delta_bar(center, min_d, conflated_hw);
                    }
                }
                b_start = b_start.saturating_add(hist_step);
            }
        }

        if !hist_mesh.indices.is_empty() {
            painter.add(Shape::mesh(hist_mesh));
        }
    }

    let total_bars = cvd_out.cvd.len();
    if total_bars == 0 {
        return;
    }
    let starts = &cvd_out.session_starts;
    let first_pos = starts
        .partition_point(|&s| s <= culling_range.start)
        .saturating_sub(1);
    let last_pos = starts.partition_point(|&s| s < culling_range.end);
    let visible_sessions = if starts.is_empty() {
        1
    } else {
        last_pos.saturating_sub(first_pos).max(1)
    };
    let step = conflation_factor.max(1);
    let bar_spacing = vp.time_scale.bar_spacing();
    let line_step = overlay_conflation_step(step, bar_spacing);
    let use_dense_mesh = line_step >= 8 || visible_sessions > 32;
    let mut sma_mesh = use_dense_mesh.then(|| Mesh::with_texture(egui::TextureId::default()));
    let mut cvd_mesh = use_dense_mesh.then(|| Mesh::with_texture(egui::TextureId::default()));
    let mut pts_scratch = Vec::with_capacity(256);

    if starts.is_empty() {
        append_conflated_line_segment(
            painter,
            &mut sma_mesh,
            &mut pts_scratch,
            &cvd_out.cvd_sma,
            Some(&cache.cvd_sma_idx),
            0,
            0..total_bars,
            culling_range.clone(),
            line_step,
            vp,
            Stroke::new(1.25, palette::IND_CVD_SMA),
            1.0,
        );
        append_conflated_line_segment(
            painter,
            &mut cvd_mesh,
            &mut pts_scratch,
            &cvd_out.cvd,
            Some(&cache.cvd_line_idx),
            0,
            0..total_bars,
            culling_range,
            line_step,
            vp,
            Stroke::new(1.6, palette::IND_CVD_LINE),
            1.0,
        );
    } else {
        let mut sma_acc = WorldBinIntervalAccumulator::new(bar_spacing, line_step, 1.25);
        let mut cvd_acc = WorldBinIntervalAccumulator::new(bar_spacing, line_step, 1.6);
        for seg_idx in first_pos..starts.len() {
            let seg_start = starts[seg_idx];
            if seg_start >= culling_range.end {
                break;
            }
            let seg_end = starts
                .get(seg_idx + 1)
                .copied()
                .unwrap_or(total_bars)
                .min(total_bars);
            if seg_start >= seg_end {
                continue;
            }
            let vis_start = seg_start.max(culling_range.start);
            let vis_end = seg_end.min(culling_range.end);
            if vis_start >= vis_end {
                continue;
            }
            let seg_px = ((seg_end - seg_start) as f64) * bar_spacing;
            let is_subpixel_seg = seg_px < 1.25
                || ((vis_end - vis_start) <= line_step && (seg_end - seg_start) <= line_step);
            if is_subpixel_seg {
                sma_acc.push_series_bounds(
                    seg_start,
                    vis_start..vis_end,
                    &cvd_out.cvd_sma,
                    Some(&cache.cvd_sma_idx),
                    vp,
                    0.6,
                    &mut sma_mesh,
                    palette::IND_CVD_SMA,
                );
                cvd_acc.push_series_bounds(
                    seg_start,
                    vis_start..vis_end,
                    &cvd_out.cvd,
                    Some(&cache.cvd_line_idx),
                    vp,
                    0.75,
                    &mut cvd_mesh,
                    palette::IND_CVD_LINE,
                );
            } else {
                sma_acc.flush(&mut sma_mesh, palette::IND_CVD_SMA);
                cvd_acc.flush(&mut cvd_mesh, palette::IND_CVD_LINE);
                let hard_range = seg_start..seg_end;
                append_conflated_line_segment(
                    painter,
                    &mut sma_mesh,
                    &mut pts_scratch,
                    &cvd_out.cvd_sma,
                    Some(&cache.cvd_sma_idx),
                    0,
                    hard_range.clone(),
                    culling_range.clone(),
                    line_step,
                    vp,
                    Stroke::new(1.25, palette::IND_CVD_SMA),
                    1.0,
                );
                append_conflated_line_segment(
                    painter,
                    &mut cvd_mesh,
                    &mut pts_scratch,
                    &cvd_out.cvd,
                    Some(&cache.cvd_line_idx),
                    0,
                    hard_range,
                    culling_range.clone(),
                    line_step,
                    vp,
                    Stroke::new(1.6, palette::IND_CVD_LINE),
                    1.0,
                );
            }
        }
        sma_acc.flush(&mut sma_mesh, palette::IND_CVD_SMA);
        cvd_acc.flush(&mut cvd_mesh, palette::IND_CVD_LINE);
    }
    if let Some(m) = sma_mesh
        && !m.is_empty()
    {
        painter.add(Shape::mesh(m));
    }
    if let Some(m) = cvd_mesh
        && !m.is_empty()
    {
        painter.add(Shape::mesh(m));
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_linked_crosshair(
    painter: &egui::Painter,
    main_plot_painter: &egui::Painter,
    multi_layout: &MultiPaneLayout,
    candles: &[Candle],
    tf_label: &str,
    main_price_scale: &PriceScale,
    main_vp: &PaneViewport<'_>,
    base_value: f64,
    conflation_factor: usize,
    bar_idx: usize,
) {
    if bar_idx >= candles.len() {
        return;
    }
    let snapped_screen_x = if conflation_factor > 1 {
        conflated_candle_at(candles, bar_idx, conflation_factor)
            .and_then(|b| main_vp.float_index_to_screen_x(b.center_logical_index))
            .or_else(|| main_vp.index_to_screen_x(bar_idx as i64))
    } else {
        main_vp.index_to_screen_x(bar_idx as i64)
    };
    let Some(sx) = snapped_screen_x else {
        return;
    };
    let cross_x = sx as f32;
    if cross_x < multi_layout.main_plot_rect.left() || cross_x > multi_layout.main_plot_rect.right()
    {
        return;
    }

    let stroke = Stroke::new(1.0, palette::LINKED_CROSSHAIR_LINE);
    let p_top = Pos2::new(cross_x, multi_layout.main_plot_rect.top());
    let p_bot = Pos2::new(cross_x, multi_layout.main_plot_rect.bottom());
    paint_styled_segment_phased(
        main_plot_painter,
        p_top,
        p_bot,
        p_top,
        stroke,
        LineDashStyle::Dashed,
    );

    for osc in &multi_layout.oscillators {
        let osc_painter = painter.with_clip_rect(osc.plot_rect);
        let ot = Pos2::new(cross_x, osc.plot_rect.top());
        let ob = Pos2::new(cross_x, osc.plot_rect.bottom());
        paint_styled_segment_phased(&osc_painter, ot, ob, ot, stroke, LineDashStyle::Dashed);
    }

    let close_price = candles[bar_idx].close;
    if close_price.is_finite()
        && let Some(sy) = main_vp.price_to_screen_y(close_price)
    {
        let cy = sy as f32;
        if cy >= multi_layout.main_plot_rect.top() && cy <= multi_layout.main_plot_rect.bottom() {
            let hl = Pos2::new(multi_layout.main_plot_rect.left(), cy);
            let hr = Pos2::new(multi_layout.main_plot_rect.right(), cy);
            paint_styled_segment_phased(
                main_plot_painter,
                hl,
                hr,
                hl,
                stroke,
                LineDashStyle::Dashed,
            );
            paint_axis_cursor_pill(
                painter,
                multi_layout.main_price_axis_rect,
                cy,
                &main_price_scale.format_price(close_price, base_value),
            );
        }
    }

    paint_time_axis_cursor_pill(
        painter,
        multi_layout.time_axis_rect,
        candles,
        tf_label,
        bar_idx as i64,
        cross_x,
    );
}

fn paint_relative_strength_pane(
    painter: &egui::Painter,
    plot_rect: Rect,
    cache: &IndicatorCache,
    culling_range: std::ops::Range<usize>,
    conflation_factor: usize,
    vp: &PaneViewport<'_>,
) {
    let base_ref = RS_REFERENCE_LEVELS[0].value;
    if let Some(sy) = vp.price_to_screen_y(base_ref) {
        let y = sy as f32;
        if y >= plot_rect.top() && y <= plot_rect.bottom() {
            let p0 = Pos2::new(plot_rect.left(), y);
            let p1 = Pos2::new(plot_rect.right(), y);
            paint_styled_segment_phased(
                painter,
                p0,
                p1,
                p0,
                Stroke::new(1.0, palette::IND_RS_BASELINE.gamma_multiply(0.75)),
                LineDashStyle::Dashed,
            );
        }
    }

    paint_conflated_line_series(
        painter,
        &cache.rs.rs_sma,
        Some(&cache.rs_sma_idx),
        culling_range.clone(),
        conflation_factor,
        vp,
        Stroke::new(1.25, palette::IND_RS_SMA),
    );
    paint_conflated_line_series(
        painter,
        &cache.rs.rs_line,
        Some(&cache.rs_line_idx),
        culling_range,
        conflation_factor,
        vp,
        Stroke::new(1.65, palette::IND_RS_LINE),
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

fn paint_adx_pane(
    painter: &egui::Painter,
    plot_rect: Rect,
    cache: &IndicatorCache,
    culling_range: std::ops::Range<usize>,
    conflation_factor: usize,
    vp: &PaneViewport<'_>,
) {
    for (lvl, alpha) in [
        (ADX_REFERENCE_LEVELS[0].value, 0.30),
        (ADX_REFERENCE_LEVELS[1].value, 0.55),
        (40.0, 0.35),
    ] {
        if let Some(sy) = vp.price_to_screen_y(lvl) {
            let y = sy as f32;
            if y >= plot_rect.top() && y <= plot_rect.bottom() {
                painter.line_segment(
                    [
                        Pos2::new(plot_rect.left(), y),
                        Pos2::new(plot_rect.right(), y),
                    ],
                    Stroke::new(1.0, palette::IND_ADX_LINE.gamma_multiply(alpha)),
                );
            }
        }
    }
    paint_conflated_line_series(
        painter,
        &cache.adx.plus_di,
        Some(&cache.plus_di_idx),
        culling_range.clone(),
        conflation_factor,
        vp,
        Stroke::new(1.25, palette::IND_PLUS_DI),
    );
    paint_conflated_line_series(
        painter,
        &cache.adx.minus_di,
        Some(&cache.minus_di_idx),
        culling_range.clone(),
        conflation_factor,
        vp,
        Stroke::new(1.25, palette::IND_MINUS_DI),
    );
    paint_conflated_line_series(
        painter,
        &cache.adx.adx,
        Some(&cache.adx_line_idx),
        culling_range,
        conflation_factor,
        vp,
        Stroke::new(1.65, palette::IND_ADX_LINE),
    );
}

fn paint_stochastic_pane(
    painter: &egui::Painter,
    plot_rect: Rect,
    cache: &IndicatorCache,
    culling_range: std::ops::Range<usize>,
    conflation_factor: usize,
    vp: &PaneViewport<'_>,
) {
    let lower_ref = STOCH_REFERENCE_LEVELS[0].value;
    let mid_ref = STOCH_REFERENCE_LEVELS[1].value;
    let upper_ref = STOCH_REFERENCE_LEVELS[2].value;

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
    for (lvl, alpha) in [(upper_ref, 0.50), (mid_ref, 0.25), (lower_ref, 0.50)] {
        if let Some(sy) = vp.price_to_screen_y(lvl) {
            let y = sy as f32;
            if y >= plot_rect.top() && y <= plot_rect.bottom() {
                painter.line_segment(
                    [
                        Pos2::new(plot_rect.left(), y),
                        Pos2::new(plot_rect.right(), y),
                    ],
                    Stroke::new(1.0, palette::IND_STOCH_K.gamma_multiply(alpha)),
                );
            }
        }
    }
    paint_conflated_line_series(
        painter,
        &cache.stochastic.slow_d,
        Some(&cache.stoch_d_idx),
        culling_range.clone(),
        conflation_factor,
        vp,
        Stroke::new(1.3, palette::IND_STOCH_D),
    );
    paint_conflated_line_series(
        painter,
        &cache.stochastic.slow_k,
        Some(&cache.stoch_k_idx),
        culling_range,
        conflation_factor,
        vp,
        Stroke::new(1.55, palette::IND_STOCH_K),
    );
}

fn paint_cci_pane(
    painter: &egui::Painter,
    plot_rect: Rect,
    cache: &IndicatorCache,
    culling_range: std::ops::Range<usize>,
    conflation_factor: usize,
    vp: &PaneViewport<'_>,
) {
    let lower_ref = CCI_REFERENCE_LEVELS[0].value;
    let mid_ref = CCI_REFERENCE_LEVELS[1].value;
    let upper_ref = CCI_REFERENCE_LEVELS[2].value;

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
    for (lvl, alpha) in [(upper_ref, 0.50), (mid_ref, 0.30), (lower_ref, 0.50)] {
        if let Some(sy) = vp.price_to_screen_y(lvl) {
            let y = sy as f32;
            if y >= plot_rect.top() && y <= plot_rect.bottom() {
                painter.line_segment(
                    [
                        Pos2::new(plot_rect.left(), y),
                        Pos2::new(plot_rect.right(), y),
                    ],
                    Stroke::new(1.0, palette::IND_CCI.gamma_multiply(alpha)),
                );
            }
        }
    }
    paint_conflated_line_series(
        painter,
        &cache.cci,
        Some(&cache.cci_idx),
        culling_range,
        conflation_factor,
        vp,
        Stroke::new(1.5, palette::IND_CCI),
    );
}

fn paint_mfi_pane(
    painter: &egui::Painter,
    plot_rect: Rect,
    cache: &IndicatorCache,
    culling_range: std::ops::Range<usize>,
    conflation_factor: usize,
    vp: &PaneViewport<'_>,
) {
    let lower_ref = MFI_REFERENCE_LEVELS[0].value;
    let mid_ref = MFI_REFERENCE_LEVELS[1].value;
    let upper_ref = MFI_REFERENCE_LEVELS[2].value;

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
    for (lvl, alpha) in [(upper_ref, 0.50), (mid_ref, 0.25), (lower_ref, 0.50)] {
        if let Some(sy) = vp.price_to_screen_y(lvl) {
            let y = sy as f32;
            if y >= plot_rect.top() && y <= plot_rect.bottom() {
                painter.line_segment(
                    [
                        Pos2::new(plot_rect.left(), y),
                        Pos2::new(plot_rect.right(), y),
                    ],
                    Stroke::new(1.0, palette::IND_MFI.gamma_multiply(alpha)),
                );
            }
        }
    }
    paint_conflated_line_series(
        painter,
        &cache.mfi,
        Some(&cache.mfi_idx),
        culling_range,
        conflation_factor,
        vp,
        Stroke::new(1.5, palette::IND_MFI),
    );
}

fn paint_proxy_bb_pane(
    painter: &egui::Painter,
    plot_rect: Rect,
    cache: &IndicatorCache,
    culling_range: std::ops::Range<usize>,
    conflation_factor: usize,
    vp: &PaneViewport<'_>,
) {
    for (lvl, alpha, stroke_w) in [
        (PROXY_BB_REFERENCE_LEVELS[0].value, 0.35, 1.0),
        (PROXY_BB_REFERENCE_LEVELS[1].value, 0.65, 1.15),
        (PROXY_BB_REFERENCE_LEVELS[2].value, 0.35, 1.0),
    ] {
        if let Some(sy) = vp.price_to_screen_y(lvl) {
            let y = sy as f32;
            if y >= plot_rect.top() && y <= plot_rect.bottom() {
                painter.line_segment(
                    [
                        Pos2::new(plot_rect.left(), y),
                        Pos2::new(plot_rect.right(), y),
                    ],
                    Stroke::new(stroke_w, palette::TEXT_MUTED.gamma_multiply(alpha)),
                );
            }
        }
    }
    paint_conflated_line_series(
        painter,
        &cache.proxy_bb.t50_values,
        Some(&cache.proxy_bb_t50_idx),
        culling_range.clone(),
        conflation_factor,
        vp,
        Stroke::new(1.25, palette::IND_PROXY_BB_T50),
    );
    paint_conflated_line_series(
        painter,
        &cache.proxy_bb.t05_values,
        Some(&cache.proxy_bb_t05_idx),
        culling_range.clone(),
        conflation_factor,
        vp,
        Stroke::new(1.20, palette::IND_PROXY_BB_T05),
    );
    paint_conflated_line_series(
        painter,
        &cache.proxy_bb.t20_values,
        Some(&cache.proxy_bb_t20_idx),
        culling_range,
        conflation_factor,
        vp,
        Stroke::new(1.75, palette::IND_PROXY_BB_T20),
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

#[inline]
fn clip_screen_segment_to_rect(p0: Pos2, p1: Pos2, rect: Rect) -> Option<(Pos2, Pos2)> {
    let min_x = f64::from(rect.left());
    let max_x = f64::from(rect.right());
    let min_y = f64::from(rect.top());
    let max_y = f64::from(rect.bottom());

    let x0 = f64::from(p0.x);
    let y0 = f64::from(p0.y);
    let dx = f64::from(p1.x) - x0;
    let dy = f64::from(p1.y) - y0;

    let mut t0 = 0.0_f64;
    let mut t1 = 1.0_f64;
    for (p, q) in [
        (-dx, x0 - min_x),
        (dx, max_x - x0),
        (-dy, y0 - min_y),
        (dy, max_y - y0),
    ] {
        if p.abs() <= 1e-12 {
            if q < 0.0 {
                return None;
            }
        } else {
            let r = q / p;
            if p < 0.0 {
                if r > t1 {
                    return None;
                }
                if r > t0 {
                    t0 = r;
                }
            } else {
                if r < t0 {
                    return None;
                }
                if r < t1 {
                    t1 = r;
                }
            }
        }
    }
    Some((
        Pos2::new((x0 + t0 * dx) as f32, (y0 + t0 * dy) as f32),
        Pos2::new((x0 + t1 * dx) as f32, (y0 + t1 * dy) as f32),
    ))
}

#[inline]
fn emit_mesh_thick_segment(mesh: &mut Mesh, p0: Pos2, p1: Pos2, half_w: f32, color: Color32) {
    let dx = p1.x - p0.x;
    let dy = p1.y - p0.y;
    let len_sq = dx * dx + dy * dy;
    if len_sq <= 1e-6 {
        let r = half_w.max(1.0);
        mesh.add_colored_rect(Rect::from_center_size(p0, Vec2::splat(r * 2.0)), color);
        return;
    }
    let inv = len_sq.sqrt().recip() * half_w;
    let nx = -dy * inv;
    let ny = dx * inv;
    let Ok(b) = u32::try_from(mesh.vertices.len()) else {
        return;
    };
    if b > u32::MAX - 4 {
        return;
    }
    mesh.colored_vertex(Pos2::new(p0.x + nx, p0.y + ny), color);
    mesh.colored_vertex(Pos2::new(p0.x - nx, p0.y - ny), color);
    mesh.colored_vertex(Pos2::new(p1.x - nx, p1.y - ny), color);
    mesh.colored_vertex(Pos2::new(p1.x + nx, p1.y + ny), color);
    mesh.add_triangle(b, b + 1, b + 2);
    mesh.add_triangle(b, b + 2, b + 3);
}

/// Emits a vertical-trapezoid slice between `(x0, y0) -> (x1, y1)` and `clamped_base_y`, splitting
/// analytically at `y = plot_top` and `y = plot_bot` (`[MAJOR-05]`) so the shaded polygon matches
/// the clipped stroke segment slope sub-pixel-identically without `f32` overflow at extreme zoom!
#[allow(clippy::too_many_arguments)]
fn emit_y_clipped_baseline_trapezoid(
    fill_mesh: &mut Mesh,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    clamped_base_y: f32,
    plot_top: f32,
    plot_bot: f32,
    fill_col: Color32,
) {
    if (x1 - x0).abs() <= 1e-4 {
        return;
    }
    // Collect sorted split parameters `t in [0.0, 1.0]` where `y(t)` crosses `plot_top` or `plot_bot`.
    let dy = y1 - y0;
    let mut ts = [0.0_f32, 1.0_f32, 1.0_f32, 1.0_f32];
    let mut n = 2_usize;
    if dy.abs() > 1e-5 {
        for bound_y in [plot_top, plot_bot] {
            let t = (bound_y - y0) / dy;
            if t > 1e-4 && t < 1.0 - 1e-4 {
                ts[n] = t;
                n += 1;
            }
        }
        ts[..n].sort_by(|a, b| a.total_cmp(b));
    }

    for k in 0..(n - 1) {
        let ta = ts[k];
        let tb = ts[k + 1];
        if tb - ta <= 1e-5 {
            continue;
        }
        let xa = x0 + ta * (x1 - x0);
        let xb = x0 + tb * (x1 - x0);
        let ya = (y0 + ta * dy).clamp(plot_top, plot_bot);
        let yb = (y0 + tb * dy).clamp(plot_top, plot_bot);
        let top_a = ya.min(clamped_base_y);
        let bot_a = ya.max(clamped_base_y);
        let top_b = yb.min(clamped_base_y);
        let bot_b = yb.max(clamped_base_y);
        if (bot_a - top_a) > 0.05 || (bot_b - top_b) > 0.05 {
            let Ok(bv) = u32::try_from(fill_mesh.vertices.len()) else {
                return;
            };
            if bv > u32::MAX - 4 {
                return;
            }
            fill_mesh.colored_vertex(Pos2::new(xa, top_a), fill_col);
            fill_mesh.colored_vertex(Pos2::new(xa, bot_a), fill_col);
            fill_mesh.colored_vertex(Pos2::new(xb, top_b), fill_col);
            fill_mesh.colored_vertex(Pos2::new(xb, bot_b), fill_col);
            fill_mesh.add_triangle(bv, bv + 1, bv + 2);
            fill_mesh.add_triangle(bv + 1, bv + 3, bv + 2);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_oos_split_marker(
    plot_painter: &egui::Painter,
    plot_rect: Rect,
    bt_bar_count: usize,
    candle_len: usize,
    split_idx: usize,
    vp: &PaneViewport<'_>,
    show_labels: bool,
) {
    if split_idx == 0 || split_idx >= bt_bar_count.min(candle_len) {
        return;
    }
    let Some(sx) = vp.float_index_to_screen_x((split_idx as f64) - 0.5) else {
        return;
    };
    let x = sx as f32;
    if x < plot_rect.left() - 2.0 || x > plot_rect.right() + 2.0 {
        return;
    }

    let mut y = plot_rect.top();
    let bot = plot_rect.bottom();
    let stroke = Stroke::new(1.25, palette::BACKTEST_OOS_SPLIT.gamma_multiply(0.85));
    while y < bot {
        let y_end = (y + 5.0).min(bot);
        plot_painter.line_segment([Pos2::new(x, y), Pos2::new(x, y_end)], stroke);
        y += 9.0;
    }

    if show_labels && plot_rect.height() >= 38.0 {
        let tag_y = (plot_rect.bottom() - 11.0).max(plot_rect.top() + 10.0);
        if x - plot_rect.left() >= 28.0 {
            plot_painter.text(
                Pos2::new(x - 5.0, tag_y),
                Align2::RIGHT_CENTER,
                "IS",
                FontId::monospace(9.5),
                palette::TEXT_MUTED,
            );
        }
        if plot_rect.right() - x >= 34.0 {
            plot_painter.text(
                Pos2::new(x + 5.0, tag_y),
                Align2::LEFT_CENTER,
                "OOS",
                FontId::monospace(9.5),
                palette::BACKTEST_OOS_SPLIT,
            );
        }
    }
}

#[inline]
#[must_use]
pub fn multi_strategy_curve_color(kind: ComparisonCurveKind) -> Color32 {
    match kind {
        ComparisonCurveKind::StrategyPreset(StrategyPresetKind::SmaEmaTrendCross) => {
            Color32::from_rgb(56, 189, 248)
        }
        ComparisonCurveKind::StrategyPreset(StrategyPresetKind::RsiMeanReversion) => {
            Color32::from_rgb(168, 85, 247)
        }
        ComparisonCurveKind::StrategyPreset(StrategyPresetKind::MacdMomentum) => {
            Color32::from_rgb(34, 197, 94)
        }
        ComparisonCurveKind::StrategyPreset(StrategyPresetKind::BollingerSqueezeBreakout) => {
            Color32::from_rgb(249, 115, 22)
        }
        ComparisonCurveKind::StrategyPreset(StrategyPresetKind::DonchianAtrBreakout) => {
            Color32::from_rgb(236, 72, 153)
        }
        ComparisonCurveKind::StrategyPreset(StrategyPresetKind::RsiBollingerConfluence) => {
            Color32::from_rgb(45, 212, 191)
        }
        ComparisonCurveKind::StrategyPreset(StrategyPresetKind::CustomSignalRules) => {
            Color32::from_rgb(250, 204, 21)
        }
        ComparisonCurveKind::AssetBuyAndHold => Color32::from_rgb(226, 232, 240),
        ComparisonCurveKind::MarketBenchmark => Color32::from_rgb(244, 63, 94),
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_multi_strategy_equity_pane(
    axis_painter: &egui::Painter,
    plot_painter: &egui::Painter,
    plot_rect: Rect,
    price_axis_rect: Rect,
    report: &MultiStrategyComparisonReport,
    candle_len: usize,
    culling_range: std::ops::Range<usize>,
    conflation_factor: usize,
    vp: &PaneViewport<'_>,
    crosshair_mark: Option<(usize, f32)>,
) {
    // 1. `0.00%` horizontal baseline + right-axis badge
    if let Some(base_sy) = vp.price_to_screen_y(0.0) {
        let base_y = base_sy as f32;
        if base_y >= plot_rect.top() && base_y <= plot_rect.bottom() {
            let mut ref_mesh = Mesh::with_texture(egui::TextureId::default());
            append_horizontal_styled_line_mesh(
                &mut ref_mesh,
                plot_rect.left(),
                plot_rect.right(),
                base_y,
                1.0,
                palette::EQUITY_BASELINE_REF.gamma_multiply(0.70),
                LineDashStyle::Dashed,
            );
            if !ref_mesh.is_empty() {
                plot_painter.add(Shape::mesh(ref_mesh));
            }
            if let Some(pill_y) = clamp_axis_pill_center_y(base_y, price_axis_rect, 8.0) {
                let pill_rect = Rect::from_min_max(
                    Pos2::new(price_axis_rect.left() + 2.0, pill_y - 7.5),
                    Pos2::new(price_axis_rect.right() - 2.0, pill_y + 7.5),
                );
                axis_painter.rect_filled(pill_rect, 2.5, Color32::from_rgb(31, 39, 53));
                axis_painter.rect_stroke(
                    pill_rect,
                    2.5,
                    Stroke::new(1.0, palette::EQUITY_BASELINE_REF.gamma_multiply(0.55)),
                    StrokeKind::Inside,
                );
                axis_painter.text(
                    Pos2::new(price_axis_rect.left() + 6.0, pill_y),
                    Align2::LEFT_CENTER,
                    "0.00%",
                    FontId::monospace(9.8),
                    palette::TEXT_PRIMARY,
                );
            }
        }
    }

    // 2. Single-pass conflated multi-line Mesh (`7` presets + `B&H` + `Benchmark`)
    let line_step = overlay_conflation_step(conflation_factor.max(1), vp.time_scale.bar_spacing());
    let mut dense_mesh = Some(Mesh::with_texture(egui::TextureId::default()));
    let mut pts_scratch = Vec::with_capacity(256);

    // Draw non-active curves first, then active preset on top so it never gets occluded.
    for pass_active in [false, true] {
        for curve in &report.curves {
            if curve.is_active_preset != pass_active {
                continue;
            }
            let total = curve.return_pct_values.len().min(candle_len);
            if total == 0 {
                continue;
            }
            let base_col = multi_strategy_curve_color(curve.kind);
            let (width, color) = if curve.is_active_preset {
                (2.05, base_col)
            } else if matches!(
                curve.kind,
                ComparisonCurveKind::AssetBuyAndHold | ComparisonCurveKind::MarketBenchmark
            ) {
                (1.50, base_col.gamma_multiply(0.92))
            } else {
                (1.18, base_col.gamma_multiply(0.76))
            };
            append_conflated_line_segment(
                plot_painter,
                &mut dense_mesh,
                &mut pts_scratch,
                &curve.return_pct_values[..total],
                Some(&curve.block_index),
                0,
                0..total,
                culling_range.clone(),
                line_step,
                vp,
                Stroke::new(width, color),
                1.0,
            );
        }
    }

    if let Some(mesh) = dense_mesh
        && !mesh.is_empty()
    {
        plot_painter.add(Shape::mesh(mesh));
    }

    // 3. Synchronized Crosshair Dot Marks (`LWC CrosshairMarksPaneView` parity —
    // suppressed in right-margin whitespace, snapped to conflated bucket center & `end_index`,
    // and rendered for both direct hover and linked multi-chart crosshairs `[CRITIC-R17-MINOR-01]`).
    if let Some((mark_idx, x)) = crosshair_mark
        && mark_idx < candle_len
        && x >= plot_rect.left()
        && x <= plot_rect.right()
    {
        for curve in &report.curves {
            if let Some(v) = curve
                .return_pct_values
                .get(mark_idx)
                .copied()
                .flatten()
                .filter(|v| v.is_finite())
                && let Some(sy) = vp.price_to_screen_y(v)
            {
                let y = sy as f32;
                if y >= plot_rect.top() && y <= plot_rect.bottom() {
                    let col = multi_strategy_curve_color(curve.kind);
                    let r = if curve.is_active_preset { 3.4 } else { 2.4 };
                    plot_painter.circle_filled(Pos2::new(x, y), r, col);
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_equity_baseline_pane(
    axis_painter: &egui::Painter,
    plot_painter: &egui::Painter,
    plot_rect: Rect,
    price_axis_rect: Rect,
    eq_ov: EquityCurveOverlay<'_>,
    monte_carlo_fan: Option<&MonteCarloFanSeries>,
    candle_len: usize,
    culling_range: std::ops::Range<usize>,
    conflation_factor: usize,
    price_scale: &PriceScale,
    vp: &PaneViewport<'_>,
) {
    let baseline = eq_ov.baseline_cash.max(1.0);
    let Some(base_sy) = vp.price_to_screen_y(baseline) else {
        return;
    };
    let base_y = base_sy as f32;
    let clamped_base_y = base_y.clamp(plot_rect.top(), plot_rect.bottom());

    // 1. Dashed horizontal `Initial Capital` (`0.0%`) baseline guide + subtle right-axis marker
    if base_y >= plot_rect.top() && base_y <= plot_rect.bottom() {
        let mut ref_mesh = Mesh::with_texture(egui::TextureId::default());
        append_horizontal_styled_line_mesh(
            &mut ref_mesh,
            plot_rect.left(),
            plot_rect.right(),
            base_y,
            1.0,
            palette::EQUITY_BASELINE_REF.gamma_multiply(0.65),
            LineDashStyle::Dashed,
        );
        if !ref_mesh.is_empty() {
            plot_painter.add(Shape::mesh(ref_mesh));
        }

        if let Some(pill_y) = clamp_axis_pill_center_y(base_y, price_axis_rect, 8.0) {
            let pill_rect = Rect::from_min_max(
                Pos2::new(price_axis_rect.left() + 2.0, pill_y - 7.5),
                Pos2::new(price_axis_rect.right() - 2.0, pill_y + 7.5),
            );
            axis_painter.rect_filled(pill_rect, 2.5, Color32::from_rgb(31, 39, 53));
            axis_painter.rect_stroke(
                pill_rect,
                2.5,
                Stroke::new(1.0, palette::EQUITY_BASELINE_REF.gamma_multiply(0.55)),
                StrokeKind::Inside,
            );
            axis_painter.text(
                Pos2::new(price_axis_rect.left() + 6.0, pill_y),
                Align2::LEFT_CENTER,
                price_scale.format_price_for_tick_density(baseline, vp.base_value, 24.0, 3, 6),
                FontId::monospace(9.8),
                palette::TEXT_PRIMARY,
            );
        }
    }

    // 2. Optional `IS | OOS` walk-forward split boundary
    if let Some(split_idx) = eq_ov.split_bar_index {
        paint_oos_split_marker(
            plot_painter,
            plot_rect,
            eq_ov.bar_count,
            candle_len,
            split_idx,
            vp,
            true,
        );
    }

    // 2b. Optional Deterministic Monte Carlo Bootstrap Percentile Fan (`p5..p95`, `p25..p75`, `p50` median)
    if let Some(fan) = monte_carlo_fan {
        paint_monte_carlo_fan_bands(
            plot_painter,
            plot_rect,
            fan,
            candle_len,
            culling_range.clone(),
            conflation_factor.max(1),
            vp,
        );
    }

    // 3. Single-pass `egui::Mesh` Baseline-Split Area + Stroke (`LWC BaselineSeries` exact crossing split)
    let total = eq_ov.equity_values.len().min(candle_len);
    let step = conflation_factor.max(1);
    let clamped_start = culling_range.start.min(total);
    let clamped_end = culling_range.end.min(total);
    if clamped_start >= clamped_end {
        return;
    }

    let est_samples = ((clamped_end - clamped_start) / step).saturating_add(4);
    let mut fill_mesh = Mesh::with_texture(egui::TextureId::default());
    fill_mesh.vertices.reserve(est_samples * 4);
    fill_mesh.indices.reserve(est_samples * 6);
    let mut stroke_mesh = Mesh::with_texture(egui::TextureId::default());
    stroke_mesh.vertices.reserve(est_samples * 4);
    stroke_mesh.indices.reserve(est_samples * 6);

    let plot_top = plot_rect.top();
    let plot_bot = plot_rect.bottom();
    let half_w = 0.80_f32;

    let mut emit_same_side = |x0: f32, y0: f32, x1: f32, y1: f32, is_above: bool| {
        let (fill_col, line_col) = if is_above {
            (palette::EQUITY_ABOVE_FILL, palette::EQUITY_ABOVE_LINE)
        } else {
            (palette::EQUITY_BELOW_FILL, palette::EQUITY_BELOW_LINE)
        };

        emit_y_clipped_baseline_trapezoid(
            &mut fill_mesh,
            x0,
            y0,
            x1,
            y1,
            clamped_base_y,
            plot_top,
            plot_bot,
            fill_col,
        );

        if let Some((cp0, cp1)) =
            clip_screen_segment_to_rect(Pos2::new(x0, y0), Pos2::new(x1, y1), plot_rect.expand(2.0))
        {
            emit_mesh_thick_segment(&mut stroke_mesh, cp0, cp1, half_w, line_col);
        }
    };

    let mut prev_pt: Option<(f32, f32, f64)> = None;
    let mut push_sample = |x: f32, y: f32, val: f64, prev: &mut Option<(f32, f32, f64)>| {
        let d1 = val - baseline;
        if let Some((x0, y0, d0)) = *prev {
            if (d0 > 0.0 && d1 < 0.0) || (d0 < 0.0 && d1 > 0.0) {
                let denom = y - y0;
                let t = if denom.abs() > 1e-6 {
                    ((base_y - y0) / denom).clamp(0.0, 1.0)
                } else {
                    0.5
                };
                let x_cross = x0 + t * (x - x0);
                emit_same_side(x0, y0, x_cross, base_y, d0 >= 0.0);
                emit_same_side(x_cross, base_y, x, y, d1 >= 0.0);
            } else {
                let is_above = if d0.abs() >= d1.abs() {
                    d0 >= 0.0
                } else {
                    d1 >= 0.0
                };
                emit_same_side(x0, y0, x, y, is_above);
            }
        }
        *prev = Some((x, y, d1));
    };

    if step == 1 {
        let sample_start = clamped_start.saturating_sub(1);
        let sample_end = (clamped_end + 1).min(total);
        for idx in sample_start..sample_end {
            if let Some(v) = eq_ov.equity_values[idx].filter(|v| v.is_finite())
                && let (Some(sx), Some(sy)) = (
                    vp.float_index_to_screen_x(idx as f64),
                    vp.price_to_screen_y(v),
                )
            {
                push_sample(sx as f32, sy as f32, v, &mut prev_pt);
            } else {
                prev_pt = None;
            }
        }
    } else {
        let aligned_start = (clamped_start / step).saturating_sub(1) * step;
        let aligned_end = clamped_end
            .div_ceil(step)
            .saturating_add(1)
            .saturating_mul(step)
            .min(total);
        let mut b_start = aligned_start;
        while b_start < aligned_end {
            let b_end = (b_start + step).min(total);
            if let Some(env) = bucket_line_envelope_indexed(
                eq_ov.equity_values,
                Some(eq_ov.equity_idx),
                b_start,
                b_end,
            ) && let (Some(sx), Some(sy_in), Some(sy_out)) = (
                vp.float_index_to_screen_x(env.center_logical_index),
                vp.price_to_screen_y(env.entry_val),
                vp.price_to_screen_y(env.exit_val),
            ) {
                let x = sx as f32;
                if env.has_leading_gap {
                    prev_pt = None;
                }
                let y_in = sy_in as f32;
                let y_out = sy_out as f32;
                if env.has_internal_gap {
                    push_sample(x, y_in, env.entry_val, &mut prev_pt);
                    prev_pt = None;
                    push_sample(x, y_out, env.exit_val, &mut prev_pt);
                } else if env.is_single || (y_out - y_in).abs() < 0.65 {
                    let mid_y = 0.5 * (y_in + y_out);
                    let mid_v = safe_midpoint_f64(env.entry_val, env.exit_val);
                    push_sample(x, mid_y, mid_v, &mut prev_pt);
                } else {
                    push_sample(x, y_in, env.entry_val, &mut prev_pt);
                    push_sample(x, y_out, env.exit_val, &mut prev_pt);
                }
                if env.has_trailing_gap {
                    prev_pt = None;
                }
            } else {
                prev_pt = None;
            }
            b_start = b_end;
        }
    }

    if !fill_mesh.is_empty() {
        plot_painter.add(Shape::mesh(fill_mesh));
    }
    if !stroke_mesh.is_empty() {
        plot_painter.add(Shape::mesh(stroke_mesh));
    }
}

fn paint_monte_carlo_fan_bands(
    plot_painter: &egui::Painter,
    plot_rect: Rect,
    fan: &MonteCarloFanSeries,
    candle_len: usize,
    culling_range: std::ops::Range<usize>,
    step: usize,
    vp: &PaneViewport<'_>,
) {
    let total = fan.causal_bar_count.min(fan.bar_count).min(candle_len);
    let eff_step = step.max(1);
    let clamped_start = culling_range.start.min(total);
    let start = if eff_step > 1 {
        (clamped_start / eff_step).saturating_sub(1) * eff_step
    } else {
        clamped_start.saturating_sub(1)
    };
    let end = culling_range.end.saturating_add(eff_step).min(total);
    if start + 1 >= end {
        return;
    }

    let plot_top = plot_rect.top();
    let plot_bot = plot_rect.bottom();
    let is_log = vp.price_scale.is_log();
    let log_floor = if is_log {
        let p_top = vp.screen_y_to_price(f64::from(plot_top)).unwrap_or(1.0);
        let p_bot = vp.screen_y_to_price(f64::from(plot_bot)).unwrap_or(1.0);
        let vis_lo = p_top.min(p_bot);
        if vis_lo.is_finite() && vis_lo > 1.0e-4 {
            (vis_lo * 0.92).max(1.0e-4)
        } else {
            1.0e-4
        }
    } else {
        0.0
    };
    let outer_col = Color32::from_rgba_unmultiplied(88, 166, 255, 24);
    let inner_col = Color32::from_rgba_unmultiplied(88, 166, 255, 48);
    let median_col = Color32::from_rgba_unmultiplied(245, 194, 66, 215);

    let est = ((end - start) / eff_step).saturating_add(6);
    let mut band_mesh = Mesh::with_texture(egui::TextureId::default());
    band_mesh.vertices.reserve(est * 8);
    band_mesh.indices.reserve(est * 12);
    let mut median_mesh = Mesh::with_texture(egui::TextureId::default());
    median_mesh.vertices.reserve(est * 8);
    median_mesh.indices.reserve(est * 12);

    let emit_band_quad = |mesh: &mut Mesh,
                          x0: f32,
                          y0_hi: f32,
                          y0_lo: f32,
                          x1: f32,
                          y1_hi: f32,
                          y1_lo: f32,
                          col: Color32| {
        if (x1 - x0).abs() <= 1e-4 {
            return;
        }
        let a_top = y0_hi.min(y0_lo).clamp(plot_top, plot_bot);
        let a_bot = y0_hi.max(y0_lo).clamp(plot_top, plot_bot);
        let b_top = y1_hi.min(y1_lo).clamp(plot_top, plot_bot);
        let b_bot = y1_hi.max(y1_lo).clamp(plot_top, plot_bot);
        if (a_bot - a_top) <= 0.05 && (b_bot - b_top) <= 0.05 {
            return;
        }
        let Ok(bv) = u32::try_from(mesh.vertices.len()) else {
            return;
        };
        if bv > u32::MAX - 4 {
            return;
        }
        let uv = egui::epaint::WHITE_UV;
        mesh.vertices.extend_from_slice(&[
            egui::epaint::Vertex {
                pos: Pos2::new(x0, a_top),
                uv,
                color: col,
            },
            egui::epaint::Vertex {
                pos: Pos2::new(x1, b_top),
                uv,
                color: col,
            },
            egui::epaint::Vertex {
                pos: Pos2::new(x1, b_bot),
                uv,
                color: col,
            },
            egui::epaint::Vertex {
                pos: Pos2::new(x0, a_bot),
                uv,
                color: col,
            },
        ]);
        mesh.indices
            .extend_from_slice(&[bv, bv + 1, bv + 2, bv, bv + 2, bv + 3]);
    };

    const DASH_ON_PX: f32 = 5.0;
    const DASH_GAP_PX: f32 = 3.5;
    const DASH_PERIOD_PX: f32 = DASH_ON_PX + DASH_GAP_PX;

    let emit_dashed_median = |mesh: &mut Mesh, p0: Pos2, p1: Pos2, phase_px: &mut f32| {
        let dx = p1.x - p0.x;
        let dy = p1.y - p0.y;
        let seg_len = dx.hypot(dy);
        if !seg_len.is_finite() || seg_len <= 1.0e-4 {
            return;
        }
        let ux = dx / seg_len;
        let uy = dy / seg_len;
        let mut walked = 0.0_f32;
        while walked + 1.0e-3 < seg_len {
            let rem = seg_len - walked;
            let cyc = phase_px.rem_euclid(DASH_PERIOD_PX);
            if cyc < DASH_ON_PX {
                let take = (DASH_ON_PX - cyc).min(rem);
                let a = Pos2::new(p0.x + ux * walked, p0.y + uy * walked);
                let b = Pos2::new(p0.x + ux * (walked + take), p0.y + uy * (walked + take));
                emit_mesh_thick_segment(mesh, a, b, 0.72, median_col);
                walked += take;
                *phase_px = (*phase_px + take).rem_euclid(DASH_PERIOD_PX);
            } else {
                let take = (DASH_PERIOD_PX - cyc).min(rem);
                walked += take;
                *phase_px = (*phase_px + take).rem_euclid(DASH_PERIOD_PX);
            }
        }
    };

    let map_fan_y = |v: f64| -> Option<f64> {
        let adj = if is_log { v.max(log_floor) } else { v };
        vp.price_to_screen_y(adj)
    };

    type FanPt = (f32, f32, f32, f32, f32, f32);
    let mut prev: Option<FanPt> = None;
    let mut dash_phase_px = 0.0_f32;
    let mut last_sampled: Option<usize> = None;
    let mut idx = start;
    let mut terminal_done = false;
    loop {
        let sample_idx = if idx < end {
            let s = if eff_step > 1 {
                let b_end = (idx + eff_step).min(total);
                idx + (b_end - idx) / 2
            } else {
                idx
            };
            idx = idx.saturating_add(eff_step);
            s
        } else if !terminal_done
            && eff_step > 1
            && end == total
            && total > 0
            && last_sampled != Some(total - 1)
        {
            terminal_done = true;
            total - 1
        } else {
            break;
        };
        last_sampled = Some(sample_idx);

        if let (Some(v5), Some(v25), Some(v50), Some(v75), Some(v95)) = (
            fan.p5.get(sample_idx).copied().flatten(),
            fan.p25.get(sample_idx).copied().flatten(),
            fan.p50.get(sample_idx).copied().flatten(),
            fan.p75.get(sample_idx).copied().flatten(),
            fan.p95.get(sample_idx).copied().flatten(),
        ) && let (Some(sx), Some(y5), Some(y25), Some(y50), Some(y75), Some(y95)) = (
            vp.float_index_to_screen_x(sample_idx as f64),
            map_fan_y(v5),
            map_fan_y(v25),
            map_fan_y(v50),
            map_fan_y(v75),
            map_fan_y(v95),
        ) {
            let curr = (
                sx as f32, y5 as f32, y25 as f32, y50 as f32, y75 as f32, y95 as f32,
            );
            if let Some((x0, py5, py25, py50, py75, py95)) = prev {
                emit_band_quad(
                    &mut band_mesh,
                    x0,
                    py95,
                    py5,
                    curr.0,
                    curr.5,
                    curr.1,
                    outer_col,
                );
                emit_band_quad(
                    &mut band_mesh,
                    x0,
                    py75,
                    py25,
                    curr.0,
                    curr.4,
                    curr.2,
                    inner_col,
                );
                if let Some((cp0, cp1)) = clip_screen_segment_to_rect(
                    Pos2::new(x0, py50),
                    Pos2::new(curr.0, curr.3),
                    plot_rect.expand(1.5),
                ) {
                    emit_dashed_median(&mut median_mesh, cp0, cp1, &mut dash_phase_px);
                } else {
                    dash_phase_px = 0.0;
                }
            }
            prev = Some(curr);
        } else {
            prev = None;
            dash_phase_px = 0.0;
        }
    }

    if !band_mesh.is_empty() {
        plot_painter.add(Shape::mesh(band_mesh));
    }
    if !median_mesh.is_empty() {
        plot_painter.add(Shape::mesh(median_mesh));
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_backtest_trade_links(
    plot_painter: &egui::Painter,
    plot_rect: Rect,
    candles: &[Candle],
    culling_range: std::ops::Range<usize>,
    conflation_factor: usize,
    main_vp: &PaneViewport<'_>,
    trades: &[BacktestTrade],
    source_bar_count: usize,
    focused_trade_id: Option<u64>,
    replay_cutoff_timestamp: Option<i64>,
) -> usize {
    if trades.is_empty() || candles.is_empty() || culling_range.is_empty() {
        return 0;
    }

    let cf = conflation_factor.max(1);
    let vis_start = if cf > 1 {
        (culling_range.start / cf) * cf
    } else {
        culling_range.start
    }
    .min(candles.len());
    let vis_end = if cf > 1 {
        culling_range
            .end
            .div_ceil(cf)
            .saturating_mul(cf)
            .min(candles.len())
    } else {
        culling_range.end.min(candles.len())
    };
    if vis_start >= vis_end {
        return 0;
    }

    let resolve_bar_idx = |bar_idx: usize, ts: i64| -> usize {
        if source_bar_count == candles.len()
            && candles.get(bar_idx).is_some_and(|c| c.timestamp == ts)
        {
            bar_idx
        } else {
            candles
                .partition_point(|c| c.timestamp <= ts)
                .saturating_sub(1)
                .min(candles.len() - 1)
        }
    };

    let resolve_logical_x = |idx: usize| -> f64 {
        if cf > 1 {
            conflated_candle_at(candles, idx, cf).map_or(idx as f64, |b| b.center_logical_index)
        } else {
            idx as f64
        }
    };

    // O(log T) interval overlap query on non-overlapping chronological trades
    let first_vis = if source_bar_count == candles.len() {
        trades.partition_point(|t| t.exit_bar_index < vis_start)
    } else {
        let min_ts = candles[vis_start].timestamp;
        trades.partition_point(|t| t.exit_timestamp < min_ts)
    };
    let last_vis = if source_bar_count == candles.len() {
        trades.partition_point(|t| t.entry_bar_index <= vis_end)
    } else if vis_end < candles.len() {
        let max_ts = candles[vis_end].timestamp;
        trades.partition_point(|t| t.entry_timestamp <= max_ts)
    } else {
        trades.len()
    };

    let Some(vis_slice) = trades.get(first_vis..last_vis) else {
        return 0;
    };
    if vis_slice.is_empty() {
        return 0;
    }

    const MAX_RENDERED_LINKS: usize = 192;
    let stride = vis_slice.len().div_ceil(MAX_RENDERED_LINKS).max(1);
    let cap = vis_slice.len().min(MAX_RENDERED_LINKS) + 2;

    let mut mesh = Mesh::with_texture(egui::TextureId::default());
    mesh.vertices.reserve(cap * 16);
    mesh.indices.reserve(cap * 24);
    let border_col = Color32::from_rgba_unmultiplied(10, 14, 20, 228);
    let clip_rect = plot_rect.expand(3.0);

    let mut rendered = 0_usize;
    let mut focused_badge: Option<(Pos2, &BacktestTrade, Color32)> = None;

    for (local_k, trade) in vis_slice.iter().enumerate() {
        let is_focused = focused_trade_id == Some(trade.trade_id);
        let global_k = first_vis + local_k;
        if !is_focused && stride > 1 && !global_k.is_multiple_of(stride) {
            continue;
        }
        if let Some(cutoff) = replay_cutoff_timestamp
            && trade.exit_timestamp > cutoff
        {
            continue;
        }

        let entry_idx = resolve_bar_idx(trade.entry_bar_index, trade.entry_timestamp);
        let exit_idx = resolve_bar_idx(trade.exit_bar_index, trade.exit_timestamp);
        let (Some(ex0), Some(ey0), Some(ex1), Some(ey1)) = (
            main_vp.float_index_to_screen_x(resolve_logical_x(entry_idx)),
            main_vp.price_to_screen_y(trade.entry_price),
            main_vp.float_index_to_screen_x(resolve_logical_x(exit_idx)),
            main_vp.price_to_screen_y(trade.exit_price),
        ) else {
            continue;
        };

        let p0 = Pos2::new(ex0 as f32, ey0 as f32);
        let p1 = Pos2::new(ex1 as f32, ey1 as f32);
        let Some((cp0, cp1)) = clip_screen_segment_to_rect(p0, p1, clip_rect) else {
            continue;
        };

        let base_col = if trade.net_pnl >= 0.0 {
            palette::TRADE_LINK_WIN
        } else {
            palette::TRADE_LINK_LOSS
        };

        if is_focused {
            emit_mesh_thick_segment(
                &mut mesh,
                cp0,
                cp1,
                2.5,
                palette::TRADE_FOCUS_HALO.gamma_multiply(0.45),
            );
            emit_mesh_thick_segment(&mut mesh, cp0, cp1, 1.15, palette::TRADE_FOCUS_HALO);
            let mid = Pos2::new(0.5 * (cp0.x + cp1.x), 0.5 * (cp0.y + cp1.y) - 14.0);
            focused_badge = Some((mid, trade, base_col));
        } else {
            let line_alpha = if cf >= 4 { 0.45 } else { 0.68 };
            emit_mesh_thick_segment(
                &mut mesh,
                cp0,
                cp1,
                0.72,
                base_col.gamma_multiply(line_alpha),
            );
        }

        let marker_r = if is_focused {
            5.0_f32
        } else if cf >= 8 {
            2.6_f32
        } else {
            3.8_f32
        };

        // Entry arrow triangle at `p0` if inside `plot_rect`
        if plot_rect.contains(p0) {
            let entry_col = match trade.side {
                PositionSide::Long => palette::MARKER_BUY_ARROW,
                PositionSide::Short => palette::MARKER_SELL_ARROW,
            };
            let dir = match trade.side {
                PositionSide::Long => -1.0_f32, // tip points UP
                PositionSide::Short => 1.0_f32, // tip points DOWN
            };
            let tip = Pos2::new(p0.x, p0.y + dir * marker_r);
            let bl = Pos2::new(p0.x - marker_r * 0.88, p0.y - dir * marker_r * 0.85);
            let br = Pos2::new(p0.x + marker_r * 0.88, p0.y - dir * marker_r * 0.85);
            let Ok(hb) = u32::try_from(mesh.vertices.len()) else {
                break;
            };
            if hb > u32::MAX - 8 {
                break;
            }
            mesh.colored_vertex(Pos2::new(tip.x, tip.y + dir * 0.9), border_col);
            mesh.colored_vertex(Pos2::new(bl.x - 0.8, bl.y - dir * 0.7), border_col);
            mesh.colored_vertex(Pos2::new(br.x + 0.8, br.y - dir * 0.7), border_col);
            mesh.add_triangle(hb, hb + 1, hb + 2);

            let ib = hb + 3;
            mesh.colored_vertex(tip, entry_col);
            mesh.colored_vertex(bl, entry_col);
            mesh.colored_vertex(br, entry_col);
            mesh.add_triangle(ib, ib + 1, ib + 2);
        }

        // Exit diamond at `p1` if inside `plot_rect`
        if plot_rect.contains(p1) {
            let exit_col = match trade.exit_reason {
                BacktestExitReason::StopLoss | BacktestExitReason::MarginLiquidation => {
                    palette::TRADE_STOP_LOSS
                }
                BacktestExitReason::TakeProfit => palette::TRADE_TAKE_PROFIT,
                _ => base_col,
            };
            let dr = (marker_r * 0.92).max(2.3);
            let outer = dr + 0.95;
            let Ok(ob) = u32::try_from(mesh.vertices.len()) else {
                break;
            };
            if ob > u32::MAX - 10 {
                break;
            }
            mesh.colored_vertex(Pos2::new(p1.x, p1.y - outer), border_col);
            mesh.colored_vertex(Pos2::new(p1.x + outer, p1.y), border_col);
            mesh.colored_vertex(Pos2::new(p1.x, p1.y + outer), border_col);
            mesh.colored_vertex(Pos2::new(p1.x - outer, p1.y), border_col);
            mesh.add_triangle(ob, ob + 1, ob + 2);
            mesh.add_triangle(ob, ob + 2, ob + 3);

            let ib = ob + 4;
            mesh.colored_vertex(Pos2::new(p1.x, p1.y - dr), exit_col);
            mesh.colored_vertex(Pos2::new(p1.x + dr, p1.y), exit_col);
            mesh.colored_vertex(Pos2::new(p1.x, p1.y + dr), exit_col);
            mesh.colored_vertex(Pos2::new(p1.x - dr, p1.y), exit_col);
            mesh.add_triangle(ib, ib + 1, ib + 2);
            mesh.add_triangle(ib, ib + 2, ib + 3);
        }

        rendered += 1;
    }

    if !mesh.is_empty() {
        plot_painter.add(Shape::mesh(mesh));
    }

    if let Some((anchor, trade, col)) = focused_badge {
        let r_txt = trade
            .r_multiple
            .map_or_else(String::new, |r| format!(" ({r:+.2}R)"));
        let label = format!(
            "#{} {} {:+.2}%{r_txt} · {}",
            trade.trade_id,
            match trade.side {
                PositionSide::Long => "LONG",
                PositionSide::Short => "SHORT",
            },
            trade.return_pct,
            trade.exit_reason.short_label()
        );
        let galley = plot_painter.layout_no_wrap(label, FontId::monospace(10.0), col);
        let pad = Vec2::new(6.0, 2.5);
        let size = galley.size() + pad * 2.0;
        let bx = (anchor.x - size.x * 0.5).clamp(
            plot_rect.left() + 6.0,
            (plot_rect.right() - size.x - 6.0).max(plot_rect.left() + 6.0),
        );
        let by = (anchor.y - size.y * 0.5).clamp(
            plot_rect.top() + 36.0,
            (plot_rect.bottom() - size.y - 6.0).max(plot_rect.top() + 36.0),
        );
        let badge_rect = Rect::from_min_size(Pos2::new(bx, by), size);
        plot_painter.rect_filled(badge_rect, 3.5, palette::BG_FLOATING_HUD);
        plot_painter.rect_stroke(
            badge_rect,
            3.5,
            Stroke::new(1.1, palette::TRADE_FOCUS_HALO),
            StrokeKind::Inside,
        );
        plot_painter.galley(badge_rect.min + pad, galley, col);
    }

    rendered
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

            if let Some(pill_y) = clamp_axis_pill_center_y(last_y, price_axis_rect, 10.0) {
                let pill_rect = Rect::from_min_max(
                    Pos2::new(price_axis_rect.left() + 2.0, pill_y - 10.0),
                    Pos2::new(price_axis_rect.right() - 4.0, pill_y + 10.0),
                );
                let axis_painter = painter.with_clip_rect(price_axis_rect);
                axis_painter.rect_filled(pill_rect, 3.0, last_color);
                axis_painter.text(
                    Pos2::new(price_axis_rect.left() + 7.0, pill_y),
                    Align2::LEFT_CENTER,
                    main_price_scale.format_price(last_candle.close, base_value),
                    FontId::monospace(11.0),
                    Color32::WHITE,
                );
            }
        }
    }
}

fn paint_axis_cursor_pill(
    painter: &egui::Painter,
    axis_rect: Rect,
    cross_y: f32,
    formatted_label: &str,
) {
    let Some(pill_y) = clamp_axis_pill_center_y(cross_y, axis_rect, 10.0) else {
        return;
    };
    let cursor_pill = Rect::from_min_max(
        Pos2::new(axis_rect.left() + 2.0, pill_y - 9.5),
        Pos2::new(axis_rect.right() - 4.0, pill_y + 9.5),
    );
    let axis_painter = painter.with_clip_rect(axis_rect);
    axis_painter.rect_filled(cursor_pill, 3.0, palette::ACCENT_BLUE);
    axis_painter.text(
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
    let Some(time_pill) = compute_time_axis_cursor_pill_rect(time_axis_rect, &time_label, cross_x)
    else {
        return;
    };
    let time_axis_painter = painter.with_clip_rect(time_axis_rect);
    time_axis_painter.rect_filled(time_pill, 3.0, palette::ACCENT_BLUE);
    time_axis_painter.text(
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
    let is_compact_hud = plot_rect.width() < 660.0;
    let compact_date = if is_compact_hud && inspected_bucket.date_label.len() > 12 {
        inspected_bucket
            .date_label
            .get(5..)
            .unwrap_or(inspected_bucket.date_label)
    } else {
        inspected_bucket.date_label
    };
    let (hud_text, hud_font_size) = if is_compact_hud {
        (
            format!(
                "{}·{}{confl_badge} {}  O{:.2} H{:.2} L{:.2} C{:.2} ({sign}{:.2}%) V{vol_str}",
                config.symbol,
                config.timeframe,
                compact_date,
                inspected_bucket.open,
                inspected_bucket.high,
                inspected_bucket.low,
                inspected_bucket.close,
                inspected_bucket.change_pct(),
            ),
            10.5_f32,
        )
    } else {
        (
            format!(
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
            ),
            11.5_f32,
        )
    };
    let value_color = if inspected_bucket.is_bullish() {
        palette::BULLISH
    } else {
        palette::BEARISH
    };
    let mut hud_job = LayoutJob::single_section(
        hud_text,
        TextFormat {
            font_id: FontId::monospace(hud_font_size),
            color: value_color,
            ..Default::default()
        },
    );
    let inner_max_w = (plot_rect.width() - 44.0).max(80.0);
    hud_job.wrap = egui::text::TextWrapping {
        max_width: inner_max_w,
        max_rows: 1,
        break_anywhere: false,
        overflow_character: Some('…'),
    };
    let hud_galley = painter.layout_job(hud_job);
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

#[allow(clippy::too_many_arguments)]
fn paint_main_overlay_hud(
    painter: &egui::Painter,
    plot_rect: Rect,
    indicators: &IndicatorConfig,
    cache: &IndicatorCache,
    idx: usize,
    vrvp: Option<&VolumeProfileResult>,
    compare_info: Option<(&str, &[Option<f64>], f64)>,
    price_scale: &PriceScale,
    base_value: f64,
) {
    if plot_rect.height() < 64.0 {
        return;
    }
    let mut job = LayoutJob::default();
    let font = FontId::monospace(10.8);
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
                font_id: font.clone(),
                color: palette::IND_BB_BORDER,
                ..Default::default()
            },
        );
    }
    if indicators.show_vwap {
        append_sep(&mut job, &mut has_item);
        let fmt_opt = |s: &[Option<f64>]| {
            s.get(idx).copied().flatten().map_or_else(
                || "—".to_owned(),
                |v| price_scale.format_price(v, base_value),
            )
        };
        let anchor_lbl = indicators.vwap_anchor_mode.short_label();
        let label = if plot_rect.width() >= 960.0 && indicators.vwap_band_1 > 0.0 {
            format!(
                "VWAP({anchor_lbl}) {} (±1σ {}/{})",
                fmt_opt(&cache.vwap.vwap),
                fmt_opt(&cache.vwap.lower_1),
                fmt_opt(&cache.vwap.upper_1),
            )
        } else {
            format!("VWAP({anchor_lbl}) {}", fmt_opt(&cache.vwap.vwap))
        };
        job.append(
            &label,
            0.0,
            TextFormat {
                font_id: font.clone(),
                color: palette::IND_VWAP_LINE,
                ..Default::default()
            },
        );
    }
    if let Some(profile) = vrvp {
        append_sep(&mut job, &mut has_item);
        let poc_str = price_scale.format_price(profile.poc_price, base_value);
        let label = if plot_rect.width() >= 960.0 {
            format!(
                "VRVP POC {poc_str} ({}–{})",
                price_scale.format_price(profile.val_price, base_value),
                price_scale.format_price(profile.vah_price, base_value),
            )
        } else {
            format!("VRVP POC {poc_str}")
        };
        job.append(
            &label,
            0.0,
            TextFormat {
                font_id: font.clone(),
                color: palette::VRVP_POC_GOLD,
                ..Default::default()
            },
        );
    }
    if indicators.show_market_profile
        && let Some(mp) = cache
            .market_profile
            .profiles
            .iter()
            .find(|s| s.start_index <= idx && idx <= s.end_index)
            .or_else(|| cache.market_profile.profiles.last())
    {
        append_sep(&mut job, &mut has_item);
        let poc_str = price_scale.format_price(mp.poc_price, base_value);
        let label = if plot_rect.width() >= 960.0 {
            format!(
                "TPO POC {poc_str} (VA {}–{})",
                price_scale.format_price(mp.val_price, base_value),
                price_scale.format_price(mp.vah_price, base_value),
            )
        } else {
            format!("TPO POC {poc_str}")
        };
        job.append(
            &label,
            0.0,
            TextFormat {
                font_id: font.clone(),
                color: palette::MP_POC_LINE,
                ..Default::default()
            },
        );
    }
    if indicators.show_ichimoku && !cache.ichimoku.tenkan.is_empty() {
        append_sep(&mut job, &mut has_item);
        let fmt_p = |s: &[Option<f64>]| {
            s.get(idx).copied().flatten().map_or_else(
                || "—".to_owned(),
                |v| price_scale.format_price(v, base_value),
            )
        };
        let label = if plot_rect.width() >= 960.0 {
            format!(
                "ICH({},{},{},{}) T {} K {} A {} B {}",
                indicators.ichimoku_tenkan,
                indicators.ichimoku_kijun,
                indicators.ichimoku_senkou_b,
                indicators.ichimoku_displacement,
                fmt_p(&cache.ichimoku.tenkan),
                fmt_p(&cache.ichimoku.kijun),
                fmt_p(&cache.ichimoku.senkou_a),
                fmt_p(&cache.ichimoku.senkou_b),
            )
        } else {
            format!(
                "ICH T {} K {}",
                fmt_p(&cache.ichimoku.tenkan),
                fmt_p(&cache.ichimoku.kijun),
            )
        };
        job.append(
            &label,
            0.0,
            TextFormat {
                font_id: font.clone(),
                color: palette::IND_ICHIMOKU_TENKAN,
                ..Default::default()
            },
        );
    }
    if indicators.show_keltner {
        append_sep(&mut job, &mut has_item);
        let fmt_opt = |s: &[Option<f64>]| {
            s.get(idx).copied().flatten().map_or_else(
                || "—".to_owned(),
                |v| price_scale.format_price(v, base_value),
            )
        };
        job.append(
            &format!(
                "KC({},{},{:.1}) {} / {} / {}",
                indicators.keltner_ema_period,
                indicators.keltner_atr_period,
                indicators.keltner_multiplier,
                fmt_opt(&cache.keltner.upper),
                fmt_opt(&cache.keltner.middle),
                fmt_opt(&cache.keltner.lower),
            ),
            0.0,
            TextFormat {
                font_id: font.clone(),
                color: palette::IND_KELTNER_BORDER,
                ..Default::default()
            },
        );
    }
    if indicators.show_supertrend {
        append_sep(&mut job, &mut has_item);
        let st_px = cache.supertrend.trend.get(idx).copied().flatten();
        let st_dir = cache.supertrend.direction.get(idx).copied().flatten();
        let px_str = st_px.map_or_else(
            || "—".to_owned(),
            |v| price_scale.format_price(v, base_value),
        );
        let (dir_lbl, col) = match st_dir {
            Some(d) if d >= 0 => ("BULL", palette::IND_SUPERTREND_BULL),
            Some(_) => ("BEAR", palette::IND_SUPERTREND_BEAR),
            None => ("—", palette::TEXT_MUTED),
        };
        job.append(
            &format!(
                "ST({},{:.1}) {px_str} [{dir_lbl}]",
                indicators.supertrend_period, indicators.supertrend_multiplier
            ),
            0.0,
            TextFormat {
                font_id: font.clone(),
                color: col,
                ..Default::default()
            },
        );
    }
    if indicators.show_psar {
        append_sep(&mut job, &mut has_item);
        let sar_px = cache.psar.sar.get(idx).copied().flatten();
        let is_long = cache.psar.is_long.get(idx).copied().flatten();
        let px_str = sar_px.map_or_else(
            || "—".to_owned(),
            |v| price_scale.format_price(v, base_value),
        );
        let col = match is_long {
            Some(true) => palette::IND_PSAR_BULL,
            Some(false) => palette::IND_PSAR_BEAR,
            None => palette::TEXT_MUTED,
        };
        job.append(
            &format!(
                "PSAR({:.2},{:.2}) {px_str}",
                indicators.psar_af_start, indicators.psar_af_max
            ),
            0.0,
            TextFormat {
                font_id: font.clone(),
                color: col,
                ..Default::default()
            },
        );
    }
    if indicators.show_volume_sma {
        append_sep(&mut job, &mut has_item);
        let vs_str = cache
            .volume_sma
            .get(idx)
            .copied()
            .flatten()
            .map_or_else(|| "—".to_owned(), format_compact_signed);
        job.append(
            &format!("VolSMA({}) {vs_str}", indicators.volume_sma_period),
            0.0,
            TextFormat {
                font_id: font.clone(),
                color: palette::IND_VOLUME_SMA,
                ..Default::default()
            },
        );
    }
    if indicators.show_candle_patterns
        && let Some(pat) = cache.candle_patterns.get(idx)
    {
        append_sep(&mut job, &mut has_item);
        let badge_txt = pat.primary_badge.map_or("None", |k| k.short_badge());
        let col = match pat.primary_badge {
            Some(k) if k.is_bullish() => palette::BULLISH,
            Some(k) if k.is_bearish() => palette::BEARISH,
            _ => palette::TEXT_MUTED,
        };
        let score_str = pat
            .candle_score
            .map_or_else(|| "—".to_owned(), |s| format!("{s:+.2}"));
        job.append(
            &format!("PAT {badge_txt} ({score_str})"),
            0.0,
            TextFormat {
                font_id: font.clone(),
                color: col,
                ..Default::default()
            },
        );
    }
    if let Some((bench_sym, cmp_series, cmp_scale)) = compare_info {
        append_sep(&mut job, &mut has_item);
        let val = cmp_series
            .get(idx)
            .copied()
            .flatten()
            .filter(|v| v.is_finite())
            .map(|v| v * cmp_scale)
            .filter(|v| v.is_finite())
            .map_or_else(
                || "—".to_owned(),
                |v| price_scale.format_price(v, base_value),
            );
        job.append(
            &format!("CMP({bench_sym}) {val}"),
            0.0,
            TextFormat {
                font_id: font,
                color: palette::COMPARE_BENCH_LINE,
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

#[allow(clippy::too_many_arguments)]
fn paint_oscillator_hud(
    painter: &egui::Painter,
    plot_rect: Rect,
    id: OscillatorPaneId,
    indicators: &IndicatorConfig,
    cache: &IndicatorCache,
    backtest_result: Option<&BacktestRunResult>,
    equity_override: Option<EquityCurveOverlay<'_>>,
    multi_strategy_overlay: Option<&MultiStrategyComparisonReport>,
    monte_carlo_fan: Option<&MonteCarloFanSeries>,
    rs_benchmark_label: Option<&str>,
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
        OscillatorPaneId::Cvd => {
            let mode_lbl = match indicators.cvd_reset_mode {
                CvdResetMode::Session => "Ses",
                CvdResetMode::Cumulative => "Cum",
            };
            let cvd_opt = cache.cvd.cvd.get(idx).copied().flatten();
            let sma_opt = cache.cvd.cvd_sma.get(idx).copied().flatten();
            let delta_opt = cache.cvd.delta.get(idx).copied().flatten();
            let fmt_opt = |o: Option<f64>| o.map_or_else(|| "—".to_owned(), format_compact_signed);
            let delta_color = match delta_opt {
                Some(v) if v >= 0.0 => palette::BULLISH,
                Some(_) => palette::BEARISH,
                None => palette::TEXT_MUTED,
            };
            job.append(
                &format!(
                    "CVD({mode_lbl}, SMA {})  {}  ",
                    indicators.cvd_sma_period,
                    fmt_opt(cvd_opt),
                ),
                0.0,
                TextFormat {
                    font_id: font.clone(),
                    color: palette::IND_CVD_LINE,
                    ..Default::default()
                },
            );
            job.append(
                &format!("SMA {}  ", fmt_opt(sma_opt)),
                0.0,
                TextFormat {
                    font_id: font.clone(),
                    color: palette::IND_CVD_SMA,
                    ..Default::default()
                },
            );
            job.append(
                &format!("Δ {}", fmt_opt(delta_opt)),
                0.0,
                TextFormat {
                    font_id: font,
                    color: delta_color,
                    ..Default::default()
                },
            );
        }
        OscillatorPaneId::RelativeStrength => {
            let rs_opt = cache.rs.rs_line.get(idx).copied().flatten();
            let sma_opt = cache.rs.rs_sma.get(idx).copied().flatten();
            let bench_name = rs_benchmark_label.unwrap_or("BENCHMARK");
            let rs_str = rs_opt.map_or_else(|| "—".to_owned(), |v| format!("{v:.2}"));
            let sma_str = sma_opt.map_or_else(|| "—".to_owned(), |v| format!("{v:.2}"));
            job.append(
                &format!(
                    "RS(vs {bench_name}, {})  {rs_str}",
                    indicators.rs_sma_period
                ),
                0.0,
                TextFormat {
                    font_id: font.clone(),
                    color: palette::IND_RS_LINE,
                    ..Default::default()
                },
            );
            job.append(
                &format!("   SMA {sma_str}"),
                0.0,
                TextFormat {
                    font_id: font.clone(),
                    color: palette::IND_RS_SMA,
                    ..Default::default()
                },
            );
            if let Some(rs_val) = rs_opt {
                let rel_pct = rs_val - 100.0;
                job.append(
                    &format!("   [{rel_pct:+.2}%]"),
                    0.0,
                    TextFormat {
                        font_id: font,
                        color: if rel_pct >= 0.0 {
                            palette::BULLISH
                        } else {
                            palette::BEARISH
                        },
                        ..Default::default()
                    },
                );
            }
        }
        OscillatorPaneId::Adx => {
            let fmt_v = |s: &[Option<f64>]| {
                s.get(idx)
                    .copied()
                    .flatten()
                    .map_or_else(|| "—".to_owned(), |v| format!("{v:.2}"))
            };
            job.append(
                &format!(
                    "ADX({}) {}   ",
                    indicators.adx_period,
                    fmt_v(&cache.adx.adx)
                ),
                0.0,
                TextFormat {
                    font_id: font.clone(),
                    color: palette::IND_ADX_LINE,
                    ..Default::default()
                },
            );
            job.append(
                &format!("+DI {}   ", fmt_v(&cache.adx.plus_di)),
                0.0,
                TextFormat {
                    font_id: font.clone(),
                    color: palette::IND_PLUS_DI,
                    ..Default::default()
                },
            );
            job.append(
                &format!("-DI {}", fmt_v(&cache.adx.minus_di)),
                0.0,
                TextFormat {
                    font_id: font,
                    color: palette::IND_MINUS_DI,
                    ..Default::default()
                },
            );
        }
        OscillatorPaneId::Stochastic => {
            let fmt_v = |s: &[Option<f64>]| {
                s.get(idx)
                    .copied()
                    .flatten()
                    .map_or_else(|| "—".to_owned(), |v| format!("{v:.2}"))
            };
            job.append(
                &format!(
                    "STOCH({},{},{})  ",
                    indicators.stoch_k_period, indicators.stoch_k_smooth, indicators.stoch_d_period
                ),
                0.0,
                TextFormat {
                    font_id: font.clone(),
                    color: palette::TEXT_PRIMARY,
                    ..Default::default()
                },
            );
            job.append(
                &format!("%K {}   ", fmt_v(&cache.stochastic.slow_k)),
                0.0,
                TextFormat {
                    font_id: font.clone(),
                    color: palette::IND_STOCH_K,
                    ..Default::default()
                },
            );
            job.append(
                &format!("%D {}", fmt_v(&cache.stochastic.slow_d)),
                0.0,
                TextFormat {
                    font_id: font,
                    color: palette::IND_STOCH_D,
                    ..Default::default()
                },
            );
        }
        OscillatorPaneId::Cci => {
            let val = cache
                .cci
                .get(idx)
                .copied()
                .flatten()
                .map_or_else(|| "—".to_owned(), |v| format!("{v:+.2}"));
            job.append(
                &format!("CCI({})  {val}", indicators.cci_period),
                0.0,
                TextFormat {
                    font_id: font,
                    color: palette::IND_CCI,
                    ..Default::default()
                },
            );
        }
        OscillatorPaneId::Mfi => {
            let val = cache
                .mfi
                .get(idx)
                .copied()
                .flatten()
                .map_or_else(|| "—".to_owned(), |v| format!("{v:.2}"));
            job.append(
                &format!("MFI({})  {val}", indicators.mfi_period),
                0.0,
                TextFormat {
                    font_id: font,
                    color: palette::IND_MFI,
                    ..Default::default()
                },
            );
        }
        OscillatorPaneId::ProxyBb => {
            let bar_pt = cache.proxy_bb.points.get(idx);
            let fmt_h = |h: BBHorizon| {
                bar_pt
                    .and_then(|p| p.horizon(h).bb_value)
                    .map_or_else(|| "—".to_owned(), |v| format!("{v:.2}"))
            };
            job.append(
                "PROXY-BB [OHLCV]  ",
                0.0,
                TextFormat {
                    font_id: font.clone(),
                    color: palette::TEXT_PRIMARY,
                    ..Default::default()
                },
            );
            job.append(
                &format!("T05 {}   ", fmt_h(BBHorizon::T05)),
                0.0,
                TextFormat {
                    font_id: font.clone(),
                    color: palette::IND_PROXY_BB_T05,
                    ..Default::default()
                },
            );
            job.append(
                &format!("T20 {}   ", fmt_h(BBHorizon::T20)),
                0.0,
                TextFormat {
                    font_id: font.clone(),
                    color: palette::IND_PROXY_BB_T20,
                    ..Default::default()
                },
            );
            job.append(
                &format!("T50 {}", fmt_h(BBHorizon::T50)),
                0.0,
                TextFormat {
                    font_id: font.clone(),
                    color: palette::IND_PROXY_BB_T50,
                    ..Default::default()
                },
            );
            if let Some(t20) = bar_pt.map(|p| p.horizon(BBHorizon::T20))
                && t20.bb_value.is_some()
            {
                let reg_col = match t20.regime {
                    crate::indicators::BBRegime::Positive => palette::BULLISH,
                    crate::indicators::BBRegime::Negative => palette::BEARISH,
                    crate::indicators::BBRegime::Neutral | crate::indicators::BBRegime::Unknown => {
                        palette::TEXT_MUTED
                    }
                };
                job.append(
                    &format!(
                        "   [{} · {} · run {}]",
                        t20.regime.code(),
                        t20.direction.code(),
                        t20.regime_run_length
                    ),
                    0.0,
                    TextFormat {
                        font_id: font,
                        color: reg_col,
                        ..Default::default()
                    },
                );
            }
        }
        OscillatorPaneId::Equity => {
            if let Some(ms) = multi_strategy_overlay {
                job.append(
                    "MULTI-STRAT [0% UTC]  ",
                    0.0,
                    TextFormat {
                        font_id: font.clone(),
                        color: palette::TEXT_PRIMARY,
                        ..Default::default()
                    },
                );
                for curve in &ms.curves {
                    let clamped_i = idx.min(curve.return_pct_values.len().saturating_sub(1));
                    let val_opt = curve
                        .return_pct_values
                        .get(clamped_i)
                        .copied()
                        .flatten()
                        .filter(|v| v.is_finite());
                    let col = multi_strategy_curve_color(curve.kind);
                    let short = match curve.kind {
                        ComparisonCurveKind::StrategyPreset(p) => p.short_label(),
                        ComparisonCurveKind::AssetBuyAndHold => "B&H",
                        ComparisonCurveKind::MarketBenchmark => {
                            ms.benchmark_label.as_deref().unwrap_or("Bench")
                        }
                    };
                    let val_txt = val_opt.map_or_else(|| "—".to_owned(), |v| format!("{v:+.2}%"));
                    let marker = if curve.is_active_preset { "★" } else { "●" };
                    job.append(
                        &format!("{marker}{short} {val_txt}  "),
                        0.0,
                        TextFormat {
                            font_id: font.clone(),
                            color: col,
                            ..Default::default()
                        },
                    );
                }
            } else if let Some(eq_ov) = equity_override {
                let base = eq_ov.baseline_cash.max(1.0);
                let clamped_idx = idx.min(eq_ov.equity_values.len().saturating_sub(1));
                let eq_opt = eq_ov
                    .equity_values
                    .get(clamped_idx)
                    .copied()
                    .flatten()
                    .filter(|v| v.is_finite());
                job.append(
                    &format!("EQUITY [{}]  ", eq_ov.title_label),
                    0.0,
                    TextFormat {
                        font_id: font.clone(),
                        color: palette::TEXT_PRIMARY,
                        ..Default::default()
                    },
                );
                if let Some(eq) = eq_opt {
                    let ret_pct = ((eq - base) / base) * 100.0;
                    let dd_pct = eq_ov
                        .underwater_values
                        .get(clamped_idx)
                        .copied()
                        .flatten()
                        .unwrap_or(0.0);
                    let ret_col = if ret_pct >= 0.0 {
                        palette::EQUITY_ABOVE_LINE
                    } else {
                        palette::EQUITY_BELOW_LINE
                    };
                    job.append(
                        &format!("{eq:.0} ({ret_pct:+.2}%)   "),
                        0.0,
                        TextFormat {
                            font_id: font.clone(),
                            color: ret_col,
                            ..Default::default()
                        },
                    );
                    job.append(
                        &format!("DD {dd_pct:.2}%"),
                        0.0,
                        TextFormat {
                            font_id: font.clone(),
                            color: if dd_pct < -0.01 {
                                palette::EQUITY_BELOW_LINE
                            } else {
                                palette::TEXT_MUTED
                            },
                            ..Default::default()
                        },
                    );
                    if let Some(split_idx) = eq_ov.split_bar_index {
                        let phase_lbl = if clamped_idx >= split_idx {
                            "   [OOS]"
                        } else {
                            "   [IS]"
                        };
                        job.append(
                            phase_lbl,
                            0.0,
                            TextFormat {
                                font_id: font.clone(),
                                color: if clamped_idx >= split_idx {
                                    palette::BACKTEST_OOS_SPLIT
                                } else {
                                    palette::TEXT_MUTED
                                },
                                ..Default::default()
                            },
                        );
                    }
                } else {
                    job.append(
                        "— (Pre-OOS / Warmup)",
                        0.0,
                        TextFormat {
                            font_id: font.clone(),
                            color: palette::TEXT_MUTED,
                            ..Default::default()
                        },
                    );
                }
                if let Some(fan) = monte_carlo_fan {
                    let fi = idx.min(fan.bar_count.saturating_sub(1));
                    if let (Some(v5), Some(v50), Some(v95)) = (
                        fan.p5.get(fi).copied().flatten(),
                        fan.p50.get(fi).copied().flatten(),
                        fan.p95.get(fi).copied().flatten(),
                    ) {
                        job.append(
                            &format!("   MC p50 {v50:.0} [{v5:.0}..{v95:.0}]"),
                            0.0,
                            TextFormat {
                                font_id: font,
                                color: palette::ACCENT_BLUE,
                                ..Default::default()
                            },
                        );
                    }
                }
            } else if let Some(bt) = backtest_result {
                let clamped_idx = idx.min(bt.equity_curve.len().saturating_sub(1));
                let pt = bt.equity_curve.get(clamped_idx);
                let base = bt.config.initial_cash.max(1.0);
                let eq = pt.map_or(base, |p| p.equity);
                let ret_pct = ((eq - base) / base) * 100.0;
                let dd_pct = pt.map_or(0.0, |p| p.underwater_pct);
                let ret_col = if ret_pct >= 0.0 {
                    palette::EQUITY_ABOVE_LINE
                } else {
                    palette::EQUITY_BELOW_LINE
                };
                job.append(
                    &format!(
                        "EQUITY [{} · {}]  ",
                        bt.config.preset.short_label(),
                        bt.config.direction_mode.short_label()
                    ),
                    0.0,
                    TextFormat {
                        font_id: font.clone(),
                        color: palette::TEXT_PRIMARY,
                        ..Default::default()
                    },
                );
                job.append(
                    &format!("{eq:.0} ({ret_pct:+.2}%)   "),
                    0.0,
                    TextFormat {
                        font_id: font.clone(),
                        color: ret_col,
                        ..Default::default()
                    },
                );
                job.append(
                    &format!("DD {dd_pct:.2}%"),
                    0.0,
                    TextFormat {
                        font_id: font.clone(),
                        color: if dd_pct < -0.01 {
                            palette::EQUITY_BELOW_LINE
                        } else {
                            palette::TEXT_MUTED
                        },
                        ..Default::default()
                    },
                );
                if let Some(split_idx) = bt.split_bar_index {
                    let phase_lbl = if clamped_idx >= split_idx {
                        "   [OOS]"
                    } else {
                        "   [IS]"
                    };
                    job.append(
                        phase_lbl,
                        0.0,
                        TextFormat {
                            font_id: font.clone(),
                            color: if clamped_idx >= split_idx {
                                palette::BACKTEST_OOS_SPLIT
                            } else {
                                palette::TEXT_MUTED
                            },
                            ..Default::default()
                        },
                    );
                }
                if let Some(fan) = monte_carlo_fan {
                    let fi = idx.min(fan.bar_count.saturating_sub(1));
                    if let (Some(v5), Some(v50), Some(v95)) = (
                        fan.p5.get(fi).copied().flatten(),
                        fan.p50.get(fi).copied().flatten(),
                        fan.p95.get(fi).copied().flatten(),
                    ) {
                        job.append(
                            &format!("   MC p50 {v50:.0} [{v5:.0}..{v95:.0}]"),
                            0.0,
                            TextFormat {
                                font_id: font,
                                color: palette::ACCENT_BLUE,
                                ..Default::default()
                            },
                        );
                    }
                }
            } else {
                job.append(
                    "EQUITY  —",
                    0.0,
                    TextFormat {
                        font_id: font,
                        color: palette::TEXT_MUTED,
                        ..Default::default()
                    },
                );
            }
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
    job.wrap = egui::text::TextWrapping {
        max_width: (max_width - 18.0).max(40.0),
        max_rows: 1,
        break_anywhere: false,
        overflow_character: Some('…'),
    };
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

#[inline]
fn compute_compare_viewport_scale(
    candles: &[Candle],
    cmp_series: &[Option<f64>],
    cmp_idx: &SeriesBlockIndex,
    strict_range: std::ops::Range<usize>,
) -> f64 {
    let n = candles.len().min(cmp_series.len());
    let s = strict_range.start.min(n);
    let e = strict_range.end.min(n);
    let scan_from = cmp_idx.first_valid_index(cmp_series, s..e).unwrap_or(s);
    for i in scan_from..e {
        let sc = candles[i].close;
        if sc.is_finite()
            && sc > 0.0
            && let Some(cv) = cmp_series[i]
            && cv.is_finite()
            && cv > 0.0
        {
            let ratio = sc / cv;
            if ratio.is_finite() && ratio > 0.0 {
                return ratio;
            }
        }
    }
    1.0
}

#[allow(clippy::too_many_arguments)]
fn refresh_main_pane_autoscale_and_bounds(
    candles: &[Candle],
    time_scale: &TimeScale,
    main_price_scale: &mut PriceScale,
    volume_price_scale: &mut PriceScale,
    show_volume: bool,
    conflation_factor: usize,
    overlays: Option<(&IndicatorConfig, &IndicatorCache)>,
    compare_overlay: Option<(&str, &[Option<f64>], &SeriesBlockIndex)>,
) -> Option<(f64, PriceBounds)> {
    let strict_range =
        non_empty_candle_range(time_scale.strict_visible_data_range(), candles.len());
    let visible_slice = &candles[strict_range.clone()];
    let base_value = visible_slice
        .first()
        .map_or_else(|| candles[0].close, |c| c.close);

    let (mut raw_min, mut raw_max, mut max_vol) = if let Some((_, cache)) = overlays {
        cache
            .candle_idx
            .autoscale_bounds(candles, strict_range.clone(), conflation_factor)
    } else {
        let mut r_min = f64::INFINITY;
        let mut r_max = f64::NEG_INFINITY;
        let mut m_vol = 0.0_f64;
        for bar in iter_conflated_candles(candles, strict_range.clone(), conflation_factor) {
            if bar.low.is_finite() {
                r_min = r_min.min(bar.low);
            }
            if bar.high.is_finite() {
                r_max = r_max.max(bar.high);
            }
            if bar.volume.is_finite() {
                m_vol = m_vol.max(bar.volume);
            }
        }
        (r_min, r_max, m_vol)
    };
    if show_volume
        && let Some((cfg, cache)) = overlays
        && cfg.show_volume_sma
        && let Some((_, vs_max)) = cache
            .volume_sma_idx
            .range_bounds(&cache.volume_sma, strict_range.clone())
        && vs_max.is_finite()
    {
        max_vol = max_vol.max(vs_max);
    }

    // Include visible main-pane overlays (`SMA`, `EMA`, `BollingerBands`, `Session VWAP`, `Compare`) in `main_price_scale` autoscale!
    if main_price_scale.is_auto_scale() {
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
        if let Some((cfg, cache)) = overlays {
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
                        .range_bounds(&cache.bbands.lower, strict_range.clone()),
                );
            }
            if cfg.show_vwap {
                merge_ov(
                    cache
                        .vwap_idx
                        .range_bounds(&cache.vwap.vwap, strict_range.clone()),
                );
                if cfg.vwap_band_1 > 0.0 {
                    merge_ov(
                        cache
                            .vwap_u1_idx
                            .range_bounds(&cache.vwap.upper_1, strict_range.clone()),
                    );
                    merge_ov(
                        cache
                            .vwap_l1_idx
                            .range_bounds(&cache.vwap.lower_1, strict_range.clone()),
                    );
                }
                if cfg.vwap_band_2 > 0.0 {
                    merge_ov(
                        cache
                            .vwap_u2_idx
                            .range_bounds(&cache.vwap.upper_2, strict_range.clone()),
                    );
                    merge_ov(
                        cache
                            .vwap_l2_idx
                            .range_bounds(&cache.vwap.lower_2, strict_range.clone()),
                    );
                }
            }
            if cfg.show_ichimoku && !cache.ichimoku.tenkan.is_empty() {
                merge_ov(
                    cache
                        .ichimoku_tenkan_idx
                        .range_bounds(&cache.ichimoku.tenkan, strict_range.clone()),
                );
                merge_ov(
                    cache
                        .ichimoku_kijun_idx
                        .range_bounds(&cache.ichimoku.kijun, strict_range.clone()),
                );
                if cfg.show_ichimoku_chikou {
                    merge_ov(
                        cache
                            .ichimoku_chikou_idx
                            .range_bounds(&cache.ichimoku.chikou, strict_range.clone()),
                    );
                }
                let kumo_range =
                    ichimoku_visible_index_range(time_scale, cache.ichimoku.senkou_a.len(), 1);
                if kumo_range.start < kumo_range.end {
                    merge_ov(
                        cache
                            .ichimoku_senkou_a_idx
                            .range_bounds(&cache.ichimoku.senkou_a, kumo_range.clone()),
                    );
                    merge_ov(
                        cache
                            .ichimoku_senkou_b_idx
                            .range_bounds(&cache.ichimoku.senkou_b, kumo_range),
                    );
                }
            }
            if cfg.show_keltner {
                merge_ov(
                    cache
                        .kc_upper_idx
                        .range_bounds(&cache.keltner.upper, strict_range.clone()),
                );
                merge_ov(
                    cache
                        .kc_lower_idx
                        .range_bounds(&cache.keltner.lower, strict_range.clone()),
                );
            }
            if cfg.show_supertrend {
                merge_ov(
                    cache
                        .supertrend_idx
                        .range_bounds(&cache.supertrend.trend, strict_range.clone()),
                );
            }
            if cfg.show_psar {
                merge_ov(
                    cache
                        .psar_idx
                        .range_bounds(&cache.psar.sar, strict_range.clone()),
                );
            }
        }
        if let Some((_, cmp_series, cmp_idx)) = compare_overlay {
            let cmp_scale =
                compute_compare_viewport_scale(candles, cmp_series, cmp_idx, strict_range.clone());
            let clamped =
                strict_range.start.min(cmp_series.len())..strict_range.end.min(cmp_series.len());
            let scaled_bounds = cmp_idx
                .range_bounds(cmp_series, clamped)
                .map(|(lo, hi)| (lo * cmp_scale, hi * cmp_scale));
            merge_ov(scaled_bounds);
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

#[allow(clippy::too_many_arguments)]
fn refresh_oscillator_autoscale(
    id: OscillatorPaneId,
    price_scale: &mut PriceScale,
    cache: &IndicatorCache,
    backtest_result: Option<&BacktestRunResult>,
    equity_override: Option<EquityCurveOverlay<'_>>,
    multi_strategy_overlay: Option<&MultiStrategyComparisonReport>,
    monte_carlo_fan: Option<&MonteCarloFanSeries>,
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
        OscillatorPaneId::Cvd => {
            let mut min_v = 0.0_f64;
            let mut max_v = 0.0_f64;
            let mut has_data = false;
            for b in [
                cache
                    .cvd_line_idx
                    .range_bounds(&cache.cvd.cvd, strict_range.clone()),
                cache
                    .cvd_sma_idx
                    .range_bounds(&cache.cvd.cvd_sma, strict_range.clone()),
                cache
                    .cvd_delta_idx
                    .range_bounds(&cache.cvd.delta, strict_range),
            ]
            .into_iter()
            .flatten()
            {
                min_v = min_v.min(b.0);
                max_v = max_v.max(b.1);
                has_data = true;
            }
            if !has_data || (max_v - min_v).abs() < 1e-9 {
                min_v = -1.0;
                max_v = 1.0;
            }
            price_scale.autoscale_raw_bounds(min_v.min(0.0), max_v.max(0.0), 1.0);
        }
        OscillatorPaneId::RelativeStrength => {
            let line_b = cache
                .rs_line_idx
                .range_bounds(&cache.rs.rs_line, strict_range.clone());
            let sma_b = cache
                .rs_sma_idx
                .range_bounds(&cache.rs.rs_sma, strict_range);
            let mut min_v = 99.5_f64;
            let mut max_v = 100.5_f64;
            let mut has_data = false;
            for (lo, hi) in [line_b, sma_b].into_iter().flatten() {
                if !has_data {
                    min_v = lo.min(100.0);
                    max_v = hi.max(100.0);
                    has_data = true;
                } else {
                    min_v = min_v.min(lo);
                    max_v = max_v.max(hi);
                }
            }
            if !has_data {
                min_v = 95.0;
                max_v = 105.0;
            }
            price_scale.autoscale_raw_bounds(min_v.min(99.5), max_v.max(100.5), 100.0);
        }
        OscillatorPaneId::Adx => {
            let mut min_v = 0.0_f64;
            let mut max_v = 50.0_f64;
            for (lo, hi) in [
                cache
                    .adx_line_idx
                    .range_bounds(&cache.adx.adx, strict_range.clone()),
                cache
                    .plus_di_idx
                    .range_bounds(&cache.adx.plus_di, strict_range.clone()),
                cache
                    .minus_di_idx
                    .range_bounds(&cache.adx.minus_di, strict_range),
            ]
            .into_iter()
            .flatten()
            {
                min_v = min_v.min(lo);
                max_v = max_v.max(hi);
            }
            price_scale.autoscale_raw_bounds(min_v.min(0.0), max_v.max(50.0), 25.0);
        }
        OscillatorPaneId::Stochastic => {
            let mut min_v = 0.0_f64;
            let mut max_v = 100.0_f64;
            for (lo, hi) in [
                cache
                    .stoch_k_idx
                    .range_bounds(&cache.stochastic.slow_k, strict_range.clone()),
                cache
                    .stoch_d_idx
                    .range_bounds(&cache.stochastic.slow_d, strict_range),
            ]
            .into_iter()
            .flatten()
            {
                min_v = min_v.min(lo);
                max_v = max_v.max(hi);
            }
            price_scale.autoscale_raw_bounds(min_v.min(0.0), max_v.max(100.0), 50.0);
        }
        OscillatorPaneId::Cci => {
            let (min_v, max_v) = cache
                .cci_idx
                .range_bounds(&cache.cci, strict_range)
                .unwrap_or((-120.0, 120.0));
            price_scale.autoscale_raw_bounds(min_v.min(-110.0), max_v.max(110.0), 1.0);
        }
        OscillatorPaneId::Mfi => {
            let (min_v, max_v) = cache
                .mfi_idx
                .range_bounds(&cache.mfi, strict_range)
                .unwrap_or((0.0, 100.0));
            price_scale.autoscale_raw_bounds(min_v.min(0.0), max_v.max(100.0), 50.0);
        }
        OscillatorPaneId::ProxyBb => {
            let mut min_v = 0.0_f64;
            let mut max_v = 100.0_f64;
            for (lo, hi) in [
                cache
                    .proxy_bb_t05_idx
                    .range_bounds(&cache.proxy_bb.t05_values, strict_range.clone()),
                cache
                    .proxy_bb_t20_idx
                    .range_bounds(&cache.proxy_bb.t20_values, strict_range.clone()),
                cache
                    .proxy_bb_t50_idx
                    .range_bounds(&cache.proxy_bb.t50_values, strict_range),
            ]
            .into_iter()
            .flatten()
            {
                min_v = min_v.min(lo);
                max_v = max_v.max(hi);
            }
            price_scale.autoscale_raw_bounds(min_v.min(0.0), max_v.max(100.0), 50.0);
        }
        OscillatorPaneId::Equity => {
            if let Some(ms) = multi_strategy_overlay {
                let (lo, hi) = ms.range_bounds(strict_range).unwrap_or((-2.0, 2.0));
                let min_v = lo.min(0.0);
                let max_v = hi.max(0.0);
                let span = (max_v - min_v).abs();
                let pad = if span < 0.5 { 1.0 } else { span * 0.05 };
                price_scale.autoscale_raw_bounds(min_v - pad, max_v + pad, 1.0);
            } else if let Some(eq_ov) = equity_override {
                let base = eq_ov.baseline_cash.max(1.0);
                let total = eq_ov.equity_values.len();
                let clamped = strict_range.start.min(total)..strict_range.end.min(total);
                let eq_bounds = eq_ov.equity_idx.range_bounds(eq_ov.equity_values, clamped);
                autoscale_equity_bounds_with_fan(
                    price_scale,
                    base,
                    eq_bounds,
                    monte_carlo_fan,
                    &strict_range,
                );
            } else if let Some(bt) = backtest_result {
                let base = bt.config.initial_cash.max(1.0);
                let total = bt.equity_values.len();
                let clamped = strict_range.start.min(total)..strict_range.end.min(total);
                let eq_bounds = bt.equity_idx.range_bounds(&bt.equity_values, clamped);
                autoscale_equity_bounds_with_fan(
                    price_scale,
                    base,
                    eq_bounds,
                    monte_carlo_fan,
                    &strict_range,
                );
            } else if price_scale.is_log() {
                price_scale.autoscale_raw_bounds(1.0, 100.0, 100.0);
            } else {
                price_scale.autoscale_raw_bounds(0.0, 100.0, 100.0);
            }
        }
    }
}

fn autoscale_equity_bounds_with_fan(
    price_scale: &mut PriceScale,
    base: f64,
    eq_bounds: Option<(f64, f64)>,
    monte_carlo_fan: Option<&MonteCarloFanSeries>,
    strict_range: &std::ops::Range<usize>,
) {
    let (raw_min, raw_max) = eq_bounds.unwrap_or((base, base));
    if price_scale.is_log() {
        let mut min_pos = base;
        let mut max_pos = base;
        let mut saw_bankrupt_tail = false;
        for v in [raw_min, raw_max] {
            if v.is_finite() {
                if v > 1.0e-4 {
                    min_pos = min_pos.min(v);
                    max_pos = max_pos.max(v);
                } else {
                    saw_bankrupt_tail = true;
                }
            }
        }
        if let Some(fan) = monte_carlo_fan {
            let ftot = fan.bar_count;
            let fclamp = strict_range.start.min(ftot)..strict_range.end.min(ftot);
            if let Some((f_lo, _)) = fan.p5_idx.range_bounds(&fan.p5, fclamp.clone())
                && f_lo.is_finite()
            {
                if f_lo > 1.0e-4 {
                    min_pos = min_pos.min(f_lo);
                } else {
                    saw_bankrupt_tail = true;
                }
            }
            if let Some((f50_lo, _)) = fan.p50_idx.range_bounds(&fan.p50, fclamp.clone())
                && f50_lo.is_finite()
                && f50_lo > 1.0e-4
            {
                min_pos = min_pos.min(f50_lo);
            }
            if let Some((_, f_hi)) = fan.p95_idx.range_bounds(&fan.p95, fclamp)
                && f_hi.is_finite()
                && f_hi > 1.0e-4
            {
                max_pos = max_pos.max(f_hi);
            }
        }
        if saw_bankrupt_tail {
            min_pos = (min_pos * 0.85).max(1.0e-4);
        }
        let pad = if (max_pos - min_pos).abs() < base * 1e-4 {
            base * 0.02
        } else {
            0.0
        };
        let lo = (min_pos - pad).max((min_pos * 0.5).max(1.0e-4));
        let hi = (max_pos + pad).max(lo * 1.001);
        price_scale.autoscale_raw_bounds(lo, hi, base);
    } else {
        let mut min_v = raw_min.min(base);
        let mut max_v = raw_max.max(base);
        if let Some(fan) = monte_carlo_fan {
            let ftot = fan.bar_count;
            let fclamp = strict_range.start.min(ftot)..strict_range.end.min(ftot);
            if let Some((f_lo, _)) = fan.p5_idx.range_bounds(&fan.p5, fclamp.clone()) {
                min_v = min_v.min(f_lo);
            }
            if let Some((_, f_hi)) = fan.p95_idx.range_bounds(&fan.p95, fclamp) {
                max_v = max_v.max(f_hi);
            }
        }
        let pad = if (max_v - min_v).abs() < base * 1e-4 {
            base * 0.02
        } else {
            0.0
        };
        price_scale.autoscale_raw_bounds(min_v - pad, max_v + pad, base);
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
            let hit_proj = (*proj).with_candles(candles);
            crate::drawings::hit_test_pane_drawings_causal(
                &store.drawings,
                pane,
                &hit_proj,
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
    let cut_x = safe_clamp_f32(
        (cx + bar_spacing * 0.5) as f32,
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

fn append_horizontal_styled_line_mesh(
    mesh: &mut Mesh,
    x0: f32,
    x1: f32,
    y: f32,
    stroke_width: f32,
    color: Color32,
    dash: LineDashStyle,
) {
    if x1 <= x0 {
        return;
    }
    let half_h = (stroke_width * 0.5).max(0.5);
    let y0 = y - half_h;
    let y1 = y + half_h;
    match dash {
        LineDashStyle::Solid => {
            mesh.add_colored_rect(
                Rect::from_min_max(Pos2::new(x0, y0), Pos2::new(x1, y1)),
                color,
            );
        }
        LineDashStyle::Dashed | LineDashStyle::Dotted => {
            let (dash_len, gap_len) = match dash {
                LineDashStyle::Dashed => (6.0_f32, 4.0_f32),
                LineDashStyle::Dotted => (2.0_f32, 3.0_f32),
                LineDashStyle::Solid => unreachable!(),
            };
            let period = dash_len + gap_len;
            let mut cur = x0;
            while cur < x1 {
                let end = (cur + dash_len).min(x1);
                mesh.add_colored_rect(
                    Rect::from_min_max(Pos2::new(cur, y0), Pos2::new(end, y1)),
                    color,
                );
                cur += period;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_trading_price_line_and_pill(
    axis_painter: &egui::Painter,
    plot_painter: &egui::Painter,
    line_mesh: &mut Mesh,
    placed_badges: &mut Vec<Rect>,
    placed_axis_pills: &mut Vec<f32>,
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
    if !y.is_finite() || y < plot_rect.top() || y > plot_rect.bottom() {
        return false;
    }

    append_horizontal_styled_line_mesh(
        line_mesh,
        plot_rect.left(),
        plot_rect.right(),
        y,
        stroke_width,
        color.gamma_multiply(0.90),
        dash,
    );

    if !inline_badge.is_empty() && plot_rect.width() > 160.0 {
        let font = FontId::monospace(10.0);
        let galley = axis_painter.layout_no_wrap(inline_badge.to_owned(), font, Color32::WHITE);
        let pad = Vec2::new(5.5, 2.5);
        let badge_w = galley.size().x + pad.x * 2.0;
        let badge_h = galley.size().y + pad.y * 2.0;
        let badge_rect = place_non_overlapping_order_badge(
            plot_rect,
            y,
            Vec2::new(badge_w, badge_h),
            placed_badges,
        );
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

    if let Some(pill_y) =
        resolve_non_overlapping_axis_pill_y(y, price_axis_rect, 9.5, placed_axis_pills)
    {
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
    }
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
        if buy_cnt > 3 && plot_rect.height() >= 18.0 {
            let dir = if inverted_scale { -1.0_f32 } else { 1.0_f32 };
            let oy = safe_clamp_f32(
                low_y + dir * (10.0 + 3.0 * 11.0),
                plot_rect.top() + 8.0,
                plot_rect.bottom() - 8.0,
            );
            painter.text(
                Pos2::new(cx, oy),
                Align2::CENTER_CENTER,
                format!("+{}", buy_cnt - 3),
                FontId::monospace(9.5),
                palette::MARKER_BUY_ARROW,
            );
        }
        if sell_cnt > 3 && plot_rect.height() >= 18.0 {
            let dir = if inverted_scale { 1.0_f32 } else { -1.0_f32 };
            let oy = safe_clamp_f32(
                high_y + dir * (10.0 + 3.0 * 11.0),
                plot_rect.top() + 8.0,
                plot_rect.bottom() - 8.0,
            );
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
                if slot < 3 && plot_rect.height() >= 18.0 {
                    // `belowBar` (`low` side), pointing inward toward the candle!
                    let outward = if inverted_scale { -1.0_f32 } else { 1.0_f32 };
                    let pad = 10.0_f32.min((0.5 * plot_rect.height() - 1.0).max(0.0));
                    let tip_y = safe_clamp_f32(
                        low_y + outward * (5.5 + (slot as f32) * 11.5),
                        plot_rect.top() + pad,
                        plot_rect.bottom() - pad,
                    );
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
                if slot < 3 && plot_rect.height() >= 18.0 {
                    // `aboveBar` (`high` side), pointing inward toward the candle!
                    let outward = if inverted_scale { 1.0_f32 } else { -1.0_f32 };
                    let pad = 10.0_f32.min((0.5 * plot_rect.height() - 1.0).max(0.0));
                    let tip_y = safe_clamp_f32(
                        high_y + outward * (5.5 + (slot as f32) * 11.5),
                        plot_rect.top() + pad,
                        plot_rect.bottom() - pad,
                    );
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
pub(crate) fn collect_trading_axis_pill_ys(
    session: &TradingSession,
    main_vp: &PaneViewport<'_>,
    plot_rect: Rect,
    price_axis_rect: Rect,
    seed_ys: &[f32],
) -> Vec<f32> {
    let mut placed_axis_pills: Vec<f32> = Vec::with_capacity(seed_ys.len() + 8);
    placed_axis_pills.extend_from_slice(seed_ys);

    let mut try_price = |price: f64| {
        if let Some(sy) = main_vp.price_to_screen_y(price) {
            let y = sy as f32;
            if y.is_finite() && y >= plot_rect.top() && y <= plot_rect.bottom() {
                let _ = resolve_non_overlapping_axis_pill_y(
                    y,
                    price_axis_rect,
                    9.5,
                    &mut placed_axis_pills,
                );
            }
        }
    };

    for order in session.pending_orders() {
        if let Some(trig_price) = order.kind.trigger_price() {
            try_price(trig_price);
        }
    }
    if let Some(pos) = session.open_position() {
        try_price(pos.avg_entry_price);
        if let Some(sl) = pos.stop_loss {
            try_price(sl);
        }
        if let Some(tp) = pos.take_profit {
            try_price(tp);
        }
    }

    placed_axis_pills
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn append_alert_axis_pill_ys(
    alert_engine: &AlertEngine,
    drawings: Option<&DrawingStore>,
    candle_len: usize,
    cutoff_timestamp: Option<i64>,
    main_vp: &PaneViewport<'_>,
    plot_rect: Rect,
    price_axis_rect: Rect,
    occluding_ys: &mut Vec<f32>,
) {
    let mut push_price = |price: f64| {
        if let Some(sy) = main_vp.price_to_screen_y(price) {
            let y = sy as f32;
            if y.is_finite()
                && y >= plot_rect.top()
                && y <= plot_rect.bottom()
                && let Some(pill_y) = clamp_axis_pill_center_y(y, price_axis_rect, 9.5)
            {
                occluding_ys.push(pill_y);
            }
        }
    };
    for rule in &alert_engine.rules {
        if !rule.is_causally_visible(cutoff_timestamp) || rule.status == AlertStatus::Paused {
            continue;
        }
        match rule.condition {
            AlertCondition::PriceLevel { price, .. } => push_price(price),
            AlertCondition::DrawingLine { drawing_id, .. } => {
                if candle_len > 0
                    && let Some(store) = drawings
                    && let Some(d) = store.drawings.iter().find(|d| {
                        d.id == drawing_id && d.is_causally_visible(cutoff_timestamp, candle_len)
                    })
                    && let Some(frontier_price) = drawing_line_value_at_bar(d, candle_len - 1)
                {
                    push_price(frontier_price);
                }
            }
            AlertCondition::RsiThreshold { .. }
            | AlertCondition::MacdHistogramZeroCross { .. }
            | AlertCondition::SignalCatalogTrigger { .. }
            | AlertCondition::ConfluenceThreshold { .. } => {}
        }
    }
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
    seed_axis_pills: &[f32],
) -> (usize, usize) {
    let mut price_lines = 0_usize;
    let mut line_mesh = Mesh::with_texture(egui::TextureId::default());
    let mut placed_badges: Vec<Rect> = Vec::with_capacity(8);
    let mut placed_axis_pills: Vec<f32> = Vec::with_capacity(seed_axis_pills.len() + 8);
    placed_axis_pills.extend_from_slice(seed_axis_pills);

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
                &mut line_mesh,
                &mut placed_badges,
                &mut placed_axis_pills,
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
            &mut line_mesh,
            &mut placed_badges,
            &mut placed_axis_pills,
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
                &mut line_mesh,
                &mut placed_badges,
                &mut placed_axis_pills,
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
                &mut line_mesh,
                &mut placed_badges,
                &mut placed_axis_pills,
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

    if !line_mesh.is_empty() {
        plot_painter.add(Shape::mesh(line_mesh));
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
fn paint_alert_price_line_and_pill(
    axis_painter: &egui::Painter,
    line_mesh: &mut Mesh,
    pending_badges: &mut Vec<(Rect, Vec2, std::sync::Arc<egui::Galley>, Color32)>,
    hud_keepout: Rect,
    toast_keepout: Option<Rect>,
    plot_rect: Rect,
    price_axis_rect: Rect,
    price: f64,
    color: Color32,
    dash: LineDashStyle,
    stroke_width: f32,
    draw_full_line: bool,
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

    if draw_full_line {
        append_horizontal_styled_line_mesh(
            line_mesh,
            plot_rect.left(),
            plot_rect.right(),
            y,
            stroke_width,
            color.gamma_multiply(0.90),
            dash,
        );
    }

    if !inline_badge.is_empty() && plot_rect.width() > 180.0 {
        let font = FontId::monospace(10.0);
        let galley = axis_painter.layout_no_wrap(inline_badge.to_owned(), font, color);
        let pad = Vec2::new(5.5, 2.0);
        let badge_w = galley.size().x + pad.x * 2.0;
        let badge_h = galley.size().y + pad.y * 2.0;
        let badge_y = safe_clamp_f32(
            y - badge_h * 0.5,
            plot_rect.top() + 2.0,
            (plot_rect.bottom() - badge_h - 2.0).max(plot_rect.top() + 2.0),
        );
        let cand_left = Rect::from_min_size(
            Pos2::new(plot_rect.left() + 10.0, badge_y),
            Vec2::new(badge_w, badge_h),
        );
        let cand_right = Rect::from_min_size(
            Pos2::new(
                (plot_rect.right() - badge_w - 10.0).max(plot_rect.left() + 10.0),
                badge_y,
            ),
            Vec2::new(badge_w, badge_h),
        );
        let clear_of_hud = |r: Rect| {
            !r.intersects(hud_keepout)
                && toast_keepout.is_none_or(|tk| !r.intersects(tk.expand(3.0)))
                && pending_badges
                    .iter()
                    .all(|(existing, _, _, _)| !r.intersects(existing.expand(2.0)))
        };
        let chosen = if clear_of_hud(cand_left) {
            Some(cand_left)
        } else if clear_of_hud(cand_right) {
            Some(cand_right)
        } else {
            None
        };
        if let Some(badge_rect) = chosen {
            pending_badges.push((badge_rect, pad, galley, color));
        }
    }

    if let Some(pill_y) = clamp_axis_pill_center_y(y, price_axis_rect, 9.5) {
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
            palette::ALERT_AXIS_TEXT_DARK,
        );
    }
    true
}

#[allow(clippy::too_many_arguments)]
fn paint_alert_trigger_markers(
    plot_painter: &egui::Painter,
    plot_rect: Rect,
    candles: &[Candle],
    culling_range: std::ops::Range<usize>,
    conflation_factor: usize,
    inverted_scale: bool,
    bar_spacing: f64,
    main_vp: &PaneViewport<'_>,
    triggers: &[AlertTriggerEvent],
    cutoff_timestamp: Option<i64>,
) -> usize {
    if triggers.is_empty() || candles.is_empty() || culling_range.is_empty() {
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

    let event_ts = |ev: &AlertTriggerEvent| -> i64 {
        if ev.eval_base_timestamp != 0 {
            ev.eval_base_timestamp
        } else {
            ev.bar_timestamp
        }
    };

    let min_ts = candles[aligned_start].timestamp.max(candles[0].timestamp);
    let max_excl_ts = (aligned_end < candles.len()).then(|| candles[aligned_end].timestamp);
    let ev_start = triggers.partition_point(|ev| event_ts(ev) < min_ts);
    let mut ev_end = match max_excl_ts {
        Some(t) => triggers.partition_point(|ev| event_ts(ev) < t),
        None => triggers.len(),
    };
    if let Some(cutoff) = cutoff_timestamp {
        let causal_end = triggers.partition_point(|ev| event_ts(ev) <= cutoff);
        ev_end = ev_end.min(causal_end);
    }
    if ev_start >= ev_end {
        return 0;
    }

    let mut candle_cursor = aligned_start;
    let mut rendered_markers = 0_usize;
    let cap = (ev_end - ev_start).min(256);
    let mut diamond_mesh = Mesh::with_texture(egui::TextureId::default());
    diamond_mesh.vertices.reserve(cap * 8);
    diamond_mesh.indices.reserve(cap * 12);
    let halo_col = Color32::from_rgba_unmultiplied(10, 14, 20, 232);

    let emit_diamond = |mesh: &mut Mesh, center: Pos2, radius: f32, fill_col: Color32| {
        let outer = radius + 1.1;
        let ob = mesh.vertices.len() as u32;
        mesh.colored_vertex(Pos2::new(center.x, center.y - outer), halo_col);
        mesh.colored_vertex(Pos2::new(center.x + outer, center.y), halo_col);
        mesh.colored_vertex(Pos2::new(center.x, center.y + outer), halo_col);
        mesh.colored_vertex(Pos2::new(center.x - outer, center.y), halo_col);
        mesh.add_triangle(ob, ob + 1, ob + 2);
        mesh.add_triangle(ob, ob + 2, ob + 3);

        let ib = mesh.vertices.len() as u32;
        mesh.colored_vertex(Pos2::new(center.x, center.y - radius), fill_col);
        mesh.colored_vertex(Pos2::new(center.x + radius, center.y), fill_col);
        mesh.colored_vertex(Pos2::new(center.x, center.y + radius), fill_col);
        mesh.colored_vertex(Pos2::new(center.x - radius, center.y), fill_col);
        mesh.add_triangle(ib, ib + 1, ib + 2);
        mesh.add_triangle(ib, ib + 2, ib + 3);
    };

    let mut cur_bucket_id: Option<usize> = None;
    let mut count_above_in_bucket = 0_usize;
    let mut count_below_in_bucket = 0_usize;
    let mut last_bucket_geom: Option<(f32, f32, f32)> = None;

    for ev in &triggers[ev_start..ev_end] {
        let ets = event_ts(ev);
        let step = candles[candle_cursor..aligned_end]
            .partition_point(|c| c.timestamp <= ets)
            .saturating_sub(1);
        candle_cursor += step;
        let bucket_id = candle_cursor / cf;

        if cur_bucket_id != Some(bucket_id) {
            cur_bucket_id = Some(bucket_id);
            count_above_in_bucket = 0;
            count_below_in_bucket = 0;

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

        let is_below = matches!(ev.direction, CrossDirection::CrossBelow);
        let slot = if is_below {
            let s = count_below_in_bucket;
            count_below_in_bucket += 1;
            s
        } else {
            let s = count_above_in_bucket;
            count_above_in_bucket += 1;
            s
        };
        let max_slots = if cf >= 8 { 1 } else { 3 };
        if slot >= max_slots {
            continue;
        }

        let radius = (bar_spacing as f32 * 0.36).clamp(4.0, 5.8);
        let anchor_y = if is_below { low_y } else { high_y };
        let outward = match (is_below, inverted_scale) {
            (true, false) | (false, true) => 1.0_f32,
            (true, true) | (false, false) => -1.0_f32,
        };
        if plot_rect.height() < 16.0 {
            continue;
        }
        let cy = safe_clamp_f32(
            anchor_y + outward * (15.0 + (slot as f32) * 11.0),
            plot_rect.top() + 8.0,
            plot_rect.bottom() - 8.0,
        );
        emit_diamond(
            &mut diamond_mesh,
            Pos2::new(cx, cy),
            radius,
            palette::ALERT_TRIGGERED_GOLD,
        );
        rendered_markers += 1;
    }

    if !diamond_mesh.is_empty() {
        plot_painter.add(Shape::mesh(diamond_mesh));
    }

    rendered_markers
}

#[allow(clippy::too_many_arguments)]
fn paint_alert_overlays(
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
    alert_engine: &AlertEngine,
    drawings: Option<&DrawingStore>,
    cutoff_timestamp: Option<i64>,
    frontier_base_timestamp: Option<i64>,
) -> (usize, usize) {
    let mut alert_lines = 0_usize;
    let mut line_mesh = Mesh::with_texture(egui::TextureId::default());
    let mut pending_badges: Vec<(Rect, Vec2, std::sync::Arc<egui::Galley>, Color32)> = Vec::new();
    let hud_keepout = Rect::from_min_size(plot_rect.min, Vec2::new(505.0, 62.0));
    let event_ts = |ev: &AlertTriggerEvent| -> i64 {
        if ev.eval_base_timestamp != 0 {
            ev.eval_base_timestamp
        } else {
            ev.bar_timestamp
        }
    };

    // Pre-layout optional frontier toast pill below the top OHLCV / Indicator HUD so inline alert badges never collide
    let frontier_toast = if let Some(last_bar) = candles.last()
        && let Some(latest_ev) = alert_engine.triggers.iter().rev().find(|ev| {
            let ets = event_ts(ev);
            let matches_frontier = match frontier_base_timestamp {
                Some(fb_ts) => ets == fb_ts,
                None => ets == last_bar.timestamp || ev.bar_timestamp == last_bar.timestamp,
            };
            matches_frontier && cutoff_timestamp.is_none_or(|cut| ets <= cut)
        })
        && plot_rect.width() > 260.0
        && plot_rect.height() > 110.0
    {
        let toast = format!(
            "ALERT FIRED · {} {}",
            latest_ev.symbol, latest_ev.condition_summary
        );
        let font = FontId::monospace(10.5);
        let galley = axis_painter.layout_no_wrap(toast, font, palette::ALERT_TRIGGERED_GOLD);
        let pad = Vec2::new(7.5, 3.5);
        let rect = Rect::from_min_size(
            Pos2::new(
                (plot_rect.right() - galley.size().x - pad.x * 2.0 - 10.0)
                    .max(plot_rect.left() + 10.0),
                plot_rect.top() + 62.0,
            ),
            galley.size() + pad * 2.0,
        );
        Some((rect, pad, galley))
    } else {
        None
    };
    let toast_keepout = frontier_toast.as_ref().map(|(r, _, _)| *r);

    for rule in &alert_engine.rules {
        if !rule.is_causally_visible(cutoff_timestamp) || rule.status == AlertStatus::Paused {
            continue;
        }
        let (color, dash, width, status_tag) = match rule.status {
            AlertStatus::Armed => (
                palette::ALERT_ARMED_AMBER,
                LineDashStyle::Dashed,
                1.25,
                "ALT",
            ),
            AlertStatus::Triggered => (
                palette::ALERT_TRIGGERED_GOLD.gamma_multiply(0.82),
                LineDashStyle::Dotted,
                1.10,
                "TRIG",
            ),
            AlertStatus::Paused => continue,
        };

        match rule.condition {
            AlertCondition::PriceLevel { price, direction } => {
                let badge = if rule.message.is_empty() {
                    format!(
                        "{status_tag} {} {}",
                        direction.symbol_glyph(),
                        price_scale.format_price(price, base_value)
                    )
                } else {
                    format!("{status_tag} · {}", rule.message)
                };
                if paint_alert_price_line_and_pill(
                    axis_painter,
                    &mut line_mesh,
                    &mut pending_badges,
                    hud_keepout,
                    toast_keepout,
                    plot_rect,
                    price_axis_rect,
                    price,
                    color,
                    dash,
                    width,
                    true,
                    &badge,
                    price_scale,
                    main_vp,
                    base_value,
                ) {
                    alert_lines += 1;
                }
            }
            AlertCondition::DrawingLine {
                drawing_id,
                direction,
            } => {
                if !candles.is_empty()
                    && let Some(store) = drawings
                    && let Some(d) = store.drawings.iter().find(|d| {
                        d.id == drawing_id && d.is_causally_visible(cutoff_timestamp, candles.len())
                    })
                    && let Some(frontier_price) = drawing_line_value_at_bar(d, candles.len() - 1)
                {
                    let badge = format!("{status_tag} D#{drawing_id} {}", direction.symbol_glyph());
                    if paint_alert_price_line_and_pill(
                        axis_painter,
                        &mut line_mesh,
                        &mut pending_badges,
                        hud_keepout,
                        toast_keepout,
                        plot_rect,
                        price_axis_rect,
                        frontier_price,
                        color,
                        dash,
                        width,
                        false,
                        &badge,
                        price_scale,
                        main_vp,
                        base_value,
                    ) {
                        alert_lines += 1;
                    }
                }
            }
            AlertCondition::RsiThreshold { .. }
            | AlertCondition::MacdHistogramZeroCross { .. }
            | AlertCondition::SignalCatalogTrigger { .. }
            | AlertCondition::ConfluenceThreshold { .. } => {}
        }
    }

    if !line_mesh.is_empty() {
        plot_painter.add(Shape::mesh(line_mesh));
    }
    for (badge_rect, pad, galley, color) in pending_badges {
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
        plot_painter.galley(badge_rect.min + pad, galley, color);
    }

    let marker_count = paint_alert_trigger_markers(
        plot_painter,
        plot_rect,
        candles,
        culling_range,
        conflation_factor,
        price_scale.is_inverted(),
        main_vp.time_scale.bar_spacing(),
        main_vp,
        &alert_engine.triggers,
        cutoff_timestamp,
    );

    if let Some((rect, pad, galley)) = frontier_toast {
        plot_painter.rect_filled(rect, 4.0, palette::BG_FLOATING_HUD);
        plot_painter.rect_stroke(
            rect,
            4.0,
            Stroke::new(1.0, palette::ALERT_ARMED_AMBER),
            StrokeKind::Inside,
        );
        plot_painter.galley(rect.min + pad, galley, palette::ALERT_TRIGGERED_GOLD);
    }

    (alert_lines, marker_count)
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
    candles: &[Candle],
    culling_range: std::ops::Range<usize>,
    conflation_factor: usize,
    pane_vp: &PaneViewport<'_>,
    mut r11_memo: Option<&mut R11RenderMemo>,
    candles_fp: u64,
) {
    let cutoff = store.replay_cutoff_timestamp();
    for drawing in &store.drawings {
        if !drawing.is_causally_visible(cutoff, candles.len()) || drawing.pane != pane {
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
            candles,
            culling_range.clone(),
            conflation_factor,
            pane_vp,
            is_selected,
            is_hovered,
            false,
            r11_memo.as_deref_mut(),
            candles_fp,
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
            candles,
            culling_range,
            conflation_factor,
            pane_vp,
            true,
            false,
            true,
            None,
            candles_fp,
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
    candles: &[Candle],
    culling_range: std::ops::Range<usize>,
    conflation_factor: usize,
    pane_vp: &PaneViewport<'_>,
    is_selected: bool,
    is_hovered: bool,
    is_preview: bool,
    r11_memo: Option<&mut R11RenderMemo>,
    candles_fp: u64,
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
                let label_y = safe_clamp_f32(
                    plot_rect.top() + (*y as f32) - 4.0,
                    plot_rect.top() + 10.0,
                    (plot_rect.bottom() - 4.0).max(plot_rect.top() + 10.0),
                );
                plot_painter.text(
                    Pos2::new(plot_rect.right() - 8.0, label_y),
                    Align2::RIGHT_BOTTOM,
                    label_text,
                    FontId::monospace(10.5),
                    base_color,
                );

                if is_selected
                    && let Some(sy) = clamp_axis_pill_center_y(
                        plot_rect.top() + (*y as f32),
                        price_axis_rect,
                        9.5,
                    )
                {
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
                let ty = safe_clamp_f32(
                    lp0.y - 2.0,
                    plot_rect.top() + 9.0,
                    (plot_rect.bottom() - 2.0).max(plot_rect.top() + 9.0),
                );
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
        ProjectedDrawingShape::RiskReward {
            is_long,
            entry_price,
            stop_price,
            target_price,
            gross_r,
            reward_fill,
            risk_fill,
            entry_line,
            target_line,
            stop_line,
            side_borders,
        } => {
            let emerald = Color32::from_rgb(38, 198, 164);
            let crimson = Color32::from_rgb(255, 92, 92);
            let entry_col = Color32::from_rgb(228, 235, 248);

            if (reward_fill.is_some() || risk_fill.is_some()) && drawing.style.fill_opacity > 0.001
            {
                let mut rr_mesh = Mesh::default();
                let op = drawing.style.fill_opacity.clamp(0.05, 0.45);
                if let Some(rf) = reward_fill {
                    let srect = Rect::from_min_max(
                        local_to_screen_pos(plot_rect, LocalPoint::new(rf.left, rf.top)),
                        local_to_screen_pos(plot_rect, LocalPoint::new(rf.right, rf.bottom)),
                    );
                    rr_mesh.add_colored_rect(
                        srect,
                        drawing_premultiplied_fill(DrawingColorPreset::Emerald, op),
                    );
                }
                if let Some(sf) = risk_fill {
                    let srect = Rect::from_min_max(
                        local_to_screen_pos(plot_rect, LocalPoint::new(sf.left, sf.top)),
                        local_to_screen_pos(plot_rect, LocalPoint::new(sf.right, sf.bottom)),
                    );
                    rr_mesh.add_colored_rect(
                        srect,
                        drawing_premultiplied_fill(DrawingColorPreset::Crimson, op),
                    );
                }
                plot_painter.add(Shape::mesh(rr_mesh));
            }

            let border_stroke = Stroke::new(
                1.0,
                Color32::from_rgba_unmultiplied(150, 168, 195, if is_selected { 145 } else { 90 }),
            );
            for side_seg in side_borders.iter().flatten() {
                let p0 = local_to_screen_pos(plot_rect, side_seg.start);
                let p1 = local_to_screen_pos(plot_rect, side_seg.end);
                plot_painter.line_segment([p0, p1], border_stroke);
            }

            for (seg_opt, col, is_entry) in [
                (*target_line, emerald, false),
                (*stop_line, crimson, false),
                (*entry_line, entry_col, true),
            ] {
                if let Some(seg) = seg_opt {
                    let p0 = local_to_screen_pos(plot_rect, seg.start);
                    let p1 = local_to_screen_pos(plot_rect, seg.end);
                    if let Some(glow) = highlight_glow {
                        plot_painter.line_segment([p0, p1], glow);
                    }
                    let w = if is_entry { stroke_w + 0.25 } else { stroke_w };
                    paint_styled_segment_phased(
                        plot_painter,
                        p0,
                        p1,
                        Pos2::new(bar0_screen_x, p0.y),
                        Stroke::new(w, col),
                        if is_entry { LineDashStyle::Solid } else { dash },
                    );
                }
            }

            // Compact non-overlapping HUD pill & price tags inside the box when wide enough (`>= 72 px`).
            if let Some(eseg) = entry_line {
                let ep0 = local_to_screen_pos(plot_rect, eseg.start);
                let ep1 = local_to_screen_pos(plot_rect, eseg.end);
                let span_w = ep1.x - ep0.x;
                if span_w >= 72.0 && *entry_price > 0.0 {
                    let tp_pct = ((*target_price - *entry_price) / *entry_price) * 100.0;
                    let sl_pct = ((*stop_price - *entry_price) / *entry_price) * 100.0;
                    let dir_txt = if *is_long { "LONG" } else { "SHORT" };
                    let summary = if span_w >= 175.0 {
                        format!("{dir_txt} {gross_r:.2}R · TP {tp_pct:+.1}% / SL {sl_pct:+.1}%")
                    } else {
                        format!("{dir_txt} {gross_r:.2}R")
                    };
                    let pill_y = safe_clamp_f32(
                        ep0.y - 9.0,
                        plot_rect.top() + 10.0,
                        (plot_rect.bottom() - 10.0).max(plot_rect.top() + 10.0),
                    );
                    let galley = plot_painter.layout_no_wrap(
                        summary,
                        FontId::monospace(10.0),
                        if *is_long { emerald } else { crimson },
                    );
                    let pill_pos = Pos2::new(ep0.x + 5.0, pill_y - galley.size().y * 0.5);
                    let bg_rect =
                        Rect::from_min_size(pill_pos, galley.size()).expand2(Vec2::new(4.0, 2.0));
                    if plot_rect.contains_rect(bg_rect) {
                        plot_painter.rect_filled(
                            bg_rect,
                            3.0,
                            Color32::from_rgba_unmultiplied(13, 17, 26, 215),
                        );
                        plot_painter.galley(pill_pos, galley, Color32::WHITE);
                    }
                }
            }

            if is_selected {
                for (val, col) in [
                    (*target_price, emerald),
                    (*stop_price, crimson),
                    (*entry_price, base_color),
                ] {
                    if let Some(ly) = price_scale.price_to_local_y(val, base_value)
                        && let Some(sy) = clamp_axis_pill_center_y(
                            plot_rect.top() + (ly as f32),
                            price_axis_rect,
                            9.5,
                        )
                    {
                        let badge_rect = Rect::from_min_max(
                            Pos2::new(price_axis_rect.left() + 2.0, sy - 8.0),
                            Pos2::new(price_axis_rect.right() - 4.0, sy + 8.0),
                        );
                        axis_painter.rect_filled(badge_rect, 3.0, col);
                        axis_painter.text(
                            Pos2::new(price_axis_rect.left() + 6.0, sy),
                            Align2::LEFT_CENTER,
                            price_scale.format_price(val, base_value),
                            FontId::monospace(10.0),
                            palette::BG_WORKSTATION,
                        );
                    }
                }
            }
        }
        ProjectedDrawingShape::FixedRangeProfile {
            raw_x_left,
            raw_x_right,
            clipped_box,
            left_border,
            right_border,
            min_bar_index,
            max_bar_index,
            row_count,
        } => {
            if let Some(box_rect) = clipped_box {
                let srect = Rect::from_min_max(
                    local_to_screen_pos(plot_rect, LocalPoint::new(box_rect.left, box_rect.top)),
                    local_to_screen_pos(
                        plot_rect,
                        LocalPoint::new(box_rect.right, box_rect.bottom),
                    ),
                );
                let fill_alpha = if is_selected { 0.09 } else { 0.05 };
                plot_painter.rect_filled(
                    srect,
                    0.0,
                    drawing_premultiplied_fill(drawing.style.color, fill_alpha),
                );
            }
            for seg in [left_border, right_border].into_iter().flatten() {
                let p0 = local_to_screen_pos(plot_rect, seg.start);
                let p1 = local_to_screen_pos(plot_rect, seg.end);
                if let Some(glow) = highlight_glow {
                    plot_painter.line_segment([p0, p1], glow);
                }
                paint_styled_segment_phased(
                    plot_painter,
                    p0,
                    p1,
                    p0,
                    Stroke::new(stroke_w.max(1.1), base_color.gamma_multiply(0.75)),
                    dash,
                );
            }

            if !candles.is_empty() && *max_bar_index >= 0.0 {
                let s = (*min_bar_index).round().max(0.0) as usize;
                let e = ((*max_bar_index).round().max(0.0) as usize).min(candles.len() - 1);
                let vp_cfg = VolumeProfileConfig {
                    row_count: (*row_count).max(8) as usize,
                    value_area_pct: 0.70,
                };
                let profile = if s <= e {
                    match r11_memo {
                        Some(memo) => memo.get_or_compute_frvp(candles, candles_fp, s, e, vp_cfg),
                        None => {
                            compute_volume_profile(&candles[s..=e], vp_cfg).map(std::sync::Arc::new)
                        }
                    }
                } else {
                    None
                };
                if let Some(profile) = profile.as_deref() {
                    let left_sx = plot_rect.left() + (*raw_x_left as f32);
                    let right_sx = plot_rect.left() + (*raw_x_right as f32);
                    if right_sx >= plot_rect.left() && left_sx <= plot_rect.right() {
                        let span_w = (right_sx - left_sx).abs().max(12.0);
                        let max_bar_w = (span_w * 0.65).clamp(18.0, 220.0);
                        let span_lo = left_sx.max(plot_rect.left());
                        let span_hi = right_sx.min(plot_rect.right());
                        let poc_txt = format!(
                            "FRVP({}) POC {}",
                            profile.bins.len(),
                            price_scale.format_price(profile.poc_price, base_value)
                        );
                        paint_volume_profile_bars(
                            plot_painter,
                            pane_vp,
                            profile,
                            left_sx,
                            max_bar_w,
                            false,
                            (span_lo, span_hi),
                            palette::VRVP_POC_GOLD,
                            palette::VRVP_VA_LINE,
                            is_selected,
                            span_w >= 92.0,
                            Some(&poc_txt),
                        );
                        if is_selected
                            && let Some(ly) =
                                price_scale.price_to_local_y(profile.poc_price, base_value)
                            && let Some(sy) = clamp_axis_pill_center_y(
                                plot_rect.top() + (ly as f32),
                                price_axis_rect,
                                9.5,
                            )
                        {
                            let badge_rect = Rect::from_min_max(
                                Pos2::new(price_axis_rect.left() + 2.0, sy - 8.5),
                                Pos2::new(price_axis_rect.right() - 4.0, sy + 8.5),
                            );
                            axis_painter.rect_filled(badge_rect, 3.0, palette::VRVP_POC_GOLD);
                            axis_painter.text(
                                Pos2::new(price_axis_rect.left() + 6.0, sy),
                                Align2::LEFT_CENTER,
                                price_scale.format_price(profile.poc_price, base_value),
                                FontId::monospace(10.0),
                                palette::BG_WORKSTATION,
                            );
                        }
                    }
                }
            }
        }
        ProjectedDrawingShape::AnchoredVwap {
            anchor_local,
            anchor_marker,
            anchor_bar_index,
            show_bands,
            ..
        } => {
            if let Some(seg) = anchor_marker {
                let p0 = local_to_screen_pos(plot_rect, seg.start);
                let p1 = local_to_screen_pos(plot_rect, seg.end);
                plot_painter
                    .line_segment([p0, p1], Stroke::new(1.2, base_color.gamma_multiply(0.6)));
            }
            if *anchor_bar_index >= -0.5 && !candles.is_empty() {
                let causal_end = candles.len();
                let a_idx = (*anchor_bar_index).round().max(0.0) as usize;
                if a_idx < causal_end {
                    let vwap_out = match r11_memo {
                        Some(memo) => memo
                            .get_or_compute_avwap(candles, candles_fp, a_idx, causal_end, 1.0, 2.0),
                        None => {
                            let tail = &candles[a_idx..causal_end];
                            std::sync::Arc::new(compute_anchored_vwap(tail, 0, 1.0, 2.0))
                        }
                    };
                    let render_end = culling_range.end.min(causal_end);
                    let visible_start = a_idx.max(culling_range.start);
                    if visible_start < render_end {
                        paint_anchored_vwap_tail(
                            plot_painter,
                            vwap_out.as_ref(),
                            a_idx,
                            (visible_start - a_idx)..(render_end - a_idx),
                            conflation_factor,
                            pane_vp,
                            *show_bands,
                            base_color,
                        );
                    }
                    let ap = local_to_screen_pos(plot_rect, *anchor_local);
                    if plot_rect.expand(8.0).contains(ap) {
                        plot_painter.circle_filled(ap, 4.5, base_color);
                        plot_painter.circle_stroke(ap, 6.2, Stroke::new(1.3, Color32::WHITE));
                        if is_selected || is_hovered {
                            let label = if *show_bands { "AVWAP ±1σ" } else { "AVWAP" };
                            plot_painter.text(
                                Pos2::new(ap.x + 8.0, (ap.y - 10.0).max(plot_rect.top() + 10.0)),
                                Align2::LEFT_CENTER,
                                label,
                                FontId::monospace(10.0),
                                base_color,
                            );
                        }
                    }
                    if is_selected
                        && let Some(last_v) = vwap_out.vwap.last().copied().flatten()
                        && let Some(ly) = price_scale.price_to_local_y(last_v, base_value)
                        && let Some(sy) = clamp_axis_pill_center_y(
                            plot_rect.top() + (ly as f32),
                            price_axis_rect,
                            9.5,
                        )
                    {
                        let badge_rect = Rect::from_min_max(
                            Pos2::new(price_axis_rect.left() + 2.0, sy - 8.5),
                            Pos2::new(price_axis_rect.right() - 4.0, sy + 8.5),
                        );
                        axis_painter.rect_filled(badge_rect, 3.0, base_color);
                        axis_painter.text(
                            Pos2::new(price_axis_rect.left() + 6.0, sy),
                            Align2::LEFT_CENTER,
                            price_scale.format_price(last_v, base_value),
                            FontId::monospace(10.0),
                            palette::BG_WORKSTATION,
                        );
                    }
                }
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
                HandlePart::Anchor(_) | HandlePart::RrEntryLeft | HandlePart::RrEntryRight => {
                    let col = if matches!(
                        handle.part,
                        HandlePart::RrEntryLeft | HandlePart::RrEntryRight
                    ) {
                        Color32::from_rgb(228, 235, 248)
                    } else {
                        base_color
                    };
                    plot_painter.circle_filled(hp, 4.6, palette::BG_CHART_CANVAS);
                    plot_painter.circle_stroke(hp, 4.6, Stroke::new(2.0, col));
                    if is_selected || is_preview {
                        plot_painter.circle_filled(hp, 1.8, Color32::WHITE);
                    }
                }
                HandlePart::RectCorner(_) | HandlePart::RrTarget | HandlePart::RrStop => {
                    let col = match handle.part {
                        HandlePart::RrTarget => Color32::from_rgb(38, 198, 164),
                        HandlePart::RrStop => Color32::from_rgb(255, 92, 92),
                        _ => base_color,
                    };
                    let sq = Rect::from_center_size(hp, Vec2::splat(8.4));
                    plot_painter.rect_filled(sq, 1.5, palette::BG_CHART_CANVAS);
                    plot_painter.rect_stroke(sq, 1.5, Stroke::new(1.8, col), StrokeKind::Inside);
                }
            }
        }
    } else if is_selected
        && projected.locked
        && let Some(&first_pt) = projected.anchors_local.first()
    {
        let lp = local_to_screen_pos(plot_rect, first_pt);
        let pad_x = 28.0_f32.min((0.5 * plot_rect.width() - 0.5).max(0.0));
        let pad_y = 12.0_f32.min((0.5 * plot_rect.height() - 0.5).max(0.0));
        let badge_pos = Pos2::new(
            safe_clamp_f32(lp.x, plot_rect.left() + pad_x, plot_rect.right() - pad_x),
            safe_clamp_f32(
                lp.y - 10.0,
                plot_rect.top() + pad_y,
                plot_rect.bottom() - pad_y,
            ),
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

    #[test]
    fn equity_baseline_pane_layout_separator_resize_and_conflated_render() {
        use crate::backtest::{
            BacktestConfig, BacktestRunOptions, StrategyDirectionMode, StrategyPresetKind,
            run_backtest_with_options,
        };
        use crate::data::Timeframe;

        let candles = generate_synthetic_ohlcv(4_000);
        let cfg = BacktestConfig {
            preset: StrategyPresetKind::SmaEmaTrendCross,
            direction_mode: StrategyDirectionMode::LongShort,
            in_sample_split_pct: Some(0.70),
            ..BacktestConfig::default()
        };
        let bt = run_backtest_with_options(
            "SYNTH",
            Timeframe::D1,
            &candles,
            &cfg,
            BacktestRunOptions::default(),
        );
        let ind = IndicatorConfig {
            show_sma: true,
            show_ema: true,
            show_bbands: false,
            show_rsi: true,
            show_macd: true,
            show_atr: true,
            ..IndicatorConfig::default()
        };
        let mut cache = IndicatorCache::new();
        cache.sync(&candles, 1, &ind);

        let mut vp = ChartViewportState::new();
        vp.time_scale.set_total_bars(candles.len());
        let mut drawings = DrawingStore::new();

        let ctx = egui::Context::default();
        let screen_rect = Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 800.0)));
        let mut rep_opt = None;
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect,
                ..Default::default()
            },
            |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    rep_opt = render_workstation_chart_full(
                        ui,
                        &candles,
                        &mut vp,
                        ChartDisplayConfig {
                            symbol: "SYNTH",
                            timeframe: "1D",
                            show_grid: true,
                            show_crosshair: true,
                            show_volume: true,
                        },
                        &ind,
                        &cache,
                        &mut drawings,
                        ChartReplayTradingOverlay {
                            backtest_result: Some(&bt),
                            show_equity_pane: true,
                            show_backtest_trades_on_chart: true,
                            focused_backtest_trade_id: bt.trades.first().map(|t| t.trade_id),
                            ..ChartReplayTradingOverlay::default()
                        },
                    );
                });
            },
        );
        out.textures_delta.clear();
        let rep = rep_opt.expect("5-pane render report");
        assert_eq!(
            rep.pane_count, 5,
            "Main + RSI + MACD + ATR + Equity must produce 5 synchronized panes"
        );
        assert!(rep.equity_pane_active);

        // Separator drag between ATR (`Some(Atr)`) and Equity (`Equity`) adjusts stretches cleanly!
        let prev_eq_stretch = vp.equity_stretch;
        vp.apply_separator_drag_delta(
            Some(OscillatorPaneId::Atr),
            OscillatorPaneId::Equity,
            120.0,
            -24.0,
        );
        assert!(vp.equity_stretch > prev_eq_stretch);

        // Full-fit (`conflation > 1`) renders 2-Mesh baseline equity + trade links cleanly!
        vp.fit_all();
        let mut fit_rep_opt = None;
        let mut fit_out = ctx.run_ui(
            egui::RawInput {
                screen_rect,
                ..Default::default()
            },
            |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    fit_rep_opt = render_workstation_chart_full(
                        ui,
                        &candles,
                        &mut vp,
                        ChartDisplayConfig {
                            symbol: "SYNTH",
                            timeframe: "1D",
                            show_grid: true,
                            show_crosshair: true,
                            show_volume: true,
                        },
                        &ind,
                        &cache,
                        &mut drawings,
                        ChartReplayTradingOverlay {
                            backtest_result: Some(&bt),
                            show_equity_pane: true,
                            show_backtest_trades_on_chart: true,
                            focused_backtest_trade_id: bt.trades.last().map(|t| t.trade_id),
                            ..ChartReplayTradingOverlay::default()
                        },
                    );
                });
            },
        );
        fit_out.textures_delta.clear();
        let fit_rep = fit_rep_opt.expect("conflated equity render report");
        assert!(fit_rep.conflation_factor >= 2);
        assert!(fit_rep.equity_pane_active);
        assert_eq!(fit_rep.rendered_backtest_trade_count, bt.trades.len());
    }

    #[test]
    fn backtest_trade_links_binary_search_culling_offscreen_span_and_replay_cutoff() {
        use crate::backtest::{
            BacktestConfig, BacktestExitReason, BacktestRunOptions, BacktestTrade,
            run_backtest_with_options,
        };
        use crate::data::Timeframe;
        use crate::trading::PositionSide;

        let candles = generate_synthetic_ohlcv(300);
        let mut bt = run_backtest_with_options(
            "SPAN",
            Timeframe::D1,
            &candles,
            &BacktestConfig::default(),
            BacktestRunOptions::default(),
        );
        // Inject two explicit trades:
        // Trade #1 spans `[10 ..= 280]` (enters far left of viewport, exits far right of viewport `[100..200]`)!
        // Trade #2 spans `[140 ..= 160]` (inside viewport, exits at bar `160`)!
        bt.trades = vec![
            BacktestTrade {
                trade_id: 1,
                side: PositionSide::Long,
                quantity: 10.0,
                entry_bar_index: 10,
                exit_bar_index: 280,
                entry_timestamp: candles[10].timestamp,
                exit_timestamp: candles[280].timestamp,
                entry_price: candles[10].open,
                exit_price: candles[280].close,
                bars_held: 270,
                gross_pnl: 500.0,
                entry_fee: 1.0,
                exit_fee: 1.0,
                exit_tax: 0.0,
                net_pnl: 498.0,
                return_pct: 5.0,
                r_multiple: Some(1.5),
                mae_pct: -1.0,
                mfe_pct: 6.0,
                exit_reason: BacktestExitReason::SignalExit,
            },
            BacktestTrade {
                trade_id: 2,
                side: PositionSide::Short,
                quantity: 10.0,
                entry_bar_index: 140,
                exit_bar_index: 160,
                entry_timestamp: candles[140].timestamp,
                exit_timestamp: candles[160].timestamp,
                entry_price: candles[140].open,
                exit_price: candles[160].close,
                bars_held: 20,
                gross_pnl: -120.0,
                entry_fee: 1.0,
                exit_fee: 1.0,
                exit_tax: 0.0,
                net_pnl: -122.0,
                return_pct: -1.2,
                r_multiple: Some(-1.0),
                mae_pct: -1.5,
                mfe_pct: 0.4,
                exit_reason: BacktestExitReason::StopLoss,
            },
        ];

        let ind = IndicatorConfig::default();
        let mut cache = IndicatorCache::new();
        cache.sync(&candles, 1, &ind);
        let mut vp = ChartViewportState::new();
        vp.time_scale.set_total_bars(candles.len());
        vp.time_scale.set_width(800.0);
        vp.time_scale.set_bar_spacing(10.0); // ~80 visible bars
        // Position viewport around bars `110..190` so Trade #1's endpoints (`10`, `280`) are BOTH off-screen!
        vp.time_scale.set_right_offset(-109.0);
        let mut drawings = DrawingStore::new();

        let ctx = egui::Context::default();
        let screen_rect = Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 640.0)));

        // 1. Live mode (`replay_cutoff_timestamp: None`): BOTH spanning Trade #1 and local Trade #2 must render!
        let mut rep_live = None;
        let mut out1 = ctx.run_ui(
            egui::RawInput {
                screen_rect,
                ..Default::default()
            },
            |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    rep_live = render_workstation_chart_full(
                        ui,
                        &candles,
                        &mut vp,
                        ChartDisplayConfig {
                            symbol: "SPAN",
                            timeframe: "1D",
                            show_grid: true,
                            show_crosshair: false,
                            show_volume: false,
                        },
                        &ind,
                        &cache,
                        &mut drawings,
                        ChartReplayTradingOverlay {
                            backtest_result: Some(&bt),
                            show_equity_pane: false,
                            show_backtest_trades_on_chart: true,
                            focused_backtest_trade_id: Some(1),
                            replay_cutoff_timestamp: None,
                            ..ChartReplayTradingOverlay::default()
                        },
                    );
                });
            },
        );
        out1.textures_delta.clear();
        assert_eq!(
            rep_live.expect("live report").rendered_backtest_trade_count,
            2,
            "off-screen-endpoint spanning trade [10..280] must still render its clipped connector across [110..190]"
        );

        // 2. Causal Replay cutoff at bar `200` (`DEF-001`): Trade #1 exits at `280 > 200`, so ONLY Trade #2 (`exit=160 <= 200`) may render!
        let mut rep_cut = None;
        let mut out2 = ctx.run_ui(
            egui::RawInput {
                screen_rect,
                ..Default::default()
            },
            |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    rep_cut = render_workstation_chart_full(
                        ui,
                        &candles,
                        &mut vp,
                        ChartDisplayConfig {
                            symbol: "SPAN",
                            timeframe: "1D",
                            show_grid: true,
                            show_crosshair: false,
                            show_volume: false,
                        },
                        &ind,
                        &cache,
                        &mut drawings,
                        ChartReplayTradingOverlay {
                            replay_active: true,
                            backtest_result: Some(&bt),
                            show_equity_pane: false,
                            show_backtest_trades_on_chart: true,
                            focused_backtest_trade_id: Some(1),
                            replay_cutoff_timestamp: Some(candles[200].timestamp),
                            ..ChartReplayTradingOverlay::default()
                        },
                    );
                });
            },
        );
        out2.textures_delta.clear();
        assert_eq!(
            rep_cut
                .expect("cutoff report")
                .rendered_backtest_trade_count,
            1,
            "DEF-001: trade exiting after replay_cutoff_timestamp must never render on chart"
        );
    }

    #[test]
    fn critic_r09_regression_equity_y_clipped_trapezoid_and_pan_invariant_trade_decimation() {
        // 1. Verify `emit_y_clipped_baseline_trapezoid` splits cleanly at `plot_top = 100.0` and `plot_bot = 300.0`,
        // keeping 100% of mesh vertices inside `[100.0, 300.0]` and intersecting `plot_top` at exact midpoint `x = 50.0`!
        let mut fill_mesh = Mesh::with_texture(egui::TextureId::default());
        emit_y_clipped_baseline_trapezoid(
            &mut fill_mesh,
            0.0,
            140.0, // inside plot
            100.0,
            60.0, // 40px above `plot_top = 100.0` -> crosses `plot_top` at `t = 0.5` (`x = 50.0`)!
            220.0, // baseline inside plot
            100.0,
            300.0,
            palette::EQUITY_ABOVE_FILL,
        );
        assert_eq!(
            fill_mesh.vertices.len(),
            8,
            "must split into 2 exact sub-trapezoids (8 vertices)"
        );
        for v in &fill_mesh.vertices {
            assert!(
                v.pos.y >= 100.0 - 1e-4 && v.pos.y <= 300.0 + 1e-4,
                "vertex y={} escaped [plot_top, plot_bot]",
                v.pos.y
            );
        }
        assert!(
            fill_mesh
                .vertices
                .iter()
                .any(|v| (v.pos.x - 50.0).abs() < 1e-3 && (v.pos.y - 100.0).abs() < 1e-3),
            "trapezoid split must intersect plot_top at exact analytical x_clip = 50.0"
        );

        // 2. Verify global-index decimation (`global_k % stride == 0`) is invariant when `first_vis` shifts by 1!
        let stride = 2_usize;
        let selected_at_pan_0: Vec<usize> = (10..260)
            .enumerate()
            .filter_map(|(local_k, trade_idx)| {
                let global_k = 10 + local_k;
                global_k.is_multiple_of(stride).then_some(trade_idx)
            })
            .collect();
        let selected_at_pan_1: Vec<usize> = (11..261)
            .enumerate()
            .filter_map(|(local_k, trade_idx)| {
                let global_k = 11 + local_k;
                global_k.is_multiple_of(stride).then_some(trade_idx)
            })
            .collect();
        // Every shared visible trade in `[11..260)` must have identical selection status (zero panning strobe!).
        for tid in 11..260 {
            assert_eq!(
                selected_at_pan_0.contains(&tid),
                selected_at_pan_1.contains(&tid),
                "trade #{tid} strobed when panning by 1 bar"
            );
        }
    }

    #[test]
    fn r11_render_memo_invalidates_on_mid_array_last_bar_and_generation_changes() {
        let build_bars = |len: usize| -> Vec<Candle> {
            (0..len)
                .map(|i| {
                    let base = 100.0 + (i as f64) * 0.15 + ((i % 7) as f64) * 0.25;
                    Candle {
                        timestamp: 1_700_000_000 + (i as i64) * 60,
                        date_label: format!("2026-01-{:02}", (i % 28) + 1),
                        open: base,
                        high: base + 1.2,
                        low: base - 0.8,
                        close: base + 0.5,
                        volume: 1_000.0 + (i as f64) * 10.0,
                    }
                })
                .collect()
        };

        let mut viewport = ChartViewportState::new();
        let mut candles = build_bars(300);
        let cfg = VolumeProfileConfig::default();
        let gen0: u64 = 1;
        let fp0 = candles_fast_fingerprint(&candles) ^ gen0.rotate_left(17);

        let v1 = viewport
            .r11_memo
            .get_or_compute_vrvp(&candles, fp0, 50..250, cfg)
            .expect("vrvp");
        let v2 = viewport
            .r11_memo
            .get_or_compute_vrvp(&candles, fp0, 50..250, cfg)
            .expect("vrvp cache hit");
        assert!(
            std::sync::Arc::ptr_eq(&v1, &v2),
            "VRVP memo must reuse Arc on unchanged frame"
        );

        let f1 = viewport
            .r11_memo
            .get_or_compute_frvp(&candles, fp0, 80, 220, cfg)
            .expect("frvp");
        let f2 = viewport
            .r11_memo
            .get_or_compute_frvp(&candles, fp0, 80, 220, cfg)
            .expect("frvp cache hit");
        assert!(
            std::sync::Arc::ptr_eq(&f1, &f2),
            "FRVP memo must reuse Arc on unchanged frame"
        );

        let a1 = viewport
            .r11_memo
            .get_or_compute_avwap(&candles, fp0, 60, 280, 1.0, 2.0);
        let a2 = viewport
            .r11_memo
            .get_or_compute_avwap(&candles, fp0, 60, 280, 1.0, 2.0);
        assert!(
            std::sync::Arc::ptr_eq(&a1, &a2),
            "AVWAP memo must reuse Arc on unchanged frame"
        );

        // Mutate an off-stride mid-array bar (index 137): all three memos must invalidate.
        candles[137].volume += 2_500_000.0;
        candles[137].close += 18.0;
        candles[137].high += 20.0;
        let fp_mid = candles_fast_fingerprint(&candles) ^ gen0.rotate_left(17);
        let v_mid = viewport
            .r11_memo
            .get_or_compute_vrvp(&candles, fp_mid, 50..250, cfg)
            .expect("vrvp recompute");
        let f_mid = viewport
            .r11_memo
            .get_or_compute_frvp(&candles, fp_mid, 80, 220, cfg)
            .expect("frvp recompute");
        let a_mid = viewport
            .r11_memo
            .get_or_compute_avwap(&candles, fp_mid, 60, 280, 1.0, 2.0);
        assert!(!std::sync::Arc::ptr_eq(&v1, &v_mid));
        assert!(!std::sync::Arc::ptr_eq(&f1, &f_mid));
        assert!(!std::sync::Arc::ptr_eq(&a1, &a_mid));

        // Mutate the last bar inside the active range (219 for FRVP, 249 for VRVP, 279 for AVWAP).
        candles[219].volume += 900_000.0;
        let fp_last_frvp = candles_fast_fingerprint(&candles) ^ gen0.rotate_left(17);
        let f_last = viewport
            .r11_memo
            .get_or_compute_frvp(&candles, fp_last_frvp, 80, 220, cfg)
            .expect("frvp last-bar recompute");
        assert!(!std::sync::Arc::ptr_eq(&f_mid, &f_last));

        candles[299].close += 5.0;
        let fp_last = candles_fast_fingerprint(&candles) ^ gen0.rotate_left(17);
        let v_last = viewport
            .r11_memo
            .get_or_compute_vrvp(&candles, fp_last, 50..250, cfg)
            .expect("vrvp tail recompute");
        let a_last = viewport
            .r11_memo
            .get_or_compute_avwap(&candles, fp_last, 60, 300, 1.0, 2.0);
        assert!(!std::sync::Arc::ptr_eq(&v_mid, &v_last));
        assert!(!std::sync::Arc::ptr_eq(&a_mid, &a_last));

        // Large dataset (> 1024 bars): bumping dataset_generation forces recompute cleanly.
        let large = build_bars(2048);
        let fp_large_g1 = candles_fast_fingerprint(&large) ^ 1_u64.rotate_left(17);
        let fp_large_g2 = candles_fast_fingerprint(&large) ^ 2_u64.rotate_left(17);
        let lg1 = viewport
            .r11_memo
            .get_or_compute_vrvp(&large, fp_large_g1, 0..2048, cfg)
            .expect("large g1");
        let lg2 = viewport
            .r11_memo
            .get_or_compute_vrvp(&large, fp_large_g2, 0..2048, cfg)
            .expect("large g2");
        assert!(!std::sync::Arc::ptr_eq(&lg1, &lg2));

        // Bounded LRU cap (<= 16 entries) + explicit clear on sanitize_deserialized / reset_to_default.
        for k in 0..24_usize {
            let _ = viewport
                .r11_memo
                .get_or_compute_frvp(&candles, fp_last, k, k + 40, cfg);
        }
        assert!(viewport.r11_memo.frvp.len() <= MAX_R11_DRAWING_MEMO_ENTRIES);

        viewport.sanitize_deserialized();
        assert!(viewport.r11_memo.vrvp.is_none());
        assert!(viewport.r11_memo.frvp.is_empty());
        assert!(viewport.r11_memo.avwap.is_empty());

        let _ = viewport
            .r11_memo
            .get_or_compute_avwap(&candles, fp_last, 60, 300, 1.0, 2.0);
        assert!(!viewport.r11_memo.avwap.is_empty());
        viewport.reset_to_default();
        assert!(viewport.r11_memo.avwap.is_empty());
    }

    fn collect_vwap_paint_shapes(
        vwap: &VwapBandsOutput,
        n_bars: usize,
        conflation_factor: usize,
        min_price: f64,
        max_price: f64,
    ) -> (Vec<egui::Shape>, f32, f32) {
        let ctx = egui::Context::default();
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 400.0));
        let mut ts = crate::viewport::TimeScale::with_data(
            n_bars,
            f64::from(rect.width()),
            crate::viewport::HorzScaleOptions::default(),
        );
        ts.set_bar_spacing((f64::from(rect.width()) / (n_bars as f64)).max(1.0));
        ts.set_right_offset(0.0);
        let mut ps = crate::viewport::PriceScale::new(
            "right",
            crate::viewport::PriceScaleOptions::default(),
        );
        ps.set_height(f64::from(rect.height()));
        ps.set_auto_scale(false);
        ps.set_raw_price_range(min_price, max_price, min_price);
        let vp = PaneViewport::new(&ts, &ps, PlotRect::from_egui(rect), min_price);

        let mut full_out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(rect),
                ..Default::default()
            },
            |ui| {
                let painter = ui.painter_at(rect);
                paint_vwap_output(
                    &painter,
                    vwap,
                    None,
                    0..n_bars,
                    conflation_factor,
                    &vp,
                    true,
                    true,
                    palette::IND_VWAP_LINE,
                );
            },
        );
        full_out.textures_delta.clear();
        let x_before = vp.float_index_to_screen_x(31.0).unwrap() as f32;
        let x_after = vp.float_index_to_screen_x(32.0).unwrap() as f32;
        (
            full_out.shapes.into_iter().map(|cs| cs.shape).collect(),
            x_before,
            x_after,
        )
    }

    fn assert_no_session_bridge_in_shape(shape: &egui::Shape, split_y_low: f32, split_y_high: f32) {
        match shape {
            egui::Shape::Vec(children) => {
                for ch in children {
                    assert_no_session_bridge_in_shape(ch, split_y_low, split_y_high);
                }
            }
            egui::Shape::LineSegment { points, .. } => {
                let min_y = points[0].y.min(points[1].y);
                let max_y = points[0].y.max(points[1].y);
                assert!(
                    !(min_y < split_y_low && max_y > split_y_high),
                    "LineSegment bridged overnight VWAP reset: {points:?}"
                );
            }
            egui::Shape::Path(path) => {
                for seg in path.points.windows(2) {
                    let min_y = seg[0].y.min(seg[1].y);
                    let max_y = seg[0].y.max(seg[1].y);
                    assert!(
                        !(min_y < split_y_low && max_y > split_y_high),
                        "Path bridged overnight VWAP reset: {seg:?}"
                    );
                }
            }
            egui::Shape::Mesh(mesh) => {
                for &tri in mesh.indices.as_chunks::<3>().0 {
                    let p0 = mesh.vertices[tri[0] as usize].pos;
                    let p1 = mesh.vertices[tri[1] as usize].pos;
                    let p2 = mesh.vertices[tri[2] as usize].pos;
                    let min_y = p0.y.min(p1.y).min(p2.y);
                    let max_y = p0.y.max(p1.y).max(p2.y);
                    assert!(
                        !(min_y < split_y_low && max_y > split_y_high),
                        "Mesh triangle bridged overnight VWAP reset: min_y={min_y} max_y={max_y}"
                    );
                }
            }
            _ => {}
        }
    }

    #[test]
    fn critic_r11_regression_vwap_zoomed_out_session_breaks() {
        // B1: Session resets (`0..32` near 100.0 vs `32..64` near 500.0) must NEVER bridge at conflation 4x or 16x.
        let mut vwap = VwapBandsOutput {
            vwap: vec![Some(100.0); 32],
            upper_1: vec![Some(105.0); 32],
            lower_1: vec![Some(95.0); 32],
            upper_2: vec![Some(110.0); 32],
            lower_2: vec![Some(90.0); 32],
            session_starts: vec![0, 32],
        };
        vwap.vwap.extend(vec![Some(500.0); 32]);
        vwap.upper_1.extend(vec![Some(505.0); 32]);
        vwap.lower_1.extend(vec![Some(495.0); 32]);
        vwap.upper_2.extend(vec![Some(510.0); 32]);
        vwap.lower_2.extend(vec![Some(490.0); 32]);

        for conflation in [4_usize, 16_usize] {
            let (shapes, _xb, _xa) = collect_vwap_paint_shapes(&vwap, 64, conflation, 50.0, 550.0);
            assert!(!shapes.is_empty());
            for sh in &shapes {
                assert_no_session_bridge_in_shape(sh, 150.0, 260.0);
            }
        }
    }

    #[test]
    fn critic_r11_regression_r11_memo_no_xor_cancel_and_avwap_pan_invariance() {
        // M1 (a): `r11_subslice_seed` prevents XOR self-cancellation (`fp ^ fp == 0`) on full `0..len` slices.
        let make_candles = |offset: f64| -> Vec<Candle> {
            (0..60)
                .map(|i| {
                    let b = offset + (i as f64) * 0.5;
                    Candle {
                        timestamp: 1_700_000_000 + (i as i64) * 3_600,
                        date_label: format!("c{i}"),
                        open: b,
                        high: b + 1.0,
                        low: b - 1.0,
                        close: b + 0.25,
                        volume: 15_000.0 + (i as f64) * 100.0,
                    }
                })
                .collect()
        };
        let ds_a = make_candles(100.0);
        let ds_b = make_candles(320.0);
        let mut memo = R11RenderMemo::default();
        let cfg = VolumeProfileConfig {
            row_count: 16,
            value_area_pct: 0.70,
        };
        let va = memo
            .get_or_compute_vrvp(&ds_a, candles_fast_fingerprint(&ds_a), 0..ds_a.len(), cfg)
            .unwrap();
        let vb = memo
            .get_or_compute_vrvp(&ds_b, candles_fast_fingerprint(&ds_b), 0..ds_b.len(), cfg)
            .unwrap();
        assert!(!std::sync::Arc::ptr_eq(&va, &vb));

        let fa = memo
            .get_or_compute_frvp(
                &ds_a,
                candles_fast_fingerprint(&ds_a),
                0,
                ds_a.len() - 1,
                cfg,
            )
            .unwrap();
        let fb = memo
            .get_or_compute_frvp(
                &ds_b,
                candles_fast_fingerprint(&ds_b),
                0,
                ds_b.len() - 1,
                cfg,
            )
            .unwrap();
        assert!(!std::sync::Arc::ptr_eq(&fa, &fb));

        let fp_a = candles_fast_fingerprint(&ds_a);
        let fp_b = candles_fast_fingerprint(&ds_b);
        let aa = memo.get_or_compute_avwap(&ds_a, fp_a, 0, ds_a.len(), 1.0, 2.0);
        let ab = memo.get_or_compute_avwap(&ds_b, fp_b, 0, ds_b.len(), 1.0, 2.0);
        assert!(!std::sync::Arc::ptr_eq(&aa, &ab));

        // M1 (b): `candles_fast_fingerprint` detects mutation at `n - 2` even when `n > 2048`.
        let mut big_a = make_candles(100.0);
        while big_a.len() < 2600 {
            let next_i = big_a.len();
            big_a.push(Candle {
                timestamp: 1_700_000_000 + (next_i as i64) * 60,
                date_label: format!("big{next_i}"),
                open: 150.0,
                high: 151.0,
                low: 149.0,
                close: 150.0,
                volume: 5000.0,
            });
        }
        let mut big_b = big_a.clone();
        let n2 = big_b.len() - 2;
        big_b[n2].close += 25.0;
        assert_ne!(
            candles_fast_fingerprint(&big_a),
            candles_fast_fingerprint(&big_b)
        );

        // M1 (c): Keeping `causal_end = candles.len()` ensures AVWAP memo hits during panning while range slices shrink.
        let aa_again = memo.get_or_compute_avwap(&ds_a, fp_a, 0, ds_a.len(), 1.0, 2.0);
        assert!(std::sync::Arc::ptr_eq(
            &memo.get_or_compute_avwap(&ds_a, fp_a, 0, ds_a.len(), 1.0, 2.0),
            &aa_again
        ));

        // M2: `paint_anchored_vwap_tail` must keep global bucket centers invariant on pan, split None gaps,
        // and render visible geometry even for a single-bar tail (`local_range = 0..1`).
        let mut gapped = VwapBandsOutput {
            vwap: (0..40)
                .map(|i| {
                    if (14..18).contains(&i) {
                        None
                    } else {
                        Some(if i < 14 { 100.0 } else { 300.0 })
                    }
                })
                .collect(),
            upper_1: (0..40)
                .map(|i| {
                    if (14..18).contains(&i) {
                        None
                    } else {
                        Some(if i < 14 { 104.0 } else { 304.0 })
                    }
                })
                .collect(),
            lower_1: (0..40)
                .map(|i| {
                    if (14..18).contains(&i) {
                        None
                    } else {
                        Some(if i < 14 { 96.0 } else { 296.0 })
                    }
                })
                .collect(),
            upper_2: vec![None; 40],
            lower_2: vec![None; 40],
            session_starts: Vec::new(),
        };
        let ctx = egui::Context::default();
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 400.0));
        let mut ts = crate::viewport::TimeScale::with_data(
            60,
            800.0,
            crate::viewport::HorzScaleOptions::default(),
        );
        ts.set_bar_spacing(12.0);
        ts.set_right_offset(0.0);
        let mut ps = crate::viewport::PriceScale::new(
            "right",
            crate::viewport::PriceScaleOptions::default(),
        );
        ps.set_height(400.0);
        ps.set_auto_scale(false);
        ps.set_raw_price_range(80.0, 320.0, 100.0);
        let vp = PaneViewport::new(&ts, &ps, PlotRect::from_egui(rect), 100.0);

        let extract_xs = |local_s: usize| -> Vec<i32> {
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(rect),
                    ..Default::default()
                },
                |ui| {
                    let p = ui.painter_at(rect);
                    paint_anchored_vwap_tail(
                        &p,
                        &gapped,
                        10,
                        local_s..40,
                        4,
                        &vp,
                        true,
                        palette::IND_VWAP_LINE,
                    );
                },
            );
            out.textures_delta.clear();
            for cs in &out.shapes {
                assert_no_session_bridge_in_shape(&cs.shape, 120.0, 260.0);
            }
            let mut xs = Vec::new();
            for cs in &out.shapes {
                if let egui::Shape::Path(path) = &cs.shape {
                    for pt in &path.points {
                        xs.push((pt.x * 100.0).round() as i32);
                    }
                }
            }
            xs
        };
        let xs_0 = extract_xs(0);
        let xs_5 = extract_xs(5); // panned past bar 0..4; shared bucket `>= 20` must align identically
        assert!(!xs_0.is_empty());
        assert!(
            xs_5.iter().all(|x| xs_0.contains(x)),
            "AVWAP bucket X shifted on pan: xs_0={xs_0:?} xs_5={xs_5:?}"
        );

        // Single-bar AVWAP tail must emit a visible shape (not silently skipped!).
        gapped.vwap = vec![Some(200.0)];
        gapped.upper_1 = vec![Some(200.0)];
        gapped.lower_1 = vec![Some(200.0)];
        let mut single_out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(rect),
                ..Default::default()
            },
            |ui| {
                let p = ui.painter_at(rect);
                paint_anchored_vwap_tail(
                    &p,
                    &gapped,
                    10,
                    0..1,
                    1,
                    &vp,
                    true,
                    palette::IND_VWAP_LINE,
                );
            },
        );
        single_out.textures_delta.clear();
        assert!(
            !single_out.shapes.is_empty(),
            "single-bar AVWAP must render visible point"
        );
    }

    #[test]
    fn r12_kumo_twist_triangle_splitting_and_headless_ichimoku_cvd_render() {
        let mut mesh = Mesh::default();
        // Crossing segment: Senkou A rises from 90->110 (`ya0` below -> `ya1` above in screen Y),
        // Senkou B flat at 100 -> exact twist crossover at `t = 0.5`!
        emit_ichimoku_kumo_segment_triangles(
            &mut mesh,
            100.0,
            200.0, // ya0 (screen Y, lower price 90)
            150.0, // yb0 (screen Y, price 100)
            90.0,  // a0 < b0 (bearish left)
            100.0,
            200.0,
            100.0, // ya1 (screen Y, higher price 110)
            150.0, // yb1
            110.0, // a1 > b1 (bullish right)
            100.0,
            (0.0, 500.0),
        );
        assert_eq!(
            mesh.indices.len(),
            6,
            "twist segment must emit 2 sub-triangles"
        );
        assert_eq!(mesh.vertices[0].color, palette::IND_KUMO_BEAR_FILL);
        assert_eq!(mesh.vertices[3].color, palette::IND_KUMO_BULL_FILL);
        assert!((mesh.vertices[2].pos.x - 150.0).abs() < 1e-4);
        assert!((mesh.vertices[2].pos.y - 150.0).abs() < 1e-4);

        // Non-crossing bullish segment emits 2 triangles filling quad cleanly
        let mut bull_mesh = Mesh::default();
        emit_ichimoku_kumo_segment_triangles(
            &mut bull_mesh,
            100.0,
            120.0,
            180.0,
            110.0,
            95.0,
            200.0,
            115.0,
            175.0,
            112.0,
            97.0,
            (0.0, 500.0),
        );
        assert_eq!(bull_mesh.indices.len(), 6);
        assert!(
            bull_mesh
                .vertices
                .iter()
                .all(|v| v.color == palette::IND_KUMO_BULL_FILL)
        );

        // Full headless chart render with Ichimoku +26 right whitespace & CVD sub-pane
        let candles = generate_synthetic_ohlcv(140);
        let cfg = IndicatorConfig {
            show_ichimoku: true,
            show_cvd: true,
            show_rsi: false,
            show_macd: false,
            show_atr: false,
            ..IndicatorConfig::default()
        };
        let mut cache = IndicatorCache::new();
        cache.sync(&candles, 1, &cfg);

        let mut vp = ChartViewportState::new();
        vp.time_scale.set_width(820.0);
        vp.time_scale.set_bar_spacing(7.5);
        vp.time_scale.set_right_offset(28.0);
        let mut drawings = DrawingStore::new();
        let ctx = egui::Context::default();
        let mut rep = None;
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(920.0, 620.0))),
                ..Default::default()
            },
            |ui| {
                rep = render_workstation_chart_full(
                    ui,
                    &candles,
                    &mut vp,
                    ChartDisplayConfig {
                        symbol: "VNINDEX",
                        timeframe: "1D",
                        show_grid: true,
                        show_crosshair: true,
                        show_volume: true,
                    },
                    &cfg,
                    &cache,
                    &mut drawings,
                    ChartReplayTradingOverlay {
                        external_crosshair_timestamp: Some(candles[candles.len() - 10].timestamp),
                        ..ChartReplayTradingOverlay::default()
                    },
                );
            },
        );
        out.textures_delta.clear();
        let rep = rep.expect("chart render report");
        assert!(rep.ichimoku_active);
        assert!(rep.cvd_pane_active);
        assert!(rep.pane_count >= 2);
    }

    #[test]
    fn critic_r12_regression_linked_crosshair_causal_cutoff() {
        let t0 = 1_750_032_000_i64;
        let mut daily = Vec::new();
        for i in 0..5 {
            daily.push(Candle {
                timestamp: t0 + (i as i64) * 86_400,
                date_label: format!("d{i}"),
                open: 100.0 + (i as f64),
                high: if i == 4 { 9_999.0 } else { 102.0 + (i as f64) },
                low: 99.0 + (i as f64),
                close: if i == 4 { 9_888.0 } else { 101.0 + (i as f64) },
                volume: 10_000.0,
            });
        }
        let fc = t0 + 4 * 86_400 + 3_600;
        let ext_ts = t0 + 4 * 86_400 + 7_200;
        assert_eq!(
            resolve_linked_crosshair_bar(&daily, 86_400, ext_ts, Some(fc)),
            Some(3),
            "unclosed daily bar #4 must be rejected under DEF-001 cutoff"
        );
        assert_eq!(
            resolve_linked_crosshair_bar(&daily[4..], 86_400, ext_ts, Some(fc)),
            None,
            "single unclosed daily bar must resolve to None"
        );

        let cfg = IndicatorConfig::default();
        let mut cache = IndicatorCache::new();
        cache.sync(&daily, 1, &cfg);
        let mut vp = ChartViewportState::new();
        let mut drawings = DrawingStore::new();
        let ctx = egui::Context::default();
        let mut rep = None;
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 500.0))),
                ..Default::default()
            },
            |ui| {
                rep = render_workstation_chart_full(
                    ui,
                    &daily,
                    &mut vp,
                    ChartDisplayConfig {
                        symbol: "FPT",
                        timeframe: "1D",
                        show_grid: true,
                        show_crosshair: true,
                        show_volume: false,
                    },
                    &cfg,
                    &cache,
                    &mut drawings,
                    ChartReplayTradingOverlay {
                        replay_active: true,
                        causal_close_cap: Some(fc),
                        external_crosshair_timestamp: Some(ext_ts),
                        ..ChartReplayTradingOverlay::default()
                    },
                );
            },
        );
        out.textures_delta.clear();
        let rep = rep.expect("render report");
        assert_eq!(
            rep.inspected_data_index, 3,
            "ChartRenderer must pass causal_close_cap so unclosed bar #4 is not inspected"
        );
    }

    #[test]
    fn critic_r12_regression_kumo_twist_unclamped_split_and_no_bowtie() {
        // Offscreen vertical zoom: ya1 = -9000.0 (unclamped) -> exact t = 50 / 9200, not clamped 50 / 200
        let mut offscreen_mesh = Mesh::default();
        emit_ichimoku_kumo_segment_triangles(
            &mut offscreen_mesh,
            100.0,
            200.0,
            150.0,
            90.0,
            100.0,
            200.0,
            -9000.0,
            150.0,
            500.0,
            100.0,
            (0.0, 500.0),
        );
        assert_eq!(offscreen_mesh.indices.len(), 6);
        let expected_xm = 100.0 + (50.0_f32 / 9200.0) * 100.0;
        assert!(
            (offscreen_mesh.vertices[2].pos.x - expected_xm).abs() < 1e-3,
            "expected unclamped crossover xm={expected_xm}, got {}",
            offscreen_mesh.vertices[2].pos.x
        );
        assert!(
            offscreen_mesh
                .vertices
                .iter()
                .all(|v| v.pos.y >= 0.0 && v.pos.y <= 500.0),
            "emitted vertices must remain clamped inside plot rect"
        );

        // Shallow sub-pixel crossing where |(ya0-yb0) - (ya1-yb1)| = 5e-7 <= 1e-6: must split, never bowtie!
        let mut shallow_mesh = Mesh::default();
        emit_ichimoku_kumo_segment_triangles(
            &mut shallow_mesh,
            100.0,
            150.0000003,
            150.0,
            99.9999,
            100.0,
            200.0,
            149.9999998,
            150.0,
            100.0001,
            100.0,
            (0.0, 500.0),
        );
        assert_eq!(shallow_mesh.indices.len(), 6);
        assert_eq!(
            shallow_mesh.vertices[0].color,
            palette::IND_KUMO_BEAR_FILL,
            "left half must stay bearish"
        );
        assert_eq!(
            shallow_mesh.vertices[3].color,
            palette::IND_KUMO_BULL_FILL,
            "right half must split cleanly to bullish"
        );
        assert!(
            (shallow_mesh.vertices[2].pos.x - shallow_mesh.vertices[3].pos.x).abs() < 1e-4,
            "split vertex must be shared cleanly"
        );
    }

    #[test]
    fn critic_r12_regression_kumo_conflated_respects_nan_gaps() {
        let mut candles = generate_synthetic_ohlcv(1_200);
        for c in &mut candles[500..600] {
            c.high = f64::NAN;
            c.low = f64::NAN;
            c.close = f64::NAN;
        }
        let cfg = crate::indicators::IchimokuConfig::default();
        let ichi = crate::indicators::compute_ichimoku(&candles, cfg);
        let first_gap = (0..candles.len())
            .find(|&i| i >= 500 && ichi.senkou_a[i].is_none() && ichi.senkou_b[i].is_none())
            .expect("gap start");
        let after_gap = (first_gap..ichi.senkou_a.len())
            .find(|&i| ichi.senkou_a[i].is_some() && ichi.senkou_b[i].is_some())
            .expect("post-gap resume");
        assert!(
            after_gap - first_gap >= 24,
            "need wide Kumo None gap for conflation test ({first_gap}..{after_gap})"
        );

        let ind_cfg = IndicatorConfig {
            show_ichimoku: true,
            show_rsi: false,
            show_macd: false,
            show_atr: false,
            ..IndicatorConfig::default()
        };
        let mut cache = IndicatorCache::new();
        cache.sync(&candles, 1, &ind_cfg);

        let mut vp = ChartViewportState::new();
        vp.time_scale.set_width(300.0);
        vp.time_scale.set_total_bars(candles.len());
        vp.fit_all();
        let mut drawings = DrawingStore::new();
        let ctx = egui::Context::default();
        let mut rep = None;
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(400.0, 400.0))),
                ..Default::default()
            },
            |ui| {
                rep = render_workstation_chart_full(
                    ui,
                    &candles,
                    &mut vp,
                    ChartDisplayConfig {
                        symbol: "VNINDEX",
                        timeframe: "1D",
                        show_grid: false,
                        show_crosshair: false,
                        show_volume: false,
                    },
                    &ind_cfg,
                    &cache,
                    &mut drawings,
                    ChartReplayTradingOverlay::default(),
                );
            },
        );
        out.textures_delta.clear();
        let rep = rep.expect("conflated gap render report");
        assert!(
            rep.conflation_factor >= 2,
            "expected conflated step >= 2, got {}",
            rep.conflation_factor
        );

        let x_void_left = vp
            .time_scale
            .index_to_local_x((first_gap + rep.conflation_factor) as i64)
            .expect("gap left x") as f32
            + rep.layout.plot_rect.left();
        let x_void_right = vp
            .time_scale
            .index_to_local_x(after_gap.saturating_sub(rep.conflation_factor) as i64)
            .expect("gap right x") as f32
            + rep.layout.plot_rect.left();
        assert!(x_void_right > x_void_left + 1.0);

        fn check_no_bridge(shape: &Shape, void_l: f32, void_r: f32) {
            match shape {
                Shape::Mesh(m) => {
                    let is_kumo = m.vertices.first().is_some_and(|v| {
                        v.color == palette::IND_KUMO_BULL_FILL
                            || v.color == palette::IND_KUMO_BEAR_FILL
                    });
                    if is_kumo {
                        for tri in m.indices.as_chunks::<3>().0 {
                            let xs = [
                                m.vertices[tri[0] as usize].pos.x,
                                m.vertices[tri[1] as usize].pos.x,
                                m.vertices[tri[2] as usize].pos.x,
                            ];
                            let min_x = xs.into_iter().fold(f32::INFINITY, f32::min);
                            let max_x = xs.into_iter().fold(f32::NEG_INFINITY, f32::max);
                            assert!(
                                !(min_x < void_l && max_x > void_r),
                                "Kumo triangle [{min_x}, {max_x}] bridged across NaN void [{void_l}, {void_r}]"
                            );
                        }
                    }
                }
                Shape::Vec(children) => {
                    for ch in children {
                        check_no_bridge(ch, void_l, void_r);
                    }
                }
                _ => {}
            }
        }
        for s in &out.shapes {
            check_no_bridge(&s.shape, x_void_left, x_void_right);
        }
    }

    #[test]
    fn critic_r12_regression_cvd_session_dense_mesh_batching() {
        let rep_data = crate::data::presets::generate_intraday_vn30f_1m_report(50_000);
        let candles = &rep_data.candles;
        let cfg = IndicatorConfig {
            show_cvd: true,
            cvd_reset_mode: CvdResetMode::Session,
            show_rsi: false,
            show_macd: false,
            show_atr: false,
            ..IndicatorConfig::default()
        };
        let mut cache = IndicatorCache::new();
        cache.sync_with_session_context(candles, 1, &cfg, Timeframe::M1, 7 * 3600);
        assert!(
            cache.cvd.session_starts.len() > 100,
            "50k 1m bars must span > 100 sessions (got {})",
            cache.cvd.session_starts.len()
        );

        let mut vp = ChartViewportState::new();
        vp.time_scale.set_width(900.0);
        vp.time_scale.set_total_bars(candles.len());
        vp.fit_all();
        let mut drawings = DrawingStore::new();
        let ctx = egui::Context::default();
        let mut rep = None;
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 600.0))),
                ..Default::default()
            },
            |ui| {
                rep = render_workstation_chart_full(
                    ui,
                    candles,
                    &mut vp,
                    ChartDisplayConfig {
                        symbol: "VN30F1M",
                        timeframe: "1m",
                        show_grid: false,
                        show_crosshair: false,
                        show_volume: false,
                    },
                    &cfg,
                    &cache,
                    &mut drawings,
                    ChartReplayTradingOverlay::default(),
                );
            },
        );
        out.textures_delta.clear();
        let rep = rep.expect("cvd batch report");
        assert!(rep.conflation_factor >= 8);
        assert!(rep.cvd_pane_active);
        assert!(
            out.shapes.len() < 80,
            "Session CVD over {} sessions must batch via dense_mesh (got {} shapes)",
            cache.cvd.session_starts.len(),
            out.shapes.len()
        );
    }

    #[test]
    fn critic_r12_regression_conflated_plot_memo_invalidates_on_pan_scale_and_indicator_toggle() {
        let candles = generate_synthetic_ohlcv(5_000);
        let mut ind_cfg = IndicatorConfig {
            show_ichimoku: false,
            show_rsi: false,
            show_macd: false,
            show_atr: false,
            show_cvd: true,
            cvd_reset_mode: CvdResetMode::Cumulative,
            ..IndicatorConfig::default()
        };
        let mut cache = IndicatorCache::new();
        cache.sync(&candles, 1, &ind_cfg);

        let mut vp = ChartViewportState::new();
        vp.time_scale.set_width(500.0);
        vp.time_scale.set_total_bars(candles.len());
        vp.fit_all();
        let mut drawings = DrawingStore::new();
        let ctx = egui::Context::default();
        let screen = Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(620.0, 480.0)));

        let mut render_once = |vp: &mut ChartViewportState,
                               ind_cfg: &IndicatorConfig,
                               cache: &IndicatorCache,
                               step: f64| {
            let mut rep = None;
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: screen,
                    time: Some(step * 0.016),
                    ..Default::default()
                },
                |ui| {
                    rep = render_workstation_chart_full(
                        ui,
                        &candles,
                        vp,
                        ChartDisplayConfig {
                            symbol: "VNINDEX",
                            timeframe: "1m",
                            show_grid: false,
                            show_crosshair: false,
                            show_volume: false,
                        },
                        ind_cfg,
                        cache,
                        &mut drawings,
                        ChartReplayTradingOverlay::default(),
                    );
                },
            );
            out.textures_delta.clear();
            (rep.expect("render report"), out.shapes)
        };

        fn collect_first_mesh_signature(shapes: &[egui::epaint::ClippedShape]) -> Vec<(u32, u32)> {
            let mut pts = Vec::new();
            for cs in shapes {
                if let Shape::Mesh(m) = &cs.shape {
                    for v in m.vertices.iter().take(12) {
                        pts.push((v.pos.x.to_bits(), v.pos.y.to_bits()));
                    }
                }
            }
            pts
        }

        let (rep1, shapes1) = render_once(&mut vp, &ind_cfg, &cache, 1.0);
        assert!(rep1.conflation_factor >= 2);
        assert!(vp.r11_memo.plot_data.is_some());

        let (rep2, shapes2) = render_once(&mut vp, &ind_cfg, &cache, 2.0);
        assert_eq!(rep1.rendered_candle_count, rep2.rendered_candle_count);
        assert_eq!(shapes1.len(), shapes2.len());
        assert_eq!(
            collect_first_mesh_signature(&shapes1),
            collect_first_mesh_signature(&shapes2)
        );

        ind_cfg.show_ichimoku = true;
        cache.sync(&candles, 1, &ind_cfg);
        let (rep3, shapes3) = render_once(&mut vp, &ind_cfg, &cache, 3.0);
        assert!(rep3.ichimoku_active);
        assert!(
            shapes3.len() > shapes2.len(),
            "enabling Ichimoku must invalidate cached shapes ({} vs {})",
            shapes3.len(),
            shapes2.len()
        );
        let sig3 = collect_first_mesh_signature(&shapes3);

        vp.main_price_scale
            .set_mode(crate::viewport::PriceScaleMode::Logarithmic);
        vp.time_scale.scroll_by_bars(15.0);
        let (rep4, shapes4) = render_once(&mut vp, &ind_cfg, &cache, 4.0);
        assert!(rep4.ichimoku_active);
        let sig4 = collect_first_mesh_signature(&shapes4);
        assert_ne!(
            sig3, sig4,
            "switching to Log scale + panning must invalidate retained mesh coordinates"
        );
    }

    #[test]
    fn r13_safe_midpoint_compact_cvd_axis_and_market_profile_mesh_render() {
        // 1. Overflow/subnormal/NaN safety for `safe_midpoint_f64`
        assert_eq!(
            safe_midpoint_f64(f64::MAX, f64::MAX),
            f64::MAX,
            "large equal finite operands must never overflow to +inf"
        );
        assert_eq!(safe_midpoint_f64(-f64::MAX, -f64::MAX), -f64::MAX);
        assert!(safe_midpoint_f64(f64::MAX, f64::MAX * 0.5).is_finite());
        assert_eq!(safe_midpoint_f64(1.0e-310, 3.0e-310), 2.0e-310);
        assert!(safe_midpoint_f64(f64::NAN, 10.0).is_nan());

        // 2. Headless workstation render with Session Market Profile + Weekly VWAP + Compact CVD
        let candles = generate_synthetic_ohlcv(960);
        let ind_cfg = IndicatorConfig {
            show_sma: false,
            show_ema: false,
            show_bbands: false,
            show_vwap: true,
            vwap_anchor_mode: crate::indicators::VwapAnchorMode::Weekly,
            show_market_profile: true,
            mp_num_bins: 20,
            mp_ib_bars: 4,
            mp_extend_naked_poc: true,
            show_cvd: true,
            cvd_reset_mode: CvdResetMode::Cumulative,
            ..IndicatorConfig::default()
        };
        let mut cache = IndicatorCache::new();
        cache.sync(&candles, Timeframe::M15.seconds() as u64, &ind_cfg);

        let mut vp = ChartViewportState::new();
        vp.time_scale.set_width(720.0);
        vp.time_scale.set_total_bars(candles.len());
        vp.fit_all();
        assert_eq!(
            vp.cvd_price_scale.format_mode(),
            crate::viewport::PriceAxisFormatMode::CompactVolume
        );

        let mut drawings = DrawingStore::new();
        let ctx = egui::Context::default();
        let mut rep = None;
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(820.0, 600.0))),
                time: Some(0.016),
                ..Default::default()
            },
            |ui| {
                rep = render_workstation_chart_full(
                    ui,
                    &candles,
                    &mut vp,
                    ChartDisplayConfig {
                        symbol: "VN30F1M",
                        timeframe: "15m",
                        show_grid: true,
                        show_crosshair: false,
                        show_volume: true,
                    },
                    &ind_cfg,
                    &cache,
                    &mut drawings,
                    ChartReplayTradingOverlay::default(),
                );
            },
        );
        out.textures_delta.clear();
        let report = rep.expect("chart render report");
        assert!(report.market_profile_active);
        assert!(report.active_overlay_count >= 1);
        assert!(report.cvd_pane_active);
        // Confirm Market Profile batches into Mesh shapes rather than hundreds of per-bar Rect calls
        let mesh_count = out
            .shapes
            .iter()
            .filter(|cs| matches!(cs.shape, Shape::Mesh(_)))
            .count();
        assert!(mesh_count >= 1, "expected batched TPO/chart Mesh shapes");
    }

    #[test]
    fn critic_r13_regression_subpixel_vwap_cvd_pan_invariance_and_spike_preservation() {
        // 1. Construct a 10,000-bar series with 2,000 5-bar sessions (sub-pixel at fit_all).
        //    Session #500 has an extreme spike (`100 -> 260`), immediately next to baseline `100`.
        //    Panning fractionally MUST NEVER drop the spike or flip its rendered geometry.
        let n = 10_000_usize;
        let mut vwap = VwapBandsOutput {
            vwap: vec![Some(100.0); n],
            upper_1: vec![Some(102.0); n],
            lower_1: vec![Some(98.0); n],
            upper_2: vec![Some(104.0); n],
            lower_2: vec![Some(96.0); n],
            session_starts: (0..n).step_by(5).collect(),
        };
        for i in 2500..2505 {
            vwap.vwap[i] = Some(260.0);
        }
        let ctx = egui::Context::default();
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(600.0, 320.0));
        let render_subpixel_vwap = |right_offset: f64| -> (f32, usize) {
            let mut ts =
                TimeScale::with_data(n, f64::from(screen.width()), HorzScaleOptions::default());
            ts.set_bar_spacing(0.055);
            ts.set_right_offset(right_offset);
            let mut ps = PriceScale::new("right", crate::viewport::PriceScaleOptions::default());
            ps.set_height(f64::from(screen.height()));
            ps.set_auto_scale(false);
            ps.set_raw_price_range(80.0, 280.0, 100.0);
            let vp = PaneViewport::new(&ts, &ps, PlotRect::from_egui(screen), 100.0);
            let mut min_y = f32::INFINITY;
            let mut vert_count = 0_usize;
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(screen),
                    ..Default::default()
                },
                |ui| {
                    paint_vwap_output(
                        ui.painter(),
                        &vwap,
                        None,
                        0..n,
                        32,
                        &vp,
                        true,
                        false,
                        palette::IND_VWAP_LINE,
                    );
                },
            );
            out.textures_delta.clear();
            for cs in &out.shapes {
                if let Shape::Mesh(m) = &cs.shape {
                    vert_count += m.vertices.len();
                    for v in &m.vertices {
                        min_y = min_y.min(v.pos.y);
                    }
                }
            }
            (min_y, vert_count)
        };
        let (min_y_a, verts_a) = render_subpixel_vwap(0.0);
        let (min_y_b, verts_b) = render_subpixel_vwap(3.2);
        let spike_y = {
            let mut ps = PriceScale::new("right", crate::viewport::PriceScaleOptions::default());
            ps.set_height(f64::from(screen.height()));
            ps.set_auto_scale(false);
            ps.set_raw_price_range(80.0, 280.0, 100.0);
            let mut ts =
                TimeScale::with_data(n, f64::from(screen.width()), HorzScaleOptions::default());
            ts.set_bar_spacing(0.055);
            PaneViewport::new(&ts, &ps, PlotRect::from_egui(screen), 100.0)
                .price_to_screen_y(260.0)
                .unwrap() as f32
        };
        assert!(
            min_y_a <= spike_y + 2.0 && min_y_b <= spike_y + 2.0,
            "subpixel session spike must never be dropped ({min_y_a} / {min_y_b} vs {spike_y})"
        );
        assert_eq!(
            verts_a, verts_b,
            "pan-invariant bucket geometry under fractional pan"
        );

        // 2. Verify signed delta positive+negative cancellation guard in `paint_cvd_pane`
        let mut candles = generate_synthetic_ohlcv(256);
        for (i, c) in candles.iter_mut().enumerate() {
            c.low = 100.0;
            c.high = 105.0;
            c.volume = 500_000.0;
            if i % 2 == 0 {
                c.open = 100.0;
                c.close = 105.0;
            } else {
                c.open = 105.0;
                c.close = 100.0;
            }
        }
        let cfg = IndicatorConfig {
            show_cvd: true,
            cvd_reset_mode: CvdResetMode::Cumulative,
            ..IndicatorConfig::default()
        };
        let mut cache = IndicatorCache::new();
        cache.sync(&candles, 3_600, &cfg);
        let mut ts = TimeScale::with_data(
            candles.len(),
            f64::from(screen.width()),
            HorzScaleOptions::default(),
        );
        ts.set_bar_spacing(0.8);
        ts.set_right_offset(0.0);
        let mut ps = PriceScale::new(
            "cvd-right",
            crate::viewport::PriceScaleOptions::sumi_cvd_oscillator(),
        );
        ps.set_height(f64::from(screen.height()));
        ps.set_auto_scale(false);
        ps.set_raw_price_range(-700_000.0, 700_000.0, 1.0);
        let vp = PaneViewport::new(&ts, &ps, PlotRect::from_egui(screen), 1.0);
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            |ui| {
                paint_cvd_pane(
                    ui.painter(),
                    screen,
                    &cache,
                    0..candles.len(),
                    8,
                    &vp,
                    CandlestickBarMetrics::compute(0.8, 1.0),
                );
            },
        );
        out.textures_delta.clear();
        let mut has_above_zero = false;
        let mut has_below_zero = false;
        let zero_y = vp.price_to_screen_y(0.0).unwrap() as f32;
        for cs in &out.shapes {
            if let Shape::Mesh(m) = &cs.shape {
                for v in &m.vertices {
                    if v.pos.y < zero_y - 10.0 {
                        has_above_zero = true;
                    }
                    if v.pos.y > zero_y + 10.0 {
                        has_below_zero = true;
                    }
                }
            }
        }
        assert!(
            has_above_zero && has_below_zero,
            "conflated opposing delta bars must preserve both positive and negative extremes"
        );
    }

    #[test]
    fn critic_r14_regression_tiny_and_zero_height_axis_pill_no_panic() {
        use crate::alerts::AlertFrequency;
        use crate::trading::{AccountConfig, OrderKind, OrderRequest};

        assert!((safe_clamp_f32(5.0, 20.0, 10.0) - 15.0).abs() < 1e-6);
        let tiny_axis = Rect::from_min_size(Pos2::ZERO, Vec2::new(72.0, 8.0));
        assert_eq!(clamp_axis_pill_center_y(4.0, tiny_axis, 11.5), None);
        let ok_axis = Rect::from_min_size(Pos2::ZERO, Vec2::new(72.0, 18.0));
        assert!(clamp_axis_pill_center_y(9.0, ok_axis, 11.5).is_some());

        let candles = generate_synthetic_ohlcv(80);
        let cfg = IndicatorConfig {
            show_vwap: true,
            show_cvd: true,
            show_rsi: true,
            ..IndicatorConfig::default()
        };
        let mut cache = IndicatorCache::new();
        cache.sync(&candles, 86_400, &cfg);
        let mut drawings = DrawingStore::new();
        let mut session = TradingSession::new(AccountConfig {
            initial_cash: 1_000_000.0,
            ..AccountConfig::default()
        });
        let last_idx = candles.len() - 1;
        let mark = candles[last_idx].close;
        session
            .submit_order(
                OrderRequest {
                    side: OrderSide::Buy,
                    kind: OrderKind::Market,
                    quantity: 100.0,
                    stop_loss: Some(mark * 0.96),
                    take_profit: Some(mark * 1.05),
                },
                &candles,
                last_idx,
            )
            .expect("market fill");
        session
            .submit_order(
                OrderRequest {
                    side: OrderSide::Buy,
                    kind: OrderKind::Limit {
                        limit_price: mark * 0.99,
                    },
                    quantity: 100.0,
                    stop_loss: None,
                    take_profit: None,
                },
                &candles,
                last_idx,
            )
            .expect("working order");

        let mut alerts = AlertEngine::new();
        let _ = alerts.add_alert(
            "VCB",
            Timeframe::D1,
            AlertCondition::PriceLevel {
                price: mark * 1.01,
                direction: CrossDirection::CrossAny,
            },
            AlertFrequency::Once,
            None,
            &candles,
            last_idx,
            None,
            None,
        );

        let ctx = egui::Context::default();
        for &h in &[18.0_f32, 8.0_f32, 1.0_f32, 0.0_f32] {
            let mut vp = ChartViewportState::new();
            vp.time_scale.set_total_bars(candles.len());
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(640.0, 200.0))),
                    ..Default::default()
                },
                |ui| {
                    let area = Rect::from_min_size(Pos2::ZERO, Vec2::new(640.0, h));
                    let _ = ui.scope_builder(egui::UiBuilder::new().max_rect(area), |child| {
                        let _ = render_workstation_chart_full(
                            child,
                            &candles,
                            &mut vp,
                            ChartDisplayConfig {
                                symbol: "VCB",
                                timeframe: "1D",
                                show_grid: true,
                                show_crosshair: true,
                                show_volume: true,
                            },
                            &cfg,
                            &cache,
                            &mut drawings,
                            ChartReplayTradingOverlay {
                                trading_session: Some(&session),
                                alert_engine: Some(&alerts),
                                external_crosshair_timestamp: Some(candles[last_idx - 2].timestamp),
                                ..ChartReplayTradingOverlay::default()
                            },
                        );
                    });
                },
            );
            out.textures_delta.clear();
        }
    }

    #[test]
    fn critic_r14_regression_time_axis_tick_culled_under_cursor_pill() {
        let time_axis = Rect::from_min_max(Pos2::new(0.0, 500.0), Pos2::new(900.0, 528.0));
        let pill = compute_time_axis_cursor_pill_rect(time_axis, "2026-11-15 10:30", 420.0)
            .expect("cursor pill rect");
        let span = Some((pill.left(), pill.right()));
        assert!(time_tick_label_occluded(420.0, "10:15", span));
        assert!(time_tick_label_occluded(pill.left() + 4.0, "10:00", span));
        assert!(!time_tick_label_occluded(90.0, "09:00", span));
        assert!(!time_tick_label_occluded(810.0, "14:45", span));
    }

    #[test]
    fn critic_r14_regression_working_order_pills_no_overlap() {
        let plot_rect = Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(820.0, 480.0));
        let axis_rect = Rect::from_min_size(Pos2::new(820.0, 0.0), Vec2::new(86.0, 480.0));
        let badge_size = Vec2::new(152.0, 17.0);

        let mut placed_badges = Vec::new();
        let mut placed_axis_ys = Vec::new();
        for &raw_y in &[220.0_f32, 221.0_f32, 222.0_f32] {
            let badge =
                place_non_overlapping_order_badge(plot_rect, raw_y, badge_size, &mut placed_badges);
            assert!(plot_rect.contains(badge.center()));
            let _ =
                resolve_non_overlapping_axis_pill_y(raw_y, axis_rect, 11.5, &mut placed_axis_ys)
                    .expect("axis pill slot");
        }
        for i in 0..placed_badges.len() {
            for j in (i + 1)..placed_badges.len() {
                assert!(
                    !placed_badges[i].intersects(placed_badges[j]),
                    "order badges {i} and {j} must not intersect: {:?} vs {:?}",
                    placed_badges[i],
                    placed_badges[j]
                );
            }
        }
        assert_eq!(placed_axis_ys.len(), 3);
        for i in 0..placed_axis_ys.len() {
            for j in (i + 1)..placed_axis_ys.len() {
                assert!(
                    (placed_axis_ys[i] - placed_axis_ys[j]).abs() >= 15.0 - 1e-3,
                    "axis pills {i} and {j} overlap: {} vs {}",
                    placed_axis_ys[i],
                    placed_axis_ys[j]
                );
            }
        }
    }

    #[test]
    fn r15_r14_carryover_equity_baseline_badge_uses_vp_base_value_and_trading_pills_occlude() {
        use crate::backtest::{
            BacktestConfig, MonteCarloConfig, run_backtest, run_monte_carlo_bootstrap,
        };
        use crate::data::generate_synthetic_ohlcv;
        use crate::trading::{OrderKind, OrderRequest};
        use crate::viewport::PriceScaleMode;

        let candles = generate_synthetic_ohlcv(120);
        let bt_cfg = BacktestConfig {
            initial_cash: 100_000.0,
            ..BacktestConfig::default()
        };
        let bt = run_backtest("VNINDEX", Timeframe::D1, &candles, &bt_cfg, None);
        let mc = run_monte_carlo_bootstrap(
            bt_cfg.initial_cash,
            &bt.trades,
            &bt.equity_curve,
            candles.len(),
            None,
            MonteCarloConfig::default(),
        );

        // 1. Verify PriceScale::format_price_for_tick_density(baseline, vp.base_value, ...) outputs +0.00% and 100.00
        let mut eq_ps = PriceScale::new("eq", PriceScaleOptions::default());
        eq_ps.set_height(180.0);
        eq_ps.autoscale_raw_bounds(92_000.0, 118_000.0, 100_000.0);
        eq_ps.set_mode(PriceScaleMode::Percentage);
        assert_eq!(
            eq_ps.format_price_for_tick_density(100_000.0, 100_000.0, 24.0, 3, 6),
            "+0.00%"
        );
        eq_ps.set_mode(PriceScaleMode::IndexedTo100);
        assert_eq!(
            eq_ps.format_price_for_tick_density(100_000.0, 100_000.0, 24.0, 3, 6),
            "100.00"
        );

        // 2. Verify collect_trading_axis_pill_ys includes working Limit/Stop orders & position Entry/SL/TP
        let mut session = TradingSession::default();
        let last_idx = candles.len() - 1;
        let mark = candles[last_idx].close;
        let _ = session.submit_order(
            OrderRequest {
                side: OrderSide::Buy,
                kind: OrderKind::Market,
                quantity: 10.0,
                stop_loss: Some(mark * 0.97),
                take_profit: Some(mark * 1.03),
            },
            &candles,
            last_idx,
        );
        let _ = session.submit_order(
            OrderRequest {
                side: OrderSide::Buy,
                kind: OrderKind::Limit {
                    limit_price: mark * 0.985,
                },
                quantity: 5.0,
                stop_loss: None,
                take_profit: None,
            },
            &candles,
            last_idx,
        );

        let mut ts = TimeScale::with_data(candles.len(), 800.0, HorzScaleOptions::default());
        ts.set_bar_spacing(8.0);
        let mut ps = PriceScale::new("main", PriceScaleOptions::default());
        ps.set_height(460.0);
        ps.autoscale_raw_bounds(mark * 0.90, mark * 1.10, mark);
        let plot_rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 460.0));
        let axis_rect = Rect::from_min_size(Pos2::new(800.0, 0.0), Vec2::new(84.0, 460.0));
        let main_vp = PaneViewport::new(&ts, &ps, PlotRect::from_egui(plot_rect), mark);
        let seeds = [230.0_f32];
        let occluding =
            collect_trading_axis_pill_ys(&session, &main_vp, plot_rect, axis_rect, &seeds);
        assert_eq!(
            occluding.len(),
            5,
            "seed + 1 pending limit + 1 entry + 1 SL + 1 TP must all be registered in main_occluding_ys"
        );

        // 3. Headless render pass with Equity pane + Monte Carlo fan in Percentage mode
        let ctx = egui::Context::default();
        let mut vp = ChartViewportState::new();
        vp.time_scale.set_total_bars(candles.len());
        vp.equity_price_scale.set_mode(PriceScaleMode::Percentage);
        let ind = IndicatorConfig::default();
        let mut cache = IndicatorCache::new();
        cache.sync(&candles, 1, &ind);
        let mut drawings = DrawingStore::new();
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1024.0, 680.0))),
                ..Default::default()
            },
            |ui| {
                let rep = render_workstation_chart_full(
                    ui,
                    &candles,
                    &mut vp,
                    ChartDisplayConfig {
                        symbol: "VNINDEX",
                        timeframe: "1D",
                        show_grid: true,
                        show_crosshair: true,
                        show_volume: true,
                    },
                    &ind,
                    &cache,
                    &mut drawings,
                    ChartReplayTradingOverlay {
                        trading_session: Some(&session),
                        backtest_result: Some(&bt),
                        monte_carlo_fan: mc.bar_fan.as_ref(),
                        show_equity_pane: true,
                        ..ChartReplayTradingOverlay::default()
                    },
                )
                .expect("equity + mc fan render report");
                assert!(rep.equity_pane_active);
            },
        );
        out.textures_delta.clear();
    }

    #[test]
    fn critic_r15_regression_log_equity_bankrupt_p5_no_collapse_and_dashed_conflated_fan() {
        use crate::backtest::{
            BacktestConfig, MonteCarloConfig, MonteCarloFanSeries, run_backtest,
            run_monte_carlo_bootstrap,
        };
        use crate::viewport::PriceScaleMode;

        // 1. R15-M03: `PriceScaleMode::Logarithmic` with bankrupt `p5 == 0.0` must NOT crush `$80k..$180k` into 3px!
        let n = 64_usize;
        let p5 = vec![Some(0.0); n];
        let p25 = vec![Some(85_000.0); n];
        let p50 = vec![Some(110_000.0); n];
        let p75 = vec![Some(140_000.0); n];
        let p95 = vec![Some(180_000.0); n];
        let fan = MonteCarloFanSeries {
            bar_count: n,
            causal_bar_count: n,
            p5_idx: SeriesBlockIndex::build(&p5),
            p50_idx: SeriesBlockIndex::build(&p50),
            p95_idx: SeriesBlockIndex::build(&p95),
            p5,
            p25,
            p50,
            p75,
            p95,
        };
        let mut log_eq_ps = PriceScale::new("eq-log", PriceScaleOptions::default());
        log_eq_ps.set_height(200.0);
        log_eq_ps.set_mode(PriceScaleMode::Logarithmic);
        autoscale_equity_bounds_with_fan(
            &mut log_eq_ps,
            100_000.0,
            Some((80_000.0, 155_000.0)),
            Some(&fan),
            &(0..n),
        );
        let y_80k = log_eq_ps
            .price_to_local_y(80_000.0, 100_000.0)
            .expect("y_80k");
        let y_180k = log_eq_ps
            .price_to_local_y(180_000.0, 100_000.0)
            .expect("y_180k");
        assert!(
            (y_80k - y_180k).abs() > 90.0,
            "Logarithmic Equity scale must preserve visual span for $80k..$180k even when p5 hits 0.0: span={}",
            (y_80k - y_180k).abs()
        );

        // 2. R15-m02: Conflated `step = 64` fan render reaches terminal bar & emits dashed `p50` segments cleanly
        let candles = generate_synthetic_ohlcv(2_048);
        let bt = run_backtest(
            "VNINDEX",
            Timeframe::D1,
            &candles,
            &BacktestConfig::default(),
            None,
        );
        let mc = run_monte_carlo_bootstrap(
            100_000.0,
            &bt.trades,
            &bt.equity_curve,
            candles.len(),
            None,
            MonteCarloConfig {
                simulations: 250,
                ..MonteCarloConfig::default()
            },
        );
        let ctx = egui::Context::default();
        let mut vp = ChartViewportState::new();
        vp.time_scale.set_total_bars(candles.len());
        vp.time_scale.set_width(800.0);
        vp.fit_all();
        vp.equity_price_scale.set_mode(PriceScaleMode::Logarithmic);
        let ind = IndicatorConfig::default();
        let mut cache = IndicatorCache::new();
        cache.sync(&candles, 1, &ind);
        let mut drawings = DrawingStore::new();
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1024.0, 680.0))),
                ..Default::default()
            },
            |ui| {
                let rep = render_workstation_chart_full(
                    ui,
                    &candles,
                    &mut vp,
                    ChartDisplayConfig {
                        symbol: "VNINDEX",
                        timeframe: "1D",
                        show_grid: true,
                        show_crosshair: false,
                        show_volume: false,
                    },
                    &ind,
                    &cache,
                    &mut drawings,
                    ChartReplayTradingOverlay {
                        backtest_result: Some(&bt),
                        monte_carlo_fan: mc.bar_fan.as_ref(),
                        show_equity_pane: true,
                        ..ChartReplayTradingOverlay::default()
                    },
                )
                .expect("conflated log equity + mc fan render");
                assert!(rep.conflation_factor >= 2);
                assert!(rep.equity_pane_active);
            },
        );
        out.textures_delta.clear();
    }

    #[test]
    fn r16_alert_markers_independent_above_below_slot_stacking_and_signal_overlay_render() {
        use crate::alerts::{
            AlertCondition, AlertEngine, AlertFrequency, AlertTriggerEvent, CrossDirection,
        };
        use crate::scanner::ConfluenceWeightProfile;

        let candles = generate_synthetic_ohlcv(120);
        let mut alerts = AlertEngine::new();
        let _ = alerts.add_alert(
            "VNINDEX",
            Timeframe::D1,
            AlertCondition::SignalCatalogTrigger {
                catalog_id: "volume.spike_2x".to_owned(),
                float_level: None,
                direction: CrossDirection::CrossAbove,
            },
            AlertFrequency::OncePerBarClose,
            None,
            &candles,
            60,
            None,
            None,
        );
        let _ = alerts.add_alert(
            "VNINDEX",
            Timeframe::D1,
            AlertCondition::ConfluenceThreshold {
                profile: ConfluenceWeightProfile::Balanced,
                threshold: 15.0,
                direction: CrossDirection::CrossBelow,
            },
            AlertFrequency::OncePerBarClose,
            None,
            &candles,
            60,
            None,
            None,
        );

        let same_bar_ts = candles[90].timestamp;
        let triggers = vec![
            AlertTriggerEvent {
                event_id: 1,
                alert_id: 1,
                symbol: "VNINDEX".to_owned(),
                timeframe: Timeframe::D1,
                bar_timestamp: same_bar_ts,
                eval_base_timestamp: same_bar_ts,
                direction: CrossDirection::CrossAbove,
                threshold_value: 1.0,
                observed_value: 1.0,
                condition_summary: "VOL_SPIKE_2X ▲".to_owned(),
                message: "up1".to_owned(),
            },
            AlertTriggerEvent {
                event_id: 2,
                alert_id: 2,
                symbol: "VNINDEX".to_owned(),
                timeframe: Timeframe::D1,
                bar_timestamp: same_bar_ts,
                eval_base_timestamp: same_bar_ts,
                direction: CrossDirection::CrossBelow,
                threshold_value: 15.0,
                observed_value: -8.0,
                condition_summary: "Confluence ▼ 15.0".to_owned(),
                message: "dn1".to_owned(),
            },
            AlertTriggerEvent {
                event_id: 3,
                alert_id: 1,
                symbol: "VNINDEX".to_owned(),
                timeframe: Timeframe::D1,
                bar_timestamp: same_bar_ts,
                eval_base_timestamp: same_bar_ts,
                direction: CrossDirection::CrossAbove,
                threshold_value: 1.0,
                observed_value: 1.0,
                condition_summary: "VOL_SPIKE_2X ▲".to_owned(),
                message: "up2".to_owned(),
            },
        ];

        alerts.triggers = triggers;

        let ctx = egui::Context::default();
        let mut vp = ChartViewportState::new();
        vp.time_scale.set_total_bars(candles.len());
        vp.time_scale.set_width(800.0);
        vp.fit_all();
        let ind = IndicatorConfig::default();
        let mut cache = IndicatorCache::new();
        cache.sync(&candles, 1, &ind);
        let mut drawings = DrawingStore::new();
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1024.0, 680.0))),
                ..Default::default()
            },
            |ui| {
                let rep = render_workstation_chart_full(
                    ui,
                    &candles,
                    &mut vp,
                    ChartDisplayConfig {
                        symbol: "VNINDEX",
                        timeframe: "1D",
                        show_grid: true,
                        show_crosshair: false,
                        show_volume: true,
                    },
                    &ind,
                    &cache,
                    &mut drawings,
                    ChartReplayTradingOverlay {
                        alert_engine: Some(&alerts),
                        ..ChartReplayTradingOverlay::default()
                    },
                )
                .expect("render mixed above/below alert markers");
                assert!(rep.rendered_candle_count > 0);
            },
        );
        out.textures_delta.clear();
    }

    #[test]
    fn r17_multistrategy_equity_overlay_rendering_and_log_scale_fallback() {
        use crate::backtest::{BacktestConfig, run_multi_strategy_comparison};
        use crate::data::{DataSourcePreset, generate_synthetic_ohlcv};
        use crate::viewport::{PriceAxisFormatMode, PriceScaleMode};

        let candles = generate_synthetic_ohlcv(240);
        let cfg = BacktestConfig::default();
        let ms = run_multi_strategy_comparison(
            "VNINDEX",
            DataSourcePreset::SyntheticVnindex,
            Timeframe::D1,
            &candles,
            &cfg,
            &[],
            None,
            false,
        );
        assert!(ms.curves.len() >= 8);

        let ctx = egui::Context::default();
        let mut vp = ChartViewportState::new();
        vp.time_scale.set_total_bars(candles.len());
        vp.time_scale.set_width(820.0);
        vp.fit_all();
        // Start Equity scale in Logarithmic mode to verify auto-fallback to Normal + SignedPercentage!
        vp.equity_price_scale.set_mode(PriceScaleMode::Logarithmic);

        let ind = IndicatorConfig::default();
        let mut cache = IndicatorCache::new();
        cache.sync(&candles, 1, &ind);
        let mut drawings = DrawingStore::new();

        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1024.0, 680.0))),
                ..Default::default()
            },
            |ui| {
                let rep = render_workstation_chart_full(
                    ui,
                    &candles,
                    &mut vp,
                    ChartDisplayConfig {
                        symbol: "VNINDEX",
                        timeframe: "1D",
                        show_grid: true,
                        show_crosshair: true,
                        show_volume: true,
                    },
                    &ind,
                    &cache,
                    &mut drawings,
                    ChartReplayTradingOverlay {
                        show_equity_pane: true,
                        multi_strategy_overlay: Some(&ms),
                        ..ChartReplayTradingOverlay::default()
                    },
                )
                .expect("render MultiStrategyComparisonReport on Equity pane");
                assert!(rep.equity_pane_active);
            },
        );
        out.textures_delta.clear();
        assert_eq!(vp.equity_price_scale.mode(), PriceScaleMode::Normal);
        assert_eq!(
            vp.equity_price_scale.format_mode(),
            PriceAxisFormatMode::SignedPercentage
        );
    }

    #[test]
    fn critic_r17_regression_multistrategy_whitespace_conflated_and_linked_crosshair_dots() {
        use crate::backtest::{BacktestConfig, run_multi_strategy_comparison};
        use crate::data::{DataSourcePreset, load_preset_dataset};
        use crate::viewport::{PriceAxisFormatMode, PriceScaleMode};

        let loaded = load_preset_dataset(DataSourcePreset::SyntheticVnindex, 240, None);
        let candles = loaded.candles;
        let ms = run_multi_strategy_comparison(
            "VNINDEX",
            DataSourcePreset::SyntheticVnindex,
            Timeframe::D1,
            &candles,
            &BacktestConfig::default(),
            &[],
            None,
            false,
        );

        let count_circles = |shapes: &[egui::epaint::ClippedShape]| -> usize {
            shapes
                .iter()
                .filter(|cs| matches!(cs.shape, Shape::Circle(_)))
                .count()
        };

        let ctx = egui::Context::default();
        let mut vp = ChartViewportState::new();
        vp.time_scale.set_total_bars(candles.len());
        vp.time_scale.set_width(820.0);
        vp.time_scale.set_right_offset(30.0); // Generous right-margin whitespace!
        vp.equity_price_scale.set_mode(PriceScaleMode::Logarithmic);

        let ind = IndicatorConfig::default();
        let mut cache = IndicatorCache::new();
        cache.sync(&candles, 1, &ind);
        let mut drawings = DrawingStore::new();

        // Pass 1: Hover inside right-margin whitespace -> ZERO phantom circle markers on Equity pane!
        let mut plot_rect = Rect::NOTHING;
        let mut out_ws = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1024.0, 680.0))),
                events: vec![egui::Event::PointerMoved(Pos2::new(910.0, 260.0))],
                ..Default::default()
            },
            |ui| {
                let rep = render_workstation_chart_full(
                    ui,
                    &candles,
                    &mut vp,
                    ChartDisplayConfig {
                        symbol: "VNINDEX",
                        timeframe: "1D",
                        show_grid: true,
                        show_crosshair: true,
                        show_volume: true,
                    },
                    &ind,
                    &cache,
                    &mut drawings,
                    ChartReplayTradingOverlay {
                        show_equity_pane: true,
                        multi_strategy_overlay: Some(&ms),
                        ..ChartReplayTradingOverlay::default()
                    },
                )
                .unwrap();
                plot_rect = rep.layout.plot_rect;
            },
        );
        // Hover near `plot_rect.right() - 4.0` (deep inside the 30-bar right whitespace)
        let ws_pos = Pos2::new(plot_rect.right() - 4.0, plot_rect.center().y);
        let mut out_ws2 = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1024.0, 680.0))),
                events: vec![egui::Event::PointerMoved(ws_pos)],
                ..Default::default()
            },
            |ui| {
                let _ = render_workstation_chart_full(
                    ui,
                    &candles,
                    &mut vp,
                    ChartDisplayConfig {
                        symbol: "VNINDEX",
                        timeframe: "1D",
                        show_grid: true,
                        show_crosshair: true,
                        show_volume: true,
                    },
                    &ind,
                    &cache,
                    &mut drawings,
                    ChartReplayTradingOverlay {
                        show_equity_pane: true,
                        multi_strategy_overlay: Some(&ms),
                        ..ChartReplayTradingOverlay::default()
                    },
                );
            },
        );
        out_ws.textures_delta.clear();
        out_ws2.textures_delta.clear();
        assert_eq!(
            count_circles(&out_ws2.shapes),
            0,
            "hovering right whitespace must NOT render phantom curve dot markers"
        );

        // Pass 2: Linked multi-chart crosshair (`external_crosshair_timestamp = Some(ts)`, no local hover)
        // MUST render curve dot markers on all active curves!
        let target_ts = candles[candles.len() - 10].timestamp;
        let mut out_linked = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1024.0, 680.0))),
                events: vec![egui::Event::PointerGone],
                ..Default::default()
            },
            |ui| {
                let _ = render_workstation_chart_full(
                    ui,
                    &candles,
                    &mut vp,
                    ChartDisplayConfig {
                        symbol: "VNINDEX",
                        timeframe: "1D",
                        show_grid: true,
                        show_crosshair: true,
                        show_volume: true,
                    },
                    &ind,
                    &cache,
                    &mut drawings,
                    ChartReplayTradingOverlay {
                        show_equity_pane: true,
                        multi_strategy_overlay: Some(&ms),
                        external_crosshair_timestamp: Some(target_ts),
                        ..ChartReplayTradingOverlay::default()
                    },
                );
            },
        );
        assert!(
            count_circles(&out_linked.shapes) >= 8,
            "linked multi-chart crosshair must render synchronized curve dot markers"
        );
        assert_eq!(vp.equity_price_scale.mode(), PriceScaleMode::Normal);
        assert_eq!(
            vp.equity_price_scale.format_mode(),
            PriceAxisFormatMode::SignedPercentage
        );
        out_linked.textures_delta.clear();
    }
}
