//! Pure `f64`, `wasm32-unknown-unknown`-compatible Order-Flow Volume Profile Engine (`VRVP` & `FRVP`).
//!
//! Computes exact overlap-conserving price histograms with intrabar `OHLCV_PROXY` buy/sell delta
//! attribution (`pressure = clamp((2 * close - high - low) / (high - low), -1, 1)`), Point of Control (`POC`),
//! CME Market Profile dual-row `70%` Value Area (`VAH` / `VAL`), and High/Low Volume Nodes (`HVN` / `LVN`),
//! with an automatic TPO (equal-weight) fallback on volume-omitted OHLC feeds such as `SPY.csv`.

use serde::{Deserialize, Serialize};

use crate::model::Candle;

/// Minimum allowed bin count for Volume Profile histograms.
pub const MIN_VOLUME_PROFILE_ROWS: usize = 8;
/// Maximum allowed bin count for Volume Profile histograms.
pub const MAX_VOLUME_PROFILE_ROWS: usize = 200;

/// Configuration parameters for computing a Visible Range (`VRVP`) or Fixed Range (`FRVP`) profile.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct VolumeProfileConfig {
    pub row_count: usize,
    pub value_area_pct: f64,
}

impl Default for VolumeProfileConfig {
    fn default() -> Self {
        Self {
            row_count: 40,
            value_area_pct: 0.70,
        }
    }
}

impl VolumeProfileConfig {
    #[must_use]
    pub fn sanitized(self) -> Self {
        let pct = if self.value_area_pct.is_finite() {
            self.value_area_pct.clamp(0.10, 0.95)
        } else {
            0.70
        };
        Self {
            row_count: self
                .row_count
                .clamp(MIN_VOLUME_PROFILE_ROWS, MAX_VOLUME_PROFILE_ROWS),
            value_area_pct: pct,
        }
    }
}

/// Single price bin inside a [`VolumeProfileResult`], ordered strictly from lowest price (`index = 0`)
/// to highest price (`index = row_count - 1`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VolumeProfileBin {
    pub price_low: f64,
    pub price_high: f64,
    pub price_mid: f64,
    pub buy_volume: f64,
    pub sell_volume: f64,
    pub total_volume: f64,
    pub delta_volume: f64,
    pub in_value_area: bool,
    pub is_poc: bool,
    pub is_hvn: bool,
    pub is_lvn: bool,
}

/// Complete price-binned Volume Profile result across a candle slice.
#[derive(Debug, Clone, PartialEq)]
pub struct VolumeProfileResult {
    pub range_low: f64,
    pub range_high: f64,
    pub bin_height: f64,
    pub total_volume: f64,
    pub total_buy_volume: f64,
    pub total_sell_volume: f64,
    pub net_delta: f64,
    pub max_bin_volume: f64,
    pub poc_index: usize,
    pub poc_price: f64,
    pub vah_price: f64,
    pub val_price: f64,
    pub uses_tpo_fallback: bool,
    pub bins: Vec<VolumeProfileBin>,
}

/// Computes the intrabar `OHLCV_PROXY` buy fraction in `[0.0, 1.0]` for a candle.
#[inline]
#[must_use]
pub fn intrabar_buy_fraction(candle: &Candle) -> f64 {
    let range = candle.high - candle.low;
    if range.is_finite() && range > 1.0e-12 {
        let pressure = ((2.0 * candle.close - candle.high - candle.low) / range).clamp(-1.0, 1.0);
        0.5 * (1.0 + pressure)
    } else if candle.close > candle.open {
        1.0
    } else if candle.close < candle.open {
        0.0
    } else {
        0.5
    }
}

/// Computes a conservative price-binned Volume Profile (`VRVP` or `FRVP`) over `candles`.
///
/// Returns `None` when `candles` contains no valid finite OHLC bars (`high >= low`).
#[must_use]
pub fn compute_volume_profile(
    candles: &[Candle],
    config: VolumeProfileConfig,
) -> Option<VolumeProfileResult> {
    let cfg = config.sanitized();
    let row_count = cfg.row_count;

    let mut raw_low = f64::INFINITY;
    let mut raw_high = f64::NEG_INFINITY;
    let mut valid_count = 0_usize;
    let mut has_positive_vol = false;

    for c in candles {
        if !c.open.is_finite()
            || !c.high.is_finite()
            || !c.low.is_finite()
            || !c.close.is_finite()
            || c.high < c.low
        {
            continue;
        }
        valid_count += 1;
        raw_low = raw_low.min(c.low);
        raw_high = raw_high.max(c.high);
        if c.volume.is_finite() && c.volume > 0.0 {
            has_positive_vol = true;
        }
    }

    if valid_count == 0 || !raw_low.is_finite() || !raw_high.is_finite() {
        return None;
    }

    let (range_low, range_high) = if (raw_high - raw_low) <= 1.0e-12 {
        let pad = if raw_low > 0.0 {
            (raw_low * 1.0e-4).max(1.0e-12).min(raw_low * 0.5)
        } else {
            (raw_low.abs() * 1.0e-4).max(1.0e-4)
        };
        (raw_low - pad, raw_high + pad)
    } else {
        (raw_low, raw_high)
    };
    let total_span = range_high - range_low;
    let bin_height = total_span / (row_count as f64);
    if !bin_height.is_finite() || bin_height <= 0.0 {
        return None;
    }

    let mut buy_bins = vec![0.0_f64; row_count];
    let mut sell_bins = vec![0.0_f64; row_count];
    let mut vwap_center = 0.0_f64;
    let mut total_vol = 0.0_f64;
    let mut total_buy = 0.0_f64;
    let mut total_sell = 0.0_f64;

    let uses_tpo_fallback = !has_positive_vol;

    for c in candles {
        if !c.open.is_finite()
            || !c.high.is_finite()
            || !c.low.is_finite()
            || !c.close.is_finite()
            || c.high < c.low
        {
            continue;
        }
        let vol = if uses_tpo_fallback {
            1.0
        } else if c.volume.is_finite() && c.volume > 0.0 {
            c.volume
        } else {
            0.0
        };
        if vol <= 0.0 {
            continue;
        }

        let buy_frac = intrabar_buy_fraction(c);
        let buy_v = vol * buy_frac;
        let sell_v = vol - buy_v;
        total_vol += vol;
        total_buy += buy_v;
        total_sell += sell_v;
        let tp = c.high / 3.0 + c.low / 3.0 + c.close / 3.0;
        if total_vol > 0.0 && total_vol.is_finite() && tp.is_finite() {
            let step_w = vol / total_vol;
            let delta_tp = tp - vwap_center;
            if delta_tp.is_finite() {
                vwap_center += step_w * delta_tp;
            }
        }

        let candle_span = c.high - c.low;
        if candle_span <= 1.0e-12 {
            let idx =
                (((c.close - range_low) / bin_height).floor().max(0.0) as usize).min(row_count - 1);
            buy_bins[idx] += buy_v;
            sell_bins[idx] += sell_v;
            continue;
        }

        let b0 = (((c.low - range_low) / bin_height).floor().max(0.0) as usize).min(row_count - 1);
        let raw_b1 = ((c.high - range_low) / bin_height).ceil().max(0.0) as usize;
        let b1 = raw_b1.saturating_sub(1).max(b0).min(row_count - 1);

        let mut overlap_sum = 0.0_f64;
        for bin_idx in b0..=b1 {
            let bin_lo = range_low + (bin_idx as f64) * bin_height;
            let bin_hi = if bin_idx + 1 == row_count {
                range_high
            } else {
                range_low + ((bin_idx + 1) as f64) * bin_height
            };
            overlap_sum += (bin_hi.min(c.high) - bin_lo.max(c.low)).max(0.0);
        }

        if overlap_sum <= 1.0e-15 {
            let mid_price = 0.5 * c.low + 0.5 * c.high;
            let mid_idx = (((mid_price - range_low) / bin_height).floor().max(0.0) as usize)
                .min(row_count - 1);
            buy_bins[mid_idx] += buy_v;
            sell_bins[mid_idx] += sell_v;
        } else {
            for bin_idx in b0..=b1 {
                let bin_lo = range_low + (bin_idx as f64) * bin_height;
                let bin_hi = if bin_idx + 1 == row_count {
                    range_high
                } else {
                    range_low + ((bin_idx + 1) as f64) * bin_height
                };
                let w = (bin_hi.min(c.high) - bin_lo.max(c.low)).max(0.0) / overlap_sum;
                buy_bins[bin_idx] += buy_v * w;
                sell_bins[bin_idx] += sell_v * w;
            }
        }
    }

    if total_vol <= 0.0 || !total_vol.is_finite() {
        return None;
    }

    let mut bins = Vec::with_capacity(row_count);
    let mut poc_index = 0_usize;
    let mut max_bin_volume = -1.0_f64;
    let mut best_dist = f64::INFINITY;
    let mean_bin_vol = total_vol / (row_count as f64);

    for i in 0..row_count {
        let price_low = range_low + (i as f64) * bin_height;
        let price_high = if i + 1 == row_count {
            range_high
        } else {
            range_low + ((i + 1) as f64) * bin_height
        };
        let price_mid = 0.5 * price_low + 0.5 * price_high;
        let buy_v = buy_bins[i];
        let sell_v = sell_bins[i];
        let tot_v = buy_v + sell_v;
        let dist = (price_mid - vwap_center).abs();
        let vol_tie_eps = (max_bin_volume.max(0.0) * 1.0e-12).max(1.0e-15);

        if tot_v > max_bin_volume + vol_tie_eps
            || ((tot_v - max_bin_volume).abs() <= vol_tie_eps && dist < best_dist)
        {
            max_bin_volume = tot_v;
            best_dist = dist;
            poc_index = i;
        }

        bins.push(VolumeProfileBin {
            price_low,
            price_high,
            price_mid,
            buy_volume: buy_v,
            sell_volume: sell_v,
            total_volume: tot_v,
            delta_volume: buy_v - sell_v,
            in_value_area: false,
            is_poc: false,
            is_hvn: tot_v >= 1.5 * mean_bin_vol,
            is_lvn: tot_v > 0.0 && tot_v <= 0.5 * mean_bin_vol,
        });
    }

    // CME Market Profile dual-row step Value Area expansion starting at POC.
    let target_va_vol = total_vol * cfg.value_area_pct;
    let va_limit = target_va_vol * (1.0 - 1.0e-12);
    let mut lo = poc_index;
    let mut hi = poc_index;
    let mut acc_vol = bins[poc_index].total_volume;

    while acc_vol < va_limit && (lo > 0 || hi + 1 < row_count) {
        let up_cnt = (row_count - 1 - hi).min(2);
        let dn_cnt = lo.min(2);
        let up_vol: f64 = (1..=up_cnt).map(|d| bins[hi + d].total_volume).sum();
        let dn_vol: f64 = (1..=dn_cnt).map(|d| bins[lo - d].total_volume).sum();

        let go_up = if up_cnt == 0 {
            false
        } else if dn_cnt == 0 {
            true
        } else if (up_vol - dn_vol).abs() <= (total_vol * 1.0e-12).max(1.0e-15) {
            up_cnt >= dn_cnt
        } else {
            up_vol > dn_vol
        };

        if go_up {
            hi += up_cnt;
            acc_vol += up_vol;
        } else {
            lo -= dn_cnt;
            acc_vol += dn_vol;
        }
    }

    bins[poc_index].is_poc = true;
    for bin in &mut bins[lo..=hi] {
        bin.in_value_area = true;
    }

    let poc_price = bins[poc_index].price_mid;
    let val_price = bins[lo].price_low;
    let vah_price = bins[hi].price_high;

    Some(VolumeProfileResult {
        range_low,
        range_high,
        bin_height,
        total_volume: total_vol,
        total_buy_volume: total_buy,
        total_sell_volume: total_sell,
        net_delta: total_buy - total_sell,
        max_bin_volume: max_bin_volume.max(0.0),
        poc_index,
        poc_price,
        vah_price,
        val_price,
        uses_tpo_fallback,
        bins,
    })
}
