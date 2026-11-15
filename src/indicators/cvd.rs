//! Pure `f64`, `wasm32-unknown-unknown`-compatible Cumulative Volume Delta (`CVD`) Engine.
//!
//! Derives intrabar signed order flow from OHLCV candles using the exact `OHLCV_PROXY` conservation
//! identity (`buy + sell == volume`, `delta = buy - sell`), unlike `VolumeProfile` falling back to
//! `0.0` delta on zero-volume bars (never inventing synthetic order flow from thin air).
//!
//! Supports two reset regimes:
//! - [`CvdResetMode::Session`]: resets cumulative delta and signal average at each exchange session
//!   boundary (`UTC+7` HOSE/VN30F midnight on intraday, month on `1D`, year on `1W` via [`session_bucket_key`]).
//! - [`CvdResetMode::Cumulative`]: accumulates continuously across the entire loaded series.

use serde::{Deserialize, Serialize};

use crate::data::Timeframe;
use crate::model::Candle;

use super::ReferenceLevel;
use super::volume_profile::intrabar_buy_fraction;
use super::vwap::session_bucket_key;

/// Horizontal zero-balance reference line inside the `CVD` oscillator sub-pane.
pub const CVD_REFERENCE_LEVELS: [ReferenceLevel; 1] = [ReferenceLevel {
    value: 0.0,
    label: "0",
}];

/// Accumulation regime for [`compute_cvd`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum CvdResetMode {
    /// Reset cumulative delta back to `0` at every exchange session boundary (`session_bucket_key`).
    #[default]
    Session,
    /// Continuous lifetime accumulation across all visible/loaded bars.
    Cumulative,
}

impl CvdResetMode {
    #[inline]
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::Session => "Session",
            Self::Cumulative => "Cumul",
        }
    }
}

/// Configuration for Cumulative Volume Delta evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CvdConfig {
    pub reset_mode: CvdResetMode,
    pub timeframe: Timeframe,
    pub utc_offset_seconds: i64,
    pub sma_period: usize,
}

impl Default for CvdConfig {
    fn default() -> Self {
        Self {
            reset_mode: CvdResetMode::Session,
            timeframe: Timeframe::D1,
            utc_offset_seconds: 0,
            sma_period: 9,
        }
    }
}

impl CvdConfig {
    #[must_use]
    pub fn sanitized(self) -> Self {
        Self {
            reset_mode: self.reset_mode,
            timeframe: self.timeframe,
            utc_offset_seconds: self.utc_offset_seconds.clamp(-50_400, 50_400),
            sma_period: self.sma_period.clamp(1, 500),
        }
    }
}

/// Complete Cumulative Volume Delta output aligned 1-for-1 with `candles`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CvdOutput {
    /// Bar-level `OHLCV_PROXY` signed volume delta (`buy_volume - sell_volume`).
    pub delta: Vec<Option<f64>>,
    /// Running cumulative sum of `delta` (resetting per [`CvdResetMode::Session`] bucket when configured).
    pub cvd: Vec<Option<f64>>,
    /// Rolling simple moving average of `cvd` (reseeded per session to avoid cross-reset jumps).
    pub cvd_sma: Vec<Option<f64>>,
    /// Strictly ascending indices where a new accumulation segment/session starts.
    pub session_starts: Vec<usize>,
}

/// Computes signed bar volume delta (`buy - sell`) for one candle using `OHLCV_PROXY` attribution.
///
/// Returns `None` for non-finite/negative volume or inverted `high < low` bars, and `Some(0.0)`
/// when `volume == 0.0` so zero-volume feeds never fabricate order-flow pressure.
#[inline]
#[must_use]
pub fn compute_bar_volume_delta(candle: &Candle) -> Option<f64> {
    if !candle.open.is_finite()
        || !candle.high.is_finite()
        || !candle.low.is_finite()
        || !candle.close.is_finite()
        || !candle.volume.is_finite()
        || candle.volume < 0.0
        || candle.high < candle.low
    {
        return None;
    }
    if candle.volume == 0.0 {
        return Some(0.0);
    }
    let buy_frac = intrabar_buy_fraction(candle);
    let buy_vol = candle.volume * buy_frac;
    let sell_vol = candle.volume - buy_vol;
    let delta = buy_vol - sell_vol;
    if delta.is_finite() { Some(delta) } else { None }
}

/// Computes `OHLCV_PROXY` bar delta, Cumulative Volume Delta (`CVD`), and session-isolated `CVD SMA`.
#[must_use]
pub fn compute_cvd(candles: &[Candle], config: CvdConfig) -> CvdOutput {
    let n = candles.len();
    if n == 0 {
        return CvdOutput::default();
    }

    let cfg = config.sanitized();
    let sma_p = cfg.sma_period;

    let mut delta = vec![None; n];
    let mut cvd = vec![None; n];
    let mut cvd_sma = vec![None; n];
    let mut session_starts = Vec::new();

    let mut current_bucket: Option<i64> = None;
    let mut running_cvd = 0.0_f64;
    let mut sma_run = 0_usize;
    let mut sma_sum = 0.0_f64;

    for (i, c) in candles.iter().enumerate() {
        let reset_here = match cfg.reset_mode {
            CvdResetMode::Session => {
                let b = session_bucket_key(c.timestamp, cfg.timeframe, cfg.utc_offset_seconds);
                if current_bucket != Some(b) {
                    current_bucket = Some(b);
                    session_starts.push(i);
                    true
                } else {
                    false
                }
            }
            CvdResetMode::Cumulative => {
                if i == 0 {
                    session_starts.push(0);
                    true
                } else {
                    false
                }
            }
        };

        if reset_here {
            running_cvd = 0.0;
            sma_run = 0;
            sma_sum = 0.0;
        }

        let Some(d) = compute_bar_volume_delta(c) else {
            sma_run = 0;
            sma_sum = 0.0;
            continue;
        };

        let next_cvd = running_cvd + d;
        if !next_cvd.is_finite() {
            sma_run = 0;
            sma_sum = 0.0;
            continue;
        }
        running_cvd = next_cvd;
        delta[i] = Some(d);
        cvd[i] = Some(running_cvd);

        sma_sum += running_cvd;
        sma_run += 1;
        if sma_run > sma_p {
            if let Some(leaving) = cvd[i - sma_p].filter(|v| v.is_finite()) {
                sma_sum -= leaving;
            }
            sma_run -= 1;
        }
        if sma_run == sma_p {
            if i.is_multiple_of(64) || !sma_sum.is_finite() {
                sma_sum = cvd[i + 1 - sma_p..=i]
                    .iter()
                    .filter_map(|v| v.filter(|x| x.is_finite()))
                    .sum();
            }
            let mean = if sma_sum.is_finite() {
                sma_sum / (sma_p as f64)
            } else {
                cvd[i + 1 - sma_p..=i]
                    .iter()
                    .filter_map(|v| v.filter(|x| x.is_finite()))
                    .map(|v| v / (sma_p as f64))
                    .sum()
            };
            if mean.is_finite() {
                cvd_sma[i] = Some(mean);
            }
        }
    }

    CvdOutput {
        delta,
        cvd,
        cvd_sma,
        session_starts,
    }
}
