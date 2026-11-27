use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sumi_core::scanner::qmv_universe::{
    QMV_DT1_SUMI_SYMBOLS, QMV30_SYMBOLS, QMV60_SYMBOLS, YUMEKO_CORE10_SYMBOLS,
};

use crate::error::{DesktopError, DesktopResult};

const WORKSPACE_SCHEMA_VERSION: i32 = 7;
const QMV_WORKSTATION_MIGRATION_KEY: &str = "qmv_workstation_v1_migrated";

fn qmv_prefixed_ticker(symbol: &str) -> String {
    match symbol {
        "IDC" | "MBS" | "PVS" | "SHS" | "HNXINDEX" | "VN30F1!" => format!("HNX:{symbol}"),
        "BSR" | "DDV" | "STH" | "VGI" => format!("UPCOM:{symbol}"),
        _ => format!("HOSE:{symbol}"),
    }
}

fn default_alert_kind() -> String {
    "price".to_owned()
}

fn default_sound_enabled() -> bool {
    true
}

fn default_alert_frequency() -> String {
    "once".to_owned()
}

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
pub struct SavedScreenRecord {
    pub id: String,
    pub name: String,
    pub filter_json: Value,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SymbolFlagRecord {
    pub ticker: String,
    pub color: String,
    pub updated_at_ms: i64,
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
    /// `"price"` (default) | `"drawing"` (trendline/ray/horizontal) | `"indicator"` (RSI/MACD/EMA/Signal)
    #[serde(default = "default_alert_kind")]
    pub alert_kind: String,
    /// `"cross_above"` | `"cross_below"` | `"cross_any"` | indicator-specific condition
    pub condition: String,
    pub target_price: f64,
    #[serde(default)]
    pub interval: Option<String>,
    #[serde(default)]
    pub params_json: Option<Value>,
    #[serde(default = "default_sound_enabled")]
    pub sound_enabled: bool,
    pub message: String,
    /// `"armed"` | `"triggered"` | `"paused"`
    pub status: String,
    /// `"once"` (default) | `"once_per_bar_close"`
    #[serde(default = "default_alert_frequency")]
    pub frequency: String,
    #[serde(default)]
    pub cooldown_bars: i32,
    #[serde(default)]
    pub last_fired_bar_ts: Option<i64>,
    pub created_at_ms: i64,
    pub triggered_at_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertTriggerLogRecord {
    pub id: String,
    pub alert_id: String,
    pub ticker: String,
    pub alert_kind: String,
    pub condition: String,
    pub trigger_price: f64,
    pub bar_timestamp_ms: i64,
    pub triggered_at_ms: i64,
    pub message: String,
    pub delivered_channels: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndicatorTemplateRecord {
    pub id: String,
    pub name: String,
    pub description: String,
    pub indicators: Vec<SavedIndicatorSpec>,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedReplaySessionRecord {
    pub id: String,
    pub name: String,
    pub ticker: String,
    pub base_interval: String,
    pub tracked_intervals: Vec<String>,
    pub cursor_timestamp_ms: i64,
    pub speed_multiplier: f64,
    pub blind_mode: bool,
    pub broker_snapshot_json: Option<Value>,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalAnnotationRecord {
    pub trade_id: u64,
    pub ticker: String,
    pub setup_tag: String,
    pub mistake_tag: String,
    pub note: String,
    #[serde(default)]
    pub checklist_json: Option<Value>,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncTombstoneRecord {
    pub entity_kind: String,
    pub entity_key: String,
    pub deleted_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KvRecord {
    pub key: String,
    pub value: Value,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaneIndicatorsRecord {
    pub slot_id: String,
    pub indicators: Vec<SavedIndicatorSpec>,
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
        store.seed_default_indicator_templates()?;
        store.migrate_qmv_workstation_v1()?;
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
        store.seed_default_indicator_templates()?;
        store.migrate_qmv_workstation_v1()?;
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
                alert_kind      TEXT NOT NULL DEFAULT 'price',
                condition       TEXT NOT NULL,
                target_price    REAL NOT NULL,
                interval        TEXT,
                params_json     TEXT,
                sound_enabled   INTEGER NOT NULL DEFAULT 1,
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

            CREATE TABLE IF NOT EXISTS indicator_templates (
                id              TEXT PRIMARY KEY,
                name            TEXT NOT NULL,
                description     TEXT NOT NULL DEFAULT '',
                indicators_json TEXT NOT NULL,
                updated_at_ms   INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS replay_sessions (
                id                   TEXT PRIMARY KEY,
                name                 TEXT NOT NULL,
                ticker               TEXT NOT NULL,
                base_interval        TEXT NOT NULL,
                tracked_intervals    TEXT NOT NULL,
                cursor_timestamp_ms  INTEGER NOT NULL,
                speed_multiplier     REAL NOT NULL DEFAULT 1.0,
                blind_mode           INTEGER NOT NULL DEFAULT 1,
                broker_snapshot_json TEXT,
                updated_at_ms        INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS sync_tombstones (
                entity_kind   TEXT NOT NULL,
                entity_key    TEXT NOT NULL,
                deleted_at_ms INTEGER NOT NULL,
                PRIMARY KEY (entity_kind, entity_key)
            );

            CREATE TABLE IF NOT EXISTS alert_trigger_log (
                id                 TEXT PRIMARY KEY,
                alert_id           TEXT NOT NULL,
                ticker             TEXT NOT NULL,
                alert_kind         TEXT NOT NULL,
                condition          TEXT NOT NULL,
                trigger_price      REAL NOT NULL,
                bar_timestamp_ms   INTEGER NOT NULL,
                triggered_at_ms    INTEGER NOT NULL,
                message            TEXT NOT NULL,
                delivered_channels TEXT NOT NULL DEFAULT 'in_app'
            );
            CREATE INDEX IF NOT EXISTS idx_alert_log_ts ON alert_trigger_log(triggered_at_ms DESC);

            CREATE TABLE IF NOT EXISTS saved_screens (
                id            TEXT PRIMARY KEY,
                name          TEXT NOT NULL,
                filter_json   TEXT NOT NULL,
                updated_at_ms INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS symbol_flags (
                ticker        TEXT PRIMARY KEY,
                color         TEXT NOT NULL,
                updated_at_ms INTEGER NOT NULL
            );
            "#,
        )?;

        // Idempotent additive columns for existing v1–v4 tables
        for ddl in [
            "ALTER TABLE price_alerts ADD COLUMN alert_kind TEXT NOT NULL DEFAULT 'price'",
            "ALTER TABLE price_alerts ADD COLUMN interval TEXT",
            "ALTER TABLE price_alerts ADD COLUMN params_json TEXT",
            "ALTER TABLE price_alerts ADD COLUMN sound_enabled INTEGER NOT NULL DEFAULT 1",
            "ALTER TABLE price_alerts ADD COLUMN frequency TEXT NOT NULL DEFAULT 'once'",
            "ALTER TABLE price_alerts ADD COLUMN cooldown_bars INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE price_alerts ADD COLUMN last_fired_bar_ts INTEGER",
            "ALTER TABLE journal_annotations ADD COLUMN checklist_json TEXT",
            "ALTER TABLE watchlists ADD COLUMN updated_at_ms INTEGER NOT NULL DEFAULT 0",
        ] {
            let _ = conn.execute(ddl, []);
        }

        conn.pragma_update(None, "user_version", WORKSPACE_SCHEMA_VERSION)?;
        Ok(())
    }

    fn default_qmv_watchlists() -> Vec<(&'static str, &'static str, i32, Vec<String>)> {
        let qmv30: Vec<String> = QMV30_SYMBOLS.iter().map(|s| qmv_prefixed_ticker(s)).collect();
        let qmv60: Vec<String> = QMV60_SYMBOLS.iter().map(|s| qmv_prefixed_ticker(s)).collect();
        let core10: Vec<String> = YUMEKO_CORE10_SYMBOLS
            .iter()
            .map(|s| qmv_prefixed_ticker(s))
            .collect();
        let dt1: Vec<String> = QMV_DT1_SUMI_SYMBOLS
            .iter()
            .map(|s| qmv_prefixed_ticker(s))
            .collect();
        let indices = vec![
            "HOSE:VNINDEX".to_owned(),
            "HOSE:VN30".to_owned(),
            "HNX:HNXINDEX".to_owned(),
            "HOSE:VNXALL".to_owned(),
            "HNX:VN30F1!".to_owned(),
        ];

        vec![
            ("wl_qmv30", "QMV30 — 30 Cổ Phiếu Trụ Cột", 0, qmv30),
            ("wl_qmv60", "QMV60 — Rổ Theo Dõi Trọng Điểm", 1, qmv60),
            (
                "wl_yumeko_core",
                "Danh Mục Tham Chiếu Yumeko Core 10",
                2,
                core10,
            ),
            (
                "wl_qmv_dt1",
                "Dòng Tiền Loại 1 (DT1 — Tạo Lập)",
                3,
                dt1,
            ),
            (
                "wl_vn_indices",
                "Chỉ Số & Rào Chắn Thị Trường VN",
                4,
                indices,
            ),
        ]
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

        let defaults = Self::default_qmv_watchlists();
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
        let now_ms = chrono::Utc::now().timestamp_millis();
        conn.execute(
            "DELETE FROM drawings WHERE ticker = ?1 AND id = ?2",
            params![&normalized, id],
        )?;
        let _ = conn.execute(
            r#"
            INSERT INTO sync_tombstones (entity_kind, entity_key, deleted_at_ms)
            VALUES ('drawing', ?1, ?2)
            ON CONFLICT(entity_kind, entity_key) DO UPDATE SET deleted_at_ms = MAX(deleted_at_ms, excluded.deleted_at_ms)
            "#,
            params![format!("{normalized}::{id}"), now_ms],
        );
        Ok(())
    }

    pub fn clear_drawings(&self, ticker: &str) -> DesktopResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        let normalized = ticker.trim().to_ascii_uppercase();
        let now_ms = chrono::Utc::now().timestamp_millis();
        let ids: Vec<String> = {
            let mut stmt = conn.prepare("SELECT id FROM drawings WHERE ticker = ?1")?;
            stmt.query_map(params![&normalized], |r| r.get(0))?
                .filter_map(Result::ok)
                .collect()
        };
        conn.execute(
            "DELETE FROM drawings WHERE ticker = ?1",
            params![&normalized],
        )?;
        for id in ids {
            let _ = conn.execute(
                r#"
                INSERT INTO sync_tombstones (entity_kind, entity_key, deleted_at_ms)
                VALUES ('drawing', ?1, ?2)
                ON CONFLICT(entity_kind, entity_key) DO UPDATE SET deleted_at_ms = MAX(deleted_at_ms, excluded.deleted_at_ms)
                "#,
                params![format!("{normalized}::{id}"), now_ms],
            );
        }
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
        let now_ms = chrono::Utc::now().timestamp_millis();
        conn.execute("DELETE FROM chart_layouts WHERE id = ?1", params![id])?;
        let _ = conn.execute(
            r#"
            INSERT INTO sync_tombstones (entity_kind, entity_key, deleted_at_ms)
            VALUES ('layout', ?1, ?2)
            ON CONFLICT(entity_kind, entity_key) DO UPDATE SET deleted_at_ms = MAX(deleted_at_ms, excluded.deleted_at_ms)
            "#,
            params![id, now_ms],
        );
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
        let now_ms = chrono::Utc::now().timestamp_millis();
        conn.execute("DELETE FROM watchlists WHERE id = ?1", params![id])?;
        let _ = conn.execute(
            r#"
            INSERT INTO sync_tombstones (entity_kind, entity_key, deleted_at_ms)
            VALUES ('watchlist', ?1, ?2)
            ON CONFLICT(entity_kind, entity_key) DO UPDATE SET deleted_at_ms = MAX(deleted_at_ms, excluded.deleted_at_ms)
            "#,
            params![id, now_ms],
        );
        Ok(())
    }

    pub fn add_watchlist_symbol(&self, watchlist_id: &str, ticker: &str) -> DesktopResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        let normalized = if ticker.trim().starts_with("###") {
            ticker.trim().to_owned()
        } else {
            ticker.trim().to_ascii_uppercase()
        };
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

    pub fn rename_watchlist(&self, id: &str, name: &str) -> DesktopResult<()> {
        let clean = name.trim();
        if clean.is_empty() {
            return Err(DesktopError::Validation(
                "Tên danh mục theo dõi không được để trống".to_owned(),
            ));
        }
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        let now_ms = chrono::Utc::now().timestamp_millis();
        conn.execute(
            "UPDATE watchlists SET name = ?2, updated_at_ms = ?3 WHERE id = ?1",
            params![id, clean, now_ms],
        )?;
        Ok(())
    }

    pub fn reorder_watchlist_symbols(
        &self,
        watchlist_id: &str,
        tickers: &[String],
    ) -> DesktopResult<()> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        let now_ms = chrono::Utc::now().timestamp_millis();
        let tx = conn.transaction()?;
        tx.execute(
            "DELETE FROM watchlist_items WHERE watchlist_id = ?1",
            params![watchlist_id],
        )?;
        for (idx, raw) in tickers.iter().enumerate() {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                continue;
            }
            let normalized = if trimmed.starts_with("###") {
                trimmed.to_owned()
            } else {
                trimmed.to_ascii_uppercase()
            };
            tx.execute(
                "INSERT OR IGNORE INTO watchlist_items (watchlist_id, ticker, sort_order) VALUES (?1, ?2, ?3)",
                params![watchlist_id, normalized, idx as i32],
            )?;
        }
        tx.execute(
            "UPDATE watchlists SET updated_at_ms = ?2 WHERE id = ?1",
            params![watchlist_id, now_ms],
        )?;
        tx.commit()?;
        Ok(())
    }

    // ─── Symbol Color Flags (Wave 5, v7) ─────────────────────────────────────

    pub fn list_symbol_flags(&self) -> DesktopResult<Vec<SymbolFlagRecord>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        let mut stmt = conn.prepare(
            "SELECT ticker, color, updated_at_ms FROM symbol_flags ORDER BY ticker ASC",
        )?;
        let rows = stmt
            .query_map([], |r| {
                Ok(SymbolFlagRecord {
                    ticker: r.get(0)?,
                    color: r.get(1)?,
                    updated_at_ms: r.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn set_symbol_flag(&self, ticker: &str, color: &str) -> DesktopResult<SymbolFlagRecord> {
        let norm_ticker = ticker.trim().to_ascii_uppercase();
        let norm_color = color.trim().to_ascii_lowercase();
        if norm_ticker.is_empty() || norm_color.is_empty() {
            return Err(DesktopError::Validation(
                "Mã chứng khoán và màu cờ đánh dấu không được để trống".to_owned(),
            ));
        }
        let rec = SymbolFlagRecord {
            ticker: norm_ticker,
            color: norm_color,
            updated_at_ms: chrono::Utc::now().timestamp_millis(),
        };
        self.set_symbol_flag_with_ts(&rec)?;
        Ok(rec)
    }

    pub fn set_symbol_flag_with_ts(&self, rec: &SymbolFlagRecord) -> DesktopResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        conn.execute(
            r#"
            INSERT INTO symbol_flags (ticker, color, updated_at_ms)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(ticker) DO UPDATE SET
                color = excluded.color,
                updated_at_ms = excluded.updated_at_ms
            WHERE excluded.updated_at_ms >= symbol_flags.updated_at_ms
            "#,
            params![&rec.ticker, &rec.color, rec.updated_at_ms],
        )?;
        Ok(())
    }

    pub fn clear_symbol_flag(&self, ticker: &str) -> DesktopResult<()> {
        let norm_ticker = ticker.trim().to_ascii_uppercase();
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        let now_ms = chrono::Utc::now().timestamp_millis();
        conn.execute(
            "DELETE FROM symbol_flags WHERE ticker = ?1",
            params![&norm_ticker],
        )?;
        conn.execute(
            r#"
            INSERT INTO sync_tombstones (entity_kind, entity_key, deleted_at_ms)
            VALUES ('symbol_flag', ?1, ?2)
            ON CONFLICT(entity_kind, entity_key) DO UPDATE SET
                deleted_at_ms = MAX(deleted_at_ms, excluded.deleted_at_ms)
            "#,
            params![&norm_ticker, now_ms],
        )?;
        Ok(())
    }

    // ─── Price / Drawing / Indicator Alerts ──────────────────────────────────

    pub fn list_alerts(&self) -> DesktopResult<Vec<PriceAlertRecord>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        let mut stmt = conn.prepare(
            r#"
            SELECT id, ticker, alert_kind, condition, target_price, interval, params_json,
                   sound_enabled, message, status, frequency, cooldown_bars, last_fired_bar_ts,
                   created_at_ms, triggered_at_ms
            FROM price_alerts
            ORDER BY created_at_ms DESC
            "#,
        )?;
        let rows = stmt
            .query_map([], |r| {
                let params_raw: Option<String> = r.get(6)?;
                let params_json = params_raw.and_then(|s| serde_json::from_str::<Value>(&s).ok());
                Ok(PriceAlertRecord {
                    id: r.get(0)?,
                    ticker: r.get(1)?,
                    alert_kind: r.get::<_, Option<String>>(2)?.unwrap_or_else(default_alert_kind),
                    condition: r.get(3)?,
                    target_price: r.get(4)?,
                    interval: r.get(5)?,
                    params_json,
                    sound_enabled: r.get::<_, Option<i64>>(7)?.unwrap_or(1) != 0,
                    message: r.get(8)?,
                    status: r.get(9)?,
                    frequency: r
                        .get::<_, Option<String>>(10)?
                        .unwrap_or_else(default_alert_frequency),
                    cooldown_bars: r.get::<_, Option<i32>>(11)?.unwrap_or(0),
                    last_fired_bar_ts: r.get(12)?,
                    created_at_ms: r.get(13)?,
                    triggered_at_ms: r.get(14)?,
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
        let params_str = alert
            .params_json
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        let kind = if alert.alert_kind.trim().is_empty() {
            "price"
        } else {
            alert.alert_kind.trim()
        };
        let freq = if alert.frequency.trim().is_empty() {
            "once"
        } else {
            alert.frequency.trim()
        };
        conn.execute(
            r#"
            INSERT INTO price_alerts (
                id, ticker, alert_kind, condition, target_price, interval, params_json,
                sound_enabled, message, status, frequency, cooldown_bars, last_fired_bar_ts,
                created_at_ms, triggered_at_ms
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
            ON CONFLICT(id) DO UPDATE SET
                alert_kind = excluded.alert_kind,
                condition = excluded.condition,
                target_price = excluded.target_price,
                interval = excluded.interval,
                params_json = excluded.params_json,
                sound_enabled = excluded.sound_enabled,
                message = excluded.message,
                status = excluded.status,
                frequency = excluded.frequency,
                cooldown_bars = excluded.cooldown_bars,
                last_fired_bar_ts = excluded.last_fired_bar_ts,
                triggered_at_ms = excluded.triggered_at_ms
            "#,
            params![
                alert.id,
                alert.ticker.trim().to_ascii_uppercase(),
                kind,
                alert.condition,
                alert.target_price,
                alert.interval,
                params_str,
                i64::from(alert.sound_enabled),
                alert.message,
                alert.status,
                freq,
                alert.cooldown_bars.clamp(0, 500),
                alert.last_fired_bar_ts,
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

    pub fn record_alert_fire(
        &self,
        alert: &PriceAlertRecord,
        next_status: &str,
        bar_timestamp_ms: i64,
        trigger_price: f64,
        delivered_channels: &str,
    ) -> DesktopResult<AlertTriggerLogRecord> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        let now_ms = chrono::Utc::now().timestamp_millis();
        conn.execute(
            "UPDATE price_alerts SET status = ?2, triggered_at_ms = ?3, last_fired_bar_ts = ?4 WHERE id = ?1",
            params![&alert.id, next_status, now_ms, bar_timestamp_ms],
        )?;
        let log = AlertTriggerLogRecord {
            id: format!("alog_{}_{}", now_ms, uuid::Uuid::new_v4().simple()),
            alert_id: alert.id.clone(),
            ticker: alert.ticker.clone(),
            alert_kind: alert.alert_kind.clone(),
            condition: alert.condition.clone(),
            trigger_price,
            bar_timestamp_ms,
            triggered_at_ms: now_ms,
            message: alert.message.clone(),
            delivered_channels: delivered_channels.to_owned(),
        };
        conn.execute(
            r#"
            INSERT INTO alert_trigger_log (
                id, alert_id, ticker, alert_kind, condition, trigger_price,
                bar_timestamp_ms, triggered_at_ms, message, delivered_channels
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            "#,
            params![
                &log.id,
                &log.alert_id,
                &log.ticker,
                &log.alert_kind,
                &log.condition,
                log.trigger_price,
                log.bar_timestamp_ms,
                log.triggered_at_ms,
                &log.message,
                &log.delivered_channels,
            ],
        )?;
        // Keep bounded history (500 latest log rows)
        let _ = conn.execute(
            "DELETE FROM alert_trigger_log WHERE id NOT IN (SELECT id FROM alert_trigger_log ORDER BY triggered_at_ms DESC LIMIT 500)",
            [],
        );
        Ok(log)
    }

    pub fn list_alert_trigger_logs(&self, limit: usize) -> DesktopResult<Vec<AlertTriggerLogRecord>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        let mut stmt = conn.prepare(
            r#"
            SELECT id, alert_id, ticker, alert_kind, condition, trigger_price,
                   bar_timestamp_ms, triggered_at_ms, message, delivered_channels
            FROM alert_trigger_log
            ORDER BY triggered_at_ms DESC
            LIMIT ?1
            "#,
        )?;
        let rows = stmt
            .query_map(params![limit.clamp(1, 500) as i64], |r| {
                Ok(AlertTriggerLogRecord {
                    id: r.get(0)?,
                    alert_id: r.get(1)?,
                    ticker: r.get(2)?,
                    alert_kind: r.get(3)?,
                    condition: r.get(4)?,
                    trigger_price: r.get(5)?,
                    bar_timestamp_ms: r.get(6)?,
                    triggered_at_ms: r.get(7)?,
                    message: r.get(8)?,
                    delivered_channels: r.get(9)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn clear_alert_trigger_logs(&self) -> DesktopResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        conn.execute("DELETE FROM alert_trigger_log", [])?;
        Ok(())
    }

    pub fn delete_alert(&self, id: &str) -> DesktopResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        let now_ms = chrono::Utc::now().timestamp_millis();
        conn.execute("DELETE FROM price_alerts WHERE id = ?1", params![id])?;
        let _ = conn.execute(
            r#"
            INSERT INTO sync_tombstones (entity_kind, entity_key, deleted_at_ms)
            VALUES ('alert', ?1, ?2)
            ON CONFLICT(entity_kind, entity_key) DO UPDATE SET deleted_at_ms = MAX(deleted_at_ms, excluded.deleted_at_ms)
            "#,
            params![id, now_ms],
        );
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
            SELECT trade_id, ticker, setup_tag, mistake_tag, note, checklist_json, updated_at_ms
            FROM journal_annotations
            WHERE ticker = ?1
            ORDER BY trade_id DESC
            "#,
        )?;
        let rows = stmt
            .query_map(params![normalized], |r| {
                let ck_raw: Option<String> = r.get(5)?;
                Ok(JournalAnnotationRecord {
                    trade_id: r.get::<_, i64>(0)?.max(0) as u64,
                    ticker: r.get(1)?,
                    setup_tag: r.get(2)?,
                    mistake_tag: r.get(3)?,
                    note: r.get(4)?,
                    checklist_json: ck_raw.and_then(|s| serde_json::from_str::<Value>(&s).ok()),
                    updated_at_ms: r.get(6)?,
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
        self.upsert_journal_annotation_full(ticker, trade_id, setup_tag, mistake_tag, note, None)
    }

    pub fn upsert_journal_annotation_full(
        &self,
        ticker: &str,
        trade_id: u64,
        setup_tag: &str,
        mistake_tag: &str,
        note: &str,
        checklist_json: Option<Value>,
    ) -> DesktopResult<JournalAnnotationRecord> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        let normalized = ticker.trim().to_ascii_uppercase();
        let now_ms = chrono::Utc::now().timestamp_millis();
        let ck_str = checklist_json
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        conn.execute(
            r#"
            INSERT INTO journal_annotations (trade_id, ticker, setup_tag, mistake_tag, note, checklist_json, updated_at_ms)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT(ticker, trade_id) DO UPDATE SET
                setup_tag = excluded.setup_tag,
                mistake_tag = excluded.mistake_tag,
                note = excluded.note,
                checklist_json = COALESCE(excluded.checklist_json, journal_annotations.checklist_json),
                updated_at_ms = excluded.updated_at_ms
            "#,
            params![trade_id as i64, &normalized, setup_tag, mistake_tag, note, ck_str, now_ms],
        )?;
        Ok(JournalAnnotationRecord {
            trade_id,
            ticker: normalized,
            setup_tag: setup_tag.to_owned(),
            mistake_tag: mistake_tag.to_owned(),
            note: note.to_owned(),
            checklist_json,
            updated_at_ms: now_ms,
        })
    }

    // ─── Indicator Templates (QMV Workstation v2.2) ──────────────────────────

    fn default_qmv_indicator_templates(now_ms: i64) -> Vec<IndicatorTemplateRecord> {
        vec![
            IndicatorTemplateRecord {
                id: "tpl_qmv_core_kt01".to_owned(),
                name: "1. QMV Cốt Lõi — Giá Lượng & Xu Hướng (KT01)".to_owned(),
                description:
                    "MA (20, 50, 100, 200) + Bollinger (20, 2) + Khối lượng (MA5, MA20) + RSI (14)"
                        .to_owned(),
                indicators: vec![
                    SavedIndicatorSpec {
                        id: "tpl_ma_kt01".to_owned(),
                        name: "MA".to_owned(),
                        pane_id: "candle_pane".to_owned(),
                        is_main: true,
                        calc_params: vec![20.0, 50.0, 100.0, 200.0],
                        visible: true,
                        styles: None,
                    },
                    SavedIndicatorSpec {
                        id: "tpl_boll_kt01".to_owned(),
                        name: "BOLL".to_owned(),
                        pane_id: "candle_pane".to_owned(),
                        is_main: true,
                        calc_params: vec![20.0, 2.0],
                        visible: true,
                        styles: None,
                    },
                    SavedIndicatorSpec {
                        id: "tpl_vol_kt01".to_owned(),
                        name: "VOL".to_owned(),
                        pane_id: "pane_vol".to_owned(),
                        is_main: false,
                        calc_params: vec![5.0, 20.0],
                        visible: true,
                        styles: None,
                    },
                    SavedIndicatorSpec {
                        id: "tpl_rsi_kt01".to_owned(),
                        name: "RSI".to_owned(),
                        pane_id: "pane_rsi".to_owned(),
                        is_main: false,
                        calc_params: vec![14.0],
                        visible: true,
                        styles: None,
                    },
                ],
                updated_at_ms: now_ms + 4,
            },
            IndicatorTemplateRecord {
                id: "tpl_qmv_ichimoku_v7".to_owned(),
                name: "2. QMV Ichimoku V7 & Phân Kỳ".to_owned(),
                description:
                    "Mây Ichimoku chuẩn QMV V7 (6, 18, 36, 18) + MA (50, 200) + VOL (5, 20) + CCI (14) + MACD (12, 26, 9)"
                        .to_owned(),
                indicators: vec![
                    SavedIndicatorSpec {
                        id: "tpl_ichi_v7".to_owned(),
                        name: "ICHIMOKU".to_owned(),
                        pane_id: "candle_pane".to_owned(),
                        is_main: true,
                        calc_params: vec![6.0, 18.0, 36.0, 18.0],
                        visible: true,
                        styles: None,
                    },
                    SavedIndicatorSpec {
                        id: "tpl_ma_v7".to_owned(),
                        name: "MA".to_owned(),
                        pane_id: "candle_pane".to_owned(),
                        is_main: true,
                        calc_params: vec![50.0, 200.0],
                        visible: true,
                        styles: None,
                    },
                    SavedIndicatorSpec {
                        id: "tpl_vol_v7".to_owned(),
                        name: "VOL".to_owned(),
                        pane_id: "pane_vol".to_owned(),
                        is_main: false,
                        calc_params: vec![5.0, 20.0],
                        visible: true,
                        styles: None,
                    },
                    SavedIndicatorSpec {
                        id: "tpl_cci_v7".to_owned(),
                        name: "CCI".to_owned(),
                        pane_id: "pane_cci".to_owned(),
                        is_main: false,
                        calc_params: vec![14.0],
                        visible: true,
                        styles: None,
                    },
                    SavedIndicatorSpec {
                        id: "tpl_macd_v7".to_owned(),
                        name: "MACD".to_owned(),
                        pane_id: "pane_macd".to_owned(),
                        is_main: false,
                        calc_params: vec![12.0, 26.0, 9.0],
                        visible: true,
                        styles: None,
                    },
                ],
                updated_at_ms: now_ms + 3,
            },
            IndicatorTemplateRecord {
                id: "tpl_qmv_flow_cycle".to_owned(),
                name: "3. QMV Chu Kỳ Dòng Tiền T+ / Sóng (OHLCV Proxy)".to_owned(),
                description:
                    "MA (20, 50) + VOL (5, 20) + Chu kỳ Dòng tiền QMV Proxy (3, 5, 10, 20) + Sức mạnh RS vs VNINDEX"
                        .to_owned(),
                indicators: vec![
                    SavedIndicatorSpec {
                        id: "tpl_ma_flow".to_owned(),
                        name: "MA".to_owned(),
                        pane_id: "candle_pane".to_owned(),
                        is_main: true,
                        calc_params: vec![20.0, 50.0],
                        visible: true,
                        styles: None,
                    },
                    SavedIndicatorSpec {
                        id: "tpl_vol_flow".to_owned(),
                        name: "VOL".to_owned(),
                        pane_id: "pane_vol".to_owned(),
                        is_main: false,
                        calc_params: vec![5.0, 20.0],
                        visible: true,
                        styles: None,
                    },
                    SavedIndicatorSpec {
                        id: "tpl_qmv_flow".to_owned(),
                        name: "QMV_FLOW".to_owned(),
                        pane_id: "pane_qmv_flow".to_owned(),
                        is_main: false,
                        calc_params: vec![3.0, 5.0, 10.0, 20.0],
                        visible: true,
                        styles: None,
                    },
                    SavedIndicatorSpec {
                        id: "tpl_rs_vnindex".to_owned(),
                        name: "RS_VNINDEX".to_owned(),
                        pane_id: "pane_rs".to_owned(),
                        is_main: false,
                        calc_params: vec![20.0],
                        visible: true,
                        styles: None,
                    },
                ],
                updated_at_ms: now_ms + 2,
            },
            IndicatorTemplateRecord {
                id: "tpl_qmv_safe_bottom".to_owned(),
                name: "4. QMV Bắt Đáy An Toàn & Tái Cân Bằng (DT1/DT2)".to_owned(),
                description:
                    "Bollinger (20, 2) + MA (20, 50, 200) + VOL (5, 20) + RSI (14) + MACD (12, 26, 9) tìm phân kỳ đáy"
                        .to_owned(),
                indicators: vec![
                    SavedIndicatorSpec {
                        id: "tpl_boll_bot".to_owned(),
                        name: "BOLL".to_owned(),
                        pane_id: "candle_pane".to_owned(),
                        is_main: true,
                        calc_params: vec![20.0, 2.0],
                        visible: true,
                        styles: None,
                    },
                    SavedIndicatorSpec {
                        id: "tpl_ma_bot".to_owned(),
                        name: "MA".to_owned(),
                        pane_id: "candle_pane".to_owned(),
                        is_main: true,
                        calc_params: vec![20.0, 50.0, 200.0],
                        visible: true,
                        styles: None,
                    },
                    SavedIndicatorSpec {
                        id: "tpl_vol_bot".to_owned(),
                        name: "VOL".to_owned(),
                        pane_id: "pane_vol".to_owned(),
                        is_main: false,
                        calc_params: vec![5.0, 20.0],
                        visible: true,
                        styles: None,
                    },
                    SavedIndicatorSpec {
                        id: "tpl_rsi_bot".to_owned(),
                        name: "RSI".to_owned(),
                        pane_id: "pane_rsi".to_owned(),
                        is_main: false,
                        calc_params: vec![14.0],
                        visible: true,
                        styles: None,
                    },
                    SavedIndicatorSpec {
                        id: "tpl_macd_bot".to_owned(),
                        name: "MACD".to_owned(),
                        pane_id: "pane_macd".to_owned(),
                        is_main: false,
                        calc_params: vec![12.0, 26.0, 9.0],
                        visible: true,
                        styles: None,
                    },
                ],
                updated_at_ms: now_ms + 1,
            },
        ]
    }

    fn seed_default_indicator_templates(&self) -> DesktopResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        let count: i64 =
            conn.query_row("SELECT COUNT(*) FROM indicator_templates", [], |r| r.get(0))?;
        if count > 0 {
            return Ok(());
        }
        drop(conn);

        let now_ms = chrono::Utc::now().timestamp_millis();
        for tpl in Self::default_qmv_indicator_templates(now_ms) {
            let conn = self.conn.lock().map_err(|_| {
                DesktopError::Internal("workspace sqlite mutex poisoned".to_owned())
            })?;
            let json_str = serde_json::to_string(&tpl.indicators)?;
            conn.execute(
                r#"
                INSERT INTO indicator_templates (id, name, description, indicators_json, updated_at_ms)
                VALUES (?1, ?2, ?3, ?4, ?5)
                ON CONFLICT(id) DO UPDATE SET
                    name = excluded.name,
                    description = excluded.description,
                    indicators_json = excluded.indicators_json,
                    updated_at_ms = excluded.updated_at_ms
                "#,
                params![tpl.id, tpl.name, tpl.description, json_str, tpl.updated_at_ms],
            )?;
            drop(conn);
        }
        Ok(())
    }

    /// One-time non-destructive upgrade for existing desktop workspaces so legacy non-QMV
    /// default watchlists (`wl_crypto`, `wl_global`, `wl_vn30`) and templates (`tpl_vn30f_scalp`, etc.)
    /// are upgraded to the canonical QMV workstation defaults while preserving user custom records.
    fn migrate_qmv_workstation_v1(&self) -> DesktopResult<()> {
        if self.get_kv(QMV_WORKSTATION_MIGRATION_KEY)?.is_some() {
            return Ok(());
        }

        let now_ms = chrono::Utc::now().timestamp_millis();
        {
            let mut conn = self.conn.lock().map_err(|_| {
                DesktopError::Internal("workspace sqlite mutex poisoned".to_owned())
            })?;
            let tx = conn.transaction()?;
            tx.execute(
                "DELETE FROM watchlists WHERE id IN ('wl_crypto', 'wl_global', 'wl_vn30')",
                [],
            )?;
            for (id, name, sort_order, tickers) in Self::default_qmv_watchlists() {
                tx.execute(
                    r#"
                    INSERT INTO watchlists (id, name, sort_order)
                    VALUES (?1, ?2, ?3)
                    ON CONFLICT(id) DO UPDATE SET
                        name = excluded.name,
                        sort_order = excluded.sort_order
                    "#,
                    params![id, name, sort_order],
                )?;
                tx.execute(
                    "DELETE FROM watchlist_items WHERE watchlist_id = ?1",
                    params![id],
                )?;
                for (idx, ticker) in tickers.iter().enumerate() {
                    tx.execute(
                        "INSERT INTO watchlist_items (watchlist_id, ticker, sort_order) VALUES (?1, ?2, ?3)",
                        params![id, ticker, idx as i32],
                    )?;
                }
            }

            tx.execute(
                "DELETE FROM indicator_templates WHERE id IN ('tpl_vn_price_action', 'tpl_ichimoku_pro', 'tpl_vn30f_scalp')",
                [],
            )?;
            for tpl in Self::default_qmv_indicator_templates(now_ms) {
                let json_str = serde_json::to_string(&tpl.indicators)?;
                tx.execute(
                    r#"
                    INSERT INTO indicator_templates (id, name, description, indicators_json, updated_at_ms)
                    VALUES (?1, ?2, ?3, ?4, ?5)
                    ON CONFLICT(id) DO UPDATE SET
                        name = excluded.name,
                        description = excluded.description,
                        indicators_json = excluded.indicators_json,
                        updated_at_ms = excluded.updated_at_ms
                    "#,
                    params![tpl.id, tpl.name, tpl.description, json_str, tpl.updated_at_ms],
                )?;
            }
            tx.commit()?;
        }

        self.set_kv(QMV_WORKSTATION_MIGRATION_KEY, &serde_json::json!(true))?;
        Ok(())
    }

    pub fn list_indicator_templates(&self) -> DesktopResult<Vec<IndicatorTemplateRecord>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        let mut stmt = conn.prepare(
            "SELECT id, name, description, indicators_json, updated_at_ms FROM indicator_templates ORDER BY updated_at_ms DESC, name ASC",
        )?;
        let rows = stmt
            .query_map([], |r| {
                let raw: String = r.get(3)?;
                let indicators: Vec<SavedIndicatorSpec> =
                    serde_json::from_str(&raw).unwrap_or_default();
                Ok(IndicatorTemplateRecord {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    description: r.get(2)?,
                    indicators,
                    updated_at_ms: r.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn save_indicator_template(
        &self,
        tpl: &IndicatorTemplateRecord,
    ) -> DesktopResult<IndicatorTemplateRecord> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        let now_ms = chrono::Utc::now().timestamp_millis();
        let json_str = serde_json::to_string(&tpl.indicators)?;
        conn.execute(
            r#"
            INSERT INTO indicator_templates (id, name, description, indicators_json, updated_at_ms)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                description = excluded.description,
                indicators_json = excluded.indicators_json,
                updated_at_ms = excluded.updated_at_ms
            "#,
            params![tpl.id, tpl.name, tpl.description, json_str, now_ms],
        )?;
        let mut saved = tpl.clone();
        saved.updated_at_ms = now_ms;
        Ok(saved)
    }

    pub fn delete_indicator_template(&self, id: &str) -> DesktopResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        let now_ms = chrono::Utc::now().timestamp_millis();
        conn.execute("DELETE FROM indicator_templates WHERE id = ?1", params![id])?;
        let _ = conn.execute(
            r#"
            INSERT INTO sync_tombstones (entity_kind, entity_key, deleted_at_ms)
            VALUES ('indicator_template', ?1, ?2)
            ON CONFLICT(entity_kind, entity_key) DO UPDATE SET deleted_at_ms = MAX(deleted_at_ms, excluded.deleted_at_ms)
            "#,
            params![id, now_ms],
        );
        Ok(())
    }

    // ─── Saved Bar Replay Sessions (v2.2) ────────────────────────────────────

    pub fn list_replay_sessions(&self) -> DesktopResult<Vec<SavedReplaySessionRecord>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        let mut stmt = conn.prepare(
            r#"
            SELECT id, name, ticker, base_interval, tracked_intervals, cursor_timestamp_ms,
                   speed_multiplier, blind_mode, broker_snapshot_json, updated_at_ms
            FROM replay_sessions
            ORDER BY updated_at_ms DESC
            "#,
        )?;
        let rows = stmt
            .query_map([], |r| {
                let ivs_raw: String = r.get(4)?;
                let tracked_intervals: Vec<String> =
                    serde_json::from_str(&ivs_raw).unwrap_or_default();
                let broker_raw: Option<String> = r.get(8)?;
                let broker_snapshot_json =
                    broker_raw.and_then(|s| serde_json::from_str::<Value>(&s).ok());
                Ok(SavedReplaySessionRecord {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    ticker: r.get(2)?,
                    base_interval: r.get(3)?,
                    tracked_intervals,
                    cursor_timestamp_ms: r.get(5)?,
                    speed_multiplier: r.get(6)?,
                    blind_mode: r.get::<_, i64>(7)? != 0,
                    broker_snapshot_json,
                    updated_at_ms: r.get(9)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn get_replay_session(
        &self,
        id: &str,
    ) -> DesktopResult<Option<SavedReplaySessionRecord>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        let row = conn
            .query_row(
                r#"
                SELECT id, name, ticker, base_interval, tracked_intervals, cursor_timestamp_ms,
                       speed_multiplier, blind_mode, broker_snapshot_json, updated_at_ms
                FROM replay_sessions
                WHERE id = ?1
                "#,
                params![id],
                |r| {
                    let ivs_raw: String = r.get(4)?;
                    let tracked_intervals: Vec<String> =
                        serde_json::from_str(&ivs_raw).unwrap_or_default();
                    let broker_raw: Option<String> = r.get(8)?;
                    let broker_snapshot_json =
                        broker_raw.and_then(|s| serde_json::from_str::<Value>(&s).ok());
                    Ok(SavedReplaySessionRecord {
                        id: r.get(0)?,
                        name: r.get(1)?,
                        ticker: r.get(2)?,
                        base_interval: r.get(3)?,
                        tracked_intervals,
                        cursor_timestamp_ms: r.get(5)?,
                        speed_multiplier: r.get(6)?,
                        blind_mode: r.get::<_, i64>(7)? != 0,
                        broker_snapshot_json,
                        updated_at_ms: r.get(9)?,
                    })
                },
            )
            .optional()?;
        Ok(row)
    }

    pub fn save_replay_session(
        &self,
        session: &SavedReplaySessionRecord,
    ) -> DesktopResult<SavedReplaySessionRecord> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        let now_ms = chrono::Utc::now().timestamp_millis();
        let ivs_json = serde_json::to_string(&session.tracked_intervals)?;
        let broker_str = session
            .broker_snapshot_json
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        conn.execute(
            r#"
            INSERT INTO replay_sessions (
                id, name, ticker, base_interval, tracked_intervals, cursor_timestamp_ms,
                speed_multiplier, blind_mode, broker_snapshot_json, updated_at_ms
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                ticker = excluded.ticker,
                base_interval = excluded.base_interval,
                tracked_intervals = excluded.tracked_intervals,
                cursor_timestamp_ms = excluded.cursor_timestamp_ms,
                speed_multiplier = excluded.speed_multiplier,
                blind_mode = excluded.blind_mode,
                broker_snapshot_json = excluded.broker_snapshot_json,
                updated_at_ms = excluded.updated_at_ms
            "#,
            params![
                session.id,
                session.name,
                session.ticker.trim().to_ascii_uppercase(),
                session.base_interval,
                ivs_json,
                session.cursor_timestamp_ms,
                session.speed_multiplier,
                i64::from(session.blind_mode),
                broker_str,
                now_ms,
            ],
        )?;
        let mut saved = session.clone();
        saved.updated_at_ms = now_ms;
        Ok(saved)
    }

    pub fn delete_replay_session(&self, id: &str) -> DesktopResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        let now_ms = chrono::Utc::now().timestamp_millis();
        conn.execute("DELETE FROM replay_sessions WHERE id = ?1", params![id])?;
        let _ = conn.execute(
            r#"
            INSERT INTO sync_tombstones (entity_kind, entity_key, deleted_at_ms)
            VALUES ('replay_session', ?1, ?2)
            ON CONFLICT(entity_kind, entity_key) DO UPDATE SET deleted_at_ms = MAX(deleted_at_ms, excluded.deleted_at_ms)
            "#,
            params![id, now_ms],
        );
        Ok(())
    }

    // ─── Cross-Device Workspace Sync Helpers (v3.0) ──────────────────────────

    pub fn list_all_kv(&self) -> DesktopResult<Vec<KvRecord>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        let mut stmt = conn.prepare(
            "SELECT key, value_json, updated_at_ms FROM kv_store ORDER BY key ASC",
        )?;
        let rows = stmt
            .query_map([], |r| {
                let raw: String = r.get(1)?;
                let value: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);
                Ok(KvRecord {
                    key: r.get(0)?,
                    value,
                    updated_at_ms: r.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn set_kv_with_ts(&self, key: &str, value: &Value, updated_at_ms: i64) -> DesktopResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        let json = serde_json::to_string(value)?;
        conn.execute(
            r#"
            INSERT INTO kv_store (key, value_json, updated_at_ms)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(key) DO UPDATE SET
                value_json = excluded.value_json,
                updated_at_ms = excluded.updated_at_ms
            WHERE excluded.updated_at_ms >= kv_store.updated_at_ms
            "#,
            params![key, json, updated_at_ms],
        )?;
        Ok(())
    }

    pub fn list_all_drawings(&self) -> DesktopResult<Vec<SavedDrawingRecord>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        let mut stmt = conn.prepare(
            r#"
            SELECT id, ticker, name, pane_id, group_id, points_json, styles_json,
                   extend_data_json, locked, visible, mode, updated_at_ms
            FROM drawings
            ORDER BY ticker ASC, id ASC
            "#,
        )?;
        let iter = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<String>>(7)?,
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
                ticker,
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
            out.push(SavedDrawingRecord {
                id,
                ticker,
                name,
                pane_id,
                group_id,
                points: serde_json::from_str(&points_raw)?,
                styles: styles_raw.as_deref().map(serde_json::from_str).transpose()?,
                extend_data: extend_raw.as_deref().map(serde_json::from_str).transpose()?,
                lock,
                visible,
                mode,
                updated_at_ms,
            });
        }
        Ok(out)
    }

    pub fn list_all_pane_indicators(&self) -> DesktopResult<Vec<PaneIndicatorsRecord>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        let mut stmt = conn.prepare(
            "SELECT slot_id, indicators_json, updated_at_ms FROM pane_indicators ORDER BY slot_id ASC",
        )?;
        let rows = stmt
            .query_map([], |r| {
                let raw: String = r.get(1)?;
                let indicators: Vec<SavedIndicatorSpec> =
                    serde_json::from_str(&raw).unwrap_or_default();
                Ok(PaneIndicatorsRecord {
                    slot_id: r.get(0)?,
                    indicators,
                    updated_at_ms: r.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn save_pane_indicators_with_ts(
        &self,
        slot_id: &str,
        indicators: &[SavedIndicatorSpec],
        updated_at_ms: i64,
    ) -> DesktopResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        let json = serde_json::to_string(indicators)?;
        conn.execute(
            r#"
            INSERT INTO pane_indicators (slot_id, indicators_json, updated_at_ms)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(slot_id) DO UPDATE SET
                indicators_json = excluded.indicators_json,
                updated_at_ms = excluded.updated_at_ms
            WHERE excluded.updated_at_ms >= pane_indicators.updated_at_ms
            "#,
            params![slot_id, json, updated_at_ms],
        )?;
        Ok(())
    }

    pub fn list_all_journal_annotations(&self) -> DesktopResult<Vec<JournalAnnotationRecord>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        let mut stmt = conn.prepare(
            r#"
            SELECT trade_id, ticker, setup_tag, mistake_tag, note, checklist_json, updated_at_ms
            FROM journal_annotations
            ORDER BY ticker ASC, trade_id ASC
            "#,
        )?;
        let rows = stmt
            .query_map([], |r| {
                let ck_raw: Option<String> = r.get(5)?;
                Ok(JournalAnnotationRecord {
                    trade_id: r.get::<_, i64>(0)?.max(0) as u64,
                    ticker: r.get(1)?,
                    setup_tag: r.get(2)?,
                    mistake_tag: r.get(3)?,
                    note: r.get(4)?,
                    checklist_json: ck_raw.and_then(|s| serde_json::from_str::<Value>(&s).ok()),
                    updated_at_ms: r.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn upsert_journal_annotation_with_ts(&self, rec: &JournalAnnotationRecord) -> DesktopResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        let normalized = rec.ticker.trim().to_ascii_uppercase();
        let ck_str = rec
            .checklist_json
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        conn.execute(
            r#"
            INSERT INTO journal_annotations (trade_id, ticker, setup_tag, mistake_tag, note, checklist_json, updated_at_ms)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT(ticker, trade_id) DO UPDATE SET
                setup_tag = excluded.setup_tag,
                mistake_tag = excluded.mistake_tag,
                note = excluded.note,
                checklist_json = excluded.checklist_json,
                updated_at_ms = excluded.updated_at_ms
            WHERE excluded.updated_at_ms >= journal_annotations.updated_at_ms
            "#,
            params![
                rec.trade_id as i64,
                normalized,
                &rec.setup_tag,
                &rec.mistake_tag,
                &rec.note,
                ck_str,
                rec.updated_at_ms,
            ],
        )?;
        Ok(())
    }

    pub fn upsert_watchlist_full(&self, group: &WatchlistGroup, updated_at_ms: i64) -> DesktopResult<()> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        let tx = conn.transaction()?;
        tx.execute(
            r#"
            INSERT INTO watchlists (id, name, sort_order, updated_at_ms)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                sort_order = excluded.sort_order,
                updated_at_ms = excluded.updated_at_ms
            "#,
            params![&group.id, &group.name, group.sort_order, updated_at_ms],
        )?;
        tx.execute(
            "DELETE FROM watchlist_items WHERE watchlist_id = ?1",
            params![&group.id],
        )?;
        for (idx, ticker) in group.tickers.iter().enumerate() {
            tx.execute(
                "INSERT OR IGNORE INTO watchlist_items (watchlist_id, ticker, sort_order) VALUES (?1, ?2, ?3)",
                params![&group.id, ticker.trim().to_ascii_uppercase(), idx as i32],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn list_tombstones(&self) -> DesktopResult<Vec<SyncTombstoneRecord>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        let mut stmt = conn.prepare(
            "SELECT entity_kind, entity_key, deleted_at_ms FROM sync_tombstones ORDER BY entity_kind ASC, entity_key ASC",
        )?;
        let rows = stmt
            .query_map([], |r| {
                Ok(SyncTombstoneRecord {
                    entity_kind: r.get(0)?,
                    entity_key: r.get(1)?,
                    deleted_at_ms: r.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn upsert_tombstone(&self, rec: &SyncTombstoneRecord) -> DesktopResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        conn.execute(
            r#"
            INSERT INTO sync_tombstones (entity_kind, entity_key, deleted_at_ms)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(entity_kind, entity_key) DO UPDATE SET
                deleted_at_ms = MAX(deleted_at_ms, excluded.deleted_at_ms)
            "#,
            params![&rec.entity_kind, &rec.entity_key, rec.deleted_at_ms],
        )?;
        Ok(())
    }

    pub fn save_layout_with_ts(&self, rec: &SavedLayoutRecord) -> DesktopResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        let raw = serde_json::to_string(&rec.layout_json)?;
        conn.execute(
            r#"
            INSERT INTO chart_layouts (id, name, layout_json, updated_at_ms)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                layout_json = excluded.layout_json,
                updated_at_ms = excluded.updated_at_ms
            WHERE excluded.updated_at_ms >= chart_layouts.updated_at_ms
            "#,
            params![&rec.id, &rec.name, raw, rec.updated_at_ms],
        )?;
        Ok(())
    }

    pub fn save_indicator_template_with_ts(&self, tpl: &IndicatorTemplateRecord) -> DesktopResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        let json_str = serde_json::to_string(&tpl.indicators)?;
        conn.execute(
            r#"
            INSERT INTO indicator_templates (id, name, description, indicators_json, updated_at_ms)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                description = excluded.description,
                indicators_json = excluded.indicators_json,
                updated_at_ms = excluded.updated_at_ms
            WHERE excluded.updated_at_ms >= indicator_templates.updated_at_ms
            "#,
            params![&tpl.id, &tpl.name, &tpl.description, json_str, tpl.updated_at_ms],
        )?;
        Ok(())
    }

    pub fn save_replay_session_with_ts(&self, session: &SavedReplaySessionRecord) -> DesktopResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        let ivs_json = serde_json::to_string(&session.tracked_intervals)?;
        let broker_str = session
            .broker_snapshot_json
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        conn.execute(
            r#"
            INSERT INTO replay_sessions (
                id, name, ticker, base_interval, tracked_intervals, cursor_timestamp_ms,
                speed_multiplier, blind_mode, broker_snapshot_json, updated_at_ms
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                ticker = excluded.ticker,
                base_interval = excluded.base_interval,
                tracked_intervals = excluded.tracked_intervals,
                cursor_timestamp_ms = excluded.cursor_timestamp_ms,
                speed_multiplier = excluded.speed_multiplier,
                blind_mode = excluded.blind_mode,
                broker_snapshot_json = excluded.broker_snapshot_json,
                updated_at_ms = excluded.updated_at_ms
            WHERE excluded.updated_at_ms >= replay_sessions.updated_at_ms
            "#,
            params![
                &session.id,
                &session.name,
                session.ticker.trim().to_ascii_uppercase(),
                &session.base_interval,
                ivs_json,
                session.cursor_timestamp_ms,
                session.speed_multiplier,
                i64::from(session.blind_mode),
                broker_str,
                session.updated_at_ms,
            ],
        )?;
        Ok(())
    }

    pub fn list_saved_screens(&self) -> DesktopResult<Vec<SavedScreenRecord>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        let mut stmt = conn.prepare(
            "SELECT id, name, filter_json, updated_at_ms FROM saved_screens ORDER BY updated_at_ms DESC",
        )?;
        let rows = stmt
            .query_map([], |r| {
                let raw: String = r.get(2)?;
                Ok(SavedScreenRecord {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    filter_json: serde_json::from_str(&raw).unwrap_or(Value::Null),
                    updated_at_ms: r.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn save_screen(
        &self,
        id: &str,
        name: &str,
        filter_json: &Value,
    ) -> DesktopResult<SavedScreenRecord> {
        let now_ms = chrono::Utc::now().timestamp_millis();
        let rec = SavedScreenRecord {
            id: id.trim().to_owned(),
            name: name.trim().to_owned(),
            filter_json: filter_json.clone(),
            updated_at_ms: now_ms,
        };
        self.save_screen_with_ts(&rec)?;
        Ok(rec)
    }

    pub fn save_screen_with_ts(&self, rec: &SavedScreenRecord) -> DesktopResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        let raw = serde_json::to_string(&rec.filter_json)?;
        conn.execute(
            r#"
            INSERT INTO saved_screens (id, name, filter_json, updated_at_ms)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                filter_json = excluded.filter_json,
                updated_at_ms = excluded.updated_at_ms
            WHERE excluded.updated_at_ms >= saved_screens.updated_at_ms
            "#,
            params![&rec.id, &rec.name, raw, rec.updated_at_ms],
        )?;
        Ok(())
    }

    pub fn delete_screen(&self, id: &str) -> DesktopResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| DesktopError::Internal("workspace sqlite mutex poisoned".to_owned()))?;
        let now_ms = chrono::Utc::now().timestamp_millis();
        conn.execute("DELETE FROM saved_screens WHERE id = ?1", params![id])?;
        conn.execute(
            r#"
            INSERT INTO sync_tombstones (entity_kind, entity_key, deleted_at_ms)
            VALUES ('saved_screen', ?1, ?2)
            ON CONFLICT(entity_kind, entity_key) DO UPDATE SET
                deleted_at_ms = MAX(deleted_at_ms, excluded.deleted_at_ms)
            "#,
            params![id, now_ms],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn workspace_store_crud_drawings_indicators_watchlists_alerts() {
        let store = WorkspaceStore::open_in_memory().unwrap();

        // Default QMV watchlists seeded automatically
        let lists = store.list_watchlists().unwrap();
        assert_eq!(lists.len(), 5);
        assert_eq!(lists[0].id, "wl_qmv30");
        assert_eq!(lists[0].tickers.len(), 30);
        assert!(lists[0].tickers.contains(&"HOSE:FPT".to_owned()));
        assert!(lists[0].tickers.contains(&"UPCOM:BSR".to_owned()));
        assert!(lists[0].tickers.contains(&"HNX:IDC".to_owned()));
        assert_eq!(lists[1].id, "wl_qmv60");
        assert_eq!(lists[1].tickers.len(), 60);

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

        // Alerts (price + drawing + indicator)
        store
            .upsert_alert(&PriceAlertRecord {
                id: "al_1".to_owned(),
                ticker: "HOSE:FPT".to_owned(),
                alert_kind: "price".to_owned(),
                condition: "cross_above".to_owned(),
                target_price: 135_000.0,
                interval: Some("1D".to_owned()),
                params_json: None,
                sound_enabled: true,
                message: "FPT breakout".to_owned(),
                status: "armed".to_owned(),
                frequency: "once".to_owned(),
                cooldown_bars: 0,
                last_fired_bar_ts: None,
                created_at_ms: 1_700_000_000_000,
                triggered_at_ms: None,
            })
            .unwrap();
        assert_eq!(store.list_alerts().unwrap().len(), 1);

        // Indicator templates seeded + CRUD
        let tpls = store.list_indicator_templates().unwrap();
        assert_eq!(tpls.len(), 4);
        assert!(tpls.iter().any(|t| t.id == "tpl_qmv_ichimoku_v7"));
        store
            .save_indicator_template(&IndicatorTemplateRecord {
                id: "tpl_custom".to_owned(),
                name: "My Custom Setup".to_owned(),
                description: "EMA + RSI".to_owned(),
                indicators: inds.clone(),
                updated_at_ms: 0,
            })
            .unwrap();
        assert_eq!(store.list_indicator_templates().unwrap().len(), 5);
        store.delete_indicator_template("tpl_custom").unwrap();
        assert_eq!(store.list_indicator_templates().unwrap().len(), 4);

        // Saved Replay Sessions roundtrip
        store
            .save_replay_session(&SavedReplaySessionRecord {
                id: "rep_1".to_owned(),
                name: "FPT 1h+4h Q3/2026".to_owned(),
                ticker: "HOSE:FPT".to_owned(),
                base_interval: "1h".to_owned(),
                tracked_intervals: vec!["1h".to_owned(), "4h".to_owned()],
                cursor_timestamp_ms: 1_790_000_000_000,
                speed_multiplier: 2.0,
                blind_mode: true,
                broker_snapshot_json: Some(json!({"initial_cash": 500_000_000.0})),
                updated_at_ms: 0,
            })
            .unwrap();
        assert_eq!(store.list_replay_sessions().unwrap().len(), 1);
        assert!(store.get_replay_session("rep_1").unwrap().is_some());
        store.delete_replay_session("rep_1").unwrap();
        assert!(store.list_replay_sessions().unwrap().is_empty());

        // Saved Screener Presets (v6, Wave 4) roundtrip + tombstone
        let scr = store
            .save_screen("scr_1", "QMV30 Breakout", &json!({"exchangeFilter": "QMV30", "maxPe": 15}))
            .unwrap();
        assert_eq!(scr.name, "QMV30 Breakout");
        assert_eq!(store.list_saved_screens().unwrap().len(), 1);
        store.delete_screen("scr_1").unwrap();
        assert!(store.list_saved_screens().unwrap().is_empty());
        assert!(
            store
                .list_tombstones()
                .unwrap()
                .iter()
                .any(|t| t.entity_kind == "saved_screen" && t.entity_key == "scr_1")
        );

        // Wave 5 Watchlist Pro (v7): rename, reorder (including ###SECTION headers), symbol flags + tombstone
        store.rename_watchlist("wl_qmv30", "QMV30 Core Pro").unwrap();
        let renamed = store.list_watchlists().unwrap();
        assert_eq!(renamed[0].name, "QMV30 Core Pro");

        store
            .reorder_watchlist_symbols(
                "wl_qmv30",
                &[
                    "###SECTION:NGÂN HÀNG".to_owned(),
                    "HOSE:VCB".to_owned(),
                    "HOSE:FPT".to_owned(),
                ],
            )
            .unwrap();
        let reordered = store.list_watchlists().unwrap();
        assert_eq!(
            reordered[0].tickers,
            vec!["###SECTION:NGÂN HÀNG", "HOSE:VCB", "HOSE:FPT"]
        );

        let flag = store.set_symbol_flag("hose:fpt", "green").unwrap();
        assert_eq!(flag.ticker, "HOSE:FPT");
        assert_eq!(flag.color, "green");
        assert_eq!(store.list_symbol_flags().unwrap().len(), 1);
        store.clear_symbol_flag("HOSE:FPT").unwrap();
        assert!(store.list_symbol_flags().unwrap().is_empty());
        assert!(
            store
                .list_tombstones()
                .unwrap()
                .iter()
                .any(|t| t.entity_kind == "symbol_flag" && t.entity_key == "HOSE:FPT")
        );
    }
}
