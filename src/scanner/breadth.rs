//! Pure `f64`/`i64`, `wasm32-unknown-unknown`-compatible Causal Market Breadth & Relative Rotation
//! Graph (`RRG`) Engine (`Milestone R13`).
//!
//! Enforces:
//! - `AT05` / `AT09`: aggregate signed traded values (`sum(buy_val) / sum(total_val)`) BEFORE division.
//! - `AT17`: separate `VN30` core basket participation alongside full universe breadth.
//! - `DEF-001`: strictly causal closed-bar slicing (`causal_closed_prefix_len`) on both equities and
//!   benchmark candles under Bar Replay and multi-timeframe alignment.

use serde::{Deserialize, Serialize};

use crate::data::{Timeframe, VN30_BASKET_SYMBOLS};
use crate::indicators::{
    compute_ema_optional_slice, compute_ema_slices, compute_relative_strength, compute_sma_slices,
    intrabar_buy_fraction,
};
use crate::model::Candle;
use crate::viewport::causal_closed_prefix_len;

/// Relative Rotation Graph (`RRG`) 4-quadrant classification centered at `(RS-Ratio = 100, RS-Mom = 100)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum RrgQuadrant {
    #[default]
    Leading,
    Weakening,
    Lagging,
    Improving,
}

impl RrgQuadrant {
    pub const ALL: [Self; 4] = [
        Self::Leading,
        Self::Weakening,
        Self::Lagging,
        Self::Improving,
    ];

    #[inline]
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::Leading => 0,
            Self::Weakening => 1,
            Self::Lagging => 2,
            Self::Improving => 3,
        }
    }

    #[inline]
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::Leading => "Lead",
            Self::Weakening => "Weak",
            Self::Lagging => "Lag",
            Self::Improving => "Impr",
        }
    }

    #[inline]
    #[must_use]
    pub const fn full_label(self) -> &'static str {
        match self {
            Self::Leading => "Leading",
            Self::Weakening => "Weakening",
            Self::Lagging => "Lagging",
            Self::Improving => "Improving",
        }
    }

    /// Classifies `(rs_ratio, rs_momentum)` around the canonical `(100.0, 100.0)` crosshairs.
    #[inline]
    #[must_use]
    pub fn classify(rs_ratio: f64, rs_momentum: f64) -> Self {
        match (rs_ratio >= 100.0, rs_momentum >= 100.0) {
            (true, true) => Self::Leading,
            (true, false) => Self::Weakening,
            (false, false) => Self::Lagging,
            (false, true) => Self::Improving,
        }
    }
}

/// Borrowed equity / instrument input slice for causal Market Breadth & `RRG` evaluation.
#[derive(Debug, Clone, Copy)]
pub struct BreadthFeedInput<'a> {
    pub instrument_id: &'a str,
    pub symbol: &'a str,
    pub preset_label: &'a str,
    pub timeframe: Timeframe,
    pub candles: &'a [Candle],
    pub exclude_incomplete_tail: bool,
}

/// Causal single-instrument breadth + `RRG` coordinates and 4-bar trailing trajectory.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SymbolBreadthRotation {
    pub instrument_id: String,
    pub symbol: String,
    pub preset_label: String,
    pub last_close: f64,
    pub return_1bar_pct: f64,
    pub traded_value: f64,
    pub buy_flow_ratio_pct: f64,
    pub above_sma20: Option<bool>,
    pub above_ema50: Option<bool>,
    pub rs_ratio: f64,
    pub rs_momentum: f64,
    pub quadrant: RrgQuadrant,
    pub trail: Vec<(f64, f64)>,
}

/// Cross-sectional Market Breadth & 4-Quadrant `RRG` Snapshot obeying `AT05`/`AT09`/`AT17`/`DEF-001`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketBreadthSnapshot {
    pub benchmark_symbol: String,
    pub evaluated_timestamp: Option<i64>,
    pub participating_count: usize,
    pub advancers: usize,
    pub decliners: usize,
    pub unchanged: usize,
    pub adv_dec_ratio: f64,
    pub pct_above_sma20: f64,
    pub pct_above_ema50: f64,
    pub total_traded_value: f64,
    pub aggregate_buy_value: f64,
    pub aggregate_sell_value: f64,
    pub money_flow_buy_pct: f64,
    pub vn30_participating_count: usize,
    pub vn30_money_flow_buy_pct: f64,
    pub quadrant_counts: [usize; 4],
    pub rotations: Vec<SymbolBreadthRotation>,
}

impl Default for MarketBreadthSnapshot {
    fn default() -> Self {
        Self {
            benchmark_symbol: "VNINDEX".to_owned(),
            evaluated_timestamp: None,
            participating_count: 0,
            advancers: 0,
            decliners: 0,
            unchanged: 0,
            adv_dec_ratio: 0.0,
            pct_above_sma20: 0.0,
            pct_above_ema50: 0.0,
            total_traded_value: 0.0,
            aggregate_buy_value: 0.0,
            aggregate_sell_value: 0.0,
            money_flow_buy_pct: 50.0,
            vn30_participating_count: 0,
            vn30_money_flow_buy_pct: 50.0,
            quadrant_counts: [0; 4],
            rotations: Vec::new(),
        }
    }
}

/// Evaluates causal cross-sectional Market Breadth & `RRG` rotation across `feeds` relative to
/// `benchmark_candles`, gating every feed strictly to `<= replay_causal_close_ts` (`DEF-001`).
#[must_use]
pub fn compute_market_breadth_and_rrg(
    feeds: &[BreadthFeedInput<'_>],
    benchmark_candles: &[Candle],
    benchmark_tf: Timeframe,
    benchmark_symbol: &str,
    replay_causal_close_ts: Option<i64>,
) -> MarketBreadthSnapshot {
    let bench_prefix = causal_closed_prefix_len(
        benchmark_candles,
        benchmark_tf.seconds(),
        replay_causal_close_ts,
    );
    let causal_bench = &benchmark_candles[..bench_prefix];
    let mut evaluated_timestamp = causal_bench.last().map(|c| c.timestamp);

    let mut advancers = 0_usize;
    let mut decliners = 0_usize;
    let mut unchanged = 0_usize;
    let mut sma20_eligible = 0_usize;
    let mut sma20_above = 0_usize;
    let mut ema50_eligible = 0_usize;
    let mut ema50_above = 0_usize;
    let mut total_traded_value = 0.0_f64;
    let mut aggregate_buy_value = 0.0_f64;
    let mut aggregate_sell_value = 0.0_f64;
    let mut vn30_participating_count = 0_usize;
    let mut vn30_total_value = 0.0_f64;
    let mut vn30_buy_value = 0.0_f64;
    let mut quadrant_counts = [0_usize; 4];
    let mut rotations = Vec::with_capacity(feeds.len());

    for feed in feeds {
        if feed.symbol.eq_ignore_ascii_case(benchmark_symbol) {
            continue;
        }
        let mut prefix = causal_closed_prefix_len(
            feed.candles,
            feed.timeframe.seconds(),
            replay_causal_close_ts,
        );
        if feed.exclude_incomplete_tail && prefix > 0 {
            prefix -= 1;
        }
        if prefix == 0 {
            continue;
        }
        let slice = &feed.candles[..prefix];
        let last = &slice[prefix - 1];
        if !last.close.is_finite() {
            continue;
        }
        if evaluated_timestamp.is_none_or(|prev| last.timestamp > prev) && causal_bench.is_empty() {
            evaluated_timestamp = Some(last.timestamp);
        }

        let return_1bar_pct = if prefix >= 2 {
            let prev_c = slice[prefix - 2].close;
            if prev_c.is_finite() && prev_c.abs() > 1.0e-12 {
                let pct = 100.0 * (last.close - prev_c) / prev_c;
                if pct.is_finite() { pct } else { 0.0 }
            } else {
                0.0
            }
        } else {
            0.0
        };

        if return_1bar_pct > 1.0e-6 {
            advancers += 1;
        } else if return_1bar_pct < -1.0e-6 {
            decliners += 1;
        } else {
            unchanged += 1;
        }

        let closes: Vec<f64> = slice.iter().map(|c| c.close).collect();
        let sma20 = compute_sma_slices(&closes, 20);
        let ema50 = compute_ema_slices(&closes, 50);
        let above_sma20 = sma20
            .last()
            .copied()
            .flatten()
            .filter(|v| v.is_finite())
            .map(|v| last.close > v);
        let above_ema50 = ema50
            .last()
            .copied()
            .flatten()
            .filter(|v| v.is_finite())
            .map(|v| last.close > v);
        if let Some(above) = above_sma20 {
            sma20_eligible += 1;
            if above {
                sma20_above += 1;
            }
        }
        if let Some(above) = above_ema50 {
            ema50_eligible += 1;
            if above {
                ema50_above += 1;
            }
        }

        // AT05 / AT09: accumulate raw traded value + signed buy/sell dollar flow before ratios.
        let vol = if last.volume.is_finite() && last.volume > 0.0 {
            last.volume
        } else {
            0.0
        };
        let traded_value = if last.close > 0.0 && last.close.is_finite() {
            let tv = last.close * vol;
            if tv.is_finite() { tv } else { 0.0 }
        } else {
            0.0
        };
        let buy_frac = if vol > 0.0 {
            intrabar_buy_fraction(last)
        } else {
            0.5
        };
        let buy_val = traded_value * buy_frac;
        let sell_val = (traded_value - buy_val).max(0.0);
        total_traded_value += traded_value;
        aggregate_buy_value += buy_val;
        aggregate_sell_value += sell_val;

        if VN30_BASKET_SYMBOLS
            .iter()
            .any(|s| s.eq_ignore_ascii_case(feed.symbol))
        {
            vn30_participating_count += 1;
            vn30_total_value += traded_value;
            vn30_buy_value += buy_val;
        }

        let (rs_ratio, rs_momentum, trail) = compute_causal_rrg_trajectory(
            slice,
            feed.timeframe,
            causal_bench,
            benchmark_tf,
            replay_causal_close_ts,
        );
        let quadrant = RrgQuadrant::classify(rs_ratio, rs_momentum);
        quadrant_counts[quadrant.index()] += 1;

        rotations.push(SymbolBreadthRotation {
            instrument_id: feed.instrument_id.to_owned(),
            symbol: feed.symbol.to_owned(),
            preset_label: feed.preset_label.to_owned(),
            last_close: last.close,
            return_1bar_pct,
            traded_value,
            buy_flow_ratio_pct: 100.0 * buy_frac,
            above_sma20,
            above_ema50,
            rs_ratio,
            rs_momentum,
            quadrant,
            trail,
        });
    }

    rotations.sort_by(|a, b| {
        b.traded_value
            .total_cmp(&a.traded_value)
            .then_with(|| a.symbol.cmp(&b.symbol))
    });

    let participating_count = rotations.len();
    let adv_dec_ratio = if decliners > 0 {
        advancers as f64 / decliners as f64
    } else {
        advancers as f64
    };
    let pct_above_sma20 = if sma20_eligible > 0 {
        100.0 * (sma20_above as f64) / (sma20_eligible as f64)
    } else {
        0.0
    };
    let pct_above_ema50 = if ema50_eligible > 0 {
        100.0 * (ema50_above as f64) / (ema50_eligible as f64)
    } else {
        0.0
    };
    let money_flow_buy_pct = if total_traded_value > 0.0 && total_traded_value.is_finite() {
        (100.0 * aggregate_buy_value / total_traded_value).clamp(0.0, 100.0)
    } else {
        50.0
    };
    let vn30_money_flow_buy_pct = if vn30_total_value > 0.0 && vn30_total_value.is_finite() {
        (100.0 * vn30_buy_value / vn30_total_value).clamp(0.0, 100.0)
    } else {
        50.0
    };

    MarketBreadthSnapshot {
        benchmark_symbol: benchmark_symbol.to_owned(),
        evaluated_timestamp,
        participating_count,
        advancers,
        decliners,
        unchanged,
        adv_dec_ratio,
        pct_above_sma20,
        pct_above_ema50,
        total_traded_value,
        aggregate_buy_value,
        aggregate_sell_value,
        money_flow_buy_pct,
        vn30_participating_count,
        vn30_money_flow_buy_pct,
        quadrant_counts,
        rotations,
    }
}

fn compute_causal_rrg_trajectory(
    candles: &[Candle],
    tf: Timeframe,
    bench_candles: &[Candle],
    bench_tf: Timeframe,
    replay_causal_close_ts: Option<i64>,
) -> (f64, f64, Vec<(f64, f64)>) {
    if candles.is_empty() || bench_candles.is_empty() {
        return (100.0, 100.0, vec![(100.0, 100.0)]);
    }

    let rs_out = compute_relative_strength(
        candles,
        tf,
        bench_candles,
        bench_tf,
        10,
        replay_causal_close_ts,
    );
    let ema10 = compute_ema_optional_slice(&rs_out.rs_line, 10);
    let ema26 = compute_ema_optional_slice(&rs_out.rs_line, 26);

    let mut rs_ratio_series = Vec::with_capacity(rs_out.rs_line.len());
    let mut running_sum = 0.0_f64;
    let mut running_cnt = 0_usize;

    for i in 0..rs_out.rs_line.len() {
        if let Some(raw) = rs_out.rs_line[i].filter(|v| v.is_finite()) {
            running_sum += raw;
            running_cnt += 1;
        }
        let ratio = match (ema10[i], ema26[i]) {
            (Some(e10), Some(e26)) if e10.is_finite() && e26.is_finite() && e26.abs() > 1.0e-12 => {
                let v = 100.0 * (1.0 + (e10 - e26) / e26.abs());
                v.is_finite().then_some(v)
            }
            _ if running_cnt > 0 => {
                let slow_mean = running_sum / (running_cnt as f64);
                let fast = ema10[i].or(rs_out.rs_line[i]).filter(|v| v.is_finite());
                match (fast, slow_mean.abs() > 1.0e-12) {
                    (Some(f), true) => {
                        let v = 100.0 * (1.0 + (f - slow_mean) / slow_mean.abs());
                        v.is_finite().then_some(v)
                    }
                    _ => None,
                }
            }
            _ => None,
        };
        rs_ratio_series.push(ratio);
    }

    let ema9_ratio = compute_ema_optional_slice(&rs_ratio_series, 9);
    let mut pts = Vec::new();
    let mut mom_sum = 0.0_f64;
    let mut mom_cnt = 0_usize;

    for i in 0..rs_ratio_series.len() {
        let Some(r) = rs_ratio_series[i].filter(|v| v.is_finite()) else {
            continue;
        };
        mom_sum += r;
        mom_cnt += 1;
        let baseline = ema9_ratio[i]
            .filter(|v| v.is_finite())
            .unwrap_or_else(|| mom_sum / (mom_cnt as f64));
        let mom = 100.0 * (1.0 + (r - baseline) / 100.0);
        if mom.is_finite() {
            pts.push((r, mom));
        }
    }

    if pts.is_empty() {
        return (100.0, 100.0, vec![(100.0, 100.0)]);
    }

    let keep_start = pts.len().saturating_sub(4);
    let trail = pts[keep_start..].to_vec();
    let &(rs_ratio, rs_momentum) = trail.last().unwrap_or(&(100.0, 100.0));
    (rs_ratio, rs_momentum, trail)
}
