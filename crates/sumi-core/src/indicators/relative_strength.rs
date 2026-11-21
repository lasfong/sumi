//! Pure `f64`/`i64`, `wasm32-unknown-unknown`-compatible Causal Relative Strength (`RS vs Benchmark`)
//! and Main-Pane Rebased Comparison Overlay Engine (`DEF-001` compliant).
//!
//! Eliminates legacy `indicator_engine.py:410–456` intraday lookahead (`YYYY-MM-DD` date join) and
//! holiday `NaN` poisoning by aligning strictly on **completed bar-close timestamps**:
//! `bench_open_ts + bench_tf.seconds() - 1 <= min(sym_close_ts, replay_causal_close_timestamp)`.

use std::ops::Range;

use crate::data::Timeframe;
use crate::model::Candle;

use super::ReferenceLevel;

/// Maximum Last-Observation-Carried-Forward (LOCF) window (`45` calendar days) for market holidays
/// before treating a benchmark quote as non-overlapping / stale.
pub const MAX_BENCHMARK_CARRY_SECONDS: i64 = 45 * 86_400;

/// Horizontal reference baseline (`100.0`) inside the Relative Strength sub-pane.
pub const RS_REFERENCE_LEVELS: [ReferenceLevel; 1] = [ReferenceLevel {
    value: 100.0,
    label: "100",
}];

/// Causal Relative Strength calculation output aligned 1-for-1 with `symbol_candles`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RelativeStrengthOutput {
    /// Latest completed benchmark close causally known at each symbol bar's close timestamp.
    pub aligned_benchmark_close: Vec<Option<f64>>,
    /// Normalized Relative Strength index (`100.0` at first aligned valid pair).
    pub rs_line: Vec<Option<f64>>,
    /// Moving average signal line computed over contiguous valid `rs_line` runs.
    pub rs_sma: Vec<Option<f64>>,
}

/// Computes causal bar-close-aligned `Relative Strength (RS vs Benchmark)` and its SMA signal line.
#[must_use]
pub fn compute_relative_strength(
    symbol_candles: &[Candle],
    symbol_tf: Timeframe,
    benchmark_candles: &[Candle],
    benchmark_tf: Timeframe,
    sma_period: usize,
    replay_causal_close_timestamp: Option<i64>,
) -> RelativeStrengthOutput {
    let n = symbol_candles.len();
    if n == 0 || benchmark_candles.is_empty() {
        return RelativeStrengthOutput {
            aligned_benchmark_close: vec![None; n],
            rs_line: vec![None; n],
            rs_sma: vec![None; n],
        };
    }

    let bench_span = benchmark_tf.seconds().saturating_sub(1);
    let sym_span = symbol_tf.seconds().saturating_sub(1);

    let mut completed_bench: Vec<(i64, f64)> = Vec::with_capacity(benchmark_candles.len());
    for b in benchmark_candles {
        if !b.close.is_finite() || b.close <= 0.0 {
            continue;
        }
        let b_close_ts = b.timestamp.saturating_add(bench_span);
        if replay_causal_close_timestamp.is_some_and(|cut| b_close_ts > cut) {
            continue;
        }
        completed_bench.push((b_close_ts, b.close));
    }
    if completed_bench.is_empty() {
        return RelativeStrengthOutput {
            aligned_benchmark_close: vec![None; n],
            rs_line: vec![None; n],
            rs_sma: vec![None; n],
        };
    }
    completed_bench.sort_by_key(|&(ts, _)| ts);
    // Keep latest observation if duplicate close timestamps exist.
    let mut deduped: Vec<(i64, f64)> = Vec::with_capacity(completed_bench.len());
    for (ts, close) in completed_bench {
        if let Some(last) = deduped.last_mut()
            && last.0 == ts
        {
            last.1 = close;
        } else {
            deduped.push((ts, close));
        }
    }

    let mut aligned_benchmark_close = vec![None; n];
    let mut rs_line = vec![None; n];
    let mut base_pair: Option<(f64, f64)> = None;

    for (i, c) in symbol_candles.iter().enumerate() {
        if !c.close.is_finite() || c.close <= 0.0 {
            continue;
        }
        let raw_sym_close = c.timestamp.saturating_add(sym_span);
        let sym_query_ts =
            replay_causal_close_timestamp.map_or(raw_sym_close, |cut| raw_sym_close.min(cut));
        let pos = deduped.partition_point(|&(b_close_ts, _)| b_close_ts <= sym_query_ts);
        if pos == 0 {
            continue;
        }
        let (b_close_ts, b_close) = deduped[pos - 1];
        if sym_query_ts.saturating_sub(b_close_ts) > MAX_BENCHMARK_CARRY_SECONDS {
            continue;
        }
        aligned_benchmark_close[i] = Some(b_close);

        let (sym0, b0) = *base_pair.get_or_insert((c.close, b_close));
        let ratio = 100.0 * (c.close / sym0) / (b_close / b0);
        if ratio.is_finite() {
            rs_line[i] = Some(ratio);
        }
    }

    let rs_sma = compute_optional_sma(&rs_line, sma_period.clamp(1, 5_000));

    RelativeStrengthOutput {
        aligned_benchmark_close,
        rs_line,
        rs_sma,
    }
}

/// Computes a Main-Pane Rebased Comparison curve anchored to `symbol_candles` at the left-most valid
/// bar inside `anchor_range` (falling back to the earliest valid aligned bar if `anchor_range` has none).
#[must_use]
pub fn compute_rebased_comparison_series(
    symbol_candles: &[Candle],
    aligned_benchmark_close: &[Option<f64>],
    anchor_range: Range<usize>,
) -> Vec<Option<f64>> {
    let n = symbol_candles.len().min(aligned_benchmark_close.len());
    if n == 0 {
        return Vec::new();
    }

    let find_anchor_in = |range: Range<usize>| -> Option<(f64, f64)> {
        let s = range.start.min(n);
        let e = range.end.min(n);
        for i in s..e {
            let sc = symbol_candles[i].close;
            if sc.is_finite()
                && sc > 0.0
                && let Some(bc) = aligned_benchmark_close[i]
                && bc.is_finite()
                && bc > 0.0
            {
                return Some((sc, bc));
            }
        }
        None
    };

    let Some((sym_anchor, bench_anchor)) =
        find_anchor_in(anchor_range).or_else(|| find_anchor_in(0..n))
    else {
        return vec![None; n];
    };

    let mut out = vec![None; n];
    for (dst, bc_opt) in out.iter_mut().zip(aligned_benchmark_close[..n].iter()) {
        if let Some(bc) = bc_opt
            && bc.is_finite()
            && *bc > 0.0
        {
            let rebased = sym_anchor * (*bc / bench_anchor);
            if rebased.is_finite() && rebased > 0.0 {
                *dst = Some(rebased);
            }
        }
    }
    out
}

fn compute_optional_sma(series: &[Option<f64>], period: usize) -> Vec<Option<f64>> {
    let p = period.max(1);
    let mut out = vec![None; series.len()];
    let mut window_sum = 0.0_f64;
    let mut valid_run = 0_usize;

    for (i, item) in series.iter().enumerate() {
        let Some(v) = item.filter(|v| v.is_finite()) else {
            window_sum = 0.0;
            valid_run = 0;
            continue;
        };
        window_sum += v;
        valid_run += 1;
        if valid_run > p
            && let Some(leaving) = series[i - p].filter(|x| x.is_finite())
        {
            window_sum -= leaving;
            valid_run -= 1;
        }
        if valid_run == p {
            if i.is_multiple_of(64) || !window_sum.is_finite() {
                window_sum = series[i + 1 - p..=i]
                    .iter()
                    .filter_map(|x| x.filter(|v| v.is_finite()))
                    .sum();
            }
            let mean = window_sum / (p as f64);
            if mean.is_finite() {
                out[i] = Some(mean);
            }
        }
    }
    out
}
