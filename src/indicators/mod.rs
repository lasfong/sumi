//! Pure `f64`, `wasm32-unknown-unknown`-compatible Technical Indicator Engine (`Milestone R04 / Wave 2`).
//!
//! Verified against open-source TA-Lib (`ta_SMA.c`, `ta_EMA.c`, `ta_BBANDS.c`, `ta_RSI.c`,
//! `ta_MACD.c`, `ta_ATR.c`) and legacy Sumi (`indicatorDomain.ts` / `IndicatorRenderRegistry.ts`).
//! Contains zero UI or OS-specific dependencies.

pub mod bands;
pub mod moving_averages;
pub mod oscillators;

#[cfg(test)]
mod tests;

use std::ops::Range;

use serde::{Deserialize, Serialize};

use crate::model::Candle;

pub use bands::{
    BollingerBandsIndicator, BollingerOutput, compute_bollinger_bands, compute_bollinger_slices,
};
pub use moving_averages::{
    EmaIndicator, SmaIndicator, compute_ema, compute_ema_optional_slice, compute_ema_slices,
    compute_sma, compute_sma_slices,
};
pub use oscillators::{
    AtrIndicator, MACD_REFERENCE_LEVELS, MacdIndicator, MacdOutput, RSI_FIXED_BOUNDS_HINT,
    RSI_REFERENCE_LEVELS, RsiIndicator, compute_atr, compute_macd, compute_macd_slices,
    compute_rsi, compute_rsi_slices, true_range,
};

/// Target pane placement for an indicator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndicatorPlacement {
    /// Drawn directly over the main candlestick pane sharing `main_price_scale`.
    PriceOverlay,
    /// Drawn in a dedicated stacked oscillator sub-pane with its own `PriceScale` and shared `TimeScale`.
    OscillatorPane,
}

/// Primitive geometry family for a single output series.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SeriesRenderKind {
    Line,
    Histogram,
}

/// Horizontal reference guide inside an oscillator pane (e.g., `RSI` `30`/`50`/`70` or `MACD` `0`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReferenceLevel {
    pub value: f64,
    pub label: &'static str,
}

/// One computed output series aligned index-for-index (`len == candles.len()`) with the input `Candle` slice.
#[derive(Debug, Clone, PartialEq)]
pub struct IndicatorSeriesOutput {
    pub key: &'static str,
    pub label: String,
    pub kind: SeriesRenderKind,
    pub values: Vec<Option<f64>>,
}

/// Complete polymorphic evaluation result returned by any `Indicator`.
#[derive(Debug, Clone, PartialEq)]
pub struct IndicatorOutput {
    pub id: &'static str,
    pub title: String,
    pub placement: IndicatorPlacement,
    pub series: Vec<IndicatorSeriesOutput>,
    pub reference_levels: Vec<ReferenceLevel>,
    pub fixed_bounds_hint: Option<(f64, f64)>,
}

/// Core domain trait implemented by all Sumi technical indicators.
pub trait Indicator: Send + Sync {
    fn id(&self) -> &'static str;
    fn title(&self) -> String;
    fn placement(&self) -> IndicatorPlacement;
    fn warmup_bars(&self) -> usize;
    fn evaluate(&self, candles: &[Candle]) -> IndicatorOutput;
}

/// Identifier for stacked oscillator / strategy sub-panes below the main candlestick pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OscillatorPaneId {
    Rsi,
    Macd,
    Atr,
    Equity,
}

impl OscillatorPaneId {
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Rsi => "osc-rsi",
            Self::Macd => "osc-macd",
            Self::Atr => "osc-atr",
            Self::Equity => "osc-equity",
        }
    }

    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::Rsi => "RSI",
            Self::Macd => "MACD",
            Self::Atr => "ATR",
            Self::Equity => "EQUITY",
        }
    }
}

/// Serializable workstation indicator configuration with validation/sanitization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct IndicatorConfig {
    // Main-pane price overlays
    pub show_sma: bool,
    pub sma_period: usize,
    pub show_ema: bool,
    pub ema_period: usize,
    pub show_bbands: bool,
    pub bb_period: usize,
    pub bb_std_mult: f64,

    // Stacked oscillator sub-panes
    pub show_rsi: bool,
    pub rsi_period: usize,
    pub show_macd: bool,
    pub macd_fast: usize,
    pub macd_slow: usize,
    pub macd_signal: usize,
    pub show_atr: bool,
    pub atr_period: usize,
}

impl Default for IndicatorConfig {
    fn default() -> Self {
        Self {
            show_sma: true,
            sma_period: 20,
            show_ema: true,
            ema_period: 50,
            show_bbands: false,
            bb_period: 20,
            bb_std_mult: 2.0,
            show_rsi: true,
            rsi_period: 14,
            show_macd: false,
            macd_fast: 12,
            macd_slow: 26,
            macd_signal: 9,
            show_atr: false,
            atr_period: 14,
        }
    }
}

impl IndicatorConfig {
    /// Clamps deserialized or user-edited parameters into valid mathematical domains.
    pub fn sanitize_deserialized(&mut self) {
        self.sma_period = self.sma_period.clamp(1, 5_000);
        self.ema_period = self.ema_period.clamp(1, 5_000);
        self.bb_period = self.bb_period.clamp(2, 5_000);
        if !self.bb_std_mult.is_finite() {
            self.bb_std_mult = 2.0;
        } else {
            self.bb_std_mult = self.bb_std_mult.clamp(0.1, 20.0);
        }
        self.rsi_period = self.rsi_period.clamp(2, 5_000);
        self.macd_fast = self.macd_fast.clamp(2, 5_000);
        self.macd_slow = self.macd_slow.clamp(2, 5_000);
        if self.macd_slow < self.macd_fast {
            std::mem::swap(&mut self.macd_fast, &mut self.macd_slow);
        }
        self.macd_signal = self.macd_signal.clamp(1, 5_000);
        self.atr_period = self.atr_period.clamp(1, 5_000);
    }

    /// Returns the ordered list of active oscillator sub-panes below the main price pane.
    #[must_use]
    pub fn active_oscillator_panes(&self) -> Vec<OscillatorPaneId> {
        let mut panes = Vec::with_capacity(3);
        if self.show_rsi {
            panes.push(OscillatorPaneId::Rsi);
        }
        if self.show_macd {
            panes.push(OscillatorPaneId::Macd);
        }
        if self.show_atr {
            panes.push(OscillatorPaneId::Atr);
        }
        panes
    }

    #[must_use]
    pub const fn any_overlay_enabled(&self) -> bool {
        self.show_sma || self.show_ema || self.show_bbands
    }
}

/// Precomputed 16-bar associative extrema & continuity summary enabling $16\times$ faster exact
/// range-bounds, gap-faithful polyline decimation, and zero-bleed ribbon rendering on `100,000+`-bar
/// series with bit-for-bit identical output.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlockExtrema {
    pub first_idx: usize,
    pub first_val: f64,
    pub last_idx: usize,
    pub last_val: f64,
    pub min_idx: usize,
    pub min_val: f64,
    pub max_idx: usize,
    pub max_val: f64,
    pub dom_val: f64,
    pub valid_count: u8,
}

/// Hierarchical 16-bar block index companion to an `Option<f64>` indicator series.
#[derive(Debug, Clone, Default)]
pub struct SeriesBlockIndex {
    blocks: Vec<Option<BlockExtrema>>,
}

impl SeriesBlockIndex {
    pub const BLOCK_SIZE: usize = 16;

    #[must_use]
    pub fn build(series: &[Option<f64>]) -> Self {
        let mut blocks = Vec::with_capacity(series.len().div_ceil(Self::BLOCK_SIZE));
        for (blk_i, chunk) in series.chunks(Self::BLOCK_SIZE).enumerate() {
            let base = blk_i * Self::BLOCK_SIZE;
            let mut first_pair: Option<(usize, f64)> = None;
            let mut last_pair: Option<(usize, f64)> = None;
            let mut min_pair: Option<(usize, f64)> = None;
            let mut max_pair: Option<(usize, f64)> = None;
            let mut dom_val: Option<f64> = None;
            let mut valid_count = 0_u8;

            for (offset, item) in chunk.iter().enumerate() {
                if let Some(v) = item.filter(|v| v.is_finite()) {
                    let abs_i = base + offset;
                    valid_count += 1;
                    if first_pair.is_none() {
                        first_pair = Some((abs_i, v));
                    }
                    last_pair = Some((abs_i, v));
                    if min_pair.is_none_or(|(_, mv)| v < mv) {
                        min_pair = Some((abs_i, v));
                    }
                    if max_pair.is_none_or(|(_, mv)| v > mv) {
                        max_pair = Some((abs_i, v));
                    }
                    if dom_val.is_none_or(|dv| v.abs() >= dv.abs()) {
                        dom_val = Some(v);
                    }
                }
            }

            if let (
                Some((first_idx, first_val)),
                Some((last_idx, last_val)),
                Some((min_idx, min_val)),
                Some((max_idx, max_val)),
                Some(dom),
            ) = (first_pair, last_pair, min_pair, max_pair, dom_val)
            {
                blocks.push(Some(BlockExtrema {
                    first_idx,
                    first_val,
                    last_idx,
                    last_val,
                    min_idx,
                    min_val,
                    max_idx,
                    max_val,
                    dom_val: dom,
                    valid_count,
                }));
            } else {
                blocks.push(None);
            }
        }
        Self { blocks }
    }

    pub fn clear(&mut self) {
        self.blocks.clear();
    }

    #[inline]
    #[must_use]
    pub fn is_synced_with(&self, series: &[Option<f64>]) -> bool {
        self.blocks.len() == series.len().div_ceil(Self::BLOCK_SIZE)
    }

    /// Returns exact `(min, max)` of `series[range]` in $O(|range| / 16)$ operations.
    #[must_use]
    pub fn range_bounds(&self, series: &[Option<f64>], range: Range<usize>) -> Option<(f64, f64)> {
        let start = range.start.min(series.len());
        let end = range.end.min(series.len());
        if start >= end {
            return None;
        }
        if end - start < Self::BLOCK_SIZE * 2 || !self.is_synced_with(series) {
            return visible_series_bounds(&[series], start..end);
        }

        let mut min_v = f64::INFINITY;
        let mut max_v = f64::NEG_INFINITY;

        let first_aligned = start.div_ceil(Self::BLOCK_SIZE) * Self::BLOCK_SIZE;
        let last_aligned = (end / Self::BLOCK_SIZE) * Self::BLOCK_SIZE;

        if first_aligned > last_aligned {
            return visible_series_bounds(&[series], start..end);
        }

        for v in series[start..first_aligned].iter().flatten() {
            if v.is_finite() {
                min_v = min_v.min(*v);
                max_v = max_v.max(*v);
            }
        }

        let b_start = first_aligned / Self::BLOCK_SIZE;
        let b_end = last_aligned / Self::BLOCK_SIZE;
        for blk in self.blocks[b_start..b_end].iter().flatten() {
            min_v = min_v.min(blk.min_val);
            max_v = max_v.max(blk.max_val);
        }

        for v in series[last_aligned..end].iter().flatten() {
            if v.is_finite() {
                min_v = min_v.min(*v);
                max_v = max_v.max(*v);
            }
        }

        (min_v.is_finite() && max_v.is_finite()).then_some((min_v, max_v))
    }
}

/// Dirty-tracked cache of computed indicator series (plus 16-bar block indices) so panning,
/// zooming, resizing, and `100,000`-bar full-fit rendering remain well below `4.0 ms`.
#[derive(Debug, Clone, Default)]
pub struct IndicatorCache {
    dataset_generation: u64,
    dataset_len: usize,
    applied_config: Option<IndicatorConfig>,
    pub sma: Vec<Option<f64>>,
    pub sma_idx: SeriesBlockIndex,
    pub ema: Vec<Option<f64>>,
    pub ema_idx: SeriesBlockIndex,
    pub bbands: BollingerOutput,
    pub bb_upper_idx: SeriesBlockIndex,
    pub bb_middle_idx: SeriesBlockIndex,
    pub bb_lower_idx: SeriesBlockIndex,
    pub rsi: Vec<Option<f64>>,
    pub rsi_idx: SeriesBlockIndex,
    pub macd: MacdOutput,
    pub macd_line_idx: SeriesBlockIndex,
    pub macd_signal_idx: SeriesBlockIndex,
    pub macd_hist_idx: SeriesBlockIndex,
    pub atr: Vec<Option<f64>>,
    pub atr_idx: SeriesBlockIndex,
}

impl IndicatorCache {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the sanitized `IndicatorConfig` last applied by `sync`, if any.
    #[must_use]
    pub const fn applied_config(&self) -> Option<&IndicatorConfig> {
        self.applied_config.as_ref()
    }

    /// Ensures all currently enabled indicators in `config` are up-to-date for `(candles, dataset_generation)`.
    /// Recomputes **only** the specific indicator(s) whose toggle, parameter, or dataset generation changed.
    pub fn sync(&mut self, candles: &[Candle], dataset_generation: u64, config: &IndicatorConfig) {
        let mut clean_cfg = config.clone();
        clean_cfg.sanitize_deserialized();

        let dataset_changed =
            self.dataset_generation != dataset_generation || self.dataset_len != candles.len();

        let prev_cfg = if dataset_changed {
            self.sma.clear();
            self.sma_idx.clear();
            self.ema.clear();
            self.ema_idx.clear();
            self.bbands = BollingerOutput::default();
            self.bb_upper_idx.clear();
            self.bb_middle_idx.clear();
            self.bb_lower_idx.clear();
            self.rsi.clear();
            self.rsi_idx.clear();
            self.macd = MacdOutput::default();
            self.macd_line_idx.clear();
            self.macd_signal_idx.clear();
            self.macd_hist_idx.clear();
            self.atr.clear();
            self.atr_idx.clear();
            None
        } else {
            self.applied_config.as_ref()
        };

        // Share a single `closes` extraction only when at least one close-based indicator needs evaluation!
        let need_sma = clean_cfg.show_sma
            && (self.sma.len() != candles.len()
                || prev_cfg.is_none_or(|p| p.sma_period != clean_cfg.sma_period));
        let need_ema = clean_cfg.show_ema
            && (self.ema.len() != candles.len()
                || prev_cfg.is_none_or(|p| p.ema_period != clean_cfg.ema_period));
        let need_bb = clean_cfg.show_bbands
            && (self.bbands.middle.len() != candles.len()
                || prev_cfg.is_none_or(|p| {
                    p.bb_period != clean_cfg.bb_period
                        || (p.bb_std_mult - clean_cfg.bb_std_mult).abs() > 1e-12
                }));
        let need_rsi = clean_cfg.show_rsi
            && (self.rsi.len() != candles.len()
                || prev_cfg.is_none_or(|p| p.rsi_period != clean_cfg.rsi_period));
        let need_macd = clean_cfg.show_macd
            && (self.macd.macd_line.len() != candles.len()
                || prev_cfg.is_none_or(|p| {
                    p.macd_fast != clean_cfg.macd_fast
                        || p.macd_slow != clean_cfg.macd_slow
                        || p.macd_signal != clean_cfg.macd_signal
                }));
        let need_atr = clean_cfg.show_atr
            && (self.atr.len() != candles.len()
                || prev_cfg.is_none_or(|p| p.atr_period != clean_cfg.atr_period));

        if need_sma || need_ema || need_bb || need_rsi || need_macd {
            let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();
            if need_sma {
                self.sma = compute_sma_slices(&closes, clean_cfg.sma_period);
                self.sma_idx = SeriesBlockIndex::build(&self.sma);
            }
            if need_ema {
                self.ema = compute_ema_slices(&closes, clean_cfg.ema_period);
                self.ema_idx = SeriesBlockIndex::build(&self.ema);
            }
            if need_bb {
                self.bbands =
                    compute_bollinger_slices(&closes, clean_cfg.bb_period, clean_cfg.bb_std_mult);
                self.bb_upper_idx = SeriesBlockIndex::build(&self.bbands.upper);
                self.bb_middle_idx = SeriesBlockIndex::build(&self.bbands.middle);
                self.bb_lower_idx = SeriesBlockIndex::build(&self.bbands.lower);
            }
            if need_rsi {
                self.rsi = compute_rsi_slices(&closes, clean_cfg.rsi_period);
                self.rsi_idx = SeriesBlockIndex::build(&self.rsi);
            }
            if need_macd {
                self.macd = compute_macd_slices(
                    &closes,
                    clean_cfg.macd_fast,
                    clean_cfg.macd_slow,
                    clean_cfg.macd_signal,
                );
                self.macd_line_idx = SeriesBlockIndex::build(&self.macd.macd_line);
                self.macd_signal_idx = SeriesBlockIndex::build(&self.macd.signal_line);
                self.macd_hist_idx = SeriesBlockIndex::build(&self.macd.histogram);
            }
        }
        if need_atr {
            self.atr = compute_atr(candles, clean_cfg.atr_period);
            self.atr_idx = SeriesBlockIndex::build(&self.atr);
        }

        self.dataset_generation = dataset_generation;
        self.dataset_len = candles.len();
        self.applied_config = Some(clean_cfg);
    }
}

/// One projected `(logical_index, value)` point emitted by bucket-aligned conflated line decimation,
/// or `None` when a gap (`None` warmup/missing value) breaks polyline continuity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConflatedLineSample {
    pub center_logical_index: f64,
    pub value: f64,
}

/// One conflated histogram bar (`MACD` histogram) aligned to the exact global `step`-bar grid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConflatedHistogramBar {
    pub center_logical_index: f64,
    pub value: f64,
    pub prev_value: Option<f64>,
}

/// Visits bucket-aligned conflated line segments across `data_range` using the exact global
/// `[k * step, (k + 1) * step)` alignment of `iter_conflated_candles` so panning by 1px never shifts
/// buckets or causes visual shimmer.
pub fn visit_conflated_line_segments<F>(
    series: &[Option<f64>],
    data_range: Range<usize>,
    conflation_factor: usize,
    on_segment_point: F,
) where
    F: FnMut(Option<ConflatedLineSample>),
{
    visit_conflated_line_segments_indexed(
        series,
        None,
        data_range,
        conflation_factor,
        on_segment_point,
    );
}

/// Chronological entry/exit envelope & gap flags for one conflated `[bucket_start, bucket_end)` bucket.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BucketLineEnvelope {
    pub center_logical_index: f64,
    pub entry_val: f64,
    pub exit_val: f64,
    pub is_single: bool,
    pub has_leading_gap: bool,
    pub has_internal_gap: bool,
    pub has_trailing_gap: bool,
}

/// Computes the exact chronological `(entry_val, exit_val)` and `None`-gap continuity flags for
/// `series[bucket_start..bucket_end]` in $O(\text{step} / 16)$ when `block_index` is aligned.
#[must_use]
pub fn bucket_line_envelope_indexed(
    series: &[Option<f64>],
    block_index: Option<&SeriesBlockIndex>,
    bucket_start: usize,
    bucket_end: usize,
) -> Option<BucketLineEnvelope> {
    if bucket_start >= bucket_end || bucket_end > series.len() {
        return None;
    }
    let step = bucket_end - bucket_start;
    let center_x = if step == 1 {
        bucket_start as f64
    } else {
        ((bucket_start + (bucket_end - 1)) as f64) * 0.5
    };

    let mut first_pair: Option<(usize, f64)> = None;
    let mut last_pair: Option<(usize, f64)> = None;
    let mut min_pair: Option<(usize, f64)> = None;
    let mut max_pair: Option<(usize, f64)> = None;
    let mut valid_count = 0_usize;

    let use_blocks = step >= SeriesBlockIndex::BLOCK_SIZE
        && bucket_start.is_multiple_of(SeriesBlockIndex::BLOCK_SIZE)
        && (bucket_end == series.len() || bucket_end.is_multiple_of(SeriesBlockIndex::BLOCK_SIZE))
        && block_index.is_some_and(|bi| bi.is_synced_with(series));

    if use_blocks && let Some(bi) = block_index {
        let b_start = bucket_start / SeriesBlockIndex::BLOCK_SIZE;
        let b_end = bucket_end.div_ceil(SeriesBlockIndex::BLOCK_SIZE);
        for blk in bi.blocks[b_start..b_end].iter().flatten() {
            valid_count += usize::from(blk.valid_count);
            if first_pair.is_none() {
                first_pair = Some((blk.first_idx, blk.first_val));
            }
            last_pair = Some((blk.last_idx, blk.last_val));
            if min_pair.is_none_or(|(_, mv)| blk.min_val < mv) {
                min_pair = Some((blk.min_idx, blk.min_val));
            }
            if max_pair.is_none_or(|(_, mv)| blk.max_val > mv) {
                max_pair = Some((blk.max_idx, blk.max_val));
            }
        }
    } else {
        for (offset, item) in series[bucket_start..bucket_end].iter().enumerate() {
            if let Some(v) = item.filter(|v| v.is_finite()) {
                let abs_i = bucket_start + offset;
                valid_count += 1;
                if first_pair.is_none() {
                    first_pair = Some((abs_i, v));
                }
                last_pair = Some((abs_i, v));
                if min_pair.is_none_or(|(_, mv)| v < mv) {
                    min_pair = Some((abs_i, v));
                }
                if max_pair.is_none_or(|(_, mv)| v > mv) {
                    max_pair = Some((abs_i, v));
                }
            }
        }
    }

    let ((first_idx, first_val), (last_idx, last_val), (min_idx, min_val), (max_idx, max_val)) =
        (first_pair?, last_pair?, min_pair?, max_pair?);

    let has_leading_gap = first_idx > bucket_start;
    let has_trailing_gap = last_idx + 1 < bucket_end;
    let has_internal_gap = valid_count < (last_idx - first_idx + 1);

    let (entry_val, exit_val, is_single) = if has_internal_gap {
        (first_val, last_val, false)
    } else if min_idx == max_idx || (max_val - min_val).abs() <= 1e-12 {
        (min_val, min_val, true)
    } else if min_idx < max_idx {
        (min_val, max_val, false)
    } else {
        (max_val, min_val, false)
    };

    Some(BucketLineEnvelope {
        center_logical_index: center_x,
        entry_val,
        exit_val,
        is_single,
        has_leading_gap,
        has_internal_gap,
        has_trailing_gap,
    })
}

/// Block-index-accelerated variant of `visit_conflated_line_segments` ($16\times$ faster when `step >= 16`
/// and `step % 16 == 0`, with bit-for-bit identical output and exact `None`-gap breaks).
pub fn visit_conflated_line_segments_indexed<F>(
    series: &[Option<f64>],
    block_index: Option<&SeriesBlockIndex>,
    data_range: Range<usize>,
    conflation_factor: usize,
    mut on_segment_point: F,
) where
    F: FnMut(Option<ConflatedLineSample>),
{
    let step = conflation_factor.max(1);
    let total = series.len();
    let clamped_start = data_range.start.min(total);
    let clamped_end = data_range.end.min(total);
    if clamped_start >= clamped_end {
        return;
    }

    if step == 1 {
        let mut in_seg = false;
        for (idx, item) in series
            .iter()
            .enumerate()
            .take(clamped_end)
            .skip(clamped_start)
        {
            if let Some(v) = item.filter(|v| v.is_finite()) {
                on_segment_point(Some(ConflatedLineSample {
                    center_logical_index: idx as f64,
                    value: v,
                }));
                in_seg = true;
            } else if in_seg {
                on_segment_point(None);
                in_seg = false;
            }
        }
        return;
    }

    let aligned_start = (clamped_start / step) * step;
    let aligned_end = clamped_end.div_ceil(step).saturating_mul(step).min(total);
    let mut bucket_start = aligned_start;
    let mut in_seg = false;

    while bucket_start < aligned_end {
        let bucket_end = (bucket_start + step).min(total);
        if let Some(env) =
            bucket_line_envelope_indexed(series, block_index, bucket_start, bucket_end)
        {
            if env.has_leading_gap && in_seg {
                on_segment_point(None);
            }
            if env.has_internal_gap {
                on_segment_point(Some(ConflatedLineSample {
                    center_logical_index: env.center_logical_index,
                    value: env.entry_val,
                }));
                on_segment_point(None);
                on_segment_point(Some(ConflatedLineSample {
                    center_logical_index: env.center_logical_index,
                    value: env.exit_val,
                }));
            } else if env.is_single {
                on_segment_point(Some(ConflatedLineSample {
                    center_logical_index: env.center_logical_index,
                    value: env.entry_val,
                }));
            } else {
                on_segment_point(Some(ConflatedLineSample {
                    center_logical_index: env.center_logical_index,
                    value: env.entry_val,
                }));
                on_segment_point(Some(ConflatedLineSample {
                    center_logical_index: env.center_logical_index,
                    value: env.exit_val,
                }));
            }
            if env.has_trailing_gap {
                on_segment_point(None);
                in_seg = false;
            } else {
                in_seg = true;
            }
        } else if in_seg {
            on_segment_point(None);
            in_seg = false;
        }

        bucket_start = bucket_end;
    }
}

/// Visits bucket-aligned conflated histogram bars (`MACD` histogram) across `data_range`,
/// selecting the dominant magnitude value inside each `[k * step, (k + 1) * step)` bucket so peak
/// momentum spikes remain visible when zoomed out to `100,000` bars.
pub fn visit_conflated_histogram_bars<F>(
    series: &[Option<f64>],
    data_range: Range<usize>,
    conflation_factor: usize,
    on_bar: F,
) where
    F: FnMut(ConflatedHistogramBar),
{
    visit_conflated_histogram_bars_indexed(series, None, data_range, conflation_factor, on_bar);
}

/// Block-index-accelerated variant of `visit_conflated_histogram_bars`.
pub fn visit_conflated_histogram_bars_indexed<F>(
    series: &[Option<f64>],
    block_index: Option<&SeriesBlockIndex>,
    data_range: Range<usize>,
    conflation_factor: usize,
    mut on_bar: F,
) where
    F: FnMut(ConflatedHistogramBar),
{
    let step = conflation_factor.max(1);
    let total = series.len();
    let clamped_start = data_range.start.min(total);
    let clamped_end = data_range.end.min(total);
    if clamped_start >= clamped_end {
        return;
    }

    if step == 1 {
        let mut prev_finite = clamped_start
            .checked_sub(1)
            .and_then(|p| series.get(p).copied().flatten())
            .filter(|v| v.is_finite());
        for (idx, item) in series
            .iter()
            .enumerate()
            .take(clamped_end)
            .skip(clamped_start)
        {
            if let Some(v) = item.filter(|v| v.is_finite()) {
                on_bar(ConflatedHistogramBar {
                    center_logical_index: idx as f64,
                    value: v,
                    prev_value: prev_finite,
                });
                prev_finite = Some(v);
            }
        }
        return;
    }

    let aligned_start = (clamped_start / step) * step;
    let aligned_end = clamped_end.div_ceil(step).saturating_mul(step).min(total);
    let mut bucket_start = aligned_start;
    let use_blocks = step.is_multiple_of(SeriesBlockIndex::BLOCK_SIZE)
        && block_index.is_some_and(|bi| bi.is_synced_with(series));

    let bucket_dom = |b_s: usize, b_e: usize| -> Option<f64> {
        if use_blocks && let Some(bi) = block_index {
            let i0 = b_s / SeriesBlockIndex::BLOCK_SIZE;
            let i1 = b_e.div_ceil(SeriesBlockIndex::BLOCK_SIZE);
            let mut best: Option<f64> = None;
            for blk in bi.blocks[i0..i1].iter().flatten() {
                if best.is_none_or(|b| blk.dom_val.abs() >= b.abs()) {
                    best = Some(blk.dom_val);
                }
            }
            best
        } else {
            dominant_bucket_value(&series[b_s..b_e])
        }
    };

    let mut prev_bucket_val: Option<f64> = if aligned_start >= step {
        bucket_dom(aligned_start - step, aligned_start)
    } else {
        None
    };

    while bucket_start < aligned_end {
        let bucket_end = (bucket_start + step).min(total);
        if let Some(dom) = bucket_dom(bucket_start, bucket_end) {
            let center_x = ((bucket_start + (bucket_end - 1)) as f64) * 0.5;
            on_bar(ConflatedHistogramBar {
                center_logical_index: center_x,
                value: dom,
                prev_value: prev_bucket_val,
            });
            prev_bucket_val = Some(dom);
        }
        bucket_start = bucket_end;
    }
}

#[inline]
fn dominant_bucket_value(slice: &[Option<f64>]) -> Option<f64> {
    let mut best: Option<f64> = None;
    for item in slice {
        if let Some(v) = item.filter(|v| v.is_finite())
            && best.is_none_or(|b| v.abs() >= b.abs())
        {
            best = Some(v);
        }
    }
    best
}

/// Computes `(min, max)` across multiple `Option<f64>` series over `visible_range`.
#[must_use]
pub fn visible_series_bounds(
    series_list: &[&[Option<f64>]],
    visible_range: Range<usize>,
) -> Option<(f64, f64)> {
    let mut min_v = f64::INFINITY;
    let mut max_v = f64::NEG_INFINITY;

    for series in series_list {
        let start = visible_range.start.min(series.len());
        let end = visible_range.end.min(series.len());
        if start >= end {
            continue;
        }
        for v in series[start..end].iter().flatten() {
            if v.is_finite() {
                min_v = min_v.min(*v);
                max_v = max_v.max(*v);
            }
        }
    }

    (min_v.is_finite() && max_v.is_finite()).then_some((min_v, max_v))
}
