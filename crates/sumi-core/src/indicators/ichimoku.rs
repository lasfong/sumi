//! Pure `f64`, `wasm32-unknown-unknown`-compatible Ichimoku Kinko Hyo (`Ichimoku Cloud`) Engine.
//!
//! Institutional Ichimoku Kinko Hyo implementation invariants:
//! - Strict $O(N)$ sliding-window extrema via dual monotonic `VecDeque<usize>` buffers (replacing $O(N \cdot P)$ loops).
//! - Overflow-safe midpoint `lo + 0.5 * (hi - lo)` and finite-bar corruption recovery.
//! - Forward-displaced `Senkou Span A` & `Senkou Span B` (`len = N + D` when `N > 0`), keeping right-whitespace
//!   Kumo Cloud `[N .. N + D)` 100% causal (`DEF-001` compliant — uses only bars `<= N - 1`).
//! - Strictly causal `Chikou Span` (`len = N`): `chikou[k] = close[k + D]` ONLY while `k + D < N`; indices
//!   `k >= N.saturating_sub(D)` MUST be `None` so Bar Replay and live charts never leak future closes (`DEF-001`).

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use crate::model::Candle;

use super::{
    Indicator, IndicatorOutput, IndicatorPlacement, IndicatorSeriesOutput, SeriesRenderKind,
};

/// Maximum forward/backward displacement supported for Ichimoku spans.
pub const MAX_ICHIMOKU_DISPLACEMENT: usize = 250;

/// Configuration parameters for [`compute_ichimoku`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct IchimokuConfig {
    pub tenkan_period: usize,
    pub kijun_period: usize,
    pub senkou_b_period: usize,
    pub displacement: usize,
    pub show_chikou: bool,
}

impl Default for IchimokuConfig {
    fn default() -> Self {
        Self {
            tenkan_period: 9,
            kijun_period: 26,
            senkou_b_period: 52,
            displacement: 26,
            show_chikou: true,
        }
    }
}

impl IchimokuConfig {
    /// Canonical QMV V7 Ichimoku configuration (`Tenkan=6, Kijun=18, SenkouB=36, Displacement=18`).
    #[inline]
    #[must_use]
    pub const fn qmv_v7() -> Self {
        Self {
            tenkan_period: 6,
            kijun_period: 18,
            senkou_b_period: 36,
            displacement: 18,
            show_chikou: true,
        }
    }

    /// Short-horizon QMV Ichimoku variant (`Tenkan=3, Kijun=6, SenkouB=18, Displacement=18`).
    #[inline]
    #[must_use]
    pub const fn qmv_short() -> Self {
        Self {
            tenkan_period: 3,
            kijun_period: 6,
            senkou_b_period: 18,
            displacement: 18,
            show_chikou: true,
        }
    }

    /// Clamps configuration fields into valid mathematical domains (`1..=500` periods, `0..=250` shift).
    #[must_use]
    pub fn sanitized(self) -> Self {
        Self {
            tenkan_period: self.tenkan_period.clamp(1, 500),
            kijun_period: self.kijun_period.clamp(1, 500),
            senkou_b_period: self.senkou_b_period.clamp(1, 500),
            displacement: self.displacement.min(MAX_ICHIMOKU_DISPLACEMENT),
            show_chikou: self.show_chikou,
        }
    }
}

/// Complete aligned Ichimoku Kinko Hyo output series.
///
/// - `tenkan`, `kijun`, and `chikou` have length `candles.len()`.
/// - `senkou_a` and `senkou_b` have length `if candles.is_empty() { 0 } else { candles.len() + displacement }`,
///   allowing the $+D$-bar forward Kumo Cloud inside right-hand chart whitespace.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct IchimokuOutput {
    pub tenkan: Vec<Option<f64>>,
    pub kijun: Vec<Option<f64>>,
    pub senkou_a: Vec<Option<f64>>,
    pub senkou_b: Vec<Option<f64>>,
    pub chikou: Vec<Option<f64>>,
    pub displacement: usize,
}

/// Computes the $O(N)$ rolling Donchian midpoint `0.5 * (max_high(period) + min_low(period))`
/// using dual monotonic deques.
///
/// - Warmup: first valid point at index `period - 1` (`0` when `period == 1`).
/// - Returns `vec![None; candles.len()]` when `period == 0`, `candles.is_empty()`, or `period > candles.len()`.
/// - Recovers cleanly after any non-finite or inverted (`high < low`) candle.
#[must_use]
pub fn donchian_midpoint_series(candles: &[Candle], period: usize) -> Vec<Option<f64>> {
    let n = candles.len();
    let mut out = vec![None; n];
    if period == 0 || period > n {
        return out;
    }

    if period == 1 {
        for (dst, c) in out.iter_mut().zip(candles.iter()) {
            if c.high.is_finite() && c.low.is_finite() && c.high >= c.low {
                *dst = safe_midpoint_two(c.low, c.high);
            }
        }
        return out;
    }

    let mut max_q: VecDeque<usize> = VecDeque::with_capacity(period.min(256));
    let mut min_q: VecDeque<usize> = VecDeque::with_capacity(period.min(256));
    let mut finite_run = 0_usize;

    for (i, c) in candles.iter().enumerate() {
        if !c.high.is_finite() || !c.low.is_finite() || c.high < c.low {
            max_q.clear();
            min_q.clear();
            finite_run = 0;
            continue;
        }

        finite_run += 1;
        let window_start = i.saturating_sub(period - 1);

        while max_q.front().is_some_and(|&idx| idx < window_start) {
            max_q.pop_front();
        }
        while min_q.front().is_some_and(|&idx| idx < window_start) {
            min_q.pop_front();
        }

        while max_q.back().is_some_and(|&idx| candles[idx].high <= c.high) {
            max_q.pop_back();
        }
        max_q.push_back(i);

        while min_q.back().is_some_and(|&idx| candles[idx].low >= c.low) {
            min_q.pop_back();
        }
        min_q.push_back(i);

        if finite_run >= period
            && let (Some(&hi_idx), Some(&lo_idx)) = (max_q.front(), min_q.front())
        {
            let hi = candles[hi_idx].high;
            let lo = candles[lo_idx].low;
            out[i] = safe_midpoint_two(lo, hi);
        }
    }

    out
}

#[inline]
fn safe_midpoint_two(a: f64, b: f64) -> Option<f64> {
    if !a.is_finite() || !b.is_finite() {
        return None;
    }
    let lo = a.min(b);
    let hi = a.max(b);
    let mid = lo + 0.5 * (hi - lo);
    if mid.is_finite() {
        Some(mid)
    } else {
        let half_sum = 0.5 * a + 0.5 * b;
        half_sum.is_finite().then_some(half_sum)
    }
}

/// Computes complete causal Ichimoku Kinko Hyo series (`Tenkan`, `Kijun`, `Senkou A/B` + `Chikou`).
///
/// Zero-period fields produce `None` on that span rather than panicking, and `displacement` is bounded
/// to `MAX_ICHIMOKU_DISPLACEMENT` (`250`).
#[must_use]
pub fn compute_ichimoku(candles: &[Candle], config: IchimokuConfig) -> IchimokuOutput {
    let disp = config.displacement.min(MAX_ICHIMOKU_DISPLACEMENT);
    let n = candles.len();
    if n == 0 {
        return IchimokuOutput {
            tenkan: Vec::new(),
            kijun: Vec::new(),
            senkou_a: Vec::new(),
            senkou_b: Vec::new(),
            chikou: Vec::new(),
            displacement: disp,
        };
    }

    let tenkan = donchian_midpoint_series(candles, config.tenkan_period);
    let kijun = donchian_midpoint_series(candles, config.kijun_period);
    let raw_senkou_b = donchian_midpoint_series(candles, config.senkou_b_period);

    let cloud_len = n.saturating_add(disp);
    let mut senkou_a = vec![None; cloud_len];
    let mut senkou_b = vec![None; cloud_len];

    for i in 0..n {
        let dst = i + disp;
        if let (Some(t), Some(k)) = (tenkan[i], kijun[i]) {
            senkou_a[dst] = safe_midpoint_two(t, k);
        }
        if let Some(sb) = raw_senkou_b[i] {
            senkou_b[dst] = Some(sb);
        }
    }

    let mut chikou = vec![None; n];
    if config.show_chikou {
        let limit = n.saturating_sub(disp);
        for (k, dst) in chikou[..limit].iter_mut().enumerate() {
            let future_close = candles[k + disp].close;
            if future_close.is_finite() {
                *dst = Some(future_close);
            }
        }
    }

    IchimokuOutput {
        tenkan,
        kijun,
        senkou_a,
        senkou_b,
        chikou,
        displacement: disp,
    }
}

/// Trait adapter exposing Ichimoku spans over the `0..candles.len()` bar range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IchimokuIndicator {
    pub config: IchimokuConfig,
}

impl IchimokuIndicator {
    #[must_use]
    pub fn new(config: IchimokuConfig) -> Self {
        Self {
            config: config.sanitized(),
        }
    }
}

impl Default for IchimokuIndicator {
    fn default() -> Self {
        Self::new(IchimokuConfig::default())
    }
}

impl Indicator for IchimokuIndicator {
    fn id(&self) -> &'static str {
        "ichimoku"
    }

    fn title(&self) -> String {
        format!(
            "Ichimoku ({}, {}, {}, {})",
            self.config.tenkan_period,
            self.config.kijun_period,
            self.config.senkou_b_period,
            self.config.displacement
        )
    }

    fn placement(&self) -> IndicatorPlacement {
        IndicatorPlacement::PriceOverlay
    }

    fn warmup_bars(&self) -> usize {
        self.config
            .tenkan_period
            .max(self.config.kijun_period)
            .max(self.config.senkou_b_period)
            .saturating_sub(1)
    }

    fn evaluate(&self, candles: &[Candle]) -> IndicatorOutput {
        let out = compute_ichimoku(candles, self.config);
        let n = candles.len();
        IndicatorOutput {
            id: self.id(),
            title: self.title(),
            placement: self.placement(),
            series: vec![
                IndicatorSeriesOutput {
                    key: "tenkan",
                    label: format!("Tenkan ({})", self.config.tenkan_period),
                    kind: SeriesRenderKind::Line,
                    values: out.tenkan,
                },
                IndicatorSeriesOutput {
                    key: "kijun",
                    label: format!("Kijun ({})", self.config.kijun_period),
                    kind: SeriesRenderKind::Line,
                    values: out.kijun,
                },
                IndicatorSeriesOutput {
                    key: "senkou_a",
                    label: "Senkou A".to_owned(),
                    kind: SeriesRenderKind::Line,
                    values: out.senkou_a.into_iter().take(n).collect(),
                },
                IndicatorSeriesOutput {
                    key: "senkou_b",
                    label: format!("Senkou B ({})", self.config.senkou_b_period),
                    kind: SeriesRenderKind::Line,
                    values: out.senkou_b.into_iter().take(n).collect(),
                },
                IndicatorSeriesOutput {
                    key: "chikou",
                    label: format!("Chikou ({})", self.config.displacement),
                    kind: SeriesRenderKind::Line,
                    values: out.chikou,
                },
            ],
            reference_levels: Vec::new(),
            fixed_bounds_hint: None,
        }
    }
}
