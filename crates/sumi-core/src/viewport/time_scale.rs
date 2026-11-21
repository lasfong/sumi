use serde::{Deserialize, Serialize};
use std::ops::Range;

use super::lwc_clamp;

/// Options governing the shared horizontal time axis (`TimeScale`).
///
/// Mirrors Lightweight Charts v5.2.0 `HorzScaleOptions` (`src/model/time-scale.ts`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HorzScaleOptions {
    /// Signed bar offset from the right edge (`> 0` = future whitespace bars, `< 0` = scrolled into past).
    pub right_offset: f64,
    /// Optional pixel-based right margin that overrides `right_offset` proportionally on zoom.
    pub right_offset_pixels: Option<f64>,
    /// Default horizontal slot width per bar in logical pixels.
    pub bar_spacing: f64,
    /// Minimum allowed `bar_spacing` when zooming out (default `0.5` px/bar).
    pub min_bar_spacing: f64,
    /// Explicit maximum `bar_spacing` (`<= 0.0` means `0.5 * width`, matching LWC `_maxBarSpacing`).
    pub max_bar_spacing: f64,
    /// Minimum number of data bars kept inside the viewport when scrolling to edges (`MinVisibleBarsCount = 2`).
    pub min_visible_bars_count: usize,
    /// Prevents scrolling left past bar `0`.
    pub fix_left_edge: bool,
    /// Prevents scrolling right into empty future whitespace (`max_right_offset = 0`).
    pub fix_right_edge: bool,
    /// Preserves the visible logical range on window resize by scaling `bar_spacing` proportionally.
    pub lock_visible_time_range_on_resize: bool,
    /// When `true`, zoom anchors at the right edge instead of keeping the bar under the cursor fixed.
    pub right_bar_stays_on_scroll: bool,
    /// Automatically shifts viewport when new realtime bars append while viewing the latest bar.
    pub shift_visible_range_on_new_bar: bool,
    /// Enables power-of-2 OHLCV bar conflation when `bar_spacing` becomes sub-pixel.
    pub enable_conflation: bool,
    /// Multiplier on `(1.0 / device_pixel_ratio)` below which conflation engages.
    pub conflation_threshold_factor: f64,
}

impl Default for HorzScaleOptions {
    fn default() -> Self {
        Self::sumi_default()
    }
}

impl HorzScaleOptions {
    /// Exact Lightweight Charts v5.2.0 upstream defaults (`rightOffset = 0`, `barSpacing = 6`).
    #[must_use]
    pub fn lwc_default() -> Self {
        Self {
            right_offset: 0.0,
            right_offset_pixels: None,
            bar_spacing: 6.0,
            min_bar_spacing: 0.5,
            max_bar_spacing: 0.0,
            min_visible_bars_count: 2,
            fix_left_edge: false,
            fix_right_edge: false,
            lock_visible_time_range_on_resize: false,
            right_bar_stays_on_scroll: false,
            shift_visible_range_on_new_bar: true,
            enable_conflation: true,
            conflation_threshold_factor: 1.0,
        }
    }

    /// Sumi Workstation defaults (`right_offset = 12.0` future bars, `bar_spacing = 8.0` px/bar,
    /// `min_bar_spacing = 0.005` so `fit_content` and wide zoom-out work seamlessly up to 100K+ bars
    /// with automatic DPR-aware OHLCV conflation).
    #[must_use]
    pub fn sumi_default() -> Self {
        Self {
            right_offset: 12.0,
            bar_spacing: 8.0,
            min_bar_spacing: 0.005,
            ..Self::lwc_default()
        }
    }
}

/// Continuous floating-point logical bar range `[left, right]`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LogicalRange {
    pub left: f64,
    pub right: f64,
}

impl LogicalRange {
    #[must_use]
    pub fn span(&self) -> f64 {
        self.right - self.left
    }
}

/// Integer bar index bounds `[left, right]` rounded outward (`floor(left)..=ceil(right)`).
///
/// Mirrors LWC `TimeScaleVisibleRange::strictRange` (`src/model/time-scale-visible-range.ts:11-20`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StrictBarRange {
    pub left: i64,
    pub right: i64,
}

impl StrictBarRange {
    #[must_use]
    pub fn count(&self) -> i64 {
        let diff = i128::from(self.right) - i128::from(self.left) + 1;
        diff.clamp(0, i128::from(i64::MAX)) as i64
    }
}

/// Saved transition state during an active horizontal drag-scroll or time-axis drag-scale gesture.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TransitionSnapshot {
    pub bar_spacing: f64,
    pub right_offset: f64,
}

/// Shared horizontal time axis state across all panes in a chart workspace.
///
/// Coordinates accepted and emitted by `TimeScale` are **pane-local** horizontal logical pixels
/// in `[0.0, width)` (origin `0.0` at the left edge of the pane plot area).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeScale {
    options: HorzScaleOptions,
    width: f64,
    base_index: Option<i64>,
    total_bars: usize,
    right_offset: f64,
    bar_spacing: f64,
    scroll_start_local_x: Option<f64>,
    scale_start_local_x: Option<f64>,
    transition_start: Option<TransitionSnapshot>,
}

impl TimeScale {
    /// Creates a new `TimeScale` before plot width or series data are attached.
    #[must_use]
    pub fn new(options: HorzScaleOptions) -> Self {
        let initial_bs = sanitize_initial_spacing(options.bar_spacing);
        let initial_ro = if let Some(px) = options.right_offset_pixels {
            if px.is_finite() { px / initial_bs } else { 0.0 }
        } else if options.right_offset.is_finite() {
            options.right_offset
        } else {
            0.0
        };

        Self {
            options,
            width: 0.0,
            base_index: None,
            total_bars: 0,
            right_offset: initial_ro,
            bar_spacing: initial_bs,
            scroll_start_local_x: None,
            scale_start_local_x: None,
            transition_start: None,
        }
    }

    /// Sanitizes state restored from persistent storage (`serde`) against NaN/Infinity or corrupted fields.
    pub fn sanitize_deserialized(&mut self) {
        self.scroll_start_local_x = None;
        self.scale_start_local_x = None;
        self.transition_start = None;

        if !self.options.bar_spacing.is_finite() || self.options.bar_spacing <= 0.0 {
            self.options.bar_spacing = 8.0;
        }
        if !self.options.min_bar_spacing.is_finite() || self.options.min_bar_spacing <= 0.0 {
            self.options.min_bar_spacing = 0.005;
        }
        if !self.options.max_bar_spacing.is_finite() || self.options.max_bar_spacing < 0.0 {
            self.options.max_bar_spacing = 0.0;
        }
        if !self.options.right_offset.is_finite() {
            self.options.right_offset = 12.0;
        }
        if self
            .options
            .right_offset_pixels
            .is_some_and(|px| !px.is_finite() || px < 0.0)
        {
            self.options.right_offset_pixels = None;
        }
        if !self.options.conflation_threshold_factor.is_finite()
            || self.options.conflation_threshold_factor <= 0.0
        {
            self.options.conflation_threshold_factor = 1.0;
        }
        self.options.min_visible_bars_count = self.options.min_visible_bars_count.max(1);

        if !self.width.is_finite() || self.width < 0.0 {
            self.width = 0.0;
        }
        if !self.bar_spacing.is_finite() || self.bar_spacing <= 0.0 {
            self.bar_spacing = sanitize_initial_spacing(self.options.bar_spacing);
        }
        if !self.right_offset.is_finite() {
            self.right_offset = self.options.right_offset;
        }
        self.base_index = self.total_bars.checked_sub(1).map(|idx| idx as i64);

        if !self.is_empty() {
            self.correct_bar_spacing();
            self.correct_offset();
            self.do_fix_left_edge();
        }
    }

    /// Creates a `TimeScale` initialized with `total_bars` and plot `width`.
    #[must_use]
    pub fn with_data(total_bars: usize, width: f64, options: HorzScaleOptions) -> Self {
        let mut ts = Self::new(options);
        ts.total_bars = total_bars;
        ts.base_index = total_bars.checked_sub(1).map(|idx| idx as i64);
        if width.is_finite() && width > 0.0 {
            ts.width = width;
            ts.correct_bar_spacing();
            ts.correct_offset();
            ts.do_fix_left_edge();
        }
        ts
    }

    /// Returns `true` when width/spacing/offset are invalid or no data bars exist (`time-scale.ts:450-452`).
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.width <= 0.0
            || !self.width.is_finite()
            || self.total_bars == 0
            || self.base_index.is_none()
            || !self.bar_spacing.is_finite()
            || self.bar_spacing <= 0.0
            || !self.right_offset.is_finite()
    }

    #[inline]
    #[must_use]
    pub fn width(&self) -> f64 {
        self.width
    }

    #[inline]
    #[must_use]
    pub fn total_bars(&self) -> usize {
        self.total_bars
    }

    #[inline]
    #[must_use]
    pub fn base_index(&self) -> Option<i64> {
        self.base_index
    }

    #[inline]
    #[must_use]
    pub fn bar_spacing(&self) -> f64 {
        self.bar_spacing
    }

    #[inline]
    #[must_use]
    pub fn right_offset(&self) -> f64 {
        self.right_offset
    }

    #[inline]
    #[must_use]
    pub fn options(&self) -> &HorzScaleOptions {
        &self.options
    }

    #[inline]
    #[must_use]
    pub fn is_scrolling(&self) -> bool {
        self.scroll_start_local_x.is_some()
    }

    #[inline]
    #[must_use]
    pub fn is_scaling(&self) -> bool {
        self.scale_start_local_x.is_some()
    }

    /// Updates horizontal options and enforces spacing and edge invariants (`time-scale.ts:381-420`).
    pub fn apply_options(&mut self, options: HorzScaleOptions) {
        let prev_bs_opt = self.options.bar_spacing;
        let prev_ro_opt = self.options.right_offset;
        let prev_rop_opt = self.options.right_offset_pixels;
        self.options = options;

        if (self.options.bar_spacing - prev_bs_opt).abs() > f64::EPSILON
            && self.options.bar_spacing.is_finite()
            && self.options.bar_spacing > 0.0
        {
            self.bar_spacing = self.options.bar_spacing.max(1e-6);
        }
        self.correct_bar_spacing();

        if let Some(px) = self.options.right_offset_pixels {
            if px.is_finite() && self.bar_spacing > 0.0 {
                self.right_offset = px / self.bar_spacing;
            }
        } else if ((self.options.right_offset - prev_ro_opt).abs() > f64::EPSILON
            || prev_rop_opt.is_some())
            && self.options.right_offset.is_finite()
        {
            self.right_offset = self.options.right_offset;
        }

        self.correct_offset();
        if self.options.fix_left_edge {
            self.do_fix_left_edge();
        }
        if self.options.fix_right_edge {
            self.correct_offset();
            self.correct_bar_spacing();
        }
    }

    /// Updates the total bar count of the shared series (`base_index = total_bars - 1`),
    /// honoring LWC `shiftVisibleRangeOnNewBar` semantics (`chart-model.ts:969-980`).
    pub fn set_total_bars(&mut self, total_bars: usize) {
        let old_last_bar_visible =
            if let (Some(old_b), Some(strict)) = (self.base_index, self.visible_strict_range()) {
                old_b >= strict.left && old_b <= strict.right
            } else {
                true
            };
        let prev_base = self.base_index;
        self.total_bars = total_bars;
        self.base_index = total_bars.checked_sub(1).map(|idx| idx as i64);

        if let (Some(old_b), Some(new_b)) = (prev_base, self.base_index)
            && new_b > old_b
        {
            let should_shift = self.options.shift_visible_range_on_new_bar && old_last_bar_visible;
            if !should_shift {
                self.right_offset -= (new_b - old_b) as f64;
            }
        }

        if !self.is_empty() {
            self.correct_bar_spacing();
            self.correct_offset();
            self.do_fix_left_edge();
        }
    }

    /// Updates plot width while preserving invariants (`src/model/time-scale.ts:507-549`).
    pub fn set_width(&mut self, new_width: f64) {
        if !new_width.is_finite() || new_width <= 0.0 {
            return;
        }
        if (self.width - new_width).abs() < f64::EPSILON {
            return;
        }

        let previous_visible_range = self.visible_logical_range();
        let old_width = self.width;
        self.width = new_width;

        if self.options.lock_visible_time_range_on_resize && old_width > 0.0 {
            let scaled = self.bar_spacing * new_width / old_width;
            if scaled.is_finite() {
                self.bar_spacing = scaled.max(1e-6);
            }
        }

        if self.options.fix_left_edge
            && let Some(prev) = previous_visible_range
            && prev.left <= 0.0
            && self.bar_spacing > 0.0
        {
            let delta = old_width - new_width;
            self.right_offset -= (delta / self.bar_spacing).round() + 1.0;
        }

        self.correct_bar_spacing();
        self.correct_offset();
    }

    /// Converts an integer bar `index` to its centered pane-local horizontal coordinate (`x_local`).
    ///
    /// Exact formula from LWC `TimeScale::indexToCoordinate` (`src/model/time-scale.ts:551-560`):
    /// `x = width - (baseIndex + rightOffset - index + 0.5) * barSpacing - 1`
    #[inline]
    #[must_use]
    pub fn index_to_local_x(&self, index: i64) -> Option<f64> {
        self.float_index_to_local_x(index as f64)
    }

    /// Continuous generalization of `index_to_local_x` for fractional/conflated bar centers.
    #[inline]
    #[must_use]
    pub fn float_index_to_local_x(&self, center_index: f64) -> Option<f64> {
        if self.is_empty() || !center_index.is_finite() {
            return None;
        }
        let base = self.base_index? as f64;
        let delta_from_right = base + self.right_offset - center_index;
        Some(self.width - (delta_from_right + 0.5) * self.bar_spacing - 1.0)
    }

    /// Converts a pane-local `local_x` coordinate into LWC's `_coordinateToFloatIndex` value.
    ///
    /// Exact formula from LWC `TimeScale::_coordinateToFloatIndex` (`src/model/time-scale.ts:996-1008`):
    /// `round((baseIndex + rightOffset - (width - 1 - x) / barSpacing) * 1_000_000) / 1_000_000`
    ///
    /// **Note on half-slot semantics:** At the exact center `x = index_to_local_x(i)` of integer bar `i`,
    /// `local_x_to_float_index(x) == i as f64 - 0.5`, so `ceil(float_index) == i`.
    #[inline]
    #[must_use]
    pub fn local_x_to_float_index(&self, local_x: f64) -> Option<f64> {
        if self.is_empty() || !local_x.is_finite() {
            return None;
        }
        let base = self.base_index? as f64;
        let delta_from_right = (self.width - 1.0 - local_x) / self.bar_spacing;
        let index = base + self.right_offset - delta_from_right;
        Some((index * 1_000_000.0).round() / 1_000_000.0)
    }

    /// Continuous inverse of `float_index_to_local_x` (`local_x_to_float_index(x) + 0.5`).
    ///
    /// At the exact center of bar `i`, this returns `i as f64`.
    #[inline]
    #[must_use]
    pub fn local_x_to_center_float_index(&self, local_x: f64) -> Option<f64> {
        if self.is_empty() || !local_x.is_finite() {
            return None;
        }
        let base = self.base_index? as f64;
        let delta_from_right = (self.width - 1.0 - local_x) / self.bar_spacing;
        Some(base + self.right_offset + 0.5 - delta_from_right)
    }

    /// Converts a pane-local `local_x` coordinate to the discrete bar slot index (`ceil(float_index)`).
    ///
    /// Mirrors LWC `TimeScale::coordinateToIndex` (`src/model/time-scale.ts:575-585`).
    #[inline]
    #[must_use]
    pub fn local_x_to_index(&self, local_x: f64) -> Option<i64> {
        self.local_x_to_float_index(local_x)
            .map(|f| f.ceil() as i64)
    }

    /// Snaps a pane-local `local_x` to the nearest available data bar in `0..total_bars`.
    #[inline]
    #[must_use]
    pub fn local_x_to_clamped_data_index(&self, local_x: f64) -> Option<usize> {
        let idx = self.local_x_to_index(local_x)?;
        if self.total_bars == 0 {
            return None;
        }
        let last = (self.total_bars - 1) as i64;
        Some(idx.clamp(0, last) as usize)
    }

    /// Sets `bar_spacing` and enforces clamping + `right_offset_pixels` + offset bounds (`time-scale.ts:599-614`).
    pub fn set_bar_spacing(&mut self, new_bar_spacing: f64) {
        if !new_bar_spacing.is_finite() {
            return;
        }
        let old_bar_spacing = self.bar_spacing;
        self.bar_spacing = new_bar_spacing.max(1e-6);
        self.correct_bar_spacing();

        if self.options.right_offset_pixels.is_some()
            && old_bar_spacing > 0.0
            && self.bar_spacing > 0.0
        {
            self.right_offset = self.right_offset * old_bar_spacing / self.bar_spacing;
        }

        self.correct_offset();
    }

    /// Sets `right_offset` (in bars) and clamps to `[min_right_offset, max_right_offset]` (`time-scale.ts:587-593`).
    pub fn set_right_offset(&mut self, offset: f64) {
        if !offset.is_finite() {
            return;
        }
        self.right_offset = offset;
        self.correct_offset();
    }

    /// Cursor-anchored zoom around `local_zoom_x` by `scale` (`scale > 0` = zoom in, `scale < 0` = zoom out).
    ///
    /// Mirrors `ChartModel::zoomTime` (`src/model/chart-model.ts:829-841`) +
    /// `TimeScale::zoom` (`src/model/time-scale.ts:732-745`).
    pub fn zoom(&mut self, local_zoom_x: f64, scale: f64) {
        if self.is_empty() || !local_zoom_x.is_finite() || !scale.is_finite() || scale == 0.0 {
            return;
        }

        let zoom_point = lwc_clamp(local_zoom_x, 1.0, self.width);
        let Some(float_index_at_zoom_point) = self.local_x_to_float_index(zoom_point) else {
            return;
        };

        let bar_spacing = self.bar_spacing;
        let new_bar_spacing = (bar_spacing + scale * (bar_spacing / 10.0)).max(1e-4);
        self.set_bar_spacing(new_bar_spacing);

        if !self.options.right_bar_stays_on_scroll
            && let Some(after_float_index) = self.local_x_to_float_index(zoom_point)
        {
            let corrected = self.right_offset + (float_index_at_zoom_point - after_float_index);
            self.set_right_offset(corrected);
        }
    }

    /// Begins a horizontal drag-scale gesture on the time axis (`time-scale.ts:747-762`).
    pub fn start_scale(&mut self, local_x: f64) {
        if self.is_scrolling() {
            self.end_scroll();
        }
        if self.scale_start_local_x.is_some()
            || self.transition_start.is_some()
            || self.is_empty()
            || !local_x.is_finite()
        {
            return;
        }
        self.scale_start_local_x = Some(local_x);
        self.transition_start = Some(TransitionSnapshot {
            bar_spacing: self.bar_spacing,
            right_offset: self.right_offset,
        });
    }

    /// Updates `bar_spacing` during a horizontal time-axis drag-scale gesture (`time-scale.ts:764-778`).
    pub fn scale_to(&mut self, local_x: f64) {
        let (Some(start_x), Some(snapshot)) = (self.scale_start_local_x, self.transition_start)
        else {
            return;
        };
        if !local_x.is_finite() || self.is_empty() {
            return;
        }
        let start_length_from_right = lwc_clamp(self.width - local_x, 0.0, self.width);
        let current_length_from_right = lwc_clamp(self.width - start_x, 0.0, self.width);
        if start_length_from_right <= 0.0 || current_length_from_right <= 0.0 {
            return;
        }
        self.set_bar_spacing(
            snapshot.bar_spacing * start_length_from_right / current_length_from_right,
        );
    }

    /// Ends an active horizontal time-axis drag-scale gesture (`time-scale.ts:780-787`).
    pub fn end_scale(&mut self) {
        self.scale_start_local_x = None;
        self.transition_start = None;
    }

    /// Begins a horizontal drag-pan gesture at `local_x` (`time-scale.ts:789-800`).
    pub fn start_scroll(&mut self, local_x: f64) {
        if self.scroll_start_local_x.is_some()
            || self.transition_start.is_some()
            || self.is_empty()
            || !local_x.is_finite()
        {
            return;
        }
        self.scroll_start_local_x = Some(local_x);
        self.transition_start = Some(TransitionSnapshot {
            bar_spacing: self.bar_spacing,
            right_offset: self.right_offset,
        });
    }

    /// Updates `right_offset` during a horizontal drag-pan gesture (`time-scale.ts:802-813`).
    pub fn scroll_to(&mut self, local_x: f64) {
        let (Some(start_x), Some(snapshot)) = (self.scroll_start_local_x, self.transition_start)
        else {
            return;
        };
        if !local_x.is_finite() || self.bar_spacing <= 0.0 {
            return;
        }
        let shift_in_logical = (start_x - local_x) / self.bar_spacing;
        self.right_offset = snapshot.right_offset + shift_in_logical;
        self.correct_offset();
    }

    /// Ends an active horizontal drag-pan gesture (`time-scale.ts:815-822`).
    pub fn end_scroll(&mut self) {
        self.scroll_start_local_x = None;
        self.transition_start = None;
    }

    /// One-shot pixel scroll matching `ChartModel::scrollChart` (`chart-model.ts:843-847`).
    pub fn scroll_by_pixels(&mut self, delta_px: f64) {
        if self.is_empty() || !delta_px.is_finite() || delta_px == 0.0 {
            return;
        }
        if self.is_scrolling() {
            self.end_scroll();
        }
        self.start_scroll(0.0);
        self.scroll_to(delta_px);
        self.end_scroll();
    }

    /// Shifts `right_offset` directly by `delta_bars` (`< 0` = scroll to past, `> 0` = scroll to future).
    pub fn scroll_by_bars(&mut self, delta_bars: f64) {
        if self.is_empty() || !delta_bars.is_finite() {
            return;
        }
        self.set_right_offset(self.right_offset + delta_bars);
    }

    /// Restores default `bar_spacing` and `right_offset` (`time-scale.ts:702-714`).
    pub fn restore_default(&mut self) {
        self.set_bar_spacing(self.options.bar_spacing);
        let new_offset = if let Some(px) = self.options.right_offset_pixels {
            if px.is_finite() && self.bar_spacing > 0.0 {
                px / self.bar_spacing
            } else {
                0.0
            }
        } else {
            self.options.right_offset
        };
        self.set_right_offset(new_offset);
    }

    /// Applies an explicit visible bar range `[left, right]` (`time-scale.ts:878-892`).
    pub fn set_visible_range(&mut self, left: f64, right: f64, apply_default_offset: bool) {
        if self.is_empty() || !left.is_finite() || !right.is_finite() || right < left {
            return;
        }
        let Some(base) = self.base_index else {
            return;
        };
        let length = (right - left + 1.0).max(1e-6);
        let pixel_offset = if apply_default_offset {
            self.options
                .right_offset_pixels
                .filter(|px| px.is_finite() && *px >= 0.0 && *px < self.width)
                .unwrap_or(0.0)
        } else {
            0.0
        };

        self.set_bar_spacing((self.width - pixel_offset).max(1.0) / length);
        self.right_offset = right - base as f64;
        if apply_default_offset {
            self.right_offset = if pixel_offset > 0.0 {
                pixel_offset / self.bar_spacing
            } else {
                self.options.right_offset
            };
        }
        self.correct_offset();
    }

    /// Applies a continuous logical range `[from, to]` without overriding `right_offset` with default.
    pub fn set_logical_range(&mut self, from: f64, to: f64) {
        self.set_visible_range(from, to, false);
    }

    /// Fits the entire dataset `[0, total_bars - 1]` plus default `right_offset` into `width` (`time-scale.ts:894-905`).
    pub fn fit_content(&mut self) {
        if self.is_empty() {
            return;
        }
        let first = 0.0;
        let last = (self.total_bars - 1) as f64;
        let right_offset_bars = if self.options.right_offset_pixels.is_none()
            && self.options.right_offset.is_finite()
        {
            self.options.right_offset.max(0.0)
        } else {
            0.0
        };
        self.set_visible_range(first, last + right_offset_bars, true);
    }

    /// Continuous visible logical range `[leftBorder, rightBorder]` (`time-scale.ts:1023-1042`).
    #[must_use]
    pub fn visible_logical_range(&self) -> Option<LogicalRange> {
        if self.is_empty() {
            return None;
        }
        let base = self.base_index? as f64;
        let bars_length = self.width / self.bar_spacing;
        let right_border = base + self.right_offset;
        let left_border = right_border - bars_length + 1.0;
        Some(LogicalRange {
            left: left_border,
            right: right_border,
        })
    }

    /// Outward-rounded integer visible bar range `[floor(left), ceil(right)]` (`time-scale-visible-range.ts:11-20`).
    #[must_use]
    pub fn visible_strict_range(&self) -> Option<StrictBarRange> {
        let logical = self.visible_logical_range()?;
        let left = logical.left.floor() as i64;
        let right = logical.right.ceil() as i64;
        Some(StrictBarRange {
            left: left.min(right),
            right: right.max(left),
        })
    }

    /// Exact slice bounds `start..end` into `0..total_bars` corresponding to `visible_strict_range`
    /// (used for `PriceScale` autoscale and `firstValue` lookup).
    #[must_use]
    pub fn strict_visible_data_range(&self) -> Range<usize> {
        let Some(strict) = self.visible_strict_range() else {
            return 0..0;
        };
        intersect_with_len(strict.left, strict.right, self.total_bars)
    }

    /// O(1) culling slice bounds `start..end` into `0..total_bars` with a 1-bar guard band on each side
    /// so partially visible wide candle bodies at the viewport edges never clip prematurely.
    #[must_use]
    pub fn culling_data_range(&self) -> Range<usize> {
        let Some(strict) = self.visible_strict_range() else {
            return 0..0;
        };
        intersect_with_len(
            strict.left.saturating_sub(1),
            strict.right.saturating_add(1),
            self.total_bars,
        )
    }

    /// Computes LWC's DPR-aware power-of-2 conflation factor in `[1, 512]` (`time-scale.ts:1076-1098`).
    #[must_use]
    pub fn conflation_factor(&self, device_pixel_ratio: f64) -> usize {
        if !self.options.enable_conflation || self.bar_spacing <= 0.0 {
            return 1;
        }
        let dpr = if device_pixel_ratio.is_finite() && device_pixel_ratio > 0.0 {
            device_pixel_ratio
        } else {
            1.0
        };
        let smoothing = if self.options.conflation_threshold_factor.is_finite()
            && self.options.conflation_threshold_factor > 0.0
        {
            self.options.conflation_threshold_factor
        } else {
            1.0
        };
        let adjusted_threshold = (1.0 / dpr) * smoothing;
        if self.bar_spacing >= adjusted_threshold {
            return 1;
        }
        let ratio = adjusted_threshold / self.bar_spacing;
        let exp = ratio.log2().floor().clamp(0.0, 9.0) as u32;
        (1_usize << exp).min(512)
    }

    /// Lower bound on `bar_spacing` (`time-scale.ts:1062-1070`).
    #[must_use]
    pub fn min_bar_spacing(&self) -> f64 {
        let opt_min = if self.options.min_bar_spacing.is_finite() {
            self.options.min_bar_spacing.max(1e-4)
        } else {
            0.5
        };
        if self.options.fix_left_edge
            && self.options.fix_right_edge
            && self.total_bars > 0
            && self.width > 0.0
        {
            (self.width / self.total_bars as f64).max(1e-4)
        } else {
            opt_min
        }
    }

    /// Upper bound on `bar_spacing` (`time-scale.ts:1052-1060`).
    #[must_use]
    pub fn max_bar_spacing(&self) -> f64 {
        if self.options.max_bar_spacing > 0.0 && self.options.max_bar_spacing.is_finite() {
            self.options.max_bar_spacing
        } else if self.width > 0.0 {
            (self.width * 0.5).max(1e-4)
        } else {
            f64::MAX
        }
    }

    /// Minimum allowed `right_offset` preventing scrolling infinitely past bar `0` (`time-scale.ts:1116-1128`).
    #[must_use]
    pub fn min_right_offset(&self) -> Option<f64> {
        let base = self.base_index?;
        if self.total_bars == 0 || self.width <= 0.0 || self.bar_spacing <= 0.0 {
            return None;
        }
        let first_index = 0_i64;
        let min_vis = self.options.min_visible_bars_count.min(self.total_bars) as f64;
        let bars_estimation = if self.options.fix_left_edge {
            self.width / self.bar_spacing
        } else {
            min_vis
        };
        Some((first_index - base - 1) as f64 + bars_estimation)
    }

    /// Maximum allowed `right_offset` preventing scrolling infinitely into the future (`time-scale.ts:1130-1134`).
    #[must_use]
    pub fn max_right_offset(&self) -> f64 {
        if self.options.fix_right_edge || self.width <= 0.0 || self.bar_spacing <= 0.0 {
            0.0
        } else {
            let min_vis = self.options.min_visible_bars_count.min(self.total_bars) as f64;
            (self.width / self.bar_spacing) - min_vis
        }
    }

    fn correct_bar_spacing(&mut self) {
        let current = if self.bar_spacing.is_finite() && self.bar_spacing > 0.0 {
            self.bar_spacing.max(1e-6)
        } else {
            sanitize_initial_spacing(self.options.bar_spacing)
        };
        if self.width <= 0.0 {
            let opt_min = if self.options.min_bar_spacing.is_finite() {
                self.options.min_bar_spacing.max(1e-4)
            } else {
                0.5
            };
            self.bar_spacing = current.max(opt_min);
            return;
        }
        let min_bs = self.min_bar_spacing();
        let max_bs = self.max_bar_spacing();
        self.bar_spacing = lwc_clamp(current, min_bs, max_bs);
    }

    fn correct_offset(&mut self) {
        if self.is_empty() {
            return;
        }
        if let Some(min_ro) = self.min_right_offset()
            && self.right_offset < min_ro
        {
            self.right_offset = min_ro;
        }
        let max_ro = self.max_right_offset();
        if self.right_offset > max_ro {
            self.right_offset = max_ro;
        }
    }

    fn do_fix_left_edge(&mut self) {
        if !self.options.fix_left_edge || self.is_empty() {
            return;
        }
        if let Some(strict) = self.visible_strict_range() {
            let delta = strict.left; // first_index is 0
            if delta < 0 {
                let left_edge_offset = self.right_offset - delta as f64 - 1.0;
                self.set_right_offset(left_edge_offset);
            }
        }
        self.correct_bar_spacing();
    }
}

#[inline]
fn sanitize_initial_spacing(bar_spacing: f64) -> f64 {
    if bar_spacing.is_finite() && bar_spacing > 1e-4 {
        bar_spacing
    } else {
        6.0
    }
}

#[inline]
fn intersect_with_len(left_inclusive: i64, right_inclusive: i64, len: usize) -> Range<usize> {
    if len == 0 || right_inclusive < 0 || left_inclusive >= len as i64 {
        return 0..0;
    }
    let start = left_inclusive.max(0) as usize;
    let end_incl = right_inclusive.min((len - 1) as i64) as usize;
    if start > end_incl {
        0..0
    } else {
        start..(end_incl + 1)
    }
}
