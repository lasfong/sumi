use crate::model::Candle;

use super::moving_averages::{compute_ema_optional_slice, compute_ema_slices};
use super::{
    Indicator, IndicatorOutput, IndicatorPlacement, IndicatorSeriesOutput, ReferenceLevel,
    SeriesRenderKind,
};

/// Canonical horizontal reference guides for `RSI` (`30` / `50` / `70`).
pub const RSI_REFERENCE_LEVELS: [ReferenceLevel; 3] = [
    ReferenceLevel {
        value: 70.0,
        label: "70",
    },
    ReferenceLevel {
        value: 50.0,
        label: "50",
    },
    ReferenceLevel {
        value: 30.0,
        label: "30",
    },
];

/// Canonical `[0, 100]` domain bounds hint for `RSI`.
pub const RSI_FIXED_BOUNDS_HINT: (f64, f64) = (0.0, 100.0);

/// Canonical zero-line reference guide for `MACD`.
pub const MACD_REFERENCE_LEVELS: [ReferenceLevel; 1] = [ReferenceLevel {
    value: 0.0,
    label: "0",
}];

/// Aligned three-series output for `MACD(fast, slow, signal)`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MacdOutput {
    pub macd_line: Vec<Option<f64>>,
    pub signal_line: Vec<Option<f64>>,
    pub histogram: Vec<Option<f64>>,
}

impl MacdOutput {
    #[must_use]
    pub fn empty(len: usize) -> Self {
        Self {
            macd_line: vec![None; len],
            signal_line: vec![None; len],
            histogram: vec![None; len],
        }
    }
}

/// Computes Wilder's Relative Strength Index (`RSI`) over an arbitrary `f64` slice (`ta_RSI.c:143–251`).
///
/// - Valid domain: `period >= 2` (default `14`). First valid output index: `period` (`N` deltas from `N + 1` bars).
/// - Exact TA-Lib #253 & #480 edge-case contract:
///   - `total = avg_gain + avg_loss`
///   - `total > 0.0` (exact `0.0` comparison, never a fixed quote-unit epsilon) → `100.0 * (avg_gain / total)`
///   - `total == 0.0` (flat `0/0` price series) → neutral **`50.0`**
#[must_use]
pub fn compute_rsi_slices(values: &[f64], period: usize) -> Vec<Option<f64>> {
    let n = values.len();
    let mut out = vec![None; n];
    if period < 2 || period >= n {
        return out;
    }

    let period_f = period as f64;
    let decay = (period - 1) as f64;

    let mut prev_val: Option<f64> = None;
    let mut delta_count = 0_usize;
    let mut gain_sum = 0.0_f64;
    let mut loss_sum = 0.0_f64;
    let mut avg_gain = 0.0_f64;
    let mut avg_loss = 0.0_f64;
    let mut seeded = false;

    for (i, &val) in values.iter().enumerate() {
        if !val.is_finite() {
            prev_val = None;
            delta_count = 0;
            gain_sum = 0.0;
            loss_sum = 0.0;
            seeded = false;
            continue;
        }

        let Some(prev) = prev_val else {
            prev_val = Some(val);
            continue;
        };
        prev_val = Some(val);

        let delta = val - prev;
        if !delta.is_finite() {
            delta_count = 0;
            gain_sum = 0.0;
            loss_sum = 0.0;
            seeded = false;
            continue;
        }

        let gain = if delta > 0.0 { delta } else { 0.0 };
        let loss = gain - delta; // exact non-negative loss magnitude (`ta_RSI.c:160`)

        if !seeded {
            gain_sum += gain;
            loss_sum += loss;
            delta_count += 1;
            if delta_count == period {
                if gain_sum.is_finite() && loss_sum.is_finite() {
                    avg_gain = gain_sum / period_f;
                    avg_loss = loss_sum / period_f;
                } else {
                    avg_gain = 0.0;
                    avg_loss = 0.0;
                    let start = i - period;
                    for w in values[start..=i].windows(2) {
                        let d = w[1] - w[0];
                        let g = if d > 0.0 { d } else { 0.0 };
                        let l = g - d;
                        avg_gain += g / period_f;
                        avg_loss += l / period_f;
                    }
                }
                if avg_gain.is_finite() && avg_loss.is_finite() {
                    seeded = true;
                    out[i] = Some(rsi_from_averages(avg_gain, avg_loss));
                } else {
                    delta_count = 0;
                    gain_sum = 0.0;
                    loss_sum = 0.0;
                }
            }
        } else {
            avg_gain = wilder_step(avg_gain, gain, decay, period_f);
            avg_loss = wilder_step(avg_loss, loss, decay, period_f);
            if avg_gain.is_finite() && avg_loss.is_finite() {
                out[i] = Some(rsi_from_averages(avg_gain, avg_loss));
            } else {
                delta_count = 0;
                gain_sum = 0.0;
                loss_sum = 0.0;
                seeded = false;
            }
        }
    }

    out
}

#[inline]
fn wilder_step(prev_avg: f64, sample: f64, decay: f64, period_f: f64) -> f64 {
    let direct = (prev_avg * decay + sample) / period_f;
    if direct.is_finite() {
        return direct;
    }
    let delta = sample - prev_avg;
    if delta.is_finite() {
        prev_avg + delta / period_f
    } else {
        (prev_avg / period_f) * decay + (sample / period_f)
    }
}

#[inline]
fn rsi_from_averages(avg_gain: f64, avg_loss: f64) -> f64 {
    let total = avg_gain + avg_loss;
    if !total.is_finite() {
        let scale = avg_gain.max(avg_loss);
        if scale > 0.0 && scale.is_finite() {
            let sg = avg_gain / scale;
            let sl = avg_loss / scale;
            return (100.0 * (sg / (sg + sl))).clamp(0.0, 100.0);
        }
        return 50.0;
    }
    if total > 0.0 {
        (100.0 * (avg_gain / total)).clamp(0.0, 100.0)
    } else {
        50.0
    }
}

/// Computes `RSI(period)` over `Candle::close`.
#[must_use]
pub fn compute_rsi(candles: &[Candle], period: usize) -> Vec<Option<f64>> {
    let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();
    compute_rsi_slices(&closes, period)
}

/// Computes `MACD(fast_period, slow_period, signal_period)` over an arbitrary `f64` slice.
///
/// - Auto-swaps `(fast_period, slow_period)` if `slow_period < fast_period` (`ta_MACD.c:174–180`).
/// - Valid domain: `fast_period >= 2`, `slow_period >= 2`, `signal_period >= 1`.
/// - `macd_line[t] = EMA(fast)[t] - EMA(slow)[t]` starting at `t = slow_period - 1`.
/// - `signal_line = EMA(macd_line, signal_period)` starting at `t = (slow_period - 1) + (signal_period - 1)`.
/// - When `signal_period == 1`, `signal_line = macd_line` and `histogram = 0.0` (`ta_MACD.c:212–215`).
#[must_use]
pub fn compute_macd_slices(
    values: &[f64],
    fast_period: usize,
    slow_period: usize,
    signal_period: usize,
) -> MacdOutput {
    let n = values.len();
    let mut out = MacdOutput::empty(n);
    if fast_period < 2 || slow_period < 2 || signal_period < 1 {
        return out;
    }

    let (fast, slow) = if slow_period < fast_period {
        (slow_period, fast_period)
    } else {
        (fast_period, slow_period)
    };
    if slow > n {
        return out;
    }

    let fast_ema = compute_ema_slices(values, fast);
    let slow_ema = compute_ema_slices(values, slow);

    for i in 0..n {
        if let (Some(f), Some(s)) = (fast_ema[i], slow_ema[i]) {
            let diff = f - s;
            if diff.is_finite() {
                out.macd_line[i] = Some(diff);
            }
        }
    }

    if signal_period == 1 {
        for i in 0..n {
            if let Some(m) = out.macd_line[i] {
                out.signal_line[i] = Some(m);
                out.histogram[i] = Some(0.0);
            }
        }
        return out;
    }

    out.signal_line = compute_ema_optional_slice(&out.macd_line, signal_period);
    for i in 0..n {
        if let (Some(m), Some(sig)) = (out.macd_line[i], out.signal_line[i]) {
            let hist = m - sig;
            if hist.is_finite() {
                out.histogram[i] = Some(hist);
            }
        }
    }

    out
}

/// Computes `MACD(fast, slow, signal)` over `Candle::close`.
#[must_use]
pub fn compute_macd(
    candles: &[Candle],
    fast_period: usize,
    slow_period: usize,
    signal_period: usize,
) -> MacdOutput {
    let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();
    compute_macd_slices(&closes, fast_period, slow_period, signal_period)
}

/// Computes Wilder's True Range (`TR`) at bar `t >= 1` given `(high, low, prev_close)` (`ta_ATR.c:200–214`).
#[inline]
#[must_use]
pub fn true_range(high: f64, low: f64, prev_close: f64) -> Option<f64> {
    if !high.is_finite() || !low.is_finite() || !prev_close.is_finite() {
        return None;
    }
    let hi = high.max(low);
    let lo = high.min(low);
    let hl = hi - lo;
    let hc = (prev_close - hi).abs();
    let lc = (prev_close - lo).abs();
    let tr = hl.max(hc).max(lc);
    tr.is_finite().then_some(tr)
}

/// Computes Wilder's Average True Range (`ATR`) matching TA-Lib (`ta_ATR.c:64–78`, `167–274`).
///
/// - Valid domain: `period >= 1` (default `14`).
/// - Because True Range requires `prev_close` (`candles[t - 1].close`), `TR` starts at index `1`
///   and the first valid `ATR(period)` output is at index **`period`** (`14` for `ATR(14)`).
/// - Uses monotonic Wilder smoothing `prev_atr + w_alpha * (tr - prev_atr)` with overflow fallback.
#[must_use]
pub fn compute_atr(candles: &[Candle], period: usize) -> Vec<Option<f64>> {
    let n = candles.len();
    let mut out = vec![None; n];
    if period == 0 || period >= n {
        return out;
    }

    let period_f = period as f64;
    let mut w_beta = ((period - 1) as f64) / period_f;
    let w_alpha = 1.0 - w_beta;
    if w_beta < 0.5 {
        w_beta = 1.0 - w_alpha;
    }

    let mut prev_close: Option<f64> = candles[0].close.is_finite().then_some(candles[0].close);
    let mut tr_count = 0_usize;
    let mut tr_sum = 0.0_f64;
    let mut prev_atr: Option<f64> = None;

    for (i, c) in candles.iter().enumerate().skip(1) {
        let current_close_ok = c.close.is_finite();
        let tr_opt = prev_close.and_then(|pc| true_range(c.high, c.low, pc));
        prev_close = current_close_ok.then_some(c.close);

        let Some(tr) = tr_opt else {
            tr_count = 0;
            tr_sum = 0.0;
            prev_atr = None;
            continue;
        };

        if period == 1 {
            out[i] = Some(tr);
            prev_atr = Some(tr);
            continue;
        }

        if let Some(prev) = prev_atr {
            let delta = tr - prev;
            let next = if delta.is_finite() {
                w_alpha.mul_add(delta, prev)
            } else {
                w_beta.mul_add(prev, w_alpha * tr)
            };
            if next.is_finite() {
                out[i] = Some(next);
                prev_atr = Some(next);
            } else {
                tr_count = 0;
                tr_sum = 0.0;
                prev_atr = None;
            }
        } else {
            tr_sum += tr;
            tr_count += 1;
            if tr_count == period {
                let seed = if tr_sum.is_finite() {
                    tr_sum / period_f
                } else {
                    let start = i + 1 - period;
                    (start..=i)
                        .filter_map(|k| {
                            true_range(candles[k].high, candles[k].low, candles[k - 1].close)
                        })
                        .map(|t| t / period_f)
                        .sum()
                };
                if seed.is_finite() {
                    out[i] = Some(seed);
                    prev_atr = Some(seed);
                } else {
                    tr_count = 0;
                    tr_sum = 0.0;
                }
            }
        }
    }

    out
}

/// Polymorphic `Indicator` wrapper for Wilder's `RSI`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RsiIndicator {
    pub period: usize,
}

impl RsiIndicator {
    #[must_use]
    pub const fn new(period: usize) -> Self {
        Self { period }
    }
}

impl Indicator for RsiIndicator {
    fn id(&self) -> &'static str {
        "rsi"
    }

    fn title(&self) -> String {
        format!("RSI({})", self.period)
    }

    fn placement(&self) -> IndicatorPlacement {
        IndicatorPlacement::OscillatorPane
    }

    fn warmup_bars(&self) -> usize {
        self.period.max(2)
    }

    fn evaluate(&self, candles: &[Candle]) -> IndicatorOutput {
        IndicatorOutput {
            id: self.id(),
            title: self.title(),
            placement: self.placement(),
            series: vec![IndicatorSeriesOutput {
                key: "rsi",
                label: self.title(),
                kind: SeriesRenderKind::Line,
                values: compute_rsi(candles, self.period),
            }],
            reference_levels: RSI_REFERENCE_LEVELS.to_vec(),
            fixed_bounds_hint: Some(RSI_FIXED_BOUNDS_HINT),
        }
    }
}

/// Polymorphic `Indicator` wrapper for `MACD`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MacdIndicator {
    pub fast_period: usize,
    pub slow_period: usize,
    pub signal_period: usize,
}

impl MacdIndicator {
    #[must_use]
    pub const fn new(fast_period: usize, slow_period: usize, signal_period: usize) -> Self {
        Self {
            fast_period,
            slow_period,
            signal_period,
        }
    }
}

impl Indicator for MacdIndicator {
    fn id(&self) -> &'static str {
        "macd"
    }

    fn title(&self) -> String {
        format!(
            "MACD({}, {}, {})",
            self.fast_period, self.slow_period, self.signal_period
        )
    }

    fn placement(&self) -> IndicatorPlacement {
        IndicatorPlacement::OscillatorPane
    }

    fn warmup_bars(&self) -> usize {
        let slow = self.fast_period.max(self.slow_period);
        slow.saturating_sub(1)
            .saturating_add(self.signal_period.saturating_sub(1))
    }

    fn evaluate(&self, candles: &[Candle]) -> IndicatorOutput {
        let macd = compute_macd(
            candles,
            self.fast_period,
            self.slow_period,
            self.signal_period,
        );
        IndicatorOutput {
            id: self.id(),
            title: self.title(),
            placement: self.placement(),
            series: vec![
                IndicatorSeriesOutput {
                    key: "histogram",
                    label: "Hist".to_owned(),
                    kind: SeriesRenderKind::Histogram,
                    values: macd.histogram,
                },
                IndicatorSeriesOutput {
                    key: "macd",
                    label: "MACD".to_owned(),
                    kind: SeriesRenderKind::Line,
                    values: macd.macd_line,
                },
                IndicatorSeriesOutput {
                    key: "signal",
                    label: "Signal".to_owned(),
                    kind: SeriesRenderKind::Line,
                    values: macd.signal_line,
                },
            ],
            reference_levels: MACD_REFERENCE_LEVELS.to_vec(),
            fixed_bounds_hint: None,
        }
    }
}

/// Polymorphic `Indicator` wrapper for Wilder's `ATR`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AtrIndicator {
    pub period: usize,
}

impl AtrIndicator {
    #[must_use]
    pub const fn new(period: usize) -> Self {
        Self { period }
    }
}

impl Indicator for AtrIndicator {
    fn id(&self) -> &'static str {
        "atr"
    }

    fn title(&self) -> String {
        format!("ATR({})", self.period)
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
                key: "atr",
                label: self.title(),
                kind: SeriesRenderKind::Line,
                values: compute_atr(candles, self.period),
            }],
            reference_levels: Vec::new(),
            fixed_bounds_hint: None,
        }
    }
}
