//! Pure `f64`/`i64`, `wasm32-unknown-unknown`-compatible Session TPO / Market Profile Engine (`Milestone R13`).
//!
//! Computes CME/CBOT Time-Price Opportunity (`TPO`) distributions per session across intraday/daily/weekly
//! timeframes: Point of Control (`POC`), dual-row `70%` Value Area (`VAH`/`VAL`), Initial Balance (`IBH`/`IBL`),
//! interior `Single Prints`, and strictly causal `DEF-001` forward `Naked POC` rays.

use serde::{Deserialize, Serialize};

use crate::data::Timeframe;
use crate::model::Candle;

use super::vwap::{VwapAnchorMode, vwap_bucket_key};

pub const MIN_MARKET_PROFILE_ROWS: usize = 8;
pub const MAX_MARKET_PROFILE_ROWS: usize = 96;
pub const DEFAULT_MAX_PROFILE_SESSIONS: usize = 24;

/// Session grouping cadence for Session TPO / Market Profile segmentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ProfileSessionPeriod {
    /// Local calendar day on intraday timeframes (`< 1D`), ISO week on `1D`, calendar month on `1W`.
    #[default]
    Auto,
    /// Calendar day in the venue's local timezone (`UTC+offset`).
    Daily,
    /// ISO-Monday weekly session in the venue's local timezone.
    Weekly,
}

/// Configuration for Session TPO / Market Profile evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MarketProfileConfig {
    pub session_period: ProfileSessionPeriod,
    pub utc_offset_seconds: i64,
    pub num_bins: usize,
    pub value_area_pct: f64,
    pub ib_bars: usize,
    pub max_sessions: usize,
    pub extend_naked_poc: bool,
}

impl Default for MarketProfileConfig {
    fn default() -> Self {
        Self {
            session_period: ProfileSessionPeriod::Auto,
            utc_offset_seconds: 0,
            num_bins: 24,
            value_area_pct: 0.70,
            ib_bars: 4,
            max_sessions: DEFAULT_MAX_PROFILE_SESSIONS,
            extend_naked_poc: true,
        }
    }
}

impl MarketProfileConfig {
    /// Clamps parameters into mathematically safe domains and replaces non-finite inputs.
    #[must_use]
    pub fn sanitized(self) -> Self {
        let va = if self.value_area_pct.is_finite() {
            self.value_area_pct.clamp(0.10, 0.95)
        } else {
            0.70
        };
        Self {
            session_period: self.session_period,
            utc_offset_seconds: self.utc_offset_seconds.clamp(-50_400, 50_400),
            num_bins: self
                .num_bins
                .clamp(MIN_MARKET_PROFILE_ROWS, MAX_MARKET_PROFILE_ROWS),
            value_area_pct: va,
            ib_bars: self.ib_bars.clamp(1, 60),
            max_sessions: self.max_sessions.clamp(1, 64),
            extend_naked_poc: self.extend_naked_poc,
        }
    }
}

/// One horizontal TPO price level within a [`SessionMarketProfile`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TpoPriceRow {
    pub price_low: f64,
    pub price_high: f64,
    pub price_mid: f64,
    pub tpo_count: usize,
    pub first_period_slot: Option<usize>,
    pub volume: f64,
    pub in_value_area: bool,
    pub is_poc: bool,
    pub is_single_print: bool,
}

/// Complete TPO / Market Profile summary for a single contiguous trading session.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionMarketProfile {
    pub session_key: i64,
    pub start_index: usize,
    pub end_index: usize,
    pub range_low: f64,
    pub range_high: f64,
    pub bin_height: f64,
    pub poc_index: usize,
    pub poc_price: f64,
    pub vah_price: f64,
    pub val_price: f64,
    pub ib_high: f64,
    pub ib_low: f64,
    pub total_tpo_count: usize,
    pub max_row_tpo: usize,
    pub total_volume: f64,
    pub rows: Vec<TpoPriceRow>,
    /// Index of the first causal subsequent candle whose `[low, high]` mitigates `poc_price`, if any.
    pub naked_poc_mitigated_index: Option<usize>,
    /// Last candle index up to which a completed prior session's `POC` ray extends (`<= candles.len() - 1`).
    pub naked_poc_end_index: usize,
}

/// Multi-session Market Profile output ordered chronologically (`start_index` ascending).
/// Lightweight causal Naked POC ray carried forward from sessions older than the
/// rendered full-TPO window (`max_sessions`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HistoricalNakedPoc {
    pub session_end_index: usize,
    pub naked_poc_mitigated_index: Option<usize>,
    pub naked_poc_end_index: usize,
    pub poc_price: f64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MarketProfileOutput {
    pub profiles: Vec<SessionMarketProfile>,
    pub historical_naked_pocs: Vec<HistoricalNakedPoc>,
}

#[inline]
fn profile_session_bucket(
    timestamp: i64,
    timeframe: Timeframe,
    period: ProfileSessionPeriod,
    utc_offset_seconds: i64,
) -> i64 {
    let anchor = match period {
        ProfileSessionPeriod::Auto => match timeframe {
            Timeframe::M1
            | Timeframe::M5
            | Timeframe::M15
            | Timeframe::M30
            | Timeframe::H1
            | Timeframe::H4 => VwapAnchorMode::Daily,
            Timeframe::D1 => VwapAnchorMode::Weekly,
            Timeframe::W1 => VwapAnchorMode::Monthly,
            Timeframe::MN1 => VwapAnchorMode::Yearly,
        },
        ProfileSessionPeriod::Daily => VwapAnchorMode::Daily,
        ProfileSessionPeriod::Weekly => VwapAnchorMode::Weekly,
    };
    vwap_bucket_key(timestamp, timeframe, utc_offset_seconds, anchor)
}

#[inline]
fn is_valid_candle(c: &Candle) -> bool {
    c.open.is_finite()
        && c.high.is_finite()
        && c.low.is_finite()
        && c.close.is_finite()
        && c.high >= c.low
}

/// Evaluates trailing Market Profile sessions and causal `DEF-001` Naked POC rays on `candles`.
#[must_use]
pub fn compute_session_market_profiles(
    candles: &[Candle],
    timeframe: Timeframe,
    config: MarketProfileConfig,
) -> MarketProfileOutput {
    if candles.is_empty() {
        return MarketProfileOutput::default();
    }

    let cfg = config.sanitized();
    let mut ranges: Vec<(i64, usize, usize)> = Vec::new();
    let mut cur_key: Option<i64> = None;
    let mut cur_start = 0_usize;

    for (idx, c) in candles.iter().enumerate() {
        let key = profile_session_bucket(
            c.timestamp,
            timeframe,
            cfg.session_period,
            cfg.utc_offset_seconds,
        );
        match cur_key {
            Some(prev) if prev == key => {}
            Some(prev) => {
                ranges.push((prev, cur_start, idx - 1));
                cur_key = Some(key);
                cur_start = idx;
            }
            None => {
                cur_key = Some(key);
                cur_start = idx;
            }
        }
    }
    if let Some(last_key) = cur_key {
        ranges.push((last_key, cur_start, candles.len() - 1));
    }

    let keep_from = ranges.len().saturating_sub(cfg.max_sessions);
    let first_kept_start = ranges.get(keep_from).map_or(0, |r| r.1);
    let mut historical_naked_pocs = Vec::new();
    if cfg.extend_naked_poc && keep_from > 0 && !candles.is_empty() {
        let hist_from = keep_from.saturating_sub(96);
        for &(session_key, start_index, end_index) in &ranges[hist_from..keep_from] {
            if end_index + 1 >= candles.len() {
                continue;
            }
            if let Some(old_prof) =
                build_single_session_profile(candles, session_key, start_index, end_index, cfg)
            {
                let poc_price = old_prof.poc_price;
                let mut mitigated_at = None;
                for (offset, c) in candles[(end_index + 1)..].iter().enumerate() {
                    if is_valid_candle(c) && c.low <= poc_price && poc_price <= c.high {
                        mitigated_at = Some(end_index + 1 + offset);
                        break;
                    }
                }
                let naked_end = mitigated_at.unwrap_or(candles.len() - 1);
                if mitigated_at.is_none() || naked_end >= first_kept_start {
                    historical_naked_pocs.push(HistoricalNakedPoc {
                        session_end_index: end_index,
                        naked_poc_mitigated_index: mitigated_at,
                        naked_poc_end_index: naked_end,
                        poc_price,
                    });
                }
            }
        }
    }

    let mut profiles = Vec::with_capacity(ranges.len() - keep_from);
    for &(session_key, start_index, end_index) in &ranges[keep_from..] {
        if let Some(profile) =
            build_single_session_profile(candles, session_key, start_index, end_index, cfg)
        {
            profiles.push(profile);
        }
    }

    // Causal DEF-001 Naked POC scan: completed prior sessions extend strictly into subsequent observed
    // bars only (`(end_index + 1)..candles.len()`), stopping on first mitigation touch.
    let profile_count = profiles.len();
    for (s_idx, profile) in profiles.iter_mut().enumerate() {
        let end_idx = profile.end_index;
        if !cfg.extend_naked_poc || s_idx + 1 >= profile_count || end_idx + 1 >= candles.len() {
            profile.naked_poc_mitigated_index = None;
            profile.naked_poc_end_index = end_idx;
            continue;
        }
        let poc_price = profile.poc_price;
        let mut mitigated_at = None;
        for (offset, c) in candles[(end_idx + 1)..].iter().enumerate() {
            if is_valid_candle(c) && c.low <= poc_price && poc_price <= c.high {
                mitigated_at = Some(end_idx + 1 + offset);
                break;
            }
        }
        profile.naked_poc_mitigated_index = mitigated_at;
        profile.naked_poc_end_index = mitigated_at.unwrap_or(candles.len() - 1);
    }

    MarketProfileOutput {
        profiles,
        historical_naked_pocs,
    }
}

fn build_single_session_profile(
    candles: &[Candle],
    session_key: i64,
    start_index: usize,
    end_index: usize,
    cfg: MarketProfileConfig,
) -> Option<SessionMarketProfile> {
    let slice = &candles[start_index..=end_index];
    let mut range_low = f64::INFINITY;
    let mut range_high = f64::NEG_INFINITY;
    let mut ib_low = f64::INFINITY;
    let mut ib_high = f64::NEG_INFINITY;
    let mut valid_count = 0_usize;

    for c in slice {
        if !is_valid_candle(c) {
            continue;
        }
        let valid_slot = valid_count;
        valid_count += 1;
        range_low = range_low.min(c.low);
        range_high = range_high.max(c.high);
        if valid_slot < cfg.ib_bars {
            ib_low = ib_low.min(c.low);
            ib_high = ib_high.max(c.high);
        }
    }

    if valid_count == 0 || !range_low.is_finite() || !range_high.is_finite() {
        return None;
    }
    if !ib_low.is_finite() || !ib_high.is_finite() {
        ib_low = range_low;
        ib_high = range_high;
    }

    let row_count = cfg.num_bins;
    let span = range_high - range_low;

    if span <= 1.0e-12 {
        let mut total_volume = 0.0_f64;
        for c in slice.iter().filter(|c| is_valid_candle(c)) {
            if c.volume.is_finite() && c.volume > 0.0 {
                total_volume += c.volume;
            }
        }
        let rows = (0..row_count)
            .map(|i| TpoPriceRow {
                price_low: range_low,
                price_high: range_high,
                price_mid: range_low,
                tpo_count: if i == 0 { valid_count } else { 0 },
                first_period_slot: if i == 0 { Some(0) } else { None },
                volume: if i == 0 { total_volume } else { 0.0 },
                in_value_area: i == 0,
                is_poc: i == 0,
                is_single_print: false,
            })
            .collect();
        return Some(SessionMarketProfile {
            session_key,
            start_index,
            end_index,
            range_low,
            range_high,
            bin_height: 0.0,
            poc_index: 0,
            poc_price: range_low,
            vah_price: range_high,
            val_price: range_low,
            ib_high,
            ib_low,
            total_tpo_count: valid_count,
            max_row_tpo: valid_count,
            total_volume,
            rows,
            naked_poc_mitigated_index: None,
            naked_poc_end_index: end_index,
        });
    }

    let bin_height = span / (row_count as f64);
    let mut tpo_counts = vec![0_usize; row_count];
    let mut first_slots: Vec<Option<usize>> = vec![None; row_count];
    let mut volumes = vec![0.0_f64; row_count];
    let mut total_volume = 0.0_f64;

    for (slot_idx, c) in slice.iter().enumerate() {
        if !is_valid_candle(c) {
            continue;
        }
        let vol = if c.volume.is_finite() && c.volume > 0.0 {
            c.volume
        } else {
            0.0
        };
        total_volume += vol;

        let candle_span = c.high - c.low;
        if candle_span <= 1.0e-12 {
            let idx =
                (((c.close - range_low) / bin_height).floor().max(0.0) as usize).min(row_count - 1);
            tpo_counts[idx] += 1;
            first_slots[idx].get_or_insert(slot_idx);
            volumes[idx] += vol;
            continue;
        }

        let b0 = (((c.low - range_low) / bin_height).floor().max(0.0) as usize).min(row_count - 1);
        let raw_b1 = ((c.high - range_low) / bin_height).ceil().max(0.0) as usize;
        let b1 = raw_b1.saturating_sub(1).max(b0).min(row_count - 1);

        let mut overlap_sum = 0.0_f64;
        for bin_idx in b0..=b1 {
            tpo_counts[bin_idx] += 1;
            first_slots[bin_idx].get_or_insert(slot_idx);
            if vol > 0.0 {
                let bin_lo = range_low + (bin_idx as f64) * bin_height;
                let bin_hi = if bin_idx + 1 == row_count {
                    range_high
                } else {
                    range_low + ((bin_idx + 1) as f64) * bin_height
                };
                overlap_sum += (bin_hi.min(c.high) - bin_lo.max(c.low)).max(0.0);
            }
        }
        if vol > 0.0 {
            if overlap_sum <= 1.0e-15 {
                let mid = 0.5 * c.low + 0.5 * c.high;
                let mid_idx =
                    (((mid - range_low) / bin_height).floor().max(0.0) as usize).min(row_count - 1);
                volumes[mid_idx] += vol;
            } else {
                for (bin_idx, vol_slot) in volumes.iter_mut().enumerate().take(b1 + 1).skip(b0) {
                    let bin_lo = range_low + (bin_idx as f64) * bin_height;
                    let bin_hi = if bin_idx + 1 == row_count {
                        range_high
                    } else {
                        range_low + ((bin_idx + 1) as f64) * bin_height
                    };
                    let w = (bin_hi.min(c.high) - bin_lo.max(c.low)).max(0.0) / overlap_sum;
                    *vol_slot += vol * w;
                }
            }
        }
    }

    let total_tpo_count: usize = tpo_counts.iter().sum();
    let max_row_tpo = tpo_counts.iter().copied().max().unwrap_or(0);
    if total_tpo_count == 0 {
        return None;
    }

    let session_mid = 0.5 * range_low + 0.5 * range_high;
    let mut poc_index = 0_usize;
    let mut best_tpo = 0_usize;
    let mut best_vol = -1.0_f64;
    let mut best_dist = f64::INFINITY;
    let vol_tie_eps = (total_volume.max(0.0) * 1.0e-12).max(1.0e-15);

    let min_visited = tpo_counts.iter().position(|&c| c > 0).unwrap_or(0);
    let max_visited = tpo_counts
        .iter()
        .rposition(|&c| c > 0)
        .unwrap_or(row_count - 1);

    let mut rows = Vec::with_capacity(row_count);
    for i in 0..row_count {
        let price_low = range_low + (i as f64) * bin_height;
        let price_high = if i + 1 == row_count {
            range_high
        } else {
            range_low + ((i + 1) as f64) * bin_height
        };
        let price_mid = 0.5 * price_low + 0.5 * price_high;
        let tpo = tpo_counts[i];
        let vol = volumes[i];
        let dist = (price_mid - session_mid).abs();

        let better_poc = tpo > best_tpo
            || (tpo == best_tpo
                && (vol > best_vol + vol_tie_eps
                    || ((vol - best_vol).abs() <= vol_tie_eps && dist < best_dist)));
        if better_poc {
            best_tpo = tpo;
            best_vol = vol;
            best_dist = dist;
            poc_index = i;
        }

        let is_single_print = tpo == 1 && i > min_visited && i < max_visited;
        rows.push(TpoPriceRow {
            price_low,
            price_high,
            price_mid,
            tpo_count: tpo,
            first_period_slot: first_slots[i],
            volume: vol,
            in_value_area: false,
            is_poc: false,
            is_single_print,
        });
    }

    // CBOT / CME dual-row step Value Area expansion over total TPO count starting from POC.
    let va_target = (total_tpo_count as f64) * cfg.value_area_pct * (1.0 - 1.0e-12);
    let mut lo = poc_index;
    let mut hi = poc_index;
    let mut acc_tpo = rows[poc_index].tpo_count as f64;

    while acc_tpo < va_target && (lo > 0 || hi + 1 < row_count) {
        let up_cnt = (row_count - 1 - hi).min(2);
        let dn_cnt = lo.min(2);
        let up_tpo: usize = (1..=up_cnt).map(|d| rows[hi + d].tpo_count).sum();
        let dn_tpo: usize = (1..=dn_cnt).map(|d| rows[lo - d].tpo_count).sum();

        let go_up = if up_cnt == 0 {
            false
        } else if dn_cnt == 0 {
            true
        } else if up_tpo == dn_tpo {
            let up_vol: f64 = (1..=up_cnt).map(|d| rows[hi + d].volume).sum();
            let dn_vol: f64 = (1..=dn_cnt).map(|d| rows[lo - d].volume).sum();
            if (up_vol - dn_vol).abs() > vol_tie_eps {
                up_vol > dn_vol
            } else {
                up_cnt >= dn_cnt
            }
        } else {
            up_tpo > dn_tpo
        };

        if go_up {
            hi += up_cnt;
            acc_tpo += up_tpo as f64;
        } else {
            lo -= dn_cnt;
            acc_tpo += dn_tpo as f64;
        }
    }

    rows[poc_index].is_poc = true;
    for row in &mut rows[lo..=hi] {
        row.in_value_area = true;
    }

    let poc_price = rows[poc_index].price_mid;
    let val_price = rows[lo].price_low;
    let vah_price = rows[hi].price_high;

    Some(SessionMarketProfile {
        session_key,
        start_index,
        end_index,
        range_low,
        range_high,
        bin_height,
        poc_index,
        poc_price,
        vah_price,
        val_price,
        ib_high,
        ib_low,
        total_tpo_count,
        max_row_tpo,
        total_volume,
        rows,
        naked_poc_mitigated_index: None,
        naked_poc_end_index: end_index,
    })
}
