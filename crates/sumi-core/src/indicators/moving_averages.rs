use crate::model::Candle;

use super::{
    Indicator, IndicatorOutput, IndicatorPlacement, IndicatorSeriesOutput, SeriesRenderKind,
};

/// Computes the Simple Moving Average (`SMA`) over an arbitrary `f64` slice.
///
/// - Warmup: first valid value at index `period - 1` (`0` when `period == 1`).
/// - Returns `vec![None; values.len()]` if `period == 0` or `period > values.len()`.
/// - Maintains an $O(1)$ sliding sum and reseeds every `32 * period` bars (matching TA-Lib
///   `ta_BBANDS.c:267`) so `100,000+` bar series never accumulate floating-point subtraction drift.
/// - Recovers cleanly after any non-finite (`NaN` / `±Inf`) input bar.
#[must_use]
pub fn compute_sma_slices(values: &[f64], period: usize) -> Vec<Option<f64>> {
    let n = values.len();
    let mut out = vec![None; n];
    if period == 0 || period > n {
        return out;
    }
    if period == 1 {
        for (dst, &v) in out.iter_mut().zip(values.iter()) {
            if v.is_finite() {
                *dst = Some(v);
            }
        }
        return out;
    }

    let period_f = period as f64;
    let reseed_interval = period.saturating_mul(32).max(64);
    let mut finite_run = 0_usize;
    let mut sum = 0.0_f64;
    let mut peak_abs_val = 0.0_f64;
    let mut steps_since_reseed = 0_usize;

    for (i, &val) in values.iter().enumerate() {
        if !val.is_finite() {
            finite_run = 0;
            sum = 0.0;
            peak_abs_val = 0.0;
            steps_since_reseed = 0;
            continue;
        }

        peak_abs_val = peak_abs_val.max(val.abs());

        if finite_run < period {
            sum += val;
            finite_run += 1;
            if finite_run == period {
                if sum.is_finite() && sum.abs() >= 1e-6 * peak_abs_val {
                    out[i] = Some(sum / period_f);
                } else {
                    let start = i + 1 - period;
                    let scaled_mean: f64 = values[start..=i].iter().map(|&x| x / period_f).sum();
                    if scaled_mean.is_finite() {
                        out[i] = Some(scaled_mean);
                    }
                }
                steps_since_reseed = 0;
            }
        } else {
            let trailing = values[i - period];
            sum += val - trailing;
            steps_since_reseed += 1;
            if steps_since_reseed >= reseed_interval
                || !sum.is_finite()
                || sum.abs() < 1e-6 * peak_abs_val
            {
                let start = i + 1 - period;
                sum = values[start..=i].iter().copied().sum();
                peak_abs_val = values[start..=i]
                    .iter()
                    .fold(0.0_f64, |acc, &x| acc.max(x.abs()));
                steps_since_reseed = 0;
            }
            let mean = if sum.is_finite() {
                sum / period_f
            } else {
                let start = i + 1 - period;
                values[start..=i].iter().map(|&x| x / period_f).sum()
            };
            if mean.is_finite() {
                out[i] = Some(mean);
            }
        }
    }

    out
}

/// Computes `SMA(period)` over `Candle::close`.
#[must_use]
pub fn compute_sma(candles: &[Candle], period: usize) -> Vec<Option<f64>> {
    let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();
    compute_sma_slices(&closes, period)
}

#[inline]
fn ema_sterbenz_weights(period: usize) -> (f64, f64) {
    // Exact TA-Lib Sterbenz-lemma weight pair (`ta_EMA.c:130–135`):
    // `beta = (N - 1) / (N + 1)` and `k = 1.0 - beta` (with `if beta < 0.5 { beta = 1.0 - k; }`
    // for `N == 2`) guarantees `beta + k == 1.0` in IEEE-754 binary64 for every `period >= 2`.
    let mut beta = ((period - 1) as f64) / ((period + 1) as f64);
    let k = 1.0 - beta;
    if beta < 0.5 {
        beta = 1.0 - k;
    }
    (beta, k)
}

/// Computes the Exponential Moving Average (`EMA`) over an arbitrary `f64` slice.
///
/// - Warmup: first valid value at index `period - 1`, seeded with the exact `SMA(period)` of the
///   first `period` contiguous finite inputs (`ta_EMA.c:171–178`).
/// - Smoothing weights: Sterbenz-exact `(beta, k)` from `ta_EMA.c:130–135` where `beta + k == 1.0`.
///   Uses `k.mul_add(x - prev, prev)` when `x - prev` is finite (guaranteeing zero-ULP fixed-point
///   invariance on flat price series and within `<= 1 ULP` of `ta_EMA.c` `fma(beta, prev, k * x)`),
///   falling back to `beta.mul_add(prev, k * x)` when `x - prev` overflows `f64::MAX`.
#[must_use]
pub fn compute_ema_slices(values: &[f64], period: usize) -> Vec<Option<f64>> {
    let n = values.len();
    let mut out = vec![None; n];
    if period == 0 || period > n {
        return out;
    }
    if period == 1 {
        for (dst, &v) in out.iter_mut().zip(values.iter()) {
            if v.is_finite() {
                *dst = Some(v);
            }
        }
        return out;
    }

    let period_f = period as f64;
    let (beta, k) = ema_sterbenz_weights(period);

    let mut seed_count = 0_usize;
    let mut seed_sum = 0.0_f64;
    let mut prev_ema: Option<f64> = None;

    for (i, &val) in values.iter().enumerate() {
        if !val.is_finite() {
            seed_count = 0;
            seed_sum = 0.0;
            prev_ema = None;
            continue;
        }

        if let Some(prev) = prev_ema {
            let delta = val - prev;
            let next = if delta.is_finite() {
                k.mul_add(delta, prev)
            } else {
                beta.mul_add(prev, k * val)
            };
            if next.is_finite() {
                out[i] = Some(next);
                prev_ema = Some(next);
            } else {
                prev_ema = None;
                seed_count = 0;
                seed_sum = 0.0;
            }
        } else {
            seed_sum += val;
            seed_count += 1;
            if seed_count == period {
                let seed = if seed_sum.is_finite() {
                    seed_sum / period_f
                } else {
                    let start = i + 1 - period;
                    values[start..=i].iter().map(|&x| x / period_f).sum()
                };
                if seed.is_finite() {
                    out[i] = Some(seed);
                    prev_ema = Some(seed);
                } else {
                    seed_count = 0;
                    seed_sum = 0.0;
                }
            }
        }
    }

    out
}

/// Computes `EMA(period)` over an `Option<f64>` series (e.g. `MACD` signal line over `macd_line`),
/// preserving exact bar-index alignment and high-magnitude IEEE-754 overflow protection.
#[must_use]
pub fn compute_ema_optional_slice(values: &[Option<f64>], period: usize) -> Vec<Option<f64>> {
    let n = values.len();
    let mut out = vec![None; n];
    if period == 0 || period > n {
        return out;
    }
    if period == 1 {
        for (dst, &opt) in out.iter_mut().zip(values.iter()) {
            if let Some(v) = opt
                && v.is_finite()
            {
                *dst = Some(v);
            }
        }
        return out;
    }

    let period_f = period as f64;
    let (beta, k) = ema_sterbenz_weights(period);

    let mut seed_count = 0_usize;
    let mut seed_sum = 0.0_f64;
    let mut prev_ema: Option<f64> = None;

    for (i, &opt) in values.iter().enumerate() {
        let Some(val) = opt.filter(|v| v.is_finite()) else {
            seed_count = 0;
            seed_sum = 0.0;
            prev_ema = None;
            continue;
        };

        if let Some(prev) = prev_ema {
            let delta = val - prev;
            let next = if delta.is_finite() {
                k.mul_add(delta, prev)
            } else {
                beta.mul_add(prev, k * val)
            };
            if next.is_finite() {
                out[i] = Some(next);
                prev_ema = Some(next);
            } else {
                prev_ema = None;
                seed_count = 0;
                seed_sum = 0.0;
            }
        } else {
            seed_sum += val;
            seed_count += 1;
            if seed_count == period {
                let seed = if seed_sum.is_finite() {
                    seed_sum / period_f
                } else {
                    let start = i + 1 - period;
                    values[start..=i]
                        .iter()
                        .filter_map(|&item| item)
                        .map(|x| x / period_f)
                        .sum()
                };
                if seed.is_finite() {
                    out[i] = Some(seed);
                    prev_ema = Some(seed);
                } else {
                    seed_count = 0;
                    seed_sum = 0.0;
                }
            }
        }
    }

    out
}

/// Computes `EMA(period)` over `Candle::close`.
#[must_use]
pub fn compute_ema(candles: &[Candle], period: usize) -> Vec<Option<f64>> {
    let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();
    compute_ema_slices(&closes, period)
}

/// Polymorphic `Indicator` wrapper for Simple Moving Average (`SMA`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SmaIndicator {
    pub period: usize,
}

impl SmaIndicator {
    #[must_use]
    pub const fn new(period: usize) -> Self {
        Self { period }
    }
}

impl Indicator for SmaIndicator {
    fn id(&self) -> &'static str {
        "sma"
    }

    fn title(&self) -> String {
        format!("SMA({})", self.period)
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
                key: "sma",
                label: self.title(),
                kind: SeriesRenderKind::Line,
                values: compute_sma(candles, self.period),
            }],
            reference_levels: Vec::new(),
            fixed_bounds_hint: None,
        }
    }
}

/// Polymorphic `Indicator` wrapper for Exponential Moving Average (`EMA`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmaIndicator {
    pub period: usize,
}

impl EmaIndicator {
    #[must_use]
    pub const fn new(period: usize) -> Self {
        Self { period }
    }
}

impl Indicator for EmaIndicator {
    fn id(&self) -> &'static str {
        "ema"
    }

    fn title(&self) -> String {
        format!("EMA({})", self.period)
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
                key: "ema",
                label: self.title(),
                kind: SeriesRenderKind::Line,
                values: compute_ema(candles, self.period),
            }],
            reference_levels: Vec::new(),
            fixed_bounds_hint: None,
        }
    }
}
