//! Pure `f64`/`i64`, `wasm32-unknown-unknown`-compatible Synchronized Multi-Chart Grid Engine
//! (`Milestone R12 / Wave 11`).
//!
//! Provides synchronized multi-chart grid geometry (`1x1`, `1x2`, `2x1`, `2x2`) with:
//! - Deterministic finite grid subdivision (`1x1`, `1x2`, `2x1`, `2x2`).
//! - Independent per-chart `PriceScale` preservation while synchronizing horizontal time windows
//!   (`SyncedTimeWindow`) and causal crosshair timestamps (`resolve_linked_crosshair_bar`).
//! - Strict `DEF-001` multi-timeframe causal closure (`causal_closed_prefix_len`) preventing secondary
//!   higher-timeframe slots from leaking future intra-week/intra-hour closes during Bar Replay.

use serde::{Deserialize, Serialize};

use crate::model::Candle;

use super::pane_viewport::PlotRect;
use super::time_scale::TimeScale;

/// Maximum number of chart slots supported in a synchronized workstation grid (`2x2`).
pub const MAX_MULTI_CHART_SLOTS: usize = 4;

/// Grid layout arrangement for multi-chart analysis (`1x1`, `1x2`, `2x1`, `2x2`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum MultiChartLayoutKind {
    #[default]
    Single1x1,
    SplitVertical1x2,
    SplitHorizontal2x1,
    Quad2x2,
}

impl MultiChartLayoutKind {
    pub const ALL: [Self; 4] = [
        Self::Single1x1,
        Self::SplitVertical1x2,
        Self::SplitHorizontal2x1,
        Self::Quad2x2,
    ];

    /// Total visible chart slots rendered by this layout (`1`, `2`, `2`, or `4`).
    #[inline]
    #[must_use]
    pub const fn slot_count(self) -> usize {
        match self {
            Self::Single1x1 => 1,
            Self::SplitVertical1x2 | Self::SplitHorizontal2x1 => 2,
            Self::Quad2x2 => 4,
        }
    }

    /// Grid dimensions `(rows, cols)`.
    #[inline]
    #[must_use]
    pub const fn rows_cols(self) -> (usize, usize) {
        match self {
            Self::Single1x1 => (1, 1),
            Self::SplitVertical1x2 => (1, 2),
            Self::SplitHorizontal2x1 => (2, 1),
            Self::Quad2x2 => (2, 2),
        }
    }

    #[inline]
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::Single1x1 => "1x1",
            Self::SplitVertical1x2 => "1x2",
            Self::SplitHorizontal2x1 => "2x1",
            Self::Quad2x2 => "2x2",
        }
    }

    #[inline]
    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::Single1x1 => "Single Chart (1x1)",
            Self::SplitVertical1x2 => "Side-by-Side Split (1x2)",
            Self::SplitHorizontal2x1 => "Stacked Split (2x1)",
            Self::Quad2x2 => "Quad Matrix (2x2)",
        }
    }

    /// Parses canonical labels (`"1x1"`, `"1x2"`, `"2x1"`, `"2x2"` or variant names).
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        let norm = raw.trim().to_ascii_lowercase();
        match norm.as_str() {
            "1x1" | "single1x1" | "single" => Some(Self::Single1x1),
            "1x2" | "splitvertical1x2" | "vertical" => Some(Self::SplitVertical1x2),
            "2x1" | "splithorizontal2x1" | "horizontal" => Some(Self::SplitHorizontal2x1),
            "2x2" | "quad2x2" | "quad" => Some(Self::Quad2x2),
            _ => None,
        }
    }

    /// Subdivides `outer` into `slot_count()` row-major finite [`PlotRect`] cells separated by `gap`.
    ///
    /// Guarantees `width >= 0.0`, `height >= 0.0`, finite coordinates, and non-overlapping cells
    /// contained inside `outer` even for microscopic (`<= gap`) outer containers.
    #[must_use]
    pub fn compute_grid_cell_rects(self, outer: PlotRect, gap: f64) -> Vec<PlotRect> {
        let left = if outer.left.is_finite() {
            outer.left
        } else {
            0.0
        };
        let top = if outer.top.is_finite() {
            outer.top
        } else {
            0.0
        };
        let total_w = if outer.width.is_finite() {
            outer.width.max(0.0)
        } else {
            0.0
        };
        let total_h = if outer.height.is_finite() {
            outer.height.max(0.0)
        } else {
            0.0
        };
        let safe_gap = if gap.is_finite() { gap.max(0.0) } else { 0.0 };

        let (rows, cols) = self.rows_cols();
        let eff_gap_x = if cols > 1 {
            safe_gap.min(total_w / (cols - 1) as f64)
        } else {
            0.0
        };
        let eff_gap_y = if rows > 1 {
            safe_gap.min(total_h / (rows - 1) as f64)
        } else {
            0.0
        };

        let cell_w = ((total_w - eff_gap_x * (cols - 1) as f64) / cols as f64).max(0.0);
        let cell_h = ((total_h - eff_gap_y * (rows - 1) as f64) / rows as f64).max(0.0);

        let mut rects = Vec::with_capacity(rows * cols);
        for r in 0..rows {
            let y = top + (r as f64) * (cell_h + eff_gap_y);
            let h = if r + 1 == rows {
                (top + total_h - y).max(0.0)
            } else {
                cell_h
            };
            for c in 0..cols {
                let x = left + (c as f64) * (cell_w + eff_gap_x);
                let w = if c + 1 == cols {
                    (left + total_w - x).max(0.0)
                } else {
                    cell_w
                };
                rects.push(PlotRect::new(x, y, w, h));
            }
        }
        rects
    }
}

/// Toggle flags governing which dimensions synchronize automatically across visible grid slots.
///
/// Vertical `PriceScale`s are **never** synchronized across slots so multi-symbol and
/// multi-indicator charts retain independent autoscaling (`LWC v5.2.0` per-pane scale isolation).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct MultiChartSyncConfig {
    pub sync_crosshair: bool,
    pub sync_time_range: bool,
    pub sync_symbol: bool,
    pub sync_timeframe: bool,
}

impl Default for MultiChartSyncConfig {
    fn default() -> Self {
        Self {
            sync_crosshair: true,
            sync_time_range: true,
            sync_symbol: false,
            sync_timeframe: false,
        }
    }
}

/// Returns the strictly causal prefix length of `candles` whose completed bar close timestamps
/// `c.timestamp + slot_interval_secs - 1 <= replay_frontier_close_ts` (`DEF-001` safeguard).
///
/// When `replay_frontier_close_ts` is `None` (live workstation mode), returns `candles.len()`.
#[must_use]
pub fn causal_closed_prefix_len(
    candles: &[Candle],
    slot_interval_secs: i64,
    replay_frontier_close_ts: Option<i64>,
) -> usize {
    let Some(fc) = replay_frontier_close_ts else {
        return candles.len();
    };
    let span = slot_interval_secs.max(1).saturating_sub(1);
    candles.partition_point(|c| c.timestamp.saturating_add(span) <= fc)
}

/// Resolves the causal bar index in `candles` corresponding to a linked crosshair `target_timestamp`.
///
/// - When `replay_frontier_close_ts` is `Some(fc)`, restricts eligibility strictly to closed bars
///   whose completion `c.timestamp + slot_interval_secs - 1 <= fc` via [`causal_closed_prefix_len`],
///   and clamps `target_timestamp` to `<= fc` (`DEF-001`).
/// - Matches an exact bar containing `target_timestamp` in `[c.timestamp, c.timestamp + slot_interval_secs)`,
///   or snaps to the nearest neighbor within `max(4 * slot_interval_secs, 4 days)` across weekend/session
///   gaps; returns `None` when no causal bar falls within tolerance.
#[must_use]
pub fn resolve_linked_crosshair_bar(
    candles: &[Candle],
    slot_interval_secs: i64,
    target_timestamp: i64,
    replay_frontier_close_ts: Option<i64>,
) -> Option<usize> {
    if candles.is_empty() {
        return None;
    }
    let interval = slot_interval_secs.max(1);
    let search_len = causal_closed_prefix_len(candles, interval, replay_frontier_close_ts);
    if search_len == 0 {
        return None;
    }

    let query_ts = match replay_frontier_close_ts {
        Some(fc) => target_timestamp.min(fc),
        None => target_timestamp,
    };

    let slice = &candles[..search_len];
    let upper = slice.partition_point(|c| c.timestamp <= query_ts);
    if upper > 0 {
        let left_idx = upper - 1;
        let left_ts = slice[left_idx].timestamp;
        if query_ts < left_ts.saturating_add(interval) {
            return Some(left_idx);
        }
    }

    let tolerance = interval.saturating_mul(4).max(4 * 86_400);
    let mut best: Option<(usize, i128)> = None;
    if upper > 0 {
        let left_idx = upper - 1;
        let dist = (i128::from(query_ts) - i128::from(slice[left_idx].timestamp)).abs();
        if dist <= i128::from(tolerance) {
            best = Some((left_idx, dist));
        }
    }
    if upper < search_len {
        let right_idx = upper;
        let dist = (i128::from(slice[right_idx].timestamp) - i128::from(query_ts)).abs();
        if dist <= i128::from(tolerance) && best.is_none_or(|(_, prev_dist)| dist < prev_dist) {
            best = Some((right_idx, dist));
        }
    }
    best.map(|(idx, _)| idx)
}

/// Time-domain viewport descriptor extracted from an active chart for cross-slot pan/zoom linking.
///
/// Encodes UTC `[start_ts, end_ts]` over historical data bars plus `right_whitespace_bars_frac`
/// (the fraction of viewport width reserved to the right of the final candle, e.g. for Ichimoku
/// `+26` future Kumo projection) so both equal-timeframe and mixed-timeframe slots align without
/// distorting right margins or touching `PriceScale`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SyncedTimeWindow {
    pub start_ts: i64,
    pub end_ts: i64,
    pub right_whitespace_bars_frac: f64,
}

#[inline]
fn logical_index_to_utc_ts(candles: &[Candle], slot_interval_secs: i64, idx_f: f64) -> Option<i64> {
    if candles.is_empty() || !idx_f.is_finite() {
        return None;
    }
    let step = slot_interval_secs.max(1) as f64;
    let n = candles.len();
    let ts_f = if idx_f <= 0.0 || n == 1 {
        (candles[0].timestamp as f64) + idx_f * step
    } else if idx_f >= (n - 1) as f64 {
        (candles[n - 1].timestamp as f64) + (idx_f - (n - 1) as f64) * step
    } else {
        let i = idx_f.floor() as usize;
        let frac = idx_f - (i as f64);
        let t0 = candles[i].timestamp as f64;
        let t1 = candles[i + 1].timestamp as f64;
        let span = if t1 > t0 { t1 - t0 } else { step };
        t0 + frac * span
    };
    ts_f.is_finite().then_some(ts_f.round() as i64)
}

#[inline]
fn utc_ts_to_logical_index(candles: &[Candle], slot_interval_secs: i64, ts: i64) -> Option<f64> {
    if candles.is_empty() {
        return None;
    }
    let step = slot_interval_secs.max(1) as f64;
    let n = candles.len();
    let first_ts = candles[0].timestamp as f64;
    if ts <= candles[0].timestamp || n == 1 {
        return Some(((ts as f64) - first_ts) / step);
    }
    let last_ts = candles[n - 1].timestamp as f64;
    if ts >= candles[n - 1].timestamp {
        return Some(((n - 1) as f64) + ((ts as f64) - last_ts) / step);
    }
    let upper = candles.partition_point(|c| c.timestamp <= ts);
    let i = upper.saturating_sub(1).min(n - 2);
    let t0 = candles[i].timestamp as f64;
    let t1 = candles[i + 1].timestamp as f64;
    let frac = if t1 > t0 {
        (((ts as f64) - t0) / (t1 - t0)).clamp(0.0, 1.0)
    } else {
        0.0
    };
    Some((i as f64) + frac)
}

/// Extracts the visible UTC time window from `(candles, time_scale)`.
#[must_use]
pub fn extract_visible_time_window(
    candles: &[Candle],
    slot_interval_secs: i64,
    time_scale: &TimeScale,
) -> Option<SyncedTimeWindow> {
    if candles.is_empty() {
        return None;
    }
    let logical = time_scale.visible_logical_range()?;
    let total_bars_len = (logical.right - logical.left + 1.0).max(1.0);
    let last_idx = (candles.len() - 1) as f64;

    if logical.right > last_idx {
        let right_ws_bars = (logical.right - last_idx).max(0.0);
        // Allow up to 96% right whitespace (instead of clamping hard at 85%) so `Sync T`
        // preserves extreme future-whitespace pans (e.g. Ichimoku +26 or wide zoom) (`BUG-022`).
        let right_whitespace_bars_frac = (right_ws_bars / total_bars_len).clamp(0.0, 0.96);
        let start_ts = logical_index_to_utc_ts(candles, slot_interval_secs, logical.left)?;
        let end_ts = candles[candles.len() - 1].timestamp;
        Some(SyncedTimeWindow {
            start_ts: start_ts.min(end_ts),
            end_ts,
            right_whitespace_bars_frac,
        })
    } else {
        let start_ts = logical_index_to_utc_ts(candles, slot_interval_secs, logical.left)?;
        let end_ts = logical_index_to_utc_ts(candles, slot_interval_secs, logical.right)?;
        Some(SyncedTimeWindow {
            start_ts: start_ts.min(end_ts),
            end_ts: end_ts.max(start_ts),
            right_whitespace_bars_frac: 0.0,
        })
    }
}

/// Applies `window` onto a target slot's `TimeScale` (`set_logical_range`) while leaving the
/// target slot's vertical `PriceScale` untouched. Returns `true` when applied cleanly.
pub fn apply_synced_time_window(
    candles: &[Candle],
    slot_interval_secs: i64,
    time_scale: &mut TimeScale,
    window: &SyncedTimeWindow,
) -> bool {
    if candles.is_empty() || time_scale.is_empty() || !window.right_whitespace_bars_frac.is_finite()
    {
        return false;
    }
    let Some(left_idx) = utc_ts_to_logical_index(candles, slot_interval_secs, window.start_ts)
    else {
        return false;
    };
    let Some(data_right_idx) = utc_ts_to_logical_index(candles, slot_interval_secs, window.end_ts)
    else {
        return false;
    };
    let frac = window.right_whitespace_bars_frac.clamp(0.0, 0.96);
    let right_idx = if frac > 0.0 {
        let data_span = (data_right_idx - left_idx + 1.0).max(1.0);
        let full_span = data_span / (1.0 - frac).max(0.04);
        left_idx + full_span - 1.0
    } else {
        data_right_idx.max(left_idx + 0.5)
    };
    if !left_idx.is_finite() || !right_idx.is_finite() || right_idx < left_idx {
        return false;
    }
    time_scale.set_logical_range(left_idx, right_idx);
    true
}
