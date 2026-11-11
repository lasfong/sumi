//! Zero-dependency RFC-8259 streaming JSON OHLCV parser (`Milestone R06`).
//!
//! Supports:
//! 1. Top-level array of candle objects: `[{"time": ..., "open": ..., "high": ..., "low": ..., "close": ..., "volume": ...}, ...]`
//! 2. Top-level array of exchange kline tuples: `[[timestamp, open, high, low, close, optional_volume], ...]`
//! 3. Envelope object: `{"symbol": "BTCUSDT", "timeframe": "15m", "candles" | "data" | "klines" | "bars" | "ohlcv" | "results": [...]}`
//!
//! Enforces strict RFC-8259 number/string/escape grammar, UTF-16 surrogate pair decoding,
//! `MAX_JSON_DEPTH = 16`, `MAX_IMPORT_BYTES`, `MAX_IMPORT_RECORDS`, `MAX_SCANNED_RECORDS`,
//! series-level `TimestampUnit::Auto` inference, and lossless `i128`/decimal timestamp parsing.

use super::calendar::{
    RawTimestampToken, TimestampUnit, classify_string_timestamp_token,
    parse_numeric_timestamp_token, parse_ymd_8digit, resolve_series_timestamps,
};
use super::timeframe::Timeframe;
use super::{
    DataLoadError, DataLoadOptions, DataLoadReport, MAX_IMPORT_BYTES, MAX_IMPORT_RECORDS,
    MAX_SCANNED_RECORDS, RawCandleRecord, SymbolTracker, normalize_loaded_candles,
};

/// Maximum nested JSON container depth accepted before rejecting input (`DoS` recursion guard).
pub const MAX_JSON_DEPTH: usize = 16;

#[derive(Debug, Clone)]
struct PendingJsonCandle {
    source_row: usize,
    ts_token: RawTimestampToken,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
}

/// Parses an in-memory UTF-8 JSON document into normalized, chronologically sorted OHLCV candles.
pub fn load_ohlcv_json(
    json_text: &str,
    options: &DataLoadOptions,
) -> Result<DataLoadReport, DataLoadError> {
    if json_text.len() > MAX_IMPORT_BYTES {
        return Err(DataLoadError::InputTooLarge {
            bytes: json_text.len(),
            max_bytes: MAX_IMPORT_BYTES,
        });
    }
    options.validate()?;

    let stripped = json_text.strip_prefix('\u{FEFF}').unwrap_or(json_text);
    let mut cursor = JsonCursor::new(stripped);
    cursor.skip_ws();
    if cursor.is_eof() {
        return Err(DataLoadError::EmptyDataset);
    }

    let mut pending_records: Vec<PendingJsonCandle> = Vec::new();
    let mut symbols = SymbolTracker::default();
    let mut active_symbol: Option<String> = None;
    let mut filtered_other_symbol_rows = 0_usize;
    let mut envelope_symbol: Option<String> = None;
    let mut envelope_timeframe: Option<Timeframe> = None;
    let mut found_candles_field = false;

    match cursor.peek_byte() {
        Some(b'[') => {
            cursor.parse_candles_array(
                1,
                options,
                None,
                &mut pending_records,
                &mut symbols,
                &mut active_symbol,
                &mut filtered_other_symbol_rows,
            )?;
            found_candles_field = true;
        }
        Some(b'{') => {
            cursor.advance(1);
            cursor.skip_ws();
            if cursor.consume_if(b'}') {
                return Err(DataLoadError::MissingRequiredColumns {
                    missing: "candles/data/klines/bars/ohlcv/results".to_owned(),
                });
            }
            let mut deferred_entries: Vec<(Option<String>, PendingJsonCandle)> = Vec::new();

            loop {
                cursor.skip_ws();
                let key = cursor.parse_string()?;
                cursor.skip_ws();
                cursor.expect_byte(b':')?;
                cursor.skip_ws();

                let key_norm = key.trim().to_ascii_lowercase();
                match key_norm.as_str() {
                    "symbol" | "ticker" | "instrument" => {
                        if cursor.peek_byte() == Some(b'n') {
                            cursor.parse_null()?;
                        } else {
                            let sym = cursor.parse_string()?;
                            let trimmed = sym.trim();
                            if !trimmed.is_empty() {
                                envelope_symbol = Some(trimmed.to_owned());
                            }
                        }
                    }
                    "timeframe" | "interval" | "resolution" => {
                        if cursor.peek_byte() == Some(b'n') {
                            cursor.parse_null()?;
                        } else {
                            let tf_str = cursor.parse_string()?;
                            let parsed = Timeframe::parse_label(&tf_str).ok_or_else(|| {
                                DataLoadError::InvalidFieldValue {
                                    row: 0,
                                    field: "timeframe",
                                    message: format!("unsupported timeframe '{tf_str}'"),
                                }
                            })?;
                            envelope_timeframe = Some(parsed);
                        }
                    }
                    "candles" | "data" | "klines" | "bars" | "ohlcv" | "results" => {
                        if found_candles_field {
                            return Err(
                                cursor.err("duplicate candles array field in JSON envelope")
                            );
                        }
                        found_candles_field = true;
                        cursor.parse_deferred_candles_array(2, options, &mut deferred_entries)?;
                    }
                    _ => {
                        cursor.skip_value(2)?;
                    }
                }

                cursor.skip_ws();
                if cursor.consume_if(b',') {
                    continue;
                }
                cursor.expect_byte(b'}')?;
                break;
            }

            if !found_candles_field {
                return Err(DataLoadError::MissingRequiredColumns {
                    missing: "candles/data/klines/bars/ohlcv/results".to_owned(),
                });
            }

            // Only prefer `envelope_symbol` as the default filter when at least one row omits `"symbol"`
            // or explicitly matches `envelope_symbol` (`HIGH-05`: handles feed-wrapper `"symbol": "HOSE_FEED"`
            // wrapping per-row `"symbol": "VNINDEX"` / `"HNX-INDEX"` without wiping all rows!).
            let env_sym_matches_any_row = envelope_symbol.as_deref().is_some_and(|env_s| {
                deferred_entries.iter().any(|(r_sym, _)| match r_sym {
                    None => true,
                    Some(s) => s.eq_ignore_ascii_case(env_s),
                })
            });

            pending_records.reserve(deferred_entries.len().min(MAX_IMPORT_RECORDS));
            for (row_sym_opt, rec) in deferred_entries {
                let effective_row_sym = row_sym_opt.as_deref().or(envelope_symbol.as_deref());
                if let Some(sym_raw) = effective_row_sym {
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
                            None => {
                                let preferred = if env_sym_matches_any_row {
                                    envelope_symbol.as_deref().unwrap_or(sym_raw)
                                } else {
                                    sym_raw
                                };
                                active_symbol = Some(preferred.to_owned());
                                if !sym_raw.eq_ignore_ascii_case(preferred) {
                                    filtered_other_symbol_rows += 1;
                                    continue;
                                }
                            }
                            Some(chosen) => {
                                if !sym_raw.eq_ignore_ascii_case(chosen) {
                                    filtered_other_symbol_rows += 1;
                                    continue;
                                }
                            }
                        }
                    }
                }
                if pending_records.len() >= MAX_IMPORT_RECORDS {
                    return Err(DataLoadError::TooManyRecords {
                        max_records: MAX_IMPORT_RECORDS,
                    });
                }
                pending_records.push(rec);
            }
        }
        _ => {
            return Err(
                cursor.err("expected top-level JSON array `[...]` or envelope object `{...}`")
            );
        }
    }

    cursor.skip_ws();
    if !cursor.is_eof() {
        return Err(cursor.err("unexpected trailing characters after root JSON value"));
    }
    let _ = found_candles_field;

    let has_any_symbol_metadata = !symbols.is_empty();
    if let Some(ref filter) = options.symbol_filter
        && has_any_symbol_metadata
        && active_symbol.is_none()
    {
        return Err(DataLoadError::SymbolNotFound {
            requested: filter.trim().to_owned(),
            available: symbols.into_vec_with_fallback(""),
        });
    }

    let resolved_symbol = active_symbol
        .or(envelope_symbol)
        .or_else(|| options.symbol_filter.as_ref().map(|s| s.trim().to_owned()))
        .or_else(|| options.default_symbol.clone())
        .unwrap_or_else(|| "CUSTOM".to_owned());

    let available_symbols = symbols.into_vec_with_fallback(&resolved_symbol);

    let tokens: Vec<RawTimestampToken> =
        pending_records.iter().map(|r| r.ts_token.clone()).collect();
    let resolved_timestamps =
        resolve_series_timestamps(&tokens, options.timestamp_unit).map_err(|(row, message)| {
            DataLoadError::InvalidFieldValue {
                row,
                field: "timestamp",
                message,
            }
        })?;

    let raw_records: Vec<RawCandleRecord> = pending_records
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

    normalize_loaded_candles(
        raw_records,
        resolved_symbol,
        available_symbols,
        filtered_other_symbol_rows,
        envelope_timeframe,
        options,
    )
}

#[derive(Debug, Clone)]
enum ScalarOrString<'a> {
    Number(&'a str),
    String(String),
}

struct JsonCursor<'a> {
    src: &'a str,
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> JsonCursor<'a> {
    const fn new(src: &'a str) -> Self {
        Self {
            src,
            bytes: src.as_bytes(),
            pos: 0,
        }
    }

    #[inline]
    const fn is_eof(&self) -> bool {
        self.pos >= self.bytes.len()
    }

    #[inline]
    fn peek_byte(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    #[inline]
    fn advance(&mut self, n: usize) {
        self.pos += n;
    }

    fn err(&self, msg: impl Into<String>) -> DataLoadError {
        DataLoadError::MalformedJson {
            offset: self.pos,
            message: msg.into(),
        }
    }

    fn skip_ws(&mut self) {
        while self.pos < self.bytes.len() {
            match self.bytes[self.pos] {
                b' ' | b'\t' | b'\r' | b'\n' => self.pos += 1,
                _ => break,
            }
        }
    }

    fn consume_if(&mut self, expected: u8) -> bool {
        if self.peek_byte() == Some(expected) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expect_byte(&mut self, expected: u8) -> Result<(), DataLoadError> {
        if self.consume_if(expected) {
            Ok(())
        } else {
            Err(self.err(format!("expected '{}'", expected as char)))
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn parse_candles_array(
        &mut self,
        depth: usize,
        options: &DataLoadOptions,
        envelope_symbol: Option<&str>,
        out_records: &mut Vec<PendingJsonCandle>,
        symbols: &mut SymbolTracker,
        active_symbol: &mut Option<String>,
        filtered_other_symbol_rows: &mut usize,
    ) -> Result<(), DataLoadError> {
        if depth > MAX_JSON_DEPTH {
            return Err(self.err("JSON nesting exceeds MAX_JSON_DEPTH (16)"));
        }
        self.expect_byte(b'[')?;
        self.skip_ws();
        if self.consume_if(b']') {
            return Ok(());
        }

        let mut ordinal = 1_usize;
        loop {
            if ordinal > MAX_SCANNED_RECORDS {
                return Err(DataLoadError::TooManyRecords {
                    max_records: MAX_SCANNED_RECORDS,
                });
            }
            self.skip_ws();
            let (row_sym, rec) = self.parse_single_candle_item(depth + 1, ordinal, options)?;
            let effective_sym = row_sym.as_deref().or(envelope_symbol);
            if let Some(sym_raw) = effective_sym {
                symbols.record(sym_raw);
                let keep = if let Some(ref filter) = options.symbol_filter {
                    if sym_raw.eq_ignore_ascii_case(filter.trim()) {
                        if active_symbol.is_none() {
                            *active_symbol = Some(sym_raw.to_owned());
                        }
                        true
                    } else {
                        false
                    }
                } else {
                    match active_symbol {
                        None => {
                            *active_symbol = Some(sym_raw.to_owned());
                            true
                        }
                        Some(chosen) => sym_raw.eq_ignore_ascii_case(chosen),
                    }
                };
                if !keep {
                    *filtered_other_symbol_rows += 1;
                } else {
                    if out_records.len() >= MAX_IMPORT_RECORDS {
                        return Err(DataLoadError::TooManyRecords {
                            max_records: MAX_IMPORT_RECORDS,
                        });
                    }
                    out_records.push(rec);
                }
            } else {
                if out_records.len() >= MAX_IMPORT_RECORDS {
                    return Err(DataLoadError::TooManyRecords {
                        max_records: MAX_IMPORT_RECORDS,
                    });
                }
                out_records.push(rec);
            }

            ordinal += 1;
            self.skip_ws();
            if self.consume_if(b',') {
                self.skip_ws();
                if self.peek_byte() == Some(b']') {
                    return Err(self.err("trailing comma in JSON array"));
                }
                continue;
            }
            self.expect_byte(b']')?;
            break;
        }
        Ok(())
    }

    fn parse_deferred_candles_array(
        &mut self,
        depth: usize,
        options: &DataLoadOptions,
        out_entries: &mut Vec<(Option<String>, PendingJsonCandle)>,
    ) -> Result<(), DataLoadError> {
        if depth > MAX_JSON_DEPTH {
            return Err(self.err("JSON nesting exceeds MAX_JSON_DEPTH (16)"));
        }
        self.expect_byte(b'[')?;
        self.skip_ws();
        if self.consume_if(b']') {
            return Ok(());
        }

        let mut ordinal = 1_usize;
        loop {
            if ordinal > MAX_SCANNED_RECORDS {
                return Err(DataLoadError::TooManyRecords {
                    max_records: MAX_SCANNED_RECORDS,
                });
            }
            self.skip_ws();
            let entry = self.parse_single_candle_item(depth + 1, ordinal, options)?;
            out_entries.push(entry);
            ordinal += 1;
            self.skip_ws();
            if self.consume_if(b',') {
                self.skip_ws();
                if self.peek_byte() == Some(b']') {
                    return Err(self.err("trailing comma in JSON candles array"));
                }
                continue;
            }
            self.expect_byte(b']')?;
            break;
        }
        Ok(())
    }

    fn parse_single_candle_item(
        &mut self,
        depth: usize,
        ordinal: usize,
        options: &DataLoadOptions,
    ) -> Result<(Option<String>, PendingJsonCandle), DataLoadError> {
        if depth > MAX_JSON_DEPTH {
            return Err(self.err("JSON nesting exceeds MAX_JSON_DEPTH (16)"));
        }
        match self.peek_byte() {
            Some(b'{') => self.parse_candle_object(depth, ordinal, options),
            Some(b'[') => {
                let rec = self.parse_candle_tuple(depth, ordinal, options)?;
                Ok((None, rec))
            }
            _ => Err(self.err(format!(
                "expected candle object `{{...}}` or tuple `[...]` at item {ordinal}"
            ))),
        }
    }

    fn parse_candle_object(
        &mut self,
        depth: usize,
        ordinal: usize,
        options: &DataLoadOptions,
    ) -> Result<(Option<String>, PendingJsonCandle), DataLoadError> {
        self.expect_byte(b'{')?;
        self.skip_ws();
        if self.consume_if(b'}') {
            return Err(DataLoadError::MissingRequiredColumns {
                missing: "time, open, high, low, close".to_owned(),
            });
        }

        let mut sym: Option<String> = None;
        let mut ts_token: Option<RawTimestampToken> = None;
        let mut open: Option<f64> = None;
        let mut high: Option<f64> = None;
        let mut low: Option<f64> = None;
        let mut close: Option<f64> = None;
        let mut volume: Option<f64> = None;

        loop {
            self.skip_ws();
            let key = self.parse_string()?;
            self.skip_ws();
            self.expect_byte(b':')?;
            self.skip_ws();

            let norm = key.trim().to_ascii_lowercase();
            match norm.as_str() {
                "symbol" | "ticker" | "s" => {
                    let s_val = self.parse_string()?;
                    let trimmed = s_val.trim();
                    if trimmed.is_empty() {
                        return Err(DataLoadError::InvalidFieldValue {
                            row: ordinal,
                            field: "symbol",
                            message: "empty symbol string in JSON candle".to_owned(),
                        });
                    }
                    sym = Some(trimmed.to_owned());
                }
                "dtyyyymmdd" | "yyyymmdd" => {
                    let val = self.parse_scalar_or_string()?;
                    let text = match val {
                        ScalarOrString::Number(n) => n.to_owned(),
                        ScalarOrString::String(s) => s,
                    };
                    let pts =
                        parse_ymd_8digit(&text).map_err(|m| DataLoadError::InvalidFieldValue {
                            row: ordinal,
                            field: "date",
                            message: m,
                        })?;
                    ts_token = Some(RawTimestampToken::AlreadyParsed(pts));
                }
                "time" | "timestamp" | "date" | "datetime" | "open_time" | "t" => {
                    let val = self.parse_scalar_or_string()?;
                    ts_token = Some(classify_scalar_timestamp(
                        val,
                        options.timestamp_unit,
                        ordinal,
                    )?);
                }
                "open" | "o" => {
                    open = Some(self.parse_f64_scalar(ordinal, "open")?);
                }
                "high" | "h" => {
                    high = Some(self.parse_f64_scalar(ordinal, "high")?);
                }
                "low" | "l" => {
                    low = Some(self.parse_f64_scalar(ordinal, "low")?);
                }
                "close" | "c" => {
                    close = Some(self.parse_f64_scalar(ordinal, "close")?);
                }
                "volume" | "vol" | "v" => {
                    if self.peek_byte() == Some(b'n') {
                        self.parse_null()?;
                        volume = Some(0.0);
                    } else {
                        volume = Some(self.parse_f64_scalar(ordinal, "volume")?);
                    }
                }
                _ => {
                    self.skip_value(depth + 1)?;
                }
            }

            self.skip_ws();
            if self.consume_if(b',') {
                self.skip_ws();
                if self.peek_byte() == Some(b'}') {
                    return Err(self.err("trailing comma in JSON candle object"));
                }
                continue;
            }
            self.expect_byte(b'}')?;
            break;
        }

        let mut missing = Vec::new();
        if ts_token.is_none() {
            missing.push("time");
        }
        if open.is_none() {
            missing.push("open");
        }
        if high.is_none() {
            missing.push("high");
        }
        if low.is_none() {
            missing.push("low");
        }
        if close.is_none() {
            missing.push("close");
        }
        if !missing.is_empty() {
            return Err(DataLoadError::MissingRequiredColumns {
                missing: missing.join(", "),
            });
        }

        Ok((
            sym,
            PendingJsonCandle {
                source_row: ordinal,
                ts_token: ts_token.expect("checked"),
                open: open.expect("checked"),
                high: high.expect("checked"),
                low: low.expect("checked"),
                close: close.expect("checked"),
                volume: volume.unwrap_or(0.0),
            },
        ))
    }

    fn parse_candle_tuple(
        &mut self,
        depth: usize,
        ordinal: usize,
        options: &DataLoadOptions,
    ) -> Result<PendingJsonCandle, DataLoadError> {
        self.expect_byte(b'[')?;
        self.skip_ws();
        if self.consume_if(b']') {
            return Err(DataLoadError::MissingRequiredColumns {
                missing: "tuple [timestamp, open, high, low, close]".to_owned(),
            });
        }

        let ts_val = self.parse_scalar_or_string()?;
        let ts_token = classify_scalar_timestamp(ts_val, options.timestamp_unit, ordinal)?;

        let mut ohlc = [0.0_f64; 4];
        let names = ["open", "high", "low", "close"];
        for (i, field_name) in names.iter().enumerate() {
            self.skip_ws();
            if !self.consume_if(b',') {
                return Err(DataLoadError::MissingRequiredColumns {
                    missing: (*field_name).to_owned(),
                });
            }
            self.skip_ws();
            ohlc[i] = self.parse_f64_scalar(ordinal, field_name)?;
        }

        self.skip_ws();
        let mut volume = 0.0_f64;
        if self.consume_if(b',') {
            self.skip_ws();
            if self.peek_byte() == Some(b']') {
                return Err(self.err("trailing comma in JSON candle tuple"));
            }
            if self.peek_byte() == Some(b'n') {
                self.parse_null()?;
            } else {
                volume = self.parse_f64_scalar(ordinal, "volume")?;
            }
            loop {
                self.skip_ws();
                if self.consume_if(b',') {
                    self.skip_ws();
                    if self.peek_byte() == Some(b']') {
                        return Err(self.err("trailing comma in JSON candle tuple"));
                    }
                    self.skip_value(depth + 1)?;
                } else {
                    break;
                }
            }
        }
        self.skip_ws();
        self.expect_byte(b']')?;

        Ok(PendingJsonCandle {
            source_row: ordinal,
            ts_token,
            open: ohlc[0],
            high: ohlc[1],
            low: ohlc[2],
            close: ohlc[3],
            volume,
        })
    }

    fn parse_f64_scalar(
        &mut self,
        ordinal: usize,
        field: &'static str,
    ) -> Result<f64, DataLoadError> {
        let val = self.parse_scalar_or_string()?;
        let s_ref = match &val {
            ScalarOrString::Number(n) => *n,
            ScalarOrString::String(s) => s.trim(),
        };
        let parsed: f64 = s_ref
            .parse()
            .map_err(|_| DataLoadError::InvalidFieldValue {
                row: ordinal,
                field,
                message: format!("invalid number '{s_ref}'"),
            })?;
        if !parsed.is_finite() {
            return Err(DataLoadError::InvalidFieldValue {
                row: ordinal,
                field,
                message: format!("non-finite number '{s_ref}'"),
            });
        }
        Ok(parsed)
    }

    fn parse_scalar_or_string(&mut self) -> Result<ScalarOrString<'a>, DataLoadError> {
        match self.peek_byte() {
            Some(b'"') => Ok(ScalarOrString::String(self.parse_string()?)),
            Some(b'-' | b'0'..=b'9') => Ok(ScalarOrString::Number(self.scan_number()?)),
            _ => Err(self.err("expected JSON number or string")),
        }
    }

    fn parse_string(&mut self) -> Result<String, DataLoadError> {
        self.expect_byte(b'"')?;
        let start = self.pos;
        while self.pos < self.bytes.len() {
            let b = self.bytes[self.pos];
            if b == b'"' {
                let s = self.src[start..self.pos].to_owned();
                self.pos += 1;
                return Ok(s);
            }
            if b == b'\\' {
                break;
            }
            if b < 0x20 {
                return Err(self.err("unescaped control character in JSON string"));
            }
            self.pos += 1;
        }
        if self.pos >= self.bytes.len() {
            return Err(self.err("unterminated JSON string"));
        }

        let mut out = String::from(&self.src[start..self.pos]);
        while self.pos < self.bytes.len() {
            let b = self.bytes[self.pos];
            if b == b'"' {
                self.pos += 1;
                return Ok(out);
            }
            if b < 0x20 {
                return Err(self.err("unescaped control character in JSON string"));
            }
            if b == b'\\' {
                self.pos += 1;
                let esc = self
                    .peek_byte()
                    .ok_or_else(|| self.err("unterminated escape in JSON string"))?;
                self.pos += 1;
                match esc {
                    b'"' => out.push('"'),
                    b'\\' => out.push('\\'),
                    b'/' => out.push('/'),
                    b'b' => out.push('\u{0008}'),
                    b'f' => out.push('\u{000C}'),
                    b'n' => out.push('\n'),
                    b'r' => out.push('\r'),
                    b't' => out.push('\t'),
                    b'u' => {
                        let u1 = self.read_hex4()?;
                        if (0xD800..=0xDBFF).contains(&u1) {
                            if self.pos + 2 > self.bytes.len()
                                || self.bytes[self.pos] != b'\\'
                                || self.bytes[self.pos + 1] != b'u'
                            {
                                return Err(
                                    self.err("unpaired UTF-16 high surrogate in JSON string")
                                );
                            }
                            self.pos += 2;
                            let u2 = self.read_hex4()?;
                            if !(0xDC00..=0xDFFF).contains(&u2) {
                                return Err(self.err("invalid UTF-16 low surrogate in JSON string"));
                            }
                            let codepoint = 0x10000
                                + (((u32::from(u1) - 0xD800) << 10) | (u32::from(u2) - 0xDC00));
                            let ch = char::from_u32(codepoint)
                                .ok_or_else(|| self.err("invalid surrogate scalar"))?;
                            out.push(ch);
                        } else if (0xDC00..=0xDFFF).contains(&u1) {
                            return Err(self.err("unpaired UTF-16 low surrogate in JSON string"));
                        } else {
                            let ch = char::from_u32(u32::from(u1))
                                .ok_or_else(|| self.err("invalid unicode escape"))?;
                            out.push(ch);
                        }
                    }
                    _ => return Err(self.err("invalid escape sequence in JSON string")),
                }
            } else {
                let ch = self.src[self.pos..]
                    .chars()
                    .next()
                    .expect("valid UTF-8 slice");
                out.push(ch);
                self.pos += ch.len_utf8();
            }
        }
        Err(self.err("unterminated JSON string"))
    }

    fn read_hex4(&mut self) -> Result<u16, DataLoadError> {
        if self.pos + 4 > self.bytes.len() {
            return Err(self.err("incomplete \\uXXXX escape in JSON string"));
        }
        let hex_str = &self.src[self.pos..self.pos + 4];
        if !hex_str.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(self.err("non-hex digit in \\uXXXX escape"));
        }
        let val =
            u16::from_str_radix(hex_str, 16).map_err(|_| self.err("invalid \\uXXXX escape"))?;
        self.pos += 4;
        Ok(val)
    }

    /// Scans and validates an RFC-8259 JSON number token, returning its exact slice in `self.src`.
    fn scan_number(&mut self) -> Result<&'a str, DataLoadError> {
        let start = self.pos;
        if self.peek_byte() == Some(b'-') {
            self.pos += 1;
        }
        match self.peek_byte() {
            Some(b'0') => {
                self.pos += 1;
                if matches!(self.peek_byte(), Some(b'0'..=b'9')) {
                    return Err(self.err("leading zeros are not allowed in RFC-8259 JSON numbers"));
                }
            }
            Some(b'1'..=b'9') => {
                self.pos += 1;
                while matches!(self.peek_byte(), Some(b'0'..=b'9')) {
                    self.pos += 1;
                }
            }
            _ => return Err(self.err("invalid JSON number")),
        }

        if self.peek_byte() == Some(b'.') {
            self.pos += 1;
            if !matches!(self.peek_byte(), Some(b'0'..=b'9')) {
                return Err(self.err("expected digit after decimal point in JSON number"));
            }
            while matches!(self.peek_byte(), Some(b'0'..=b'9')) {
                self.pos += 1;
            }
        }

        if matches!(self.peek_byte(), Some(b'e' | b'E')) {
            self.pos += 1;
            if matches!(self.peek_byte(), Some(b'+' | b'-')) {
                self.pos += 1;
            }
            if !matches!(self.peek_byte(), Some(b'0'..=b'9')) {
                return Err(self.err("expected digit in JSON number exponent"));
            }
            while matches!(self.peek_byte(), Some(b'0'..=b'9')) {
                self.pos += 1;
            }
        }

        Ok(&self.src[start..self.pos])
    }

    fn parse_null(&mut self) -> Result<(), DataLoadError> {
        self.expect_literal("null")
    }

    fn expect_literal(&mut self, lit: &str) -> Result<(), DataLoadError> {
        if self.src[self.pos..].starts_with(lit) {
            self.pos += lit.len();
            Ok(())
        } else {
            Err(self.err(format!("expected JSON literal '{lit}'")))
        }
    }

    fn skip_value(&mut self, depth: usize) -> Result<(), DataLoadError> {
        if depth > MAX_JSON_DEPTH {
            return Err(self.err("JSON nesting exceeds MAX_JSON_DEPTH (16)"));
        }
        self.skip_ws();
        match self.peek_byte() {
            Some(b'"') => {
                let _ = self.parse_string()?;
                Ok(())
            }
            Some(b'-' | b'0'..=b'9') => {
                let _ = self.scan_number()?;
                Ok(())
            }
            Some(b't') => self.expect_literal("true"),
            Some(b'f') => self.expect_literal("false"),
            Some(b'n') => self.expect_literal("null"),
            Some(b'[') => {
                self.pos += 1;
                self.skip_ws();
                if self.consume_if(b']') {
                    return Ok(());
                }
                loop {
                    self.skip_value(depth + 1)?;
                    self.skip_ws();
                    if self.consume_if(b',') {
                        self.skip_ws();
                        if self.peek_byte() == Some(b']') {
                            return Err(self.err("trailing comma in skipped JSON array"));
                        }
                        continue;
                    }
                    self.expect_byte(b']')?;
                    break;
                }
                Ok(())
            }
            Some(b'{') => {
                self.pos += 1;
                self.skip_ws();
                if self.consume_if(b'}') {
                    return Ok(());
                }
                loop {
                    self.skip_ws();
                    let _ = self.parse_string()?;
                    self.skip_ws();
                    self.expect_byte(b':')?;
                    self.skip_value(depth + 1)?;
                    self.skip_ws();
                    if self.consume_if(b',') {
                        self.skip_ws();
                        if self.peek_byte() == Some(b'}') {
                            return Err(self.err("trailing comma in skipped JSON object"));
                        }
                        continue;
                    }
                    self.expect_byte(b'}')?;
                    break;
                }
                Ok(())
            }
            _ => Err(self.err("unexpected token in JSON value")),
        }
    }
}

fn classify_scalar_timestamp(
    val: ScalarOrString<'_>,
    unit: TimestampUnit,
    ordinal: usize,
) -> Result<RawTimestampToken, DataLoadError> {
    match val {
        ScalarOrString::Number(num_token) => {
            if unit != TimestampUnit::Auto {
                let pts = parse_numeric_timestamp_token(num_token, unit).map_err(|msg| {
                    DataLoadError::InvalidFieldValue {
                        row: ordinal,
                        field: "timestamp",
                        message: msg,
                    }
                })?;
                return Ok(RawTimestampToken::AlreadyParsed(pts));
            }
            // Validate number magnitude/overflow immediately so skipped symbol rows still reject garbage.
            let _ = parse_numeric_timestamp_token(num_token, TimestampUnit::Nanoseconds).map_err(
                |msg| DataLoadError::InvalidFieldValue {
                    row: ordinal,
                    field: "timestamp",
                    message: msg,
                },
            )?;
            Ok(RawTimestampToken::JsonNumber {
                row: ordinal,
                raw: num_token.to_owned(),
            })
        }
        ScalarOrString::String(s) => {
            classify_string_timestamp_token(&s, ordinal, unit).map_err(|msg| {
                DataLoadError::InvalidFieldValue {
                    row: ordinal,
                    field: "timestamp",
                    message: msg,
                }
            })
        }
    }
}
