//! Pure `f64`/`i64`, `wasm32-unknown-unknown`-compatible Data Readiness, Calendar-Aware Missing-Session
//! Gap & Outlier Audit, and `CafeF SolieuGD` 4-Decimal Post-Split Normalization Engine (`Milestone R19 / C3`).

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::journal::{
    ExchangeVenue, VnPriceUnit, compute_daily_price_band, is_vn_exchange_trading_day_days,
};
use crate::model::Candle;

use super::calendar::{civil_from_days, days_from_civil, format_utc_date, weekday_from_days};
use super::presets::{DataSourcePreset, WatchlistFeedEntry};
use super::timeframe::Timeframe;

/// Maximum missing session ISO dates retained inline inside [`DataQualityAuditReport`] for UI inspection.
pub const MAX_AUDIT_SAMPLE_GAPS: usize = 32;

/// Maximum outlier / anomaly diagnostics retained inline inside [`DataQualityAuditReport`].
pub const MAX_AUDIT_ANOMALY_SAMPLES: usize = 24;

/// Single-ULP tolerance (`0.0001 kVND == 0.1 VND`) for `CafeF SolieuGD` post-split rounding (`cafef_importer.py`).
pub const CAFEF_SINGLE_ULP_TOL_VND: f64 = 0.100_001;

/// Maximum relative envelope discrepancy (`6 bps = 0.06%`) permitted on `Close` for `CafeF SolieuGD` split rows.
pub const CAFEF_MAX_CLOSE_REL_TOL: f64 = 0.0006;

/// Maximum absolute envelope discrepancy (`25.0 VND`) permitted on `Close` for `CafeF SolieuGD` split rows.
pub const CAFEF_MAX_CLOSE_ABS_TOL_VND: f64 = 25.0;

/// High-level readiness classification matching & extending `readiness_service.py` + `DashboardPage.tsx`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum DataReadinessStatus {
    #[default]
    Ready,
    PartialWarning,
    Empty,
}

impl DataReadinessStatus {
    #[inline]
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::Ready => "DATA READY",
            Self::PartialWarning => "CHECK GAPS / ANOMALIES",
            Self::Empty => "EMPTY",
        }
    }
}

/// Outcome of the 5-rule `CafeF SolieuGD` 4-decimal post-split OHLC envelope check (`cafef_importer.py:49-167`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CafeFSplitNormOutcome {
    /// Row already satisfies `low <= min(open, close) <= max(open, close) <= high`.
    UnchangedValid,
    /// Post-split 4-decimal truncation artifact repaired by expanding `[low, high]` while preserving `open`/`close`/`volume`.
    NormalizedSplitPrecision {
        new_high: f64,
        new_low: f64,
        delta_vnd: f64,
    },
    /// Genuine OHLC envelope corruption (e.g. `low > high`, index symbol, unadjusted round ticks, or `> 6 bps` / `> 25 VND`).
    RejectedGenuineViolation,
}

#[inline]
fn round_4dp(v: f64) -> f64 {
    (v * 10_000.0).round() / 10_000.0
}

#[inline]
fn is_vn_listed_equity_etf_or_cw_symbol(symbol: &str) -> bool {
    let upper = symbol.trim().to_ascii_uppercase();
    if upper.is_empty()
        || upper.starts_with('^')
        || upper.contains("INDEX")
        || upper.starts_with("VN30F")
        || upper.ends_with("USDT")
        || upper.ends_with("USD")
    {
        return false;
    }
    // Full `KNOWN_INDEX_SYMBOLS` set from `cafef_importer.py:26-33` + common US/Crypto 3-letter tickers:
    if matches!(
        upper.as_str(),
        "VNINDEX"
            | "VN-INDEX"
            | "HNX-INDEX"
            | "HNXINDEX"
            | "UPCOM-INDEX"
            | "UPCOMINDEX"
            | "VN30"
            | "VN100"
            | "VNALL"
            | "VNMID"
            | "VNSML"
            | "HNX"
            | "HNX30"
            | "HNXLCAP"
            | "HNXSMCAP"
            | "HNXFIN"
            | "HNXCON"
            | "HNXMAN"
            | "UPCOM"
            | "VNX50"
            | "VNXALL"
            | "VN30F1M"
            | "VN30F2M"
            | "VNSI"
            | "VNHEAL"
            | "VNFIN"
            | "VNREAL"
            | "VNMAT"
            | "VNIT"
            | "VNIND"
            | "VNENE"
            | "VNDIAMOND"
            | "VNFINLEAD"
            | "VNFINSELECT"
            | "SPY"
            | "QQQ"
            | "IWM"
            | "DIA"
            | "BTC"
            | "ETH"
            | "SOL"
    ) {
        return false;
    }

    let bytes = upper.as_bytes();
    if !bytes.iter().all(u8::is_ascii_alphanumeric) {
        return false;
    }
    match bytes.len() {
        // Standard 3-character listed HOSE/HNX/UPCOM stock (`FPT`, `MWG`, `NT2`, `C4G`, `PC1`):
        3 => bytes[0].is_ascii_uppercase(),
        // 8-character covered warrant (`CFPT2501`) or domestic ETF (`E1VFVN30`, `FUEVFVND`, `FUESSVFL`):
        8 => bytes[0] == b'C' || upper == "E1VFVN30" || upper.starts_with("FUE"),
        _ => false,
    }
}

/// Evaluates the exact 5-rule `CafeF SolieuGD` post-split 4-decimal OHLC precision gate (`cafef_importer.py`).
///
/// Automatically handles both `1,000 VND` quotes (`< 500.0` or `<= 2,000.0` with `>2` decimals, e.g. `28.5432` or `1000.004`)
/// and full `VND` quotes (`>= 500.0` with `<=2` decimals, e.g. `950.4` or `28,543.2`).
#[must_use]
pub fn evaluate_and_normalize_cafef_split_ohlc(
    symbol: &str,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
) -> CafeFSplitNormOutcome {
    if !open.is_finite()
        || !high.is_finite()
        || !low.is_finite()
        || !close.is_finite()
        || !volume.is_finite()
        || open <= 0.0
        || high <= 0.0
        || low <= 0.0
        || close <= 0.0
        || volume < 0.0
        || low > high
    {
        return CafeFSplitNormOutcome::RejectedGenuineViolation;
    }

    let body_max = open.max(close);
    let body_min = open.min(close);
    if high >= body_max && low <= body_min {
        return CafeFSplitNormOutcome::UnchangedValid;
    }

    // Rule 1: Only Vietnam listed equity / ETF / covered-warrant symbols qualify (never indices, futures, US, or crypto).
    if !is_vn_listed_equity_etf_or_cw_symbol(symbol) {
        return CafeFSplitNormOutcome::RejectedGenuineViolation;
    }

    let max_px = open.max(high).max(low).max(close);
    let has_more_than_2dp = [open, high, low, close]
        .into_iter()
        .any(|v| (v * 100.0 - (v * 100.0).round()).abs() > 1.0e-3);
    let scale_to_vnd = if (max_px <= 2_000.0 && has_more_than_2dp) || max_px < 500.0 {
        1_000.0
    } else {
        1.0
    };
    let o_vnd = round_4dp(open * scale_to_vnd);
    let h_vnd = round_4dp(high * scale_to_vnd);
    let l_vnd = round_4dp(low * scale_to_vnd);
    let c_vnd = round_4dp(close * scale_to_vnd);

    // Rule 2: Sub-10-VND post-split decimal precision must be present in `{O, H, L, C}`.
    let has_split_precision = [o_vnd, h_vnd, l_vnd, c_vnd]
        .into_iter()
        .any(|v| (v - 10.0 * (v / 10.0).round()).abs() > 1.0e-3);
    if !has_split_precision {
        return CafeFSplitNormOutcome::RejectedGenuineViolation;
    }

    // Rule 3: `Open` discrepancy capped at `1 ULP` (`<= 0.100001 VND`).
    let delta_open_vnd = (o_vnd - h_vnd).max(l_vnd - o_vnd).max(0.0);
    if delta_open_vnd > CAFEF_SINGLE_ULP_TOL_VND {
        return CafeFSplitNormOutcome::RejectedGenuineViolation;
    }

    // Rule 4: `Close` / max discrepancy capped at `1 ULP` OR (`<= 6 bps` AND `<= 25.0 VND`).
    let delta_close_vnd = (c_vnd - h_vnd).max(l_vnd - c_vnd).max(0.0);
    let delta_max_vnd = delta_open_vnd.max(delta_close_vnd);
    let within_cap = delta_max_vnd <= CAFEF_SINGLE_ULP_TOL_VND
        || ((delta_max_vnd / h_vnd.max(1.0e-9)) <= CAFEF_MAX_CLOSE_REL_TOL
            && delta_max_vnd <= CAFEF_MAX_CLOSE_ABS_TOL_VND);
    if !within_cap {
        return CafeFSplitNormOutcome::RejectedGenuineViolation;
    }

    // Rule 5: Discrepancy must remain strictly `< 0.5 * min_positive_intraday_step` when positive steps exist.
    let mut min_step: Option<f64> = None;
    let candidate_steps: &[f64] = if delta_open_vnd > 0.0 {
        &[h_vnd - l_vnd]
    } else {
        &[h_vnd - l_vnd, h_vnd - o_vnd, o_vnd - l_vnd]
    };
    for &s in candidate_steps {
        if s > 1.0e-6 {
            min_step = Some(min_step.map_or(s, |prev| prev.min(s)));
        }
    }
    match min_step {
        Some(step) if delta_max_vnd >= 0.5 * step => {
            return CafeFSplitNormOutcome::RejectedGenuineViolation;
        }
        None if delta_max_vnd > CAFEF_SINGLE_ULP_TOL_VND => {
            return CafeFSplitNormOutcome::RejectedGenuineViolation;
        }
        _ => {}
    }

    let new_high = high.max(body_max);
    let new_low = low.min(body_min);
    CafeFSplitNormOutcome::NormalizedSplitPrecision {
        new_high,
        new_low,
        delta_vnd: delta_max_vnd,
    }
}

/// Specific bar-level data anomaly surfaced in [`DataQualityAuditReport`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataAuditAnomaly {
    pub bar_index: usize,
    pub date_label: String,
    pub kind: String,
    pub detail: String,
}

/// Comprehensive single-series health, calendar coverage, gap, outlier, and split-rounding report (`Milestone R19 / C3`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataQualityAuditReport {
    pub symbol: String,
    pub venue: ExchangeVenue,
    pub timeframe: Timeframe,
    pub status: DataReadinessStatus,
    pub bar_count: usize,
    pub earliest_date: Option<String>,
    pub latest_date: Option<String>,
    pub expected_sessions: usize,
    pub observed_sessions: usize,
    pub coverage_pct: f64,
    pub missing_session_count: usize,
    pub missing_session_samples: Vec<String>,
    pub holiday_or_weekend_bar_count: usize,
    pub non_monotonic_count: usize,
    pub duplicate_timestamp_count: usize,
    pub zero_volume_count: usize,
    pub extreme_range_bar_count: usize,
    pub band_exceedance_split_hint_count: usize,
    pub volume_spike_10x_count: usize,
    pub loader_deduplicated_rows: usize,
    pub loader_repaired_envelope_rows: usize,
    pub anomalies: Vec<DataAuditAnomaly>,
}

/// Compact per-feed readiness row surfaced in [`UniverseDataReadinessSummary`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UniverseFeedAuditRow {
    pub symbol: String,
    pub venue: ExchangeVenue,
    pub timeframe: Timeframe,
    pub bar_count: usize,
    pub coverage_pct: f64,
    pub missing_session_count: usize,
    pub status: DataReadinessStatus,
}

/// Multi-symbol watchlist readiness Rollup (`DashboardPage.tsx` / `readiness_service.py` parity + coverage).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UniverseDataReadinessSummary {
    pub status: DataReadinessStatus,
    pub symbols_ready: usize,
    pub symbols_total: usize,
    pub total_candles: usize,
    pub distinct_timeframes: Vec<Timeframe>,
    pub earliest_date: Option<String>,
    pub latest_date: Option<String>,
    #[serde(default)]
    pub feed_audits: Vec<UniverseFeedAuditRow>,
}

/// Computes Gregorian Easter Sunday `(month, day)` via the Anonymous Gregorian Computus (`1900..=2199`).
#[inline]
fn easter_sunday_month_day(year: i32) -> (u32, u32) {
    let a = year.rem_euclid(19);
    let b = year.div_euclid(100);
    let c = year.rem_euclid(100);
    let d = b.div_euclid(4);
    let e = b.rem_euclid(4);
    let f = (b + 8).div_euclid(25);
    let g = (b - f + 1).div_euclid(3);
    let h = (19 * a + b - d - g + 15).rem_euclid(30);
    let i = c.div_euclid(4);
    let k = c.rem_euclid(4);
    let l = (32 + 2 * e + 2 * i - h - k).rem_euclid(7);
    let m = (a + 11 * h + 22 * l).div_euclid(451);
    let month = ((h + l - 7 * m + 114).div_euclid(31)) as u32;
    let day = (((h + l - 7 * m + 114).rem_euclid(31)) + 1) as u32;
    (month, day)
}

/// Full NYSE weekday holiday & historical unscheduled closure check (`1990..=2099`).
#[must_use]
pub fn is_us_nyse_holiday_ymd(year: i32, month: u32, day: u32) -> bool {
    let d = days_from_civil(year, month, day);
    let wd = weekday_from_days(d);
    if wd >= 5 {
        return false;
    }

    // Historical unscheduled NYSE full-day closures (`1994..=2025`):
    let packed = year
        .saturating_mul(10_000)
        .saturating_add((month as i32) * 100)
        .saturating_add(day as i32);
    if matches!(
        packed,
        19_940_427 // President Nixon funeral
            | 20_010_911
            | 20_010_912
            | 20_010_913
            | 20_010_914 // Sept 11 closures
            | 20_040_611 // President Reagan funeral
            | 20_070_102 // President Ford funeral
            | 20_121_029
            | 20_121_030 // Hurricane Sandy
            | 20_181_205 // President G.H.W. Bush funeral
            | 20_250_109 // President Carter funeral
    ) {
        return true;
    }

    // 1. New Year's Day (NYSE Rule 7.2: Jan 1, or Mon Jan 2 when Jan 1 is Sunday; Sat Jan 1 is NOT observed on Dec 31):
    if month == 1 && (day == 1 || (day == 2 && weekday_from_days(days_from_civil(year, 1, 1)) == 6))
    {
        return true;
    }
    // 2. Martin Luther King Jr. Day (`1998+`: 3rd Monday of January):
    if year >= 1998 && month == 1 && wd == 0 && (15..=21).contains(&day) {
        return true;
    }
    // 3. Washington's Birthday / Presidents' Day (3rd Monday of February):
    if month == 2 && wd == 0 && (15..=21).contains(&day) {
        return true;
    }
    // 4. Good Friday (2 days before Easter Sunday; NYSE closed every Good Friday):
    if (1900..=2199).contains(&year) && (month == 3 || month == 4) {
        let (em, ed) = easter_sunday_month_day(year);
        if d == days_from_civil(year, em, ed) - 2 {
            return true;
        }
    }
    // 5. Memorial Day (last Monday of May):
    if month == 5 && wd == 0 && day >= 25 {
        return true;
    }
    // 6. Juneteenth (`2022+`: June 19, observed Fri June 18 or Mon June 20):
    // 7. Independence Day (July 4, observed Fri July 3 or Mon July 5):
    // 8. Christmas Day (Dec 25, observed Fri Dec 24 or Mon Dec 26):
    let fixed: &[(u32, u32)] = if year >= 2022 {
        &[(6, 19), (7, 4), (12, 25)]
    } else {
        &[(7, 4), (12, 25)]
    };
    for &(fm, fd) in fixed {
        if month == fm && day == fd {
            return true;
        }
        let h_days = days_from_civil(year, fm, fd);
        let h_wd = weekday_from_days(h_days);
        if (h_wd == 5 && d == h_days - 1) || (h_wd == 6 && d == h_days + 1) {
            return true;
        }
    }
    // 9. Labor Day (1st Monday of September):
    if month == 9 && wd == 0 && day <= 7 {
        return true;
    }
    // 10. Thanksgiving Day (4th Thursday of November):
    if month == 11 && wd == 3 && (22..=28).contains(&day) {
        return true;
    }
    false
}

#[inline]
fn format_ict_days(ict_days: i64) -> String {
    let (y, m, d) = civil_from_days(ict_days);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Audits a causal `candles` slice for calendar session coverage, missing trading days (`VIETNAM_HOLIDAYS_V1`-aware),
/// holiday/weekend bars, extreme range/band/volume outliers, and loader repair statistics (`DEF-001` safe).
#[must_use]
pub fn audit_ohlcv_series(
    symbol: &str,
    preset: DataSourcePreset,
    timeframe: Timeframe,
    candles: &[Candle],
    loader_stats: Option<(usize, usize)>,
) -> DataQualityAuditReport {
    let venue = ExchangeVenue::infer_from_symbol_and_preset(symbol, preset);
    audit_ohlcv_series_with_venue(symbol, venue, timeframe, candles, loader_stats)
}

/// Audits a causal `candles` slice under an explicit [`ExchangeVenue`] (`Milestone R19 / C3`).
#[must_use]
pub fn audit_ohlcv_series_with_venue(
    symbol: &str,
    venue: ExchangeVenue,
    timeframe: Timeframe,
    candles: &[Candle],
    loader_stats: Option<(usize, usize)>,
) -> DataQualityAuditReport {
    let (loader_deduplicated_rows, loader_repaired_envelope_rows) = loader_stats.unwrap_or((0, 0));

    if candles.is_empty() {
        return DataQualityAuditReport {
            symbol: symbol.trim().to_owned(),
            venue,
            timeframe,
            status: DataReadinessStatus::Empty,
            bar_count: 0,
            earliest_date: None,
            latest_date: None,
            expected_sessions: 0,
            observed_sessions: 0,
            coverage_pct: 0.0,
            missing_session_count: 0,
            missing_session_samples: Vec::new(),
            holiday_or_weekend_bar_count: 0,
            non_monotonic_count: 0,
            duplicate_timestamp_count: 0,
            zero_volume_count: 0,
            extreme_range_bar_count: 0,
            band_exceedance_split_hint_count: 0,
            volume_spike_10x_count: 0,
            loader_deduplicated_rows,
            loader_repaired_envelope_rows,
            anomalies: Vec::new(),
        };
    }

    let mut min_bar_ts = candles[0].timestamp;
    let mut max_bar_ts = candles[0].timestamp;
    for c in candles.iter().skip(1) {
        min_bar_ts = min_bar_ts.min(c.timestamp);
        max_bar_ts = max_bar_ts.max(c.timestamp);
    }
    let earliest_date = Some(format_utc_date(min_bar_ts));
    let latest_date = Some(format_utc_date(max_bar_ts));

    let mut non_monotonic_count = 0_usize;
    let mut duplicate_timestamp_count = 0_usize;
    let mut zero_volume_count = 0_usize;
    let mut extreme_range_bar_count = 0_usize;
    let mut band_exceedance_split_hint_count = 0_usize;
    let mut volume_spike_10x_count = 0_usize;
    let mut holiday_or_weekend_bar_count = 0_usize;
    let mut anomalies: Vec<DataAuditAnomaly> = Vec::new();

    let is_vn_calendar = matches!(
        venue,
        ExchangeVenue::Hose
            | ExchangeVenue::Hnx
            | ExchangeVenue::Upcom
            | ExchangeVenue::Vn30Futures
            | ExchangeVenue::VnIndexBenchmark
    );

    let mut vol_win_sum = 0.0_f64;
    for (i, c) in candles.iter().enumerate() {
        if i > 0 {
            if c.timestamp < candles[i - 1].timestamp {
                non_monotonic_count += 1;
                if anomalies.len() < MAX_AUDIT_ANOMALY_SAMPLES {
                    anomalies.push(DataAuditAnomaly {
                        bar_index: i,
                        date_label: c.date_label.clone(),
                        kind: "OUT_OF_ORDER".to_owned(),
                        detail: format!("ts {} < prior {}", c.timestamp, candles[i - 1].timestamp),
                    });
                }
            } else if c.timestamp == candles[i - 1].timestamp {
                duplicate_timestamp_count += 1;
            }
        }

        if c.volume <= 0.0 {
            zero_volume_count += 1;
        }

        // Prior-20-bar causal RVOL >= 10x spike check
        if i >= 20 {
            let mean_vol = vol_win_sum / 20.0;
            if mean_vol > 1.0e-9 && c.volume >= 10.0 * mean_vol {
                volume_spike_10x_count += 1;
                if anomalies.len() < MAX_AUDIT_ANOMALY_SAMPLES {
                    anomalies.push(DataAuditAnomaly {
                        bar_index: i,
                        date_label: c.date_label.clone(),
                        kind: "VOL_SPIKE_10X".to_owned(),
                        detail: format!("RVOL(20) = {:.1}x", c.volume / mean_vol),
                    });
                }
            }
            vol_win_sum -= candles[i - 20].volume.max(0.0);
        }
        vol_win_sum += c.volume.max(0.0);

        if c.low > 0.0 && c.high >= c.low {
            let range_pct = ((c.high - c.low) / c.low) * 100.0;
            let extreme_thresh = if venue == ExchangeVenue::CryptoSpotPerp {
                45.0
            } else {
                25.0
            };
            if range_pct > extreme_thresh {
                extreme_range_bar_count += 1;
                if anomalies.len() < MAX_AUDIT_ANOMALY_SAMPLES {
                    anomalies.push(DataAuditAnomaly {
                        bar_index: i,
                        date_label: c.date_label.clone(),
                        kind: "EXTREME_RANGE".to_owned(),
                        detail: format!("(H-L)/L = {range_pct:.1}%"),
                    });
                }
            }
        }

        // Unadjusted split / corporate-action jump hint on `D1` VN cash equities:
        if timeframe == Timeframe::D1
            && venue.is_vn_cash_equity()
            && i > 0
            && candles[i - 1].close > 0.0
            && let Some(band) =
                compute_daily_price_band(venue, candles[i - 1].close, VnPriceUnit::Auto)
        {
            let slack = band.reference_price * 0.015;
            if c.close > band.ceiling_price + slack || c.close < (band.floor_price - slack).max(0.0)
            {
                band_exceedance_split_hint_count += 1;
                if anomalies.len() < MAX_AUDIT_ANOMALY_SAMPLES {
                    anomalies.push(DataAuditAnomaly {
                        bar_index: i,
                        date_label: c.date_label.clone(),
                        kind: "BAND_JUMP_SPLIT_HINT".to_owned(),
                        detail: format!(
                            "Close {:.2} outside [{:.2}..{:.2}] (possible split/dividend)",
                            c.close, band.floor_price, band.ceiling_price
                        ),
                    });
                }
            }
        }

        // Holiday / weekend bar check on `D1` VN & US calendars (`BUG-DQ-04` fix):
        if timeframe == Timeframe::D1 && venue != ExchangeVenue::CryptoSpotPerp {
            if is_vn_calendar {
                let ict_day = c.timestamp.saturating_add(7 * 3_600).div_euclid(86_400);
                if !is_vn_exchange_trading_day_days(ict_day) {
                    holiday_or_weekend_bar_count += 1;
                    if anomalies.len() < MAX_AUDIT_ANOMALY_SAMPLES {
                        anomalies.push(DataAuditAnomaly {
                            bar_index: i,
                            date_label: c.date_label.clone(),
                            kind: "NON_TRADING_DATE".to_owned(),
                            detail: format!(
                                "{} falls on weekend / VIETNAM_HOLIDAYS_V1",
                                format_ict_days(ict_day)
                            ),
                        });
                    }
                }
            } else {
                let utc_day = c.timestamp.div_euclid(86_400);
                let (y, m, d) = civil_from_days(utc_day);
                if weekday_from_days(utc_day) >= 5 || is_us_nyse_holiday_ymd(y, m, d) {
                    holiday_or_weekend_bar_count += 1;
                }
            }
        }
    }

    // Calendar-aware missing-session gap detection (`BUG-DQ-02` & `BUG-DQ-06` fixed:
    // daily `TradingCalendar` loop runs ONLY on `Timeframe::D1`; `W1` checks ISO-week continuity!).
    let mut missing_session_count = 0_usize;
    let mut missing_session_samples: Vec<String> = Vec::new();
    let expected_sessions: usize;
    let observed_sessions: usize;

    match timeframe {
        Timeframe::D1 => {
            if is_vn_calendar {
                let mut seen_days = BTreeSet::new();
                for c in candles {
                    seen_days.insert(c.timestamp.saturating_add(7 * 3_600).div_euclid(86_400));
                }
                let first_d = *seen_days.first().unwrap_or(&0);
                let last_d = *seen_days.last().unwrap_or(&0);
                let mut exp = 0_usize;
                let mut obs_valid = 0_usize;
                for d in first_d..=last_d {
                    if is_vn_exchange_trading_day_days(d) {
                        exp += 1;
                        if seen_days.contains(&d) {
                            obs_valid += 1;
                        } else {
                            missing_session_count += 1;
                            if missing_session_samples.len() < MAX_AUDIT_SAMPLE_GAPS {
                                missing_session_samples.push(format_ict_days(d));
                            }
                        }
                    }
                }
                expected_sessions = exp.max(1);
                observed_sessions = obs_valid;
            } else if venue == ExchangeVenue::CryptoSpotPerp {
                let mut seen_days = BTreeSet::new();
                for c in candles {
                    seen_days.insert(c.timestamp.div_euclid(86_400));
                }
                let first_d = *seen_days.first().unwrap_or(&0);
                let last_d = *seen_days.last().unwrap_or(&0);
                let mut exp = 0_usize;
                for d in first_d..=last_d {
                    exp += 1;
                    if !seen_days.contains(&d) {
                        missing_session_count += 1;
                        if missing_session_samples.len() < MAX_AUDIT_SAMPLE_GAPS {
                            missing_session_samples.push(format_ict_days(d));
                        }
                    }
                }
                expected_sessions = exp.max(1);
                observed_sessions = seen_days.len();
            } else {
                let mut seen_days = BTreeSet::new();
                for c in candles {
                    seen_days.insert(c.timestamp.div_euclid(86_400));
                }
                let first_d = *seen_days.first().unwrap_or(&0);
                let last_d = *seen_days.last().unwrap_or(&0);
                let mut exp = 0_usize;
                let mut obs_valid = 0_usize;
                for d in first_d..=last_d {
                    let (y, m, day) = civil_from_days(d);
                    if weekday_from_days(d) < 5 && !is_us_nyse_holiday_ymd(y, m, day) {
                        exp += 1;
                        if seen_days.contains(&d) {
                            obs_valid += 1;
                        } else {
                            missing_session_count += 1;
                            if missing_session_samples.len() < MAX_AUDIT_SAMPLE_GAPS {
                                missing_session_samples.push(format_ict_days(d));
                            }
                        }
                    }
                }
                expected_sessions = exp.max(1);
                observed_sessions = obs_valid;
            }
        }
        Timeframe::W1 => {
            // Check ISO-week bucket continuity (never daily `is_trading_day`!).
            let mut seen_weeks = BTreeSet::new();
            for c in candles {
                seen_weeks.insert(Timeframe::W1.bucket_start_timestamp(c.timestamp));
            }
            let first_w = *seen_weeks.first().unwrap_or(&0);
            let last_w = *seen_weeks.last().unwrap_or(&0);
            let mut exp = 0_usize;
            let mut obs_valid = 0_usize;
            let mut cur_w = first_w;
            while cur_w <= last_w {
                // Skip full-week Tết breaks when none of Mon..Fri in `cur_w` is a VN trading day!
                let week_ict_day = cur_w.saturating_add(7 * 3_600).div_euclid(86_400);
                let has_open_day = !is_vn_calendar
                    || (0..5).any(|off| is_vn_exchange_trading_day_days(week_ict_day + off));
                if has_open_day {
                    exp += 1;
                    if seen_weeks.contains(&cur_w) {
                        obs_valid += 1;
                    } else {
                        missing_session_count += 1;
                        if missing_session_samples.len() < MAX_AUDIT_SAMPLE_GAPS {
                            missing_session_samples.push(format_utc_date(cur_w));
                        }
                    }
                }
                cur_w = cur_w.saturating_add(604_800);
            }
            expected_sessions = exp.max(1);
            observed_sessions = obs_valid;
        }
        _ => {
            // Intraday (`1m`..`4h`): check continuous bucket steps on 24/7 Crypto, or active trading dates on exchange venues.
            let step_s = timeframe.seconds().max(60);
            if venue == ExchangeVenue::CryptoSpotPerp {
                let mut gaps = 0_usize;
                for w in candles.windows(2) {
                    let dt = w[1].timestamp.saturating_sub(w[0].timestamp);
                    if dt > step_s {
                        let missed = usize::try_from((dt / step_s).saturating_sub(1)).unwrap_or(0);
                        gaps = gaps.saturating_add(missed);
                        if missing_session_samples.len() < MAX_AUDIT_SAMPLE_GAPS {
                            missing_session_samples
                                .push(format_utc_date(w[0].timestamp.saturating_add(step_s)));
                        }
                    }
                }
                missing_session_count = gaps;
                observed_sessions = candles.len();
                expected_sessions = candles.len().saturating_add(gaps).max(1);
            } else {
                // Intraday equity/futures: check trading-day session continuity across covered dates!
                let mut seen_days = BTreeSet::new();
                for c in candles {
                    let d = if is_vn_calendar {
                        c.timestamp.saturating_add(7 * 3_600).div_euclid(86_400)
                    } else {
                        c.timestamp.div_euclid(86_400)
                    };
                    seen_days.insert(d);
                }
                let first_d = *seen_days.first().unwrap_or(&0);
                let last_d = *seen_days.last().unwrap_or(&0);
                let mut exp_d = 0_usize;
                let mut obs_d = 0_usize;
                for d in first_d..=last_d {
                    let open_day = if is_vn_calendar {
                        is_vn_exchange_trading_day_days(d)
                    } else {
                        let (y, m, day) = civil_from_days(d);
                        weekday_from_days(d) < 5 && !is_us_nyse_holiday_ymd(y, m, day)
                    };
                    if open_day {
                        exp_d += 1;
                        if seen_days.contains(&d) {
                            obs_d += 1;
                        } else {
                            missing_session_count += 1;
                            if missing_session_samples.len() < MAX_AUDIT_SAMPLE_GAPS {
                                missing_session_samples.push(format_ict_days(d));
                            }
                        }
                    }
                }
                expected_sessions = exp_d.max(1);
                observed_sessions = obs_d;
            }
        }
    }

    let coverage_pct =
        (((observed_sessions as f64) / (expected_sessions as f64)) * 100.0).clamp(0.0, 100.0);
    let status = if missing_session_count > 0
        || non_monotonic_count > 0
        || holiday_or_weekend_bar_count > 0
        || extreme_range_bar_count > 0
    {
        DataReadinessStatus::PartialWarning
    } else {
        DataReadinessStatus::Ready
    };

    DataQualityAuditReport {
        symbol: symbol.trim().to_owned(),
        venue,
        timeframe,
        status,
        bar_count: candles.len(),
        earliest_date,
        latest_date,
        expected_sessions,
        observed_sessions,
        coverage_pct,
        missing_session_count,
        missing_session_samples,
        holiday_or_weekend_bar_count,
        non_monotonic_count,
        duplicate_timestamp_count,
        zero_volume_count,
        extreme_range_bar_count,
        band_exceedance_split_hint_count,
        volume_spike_10x_count,
        loader_deduplicated_rows,
        loader_repaired_envelope_rows,
        anomalies,
    }
}

/// Computes multi-symbol readiness rollup across `feeds` (`DashboardPage.tsx` / `readiness_service.py`),
/// optionally gated by `replay_causal_close_timestamp` (`DEF-001`).
#[must_use]
pub fn summarize_universe_readiness(
    feeds: &[WatchlistFeedEntry],
    replay_causal_close_timestamp: Option<i64>,
) -> UniverseDataReadinessSummary {
    if feeds.is_empty() {
        return UniverseDataReadinessSummary {
            status: DataReadinessStatus::Empty,
            symbols_ready: 0,
            symbols_total: 0,
            total_candles: 0,
            distinct_timeframes: Vec::new(),
            earliest_date: None,
            latest_date: None,
            feed_audits: Vec::new(),
        };
    }

    let symbols_total = feeds.len();
    let mut symbols_ready = 0_usize;
    let mut total_candles = 0_usize;
    let mut tf_set = BTreeSet::new();
    let mut min_ts: Option<i64> = None;
    let mut max_ts: Option<i64> = None;
    let mut feed_audits = Vec::with_capacity(symbols_total);

    for f in feeds {
        let slice = crate::backtest::engine::slice_causal_series_for_symbol_and_options(
            Some(&f.symbol),
            &f.candles,
            f.base_timeframe,
            crate::backtest::BacktestRunOptions {
                replay_cutoff_timestamp: replay_causal_close_timestamp,
                incomplete_tail_bar: false,
            },
        );
        let rep = audit_ohlcv_series(&f.symbol, f.preset, f.base_timeframe, slice, None);
        if !slice.is_empty() {
            if rep.status == DataReadinessStatus::Ready {
                symbols_ready += 1;
            }
            total_candles = total_candles.saturating_add(slice.len());
            tf_set.insert(f.base_timeframe);
            for c in slice {
                min_ts = Some(min_ts.map_or(c.timestamp, |p| p.min(c.timestamp)));
                max_ts = Some(max_ts.map_or(c.timestamp, |p| p.max(c.timestamp)));
            }
        }
        feed_audits.push(UniverseFeedAuditRow {
            symbol: rep.symbol,
            venue: rep.venue,
            timeframe: rep.timeframe,
            bar_count: rep.bar_count,
            coverage_pct: rep.coverage_pct,
            missing_session_count: rep.missing_session_count,
            status: rep.status,
        });
    }

    let status = if total_candles == 0 {
        DataReadinessStatus::Empty
    } else if symbols_ready < symbols_total {
        DataReadinessStatus::PartialWarning
    } else {
        DataReadinessStatus::Ready
    };

    UniverseDataReadinessSummary {
        status,
        symbols_ready,
        symbols_total,
        total_candles,
        distinct_timeframes: tf_set.into_iter().collect(),
        earliest_date: min_ts.map(format_utc_date),
        latest_date: max_ts.map(format_utc_date),
        feed_audits,
    }
}
