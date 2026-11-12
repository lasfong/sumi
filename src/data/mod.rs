//! Zero-dependency, `wasm32-unknown-unknown`-safe OHLCV Data Loader & Multi-Timeframe
//! Aggregation Engine (`Milestone R06 / Wave 5`).
//!
//! Provides:
//! - [`calendar`]: Proleptic Gregorian UTC calendar, multi-format timestamp parser (`YYYYMMDD`,
//!   `YYYY-MM-DD`, full `ISO-8601` with timezone offsets, CafeF intraday `<Time>`, Unix `s`/`ms`/`µs`/`ns`
//!   with series-level `Auto` resolution), and LWC-aligned adaptive time-axis tick formatting.
//! - [`csv_loader`]: Streaming RFC-4180 CSV loader supporting UTF-8 BOM, `CRLF`/`LF`, auto-delimiter
//!   (`,`, `;`, `\t`), CafeF `<Ticker>,<DTYYYYMMDD>[,<Time>]` headers, volume-optional `SPY.csv`, and strict
//!   multi-symbol isolation.
//! - [`json_loader`]: Streaming RFC-8259 JSON loader supporting object arrays, kline tuple arrays,
//!   and `{symbol, timeframe, candles|data|klines|bars|ohlcv|results}` envelopes with `MAX_JSON_DEPTH = 16`.
//! - [`timeframe`]: Upward timeframe lattice (`1m`, `5m`, `15m`, `1h`, `4h`, `1D`, `1W`), ISO-Monday
//!   `00:00:00 UTC` weekly alignment, causal `aggregate_ohlcv_up_to`, and $O(\log N)$ timestamp
//!   <-> `bar_index` projection.
//! - [`presets`]: Bundled workstation feeds + deterministic synthetic generators.

pub mod calendar;
pub mod csv_loader;
pub mod json_loader;
pub mod presets;
pub mod synthetic;
pub mod timeframe;

#[cfg(test)]
mod tests;

use std::collections::HashSet;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::model::Candle;

pub use calendar::{
    AUTO_EPOCH_SECONDS_CAP, MAX_SUPPORTED_EPOCH_SECS, MIN_SUPPORTED_EPOCH_SECS, ParsedTimestamp,
    TimestampUnit, UtcDateTime, civil_from_days, days_from_civil, days_in_month_i32, decompose_utc,
    format_time_axis_tick, format_utc_date, format_utc_datetime_hm, is_leap_year, is_valid_ymd,
    parse_epoch_i128, parse_time_of_day_hms, parse_timestamp, parse_ymd_8digit, weekday_from_days,
};
pub use csv_loader::load_ohlcv_csv;
pub use json_loader::{MAX_JSON_DEPTH, load_ohlcv_json};
pub use presets::{
    DataSourcePreset, VN30_BASKET_SYMBOLS, WatchlistFeedEntry, build_btcusdt_15m_json_document,
    generate_intraday_vn30f_1m_report, generate_vn30_basket_report,
    load_bundled_watchlist_universe, load_preset_dataset,
};
pub use synthetic::generate_synthetic_ohlcv;
pub use timeframe::{
    AggregationError, Timeframe, aggregate_ohlcv, aggregate_ohlcv_up_to, infer_base_timeframe,
    remap_timestamp_to_bar_index, resolve_anchor_timestamp, timestamp_to_bar_index,
};

/// Maximum raw input document size accepted by CSV/JSON loaders (`64 MiB`).
pub const MAX_IMPORT_BYTES: usize = 64 * 1024 * 1024;

/// Maximum number of candle records retained for a single instrument import (`500,000` bars).
pub const MAX_IMPORT_RECORDS: usize = 500_000;

/// Maximum total records scanned across multi-symbol CSV/JSON documents (`2,000,000` rows).
pub const MAX_SCANNED_RECORDS: usize = 2_000_000;

/// Maximum distinct symbols tracked in `DataLoadReport::available_symbols` (`256` tickers).
pub const MAX_DISCOVERED_SYMBOLS: usize = 256;

/// Maximum absolute price accepted in normalized OHLCV series (aligned with `PriceScale` domain guard).
pub const MAX_ABS_PRICE: f64 = 1.0e15;

/// Maximum volume accepted per bar before rejecting input.
pub const MAX_CANDLE_VOLUME: f64 = 1.0e18;

/// Deduplication policy when multiple input rows share the exact same UTC epoch second.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum DuplicateTimestampPolicy {
    /// Retains the last row encountered in input order (standard exchange backfill / amendment semantics).
    #[default]
    KeepLast,
    /// Retains the first row encountered in input order.
    KeepFirst,
    /// Rejects the import if two duplicate-timestamp rows carry differing OHLCV values.
    RejectConflict,
}

/// Validation/repair policy for the candlestick envelope invariant `low <= min(open, close) <= max(open, close) <= high`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum OhlcEnvelopePolicy {
    /// Rejects any row violating `low <= min(open, close) <= max(open, close) <= high`.
    #[default]
    RejectRow,
    /// Repairs minor vendor tick inversion by clamping `high = high.max(open.max(close))` and
    /// `low = low.min(open.min(close))`, recording the count in [`DataLoadReport::repaired_envelope_count`].
    ClampToBody,
}

/// Input format selector for [`load_ohlcv_bytes`] / `load_ohlcv_file`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum DataFileFormat {
    #[default]
    Auto,
    Csv,
    Json,
}

/// Configuration options controlling CSV/JSON OHLCV parsing and normalization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataLoadOptions {
    /// Selects a specific ticker when importing multi-symbol CSV/JSON files (case-insensitive).
    /// When `None`, defaults to the first symbol encountered in the file.
    pub symbol_filter: Option<String>,
    /// Fallback symbol label when the input file has no symbol column/field (`"SPY"`, `"CUSTOM"`, ...).
    pub default_symbol: Option<String>,
    /// Optional explicit base timeframe override (when `None`, inferred from envelope metadata or lower-quartile `Δt`).
    pub explicit_timeframe: Option<Timeframe>,
    /// Numeric timestamp unit policy (`Auto`, `Seconds`, `Milliseconds`, `Microseconds`, `Nanoseconds`).
    pub timestamp_unit: TimestampUnit,
    /// Deduplication policy for colliding timestamps.
    pub duplicate_policy: DuplicateTimestampPolicy,
    /// Envelope validation/repair policy for `low <= min(open, close) <= max(open, close) <= high`.
    pub envelope_policy: OhlcEnvelopePolicy,
    /// Multiplier applied to `(open, high, low, close)` (e.g. `1000.0` for legacy CafeF thousand-VND quotes).
    pub price_multiplier: f64,
    /// Requires strictly positive prices (`> 0.0`, default `true` for standard equity/crypto/FX candles).
    pub require_positive_prices: bool,
}

impl Default for DataLoadOptions {
    fn default() -> Self {
        Self {
            symbol_filter: None,
            default_symbol: None,
            explicit_timeframe: None,
            timestamp_unit: TimestampUnit::Auto,
            duplicate_policy: DuplicateTimestampPolicy::KeepLast,
            envelope_policy: OhlcEnvelopePolicy::RejectRow,
            price_multiplier: 1.0,
            require_positive_prices: true,
        }
    }
}

impl DataLoadOptions {
    pub fn validate(&self) -> Result<(), DataLoadError> {
        if !self.price_multiplier.is_finite()
            || self.price_multiplier <= 0.0
            || self.price_multiplier > 1.0e9
        {
            return Err(DataLoadError::InvalidConfiguration(format!(
                "price_multiplier must be finite in (0.0, 1.0e9], got {}",
                self.price_multiplier
            )));
        }
        if let Some(ref sym) = self.symbol_filter
            && sym.trim().is_empty()
        {
            return Err(DataLoadError::InvalidConfiguration(
                "symbol_filter must not be empty whitespace".to_owned(),
            ));
        }
        Ok(())
    }
}

/// Normalized output and audit counters produced by [`load_ohlcv_csv`] and [`load_ohlcv_json`].
#[derive(Debug, Clone, PartialEq)]
pub struct DataLoadReport {
    pub symbol: String,
    pub available_symbols: Vec<String>,
    pub base_timeframe: Timeframe,
    pub candles: Vec<Candle>,
    pub raw_row_count: usize,
    pub deduplicated_count: usize,
    pub repaired_envelope_count: usize,
    pub filtered_other_symbol_rows: usize,
}

/// Structured errors returned by the CSV/JSON OHLCV loading pipeline.
#[derive(Debug, Clone, PartialEq)]
pub enum DataLoadError {
    EmptyDataset,
    InputTooLarge {
        bytes: usize,
        max_bytes: usize,
    },
    TooManyRecords {
        max_records: usize,
    },
    InvalidUtf8,
    InvalidConfiguration(String),
    MissingRequiredColumns {
        missing: String,
    },
    SymbolNotFound {
        requested: String,
        available: Vec<String>,
    },
    MalformedCsv {
        line: usize,
        message: String,
    },
    MalformedJson {
        offset: usize,
        message: String,
    },
    InvalidFieldValue {
        row: usize,
        field: &'static str,
        message: String,
    },
    InvalidOhlcEnvelope {
        row: usize,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
    },
    DuplicateTimestampConflict {
        timestamp: i64,
        first_row: usize,
        second_row: usize,
    },
    IoError(String),
}

impl fmt::Display for DataLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyDataset => f.write_str("dataset contains no OHLCV records"),
            Self::InputTooLarge { bytes, max_bytes } => {
                write!(
                    f,
                    "input size {bytes} bytes exceeds limit {max_bytes} bytes"
                )
            }
            Self::TooManyRecords { max_records } => {
                write!(f, "record count exceeds limit of {max_records} candles")
            }
            Self::InvalidUtf8 => f.write_str("input bytes are not valid UTF-8"),
            Self::InvalidConfiguration(msg) => write!(f, "invalid load configuration: {msg}"),
            Self::MissingRequiredColumns { missing } => {
                write!(f, "missing required OHLCV columns/fields: {missing}")
            }
            Self::SymbolNotFound {
                requested,
                available,
            } => write!(
                f,
                "symbol '{requested}' not found (available: {})",
                available.join(", ")
            ),
            Self::MalformedCsv { line, message } => {
                write!(f, "malformed CSV at line {line}: {message}")
            }
            Self::MalformedJson { offset, message } => {
                write!(f, "malformed JSON at byte offset {offset}: {message}")
            }
            Self::InvalidFieldValue {
                row,
                field,
                message,
            } => write!(f, "invalid '{field}' at row {row}: {message}"),
            Self::InvalidOhlcEnvelope {
                row,
                open,
                high,
                low,
                close,
            } => write!(
                f,
                "OHLC envelope violation at row {row}: open={open}, high={high}, low={low}, close={close}"
            ),
            Self::DuplicateTimestampConflict {
                timestamp,
                first_row,
                second_row,
            } => write!(
                f,
                "conflicting duplicate timestamp {timestamp} between rows {first_row} and {second_row}"
            ),
            Self::IoError(msg) => write!(f, "file I/O error: {msg}"),
        }
    }
}

impl std::error::Error for DataLoadError {}

/// Loads OHLCV candles from raw UTF-8 bytes (`wasm32`-compatible), auto-detecting JSON vs CSV when `format == Auto`.
pub fn load_ohlcv_bytes(
    bytes: &[u8],
    format: DataFileFormat,
    options: &DataLoadOptions,
) -> Result<DataLoadReport, DataLoadError> {
    if bytes.len() > MAX_IMPORT_BYTES {
        return Err(DataLoadError::InputTooLarge {
            bytes: bytes.len(),
            max_bytes: MAX_IMPORT_BYTES,
        });
    }
    let text = std::str::from_utf8(bytes).map_err(|_| DataLoadError::InvalidUtf8)?;
    let stripped = text.strip_prefix('\u{FEFF}').unwrap_or(text);
    let resolved_format = match format {
        DataFileFormat::Csv => DataFileFormat::Csv,
        DataFileFormat::Json => DataFileFormat::Json,
        DataFileFormat::Auto => {
            let first_non_ws = stripped.trim_start().as_bytes().first().copied();
            if matches!(first_non_ws, Some(b'[' | b'{')) {
                DataFileFormat::Json
            } else {
                DataFileFormat::Csv
            }
        }
    };
    match resolved_format {
        DataFileFormat::Json => load_ohlcv_json(stripped, options),
        DataFileFormat::Csv | DataFileFormat::Auto => load_ohlcv_csv(stripped, options),
    }
}

/// Native filesystem convenience helper loading a `.csv`, `.tsv`, `.txt`, or `.json` OHLCV file.
/// Opens a single file handle, checks metadata length, and bounds `read_to_end` via `io::Read::take`
/// so FIFOs, special files (`/dev/zero`), or concurrently growing logs never exhaust RAM (`HIGH-06`).
#[cfg(not(target_arch = "wasm32"))]
pub fn load_ohlcv_file(
    path: impl AsRef<std::path::Path>,
    options: &DataLoadOptions,
) -> Result<DataLoadReport, DataLoadError> {
    use std::io::Read as _;

    let path_ref = path.as_ref();
    let file = std::fs::File::open(path_ref).map_err(|e| DataLoadError::IoError(e.to_string()))?;
    let meta = file
        .metadata()
        .map_err(|e| DataLoadError::IoError(e.to_string()))?;
    if meta.len() > (MAX_IMPORT_BYTES as u64) {
        return Err(DataLoadError::InputTooLarge {
            bytes: usize::try_from(meta.len()).unwrap_or(usize::MAX),
            max_bytes: MAX_IMPORT_BYTES,
        });
    }

    let cap_hint = usize::try_from(meta.len().min(MAX_IMPORT_BYTES as u64)).unwrap_or(0);
    let mut bytes = Vec::with_capacity(cap_hint);
    file.take((MAX_IMPORT_BYTES as u64) + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| DataLoadError::IoError(e.to_string()))?;
    if bytes.len() > MAX_IMPORT_BYTES {
        return Err(DataLoadError::InputTooLarge {
            bytes: bytes.len(),
            max_bytes: MAX_IMPORT_BYTES,
        });
    }

    let mut derived_opts = options.clone();
    if derived_opts.default_symbol.is_none()
        && let Some(stem) = path_ref.file_stem().and_then(|s| s.to_str())
    {
        let clean = stem
            .split('.')
            .next()
            .unwrap_or(stem)
            .trim()
            .to_ascii_uppercase();
        if !clean.is_empty() {
            derived_opts.default_symbol = Some(clean);
        }
    }

    let ext = path_ref
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let fmt = match ext.as_str() {
        "json" => DataFileFormat::Json,
        "csv" | "tsv" | "txt" => DataFileFormat::Csv,
        _ => DataFileFormat::Auto,
    };

    load_ohlcv_bytes(&bytes, fmt, &derived_opts)
}

#[derive(Debug, Clone)]
pub(crate) struct RawCandleRecord {
    pub source_row: usize,
    pub timestamp: i64,
    pub has_time_component: bool,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

/// $O(1)$ case-insensitive symbol discovery tracker bounded by [`MAX_DISCOVERED_SYMBOLS`].
#[derive(Debug, Default)]
pub(crate) struct SymbolTracker {
    seen_lower: HashSet<String>,
    ordered: Vec<String>,
}

impl SymbolTracker {
    pub fn record(&mut self, candidate: &str) {
        if self.ordered.len() >= MAX_DISCOVERED_SYMBOLS {
            return;
        }
        let lower = candidate.to_ascii_lowercase();
        if self.seen_lower.insert(lower) {
            self.ordered.push(candidate.to_owned());
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.ordered.is_empty()
    }

    pub fn into_vec_with_fallback(mut self, fallback: &str) -> Vec<String> {
        if self.ordered.is_empty() {
            self.ordered.push(fallback.to_owned());
        }
        self.ordered
    }
}

#[inline]
fn rel_identical(a: f64, b: f64, rel_tol: f64) -> bool {
    if a == b {
        return true;
    }
    let scale = a.abs().max(b.abs());
    (a - b).abs() <= scale * rel_tol
}

pub(crate) fn normalize_loaded_candles(
    mut raw_records: Vec<RawCandleRecord>,
    symbol: String,
    available_symbols: Vec<String>,
    filtered_other_symbol_rows: usize,
    envelope_timeframe: Option<Timeframe>,
    options: &DataLoadOptions,
) -> Result<DataLoadReport, DataLoadError> {
    if raw_records.is_empty() {
        return Err(DataLoadError::EmptyDataset);
    }

    let raw_row_count = raw_records.len();
    let mult = options.price_multiplier;
    let mut repaired_envelope_count = 0_usize;
    let mut any_intraday_time = false;

    for rec in &mut raw_records {
        if !(MIN_SUPPORTED_EPOCH_SECS..=MAX_SUPPORTED_EPOCH_SECS).contains(&rec.timestamp) {
            return Err(DataLoadError::InvalidFieldValue {
                row: rec.source_row,
                field: "timestamp",
                message: format!("timestamp {} out of supported UTC range", rec.timestamp),
            });
        }
        if rec.has_time_component {
            any_intraday_time = true;
        }

        rec.open *= mult;
        rec.high *= mult;
        rec.low *= mult;
        rec.close *= mult;

        for (field, val) in [
            ("open", rec.open),
            ("high", rec.high),
            ("low", rec.low),
            ("close", rec.close),
        ] {
            if !val.is_finite() || val.abs() > MAX_ABS_PRICE {
                return Err(DataLoadError::InvalidFieldValue {
                    row: rec.source_row,
                    field,
                    message: format!("price {val} exceeds finite domain guard (±1e15)"),
                });
            }
            if options.require_positive_prices && val <= 0.0 {
                return Err(DataLoadError::InvalidFieldValue {
                    row: rec.source_row,
                    field,
                    message: format!("price {val} must be > 0.0"),
                });
            }
        }

        if !rec.volume.is_finite() || rec.volume < 0.0 || rec.volume > MAX_CANDLE_VOLUME {
            return Err(DataLoadError::InvalidFieldValue {
                row: rec.source_row,
                field: "volume",
                message: format!("volume {} must be finite in [0.0, 1.0e18]", rec.volume),
            });
        }

        let body_max = rec.open.max(rec.close);
        let body_min = rec.open.min(rec.close);
        if rec.high < body_max || rec.low > body_min || rec.low > rec.high {
            match options.envelope_policy {
                OhlcEnvelopePolicy::RejectRow => {
                    return Err(DataLoadError::InvalidOhlcEnvelope {
                        row: rec.source_row,
                        open: rec.open,
                        high: rec.high,
                        low: rec.low,
                        close: rec.close,
                    });
                }
                OhlcEnvelopePolicy::ClampToBody => {
                    rec.high = rec.high.max(body_max);
                    rec.low = rec.low.min(body_min);
                    repaired_envelope_count += 1;
                }
            }
        }
    }

    // Stable chronological sort by `(timestamp, original_ordinal)`.
    raw_records.sort_by_key(|r| r.timestamp);

    // Deterministic duplicate-timestamp resolution using scale-relative tolerances (`MEDIUM-03`).
    let mut deduped: Vec<RawCandleRecord> = Vec::with_capacity(raw_records.len());
    let mut deduplicated_count = 0_usize;

    for rec in raw_records {
        if let Some(last) = deduped.last_mut()
            && last.timestamp == rec.timestamp
        {
            deduplicated_count += 1;
            match options.duplicate_policy {
                DuplicateTimestampPolicy::KeepLast => {
                    *last = rec;
                }
                DuplicateTimestampPolicy::KeepFirst => {}
                DuplicateTimestampPolicy::RejectConflict => {
                    let identical = rel_identical(last.open, rec.open, 1e-12)
                        && rel_identical(last.high, rec.high, 1e-12)
                        && rel_identical(last.low, rec.low, 1e-12)
                        && rel_identical(last.close, rec.close, 1e-12)
                        && rel_identical(last.volume, rec.volume, 1e-9);
                    if !identical {
                        return Err(DataLoadError::DuplicateTimestampConflict {
                            timestamp: rec.timestamp,
                            first_row: last.source_row,
                            second_row: rec.source_row,
                        });
                    }
                }
            }
        } else {
            deduped.push(rec);
        }
    }

    let mut candles: Vec<Candle> = deduped
        .into_iter()
        .map(|r| Candle {
            timestamp: r.timestamp,
            date_label: String::new(),
            open: r.open,
            high: r.high,
            low: r.low,
            close: r.close,
            volume: r.volume,
        })
        .collect();

    let inferred_tf = infer_base_timeframe(&candles, any_intraday_time);
    let base_timeframe = options
        .explicit_timeframe
        .or(envelope_timeframe)
        .unwrap_or(inferred_tf);

    for candle in &mut candles {
        candle.date_label = base_timeframe.format_candle_label(candle.timestamp);
        debug_assert!(candle.is_valid());
    }

    // Canonicalize into 1-bar-per-bucket OHLCV if an explicit/envelope `base_timeframe` is coarser
    // than the raw sub-bucket cadence (`MEDIUM-04`), keeping `base_tf -> W1 -> base_tf` idempotent.
    let has_shared_buckets = candles.windows(2).any(|w| {
        base_timeframe.bucket_start_timestamp(w[0].timestamp)
            == base_timeframe.bucket_start_timestamp(w[1].timestamp)
    });
    if has_shared_buckets {
        let rollup_src = if inferred_tf.can_aggregate_to(base_timeframe) {
            inferred_tf
        } else {
            base_timeframe
        };
        candles = aggregate_ohlcv(&candles, rollup_src, base_timeframe).map_err(|e| {
            DataLoadError::InvalidConfiguration(format!(
                "failed to canonicalize {base_timeframe} buckets: {e}"
            ))
        })?;
    }

    Ok(DataLoadReport {
        symbol,
        available_symbols,
        base_timeframe,
        candles,
        raw_row_count,
        deduplicated_count,
        repaired_envelope_count,
        filtered_other_symbol_rows,
    })
}
