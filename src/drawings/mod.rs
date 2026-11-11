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
    DRAG_COMMIT_DISTANCE_PX, DrawingCommand, DrawingInteractionPhase, DrawingStore,
    EDIT_DRAG_THRESHOLD_PX, MAX_PERSISTED_DRAWINGS, MAX_UNDO_STEPS, PREVIEW_DRAWING_ID,
    PointerCaptureOutcome,
};
pub use geometry::{
    HandlePart, LocalPoint, LocalRect, LocalSegment, PaneDrawingProjection, ProjectedDrawing,
    ProjectedDrawingShape, ProjectedFibBand, ProjectedFibLevel, ProjectedHandle,
    clip_parametric_line, distance_to_segment, fibonacci_level_value,
};
pub use hit_test::{
    HANDLE_HIT_RADIUS_PX, HitMatch, HitPriority, HitTarget, MAGNET_THRESHOLD_PX,
    SELECTED_HANDLE_HIT_RADIUS_PX, STROKE_HIT_TOLERANCE_PX, hit_test_pane_drawings,
    hit_test_projected_drawing, snap_to_visible_ohlc,
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
        }
    }

    #[inline]
    #[must_use]
    pub const fn required_anchors(self) -> usize {
        match self {
            Self::Select => 0,
            Self::HorizontalLine => 1,
            Self::TrendLine | Self::Ray | Self::Rectangle | Self::FibonacciRetracement => 2,
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
}

impl DrawingGeometry {
    #[inline]
    #[must_use]
    pub const fn tool_kind(&self) -> DrawingTool {
        match self {
            Self::HorizontalLine { .. } => DrawingTool::HorizontalLine,
            Self::TrendLine { .. } => DrawingTool::TrendLine,
            Self::Ray { .. } => DrawingTool::Ray,
            Self::Rectangle { .. } => DrawingTool::Rectangle,
            Self::FibonacciRetracement { .. } => DrawingTool::FibonacciRetracement,
        }
    }

    #[must_use]
    pub fn anchors(&self) -> [DrawingAnchor; 2] {
        match *self {
            Self::HorizontalLine { anchor } => [anchor, anchor],
            Self::TrendLine { start, end }
            | Self::Ray {
                origin: start,
                through: end,
            }
            | Self::Rectangle {
                corner_a: start,
                corner_b: end,
            }
            | Self::FibonacciRetracement { start, end, .. } => [start, end],
        }
    }

    #[inline]
    #[must_use]
    pub const fn anchor_count(&self) -> usize {
        match self {
            Self::HorizontalLine { .. } => 1,
            Self::TrendLine { .. }
            | Self::Ray { .. }
            | Self::Rectangle { .. }
            | Self::FibonacciRetracement { .. } => 2,
        }
    }

    #[must_use]
    pub fn is_valid(&self) -> bool {
        match *self {
            Self::HorizontalLine { anchor } => anchor.is_valid(),
            Self::TrendLine { start, end }
            | Self::Rectangle {
                corner_a: start,
                corner_b: end,
            }
            | Self::FibonacciRetracement { start, end, .. } => start.is_valid() && end.is_valid(),
            Self::Ray { origin, through } => {
                origin.is_valid() && through.is_valid() && has_non_zero_direction(origin, through)
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
        }
    }

    #[inline]
    #[must_use]
    pub const fn tool_kind(&self) -> DrawingTool {
        self.geometry.tool_kind()
    }

    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.id > 0 && self.id < PREVIEW_DRAWING_ID && self.geometry.is_valid()
    }
}
