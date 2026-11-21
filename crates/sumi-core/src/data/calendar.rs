//! Zero-dependency proleptic Gregorian UTC calendar, multi-format timestamp parser,
//! series-level `TimestampUnit::Auto` resolver, and LWC `TickMarkWeight`-aligned adaptive
//! axis formatter (`Milestone R06`).

use serde::{Deserialize, Serialize};

/// Minimum UTC epoch second accepted by the Sumi calendar (`0001-01-01T00:00:00Z`).
pub const MIN_SUPPORTED_EPOCH_SECS: i64 = -62_135_596_800;

/// Maximum UTC epoch second accepted by the Sumi calendar (`9999-12-31T23:59:59Z`).
pub const MAX_SUPPORTED_EPOCH_SECS: i64 = 253_402_300_799;

/// Single-value `TimestampUnit::Auto` boundary between Unix seconds (`|v| < 10^10`, years `1653..2286`)
/// and Unix milliseconds (`10^10 <= |v| < 10^13`), preventing `1965..1978` ms feeds from being misread as seconds.
pub const AUTO_EPOCH_SECONDS_CAP: u128 = 10_000_000_000;
const AUTO_EPOCH_MILLIS_CAP: u128 = 10_000_000_000_000;
const AUTO_EPOCH_MICROS_CAP: u128 = 10_000_000_000_000_000;

/// Interpretation policy for numeric Unix epoch timestamps in CSV/JSON feeds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum TimestampUnit {
    /// Automatically detects seconds, milliseconds, microseconds, nanoseconds, or 8-digit `YYYYMMDD`.
    #[default]
    Auto,
    Seconds,
    Milliseconds,
    Microseconds,
    Nanoseconds,
}

impl TimestampUnit {
    #[inline]
    const fn decimal_power(self) -> i32 {
        match self {
            Self::Auto | Self::Seconds => 0,
            Self::Milliseconds => 3,
            Self::Microseconds => 6,
            Self::Nanoseconds => 9,
        }
    }
}

/// Parsed UTC timestamp together with provenance metadata used during timeframe inference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParsedTimestamp {
    /// Canonical UTC epoch timestamp in whole seconds (`[MIN_SUPPORTED_EPOCH_SECS..=MAX_SUPPORTED_EPOCH_SECS]`).
    pub utc_seconds: i64,
    /// `true` when the input explicitly carried an intraday time component or numeric Unix timestamp;
    /// `false` when parsed from a pure calendar date (`YYYYMMDD` or `YYYY-MM-DD`).
    pub has_time_component: bool,
}

/// Intermediate per-row timestamp representation enabling consistent series-level `TimestampUnit::Auto`
/// and all-or-nothing 8-digit `YYYYMMDD` resolution across multi-row CSV/JSON feeds.
#[derive(Debug, Clone)]
pub(crate) enum RawTimestampToken {
    AlreadyParsed(ParsedTimestamp),
    StringNumeric { row: usize, raw: String },
    JsonNumber { row: usize, raw: String },
}

/// Decomposed UTC date-time components (`year ∈ 1..=9999`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UtcDateTime {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
    /// ISO-8601 weekday: `0 = Monday .. 6 = Sunday`.
    pub weekday: u8,
}

/// Returns `true` if `year` is a proleptic Gregorian leap year.
#[inline]
#[must_use]
pub const fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

/// Number of calendar days in `(year, month)` (`month ∈ 1..=12`).
#[inline]
#[must_use]
pub const fn days_in_month_i32(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

/// Validates a calendar triple `(year, month, day)` within `0001-01-01..=9999-12-31`.
#[inline]
#[must_use]
pub const fn is_valid_ymd(year: i32, month: u32, day: u32) -> bool {
    if year < 1 || year > 9999 || month < 1 || month > 12 || day < 1 {
        return false;
    }
    day <= days_in_month_i32(year, month)
}

/// Exact proleptic Gregorian civil date to Unix epoch day count (`1970-01-01` -> `0`).
///
/// Implements Howard Hinnant's constant-time `days_from_civil` bijection.
#[inline]
#[must_use]
pub const fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let y = (year as i64) - if month <= 2 { 1 } else { 0 };
    let era = y.div_euclid(400);
    let yoe = y - era * 400; // [0, 399]
    let m = month as i64;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + (day as i64) - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    era * 146_097 + doe - 719_468
}

/// Exact Unix epoch day count (`1970-01-01` -> `0`) to proleptic Gregorian `(year, month, day)`.
///
/// Implements Howard Hinnant's constant-time `civil_from_days` bijection.
#[inline]
#[must_use]
pub const fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    let year = (y + if m <= 2 { 1 } else { 0 }) as i32;
    (year, m as u32, d as u32)
}

/// ISO-8601 weekday from Unix epoch day count (`0 = Monday .. 6 = Sunday`; `1970-01-01` = Thursday = `3`).
#[inline]
#[must_use]
pub const fn weekday_from_days(days: i64) -> u8 {
    (days + 3).rem_euclid(7) as u8
}

/// Decomposes a UTC epoch second into [`UtcDateTime`]. Clamps outside `[0001..=9999]` safely.
#[inline]
#[must_use]
pub fn decompose_utc(timestamp: i64) -> UtcDateTime {
    let clamped = timestamp.clamp(MIN_SUPPORTED_EPOCH_SECS, MAX_SUPPORTED_EPOCH_SECS);
    let days = clamped.div_euclid(86_400);
    let secs_of_day = clamped.rem_euclid(86_400) as u32;
    let (year, month, day) = civil_from_days(days);
    let hour = secs_of_day / 3_600;
    let rem = secs_of_day % 3_600;
    let minute = rem / 60;
    let second = rem % 60;
    let weekday = weekday_from_days(days);
    UtcDateTime {
        year,
        month,
        day,
        hour,
        minute,
        second,
        weekday,
    }
}

/// Formats `timestamp` (UTC seconds) as `"YYYY-MM-DD"`.
#[must_use]
pub fn format_utc_date(timestamp: i64) -> String {
    let dt = decompose_utc(timestamp);
    format!("{:04}-{:02}-{:02}", dt.year, dt.month, dt.day)
}

/// Formats `timestamp` (UTC seconds) as `"YYYY-MM-DD HH:MM"`.
#[must_use]
pub fn format_utc_datetime_hm(timestamp: i64) -> String {
    let dt = decompose_utc(timestamp);
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}",
        dt.year, dt.month, dt.day, dt.hour, dt.minute
    )
}

/// Formats a bottom time-axis tick label inspired by LWC v5.2.0 `TickMarkWeight` hierarchy:
/// - Daily/Weekly (`is_intraday == false`): `"YYYY-MM-DD"`
/// - Intraday (`is_intraday == true`):
///   - First visible tick or year boundary change: `"YYYY-MM-DD HH:MM"` (or `"YYYY-MM-DD"` if `00:00`)
///   - Day/month boundary change within same year: `"MM-DD HH:MM"`
///   - Intra-day tick within the same UTC date: `"HH:MM"`
#[must_use]
pub fn format_time_axis_tick(
    timestamp: i64,
    prev_tick_timestamp: Option<i64>,
    is_intraday: bool,
) -> String {
    let dt = decompose_utc(timestamp);
    if !is_intraday {
        return format!("{:04}-{:02}-{:02}", dt.year, dt.month, dt.day);
    }
    let fmt_full = || {
        if dt.hour == 0 && dt.minute == 0 {
            format!("{:04}-{:02}-{:02}", dt.year, dt.month, dt.day)
        } else {
            format!(
                "{:04}-{:02}-{:02} {:02}:{:02}",
                dt.year, dt.month, dt.day, dt.hour, dt.minute
            )
        }
    };
    match prev_tick_timestamp {
        None => fmt_full(),
        Some(prev_ts) => {
            let prev = decompose_utc(prev_ts);
            if dt.year != prev.year {
                fmt_full()
            } else if dt.month != prev.month || dt.day != prev.day {
                format!(
                    "{:02}-{:02} {:02}:{:02}",
                    dt.month, dt.day, dt.hour, dt.minute
                )
            } else {
                format!("{:02}:{:02}", dt.hour, dt.minute)
            }
        }
    }
}

/// Strictly parses an 8-digit CafeF/Metastock `YYYYMMDD` date string (`"20261005"` -> `2026-10-05T00:00:00Z`).
pub fn parse_ymd_8digit(raw: &str) -> Result<ParsedTimestamp, String> {
    let s = raw.trim();
    if s.len() != 8 || !s.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!("expected 8-digit YYYYMMDD date, got '{s}'"));
    }
    let year: i32 = s[0..4]
        .parse()
        .map_err(|_| format!("invalid year in '{s}'"))?;
    let month: u32 = s[4..6]
        .parse()
        .map_err(|_| format!("invalid month in '{s}'"))?;
    let day: u32 = s[6..8]
        .parse()
        .map_err(|_| format!("invalid day in '{s}'"))?;
    if !is_valid_ymd(year, month, day) {
        return Err(format!("invalid calendar date '{s}'"));
    }
    let utc_seconds = days_from_civil(year, month, day) * 86_400;
    Ok(ParsedTimestamp {
        utc_seconds,
        has_time_component: false,
    })
}

/// Parses an intraday time-of-day column (`<Time>`) paired with `<DTYYYYMMDD>` in 8-column
/// CafeF/Metastock/AmiBroker CSV exports:
/// - `"HH:MM"`, `"HH:MM:SS"`, `"HH:MM:SS.fff"`, `"HH:MM:SS,fff"` (with optional `Z` / `±HH:MM` / `±HHMM` / `±HH`)
/// - Compact digits `"HHMMSS"` (`091500`), `"HMMSS"` (`91500`), or `"HHMM"` (`0915`)
///
/// Returns signed UTC offset in seconds relative to `00:00:00Z` of the base date.
pub fn parse_time_of_day_hms(raw: &str) -> Result<i64, String> {
    let s = raw.trim();
    if s.is_empty() {
        return Err("empty time-of-day field".to_owned());
    }

    let (time_part, tz_offset_secs) = split_and_parse_tz(s, s)?;
    if time_part.contains(':') {
        let secs = parse_hms_fraction(time_part, tz_offset_secs, s)?;
        return Ok(i64::from(secs) - tz_offset_secs);
    }

    let (digits_str, frac_opt) = time_part
        .split_once('.')
        .or_else(|| time_part.split_once(','))
        .map_or((time_part, None), |(di, df)| (di, Some(df)));

    if let Some(frac) = frac_opt
        && (frac.is_empty() || !frac.bytes().all(|b| b.is_ascii_digit()))
    {
        return Err(format!("invalid fractional time-of-day '{s}'"));
    }
    if !digits_str.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!("invalid time-of-day '{s}'"));
    }

    let (hour, minute, raw_s): (u32, u32, u32) = match digits_str.len() {
        6 => (
            digits_str[0..2].parse().unwrap_or(99),
            digits_str[2..4].parse().unwrap_or(99),
            digits_str[4..6].parse().unwrap_or(99),
        ),
        5 => (
            digits_str[0..1].parse().unwrap_or(99),
            digits_str[1..3].parse().unwrap_or(99),
            digits_str[3..5].parse().unwrap_or(99),
        ),
        4 => (
            digits_str[0..2].parse().unwrap_or(99),
            digits_str[2..4].parse().unwrap_or(99),
            0,
        ),
        _ => return Err(format!("unrecognized compact time-of-day '{s}'")),
    };

    if hour > 23 || minute > 59 {
        return Err(format!("out-of-range hour/minute in '{s}'"));
    }
    validate_leap_or_normal_second(hour, minute, raw_s, tz_offset_secs, s)?;
    let second = raw_s.min(59);
    Ok(i64::from(hour * 3_600 + minute * 60 + second) - tz_offset_secs)
}

#[inline]
fn infer_auto_unit_from_abs(abs_val: u128) -> TimestampUnit {
    if abs_val < AUTO_EPOCH_SECONDS_CAP {
        TimestampUnit::Seconds
    } else if abs_val < AUTO_EPOCH_MILLIS_CAP {
        TimestampUnit::Milliseconds
    } else if abs_val < AUTO_EPOCH_MICROS_CAP {
        TimestampUnit::Microseconds
    } else {
        TimestampUnit::Nanoseconds
    }
}

/// Converts a raw `i128` numeric Unix epoch value into UTC seconds using `unit`.
///
/// Purely numeric `i128` inputs are NEVER hijacked as `YYYYMMDD` dates, preserving monotonic
/// continuity across 1970 Unix second feeds (`20_260_999`, `20_261_005`, ...).
pub fn parse_epoch_i128(value: i128, unit: TimestampUnit) -> Result<ParsedTimestamp, String> {
    let min_sec = i128::from(MIN_SUPPORTED_EPOCH_SECS);
    let max_sec = i128::from(MAX_SUPPORTED_EPOCH_SECS);
    let effective_unit = if unit == TimestampUnit::Auto {
        infer_auto_unit_from_abs(value.unsigned_abs())
    } else {
        unit
    };

    let utc_i128 = match effective_unit {
        TimestampUnit::Seconds | TimestampUnit::Auto => value,
        TimestampUnit::Milliseconds => value.div_euclid(1_000),
        TimestampUnit::Microseconds => value.div_euclid(1_000_000),
        TimestampUnit::Nanoseconds => value.div_euclid(1_000_000_000),
    };

    if utc_i128 < min_sec || utc_i128 > max_sec {
        return Err(format!(
            "Unix timestamp {value} ({effective_unit:?}) out of supported range [0001-01-01..9999-12-31]"
        ));
    }

    Ok(ParsedTimestamp {
        utc_seconds: utc_i128 as i64,
        has_time_component: true,
    })
}

/// Parses a single multi-format timestamp string:
/// - 8-digit `YYYYMMDD` (when `unit == TimestampUnit::Auto` and valid `1900..=2999` calendar date)
/// - `YYYY-MM-DD` or `YYYY/MM/DD`
/// - `ISO-8601` date-time (`YYYY-MM-DD[T ]HH:MM[:SS[.fff|,fff]][Z|±HH:MM|±HHMM|±HH]`)
/// - Numeric Unix epoch (`i128` or exact arbitrary-precision decimal/scientific notation)
pub fn parse_timestamp(raw: &str, unit: TimestampUnit) -> Result<ParsedTimestamp, String> {
    let s = raw.trim();
    if s.is_empty() {
        return Err("empty timestamp".to_owned());
    }

    // 1. Check 8-digit `YYYYMMDD` first on standalone string calls when `Auto` is active.
    if unit == TimestampUnit::Auto && is_valid_8digit_ymd_1900_2999(s) {
        return parse_ymd_8digit(s);
    }

    // 2. Pure integer Unix timestamp (preserves exact `i128` precision up to nanoseconds!).
    if is_ascii_signed_integer(s) {
        let val: i128 = s
            .parse()
            .map_err(|_| format!("integer timestamp overflow in '{s}'"))?;
        return parse_epoch_i128(val, unit);
    }

    // 3. Calendar date or ISO-8601 date-time (`YYYY-MM-DD...` or `YYYY/MM/DD...`).
    if s.len() >= 8 && s.bytes().take(4).all(|b| b.is_ascii_digit()) {
        let sep = s.as_bytes()[4];
        if sep == b'-' || sep == b'/' {
            return parse_iso_date_or_datetime(s, sep as char);
        }
    }

    // 4. Exact decimal/scientific numeric Unix timestamp (zero `f64` rounding on µs/ns decimals!).
    parse_decimal_epoch_token(s, unit)
}

/// Parses a validated JSON/CSV numeric epoch token with lossless decimal digit scaling (zero `f64` rounding).
pub fn parse_numeric_timestamp_token(
    token: &str,
    unit: TimestampUnit,
) -> Result<ParsedTimestamp, String> {
    let trimmed = token.trim();
    if is_ascii_signed_integer(trimmed) {
        let val: i128 = trimmed
            .parse()
            .map_err(|_| format!("integer timestamp overflow in '{trimmed}'"))?;
        return parse_epoch_i128(val, unit);
    }
    parse_decimal_epoch_token(trimmed, unit)
}

/// Classifies a timestamp string cell from CSV/JSON into [`RawTimestampToken`] for series-level resolution.
pub(crate) fn classify_string_timestamp_token(
    raw: &str,
    row: usize,
    unit: TimestampUnit,
) -> Result<RawTimestampToken, String> {
    let s = raw.trim();
    if s.is_empty() {
        return Err("empty timestamp".to_owned());
    }
    if s.len() >= 8 && s.bytes().take(4).all(|b| b.is_ascii_digit()) {
        let sep = s.as_bytes()[4];
        if sep == b'-' || sep == b'/' {
            return parse_iso_date_or_datetime(s, sep as char)
                .map(RawTimestampToken::AlreadyParsed);
        }
    }
    if unit != TimestampUnit::Auto {
        return parse_numeric_timestamp_token(s, unit).map(RawTimestampToken::AlreadyParsed);
    }
    // Validate that `s` is a syntactically valid numeric token before deferring to series resolution.
    let _ = inspect_decimal_magnitude(s)?;
    Ok(RawTimestampToken::StringNumeric {
        row,
        raw: s.to_owned(),
    })
}

/// Resolves an entire series of [`RawTimestampToken`]s with uniform `TimestampUnit::Auto` and
/// all-or-nothing 8-digit `YYYYMMDD` semantics (`CRITICAL-03` remediation).
pub(crate) fn resolve_series_timestamps(
    tokens: &[RawTimestampToken],
    unit: TimestampUnit,
) -> Result<Vec<ParsedTimestamp>, (usize, String)> {
    if tokens.is_empty() {
        return Ok(Vec::new());
    }

    if unit != TimestampUnit::Auto {
        let mut out = Vec::with_capacity(tokens.len());
        for tok in tokens {
            match tok {
                RawTimestampToken::AlreadyParsed(pts) => out.push(*pts),
                RawTimestampToken::StringNumeric { row, raw }
                | RawTimestampToken::JsonNumber { row, raw } => {
                    let pts = parse_numeric_timestamp_token(raw, unit).map_err(|m| (*row, m))?;
                    out.push(pts);
                }
            }
        }
        return Ok(out);
    }

    // 1. All-or-nothing 8-digit `YYYYMMDD` detection on string cells:
    //    Only treat 8-digit strings as `YYYYMMDD` when EVERY numeric token in the series is a
    //    `StringNumeric` valid `1900..=2999` `YYYYMMDD` date (never on JSON numbers or mixed 1970 epoch feeds!).
    let mut string_numeric_count = 0_usize;
    let mut json_number_count = 0_usize;
    let mut all_valid_ymd8 = true;

    for tok in tokens {
        match tok {
            RawTimestampToken::AlreadyParsed(_) => {}
            RawTimestampToken::StringNumeric { raw, .. } => {
                string_numeric_count += 1;
                if !is_valid_8digit_ymd_1900_2999(raw) {
                    all_valid_ymd8 = false;
                }
            }
            RawTimestampToken::JsonNumber { .. } => {
                json_number_count += 1;
                all_valid_ymd8 = false;
            }
        }
    }

    if string_numeric_count > 0 && json_number_count == 0 && all_valid_ymd8 {
        let mut out = Vec::with_capacity(tokens.len());
        for tok in tokens {
            match tok {
                RawTimestampToken::AlreadyParsed(pts) => out.push(*pts),
                RawTimestampToken::StringNumeric { row, raw } => {
                    let pts = parse_ymd_8digit(raw).map_err(|m| (*row, m))?;
                    out.push(pts);
                }
                RawTimestampToken::JsonNumber { .. } => unreachable!(),
            }
        }
        return Ok(out);
    }

    // 2. Infer ONE uniform `TimestampUnit` across all numeric epoch tokens in the series!
    let mut max_abs: u128 = 0;
    let mut int_samples: Vec<i128> = Vec::new();
    let mut all_exact_multiples_of_1000 = true;

    for tok in tokens {
        match tok {
            RawTimestampToken::AlreadyParsed(_) => {}
            RawTimestampToken::StringNumeric { row, raw }
            | RawTimestampToken::JsonNumber { row, raw } => {
                let (approx_abs, exact_int_opt) =
                    inspect_decimal_magnitude(raw).map_err(|m| (*row, m))?;
                if approx_abs > max_abs {
                    max_abs = approx_abs;
                }
                if let Some(iv) = exact_int_opt {
                    if iv.rem_euclid(1_000) != 0 {
                        all_exact_multiples_of_1000 = false;
                    }
                    int_samples.push(iv);
                } else {
                    all_exact_multiples_of_1000 = false;
                }
            }
        }
    }

    let mut series_unit = infer_auto_unit_from_abs(max_abs);
    // Disambiguate near-1970 millisecond series (`|v| < 631_152_000`, i.e., `1969-12..1970-01` in ms)
    // without misclassifying modern Unix-seconds series (`>= 1990`, e.g., `2024-10-04 = 1_728_000_000`)
    // whose timestamps happen to be exact multiples of `1_000` (`BUG-013`).
    if series_unit == TimestampUnit::Seconds
        && all_exact_multiples_of_1000
        && int_samples.len() >= 2
        && max_abs < 631_152_000_u128
    {
        int_samples.sort_unstable();
        int_samples.dedup();
        if int_samples.len() >= 2 {
            let min_dt = int_samples
                .windows(2)
                .map(|w| w[1].saturating_sub(w[0]))
                .min()
                .unwrap_or(0);
            if min_dt > 604_800 || (min_dt >= 1_000 && min_dt.rem_euclid(1_000) == 0) {
                series_unit = TimestampUnit::Milliseconds;
            }
        }
    }

    let mut out = Vec::with_capacity(tokens.len());
    for tok in tokens {
        match tok {
            RawTimestampToken::AlreadyParsed(pts) => out.push(*pts),
            RawTimestampToken::StringNumeric { row, raw }
            | RawTimestampToken::JsonNumber { row, raw } => {
                let pts = parse_numeric_timestamp_token(raw, series_unit).map_err(|m| (*row, m))?;
                out.push(pts);
            }
        }
    }
    Ok(out)
}

#[inline]
fn is_valid_8digit_ymd_1900_2999(s: &str) -> bool {
    if s.len() != 8 || !s.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    let year: i32 = s[0..4].parse().unwrap_or(0);
    let month: u32 = s[4..6].parse().unwrap_or(0);
    let day: u32 = s[6..8].parse().unwrap_or(0);
    (1900..=2999).contains(&year) && is_valid_ymd(year, month, day)
}

struct DecimalParts<'a> {
    negative: bool,
    int_digits: &'a [u8],
    frac_digits: &'a [u8],
    exp: i32,
}

fn split_decimal_parts(s: &str) -> Result<DecimalParts<'_>, String> {
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return Err("empty numeric timestamp".to_owned());
    }
    let mut pos = 0_usize;
    let mut negative = false;
    if bytes[pos] == b'-' {
        negative = true;
        pos += 1;
    } else if bytes[pos] == b'+' {
        pos += 1;
    }
    if pos >= bytes.len() {
        return Err(format!("unrecognized timestamp format '{s}'"));
    }

    let int_start = pos;
    while pos < bytes.len() && bytes[pos].is_ascii_digit() {
        pos += 1;
    }
    let int_digits = &bytes[int_start..pos];

    let frac_digits: &[u8] = if pos < bytes.len() && bytes[pos] == b'.' {
        pos += 1;
        let frac_start = pos;
        while pos < bytes.len() && bytes[pos].is_ascii_digit() {
            pos += 1;
        }
        &bytes[frac_start..pos]
    } else {
        &[]
    };

    if int_digits.is_empty() && frac_digits.is_empty() {
        return Err(format!("unrecognized timestamp format '{s}'"));
    }

    let mut exp: i32 = 0;
    if pos < bytes.len() && (bytes[pos] == b'e' || bytes[pos] == b'E') {
        pos += 1;
        if pos >= bytes.len() {
            return Err(format!("missing exponent digits in '{s}'"));
        }
        let mut exp_neg = false;
        if bytes[pos] == b'-' {
            exp_neg = true;
            pos += 1;
        } else if bytes[pos] == b'+' {
            pos += 1;
        }
        let exp_start = pos;
        while pos < bytes.len() && bytes[pos].is_ascii_digit() {
            pos += 1;
        }
        if exp_start == pos {
            return Err(format!("missing exponent digits in '{s}'"));
        }
        let exp_str = &s[exp_start..pos];
        let parsed_exp: i32 = exp_str
            .parse::<i32>()
            .map_err(|_| format!("exponent overflow in '{s}'"))?;
        exp = if exp_neg { -parsed_exp } else { parsed_exp };
    }

    if pos != bytes.len() {
        return Err(format!("unrecognized timestamp format '{s}'"));
    }

    Ok(DecimalParts {
        negative,
        int_digits,
        frac_digits,
        exp,
    })
}

/// Returns `(floored_abs_magnitude_capped, exact_integer_if_integral)` without `f64` rounding.
fn inspect_decimal_magnitude(s: &str) -> Result<(u128, Option<i128>), String> {
    if is_ascii_signed_integer(s) {
        let iv: i128 = s
            .parse()
            .map_err(|_| format!("integer timestamp overflow in '{s}'"))?;
        return Ok((iv.unsigned_abs(), Some(iv)));
    }
    let parts = split_decimal_parts(s)?;
    let mut sig: Vec<u8> = Vec::with_capacity(parts.int_digits.len() + parts.frac_digits.len());
    for &b in parts.int_digits.iter().chain(parts.frac_digits.iter()) {
        if !(sig.is_empty() && b == b'0') {
            sig.push(b);
        }
    }
    if sig.is_empty() {
        return Ok((0, Some(0)));
    }
    let effective_exp = parts
        .exp
        .checked_sub(parts.frac_digits.len() as i32)
        .ok_or_else(|| format!("exponent underflow in '{s}'"))?;
    let total_int_digits = (sig.len() as i32)
        .checked_add(effective_exp)
        .ok_or_else(|| format!("exponent overflow in '{s}'"))?;

    if total_int_digits <= 0 {
        return Ok((0, None));
    }
    if total_int_digits > 36 {
        return Ok((u128::MAX, None));
    }
    let int_len = total_int_digits as usize;
    let mut abs_val: u128 = 0;
    for idx in 0..int_len {
        let d = sig.get(idx).map_or(0_u128, |&b| u128::from(b - b'0'));
        abs_val = abs_val
            .checked_mul(10)
            .and_then(|v| v.checked_add(d))
            .unwrap_or(u128::MAX);
    }
    let has_frac = int_len < sig.len() && sig[int_len..].iter().any(|&b| b != b'0');
    let exact_int = if !has_frac && abs_val <= (i128::MAX as u128) {
        let signed = if parts.negative {
            -(abs_val as i128)
        } else {
            abs_val as i128
        };
        Some(signed)
    } else {
        None
    };
    Ok((abs_val, exact_int))
}

fn parse_decimal_epoch_token(s: &str, unit: TimestampUnit) -> Result<ParsedTimestamp, String> {
    let parts = split_decimal_parts(s)?;
    let mut sig: Vec<u8> = Vec::with_capacity(parts.int_digits.len() + parts.frac_digits.len());
    for &b in parts.int_digits.iter().chain(parts.frac_digits.iter()) {
        if !(sig.is_empty() && b == b'0') {
            sig.push(b);
        }
    }
    if sig.is_empty() {
        return Ok(ParsedTimestamp {
            utc_seconds: 0,
            has_time_component: true,
        });
    }

    let effective_exp = parts
        .exp
        .checked_sub(parts.frac_digits.len() as i32)
        .ok_or_else(|| format!("exponent underflow in '{s}'"))?;

    let resolved_unit = if unit == TimestampUnit::Auto {
        let (approx_abs, _) = inspect_decimal_magnitude(s)?;
        infer_auto_unit_from_abs(approx_abs)
    } else {
        unit
    };

    let shifted_exp = effective_exp
        .checked_sub(resolved_unit.decimal_power())
        .ok_or_else(|| format!("exponent underflow in '{s}'"))?;
    let scaled_int_digits = (sig.len() as i32)
        .checked_add(shifted_exp)
        .ok_or_else(|| format!("exponent overflow in '{s}'"))?;

    if scaled_int_digits > 25 {
        return Err(format!(
            "numeric timestamp '{s}' out of supported calendar range"
        ));
    }

    let utc_i128: i128 = if scaled_int_digits <= 0 {
        if parts.negative { -1 } else { 0 }
    } else {
        let int_len = scaled_int_digits as usize;
        let mut abs_int: u128 = 0;
        for idx in 0..int_len {
            let d = sig.get(idx).map_or(0_u128, |&b| u128::from(b - b'0'));
            abs_int = abs_int * 10 + d;
        }
        let has_frac_remainder = int_len < sig.len() && sig[int_len..].iter().any(|&b| b != b'0');
        if parts.negative {
            -(abs_int as i128) - i128::from(has_frac_remainder)
        } else {
            abs_int as i128
        }
    };

    if utc_i128 < i128::from(MIN_SUPPORTED_EPOCH_SECS)
        || utc_i128 > i128::from(MAX_SUPPORTED_EPOCH_SECS)
    {
        return Err(format!(
            "numeric timestamp '{s}' out of supported calendar range"
        ));
    }

    Ok(ParsedTimestamp {
        utc_seconds: utc_i128 as i64,
        has_time_component: true,
    })
}

fn is_ascii_signed_integer(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    let digits = if bytes[0] == b'+' || bytes[0] == b'-' {
        &bytes[1..]
    } else {
        bytes
    };
    !digits.is_empty() && digits.iter().all(u8::is_ascii_digit)
}

fn parse_iso_date_or_datetime(s: &str, date_sep: char) -> Result<ParsedTimestamp, String> {
    let mut parts = s.splitn(3, date_sep);
    let y_str = parts
        .next()
        .ok_or_else(|| format!("missing year in '{s}'"))?;
    let m_str = parts
        .next()
        .ok_or_else(|| format!("missing month in '{s}'"))?;
    let rest_after_month = parts
        .next()
        .ok_or_else(|| format!("missing day in '{s}'"))?;

    if y_str.len() != 4 || !y_str.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!("invalid 4-digit year in '{s}'"));
    }
    if !(1..=2).contains(&m_str.len()) || !m_str.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!("invalid month in '{s}'"));
    }

    let day_len = rest_after_month
        .bytes()
        .take_while(u8::is_ascii_digit)
        .count();
    if !(1..=2).contains(&day_len) {
        return Err(format!("invalid day in '{s}'"));
    }
    let d_str = &rest_after_month[..day_len];
    let tail = rest_after_month[day_len..].trim();

    let year: i32 = y_str
        .parse()
        .map_err(|_| format!("invalid year in '{s}'"))?;
    let month: u32 = m_str
        .parse()
        .map_err(|_| format!("invalid month in '{s}'"))?;
    let day: u32 = d_str.parse().map_err(|_| format!("invalid day in '{s}'"))?;
    if !is_valid_ymd(year, month, day) {
        return Err(format!(
            "out-of-range calendar date {year:04}-{month:02}-{day:02} in '{s}'"
        ));
    }

    let base_day_secs = days_from_civil(year, month, day) * 86_400;
    if tail.is_empty() {
        return Ok(ParsedTimestamp {
            utc_seconds: base_day_secs,
            has_time_component: false,
        });
    }

    let raw_tail = &rest_after_month[day_len..];
    let first_ch = raw_tail
        .chars()
        .next()
        .ok_or_else(|| format!("unexpected trailing characters in '{s}'"))?;
    if first_ch != 'T' && first_ch != 't' && !first_ch.is_ascii_whitespace() {
        return Err(format!("invalid date/time separator in '{s}'"));
    }
    let time_and_tz = raw_tail[first_ch.len_utf8()..].trim();
    if time_and_tz.is_empty() {
        return Err(format!("missing time after separator in '{s}'"));
    }

    let (time_part, tz_offset_secs) = split_and_parse_tz(time_and_tz, s)?;
    let time_of_day_secs = parse_hms_fraction(time_part, tz_offset_secs, s)?;

    let utc_seconds = base_day_secs + i64::from(time_of_day_secs) - tz_offset_secs;
    if !(MIN_SUPPORTED_EPOCH_SECS..=MAX_SUPPORTED_EPOCH_SECS).contains(&utc_seconds) {
        return Err(format!(
            "UTC-adjusted timestamp '{s}' falls outside supported calendar [0001..=9999]"
        ));
    }

    Ok(ParsedTimestamp {
        utc_seconds,
        has_time_component: true,
    })
}

fn split_and_parse_tz<'a>(time_and_tz: &'a str, full: &str) -> Result<(&'a str, i64), String> {
    if let Some(stripped) = time_and_tz
        .strip_suffix('Z')
        .or_else(|| time_and_tz.strip_suffix('z'))
    {
        return Ok((stripped.trim_end(), 0));
    }

    if let Some(sign_pos) = time_and_tz
        .char_indices()
        .skip(1)
        .find_map(|(idx, ch)| ((ch == '+' || ch == '-') && idx >= 4).then_some(idx))
    {
        let time_part = time_and_tz[..sign_pos].trim_end();
        let tz_raw = time_and_tz[sign_pos..].trim();
        let sign: i64 = if tz_raw.as_bytes()[0] == b'-' { -1 } else { 1 };
        let offset_digits = &tz_raw[1..];
        let (tz_h, tz_m): (u32, u32) = if let Some((h_s, m_s)) = offset_digits.split_once(':') {
            if h_s.len() != 2
                || m_s.len() != 2
                || !h_s.bytes().all(|b| b.is_ascii_digit())
                || !m_s.bytes().all(|b| b.is_ascii_digit())
            {
                return Err(format!("malformed timezone offset '{tz_raw}' in '{full}'"));
            }
            (h_s.parse().unwrap_or(99), m_s.parse().unwrap_or(99))
        } else if offset_digits.len() == 4 && offset_digits.bytes().all(|b| b.is_ascii_digit()) {
            (
                offset_digits[0..2].parse().unwrap_or(99),
                offset_digits[2..4].parse().unwrap_or(99),
            )
        } else if offset_digits.len() == 2 && offset_digits.bytes().all(|b| b.is_ascii_digit()) {
            (offset_digits.parse().unwrap_or(99), 0)
        } else {
            return Err(format!("malformed timezone offset '{tz_raw}' in '{full}'"));
        };

        if tz_h > 23 || tz_m > 59 {
            return Err(format!(
                "out-of-range timezone offset '{tz_raw}' in '{full}'"
            ));
        }
        let offset_secs = sign * (i64::from(tz_h) * 3_600 + i64::from(tz_m) * 60);
        return Ok((time_part, offset_secs));
    }

    Ok((time_and_tz, 0))
}

fn validate_leap_or_normal_second(
    hour: u32,
    minute: u32,
    raw_s: u32,
    tz_offset_secs: i64,
    full: &str,
) -> Result<(), String> {
    if raw_s < 60 {
        return Ok(());
    }
    if raw_s > 60 {
        return Err(format!("out-of-range seconds '{raw_s}' in '{full}'"));
    }
    // ISO-8601 leap second `:60` is valid ONLY at `23:59:60 UTC` (accounting for timezone offset)!
    let utc_minute_of_day =
        (i64::from(hour * 3_600 + minute * 60) - tz_offset_secs).rem_euclid(86_400);
    if utc_minute_of_day == 23 * 3_600 + 59 * 60 {
        Ok(())
    } else {
        Err(format!(
            "invalid leap second ':60' outside 23:59 UTC in '{full}'"
        ))
    }
}

fn parse_hms_fraction(time_part: &str, tz_offset_secs: i64, full: &str) -> Result<u32, String> {
    let cols: Vec<&str> = time_part.split(':').collect();
    if cols.len() != 2 && cols.len() != 3 {
        return Err(format!("invalid time-of-day '{time_part}' in '{full}'"));
    }
    let h_str = cols[0];
    let m_str = cols[1];
    if !(1..=2).contains(&h_str.len())
        || m_str.len() != 2
        || !h_str.bytes().all(|b| b.is_ascii_digit())
        || !m_str.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(format!("invalid HH:MM '{time_part}' in '{full}'"));
    }
    let hour: u32 = h_str.parse().unwrap_or(99);
    let minute: u32 = m_str.parse().unwrap_or(99);
    if hour > 23 || minute > 59 {
        return Err(format!(
            "out-of-range hour/minute '{time_part}' in '{full}'"
        ));
    }

    let second: u32 = if cols.len() == 3 {
        let sec_raw = cols[2];
        let (sec_int_str, frac_opt) = sec_raw
            .split_once('.')
            .or_else(|| sec_raw.split_once(','))
            .map_or((sec_raw, None), |(si, sf)| (si, Some(sf)));
        if sec_int_str.len() != 2 || !sec_int_str.bytes().all(|b| b.is_ascii_digit()) {
            return Err(format!("invalid seconds '{sec_raw}' in '{full}'"));
        }
        if let Some(frac) = frac_opt
            && (frac.is_empty() || !frac.bytes().all(|b| b.is_ascii_digit()))
        {
            return Err(format!(
                "invalid fractional seconds '{sec_raw}' in '{full}'"
            ));
        }
        let raw_s: u32 = sec_int_str.parse().unwrap_or(99);
        validate_leap_or_normal_second(hour, minute, raw_s, tz_offset_secs, full)?;
        raw_s.min(59)
    } else {
        0
    };

    Ok(hour * 3_600 + minute * 60 + second)
}
