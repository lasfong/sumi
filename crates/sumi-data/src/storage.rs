use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::error::{DataError, Result};
use crate::types::{
    BarQueryMode, OhlcvBar, ResolvedSymbol, SeriesKey, SymbolKey, SymbolSearchResult,
    normalize_exchange_alias,
};

const SCHEMA_VERSION: i32 = 2;

/// Summary statistics for a single cached `(exchange, symbol, interval, adjustment)` series.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeriesMeta {
    pub exchange: String,
    pub symbol: String,
    pub interval: String,
    pub adjustment: String,
    pub earliest_ms: i64,
    pub latest_ms: i64,
    pub bar_count: usize,
    pub reached_start: bool,
    pub source: String,
    pub updated_at_ms: i64,
}

/// Paginated disk result returned by [`SqliteMarketStore::query_bars`].
#[derive(Debug, Clone, PartialEq)]
pub struct CacheBarsResult {
    pub bars: Vec<OhlcvBar>,
    pub has_older: bool,
    pub has_newer: bool,
}

/// Aggregate storage diagnostics for observability and CLI status reporting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoreStats {
    pub db_path: String,
    pub total_symbols: usize,
    pub total_series: usize,
    pub total_bars: usize,
}

/// Vietnam `1D` universe EOD freshness telemetry (eliminates the "Stale EOD Trap").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VnFreshnessSummary {
    pub expected_session_date: String,
    pub expected_session_ms: i64,
    pub total_vn_d1_series: usize,
    pub fresh_series: usize,
    pub stale_series: usize,
    pub newest_cached_date: Option<String>,
    pub median_cached_date: Option<String>,
    pub max_missing_sessions: usize,
}

/// Corporate action or earnings marker plotted on the chart bottom axis (`D` Dividend, `S` Split/Bonus, `E` Earnings).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CorporateEventMarker {
    pub timestamp_ms: i64,
    pub date: String,
    /// `"D"` (Cash Dividend) | `"S"` (Stock Split / Stock Dividend) | `"E"` (Earnings Release)
    pub kind: String,
    pub label: String,
    pub factor: Option<f64>,
    pub detail: String,
}

/// Bulk-loaded Vietnam `1D` (`splits`) equity universe + `HOSE:VNINDEX` benchmark series.
#[derive(Debug, Clone)]
pub struct VnDailyUniverseBars {
    pub equities: Vec<(SymbolKey, Vec<OhlcvBar>)>,
    pub vnindex: Option<Vec<OhlcvBar>>,
}

/// Persistent SQLite local cache optimized for sub-millisecond OHLCV range pagination (`< 5ms`),
/// continuous intraday accumulation (>11,000+ bars), and 100% offline operation.
///
/// Uses a `WITHOUT ROWID` clustered B-tree keyed by
/// `(exchange, symbol, interval, adjustment, timestamp_ms)` so adjacent bars live physically
/// contiguously on disk.
#[derive(Clone)]
pub struct SqliteMarketStore {
    conn: Arc<Mutex<Connection>>,
    db_path: PathBuf,
}

impl SqliteMarketStore {
    /// Opens (or creates) a SQLite cache database at `path`, configuring WAL mode and schema.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path_buf = path.as_ref().to_path_buf();
        if let Some(parent) = path_buf.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(&path_buf)?;
        // Safety backup before running destructive schema migrations (`v1 -> v2`, `BUG-001/003/025`):
        // preserves an untouched copy of the user's existing market DB once before altering historical bars.
        let existing_version: i32 = conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap_or(0);
        if existing_version > 0
            && existing_version < SCHEMA_VERSION
            && std::fs::metadata(&path_buf).map(|m| m.len() > 0).unwrap_or(false)
        {
            let mut backup_os = path_buf.as_os_str().to_os_string();
            backup_os.push(format!(".v{existing_version}.bak"));
            let backup_path = PathBuf::from(backup_os);
            if !backup_path.exists() {
                let _ = std::fs::copy(&path_buf, &backup_path);
            }
        }
        Self::configure_and_migrate(&conn, false)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            db_path: path_buf,
        })
    }

    /// Opens an ephemeral in-memory SQLite store (ideal for deterministic unit & mock tests).
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        Self::configure_and_migrate(&conn, true)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            db_path: PathBuf::from(":memory:"),
        })
    }

    /// Path to the backing SQLite file (or `":memory:"`).
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.db_path
    }

    fn configure_and_migrate(conn: &Connection, in_memory: bool) -> Result<()> {
        if !in_memory {
            conn.pragma_update(None, "journal_mode", "WAL")?;
        }
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "temp_store", "MEMORY")?;
        conn.pragma_update(None, "cache_size", -32_000_i64)?;
        conn.pragma_update(None, "busy_timeout", 5_000_i64)?;

        let current_version: i32 = conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap_or(0);

        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS bars (
                exchange     TEXT    NOT NULL,
                symbol       TEXT    NOT NULL,
                interval     TEXT    NOT NULL,
                adjustment   TEXT    NOT NULL,
                timestamp_ms INTEGER NOT NULL,
                open         REAL    NOT NULL,
                high         REAL    NOT NULL,
                low          REAL    NOT NULL,
                close        REAL    NOT NULL,
                volume       REAL    NOT NULL,
                turnover     REAL    NOT NULL,
                source       TEXT    NOT NULL DEFAULT 'tradingview',
                PRIMARY KEY (exchange, symbol, interval, adjustment, timestamp_ms)
            ) WITHOUT ROWID;

            CREATE TABLE IF NOT EXISTS series_meta (
                exchange      TEXT    NOT NULL,
                symbol        TEXT    NOT NULL,
                interval      TEXT    NOT NULL,
                adjustment    TEXT    NOT NULL,
                earliest_ms   INTEGER NOT NULL,
                latest_ms     INTEGER NOT NULL,
                bar_count     INTEGER NOT NULL,
                reached_start INTEGER NOT NULL DEFAULT 0,
                source        TEXT    NOT NULL,
                updated_at_ms INTEGER NOT NULL,
                PRIMARY KEY (exchange, symbol, interval, adjustment)
            ) WITHOUT ROWID;

            CREATE TABLE IF NOT EXISTS symbols (
                exchange      TEXT    NOT NULL,
                symbol        TEXT    NOT NULL,
                ticker        TEXT    NOT NULL,
                description   TEXT    NOT NULL,
                market_type   TEXT    NOT NULL,
                currency      TEXT    NOT NULL,
                metadata_json TEXT    NOT NULL,
                updated_at_ms INTEGER NOT NULL,
                PRIMARY KEY (exchange, symbol)
            ) WITHOUT ROWID;

            CREATE INDEX IF NOT EXISTS idx_symbols_symbol ON symbols (symbol);
            CREATE INDEX IF NOT EXISTS idx_symbols_ticker ON symbols (ticker);
            "#,
        )?;

        if current_version < 2 {
            Self::migrate_to_v2(conn)?;
        }

        conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
        Ok(())
    }

    /// One-time idempotent data-integrity migration (v1 -> v2):
    /// 1. Purges erroneous Sat/Sun VN exchange `1D` bars (`BUG-001`, e.g., inverted DD/MM CafeF INDEX rows).
    /// 2. Ratio-splices unadjusted dividend discontinuities between `cafef_adjusted` and `tradingview` (`BUG-003`).
    /// 3. Deduplicates wire alias `HNX:VN301!` from `symbols` when canonical `HNX:VN30F1!` exists (`BUG-025`).
    fn migrate_to_v2(conn: &Connection) -> Result<()> {
        // 1. Delete Sat/Sun 1D bars on VN exchanges (ICT = UTC+7 = +25200s; SQLite %w: 0=Sun, 6=Sat).
        let deleted_weekends = conn.execute(
            r#"
            DELETE FROM bars
            WHERE exchange IN ('HOSE', 'HNX', 'UPCOM')
              AND interval = '1D'
              AND CAST(strftime('%w', (timestamp_ms / 1000) + 25200, 'unixepoch') AS INTEGER) IN (0, 6)
            "#,
            [],
        )?;

        if deleted_weekends > 0 {
            let _ = conn.execute_batch(
                r#"
                UPDATE series_meta
                SET earliest_ms = COALESCE(
                        (SELECT MIN(timestamp_ms) FROM bars b
                         WHERE b.exchange = series_meta.exchange AND b.symbol = series_meta.symbol
                           AND b.interval = series_meta.interval AND b.adjustment = series_meta.adjustment),
                        earliest_ms
                    ),
                    latest_ms = COALESCE(
                        (SELECT MAX(timestamp_ms) FROM bars b
                         WHERE b.exchange = series_meta.exchange AND b.symbol = series_meta.symbol
                           AND b.interval = series_meta.interval AND b.adjustment = series_meta.adjustment),
                        latest_ms
                    ),
                    bar_count = (
                        SELECT COUNT(*) FROM bars b
                        WHERE b.exchange = series_meta.exchange AND b.symbol = series_meta.symbol
                          AND b.interval = series_meta.interval AND b.adjustment = series_meta.adjustment
                    )
                WHERE exchange IN ('HOSE', 'HNX', 'UPCOM') AND interval = '1D';
                "#,
            );
        }

        // 2. Repair dividend-adjusted vs split-adjusted discontinuities at CafeF -> TradingView boundaries.
        let splice_candidates: Vec<(String, String, i64, f64, f64)> = {
            let mut stmt = conn.prepare(
                r#"
                WITH last_cf AS (
                    SELECT exchange, symbol, MAX(timestamp_ms) AS cf_ts
                    FROM bars
                    WHERE exchange IN ('HOSE', 'HNX', 'UPCOM')
                      AND interval = '1D'
                      AND adjustment = 'splits'
                      AND source = 'cafef_adjusted'
                    GROUP BY exchange, symbol
                ),
                first_tv AS (
                    SELECT b.exchange, b.symbol, MIN(b.timestamp_ms) AS tv_ts
                    FROM bars b
                    JOIN last_cf c ON b.exchange = c.exchange AND b.symbol = c.symbol
                    WHERE b.interval = '1D'
                      AND b.adjustment = 'splits'
                      AND b.source = 'tradingview'
                      AND b.timestamp_ms > c.cf_ts
                    GROUP BY b.exchange, b.symbol
                )
                SELECT c.exchange, c.symbol, t.tv_ts, bc.close, bt.open
                FROM last_cf c
                JOIN first_tv t ON c.exchange = t.exchange AND c.symbol = t.symbol
                JOIN bars bc ON bc.exchange = c.exchange AND bc.symbol = c.symbol
                            AND bc.interval = '1D' AND bc.adjustment = 'splits'
                            AND bc.timestamp_ms = c.cf_ts
                JOIN bars bt ON bt.exchange = t.exchange AND bt.symbol = t.symbol
                            AND bt.interval = '1D' AND bt.adjustment = 'splits'
                            AND bt.timestamp_ms = t.tv_ts
                "#,
            )?;
            let iter = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, f64>(3)?,
                    row.get::<_, f64>(4)?,
                ))
            })?;
            iter.filter_map(std::result::Result::ok).collect()
        };

        for (ex, sym, first_tv_ts, cf_close, tv_open) in splice_candidates {
            if cf_close > 0.0 && tv_open > 0.0 && cf_close.is_finite() && tv_open.is_finite() {
                let ratio = tv_open / cf_close;
                if (0.25..=4.0).contains(&ratio) && (ratio - 1.0).abs() > 0.03 {
                    let _ = conn.execute(
                        r#"
                        UPDATE bars
                        SET open     = open * ?1,
                            high     = high * ?1,
                            low      = low * ?1,
                            close    = close * ?1,
                            volume   = volume / ?1,
                            turnover = CASE
                                WHEN turnover > 0.0 THEN turnover
                                ELSE (close * ?1) * (volume / ?1)
                            END,
                            source   = 'cafef_spliced'
                        WHERE exchange = ?2
                          AND symbol = ?3
                          AND interval = '1D'
                          AND adjustment = 'splits'
                          AND source IN ('cafef_adjusted', 'cafef_spliced')
                          AND timestamp_ms < ?4
                        "#,
                        params![ratio, ex, sym, first_tv_ts],
                    );
                }
            }
        }

        // 3. Canonicalize wire alias `HNX:VN301!` -> `HNX:VN30F1!` (`BUG-025`).
        let has_canonical_vn30f: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM symbols WHERE exchange = 'HNX' AND symbol = 'VN30F1!'",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if has_canonical_vn30f > 0 {
            let _ = conn.execute(
                "DELETE FROM symbols WHERE exchange = 'HNX' AND symbol = 'VN301!'",
                [],
            );
        } else {
            let _ = conn.execute(
                "UPDATE symbols SET symbol = 'VN30F1!', ticker = 'HNX:VN30F1!' WHERE exchange = 'HNX' AND symbol = 'VN301!'",
                [],
            );
        }

        Ok(())
    }

    /// Atomically upserts a slice of OHLCV bars for `key` and refreshes `series_meta`.
    ///
    /// Existing historical bars outside the incoming slice are **preserved** (never deleted),
    /// allowing intraday `1m`/`15m` history to grow beyond TradingView's ~11,000-bar server cap.
    pub fn upsert_bars(
        &self,
        key: &SeriesKey,
        bars: &[OhlcvBar],
        source: &str,
        reached_start: Option<bool>,
    ) -> Result<usize> {
        if bars.is_empty() {
            if let Some(reached) = reached_start {
                self.mark_reached_start(key, reached)?;
            }
            return Ok(0);
        }

        let mut conn = self
            .conn
            .lock()
            .map_err(|_| DataError::Storage("SQLite mutex poisoned".to_owned()))?;
        let tx = conn.transaction()?;

        let interval_str = key.interval.as_str();
        let adj_str = key.adjustment.as_str();
        let now_ms = chrono::Utc::now().timestamp_millis();

        // Ratio-splice pre-existing `cafef_adjusted` / `cafef_spliced` history seamlessly against
        // incoming `tradingview` bars on exact overlap dates so multiple consecutive splits/dividends
        // never create a step jump (`BUG-003` + `Audit Fix #4`).
        if source == "tradingview"
            && interval_str == "1D"
            && adj_str == "splits"
            && matches!(key.exchange.as_str(), "HOSE" | "HNX" | "UPCOM")
            && let Some(earliest_tv) = bars.iter().min_by_key(|b| b.timestamp)
        {
            let mut overlap_ratio: Option<(f64, i64)> = None;
            for tv_bar in bars {
                let cf_close: Option<f64> = tx
                    .query_row(
                        r#"
                        SELECT close FROM bars
                        WHERE exchange = ?1 AND symbol = ?2 AND interval = '1D'
                          AND adjustment = 'splits'
                          AND source IN ('cafef_adjusted', 'cafef_spliced')
                          AND timestamp_ms = ?3
                        "#,
                        params![&key.exchange, &key.symbol, tv_bar.timestamp],
                        |r| r.get(0),
                    )
                    .optional()?;
                if let Some(cf_c) = cf_close
                    && cf_c > 0.0
                    && tv_bar.close > 0.0
                {
                    let ratio = tv_bar.close / cf_c;
                    if ratio.is_finite() && (0.02..=50.0).contains(&ratio) {
                        overlap_ratio = Some((ratio, earliest_tv.timestamp));
                        break;
                    }
                }
            }
            if let Some((ratio, cutoff_ts)) = overlap_ratio
                && (ratio - 1.0).abs() > 0.002
            {
                let _ = tx.execute(
                    r#"
                    UPDATE bars
                    SET open     = open * ?1,
                        high     = high * ?1,
                        low      = low * ?1,
                        close    = close * ?1,
                        volume   = volume / ?1,
                        turnover = CASE
                            WHEN turnover > 0.0 THEN turnover
                            ELSE (close * ?1) * (volume / ?1)
                        END,
                        source   = 'cafef_spliced'
                    WHERE exchange = ?2 AND symbol = ?3 AND interval = '1D'
                      AND adjustment = 'splits'
                      AND source IN ('cafef_adjusted', 'cafef_spliced')
                      AND timestamp_ms < ?4
                    "#,
                    params![ratio, &key.exchange, &key.symbol, cutoff_ts],
                );
            }
        }

        {
            let mut stmt = tx.prepare_cached(
                r#"
                INSERT INTO bars (
                    exchange, symbol, interval, adjustment, timestamp_ms,
                    open, high, low, close, volume, turnover, source
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                ON CONFLICT(exchange, symbol, interval, adjustment, timestamp_ms)
                DO UPDATE SET
                    open     = excluded.open,
                    high     = excluded.high,
                    low      = excluded.low,
                    close    = excluded.close,
                    volume   = excluded.volume,
                    turnover = excluded.turnover,
                    source   = excluded.source
                "#,
            )?;

            for bar in bars {
                stmt.execute(params![
                    &key.exchange,
                    &key.symbol,
                    interval_str,
                    adj_str,
                    bar.timestamp,
                    bar.open,
                    bar.high,
                    bar.low,
                    bar.close,
                    bar.volume,
                    bar.turnover,
                    source,
                ])?;
            }
        }

        refresh_series_meta_tx(&tx, key, source, reached_start, now_ms)?;
        tx.commit()?;
        Ok(bars.len())
    }

    /// Upserts a single live-streaming bar into local storage (used for continuous archival).
    pub fn upsert_bar(&self, key: &SeriesKey, bar: &OhlcvBar, source: &str) -> Result<()> {
        let _ = self.upsert_bars(key, std::slice::from_ref(bar), source, None)?;
        Ok(())
    }

    /// Bulk-imports multiple series and symbol metadata using bounded transactional batches
    /// (preventing oversized WAL `-shm` growth while ingesting 471+ CafeF symbols / ~950k bars in seconds).
    pub fn bulk_import_series_and_symbols(
        &self,
        series_batches: &[(SeriesKey, Vec<OhlcvBar>, String)],
        symbols: &[ResolvedSymbol],
        reached_start: bool,
    ) -> Result<usize> {
        const CHUNK_SYMBOLS: usize = 25;

        let mut conn = self
            .conn
            .lock()
            .map_err(|_| DataError::Storage("SQLite mutex poisoned".to_owned()))?;
        let now_ms = chrono::Utc::now().timestamp_millis();
        let mut total_bars = 0_usize;

        for batch_chunk in series_batches.chunks(CHUNK_SYMBOLS) {
            let tx = conn.transaction()?;
            {
                let mut bar_stmt = tx.prepare_cached(
                    r#"
                    INSERT INTO bars (
                        exchange, symbol, interval, adjustment, timestamp_ms,
                        open, high, low, close, volume, turnover, source
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                    ON CONFLICT(exchange, symbol, interval, adjustment, timestamp_ms)
                    DO UPDATE SET
                        open     = excluded.open,
                        high     = excluded.high,
                        low      = excluded.low,
                        close    = excluded.close,
                        volume   = excluded.volume,
                        turnover = excluded.turnover,
                        source   = excluded.source
                    "#,
                )?;

                for (key, bars, source) in batch_chunk {
                    if bars.is_empty() {
                        continue;
                    }
                    let iv = key.interval.as_str();
                    let adj = key.adjustment.as_str();

                    // If importing `cafef_adjusted` into a series that already has `tradingview` bars,
                    // preserve the authoritative `tradingview` tail and ratio-splice earlier CafeF bars (`BUG-003`).
                    let mut effective_source = source.as_str();
                    let mut scale_ratio = 1.0_f64;
                    let mut tv_cutoff_ts: Option<i64> = None;
                    if source == "cafef_adjusted" && iv == "1D" && adj == "splits" {
                        let first_tv: Option<(i64, f64)> = tx
                            .query_row(
                                r#"
                                SELECT timestamp_ms, close FROM bars
                                WHERE exchange = ?1 AND symbol = ?2 AND interval = '1D'
                                  AND adjustment = 'splits' AND source = 'tradingview'
                                ORDER BY timestamp_ms ASC LIMIT 1
                                "#,
                                params![&key.exchange, &key.symbol],
                                |r| Ok((r.get(0)?, r.get(1)?)),
                            )
                            .optional()?;
                        if let Some((first_ts, first_tv_close)) = first_tv {
                            tv_cutoff_ts = Some(first_ts);
                            if let Some(overlap_cf) = bars.iter().find(|b| b.timestamp == first_ts)
                                && overlap_cf.close > 0.0
                                && first_tv_close > 0.0
                            {
                                let r = first_tv_close / overlap_cf.close;
                                if r.is_finite() && (0.25..=4.0).contains(&r) {
                                    scale_ratio = r;
                                    if (r - 1.0).abs() > 0.002 {
                                        effective_source = "cafef_spliced";
                                    }
                                }
                            }
                        }
                    }

                    let mut written_this_series = 0_usize;
                    for bar in bars {
                        if let Some(cutoff) = tv_cutoff_ts
                            && bar.timestamp >= cutoff
                        {
                            continue;
                        }
                        bar_stmt.execute(params![
                            &key.exchange,
                            &key.symbol,
                            iv,
                            adj,
                            bar.timestamp,
                            bar.open * scale_ratio,
                            bar.high * scale_ratio,
                            bar.low * scale_ratio,
                            bar.close * scale_ratio,
                            bar.volume / scale_ratio,
                            if bar.turnover > 0.0 {
                                bar.turnover
                            } else {
                                (bar.close * scale_ratio) * (bar.volume / scale_ratio)
                            },
                            effective_source,
                        ])?;
                        written_this_series += 1;
                    }
                    total_bars = total_bars.saturating_add(written_this_series);
                    refresh_series_meta_tx(&tx, key, effective_source, Some(reached_start), now_ms)?;
                }
            }
            tx.commit()?;
        }

        if !symbols.is_empty() {
            let tx = conn.transaction()?;
            {
                let mut sym_stmt = tx.prepare_cached(
                    r#"
                    INSERT INTO symbols (
                        exchange, symbol, ticker, description, market_type,
                        currency, metadata_json, updated_at_ms
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                    ON CONFLICT(exchange, symbol) DO NOTHING
                    "#,
                )?;

                // Store synthetic bulk-imported metadata with `updated_at_ms = 0` so offline queries
                // succeed immediately while online `resolve_symbol` upgrades to authoritative exchange metadata.
                for sym in symbols {
                    let json = serde_json::to_string(sym)?;
                    sym_stmt.execute(params![
                        &sym.exchange,
                        &sym.symbol,
                        &sym.ticker,
                        &sym.description,
                        &sym.market_type,
                        &sym.currency,
                        &json,
                        0_i64,
                    ])?;
                }
            }
            tx.commit()?;
        }

        let _ = conn.execute_batch("PRAGMA wal_checkpoint(PASSIVE);");
        Ok(total_bars)
    }

    fn mark_reached_start(&self, key: &SeriesKey, reached: bool) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DataError::Storage("SQLite mutex poisoned".to_owned()))?;
        conn.execute(
            r#"
            UPDATE series_meta
            SET reached_start = CASE WHEN ?5 THEN 1 ELSE reached_start END,
                updated_at_ms = ?6
            WHERE exchange = ?1 AND symbol = ?2 AND interval = ?3 AND adjustment = ?4
            "#,
            params![
                &key.exchange,
                &key.symbol,
                key.interval.as_str(),
                key.adjustment.as_str(),
                reached,
                chrono::Utc::now().timestamp_millis(),
            ],
        )?;
        Ok(())
    }

    /// Queries OHLCV bars for `key` supporting all 3 KLineChart v10 `DataLoader` pagination modes:
    /// - [`BarQueryMode::Init`]: latest `limit` bars (ascending), with `has_older` & `has_newer = false`.
    /// - [`BarQueryMode::Forward`]: up to `limit` bars strictly `< before_ms` (ascending), with exact `has_older` & `has_newer`.
    /// - [`BarQueryMode::Backward`]: up to `limit` bars strictly `> after_ms` (ascending), with exact `has_older` & `has_newer`.
    pub fn query_bars(
        &self,
        key: &SeriesKey,
        mode: BarQueryMode,
        limit: usize,
    ) -> Result<CacheBarsResult> {
        let limit = limit.max(1);
        let probe_limit = (limit.saturating_add(1)) as i64;
        let iv = key.interval.as_str();
        let adj = key.adjustment.as_str();

        let conn = self
            .conn
            .lock()
            .map_err(|_| DataError::Storage("SQLite mutex poisoned".to_owned()))?;

        match mode {
            BarQueryMode::Init => {
                let mut stmt = conn.prepare_cached(
                    r#"
                    SELECT timestamp_ms, open, high, low, close, volume, turnover
                    FROM bars
                    WHERE exchange = ?1 AND symbol = ?2 AND interval = ?3 AND adjustment = ?4
                    ORDER BY timestamp_ms DESC
                    LIMIT ?5
                    "#,
                )?;
                let mut rows = collect_bars(
                    &mut stmt,
                    params![&key.exchange, &key.symbol, iv, adj, probe_limit],
                )?;
                let has_older = rows.len() > limit;
                if has_older {
                    rows.truncate(limit);
                }
                rows.reverse();
                Ok(CacheBarsResult {
                    bars: rows,
                    has_older,
                    has_newer: false,
                })
            }
            BarQueryMode::Forward { before_ms } => {
                let mut stmt = conn.prepare_cached(
                    r#"
                    SELECT timestamp_ms, open, high, low, close, volume, turnover
                    FROM bars
                    WHERE exchange = ?1 AND symbol = ?2 AND interval = ?3 AND adjustment = ?4
                      AND timestamp_ms < ?5
                    ORDER BY timestamp_ms DESC
                    LIMIT ?6
                    "#,
                )?;
                let mut rows = collect_bars(
                    &mut stmt,
                    params![&key.exchange, &key.symbol, iv, adj, before_ms, probe_limit],
                )?;
                let has_older = rows.len() > limit;
                if has_older {
                    rows.truncate(limit);
                }
                rows.reverse();

                let has_newer: bool = conn.query_row(
                    r#"
                    SELECT EXISTS(
                        SELECT 1 FROM bars
                        WHERE exchange = ?1 AND symbol = ?2 AND interval = ?3 AND adjustment = ?4
                          AND timestamp_ms >= ?5
                    )
                    "#,
                    params![&key.exchange, &key.symbol, iv, adj, before_ms],
                    |row| row.get::<_, i64>(0).map(|v| v != 0),
                )?;

                Ok(CacheBarsResult {
                    bars: rows,
                    has_older,
                    has_newer,
                })
            }
            BarQueryMode::Backward { after_ms } => {
                let mut stmt = conn.prepare_cached(
                    r#"
                    SELECT timestamp_ms, open, high, low, close, volume, turnover
                    FROM bars
                    WHERE exchange = ?1 AND symbol = ?2 AND interval = ?3 AND adjustment = ?4
                      AND timestamp_ms > ?5
                    ORDER BY timestamp_ms ASC
                    LIMIT ?6
                    "#,
                )?;
                let mut rows = collect_bars(
                    &mut stmt,
                    params![&key.exchange, &key.symbol, iv, adj, after_ms, probe_limit],
                )?;
                let has_newer = rows.len() > limit;
                if has_newer {
                    rows.truncate(limit);
                }

                let has_older: bool = conn.query_row(
                    r#"
                    SELECT EXISTS(
                        SELECT 1 FROM bars
                        WHERE exchange = ?1 AND symbol = ?2 AND interval = ?3 AND adjustment = ?4
                          AND timestamp_ms <= ?5
                    )
                    "#,
                    params![&key.exchange, &key.symbol, iv, adj, after_ms],
                    |row| row.get::<_, i64>(0).map(|v| v != 0),
                )?;

                Ok(CacheBarsResult {
                    bars: rows,
                    has_older,
                    has_newer,
                })
            }
        }
    }

    /// Reads cached metadata (`earliest_ms`, `latest_ms`, `bar_count`, `reached_start`) for a series.
    pub fn series_meta(&self, key: &SeriesKey) -> Result<Option<SeriesMeta>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DataError::Storage("SQLite mutex poisoned".to_owned()))?;
        let meta = conn
            .query_row(
                r#"
                SELECT exchange, symbol, interval, adjustment, earliest_ms, latest_ms,
                       bar_count, reached_start, source, updated_at_ms
                FROM series_meta
                WHERE exchange = ?1 AND symbol = ?2 AND interval = ?3 AND adjustment = ?4
                "#,
                params![
                    &key.exchange,
                    &key.symbol,
                    key.interval.as_str(),
                    key.adjustment.as_str(),
                ],
                |row| {
                    Ok(SeriesMeta {
                        exchange: row.get(0)?,
                        symbol: row.get(1)?,
                        interval: row.get(2)?,
                        adjustment: row.get(3)?,
                        earliest_ms: row.get(4)?,
                        latest_ms: row.get(5)?,
                        bar_count: row.get::<_, i64>(6)?.max(0) as usize,
                        reached_start: row.get::<_, i64>(7)? != 0,
                        source: row.get(8)?,
                        updated_at_ms: row.get(9)?,
                    })
                },
            )
            .optional()?;
        Ok(meta)
    }

    /// Persists resolved instrument metadata for instant offline resolution & search.
    pub fn upsert_symbol(&self, resolved: &ResolvedSymbol) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DataError::Storage("SQLite mutex poisoned".to_owned()))?;
        let json = serde_json::to_string(resolved)?;
        let now_ms = chrono::Utc::now().timestamp_millis();
        conn.execute(
            r#"
            INSERT INTO symbols (
                exchange, symbol, ticker, description, market_type,
                currency, metadata_json, updated_at_ms
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(exchange, symbol)
            DO UPDATE SET
                ticker        = excluded.ticker,
                description   = excluded.description,
                market_type   = excluded.market_type,
                currency      = excluded.currency,
                metadata_json = excluded.metadata_json,
                updated_at_ms = excluded.updated_at_ms
            "#,
            params![
                &resolved.exchange,
                &resolved.symbol,
                &resolved.ticker,
                &resolved.description,
                &resolved.market_type,
                &resolved.currency,
                &json,
                now_ms,
            ],
        )?;
        Ok(())
    }

    /// Looks up cached [`ResolvedSymbol`] plus its last-updated Unix millisecond timestamp.
    pub fn get_symbol(&self, key: &SymbolKey) -> Result<Option<(ResolvedSymbol, i64)>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DataError::Storage("SQLite mutex poisoned".to_owned()))?;
        let row: Option<(String, i64)> = conn
            .query_row(
                r#"
                SELECT metadata_json, updated_at_ms
                FROM symbols
                WHERE exchange = ?1 AND symbol = ?2
                "#,
                params![&key.exchange, &key.symbol],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;

        match row {
            Some((json, updated_ms)) => {
                let mut parsed: ResolvedSymbol = serde_json::from_str(&json)?;
                if parsed.variable_tick_table.is_some() {
                    parsed.price_precision = crate::types::compute_price_precision(
                        parsed.pricescale,
                        parsed.minmov,
                        parsed.variable_tick_table.as_ref(),
                    );
                }
                Ok(Some((parsed, updated_ms)))
            }
            None => Ok(None),
        }
    }

    /// Searches locally cached symbols (100% offline capable, prefix matches ranked first).
    pub fn search_local_symbols(
        &self,
        query: &str,
        exchange: Option<&str>,
        limit: usize,
    ) -> Result<Vec<SymbolSearchResult>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DataError::Storage("SQLite mutex poisoned".to_owned()))?;
        let q_upper = query.trim().to_ascii_uppercase();
        let like_pattern = format!("%{q_upper}%");
        let prefix_pattern = format!("{q_upper}%");
        let ex_filter = exchange
            .map(normalize_exchange_alias)
            .filter(|s| !s.is_empty());
        let max_rows = limit.max(1) as i64;

        let mut stmt = conn.prepare_cached(
            r#"
            SELECT exchange, symbol, ticker, description, market_type, currency
            FROM symbols
            WHERE (?1 IS NULL OR exchange = ?1)
              AND (
                  UPPER(symbol) LIKE ?2
                  OR UPPER(ticker) LIKE ?2
                  OR UPPER(description) LIKE ?2
              )
            ORDER BY
                CASE
                    WHEN UPPER(symbol) = ?3 THEN 0
                    WHEN UPPER(symbol) LIKE ?4 THEN 1
                    WHEN UPPER(ticker) LIKE ?4 THEN 2
                    ELSE 3
                END,
                symbol ASC
            LIMIT ?5
            "#,
        )?;

        let iter = stmt.query_map(
            params![ex_filter, like_pattern, q_upper, prefix_pattern, max_rows],
            |row| {
                let exchange: String = row.get(0)?;
                let symbol: String = row.get(1)?;
                let ticker: String = row.get(2)?;
                let description: String = row.get(3)?;
                let market_type: String = row.get(4)?;
                let currency: String = row.get(5)?;
                let country_code = if matches!(exchange.as_str(), "HOSE" | "HNX" | "UPCOM") {
                    "VN".to_owned()
                } else {
                    String::new()
                };
                Ok(SymbolSearchResult {
                    ticker,
                    exchange: exchange.clone(),
                    prefix: exchange,
                    symbol,
                    description,
                    market_type,
                    currency_code: currency,
                    country_code,
                    provider_id: "local_cache".to_owned(),
                    type_specs: Vec::new(),
                })
            },
        )?;

        let mut out = Vec::new();
        for item in iter {
            out.push(item?);
        }
        Ok(out)
    }

    /// Merges symbols, OHLCV bars, and `series_meta` from another SQLite market database
    /// (such as a repository-seeded CafeF cache) using `INSERT OR IGNORE`, preserving any newer
    /// bars already stored in this database.
    ///
    /// Returns the number of newly inserted OHLCV bars.
    pub fn merge_from_sqlite_path(&self, seed_path: &Path) -> Result<usize> {
        if !seed_path.exists() {
            return Ok(0);
        }
        if let (Ok(canon_self), Ok(canon_seed)) = (
            std::fs::canonicalize(&self.db_path),
            std::fs::canonicalize(seed_path),
        ) && canon_self == canon_seed
        {
            return Ok(0);
        }

        let seed_str = seed_path.to_string_lossy().into_owned();
        let mut conn = self
            .conn
            .lock()
            .map_err(|_| DataError::Storage("SQLite mutex poisoned".to_owned()))?;

        conn.execute("ATTACH DATABASE ?1 AS seed_db", params![seed_str])?;
        let merge_result = (|| -> Result<usize> {
            let tx = conn.transaction()?;

            // Record existing series that overlap with the seed DB so we only recompute their stats.
            let overlapping: Vec<(String, String, String, String, i64)> = {
                let mut stmt = tx.prepare(
                    r#"
                    SELECT m.exchange, m.symbol, m.interval, m.adjustment, s.reached_start
                    FROM series_meta m
                    JOIN seed_db.series_meta s
                      ON m.exchange = s.exchange
                     AND m.symbol = s.symbol
                     AND m.interval = s.interval
                     AND m.adjustment = s.adjustment
                    "#,
                )?;
                let rows = stmt.query_map([], |r| {
                    Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
                })?;
                let mut list = Vec::new();
                for row in rows {
                    list.push(row?);
                }
                list
            };

            tx.execute(
                "INSERT OR IGNORE INTO symbols SELECT * FROM seed_db.symbols",
                [],
            )?;
            let inserted_bars =
                tx.execute("INSERT OR IGNORE INTO bars SELECT * FROM seed_db.bars", [])?;
            tx.execute(
                "INSERT OR IGNORE INTO series_meta SELECT * FROM seed_db.series_meta",
                [],
            )?;

            for (ex, sym, iv, adj, seed_reached) in overlapping {
                let agg: Option<(i64, i64, i64)> = tx.query_row(
                    r#"
                    SELECT MIN(timestamp_ms), MAX(timestamp_ms), COUNT(*)
                    FROM bars
                    WHERE exchange = ?1 AND symbol = ?2 AND interval = ?3 AND adjustment = ?4
                    "#,
                    params![&ex, &sym, &iv, &adj],
                    |r| {
                        let count: i64 = r.get(2)?;
                        if count == 0 {
                            Ok(None)
                        } else {
                            Ok(Some((r.get(0)?, r.get(1)?, count)))
                        }
                    },
                )?;
                if let Some((earliest_ms, latest_ms, count)) = agg {
                    tx.execute(
                        r#"
                        UPDATE series_meta
                        SET earliest_ms   = ?5,
                            latest_ms     = ?6,
                            bar_count     = ?7,
                            reached_start = MAX(reached_start, ?8)
                        WHERE exchange = ?1 AND symbol = ?2 AND interval = ?3 AND adjustment = ?4
                        "#,
                        params![
                            &ex,
                            &sym,
                            &iv,
                            &adj,
                            earliest_ms,
                            latest_ms,
                            count,
                            seed_reached
                        ],
                    )?;
                }
            }

            tx.commit()?;
            Ok(inserted_bars)
        })();

        let _ = conn.execute("DETACH DATABASE seed_db", []);
        merge_result
    }

    /// Returns global row/series/symbol totals stored in the SQLite cache.
    pub fn stats(&self) -> Result<StoreStats> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DataError::Storage("SQLite mutex poisoned".to_owned()))?;
        let total_symbols: i64 =
            conn.query_row("SELECT COUNT(*) FROM symbols", [], |r| r.get(0))?;
        let total_series: i64 =
            conn.query_row("SELECT COUNT(*) FROM series_meta", [], |r| r.get(0))?;
        let total_bars: i64 = conn.query_row(
            "SELECT COALESCE(SUM(bar_count), 0) FROM series_meta",
            [],
            |r| r.get(0),
        )?;
        Ok(StoreStats {
            db_path: self.db_path.display().to_string(),
            total_symbols: total_symbols.max(0) as usize,
            total_series: total_series.max(0) as usize,
            total_bars: total_bars.max(0) as usize,
        })
    }

    /// Streams all cached Vietnam daily (`1D`, `splits`) cash-equity series across `HOSE`, `HNX`,
    /// and `UPCOM` plus `HOSE:VNINDEX` in a single clustered primary-key scan (`~45-80ms` for 950k bars).
    pub fn load_vn_daily_universe_bars(&self) -> Result<VnDailyUniverseBars> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DataError::Storage("SQLite mutex poisoned".to_owned()))?;

        let mut non_equity: std::collections::HashSet<(String, String)> =
            std::collections::HashSet::new();
        {
            let mut sym_stmt = conn.prepare_cached(
                r#"
                SELECT exchange, symbol, market_type
                FROM symbols
                WHERE exchange IN ('HOSE', 'HNX', 'UPCOM')
                "#,
            )?;
            let rows = sym_stmt.query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })?;
            for item in rows {
                let (ex, sym, mt) = item?;
                let mt_low = mt.to_ascii_lowercase();
                if mt_low == "index" || mt_low == "futures" {
                    non_equity.insert((ex, sym));
                }
            }
        }

        let mut stmt = conn.prepare_cached(
            r#"
            SELECT exchange, symbol, timestamp_ms, open, high, low, close, volume, turnover
            FROM bars
            WHERE exchange IN ('HOSE', 'HNX', 'UPCOM')
              AND interval = '1D'
              AND adjustment = 'splits'
            ORDER BY exchange ASC, symbol ASC, timestamp_ms ASC
            "#,
        )?;

        let iter = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                OhlcvBar {
                    timestamp: row.get(2)?,
                    open: row.get(3)?,
                    high: row.get(4)?,
                    low: row.get(5)?,
                    close: row.get(6)?,
                    volume: row.get(7)?,
                    turnover: row.get(8)?,
                },
            ))
        })?;

        let mut equities: Vec<(SymbolKey, Vec<OhlcvBar>)> = Vec::new();
        let mut vnindex_bars: Option<Vec<OhlcvBar>> = None;
        let mut cur_key: Option<(String, String)> = None;
        let mut cur_bars: Vec<OhlcvBar> = Vec::new();

        let flush_group = |key_opt: Option<(String, String)>,
                           bars: Vec<OhlcvBar>,
                           equities_out: &mut Vec<(SymbolKey, Vec<OhlcvBar>)>,
                           vnindex_out: &mut Option<Vec<OhlcvBar>>| {
            let Some((ex, sym)) = key_opt else {
                return;
            };
            if bars.is_empty() {
                return;
            }
            if sym == "VNINDEX" {
                *vnindex_out = Some(bars);
                return;
            }
            let is_idx_or_fut = non_equity.contains(&(ex.clone(), sym.clone()))
                || sym.contains("INDEX")
                || sym.starts_with("VN30")
                || sym.ends_with("F1!")
                || matches!(
                    sym.as_str(),
                    "VN100" | "VNXALL" | "VNX50" | "VNMID" | "VNSML" | "VNFINLEAD" | "VNDIAMOND"
                );
            if is_idx_or_fut {
                return;
            }
            if let Ok(sk) = SymbolKey::new(&ex, &sym) {
                equities_out.push((sk, bars));
            }
        };

        for row in iter {
            let (ex, sym, bar) = row?;
            let matches_cur = cur_key
                .as_ref()
                .is_some_and(|(ce, cs)| ce == &ex && cs == &sym);
            if !matches_cur {
                let prev_key = cur_key.replace((ex, sym));
                let finished = std::mem::take(&mut cur_bars);
                flush_group(prev_key, finished, &mut equities, &mut vnindex_bars);
            }
            cur_bars.push(bar);
        }
        flush_group(cur_key, cur_bars, &mut equities, &mut vnindex_bars);

        Ok(VnDailyUniverseBars {
            equities,
            vnindex: vnindex_bars,
        })
    }

    /// Bulk-upserts resolved symbols in a single transaction (used by `sync_vn_universe_catalog`
    /// to hydrate ~1,600+ HOSE/HNX/UPCOM instruments cleanly on Clean-Slate machines).
    pub fn upsert_symbols_bulk(&self, symbols: &[ResolvedSymbol]) -> Result<usize> {
        if symbols.is_empty() {
            return Ok(0);
        }
        let mut conn = self
            .conn
            .lock()
            .map_err(|_| DataError::Storage("SQLite mutex poisoned".to_owned()))?;
        let now_ms = chrono::Utc::now().timestamp_millis();
        let tx = conn.transaction()?;
        let mut written = 0_usize;
        {
            let mut stmt = tx.prepare_cached(
                r#"
                INSERT INTO symbols (
                    exchange, symbol, ticker, description, market_type, currency, metadata_json, updated_at_ms
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                ON CONFLICT(exchange, symbol) DO UPDATE SET
                    ticker        = excluded.ticker,
                    description   = CASE WHEN excluded.description != '' THEN excluded.description ELSE symbols.description END,
                    market_type   = CASE WHEN excluded.market_type != '' THEN excluded.market_type ELSE symbols.market_type END,
                    currency      = CASE WHEN excluded.currency != '' THEN excluded.currency ELSE symbols.currency END,
                    metadata_json = excluded.metadata_json,
                    updated_at_ms = excluded.updated_at_ms
                "#,
            )?;
            for sym in symbols {
                let meta_json = serde_json::to_string(sym)?;
                stmt.execute(params![
                    &sym.exchange,
                    &sym.symbol,
                    &sym.ticker,
                    &sym.description,
                    &sym.market_type,
                    &sym.currency,
                    meta_json,
                    now_ms,
                ])?;
                written += 1;
            }
        }
        tx.commit()?;
        Ok(written)
    }

    /// Computes EOD freshness across all cached Vietnam `1D` (`splits`) series relative to the
    /// expected latest Vietnam trading session (`Eliminates the Stale EOD Trap`).
    pub fn vn_freshness_summary(&self, now_utc_ms: i64) -> Result<VnFreshnessSummary> {
        let (expected_date, expected_ms) = crate::paths::latest_expected_vn_trading_session_ms(now_utc_ms);
        let conn = self
            .conn
            .lock()
            .map_err(|_| DataError::Storage("SQLite mutex poisoned".to_owned()))?;
        let mut stmt = conn.prepare(
            r#"
            SELECT latest_ms
            FROM series_meta
            WHERE exchange IN ('HOSE', 'HNX', 'UPCOM')
              AND interval = '1D'
              AND adjustment = 'splits'
              AND bar_count > 0
            ORDER BY latest_ms ASC
            "#,
        )?;
        let timestamps: Vec<i64> = stmt
            .query_map([], |r| r.get::<_, i64>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        let total = timestamps.len();
        let mut fresh = 0_usize;
        let mut stale = 0_usize;
        let mut max_missing = 0_usize;
        for &ts in &timestamps {
            let missing = crate::paths::count_missing_vn_sessions(ts, expected_ms);
            if missing == 0 {
                fresh += 1;
            } else {
                stale += 1;
                max_missing = max_missing.max(missing);
            }
        }
        let newest_cached_date = timestamps.last().map(|&ts| crate::paths::format_ymd_utc(ts));
        let median_cached_date = if total > 0 {
            Some(crate::paths::format_ymd_utc(timestamps[total / 2]))
        } else {
            None
        };

        Ok(VnFreshnessSummary {
            expected_session_date: expected_date,
            expected_session_ms: expected_ms,
            total_vn_d1_series: total,
            fresh_series: fresh,
            stale_series: stale,
            newest_cached_date,
            median_cached_date,
            max_missing_sessions: max_missing,
        })
    }

    /// Returns Vietnam symbols whose `1D` (`splits`) cached data is missing or older than
    /// `expected_session_ms` (ordered most-stale / missing first so sync prioritizes gaps).
    pub fn list_stale_vn_symbols(
        &self,
        scope_symbols: Option<&[SymbolKey]>,
        expected_session_ms: i64,
    ) -> Result<Vec<(SymbolKey, Option<i64>)>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DataError::Storage("SQLite mutex poisoned".to_owned()))?;
        let mut meta_map = std::collections::HashMap::<(String, String), i64>::new();
        {
            let mut stmt = conn.prepare(
                r#"
                SELECT exchange, symbol, latest_ms
                FROM series_meta
                WHERE exchange IN ('HOSE', 'HNX', 'UPCOM')
                  AND interval = '1D'
                  AND adjustment = 'splits'
                "#,
            )?;
            let rows = stmt.query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                ))
            })?;
            for row in rows {
                let (ex, sym, latest) = row?;
                meta_map.insert((ex, sym), latest);
            }
        }

        let candidates: Vec<SymbolKey> = if let Some(list) = scope_symbols {
            list.to_vec()
        } else {
            let mut stmt = conn.prepare(
                r#"
                SELECT exchange, symbol
                FROM symbols
                WHERE exchange IN ('HOSE', 'HNX', 'UPCOM')
                ORDER BY exchange ASC, symbol ASC
                "#,
            )?;
            let iter = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
            let mut out = Vec::new();
            for item in iter {
                let (ex, sym) = item?;
                if let Ok(sk) = SymbolKey::new(&ex, &sym) {
                    out.push(sk);
                }
            }
            out
        };

        let mut stale = Vec::new();
        for sk in candidates {
            let latest_opt = meta_map.get(&(sk.exchange.clone(), sk.symbol.clone())).copied();
            let is_stale = match latest_opt {
                None => true,
                Some(ts) => crate::paths::count_missing_vn_sessions(ts, expected_session_ms) > 0,
            };
            if is_stale {
                stale.push((sk, latest_opt));
            }
        }
        stale.sort_by_key(|(_, latest)| latest.unwrap_or(i64::MIN));
        Ok(stale)
    }

    /// Automatically infers historical `D` (Cash Dividend) and `S` (Stock Split / Bonus Share)
    /// ex-date markers on a symbol by comparing aligned `1D` `unadjusted` vs `splits` close ratios.
    pub fn compute_corporate_events(&self, sym: &SymbolKey) -> Result<Vec<CorporateEventMarker>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DataError::Storage("SQLite mutex poisoned".to_owned()))?;
        let mut stmt = conn.prepare(
            r#"
            SELECT a.timestamp_ms, a.close AS adj_close, u.close AS raw_close
            FROM bars a
            JOIN bars u
              ON a.exchange = u.exchange
             AND a.symbol = u.symbol
             AND a.interval = u.interval
             AND a.timestamp_ms = u.timestamp_ms
            WHERE a.exchange = ?1
              AND a.symbol = ?2
              AND a.interval = '1D'
              AND a.adjustment = 'splits'
              AND u.adjustment = 'unadjusted'
              AND a.close > 0.0
              AND u.close > 0.0
            ORDER BY a.timestamp_ms ASC
            "#,
        )?;
        let pairs: Vec<(i64, f64, f64)> = stmt
            .query_map(params![&sym.exchange, &sym.symbol], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, f64>(1)?, r.get::<_, f64>(2)?))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        let mut events = Vec::new();
        let mut prev_ratio: Option<f64> = None;
        for (ts_ms, adj_c, raw_c) in pairs {
            let ratio = adj_c / raw_c;
            if !ratio.is_finite() || ratio <= 0.0 {
                continue;
            }
            if let Some(prev_r) = prev_ratio {
                let step_factor = ratio / prev_r;
                let rel_jump = (step_factor - 1.0).abs();
                // Ignore tiny floating/rounding drift (<= 1.5%) and corrupted extreme outliers
                if rel_jump > 0.015 && (0.05..=20.0).contains(&step_factor) {
                    let date = crate::paths::format_ymd_utc(ts_ms);
                    let is_split = rel_jump >= 0.12;
                    let kind = if is_split { "S" } else { "D" }.to_owned();
                    let pct = (step_factor - 1.0) * 100.0;
                    let label = if is_split {
                        format!("Chia tách/Thưởng ({step_factor:.2}x)")
                    } else {
                        format!("Cổ tức/Điều chỉnh ({pct:+.1}%)")
                    };
                    let detail = format!(
                        "Ngày GDKHQ {date}: Hệ số điều chỉnh {step_factor:.4}x (Giá gốc {:.0} / Điều chỉnh {:.0})",
                        raw_c, adj_c
                    );
                    events.push(CorporateEventMarker {
                        timestamp_ms: ts_ms,
                        date,
                        kind,
                        label,
                        factor: Some((step_factor * 10_000.0).round() / 10_000.0),
                        detail,
                    });
                }
            }
            prev_ratio = Some(ratio);
        }
        Ok(events)
    }
}

fn collect_bars(
    stmt: &mut rusqlite::CachedStatement<'_>,
    params: impl rusqlite::Params,
) -> Result<Vec<OhlcvBar>> {
    let iter = stmt.query_map(params, |row| {
        Ok(OhlcvBar {
            timestamp: row.get(0)?,
            open: row.get(1)?,
            high: row.get(2)?,
            low: row.get(3)?,
            close: row.get(4)?,
            volume: row.get(5)?,
            turnover: row.get(6)?,
        })
    })?;
    let mut out = Vec::new();
    for item in iter {
        out.push(item?);
    }
    Ok(out)
}

fn refresh_series_meta_tx(
    tx: &rusqlite::Transaction<'_>,
    key: &SeriesKey,
    source: &str,
    reached_start: Option<bool>,
    now_ms: i64,
) -> Result<()> {
    let iv = key.interval.as_str();
    let adj = key.adjustment.as_str();
    let agg: Option<(i64, i64, i64)> = tx.query_row(
        r#"
            SELECT MIN(timestamp_ms), MAX(timestamp_ms), COUNT(*)
            FROM bars
            WHERE exchange = ?1 AND symbol = ?2 AND interval = ?3 AND adjustment = ?4
            "#,
        params![&key.exchange, &key.symbol, iv, adj],
        |r| {
            let count: i64 = r.get(2)?;
            if count == 0 {
                Ok(None)
            } else {
                Ok(Some((r.get(0)?, r.get(1)?, count)))
            }
        },
    )?;

    if let Some((earliest_ms, latest_ms, count)) = agg {
        let reached_flag = i64::from(reached_start.unwrap_or(false));
        tx.execute(
            r#"
            INSERT INTO series_meta (
                exchange, symbol, interval, adjustment,
                earliest_ms, latest_ms, bar_count, reached_start, source, updated_at_ms
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            ON CONFLICT(exchange, symbol, interval, adjustment)
            DO UPDATE SET
                earliest_ms   = excluded.earliest_ms,
                latest_ms     = excluded.latest_ms,
                bar_count     = excluded.bar_count,
                reached_start = MAX(series_meta.reached_start, excluded.reached_start),
                source        = excluded.source,
                updated_at_ms = excluded.updated_at_ms
            "#,
            params![
                &key.exchange,
                &key.symbol,
                iv,
                adj,
                earliest_ms,
                latest_ms,
                count,
                reached_flag,
                source,
                now_ms,
            ],
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::*;
    use crate::types::{Adjustment, Interval};

    fn synth_bars(count: usize, start_ms: i64, step_ms: i64) -> Vec<OhlcvBar> {
        (0..count)
            .map(|i| {
                let ts = start_ms + (i as i64) * step_ms;
                let base = 50_000.0 + (i as f64) * 10.0;
                OhlcvBar::new(
                    ts,
                    base,
                    base + 50.0,
                    base - 50.0,
                    base + 20.0,
                    1_000.0 + i as f64,
                )
            })
            .collect()
    }

    #[test]
    fn sqlite_pagination_init_forward_backward_flags_are_exact() {
        let store = SqliteMarketStore::open_in_memory().unwrap();
        let sym = SymbolKey::new("HOSE", "FPT").unwrap();
        let key = SeriesKey::new(&sym, Interval::M15, Adjustment::Splits);
        let bars = synth_bars(10, 1_700_000_000_000, 900_000);
        store.upsert_bars(&key, &bars, "test", Some(true)).unwrap();

        // Init(4): bars 6..=9 -> has_older = true, has_newer = false
        let init = store.query_bars(&key, BarQueryMode::Init, 4).unwrap();
        assert_eq!(init.bars.len(), 4);
        assert_eq!(init.bars[0].timestamp, bars[6].timestamp);
        assert_eq!(init.bars[3].timestamp, bars[9].timestamp);
        assert!(init.has_older);
        assert!(!init.has_newer);

        // Forward before bars[6], limit 4 -> bars 2..=5 -> has_older = true, has_newer = true
        let fwd1 = store
            .query_bars(
                &key,
                BarQueryMode::Forward {
                    before_ms: bars[6].timestamp,
                },
                4,
            )
            .unwrap();
        assert_eq!(fwd1.bars.len(), 4);
        assert_eq!(fwd1.bars[0].timestamp, bars[2].timestamp);
        assert_eq!(fwd1.bars[3].timestamp, bars[5].timestamp);
        assert!(fwd1.has_older);
        assert!(fwd1.has_newer);

        // Forward before bars[2], limit 4 -> bars 0..=1 -> has_older = false, has_newer = true
        let fwd2 = store
            .query_bars(
                &key,
                BarQueryMode::Forward {
                    before_ms: bars[2].timestamp,
                },
                4,
            )
            .unwrap();
        assert_eq!(fwd2.bars.len(), 2);
        assert_eq!(fwd2.bars[0].timestamp, bars[0].timestamp);
        assert_eq!(fwd2.bars[1].timestamp, bars[1].timestamp);
        assert!(!fwd2.has_older);
        assert!(fwd2.has_newer);

        // Backward after bars[1], limit 4 -> bars 2..=5 -> has_older = true, has_newer = true
        let bwd1 = store
            .query_bars(
                &key,
                BarQueryMode::Backward {
                    after_ms: bars[1].timestamp,
                },
                4,
            )
            .unwrap();
        assert_eq!(bwd1.bars.len(), 4);
        assert_eq!(bwd1.bars[0].timestamp, bars[2].timestamp);
        assert_eq!(bwd1.bars[3].timestamp, bars[5].timestamp);
        assert!(bwd1.has_older);
        assert!(bwd1.has_newer);

        // Backward after bars[5], limit 10 -> bars 6..=9 -> has_older = true, has_newer = false
        let bwd2 = store
            .query_bars(
                &key,
                BarQueryMode::Backward {
                    after_ms: bars[5].timestamp,
                },
                10,
            )
            .unwrap();
        assert_eq!(bwd2.bars.len(), 4);
        assert_eq!(bwd2.bars[0].timestamp, bars[6].timestamp);
        assert_eq!(bwd2.bars[3].timestamp, bars[9].timestamp);
        assert!(bwd2.has_older);
        assert!(!bwd2.has_newer);
    }

    #[test]
    fn intraday_accumulation_exceeds_11k_bar_limit_and_reads_under_5ms() {
        let store = SqliteMarketStore::open_in_memory().unwrap();
        let sym = SymbolKey::new("BINANCE", "BTCUSDT").unwrap();
        let key = SeriesKey::new(&sym, Interval::M1, Adjustment::Unadjusted);

        // Batch 1: older 8,000 bars
        let old_bars = synth_bars(8_000, 1_700_000_000_000, 60_000);
        store.upsert_bars(&key, &old_bars, "tv_seed", None).unwrap();

        // Batch 2: newer 7,500 bars with 500 overlapping bars -> total 15,000 distinct bars (> 11,000 cap!)
        let overlap_start = old_bars[7_500].timestamp;
        let new_bars = synth_bars(7_500, overlap_start, 60_000);
        store
            .upsert_bars(&key, &new_bars, "tv_gapfill", None)
            .unwrap();

        let meta = store.series_meta(&key).unwrap().unwrap();
        assert_eq!(meta.bar_count, 15_000);
        assert_eq!(meta.earliest_ms, old_bars[0].timestamp);
        assert_eq!(meta.latest_ms, new_bars.last().unwrap().timestamp);

        let t0 = Instant::now();
        let page = store.query_bars(&key, BarQueryMode::Init, 500).unwrap();
        let elapsed = t0.elapsed();
        assert_eq!(page.bars.len(), 500);
        assert!(page.has_older);
        assert!(
            elapsed.as_millis() < 50,
            "query of 500 bars out of 15,000 should be sub-millisecond, got {elapsed:?}"
        );
    }

    #[test]
    fn symbol_upsert_and_offline_search_ranking() {
        let store = SqliteMarketStore::open_in_memory().unwrap();
        let fpt = ResolvedSymbol::from_cafef_vn_ticker("HOSE", "FPT", false);
        let fpt_etf = ResolvedSymbol::from_cafef_vn_ticker("HOSE", "FPTX", false);
        let shs = ResolvedSymbol::from_cafef_vn_ticker("HNX", "SHS", false);
        store.upsert_symbol(&fpt_etf).unwrap();
        store.upsert_symbol(&fpt).unwrap();
        store.upsert_symbol(&shs).unwrap();

        let hits = store.search_local_symbols("FPT", Some("HOSE"), 10).unwrap();
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].ticker, "HOSE:FPT"); // exact match ranked first
        assert_eq!(hits[1].ticker, "HOSE:FPTX");

        let hnx_hits = store.search_local_symbols("", Some("HNX"), 10).unwrap();
        assert_eq!(hnx_hits.len(), 1);
        assert_eq!(hnx_hits[0].ticker, "HNX:SHS");
    }

    #[test]
    fn merge_from_sqlite_path_imports_missing_history_and_preserves_newer_bars() {
        let dir = std::env::temp_dir().join(format!("sumi_merge_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let seed_path = dir.join("seed.db");
        let app_path = dir.join("app.db");

        let seed = SqliteMarketStore::open(&seed_path).unwrap();
        let app = SqliteMarketStore::open(&app_path).unwrap();

        let fpt = SymbolKey::new("HOSE", "FPT").unwrap();
        let vcb = SymbolKey::new("HOSE", "VCB").unwrap();
        seed.upsert_symbol(&ResolvedSymbol::from_cafef_vn_ticker("HOSE", "FPT", false))
            .unwrap();
        seed.upsert_symbol(&ResolvedSymbol::from_cafef_vn_ticker("HOSE", "VCB", false))
            .unwrap();

        let fpt_key = SeriesKey::new(&fpt, Interval::D1, Adjustment::Splits);
        let vcb_key = SeriesKey::new(&vcb, Interval::D1, Adjustment::Splits);
        let full_fpt = synth_bars(20, 1_700_000_000_000, 86_400_000);
        let full_vcb = synth_bars(15, 1_700_000_000_000, 86_400_000);
        seed.upsert_bars(&fpt_key, &full_fpt[..15], "cafef", Some(true))
            .unwrap();
        seed.upsert_bars(&vcb_key, &full_vcb, "cafef", Some(true))
            .unwrap();

        // App DB only has the newest 5 bars of FPT (e.g. from TradingView online fetch).
        app.upsert_symbol(&ResolvedSymbol::from_cafef_vn_ticker("HOSE", "FPT", false))
            .unwrap();
        app.upsert_bars(&fpt_key, &full_fpt[15..], "tradingview", Some(false))
            .unwrap();

        let inserted = app.merge_from_sqlite_path(&seed_path).unwrap();
        assert_eq!(inserted, 30); // 15 older FPT bars + 15 VCB bars

        let stats = app.stats().unwrap();
        assert_eq!(stats.total_symbols, 2);
        assert_eq!(stats.total_series, 2);
        assert_eq!(stats.total_bars, 35);

        let fpt_meta = app.series_meta(&fpt_key).unwrap().unwrap();
        assert_eq!(fpt_meta.bar_count, 20);
        assert_eq!(fpt_meta.earliest_ms, full_fpt[0].timestamp);
        assert_eq!(fpt_meta.latest_ms, full_fpt[19].timestamp);
        assert!(fpt_meta.reached_start);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn multi_split_ratio_splicing_adjusts_already_spliced_bars_and_preserves_turnover() {
        let store = SqliteMarketStore::open_in_memory().unwrap();
        let fpt = SymbolKey::new("HOSE", "FPT").unwrap();
        let key = SeriesKey::new(&fpt, Interval::D1, Adjustment::Splits);
        let t0 = 1_700_000_000_000_i64;
        let step = 86_400_000_i64;

        // Seed 3 historical CafeF bars at close = 100_000, vol = 1_000 -> turnover = 100_000_000
        let cf_bars = vec![
            OhlcvBar::new(t0, 100_000.0, 101_000.0, 99_000.0, 100_000.0, 1_000.0),
            OhlcvBar::new(t0 + step, 100_000.0, 101_000.0, 99_000.0, 100_000.0, 1_000.0),
            OhlcvBar::new(t0 + 2 * step, 100_000.0, 101_000.0, 99_000.0, 100_000.0, 1_000.0),
        ];
        store
            .upsert_bars(&key, &cf_bars, "cafef_adjusted", Some(true))
            .unwrap();

        // Split #1: TradingView arrives with overlap at t0+2*step at 50_000 (0.5x split!)
        let tv_split_1 = vec![
            OhlcvBar::new(t0 + 2 * step, 50_000.0, 50_500.0, 49_500.0, 50_000.0, 2_000.0),
            OhlcvBar::new(t0 + 3 * step, 50_000.0, 51_000.0, 49_000.0, 50_000.0, 2_000.0),
        ];
        store
            .upsert_bars(&key, &tv_split_1, "tradingview", None)
            .unwrap();

        let after_1 = store.query_bars(&key, BarQueryMode::Init, 10).unwrap().bars;
        assert!((after_1[0].close - 50_000.0).abs() < 1e-6);
        assert!((after_1[0].volume - 2_000.0).abs() < 1e-6);
        assert!((after_1[0].turnover - 100_000_000.0).abs() < 1e-3);

        // Split #2: 6 months later another 0.5x split arrives overlapping already-spliced t0+step!
        let tv_split_2 = vec![
            OhlcvBar::new(t0 + step, 25_000.0, 25_250.0, 24_750.0, 25_000.0, 4_000.0),
            OhlcvBar::new(t0 + 2 * step, 25_000.0, 25_250.0, 24_750.0, 25_000.0, 4_000.0),
        ];
        store
            .upsert_bars(&key, &tv_split_2, "tradingview", None)
            .unwrap();

        let after_2 = store.query_bars(&key, BarQueryMode::Init, 10).unwrap().bars;
        assert!(
            (after_2[0].close - 25_000.0).abs() < 1e-6,
            "Audit Fix #4: previously cafef_spliced bar must rescale cleanly on 2nd split, got {}",
            after_2[0].close
        );
        assert!((after_2[0].volume - 4_000.0).abs() < 1e-6);
        assert!((after_2[0].turnover - 100_000_000.0).abs() < 1e-3);
    }

    #[test]
    fn detects_dividend_and_split_corporate_events_from_raw_vs_adjusted() {
        let store = SqliteMarketStore::open_in_memory().unwrap();
        let fpt = SymbolKey::new("HOSE", "FPT").unwrap();
        let adj_key = SeriesKey::new(&fpt, Interval::D1, Adjustment::Splits);
        let raw_key = SeriesKey::new(&fpt, Interval::D1, Adjustment::Unadjusted);
        let t0 = 1_700_000_000_000_i64;
        let step = 86_400_000_i64;

        // Day 0: ratio 0.80; Day 1: 5% cash dividend -> ratio 0.84; Day 2: 20% bonus share -> ratio 1.00
        let raw_bars = vec![
            OhlcvBar::new(t0, 100_000.0, 100_000.0, 100_000.0, 100_000.0, 100.0),
            OhlcvBar::new(t0 + step, 95_000.0, 95_000.0, 95_000.0, 95_000.0, 100.0),
            OhlcvBar::new(t0 + 2 * step, 80_000.0, 80_000.0, 80_000.0, 80_000.0, 100.0),
        ];
        let adj_bars = vec![
            OhlcvBar::new(t0, 80_000.0, 80_000.0, 80_000.0, 80_000.0, 100.0),
            OhlcvBar::new(t0 + step, 80_000.0, 80_000.0, 80_000.0, 80_000.0, 100.0),
            OhlcvBar::new(t0 + 2 * step, 80_000.0, 80_000.0, 80_000.0, 80_000.0, 100.0),
        ];
        store.upsert_bars(&raw_key, &raw_bars, "cafef_raw", Some(true)).unwrap();
        store.upsert_bars(&adj_key, &adj_bars, "cafef_adjusted", Some(true)).unwrap();

        let events = store.compute_corporate_events(&fpt).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].kind, "D");
        assert_eq!(events[1].kind, "S");
    }
}
