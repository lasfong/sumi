use crate::model::Candle;

use super::{
    Indicator, IndicatorOutput, IndicatorPlacement, IndicatorSeriesOutput, SeriesRenderKind,
};

/// Aligned three-band output for Bollinger Bands (`upper`, `middle`, `lower`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BollingerOutput {
    pub upper: Vec<Option<f64>>,
    pub middle: Vec<Option<f64>>,
    pub lower: Vec<Option<f64>>,
}

impl BollingerOutput {
    #[must_use]
    pub fn empty(len: usize) -> Self {
        Self {
            upper: vec![None; len],
            middle: vec![None; len],
            lower: vec![None; len],
        }
    }
}

/// Computes Bollinger Bands (`middle = SMA(N)`, `upper = middle + k * sigma`, `lower = middle - k * sigma`)
/// using TA-Lib's exact **population standard deviation** (`1 / N`, `ddof = 0`, `ta_BBANDS.c:157, 251, 277–330`)
/// with shifted-mean numerical stability, max-abs scaled overflow protection, and periodic reseed every
/// `32 * period` bars.
///
/// - Valid domain: `period >= 2` and `std_mult >= 0.0` (matching `ta_BBANDS.c:157` and `IndicatorConfig`).
#[must_use]
pub fn compute_bollinger_slices(values: &[f64], period: usize, std_mult: f64) -> BollingerOutput {
    let n = values.len();
    let mut out = BollingerOutput::empty(n);
    if period < 2 || period > n || !std_mult.is_finite() || std_mult < 0.0 {
        return out;
    }

    let inv_period = 1.0 / (period as f64);
    let reseed_interval = period.saturating_mul(32).max(64);

    let mut finite_run = 0_usize;
    let mut shift = 0.0_f64;
    let mut ma_sum = 0.0_f64;
    let mut var_sum1 = 0.0_f64;
    let mut var_sum2 = 0.0_f64;
    let mut peak_sum2 = 0.0_f64;
    let mut steps_since_reseed = 0_usize;

    for (i, &val) in values.iter().enumerate() {
        if !val.is_finite() {
            finite_run = 0;
            ma_sum = 0.0;
            var_sum1 = 0.0;
            var_sum2 = 0.0;
            peak_sum2 = 0.0;
            steps_since_reseed = 0;
            continue;
        }

        if finite_run == 0 {
            shift = val;
        }

        if finite_run < period {
            ma_sum += val;
            let d = val - shift;
            var_sum1 += d;
            var_sum2 += d * d;
            finite_run += 1;

            if finite_run == period {
                peak_sum2 = if var_sum2.is_finite() { var_sum2 } else { 0.0 };
                let (mean, stddev) = eval_window_stats(
                    values,
                    i + 1 - period,
                    period,
                    inv_period,
                    ma_sum,
                    var_sum1,
                    var_sum2,
                    false,
                );
                write_band(&mut out, i, mean, stddev, std_mult);
                steps_since_reseed = 0;
            }
        } else {
            let trailing_idx = i - period;
            let trailing = values[trailing_idx];

            ma_sum += val - trailing;
            let d_in = val - shift;
            let d_out = trailing - shift;
            var_sum1 += d_in - d_out;
            var_sum2 += d_in * d_in - d_out * d_out;
            if var_sum2.is_finite() {
                peak_sum2 = peak_sum2.max(var_sum2);
            }
            steps_since_reseed += 1;

            let mean1 = var_sum1 * inv_period;
            let raw_var = var_sum2 * inv_period - mean1 * mean1;
            let need_reseed = !ma_sum.is_finite()
                || !var_sum2.is_finite()
                || !raw_var.is_finite()
                || raw_var < 1e-6 * (peak_sum2 * inv_period)
                || steps_since_reseed >= reseed_interval;

            let window_start = i + 1 - period;
            let (mean, stddev) = if need_reseed {
                // Reseed `shift`, `ma_sum`, `var_sum1`, `var_sum2` directly from window (`ta_BBANDS.c:289–330`)
                let exact_mean: f64 = values[window_start..=i]
                    .iter()
                    .map(|&x| x * inv_period)
                    .sum();
                shift = if exact_mean.is_finite() {
                    exact_mean
                } else {
                    val
                };
                ma_sum = values[window_start..=i].iter().copied().sum();
                var_sum1 = 0.0;
                var_sum2 = 0.0;
                for &w in &values[window_start..=i] {
                    let d = w - shift;
                    var_sum1 += d;
                    var_sum2 += d * d;
                }
                peak_sum2 = if var_sum2.is_finite() { var_sum2 } else { 0.0 };
                steps_since_reseed = 0;
                eval_window_stats(
                    values,
                    window_start,
                    period,
                    inv_period,
                    ma_sum,
                    var_sum1,
                    var_sum2,
                    true,
                )
            } else {
                let clamped_var = if raw_var < 1e-12 * (var_sum2 * inv_period) {
                    0.0
                } else {
                    raw_var.max(0.0)
                };
                (ma_sum * inv_period, clamped_var.sqrt())
            };

            write_band(&mut out, i, mean, stddev, std_mult);
        }
    }

    out
}

#[allow(clippy::too_many_arguments)]
fn eval_window_stats(
    values: &[f64],
    window_start: usize,
    period: usize,
    inv_period: f64,
    ma_sum: f64,
    var_sum1: f64,
    var_sum2: f64,
    already_centered: bool,
) -> (f64, f64) {
    let window = &values[window_start..window_start + period];
    let mean = if ma_sum.is_finite() {
        ma_sum * inv_period
    } else {
        window.iter().map(|&x| x * inv_period).sum()
    };
    if !mean.is_finite() {
        return (f64::NAN, f64::NAN);
    }

    let mean1 = var_sum1 * inv_period;
    let mut variance = var_sum2 * inv_period - mean1 * mean1;
    if !variance.is_finite() || (!already_centered && variance < 1e-6 * (var_sum2 * inv_period)) {
        let mut s2 = 0.0_f64;
        for &w in window {
            let d = w - mean;
            s2 += (d * d) * inv_period;
        }
        variance = s2;
    } else if variance < 1e-12 * (var_sum2 * inv_period) {
        variance = 0.0;
    }

    if variance.is_finite() {
        return (mean, if variance > 0.0 { variance.sqrt() } else { 0.0 });
    }

    // Overflow-safe scaled population standard deviation (`|w - mean| > sqrt(f64::MAX)`):
    // never emit a bogus `stddev = 0.0` when `d * d` overflows (`ta_BBANDS.c:345–353`).
    let mut max_abs_dev = 0.0_f64;
    for &w in window {
        let ad = (w - mean).abs();
        if ad > max_abs_dev || ad.is_nan() {
            max_abs_dev = ad;
        }
    }
    if max_abs_dev == 0.0 {
        return (mean, 0.0);
    }
    if max_abs_dev.is_finite() {
        let mut norm_s2 = 0.0_f64;
        for &w in window {
            let nd = (w - mean) / max_abs_dev;
            norm_s2 += (nd * nd) * inv_period;
        }
        let sd = max_abs_dev * norm_s2.sqrt();
        return (mean, if sd.is_finite() { sd } else { f64::NAN });
    }

    // Extreme opposite-sign window where `w - mean` itself exceeded `f64::MAX`: scale by `max(|w|, |mean|)`.
    let scale = window.iter().fold(mean.abs(), |acc, &w| acc.max(w.abs()));
    if !scale.is_finite() || scale == 0.0 {
        return (mean, f64::NAN);
    }
    let scaled_mean = mean / scale;
    let mut norm_s2 = 0.0_f64;
    for &w in window {
        let nd = (w / scale) - scaled_mean;
        norm_s2 += (nd * nd) * inv_period;
    }
    let sd = scale * norm_s2.sqrt();
    (mean, if sd.is_finite() { sd } else { f64::NAN })
}

#[inline]
fn write_band(out: &mut BollingerOutput, idx: usize, mean: f64, stddev: f64, std_mult: f64) {
    if !mean.is_finite() || !stddev.is_finite() {
        return;
    }
    let offset = stddev * std_mult;
    let upper = mean + offset;
    let lower = mean - offset;
    if upper.is_finite() && lower.is_finite() {
        out.middle[idx] = Some(mean);
        out.upper[idx] = Some(upper);
        out.lower[idx] = Some(lower);
    }
}

/// Computes `BollingerBands(period, std_mult)` over `Candle::close`.
#[must_use]
pub fn compute_bollinger_bands(
    candles: &[Candle],
    period: usize,
    std_mult: f64,
) -> BollingerOutput {
    let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();
    compute_bollinger_slices(&closes, period, std_mult)
}

/// Polymorphic `Indicator` wrapper for Bollinger Bands.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BollingerBandsIndicator {
    pub period: usize,
    pub std_mult: f64,
}

impl BollingerBandsIndicator {
    #[must_use]
    pub const fn new(period: usize, std_mult: f64) -> Self {
        Self { period, std_mult }
    }
}

impl Indicator for BollingerBandsIndicator {
    fn id(&self) -> &'static str {
        "bbands"
    }

    fn title(&self) -> String {
        format!("BB({}, {:.1})", self.period, self.std_mult)
    }

    fn placement(&self) -> IndicatorPlacement {
        IndicatorPlacement::PriceOverlay
    }

    fn warmup_bars(&self) -> usize {
        self.period.max(2) - 1
    }

    fn evaluate(&self, candles: &[Candle]) -> IndicatorOutput {
        let bb = compute_bollinger_bands(candles, self.period, self.std_mult);
        IndicatorOutput {
            id: self.id(),
            title: self.title(),
            placement: self.placement(),
            series: vec![
                IndicatorSeriesOutput {
                    key: "upper",
                    label: "BB Upper".to_owned(),
                    kind: SeriesRenderKind::Line,
                    values: bb.upper,
                },
                IndicatorSeriesOutput {
                    key: "middle",
                    label: "BB Basis".to_owned(),
                    kind: SeriesRenderKind::Line,
                    values: bb.middle,
                },
                IndicatorSeriesOutput {
                    key: "lower",
                    label: "BB Lower".to_owned(),
                    kind: SeriesRenderKind::Line,
                    values: bb.lower,
                },
            ],
            reference_levels: Vec::new(),
            fixed_bounds_hint: None,
        }
    }
}
