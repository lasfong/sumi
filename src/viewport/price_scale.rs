use serde::{Deserialize, Serialize};

use crate::model::Candle;

/// Vertical price scale transformation mode (`src/model/price-scale.ts:42-59`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PriceScaleMode {
    Normal,
    Logarithmic,
    Percentage,
    IndexedTo100,
}

impl PriceScaleMode {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Normal => "Linear",
            Self::Logarithmic => "Log",
            Self::Percentage => "%",
            Self::IndexedTo100 => "Idx100",
        }
    }
}

/// Adaptive logarithmic mapping parameters (`src/model/price-scale-conversions.ts:3-11`).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LogFormula {
    pub logical_offset: f64,
    pub coord_offset: f64,
}

impl Default for LogFormula {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl LogFormula {
    pub const DEFAULT: Self = Self {
        logical_offset: 4.0,
        coord_offset: 0.0001,
    };

    /// Computes adaptive `LogFormula` precision for sub-unit price spans (`price-scale-conversions.ts:108-126`).
    #[must_use]
    pub fn for_price_range(raw_range: Option<PriceRange>) -> Self {
        let Some(range) = raw_range else {
            return Self::DEFAULT;
        };
        let diff = (range.max - range.min).abs();
        if !diff.is_finite() || !(1e-15..1.0).contains(&diff) {
            return Self::DEFAULT;
        }
        let digits = diff.log10().abs().ceil();
        let logical_offset = (Self::DEFAULT.logical_offset + digits).min(20.0);
        let coord_offset = 1.0 / 10_f64.powf(logical_offset);
        Self {
            logical_offset,
            coord_offset,
        }
    }
}

/// Unpadded logical price range `[min, max]` (`src/model/price-range-impl.ts`).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PriceRange {
    pub min: f64,
    pub max: f64,
}

impl PriceRange {
    #[must_use]
    pub fn new(min: f64, max: f64) -> Option<Self> {
        if !min.is_finite() || !max.is_finite() {
            return None;
        }
        if min <= max {
            Some(Self { min, max })
        } else {
            Some(Self { min: max, max: min })
        }
    }

    #[inline]
    #[must_use]
    pub fn len(&self) -> f64 {
        self.max - self.min
    }

    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        !self.min.is_finite() || !self.max.is_finite() || (self.max - self.min).abs() == 0.0
    }

    #[must_use]
    pub fn merge(self, other: Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    pub fn scale_around_center(&mut self, coeff: f64) {
        if !coeff.is_finite() || coeff <= 0.0 {
            return;
        }
        let delta = self.max - self.min;
        if delta == 0.0 {
            return;
        }
        let center = (self.max + self.min) * 0.5;
        let half = (delta * 0.5) * coeff;
        self.min = center - half;
        self.max = center + half;
    }

    pub fn shift(&mut self, delta: f64) {
        if !delta.is_finite() {
            return;
        }
        self.min += delta;
        self.max += delta;
    }
}

/// Converts raw `price` to LWC logarithmic space (`price-scale-conversions.ts:52-60`).
#[inline]
#[must_use]
pub fn to_log(price: f64, formula: LogFormula) -> f64 {
    if !price.is_finite() {
        return 0.0;
    }
    let m = price.abs();
    if m < 1e-15 {
        return 0.0;
    }
    let res = (m + formula.coord_offset).log10() + formula.logical_offset;
    if price < 0.0 { -res } else { res }
}

/// Converts logical logarithmic coordinate back to raw `price` (`price-scale-conversions.ts:62-70`).
#[inline]
#[must_use]
pub fn from_log(logical: f64, formula: LogFormula) -> f64 {
    if !logical.is_finite() {
        return 0.0;
    }
    let m = logical.abs();
    if m < 1e-15 {
        return 0.0;
    }
    let res = 10_f64.powf(m - formula.logical_offset) - formula.coord_offset;
    if logical < 0.0 { -res } else { res }
}

/// Converts raw `value` to percentage change relative to `base_value` (`price-scale-conversions.ts:21-24`).
#[inline]
#[must_use]
pub fn to_percent(value: f64, base_value: f64) -> f64 {
    if !value.is_finite() || !base_value.is_finite() || base_value.abs() < 1e-15 {
        return 0.0;
    }
    let result = 100.0 * (value - base_value) / base_value;
    if base_value < 0.0 { -result } else { result }
}

/// Inverse of `to_percent` (`price-scale-conversions.ts:13-19`).
#[inline]
#[must_use]
pub fn from_percent(percent: f64, base_value: f64) -> f64 {
    if !percent.is_finite() || !base_value.is_finite() || base_value.abs() < 1e-15 {
        return base_value;
    }
    let signed = if base_value < 0.0 { -percent } else { percent };
    (signed / 100.0) * base_value + base_value
}

/// Converts raw `value` to Indexed-to-100 space (`100.0` at `base_value`), preserving exact invertibility
/// for both positive and negative `base_value`.
#[inline]
#[must_use]
pub fn to_indexed_to_100(value: f64, base_value: f64) -> f64 {
    to_percent(value, base_value) + 100.0
}

/// Inverse of `to_indexed_to_100` (`price-scale-conversions.ts:32-39`).
#[inline]
#[must_use]
pub fn from_indexed_to_100(indexed: f64, base_value: f64) -> f64 {
    from_percent(indexed - 100.0, base_value)
}

/// Right-axis value formatting policy for a `PriceScale`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PriceAxisFormatMode {
    /// Standard adaptive price/oscillator decimal formatting.
    #[default]
    Standard,
    /// Compact K/M/B/T suffix formatting for high-magnitude cumulative volume / delta axes.
    CompactVolume,
}

/// Configuration for a single `PriceScale` instance on a pane (`left`, `right`, or `overlay`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PriceScaleOptions {
    pub mode: PriceScaleMode,
    pub auto_scale: bool,
    pub invert_scale: bool,
    /// Fractional top margin in `[0.0, 1.0)` (`top + bottom < 1.0`).
    pub scale_margin_top: f64,
    /// Fractional bottom margin in `[0.0, 1.0)` (`top + bottom < 1.0`).
    pub scale_margin_bottom: f64,
    /// Minimum price tick step used for `±5 * min_move` degenerate flat-range expansion.
    pub min_move: f64,
    /// Pins raw minimum to `0.0` during autoscale (used by Volume histogram overlays).
    pub pin_zero_bottom: bool,
    /// Value formatting mode for tick labels and crosshair pills on this scale.
    #[serde(default)]
    pub format_mode: PriceAxisFormatMode,
}

impl Default for PriceScaleOptions {
    fn default() -> Self {
        Self::lwc_default()
    }
}

impl PriceScaleOptions {
    /// Upstream LWC defaults (`top = 0.20, bottom = 0.10`, `price-scale-options-defaults.ts:13-16`).
    #[must_use]
    pub fn lwc_default() -> Self {
        Self {
            mode: PriceScaleMode::Normal,
            auto_scale: true,
            invert_scale: false,
            scale_margin_top: 0.20,
            scale_margin_bottom: 0.10,
            min_move: 0.01,
            pin_zero_bottom: false,
            format_mode: PriceAxisFormatMode::Standard,
        }
    }

    /// Main candlestick price scale defaults leaving clean bottom breathing room for volume overlay.
    #[must_use]
    pub fn sumi_main_price() -> Self {
        Self {
            mode: PriceScaleMode::Normal,
            auto_scale: true,
            invert_scale: false,
            scale_margin_top: 0.10,
            scale_margin_bottom: 0.20,
            min_move: 0.01,
            pin_zero_bottom: false,
            format_mode: PriceAxisFormatMode::Standard,
        }
    }

    /// Volume overlay scale defaults occupying the bottom 18% of the main pane.
    #[must_use]
    pub fn sumi_volume_overlay() -> Self {
        Self {
            mode: PriceScaleMode::Normal,
            auto_scale: true,
            invert_scale: false,
            scale_margin_top: 0.82,
            scale_margin_bottom: 0.00,
            min_move: 1.0,
            pin_zero_bottom: true,
            format_mode: PriceAxisFormatMode::Standard,
        }
    }

    /// Oscillator sub-pane scale defaults (`RSI`, `MACD`, `ATR`) with symmetric 8% vertical breathing room.
    #[must_use]
    pub fn sumi_oscillator() -> Self {
        Self {
            mode: PriceScaleMode::Normal,
            auto_scale: true,
            invert_scale: false,
            scale_margin_top: 0.08,
            scale_margin_bottom: 0.08,
            min_move: 0.01,
            pin_zero_bottom: false,
            format_mode: PriceAxisFormatMode::Standard,
        }
    }

    /// Cumulative Volume Delta (`CVD`) sub-pane scale defaults with compact `K`/`M`/`B`/`T` axis labels.
    #[must_use]
    pub fn sumi_cvd_oscillator() -> Self {
        Self {
            mode: PriceScaleMode::Normal,
            auto_scale: true,
            invert_scale: false,
            scale_margin_top: 0.08,
            scale_margin_bottom: 0.08,
            min_move: 1.0,
            pin_zero_bottom: false,
            format_mode: PriceAxisFormatMode::CompactVolume,
        }
    }
}

/// Formatted horizontal grid/axis tick mark produced by `PriceScale::nice_ticks`.
#[derive(Debug, Clone, PartialEq)]
pub struct PriceTickMark {
    pub local_y: f64,
    pub logical_value: f64,
    pub raw_price: f64,
    pub label: String,
}

/// Independent vertical price scale scoped to a specific pane and axis (`left`, `right`, or `overlay`).
///
/// Coordinates accepted and emitted by `PriceScale` are **pane-local** vertical logical pixels
/// in `[0.0, height)` (`0.0` at the top edge of the pane plot area).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceScale {
    id: String,
    options: PriceScaleOptions,
    height: f64,
    margin_above_px: f64,
    margin_below_px: f64,
    price_range: Option<PriceRange>,
    log_formula: LogFormula,
    is_custom_price_range: bool,
    scroll_snapshot: Option<(f64, PriceRange)>,
    scale_snapshot: Option<(f64, PriceRange)>,
}

impl PriceScale {
    #[must_use]
    pub fn new(id: impl Into<String>, mut options: PriceScaleOptions) -> Self {
        sanitize_margins(
            &mut options.scale_margin_top,
            &mut options.scale_margin_bottom,
        );
        options.min_move = if options.min_move.is_finite() && options.min_move > 0.0 {
            options.min_move
        } else {
            0.01
        };
        Self {
            id: id.into(),
            options,
            height: 0.0,
            margin_above_px: 0.0,
            margin_below_px: 0.0,
            price_range: None,
            log_formula: LogFormula::DEFAULT,
            is_custom_price_range: false,
            scroll_snapshot: None,
            scale_snapshot: None,
        }
    }

    /// Sanitizes state restored from persistent storage (`serde`) against NaN/Infinity or mode mismatches.
    pub fn sanitize_deserialized(&mut self) {
        self.scroll_snapshot = None;
        self.scale_snapshot = None;

        sanitize_margins(
            &mut self.options.scale_margin_top,
            &mut self.options.scale_margin_bottom,
        );
        if !self.options.min_move.is_finite() || self.options.min_move <= 0.0 {
            self.options.min_move = 0.01;
        }
        if !self.height.is_finite() || self.height < 0.0 {
            self.height = 0.0;
        }
        if !self.margin_above_px.is_finite() || self.margin_above_px < 0.0 {
            self.margin_above_px = 0.0;
        }
        if !self.margin_below_px.is_finite() || self.margin_below_px < 0.0 {
            self.margin_below_px = 0.0;
        }
        if !self.log_formula.logical_offset.is_finite()
            || !self.log_formula.coord_offset.is_finite()
            || self.log_formula.coord_offset <= 0.0
        {
            self.log_formula = LogFormula::DEFAULT;
        }
        if matches!(
            self.options.mode,
            PriceScaleMode::Percentage | PriceScaleMode::IndexedTo100
        ) {
            self.options.auto_scale = true;
            self.is_custom_price_range = false;
        }
        if let Some(range) = self.price_range
            && !self.is_valid_logical_range(range)
        {
            self.price_range = None;
            self.options.auto_scale = true;
            self.is_custom_price_range = false;
        }
    }

    #[inline]
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[inline]
    #[must_use]
    pub fn options(&self) -> &PriceScaleOptions {
        &self.options
    }

    #[inline]
    #[must_use]
    pub fn mode(&self) -> PriceScaleMode {
        self.options.mode
    }

    #[inline]
    #[must_use]
    pub fn format_mode(&self) -> PriceAxisFormatMode {
        self.options.format_mode
    }

    #[inline]
    pub fn set_format_mode(&mut self, format_mode: PriceAxisFormatMode) {
        self.options.format_mode = format_mode;
    }

    #[inline]
    #[must_use]
    pub fn is_auto_scale(&self) -> bool {
        self.options.auto_scale
    }

    #[inline]
    #[must_use]
    pub fn is_inverted(&self) -> bool {
        self.options.invert_scale
    }

    #[inline]
    #[must_use]
    pub fn is_log(&self) -> bool {
        self.options.mode == PriceScaleMode::Logarithmic
    }

    #[inline]
    #[must_use]
    pub fn height(&self) -> f64 {
        self.height
    }

    #[inline]
    #[must_use]
    pub fn log_formula(&self) -> LogFormula {
        self.log_formula
    }

    #[inline]
    #[must_use]
    pub fn price_range(&self) -> Option<PriceRange> {
        self.price_range
    }

    /// Returns `true` when pane height is invalid, `internal_height <= 1.0`, or no non-empty range is set.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.height <= 0.0
            || !self.height.is_finite()
            || self.internal_height() <= 1.0
            || self.price_range.is_none_or(|r| r.is_empty())
    }

    pub fn set_height(&mut self, height: f64) {
        if !height.is_finite() || height < 0.0 {
            self.height = 0.0;
            return;
        }
        self.height = height;
    }

    pub fn set_margins(&mut self, mut top: f64, mut bottom: f64) {
        sanitize_margins(&mut top, &mut bottom);
        self.options.scale_margin_top = top;
        self.options.scale_margin_bottom = bottom;
    }

    pub fn set_pixel_margins(&mut self, above_px: f64, below_px: f64) {
        self.margin_above_px = if above_px.is_finite() {
            above_px.max(0.0)
        } else {
            0.0
        };
        self.margin_below_px = if below_px.is_finite() {
            below_px.max(0.0)
        } else {
            0.0
        };
    }

    pub fn set_inverted(&mut self, inverted: bool) {
        self.options.invert_scale = inverted;
    }

    /// Switches `PriceScaleMode`, converting existing logical `price_range` cleanly (`price-scale.ts:353-407`).
    pub fn set_mode(&mut self, new_mode: PriceScaleMode) {
        self.set_mode_with_base(new_mode, None);
    }

    /// Switches `PriceScaleMode` using optional `base_value` to convert to/from `Percentage` and `IndexedTo100`.
    pub fn set_mode_with_base(&mut self, new_mode: PriceScaleMode, base_value: Option<f64>) {
        let old_mode = self.options.mode;
        if old_mode == new_mode {
            return;
        }

        let valid_base = base_value.filter(|b| b.is_finite() && b.abs() >= 1e-15);

        // 1. Convert current `price_range` back to raw linear price units (or reset to autoscale if unconvertible)
        let raw_range = match (old_mode, self.price_range) {
            (PriceScaleMode::Normal, Some(r)) if !r.is_empty() => Some(r),
            (PriceScaleMode::Logarithmic, Some(log_r)) => {
                let rmin = from_log(log_r.min, self.log_formula);
                let rmax = from_log(log_r.max, self.log_formula);
                PriceRange::new(rmin, rmax).filter(|r| !r.is_empty())
            }
            (PriceScaleMode::Percentage, Some(pct_r)) => valid_base.and_then(|bv| {
                PriceRange::new(from_percent(pct_r.min, bv), from_percent(pct_r.max, bv))
                    .filter(|r| !r.is_empty())
            }),
            (PriceScaleMode::IndexedTo100, Some(idx_r)) => valid_base.and_then(|bv| {
                PriceRange::new(
                    from_indexed_to_100(idx_r.min, bv),
                    from_indexed_to_100(idx_r.max, bv),
                )
                .filter(|r| !r.is_empty())
            }),
            _ => None,
        };

        self.options.mode = new_mode;
        self.scroll_snapshot = None;
        self.scale_snapshot = None;

        if matches!(
            old_mode,
            PriceScaleMode::Percentage | PriceScaleMode::IndexedTo100
        ) || matches!(
            new_mode,
            PriceScaleMode::Percentage | PriceScaleMode::IndexedTo100
        ) || raw_range.is_none()
        {
            self.options.auto_scale = true;
            self.is_custom_price_range = false;
        }

        // 2. Project `raw_range` into the target `new_mode` space
        self.price_range = match (new_mode, raw_range) {
            (PriceScaleMode::Normal, Some(raw)) => Some(raw),
            (PriceScaleMode::Logarithmic, Some(raw)) => {
                self.log_formula = LogFormula::for_price_range(Some(raw));
                let lmin = to_log(raw.min, self.log_formula);
                let lmax = to_log(raw.max, self.log_formula);
                PriceRange::new(lmin, lmax).filter(|r| self.is_valid_logical_range(*r))
            }
            (PriceScaleMode::Percentage | PriceScaleMode::IndexedTo100, Some(raw)) => {
                if let Some(bv) = valid_base {
                    let l1 = self.raw_to_logical(raw.min, bv);
                    let l2 = self.raw_to_logical(raw.max, bv);
                    PriceRange::new(l1, l2).filter(|r| !r.is_empty())
                } else {
                    None
                }
            }
            _ => None,
        };
    }

    /// Sets an explicit raw `[min_price, max_price]` range (converting into logical mode space if needed).
    pub fn set_raw_price_range(&mut self, min_price: f64, max_price: f64, base_value: f64) {
        let Some(mut raw) = PriceRange::new(min_price, max_price) else {
            return;
        };
        if raw.is_empty() {
            let ext = 5.0 * self.options.min_move;
            raw.min -= ext;
            raw.max += ext;
        }
        if self.is_log() {
            self.log_formula = LogFormula::for_price_range(Some(raw));
        }
        let l1 = self.raw_to_logical(raw.min, base_value);
        let l2 = self.raw_to_logical(raw.max, base_value);
        self.price_range = PriceRange::new(l1, l2);
    }

    /// Re-enables `auto_scale` (invoked e.g. on double-click of the price axis).
    pub fn reset_auto_scale(&mut self) {
        self.options.auto_scale = true;
        self.is_custom_price_range = false;
        self.scroll_snapshot = None;
        self.scale_snapshot = None;
    }

    /// Updates `auto_scale`, enforcing LWC's rule that `Percentage` and `IndexedTo100` stay autoscaled (`price-scale.ts:363-367`).
    pub fn set_auto_scale(&mut self, auto_scale: bool) {
        if matches!(
            self.options.mode,
            PriceScaleMode::Percentage | PriceScaleMode::IndexedTo100
        ) {
            self.options.auto_scale = true;
            self.is_custom_price_range = false;
            return;
        }
        self.options.auto_scale = auto_scale;
        if auto_scale {
            self.is_custom_price_range = false;
        }
    }

    /// Top margin in pixels (`price-scale.ts:911-915`).
    #[inline]
    #[must_use]
    pub fn top_margin_px(&self) -> f64 {
        if self.options.invert_scale {
            self.options.scale_margin_bottom * self.height + self.margin_below_px
        } else {
            self.options.scale_margin_top * self.height + self.margin_above_px
        }
    }

    /// Bottom margin in pixels (`price-scale.ts:917-921`).
    #[inline]
    #[must_use]
    pub fn bottom_margin_px(&self) -> f64 {
        if self.options.invert_scale {
            self.options.scale_margin_top * self.height + self.margin_above_px
        } else {
            self.options.scale_margin_bottom * self.height + self.margin_below_px
        }
    }

    /// Active vertical span inside margins (`height - top_margin_px - bottom_margin_px`, `price-scale.ts:431-439`).
    #[inline]
    #[must_use]
    pub fn internal_height(&self) -> f64 {
        self.height - self.top_margin_px() - self.bottom_margin_px()
    }

    #[inline]
    fn inverted_coordinate(&self, coord: f64) -> f64 {
        if self.options.invert_scale {
            coord
        } else {
            self.height - 1.0 - coord
        }
    }

    #[inline]
    fn raw_to_logical(&self, price: f64, base_value: f64) -> f64 {
        match self.options.mode {
            PriceScaleMode::Normal => price,
            PriceScaleMode::Logarithmic => to_log(price, self.log_formula),
            PriceScaleMode::Percentage => to_percent(price, base_value),
            PriceScaleMode::IndexedTo100 => to_indexed_to_100(price, base_value),
        }
    }

    #[inline]
    fn logical_to_raw(&self, logical: f64, base_value: f64) -> f64 {
        match self.options.mode {
            PriceScaleMode::Normal => logical,
            PriceScaleMode::Logarithmic => from_log(logical, self.log_formula),
            PriceScaleMode::Percentage => from_percent(logical, base_value),
            PriceScaleMode::IndexedTo100 => from_indexed_to_100(logical, base_value),
        }
    }

    /// Projects a raw `price` to pane-local vertical coordinate `local_y` (`price-scale.ts:473-481, 934-946`).
    ///
    /// Safely guards `internal_height() <= 1.0` and `range.len() <= 0.0` (never divides by `0.0`).
    #[inline]
    #[must_use]
    pub fn price_to_local_y(&self, price: f64, base_value: f64) -> Option<f64> {
        if self.is_empty() || !price.is_finite() {
            return None;
        }
        let range = self.price_range?;
        let ih_minus_one = self.internal_height() - 1.0;
        let span = range.len();
        if ih_minus_one <= 0.0 || span <= 0.0 || !span.is_finite() {
            return None;
        }
        let logical = self.raw_to_logical(price, base_value);
        let inv_coord = self.bottom_margin_px() + ih_minus_one * (logical - range.min) / span;
        let y = self.inverted_coordinate(inv_coord);
        y.is_finite().then_some(y)
    }

    /// Converts a pane-local vertical coordinate `local_y` back into raw `price` (`price-scale.ts:565-578, 948-959`).
    ///
    /// Safely guards `internal_height() <= 1.0` (never divides by `0.0`).
    #[inline]
    #[must_use]
    pub fn local_y_to_price(&self, local_y: f64, base_value: f64) -> Option<f64> {
        if self.is_empty() || !local_y.is_finite() {
            return None;
        }
        let range = self.price_range?;
        let ih_minus_one = self.internal_height() - 1.0;
        let span = range.len();
        if ih_minus_one <= 0.0 || span <= 0.0 || !span.is_finite() {
            return None;
        }
        let inv_coord = self.inverted_coordinate(local_y);
        let logical = range.min + span * ((inv_coord - self.bottom_margin_px()) / ih_minus_one);
        let raw = self.logical_to_raw(logical, base_value);
        raw.is_finite().then_some(raw)
    }

    /// Recalculates `price_range` from precomputed raw `[raw_min, raw_max]` bounds when `auto_scale` is enabled.
    pub fn autoscale_raw_bounds(&mut self, raw_min: f64, raw_max: f64, base_value: f64) {
        if !self.options.auto_scale {
            return;
        }
        let effective_min = if self.options.pin_zero_bottom && raw_max.is_finite() {
            if raw_min.is_finite() {
                raw_min.min(0.0)
            } else {
                0.0
            }
        } else {
            raw_min
        };
        self.apply_autoscaled_raw_bounds(effective_min, raw_max, base_value);
    }

    /// Recalculates `price_range` from visible OHLC candles when `auto_scale` is enabled (`price-scale.ts:967-1076`).
    pub fn autoscale_candles(&mut self, visible_candles: &[Candle], base_value: f64) {
        if !self.options.auto_scale {
            return;
        }
        let mut raw_min = f64::INFINITY;
        let mut raw_max = f64::NEG_INFINITY;
        for c in visible_candles {
            if c.low.is_finite() {
                raw_min = raw_min.min(c.low);
            }
            if c.high.is_finite() {
                raw_max = raw_max.max(c.high);
            }
        }
        self.apply_autoscaled_raw_bounds(raw_min, raw_max, base_value);
    }

    /// Recalculates `price_range` from visible candle volumes with zero heap allocation when `auto_scale` is enabled.
    pub fn autoscale_volume(&mut self, visible_candles: &[Candle]) {
        if !self.options.auto_scale {
            return;
        }
        let mut raw_min = if self.options.pin_zero_bottom {
            0.0
        } else {
            f64::INFINITY
        };
        let mut raw_max = f64::NEG_INFINITY;
        for c in visible_candles {
            if c.volume.is_finite() {
                raw_min = raw_min.min(c.volume);
                raw_max = raw_max.max(c.volume);
            }
        }
        self.apply_autoscaled_raw_bounds(raw_min, raw_max, 1.0);
    }

    /// Recalculates `price_range` from scalar values (e.g. Volume bars or indicator series) when `auto_scale` is enabled.
    pub fn autoscale_values(&mut self, visible_values: &[f64], base_value: f64) {
        if !self.options.auto_scale {
            return;
        }
        let mut raw_min = if self.options.pin_zero_bottom {
            0.0
        } else {
            f64::INFINITY
        };
        let mut raw_max = f64::NEG_INFINITY;
        for &v in visible_values {
            if v.is_finite() {
                raw_min = raw_min.min(v);
                raw_max = raw_max.max(v);
            }
        }
        self.apply_autoscaled_raw_bounds(raw_min, raw_max, base_value);
    }

    fn apply_autoscaled_raw_bounds(&mut self, mut raw_min: f64, mut raw_max: f64, base_value: f64) {
        if !raw_min.is_finite() || !raw_max.is_finite() || raw_min > raw_max {
            if self.price_range.is_none() {
                self.price_range = PriceRange::new(-0.5, 0.5);
                self.log_formula = LogFormula::DEFAULT;
            }
            return;
        }

        if self.options.pin_zero_bottom {
            raw_min = raw_min.min(0.0);
        }

        // Degenerate flat-price expansion in raw linear space (`price-scale.ts:1039-1053`): ±5 * minMove.
        if (raw_max - raw_min).abs() < 1e-15 {
            let extend = 5.0 * self.options.min_move;
            if self.options.pin_zero_bottom {
                raw_max += extend * 2.0;
            } else {
                raw_min -= extend;
                raw_max += extend;
            }
        }

        if self.is_log() {
            let new_formula = LogFormula::for_price_range(PriceRange::new(raw_min, raw_max));
            if new_formula != self.log_formula {
                if let Some((start_y, snap)) = self.scale_snapshot {
                    let s_min = from_log(snap.min, self.log_formula);
                    let s_max = from_log(snap.max, self.log_formula);
                    if let Some(updated) =
                        PriceRange::new(to_log(s_min, new_formula), to_log(s_max, new_formula))
                    {
                        self.scale_snapshot = Some((start_y, updated));
                    }
                }
                self.log_formula = new_formula;
            }
        }

        let l1 = self.raw_to_logical(raw_min, base_value);
        let l2 = self.raw_to_logical(raw_max, base_value);
        if let Some(mut logical_range) = PriceRange::new(l1, l2) {
            if logical_range.is_empty() {
                logical_range.min -= 0.5;
                logical_range.max += 0.5;
            }
            self.price_range = Some(logical_range);
        }
    }

    /// Starts vertical drag-scaling on the price axis (`price-scale.ts:671-687`).
    pub fn start_scale(&mut self, local_y: f64) {
        if matches!(
            self.options.mode,
            PriceScaleMode::Percentage | PriceScaleMode::IndexedTo100
        ) {
            return;
        }
        if self.scale_snapshot.is_some()
            || self.scroll_snapshot.is_some()
            || self.is_empty()
            || !local_y.is_finite()
        {
            return;
        }
        if let Some(range) = self.price_range {
            self.scale_snapshot = Some((self.height - local_y, range));
        }
    }

    /// Updates vertical drag-scaling around logical range center (`price-scale.ts:689-715`).
    pub fn scale_to(&mut self, local_y: f64) {
        if matches!(
            self.options.mode,
            PriceScaleMode::Percentage | PriceScaleMode::IndexedTo100
        ) {
            return;
        }
        let Some((start_inv_y, snapshot)) = self.scale_snapshot else {
            return;
        };
        if !local_y.is_finite() || self.height <= 1.0 {
            return;
        }
        self.options.auto_scale = false;
        self.is_custom_price_range = true;

        let inv_y = (self.height - local_y).max(0.0);
        let damp = (self.height - 1.0) * 0.2;
        let denom = inv_y + damp;
        if denom <= 0.0 {
            return;
        }
        let coeff = ((start_inv_y + damp) / denom).max(0.1);
        let mut next_range = snapshot;
        next_range.scale_around_center(coeff);
        if self.is_valid_logical_range(next_range) {
            self.price_range = Some(next_range);
        }
    }

    /// Ends vertical drag-scaling on the price axis (`price-scale.ts:717-724`).
    pub fn end_scale(&mut self) {
        self.scale_snapshot = None;
    }

    #[inline]
    #[must_use]
    pub fn is_scaling(&self) -> bool {
        self.scale_snapshot.is_some()
    }

    /// Starts manual vertical panning when `!auto_scale` (`price-scale.ts:726-741`).
    pub fn start_scroll(&mut self, local_y: f64) {
        if self.options.auto_scale
            || self.scroll_snapshot.is_some()
            || self.scale_snapshot.is_some()
            || self.is_empty()
            || !local_y.is_finite()
        {
            return;
        }
        if let Some(range) = self.price_range {
            self.scroll_snapshot = Some((local_y, range));
        }
    }

    /// Updates manual vertical panning when `!auto_scale` (`price-scale.ts:743-765`).
    pub fn scroll_to(&mut self, local_y: f64) {
        if self.options.auto_scale {
            return;
        }
        let Some((start_y, snapshot)) = self.scroll_snapshot else {
            return;
        };
        let Some(current_range) = self.price_range else {
            return;
        };
        let ih_minus_one = self.internal_height() - 1.0;
        if !local_y.is_finite() || ih_minus_one <= 0.0 {
            return;
        }
        let units_per_px = current_range.len() / ih_minus_one;
        let mut pixel_delta = local_y - start_y;
        if self.options.invert_scale {
            pixel_delta = -pixel_delta;
        }
        let mut next_range = snapshot;
        next_range.shift(pixel_delta * units_per_px);
        if self.is_valid_logical_range(next_range) {
            self.price_range = Some(next_range);
        }
    }

    /// Ends manual vertical panning (`price-scale.ts:767-778`).
    pub fn end_scroll(&mut self) {
        self.scroll_snapshot = None;
    }

    /// Verifies that a candidate logical `PriceRange` stays finite and non-overflowing across
    /// top/bottom margin padding (mirroring LWC `canConvertPriceRangeFromLog`, `price-scale-conversions.ts:72-90`).
    fn is_valid_logical_range(&self, candidate: PriceRange) -> bool {
        if candidate.is_empty() || !candidate.len().is_finite() || candidate.len() < 1e-12 {
            return false;
        }
        let ih = (self.internal_height() - 1.0).max(1.0);
        let ext_min = candidate.min - candidate.len() * (self.bottom_margin_px() / ih);
        let ext_max = candidate.max + candidate.len() * (self.top_margin_px() / ih);
        if !ext_min.is_finite() || !ext_max.is_finite() || !(ext_max - ext_min).is_finite() {
            return false;
        }
        match self.options.mode {
            PriceScaleMode::Logarithmic => {
                let max_log_exp = 100.0;
                (ext_min.abs() - self.log_formula.logical_offset) <= max_log_exp
                    && (ext_max.abs() - self.log_formula.logical_offset) <= max_log_exp
                    && from_log(ext_min, self.log_formula).is_finite()
                    && from_log(ext_max, self.log_formula).is_finite()
            }
            PriceScaleMode::Normal => {
                ext_min.abs() <= 1e15 && ext_max.abs() <= 1e15 && (ext_max - ext_min) <= 2e15
            }
            PriceScaleMode::Percentage | PriceScaleMode::IndexedTo100 => {
                ext_min.abs() <= 1e9 && ext_max.abs() <= 1e9
            }
        }
    }

    /// Formats a raw price according to the active `PriceScaleMode` and `PriceAxisFormatMode` (for crosshair pills & badges).
    #[must_use]
    pub fn format_price(&self, raw_price: f64, base_value: f64) -> String {
        match self.options.mode {
            PriceScaleMode::Percentage => format!("{:+.2}%", to_percent(raw_price, base_value)),
            PriceScaleMode::IndexedTo100 => {
                format!("{:.2}", to_indexed_to_100(raw_price, base_value))
            }
            PriceScaleMode::Normal | PriceScaleMode::Logarithmic => {
                match self.options.format_mode {
                    PriceAxisFormatMode::Standard => format_price_adaptive(raw_price),
                    PriceAxisFormatMode::CompactVolume => {
                        format_compact_volume_axis(raw_price, None)
                    }
                }
            }
        }
    }

    /// Generates clean, human-readable axis tick marks across `[0, height)` in the active scale mode.
    #[must_use]
    pub fn nice_ticks(&self, target_count: usize, base_value: f64) -> Vec<PriceTickMark> {
        if self.is_empty() || target_count == 0 {
            return Vec::new();
        }
        let Some(top_price) = self.local_y_to_price(0.0, base_value) else {
            return Vec::new();
        };
        let Some(bottom_price) = self.local_y_to_price(self.height - 1.0, base_value) else {
            return Vec::new();
        };

        let mut marks = Vec::new();
        match self.options.mode {
            PriceScaleMode::Percentage | PriceScaleMode::IndexedTo100 => {
                let Some(range) = self.price_range else {
                    return Vec::new();
                };
                let ih = (self.internal_height() - 1.0).max(1.0);
                let l_low = range.min - range.len() * (self.bottom_margin_px() / ih);
                let l_high = range.max + range.len() * (self.top_margin_px() / ih);
                let (steps, step_size) = generate_nice_linear_steps(l_low, l_high, target_count);
                let dec = decimals_for_step(step_size, 2, 6);
                let mut last_accepted_y: Option<f64> = None;
                for logical in steps {
                    let raw = self.logical_to_raw(logical, base_value);
                    if let Some(ly) = self.price_to_local_y(raw, base_value)
                        && (4.0..=(self.height - 6.0)).contains(&ly)
                    {
                        if last_accepted_y.is_some_and(|prev_y| (prev_y - ly).abs() < 22.0) {
                            continue;
                        }
                        last_accepted_y = Some(ly);
                        let label = if self.options.mode == PriceScaleMode::Percentage {
                            format!("{logical:+.dec$}%")
                        } else {
                            format!("{logical:.dec$}")
                        };
                        marks.push(PriceTickMark {
                            local_y: ly,
                            logical_value: logical,
                            raw_price: raw,
                            label,
                        });
                    }
                }
            }
            PriceScaleMode::Normal | PriceScaleMode::Logarithmic => {
                let p_min = top_price.min(bottom_price);
                let p_max = top_price.max(bottom_price);
                let (candidates, step_hint) = if self.options.mode == PriceScaleMode::Logarithmic {
                    generate_nice_log_or_linear_steps(p_min, p_max, target_count)
                } else {
                    let (v, step) = generate_nice_linear_steps(p_min, p_max, target_count);
                    (v, Some(step))
                };
                let mut last_accepted_y: Option<f64> = None;
                let edge_pad = (self.height * 0.1).min(9.0);
                let min_gap = if self.height < 130.0 { 16.0 } else { 22.0 };
                for raw in candidates {
                    if let Some(ly) = self.price_to_local_y(raw, base_value)
                        && (edge_pad..=(self.height - edge_pad)).contains(&ly)
                    {
                        if last_accepted_y.is_some_and(|prev_y| (prev_y - ly).abs() < min_gap) {
                            continue;
                        }
                        last_accepted_y = Some(ly);
                        let label = match self.options.format_mode {
                            PriceAxisFormatMode::CompactVolume => {
                                format_compact_volume_axis(raw, step_hint)
                            }
                            PriceAxisFormatMode::Standard => {
                                if let Some(step) = step_hint {
                                    format_price_with_step(raw, step)
                                } else {
                                    format_price_adaptive(raw)
                                }
                            }
                        };
                        marks.push(PriceTickMark {
                            local_y: ly,
                            logical_value: self.raw_to_logical(raw, base_value),
                            raw_price: raw,
                            label,
                        });
                    }
                }
            }
        }
        marks
    }
}

fn sanitize_margins(top: &mut f64, bottom: &mut f64) {
    *top = if top.is_finite() {
        top.clamp(0.0, 0.95)
    } else {
        0.10
    };
    *bottom = if bottom.is_finite() {
        bottom.clamp(0.0, 0.95)
    } else {
        0.10
    };
    if *top + *bottom >= 0.96 {
        let sum = *top + *bottom;
        *top = (*top / sum) * 0.90;
        *bottom = (*bottom / sum) * 0.90;
    }
}

fn generate_nice_linear_steps(min_val: f64, max_val: f64, target_count: usize) -> (Vec<f64>, f64) {
    if !min_val.is_finite() || !max_val.is_finite() || max_val <= min_val || target_count == 0 {
        return (Vec::new(), 0.0);
    }
    let n = target_count.max(2) as f64;
    let span = max_val - min_val;
    let rough_step = if span.is_finite() {
        span / n
    } else {
        (max_val / n) - (min_val / n)
    };
    if !rough_step.is_finite() || rough_step <= 0.0 {
        return (Vec::new(), 0.0);
    }
    let mag = 10_f64.powf(rough_step.log10().floor());
    if !mag.is_finite() || mag <= 0.0 {
        return (Vec::new(), 0.0);
    }
    let residual = rough_step / mag;
    let nice_mult = if residual <= 1.5 {
        1.0
    } else if residual <= 3.0 {
        2.0
    } else if residual <= 7.0 {
        5.0
    } else {
        10.0
    };
    let step = nice_mult * mag;
    if step <= 0.0 || !step.is_finite() {
        return (Vec::new(), 0.0);
    }
    let start_f = (min_val / step).ceil();
    let end_f = (max_val / step).floor();
    if !start_f.is_finite() || !end_f.is_finite() || start_f.abs() > 1e15 || end_f.abs() > 1e15 {
        return (Vec::new(), step);
    }
    let start_k = start_f as i64;
    let end_k = end_f as i64;
    let mut out = Vec::new();
    let max_iters = 64_i64;
    for k in start_k..=end_k.min(start_k.saturating_add(max_iters)) {
        out.push(k as f64 * step);
    }
    (out, step)
}

fn generate_nice_log_or_linear_steps(
    min_val: f64,
    max_val: f64,
    target_count: usize,
) -> (Vec<f64>, Option<f64>) {
    if min_val <= 0.0 || max_val / min_val.max(1e-12) < 3.0 {
        let (steps, step) = generate_nice_linear_steps(min_val, max_val, target_count);
        return (steps, Some(step));
    }
    let start_exp = min_val.log10().floor() as i32;
    let end_exp = max_val.log10().ceil() as i32;
    let decades = (end_exp - start_exp).max(1);
    let ticks_per_decade = (target_count.max(2) as f64) / (decades as f64);

    let decade_stride = if decades > (target_count.max(2) as i32) {
        (decades as f64 / (target_count.max(2) as f64)).ceil() as i32
    } else {
        1
    };
    let multipliers: &[f64] = if ticks_per_decade < 1.8 {
        &[1.0]
    } else if ticks_per_decade < 3.5 {
        &[1.0, 2.0, 5.0]
    } else {
        &[1.0, 1.5, 2.0, 3.0, 5.0, 7.0]
    };

    let mut out = Vec::new();
    let mut exp = start_exp.div_euclid(decade_stride) * decade_stride;
    let max_exp = end_exp.min(start_exp.saturating_add(120));
    while exp <= max_exp {
        let base = 10_f64.powi(exp);
        if base.is_finite() && base > 0.0 {
            for &mult in multipliers {
                let v = base * mult;
                if v.is_finite() && v >= min_val && v <= max_val {
                    out.push(v);
                }
            }
        }
        exp = exp.saturating_add(decade_stride);
    }
    if out.len() < 2 {
        let (steps, step) = generate_nice_linear_steps(min_val, max_val, target_count);
        (steps, Some(step))
    } else {
        (out, None)
    }
}

fn decimals_for_step(step: f64, min_dec: usize, max_dec: usize) -> usize {
    if !step.is_finite() || step <= 0.0 || step >= 1.0 {
        return min_dec;
    }
    let step_dec = (-step.log10().floor()).max(0.0) as usize;
    step_dec.clamp(min_dec, max_dec)
}

fn format_price_with_step(price: f64, step: f64) -> String {
    let abs = price.abs();
    let base_dec = if abs >= 100.0 || abs == 0.0 {
        2
    } else if abs >= 1.0 {
        4
    } else {
        6
    };
    let dec = decimals_for_step(step, base_dec, 8);
    format!("{price:.dec$}")
}

#[must_use]
pub fn format_price_adaptive(price: f64) -> String {
    let abs = price.abs();
    if abs >= 100.0 || abs == 0.0 {
        format!("{price:.2}")
    } else if abs >= 1.0 {
        format!("{price:.4}")
    } else {
        format!("{price:.6}")
    }
}

/// Formats signed large quantities (`K`, `M`, `B`, `T`) with rollover promotion so
/// `CVD` right-axis ticks, cursor pills, and drawing badges remain concise inside narrow gutters.
#[must_use]
pub fn format_compact_volume_axis(value: f64, step_hint: Option<f64>) -> String {
    if !value.is_finite() {
        return "—".to_owned();
    }
    let abs = value.abs();
    if abs < 1.0e-9 {
        return "0".to_owned();
    }
    let sign = if value < 0.0 { "-" } else { "" };
    const TIERS: [(f64, &str); 5] = [
        (1.0e12, "T"),
        (1.0e9, "B"),
        (1.0e6, "M"),
        (1.0e3, "K"),
        (1.0, ""),
    ];
    let mut tier_idx = if abs >= 999_950_000_000.0 {
        0
    } else if abs >= 999_950_000.0 {
        1
    } else if abs >= 999_950.0 {
        2
    } else if abs >= 999.95 {
        3
    } else {
        4
    };
    let valid_step = step_hint.filter(|s| s.is_finite() && *s > 0.0);

    loop {
        let (divisor, suffix) = TIERS[tier_idx];
        let mag = abs / divisor;
        let dec = if suffix.is_empty() {
            if let Some(step) = valid_step {
                if step < 1.0 {
                    decimals_for_step(step, 0, 2)
                } else if mag >= 10.0 {
                    0
                } else {
                    1
                }
            } else if mag >= 10.0 || (mag - mag.round()).abs() < 1.0e-6 {
                0
            } else {
                2
            }
        } else if let Some(step) = valid_step {
            let scaled_step = step / divisor;
            if scaled_step >= 0.999_9 {
                0
            } else if mag >= 100.0
                || ((scaled_step * 10.0 - (scaled_step * 10.0).round()).abs() < 1.0e-4
                    && scaled_step >= 0.099_9)
            {
                1
            } else {
                2
            }
        } else if mag >= 100.0 {
            1
        } else {
            2
        };

        let scale_pow = 10_f64.powi(dec as i32);
        let rounded = (mag * scale_pow).round() / scale_pow;
        if rounded >= 1000.0 && tier_idx > 0 {
            tier_idx -= 1;
            continue;
        }
        if rounded == 0.0 {
            return "0".to_owned();
        }

        let mut body = format!("{rounded:.dec$}");
        if dec > 0 && body.contains('.') {
            while body.ends_with('0') {
                body.pop();
            }
            if body.ends_with('.') {
                body.pop();
            }
        }
        return format!("{sign}{body}{suffix}");
    }
}
