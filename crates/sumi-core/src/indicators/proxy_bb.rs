//! Pure `f64`, `wasm32-unknown-unknown`-compatible Multi-Horizon Money Flow Blackbox (`ProxyBB`)
//! Engine (`Milestone R18 / Wave 17` — `B4`).
//!
//! Implements the `bb_v1_ohlcv_proxy` rolling true-range order-imbalance proxy across `6` horizons
//! (`T03`, `T05`, `T10`, `T20`, `T50`, `T200`) strictly enforcing:
//! - `AT01`: Bounds `oib_raw in [-1.0, +1.0]`, `bb_value in [0.0, 100.0]`.
//! - `AT05`: Zero rolling trading value yields `bb_value = None` (never silently `50.0`!).
//! - `AT06` / `AT07`: Strictly causal rolling `[t + 1 - H ..= t]` window with future invariance.
//! - `AT08`: Warmup `t + 1 < H` yields `is_warmup = true` and `bb_value = None`.
//! - `AT15`: True-range span `max(H, prev_C) - min(L, prev_C)` with scale-relative flat-bar `0.0` pressure.
//! - `AT18`: Causal `BBDirection` (`±1e-7` hysteresis vs prior valid score) and `BBRegime` (`50.0 ± 1e-7`).

use serde::{Deserialize, Serialize};

use crate::model::Candle;

use super::ReferenceLevel;

/// Canonical methodology tag emitted for OHLCV-derived Money Flow Blackbox series (`AT13`).
pub const PROXY_BB_METHODOLOGY_VERSION: &str = "bb_v1_ohlcv_proxy";

/// Reference guides for [`OscillatorPaneId::ProxyBb`] (`[30, 50, 70]`).
pub const PROXY_BB_REFERENCE_LEVELS: [ReferenceLevel; 3] = [
    ReferenceLevel {
        value: 30.0,
        label: "30",
    },
    ReferenceLevel {
        value: 50.0,
        label: "50",
    },
    ReferenceLevel {
        value: 70.0,
        label: "70",
    },
];

/// Fixed bounds hint for `ProxyBB` (`[0, 100]`).
pub const PROXY_BB_FIXED_BOUNDS_HINT: (f64, f64) = (0.0, 100.0);

/// Canonical `6` rolling horizons of Money Flow `ProxyBB`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum BBHorizon {
    T03,
    T05,
    T10,
    #[default]
    T20,
    T50,
    T200,
}

impl BBHorizon {
    pub const ALL: [Self; 6] = [
        Self::T03,
        Self::T05,
        Self::T10,
        Self::T20,
        Self::T50,
        Self::T200,
    ];

    #[inline]
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::T03 => 0,
            Self::T05 => 1,
            Self::T10 => 2,
            Self::T20 => 3,
            Self::T50 => 4,
            Self::T200 => 5,
        }
    }

    #[inline]
    #[must_use]
    pub const fn lookback_bars(self) -> usize {
        match self {
            Self::T03 => 3,
            Self::T05 => 5,
            Self::T10 => 10,
            Self::T20 => 20,
            Self::T50 => 50,
            Self::T200 => 200,
        }
    }

    #[inline]
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::T03 => "T03",
            Self::T05 => "T05",
            Self::T10 => "T10",
            Self::T20 => "T20",
            Self::T50 => "T50",
            Self::T200 => "T200",
        }
    }

    #[must_use]
    pub fn from_lookback(bars: usize) -> Self {
        match bars {
            0..=3 => Self::T03,
            4..=5 => Self::T05,
            6..=10 => Self::T10,
            11..=20 => Self::T20,
            21..=50 => Self::T50,
            _ => Self::T200,
        }
    }

    #[must_use]
    pub fn parse_str(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_uppercase().as_str() {
            "T03" | "3" => Some(Self::T03),
            "T05" | "5" => Some(Self::T05),
            "T10" | "10" => Some(Self::T10),
            "T20" | "20" => Some(Self::T20),
            "T50" | "50" => Some(Self::T50),
            "T200" | "200" => Some(Self::T200),
            _ => None,
        }
    }
}

/// Bar-over-bar directional trajectory of a `ProxyBB` horizon score (`AT18`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum BBDirection {
    Rising,
    Falling,
    Flat,
    #[default]
    Unknown,
}

impl BBDirection {
    #[inline]
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Rising => "RISING",
            Self::Falling => "FALLING",
            Self::Flat => "FLAT",
            Self::Unknown => "UNKNOWN",
        }
    }

    #[inline]
    #[must_use]
    pub const fn arrow(self) -> &'static str {
        match self {
            Self::Rising => "↑",
            Self::Falling => "↓",
            Self::Flat => "→",
            Self::Unknown => "·",
        }
    }
}

/// Equilibrium regime around `50.0` (`Positive > 50`, `Negative < 50`, `Neutral == 50`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum BBRegime {
    Positive,
    Negative,
    Neutral,
    #[default]
    Unknown,
}

impl BBRegime {
    #[inline]
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Positive => "POSITIVE",
            Self::Negative => "NEGATIVE",
            Self::Neutral => "NEUTRAL",
            Self::Unknown => "UNKNOWN",
        }
    }
}

/// Single-bar single-horizon evaluation of `ProxyBB` (`0` heap allocations).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct ProxyBBHorizonPoint {
    pub horizon: BBHorizon,
    /// Rolling `ProxyBB` score in `[0.0, 100.0]`, or `None` during warmup (`AT08`), zero liquidity (`AT05`), or invalid bar data.
    pub bb_value: Option<f64>,
    /// Normalized order-imbalance proxy in `[-1.0, +1.0]`, or `None` when `bb_value` is `None`.
    pub oib_raw: Option<f64>,
    pub raw_numerator: f64,
    pub raw_denominator: f64,
    pub is_warmup: bool,
    pub is_zero_activity: bool,
    pub is_invalid: bool,
    pub direction: BBDirection,
    pub regime: BBRegime,
    pub regime_run_length: u32,
}

/// Complete 6-horizon `ProxyBB` evaluation on one bar `t`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ProxyBBBarPoint {
    pub bar_index: usize,
    pub timestamp: i64,
    pub daily_pressure: f64,
    pub daily_buy_value: f64,
    pub daily_sell_value: f64,
    pub daily_trading_value: f64,
    pub is_bar_valid: bool,
    pub horizons: [ProxyBBHorizonPoint; 6],
}

impl ProxyBBBarPoint {
    #[inline]
    #[must_use]
    pub const fn horizon(&self, h: BBHorizon) -> ProxyBBHorizonPoint {
        self.horizons[h.index()]
    }
}

/// Decomposed single-bar `OHLCV_PROXY` values (`buy_value`, `sell_value`, `trading_value`, `pressure`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DecomposedBarFlow {
    pub buy_value: f64,
    pub sell_value: f64,
    pub trading_value: f64,
    pub signed_pressure: f64,
    pub pressure: f64,
    pub is_valid: bool,
}

/// Decomposes a single [`Candle`] into active `buy_value` and `sell_value` under `OHLCV_PROXY`
/// (`buy + sell == trading_value`, `buy - sell == pressure * trading_value`, `AT15`).
#[inline]
#[must_use]
pub fn decompose_bar_ohlcv_proxy(bar: &Candle, prev_close: Option<f64>) -> DecomposedBarFlow {
    let valid_ohlcv = bar.open.is_finite()
        && bar.high.is_finite()
        && bar.low.is_finite()
        && bar.close.is_finite()
        && bar.volume.is_finite()
        && bar.high >= bar.low
        && bar.volume >= 0.0;
    if !valid_ohlcv {
        return DecomposedBarFlow {
            buy_value: 0.0,
            sell_value: 0.0,
            trading_value: 0.0,
            signed_pressure: 0.0,
            pressure: 0.0,
            is_valid: false,
        };
    }

    let (true_high, true_low) = match prev_close.filter(|p| p.is_finite()) {
        Some(pc) => (bar.high.max(pc), bar.low.min(pc)),
        None => (bar.high, bar.low),
    };
    let tr_span = true_high - true_low;
    let scale_eps = 1.0e-12 * (true_high.abs() + true_low.abs()).max(1.0e-30);
    let pressure = if tr_span <= scale_eps {
        0.0
    } else {
        ((2.0 * bar.close - true_high - true_low) / tr_span).clamp(-1.0, 1.0)
    };

    let tp = ((bar.high + bar.low + bar.close) / 3.0).max(0.0);
    let trading_value = if bar.volume > 0.0 && tp > 0.0 {
        let v = tp * bar.volume;
        if v.is_finite() { v } else { 0.0 }
    } else {
        0.0
    };
    if !(tp * bar.volume).is_finite() && bar.volume > 0.0 {
        return DecomposedBarFlow {
            buy_value: 0.0,
            sell_value: 0.0,
            trading_value: 0.0,
            signed_pressure: 0.0,
            pressure: 0.0,
            is_valid: false,
        };
    }

    let buy_value = 0.5 * (1.0 + pressure) * trading_value;
    let sell_value = 0.5 * (1.0 - pressure) * trading_value;
    let signed_pressure = pressure * trading_value;

    DecomposedBarFlow {
        buy_value,
        sell_value,
        trading_value,
        signed_pressure,
        pressure,
        is_valid: true,
    }
}

/// Complete multi-horizon `ProxyBB` output cached in [`super::IndicatorCache`] and consumed by UI / Scanner.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProxyBBSeriesOutput {
    pub points: Vec<ProxyBBBarPoint>,
    pub t05_values: Vec<Option<f64>>,
    pub t20_values: Vec<Option<f64>>,
    pub t50_values: Vec<Option<f64>>,
}

/// Computes all `6` horizons (`T03, T05, T10, T20, T50, T200`) across `candles` in $O(N)$ linear time
/// with `zero_val_run` + periodic exact resummation so `AT05` zero-liquidity windows never suffer
/// floating-point cancellation drift even across `100,000+` bars.
#[must_use]
pub fn compute_proxy_bb_series(candles: &[Candle]) -> ProxyBBSeriesOutput {
    let n = candles.len();
    if n == 0 {
        return ProxyBBSeriesOutput::default();
    }

    let mut decomposed = Vec::with_capacity(n);
    let mut prev_valid_close: Option<f64> = None;
    for c in candles {
        let dec = decompose_bar_ohlcv_proxy(c, prev_valid_close);
        if dec.is_valid {
            prev_valid_close = Some(c.close);
        } else {
            prev_valid_close = None;
        }
        decomposed.push(dec);
    }

    #[derive(Clone, Copy, Default)]
    struct HorizonRollingState {
        sp_sum: f64,
        val_sum: f64,
        zero_val_run: usize,
        invalid_count: usize,
        prev_score: Option<f64>,
        prev_regime: BBRegime,
        run_len: u32,
    }

    let mut states = [HorizonRollingState::default(); 6];
    let mut points = Vec::with_capacity(n);
    let mut t05_values = Vec::with_capacity(n);
    let mut t20_values = Vec::with_capacity(n);
    let mut t50_values = Vec::with_capacity(n);

    for t in 0..n {
        let dec = decomposed[t];
        let mut h_pts = [ProxyBBHorizonPoint::default(); 6];

        for h in BBHorizon::ALL {
            let hi = h.index();
            let lookback = h.lookback_bars();
            let st = &mut states[hi];

            if dec.is_valid {
                st.sp_sum += dec.signed_pressure;
                st.val_sum += dec.trading_value;
                if dec.trading_value <= 0.0 {
                    st.zero_val_run = st.zero_val_run.saturating_add(1);
                } else {
                    st.zero_val_run = 0;
                }
            } else {
                st.invalid_count += 1;
                st.zero_val_run = 0;
            }

            if t >= lookback {
                let old = decomposed[t - lookback];
                if old.is_valid {
                    st.sp_sum -= old.signed_pressure;
                    st.val_sum -= old.trading_value;
                } else {
                    st.invalid_count = st.invalid_count.saturating_sub(1);
                }
            }

            if t + 1 < lookback {
                h_pts[hi] = ProxyBBHorizonPoint {
                    horizon: h,
                    bb_value: None,
                    oib_raw: None,
                    raw_numerator: 0.0,
                    raw_denominator: 0.0,
                    is_warmup: true,
                    is_zero_activity: false,
                    is_invalid: false,
                    direction: BBDirection::Unknown,
                    regime: BBRegime::Unknown,
                    regime_run_length: 0,
                };
                continue;
            }

            if st.invalid_count > 0 {
                st.prev_score = None;
                st.prev_regime = BBRegime::Unknown;
                st.run_len = 0;
                h_pts[hi] = ProxyBBHorizonPoint {
                    horizon: h,
                    bb_value: None,
                    oib_raw: None,
                    raw_numerator: 0.0,
                    raw_denominator: 0.0,
                    is_warmup: false,
                    is_zero_activity: false,
                    is_invalid: true,
                    direction: BBDirection::Unknown,
                    regime: BBRegime::Unknown,
                    regime_run_length: 0,
                };
                continue;
            }

            // Periodic exact resummation prevents drift on 100k+ bars (`H <= 200`, amortized < 1 op/bar).
            if st.zero_val_run >= lookback {
                st.sp_sum = 0.0;
                st.val_sum = 0.0;
            } else if t.is_multiple_of(256) || st.val_sum < 0.0 {
                let win = &decomposed[t + 1 - lookback..=t];
                st.sp_sum = win.iter().map(|d| d.signed_pressure).sum();
                st.val_sum = win.iter().map(|d| d.trading_value).sum();
            }

            // AT05: Zero activity yields `bb_value = None` (never silently 50.0!).
            if st.val_sum <= 0.0 {
                st.prev_score = None;
                st.prev_regime = BBRegime::Neutral;
                st.run_len = 0;
                h_pts[hi] = ProxyBBHorizonPoint {
                    horizon: h,
                    bb_value: None,
                    oib_raw: None,
                    raw_numerator: st.sp_sum,
                    raw_denominator: 0.0,
                    is_warmup: false,
                    is_zero_activity: true,
                    is_invalid: false,
                    direction: BBDirection::Unknown,
                    regime: BBRegime::Neutral,
                    regime_run_length: 0,
                };
                continue;
            }

            let oib_raw = (st.sp_sum / st.val_sum).clamp(-1.0, 1.0);
            let bb_val = (50.0 * (1.0 + oib_raw)).clamp(0.0, 100.0);

            let direction = match st.prev_score {
                None => BBDirection::Unknown,
                Some(prev) if bb_val > prev + 1.0e-7 => BBDirection::Rising,
                Some(prev) if bb_val < prev - 1.0e-7 => BBDirection::Falling,
                Some(_) => BBDirection::Flat,
            };
            let regime = if bb_val > 50.0 + 1.0e-7 {
                BBRegime::Positive
            } else if bb_val < 50.0 - 1.0e-7 {
                BBRegime::Negative
            } else {
                BBRegime::Neutral
            };
            let run_len = if regime == st.prev_regime {
                st.run_len.saturating_add(1)
            } else {
                1
            };

            st.prev_score = Some(bb_val);
            st.prev_regime = regime;
            st.run_len = run_len;

            h_pts[hi] = ProxyBBHorizonPoint {
                horizon: h,
                bb_value: Some(bb_val),
                oib_raw: Some(oib_raw),
                raw_numerator: st.sp_sum,
                raw_denominator: st.val_sum,
                is_warmup: false,
                is_zero_activity: false,
                is_invalid: false,
                direction,
                regime,
                regime_run_length: run_len,
            };
        }

        t05_values.push(h_pts[BBHorizon::T05.index()].bb_value);
        t20_values.push(h_pts[BBHorizon::T20.index()].bb_value);
        t50_values.push(h_pts[BBHorizon::T50.index()].bb_value);
        points.push(ProxyBBBarPoint {
            bar_index: t,
            timestamp: candles[t].timestamp,
            daily_pressure: dec.pressure,
            daily_buy_value: dec.buy_value,
            daily_sell_value: dec.sell_value,
            daily_trading_value: dec.trading_value,
            is_bar_valid: dec.is_valid,
            horizons: h_pts,
        });
    }

    ProxyBBSeriesOutput {
        points,
        t05_values,
        t20_values,
        t50_values,
    }
}
