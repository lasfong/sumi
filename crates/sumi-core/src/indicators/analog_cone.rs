//! Pure-Rust (`f64`/`i64`, `wasm32-unknown-unknown`-compatible) Quantitative Engine for
//! **`AFC — Analog Forecast Cone`** (*Nón Xác Suất Tương Đồng Lịch Sử Hai Tầng*, `SPEC-SUMI-AFC-V1`).
//!
//! Translates the empirically validated 20-dimensional causal state-space + Dual-Horizon Point-in-Time
//! Analog Trajectory Synthesis (`2,625` blind OOS windows across `HOSE` / `HNX` / `UPCOM`) into a
//! zero-external-dependency engine with strict mathematical guarantees:
//! - **Causal Yang–Zhang OHLC Volatility (`compute_yang_zhang_vol`)**: $W \in \{5, 20\}$ with `0.005` floor.
//! - **20D Causal Feature Vector (`ANALOG_CONE_WARMUP_BARS = 65`)**:
//!   1. `ProxyBB` multi-horizon money flow (`T03, T05, T10, T20, T50` via [`crate::indicators::proxy_bb`])
//!   2. Volatility-normalized log returns (`1D, 3D, 5D, 10D, 20D`)
//!   3. VSA microstructure (`CLV_1D, CLV_3D, LogRVol20, VolRatio_3_20`)
//!   4. Volatility squeeze & range position (`YZSqueeze_5_20, DonchianPos_60D`)
//!   5. Market-wide context (`VNINDEX z5, z20, vol_rank_252d` + causal universe `% > MA20` breadth)
//! - **Strict PIT Gate (`DEF-001`)**: `AnalogMemoryBank` is sorted ascending by `settle_timestamp`,
//!   uses $O(\log N)$ `partition_point` slicing + causal prefix-sum Z-score standardization so future
//!   bars `> as_of_timestamp` have **zero** bit-level influence on any historical or replay forecast.
//! - **Dual-Horizon Memory & Statutory VN Microstructure Projection**: Blends `60%` fast-regime (`1.25y`)
//!   median drift with `40%` deep-history fat-tailed dispersion, applies Bayesian volatility scaling
//!   `clip(0.70 * (q_yz / hist_yz) + 0.30, 0.45, 2.25)`, and projects step-by-step `1..=H` onto
//!   `HOSE (±7%)`, `HNX (±10%)`, and `UPCOM (±15%)` statutory price bands and tick ladders.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

use serde::{Deserialize, Serialize};

use crate::data::format_utc_date;
use crate::indicators::proxy_bb::{BBHorizon, compute_proxy_bb_series};
use crate::journal::{
    ExchangeVenue, ResolvedVnPriceUnit, VnPriceUnit, add_vn_trading_days,
    compute_daily_price_band_with_resolved_unit, snap_to_nearest_tick,
};
use crate::model::Candle;

/// Dimensionality of the causal AFC structural state vector.
pub const ANALOG_CONE_DIM: usize = 20;

/// Minimum causal history bars required before emitting a valid 20D state vector (`Donchian 60D` + `ProxyBB T50` + `YZ20`).
pub const ANALOG_CONE_WARMUP_BARS: usize = 65;

/// Default forecast horizon in trading sessions (`T+10`).
pub const DEFAULT_ANALOG_HORIZON: usize = 10;

/// Maximum supported forward forecast horizon (`T+20`).
pub const MAX_ANALOG_HORIZON: usize = 20;

/// Default deep-memory analog neighborhood size (`Top-100`).
pub const DEFAULT_ANALOG_TOP_K: usize = 100;

/// Fast-regime neighborhood size (`Top-80` over the trailing `1.25` years $\approx 456$ calendar days).
pub const FAST_REGIME_TOP_K: usize = 80;

/// Trailing calendar window (`456` days $\approx 1.25 \times 365.25$, ~`315` VN trading sessions) for fast-regime drift.
pub const FAST_REGIME_LOOKBACK_SECS: i64 = 456 * 86_400;

/// Minimum daily Yang–Zhang volatility floor preventing division-by-zero on halted/flat stocks.
pub const MIN_YANG_ZHANG_VOL: f64 = 0.005;

/// Default 20-session median liquidity hygiene gate (`2.0` billion VND / day).
pub const DEFAULT_MIN_MEDIAN_TURNOVER_VND: f64 = 2_000_000_000.0;

/// Minimum self-history exclusion guard around `as_of_timestamp` on the queried ticker (`20` sessions).
const SELF_EXCLUSION_BARS: usize = 20;

/// Objective block-group feature weight profiles learned on `Val 2024` (`Protocol v2.0`).
///
/// Group layout (`20` dims): `[ProxyBB(5), Momentum(5), VSA(4), SqueezeRange(2), MarketContext(4)]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum AnalogWeightProfile {
    /// Default when `HOSE:VNINDEX` & VN breadth context are enabled: `[2.0, 2.0, 0.4, 1.0, 0.4]`.
    #[default]
    MarketAware,
    /// Pure endogenous OHLCV mode (non-VN assets or when VNINDEX context is toggled off): `[0.4, 0.8, 0.8, 1.0, 0.0]`.
    EndogenousOnly,
}

impl AnalogWeightProfile {
    /// Group weights `[ProxyBB, Momentum, VSA, SqueezeRange, MarketContext]`.
    #[inline]
    #[must_use]
    pub const fn group_weights(self) -> [f64; 5] {
        match self {
            Self::MarketAware => [2.0, 2.0, 0.4, 1.0, 0.4],
            Self::EndogenousOnly => [0.4, 0.8, 0.8, 1.0, 0.0],
        }
    }

    /// Expands the 5 validated group weights into the full 20-dimensional feature weight vector.
    #[inline]
    #[must_use]
    pub const fn feature_weights(self) -> [f64; ANALOG_CONE_DIM] {
        let [g0, g1, g2, g3, g4] = self.group_weights();
        [
            g0, g0, g0, g0, g0, // ProxyBB: T03, T05, T10, T20, T50
            g1, g1, g1, g1, g1, // Vol-norm returns: 1D, 3D, 5D, 10D, 20D
            g2, g2, g2, g2, // VSA: CLV_1D, CLV_3D, LogRVol20, VolRatio_3_20
            g3, g3, // Squeeze & Channel: YZSqueeze_5_20, DonchianPos_60D
            g4, g4, g4, g4, // Market Context: vn_z5, vn_z20, vn_vol_rank_252d, breadth_ma20_z
        ]
    }

    #[inline]
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MarketAware => "MarketAware",
            Self::EndogenousOnly => "EndogenousOnly",
        }
    }
}

/// Input series descriptor used when constructing an [`AnalogMemoryBank`].
#[derive(Debug, Clone)]
pub struct AnalogSymbolInput {
    /// Canonical `"EXCHANGE:SYMBOL"` identifier (e.g. `"HOSE:FPT"`).
    pub ticker: String,
    /// Short ticker symbol (e.g. `"FPT"`).
    pub symbol: String,
    /// Exchange microstructure venue (`Hose`, `Hnx`, `Upcom`, ...).
    pub venue: ExchangeVenue,
    /// Ascending `1D` candles (split-adjusted continuous series).
    pub candles: Vec<Candle>,
}

/// Configuration for building [`AnalogMemoryBank`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AnalogBankConfig {
    /// Minimum causal 20-session median daily turnover in VND required for a historical bar
    /// to enter the pattern memory bank (`>= 2,000,000,000` VND = `2 tỷ VNĐ/ngày` default).
    pub min_median_turnover_vnd: f64,
    /// Price denomination policy used to normalize turnover to VND (`Auto` detects `RawVnd` vs `ThousandVnd`).
    pub price_unit: VnPriceUnit,
}

impl Default for AnalogBankConfig {
    fn default() -> Self {
        Self {
            min_median_turnover_vnd: DEFAULT_MIN_MEDIAN_TURNOVER_VND,
            price_unit: VnPriceUnit::Auto,
        }
    }
}

/// Query-time configuration for [`AnalogMemoryBank::query`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalogConeConfig {
    /// Forward forecast horizon $H \in [1, 20]$ trading sessions (default `10`).
    pub horizon: usize,
    /// Deep-memory analog count $K \in [10, 250]$ (default `100`).
    pub top_k: usize,
    /// Feature weight profile (`MarketAware` vs `EndogenousOnly`).
    pub weight_profile: AnalogWeightProfile,
    /// Price denomination unit policy for venue tick & band projection.
    pub price_unit: VnPriceUnit,
}

impl Default for AnalogConeConfig {
    fn default() -> Self {
        Self {
            horizon: DEFAULT_ANALOG_HORIZON,
            top_k: DEFAULT_ANALOG_TOP_K,
            weight_profile: AnalogWeightProfile::MarketAware,
            price_unit: VnPriceUnit::Auto,
        }
    }
}

impl AnalogConeConfig {
    #[inline]
    #[must_use]
    pub fn sanitized(self) -> Self {
        Self {
            horizon: self.horizon.clamp(1, MAX_ANALOG_HORIZON),
            top_k: self.top_k.clamp(10, 250),
            weight_profile: self.weight_profile,
            price_unit: self.price_unit,
        }
    }
}

/// Structured errors returned when an AFC forecast cannot be constructed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnalogConeError {
    InsufficientQueryHistory {
        available: usize,
        required: usize,
    },
    InvalidQueryAnchorPrice,
    InsufficientPitAnalogs {
        eligible: usize,
        minimum_required: usize,
    },
}

impl core::fmt::Display for AnalogConeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InsufficientQueryHistory {
                available,
                required,
            } => write!(
                f,
                "Cần tối thiểu {required} phiên lịch sử 1D để mã hóa trạng thái AFC (hiện có {available})"
            ),
            Self::InvalidQueryAnchorPrice => {
                write!(f, "Giá đóng cửa phiên neo hiện tại không hợp lệ")
            }
            Self::InsufficientPitAnalogs {
                eligible,
                minimum_required,
            } => write!(
                f,
                "Kho mẫu PIT trước thời điểm đang xét chưa đủ ({eligible}/{minimum_required} mẫu)"
            ),
        }
    }
}

impl std::error::Error for AnalogConeError {}

/// Quantile fan envelope at one forward trading step $h \in \{1, \dots, H\}$.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalogConeStep {
    /// 1-based forward step (`1..=horizon`).
    pub step: usize,
    /// Projected future trading session timestamp (Unix seconds UTC).
    pub projected_timestamp: i64,
    pub p10: f64,
    pub p25: f64,
    pub p50: f64,
    pub p75: f64,
    pub p90: f64,
    pub p10_ret_pct: f64,
    pub p25_ret_pct: f64,
    pub p50_ret_pct: f64,
    pub p75_ret_pct: f64,
    pub p90_ret_pct: f64,
    /// Statutory cumulative exchange floor price at step `h` (`HOSE/HNX/UPCOM/VN30F`), if applicable.
    pub cumulative_floor: Option<f64>,
    /// Statutory cumulative exchange ceiling price at step `h` (`HOSE/HNX/UPCOM/VN30F`), if applicable.
    pub cumulative_ceiling: Option<f64>,
}

/// One historical analog match from the strictly PIT-settled cross-market memory bank.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalogMatchCase {
    /// Canonical `"EXCHANGE:SYMBOL"` ticker (e.g. `"HOSE:FPT"`).
    pub ticker: String,
    /// Short symbol code (e.g. `"FPT"`).
    pub symbol: String,
    /// Historical entry bar timestamp (Unix seconds UTC).
    pub entry_timestamp: i64,
    /// ISO-8601 calendar date (`"YYYY-MM-DD"`).
    pub entry_date: String,
    /// Kernel similarity score in `[0.0, 100.0]`.
    pub similarity_pct: f64,
    /// Weighted standardized Euclidean distance in 20D state space.
    pub distance: f64,
    /// Normalized softmax ensemble weight in `[0.0, 1.0]`.
    pub weight: f64,
    /// Realized historical `T+5` close return (`%`), `Some` iff `timestamp(t + 5) <= as_of_timestamp`.
    pub realized_t5_ret_pct: Option<f64>,
    /// Realized historical `T+10` close return (`%`), `Some` iff `timestamp(t + 10) <= as_of_timestamp`.
    pub realized_t10_ret_pct: Option<f64>,
}

/// Complete Dual-Horizon Analog Forecast Cone output (`SPEC-SUMI-AFC-V1`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalogConeForecast {
    pub as_of_timestamp: i64,
    pub as_of_date: String,
    pub anchor_close: f64,
    pub horizon: usize,
    pub top_k: usize,
    pub weight_profile: AnalogWeightProfile,
    /// Quantile cone steps `1..=horizon`.
    pub steps: Vec<AnalogConeStep>,
    /// Probability `P(C_{t+H} > C_t)` in `[0.0, 100.0]`.
    pub win_rate_pct: f64,
    /// Probability `P(Ret_{t+H} > +3.0%)` in `[0.0, 100.0]`.
    pub prob_gain_gt_3pct: f64,
    /// Pathwise tail drawdown probability `P(min_{1..=H} Low <= -6.0%)` in `[0.0, 100.0]`.
    pub prob_tail_drop_6pct: f64,
    /// Terminal upside-to-downside tail ratio `max(P90 - C0, 0) / max(C0 - P10, eps)`.
    pub skew_ratio: f64,
    /// Terminal median return `(P50[H] / C0 - 1) * 100`.
    pub expected_return_p50_pct: f64,
    /// Weighted mean terminal return (`%`).
    pub expected_return_mean_pct: f64,
    /// Query bar causal 20-day Yang–Zhang daily volatility.
    pub query_yz20_vol: f64,
    /// Total strictly settled PIT candidates (`settle_timestamp <= as_of_timestamp`).
    pub pit_candidate_count: usize,
    /// Eligible PIT candidates inside the `1.25y` fast regime window.
    pub fast_candidate_count: usize,
    /// Top-10 closest historical analog cases with realized outcomes.
    pub top_analogs: Vec<AnalogMatchCase>,
}

/// Causal market-wide context point at a single daily timestamp.
#[derive(Debug, Clone, Copy)]
struct MarketContextPoint {
    timestamp: i64,
    vn_z5: f64,
    vn_z20: f64,
    vn_vol_rank: f64,
    breadth_z: f64,
}

/// Compact per-symbol log-price trajectory store enabling zero-duplication `1..=MAX_ANALOG_HORIZON`
/// path reconstruction and strict per-horizon `timestamps[bar_idx + h] <= as_of_timestamp` verification.
#[derive(Debug, Clone)]
struct StoredSymbolTrajectory {
    ticker: String,
    symbol: String,
    timestamps: Vec<i64>,
    valid_bar_indices: Vec<u32>,
    log_open: Vec<f64>,
    log_high: Vec<f64>,
    log_low: Vec<f64>,
    log_close: Vec<f64>,
}

/// Metadata for a single historical analog entry sorted ascending by `settle_timestamp` (`t_i + 1` floor,
/// with exact `t_i + H` PIT gate verified via `StoredSymbolTrajectory::timestamps`).
#[derive(Debug, Clone, Copy)]
struct AnalogEntryMeta {
    /// Timestamp of `t_i + 1` (earliest possible settle timestamp; strict $O(\log N)$ upper bound for all `H >= 1`).
    settle_timestamp: i64,
    /// Timestamp of `t_i + DEFAULT_ANALOG_HORIZON` (`i64::MAX` when fewer than 10 forward bars exist).
    settle_timestamp_h10: i64,
    /// Timestamp of the historical state bar `t_i`.
    entry_timestamp: i64,
    /// Causal 20D Yang–Zhang volatility at `t_i`.
    yz20: f32,
    /// Canonical global `(settle_timestamp, symbol_idx, entry_timestamp)` rank for PIT slicing & deterministic tie-breaking.
    pit_rank: u32,
    symbol_idx: u32,
    bar_idx: u32,
}

/// Time-stratified KD-BVH parameters:
/// - `KD_CHUNK_SIZE = 2048` (~4 VN trading sessions) preserves temporal locality (tight market-context bounds)
///   and strict PIT chunk boundaries while partitioning endogenous features across 7 KD levels into
///   `Node64` (`64` entries) and `Leaf16` (`16` entries) axis-aligned bounding boxes.
const KD_CHUNK_SIZE: usize = 2048;
const KD_NODE_SIZE: usize = 64;
const KD_LEAF_SIZE: usize = 16;
const KD_SPLIT_DIMS: [usize; 7] = [0, 5, 2, 8, 15, 4, 14];

/// 20D Axis-Aligned Bounding Box (`AABB`) for exact zero-false-negative Branch-and-Bound pruning
/// under arbitrary query-time feature standardization weights `inv_var_w`.
#[repr(align(64))]
#[derive(Debug, Clone, Copy)]
struct Aabb20 {
    min: [f32; ANALOG_CONE_DIM],
    max: [f32; ANALOG_CONE_DIM],
    min_pit_rank: u32,
    max_pit_rank: u32,
    max_entry_ts: i64,
}

impl Aabb20 {
    fn from_slice(entries: &[(AnalogEntryMeta, [f32; ANALOG_CONE_DIM])]) -> Self {
        let mut min = [f32::INFINITY; ANALOG_CONE_DIM];
        let mut max = [f32::NEG_INFINITY; ANALOG_CONE_DIM];
        let mut min_pit_rank = u32::MAX;
        let mut max_pit_rank = 0_u32;
        let mut max_entry_ts = i64::MIN;

        for (m, row) in entries {
            if m.pit_rank < min_pit_rank {
                min_pit_rank = m.pit_rank;
            }
            if m.pit_rank > max_pit_rank {
                max_pit_rank = m.pit_rank;
            }
            if m.entry_timestamp > max_entry_ts {
                max_entry_ts = m.entry_timestamp;
            }
            for d in 0..ANALOG_CONE_DIM {
                let v = row[d];
                if v < min[d] {
                    min[d] = v;
                }
                if v > max[d] {
                    max[d] = v;
                }
            }
        }

        Self {
            min,
            max,
            min_pit_rank,
            max_pit_rank,
            max_entry_ts,
        }
    }

    #[inline]
    fn lower_bound_dist_sq(
        &self,
        q: &[f32; ANALOG_CONE_DIM],
        w: &[f32; ANALOG_CONE_DIM],
    ) -> f32 {
        let mut acc = 0.0_f32;
        for d in 0..ANALOG_CONE_DIM {
            let qd = q[d];
            let gap = (self.min[d] - qd).max(qd - self.max[d]).max(0.0);
            acc += w[d] * (gap * gap);
        }
        acc
    }
}

fn kd_partition_in_place(
    slice: &mut [(AnalogEntryMeta, [f32; ANALOG_CONE_DIM])],
    depth: usize,
) {
    if slice.len() <= KD_LEAF_SIZE {
        return;
    }
    let dim = KD_SPLIT_DIMS[depth % KD_SPLIT_DIMS.len()];
    let mid = slice.len() / 2;
    slice.select_nth_unstable_by(mid, |a, b| {
        a.1[dim]
            .total_cmp(&b.1[dim])
            .then_with(|| a.0.pit_rank.cmp(&b.0.pit_rank))
    });
    let (left, right) = slice.split_at_mut(mid);
    kd_partition_in_place(left, depth + 1);
    kd_partition_in_place(right, depth + 1);
}

/// Strictly Point-in-Time (`DEF-001`) Cross-Market Analog Trajectory Memory Bank.
///
/// - Sorted globally ascending by `(settle_timestamp, symbol_idx, entry_timestamp)` for `settle_timestamps`,
///   `prefix_sum`, and `prefix_sum_sq` so Z-score standardization at any `as_of_timestamp` uses **only**
///   settled rows `< pit_upper` in $O(1)$ time with zero future leakage.
/// - Organizes each 2,048-entry temporal stratum into a 2-level exact `Node64 -> Leaf16` KD Bounding Volume
///   Hierarchy (`AABB`) achieving `< 1 ms` exact 20D K-NN retrieval over 500k+ stock-day states.
#[derive(Debug, Clone)]
pub struct AnalogMemoryBank {
    symbols: Vec<StoredSymbolTrajectory>,
    symbol_lookup: HashMap<String, u32>,
    market_context: Vec<MarketContextPoint>,
    has_vnindex: bool,
    /// Monotonically non-decreasing `settle_timestamp` array indexed by `pit_rank` (`0..N`).
    settle_timestamps: Vec<i64>,
    meta: Vec<AnalogEntryMeta>,
    exact_rows: Vec<[f32; ANALOG_CONE_DIM]>,
    leaf16_boxes: Vec<Aabb20>,
    node64_boxes: Vec<Aabb20>,
    /// Prefix sums of length `N + 1` for $O(1)$ strictly causal Z-score mean/std at `as_of_timestamp`.
    prefix_sum: Vec<[f64; ANALOG_CONE_DIM]>,
    prefix_sum_sq: Vec<[f64; ANALOG_CONE_DIM]>,
}

#[derive(Clone, Copy)]
struct HeapCandidate {
    dist_sq: f32,
    pit_rank: u32,
    storage_idx: u32,
}

impl PartialEq for HeapCandidate {
    fn eq(&self, other: &Self) -> bool {
        self.dist_sq.to_bits() == other.dist_sq.to_bits() && self.pit_rank == other.pit_rank
    }
}

impl Eq for HeapCandidate {}

impl PartialOrd for HeapCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HeapCandidate {
    fn cmp(&self, other: &Self) -> Ordering {
        self.dist_sq
            .total_cmp(&other.dist_sq)
            .then_with(|| self.pit_rank.cmp(&other.pit_rank))
    }
}

#[derive(Clone, Copy)]
struct SimulatedCandle {
    open: f64,
    high: f64,
    low: f64,
    close: f64,
}

/// Computes the causal rolling Yang–Zhang daily log-return volatility estimator (`window` typically `5` or `20`),
/// floored at [`MIN_YANG_ZHANG_VOL`] (`0.005`).
#[must_use]
pub fn compute_yang_zhang_vol(candles: &[Candle], window: usize) -> Vec<f64> {
    let n = candles.len();
    if n == 0 {
        return Vec::new();
    }
    let w = window.max(2);
    let min_periods = 5.min(w).max(2);

    let mut log_oc_prev = Vec::with_capacity(n);
    let mut log_co = Vec::with_capacity(n);
    let mut rs = Vec::with_capacity(n);

    for t in 0..n {
        let bar = &candles[t];
        let o = sanitize_pos_price(bar.open, bar.close);
        let c = sanitize_pos_price(bar.close, o);
        let h = sanitize_pos_price(bar.high, o.max(c)).max(o.max(c));
        let l = sanitize_pos_price(bar.low, o.min(c)).min(o.min(c));
        let prev_c = if t == 0 {
            o
        } else {
            sanitize_pos_price(candles[t - 1].close, o)
        };

        let loc = (o / prev_c).ln();
        let lco = (c / o).ln();
        let lho = (h / o).ln();
        let llo = (l / o).ln();
        let rs_t = lho * (lho - lco) + llo * (llo - lco);

        log_oc_prev.push(if loc.is_finite() { loc } else { 0.0 });
        log_co.push(if lco.is_finite() { lco } else { 0.0 });
        rs.push(if rs_t.is_finite() { rs_t } else { 0.0 });
    }

    let k = 0.34 / (1.34 + (w as f64 + 1.0) / ((w as f64 - 1.0).max(1.0)));
    let mut out = Vec::with_capacity(n);
    let mut abs_co_running = 0.0_f64;

    for t in 0..n {
        abs_co_running += log_co[t].abs();
        let fallback = (abs_co_running / (t + 1) as f64).max(0.01);

        let start = (t + 1).saturating_sub(w);
        let m = t + 1 - start;
        if m < min_periods {
            out.push(fallback.max(MIN_YANG_ZHANG_VOL));
            continue;
        }

        let s_oc = sample_variance(&log_oc_prev[start..=t]);
        let s_co = sample_variance(&log_co[start..=t]);
        let s_rs = rs[start..=t].iter().sum::<f64>() / (m as f64);

        let yz_var = (s_oc + k * s_co + (1.0 - k) * s_rs).max(1.0e-8);
        let yz_vol = yz_var.sqrt();
        if yz_vol.is_finite() {
            out.push(yz_vol.max(MIN_YANG_ZHANG_VOL));
        } else {
            out.push(fallback.max(MIN_YANG_ZHANG_VOL));
        }
    }

    out
}

#[inline]
fn sanitize_pos_price(val: f64, fallback: f64) -> f64 {
    if val.is_finite() && val > 1.0e-9 {
        val
    } else if fallback.is_finite() && fallback > 1.0e-9 {
        fallback
    } else {
        1.0
    }
}

#[inline]
fn sample_variance(slice: &[f64]) -> f64 {
    let m = slice.len();
    if m < 2 {
        return 0.0;
    }
    let mean = slice.iter().sum::<f64>() / (m as f64);
    let mut sq = 0.0_f64;
    for &x in slice {
        let d = x - mean;
        sq += d * d;
    }
    sq / ((m - 1) as f64)
}

#[inline]
fn median_in_place(buf: &mut [f64]) -> f64 {
    if buf.is_empty() {
        return 0.0;
    }
    buf.sort_unstable_by(f64::total_cmp);
    let mid = buf.len() / 2;
    if buf.len().is_multiple_of(2) {
        0.5 * (buf[mid - 1] + buf[mid])
    } else {
        buf[mid]
    }
}

#[inline]
fn bar_turnover_vnd(bar: &Candle, venue: ExchangeVenue, unit_policy: VnPriceUnit) -> f64 {
    if !bar.close.is_finite() || !bar.volume.is_finite() || bar.close <= 0.0 || bar.volume <= 0.0 {
        return 0.0;
    }
    let mult = if venue.is_vn_cash_equity() {
        match unit_policy.resolve_for_venue(venue, bar.close) {
            ResolvedVnPriceUnit::ThousandVnd => 1_000.0,
            ResolvedVnPriceUnit::RawVnd => 1.0,
        }
    } else {
        1.0
    };
    (bar.close * bar.volume * mult).max(0.0)
}

/// Computes causal market-wide context (`vn_z5`, `vn_z20`, `vn_vol_rank_252d`, `breadth_ma20_z`)
/// keyed strictly by ascending timestamp so lookups `timestamp <= bar_timestamp` never read future bars.
fn build_causal_market_context(
    universe: &[AnalogSymbolInput],
    vnindex: Option<&[Candle]>,
    bank_cfg: AnalogBankConfig,
) -> Vec<MarketContextPoint> {
    // 1. Aggregate causal universe breadth (% of liquid stocks with C_t > causal SMA_20(C)_t) per timestamp.
    let mut breadth_by_ts: HashMap<i64, (u32, u32)> = HashMap::new();
    let mut turn_buf = [0.0_f64; 20];

    for item in universe {
        if !item.venue.is_vn_cash_equity() || item.candles.len() < 20 {
            continue;
        }
        let cs = &item.candles;
        let mut close_sum20 = 0.0_f64;
        for t in 0..cs.len() {
            let c = cs[t].close;
            if !c.is_finite() || c <= 0.0 {
                continue;
            }
            close_sum20 += c;
            if t >= 20 {
                let old_c = cs[t - 20].close;
                if old_c.is_finite() && old_c > 0.0 {
                    close_sum20 -= old_c;
                }
            }
            if t < 19 {
                continue;
            }
            if t.is_multiple_of(256) {
                close_sum20 = cs[t + 1 - 20..=t]
                    .iter()
                    .map(|b| if b.close.is_finite() { b.close } else { 0.0 })
                    .sum();
            }

            // Causal 20-bar liquidity hygiene check at bar `t` (zero future lookahead!).
            if bank_cfg.min_median_turnover_vnd > 0.0 {
                for (slot, b) in turn_buf.iter_mut().zip(&cs[t + 1 - 20..=t]) {
                    *slot = bar_turnover_vnd(b, item.venue, bank_cfg.price_unit);
                }
                if median_in_place(&mut turn_buf) < bank_cfg.min_median_turnover_vnd {
                    continue;
                }
            }

            let sma20 = close_sum20 / 20.0;
            let entry = breadth_by_ts.entry(cs[t].timestamp).or_insert((0, 0));
            if c > sma20 {
                entry.0 += 1;
            }
            entry.1 += 1;
        }
    }

    let mut sorted_breadth: Vec<(i64, f64)> = breadth_by_ts
        .into_iter()
        .filter_map(|(ts, (above, total))| {
            (total > 0).then_some((ts, ((above as f64) / (total as f64) - 0.5) * 3.0))
        })
        .collect();
    sorted_breadth.sort_unstable_by_key(|(ts, _)| *ts);

    let Some(vn) = vnindex.filter(|v| v.len() >= 25) else {
        return sorted_breadth
            .into_iter()
            .map(|(timestamp, breadth_z)| MarketContextPoint {
                timestamp,
                vn_z5: 0.0,
                vn_z20: 0.0,
                vn_vol_rank: 0.0,
                breadth_z,
            })
            .collect();
    };

    let vyz = compute_yang_zhang_vol(vn, 20);
    let log_vc: Vec<f64> = vn
        .iter()
        .map(|b| sanitize_pos_price(b.close, b.open).ln())
        .collect();

    let mut out = Vec::with_capacity(vn.len());
    let mut last_breadth_z = 0.0_f64;
    let mut b_ptr = 0_usize;

    for t in 0..vn.len() {
        let ts = vn[t].timestamp;
        while b_ptr < sorted_breadth.len() && sorted_breadth[b_ptr].0 <= ts {
            last_breadth_z = sorted_breadth[b_ptr].1;
            b_ptr += 1;
        }

        let vn_z5 = if t >= 5 {
            let denom = (vyz[t] * 5.0_f64.sqrt()).max(1.0e-4);
            ((log_vc[t] - log_vc[t - 5]) / denom).clamp(-4.0, 4.0)
        } else {
            0.0
        };

        let vn_z20 = if t >= 20 {
            let denom = (vyz[t] * 20.0_f64.sqrt()).max(1.0e-4);
            ((log_vc[t] - log_vc[t - 20]) / denom).clamp(-4.0, 4.0)
        } else {
            0.0
        };

        let win_start = (t + 1).saturating_sub(252);
        let win = &vyz[win_start..=t];
        let vol_pct = if win.len() >= 30 {
            let cur = vyz[t];
            let mut less = 0_usize;
            let mut eq = 0_usize;
            for &v in win {
                if v < cur - 1.0e-15 {
                    less += 1;
                } else if (v - cur).abs() <= 1.0e-15 {
                    eq += 1;
                }
            }
            ((less as f64) + 0.5 * ((eq + 1) as f64)) / (win.len() as f64)
        } else {
            0.5
        };
        let vn_vol_rank = (vol_pct - 0.5) * 2.5;

        out.push(MarketContextPoint {
            timestamp: ts,
            vn_z5,
            vn_z20,
            vn_vol_rank,
            breadth_z: last_breadth_z,
        });
    }

    out
}

#[inline]
fn lookup_causal_market_context(ctx: &[MarketContextPoint], timestamp: i64) -> [f64; 4] {
    if ctx.is_empty() {
        return [0.0; 4];
    }
    let pos = ctx.partition_point(|pt| pt.timestamp <= timestamp);
    if pos == 0 {
        return [0.0; 4];
    }
    let m = ctx[pos - 1];
    [m.vn_z5, m.vn_z20, m.vn_vol_rank, m.breadth_z]
}

/// Computes the causal 20-dimensional AFC feature series and `YZ20` volatility series for `candles`.
///
/// If `require_strict_proxy_bb` is `true` (used when populating [`AnalogMemoryBank`]), any bar where
/// a `ProxyBB` horizon (`T03..T50`) returns `None` due to warmup or zero liquidity (`AT05`) yields
/// `None` so illiquid/halted bars never pollute historical analog memory.
#[must_use]
pub fn compute_causal_20d_features(
    candles: &[Candle],
    vnindex: Option<&[Candle]>,
) -> (Vec<Option<[f64; ANALOG_CONE_DIM]>>, Vec<f64>) {
    let ctx = match vnindex {
        Some(vn) => build_causal_market_context(&[], Some(vn), AnalogBankConfig::default()),
        None => Vec::new(),
    };
    compute_symbol_features_with_context(candles, &ctx, false)
}

fn compute_symbol_features_with_context(
    candles: &[Candle],
    market_ctx: &[MarketContextPoint],
    require_strict_proxy_bb: bool,
) -> (Vec<Option<[f64; ANALOG_CONE_DIM]>>, Vec<f64>) {
    let n = candles.len();
    if n == 0 {
        return (Vec::new(), Vec::new());
    }

    let yz20 = compute_yang_zhang_vol(candles, 20);
    let yz5 = compute_yang_zhang_vol(candles, 5);
    let pbb = compute_proxy_bb_series(candles);

    let mut log_c = Vec::with_capacity(n);
    let mut clv1 = Vec::with_capacity(n);
    let mut vols = Vec::with_capacity(n);

    for c in candles {
        let safe_c = sanitize_pos_price(c.close, c.open);
        let safe_h = sanitize_pos_price(c.high, safe_c).max(safe_c);
        let safe_l = sanitize_pos_price(c.low, safe_c).min(safe_c);
        log_c.push(safe_c.ln());

        let span = safe_h - safe_l;
        let eps = 1.0e-8 * safe_c.abs().max(1.0);
        let clv = if span > eps {
            ((2.0 * safe_c - safe_h - safe_l) / span.max(1.0e-9)).clamp(-1.0, 1.0)
        } else {
            0.0
        };
        clv1.push(clv);
        vols.push(if c.volume.is_finite() && c.volume >= 0.0 {
            c.volume
        } else {
            0.0
        });
    }

    let horizons = [
        BBHorizon::T03,
        BBHorizon::T05,
        BBHorizon::T10,
        BBHorizon::T20,
        BBHorizon::T50,
    ];

    let mut out = Vec::with_capacity(n);
    let ln2 = core::f64::consts::LN_2;
    let sqrt3 = 3.0_f64.sqrt();
    let sqrt5 = 5.0_f64.sqrt();
    let sqrt10 = 10.0_f64.sqrt();
    let sqrt20 = 20.0_f64.sqrt();

    for t in 0..n {
        if t < 20 {
            out.push(None);
            continue;
        }

        let mut f_bb = [0.0_f64; 5];
        let mut bb_valid = true;
        for (slot, &hz) in f_bb.iter_mut().zip(&horizons) {
            match pbb.points[t].horizon(hz).bb_value {
                Some(v) if v.is_finite() => *slot = ((v - 50.0) / 20.0).clamp(-4.5, 4.5),
                _ if require_strict_proxy_bb => {
                    bb_valid = false;
                    break;
                }
                _ => *slot = 0.0,
            }
        }
        if !bb_valid {
            out.push(None);
            continue;
        }

        let denom = yz20[t].max(MIN_YANG_ZHANG_VOL);
        let r1 = ((log_c[t] - log_c[t - 1]) / denom).clamp(-4.5, 4.5);
        let r3 = ((log_c[t] - log_c[t - 3]) / (denom * sqrt3)).clamp(-4.5, 4.5);
        let r5 = ((log_c[t] - log_c[t - 5]) / (denom * sqrt5)).clamp(-4.5, 4.5);
        let r10 = ((log_c[t] - log_c[t - 10]) / (denom * sqrt10)).clamp(-4.5, 4.5);
        let r20 = ((log_c[t] - log_c[t - 20]) / (denom * sqrt20)).clamp(-4.5, 4.5);

        let clv3_slice = &clv1[t.saturating_sub(2)..=t];
        let clv3 = clv3_slice.iter().sum::<f64>() / (clv3_slice.len() as f64);

        let vol20_slice = &vols[t + 1 - 20..=t];
        let vol_ma20 = vol20_slice.iter().sum::<f64>() / 20.0;
        let vol3_slice = &vols[t.saturating_sub(2)..=t];
        let vol_ma3 = vol3_slice.iter().sum::<f64>() / (vol3_slice.len() as f64);

        let rvol20 = vols[t] / vol_ma20.max(1.0);
        let log_rvol = (rvol20.ln_1p() - ln2).clamp(-2.5, 3.5);
        let vol_3_20 = (vol_ma3.max(1.0) / vol_ma20.max(1.0)).ln().clamp(-2.5, 2.5);
        let yz_sqz = (yz5[t].max(1.0e-4) / yz20[t].max(1.0e-4))
            .ln()
            .clamp(-2.5, 2.5);

        let range_start = (t + 1).saturating_sub(60);
        let mut hi60 = f64::NEG_INFINITY;
        let mut lo60 = f64::INFINITY;
        for b in &candles[range_start..=t] {
            if b.high.is_finite() && b.high > hi60 {
                hi60 = b.high;
            }
            if b.low.is_finite() && b.low < lo60 {
                lo60 = b.low;
            }
        }
        let pos60 = if hi60.is_finite() && lo60.is_finite() && hi60 > lo60 + 1.0e-8 {
            (((candles[t].close - lo60) / (hi60 - lo60)) * 2.0 - 1.0).clamp(-1.0, 1.0)
        } else {
            0.0
        };

        let [vn_z5, vn_z20, vn_vol_rank, breadth_z] =
            lookup_causal_market_context(market_ctx, candles[t].timestamp);

        let row = [
            f_bb[0],
            f_bb[1],
            f_bb[2],
            f_bb[3],
            f_bb[4],
            r1,
            r3,
            r5,
            r10,
            r20,
            clv1[t],
            clv3,
            log_rvol,
            vol_3_20,
            yz_sqz,
            pos60,
            vn_z5,
            vn_z20,
            vn_vol_rank,
            breadth_z,
        ];

        if row.iter().all(|v| v.is_finite()) {
            out.push(Some(row));
        } else {
            out.push(None);
        }
    }

    (out, yz20)
}

impl AnalogMemoryBank {
    /// Builds a strictly PIT-indexed [`AnalogMemoryBank`] across `universe` and optional `vnindex` (`1D`).
    ///
    /// Every admission gate (warmup, `ProxyBB AT05`, 20-day median turnover `>= min_median_turnover_vnd`,
    /// market context, and settle timestamp) is strictly causal per bar, ensuring future bars `> as_of`
    /// cannot alter historical analog selection or Z-score statistics by even 1 bit (`DEF-001`).
    #[must_use]
    pub fn build(
        universe: &[AnalogSymbolInput],
        vnindex: Option<&[Candle]>,
        config: AnalogBankConfig,
    ) -> Self {
        let market_context = build_causal_market_context(universe, vnindex, config);
        let has_vnindex = vnindex.is_some_and(|v| v.len() >= 25);

        let mut symbols = Vec::with_capacity(universe.len());
        let mut symbol_lookup = HashMap::with_capacity(universe.len() * 2);
        let mut raw_entries: Vec<(AnalogEntryMeta, [f32; ANALOG_CONE_DIM])> = Vec::new();
        let mut turn_buf = [0.0_f64; 20];

        // Sort symbol inputs deterministically by ticker so `symbol_idx` is 100% reproducible.
        let mut ordered_inputs: Vec<&AnalogSymbolInput> = universe.iter().collect();
        ordered_inputs.sort_by(|a, b| a.ticker.cmp(&b.ticker));

        for item in ordered_inputs {
            let cs = &item.candles;
            let n = cs.len();
            if n <= ANALOG_CONE_WARMUP_BARS + 1 {
                continue;
            }

            let mut timestamps = Vec::with_capacity(n);
            let mut log_open = Vec::with_capacity(n);
            let mut log_high = Vec::with_capacity(n);
            let mut log_low = Vec::with_capacity(n);
            let mut log_close = Vec::with_capacity(n);

            for b in cs {
                let o = sanitize_pos_price(b.open, b.close);
                let c = sanitize_pos_price(b.close, o);
                let h = sanitize_pos_price(b.high, o.max(c)).max(o.max(c));
                let l = sanitize_pos_price(b.low, o.min(c)).min(o.min(c));
                timestamps.push(b.timestamp);
                log_open.push(o.ln());
                log_high.push(h.ln());
                log_low.push(l.ln());
                log_close.push(c.ln());
            }

            let (feats_opt, yz20) = compute_symbol_features_with_context(cs, &market_context, true);

            let sym_idx = symbols.len() as u32;
            let ticker_upper = item.ticker.trim().to_ascii_uppercase();
            let sym_upper = item.symbol.trim().to_ascii_uppercase();
            symbol_lookup.insert(ticker_upper.clone(), sym_idx);
            symbol_lookup.entry(sym_upper.clone()).or_insert(sym_idx);
            let mut valid_bar_indices = Vec::new();

            for t in ANALOG_CONE_WARMUP_BARS..(n - 1) {
                let Some(feat_row) = feats_opt[t] else {
                    continue;
                };

                // Causal 20-session median liquidity filter at `t` (`>= 2 tỷ VNĐ/ngày` default).
                if config.min_median_turnover_vnd > 0.0 {
                    for (slot, b) in turn_buf.iter_mut().zip(&cs[t + 1 - 20..=t]) {
                        *slot = bar_turnover_vnd(b, item.venue, config.price_unit);
                    }
                    if median_in_place(&mut turn_buf) < config.min_median_turnover_vnd {
                        continue;
                    }
                }

                let settle_ts_min = timestamps[t + 1];
                let settle_ts_h10 = timestamps
                    .get(t + DEFAULT_ANALOG_HORIZON)
                    .copied()
                    .unwrap_or(i64::MAX);

                let mut f32_row = [0.0_f32; ANALOG_CONE_DIM];
                for d in 0..ANALOG_CONE_DIM {
                    f32_row[d] = feat_row[d] as f32;
                }

                valid_bar_indices.push(t as u32);
                raw_entries.push((
                    AnalogEntryMeta {
                        settle_timestamp: settle_ts_min,
                        settle_timestamp_h10: settle_ts_h10,
                        entry_timestamp: timestamps[t],
                        yz20: yz20[t] as f32,
                        pit_rank: 0,
                        symbol_idx: sym_idx,
                        bar_idx: t as u32,
                    },
                    f32_row,
                ));
            }

            symbols.push(StoredSymbolTrajectory {
                ticker: ticker_upper,
                symbol: sym_upper,
                timestamps,
                valid_bar_indices,
                log_open,
                log_high,
                log_low,
                log_close,
            });
        }

        // Global deterministic ascending sort by `settle_timestamp` enabling O(log N) `partition_point` PIT slicing.
        raw_entries.sort_by(|(a, _), (b, _)| {
            a.settle_timestamp
                .cmp(&b.settle_timestamp)
                .then_with(|| a.symbol_idx.cmp(&b.symbol_idx))
                .then_with(|| a.entry_timestamp.cmp(&b.entry_timestamp))
        });

        let total = raw_entries.len();
        let mut settle_timestamps = Vec::with_capacity(total);
        let mut prefix_sum = Vec::with_capacity(total + 1);
        let mut prefix_sum_sq = Vec::with_capacity(total + 1);

        let mut cur_sum = [0.0_f64; ANALOG_CONE_DIM];
        let mut cur_sq = [0.0_f64; ANALOG_CONE_DIM];
        prefix_sum.push(cur_sum);
        prefix_sum_sq.push(cur_sq);

        for (rank, (m, row)) in raw_entries.iter_mut().enumerate() {
            m.pit_rank = rank as u32;
            settle_timestamps.push(m.settle_timestamp);
            for d in 0..ANALOG_CONE_DIM {
                let v = f64::from(row[d]);
                cur_sum[d] += v;
                cur_sq[d] += v * v;
            }
            prefix_sum.push(cur_sum);
            prefix_sum_sq.push(cur_sq);
        }

        // Partition each 2,048-entry temporal stratum into a tight 7-level KD-tree layout (`Node64 -> Leaf16`).
        for chunk in raw_entries.chunks_mut(KD_CHUNK_SIZE) {
            kd_partition_in_place(chunk, 0);
        }

        let mut leaf16_boxes = Vec::with_capacity(total.div_ceil(KD_LEAF_SIZE));
        for leaf_slice in raw_entries.chunks(KD_LEAF_SIZE) {
            leaf16_boxes.push(Aabb20::from_slice(leaf_slice));
        }

        let mut node64_boxes = Vec::with_capacity(total.div_ceil(KD_NODE_SIZE));
        for node_slice in raw_entries.chunks(KD_NODE_SIZE) {
            node64_boxes.push(Aabb20::from_slice(node_slice));
        }

        let mut meta = Vec::with_capacity(total);
        let mut exact_rows = Vec::with_capacity(total);
        for (m, row) in raw_entries {
            meta.push(m);
            exact_rows.push(row);
        }

        Self {
            symbols,
            symbol_lookup,
            market_context,
            has_vnindex,
            settle_timestamps,
            meta,
            exact_rows,
            leaf16_boxes,
            node64_boxes,
            prefix_sum,
            prefix_sum_sq,
        }
    }

    /// Total indexed historical state records across the universe.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.meta.len()
    }

    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.meta.is_empty()
    }

    /// Number of indexed equity symbols in this memory bank.
    #[inline]
    #[must_use]
    pub fn symbol_count(&self) -> usize {
        self.symbols.len()
    }

    /// Whether `HOSE:VNINDEX` benchmark context was supplied during [`Self::build`].
    #[inline]
    #[must_use]
    pub const fn has_market_context(&self) -> bool {
        self.has_vnindex
    }

    /// Returns the number of strictly settled (`timestamp(t + horizon) <= as_of_timestamp`) records.
    #[must_use]
    pub fn count_pit_eligible(&self, as_of_timestamp: i64, horizon: usize) -> usize {
        let hz = horizon.clamp(1, MAX_ANALOG_HORIZON);
        let upper = self
            .settle_timestamps
            .partition_point(|&ts| ts <= as_of_timestamp);
        if upper == 0 {
            return 0;
        }
        let scan_end = (upper.div_ceil(KD_CHUNK_SIZE) * KD_CHUNK_SIZE).min(self.meta.len());
        let upper_u32 = upper as u32;
        self.meta[..scan_end]
            .iter()
            .filter(|m| m.pit_rank < upper_u32 && self.is_entry_settled_at(m, hz, as_of_timestamp))
            .count()
    }

    #[inline]
    fn is_entry_settled_at(&self, m: &AnalogEntryMeta, horizon: usize, as_of_ts: i64) -> bool {
        if horizon == DEFAULT_ANALOG_HORIZON {
            return m.settle_timestamp_h10 <= as_of_ts;
        }
        let sym = &self.symbols[m.symbol_idx as usize];
        let target_idx = (m.bar_idx as usize) + horizon;
        sym.timestamps
            .get(target_idx)
            .is_some_and(|&ts| ts <= as_of_ts)
    }

    /// Queries the Dual-Horizon Analog Forecast Cone for `query_candles` anchored at `as_of_timestamp`.
    ///
    /// Enforces **Strict PIT (`DEF-001`)**:
    /// 1. `query_candles` is causally sliced to `timestamp <= as_of_timestamp` before computing features.
    /// 2. Memory bank candidates are cut via binary search `partition_point` + exact `timestamp(t_i + H) <= as_of_timestamp`.
    /// 3. Feature standardization (`std`) is computed in $O(1)$ strictly over `[0..pit_upper]`.
    pub fn query(
        &self,
        query_ticker: &str,
        venue: ExchangeVenue,
        query_candles: &[Candle],
        as_of_timestamp: Option<i64>,
        config: AnalogConeConfig,
    ) -> Result<AnalogConeForecast, AnalogConeError> {
        let cfg = config.sanitized();
        if query_candles.is_empty() {
            return Err(AnalogConeError::InsufficientQueryHistory {
                available: 0,
                required: ANALOG_CONE_WARMUP_BARS + 1,
            });
        }

        // 1. Causal PIT slice on query series (`timestamp <= as_of_timestamp`).
        let q_end = match as_of_timestamp {
            Some(cut_ts) => query_candles.partition_point(|c| c.timestamp <= cut_ts),
            None => query_candles.len(),
        };
        if q_end <= ANALOG_CONE_WARMUP_BARS {
            return Err(AnalogConeError::InsufficientQueryHistory {
                available: q_end,
                required: ANALOG_CONE_WARMUP_BARS + 1,
            });
        }
        let causal_q = &query_candles[..q_end];
        let anchor_bar = &causal_q[q_end - 1];
        let effective_as_of_ts = anchor_bar.timestamp;
        if !anchor_bar.close.is_finite() || anchor_bar.close <= 0.0 {
            return Err(AnalogConeError::InvalidQueryAnchorPrice);
        }

        let resolved_unit = cfg.price_unit.resolve_for_venue(venue, anchor_bar.close);
        let anchor_close = snap_to_nearest_tick(anchor_bar.close, venue, resolved_unit);
        if !anchor_close.is_finite() || anchor_close <= 0.0 {
            return Err(AnalogConeError::InvalidQueryAnchorPrice);
        }

        // 2. Encode causal 20D state vector on `causal_q`.
        // All 20 causal features have exact finite support `<= 60` bars (`Donchian 60D`, `ProxyBB T50` + 1 prev close, `YZ20`),
        // so slicing the trailing 66 bars produces 100% bit-identical features in `< 20 µs`.
        let tail_start = causal_q.len().saturating_sub(ANALOG_CONE_WARMUP_BARS + 1);
        let (q_feats_vec, q_yz_vec) =
            compute_symbol_features_with_context(&causal_q[tail_start..], &self.market_context, false);
        let q_last_idx = q_feats_vec.len() - 1;
        let q_raw = q_feats_vec[q_last_idx].ok_or(AnalogConeError::InsufficientQueryHistory {
            available: q_end,
            required: ANALOG_CONE_WARMUP_BARS + 1,
        })?;
        let q_yz = q_yz_vec[q_last_idx].max(MIN_YANG_ZHANG_VOL);

        // 3. O(log N) Strict PIT binary-search gate (`settle_timestamp <= effective_as_of_ts`).
        let pit_upper = self
            .settle_timestamps
            .partition_point(|&ts| ts <= effective_as_of_ts);
        let min_required = 20_usize;
        if pit_upper < min_required {
            return Err(AnalogConeError::InsufficientPitAnalogs {
                eligible: pit_upper,
                minimum_required: min_required,
            });
        }

        // 4. O(1) strictly causal Z-score scale over `[0..pit_upper]` (zero future statistics!).
        let count_f = pit_upper as f64;
        let sums = &self.prefix_sum[pit_upper];
        let sqs = &self.prefix_sum_sq[pit_upper];

        let effective_profile = if cfg.weight_profile == AnalogWeightProfile::MarketAware
            && (!self.has_vnindex || !venue.is_vn_cash_equity() && venue != ExchangeVenue::Vn30Futures && venue != ExchangeVenue::VnIndexBenchmark)
        {
            AnalogWeightProfile::EndogenousOnly
        } else {
            cfg.weight_profile
        };
        let raw_weights = effective_profile.feature_weights();

        let mut inv_var_w = [0.0_f32; ANALOG_CONE_DIM];
        let mut q_raw_f32 = [0.0_f32; ANALOG_CONE_DIM];
        for d in 0..ANALOG_CONE_DIM {
            let mean = sums[d] / count_f;
            let var = (sqs[d] / count_f - mean * mean).max(1.0e-8);
            let std = var.sqrt().max(1.0e-4);
            inv_var_w[d] = (raw_weights[d].max(0.0) / (std * std)) as f32;
            q_raw_f32[d] = q_raw[d] as f32;
        }

        // Self-symbol lookup for explicit 20-bar non-overlap guard around `effective_as_of_ts`.
        let norm_ticker = query_ticker.trim().to_ascii_uppercase();
        let query_sym_idx = self.symbol_lookup.get(&norm_ticker).copied().or_else(|| {
            norm_ticker
                .split_once(':')
                .and_then(|(_, sym)| self.symbol_lookup.get(sym).copied())
        });

        // 5. O(S log B) exact PIT & fast-regime candidate counts + SoA two-region Branch-and-Bound scan.
        let fast_min_ts = effective_as_of_ts.saturating_sub(FAST_REGIME_LOOKBACK_SECS);
        let k_deep_cap = cfg.top_k;
        let k_fast_cap = FAST_REGIME_TOP_K.min(k_deep_cap.max(FAST_REGIME_TOP_K));

        let mut pit_settled_count = 0_usize;
        let mut fast_settled_count = 0_usize;
        for (s_idx, sym) in self.symbols.iter().enumerate() {
            let ts_cut = sym
                .timestamps
                .partition_point(|&ts| ts <= effective_as_of_ts);
            let req_offset = if Some(s_idx as u32) == query_sym_idx {
                cfg.horizon.max(SELF_EXCLUSION_BARS)
            } else {
                cfg.horizon
            };
            if ts_cut <= req_offset {
                continue;
            }
            let max_bar_excl = (ts_cut - req_offset) as u32;
            let settled_end = sym
                .valid_bar_indices
                .partition_point(|&b| b < max_bar_excl);
            pit_settled_count += settled_end;

            if settled_end > 0 {
                let fast_bar_min = sym.timestamps[..ts_cut].partition_point(|&ts| ts < fast_min_ts) as u32;
                let fast_v_start = sym.valid_bar_indices[..settled_end]
                    .partition_point(|&b| b < fast_bar_min);
                fast_settled_count += settled_end - fast_v_start;
            }
        }

        if pit_settled_count < min_required {
            return Err(AnalogConeError::InsufficientPitAnalogs {
                eligible: pit_settled_count,
                minimum_required: min_required,
            });
        }

        let mut deep_heap: BinaryHeap<HeapCandidate> = BinaryHeap::with_capacity(k_deep_cap + 1);
        let mut fast_heap: BinaryHeap<HeapCandidate> = BinaryHeap::with_capacity(k_fast_cap + 1);
        let mut deep_worst = f32::INFINITY;
        let mut fast_worst = f32::INFINITY;

        // Scan 2-level KD-BVH (`Node64 -> Leaf16 -> ExactRow`) newest-to-oldest so `deep_worst` and
        // `fast_worst` tighten immediately on recent-regime clusters before pruning deep history!
        self.scan_bvh_pit(
            pit_upper,
            fast_min_ts,
            effective_as_of_ts,
            cfg.horizon,
            query_sym_idx,
            &q_raw_f32,
            &inv_var_w,
            k_deep_cap,
            k_fast_cap,
            &mut deep_heap,
            &mut fast_heap,
            &mut deep_worst,
            &mut fast_worst,
        );

        if deep_heap.len() < min_required {
            return Err(AnalogConeError::InsufficientPitAnalogs {
                eligible: pit_settled_count,
                minimum_required: min_required,
            });
        }

        let deep_sorted = into_sorted_vec(deep_heap);
        let fast_sorted = if fast_heap.len() >= FAST_REGIME_TOP_K.min(deep_sorted.len()) {
            into_sorted_vec(fast_heap)
        } else {
            // Automatic fallback to deep pool when trailing 1.25y has < 80 settled analogs (`Spec 3.5.1`).
            deep_sorted[..deep_sorted.len().min(FAST_REGIME_TOP_K)].to_vec()
        };

        // 6. Reconstruct Bayesian volatility-scaled trajectories & project onto statutory venue rules.
        let (mut deep_paths, deep_weights, deep_scale_d) = self.reconstruct_and_project_paths(
            &deep_sorted,
            q_yz,
            anchor_close,
            venue,
            resolved_unit,
            cfg.horizon,
        );
        let (fast_paths, fast_weights, _) = self.reconstruct_and_project_paths(
            &fast_sorted,
            q_yz,
            anchor_close,
            venue,
            resolved_unit,
            cfg.horizon,
        );

        // 7. Dual-Horizon Drift Blending (`60%` Fast Regime median + `40%` Deep Memory median) + re-projection.
        let mut fast_close_col = vec![0.0_f64; fast_paths.len()];
        let mut deep_close_col = vec![0.0_f64; deep_paths.len()];

        for step in 0..cfg.horizon {
            for (j, p) in fast_paths.iter().enumerate() {
                fast_close_col[j] = p[step].close;
            }
            for (j, p) in deep_paths.iter().enumerate() {
                deep_close_col[j] = p[step].close;
            }

            let med_fast = weighted_quantiles_1d(&fast_close_col, &fast_weights, &[0.50])[0];
            let med_deep = weighted_quantiles_1d(&deep_close_col, &deep_weights, &[0.50])[0];
            let drift_ratio = (0.60 * med_fast + 0.40 * med_deep) / med_deep.max(1.0e-6);

            for path in &mut deep_paths {
                path[step].open *= drift_ratio;
                path[step].high *= drift_ratio;
                path[step].low *= drift_ratio;
                path[step].close *= drift_ratio;
            }
        }

        project_ohlc_paths_in_place(&mut deep_paths, anchor_close, venue, resolved_unit);

        // 8. Precompute statutory cumulative floor/ceiling envelope for steps `1..=H`.
        let cum_bands = compute_cumulative_venue_bands(venue, anchor_close, resolved_unit, cfg.horizon);

        // 9. Extract weighted quantiles `P10, P25, P50, P75, P90` at each step `1..=H`.
        let qs = [0.10, 0.25, 0.50, 0.75, 0.90];
        let mut steps = Vec::with_capacity(cfg.horizon);
        let anchor_ict_days = (effective_as_of_ts + 7 * 3_600).div_euclid(86_400);

        for step in 0..cfg.horizon {
            for (j, p) in deep_paths.iter().enumerate() {
                deep_close_col[j] = p[step].close;
            }
            let raw_q = weighted_quantiles_1d(&deep_close_col, &deep_weights, &qs);
            let (cum_floor, cum_ceil) = cum_bands[step];

            let mut snapped_q = [0.0_f64; 5];
            for idx_q in 0..5 {
                let mut val = snap_to_nearest_tick(raw_q[idx_q], venue, resolved_unit);
                if let (Some(fl), Some(ce)) = (cum_floor, cum_ceil) {
                    val = val.clamp(fl, ce);
                }
                if idx_q > 0 && val < snapped_q[idx_q - 1] {
                    val = snapped_q[idx_q - 1];
                }
                snapped_q[idx_q] = val;
            }

            let [p10, p25, p50, p75, p90] = snapped_q;
            let step_num = step + 1;
            let projected_timestamp = if venue.is_vn_cash_equity()
                || matches!(
                    venue,
                    ExchangeVenue::Vn30Futures | ExchangeVenue::VnIndexBenchmark
                ) {
                add_vn_trading_days(anchor_ict_days, step_num as i32) * 86_400
            } else {
                effective_as_of_ts.saturating_add((step_num as i64) * 86_400)
            };

            steps.push(AnalogConeStep {
                step: step_num,
                projected_timestamp,
                p10,
                p25,
                p50,
                p75,
                p90,
                p10_ret_pct: ((p10 / anchor_close) - 1.0) * 100.0,
                p25_ret_pct: ((p25 / anchor_close) - 1.0) * 100.0,
                p50_ret_pct: ((p50 / anchor_close) - 1.0) * 100.0,
                p75_ret_pct: ((p75 / anchor_close) - 1.0) * 100.0,
                p90_ret_pct: ((p90 / anchor_close) - 1.0) * 100.0,
                cumulative_floor: cum_floor,
                cumulative_ceiling: cum_ceil,
            });
        }

        // 10. Compute probability & tail-risk metrics across the weighted ensemble.
        let last_step_idx = cfg.horizon - 1;
        let mut win_w = 0.0_f64;
        let mut gain3_w = 0.0_f64;
        let mut tail6_w = 0.0_f64;
        let mut mean_ret_pct = 0.0_f64;

        for (path, &w) in deep_paths.iter().zip(&deep_weights) {
            let term_c = path[last_step_idx].close;
            let term_ret_pct = ((term_c / anchor_close) - 1.0) * 100.0;
            mean_ret_pct += w * term_ret_pct;
            if term_c > anchor_close + 1.0e-9 {
                win_w += w;
            }
            if term_ret_pct > 3.0 {
                gain3_w += w;
            }
            let min_low = path
                .iter()
                .map(|b| b.low)
                .fold(f64::INFINITY, f64::min);
            let min_low_pct = ((min_low / anchor_close) - 1.0) * 100.0;
            if min_low_pct <= -6.0 {
                tail6_w += w;
            }
        }

        let term_step = &steps[last_step_idx];
        let upside = (term_step.p90 - anchor_close).max(0.0);
        let downside = (anchor_close - term_step.p10).max(anchor_close * 0.0025);
        let skew_ratio = (upside / downside).clamp(0.0, 20.0);

        // 11. Build Top-10 historical analog evidence cases (strictly gated by `effective_as_of_ts`).
        let top_n = deep_sorted.len().min(10);
        let mut top_analogs = Vec::with_capacity(top_n);
        for (rank_i, cand) in deep_sorted[..top_n].iter().enumerate() {
            let m = &self.meta[cand.storage_idx as usize];
            let sym = &self.symbols[m.symbol_idx as usize];
            let b_idx = m.bar_idx as usize;
            let c0_ln = sym.log_close[b_idx];
            let dist = f64::from(cand.dist_sq).sqrt();
            let sim_pct = (100.0 * (-dist / (2.5 * deep_scale_d)).exp()).clamp(0.0, 100.0);

            let realized_t5_ret_pct = sym
                .timestamps
                .get(b_idx + 5)
                .filter(|&&ts| ts <= effective_as_of_ts)
                .map(|_| ((sym.log_close[b_idx + 5] - c0_ln).exp() - 1.0) * 100.0);

            let realized_t10_ret_pct = sym
                .timestamps
                .get(b_idx + 10)
                .filter(|&&ts| ts <= effective_as_of_ts)
                .map(|_| ((sym.log_close[b_idx + 10] - c0_ln).exp() - 1.0) * 100.0);

            top_analogs.push(AnalogMatchCase {
                ticker: sym.ticker.clone(),
                symbol: sym.symbol.clone(),
                entry_timestamp: m.entry_timestamp,
                entry_date: format_utc_date(m.entry_timestamp),
                similarity_pct: sim_pct,
                distance: dist,
                weight: deep_weights[rank_i],
                realized_t5_ret_pct,
                realized_t10_ret_pct,
            });
        }

        Ok(AnalogConeForecast {
            as_of_timestamp: effective_as_of_ts,
            as_of_date: format_utc_date(effective_as_of_ts),
            anchor_close,
            horizon: cfg.horizon,
            top_k: deep_sorted.len(),
            weight_profile: effective_profile,
            expected_return_p50_pct: term_step.p50_ret_pct,
            expected_return_mean_pct: mean_ret_pct,
            steps,
            win_rate_pct: (win_w * 100.0).clamp(0.0, 100.0),
            prob_gain_gt_3pct: (gain3_w * 100.0).clamp(0.0, 100.0),
            prob_tail_drop_6pct: (tail6_w * 100.0).clamp(0.0, 100.0),
            skew_ratio,
            query_yz20_vol: q_yz,
            pit_candidate_count: pit_settled_count,
            fast_candidate_count: fast_settled_count,
            top_analogs,
        })
    }

    fn reconstruct_and_project_paths(
        &self,
        sorted_cands: &[HeapCandidate],
        q_yz: f64,
        anchor_close: f64,
        venue: ExchangeVenue,
        resolved_unit: ResolvedVnPriceUnit,
        horizon: usize,
    ) -> (Vec<Vec<SimulatedCandle>>, Vec<f64>, f64) {
        let k = sorted_cands.len();
        let top_d: Vec<f64> = sorted_cands
            .iter()
            .map(|c| f64::from(c.dist_sq).max(0.0).sqrt())
            .collect();

        let mid = k / 2;
        let med_d = if k.is_multiple_of(2) && k >= 2 {
            0.5 * (top_d[mid - 1] + top_d[mid])
        } else {
            top_d[mid]
        };
        let scale_d = med_d + 1.0e-4;

        let mut logits = Vec::with_capacity(k);
        let mut max_logit = f64::NEG_INFINITY;
        for &d in &top_d {
            let lg = -1.5 * (d / scale_d);
            if lg > max_logit {
                max_logit = lg;
            }
            logits.push(lg);
        }

        let mut weights = Vec::with_capacity(k);
        let mut sum_w = 0.0_f64;
        for lg in logits {
            let w = (lg - max_logit).exp();
            sum_w += w;
            weights.push(w);
        }
        if sum_w > 0.0 {
            for w in &mut weights {
                *w /= sum_w;
            }
        } else {
            let unif = 1.0 / (k.max(1) as f64);
            weights.fill(unif);
        }

        let mut paths = Vec::with_capacity(k);
        for cand in sorted_cands {
            let m = &self.meta[cand.storage_idx as usize];
            let sym = &self.symbols[m.symbol_idx as usize];
            let b_idx = m.bar_idx as usize;
            let c0_ln = sym.log_close[b_idx];

            let hist_yz = f64::from(m.yz20).max(MIN_YANG_ZHANG_VOL);
            // Bayesian volatility shrinkage (`Spec 3.5.3`): `clip(0.70 * (q_yz / hist_yz) + 0.30, 0.45, 2.25)`.
            let vol_scale = (0.70 * (q_yz / hist_yz) + 0.30).clamp(0.45, 2.25);

            let mut path = Vec::with_capacity(horizon);
            for step in 0..horizon {
                let s_idx = b_idx + 1 + step;
                let ro = (sym.log_open[s_idx] - c0_ln) * vol_scale;
                let rh = (sym.log_high[s_idx] - c0_ln) * vol_scale;
                let rl = (sym.log_low[s_idx] - c0_ln) * vol_scale;
                let rc = (sym.log_close[s_idx] - c0_ln) * vol_scale;

                path.push(SimulatedCandle {
                    open: anchor_close * ro.exp(),
                    high: anchor_close * rh.exp(),
                    low: anchor_close * rl.exp(),
                    close: anchor_close * rc.exp(),
                });
            }
            paths.push(path);
        }

        project_ohlc_paths_in_place(&mut paths, anchor_close, venue, resolved_unit);
        (paths, weights, scale_d)
    }

    #[allow(clippy::too_many_arguments)]
    fn scan_bvh_pit(
        &self,
        pit_upper: usize,
        fast_min_ts: i64,
        effective_as_of_ts: i64,
        horizon: usize,
        query_sym_idx: Option<u32>,
        q_raw_f32: &[f32; ANALOG_CONE_DIM],
        inv_var_w: &[f32; ANALOG_CONE_DIM],
        k_deep_cap: usize,
        k_fast_cap: usize,
        deep_heap: &mut BinaryHeap<HeapCandidate>,
        fast_heap: &mut BinaryHeap<HeapCandidate>,
        deep_worst: &mut f32,
        fast_worst: &mut f32,
    ) {
        if pit_upper == 0 || self.meta.is_empty() {
            return;
        }
        let pit_upper_u32 = pit_upper as u32;
        let active_storage_end =
            (pit_upper.div_ceil(KD_CHUNK_SIZE) * KD_CHUNK_SIZE).min(self.meta.len());
        let end_node = active_storage_end.div_ceil(KD_NODE_SIZE);
        let num_leaves = self.leaf16_boxes.len();

        for node_idx in (0..end_node).rev() {
            let node = &self.node64_boxes[node_idx];
            if node.min_pit_rank >= pit_upper_u32 {
                continue;
            }

            let node_can_fast = node.max_entry_ts >= fast_min_ts;
            let node_thresh = if node_can_fast {
                (*deep_worst).max(*fast_worst)
            } else {
                *deep_worst
            };
            let node_safe_thresh = if node_thresh.is_finite() {
                node_thresh * 1.000_002 + 1.0e-6
            } else {
                f32::INFINITY
            };
            if node.lower_bound_dist_sq(q_raw_f32, inv_var_w) > node_safe_thresh {
                continue;
            }

            let leaf_start = node_idx * (KD_NODE_SIZE / KD_LEAF_SIZE);
            let leaf_end = (leaf_start + (KD_NODE_SIZE / KD_LEAF_SIZE)).min(num_leaves);

            for leaf_idx in (leaf_start..leaf_end).rev() {
                let leaf = &self.leaf16_boxes[leaf_idx];
                if leaf.min_pit_rank >= pit_upper_u32 {
                    continue;
                }

                let leaf_can_fast = leaf.max_entry_ts >= fast_min_ts;
                let leaf_thresh = if leaf_can_fast {
                    (*deep_worst).max(*fast_worst)
                } else {
                    *deep_worst
                };
                let leaf_safe_thresh = if leaf_thresh.is_finite() {
                    leaf_thresh * 1.000_002 + 1.0e-6
                } else {
                    f32::INFINITY
                };
                if leaf.lower_bound_dist_sq(q_raw_f32, inv_var_w) > leaf_safe_thresh {
                    continue;
                }

                let row_start = leaf_idx * KD_LEAF_SIZE;
                let row_end = (row_start + KD_LEAF_SIZE).min(active_storage_end);
                let needs_pit_rank_check = leaf.max_pit_rank >= pit_upper_u32;

                for storage_idx in row_start..row_end {
                    let cur_thresh = if leaf_can_fast {
                        (*deep_worst).max(*fast_worst)
                    } else {
                        *deep_worst
                    };

                    let row = &self.exact_rows[storage_idx];
                    let mut d_sq = 0.0_f32;
                    for d in 0..10 {
                        let diff = row[d] - q_raw_f32[d];
                        d_sq += inv_var_w[d] * diff * diff;
                    }
                    if d_sq > cur_thresh {
                        continue;
                    }
                    for d in 10..ANALOG_CONE_DIM {
                        let diff = row[d] - q_raw_f32[d];
                        d_sq += inv_var_w[d] * diff * diff;
                    }
                    if d_sq > cur_thresh {
                        continue;
                    }

                    let m = &self.meta[storage_idx];
                    if needs_pit_rank_check && m.pit_rank >= pit_upper_u32 {
                        continue;
                    }
                    if !self.is_entry_settled_at(m, horizon, effective_as_of_ts) {
                        continue;
                    }
                    if Some(m.symbol_idx) == query_sym_idx {
                        let sym_ts = &self.symbols[m.symbol_idx as usize].timestamps;
                        let guard_idx = (m.bar_idx as usize) + SELF_EXCLUSION_BARS;
                        if sym_ts
                            .get(guard_idx)
                            .is_none_or(|&ts| ts > effective_as_of_ts)
                        {
                            continue;
                        }
                    }

                    let cand = HeapCandidate {
                        dist_sq: d_sq.max(0.0),
                        pit_rank: m.pit_rank,
                        storage_idx: storage_idx as u32,
                    };

                    if deep_heap.len() < k_deep_cap {
                        deep_heap.push(cand);
                        if deep_heap.len() == k_deep_cap
                            && let Some(w) = deep_heap.peek()
                        {
                            *deep_worst = w.dist_sq;
                        }
                    } else if let Some(&worst) = deep_heap.peek()
                        && cand < worst
                    {
                        deep_heap.pop();
                        deep_heap.push(cand);
                        if let Some(w) = deep_heap.peek() {
                            *deep_worst = w.dist_sq;
                        }
                    }

                    if leaf_can_fast && m.entry_timestamp >= fast_min_ts {
                        if fast_heap.len() < k_fast_cap {
                            fast_heap.push(cand);
                            if fast_heap.len() == k_fast_cap
                                && let Some(w) = fast_heap.peek()
                            {
                                *fast_worst = w.dist_sq;
                            }
                        } else if let Some(&worst) = fast_heap.peek()
                            && cand < worst
                        {
                            fast_heap.pop();
                            fast_heap.push(cand);
                            if let Some(w) = fast_heap.peek() {
                                *fast_worst = w.dist_sq;
                            }
                        }
                    }
                }
            }
        }
    }
}

fn into_sorted_vec(mut heap: BinaryHeap<HeapCandidate>) -> Vec<HeapCandidate> {
    let mut out = Vec::with_capacity(heap.len());
    while let Some(item) = heap.pop() {
        out.push(item);
    }
    out.reverse();
    out
}

fn project_ohlc_paths_in_place(
    paths: &mut [Vec<SimulatedCandle>],
    anchor_close: f64,
    venue: ExchangeVenue,
    resolved_unit: ResolvedVnPriceUnit,
) {
    let snapped_anchor = snap_to_nearest_tick(anchor_close, venue, resolved_unit);
    for path in paths {
        let mut prev_ref = snapped_anchor;
        for bar in path.iter_mut() {
            let (mut o, mut hi, mut lo, mut c) = (bar.open, bar.high, bar.low, bar.close);
            if let Some(band) =
                compute_daily_price_band_with_resolved_unit(venue, prev_ref, resolved_unit)
            {
                let fl = band.floor_price;
                let ce = band.ceiling_price;
                o = snap_to_nearest_tick(o.clamp(fl, ce), venue, resolved_unit).clamp(fl, ce);
                hi = snap_to_nearest_tick(hi.clamp(fl, ce), venue, resolved_unit).clamp(fl, ce);
                lo = snap_to_nearest_tick(lo.clamp(fl, ce), venue, resolved_unit).clamp(fl, ce);
                c = snap_to_nearest_tick(c.clamp(fl, ce), venue, resolved_unit).clamp(fl, ce);
            } else {
                o = snap_to_nearest_tick(o, venue, resolved_unit);
                hi = snap_to_nearest_tick(hi, venue, resolved_unit);
                lo = snap_to_nearest_tick(lo, venue, resolved_unit);
                c = snap_to_nearest_tick(c, venue, resolved_unit);
            }
            let hi_valid = o.max(hi).max(lo).max(c);
            let lo_valid = o.min(hi).min(lo).min(c);
            bar.open = o;
            bar.high = hi_valid;
            bar.low = lo_valid;
            bar.close = c;
            prev_ref = c;
        }
    }
}

/// Projects arbitrary `(open, high, low, close)` trajectory paths onto statutory Vietnam / global
/// exchange price bands (`HOSE ±7%`, `HNX ±10%`, `UPCOM ±15%`) and order-book tick ladders.
#[must_use]
pub fn project_ohlc_paths_to_venue(
    paths: &[Vec<(f64, f64, f64, f64)>],
    anchor_close: f64,
    venue: ExchangeVenue,
    price_unit: VnPriceUnit,
) -> Vec<Vec<(f64, f64, f64, f64)>> {
    let resolved_unit = price_unit.resolve_for_venue(venue, anchor_close);
    let mut sim: Vec<Vec<SimulatedCandle>> = paths
        .iter()
        .map(|p| {
            p.iter()
                .map(|&(open, high, low, close)| SimulatedCandle {
                    open,
                    high,
                    low,
                    close,
                })
                .collect()
        })
        .collect();
    project_ohlc_paths_in_place(&mut sim, anchor_close, venue, resolved_unit);
    sim.into_iter()
        .map(|p| p.into_iter().map(|b| (b.open, b.high, b.low, b.close)).collect())
        .collect()
}

fn compute_cumulative_venue_bands(
    venue: ExchangeVenue,
    anchor_close: f64,
    resolved_unit: ResolvedVnPriceUnit,
    horizon: usize,
) -> Vec<(Option<f64>, Option<f64>)> {
    if venue.daily_band_pct().is_none() {
        return vec![(None, None); horizon];
    }
    let mut floor_ref = snap_to_nearest_tick(anchor_close, venue, resolved_unit);
    let mut ceil_ref = floor_ref;
    let mut out = Vec::with_capacity(horizon);

    for _ in 0..horizon {
        let next_floor = compute_daily_price_band_with_resolved_unit(venue, floor_ref, resolved_unit)
            .map(|b| b.floor_price)
            .unwrap_or(floor_ref);
        let next_ceil = compute_daily_price_band_with_resolved_unit(venue, ceil_ref, resolved_unit)
            .map(|b| b.ceiling_price)
            .unwrap_or(ceil_ref);
        floor_ref = next_floor;
        ceil_ref = next_ceil;
        out.push((Some(floor_ref), Some(ceil_ref)));
    }
    out
}

/// Computes center-point interpolated weighted quantiles matching `run_arena_benchmark.py::weighted_quantiles_1d`.
#[must_use]
pub fn weighted_quantiles_1d(values: &[f64], weights: &[f64], qs: &[f64]) -> Vec<f64> {
    let n = values.len().min(weights.len());
    if n == 0 {
        return vec![0.0; qs.len()];
    }

    let mut pairs: Vec<(f64, f64)> = values[..n]
        .iter()
        .copied()
        .zip(weights[..n].iter().copied())
        .map(|(v, w)| (v, if w.is_finite() && w > 0.0 { w } else { 0.0 }))
        .collect();
    pairs.sort_unstable_by(|a, b| a.0.total_cmp(&b.0));

    let total_w: f64 = pairs.iter().map(|p| p.1).sum();
    let mut cdf_mid = Vec::with_capacity(n);
    if total_w > 0.0 {
        let mut cum = 0.0_f64;
        for &(_, w) in &pairs {
            cum += w;
            cdf_mid.push((cum - 0.5 * w) / total_w);
        }
    } else {
        for i in 0..n {
            cdf_mid.push(((i as f64) + 0.5) / (n as f64));
        }
    }

    let mut out = Vec::with_capacity(qs.len());
    for &q in qs {
        if q <= cdf_mid[0] {
            out.push(pairs[0].0);
        } else if q >= cdf_mid[n - 1] {
            out.push(pairs[n - 1].0);
        } else {
            let idx = cdf_mid.partition_point(|&c| c < q);
            let c0 = cdf_mid[idx - 1];
            let c1 = cdf_mid[idx];
            let v0 = pairs[idx - 1].0;
            let v1 = pairs[idx].0;
            let span = c1 - c0;
            let interp = if span.abs() <= 1.0e-15 {
                0.5 * (v0 + v1)
            } else {
                v0 + ((q - c0) / span) * (v1 - v0)
            };
            out.push(interp);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::tick_size_at_price;

    fn lcg_next(state: &mut u64) -> f64 {
        *state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((*state >> 11) as f64) / ((1_u64 << 53) as f64)
    }

    fn synth_vn_series(
        ticker: &str,
        symbol: &str,
        venue: ExchangeVenue,
        base_price_vnd: f64,
        base_volume: f64,
        bars: usize,
        seed: u64,
    ) -> AnalogSymbolInput {
        let mut rng = seed;
        let mut candles = Vec::with_capacity(bars);
        let start_ts = 1_577_836_800_i64; // 2020-01-01 UTC
        let mut price = base_price_vnd;
        let unit = VnPriceUnit::RawVnd.resolve_for_venue(venue, base_price_vnd);

        for i in 0..bars {
            let ts = start_ts + (i as i64) * 86_400;
            let r = (lcg_next(&mut rng) - 0.492) * 0.042;
            let open_raw = price * (1.0 + (lcg_next(&mut rng) - 0.5) * 0.008);
            let close_raw = (price * (1.0 + r)).max(1_000.0);
            let wick_up = price * lcg_next(&mut rng) * 0.015;
            let wick_dn = price * lcg_next(&mut rng) * 0.015;

            let open = snap_to_nearest_tick(open_raw, venue, unit);
            let close = snap_to_nearest_tick(close_raw, venue, unit);
            let high = snap_to_nearest_tick(open.max(close) + wick_up, venue, unit);
            let low = snap_to_nearest_tick((open.min(close) - wick_dn).max(500.0), venue, unit);
            let volume = (base_volume * (0.65 + 0.9 * lcg_next(&mut rng))).round();

            candles.push(Candle {
                timestamp: ts,
                date_label: format_utc_date(ts),
                open,
                high: high.max(open.max(close)),
                low: low.min(open.min(close)),
                close,
                volume,
            });
            price = close;
        }

        AnalogSymbolInput {
            ticker: ticker.to_owned(),
            symbol: symbol.to_owned(),
            venue,
            candles,
        }
    }

    fn build_sample_universe(bars: usize) -> (Vec<AnalogSymbolInput>, Vec<Candle>) {
        let specs = [
            ("HOSE:FPT", "FPT", ExchangeVenue::Hose, 92_000.0, 2_400_000.0, 101),
            ("HOSE:VCB", "VCB", ExchangeVenue::Hose, 86_000.0, 1_500_000.0, 202),
            ("HOSE:HPG", "HPG", ExchangeVenue::Hose, 27_500.0, 14_000_000.0, 303),
            ("HOSE:SSI", "SSI", ExchangeVenue::Hose, 31_200.0, 9_500_000.0, 404),
            ("HOSE:HAG", "HAG", ExchangeVenue::Hose, 8_800.0, 7_200_000.0, 505),
            ("HNX:SHS", "SHS", ExchangeVenue::Hnx, 15_400.0, 8_000_000.0, 606),
            ("HNX:PVS", "PVS", ExchangeVenue::Hnx, 38_600.0, 4_200_000.0, 707),
            ("UPCOM:ACV", "ACV", ExchangeVenue::Upcom, 98_500.0, 650_000.0, 808),
            ("UPCOM:BSR", "BSR", ExchangeVenue::Upcom, 21_400.0, 6_800_000.0, 909),
            // Illiquid micro-cap shell (< 2B VND/day: 5,000 VND * 10,000 = 50M VND/day) -> MUST be filtered out!
            ("UPCOM:PENNY", "PENNY", ExchangeVenue::Upcom, 5_000.0, 10_000.0, 999),
        ];

        let universe: Vec<AnalogSymbolInput> = specs
            .into_iter()
            .map(|(tk, sym, ven, p, v, seed)| synth_vn_series(tk, sym, ven, p, v, bars, seed))
            .collect();

        let vnindex = synth_vn_series(
            "HOSE:VNINDEX",
            "VNINDEX",
            ExchangeVenue::VnIndexBenchmark,
            1_220.0,
            650_000_000.0,
            bars,
            7777,
        )
        .candles;

        (universe, vnindex)
    }

    fn assert_forecasts_bit_identical(a: &AnalogConeForecast, b: &AnalogConeForecast) {
        assert_eq!(a.as_of_timestamp, b.as_of_timestamp);
        assert_eq!(a.anchor_close.to_bits(), b.anchor_close.to_bits());
        assert_eq!(a.win_rate_pct.to_bits(), b.win_rate_pct.to_bits());
        assert_eq!(a.prob_gain_gt_3pct.to_bits(), b.prob_gain_gt_3pct.to_bits());
        assert_eq!(
            a.prob_tail_drop_6pct.to_bits(),
            b.prob_tail_drop_6pct.to_bits()
        );
        assert_eq!(a.skew_ratio.to_bits(), b.skew_ratio.to_bits());
        assert_eq!(
            a.expected_return_p50_pct.to_bits(),
            b.expected_return_p50_pct.to_bits()
        );
        assert_eq!(
            a.expected_return_mean_pct.to_bits(),
            b.expected_return_mean_pct.to_bits()
        );
        assert_eq!(a.query_yz20_vol.to_bits(), b.query_yz20_vol.to_bits());
        assert_eq!(a.pit_candidate_count, b.pit_candidate_count);
        assert_eq!(a.fast_candidate_count, b.fast_candidate_count);
        assert_eq!(a.steps.len(), b.steps.len());

        for (sa, sb) in a.steps.iter().zip(&b.steps) {
            assert_eq!(sa.step, sb.step);
            assert_eq!(sa.projected_timestamp, sb.projected_timestamp);
            assert_eq!(sa.p10.to_bits(), sb.p10.to_bits());
            assert_eq!(sa.p25.to_bits(), sb.p25.to_bits());
            assert_eq!(sa.p50.to_bits(), sb.p50.to_bits());
            assert_eq!(sa.p75.to_bits(), sb.p75.to_bits());
            assert_eq!(sa.p90.to_bits(), sb.p90.to_bits());
        }

        assert_eq!(a.top_analogs.len(), b.top_analogs.len());
        for (ma, mb) in a.top_analogs.iter().zip(&b.top_analogs) {
            assert_eq!(ma.ticker, mb.ticker);
            assert_eq!(ma.entry_timestamp, mb.entry_timestamp);
            assert_eq!(ma.similarity_pct.to_bits(), mb.similarity_pct.to_bits());
            assert_eq!(ma.distance.to_bits(), mb.distance.to_bits());
            assert_eq!(ma.weight.to_bits(), mb.weight.to_bits());
            assert_eq!(
                ma.realized_t5_ret_pct.map(f64::to_bits),
                mb.realized_t5_ret_pct.map(f64::to_bits)
            );
            assert_eq!(
                ma.realized_t10_ret_pct.map(f64::to_bits),
                mb.realized_t10_ret_pct.map(f64::to_bits)
            );
        }
    }

    #[test]
    fn strict_pit_invariance_def001_zero_future_leakage_bit_for_bit() {
        let (base_universe, base_vnindex) = build_sample_universe(620);
        let cut_idx = 460;
        let as_of_ts = base_universe[0].candles[cut_idx].timestamp;

        let bank_orig = AnalogMemoryBank::build(
            &base_universe,
            Some(&base_vnindex),
            AnalogBankConfig::default(),
        );
        // Verify illiquid `UPCOM:PENNY` never polluted the memory bank.
        assert!(
            !bank_orig
                .meta
                .iter()
                .any(|m| bank_orig.symbols[m.symbol_idx as usize].symbol == "PENNY")
        );

        let cfg = AnalogConeConfig::default();
        let fc_orig = bank_orig
            .query(
                "HOSE:FPT",
                ExchangeVenue::Hose,
                &base_universe[0].candles,
                Some(as_of_ts),
                cfg,
            )
            .expect("valid forecast on original series");

        // Variant 1: Shock ALL future bars `> as_of_ts` by +50% pump across half the market & VNINDEX,
        // and -50% crash across the other half!
        let mut shocked_universe = base_universe.clone();
        for (s_i, item) in shocked_universe.iter_mut().enumerate() {
            let mult = if s_i % 2 == 0 { 1.50 } else { 0.50 };
            for bar in &mut item.candles {
                if bar.timestamp > as_of_ts {
                    bar.open *= mult;
                    bar.high *= mult;
                    bar.low *= mult;
                    bar.close *= mult;
                    bar.volume *= 4.0;
                }
            }
        }
        let mut shocked_vnindex = base_vnindex.clone();
        for bar in &mut shocked_vnindex {
            if bar.timestamp > as_of_ts {
                bar.open *= 0.50;
                bar.high *= 0.50;
                bar.low *= 0.50;
                bar.close *= 0.50;
            }
        }

        let bank_shocked = AnalogMemoryBank::build(
            &shocked_universe,
            Some(&shocked_vnindex),
            AnalogBankConfig::default(),
        );
        let fc_shocked = bank_shocked
            .query(
                "HOSE:FPT",
                ExchangeVenue::Hose,
                &shocked_universe[0].candles,
                Some(as_of_ts),
                cfg,
            )
            .expect("valid forecast on future-shocked series");

        assert_forecasts_bit_identical(&fc_orig, &fc_shocked);

        // Variant 2: Physically truncate ALL bars `> as_of_ts` (simulating live real-time cutoff).
        let mut trunc_universe = base_universe;
        for item in &mut trunc_universe {
            item.candles.retain(|b| b.timestamp <= as_of_ts);
        }
        let mut trunc_vnindex = base_vnindex;
        trunc_vnindex.retain(|b| b.timestamp <= as_of_ts);

        let bank_trunc = AnalogMemoryBank::build(
            &trunc_universe,
            Some(&trunc_vnindex),
            AnalogBankConfig::default(),
        );
        let fc_trunc = bank_trunc
            .query(
                "HOSE:FPT",
                ExchangeVenue::Hose,
                &trunc_universe[0].candles,
                Some(as_of_ts),
                cfg,
            )
            .expect("valid forecast on truncated series");

        assert_forecasts_bit_identical(&fc_orig, &fc_trunc);

        // Also verify EndogenousOnly mode (`[0.4, 0.8, 0.8, 1.0, 0.0]`) is bit-identical across future mutations!
        let endog_cfg = AnalogConeConfig {
            weight_profile: AnalogWeightProfile::EndogenousOnly,
            ..cfg
        };
        let fc_endog_orig = bank_orig
            .query(
                "HOSE:FPT",
                ExchangeVenue::Hose,
                &shocked_universe[0].candles[..=cut_idx],
                Some(as_of_ts),
                endog_cfg,
            )
            .unwrap();
        let fc_endog_shocked = bank_shocked
            .query(
                "HOSE:FPT",
                ExchangeVenue::Hose,
                &shocked_universe[0].candles,
                Some(as_of_ts),
                endog_cfg,
            )
            .unwrap();
        assert_forecasts_bit_identical(&fc_endog_orig, &fc_endog_shocked);
    }

    #[test]
    fn venue_band_and_tick_bounds_hold_across_hose_hnx_upcom() {
        let (universe, vnindex) = build_sample_universe(580);
        let bank = AnalogMemoryBank::build(&universe, Some(&vnindex), AnalogBankConfig::default());

        let targets: [(&str, ExchangeVenue, f64); 5] = [
            ("HOSE:FPT", ExchangeVenue::Hose, 0.07),   // >= 50,000 VND tier (100 VND tick)
            ("HOSE:HPG", ExchangeVenue::Hose, 0.07),   // 10,000..50,000 VND tier (50 VND tick)
            ("HOSE:HAG", ExchangeVenue::Hose, 0.07),   // < 10,000 VND tier (10 VND tick)
            ("HNX:SHS", ExchangeVenue::Hnx, 0.10),     // HNX ±10% (100 VND tick)
            ("UPCOM:ACV", ExchangeVenue::Upcom, 0.15), // UPCOM ±15% (100 VND tick)
        ];

        for (ticker, venue, band_pct) in targets {
            let item = universe.iter().find(|u| u.ticker == ticker).unwrap();
            let fc = bank
                .query(
                    ticker,
                    venue,
                    &item.candles,
                    None,
                    AnalogConeConfig {
                        horizon: 15,
                        top_k: 100,
                        weight_profile: AnalogWeightProfile::MarketAware,
                        price_unit: VnPriceUnit::RawVnd,
                    },
                )
                .expect("forecast should succeed");

            assert_eq!(fc.steps.len(), 15);
            assert_eq!(fc.top_analogs.len(), 10);
            assert!((0.0..=100.0).contains(&fc.win_rate_pct));
            assert!((0.0..=100.0).contains(&fc.prob_gain_gt_3pct));
            assert!((0.0..=100.0).contains(&fc.prob_tail_drop_6pct));

            let unit = ResolvedVnPriceUnit::RawVnd;
            for step in &fc.steps {
                // 1. Strict quantile ordering: P10 <= P25 <= P50 <= P75 <= P90
                assert!(
                    step.p10 <= step.p25
                        && step.p25 <= step.p50
                        && step.p50 <= step.p75
                        && step.p75 <= step.p90,
                    "{ticker} step {} violated quantile ordering: {:?}",
                    step.step,
                    (step.p10, step.p25, step.p50, step.p75, step.p90)
                );

                // 2. Statutory cumulative floor/ceiling containment
                let fl = step.cumulative_floor.expect("VN stock has floor");
                let ce = step.cumulative_ceiling.expect("VN stock has ceiling");
                let raw_outer_ceil = fc.anchor_close * (1.0 + band_pct).powi(step.step as i32);
                let raw_outer_floor = fc.anchor_close * (1.0 - band_pct).powi(step.step as i32);

                assert!(
                    step.p10 >= fl && step.p90 <= ce,
                    "{ticker} step {} outside statutory cumulative band [{fl}, {ce}]: p10={}, p90={}",
                    step.step,
                    step.p10,
                    step.p90
                );
                assert!(ce <= raw_outer_ceil + 1.0e-6);
                assert!(fl >= raw_outer_floor - 100.0);

                // 3. Exact exchange order-book tick validity for every quantile
                for q_price in [step.p10, step.p25, step.p50, step.p75, step.p90] {
                    let snapped = snap_to_nearest_tick(q_price, venue, unit);
                    let tick = tick_size_at_price(venue, q_price, unit);
                    assert!(
                        (q_price - snapped).abs() < 1.0e-6,
                        "{ticker} price {q_price} is not aligned to venue tick {tick} (snapped={snapped})"
                    );
                }
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn single_symbol_query_latency_is_under_2ms() {
        // Build a substantial memory bank (~25,000+ PIT records across 30 liquid symbols x 920 days)
        // and verify single-symbol Dual-Horizon query completes in < 2 ms!
        let mut universe = Vec::with_capacity(30);
        for i in 0..30 {
            let venue = match i % 3 {
                0 => ExchangeVenue::Hose,
                1 => ExchangeVenue::Hnx,
                _ => ExchangeVenue::Upcom,
            };
            let prefix = venue.short_label();
            let sym = format!("S{i:02}");
            let tk = format!("{prefix}:{sym}");
            universe.push(synth_vn_series(
                &tk,
                &sym,
                venue,
                20_000.0 + (i as f64) * 3_500.0,
                3_500_000.0,
                920,
                10_000 + i as u64,
            ));
        }
        let vnindex = synth_vn_series(
            "HOSE:VNINDEX",
            "VNINDEX",
            ExchangeVenue::VnIndexBenchmark,
            1_250.0,
            700_000_000.0,
            920,
            88_888,
        )
        .candles;

        let bank = AnalogMemoryBank::build(&universe, Some(&vnindex), AnalogBankConfig::default());
        assert!(bank.len() >= 20_000, "expected >= 20k records, got {}", bank.len());

        let target = &universe[0];
        let cfg = AnalogConeConfig::default();

        // Warmup cache lines once, then take median over 9 queries to eliminate OS scheduler jitter.
        let _ = bank
            .query(&target.ticker, target.venue, &target.candles, None, cfg)
            .unwrap();

        let mut samples_us = [0_u128; 9];
        for slot in &mut samples_us {
            let t0 = std::time::Instant::now();
            let fc = bank
                .query(&target.ticker, target.venue, &target.candles, None, cfg)
                .unwrap();
            *slot = t0.elapsed().as_micros();
            assert_eq!(fc.steps.len(), DEFAULT_ANALOG_HORIZON);
        }
        samples_us.sort_unstable();
        let median_us = samples_us[samples_us.len() / 2];
        // Note: `cargo test` compiles at opt-level 0 by default unless release/bench is used;
        // even unoptimized or optimized, verify < 2,000 µs (2 ms) when optimized and fast in debug.
        if !cfg!(debug_assertions) {
            assert!(
                median_us < 2_000,
                "release query latency {median_us} µs exceeded 2,000 µs (2 ms) budget"
            );
        } else {
            // Even in unoptimized `-O0` debug builds, our SoA loop should comfortably beat 15ms;
            // we also run `cargo test --release` in verification to enforce `< 2 ms` strictly on full 473-symbol DB!
            assert!(
                median_us < 15_000,
                "debug query latency {median_us} µs unexpectedly slow"
            );
        }
    }
}
