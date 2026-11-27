//! Extended Pure-`f64`, `wasm32-unknown-unknown`-Compatible Technical Indicators (`Milestone R18 / Wave 17` — `B2`).
//!
//! Extended trend, channel, momentum, and money-flow oscillators
//! verified against TA-Lib (`ta_ADX.c`, `ta_STOCH.c`, `ta_SAR.c`, `ta_CCI.c`, `ta_MFI.c`):
//! 1. `ADX(14)` (`+DI`, `-DI`, `ADX`)
//! 2. `Stochastic(14, 3, 3)` (`Fast %K`, `Slow %K`, `Slow %D`)
//! 3. `SuperTrend(10, 3.0)` (`trend`, `upper_band`, `lower_band`, `direction`)
//! 4. `Parabolic SAR(0.02, 0.02, 0.20)` (`sar`, `is_long`)
//! 5. `Keltner Channels(20, 10, 2.0)` (`upper`, `middle`, `lower`)
//! 6. `CCI(20)` (scale-relative zero-deviation guard)
//! 7. `MFI(14)` (TA-Lib `nullRun` drift reset + zero-flow `50.0` neutrality)
//! 8. `Volume SMA(20)` (causal rolling volume moving average)

use std::collections::VecDeque;

use crate::model::Candle;

use super::{
    Indicator, IndicatorOutput, IndicatorPlacement, IndicatorSeriesOutput, ReferenceLevel,
    SeriesRenderKind, compute_atr, compute_ema_slices, true_range,
};

/// Canonical reference guides for [`OscillatorPaneId::Adx`].
pub const ADX_REFERENCE_LEVELS: [ReferenceLevel; 2] = [
    ReferenceLevel {
        value: 20.0,
        label: "20",
    },
    ReferenceLevel {
        value: 25.0,
        label: "25",
    },
];

/// Fixed bounds hint for `ADX` / `+DI` / `-DI` (`[0, 100]`).
pub const ADX_FIXED_BOUNDS_HINT: (f64, f64) = (0.0, 100.0);

/// Canonical reference guides for [`OscillatorPaneId::Stochastic`].
pub const STOCH_REFERENCE_LEVELS: [ReferenceLevel; 3] = [
    ReferenceLevel {
        value: 20.0,
        label: "20",
    },
    ReferenceLevel {
        value: 50.0,
        label: "50",
    },
    ReferenceLevel {
        value: 80.0,
        label: "80",
    },
];

/// Fixed bounds hint for `Stochastic` (`[0, 100]`).
pub const STOCH_FIXED_BOUNDS_HINT: (f64, f64) = (0.0, 100.0);

/// Canonical reference guides for [`OscillatorPaneId::Cci`].
pub const CCI_REFERENCE_LEVELS: [ReferenceLevel; 3] = [
    ReferenceLevel {
        value: -100.0,
        label: "-100",
    },
    ReferenceLevel {
        value: 0.0,
        label: "0",
    },
    ReferenceLevel {
        value: 100.0,
        label: "+100",
    },
];

/// Canonical reference guides for [`OscillatorPaneId::Mfi`].
pub const MFI_REFERENCE_LEVELS: [ReferenceLevel; 3] = [
    ReferenceLevel {
        value: 20.0,
        label: "20",
    },
    ReferenceLevel {
        value: 50.0,
        label: "50",
    },
    ReferenceLevel {
        value: 80.0,
        label: "80",
    },
];

/// Fixed bounds hint for `MFI` (`[0, 100]`).
pub const MFI_FIXED_BOUNDS_HINT: (f64, f64) = (0.0, 100.0);

/// Output series for Wilder's Average Directional Index (`ADX`, `+DI`, `-DI`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AdxOutput {
    pub adx: Vec<Option<f64>>,
    pub plus_di: Vec<Option<f64>>,
    pub minus_di: Vec<Option<f64>>,
}

/// Computes Wilder's `ADX(period)` (`+DI`, `-DI` from `t = period`, `ADX` from `t = 2 * period - 1`)
/// matching TA-Lib `ta_ADX.c` (`prevTR > 0.0` exact scale-free guard, issue `#253`).
#[must_use]
pub fn compute_adx(candles: &[Candle], period: usize) -> AdxOutput {
    let n = candles.len();
    let mut adx = vec![None; n];
    let mut plus_di = vec![None; n];
    let mut minus_di = vec![None; n];
    if period == 0 || n <= period {
        return AdxOutput {
            adx,
            plus_di,
            minus_di,
        };
    }

    let p_f = period as f64;
    let inv_p = 1.0 / p_f;
    let adx_start = 2 * period - 1;

    // Strictly causal contiguous-segment processing (`DEF-001` prefix invariance & NaN recovery):
    // Any invalid bar (`NaN`/`Inf` or `high < low`) emits `None` at `t` and resets segment state
    // without ever mutating historical indices `< t`.
    let mut seg_len = 0_usize;
    let mut prev_plus_dm = 0.0_f64;
    let mut prev_minus_dm = 0.0_f64;
    let mut prev_tr = 0.0_f64;
    let mut sum_dx = 0.0_f64;
    let mut prev_adx = 0.0_f64;

    for t in 0..n {
        let c = &candles[t];
        let valid =
            c.high.is_finite() && c.low.is_finite() && c.close.is_finite() && c.high >= c.low;
        if !valid {
            seg_len = 0;
            prev_plus_dm = 0.0;
            prev_minus_dm = 0.0;
            prev_tr = 0.0;
            sum_dx = 0.0;
            prev_adx = 0.0;
            continue;
        }

        let seg_idx = seg_len;
        seg_len += 1;
        if seg_idx == 0 {
            continue;
        }

        let prev = &candles[t - 1];
        let diff_p = c.high - prev.high;
        let diff_m = prev.low - c.low;
        let (plus_dm1, minus_dm1) = if diff_p > 0.0 && diff_p > diff_m {
            (diff_p, 0.0)
        } else if diff_m > 0.0 && diff_m > diff_p {
            (0.0, diff_m)
        } else {
            (0.0, 0.0)
        };
        let Some(tr1) = true_range(c.high, c.low, prev.close) else {
            seg_len = 0;
            prev_plus_dm = 0.0;
            prev_minus_dm = 0.0;
            prev_tr = 0.0;
            sum_dx = 0.0;
            prev_adx = 0.0;
            continue;
        };

        if seg_idx < period {
            prev_plus_dm += plus_dm1;
            prev_minus_dm += minus_dm1;
            prev_tr += tr1;
            continue;
        }

        if seg_idx == period && period == 1 {
            prev_plus_dm = plus_dm1;
            prev_minus_dm = minus_dm1;
            prev_tr = tr1;
        } else {
            prev_plus_dm = prev_plus_dm - prev_plus_dm * inv_p + plus_dm1;
            prev_minus_dm = prev_minus_dm - prev_minus_dm * inv_p + minus_dm1;
            prev_tr = prev_tr - prev_tr * inv_p + tr1;
        }

        if !prev_plus_dm.is_finite() || !prev_minus_dm.is_finite() || !prev_tr.is_finite() {
            seg_len = 0;
            prev_plus_dm = 0.0;
            prev_minus_dm = 0.0;
            prev_tr = 0.0;
            sum_dx = 0.0;
            prev_adx = 0.0;
            continue;
        }

        let (p_di, m_di, dx) = if prev_tr > 0.0 {
            let p_di = (100.0 * (prev_plus_dm / prev_tr)).clamp(0.0, 100.0);
            let m_di = (100.0 * (prev_minus_dm / prev_tr)).clamp(0.0, 100.0);
            let sum_di = p_di + m_di;
            let dx = if sum_di > 1.0e-14 {
                (100.0 * ((p_di - m_di).abs() / sum_di)).clamp(0.0, 100.0)
            } else {
                0.0
            };
            (p_di, m_di, dx)
        } else {
            (0.0, 0.0, 0.0)
        };

        plus_di[t] = Some(p_di);
        minus_di[t] = Some(m_di);

        if seg_idx < adx_start {
            sum_dx += dx;
        } else if seg_idx == adx_start {
            sum_dx += dx;
            prev_adx = (sum_dx * inv_p).clamp(0.0, 100.0);
            adx[t] = Some(prev_adx);
        } else {
            prev_adx = ((prev_adx * (p_f - 1.0) + dx) * inv_p).clamp(0.0, 100.0);
            adx[t] = Some(prev_adx);
        }
    }

    AdxOutput {
        adx,
        plus_di,
        minus_di,
    }
}

/// Output series for Full Stochastic Oscillator (`Fast %K`, `Slow %K`, `Slow %D`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StochasticOutput {
    pub fast_k: Vec<Option<f64>>,
    pub slow_k: Vec<Option<f64>>,
    pub slow_d: Vec<Option<f64>>,
}

fn sma_over_optional(values: &[Option<f64>], period: usize) -> Vec<Option<f64>> {
    let n = values.len();
    let mut out = vec![None; n];
    if period == 0 || n < period {
        return out;
    }
    let mut sum = 0.0_f64;
    let mut valid_run = 0_usize;
    for i in 0..n {
        if let Some(v) = values[i].filter(|v| v.is_finite()) {
            sum += v;
            valid_run += 1;
            if valid_run > period
                && let Some(old) = values[i - period].filter(|x| x.is_finite())
            {
                sum -= old;
            }
            if valid_run >= period {
                if valid_run.is_multiple_of(64) {
                    // Periodic exact resummation prevents drift over 100k+ bars.
                    sum = values[i + 1 - period..=i].iter().filter_map(|x| *x).sum();
                }
                out[i] = Some(sum / (period as f64));
            }
        } else {
            sum = 0.0;
            valid_run = 0;
        }
    }
    out
}

/// Computes `Stochastic(k_period, k_smooth, d_period)` in $O(N)$ time via monotonic extrema deques,
/// matching TA-Lib `ta_STOCH.c` (`50.0` neutral fallback on zero-range `HH == LL` windows).
#[must_use]
pub fn compute_stochastic(
    candles: &[Candle],
    k_period: usize,
    k_smooth: usize,
    d_period: usize,
) -> StochasticOutput {
    let n = candles.len();
    let mut fast_k = vec![None; n];
    if k_period == 0 || k_smooth == 0 || d_period == 0 || n < k_period {
        return StochasticOutput {
            fast_k,
            slow_k: vec![None; n],
            slow_d: vec![None; n],
        };
    }

    let mut max_dq: VecDeque<usize> = VecDeque::with_capacity(k_period + 1);
    let mut min_dq: VecDeque<usize> = VecDeque::with_capacity(k_period + 1);
    let mut invalid_run_until: Option<usize> = None;

    for t in 0..n {
        let c = &candles[t];
        if !c.high.is_finite() || !c.low.is_finite() || !c.close.is_finite() || c.high < c.low {
            max_dq.clear();
            min_dq.clear();
            invalid_run_until = Some(t.saturating_add(k_period - 1));
            continue;
        }

        let window_start = t.saturating_sub(k_period - 1);
        while max_dq.front().is_some_and(|&idx| idx < window_start) {
            max_dq.pop_front();
        }
        while min_dq.front().is_some_and(|&idx| idx < window_start) {
            min_dq.pop_front();
        }

        while max_dq
            .back()
            .is_some_and(|&idx| candles[idx].high <= c.high)
        {
            max_dq.pop_back();
        }
        max_dq.push_back(t);

        while min_dq.back().is_some_and(|&idx| candles[idx].low >= c.low) {
            min_dq.pop_back();
        }
        min_dq.push_back(t);

        if t + 1 < k_period || invalid_run_until.is_some_and(|lim| t <= lim) {
            continue;
        }

        let hh = candles[*max_dq.front().expect("non-empty max_dq")].high;
        let ll = candles[*min_dq.front().expect("non-empty min_dq")].low;
        let span = hh - ll;
        let scale_eps = 1.0e-12 * (hh.abs() + ll.abs()).max(1.0e-30);
        let fk = if span <= scale_eps {
            50.0
        } else {
            (100.0 * (c.close - ll) / span).clamp(0.0, 100.0)
        };
        fast_k[t] = Some(fk);
    }

    let slow_k: Vec<Option<f64>> = sma_over_optional(&fast_k, k_smooth)
        .into_iter()
        .map(|v| v.map(|x| x.clamp(0.0, 100.0)))
        .collect();
    let slow_d: Vec<Option<f64>> = sma_over_optional(&slow_k, d_period)
        .into_iter()
        .map(|v| v.map(|x| x.clamp(0.0, 100.0)))
        .collect();

    StochasticOutput {
        fast_k,
        slow_k,
        slow_d,
    }
}

/// Output series for `SuperTrend(period, multiplier)`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SuperTrendOutput {
    pub trend: Vec<Option<f64>>,
    pub upper_band: Vec<Option<f64>>,
    pub lower_band: Vec<Option<f64>>,
    /// `+1` for bullish uptrend (active `lower_band` support), `-1` for bearish downtrend (`upper_band` resistance).
    pub direction: Vec<Option<i8>>,
}

/// Computes `SuperTrend(period, multiplier)` using Wilder `ATR(period)` (`hl2 ± multiplier * atr`)
/// with stateful ratcheting final bands and zero lookahead.
#[must_use]
pub fn compute_supertrend(candles: &[Candle], period: usize, multiplier: f64) -> SuperTrendOutput {
    let n = candles.len();
    let mut trend = vec![None; n];
    let mut upper_band = vec![None; n];
    let mut lower_band = vec![None; n];
    let mut direction = vec![None; n];
    if period == 0 || n < period || !multiplier.is_finite() || multiplier <= 0.0 {
        return SuperTrendOutput {
            trend,
            upper_band,
            lower_band,
            direction,
        };
    }

    let atr = compute_atr(candles, period);
    let mut prev_final_upper: Option<f64> = None;
    let mut prev_final_lower: Option<f64> = None;
    let mut prev_dir: i8 = 1;

    for t in 0..n {
        let c = &candles[t];
        let Some(atr_val) = atr[t].filter(|v| v.is_finite()) else {
            prev_final_upper = None;
            prev_final_lower = None;
            continue;
        };
        if !c.high.is_finite() || !c.low.is_finite() || !c.close.is_finite() || c.high < c.low {
            prev_final_upper = None;
            prev_final_lower = None;
            continue;
        }

        let hl2 = 0.5 * c.high + 0.5 * c.low;
        let delta = multiplier * atr_val;
        let basic_upper = hl2 + delta;
        let basic_lower = hl2 - delta;
        if !hl2.is_finite() || !basic_upper.is_finite() || !basic_lower.is_finite() {
            prev_final_upper = None;
            prev_final_lower = None;
            continue;
        }

        let (final_upper, final_lower, dir) = match (prev_final_upper, prev_final_lower) {
            (Some(pfu), Some(pfl)) if t > 0 && candles[t - 1].close.is_finite() => {
                let prev_close = candles[t - 1].close;
                let fu = if basic_upper < pfu || prev_close > pfu {
                    basic_upper
                } else {
                    pfu
                };
                let fl = if basic_lower > pfl || prev_close < pfl {
                    basic_lower
                } else {
                    pfl
                };
                let d = if prev_dir == -1 && c.close > fu {
                    1
                } else if prev_dir == 1 && c.close < fl {
                    -1
                } else {
                    prev_dir
                };
                (fu, fl, d)
            }
            _ => {
                let init_dir = if c.close >= hl2 { 1 } else { -1 };
                (basic_upper, basic_lower, init_dir)
            }
        };

        let active_trend = if dir == 1 { final_lower } else { final_upper };
        if !active_trend.is_finite() || !final_upper.is_finite() || !final_lower.is_finite() {
            prev_final_upper = None;
            prev_final_lower = None;
            continue;
        }
        trend[t] = Some(active_trend);
        upper_band[t] = Some(final_upper);
        lower_band[t] = Some(final_lower);
        direction[t] = Some(dir);

        prev_final_upper = Some(final_upper);
        prev_final_lower = Some(final_lower);
        prev_dir = dir;
    }

    SuperTrendOutput {
        trend,
        upper_band,
        lower_band,
        direction,
    }
}

/// Output series for Wilder's `Parabolic SAR(af_start, af_step, af_max)`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PsarOutput {
    pub sar: Vec<Option<f64>>,
    pub is_long: Vec<Option<bool>>,
}

/// Computes Wilder's `Parabolic SAR(af_start, af_step, af_max)` matching TA-Lib `ta_SAR.c`
/// (`sar[0] = None`, initialized from bar `0 -> 1` `-DM` directional check and clamped to the
/// prior two bars' high/low envelope).
#[must_use]
pub fn compute_psar(candles: &[Candle], af_start: f64, af_step: f64, af_max: f64) -> PsarOutput {
    let n = candles.len();
    let mut sar_out = vec![None; n];
    let mut is_long_out = vec![None; n];
    if n < 2
        || !af_start.is_finite()
        || !af_step.is_finite()
        || !af_max.is_finite()
        || af_start <= 0.0
        || af_step <= 0.0
        || af_max <= 0.0
    {
        return PsarOutput {
            sar: sar_out,
            is_long: is_long_out,
        };
    }

    let max_af = af_max;
    let init_af = af_start.min(max_af);
    let step_af = af_step.min(max_af);

    // Strictly causal contiguous-segment processing (`DEF-001` prefix invariance & NaN recovery):
    // Any invalid bar (`NaN`/`Inf` or `high < low`) emits `None` at `today_idx` and re-seeds SAR
    // cleanly on the next contiguous 2-bar valid segment without mutating historical indices.
    let mut seg_len = 0_usize;
    let mut is_long = true;
    let mut af = init_af;
    let mut ep = 0.0_f64;
    let mut sar = 0.0_f64;
    let mut new_low = 0.0_f64;
    let mut new_high = 0.0_f64;

    for today_idx in 0..n {
        let c = &candles[today_idx];
        if !c.high.is_finite() || !c.low.is_finite() || c.high < c.low {
            seg_len = 0;
            continue;
        }

        let seg_idx = seg_len;
        seg_len += 1;
        if seg_idx == 0 {
            continue;
        }

        if seg_idx == 1 {
            // TA-Lib initial direction check via 1-bar -DM between segment bar 0 (`today_idx - 1`) and bar 1 (`today_idx`):
            let prev_c = &candles[today_idx - 1];
            let diff_p = c.high - prev_c.high;
            let diff_m = prev_c.low - c.low;
            let minus_dm1 = if diff_m > 0.0 && diff_m > diff_p {
                diff_m
            } else {
                0.0
            };
            is_long = minus_dm1 <= 0.0;
            af = init_af;
            ep = if is_long { c.high } else { c.low };
            sar = if is_long { prev_c.low } else { prev_c.high };
            new_low = prev_c.low;
            new_high = prev_c.high;
        }

        let prev_low = new_low;
        let prev_high = new_high;
        new_low = c.low;
        new_high = c.high;

        let emitted_sar;
        let emitted_long;
        if is_long {
            if new_low <= sar {
                is_long = false;
                sar = ep.max(prev_high).max(new_high);
                emitted_sar = sar;
                emitted_long = false;

                af = init_af;
                ep = new_low;
                sar = af.mul_add(ep - sar, sar).max(prev_high).max(new_high);
            } else {
                emitted_sar = sar;
                emitted_long = true;

                if new_high > ep {
                    ep = new_high;
                    af = (af + step_af).min(max_af);
                }
                sar = af.mul_add(ep - sar, sar).min(prev_low).min(new_low);
            }
        } else if new_high >= sar {
            is_long = true;
            sar = ep.min(prev_low).min(new_low);
            emitted_sar = sar;
            emitted_long = true;

            af = init_af;
            ep = new_high;
            sar = af.mul_add(ep - sar, sar).min(prev_low).min(new_low);
        } else {
            emitted_sar = sar;
            emitted_long = false;

            if new_low < ep {
                ep = new_low;
                af = (af + step_af).min(max_af);
            }
            sar = af.mul_add(ep - sar, sar).max(prev_high).max(new_high);
        }

        if emitted_sar.is_finite() && sar.is_finite() {
            sar_out[today_idx] = Some(emitted_sar);
            is_long_out[today_idx] = Some(emitted_long);
        } else {
            seg_len = 0;
        }
    }

    PsarOutput {
        sar: sar_out,
        is_long: is_long_out,
    }
}

/// Output series for `Keltner Channels(ema_period, atr_period, multiplier)`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct KeltnerOutput {
    pub upper: Vec<Option<f64>>,
    pub middle: Vec<Option<f64>>,
    pub lower: Vec<Option<f64>>,
}

/// Computes `Keltner Channels` (`EMA(close, ema_period) ± multiplier * ATR(atr_period)`).
#[must_use]
pub fn compute_keltner_channels(
    candles: &[Candle],
    ema_period: usize,
    atr_period: usize,
    multiplier: f64,
) -> KeltnerOutput {
    let n = candles.len();
    if ema_period == 0 || atr_period == 0 || n == 0 || !multiplier.is_finite() || multiplier <= 0.0
    {
        return KeltnerOutput {
            upper: vec![None; n],
            middle: vec![None; n],
            lower: vec![None; n],
        };
    }

    let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();
    let middle = compute_ema_slices(&closes, ema_period);
    let atr = compute_atr(candles, atr_period);
    let mut upper = vec![None; n];
    let mut lower = vec![None; n];

    for t in 0..n {
        if let (Some(m), Some(a)) = (middle[t], atr[t])
            && m.is_finite()
            && a.is_finite()
        {
            let up = m + multiplier * a;
            let lo = m - multiplier * a;
            if up.is_finite() && lo.is_finite() {
                upper[t] = Some(up);
                lower[t] = Some(lo);
            }
        }
    }

    KeltnerOutput {
        upper,
        middle,
        lower,
    }
}

/// Computes Lambert's Commodity Channel Index `CCI(period)` (`(TP - SMA(TP)) / (0.015 * MeanDev)`)
/// with TA-Lib circular-buffer resummation and scale-relative zero-deviation protection (`ta_CCI.c`).
#[must_use]
pub fn compute_cci(candles: &[Candle], period: usize) -> Vec<Option<f64>> {
    let n = candles.len();
    let mut out = vec![None; n];
    if period == 0 || n < period {
        return out;
    }

    let mut ring = vec![0.0_f64; period];
    let mut ring_idx = 0_usize;
    let mut valid_run = 0_usize;
    let inv_p = 1.0 / (period as f64);

    for (t, c) in candles.iter().enumerate() {
        if !c.high.is_finite() || !c.low.is_finite() || !c.close.is_finite() || c.high < c.low {
            valid_run = 0;
            ring_idx = 0;
            continue;
        }
        let tp = c.high / 3.0 + c.low / 3.0 + c.close / 3.0;
        if !tp.is_finite() {
            valid_run = 0;
            ring_idx = 0;
            continue;
        }
        ring[ring_idx] = tp;
        ring_idx += 1;
        if ring_idx == period {
            ring_idx = 0;
        }
        valid_run += 1;
        if valid_run < period {
            continue;
        }

        let avg = ring.iter().map(|&v| v * inv_p).sum::<f64>();
        let mean_dev = ring.iter().map(|&v| (v - avg).abs() * inv_p).sum::<f64>();
        if !avg.is_finite() || !mean_dev.is_finite() {
            continue;
        }
        let diff = tp - avg;
        let scale_band = 1.0e-14 * avg.abs();
        let denom = 0.015 * mean_dev;
        let cci = if diff.abs() <= scale_band || mean_dev <= scale_band || denom == 0.0 {
            0.0
        } else {
            let v = diff / denom;
            if v.is_finite() { v } else { 0.0 }
        };
        if cci.is_finite() {
            out[t] = Some(cci);
        }
    }

    out
}

/// Computes Volume-Weighted Money Flow Index `MFI(period)` (`t >= period`) matching TA-Lib `ta_MFI.c`
/// (`nullRun` residue reset + scale-relative typical-price dead-zone), emitting neutral `50.0` when
/// both positive and negative money flow sums over the window equal `0.0`.
#[must_use]
pub fn compute_mfi(candles: &[Candle], period: usize) -> Vec<Option<f64>> {
    let n = candles.len();
    let mut out = vec![None; n];
    if period == 0 || n <= period {
        return out;
    }

    let mut pos_ring = vec![0.0_f64; period];
    let mut neg_ring = vec![0.0_f64; period];
    let mut ring_idx = 0_usize;
    let mut pos_sum = 0.0_f64;
    let mut neg_sum = 0.0_f64;
    let mut null_run = 0_usize;
    let mut valid_bars = 0_usize;
    let mut prev_tp = 0.0_f64;

    for (t, c) in candles.iter().enumerate() {
        if !c.high.is_finite()
            || !c.low.is_finite()
            || !c.close.is_finite()
            || !c.volume.is_finite()
            || c.high < c.low
            || c.volume < 0.0
        {
            pos_ring.fill(0.0);
            neg_ring.fill(0.0);
            ring_idx = 0;
            pos_sum = 0.0;
            neg_sum = 0.0;
            null_run = 0;
            valid_bars = 0;
            continue;
        }

        let tp = c.high / 3.0 + c.low / 3.0 + c.close / 3.0;
        let raw_mf = tp * c.volume;
        if !tp.is_finite() || !raw_mf.is_finite() {
            pos_ring.fill(0.0);
            neg_ring.fill(0.0);
            ring_idx = 0;
            pos_sum = 0.0;
            neg_sum = 0.0;
            null_run = 0;
            valid_bars = 0;
            continue;
        }
        valid_bars += 1;
        if valid_bars == 1 {
            prev_tp = tp;
            continue;
        }

        if valid_bars > period + 1 {
            pos_sum -= pos_ring[ring_idx];
            neg_sum -= neg_ring[ring_idx];
        }

        let delta_tp = tp - prev_tp;
        let tp_scale = tp.abs() + prev_tp.abs();
        prev_tp = tp;

        let is_zero_move =
            !delta_tp.is_finite() || delta_tp.abs() <= 1.0e-14 * tp_scale || raw_mf <= 0.0;
        let (pos_flow, neg_flow) = if is_zero_move {
            (0.0, 0.0)
        } else if delta_tp > 0.0 {
            (raw_mf, 0.0)
        } else {
            (0.0, raw_mf)
        };

        pos_ring[ring_idx] = pos_flow;
        neg_ring[ring_idx] = neg_flow;
        pos_sum += pos_flow;
        neg_sum += neg_flow;

        if pos_flow == 0.0 && neg_flow == 0.0 {
            null_run = (null_run + 1).min(period);
            if null_run >= period {
                pos_sum = 0.0;
                neg_sum = 0.0;
            }
        } else {
            null_run = 0;
        }

        ring_idx += 1;
        if ring_idx == period {
            ring_idx = 0;
        }

        if valid_bars > period {
            if valid_bars.is_multiple_of(64) || !pos_sum.is_finite() || !neg_sum.is_finite() {
                pos_sum = pos_ring.iter().sum();
                neg_sum = neg_ring.iter().sum();
            }
            let total = pos_sum + neg_sum;
            let mfi = if !total.is_finite() {
                // Overflow-safe scaled resummation for extreme `1e300+` flows (`BUG-027`).
                let max_flow = pos_ring
                    .iter()
                    .chain(neg_ring.iter())
                    .copied()
                    .fold(0.0_f64, f64::max);
                if max_flow > 0.0 && max_flow.is_finite() {
                    let sp: f64 = pos_ring.iter().map(|&v| v / max_flow).sum();
                    let sn: f64 = neg_ring.iter().map(|&v| v / max_flow).sum();
                    let st = sp + sn;
                    if st > 0.0 && st.is_finite() {
                        (100.0 * (sp.clamp(0.0, st) / st)).clamp(0.0, 100.0)
                    } else {
                        50.0
                    }
                } else {
                    continue;
                }
            } else if total <= 0.0 {
                50.0
            } else {
                let clamped_pos = pos_sum.clamp(0.0, total);
                (100.0 * (clamped_pos / total)).clamp(0.0, 100.0)
            };
            if mfi.is_finite() {
                out[t] = Some(mfi);
            }
        }
    }

    out
}

/// Computes causal rolling `Volume SMA(period)` rejecting negative or non-finite volumes (`None`).
#[must_use]
pub fn compute_volume_sma(candles: &[Candle], period: usize) -> Vec<Option<f64>> {
    let n = candles.len();
    let mut out = vec![None; n];
    if period == 0 || n < period {
        return out;
    }

    let inv_p = 1.0 / (period as f64);
    let mut sum = 0.0_f64;
    let mut valid_run = 0_usize;

    for (t, c) in candles.iter().enumerate() {
        if !c.volume.is_finite()
            || c.volume < 0.0
            || !c.open.is_finite()
            || !c.high.is_finite()
            || !c.low.is_finite()
            || !c.close.is_finite()
            || c.high < c.low
        {
            sum = 0.0;
            valid_run = 0;
            continue;
        }
        sum += c.volume;
        valid_run += 1;
        if valid_run > period {
            sum -= candles[t - period].volume;
        }
        if valid_run >= period {
            if valid_run.is_multiple_of(64) {
                sum = candles[t + 1 - period..=t].iter().map(|b| b.volume).sum();
            }
            let mean = if sum.is_finite() {
                sum * inv_p
            } else {
                candles[t + 1 - period..=t]
                    .iter()
                    .map(|b| b.volume * inv_p)
                    .sum::<f64>()
            };
            if mean.is_finite() {
                out[t] = Some(mean.max(0.0));
            }
        }
    }

    out
}

// ============================================================================
// Polymorphic `Indicator` Trait Wrappers (`B2`)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdxIndicator {
    pub period: usize,
}

impl Indicator for AdxIndicator {
    fn id(&self) -> &'static str {
        "adx"
    }

    fn title(&self) -> String {
        format!("ADX({})", self.period)
    }

    fn placement(&self) -> IndicatorPlacement {
        IndicatorPlacement::OscillatorPane
    }

    fn warmup_bars(&self) -> usize {
        2 * self.period.max(1) - 1
    }

    fn evaluate(&self, candles: &[Candle]) -> IndicatorOutput {
        let out = compute_adx(candles, self.period);
        IndicatorOutput {
            id: self.id(),
            title: self.title(),
            placement: self.placement(),
            series: vec![
                IndicatorSeriesOutput {
                    key: "adx",
                    label: format!("ADX({})", self.period),
                    kind: SeriesRenderKind::Line,
                    values: out.adx,
                },
                IndicatorSeriesOutput {
                    key: "plus_di",
                    label: format!("+DI({})", self.period),
                    kind: SeriesRenderKind::Line,
                    values: out.plus_di,
                },
                IndicatorSeriesOutput {
                    key: "minus_di",
                    label: format!("-DI({})", self.period),
                    kind: SeriesRenderKind::Line,
                    values: out.minus_di,
                },
            ],
            reference_levels: ADX_REFERENCE_LEVELS.to_vec(),
            fixed_bounds_hint: Some(ADX_FIXED_BOUNDS_HINT),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StochasticIndicator {
    pub k_period: usize,
    pub k_smooth: usize,
    pub d_period: usize,
}

impl Indicator for StochasticIndicator {
    fn id(&self) -> &'static str {
        "stoch"
    }

    fn title(&self) -> String {
        format!(
            "Stoch({}, {}, {})",
            self.k_period, self.k_smooth, self.d_period
        )
    }

    fn placement(&self) -> IndicatorPlacement {
        IndicatorPlacement::OscillatorPane
    }

    fn warmup_bars(&self) -> usize {
        self.k_period
            .saturating_add(self.k_smooth)
            .saturating_add(self.d_period)
            .saturating_sub(3)
    }

    fn evaluate(&self, candles: &[Candle]) -> IndicatorOutput {
        let out = compute_stochastic(candles, self.k_period, self.k_smooth, self.d_period);
        IndicatorOutput {
            id: self.id(),
            title: self.title(),
            placement: self.placement(),
            series: vec![
                IndicatorSeriesOutput {
                    key: "slow_k",
                    label: "%K".to_owned(),
                    kind: SeriesRenderKind::Line,
                    values: out.slow_k,
                },
                IndicatorSeriesOutput {
                    key: "slow_d",
                    label: "%D".to_owned(),
                    kind: SeriesRenderKind::Line,
                    values: out.slow_d,
                },
            ],
            reference_levels: STOCH_REFERENCE_LEVELS.to_vec(),
            fixed_bounds_hint: Some(STOCH_FIXED_BOUNDS_HINT),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SuperTrendIndicator {
    pub period: usize,
    pub multiplier: f64,
}

impl Indicator for SuperTrendIndicator {
    fn id(&self) -> &'static str {
        "supertrend"
    }

    fn title(&self) -> String {
        format!("SuperTrend({}, {:.1})", self.period, self.multiplier)
    }

    fn placement(&self) -> IndicatorPlacement {
        IndicatorPlacement::PriceOverlay
    }

    fn warmup_bars(&self) -> usize {
        self.period
    }

    fn evaluate(&self, candles: &[Candle]) -> IndicatorOutput {
        let out = compute_supertrend(candles, self.period, self.multiplier);
        IndicatorOutput {
            id: self.id(),
            title: self.title(),
            placement: self.placement(),
            series: vec![IndicatorSeriesOutput {
                key: "supertrend",
                label: self.title(),
                kind: SeriesRenderKind::Line,
                values: out.trend,
            }],
            reference_levels: Vec::new(),
            fixed_bounds_hint: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PsarIndicator {
    pub af_start: f64,
    pub af_step: f64,
    pub af_max: f64,
}

impl Indicator for PsarIndicator {
    fn id(&self) -> &'static str {
        "psar"
    }

    fn title(&self) -> String {
        format!(
            "PSAR({:.2}, {:.2}, {:.2})",
            self.af_start, self.af_step, self.af_max
        )
    }

    fn placement(&self) -> IndicatorPlacement {
        IndicatorPlacement::PriceOverlay
    }

    fn warmup_bars(&self) -> usize {
        1
    }

    fn evaluate(&self, candles: &[Candle]) -> IndicatorOutput {
        let out = compute_psar(candles, self.af_start, self.af_step, self.af_max);
        IndicatorOutput {
            id: self.id(),
            title: self.title(),
            placement: self.placement(),
            series: vec![IndicatorSeriesOutput {
                key: "psar",
                label: self.title(),
                kind: SeriesRenderKind::Line,
                values: out.sar,
            }],
            reference_levels: Vec::new(),
            fixed_bounds_hint: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeltnerIndicator {
    pub ema_period: usize,
    pub atr_period: usize,
    pub multiplier: f64,
}

impl Indicator for KeltnerIndicator {
    fn id(&self) -> &'static str {
        "kc"
    }

    fn title(&self) -> String {
        format!(
            "KC({}, {}, {:.1})",
            self.ema_period, self.atr_period, self.multiplier
        )
    }

    fn placement(&self) -> IndicatorPlacement {
        IndicatorPlacement::PriceOverlay
    }

    fn warmup_bars(&self) -> usize {
        self.ema_period.saturating_sub(1).max(self.atr_period)
    }

    fn evaluate(&self, candles: &[Candle]) -> IndicatorOutput {
        let out =
            compute_keltner_channels(candles, self.ema_period, self.atr_period, self.multiplier);
        IndicatorOutput {
            id: self.id(),
            title: self.title(),
            placement: self.placement(),
            series: vec![
                IndicatorSeriesOutput {
                    key: "kc_upper",
                    label: "KC Upper".to_owned(),
                    kind: SeriesRenderKind::Line,
                    values: out.upper,
                },
                IndicatorSeriesOutput {
                    key: "kc_middle",
                    label: "KC Mid".to_owned(),
                    kind: SeriesRenderKind::Line,
                    values: out.middle,
                },
                IndicatorSeriesOutput {
                    key: "kc_lower",
                    label: "KC Lower".to_owned(),
                    kind: SeriesRenderKind::Line,
                    values: out.lower,
                },
            ],
            reference_levels: Vec::new(),
            fixed_bounds_hint: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CciIndicator {
    pub period: usize,
}

impl Indicator for CciIndicator {
    fn id(&self) -> &'static str {
        "cci"
    }

    fn title(&self) -> String {
        format!("CCI({})", self.period)
    }

    fn placement(&self) -> IndicatorPlacement {
        IndicatorPlacement::OscillatorPane
    }

    fn warmup_bars(&self) -> usize {
        self.period.saturating_sub(1)
    }

    fn evaluate(&self, candles: &[Candle]) -> IndicatorOutput {
        IndicatorOutput {
            id: self.id(),
            title: self.title(),
            placement: self.placement(),
            series: vec![IndicatorSeriesOutput {
                key: "cci",
                label: self.title(),
                kind: SeriesRenderKind::Line,
                values: compute_cci(candles, self.period),
            }],
            reference_levels: CCI_REFERENCE_LEVELS.to_vec(),
            fixed_bounds_hint: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MfiIndicator {
    pub period: usize,
}

impl Indicator for MfiIndicator {
    fn id(&self) -> &'static str {
        "mfi"
    }

    fn title(&self) -> String {
        format!("MFI({})", self.period)
    }

    fn placement(&self) -> IndicatorPlacement {
        IndicatorPlacement::OscillatorPane
    }

    fn warmup_bars(&self) -> usize {
        self.period
    }

    fn evaluate(&self, candles: &[Candle]) -> IndicatorOutput {
        IndicatorOutput {
            id: self.id(),
            title: self.title(),
            placement: self.placement(),
            series: vec![IndicatorSeriesOutput {
                key: "mfi",
                label: self.title(),
                kind: SeriesRenderKind::Line,
                values: compute_mfi(candles, self.period),
            }],
            reference_levels: MFI_REFERENCE_LEVELS.to_vec(),
            fixed_bounds_hint: Some(MFI_FIXED_BOUNDS_HINT),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VolumeSmaIndicator {
    pub period: usize,
}

impl Indicator for VolumeSmaIndicator {
    fn id(&self) -> &'static str {
        "volume_sma"
    }

    fn title(&self) -> String {
        format!("Vol SMA({})", self.period)
    }

    fn placement(&self) -> IndicatorPlacement {
        IndicatorPlacement::PriceOverlay
    }

    fn warmup_bars(&self) -> usize {
        self.period.saturating_sub(1)
    }

    fn evaluate(&self, candles: &[Candle]) -> IndicatorOutput {
        IndicatorOutput {
            id: self.id(),
            title: self.title(),
            placement: self.placement(),
            series: vec![IndicatorSeriesOutput {
                key: "volume_sma",
                label: self.title(),
                kind: SeriesRenderKind::Line,
                values: compute_volume_sma(candles, self.period),
            }],
            reference_levels: Vec::new(),
            fixed_bounds_hint: None,
        }
    }
}
