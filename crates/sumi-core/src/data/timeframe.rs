//! Multi-timeframe lattice (`1m`, `5m`, `15m`, `1h`, `4h`, `1D`, `1W`), ISO-Monday UTC bucket
//! alignment, upward OHLCV rollup, causal replay-safe aggregation, and $O(\log N)$ timestamp
//! <-> bar-index projection (`Milestone R06`).

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::drawings::MAX_ABS_BAR_INDEX;
use crate::model::Candle;

use super::calendar::{
    MAX_SUPPORTED_EPOCH_SECS, MIN_SUPPORTED_EPOCH_SECS, civil_from_days, days_from_civil,
    format_utc_date, format_utc_datetime_hm,
};

/// Canonical timeframe lattice supported natively by the Sumi Workstation (`1m..1M`).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
pub enum Timeframe {
    M1,
    M5,
    M15,
    M30,
    H1,
    H4,
    #[default]
    D1,
    W1,
    MN1,
}

impl Timeframe {
    /// Ordered list of all 9 supported workstation timeframes from finest (`1m`) to coarsest (`1M`).
    pub const ALL: [Self; 9] = [
        Self::M1,
        Self::M5,
        Self::M15,
        Self::M30,
        Self::H1,
        Self::H4,
        Self::D1,
        Self::W1,
        Self::MN1,
    ];

    /// Nominal duration of one bar bucket in UTC seconds.
    #[inline]
    #[must_use]
    pub const fn seconds(self) -> i64 {
        match self {
            Self::M1 => 60,
            Self::M5 => 300,
            Self::M15 => 900,
            Self::M30 => 1_800,
            Self::H1 => 3_600,
            Self::H4 => 14_400,
            Self::D1 => 86_400,
            Self::W1 => 604_800,
            Self::MN1 => 2_592_000,
        }
    }

    /// Canonical display badge (`"1m"`, `"5m"`, `"15m"`, `"30m"`, `"1h"`, `"4h"`, `"1D"`, `"1W"`, `"1M"`).
    #[inline]
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::M1 => "1m",
            Self::M5 => "5m",
            Self::M15 => "15m",
            Self::M30 => "30m",
            Self::H1 => "1h",
            Self::H4 => "4h",
            Self::D1 => "1D",
            Self::W1 => "1W",
            Self::MN1 => "1M",
        }
    }

    /// Short suffix for whitespace cursor offset pills (`"m"`, `"h"`, `"d"`, `"w"`, `"M"`).
    #[inline]
    #[must_use]
    pub const fn unit_suffix(self) -> &'static str {
        match self {
            Self::M1 | Self::M5 | Self::M15 | Self::M30 => "m",
            Self::H1 | Self::H4 => "h",
            Self::D1 => "d",
            Self::W1 => "w",
            Self::MN1 => "M",
        }
    }

    /// Numeric multiplier per bar in units of [`Self::unit_suffix`] (`1`, `5`, `15`, `30`, `1`, `4`, `1`, `1`, `1`).
    #[inline]
    #[must_use]
    pub const fn unit_multiplier(self) -> i64 {
        match self {
            Self::M1 => 1,
            Self::M5 => 5,
            Self::M15 => 15,
            Self::M30 => 30,
            Self::H1 => 1,
            Self::H4 => 4,
            Self::D1 => 1,
            Self::W1 => 1,
            Self::MN1 => 1,
        }
    }

    /// Returns `true` when bars represent sub-daily intraday intervals (`1m`, `5m`, `15m`, `30m`, `1h`, `4h`).
    #[inline]
    #[must_use]
    pub const fn is_intraday(self) -> bool {
        matches!(
            self,
            Self::M1 | Self::M5 | Self::M15 | Self::M30 | Self::H1 | Self::H4
        )
    }

    /// Returns `true` when `self` can be aggregated upward (or kept identical) into `target`.
    #[inline]
    #[must_use]
    pub const fn can_aggregate_to(self, target: Self) -> bool {
        target.seconds() >= self.seconds()
    }

    /// Parses common timeframe labels (`"1m"`, `"5m"`, `"15m"`, `"30m"`, `"1h"`, `"60m"`, `"4h"`, `"240m"`, `"1D"`, `"D"`, `"1W"`, `"W"`, `"1M"`, `"MN1"`).
    #[must_use]
    pub fn parse_label(raw: &str) -> Option<Self> {
        let trimmed = raw.trim();
        if matches!(trimmed, "1M" | "M" | "1mo" | "1MO" | "1month" | "month" | "monthly") {
            return Some(Self::MN1);
        }
        match trimmed.to_ascii_lowercase().as_str() {
            "1m" | "1min" | "m1" => Some(Self::M1),
            "5m" | "5min" | "m5" => Some(Self::M5),
            "15m" | "15min" | "m15" => Some(Self::M15),
            "30m" | "30min" | "m30" => Some(Self::M30),
            "1h" | "60m" | "h1" => Some(Self::H1),
            "4h" | "240m" | "h4" => Some(Self::H4),
            "1d" | "d" | "d1" | "daily" | "day" => Some(Self::D1),
            "1w" | "w" | "w1" | "weekly" | "week" => Some(Self::W1),
            "mn1" => Some(Self::MN1),
            _ => None,
        }
    }

    /// Aligns an arbitrary UTC epoch second `timestamp` down to the canonical start of its `self` bucket:
    /// - `1m..4h`: `timestamp.div_euclid(S) * S`
    /// - `1D`: `timestamp.div_euclid(86_400) * 86_400` (`00:00:00 UTC`)
    /// - `1W`: ISO-8601 Monday `00:00:00 UTC` (`1970-01-01` = Thursday -> `(days + 3).rem_euclid(7)`)
    /// - `1M`: First calendar day of UTC month (`YYYY-MM-01 00:00:00 UTC`)
    #[inline]
    #[must_use]
    pub const fn bucket_start_timestamp(self, timestamp: i64) -> i64 {
        match self {
            Self::M1 | Self::M5 | Self::M15 | Self::M30 | Self::H1 | Self::H4 | Self::D1 => {
                let s = self.seconds();
                timestamp.div_euclid(s) * s
            }
            Self::W1 => {
                let days = timestamp.div_euclid(86_400);
                let weekday_mon0 = (days + 3).rem_euclid(7);
                (days - weekday_mon0) * 86_400
            }
            Self::MN1 => {
                let days = timestamp.div_euclid(86_400);
                let (year, month, _) = civil_from_days(days);
                days_from_civil(year, month, 1) * 86_400
            }
        }
    }

    /// Formats a canonical `Candle::date_label` for `timestamp` at timeframe `self`.
    #[must_use]
    pub fn format_candle_label(self, timestamp: i64) -> String {
        if self.is_intraday() {
            format_utc_datetime_hm(timestamp)
        } else {
            format_utc_date(timestamp)
        }
    }
}

impl fmt::Display for Timeframe {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Errors returned when multi-timeframe aggregation preconditions are violated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AggregationError {
    CannotDownsample {
        source: Timeframe,
        target: Timeframe,
    },
    InvalidCandle {
        index: usize,
    },
    NonMonotonicTimestamps {
        index: usize,
        prev: i64,
        current: i64,
    },
}

impl fmt::Display for AggregationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CannotDownsample { source, target } => write!(
                f,
                "cannot downsample finer timeframe {target} from coarser source {source}"
            ),
            Self::InvalidCandle { index } => {
                write!(
                    f,
                    "invalid OHLCV candle or out-of-range timestamp at index {index}"
                )
            }
            Self::NonMonotonicTimestamps {
                index,
                prev,
                current,
            } => write!(
                f,
                "timestamps must be ascending before aggregation (index {index}: {current} < {prev})"
            ),
        }
    }
}

impl std::error::Error for AggregationError {}

/// Infers the native base [`Timeframe`] of a sorted, deduplicated candle slice from the
/// lower-quartile positive `Δtimestamp` step (resilient against weekend/overnight gaps even on
/// 3-bar intraday snippets), clamping pure date-only or single-bar datasets to `>= 1D`.
#[must_use]
pub fn infer_base_timeframe(candles: &[Candle], has_intraday_time: bool) -> Timeframe {
    if candles.len() < 2 {
        return Timeframe::D1;
    }

    let mut deltas = Vec::with_capacity(candles.len() - 1);
    for pair in candles.windows(2) {
        let dt = pair[1].timestamp.saturating_sub(pair[0].timestamp);
        if dt > 0 {
            deltas.push(dt);
        }
    }
    if deltas.is_empty() {
        return Timeframe::D1;
    }

    deltas.sort_unstable();
    let q1_dt = deltas[(deltas.len() - 1) / 4];

    let inferred = if q1_dt <= 90 {
        Timeframe::M1
    } else if q1_dt <= 450 {
        Timeframe::M5
    } else if q1_dt <= 1_200 {
        Timeframe::M15
    } else if q1_dt <= 2_700 {
        Timeframe::M30
    } else if q1_dt <= 7_200 {
        Timeframe::H1
    } else if q1_dt <= 43_200 {
        Timeframe::H4
    } else if q1_dt <= 345_600 {
        Timeframe::D1
    } else if q1_dt <= 1_500_000 {
        Timeframe::W1
    } else {
        Timeframe::MN1
    };

    if !has_intraday_time && inferred < Timeframe::D1 {
        Timeframe::D1
    } else {
        inferred
    }
}

/// Aggregates chronologically sorted `candles` from `source_tf` upward into `target_tf` in $O(N)$ time.
///
/// Enforces:
/// - `target_tf.seconds() >= source_tf.seconds()` (no synthetic downsampling)
/// - `Candle::is_valid()` and supported UTC epoch bounds on every input bar
/// - Monotonic ascending timestamps (`candles[i].timestamp >= candles[i - 1].timestamp`)
/// - Exact OHLCV semantics: `open = first.open`, `high = max(high)`, `low = min(low)`,
///   `close = last.close`, `volume = sum(volume)`
pub fn aggregate_ohlcv(
    candles: &[Candle],
    source_tf: Timeframe,
    target_tf: Timeframe,
) -> Result<Vec<Candle>, AggregationError> {
    if !source_tf.can_aggregate_to(target_tf) {
        return Err(AggregationError::CannotDownsample {
            source: source_tf,
            target: target_tf,
        });
    }
    if candles.is_empty() {
        return Ok(Vec::new());
    }

    for (idx, c) in candles.iter().enumerate() {
        if !c.is_valid()
            || !(MIN_SUPPORTED_EPOCH_SECS..=MAX_SUPPORTED_EPOCH_SECS).contains(&c.timestamp)
        {
            return Err(AggregationError::InvalidCandle { index: idx });
        }
        if idx > 0 && c.timestamp < candles[idx - 1].timestamp {
            return Err(AggregationError::NonMonotonicTimestamps {
                index: idx,
                prev: candles[idx - 1].timestamp,
                current: c.timestamp,
            });
        }
    }

    // Fast path when `target_tf == source_tf` and every bar already sits in a distinct bucket:
    // preserves exact original `Candle` labels/timestamps (`0` drift for `SyntheticVnindex` at `1D`).
    if target_tf == source_tf {
        let strictly_distinct_buckets = candles.windows(2).all(|w| {
            target_tf.bucket_start_timestamp(w[0].timestamp)
                < target_tf.bucket_start_timestamp(w[1].timestamp)
        });
        if strictly_distinct_buckets {
            return Ok(candles.to_vec());
        }
    }

    let ratio = (target_tf.seconds() / source_tf.seconds().max(1)).max(1) as usize;
    let mut out = Vec::with_capacity(candles.len().div_ceil(ratio));

    let mut cur_bucket_ts = target_tf.bucket_start_timestamp(candles[0].timestamp);
    let mut cur_open = candles[0].open;
    let mut cur_high = candles[0].high;
    let mut cur_low = candles[0].low;
    let mut cur_close = candles[0].close;
    let mut cur_vol = candles[0].volume;

    for c in &candles[1..] {
        let bucket_ts = target_tf.bucket_start_timestamp(c.timestamp);
        if bucket_ts == cur_bucket_ts {
            if c.high > cur_high {
                cur_high = c.high;
            }
            if c.low < cur_low {
                cur_low = c.low;
            }
            cur_close = c.close;
            cur_vol = (cur_vol + c.volume).min(1.0e300);
        } else {
            out.push(Candle {
                timestamp: cur_bucket_ts,
                date_label: target_tf.format_candle_label(cur_bucket_ts),
                open: cur_open,
                high: cur_high,
                low: cur_low,
                close: cur_close,
                volume: cur_vol,
            });
            cur_bucket_ts = bucket_ts;
            cur_open = c.open;
            cur_high = c.high;
            cur_low = c.low;
            cur_close = c.close;
            cur_vol = c.volume;
        }
    }

    out.push(Candle {
        timestamp: cur_bucket_ts,
        date_label: target_tf.format_candle_label(cur_bucket_ts),
        open: cur_open,
        high: cur_high,
        low: cur_low,
        close: cur_close,
        volume: cur_vol,
    });

    Ok(out)
}

/// Causality-safe prefix aggregation (`DEF-001` safeguard): aggregates only source bars with
/// `candle.timestamp <= cutoff_timestamp`, ensuring partial higher-timeframe bars (e.g. mid-week `1W`
/// as of Wednesday) never leak future Friday highs, lows, closes, or volume!
pub fn aggregate_ohlcv_up_to(
    candles: &[Candle],
    source_tf: Timeframe,
    target_tf: Timeframe,
    cutoff_timestamp: i64,
) -> Result<Vec<Candle>, AggregationError> {
    if !source_tf.can_aggregate_to(target_tf) {
        return Err(AggregationError::CannotDownsample {
            source: source_tf,
            target: target_tf,
        });
    }
    // Validate monotonic order across the full input first so corrupted inputs never hide past `cutoff`.
    for (idx, c) in candles.iter().enumerate() {
        if !c.is_valid()
            || !(MIN_SUPPORTED_EPOCH_SECS..=MAX_SUPPORTED_EPOCH_SECS).contains(&c.timestamp)
        {
            return Err(AggregationError::InvalidCandle { index: idx });
        }
        if idx > 0 && c.timestamp < candles[idx - 1].timestamp {
            return Err(AggregationError::NonMonotonicTimestamps {
                index: idx,
                prev: candles[idx - 1].timestamp,
                current: c.timestamp,
            });
        }
    }
    let end = candles.partition_point(|c| c.timestamp <= cutoff_timestamp);
    aggregate_ohlcv(&candles[..end], source_tf, target_tf)
}

/// Maps a continuous logical `bar_index` on `(candles, tf)` to a continuous UTC epoch timestamp (`f64`).
///
/// Forms a strictly monotonic piecewise-linear bijection with [`timestamp_to_bar_index`]:
/// - Exact bar centers `0..N` map to `candles[i].timestamp as f64`.
/// - Fractional intra-bar offsets in `[i, i + 1)` interpolate across `[t_i, t_{i+1})`.
/// - Left (`bar_index < 0`) and right (`bar_index >= N - 1`, including `right_offset` whitespace)
///   extrapolate smoothly with `tf.seconds()`.
#[must_use]
pub fn resolve_anchor_timestamp(bar_index: f64, candles: &[Candle], tf: Timeframe) -> Option<f64> {
    if candles.is_empty() || !bar_index.is_finite() {
        return None;
    }
    let step = tf.seconds() as f64;
    let n = candles.len();
    let first_ts = candles[0].timestamp as f64;
    if n == 1 || bar_index <= 0.0 {
        return Some(first_ts + bar_index * step);
    }
    let last_idx = (n - 1) as f64;
    if bar_index >= last_idx {
        let last_ts = candles[n - 1].timestamp as f64;
        return Some(last_ts + (bar_index - last_idx) * step);
    }

    let i = bar_index.floor() as usize;
    let frac = bar_index - (i as f64);
    let t0 = candles[i].timestamp as f64;
    let t1 = candles[i + 1].timestamp as f64;
    let span = if t1 > t0 { t1 - t0 } else { step };
    Some(t0 + frac * span)
}

/// Maps a continuous UTC epoch `timestamp` (`f64`) onto the continuous logical `bar_index` space
/// of `(candles, tf)` in $O(\log N)$ time via binary search.
#[must_use]
pub fn timestamp_to_bar_index(timestamp: f64, candles: &[Candle], tf: Timeframe) -> Option<f64> {
    if candles.is_empty() || !timestamp.is_finite() {
        return None;
    }
    let step = tf.seconds() as f64;
    let n = candles.len();
    let first_ts = candles[0].timestamp as f64;
    if timestamp <= first_ts || n == 1 {
        let idx = (timestamp - first_ts) / step;
        return Some(idx.clamp(-MAX_ABS_BAR_INDEX, MAX_ABS_BAR_INDEX));
    }

    let last_ts = candles[n - 1].timestamp as f64;
    if timestamp >= last_ts {
        let idx = ((n - 1) as f64) + (timestamp - last_ts) / step;
        return Some(idx.clamp(-MAX_ABS_BAR_INDEX, MAX_ABS_BAR_INDEX));
    }

    // Binary search largest `i` in `0..n-1` such that `candles[i].timestamp as f64 <= timestamp`.
    let upper = candles.partition_point(|c| (c.timestamp as f64) <= timestamp);
    let i = upper.saturating_sub(1).min(n - 2);
    let t0 = candles[i].timestamp as f64;
    let t1 = candles[i + 1].timestamp as f64;
    let frac = if t1 > t0 {
        ((timestamp - t0) / (t1 - t0)).clamp(0.0, 1.0)
    } else {
        0.0
    };
    Some(((i as f64) + frac).clamp(-MAX_ABS_BAR_INDEX, MAX_ABS_BAR_INDEX))
}

/// Remaps a continuous `timestamp` from `source_tf` onto `(candles, target_tf)`.
///
/// When switching downward (`source_tf.seconds() > target_tf.seconds()`) and `timestamp` sits on a
/// canonical `source_tf` bucket boundary (such as ISO-Monday `00:00:00 UTC` on `1W` or `00:00:00 UTC`
/// on `1D`), snaps to the first actual session candle inside that bucket so annotations placed on
/// `1W`/`1D` bars land cleanly on the opening trading bar instead of overnight/weekend whitespace!
#[must_use]
pub fn remap_timestamp_to_bar_index(
    timestamp: f64,
    source_tf: Timeframe,
    candles: &[Candle],
    target_tf: Timeframe,
) -> Option<(f64, f64)> {
    if candles.is_empty() || !timestamp.is_finite() {
        return None;
    }
    if source_tf.seconds() > target_tf.seconds()
        && timestamp >= (MIN_SUPPORTED_EPOCH_SECS as f64)
        && timestamp <= (MAX_SUPPORTED_EPOCH_SECS as f64)
    {
        let ts_i = timestamp.round() as i64;
        if (timestamp - (ts_i as f64)).abs() <= 1e-3
            && source_tf.bucket_start_timestamp(ts_i) == ts_i
        {
            let idx = candles.partition_point(|c| c.timestamp < ts_i);
            if let Some(first_bar) = candles.get(idx)
                && source_tf.bucket_start_timestamp(first_bar.timestamp) == ts_i
            {
                return Some((idx as f64, first_bar.timestamp as f64));
            }
        }
    }
    let idx = timestamp_to_bar_index(timestamp, candles, target_tf)?;
    Some((idx, timestamp))
}

/// Aggregates intraday Vietnam exchange candles (`1m..1h`) into Vietnam session-aligned half-day
/// bars in ICT (`UTC+7`):
/// - Morning session bucket: `09:00–11:30 ICT` (`02:00 UTC` bucket start)
/// - Afternoon session bucket: `13:00–14:45 ICT` (`06:00 UTC` bucket start)
///
/// Prevents UTC 4-hour boundaries (`04:00 UTC = 11:00 ICT`) from splitting the VN morning session.
pub fn aggregate_ohlcv_vn_session_h4(candles: &[Candle]) -> Result<Vec<Candle>, AggregationError> {
    const ICT_OFFSET_SECS: i64 = 7 * 3_600;
    if candles.is_empty() {
        return Ok(Vec::new());
    }
    for (idx, c) in candles.iter().enumerate() {
        if !c.is_valid()
            || !(MIN_SUPPORTED_EPOCH_SECS..=MAX_SUPPORTED_EPOCH_SECS).contains(&c.timestamp)
        {
            return Err(AggregationError::InvalidCandle { index: idx });
        }
        if idx > 0 && c.timestamp < candles[idx - 1].timestamp {
            return Err(AggregationError::NonMonotonicTimestamps {
                index: idx,
                prev: candles[idx - 1].timestamp,
                current: c.timestamp,
            });
        }
    }

    let vn_bucket = |ts: i64| -> i64 {
        let ict = ts.saturating_add(ICT_OFFSET_SECS);
        let day_start_ict = ict.div_euclid(86_400) * 86_400;
        let sec_of_day = ict - day_start_ict;
        // Morning (< 12:00 ICT) -> 09:00 ICT (02:00 UTC); Afternoon (>= 12:00 ICT) -> 13:00 ICT (06:00 UTC)
        let session_ict = if sec_of_day < 12 * 3_600 {
            day_start_ict + 9 * 3_600
        } else {
            day_start_ict + 13 * 3_600
        };
        session_ict.saturating_sub(ICT_OFFSET_SECS)
    };

    let mut out = Vec::with_capacity(candles.len().div_ceil(2));
    let mut cur_bucket = vn_bucket(candles[0].timestamp);
    let mut cur_open = candles[0].open;
    let mut cur_high = candles[0].high;
    let mut cur_low = candles[0].low;
    let mut cur_close = candles[0].close;
    let mut cur_vol = candles[0].volume;

    for c in &candles[1..] {
        let b = vn_bucket(c.timestamp);
        if b == cur_bucket {
            cur_high = cur_high.max(c.high);
            cur_low = cur_low.min(c.low);
            cur_close = c.close;
            cur_vol = (cur_vol + c.volume).min(1.0e300);
        } else {
            out.push(Candle {
                timestamp: cur_bucket,
                date_label: Timeframe::H4.format_candle_label(cur_bucket),
                open: cur_open,
                high: cur_high,
                low: cur_low,
                close: cur_close,
                volume: cur_vol,
            });
            cur_bucket = b;
            cur_open = c.open;
            cur_high = c.high;
            cur_low = c.low;
            cur_close = c.close;
            cur_vol = c.volume;
        }
    }

    out.push(Candle {
        timestamp: cur_bucket,
        date_label: Timeframe::H4.format_candle_label(cur_bucket),
        open: cur_open,
        high: cur_high,
        low: cur_low,
        close: cur_close,
        volume: cur_vol,
    });
    Ok(out)
}

/// Aggregates chronologically sorted `candles` into arbitrary custom buckets (`45m`, `2h`, `2D`, `3D`)
/// of width `bucket_secs >= 60`.
pub fn aggregate_ohlcv_custom_seconds(
    candles: &[Candle],
    bucket_secs: i64,
) -> Result<Vec<Candle>, AggregationError> {
    if candles.is_empty() {
        return Ok(Vec::new());
    }
    let step = bucket_secs.max(60);
    let is_intraday = step < 86_400;
    let mut out = Vec::with_capacity(candles.len());
    let mut cur_bucket = candles[0].timestamp.div_euclid(step) * step;
    let mut cur_open = candles[0].open;
    let mut cur_high = candles[0].high;
    let mut cur_low = candles[0].low;
    let mut cur_close = candles[0].close;
    let mut cur_vol = candles[0].volume;

    for (idx, c) in candles.iter().enumerate() {
        if !c.is_valid()
            || !(MIN_SUPPORTED_EPOCH_SECS..=MAX_SUPPORTED_EPOCH_SECS).contains(&c.timestamp)
        {
            return Err(AggregationError::InvalidCandle { index: idx });
        }
        if idx > 0 && c.timestamp < candles[idx - 1].timestamp {
            return Err(AggregationError::NonMonotonicTimestamps {
                index: idx,
                prev: candles[idx - 1].timestamp,
                current: c.timestamp,
            });
        }
        if idx == 0 {
            continue;
        }
        let b = c.timestamp.div_euclid(step) * step;
        if b == cur_bucket {
            cur_high = cur_high.max(c.high);
            cur_low = cur_low.min(c.low);
            cur_close = c.close;
            cur_vol = (cur_vol + c.volume).min(1.0e300);
        } else {
            out.push(Candle {
                timestamp: cur_bucket,
                date_label: if is_intraday {
                    format_utc_datetime_hm(cur_bucket)
                } else {
                    format_utc_date(cur_bucket)
                },
                open: cur_open,
                high: cur_high,
                low: cur_low,
                close: cur_close,
                volume: cur_vol,
            });
            cur_bucket = b;
            cur_open = c.open;
            cur_high = c.high;
            cur_low = c.low;
            cur_close = c.close;
            cur_vol = c.volume;
        }
    }

    out.push(Candle {
        timestamp: cur_bucket,
        date_label: if is_intraday {
            format_utc_datetime_hm(cur_bucket)
        } else {
            format_utc_date(cur_bucket)
        },
        open: cur_open,
        high: cur_high,
        low: cur_low,
        close: cur_close,
        volume: cur_vol,
    });
    Ok(out)
}

#[cfg(test)]
mod v3_timeframe_tests {
    use super::*;

    fn mk(ts: i64, o: f64, h: f64, l: f64, c: f64, v: f64) -> Candle {
        Candle {
            timestamp: ts,
            date_label: String::new(),
            open: o,
            high: h,
            low: l,
            close: c,
            volume: v,
        }
    }

    #[test]
    fn m30_and_mn1_bucket_alignment_and_labels() {
        assert_eq!(Timeframe::ALL.len(), 9);
        assert_eq!(Timeframe::parse_label("30m"), Some(Timeframe::M30));
        assert_eq!(Timeframe::parse_label("1M"), Some(Timeframe::MN1));
        assert_eq!(Timeframe::parse_label("1m"), Some(Timeframe::M1));
        assert_eq!(Timeframe::M30.seconds(), 1_800);
        assert!(Timeframe::M30.is_intraday());
        assert!(!Timeframe::MN1.is_intraday());

        // 2025-02-18 12:00:00 UTC -> monthly bucket must align to 2025-02-01 00:00:00 UTC
        let feb_18 = days_from_civil(2025, 2, 18) * 86_400 + 43_200;
        let feb_01 = days_from_civil(2025, 2, 1) * 86_400;
        assert_eq!(Timeframe::MN1.bucket_start_timestamp(feb_18), feb_01);
    }

    #[test]
    fn vn_session_h4_groups_morning_and_afternoon_without_splitting_11h_ict() {
        // 2026-04-20 in ICT (UTC+7):
        // Morning: 09:00 ICT (02:00 UTC), 10:00 ICT (03:00 UTC), 11:00 ICT (04:00 UTC)
        // Afternoon: 13:00 ICT (06:00 UTC), 14:00 ICT (07:00 UTC)
        let day = days_from_civil(2026, 4, 20) * 86_400;
        let bars = vec![
            mk(day + 2 * 3_600, 100.0, 102.0, 99.0, 101.0, 10.0),
            mk(day + 3 * 3_600, 101.0, 104.0, 100.0, 103.0, 20.0),
            mk(day + 4 * 3_600, 103.0, 105.0, 102.0, 104.0, 30.0),
            mk(day + 6 * 3_600, 104.0, 108.0, 103.5, 107.0, 40.0),
            mk(day + 7 * 3_600, 107.0, 109.0, 106.0, 108.5, 50.0),
        ];
        let h4 = aggregate_ohlcv_vn_session_h4(&bars).unwrap();
        assert_eq!(h4.len(), 2, "must produce exactly 1 morning bar + 1 afternoon bar");
        assert_eq!(h4[0].timestamp, day + 2 * 3_600);
        assert_eq!(h4[0].open, 100.0);
        assert_eq!(h4[0].high, 105.0);
        assert_eq!(h4[0].close, 104.0);
        assert_eq!(h4[0].volume, 60.0);
        assert_eq!(h4[1].timestamp, day + 6 * 3_600);
        assert_eq!(h4[1].open, 104.0);
        assert_eq!(h4[1].high, 109.0);
        assert_eq!(h4[1].close, 108.5);
        assert_eq!(h4[1].volume, 90.0);
    }

    #[test]
    fn custom_seconds_aggregation_rolls_up_2h_buckets() {
        let day = days_from_civil(2026, 4, 20) * 86_400;
        let bars = vec![
            mk(day, 10.0, 12.0, 9.0, 11.0, 5.0),
            mk(day + 3_600, 11.0, 13.0, 10.5, 12.5, 7.0),
            mk(day + 2 * 3_600, 12.5, 14.0, 12.0, 13.5, 9.0),
        ];
        let agg = aggregate_ohlcv_custom_seconds(&bars, 7_200).unwrap();
        assert_eq!(agg.len(), 2);
        assert_eq!(agg[0].high, 13.0);
        assert_eq!(agg[0].volume, 12.0);
        assert_eq!(agg[1].open, 12.5);
    }
}
