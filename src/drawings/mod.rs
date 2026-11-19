//! Pure `f64`, `wasm32-unknown-unknown`-compatible Interactive Drawing & Annotation Engine (`Milestone R05 / Wave 3`).
//!
//! Anchors drawings in continuous logical `(bar_index: f64, value: f64)` coordinates scoped to
//! [`DrawingPaneTarget`] (`Main` price pane or `Rsi` / `Macd` / `Atr` oscillator sub-panes),
//! projects through `TimeScale` + `PriceScale` into pane-local `f64` space, clips rays/segments
//! analytically via Liang-Barsky, and enforces LWC-aligned handle-first hit-test precedence.

pub mod controller;
pub mod geometry;
pub mod hit_test;
#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};

use crate::indicators::OscillatorPaneId;

pub use controller::{
    DRAG_COMMIT_DISTANCE_PX, DrawingCommand, DrawingInteractionPhase, DrawingPatchError,
    DrawingPropertyPatch, DrawingStore, EDIT_DRAG_THRESHOLD_PX, MAX_PERSISTED_DRAWINGS,
    MAX_UNDO_STEPS, PREVIEW_DRAWING_ID, PointerCaptureOutcome,
};
pub use geometry::{
    HandlePart, LocalPoint, LocalRect, LocalSegment, PaneDrawingProjection, ProjectedDrawing,
    ProjectedDrawingShape, ProjectedFibBand, ProjectedFibLevel, ProjectedHandle,
    clip_parametric_line, distance_to_segment, fibonacci_level_value,
};
pub use hit_test::{
    HANDLE_HIT_RADIUS_PX, HitMatch, HitPriority, HitTarget, MAGNET_THRESHOLD_PX,
    SELECTED_HANDLE_HIT_RADIUS_PX, STROKE_HIT_TOLERANCE_PX, hit_test_pane_drawings,
    hit_test_pane_drawings_causal, hit_test_projected_drawing, snap_to_visible_ohlc,
};

/// Stable unique identifier assigned monotonically to each committed [`Drawing`].
pub type DrawingId = u64;

/// Canonical Fibonacci retracement ratios (`ARCHIVE/frontend/src/features/drawings/drawingDomain.ts:32`).
pub const FIBONACCI_RATIOS: [f64; 7] = [0.0, 0.236, 0.382, 0.5, 0.618, 0.786, 1.0];

/// Maximum absolute bar-slot index accepted when sanitizing or committing anchors.
pub const MAX_ABS_BAR_INDEX: f64 = 1.0e9;

/// Maximum absolute domain price/oscillator value accepted when sanitizing or committing anchors.
/// Aligned with `PriceScale::is_valid_logical_range` (`1.0e15`) so `price_to_local_y` and `p1 - p0` never overflow `f64::MAX`.
pub const MAX_ABS_DRAWING_VALUE: f64 = 1.0e15;

/// Maximum absolute cached anchor UTC timestamp accepted during deserialization (`±1e16` seconds).
pub const MAX_ABS_ANCHOR_TIMESTAMP: f64 = 1.0e16;

/// Target pane owning a drawing's vertical coordinate space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum DrawingPaneTarget {
    #[default]
    Main,
    Oscillator(OscillatorPaneId),
}

impl DrawingPaneTarget {
    #[inline]
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Main => "main",
            Self::Oscillator(OscillatorPaneId::Rsi) => "rsi",
            Self::Oscillator(OscillatorPaneId::Macd) => "macd",
            Self::Oscillator(OscillatorPaneId::Atr) => "atr",
            Self::Oscillator(OscillatorPaneId::Equity) => "equity",
            Self::Oscillator(OscillatorPaneId::RelativeStrength) => "rs",
            Self::Oscillator(OscillatorPaneId::Cvd) => "cvd",
            Self::Oscillator(OscillatorPaneId::Adx) => "adx",
            Self::Oscillator(OscillatorPaneId::Stochastic) => "stoch",
            Self::Oscillator(OscillatorPaneId::Cci) => "cci",
            Self::Oscillator(OscillatorPaneId::Mfi) => "mfi",
            Self::Oscillator(OscillatorPaneId::ProxyBb) => "proxy-bb",
        }
    }

    #[inline]
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::Main => "MAIN",
            Self::Oscillator(OscillatorPaneId::Rsi) => "RSI",
            Self::Oscillator(OscillatorPaneId::Macd) => "MACD",
            Self::Oscillator(OscillatorPaneId::Atr) => "ATR",
            Self::Oscillator(OscillatorPaneId::Equity) => "EQUITY",
            Self::Oscillator(OscillatorPaneId::RelativeStrength) => "RS",
            Self::Oscillator(OscillatorPaneId::Cvd) => "CVD",
            Self::Oscillator(OscillatorPaneId::Adx) => "ADX",
            Self::Oscillator(OscillatorPaneId::Stochastic) => "STOCH",
            Self::Oscillator(OscillatorPaneId::Cci) => "CCI",
            Self::Oscillator(OscillatorPaneId::Mfi) => "MFI",
            Self::Oscillator(OscillatorPaneId::ProxyBb) => "PROXY-BB",
        }
    }
}

/// Active workstation pointer tool for chart annotations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum DrawingTool {
    #[default]
    Select,
    HorizontalLine,
    TrendLine,
    Ray,
    Rectangle,
    FibonacciRetracement,
    RiskRewardBox,
    FixedRangeVolumeProfile,
    AnchoredVwap,
}

impl DrawingTool {
    #[inline]
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::Select => "Cursor",
            Self::HorizontalLine => "H-Line",
            Self::TrendLine => "Trend",
            Self::Ray => "Ray",
            Self::Rectangle => "Rect",
            Self::FibonacciRetracement => "Fib",
            Self::RiskRewardBox => "Pos R:R",
            Self::FixedRangeVolumeProfile => "FRVP",
            Self::AnchoredVwap => "AVWAP",
        }
    }

    #[inline]
    #[must_use]
    pub const fn required_anchors(self) -> usize {
        match self {
            Self::Select => 0,
            Self::HorizontalLine | Self::AnchoredVwap => 1,
            Self::TrendLine
            | Self::Ray
            | Self::Rectangle
            | Self::FibonacciRetracement
            | Self::RiskRewardBox
            | Self::FixedRangeVolumeProfile => 2,
        }
    }
}

/// OHLC candle-snapping mode applied on [`DrawingPaneTarget::Main`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum MagnetMode {
    #[default]
    Off,
    Ohlc,
}

/// Stroke dash pattern for lines, rays, borders, and levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LineDashStyle {
    #[default]
    Solid,
    Dashed,
    Dotted,
}

impl LineDashStyle {
    #[inline]
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Solid => Self::Dashed,
            Self::Dashed => Self::Dotted,
            Self::Dotted => Self::Solid,
        }
    }

    #[inline]
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Solid => "Solid",
            Self::Dashed => "Dashed",
            Self::Dotted => "Dotted",
        }
    }
}

/// Direction of Fibonacci `0.0 -> 1.0` interpolation between `[anchor_0, anchor_1]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum FibDirection {
    #[default]
    StartToEnd,
    EndToStart,
}

impl FibDirection {
    #[inline]
    #[must_use]
    pub const fn reversed(self) -> Self {
        match self {
            Self::StartToEnd => Self::EndToStart,
            Self::EndToStart => Self::StartToEnd,
        }
    }
}

/// Named color presets keeping `src/drawings` free of UI-framework color types while serializing cleanly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum DrawingColorPreset {
    #[default]
    Cyan,
    ElectricBlue,
    AmberGold,
    Emerald,
    Crimson,
    Magenta,
}

impl DrawingColorPreset {
    pub const ALL: [Self; 6] = [
        Self::Cyan,
        Self::ElectricBlue,
        Self::AmberGold,
        Self::Emerald,
        Self::Crimson,
        Self::Magenta,
    ];

    #[inline]
    #[must_use]
    pub const fn rgb(self) -> [u8; 3] {
        match self {
            Self::Cyan => [77, 208, 225],         // #4DD0E1
            Self::ElectricBlue => [41, 134, 255], // #2986FF
            Self::AmberGold => [255, 193, 7],     // #FFC107
            Self::Emerald => [38, 198, 164],      // #26C6A4
            Self::Crimson => [255, 92, 92],       // #FF5C5C
            Self::Magenta => [224, 86, 253],      // #E056FD (legacy Sumi default)
        }
    }

    #[inline]
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Cyan => Self::AmberGold,
            Self::AmberGold => Self::ElectricBlue,
            Self::ElectricBlue => Self::Emerald,
            Self::Emerald => Self::Magenta,
            Self::Magenta => Self::Crimson,
            Self::Crimson => Self::Cyan,
        }
    }

    #[inline]
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::Cyan => "Cyan",
            Self::ElectricBlue => "Blue",
            Self::AmberGold => "Gold",
            Self::Emerald => "Teal",
            Self::Crimson => "Red",
            Self::Magenta => "Pink",
        }
    }
}

/// Logical data-space anchor binding continuous bar-slot index, raw series value,
/// and optional market UTC epoch timestamp (`f64` seconds) for lossless multi-timeframe remapping.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DrawingAnchor {
    pub bar_index: f64,
    pub value: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<f64>,
}

impl DrawingAnchor {
    #[inline]
    #[must_use]
    pub const fn new(bar_index: f64, value: f64) -> Self {
        Self {
            bar_index,
            value,
            timestamp: None,
        }
    }

    #[inline]
    #[must_use]
    pub const fn with_timestamp(bar_index: f64, value: f64, timestamp: f64) -> Self {
        Self {
            bar_index,
            value,
            timestamp: Some(timestamp),
        }
    }

    #[inline]
    #[must_use]
    pub fn is_valid(self) -> bool {
        let ts_ok = self
            .timestamp
            .is_none_or(|ts| ts.is_finite() && ts.abs() <= MAX_ABS_ANCHOR_TIMESTAMP);
        self.bar_index.is_finite()
            && self.bar_index.abs() <= MAX_ABS_BAR_INDEX
            && self.value.is_finite()
            && self.value.abs() <= MAX_ABS_DRAWING_VALUE
            && ts_ok
    }
}

/// Portable styling parameters for a [`Drawing`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DrawingStyle {
    pub color: DrawingColorPreset,
    pub line_width: f32,
    pub dash: LineDashStyle,
    pub fill_opacity: f32,
}

impl Default for DrawingStyle {
    fn default() -> Self {
        Self {
            color: DrawingColorPreset::Cyan,
            line_width: 1.75,
            dash: LineDashStyle::Solid,
            fill_opacity: 0.14,
        }
    }
}

impl DrawingStyle {
    #[must_use]
    pub const fn default_for_tool(tool: DrawingTool) -> Self {
        match tool {
            DrawingTool::HorizontalLine => Self {
                color: DrawingColorPreset::AmberGold,
                line_width: 1.5,
                dash: LineDashStyle::Dashed,
                fill_opacity: 0.0,
            },
            DrawingTool::TrendLine => Self {
                color: DrawingColorPreset::Cyan,
                line_width: 2.0,
                dash: LineDashStyle::Solid,
                fill_opacity: 0.0,
            },
            DrawingTool::Ray => Self {
                color: DrawingColorPreset::ElectricBlue,
                line_width: 1.75,
                dash: LineDashStyle::Solid,
                fill_opacity: 0.0,
            },
            DrawingTool::Rectangle => Self {
                color: DrawingColorPreset::Emerald,
                line_width: 1.5,
                dash: LineDashStyle::Solid,
                fill_opacity: 0.14,
            },
            DrawingTool::FibonacciRetracement => Self {
                color: DrawingColorPreset::Magenta,
                line_width: 1.25,
                dash: LineDashStyle::Solid,
                fill_opacity: 0.11,
            },
            DrawingTool::RiskRewardBox => Self {
                color: DrawingColorPreset::Emerald,
                line_width: 1.5,
                dash: LineDashStyle::Solid,
                fill_opacity: 0.16,
            },
            DrawingTool::FixedRangeVolumeProfile => Self {
                color: DrawingColorPreset::ElectricBlue,
                line_width: 1.5,
                dash: LineDashStyle::Dashed,
                fill_opacity: 0.16,
            },
            DrawingTool::AnchoredVwap => Self {
                color: DrawingColorPreset::AmberGold,
                line_width: 2.0,
                dash: LineDashStyle::Solid,
                fill_opacity: 0.14,
            },
            DrawingTool::Select => Self {
                color: DrawingColorPreset::Cyan,
                line_width: 1.75,
                dash: LineDashStyle::Solid,
                fill_opacity: 0.14,
            },
        }
    }

    pub fn sanitize(&mut self) {
        if !self.line_width.is_finite() {
            self.line_width = 1.75;
        } else {
            self.line_width = self.line_width.clamp(1.0, 8.0);
        }
        if !self.fill_opacity.is_finite() {
            self.fill_opacity = 0.14;
        } else {
            self.fill_opacity = self.fill_opacity.clamp(0.0, 0.65);
        }
    }
}

const fn default_frvp_row_count() -> u16 {
    24
}

const fn default_avwap_show_bands() -> bool {
    true
}

/// Strongly typed geometric payload and logical anchors of a [`Drawing`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DrawingGeometry {
    HorizontalLine {
        anchor: DrawingAnchor,
    },
    TrendLine {
        start: DrawingAnchor,
        end: DrawingAnchor,
    },
    Ray {
        origin: DrawingAnchor,
        through: DrawingAnchor,
    },
    Rectangle {
        corner_a: DrawingAnchor,
        corner_b: DrawingAnchor,
    },
    FibonacciRetracement {
        start: DrawingAnchor,
        end: DrawingAnchor,
        direction: FibDirection,
        extend_right: bool,
    },
    /// Dual-zone Risk-Reward Position Tool (`Milestone R10`):
    /// `start.value` = entry price, `end.value` = target price, `stop_price` = stop-loss price,
    /// horizontal box span = `[min(start.bar_index, end.bar_index), max(start.bar_index, end.bar_index)]`.
    RiskRewardBox {
        start: DrawingAnchor,
        end: DrawingAnchor,
        stop_price: f64,
    },
    /// Fixed Range Volume Profile (`Milestone R11`):
    /// Horizontal bar-span volume histogram with `POC`, `VAH`, `VAL` across `[start.bar_index, end.bar_index]`.
    FixedRangeVolumeProfile {
        start: DrawingAnchor,
        end: DrawingAnchor,
        #[serde(default = "default_frvp_row_count")]
        row_count: u16,
    },
    /// Anchored VWAP (`Milestone R11`):
    /// Cumulative volume-weighted average price + optional Welford $\pm 1\sigma/\pm 2\sigma$ bands from `anchor.bar_index`.
    AnchoredVwap {
        anchor: DrawingAnchor,
        #[serde(default = "default_avwap_show_bands")]
        show_bands: bool,
    },
}

impl DrawingGeometry {
    /// Validates strict positive prices and bracket ordering (`Long`: `stop < entry < target`,
    /// `Short`: `target < entry < stop`) for a `RiskRewardBox`.
    #[inline]
    #[must_use]
    pub fn risk_reward_side_of(
        entry_price: f64,
        target_price: f64,
        stop_price: f64,
    ) -> Option<crate::trading::PositionSide> {
        if !entry_price.is_finite()
            || !target_price.is_finite()
            || !stop_price.is_finite()
            || entry_price <= 0.0
            || target_price <= 0.0
            || stop_price <= 0.0
            || entry_price > MAX_ABS_DRAWING_VALUE
            || target_price > MAX_ABS_DRAWING_VALUE
            || stop_price > MAX_ABS_DRAWING_VALUE
        {
            return None;
        }
        let eps = (entry_price.abs() * 1.0e-7).max(1.0e-9);
        if target_price > entry_price + eps && stop_price < entry_price - eps {
            Some(crate::trading::PositionSide::Long)
        } else if target_price < entry_price - eps && stop_price > entry_price + eps {
            Some(crate::trading::PositionSide::Short)
        } else {
            None
        }
    }

    /// Returns `(side, entry_price, stop_price, target_price, gross_r_multiple)` when `self` is a valid `RiskRewardBox`.
    #[inline]
    #[must_use]
    pub fn risk_reward_metrics(
        &self,
    ) -> Option<(crate::trading::PositionSide, f64, f64, f64, f64)> {
        if let Self::RiskRewardBox {
            start,
            end,
            stop_price,
        } = *self
        {
            let side = Self::risk_reward_side_of(start.value, end.value, stop_price)?;
            let risk = (start.value - stop_price).abs();
            let reward = (end.value - start.value).abs();
            if risk <= 1.0e-12 {
                return None;
            }
            let gross_r = reward / risk;
            gross_r
                .is_finite()
                .then_some((side, start.value, stop_price, end.value, gross_r))
        } else {
            None
        }
    }

    #[inline]
    #[must_use]
    pub const fn tool_kind(&self) -> DrawingTool {
        match self {
            Self::HorizontalLine { .. } => DrawingTool::HorizontalLine,
            Self::TrendLine { .. } => DrawingTool::TrendLine,
            Self::Ray { .. } => DrawingTool::Ray,
            Self::Rectangle { .. } => DrawingTool::Rectangle,
            Self::FibonacciRetracement { .. } => DrawingTool::FibonacciRetracement,
            Self::RiskRewardBox { .. } => DrawingTool::RiskRewardBox,
            Self::FixedRangeVolumeProfile { .. } => DrawingTool::FixedRangeVolumeProfile,
            Self::AnchoredVwap { .. } => DrawingTool::AnchoredVwap,
        }
    }

    #[must_use]
    pub fn anchors(&self) -> [DrawingAnchor; 2] {
        match *self {
            Self::HorizontalLine { anchor } | Self::AnchoredVwap { anchor, .. } => [anchor, anchor],
            Self::TrendLine { start, end }
            | Self::Ray {
                origin: start,
                through: end,
            }
            | Self::Rectangle {
                corner_a: start,
                corner_b: end,
            }
            | Self::FibonacciRetracement { start, end, .. }
            | Self::RiskRewardBox { start, end, .. }
            | Self::FixedRangeVolumeProfile { start, end, .. } => [start, end],
        }
    }

    #[inline]
    #[must_use]
    pub const fn anchor_count(&self) -> usize {
        match self {
            Self::HorizontalLine { .. } | Self::AnchoredVwap { .. } => 1,
            Self::TrendLine { .. }
            | Self::Ray { .. }
            | Self::Rectangle { .. }
            | Self::FibonacciRetracement { .. }
            | Self::RiskRewardBox { .. }
            | Self::FixedRangeVolumeProfile { .. } => 2,
        }
    }

    #[must_use]
    pub fn is_valid(&self) -> bool {
        match *self {
            Self::HorizontalLine { anchor } => anchor.is_valid(),
            Self::AnchoredVwap { anchor, .. } => anchor.is_valid() && anchor.bar_index >= -0.5,
            Self::TrendLine { start, end }
            | Self::Rectangle {
                corner_a: start,
                corner_b: end,
            }
            | Self::FibonacciRetracement { start, end, .. } => start.is_valid() && end.is_valid(),
            Self::FixedRangeVolumeProfile {
                start,
                end,
                row_count,
            } => {
                let has_bar_span = (end.bar_index - start.bar_index).abs() >= 0.5;
                let has_ts_span = start
                    .timestamp
                    .zip(end.timestamp)
                    .is_some_and(|(t0, t1)| (t1 - t0).abs() > 1e-6);
                start.is_valid()
                    && end.is_valid()
                    && (has_bar_span || has_ts_span)
                    && (8..=200).contains(&row_count)
            }
            Self::Ray { origin, through } => {
                origin.is_valid() && through.is_valid() && has_non_zero_direction(origin, through)
            }
            Self::RiskRewardBox {
                start,
                end,
                stop_price,
            } => {
                start.is_valid()
                    && end.is_valid()
                    && Self::risk_reward_side_of(start.value, end.value, stop_price).is_some()
            }
        }
    }

    pub fn set_anchor(&mut self, index: usize, anchor: DrawingAnchor) -> bool {
        if !anchor.is_valid() {
            return false;
        }
        match self {
            Self::HorizontalLine { anchor: a } => {
                if index == 0 {
                    *a = anchor;
                    true
                } else {
                    false
                }
            }
            Self::AnchoredVwap { anchor: a, .. } => {
                if index == 0 && anchor.bar_index >= -0.5 {
                    *a = anchor;
                    true
                } else {
                    false
                }
            }
            Self::TrendLine { start, end }
            | Self::Rectangle {
                corner_a: start,
                corner_b: end,
            }
            | Self::FibonacciRetracement { start, end, .. } => match index {
                0 => {
                    *start = anchor;
                    true
                }
                1 => {
                    *end = anchor;
                    true
                }
                _ => false,
            },
            Self::FixedRangeVolumeProfile { start, end, .. } => match index {
                0 if (end.bar_index - anchor.bar_index).abs() >= 0.5 => {
                    *start = anchor;
                    true
                }
                1 if (anchor.bar_index - start.bar_index).abs() >= 0.5 => {
                    *end = anchor;
                    true
                }
                _ => false,
            },
            Self::Ray { origin, through } => match index {
                0 => {
                    if !has_non_zero_direction(anchor, *through) {
                        return false;
                    }
                    *origin = anchor;
                    true
                }
                1 => {
                    if !has_non_zero_direction(*origin, anchor) {
                        return false;
                    }
                    *through = anchor;
                    true
                }
                _ => false,
            },
            Self::RiskRewardBox {
                start,
                end,
                stop_price,
            } => match index {
                0 => {
                    if Self::risk_reward_side_of(anchor.value, end.value, *stop_price).is_none() {
                        return false;
                    }
                    *start = anchor;
                    true
                }
                1 => {
                    if Self::risk_reward_side_of(start.value, anchor.value, *stop_price).is_none() {
                        return false;
                    }
                    *end = anchor;
                    true
                }
                _ => false,
            },
        }
    }
}

/// Checks that two anchors define a well-posed $360^\circ$ ray direction (`P0 != P1`).
#[inline]
#[must_use]
pub fn has_non_zero_direction(a: DrawingAnchor, b: DrawingAnchor) -> bool {
    (b.bar_index - a.bar_index).abs() > 1e-9 || (b.value - a.value).abs() > 1e-12
}

/// Complete serializable chart annotation entity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Drawing {
    pub id: DrawingId,
    pub pane: DrawingPaneTarget,
    pub visible: bool,
    pub locked: bool,
    pub style: DrawingStyle,
    pub geometry: DrawingGeometry,
    /// Optional causal replay cutoff timestamp (`UTC` seconds) at which this drawing was created.
    ///
    /// - `Some(created_ts)`: authored during an active Bar Replay session at `created_ts`; visible
    ///   for all replay states with `cutoff_timestamp >= created_ts` (including anchors projected
    ///   into right-hand future whitespace), and masked if the user rewinds earlier than `created_ts`.
    /// - `None`: authored outside Bar Replay; during active replay at `cutoff_timestamp`, visible
    ///   iff every anchor lies at or before `cutoff_timestamp` (`DEF-001` zero-lookahead guard).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at_timestamp: Option<i64>,
}

impl Drawing {
    #[must_use]
    pub fn new(
        id: DrawingId,
        pane: DrawingPaneTarget,
        geometry: DrawingGeometry,
        style: DrawingStyle,
    ) -> Self {
        let mut clean_style = style;
        clean_style.sanitize();
        Self {
            id,
            pane,
            visible: true,
            locked: false,
            style: clean_style,
            geometry,
            created_at_timestamp: None,
        }
    }

    #[inline]
    #[must_use]
    pub const fn tool_kind(&self) -> DrawingTool {
        self.geometry.tool_kind()
    }

    #[must_use]
    pub fn is_valid(&self) -> bool {
        let created_ts_ok = self.created_at_timestamp.is_none_or(|ts| {
            (ts as f64).is_finite() && (ts as f64).abs() <= MAX_ABS_ANCHOR_TIMESTAMP
        });
        let pane_ok = match self.geometry {
            DrawingGeometry::RiskRewardBox { .. }
            | DrawingGeometry::FixedRangeVolumeProfile { .. }
            | DrawingGeometry::AnchoredVwap { .. } => self.pane == DrawingPaneTarget::Main,
            _ => true,
        };
        self.id > 0
            && self.id < PREVIEW_DRAWING_ID
            && pane_ok
            && self.geometry.is_valid()
            && created_ts_ok
    }

    /// Evaluates whether this drawing is causally eligible (`DEF-001` zero-lookahead guard) at
    /// `replay_cutoff_timestamp`, **independent** of the user's `self.visible` layer toggle.
    ///
    /// Used by the Interactive Drawing Property Inspector (`Milestone R15`) so hiding a drawing
    /// (`visible = false`) keeps it selectable/editable in the Inspector while still masking
    /// future-anchored drawings during Bar Replay.
    #[must_use]
    pub fn is_causally_eligible(
        &self,
        replay_cutoff_timestamp: Option<i64>,
        visible_candle_count: usize,
    ) -> bool {
        let Some(cutoff_ts) = replay_cutoff_timestamp else {
            return true;
        };
        if let Some(created_ts) = self.created_at_timestamp {
            return created_ts <= cutoff_ts;
        }
        // Pre-replay drawing (`created_at_timestamp == None`): HorizontalLine spans all X and carries
        // only a price level unless anchored to a future candle; mask any pre-replay drawing that has
        // an anchor strictly beyond `cutoff_ts` (or beyond the causal bar count when unstamped).
        let cutoff_f = (cutoff_ts as f64) + 1e-3;
        let max_bar_idx = if visible_candle_count == 0 {
            -1.0
        } else {
            ((visible_candle_count - 1) as f64) + 1e-6
        };
        let anchor_ok = |a: DrawingAnchor| -> bool {
            if let Some(ts) = a.timestamp {
                ts <= cutoff_f
            } else {
                a.bar_index <= max_bar_idx
            }
        };
        match self.geometry {
            DrawingGeometry::HorizontalLine { anchor }
            | DrawingGeometry::AnchoredVwap { anchor, .. } => anchor_ok(anchor),
            DrawingGeometry::TrendLine { start, end }
            | DrawingGeometry::Ray {
                origin: start,
                through: end,
            }
            | DrawingGeometry::Rectangle {
                corner_a: start,
                corner_b: end,
            }
            | DrawingGeometry::FibonacciRetracement { start, end, .. }
            | DrawingGeometry::RiskRewardBox { start, end, .. }
            | DrawingGeometry::FixedRangeVolumeProfile { start, end, .. } => {
                anchor_ok(start) && anchor_ok(end)
            }
        }
    }

    /// Evaluates whether this drawing is rendered and hit-testable on the chart at `replay_cutoff_timestamp`.
    #[inline]
    #[must_use]
    pub fn is_causally_visible(
        &self,
        replay_cutoff_timestamp: Option<i64>,
        visible_candle_count: usize,
    ) -> bool {
        self.visible && self.is_causally_eligible(replay_cutoff_timestamp, visible_candle_count)
    }
}
