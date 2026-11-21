//! Zero-dependency RFC-4180 CSV OHLCV parser supporting UTF-8 BOM, `CRLF`/`LF`,
//! auto-detected `,`/`;`/`\t` delimiters (quote-multiline aware), CafeF `<Ticker>,<DTYYYYMMDD>[,<Time>]`
//! headers, 5-column volume-optional feeds (`SPY.csv`), and strict multi-symbol isolation (`Milestone R06`).

use std::collections::HashMap;

use super::calendar::{
    MAX_SUPPORTED_EPOCH_SECS, MIN_SUPPORTED_EPOCH_SECS, ParsedTimestamp, RawTimestampToken,
    classify_string_timestamp_token, parse_time_of_day_hms, parse_ymd_8digit,
    resolve_series_timestamps,
};
use super::{
    DataLoadError, DataLoadOptions, DataLoadReport, InvalidRowPolicy, MAX_DISCOVERED_SYMBOLS,
    MAX_IMPORT_BYTES, MAX_IMPORT_RECORDS, MAX_SCANNED_RECORDS, RawCandleRecord, SymbolTracker,
    normalize_loaded_candles,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CsvColumnRole {
    Symbol,
    DateYmd,
    Timestamp,
    Open,
    High,
    Low,
    Close,
    Volume,
}

#[derive(Debug, Default)]
struct CsvColumnMap {
    symbol: Option<usize>,
    date_ymd: Option<usize>,
    timestamp: Option<usize>,
    open: Option<usize>,
    high: Option<usize>,
    low: Option<usize>,
    close: Option<usize>,
    volume: Option<usize>,
    min_required_len: usize,
}

struct PendingCsvRow {
    source_row: usize,
    ts_token: RawTimestampToken,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
}

/// Parses an in-memory UTF-8 CSV document into normalized, chronologically sorted OHLCV candles.
pub fn load_ohlcv_csv(
    csv_text: &str,
    options: &DataLoadOptions,
) -> Result<DataLoadReport, DataLoadError> {
    if csv_text.len() > MAX_IMPORT_BYTES {
        return Err(DataLoadError::InputTooLarge {
            bytes: csv_text.len(),
            max_bytes: MAX_IMPORT_BYTES,
        });
    }
    options.validate()?;

    let stripped = csv_text.strip_prefix('\u{FEFF}').unwrap_or(csv_text);
    if stripped.trim().is_empty() {
        return Err(DataLoadError::EmptyDataset);
    }

    let delimiter = detect_delimiter(stripped);
    let mut reader = Rfc4180RecordIter::new(stripped, delimiter);

    // Locate first non-empty record as the header row.
    let mut header_fields = Vec::new();
    let header_line = loop {
        match reader.next_record(&mut header_fields)? {
            Some(line_no) => {
                if header_fields.iter().any(|f| !f.trim().is_empty()) {
                    break line_no;
                }
            }
            None => return Err(DataLoadError::EmptyDataset),
        }
    };
    let col_map = map_header_columns(&header_fields, header_line)?;

    let quarantine = options.invalid_row_policy == InvalidRowPolicy::QuarantineRow;
    let mut pending_rows: Vec<PendingCsvRow> = Vec::new();
    let mut pre_quarantined = 0_usize;
    let mut symbols = SymbolTracker::default();
    let mut active_symbol: Option<String> = None;
    let mut filtered_other_symbol_rows = 0_usize;
    let mut scanned_records = 0_usize;
    let mut fields = Vec::with_capacity(header_fields.len().max(8));

    while let Some(line_no) = reader.next_record(&mut fields)? {
        if fields.iter().all(|f| f.trim().is_empty()) {
            continue;
        }
        scanned_records += 1;
        if scanned_records > MAX_SCANNED_RECORDS {
            return Err(DataLoadError::TooManyRecords {
                max_records: MAX_SCANNED_RECORDS,
            });
        }
        if fields.len() <= col_map.min_required_len {
            if quarantine {
                pre_quarantined += 1;
                continue;
            }
            return Err(DataLoadError::MalformedCsv {
                line: line_no,
                message: format!(
                    "expected at least {} columns, got {}",
                    col_map.min_required_len + 1,
                    fields.len()
                ),
            });
        }

        // Multi-symbol tracking & strict single-instrument isolation (`CafeF.INDEX.clean.csv`).
        if let Some(sym_idx) = col_map.symbol {
            let sym_raw = fields.get(sym_idx).map_or("", |s| s.trim());
            if sym_raw.is_empty() {
                if quarantine {
                    pre_quarantined += 1;
                    continue;
                }
                return Err(DataLoadError::InvalidFieldValue {
                    row: line_no,
                    field: "symbol",
                    message: "empty ticker/symbol cell".to_owned(),
                });
            }
            symbols.record(sym_raw);

            if let Some(ref filter) = options.symbol_filter {
                if !sym_raw.eq_ignore_ascii_case(filter.trim()) {
                    filtered_other_symbol_rows += 1;
                    continue;
                }
                if active_symbol.is_none() {
                    active_symbol = Some(sym_raw.to_owned());
                }
            } else {
                match &active_symbol {
                    None => active_symbol = Some(sym_raw.to_owned()),
                    Some(chosen) => {
                        if !sym_raw.eq_ignore_ascii_case(chosen) {
                            filtered_other_symbol_rows += 1;
                            continue;
                        }
                    }
                }
            }
        }

        if pending_rows.len() >= MAX_IMPORT_RECORDS {
            return Err(DataLoadError::TooManyRecords {
                max_records: MAX_IMPORT_RECORDS,
            });
        }

        match parse_pending_csv_row(&fields, &col_map, line_no, options) {
            Ok(row) => pending_rows.push(row),
            Err(err) => {
                if quarantine {
                    pre_quarantined += 1;
                } else {
                    return Err(err);
                }
            }
        }
    }

    if let Some(ref filter) = options.symbol_filter
        && col_map.symbol.is_some()
        && active_symbol.is_none()
    {
        return Err(DataLoadError::SymbolNotFound {
            requested: filter.trim().to_owned(),
            available: symbols.into_vec_with_fallback(""),
        });
    }

    let resolved_symbol = active_symbol
        .or_else(|| options.symbol_filter.as_ref().map(|s| s.trim().to_owned()))
        .or_else(|| options.default_symbol.clone())
        .unwrap_or_else(|| "CUSTOM".to_owned());

    let available_symbols = symbols.into_vec_with_fallback(&resolved_symbol);

    let (pending_rows, resolved_timestamps) = resolve_pending_csv_timestamps(
        pending_rows,
        options.timestamp_unit,
        quarantine,
        &mut pre_quarantined,
    )?;

    let raw_records: Vec<RawCandleRecord> = pending_rows
        .into_iter()
        .zip(resolved_timestamps)
        .map(|(row, pts)| RawCandleRecord {
            source_row: row.source_row,
            timestamp: pts.utc_seconds,
            has_time_component: pts.has_time_component,
            open: row.open,
            high: row.high,
            low: row.low,
            close: row.close,
            volume: row.volume,
        })
        .collect();

    let mut rep = normalize_loaded_candles(
        raw_records,
        resolved_symbol,
        available_symbols,
        filtered_other_symbol_rows,
        None,
        options,
    )?;
    rep.raw_row_count = rep.raw_row_count.saturating_add(pre_quarantined);
    rep.quarantined_row_count = rep.quarantined_row_count.saturating_add(pre_quarantined);
    Ok(rep)
}

fn resolve_pending_csv_timestamps(
    pending_rows: Vec<PendingCsvRow>,
    unit: super::TimestampUnit,
    quarantine: bool,
    pre_quarantined: &mut usize,
) -> Result<(Vec<PendingCsvRow>, Vec<ParsedTimestamp>), DataLoadError> {
    let tokens: Vec<RawTimestampToken> = pending_rows.iter().map(|r| r.ts_token.clone()).collect();
    match resolve_series_timestamps(&tokens, unit) {
        Ok(ts) => Ok((pending_rows, ts)),
        Err((row, message)) => {
            if !quarantine {
                return Err(DataLoadError::InvalidFieldValue {
                    row,
                    field: "timestamp",
                    message,
                });
            }
            // Quarantine only invalid individual timestamp tokens rather than dropping the entire symbol (`BUG-031`).
            let mut valid_rows = Vec::with_capacity(pending_rows.len());
            let mut valid_tokens = Vec::with_capacity(pending_rows.len());
            for item in pending_rows {
                if resolve_series_timestamps(std::slice::from_ref(&item.ts_token), unit).is_ok() {
                    valid_tokens.push(item.ts_token.clone());
                    valid_rows.push(item);
                } else {
                    *pre_quarantined = pre_quarantined.saturating_add(1);
                }
            }
            if valid_rows.is_empty() {
                return Err(DataLoadError::EmptyDataset);
            }
            let ts = resolve_series_timestamps(&valid_tokens, unit).map_err(|(r, m)| {
                DataLoadError::InvalidFieldValue {
                    row: r,
                    field: "timestamp",
                    message: m,
                }
            })?;
            Ok((valid_rows, ts))
        }
    }
}

fn parse_pending_csv_row(
    fields: &[String],
    col_map: &CsvColumnMap,
    line_no: usize,
    options: &DataLoadOptions,
) -> Result<PendingCsvRow, DataLoadError> {
    let ts_token = match (col_map.date_ymd, col_map.timestamp) {
        (Some(ymd_idx), Some(time_idx)) => {
            let base_day = parse_ymd_8digit(&fields[ymd_idx]).map_err(|msg| {
                DataLoadError::InvalidFieldValue {
                    row: line_no,
                    field: "date",
                    message: msg,
                }
            })?;
            let time_cell = fields[time_idx].trim();
            if time_cell.is_empty() {
                RawTimestampToken::AlreadyParsed(base_day)
            } else {
                let intraday_secs = parse_time_of_day_hms(time_cell).map_err(|msg| {
                    DataLoadError::InvalidFieldValue {
                        row: line_no,
                        field: "timestamp",
                        message: msg,
                    }
                })?;
                let combined = base_day.utc_seconds.saturating_add(intraday_secs);
                if !(MIN_SUPPORTED_EPOCH_SECS..=MAX_SUPPORTED_EPOCH_SECS).contains(&combined) {
                    return Err(DataLoadError::InvalidFieldValue {
                        row: line_no,
                        field: "timestamp",
                        message: format!(
                            "combined date+time '{}+{}' out of supported UTC range",
                            fields[ymd_idx].trim(),
                            time_cell
                        ),
                    });
                }
                RawTimestampToken::AlreadyParsed(ParsedTimestamp {
                    utc_seconds: combined,
                    has_time_component: true,
                })
            }
        }
        (Some(ymd_idx), None) => {
            let pts = parse_ymd_8digit(&fields[ymd_idx]).map_err(|msg| {
                DataLoadError::InvalidFieldValue {
                    row: line_no,
                    field: "date",
                    message: msg,
                }
            })?;
            RawTimestampToken::AlreadyParsed(pts)
        }
        (None, Some(ts_idx)) => {
            classify_string_timestamp_token(&fields[ts_idx], line_no, options.timestamp_unit)
                .map_err(|msg| DataLoadError::InvalidFieldValue {
                    row: line_no,
                    field: "timestamp",
                    message: msg,
                })?
        }
        (None, None) => unreachable!("validated time/date column"),
    };

    let open = parse_csv_f64(&fields[col_map.open.expect("open")], line_no, "open")?;
    let high = parse_csv_f64(&fields[col_map.high.expect("high")], line_no, "high")?;
    let low = parse_csv_f64(&fields[col_map.low.expect("low")], line_no, "low")?;
    let close = parse_csv_f64(&fields[col_map.close.expect("close")], line_no, "close")?;
    let volume = match col_map.volume.and_then(|vi| fields.get(vi)) {
        None => 0.0,
        Some(v_str) if v_str.trim().is_empty() => 0.0,
        Some(v_str) => parse_csv_f64(v_str, line_no, "volume")?,
    };

    Ok(PendingCsvRow {
        source_row: line_no,
        ts_token,
        open,
        high,
        low,
        close,
        volume,
    })
}

/// Parses all symbols in a UTF-8 CSV document in a **single streaming pass**, returning one
/// normalized [`DataLoadReport`] per valid symbol in discovery order (`471` CafeF stocks in `< 0.3s`).
pub fn load_ohlcv_csv_multi_symbol(
    csv_text: &str,
    options: &DataLoadOptions,
) -> Result<Vec<DataLoadReport>, DataLoadError> {
    if csv_text.len() > MAX_IMPORT_BYTES {
        return Err(DataLoadError::InputTooLarge {
            bytes: csv_text.len(),
            max_bytes: MAX_IMPORT_BYTES,
        });
    }
    options.validate()?;
    if options.symbol_filter.is_some() {
        return load_ohlcv_csv(csv_text, options).map(|r| vec![r]);
    }

    let stripped = csv_text.strip_prefix('\u{FEFF}').unwrap_or(csv_text);
    if stripped.trim().is_empty() {
        return Err(DataLoadError::EmptyDataset);
    }

    let delimiter = detect_delimiter(stripped);
    let mut reader = Rfc4180RecordIter::new(stripped, delimiter);

    let mut header_fields = Vec::new();
    let header_line = loop {
        match reader.next_record(&mut header_fields)? {
            Some(line_no) => {
                if header_fields.iter().any(|f| !f.trim().is_empty()) {
                    break line_no;
                }
            }
            None => return Err(DataLoadError::EmptyDataset),
        }
    };
    let col_map = map_header_columns(&header_fields, header_line)?;
    if col_map.symbol.is_none() {
        return load_ohlcv_csv(csv_text, options).map(|r| vec![r]);
    }

    let quarantine = options.invalid_row_policy == InvalidRowPolicy::QuarantineRow;
    let sym_idx = col_map.symbol.expect("symbol column");
    let mut buckets: Vec<(String, Vec<PendingCsvRow>, usize)> = Vec::new();
    let mut bucket_lookup: HashMap<String, usize> = HashMap::new();
    let mut scanned_records = 0_usize;
    let mut fields = Vec::with_capacity(header_fields.len().max(8));

    while let Some(line_no) = reader.next_record(&mut fields)? {
        if fields.iter().all(|f| f.trim().is_empty()) {
            continue;
        }
        scanned_records += 1;
        if scanned_records > MAX_SCANNED_RECORDS {
            return Err(DataLoadError::TooManyRecords {
                max_records: MAX_SCANNED_RECORDS,
            });
        }
        if fields.len() <= col_map.min_required_len {
            if quarantine {
                continue;
            }
            return Err(DataLoadError::MalformedCsv {
                line: line_no,
                message: format!(
                    "expected at least {} columns, got {}",
                    col_map.min_required_len + 1,
                    fields.len()
                ),
            });
        }

        let sym_raw = fields.get(sym_idx).map_or("", |s| s.trim());
        if sym_raw.is_empty() {
            if quarantine {
                continue;
            }
            return Err(DataLoadError::InvalidFieldValue {
                row: line_no,
                field: "symbol",
                message: "empty ticker/symbol cell".to_owned(),
            });
        }

        let lower = sym_raw.to_ascii_lowercase();
        let b_idx = if let Some(&idx) = bucket_lookup.get(&lower) {
            idx
        } else {
            if buckets.len() >= MAX_DISCOVERED_SYMBOLS {
                continue;
            }
            let idx = buckets.len();
            bucket_lookup.insert(lower, idx);
            buckets.push((sym_raw.to_owned(), Vec::new(), 0));
            idx
        };

        if buckets[b_idx].1.len() >= MAX_IMPORT_RECORDS {
            return Err(DataLoadError::TooManyRecords {
                max_records: MAX_IMPORT_RECORDS,
            });
        }

        match parse_pending_csv_row(&fields, &col_map, line_no, options) {
            Ok(row) => buckets[b_idx].1.push(row),
            Err(err) => {
                if quarantine {
                    buckets[b_idx].2 += 1;
                } else {
                    return Err(err);
                }
            }
        }
    }

    if buckets.is_empty() {
        return Err(DataLoadError::EmptyDataset);
    }

    let mut reports = Vec::with_capacity(buckets.len());
    for (sym, pending_rows, mut pre_quarantined) in buckets {
        if pending_rows.is_empty() {
            if quarantine {
                continue;
            }
            return Err(DataLoadError::EmptyDataset);
        }
        let (pending_rows, resolved_timestamps) = match resolve_pending_csv_timestamps(
            pending_rows,
            options.timestamp_unit,
            quarantine,
            &mut pre_quarantined,
        ) {
            Ok(pair) => pair,
            Err(err) => {
                if quarantine && matches!(err, DataLoadError::EmptyDataset) {
                    continue;
                }
                return Err(err);
            }
        };
        let other_rows = scanned_records.saturating_sub(pending_rows.len() + pre_quarantined);
        let raw_records: Vec<RawCandleRecord> = pending_rows
            .into_iter()
            .zip(resolved_timestamps)
            .map(|(row, pts)| RawCandleRecord {
                source_row: row.source_row,
                timestamp: pts.utc_seconds,
                has_time_component: pts.has_time_component,
                open: row.open,
                high: row.high,
                low: row.low,
                close: row.close,
                volume: row.volume,
            })
            .collect();

        match normalize_loaded_candles(raw_records, sym, Vec::new(), other_rows, None, options) {
            Ok(mut rep) => {
                rep.raw_row_count = rep.raw_row_count.saturating_add(pre_quarantined);
                rep.quarantined_row_count =
                    rep.quarantined_row_count.saturating_add(pre_quarantined);
                reports.push(rep);
            }
            Err(err) => {
                if !quarantine {
                    return Err(err);
                }
            }
        }
    }

    if reports.is_empty() {
        return Err(DataLoadError::EmptyDataset);
    }

    let available: Vec<String> = reports.iter().map(|r| r.symbol.clone()).collect();
    for rep in &mut reports {
        rep.available_symbols.clone_from(&available);
    }

    Ok(reports)
}

fn parse_csv_f64(raw: &str, line: usize, field: &'static str) -> Result<f64, DataLoadError> {
    let trimmed = raw.trim();
    let val: f64 = trimmed
        .parse()
        .map_err(|_| DataLoadError::InvalidFieldValue {
            row: line,
            field,
            message: format!("invalid number '{trimmed}'"),
        })?;
    if !val.is_finite() {
        return Err(DataLoadError::InvalidFieldValue {
            row: line,
            field,
            message: format!("non-finite number '{trimmed}'"),
        });
    }
    Ok(val)
}

/// Scans the first non-blank logical RFC-4180 record (skipping leading whitespace-only lines and
/// respecting multiline quoted header cells, `BUG-031`) to auto-detect `,`, `;`, or `\t`.
fn detect_delimiter(text: &str) -> u8 {
    // Advance past any leading blank / whitespace-only lines so delimiter detection always inspects the header!
    let mut start = 0_usize;
    let bytes = text.as_bytes();
    while start < bytes.len() {
        let mut line_end = start;
        while line_end < bytes.len() && bytes[line_end] != b'\n' && bytes[line_end] != b'\r' {
            line_end += 1;
        }
        if bytes[start..line_end]
            .iter()
            .any(|b| !matches!(*b, b' ' | b'\t'))
        {
            break;
        }
        start = line_end;
        if start < bytes.len() && bytes[start] == b'\r' {
            start += 1;
        }
        if start < bytes.len() && bytes[start] == b'\n' {
            start += 1;
        }
    }
    let trimmed = &text[start..];
    let mut commas = 0_usize;
    let mut semis = 0_usize;
    let mut tabs = 0_usize;
    let mut in_quotes = false;

    for b in trimmed.bytes() {
        match b {
            b'"' => in_quotes = !in_quotes,
            b'\r' | b'\n' if !in_quotes => break,
            b',' if !in_quotes => commas += 1,
            b';' if !in_quotes => semis += 1,
            b'\t' if !in_quotes => tabs += 1,
            _ => {}
        }
    }

    if tabs > commas && tabs >= semis {
        b'\t'
    } else if semis > commas {
        b';'
    } else {
        b','
    }
}

fn classify_header_name(raw: &str) -> Option<CsvColumnRole> {
    let trimmed = raw.trim();
    let unbracketed = trimmed
        .strip_prefix('<')
        .and_then(|s| s.strip_suffix('>'))
        .unwrap_or(trimmed)
        .trim();
    let mut collapsed = String::with_capacity(unbracketed.len());
    let mut prev_sep = false;
    for ch in unbracketed.chars() {
        if ch.is_ascii_whitespace() || ch == '_' || ch == '-' {
            if !prev_sep && !collapsed.is_empty() {
                collapsed.push('_');
                prev_sep = true;
            }
        } else {
            collapsed.push(ch.to_ascii_lowercase());
            prev_sep = false;
        }
    }
    let norm = collapsed.trim_matches('_');
    match norm {
        "ticker" | "symbol" | "instrument" | "code" | "ticker_symbol" => {
            Some(CsvColumnRole::Symbol)
        }
        "dtyyyymmdd" | "yyyymmdd" => Some(CsvColumnRole::DateYmd),
        "time" | "timestamp" | "date" | "datetime" | "open_time" | "unix" | "trade_date"
        | "tradedate" | "trading_date" | "trade_time" => Some(CsvColumnRole::Timestamp),
        "open" | "o" | "open_price" => Some(CsvColumnRole::Open),
        "high" | "h" | "high_price" => Some(CsvColumnRole::High),
        "low" | "l" | "low_price" => Some(CsvColumnRole::Low),
        "close" | "c" | "close_price" => Some(CsvColumnRole::Close),
        "volume" | "vol" | "v" => Some(CsvColumnRole::Volume),
        _ => None,
    }
}

fn map_header_columns(
    headers: &[String],
    header_line: usize,
) -> Result<CsvColumnMap, DataLoadError> {
    let mut map = CsvColumnMap::default();

    for (idx, h) in headers.iter().enumerate() {
        let Some(role) = classify_header_name(h) else {
            continue;
        };
        let slot = match role {
            CsvColumnRole::Symbol => &mut map.symbol,
            CsvColumnRole::DateYmd => &mut map.date_ymd,
            CsvColumnRole::Timestamp => &mut map.timestamp,
            CsvColumnRole::Open => &mut map.open,
            CsvColumnRole::High => &mut map.high,
            CsvColumnRole::Low => &mut map.low,
            CsvColumnRole::Close => &mut map.close,
            CsvColumnRole::Volume => &mut map.volume,
        };
        if slot.is_some() {
            return Err(DataLoadError::MalformedCsv {
                line: header_line,
                message: format!("duplicate column header '{}'", h.trim()),
            });
        }
        *slot = Some(idx);
    }

    let has_any_time_col = map.date_ymd.is_some() || map.timestamp.is_some();
    let mut missing = Vec::new();
    if !has_any_time_col {
        missing.push("time/date");
    }
    if map.open.is_none() {
        missing.push("open");
    }
    if map.high.is_none() {
        missing.push("high");
    }
    if map.low.is_none() {
        missing.push("low");
    }
    if map.close.is_none() {
        missing.push("close");
    }
    if !missing.is_empty() {
        return Err(DataLoadError::MissingRequiredColumns {
            missing: missing.join(", "),
        });
    }

    map.min_required_len = [
        map.date_ymd,
        map.timestamp,
        map.open,
        map.high,
        map.low,
        map.close,
        map.symbol,
    ]
    .into_iter()
    .flatten()
    .max()
    .unwrap_or(0);

    Ok(map)
}

/// Streaming RFC-4180 record scanner operating directly on a UTF-8 slice without regexes or external crates.
struct Rfc4180RecordIter<'a> {
    src: &'a str,
    bytes: &'a [u8],
    pos: usize,
    line: usize,
    delimiter: u8,
}

impl<'a> Rfc4180RecordIter<'a> {
    const fn new(src: &'a str, delimiter: u8) -> Self {
        Self {
            src,
            bytes: src.as_bytes(),
            pos: 0,
            line: 1,
            delimiter,
        }
    }

    fn next_record(
        &mut self,
        out_fields: &mut Vec<String>,
    ) -> Result<Option<usize>, DataLoadError> {
        out_fields.clear();
        if self.pos >= self.bytes.len() {
            return Ok(None);
        }

        let record_start_line = self.line;
        loop {
            let mut lookahead = self.pos;
            while lookahead < self.bytes.len() {
                let b = self.bytes[lookahead];
                if b == b' ' || (b == b'\t' && self.delimiter != b'\t') {
                    lookahead += 1;
                } else {
                    break;
                }
            }
            if lookahead < self.bytes.len() && self.bytes[lookahead] == b'"' {
                // Quoted RFC-4180 field (`""` escape + multiline safe)
                self.pos = lookahead + 1;
                let mut val = String::new();
                let mut closed = false;
                while self.pos < self.bytes.len() {
                    let b = self.bytes[self.pos];
                    if b == b'"' {
                        if self.pos + 1 < self.bytes.len() && self.bytes[self.pos + 1] == b'"' {
                            val.push('"');
                            self.pos += 2;
                        } else {
                            self.pos += 1;
                            closed = true;
                            break;
                        }
                    } else {
                        if b == b'\n' {
                            self.line += 1;
                        }
                        let ch = self.src[self.pos..]
                            .chars()
                            .next()
                            .expect("valid UTF-8 slice");
                        val.push(ch);
                        self.pos += ch.len_utf8();
                    }
                }
                if !closed {
                    return Err(DataLoadError::MalformedCsv {
                        line: record_start_line,
                        message: "unterminated double-quoted CSV field".to_owned(),
                    });
                }
                while self.pos < self.bytes.len() {
                    let b = self.bytes[self.pos];
                    if b == b' ' || (b == b'\t' && self.delimiter != b'\t') {
                        self.pos += 1;
                    } else {
                        break;
                    }
                }
                out_fields.push(val);
            } else {
                let start = self.pos;
                while self.pos < self.bytes.len() {
                    let b = self.bytes[self.pos];
                    if b == self.delimiter || b == b'\r' || b == b'\n' {
                        break;
                    }
                    if b == b'"' {
                        return Err(DataLoadError::MalformedCsv {
                            line: self.line,
                            message: "unexpected quote inside unquoted CSV field".to_owned(),
                        });
                    }
                    self.pos += 1;
                }
                out_fields.push(self.src[start..self.pos].to_owned());
            }

            if self.pos >= self.bytes.len() {
                break;
            }
            let sep = self.bytes[self.pos];
            if sep == self.delimiter {
                self.pos += 1;
                if self.pos >= self.bytes.len() {
                    out_fields.push(String::new());
                    break;
                }
                continue;
            }
            if sep == b'\r' {
                self.pos += 1;
                if self.pos < self.bytes.len() && self.bytes[self.pos] == b'\n' {
                    self.pos += 1;
                }
                self.line += 1;
                break;
            }
            if sep == b'\n' {
                self.pos += 1;
                self.line += 1;
                break;
            }
            return Err(DataLoadError::MalformedCsv {
                line: self.line,
                message: format!(
                    "unexpected character '{}' after CSV field",
                    self.src[self.pos..].chars().next().unwrap_or('?')
                ),
            });
        }

        Ok(Some(record_start_line))
    }
}
