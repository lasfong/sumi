//! Pure `f64`, `wasm32-unknown-unknown`-compatible Session `VWAP` and `Anchored VWAP (AVWAP)`
//! with numerically stable one-pass volume-weighted Welford $\pm 1\sigma$ and $\pm 2\sigma$ bands.
//!
//! Session boundaries follow the venue's local exchange timezone (`UTC+7` ICT for Vietnam
//! `HOSE`/`HNX`/`VN30F`, `UTC-5` EST for US Equities, `UTC` for 24/7 Crypto):
//! - Intraday (`1m`, `5m`, `15m`, `1h`, `4h`): resets at local civil midnight (`00:00`).
//! - Daily (`1D`): resets at local calendar month boundary (`MTD VWAP`).
//! - Weekly (`1W`): resets at local calendar year boundary (`YTD VWAP`).

use crate::data::{Timeframe, civil_from_days, weekday_from_days};
use crate::model::Candle;

/// Anchor reset regime for `VWAP` bands.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
pub enum VwapAnchorMode {
    /// Automatic trading-session / timeframe boundary (`session_bucket_key`).
    #[default]
    AutoSession,
    /// Calendar day boundary in the venue's timezone (`YYYY-MM-DD`).
    Daily,
    /// ISO Monday `00:00` weekly boundary in the venue's timezone.
    Weekly,
    /// Calendar month boundary (`YYYY-MM`).
    Monthly,
    /// Calendar year boundary (`YYYY`).
    Yearly,
}

impl VwapAnchorMode {
    #[inline]
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::AutoSession => "Session",
            Self::Daily => "Daily",
            Self::Weekly => "Weekly",
            Self::Monthly => "Monthly",
            Self::Yearly => "Yearly",
        }
    }

    #[inline]
    #[must_use]
    pub const fn cycle_next(self) -> Self {
        match self {
            Self::AutoSession => Self::Daily,
            Self::Daily => Self::Weekly,
            Self::Weekly => Self::Monthly,
            Self::Monthly => Self::Yearly,
            Self::Yearly => Self::AutoSession,
        }
    }
}

/// Computes the deterministic reset bucket for `(timestamp, timeframe, utc_offset_seconds, anchor_mode)`.
#[inline]
#[must_use]
pub fn vwap_bucket_key(
    timestamp: i64,
    timeframe: Timeframe,
    utc_offset_seconds: i64,
    anchor_mode: VwapAnchorMode,
) -> i64 {
    // Daily and Weekly feeds normalize candle timestamps to `00:00:00 UTC`; applying negative
    // exchange UTC offsets (`EST = -18_000`) to `00:00 UTC` would shift civil dates backward by
    // one calendar day across Monday, month-start, and Jan-1 boundaries.
    let effective_off = match timeframe {
        Timeframe::D1 | Timeframe::W1 | Timeframe::MN1 => 0,
        Timeframe::M1
        | Timeframe::M5
        | Timeframe::M15
        | Timeframe::M30
        | Timeframe::H1
        | Timeframe::H4 => utc_offset_seconds.clamp(-50_400, 50_400),
    };
    match anchor_mode {
        VwapAnchorMode::AutoSession => session_bucket_key(timestamp, timeframe, effective_off),
        VwapAnchorMode::Daily => timestamp.saturating_add(effective_off).div_euclid(86_400),
        VwapAnchorMode::Weekly => {
            let days = timestamp.saturating_add(effective_off).div_euclid(86_400);
            days - i64::from(weekday_from_days(days))
        }
        VwapAnchorMode::Monthly => {
            let days = timestamp.saturating_add(effective_off).div_euclid(86_400);
            let (year, month, _) = civil_from_days(days);
            i64::from(year).saturating_mul(16) + i64::from(month)
        }
        VwapAnchorMode::Yearly => {
            let days = timestamp.saturating_add(effective_off).div_euclid(86_400);
            let (year, _, _) = civil_from_days(days);
            i64::from(year)
        }
    }
}

/// Computed `VWAP` center line, volume-weighted standard-deviation bands, and session reset indices.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct VwapBandsOutput {
    pub vwap: Vec<Option<f64>>,
    pub upper_1: Vec<Option<f64>>,
    pub lower_1: Vec<Option<f64>>,
    pub upper_2: Vec<Option<f64>>,
    pub lower_2: Vec<Option<f64>>,
    /// Strictly ascending bar indices where each new VWAP session/anchor begins.
    pub session_starts: Vec<usize>,
}

/// Computes the deterministic session bucket identifier for `(timestamp, timeframe, utc_offset_seconds)`.
///
/// Intraday timeframes shift by `utc_offset_seconds` so exchange midnight (`00:00`) aligns in venue local time.
/// Daily (`1D`) and Weekly (`1W`) bars are normalized to civil `00:00:00 UTC`, so negative timezones (`EST`)
/// must NOT shift timestamps backward into the prior month/year.
#[inline]
#[must_use]
pub fn session_bucket_key(timestamp: i64, timeframe: Timeframe, utc_offset_seconds: i64) -> i64 {
    match timeframe {
        Timeframe::M1
        | Timeframe::M5
        | Timeframe::M15
        | Timeframe::M30
        | Timeframe::H1
        | Timeframe::H4 => {
            let local_ts = timestamp.saturating_add(utc_offset_seconds);
            local_ts.div_euclid(86_400)
        }
        Timeframe::D1 => {
            let days = timestamp.div_euclid(86_400);
            let (year, month, _) = civil_from_days(days);
            i64::from(year).saturating_mul(16) + i64::from(month)
        }
        Timeframe::W1 | Timeframe::MN1 => {
            let days = timestamp.div_euclid(86_400);
            let (year, _, _) = civil_from_days(days);
            i64::from(year)
        }
    }
}

#[inline]
fn normalize_band_multipliers(mult_1: f64, mult_2: f64) -> (f64, f64) {
    let m1 = if mult_1.is_finite() {
        mult_1.abs()
    } else {
        1.0
    };
    let m2 = if mult_2.is_finite() {
        mult_2.abs()
    } else {
        2.0
    };
    if m1 <= m2 { (m1, m2) } else { (m2, m1) }
}

#[inline]
fn safe_typical_price(c: &Candle) -> f64 {
    c.high / 3.0 + c.low / 3.0 + c.close / 3.0
}

#[derive(Clone, Copy)]
struct WelfordVwapAcc {
    cum_w: f64,
    mean: f64,
    m2_scaled: f64,
    inv_w_scale: f64,
    dev_scale: f64,
}

impl WelfordVwapAcc {
    #[inline]
    const fn new() -> Self {
        Self {
            cum_w: 0.0,
            mean: 0.0,
            m2_scaled: 0.0,
            inv_w_scale: 1.0,
            dev_scale: 1.0,
        }
    }

    #[inline]
    fn push(&mut self, tp: f64, raw_w: f64) {
        if !(raw_w > 0.0 && raw_w.is_finite() && tp.is_finite()) {
            return;
        }
        let mut w = raw_w * self.inv_w_scale;
        if w > 1.0e150 || self.cum_w + w > 1.0e150 || !(self.cum_w + w).is_finite() {
            self.cum_w *= 1.0e-150;
            self.m2_scaled *= 1.0e-150;
            self.inv_w_scale *= 1.0e-150;
            w = raw_w * self.inv_w_scale;
        }
        if !(w > 0.0 && w.is_finite()) {
            return;
        }
        let new_cum_w = self.cum_w + w;
        if !(new_cum_w > 0.0 && new_cum_w.is_finite()) {
            return;
        }
        let prev_mean = self.mean;
        let dev_prev = tp - prev_mean;
        if !dev_prev.is_finite() {
            return;
        }
        // Rescale deviation exponent before squaring so prices up to `~1e308` never overflow `m2_scaled`
        // and collapse via `Inf / Inf = NaN`.
        let needed_scale = (dev_prev.abs() * 1.0e-100).max(self.dev_scale);
        if needed_scale > self.dev_scale {
            let r = self.dev_scale / needed_scale;
            self.m2_scaled *= r * r;
            self.dev_scale = needed_scale;
        }
        let inv_s = 1.0 / self.dev_scale;
        let new_mean = prev_mean + (w / new_cum_w) * dev_prev;
        if !new_mean.is_finite() {
            return;
        }
        let dev_curr = tp - new_mean;
        if !dev_curr.is_finite() {
            return;
        }
        let d1 = dev_prev * inv_s;
        let d2 = dev_curr * inv_s;
        let inc = w * d1 * d2;
        if !inc.is_finite() || !(self.m2_scaled + inc).is_finite() {
            let extra_scale = self.dev_scale * 1.0e75;
            if extra_scale.is_finite() {
                self.m2_scaled *= 1.0e-150;
                self.dev_scale = extra_scale;
                let inv2 = 1.0 / self.dev_scale;
                let inc2 = w * (dev_prev * inv2) * (dev_curr * inv2);
                if inc2.is_finite() && (self.m2_scaled + inc2).is_finite() {
                    self.m2_scaled += inc2;
                }
            }
        } else {
            self.m2_scaled += inc;
        }
        self.cum_w = new_cum_w;
        self.mean = new_mean;
    }

    #[inline]
    fn write_point(&self, out: &mut VwapBandsOutput, idx: usize, band_1: f64, band_2: f64) {
        if !(self.cum_w > 0.0 && self.mean.is_finite()) {
            return;
        }
        let ratio = self.m2_scaled / self.cum_w;
        if !ratio.is_finite() {
            return;
        }
        let sigma = self.dev_scale * ratio.max(0.0).sqrt();
        if !sigma.is_finite() {
            return;
        }
        out.vwap[idx] = Some(self.mean);
        let b1 = band_1 * sigma;
        let b2 = band_2 * sigma;
        if b1.is_finite() {
            let u1 = self.mean + b1;
            let l1 = self.mean - b1;
            if u1.is_finite() {
                out.upper_1[idx] = Some(u1);
            }
            if l1.is_finite() {
                out.lower_1[idx] = Some(l1);
            }
        }
        if b2.is_finite() {
            let u2 = self.mean + b2;
            let l2 = self.mean - b2;
            if u2.is_finite() {
                out.upper_2[idx] = Some(u2);
            }
            if l2.is_finite() {
                out.lower_2[idx] = Some(l2);
            }
        }
    }
}

/// Computes `VWAP` with Welford volume-weighted $\pm \text{mult}_1\sigma, \pm \text{mult}_2\sigma$ bands
/// resetting along [`VwapAnchorMode`] boundaries.
#[must_use]
pub fn compute_session_vwap_with_anchor(
    candles: &[Candle],
    timeframe: Timeframe,
    utc_offset_seconds: i64,
    anchor_mode: VwapAnchorMode,
    mult_1: f64,
    mult_2: f64,
) -> VwapBandsOutput {
    let n = candles.len();
    if n == 0 {
        return VwapBandsOutput::default();
    }

    let (band_1, band_2) = normalize_band_multipliers(mult_1, mult_2);

    let mut out = VwapBandsOutput {
        vwap: vec![None; n],
        upper_1: vec![None; n],
        lower_1: vec![None; n],
        upper_2: vec![None; n],
        lower_2: vec![None; n],
        session_starts: Vec::new(),
    };

    let mut current_bucket: Option<i64> = None;
    let mut acc_vol = WelfordVwapAcc::new();
    let mut acc_eq = WelfordVwapAcc::new();
    // Strictly causal (`DEF-001` zero-lookahead): track positive-volume presence prefix-by-prefix
    // rather than scanning future candles with `.iter().any(...)`.
    let mut seen_pos_vol = false;

    for (i, c) in candles.iter().enumerate() {
        let bucket = vwap_bucket_key(c.timestamp, timeframe, utc_offset_seconds, anchor_mode);
        if current_bucket != Some(bucket) {
            current_bucket = Some(bucket);
            acc_vol = WelfordVwapAcc::new();
            acc_eq = WelfordVwapAcc::new();
            out.session_starts.push(i);
        }

        let tp = safe_typical_price(c);
        if c.volume.is_finite() && c.volume > 0.0 {
            seen_pos_vol = true;
            acc_vol.push(tp, c.volume);
        }
        acc_eq.push(tp, 1.0);

        if seen_pos_vol {
            acc_vol.write_point(&mut out, i, band_1, band_2);
        } else {
            acc_eq.write_point(&mut out, i, band_1, band_2);
        }
    }

    out
}

/// Computes Exchange-Session `VWAP` with Welford volume-weighted $\pm \text{mult}_1\sigma, \pm \text{mult}_2\sigma$ bands.
#[must_use]
pub fn compute_session_vwap(
    candles: &[Candle],
    timeframe: Timeframe,
    utc_offset_seconds: i64,
    mult_1: f64,
    mult_2: f64,
) -> VwapBandsOutput {
    compute_session_vwap_with_anchor(
        candles,
        timeframe,
        utc_offset_seconds,
        VwapAnchorMode::AutoSession,
        mult_1,
        mult_2,
    )
}

/// Computes an `Anchored VWAP (AVWAP)` starting at `anchor_index` forward to `candles.len() - 1`
/// using numerically stable one-pass volume-weighted Welford variance bands.
#[must_use]
pub fn compute_anchored_vwap(
    candles: &[Candle],
    anchor_index: usize,
    mult_1: f64,
    mult_2: f64,
) -> VwapBandsOutput {
    let n = candles.len();
    if n == 0 || anchor_index >= n {
        return VwapBandsOutput {
            vwap: vec![None; n],
            upper_1: vec![None; n],
            lower_1: vec![None; n],
            upper_2: vec![None; n],
            lower_2: vec![None; n],
            session_starts: Vec::new(),
        };
    }

    let (band_1, band_2) = normalize_band_multipliers(mult_1, mult_2);

    let mut out = VwapBandsOutput {
        vwap: vec![None; n],
        upper_1: vec![None; n],
        lower_1: vec![None; n],
        upper_2: vec![None; n],
        lower_2: vec![None; n],
        session_starts: vec![anchor_index],
    };

    let mut acc_vol = WelfordVwapAcc::new();
    let mut acc_eq = WelfordVwapAcc::new();
    let mut seen_pos_vol = false;
    for (offset, c) in candles[anchor_index..].iter().enumerate() {
        let i = anchor_index + offset;
        let tp = safe_typical_price(c);
        if c.volume.is_finite() && c.volume > 0.0 {
            seen_pos_vol = true;
            acc_vol.push(tp, c.volume);
        }
        acc_eq.push(tp, 1.0);
        if seen_pos_vol {
            acc_vol.write_point(&mut out, i, band_1, band_2);
        } else {
            acc_eq.write_point(&mut out, i, band_1, band_2);
        }
    }

    out
}
