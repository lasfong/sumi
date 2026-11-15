//! Modular financial chart viewport & coordinate transformation engine (`Milestone R03`).
//!
//! Architecture verified against Lightweight Charts v5.2.0 (`868cae2`):
//! - [`TimeScale`]: Shared horizontal scale across all panes (`bar_spacing`, `right_offset`, cursor-anchored `zoom`,
//!   sub-pixel `scroll_to`, `set_width`, `fit_content`, O(1) `culling_data_range`, power-of-2 `conflation_factor`).
//! - [`PriceScale`]: Independent vertical scale per pane and per axis (`Normal`, `Logarithmic` with adaptive
//!   [`LogFormula`], `Percentage`, `IndexedTo100`, autoscale, vertical drag-scale & pan).
//! - [`PaneViewport`]: Zero-cost 3-space coordinate bridge (`Logical/Data <-> PaneLocal <-> Screen`).
//! - [`KineticAnimation`]: Exponential-decay inertial panning on `right_offset` bar units (`MaxStartDelay = 50ms`).
//! - [`candle_metrics`]: LWC `optimal_candlestick_width_px`, wick parity alignment, and zero-allocation OHLCV conflation.

pub mod candle_metrics;
pub mod kinetic;
pub mod multi_chart;
pub mod pane_viewport;
pub mod price_scale;
pub mod time_scale;
pub mod workspace_template;

#[cfg(test)]
mod tests;

pub use candle_metrics::{
    CandlestickBarMetrics, ConflatedCandle, ConflatedCandlesIter, conflated_candle_at,
    iter_conflated_candles, optimal_bar_width_px, optimal_candlestick_width_px,
};
pub use kinetic::{KineticAnimation, KineticScrollConfig};
pub use multi_chart::{
    MAX_MULTI_CHART_SLOTS, MultiChartLayoutKind, MultiChartSyncConfig, SyncedTimeWindow,
    apply_synced_time_window, causal_closed_prefix_len, extract_visible_time_window,
    resolve_linked_crosshair_bar,
};
pub use pane_viewport::{PaneViewport, PlotRect};
pub use price_scale::{
    LogFormula, PriceAxisFormatMode, PriceRange, PriceScale, PriceScaleMode, PriceScaleOptions,
    PriceTickMark, format_compact_volume_axis, format_price_adaptive, from_indexed_to_100,
    from_log, from_percent, to_indexed_to_100, to_log, to_percent,
};
pub use time_scale::{
    HorzScaleOptions, LogicalRange, StrictBarRange, TimeScale, TransitionSnapshot,
};
pub use workspace_template::{
    WORKSPACE_TEMPLATE_SCHEMA_VERSION, WorkspacePaneStretches, WorkspaceSlotTemplate,
    WorkspaceTemplate, WorkspaceTemplateError, builtin_workspace_templates,
};

/// Panic-free clamp matching Lightweight Charts `src/helpers/mathex.ts` (`Math.min(Math.max(v, min), max)`).
///
/// Unlike Rust's `f64::clamp(min, max)` — which panics at runtime whenever `min > max` (e.g. `total_bars == 1`
/// with `fix_left_edge && fix_right_edge` or narrow viewport widths) — `lwc_clamp` resolves `min > max` to `max_val`
/// cleanly and deterministically.
#[inline]
#[must_use]
pub fn lwc_clamp(value: f64, min_val: f64, max_val: f64) -> f64 {
    value.max(min_val).min(max_val)
}
