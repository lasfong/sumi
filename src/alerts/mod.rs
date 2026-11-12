//! Pure `f64`, `wasm32-unknown-unknown`-compatible Causal Price, Indicator & Drawing-Line Alert Engine
//! (`Milestone R08 / Wave 7`).
//!
//! Evaluates `PriceLevel` (4-waypoint OHLC + inter-bar gap trajectory), `RsiThreshold` (Wilder `RSI`),
//! `MacdHistogramZeroCross` (TA-Lib `MACD`), and `DrawingLine` (`HorizontalLine`, finite `TrendLine`,
//! and $360^\circ$ `Ray` relative-offset crossings) deterministically across live feeds and Bar Replay,
//! with bit-exact causal rewind (`Once` alerts automatically re-arm when rewinding before their trigger!).

pub mod engine;
#[cfg(test)]
mod tests;

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::data::Timeframe;
use crate::drawings::DrawingId;

pub use engine::{
    AlertEngine, MAX_ALERT_RULES, MAX_MESSAGE_BYTES, MAX_TRIGGER_LOG_EVENTS,
    drawing_line_value_at_bar, evaluate_4_waypoint_cross,
};

/// Stable unique identifier assigned monotonically to each [`AlertRule`].
pub type AlertId = u64;

/// Directional crossing predicate evaluated across causal bar trajectories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum CrossDirection {
    #[default]
    CrossAbove,
    CrossBelow,
    CrossAny,
}

impl CrossDirection {
    pub const ALL: [Self; 3] = [Self::CrossAbove, Self::CrossBelow, Self::CrossAny];

    #[inline]
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::CrossAbove => "Above ▲",
            Self::CrossBelow => "Below ▼",
            Self::CrossAny => "Cross <>",
        }
    }

    #[inline]
    #[must_use]
    pub const fn symbol_glyph(self) -> &'static str {
        match self {
            Self::CrossAbove => "▲",
            Self::CrossBelow => "▼",
            Self::CrossAny => "<>",
        }
    }
}

/// Firing cadence of an [`AlertRule`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum AlertFrequency {
    /// Fires at most once on the first causal bar after `armed_at_timestamp` and transitions to
    /// [`AlertStatus::Triggered`] (automatically re-arming if Bar Replay rewinds earlier!).
    #[default]
    Once,
    /// Remains [`AlertStatus::Armed`] and logs at most one [`AlertTriggerEvent`] per distinct bar timestamp.
    OncePerBarClose,
}

impl AlertFrequency {
    pub const ALL: [Self; 2] = [Self::Once, Self::OncePerBarClose];

    #[inline]
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::Once => "Once",
            Self::OncePerBarClose => "Every Bar",
        }
    }
}

/// Lifecycle state of an [`AlertRule`] at the current causal frontier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum AlertStatus {
    #[default]
    Armed,
    Triggered,
    Paused,
}

impl AlertStatus {
    #[inline]
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::Armed => "ARMED",
            Self::Triggered => "TRIG",
            Self::Paused => "PAUSED",
        }
    }
}

/// Typed causal trigger condition for an [`AlertRule`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AlertCondition {
    PriceLevel {
        price: f64,
        direction: CrossDirection,
    },
    RsiThreshold {
        period: usize,
        level: f64,
        direction: CrossDirection,
    },
    MacdHistogramZeroCross {
        fast: usize,
        slow: usize,
        signal: usize,
        direction: CrossDirection,
    },
    DrawingLine {
        drawing_id: DrawingId,
        direction: CrossDirection,
    },
}

impl AlertCondition {
    #[inline]
    #[must_use]
    pub const fn direction(&self) -> CrossDirection {
        match *self {
            Self::PriceLevel { direction, .. }
            | Self::RsiThreshold { direction, .. }
            | Self::MacdHistogramZeroCross { direction, .. }
            | Self::DrawingLine { direction, .. } => direction,
        }
    }

    #[must_use]
    pub fn summary(&self) -> String {
        match *self {
            Self::PriceLevel { price, direction } => {
                format!("Price {} {price:.2}", direction.short_label())
            }
            Self::RsiThreshold {
                period,
                level,
                direction,
            } => {
                format!("RSI({period}) {} {level:.1}", direction.short_label())
            }
            Self::MacdHistogramZeroCross {
                fast,
                slow,
                signal,
                direction,
            } => {
                format!("MACD({fast},{slow},{signal}) {} 0", direction.short_label())
            }
            Self::DrawingLine {
                drawing_id,
                direction,
            } => {
                format!("Drawing #{drawing_id} {}", direction.short_label())
            }
        }
    }
}

/// Persistent causal alert rule scoped to an instrument.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlertRule {
    pub id: AlertId,
    pub symbol: String,
    pub timeframe: Timeframe,
    pub condition: AlertCondition,
    pub frequency: AlertFrequency,
    pub status: AlertStatus,
    /// Exact base-series UTC timestamp at which the alert was armed (`bar.timestamp > armed_at_timestamp` required).
    pub armed_at_timestamp: i64,
    /// Bucket-start UTC timestamp on `timeframe` at which the alert was armed.
    pub armed_bucket_timestamp: i64,
    /// `Some(cutoff_ts)` when authored inside an active Bar Replay session; `None` when authored live.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at_timestamp: Option<i64>,
    pub trigger_count: usize,
    pub last_triggered_timestamp: Option<i64>,
    pub message: String,
}

impl AlertRule {
    /// Returns `true` iff this alert rule existed at or before `replay_cutoff_timestamp`.
    #[inline]
    #[must_use]
    pub const fn is_causally_visible(&self, replay_cutoff_timestamp: Option<i64>) -> bool {
        match (replay_cutoff_timestamp, self.created_at_timestamp) {
            (Some(cutoff), Some(created)) => created <= cutoff,
            _ => true,
        }
    }
}

/// Immutable causal trigger record emitted into [`AlertEngine::triggers`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlertTriggerEvent {
    pub event_id: u64,
    pub alert_id: AlertId,
    pub symbol: String,
    pub timeframe: Timeframe,
    pub bar_timestamp: i64,
    /// Exact base-series UTC timestamp at which this trigger fired (`[MEDIUM-01]`).
    #[serde(default)]
    pub eval_base_timestamp: i64,
    pub threshold_value: f64,
    pub observed_value: f64,
    pub direction: CrossDirection,
    pub condition_summary: String,
    pub message: String,
}

/// Truncates `s` to at most `max_bytes` bytes on a valid UTF-8 code-point boundary (`[CRITICAL-01]`),
/// preventing panics when multi-byte characters (Vietnamese diacritics or directional glyphs) straddle `max_bytes`.
#[inline]
pub fn truncate_utf8_bytes(s: &mut String, max_bytes: usize) {
    if s.len() > max_bytes {
        let cut = s.floor_char_boundary(max_bytes);
        s.truncate(cut);
    }
}

/// Validation errors returned when creating or mutating an [`AlertRule`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlertError {
    InvalidPriceLevel,
    InvalidIndicatorParams,
    DrawingNotFound(DrawingId),
    UnsupportedDrawingGeometry,
    UnsupportedDrawingPane,
    EmptySeries,
}

impl fmt::Display for AlertError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPriceLevel => {
                write!(f, "alert price level must be finite and within (0, 1e15]")
            }
            Self::InvalidIndicatorParams => write!(
                f,
                "invalid indicator alert parameters (check RSI period/level or MACD periods)"
            ),
            Self::DrawingNotFound(id) => write!(f, "selected drawing #{id} not found"),
            Self::UnsupportedDrawingGeometry => write!(
                f,
                "drawing alerts require a Horizontal Line, Trend Line, or Ray"
            ),
            Self::UnsupportedDrawingPane => {
                write!(f, "drawing alerts require a Main price-pane line or ray")
            }
            Self::EmptySeries => write!(f, "cannot arm alert on an empty candle series"),
        }
    }
}

impl std::error::Error for AlertError {}
