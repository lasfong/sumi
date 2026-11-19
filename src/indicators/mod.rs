//! Pure `f64`, `wasm32-unknown-unknown`-compatible Technical Indicator Engine (`Milestone R04 / Wave 2`).
//!
//! Verified against open-source TA-Lib (`ta_SMA.c`, `ta_EMA.c`, `ta_BBANDS.c`, `ta_RSI.c`,
//! `ta_MACD.c`, `ta_ATR.c`) and legacy Sumi (`indicatorDomain.ts` / `IndicatorRenderRegistry.ts`).
//! Contains zero UI or OS-specific dependencies.

pub mod bands;
pub mod cvd;
pub mod extended;
pub mod ichimoku;
pub mod market_profile;
pub mod moving_averages;
pub mod oscillators;
pub mod patterns;
pub mod proxy_bb;
pub mod relative_strength;
pub mod volume_profile;
pub mod vwap;

#[cfg(test)]
mod tests;

use std::ops::Range;

use serde::{Deserialize, Serialize};

use crate::data::Timeframe;
use crate::model::Candle;
use crate::viewport::iter_conflated_candles;

pub use bands::{
    BollingerBandsIndicator, BollingerOutput, compute_bollinger_bands, compute_bollinger_slices,
};
pub use cvd::{
    CVD_REFERENCE_LEVELS, CvdConfig, CvdOutput, CvdResetMode, compute_bar_volume_delta, compute_cvd,
};
pub use extended::{
    ADX_FIXED_BOUNDS_HINT, ADX_REFERENCE_LEVELS, AdxIndicator, AdxOutput, CCI_REFERENCE_LEVELS,
    CciIndicator, KeltnerIndicator, KeltnerOutput, MFI_FIXED_BOUNDS_HINT, MFI_REFERENCE_LEVELS,
    MfiIndicator, PsarIndicator, PsarOutput, STOCH_FIXED_BOUNDS_HINT, STOCH_REFERENCE_LEVELS,
    StochasticIndicator, StochasticOutput, SuperTrendIndicator, SuperTrendOutput,
    VolumeSmaIndicator, compute_adx, compute_cci, compute_keltner_channels, compute_mfi,
    compute_psar, compute_stochastic, compute_supertrend, compute_volume_sma,
};
pub use ichimoku::{
    IchimokuConfig, IchimokuIndicator, IchimokuOutput, MAX_ICHIMOKU_DISPLACEMENT, compute_ichimoku,
    donchian_midpoint_series,
};
pub use market_profile::{
    DEFAULT_MAX_PROFILE_SESSIONS, MAX_MARKET_PROFILE_ROWS, MIN_MARKET_PROFILE_ROWS,
    MarketProfileConfig, MarketProfileOutput, ProfileSessionPeriod, SessionMarketProfile,
    TpoPriceRow, compute_session_market_profiles,
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
pub use patterns::{
    CandleGeometry, CandlePatternBarSummary, CandlestickPatternKind,
    compute_candlestick_patterns_and_scores,
};
pub use proxy_bb::{
    BBDirection, BBHorizon, BBRegime, DecomposedBarFlow, PROXY_BB_FIXED_BOUNDS_HINT,
    PROXY_BB_METHODOLOGY_VERSION, PROXY_BB_REFERENCE_LEVELS, ProxyBBBarPoint, ProxyBBHorizonPoint,
    ProxyBBSeriesOutput, compute_proxy_bb_series, decompose_bar_ohlcv_proxy,
};
pub use relative_strength::{
    MAX_BENCHMARK_CARRY_SECONDS, RS_REFERENCE_LEVELS, RelativeStrengthOutput,
    compute_rebased_comparison_series, compute_relative_strength,
};
pub use volume_profile::{
    MAX_VOLUME_PROFILE_ROWS, MIN_VOLUME_PROFILE_ROWS, VolumeProfileBin, VolumeProfileConfig,
    VolumeProfileResult, compute_volume_profile, intrabar_buy_fraction,
};
pub use vwap::{
    VwapAnchorMode, VwapBandsOutput, compute_anchored_vwap, compute_session_vwap,
    compute_session_vwap_with_anchor, session_bucket_key, vwap_bucket_key,
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
    RelativeStrength,
    Cvd,
    Adx,
    Stochastic,
    Cci,
    Mfi,
    ProxyBb,
}

impl OscillatorPaneId {
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Rsi => "osc-rsi",
            Self::Macd => "osc-macd",
            Self::Atr => "osc-atr",
            Self::Equity => "osc-equity",
            Self::RelativeStrength => "osc-rs",
            Self::Cvd => "osc-cvd",
            Self::Adx => "osc-adx",
            Self::Stochastic => "osc-stoch",
            Self::Cci => "osc-cci",
            Self::Mfi => "osc-mfi",
            Self::ProxyBb => "osc-proxy-bb",
        }
    }

    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::Rsi => "RSI",
            Self::Macd => "MACD",
            Self::Atr => "ATR",
            Self::Equity => "EQUITY",
            Self::RelativeStrength => "RS",
            Self::Cvd => "CVD",
            Self::Adx => "ADX",
            Self::Stochastic => "STOCH",
            Self::Cci => "CCI",
            Self::Mfi => "MFI",
            Self::ProxyBb => "PROXY-BB",
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
    pub show_vwap: bool,
    pub vwap_anchor_mode: VwapAnchorMode,
    pub vwap_band_1: f64,
    pub vwap_band_2: f64,
    pub show_vrvp: bool,
    pub vrvp_rows: usize,
    pub vrvp_value_area_pct: f64,
    pub show_market_profile: bool,
    pub mp_session_period: ProfileSessionPeriod,
    pub mp_num_bins: usize,
    pub mp_ib_bars: usize,
    pub mp_extend_naked_poc: bool,
    pub show_ichimoku: bool,
    pub ichimoku_tenkan: usize,
    pub ichimoku_kijun: usize,
    pub ichimoku_senkou_b: usize,
    pub ichimoku_displacement: usize,
    pub show_ichimoku_chikou: bool,
    pub show_supertrend: bool,
    pub supertrend_period: usize,
    pub supertrend_multiplier: f64,
    pub show_psar: bool,
    pub psar_af_start: f64,
    pub psar_af_step: f64,
    pub psar_af_max: f64,
    pub show_keltner: bool,
    pub keltner_ema_period: usize,
    pub keltner_atr_period: usize,
    pub keltner_multiplier: f64,
    pub show_volume_sma: bool,
    pub volume_sma_period: usize,
    pub show_candle_patterns: bool,

    // Stacked oscillator sub-panes
    pub show_rsi: bool,
    pub rsi_period: usize,
    pub show_macd: bool,
    pub macd_fast: usize,
    pub macd_slow: usize,
    pub macd_signal: usize,
    pub show_atr: bool,
    pub atr_period: usize,
    pub show_rs: bool,
    pub rs_sma_period: usize,
    pub show_cvd: bool,
    pub cvd_reset_mode: CvdResetMode,
    pub cvd_sma_period: usize,
    pub show_adx: bool,
    pub adx_period: usize,
    pub show_stochastic: bool,
    pub stoch_k_period: usize,
    pub stoch_k_smooth: usize,
    pub stoch_d_period: usize,
    pub show_cci: bool,
    pub cci_period: usize,
    pub show_mfi: bool,
    pub mfi_period: usize,
    pub show_proxy_bb: bool,
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
            show_vwap: false,
            vwap_anchor_mode: VwapAnchorMode::AutoSession,
            vwap_band_1: 1.0,
            vwap_band_2: 2.0,
            show_vrvp: false,
            vrvp_rows: 40,
            vrvp_value_area_pct: 0.70,
            show_market_profile: false,
            mp_session_period: ProfileSessionPeriod::Auto,
            mp_num_bins: 24,
            mp_ib_bars: 4,
            mp_extend_naked_poc: true,
            show_ichimoku: false,
            ichimoku_tenkan: 9,
            ichimoku_kijun: 26,
            ichimoku_senkou_b: 52,
            ichimoku_displacement: 26,
            show_ichimoku_chikou: true,
            show_supertrend: false,
            supertrend_period: 10,
            supertrend_multiplier: 3.0,
            show_psar: false,
            psar_af_start: 0.02,
            psar_af_step: 0.02,
            psar_af_max: 0.20,
            show_keltner: false,
            keltner_ema_period: 20,
            keltner_atr_period: 10,
            keltner_multiplier: 2.0,
            show_volume_sma: false,
            volume_sma_period: 20,
            show_candle_patterns: false,
            show_rsi: true,
            rsi_period: 14,
            show_macd: false,
            macd_fast: 12,
            macd_slow: 26,
            macd_signal: 9,
            show_atr: false,
            atr_period: 14,
            show_rs: false,
            rs_sma_period: 20,
            show_cvd: false,
            cvd_reset_mode: CvdResetMode::Session,
            cvd_sma_period: 9,
            show_adx: false,
            adx_period: 14,
            show_stochastic: false,
            stoch_k_period: 14,
            stoch_k_smooth: 3,
            stoch_d_period: 3,
            show_cci: false,
            cci_period: 20,
            show_mfi: false,
            mfi_period: 14,
            show_proxy_bb: false,
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
        if !self.vwap_band_1.is_finite() {
            self.vwap_band_1 = 1.0;
        } else {
            self.vwap_band_1 = self.vwap_band_1.clamp(0.1, 10.0);
        }
        if !self.vwap_band_2.is_finite() {
            self.vwap_band_2 = 2.0;
        } else {
            self.vwap_band_2 = self.vwap_band_2.clamp(0.2, 15.0);
        }
        if self.vwap_band_2 < self.vwap_band_1 {
            std::mem::swap(&mut self.vwap_band_1, &mut self.vwap_band_2);
        }
        self.vrvp_rows = self
            .vrvp_rows
            .clamp(MIN_VOLUME_PROFILE_ROWS, MAX_VOLUME_PROFILE_ROWS);
        if !self.vrvp_value_area_pct.is_finite() {
            self.vrvp_value_area_pct = 0.70;
        } else {
            self.vrvp_value_area_pct = self.vrvp_value_area_pct.clamp(0.10, 0.95);
        }
        self.mp_num_bins = self
            .mp_num_bins
            .clamp(MIN_MARKET_PROFILE_ROWS, MAX_MARKET_PROFILE_ROWS);
        self.mp_ib_bars = self.mp_ib_bars.clamp(1, 60);
        self.ichimoku_tenkan = self.ichimoku_tenkan.clamp(1, 500);
        self.ichimoku_kijun = self.ichimoku_kijun.clamp(1, 500);
        self.ichimoku_senkou_b = self.ichimoku_senkou_b.clamp(1, 500);
        self.ichimoku_displacement = self.ichimoku_displacement.min(MAX_ICHIMOKU_DISPLACEMENT);
        self.supertrend_period = self.supertrend_period.clamp(1, 500);
        if !self.supertrend_multiplier.is_finite() {
            self.supertrend_multiplier = 3.0;
        } else {
            self.supertrend_multiplier = self.supertrend_multiplier.clamp(0.1, 20.0);
        }
        if !self.psar_af_max.is_finite() {
            self.psar_af_max = 0.20;
        } else {
            self.psar_af_max = self.psar_af_max.clamp(0.001, 1.0);
        }
        if !self.psar_af_start.is_finite() {
            self.psar_af_start = 0.02_f64.min(self.psar_af_max);
        } else {
            self.psar_af_start = self.psar_af_start.clamp(0.001, self.psar_af_max);
        }
        if !self.psar_af_step.is_finite() {
            self.psar_af_step = 0.02_f64.min(self.psar_af_max);
        } else {
            self.psar_af_step = self.psar_af_step.clamp(0.001, self.psar_af_max);
        }
        self.keltner_ema_period = self.keltner_ema_period.clamp(1, 5_000);
        self.keltner_atr_period = self.keltner_atr_period.clamp(1, 5_000);
        if !self.keltner_multiplier.is_finite() {
            self.keltner_multiplier = 2.0;
        } else {
            self.keltner_multiplier = self.keltner_multiplier.clamp(0.1, 20.0);
        }
        self.volume_sma_period = self.volume_sma_period.clamp(1, 5_000);
        self.rsi_period = self.rsi_period.clamp(2, 5_000);
        self.macd_fast = self.macd_fast.clamp(2, 5_000);
        self.macd_slow = self.macd_slow.clamp(2, 5_000);
        if self.macd_slow < self.macd_fast {
            std::mem::swap(&mut self.macd_fast, &mut self.macd_slow);
        }
        self.macd_signal = self.macd_signal.clamp(1, 5_000);
        self.atr_period = self.atr_period.clamp(1, 5_000);
        self.rs_sma_period = self.rs_sma_period.clamp(1, 5_000);
        self.cvd_sma_period = self.cvd_sma_period.clamp(1, 500);
        self.adx_period = self.adx_period.clamp(1, 500);
        self.stoch_k_period = self.stoch_k_period.clamp(1, 500);
        self.stoch_k_smooth = self.stoch_k_smooth.clamp(1, 100);
        self.stoch_d_period = self.stoch_d_period.clamp(1, 100);
        self.cci_period = self.cci_period.clamp(1, 500);
        self.mfi_period = self.mfi_period.clamp(1, 500);
    }

    /// Returns the ordered list of active oscillator sub-panes below the main price pane.
    #[must_use]
    pub fn active_oscillator_panes(&self) -> Vec<OscillatorPaneId> {
        let mut panes = Vec::with_capacity(10);
        if self.show_rsi {
            panes.push(OscillatorPaneId::Rsi);
        }
        if self.show_macd {
            panes.push(OscillatorPaneId::Macd);
        }
        if self.show_atr {
            panes.push(OscillatorPaneId::Atr);
        }
        if self.show_adx {
            panes.push(OscillatorPaneId::Adx);
        }
        if self.show_stochastic {
            panes.push(OscillatorPaneId::Stochastic);
        }
        if self.show_cci {
            panes.push(OscillatorPaneId::Cci);
        }
        if self.show_mfi {
            panes.push(OscillatorPaneId::Mfi);
        }
        if self.show_proxy_bb {
            panes.push(OscillatorPaneId::ProxyBb);
        }
        if self.show_cvd {
            panes.push(OscillatorPaneId::Cvd);
        }
        if self.show_rs {
            panes.push(OscillatorPaneId::RelativeStrength);
        }
        panes
    }

    #[must_use]
    pub const fn any_overlay_enabled(&self) -> bool {
        self.show_sma
            || self.show_ema
            || self.show_bbands
            || self.show_vwap
            || self.show_vrvp
            || self.show_market_profile
            || self.show_ichimoku
            || self.show_supertrend
            || self.show_psar
            || self.show_keltner
            || self.show_volume_sma
            || self.show_candle_patterns
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
    pub valid_sum: f64,
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
            let mut valid_sum = 0.0_f64;
            let mut valid_count = 0_u8;

            for (offset, item) in chunk.iter().enumerate() {
                if let Some(v) = item.filter(|v| v.is_finite()) {
                    let abs_i = base + offset;
                    valid_sum += v;
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
                    valid_sum,
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

    /// Returns the first index `i` in `range` where `series[i]` is finite, skipping empty `16`-bar
    /// blocks via the precomputed block index.
    #[must_use]
    pub fn first_valid_index(&self, series: &[Option<f64>], range: Range<usize>) -> Option<usize> {
        let start = range.start.min(series.len());
        let end = range.end.min(series.len());
        if start >= end {
            return None;
        }
        if !self.is_synced_with(series) {
            return (start..end).find(|&i| series[i].is_some_and(f64::is_finite));
        }
        let first_blk = start / Self::BLOCK_SIZE;
        let last_blk = (end - 1) / Self::BLOCK_SIZE;
        for blk_idx in first_blk..=last_blk {
            if let Some(blk) = &self.blocks[blk_idx] {
                if blk.first_idx >= start && blk.first_idx < end {
                    return Some(blk.first_idx);
                }
                let sub_s = start.max(blk_idx * Self::BLOCK_SIZE);
                let sub_e = end.min((blk_idx + 1) * Self::BLOCK_SIZE);
                if let Some(found) = (sub_s..sub_e).find(|&i| series[i].is_some_and(f64::is_finite))
                {
                    return Some(found);
                }
            }
        }
        None
    }

    /// Returns exact `(sum, valid_count)` of finite values in `series[bucket_start..bucket_end]`
    /// using hybrid head + `16`-bar blocks + tail acceleration whenever `step >= 16`, with exact slice fallback.
    #[must_use]
    pub fn bucket_sum_and_count(
        &self,
        series: &[Option<f64>],
        bucket_start: usize,
        bucket_end: usize,
    ) -> Option<(f64, usize)> {
        let start = bucket_start.min(series.len());
        let end = bucket_end.min(series.len());
        if start >= end {
            return None;
        }
        let step = end - start;
        if step >= Self::BLOCK_SIZE && self.is_synced_with(series) {
            let first_aligned = start.div_ceil(Self::BLOCK_SIZE) * Self::BLOCK_SIZE;
            let last_aligned = (end / Self::BLOCK_SIZE) * Self::BLOCK_SIZE;
            if first_aligned <= last_aligned {
                let mut sum = 0.0_f64;
                let mut count = 0_usize;
                for slice in [&series[start..first_aligned], &series[last_aligned..end]] {
                    for item in slice {
                        if let Some(v) = item.filter(|v| v.is_finite()) {
                            sum += v;
                            count += 1;
                        }
                    }
                }
                let b_start = first_aligned / Self::BLOCK_SIZE;
                let b_end = last_aligned / Self::BLOCK_SIZE;
                for blk in self.blocks[b_start..b_end].iter().flatten() {
                    sum += blk.valid_sum;
                    count += usize::from(blk.valid_count);
                }
                if count == 0 {
                    return None;
                }
                if sum.is_finite() {
                    return Some((sum, count));
                }
            }
        }

        let mut sum = 0.0_f64;
        let mut count = 0_usize;
        for item in &series[start..end] {
            if let Some(v) = item.filter(|v| v.is_finite()) {
                sum += v;
                count += 1;
            }
        }
        (count > 0).then_some((sum, count))
    }
}

/// Compact `Copy` 16-bar OHLCV summary block eliminating per-frame `String`-padded `Candle` scans
/// during `16x`..`512x` conflated autoscale and chart painting.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CandleBlock {
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume_sum: f64,
    pub first_idx: u32,
    pub last_idx: u32,
}

/// Hierarchical 16-bar OHLCV block index for zero-allocation conflated autoscale and rendering.
#[derive(Debug, Clone, Default)]
pub struct CandleBlockIndex {
    blocks: Vec<CandleBlock>,
    source_len: usize,
}

impl CandleBlockIndex {
    pub const BLOCK_SIZE: usize = 16;

    /// Builds a compact `16`-bar OHLCV summary index for `candles`.
    #[must_use]
    pub fn build(candles: &[Candle]) -> Self {
        let mut blocks = Vec::with_capacity(candles.len().div_ceil(Self::BLOCK_SIZE));
        for (blk_i, chunk) in candles.chunks(Self::BLOCK_SIZE).enumerate() {
            let base = blk_i * Self::BLOCK_SIZE;
            let first = &chunk[0];
            let last = &chunk[chunk.len() - 1];
            let mut high = first.high;
            let mut low = first.low;
            let mut volume_sum = 0.0_f64;
            for c in chunk {
                high = high.max(c.high);
                low = low.min(c.low);
                volume_sum += c.volume;
            }
            blocks.push(CandleBlock {
                open: first.open,
                high,
                low,
                close: last.close,
                volume_sum,
                first_idx: base as u32,
                last_idx: (base + chunk.len() - 1) as u32,
            });
        }
        Self {
            blocks,
            source_len: candles.len(),
        }
    }

    pub fn clear(&mut self) {
        self.blocks.clear();
        self.source_len = 0;
    }

    #[inline]
    #[must_use]
    pub const fn source_len(&self) -> usize {
        self.source_len
    }

    #[inline]
    #[must_use]
    pub fn is_synced_with(&self, candles: &[Candle]) -> bool {
        self.source_len == candles.len()
            && self.blocks.len() == candles.len().div_ceil(Self::BLOCK_SIZE)
    }

    /// Iterates conflated OHLCV buckets matching `iter_conflated_candles` alignment in $O(N / 16)$
    /// whenever `conflation_factor >= 16` is a multiple of `16` and the block index is synced.
    pub fn for_each_conflated_bucket<F>(
        &self,
        candles: &[Candle],
        slice_range: Range<usize>,
        conflation_factor: usize,
        mut on_bar: F,
    ) where
        F: FnMut(f64, f64, f64, f64, f64, f64, bool),
    {
        let step = conflation_factor.max(1);
        let clamped_end = slice_range.end.min(candles.len());
        let raw_start = slice_range.start.min(clamped_end);
        if raw_start >= clamped_end {
            return;
        }
        if step >= Self::BLOCK_SIZE
            && step.is_multiple_of(Self::BLOCK_SIZE)
            && self.is_synced_with(candles)
        {
            let aligned_start = (raw_start / step) * step;
            let aligned_end = clamped_end
                .div_ceil(step)
                .saturating_mul(step)
                .min(candles.len());
            let blocks_per_step = step / Self::BLOCK_SIZE;
            let mut b_idx = aligned_start / Self::BLOCK_SIZE;
            let total_blocks = self.blocks.len();
            let b_limit = aligned_end.div_ceil(Self::BLOCK_SIZE).min(total_blocks);

            while b_idx < b_limit {
                let b_chunk_end = (b_idx + blocks_per_step).min(b_limit);
                let first_blk = &self.blocks[b_idx];
                let last_blk = &self.blocks[b_chunk_end - 1];
                let mut high = first_blk.high;
                let mut low = first_blk.low;
                let mut volume = 0.0_f64;
                for blk in &self.blocks[b_idx..b_chunk_end] {
                    high = high.max(blk.high);
                    low = low.min(blk.low);
                    volume += blk.volume_sum;
                }
                let start_i = first_blk.first_idx as usize;
                let end_i = last_blk.last_idx as usize;
                let center = if start_i == end_i {
                    start_i as f64
                } else {
                    (start_i + end_i) as f64 * 0.5
                };
                let open = first_blk.open;
                let close = last_blk.close;
                on_bar(center, open, high, low, close, volume, close >= open);
                b_idx = b_chunk_end;
            }
            return;
        }

        for bar in iter_conflated_candles(candles, raw_start..clamped_end, step) {
            on_bar(
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

    /// Computes `(raw_min, raw_max, max_vol)` across `strict_range` matching `iter_conflated_candles`.
    #[must_use]
    pub fn autoscale_bounds(
        &self,
        candles: &[Candle],
        strict_range: Range<usize>,
        conflation_factor: usize,
    ) -> (f64, f64, f64) {
        let mut raw_min = f64::INFINITY;
        let mut raw_max = f64::NEG_INFINITY;
        let mut max_vol = 0.0_f64;
        self.for_each_conflated_bucket(
            candles,
            strict_range,
            conflation_factor,
            |_, _, high, low, _, volume, _| {
                if low.is_finite() {
                    raw_min = raw_min.min(low);
                }
                if high.is_finite() {
                    raw_max = raw_max.max(high);
                }
                if volume.is_finite() {
                    max_vol = max_vol.max(volume);
                }
            },
        );
        (raw_min, raw_max, max_vol)
    }
}

/// Dirty-tracked cache of computed indicator series (plus 16-bar block indices) so panning,
/// zooming, resizing, and `100,000`-bar full-fit rendering remain well below `4.0 ms`.
#[derive(Debug, Clone, Default)]
pub struct IndicatorCache {
    dataset_generation: u64,
    dataset_len: usize,
    applied_config: Option<IndicatorConfig>,
    pub candle_idx: CandleBlockIndex,
    pub sma: Vec<Option<f64>>,
    pub sma_idx: SeriesBlockIndex,
    pub ema: Vec<Option<f64>>,
    pub ema_idx: SeriesBlockIndex,
    pub bbands: BollingerOutput,
    pub bb_upper_idx: SeriesBlockIndex,
    pub bb_middle_idx: SeriesBlockIndex,
    pub bb_lower_idx: SeriesBlockIndex,
    pub vwap: VwapBandsOutput,
    pub vwap_idx: SeriesBlockIndex,
    pub vwap_u1_idx: SeriesBlockIndex,
    pub vwap_l1_idx: SeriesBlockIndex,
    pub vwap_u2_idx: SeriesBlockIndex,
    pub vwap_l2_idx: SeriesBlockIndex,
    vwap_context: Option<(Timeframe, i64, VwapAnchorMode)>,
    pub market_profile: MarketProfileOutput,
    mp_computed_cfg: Option<(Timeframe, MarketProfileConfig)>,
    pub rsi: Vec<Option<f64>>,
    pub rsi_idx: SeriesBlockIndex,
    pub macd: MacdOutput,
    pub macd_line_idx: SeriesBlockIndex,
    pub macd_signal_idx: SeriesBlockIndex,
    pub macd_hist_idx: SeriesBlockIndex,
    pub atr: Vec<Option<f64>>,
    pub atr_idx: SeriesBlockIndex,
    pub ichimoku: IchimokuOutput,
    pub ichimoku_tenkan_idx: SeriesBlockIndex,
    pub ichimoku_kijun_idx: SeriesBlockIndex,
    pub ichimoku_senkou_a_idx: SeriesBlockIndex,
    pub ichimoku_senkou_b_idx: SeriesBlockIndex,
    pub ichimoku_chikou_idx: SeriesBlockIndex,
    ichimoku_computed_cfg: Option<IchimokuConfig>,
    pub cvd: CvdOutput,
    pub cvd_delta_idx: SeriesBlockIndex,
    pub cvd_line_idx: SeriesBlockIndex,
    pub cvd_sma_idx: SeriesBlockIndex,
    cvd_computed_cfg: Option<CvdConfig>,
    pub rs: RelativeStrengthOutput,
    pub rs_line_idx: SeriesBlockIndex,
    pub rs_sma_idx: SeriesBlockIndex,
    rs_cache_key: Option<RsCacheKey>,
    pub supertrend: SuperTrendOutput,
    pub supertrend_idx: SeriesBlockIndex,
    pub psar: PsarOutput,
    pub psar_idx: SeriesBlockIndex,
    pub keltner: KeltnerOutput,
    pub kc_upper_idx: SeriesBlockIndex,
    pub kc_middle_idx: SeriesBlockIndex,
    pub kc_lower_idx: SeriesBlockIndex,
    pub volume_sma: Vec<Option<f64>>,
    pub volume_sma_idx: SeriesBlockIndex,
    pub candle_patterns: Vec<CandlePatternBarSummary>,
    pub adx: AdxOutput,
    pub adx_line_idx: SeriesBlockIndex,
    pub plus_di_idx: SeriesBlockIndex,
    pub minus_di_idx: SeriesBlockIndex,
    pub stochastic: StochasticOutput,
    pub stoch_k_idx: SeriesBlockIndex,
    pub stoch_d_idx: SeriesBlockIndex,
    pub cci: Vec<Option<f64>>,
    pub cci_idx: SeriesBlockIndex,
    pub mfi: Vec<Option<f64>>,
    pub mfi_idx: SeriesBlockIndex,
    pub proxy_bb: ProxyBBSeriesOutput,
    pub proxy_bb_t05_idx: SeriesBlockIndex,
    pub proxy_bb_t20_idx: SeriesBlockIndex,
    pub proxy_bb_t50_idx: SeriesBlockIndex,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RsCacheKey {
    dataset_generation: u64,
    dataset_len: usize,
    symbol_tf: Timeframe,
    benchmark_key: String,
    benchmark_generation: u64,
    replay_causal_close_timestamp: Option<i64>,
    sma_period: usize,
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

    /// Returns the dataset generation token recorded by the most recent `sync` call.
    #[must_use]
    pub const fn dataset_generation(&self) -> u64 {
        self.dataset_generation
    }

    /// Ensures all currently enabled indicators in `config` are up-to-date for `(candles, dataset_generation)`.
    /// Uses daily UTC session defaults for `VWAP`; prefer [`Self::sync_with_session_context`] when
    /// venue timezone and active chart timeframe are known.
    pub fn sync(&mut self, candles: &[Candle], dataset_generation: u64, config: &IndicatorConfig) {
        let (tf, utc_off, _) =
            self.vwap_context
                .unwrap_or((Timeframe::D1, 0, VwapAnchorMode::AutoSession));
        self.sync_with_session_context(candles, dataset_generation, config, tf, utc_off);
    }

    /// Ensures all currently enabled indicators in `config` (including timezone-aware `Session VWAP`)
    /// are up-to-date for `(candles, dataset_generation, timeframe, utc_offset_seconds)`.
    pub fn sync_with_session_context(
        &mut self,
        candles: &[Candle],
        dataset_generation: u64,
        config: &IndicatorConfig,
        timeframe: Timeframe,
        utc_offset_seconds: i64,
    ) {
        let mut clean_cfg = config.clone();
        clean_cfg.sanitize_deserialized();

        let dataset_changed =
            self.dataset_generation != dataset_generation || self.dataset_len != candles.len();

        if candles.is_empty() {
            self.candle_idx.clear();
        } else if dataset_changed || self.candle_idx.source_len() != candles.len() {
            self.candle_idx = CandleBlockIndex::build(candles);
        }

        let prev_cfg = if dataset_changed {
            self.sma.clear();
            self.sma_idx.clear();
            self.ema.clear();
            self.ema_idx.clear();
            self.bbands = BollingerOutput::default();
            self.bb_upper_idx.clear();
            self.bb_middle_idx.clear();
            self.bb_lower_idx.clear();
            self.vwap = VwapBandsOutput::default();
            self.vwap_idx.clear();
            self.vwap_u1_idx.clear();
            self.vwap_l1_idx.clear();
            self.vwap_u2_idx.clear();
            self.vwap_l2_idx.clear();
            self.vwap_context = None;
            self.rsi.clear();
            self.rsi_idx.clear();
            self.macd = MacdOutput::default();
            self.macd_line_idx.clear();
            self.macd_signal_idx.clear();
            self.macd_hist_idx.clear();
            self.atr.clear();
            self.atr_idx.clear();
            self.ichimoku = IchimokuOutput::default();
            self.ichimoku_tenkan_idx.clear();
            self.ichimoku_kijun_idx.clear();
            self.ichimoku_senkou_a_idx.clear();
            self.ichimoku_senkou_b_idx.clear();
            self.ichimoku_chikou_idx.clear();
            self.ichimoku_computed_cfg = None;
            self.market_profile = MarketProfileOutput::default();
            self.mp_computed_cfg = None;
            self.cvd = CvdOutput::default();
            self.cvd_delta_idx.clear();
            self.cvd_line_idx.clear();
            self.cvd_sma_idx.clear();
            self.cvd_computed_cfg = None;
            self.rs = RelativeStrengthOutput::default();
            self.rs_line_idx.clear();
            self.rs_sma_idx.clear();
            self.rs_cache_key = None;
            self.supertrend = SuperTrendOutput::default();
            self.supertrend_idx.clear();
            self.psar = PsarOutput::default();
            self.psar_idx.clear();
            self.keltner = KeltnerOutput::default();
            self.kc_upper_idx.clear();
            self.kc_middle_idx.clear();
            self.kc_lower_idx.clear();
            self.volume_sma.clear();
            self.volume_sma_idx.clear();
            self.candle_patterns.clear();
            self.adx = AdxOutput::default();
            self.adx_line_idx.clear();
            self.plus_di_idx.clear();
            self.minus_di_idx.clear();
            self.stochastic = StochasticOutput::default();
            self.stoch_k_idx.clear();
            self.stoch_d_idx.clear();
            self.cci.clear();
            self.cci_idx.clear();
            self.mfi.clear();
            self.mfi_idx.clear();
            self.proxy_bb = ProxyBBSeriesOutput::default();
            self.proxy_bb_t05_idx.clear();
            self.proxy_bb_t20_idx.clear();
            self.proxy_bb_t50_idx.clear();
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
        let need_vwap = clean_cfg.show_vwap
            && (self.vwap.vwap.len() != candles.len()
                || self.vwap_context
                    != Some((timeframe, utc_offset_seconds, clean_cfg.vwap_anchor_mode))
                || prev_cfg.is_none_or(|p| {
                    (p.vwap_band_1 - clean_cfg.vwap_band_1).abs() > 1e-12
                        || (p.vwap_band_2 - clean_cfg.vwap_band_2).abs() > 1e-12
                }));
        let target_mp_cfg = MarketProfileConfig {
            session_period: clean_cfg.mp_session_period,
            utc_offset_seconds,
            num_bins: clean_cfg.mp_num_bins,
            value_area_pct: 0.70,
            ib_bars: clean_cfg.mp_ib_bars,
            max_sessions: DEFAULT_MAX_PROFILE_SESSIONS,
            extend_naked_poc: clean_cfg.mp_extend_naked_poc,
        };
        let need_mp = clean_cfg.show_market_profile
            && self.mp_computed_cfg != Some((timeframe, target_mp_cfg));
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
        let target_ichi_cfg = IchimokuConfig {
            tenkan_period: clean_cfg.ichimoku_tenkan,
            kijun_period: clean_cfg.ichimoku_kijun,
            senkou_b_period: clean_cfg.ichimoku_senkou_b,
            displacement: clean_cfg.ichimoku_displacement,
            show_chikou: clean_cfg.show_ichimoku_chikou,
        };
        let need_ichimoku = clean_cfg.show_ichimoku
            && (self.ichimoku.tenkan.len() != candles.len()
                || self.ichimoku_computed_cfg != Some(target_ichi_cfg));
        let target_cvd_cfg = CvdConfig {
            reset_mode: clean_cfg.cvd_reset_mode,
            timeframe,
            utc_offset_seconds,
            sma_period: clean_cfg.cvd_sma_period,
        };
        let need_cvd = clean_cfg.show_cvd
            && (self.cvd.cvd.len() != candles.len()
                || self.cvd_computed_cfg != Some(target_cvd_cfg));
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
        if need_vwap {
            self.vwap = compute_session_vwap_with_anchor(
                candles,
                timeframe,
                utc_offset_seconds,
                clean_cfg.vwap_anchor_mode,
                clean_cfg.vwap_band_1,
                clean_cfg.vwap_band_2,
            );
            self.vwap_idx = SeriesBlockIndex::build(&self.vwap.vwap);
            self.vwap_u1_idx = SeriesBlockIndex::build(&self.vwap.upper_1);
            self.vwap_l1_idx = SeriesBlockIndex::build(&self.vwap.lower_1);
            self.vwap_u2_idx = SeriesBlockIndex::build(&self.vwap.upper_2);
            self.vwap_l2_idx = SeriesBlockIndex::build(&self.vwap.lower_2);
            self.vwap_context = Some((timeframe, utc_offset_seconds, clean_cfg.vwap_anchor_mode));
        }
        if need_mp {
            self.market_profile =
                compute_session_market_profiles(candles, timeframe, target_mp_cfg);
            self.mp_computed_cfg = Some((timeframe, target_mp_cfg));
        }
        if need_atr {
            self.atr = compute_atr(candles, clean_cfg.atr_period);
            self.atr_idx = SeriesBlockIndex::build(&self.atr);
        }
        if need_ichimoku {
            self.ichimoku = compute_ichimoku(candles, target_ichi_cfg);
            self.ichimoku_tenkan_idx = SeriesBlockIndex::build(&self.ichimoku.tenkan);
            self.ichimoku_kijun_idx = SeriesBlockIndex::build(&self.ichimoku.kijun);
            self.ichimoku_senkou_a_idx = SeriesBlockIndex::build(&self.ichimoku.senkou_a);
            self.ichimoku_senkou_b_idx = SeriesBlockIndex::build(&self.ichimoku.senkou_b);
            self.ichimoku_chikou_idx = SeriesBlockIndex::build(&self.ichimoku.chikou);
            self.ichimoku_computed_cfg = Some(target_ichi_cfg);
        }
        if need_cvd {
            self.cvd = compute_cvd(candles, target_cvd_cfg);
            self.cvd_delta_idx = SeriesBlockIndex::build(&self.cvd.delta);
            self.cvd_line_idx = SeriesBlockIndex::build(&self.cvd.cvd);
            self.cvd_sma_idx = SeriesBlockIndex::build(&self.cvd.cvd_sma);
            self.cvd_computed_cfg = Some(target_cvd_cfg);
        }

        let need_supertrend = clean_cfg.show_supertrend
            && (self.supertrend.trend.len() != candles.len()
                || prev_cfg.is_none_or(|p| {
                    p.supertrend_period != clean_cfg.supertrend_period
                        || (p.supertrend_multiplier - clean_cfg.supertrend_multiplier).abs() > 1e-12
                }));
        if need_supertrend {
            self.supertrend = compute_supertrend(
                candles,
                clean_cfg.supertrend_period,
                clean_cfg.supertrend_multiplier,
            );
            self.supertrend_idx = SeriesBlockIndex::build(&self.supertrend.trend);
        }

        let need_psar = clean_cfg.show_psar
            && (self.psar.sar.len() != candles.len()
                || prev_cfg.is_none_or(|p| {
                    (p.psar_af_start - clean_cfg.psar_af_start).abs() > 1e-12
                        || (p.psar_af_step - clean_cfg.psar_af_step).abs() > 1e-12
                        || (p.psar_af_max - clean_cfg.psar_af_max).abs() > 1e-12
                }));
        if need_psar {
            self.psar = compute_psar(
                candles,
                clean_cfg.psar_af_start,
                clean_cfg.psar_af_step,
                clean_cfg.psar_af_max,
            );
            self.psar_idx = SeriesBlockIndex::build(&self.psar.sar);
        }

        let need_keltner = clean_cfg.show_keltner
            && (self.keltner.middle.len() != candles.len()
                || prev_cfg.is_none_or(|p| {
                    p.keltner_ema_period != clean_cfg.keltner_ema_period
                        || p.keltner_atr_period != clean_cfg.keltner_atr_period
                        || (p.keltner_multiplier - clean_cfg.keltner_multiplier).abs() > 1e-12
                }));
        if need_keltner {
            self.keltner = compute_keltner_channels(
                candles,
                clean_cfg.keltner_ema_period,
                clean_cfg.keltner_atr_period,
                clean_cfg.keltner_multiplier,
            );
            self.kc_upper_idx = SeriesBlockIndex::build(&self.keltner.upper);
            self.kc_middle_idx = SeriesBlockIndex::build(&self.keltner.middle);
            self.kc_lower_idx = SeriesBlockIndex::build(&self.keltner.lower);
        }

        let need_vol_sma = clean_cfg.show_volume_sma
            && (self.volume_sma.len() != candles.len()
                || prev_cfg.is_none_or(|p| p.volume_sma_period != clean_cfg.volume_sma_period));
        if need_vol_sma {
            self.volume_sma = compute_volume_sma(candles, clean_cfg.volume_sma_period);
            self.volume_sma_idx = SeriesBlockIndex::build(&self.volume_sma);
        }

        let need_candle_patterns =
            clean_cfg.show_candle_patterns && self.candle_patterns.len() != candles.len();
        if need_candle_patterns {
            let atr14_cow: Vec<Option<f64>>;
            let atr14_ref: &[Option<f64>] =
                if clean_cfg.atr_period == 14 && self.atr.len() == candles.len() {
                    &self.atr
                } else {
                    atr14_cow = compute_atr(candles, 14);
                    &atr14_cow
                };
            let (_, summaries) = compute_candlestick_patterns_and_scores(candles, atr14_ref);
            self.candle_patterns = summaries;
        }

        let need_adx = clean_cfg.show_adx
            && (self.adx.plus_di.len() != candles.len()
                || prev_cfg.is_none_or(|p| p.adx_period != clean_cfg.adx_period));
        if need_adx {
            self.adx = compute_adx(candles, clean_cfg.adx_period);
            self.adx_line_idx = SeriesBlockIndex::build(&self.adx.adx);
            self.plus_di_idx = SeriesBlockIndex::build(&self.adx.plus_di);
            self.minus_di_idx = SeriesBlockIndex::build(&self.adx.minus_di);
        }

        let need_stoch = clean_cfg.show_stochastic
            && (self.stochastic.slow_k.len() != candles.len()
                || prev_cfg.is_none_or(|p| {
                    p.stoch_k_period != clean_cfg.stoch_k_period
                        || p.stoch_k_smooth != clean_cfg.stoch_k_smooth
                        || p.stoch_d_period != clean_cfg.stoch_d_period
                }));
        if need_stoch {
            self.stochastic = compute_stochastic(
                candles,
                clean_cfg.stoch_k_period,
                clean_cfg.stoch_k_smooth,
                clean_cfg.stoch_d_period,
            );
            self.stoch_k_idx = SeriesBlockIndex::build(&self.stochastic.slow_k);
            self.stoch_d_idx = SeriesBlockIndex::build(&self.stochastic.slow_d);
        }

        let need_cci = clean_cfg.show_cci
            && (self.cci.len() != candles.len()
                || prev_cfg.is_none_or(|p| p.cci_period != clean_cfg.cci_period));
        if need_cci {
            self.cci = compute_cci(candles, clean_cfg.cci_period);
            self.cci_idx = SeriesBlockIndex::build(&self.cci);
        }

        let need_mfi = clean_cfg.show_mfi
            && (self.mfi.len() != candles.len()
                || prev_cfg.is_none_or(|p| p.mfi_period != clean_cfg.mfi_period));
        if need_mfi {
            self.mfi = compute_mfi(candles, clean_cfg.mfi_period);
            self.mfi_idx = SeriesBlockIndex::build(&self.mfi);
        }

        let need_proxy_bb = clean_cfg.show_proxy_bb && self.proxy_bb.points.len() != candles.len();
        if need_proxy_bb {
            self.proxy_bb = compute_proxy_bb_series(candles);
            self.proxy_bb_t05_idx = SeriesBlockIndex::build(&self.proxy_bb.t05_values);
            self.proxy_bb_t20_idx = SeriesBlockIndex::build(&self.proxy_bb.t20_values);
            self.proxy_bb_t50_idx = SeriesBlockIndex::build(&self.proxy_bb.t50_values);
        }

        self.dataset_generation = dataset_generation;
        self.dataset_len = candles.len();
        self.applied_config = Some(clean_cfg);
    }

    /// Synchronizes the causal Relative Strength (`RS vs Benchmark`) series and block indices.
    #[allow(clippy::too_many_arguments)]
    pub fn sync_relative_strength(
        &mut self,
        symbol_candles: &[Candle],
        dataset_generation: u64,
        symbol_tf: Timeframe,
        benchmark_key: &str,
        benchmark_generation: u64,
        benchmark_candles: &[Candle],
        benchmark_tf: Timeframe,
        sma_period: usize,
        replay_causal_close_timestamp: Option<i64>,
    ) {
        let clean_sma = sma_period.clamp(1, 5_000);
        let target_key = RsCacheKey {
            dataset_generation,
            dataset_len: symbol_candles.len(),
            symbol_tf,
            benchmark_key: benchmark_key.to_owned(),
            benchmark_generation,
            replay_causal_close_timestamp,
            sma_period: clean_sma,
        };
        if self.rs_cache_key.as_ref() == Some(&target_key)
            && self.rs.rs_line.len() == symbol_candles.len()
        {
            return;
        }
        self.rs = compute_relative_strength(
            symbol_candles,
            symbol_tf,
            benchmark_candles,
            benchmark_tf,
            clean_sma,
            replay_causal_close_timestamp,
        );
        self.rs_line_idx = SeriesBlockIndex::build(&self.rs.rs_line);
        self.rs_sma_idx = SeriesBlockIndex::build(&self.rs.rs_sma);
        self.rs_cache_key = Some(target_key);
    }

    /// Clears cached Relative Strength state when benchmark comparison is disabled.
    pub fn clear_relative_strength(&mut self) {
        if self.rs_cache_key.is_some() || !self.rs.rs_line.is_empty() {
            self.rs = RelativeStrengthOutput::default();
            self.rs_line_idx.clear();
            self.rs_sma_idx.clear();
            self.rs_cache_key = None;
        }
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

    #[derive(Default)]
    struct BucketEnvelopeAcc {
        first_pair: Option<(usize, f64)>,
        last_pair: Option<(usize, f64)>,
        min_pair: Option<(usize, f64)>,
        max_pair: Option<(usize, f64)>,
        valid_count: usize,
    }

    impl BucketEnvelopeAcc {
        #[inline]
        fn scan_subslice(&mut self, series: &[Option<f64>], sub_start: usize, sub_end: usize) {
            for (offset, item) in series[sub_start..sub_end].iter().enumerate() {
                if let Some(v) = item.filter(|v| v.is_finite()) {
                    let abs_i = sub_start + offset;
                    self.valid_count += 1;
                    if self.first_pair.is_none() {
                        self.first_pair = Some((abs_i, v));
                    }
                    self.last_pair = Some((abs_i, v));
                    if self.min_pair.is_none_or(|(_, mv)| v < mv) {
                        self.min_pair = Some((abs_i, v));
                    }
                    if self.max_pair.is_none_or(|(_, mv)| v > mv) {
                        self.max_pair = Some((abs_i, v));
                    }
                }
            }
        }
    }

    let mut acc = BucketEnvelopeAcc::default();
    let first_aligned =
        bucket_start.div_ceil(SeriesBlockIndex::BLOCK_SIZE) * SeriesBlockIndex::BLOCK_SIZE;
    let last_aligned = (bucket_end / SeriesBlockIndex::BLOCK_SIZE) * SeriesBlockIndex::BLOCK_SIZE;
    let use_hybrid_blocks = step >= SeriesBlockIndex::BLOCK_SIZE
        && first_aligned <= last_aligned
        && block_index.is_some_and(|bi| bi.is_synced_with(series));

    if use_hybrid_blocks && let Some(bi) = block_index {
        acc.scan_subslice(series, bucket_start, first_aligned);
        let b_start = first_aligned / SeriesBlockIndex::BLOCK_SIZE;
        let b_end = last_aligned / SeriesBlockIndex::BLOCK_SIZE;
        for blk in bi.blocks[b_start..b_end].iter().flatten() {
            acc.valid_count += usize::from(blk.valid_count);
            if acc.first_pair.is_none() {
                acc.first_pair = Some((blk.first_idx, blk.first_val));
            }
            acc.last_pair = Some((blk.last_idx, blk.last_val));
            if acc.min_pair.is_none_or(|(_, mv)| blk.min_val < mv) {
                acc.min_pair = Some((blk.min_idx, blk.min_val));
            }
            if acc.max_pair.is_none_or(|(_, mv)| blk.max_val > mv) {
                acc.max_pair = Some((blk.max_idx, blk.max_val));
            }
        }
        acc.scan_subslice(series, last_aligned, bucket_end);
    } else {
        acc.scan_subslice(series, bucket_start, bucket_end);
    }

    let ((first_idx, first_val), (last_idx, last_val), (min_idx, min_val), (max_idx, max_val)) = (
        acc.first_pair?,
        acc.last_pair?,
        acc.min_pair?,
        acc.max_pair?,
    );
    let valid_count = acc.valid_count;

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
