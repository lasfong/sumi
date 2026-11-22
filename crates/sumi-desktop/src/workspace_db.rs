use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{DesktopError, DesktopResult};

const WORKSPACE_SCHEMA_VERSION: i32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedDrawingRecord {
    pub id: String,
    pub ticker: String,
    pub name: String,
    pub pane_id: String,
    pub group_id: String,
    pub points: Value,
    pub styles: Option<Value>,
    pub extend_data: Option<Value>,
    pub lock: bool,
    pub visible: bool,
    pub mode: String,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedIndicatorSpec {
    pub id: String,
    pub name: String,
    pub pane_id: String,
    pub is_main: bool,
    pub calc_params: Vec<f64>,
    pub visible: bool,
    pub styles: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedLayoutRecord {
    pub id: String,
    pub name: String,
    pub layout_json: Value,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchlistGroup {
    pub id: String,
    pub name: String,
    pub tickers: Vec<String>,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PriceAlertRecord {
    pub id: String,
    pub ticker: String,
    /// `"cross_above"` | `"cross_below"` | `"cross_any"`
    pub condition: String,
    pub target_price: f64,
    pub message: String,
    /// `"armed"` | `"triggered"` | `"paused"`
    pub status: String,
    pub created_at_ms: i64,
    pub triggered_at_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalAnnotationRecord {
    pub trade_id: u64,
    pub ticker: String,
    pub setup_tag: String,
    pub mistake_tag: String,
    pub note: String,
    pub updated_at_ms: i64,
}

#[derive(Clone)]
pub struct WorkspaceStore {
    conn: Arc<Mutex<Connection>>,
    db_path: PathBuf,
}

impl WorkspaceStore {
    pub fn open(path: impl AsRef<Path>) -> DesktopResult<Self> {
        let path_buf = path.as_ref().to_path_buf();
        if let Some(parent) = path_buf.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(&path_buf)?;
        Self::configure_and_migrate(&conn, false)?;
        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
            db_path: path_buf,
        };
        store.seed_default_watchlists()?;
        Ok(store)
    }

    pub fn open_in_memory() -> DesktopResult<Self> {
        let conn = Connection::open_in_memory()?;
        Self::configure_and_migrate(&conn, true)?;
        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
            db_path: PathBuf::from(":memory:"),
        };
        store.seed_default_watchlists()?;
        Ok(store)
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.db_path
    }

    fn configure_and_migrate(conn: &Connection, in_memory: bool) -> DesktopResult<()> {
        if !in_memory {
            conn.pragma_update(None, "journal_mode", "WAL")?;
        }
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "busy_timeout", 5_000_i64)?;

        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS kv_store (
                key           TEXT PRIMARY KEY,
                value_json    TEXT NOT NULL,
                updated_at_ms INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS chart_layouts (
                id            TEXT PRIMARY KEY,
                name          TEXT NOT NULL,
                layout_json   TEXT NOT NULL,
                updated_at_ms INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS drawings (
                id               TEXT NOT NULL,
                ticker           TEXT NOT NULL,
                name             TEXT NOT NULL,
                pane_id          TEXT NOT NULL,
                group_id         TEXT NOT NULL,
                points_json      TEXT NOT NULL,
                styles_json      TEXT,
                extend_data_json TEXT,
                locked           INTEGER NOT NULL DEFAULT 0,
                visible          INTEGER NOT NULL DEFAULT 1,
                mode             TEXT NOT NULL DEFAULT 'normal',
                updated_at_ms    INTEGER NOT NULL,
                PRIMARY KEY (ticker, id)
            );
            CREATE INDEX IF NOT EXISTS idx_drawings_ticker ON drawings (ticker);

            CREATE TABLE IF NOT EXISTS pane_indicators (
                slot_id         TEXT PRIMARY KEY,
                indicators_json TEXT NOT NULL,
                updated_at_ms   INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS watchlists (
                id         TEXT PRIMARY KEY,
                name       TEXT NOT NULL,
                sort_order INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS watchlist_items (
                watchlist_id TEXT NOT NULL,
                ticker       TEXT NOT NULL,
                sort_order   INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (watchlist_id, ticker),
                FOREIGN KEY (watchlist_id) REFERENCES watchlists(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS price_alerts (
                id              TEXT PRIMARY KEY,
                ticker          TEXT NOT NULL,
                condition       TEXT NOT NULL,
                target_price    REAL NOT NULL,
                message         TEXT NOT NULL,
                status          TEXT NOT NULL DEFAULT 'armed',
                created_at_ms   INTEGER NOT NULL,
                triggered_at_ms INTEGER
            );
            CREATE INDEX IF NOT EXISTS idx_alerts_ticker ON price_alerts (ticker);

            CREATE TABLE IF NOT EXISTS journal_annotations (
                trade_id      INTEGER NOT NULL,
                ticker        TEXT NOT NULL,
                setup_tag     TEXT NOT NULL DEFAULT '',
                mistake_tag   TEXT NOT NULL DEFAULT '',
                note          TEXT NOT NULL DEFAULT '',
                updated_at_ms INTEGER NOT NULL,
                PRIMARY KEY (ticker, trade_id)
            );
            "#,
        )?;
        conn.pragma_update(None, "user_version", WORKSPACE_SCHEMA_VERSION)?;
        Ok(())
    }

    fn seed_default_watchlists(&self) -> DesktopResult<()> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM watchlists", [], |r| r.get(0))?;
        if count > 0 {
            return Ok(());
        }

        let defaults: [(&str, &str, i32, &[&str]); 4] = [
            (
                "wl_vn30",
                "VN30 Bluechips",
                0,
                &[
                    "HOSE:FPT", "HOSE:VCB", "HOSE:HPG", "HOSE:MWG", "HOSE:TCB", "HOSE:MBB",
                    "HOSE:SSI", "HOSE:VHM", "HOSE:VNM", "HOSE:VIC", "HOSE:ACB", "HOSE:MSN",
                ],
            ),
            (
                "wl_vn_indices",
                "Chỉ số & Phái sinh",
                1,
                &[
                    "HOSE:VNINDEX",
                    "HOSE:VN30",
                    "HNX:HNXINDEX",
                    "HNX:VN30F1!",
                    "HNX:SHS",
                    "HNX:PVS",
                    "HNX:CEO",
                    "UPCOM:ACV",
                ],
            ),
            (
                "wl_crypto",
                "Crypto Realtime",
                2,
                &[
                    "BINANCE:BTCUSDT",
                    "BINANCE:ETHUSDT",
                    "BINANCE:SOLUSDT",
                    "BINANCE:BNBUSDT",
                    "BINANCE:XRPUSDT",
                ],
            ),
            (
                "wl_global",
                "Quốc tế & FX",
                3,
                &[
                    "NASDAQ:AAPL",
                    "NASDAQ:NVDA",
                    "NASDAQ:MSFT",
                    "AMEX:SPY",
                    "FX_IDC:USDVND",
                    "OANDA:XAUUSD",
                ],
            ),
        ];

        let tx = conn.transaction()?;
        for (id, name, sort_order, tickers) in defaults {
            tx.execute(
                "INSERT INTO watchlists (id, name, sort_order) VALUES (?1, ?2, ?3)",
                params![id, name, sort_order],
            )?;
            for (idx, ticker) in tickers.iter().enumerate() {
                tx.execute(
                    "INSERT INTO watchlist_items (watchlist_id, ticker, sort_order) VALUES (?1, ?2, ?3)",
                    params![id, ticker, idx as i32],
                )?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn get_kv(&self, key: &str) -> DesktopResult<Option<Value>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        let raw: Option<String> = conn
            .query_row(
                "SELECT value_json FROM kv_store WHERE key = ?1",
                params![key],
                |r| r.get(0),
            )
            .optional()?;
        match raw {
            Some(json) => Ok(Some(serde_json::from_str(&json)?)),
            None => Ok(None),
        }
    }

    pub fn set_kv(&self, key: &str, value: &Value) -> DesktopResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        let now_ms = chrono::Utc::now().timestamp_millis();
        let json = serde_json::to_string(value)?;
        conn.execute(
            r#"
            INSERT INTO kv_store (key, value_json, updated_at_ms)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(key) DO UPDATE SET
                value_json = excluded.value_json,
                updated_at_ms = excluded.updated_at_ms
            "#,
            params![key, json, now_ms],
        )?;
        Ok(())
    }

    // ─── Drawings ────────────────────────────────────────────────────────────

    pub fn list_drawings(&self, ticker: &str) -> DesktopResult<Vec<SavedDrawingRecord>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        let normalized = ticker.trim().to_ascii_uppercase();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, ticker, name, pane_id, group_id, points_json, styles_json,
                   extend_data_json, locked, visible, mode, updated_at_ms
            FROM drawings
            WHERE ticker = ?1
            ORDER BY updated_at_ms ASC, id ASC
            "#,
        )?;
        let iter = stmt.query_map(params![normalized], |row| {
            let points_raw: String = row.get(5)?;
            let styles_raw: Option<String> = row.get(6)?;
            let extend_raw: Option<String> = row.get(7)?;
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                points_raw,
                styles_raw,
                extend_raw,
                row.get::<_, i64>(8)? != 0,
                row.get::<_, i64>(9)? != 0,
                row.get::<_, String>(10)?,
                row.get::<_, i64>(11)?,
            ))
        })?;
        let mut out = Vec::new();
        for item in iter {
            let (
                id,
                ticker_val,
                name,
                pane_id,
                group_id,
                points_raw,
                styles_raw,
                extend_raw,
                lock,
                visible,
                mode,
                updated_at_ms,
            ) = item?;
            let points: Value = serde_json::from_str(&points_raw)?;
            let styles = match styles_raw {
                Some(s) => Some(serde_json::from_str(&s)?),
                None => None,
            };
            let extend_data = match extend_raw {
                Some(s) => Some(serde_json::from_str(&s)?),
                None => None,
            };
            out.push(SavedDrawingRecord {
                id,
                ticker: ticker_val,
                name,
                pane_id,
                group_id,
                points,
                styles,
                extend_data,
                lock,
                visible,
                mode,
                updated_at_ms,
            });
        }
        Ok(out)
    }

    pub fn upsert_drawing(&self, mut record: SavedDrawingRecord) -> DesktopResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        record.ticker = record.ticker.trim().to_ascii_uppercase();
        if record.updated_at_ms <= 0 {
            record.updated_at_ms = chrono::Utc::now().timestamp_millis();
        }
        let points_json = serde_json::to_string(&record.points)?;
        let styles_json = record
            .styles
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        let extend_json = record
            .extend_data
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        conn.execute(
            r#"
            INSERT INTO drawings (
                id, ticker, name, pane_id, group_id, points_json, styles_json,
                extend_data_json, locked, visible, mode, updated_at_ms
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            ON CONFLICT(ticker, id) DO UPDATE SET
                name = excluded.name,
                pane_id = excluded.pane_id,
                group_id = excluded.group_id,
                points_json = excluded.points_json,
                styles_json = excluded.styles_json,
                extend_data_json = excluded.extend_data_json,
                locked = excluded.locked,
                visible = excluded.visible,
                mode = excluded.mode,
                updated_at_ms = excluded.updated_at_ms
            "#,
            params![
                record.id,
                record.ticker,
                record.name,
                record.pane_id,
                record.group_id,
                points_json,
                styles_json,
                extend_json,
                i64::from(record.lock),
                i64::from(record.visible),
                record.mode,
                record.updated_at_ms,
            ],
        )?;
        Ok(())
    }

    pub fn delete_drawing(&self, ticker: &str, id: &str) -> DesktopResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        let normalized = ticker.trim().to_ascii_uppercase();
        conn.execute(
            "DELETE FROM drawings WHERE ticker = ?1 AND id = ?2",
            params![normalized, id],
        )?;
        Ok(())
    }

    pub fn clear_drawings(&self, ticker: &str) -> DesktopResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        let normalized = ticker.trim().to_ascii_uppercase();
        conn.execute(
            "DELETE FROM drawings WHERE ticker = ?1",
            params![normalized],
        )?;
        Ok(())
    }

    // ─── Indicators per Pane Slot ────────────────────────────────────────────

    pub fn get_pane_indicators(&self, slot_id: &str) -> DesktopResult<Vec<SavedIndicatorSpec>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        let raw: Option<String> = conn
            .query_row(
                "SELECT indicators_json FROM pane_indicators WHERE slot_id = ?1",
                params![slot_id],
                |r| r.get(0),
            )
            .optional()?;
        match raw {
            Some(json) => Ok(serde_json::from_str(&json)?),
            None => Ok(Vec::new()),
        }
    }

    pub fn save_pane_indicators(
        &self,
        slot_id: &str,
        indicators: &[SavedIndicatorSpec],
    ) -> DesktopResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        let json = serde_json::to_string(indicators)?;
        let now_ms = chrono::Utc::now().timestamp_millis();
        conn.execute(
            r#"
            INSERT INTO pane_indicators (slot_id, indicators_json, updated_at_ms)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(slot_id) DO UPDATE SET
                indicators_json = excluded.indicators_json,
                updated_at_ms = excluded.updated_at_ms
            "#,
            params![slot_id, json, now_ms],
        )?;
        Ok(())
    }

    // ─── Layouts ─────────────────────────────────────────────────────────────

    pub fn list_layouts(&self) -> DesktopResult<Vec<SavedLayoutRecord>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        let mut stmt = conn.prepare(
            "SELECT id, name, layout_json, updated_at_ms FROM chart_layouts ORDER BY updated_at_ms DESC",
        )?;
        let iter = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })?;
        let mut out = Vec::new();
        for item in iter {
            let (id, name, raw, updated_at_ms) = item?;
            out.push(SavedLayoutRecord {
                id,
                name,
                layout_json: serde_json::from_str(&raw)?,
                updated_at_ms,
            });
        }
        Ok(out)
    }

    pub fn save_layout(
        &self,
        id: &str,
        name: &str,
        layout_json: &Value,
    ) -> DesktopResult<SavedLayoutRecord> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        let now_ms = chrono::Utc::now().timestamp_millis();
        let raw = serde_json::to_string(layout_json)?;
        conn.execute(
            r#"
            INSERT INTO chart_layouts (id, name, layout_json, updated_at_ms)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                layout_json = excluded.layout_json,
                updated_at_ms = excluded.updated_at_ms
            "#,
            params![id, name, raw, now_ms],
        )?;
        Ok(SavedLayoutRecord {
            id: id.to_owned(),
            name: name.to_owned(),
            layout_json: layout_json.clone(),
            updated_at_ms: now_ms,
        })
    }

    pub fn delete_layout(&self, id: &str) -> DesktopResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        conn.execute("DELETE FROM chart_layouts WHERE id = ?1", params![id])?;
        Ok(())
    }

    // ─── Watchlists ──────────────────────────────────────────────────────────

    pub fn list_watchlists(&self) -> DesktopResult<Vec<WatchlistGroup>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        let mut stmt = conn.prepare(
            "SELECT id, name, sort_order FROM watchlists ORDER BY sort_order ASC, name ASC",
        )?;
        let groups: Vec<(String, String, i32)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
            .collect::<Result<_, _>>()?;

        let mut item_stmt = conn.prepare(
            "SELECT ticker FROM watchlist_items WHERE watchlist_id = ?1 ORDER BY sort_order ASC, ticker ASC",
        )?;
        let mut out = Vec::with_capacity(groups.len());
        for (id, name, sort_order) in groups {
            let tickers: Vec<String> = item_stmt
                .query_map(params![&id], |r| r.get(0))?
                .collect::<Result<_, _>>()?;
            out.push(WatchlistGroup {
                id,
                name,
                tickers,
                sort_order,
            });
        }
        Ok(out)
    }

    pub fn create_watchlist(&self, id: &str, name: &str) -> DesktopResult<WatchlistGroup> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        let next_order: i32 = conn.query_row(
            "SELECT COALESCE(MAX(sort_order) + 1, 0) FROM watchlists",
            [],
            |r| r.get(0),
        )?;
        conn.execute(
            "INSERT INTO watchlists (id, name, sort_order) VALUES (?1, ?2, ?3)",
            params![id, name, next_order],
        )?;
        Ok(WatchlistGroup {
            id: id.to_owned(),
            name: name.to_owned(),
            tickers: Vec::new(),
            sort_order: next_order,
        })
    }

    pub fn delete_watchlist(&self, id: &str) -> DesktopResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        conn.execute("DELETE FROM watchlists WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn add_watchlist_symbol(&self, watchlist_id: &str, ticker: &str) -> DesktopResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        let normalized = ticker.trim().to_ascii_uppercase();
        let next_order: i32 = conn.query_row(
            "SELECT COALESCE(MAX(sort_order) + 1, 0) FROM watchlist_items WHERE watchlist_id = ?1",
            params![watchlist_id],
            |r| r.get(0),
        )?;
        conn.execute(
            "INSERT OR IGNORE INTO watchlist_items (watchlist_id, ticker, sort_order) VALUES (?1, ?2, ?3)",
            params![watchlist_id, normalized, next_order],
        )?;
        Ok(())
    }

    pub fn remove_watchlist_symbol(&self, watchlist_id: &str, ticker: &str) -> DesktopResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        let normalized = ticker.trim().to_ascii_uppercase();
        conn.execute(
            "DELETE FROM watchlist_items WHERE watchlist_id = ?1 AND ticker = ?2",
            params![watchlist_id, normalized],
        )?;
        Ok(())
    }

    // ─── Price Alerts ────────────────────────────────────────────────────────

    pub fn list_alerts(&self) -> DesktopResult<Vec<PriceAlertRecord>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        let mut stmt = conn.prepare(
            r#"
            SELECT id, ticker, condition, target_price, message, status, created_at_ms, triggered_at_ms
            FROM price_alerts
            ORDER BY created_at_ms DESC
            "#,
        )?;
        let rows = stmt
            .query_map([], |r| {
                Ok(PriceAlertRecord {
                    id: r.get(0)?,
                    ticker: r.get(1)?,
                    condition: r.get(2)?,
                    target_price: r.get(3)?,
                    message: r.get(4)?,
                    status: r.get(5)?,
                    created_at_ms: r.get(6)?,
                    triggered_at_ms: r.get(7)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn upsert_alert(&self, alert: &PriceAlertRecord) -> DesktopResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        conn.execute(
            r#"
            INSERT INTO price_alerts (
                id, ticker, condition, target_price, message, status, created_at_ms, triggered_at_ms
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(id) DO UPDATE SET
                condition = excluded.condition,
                target_price = excluded.target_price,
                message = excluded.message,
                status = excluded.status,
                triggered_at_ms = excluded.triggered_at_ms
            "#,
            params![
                alert.id,
                alert.ticker.trim().to_ascii_uppercase(),
                alert.condition,
                alert.target_price,
                alert.message,
                alert.status,
                alert.created_at_ms,
                alert.triggered_at_ms,
            ],
        )?;
        Ok(())
    }

    pub fn set_alert_status(
        &self,
        id: &str,
        status: &str,
        triggered_at_ms: Option<i64>,
    ) -> DesktopResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        conn.execute(
            "UPDATE price_alerts SET status = ?2, triggered_at_ms = COALESCE(?3, triggered_at_ms) WHERE id = ?1",
            params![id, status, triggered_at_ms],
        )?;
        Ok(())
    }

    pub fn delete_alert(&self, id: &str) -> DesktopResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        conn.execute("DELETE FROM price_alerts WHERE id = ?1", params![id])?;
        Ok(())
    }

    // ─── Trade Journal Annotations ───────────────────────────────────────────

    pub fn list_journal_annotations(
        &self,
        ticker: &str,
    ) -> DesktopResult<Vec<JournalAnnotationRecord>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        let normalized = ticker.trim().to_ascii_uppercase();
        let mut stmt = conn.prepare(
            r#"
            SELECT trade_id, ticker, setup_tag, mistake_tag, note, updated_at_ms
            FROM journal_annotations
            WHERE ticker = ?1
            ORDER BY trade_id DESC
            "#,
        )?;
        let rows = stmt
            .query_map(params![normalized], |r| {
                Ok(JournalAnnotationRecord {
                    trade_id: r.get::<_, i64>(0)?.max(0) as u64,
                    ticker: r.get(1)?,
                    setup_tag: r.get(2)?,
                    mistake_tag: r.get(3)?,
                    note: r.get(4)?,
                    updated_at_ms: r.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn upsert_journal_annotation(
        &self,
        ticker: &str,
        trade_id: u64,
        setup_tag: &str,
        mistake_tag: &str,
        note: &str,
    ) -> DesktopResult<JournalAnnotationRecord> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        let normalized = ticker.trim().to_ascii_uppercase();
        let now_ms = chrono::Utc::now().timestamp_millis();
        conn.execute(
            r#"
            INSERT INTO journal_annotations (trade_id, ticker, setup_tag, mistake_tag, note, updated_at_ms)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(ticker, trade_id) DO UPDATE SET
                setup_tag = excluded.setup_tag,
                mistake_tag = excluded.mistake_tag,
                note = excluded.note,
                updated_at_ms = excluded.updated_at_ms
            "#,
            params![trade_id as i64, &normalized, setup_tag, mistake_tag, note, now_ms],
        )?;
        Ok(JournalAnnotationRecord {
            trade_id,
            ticker: normalized,
            setup_tag: setup_tag.to_owned(),
            mistake_tag: mistake_tag.to_owned(),
            note: note.to_owned(),
            updated_at_ms: now_ms,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn workspace_store_crud_drawings_indicators_watchlists_alerts() {
        let store = WorkspaceStore::open_in_memory().unwrap();

        // Default watchlists seeded automatically
        let lists = store.list_watchlists().unwrap();
        assert_eq!(lists.len(), 4);
        assert!(lists[0].tickers.contains(&"HOSE:FPT".to_owned()));

        // Drawings roundtrip
        store
            .upsert_drawing(SavedDrawingRecord {
                id: "ov_1".to_owned(),
                ticker: "hose:fpt".to_owned(),
                name: "longPosition".to_owned(),
                pane_id: "candle_pane".to_owned(),
                group_id: "drawing".to_owned(),
                points: json!([{"timestamp": 1700000000000_i64, "value": 95000.0}]),
                styles: Some(json!({"line": {"color": "#089981"}})),
                extend_data: Some(json!({"rr": 2.5})),
                lock: false,
                visible: true,
                mode: "weak_magnet".to_owned(),
                updated_at_ms: 0,
            })
            .unwrap();
        let drawings = store.list_drawings("HOSE:FPT").unwrap();
        assert_eq!(drawings.len(), 1);
        assert_eq!(drawings[0].name, "longPosition");

        // Indicators per pane
        store
            .save_pane_indicators(
                "pane-0",
                &[SavedIndicatorSpec {
                    id: "ind_1".to_owned(),
                    name: "ICHIMOKU".to_owned(),
                    pane_id: "candle_pane".to_owned(),
                    is_main: true,
                    calc_params: vec![9.0, 26.0, 52.0],
                    visible: true,
                    styles: None,
                }],
            )
            .unwrap();
        let inds = store.get_pane_indicators("pane-0").unwrap();
        assert_eq!(inds.len(), 1);
        assert_eq!(inds[0].name, "ICHIMOKU");

        // Alerts
        store
            .upsert_alert(&PriceAlertRecord {
                id: "al_1".to_owned(),
                ticker: "HOSE:FPT".to_owned(),
                condition: "cross_above".to_owned(),
                target_price: 135_000.0,
                message: "FPT breakout".to_owned(),
                status: "armed".to_owned(),
                created_at_ms: 1_700_000_000_000,
                triggered_at_ms: None,
            })
            .unwrap();
        assert_eq!(store.list_alerts().unwrap().len(), 1);
    }
}
