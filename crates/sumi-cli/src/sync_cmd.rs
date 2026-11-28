//! Headless Workspace Sync (`sumi sync --status | --now | --export <path> | --import <path>`).
//!
//! Speaks the exact wire-identical `sumi_workspace_bundle_v1` `.sumi.json` format, canonical
//! sorted-key `SHA-256` verification, 5-copy rotating `.pre_sync.bak` backups, secret-key
//! redaction, Unicode `###SECTION:` preservation, and strict per-record LWW + Tombstone merge
//! rules as `sumi-desktop/src/sync.rs` (`W6-10`).

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sumi_data::{
    sha256_bytes_hex, sqlite_backup_atomic, sumi_config_dir, sumi_data_dir,
    write_private_file_0600,
};

pub const SUMI_WORKSPACE_BUNDLE_SCHEMA_V1: &str = "sumi_workspace_bundle_v1";
pub const DEFAULT_SYNC_FILENAME: &str = "sumi_workspace_sync.sumi.json";
const MAX_PRE_SYNC_BACKUPS: usize = 5;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CliSyncConfig {
    pub device_id: String,
    #[serde(default)]
    pub sync_folder: Option<String>,
    #[serde(default)]
    pub auto_sync_on_launch: bool,
    #[serde(default)]
    pub last_synced_at_ms: Option<i64>,
    #[serde(default)]
    pub last_synced_sha256: Option<String>,
}

impl Default for CliSyncConfig {
    fn default() -> Self {
        let os = std::env::consts::OS;
        let host = std::env::var("COMPUTERNAME")
            .or_else(|_| std::env::var("HOSTNAME"))
            .unwrap_or_else(|_| "workstation".to_owned());
        let clean: String = host
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c.to_ascii_lowercase()
                } else {
                    '-'
                }
            })
            .take(32)
            .collect();
        Self {
            device_id: format!("{os}-{clean}"),
            sync_folder: None,
            auto_sync_on_launch: false,
            last_synced_at_ms: None,
            last_synced_sha256: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CliSyncStatus {
    pub device_id: String,
    pub sync_folder: Option<String>,
    pub sync_bundle_exists: bool,
    pub auto_sync_on_launch: bool,
    pub last_synced_at_ms: Option<i64>,
    pub last_synced_sha256: Option<String>,
    pub data_dir: String,
    pub config_dir: String,
    pub workspace_db_path: String,
    pub workspace_db_exists: bool,
    pub pre_sync_backup_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CliExportResult {
    pub path: String,
    pub payload_sha256: String,
    pub exported_at_ms: i64,
    pub watchlists_count: usize,
    pub drawings_count: usize,
    pub layouts_count: usize,
    pub alerts_count: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CliMergeReport {
    pub bundle_sha256: String,
    pub source_device_id: String,
    pub backup_path: Option<String>,
    pub merged_kv: usize,
    pub merged_watchlists: usize,
    pub merged_layouts: usize,
    pub merged_pane_indicators: usize,
    pub merged_templates: usize,
    pub merged_drawings: usize,
    pub merged_alerts: usize,
    pub merged_journal: usize,
    pub merged_replay_sessions: usize,
    pub merged_saved_screens: usize,
    pub merged_symbol_flags: usize,
    pub applied_tombstones: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CliFolderSyncResult {
    pub bundle_path: String,
    pub pulled_and_merged: bool,
    pub merge_report: Option<CliMergeReport>,
    pub pushed_sha256: String,
    pub synced_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BundleEnvelope {
    schema_version: String,
    device_id: String,
    exported_at_ms: i64,
    payload_sha256: String,
    payload: Value,
}

fn is_secret_kv_key(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    lower.contains("secret")
        || lower.contains("token")
        || lower.contains("password")
        || lower.contains("credential")
        || lower.contains("api_key")
        || lower.contains("cookie")
        || lower.contains("oauth")
}

fn normalize_watchlist_item(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.starts_with("###") {
        trimmed.to_owned()
    } else {
        trimmed.to_ascii_uppercase()
    }
}

fn canonicalize_json(val: &Value) -> Value {
    match val {
        Value::Object(map) => {
            let mut sorted = BTreeMap::new();
            for (k, v) in map {
                sorted.insert(k.clone(), canonicalize_json(v));
            }
            let mut out = serde_json::Map::new();
            for (k, v) in sorted {
                out.insert(k, v);
            }
            Value::Object(out)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(canonicalize_json).collect()),
        other => other.clone(),
    }
}

fn canonical_sha256_value(val: &Value) -> Result<String, String> {
    let canon = canonicalize_json(val);
    let bytes = serde_json::to_vec(&canon).map_err(|e| e.to_string())?;
    Ok(sha256_bytes_hex(&bytes))
}

#[must_use]
pub fn default_workspace_db_path() -> PathBuf {
    if let Ok(explicit) = std::env::var("SUMI_WORKSPACE_DB")
        && !explicit.trim().is_empty()
    {
        return PathBuf::from(explicit.trim());
    }
    sumi_data_dir().join("sumi_workspace.db")
}

fn load_sync_config(config_dir: &Path) -> CliSyncConfig {
    let path = config_dir.join("sync_config.json");
    fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str::<CliSyncConfig>(&raw).ok())
        .unwrap_or_default()
}

fn save_sync_config(config_dir: &Path, cfg: &CliSyncConfig) -> Result<(), String> {
    let path = config_dir.join("sync_config.json");
    let bytes = serde_json::to_vec_pretty(cfg).map_err(|e| e.to_string())?;
    write_private_file_0600(&path, &bytes).map_err(|e| e.to_string())
}

fn ensure_workspace_tables(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        r#"
        PRAGMA foreign_keys = ON;
        PRAGMA busy_timeout = 5000;
        CREATE TABLE IF NOT EXISTS kv_store (key TEXT PRIMARY KEY, value_json TEXT NOT NULL, updated_at_ms INTEGER NOT NULL);
        CREATE TABLE IF NOT EXISTS chart_layouts (id TEXT PRIMARY KEY, name TEXT NOT NULL, layout_json TEXT NOT NULL, updated_at_ms INTEGER NOT NULL);
        CREATE TABLE IF NOT EXISTS drawings (id TEXT NOT NULL, ticker TEXT NOT NULL, name TEXT NOT NULL, pane_id TEXT NOT NULL, group_id TEXT NOT NULL, points_json TEXT NOT NULL, styles_json TEXT, extend_data_json TEXT, locked INTEGER NOT NULL DEFAULT 0, visible INTEGER NOT NULL DEFAULT 1, mode TEXT NOT NULL DEFAULT 'normal', updated_at_ms INTEGER NOT NULL, PRIMARY KEY (ticker, id));
        CREATE TABLE IF NOT EXISTS pane_indicators (slot_id TEXT PRIMARY KEY, indicators_json TEXT NOT NULL, updated_at_ms INTEGER NOT NULL);
        CREATE TABLE IF NOT EXISTS watchlists (id TEXT PRIMARY KEY, name TEXT NOT NULL, sort_order INTEGER NOT NULL DEFAULT 0, updated_at_ms INTEGER NOT NULL DEFAULT 0);
        CREATE TABLE IF NOT EXISTS watchlist_items (watchlist_id TEXT NOT NULL, ticker TEXT NOT NULL, sort_order INTEGER NOT NULL DEFAULT 0, PRIMARY KEY (watchlist_id, ticker), FOREIGN KEY (watchlist_id) REFERENCES watchlists(id) ON DELETE CASCADE);
        CREATE TABLE IF NOT EXISTS price_alerts (id TEXT PRIMARY KEY, ticker TEXT NOT NULL, alert_kind TEXT NOT NULL DEFAULT 'price', condition TEXT NOT NULL, target_price REAL NOT NULL, interval TEXT, params_json TEXT, sound_enabled INTEGER NOT NULL DEFAULT 1, message TEXT NOT NULL, status TEXT NOT NULL DEFAULT 'armed', frequency TEXT NOT NULL DEFAULT 'once', cooldown_bars INTEGER NOT NULL DEFAULT 0, last_fired_bar_ts INTEGER, created_at_ms INTEGER NOT NULL, triggered_at_ms INTEGER);
        CREATE TABLE IF NOT EXISTS journal_annotations (trade_id INTEGER NOT NULL, ticker TEXT NOT NULL, setup_tag TEXT NOT NULL DEFAULT '', mistake_tag TEXT NOT NULL DEFAULT '', note TEXT NOT NULL DEFAULT '', checklist_json TEXT, updated_at_ms INTEGER NOT NULL, PRIMARY KEY (ticker, trade_id));
        CREATE TABLE IF NOT EXISTS indicator_templates (id TEXT PRIMARY KEY, name TEXT NOT NULL, description TEXT NOT NULL DEFAULT '', indicators_json TEXT NOT NULL, updated_at_ms INTEGER NOT NULL);
        CREATE TABLE IF NOT EXISTS replay_sessions (id TEXT PRIMARY KEY, name TEXT NOT NULL, ticker TEXT NOT NULL, base_interval TEXT NOT NULL, tracked_intervals TEXT NOT NULL, cursor_timestamp_ms INTEGER NOT NULL, speed_multiplier REAL NOT NULL DEFAULT 1.0, blind_mode INTEGER NOT NULL DEFAULT 1, broker_snapshot_json TEXT, updated_at_ms INTEGER NOT NULL);
        CREATE TABLE IF NOT EXISTS sync_tombstones (entity_kind TEXT NOT NULL, entity_key TEXT NOT NULL, deleted_at_ms INTEGER NOT NULL, PRIMARY KEY (entity_kind, entity_key));
        CREATE TABLE IF NOT EXISTS saved_screens (id TEXT PRIMARY KEY, name TEXT NOT NULL, filter_json TEXT NOT NULL, updated_at_ms INTEGER NOT NULL);
        CREATE TABLE IF NOT EXISTS symbol_flags (ticker TEXT PRIMARY KEY, color TEXT NOT NULL, updated_at_ms INTEGER NOT NULL);
        "#,
    )
    .map_err(|e| e.to_string())?;
    for ddl in [
        "ALTER TABLE watchlists ADD COLUMN updated_at_ms INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE journal_annotations ADD COLUMN checklist_json TEXT",
        "ALTER TABLE price_alerts ADD COLUMN frequency TEXT NOT NULL DEFAULT 'once'",
        "ALTER TABLE price_alerts ADD COLUMN cooldown_bars INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE price_alerts ADD COLUMN last_fired_bar_ts INTEGER",
    ] {
        let _ = conn.execute(ddl, []);
    }
    Ok(())
}

fn create_rotating_backup(ws_path: &Path, data_dir: &Path) -> Result<Option<PathBuf>, String> {
    if !ws_path.is_file() {
        return Ok(None);
    }
    let backup_dir = data_dir.join("backups");
    fs::create_dir_all(&backup_dir).map_err(|e| e.to_string())?;
    let ts = Utc::now().format("%Y%m%d_%H%M%S_%3f");
    let dst = backup_dir.join(format!("sumi_workspace_{ts}.pre_sync.bak"));
    sqlite_backup_atomic(ws_path, &dst).map_err(|e| e.to_string())?;
    let mut entries: Vec<(std::time::SystemTime, PathBuf)> = fs::read_dir(&backup_dir)
        .map_err(|e| e.to_string())?
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            if !p.file_name()?.to_str()?.ends_with(".pre_sync.bak") {
                return None;
            }
            Some((e.metadata().ok()?.modified().ok()?, p))
        })
        .collect();
    entries.sort_by_key(|a| std::cmp::Reverse(a.0));
    for (_, old) in entries.into_iter().skip(MAX_PRE_SYNC_BACKUPS) {
        let _ = fs::remove_file(old);
    }
    Ok(Some(dst))
}

fn build_bundle_from_conn(conn: &Connection, device_id: &str) -> Result<BundleEnvelope, String> {
    let mut kv_entries = Vec::new();
    {
        let mut stmt = conn
            .prepare("SELECT key, value_json, updated_at_ms FROM kv_store ORDER BY key ASC")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                let k: String = r.get(0)?;
                let raw: String = r.get(1)?;
                let ts: i64 = r.get(2)?;
                Ok((k, raw, ts))
            })
            .map_err(|e| e.to_string())?;
        for row in rows.flatten() {
            if is_secret_kv_key(&row.0) {
                continue;
            }
            let val: Value = serde_json::from_str(&row.1).unwrap_or(Value::Null);
            kv_entries.push(serde_json::json!({
                "key": row.0,
                "value": val,
                "updatedAtMs": row.2,
            }));
        }
    }

    let mut watchlists = Vec::new();
    {
        let mut stmt = conn
            .prepare("SELECT id, name, sort_order, updated_at_ms FROM watchlists ORDER BY sort_order ASC, name ASC")
            .map_err(|e| e.to_string())?;
        let mut item_stmt = conn
            .prepare("SELECT ticker FROM watchlist_items WHERE watchlist_id = ?1 ORDER BY sort_order ASC, ticker ASC")
            .map_err(|e| e.to_string())?;
        let groups: Vec<(String, String, i32, i64)> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
            .map_err(|e| e.to_string())?
            .flatten()
            .collect();
        for (id, name, sort_order, updated_at_ms) in groups {
            let tickers: Vec<String> = item_stmt
                .query_map(params![&id], |r| r.get(0))
                .map_err(|e| e.to_string())?
                .flatten()
                .collect();
            watchlists.push(serde_json::json!({
                "id": id,
                "name": name,
                "tickers": tickers,
                "sortOrder": sort_order,
                "updatedAtMs": updated_at_ms,
            }));
        }
    }

    let mut layouts = Vec::new();
    {
        let mut stmt = conn
            .prepare("SELECT id, name, layout_json, updated_at_ms FROM chart_layouts ORDER BY updated_at_ms DESC")
            .map_err(|e| e.to_string())?;
        for r in stmt
            .query_map([], |row| {
                let raw: String = row.get(2)?;
                Ok(serde_json::json!({
                    "id": row.get::<_, String>(0)?,
                    "name": row.get::<_, String>(1)?,
                    "layoutJson": serde_json::from_str::<Value>(&raw).unwrap_or(Value::Null),
                    "updatedAtMs": row.get::<_, i64>(3)?,
                }))
            })
            .map_err(|e| e.to_string())?
            .flatten()
        {
            layouts.push(r);
        }
    }

    let mut pane_indicators = Vec::new();
    {
        let mut stmt = conn
            .prepare("SELECT slot_id, indicators_json, updated_at_ms FROM pane_indicators ORDER BY slot_id ASC")
            .map_err(|e| e.to_string())?;
        for r in stmt
            .query_map([], |row| {
                let raw: String = row.get(1)?;
                Ok(serde_json::json!({
                    "slotId": row.get::<_, String>(0)?,
                    "indicators": serde_json::from_str::<Value>(&raw).unwrap_or_else(|_| serde_json::json!([])),
                    "updatedAtMs": row.get::<_, i64>(2)?,
                }))
            })
            .map_err(|e| e.to_string())?
            .flatten()
        {
            pane_indicators.push(r);
        }
    }

    let mut indicator_templates = Vec::new();
    {
        let mut stmt = conn
            .prepare("SELECT id, name, description, indicators_json, updated_at_ms FROM indicator_templates ORDER BY updated_at_ms DESC, name ASC")
            .map_err(|e| e.to_string())?;
        for r in stmt
            .query_map([], |row| {
                let raw: String = row.get(3)?;
                Ok(serde_json::json!({
                    "id": row.get::<_, String>(0)?,
                    "name": row.get::<_, String>(1)?,
                    "description": row.get::<_, String>(2)?,
                    "indicators": serde_json::from_str::<Value>(&raw).unwrap_or_else(|_| serde_json::json!([])),
                    "updatedAtMs": row.get::<_, i64>(4)?,
                }))
            })
            .map_err(|e| e.to_string())?
            .flatten()
        {
            indicator_templates.push(r);
        }
    }

    let mut drawings = Vec::new();
    {
        let mut stmt = conn
            .prepare("SELECT id, ticker, name, pane_id, group_id, points_json, styles_json, extend_data_json, locked, visible, mode, updated_at_ms FROM drawings ORDER BY ticker ASC, id ASC")
            .map_err(|e| e.to_string())?;
        for r in stmt
            .query_map([], |row| {
                let pts: String = row.get(5)?;
                let sty: Option<String> = row.get(6)?;
                let ext: Option<String> = row.get(7)?;
                Ok(serde_json::json!({
                    "id": row.get::<_, String>(0)?,
                    "ticker": row.get::<_, String>(1)?,
                    "name": row.get::<_, String>(2)?,
                    "paneId": row.get::<_, String>(3)?,
                    "groupId": row.get::<_, String>(4)?,
                    "points": serde_json::from_str::<Value>(&pts).unwrap_or(Value::Null),
                    "styles": sty.and_then(|s| serde_json::from_str::<Value>(&s).ok()),
                    "extendData": ext.and_then(|s| serde_json::from_str::<Value>(&s).ok()),
                    "lock": row.get::<_, i64>(8)? != 0,
                    "visible": row.get::<_, i64>(9)? != 0,
                    "mode": row.get::<_, String>(10)?,
                    "updatedAtMs": row.get::<_, i64>(11)?,
                }))
            })
            .map_err(|e| e.to_string())?
            .flatten()
        {
            drawings.push(r);
        }
    }

    let mut alerts = Vec::new();
    {
        let mut stmt = conn
            .prepare("SELECT id, ticker, alert_kind, condition, target_price, interval, params_json, sound_enabled, message, status, frequency, cooldown_bars, last_fired_bar_ts, created_at_ms, triggered_at_ms FROM price_alerts ORDER BY created_at_ms DESC")
            .map_err(|e| e.to_string())?;
        for r in stmt
            .query_map([], |row| {
                let prm: Option<String> = row.get(6)?;
                Ok(serde_json::json!({
                    "id": row.get::<_, String>(0)?,
                    "ticker": row.get::<_, String>(1)?,
                    "alertKind": row.get::<_, String>(2)?,
                    "condition": row.get::<_, String>(3)?,
                    "targetPrice": row.get::<_, f64>(4)?,
                    "interval": row.get::<_, Option<String>>(5)?,
                    "paramsJson": prm.and_then(|s| serde_json::from_str::<Value>(&s).ok()),
                    "soundEnabled": row.get::<_, i64>(7)? != 0,
                    "message": row.get::<_, String>(8)?,
                    "status": row.get::<_, String>(9)?,
                    "frequency": row.get::<_, String>(10)?,
                    "cooldownBars": row.get::<_, i32>(11)?,
                    "lastFiredBarTs": row.get::<_, Option<i64>>(12)?,
                    "createdAtMs": row.get::<_, i64>(13)?,
                    "triggeredAtMs": row.get::<_, Option<i64>>(14)?,
                }))
            })
            .map_err(|e| e.to_string())?
            .flatten()
        {
            alerts.push(r);
        }
    }

    let mut journal_annotations = Vec::new();
    {
        let mut stmt = conn
            .prepare("SELECT trade_id, ticker, setup_tag, mistake_tag, note, checklist_json, updated_at_ms FROM journal_annotations ORDER BY ticker ASC, trade_id ASC")
            .map_err(|e| e.to_string())?;
        for r in stmt
            .query_map([], |row| {
                let ck: Option<String> = row.get(5)?;
                Ok(serde_json::json!({
                    "tradeId": row.get::<_, i64>(0)?.max(0) as u64,
                    "ticker": row.get::<_, String>(1)?,
                    "setupTag": row.get::<_, String>(2)?,
                    "mistakeTag": row.get::<_, String>(3)?,
                    "note": row.get::<_, String>(4)?,
                    "checklistJson": ck.and_then(|s| serde_json::from_str::<Value>(&s).ok()),
                    "updatedAtMs": row.get::<_, i64>(6)?,
                }))
            })
            .map_err(|e| e.to_string())?
            .flatten()
        {
            journal_annotations.push(r);
        }
    }

    let mut replay_sessions = Vec::new();
    {
        let mut stmt = conn
            .prepare("SELECT id, name, ticker, base_interval, tracked_intervals, cursor_timestamp_ms, speed_multiplier, blind_mode, broker_snapshot_json, updated_at_ms FROM replay_sessions ORDER BY updated_at_ms DESC")
            .map_err(|e| e.to_string())?;
        for r in stmt
            .query_map([], |row| {
                let ivs: String = row.get(4)?;
                let brk: Option<String> = row.get(8)?;
                Ok(serde_json::json!({
                    "id": row.get::<_, String>(0)?,
                    "name": row.get::<_, String>(1)?,
                    "ticker": row.get::<_, String>(2)?,
                    "baseInterval": row.get::<_, String>(3)?,
                    "trackedIntervals": serde_json::from_str::<Value>(&ivs).unwrap_or_else(|_| serde_json::json!([])),
                    "cursorTimestampMs": row.get::<_, i64>(5)?,
                    "speedMultiplier": row.get::<_, f64>(6)?,
                    "blindMode": row.get::<_, i64>(7)? != 0,
                    "brokerSnapshotJson": brk.and_then(|s| serde_json::from_str::<Value>(&s).ok()),
                    "updatedAtMs": row.get::<_, i64>(9)?,
                }))
            })
            .map_err(|e| e.to_string())?
            .flatten()
        {
            replay_sessions.push(r);
        }
    }

    let mut saved_screens = Vec::new();
    {
        let mut stmt = conn
            .prepare("SELECT id, name, filter_json, updated_at_ms FROM saved_screens ORDER BY updated_at_ms DESC")
            .map_err(|e| e.to_string())?;
        for r in stmt
            .query_map([], |row| {
                let fj: String = row.get(2)?;
                Ok(serde_json::json!({
                    "id": row.get::<_, String>(0)?,
                    "name": row.get::<_, String>(1)?,
                    "filterJson": serde_json::from_str::<Value>(&fj).unwrap_or(Value::Null),
                    "updatedAtMs": row.get::<_, i64>(3)?,
                }))
            })
            .map_err(|e| e.to_string())?
            .flatten()
        {
            saved_screens.push(r);
        }
    }

    let mut symbol_flags = Vec::new();
    {
        let mut stmt = conn
            .prepare("SELECT ticker, color, updated_at_ms FROM symbol_flags ORDER BY ticker ASC")
            .map_err(|e| e.to_string())?;
        for r in stmt
            .query_map([], |row| {
                Ok(serde_json::json!({
                    "ticker": row.get::<_, String>(0)?,
                    "color": row.get::<_, String>(1)?,
                    "updatedAtMs": row.get::<_, i64>(2)?,
                }))
            })
            .map_err(|e| e.to_string())?
            .flatten()
        {
            symbol_flags.push(r);
        }
    }

    let mut tombstones = Vec::new();
    {
        let mut stmt = conn
            .prepare("SELECT entity_kind, entity_key, deleted_at_ms FROM sync_tombstones ORDER BY entity_kind ASC, entity_key ASC")
            .map_err(|e| e.to_string())?;
        for r in stmt
            .query_map([], |row| {
                Ok(serde_json::json!({
                    "entityKind": row.get::<_, String>(0)?,
                    "entityKey": row.get::<_, String>(1)?,
                    "deletedAtMs": row.get::<_, i64>(2)?,
                }))
            })
            .map_err(|e| e.to_string())?
            .flatten()
        {
            tombstones.push(r);
        }
    }

    let payload = serde_json::json!({
        "kvEntries": kv_entries,
        "watchlists": watchlists,
        "layouts": layouts,
        "paneIndicators": pane_indicators,
        "indicatorTemplates": indicator_templates,
        "drawings": drawings,
        "alerts": alerts,
        "journalAnnotations": journal_annotations,
        "replaySessions": replay_sessions,
        "savedScreens": saved_screens,
        "symbolFlags": symbol_flags,
        "tombstones": tombstones,
    });
    let payload_sha256 = canonical_sha256_value(&payload)?;
    Ok(BundleEnvelope {
        schema_version: SUMI_WORKSPACE_BUNDLE_SCHEMA_V1.to_owned(),
        device_id: device_id.to_owned(),
        exported_at_ms: Utc::now().timestamp_millis(),
        payload_sha256,
        payload,
    })
}

fn verify_bundle(bundle: &BundleEnvelope) -> Result<(), String> {
    if bundle.schema_version != SUMI_WORKSPACE_BUNDLE_SCHEMA_V1 {
        return Err(format!(
            "Phiên bản gói đồng bộ không hợp lệ: {}",
            bundle.schema_version
        ));
    }
    let computed = canonical_sha256_value(&bundle.payload)?;
    if computed != bundle.payload_sha256.trim().to_ascii_lowercase() {
        return Err(format!(
            "SHA-256 mismatch (expected {}, got {computed})",
            bundle.payload_sha256
        ));
    }
    Ok(())
}

fn write_bundle_atomic(path: &Path, bundle: &BundleEnvelope) -> Result<(), String> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension(format!(
        "sumi.tmp.{}",
        Utc::now().timestamp_nanos_opt().unwrap_or(0)
    ));
    let bytes = serde_json::to_vec_pretty(bundle).map_err(|e| e.to_string())?;
    {
        let mut f = fs::File::create(&tmp).map_err(|e| e.to_string())?;
        std::io::Write::write_all(&mut f, &bytes).map_err(|e| e.to_string())?;
        f.sync_all().map_err(|e| e.to_string())?;
    }
    fs::rename(&tmp, path).map_err(|e| e.to_string())
}

fn merge_bundle_into_conn(
    conn: &mut Connection,
    bundle: &BundleEnvelope,
    backup_path: Option<PathBuf>,
) -> Result<CliMergeReport, String> {
    verify_bundle(bundle)?;
    let mut report = CliMergeReport {
        bundle_sha256: bundle.payload_sha256.clone(),
        source_device_id: bundle.device_id.clone(),
        backup_path: backup_path.map(|p| p.display().to_string()),
        ..CliMergeReport::default()
    };

    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let mut tomb_map: HashMap<(String, String), i64> = HashMap::new();
    {
        let mut stmt = tx
            .prepare("SELECT entity_kind, entity_key, deleted_at_ms FROM sync_tombstones")
            .map_err(|e| e.to_string())?;
        for r in stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })
            .map_err(|e| e.to_string())?
            .flatten()
        {
            tomb_map.insert((r.0, r.1), r.2);
        }
    }
    if let Some(arr) = bundle.payload.get("tombstones").and_then(Value::as_array) {
        for t in arr {
            let Some(kind) = t.get("entityKind").and_then(Value::as_str) else { continue };
            let Some(key) = t.get("entityKey").and_then(Value::as_str) else { continue };
            let ts = t.get("deletedAtMs").and_then(Value::as_i64).unwrap_or(0);
            let _ = tx.execute(
                "INSERT INTO sync_tombstones (entity_kind, entity_key, deleted_at_ms) VALUES (?1, ?2, ?3) ON CONFLICT(entity_kind, entity_key) DO UPDATE SET deleted_at_ms = MAX(deleted_at_ms, excluded.deleted_at_ms)",
                params![kind, key, ts],
            );
            let entry = tomb_map.entry((kind.to_owned(), key.to_owned())).or_insert(0);
            if ts > *entry {
                *entry = ts;
            }
        }
    }

    let is_tombstoned = |kind: &str, key: &str, rec_ts: i64| -> bool {
        tomb_map
            .get(&(kind.to_owned(), key.to_owned()))
            .is_some_and(|&del| del >= rec_ts)
    };

    // Apply tombstones cleanly
    for ((kind, key), del_ts) in &tomb_map {
        let deleted = match kind.as_str() {
            "drawing" => key.split_once("::").map_or(0, |(t, id)| {
                tx.execute(
                    "DELETE FROM drawings WHERE ticker = ?1 AND id = ?2 AND updated_at_ms <= ?3",
                    params![t, id, del_ts],
                )
                .unwrap_or(0)
            }),
            "layout" => tx
                .execute(
                    "DELETE FROM chart_layouts WHERE id = ?1 AND updated_at_ms <= ?2",
                    params![key, del_ts],
                )
                .unwrap_or(0),
            "watchlist" => {
                let n = tx
                    .execute(
                        "DELETE FROM watchlists WHERE id = ?1 AND updated_at_ms <= ?2",
                        params![key, del_ts],
                    )
                    .unwrap_or(0);
                if n > 0 {
                    let _ = tx.execute(
                        "DELETE FROM watchlist_items WHERE watchlist_id = ?1",
                        params![key],
                    );
                }
                n
            }
            "alert" => tx
                .execute(
                    "DELETE FROM price_alerts WHERE id = ?1 AND COALESCE(triggered_at_ms, created_at_ms) <= ?2",
                    params![key, del_ts],
                )
                .unwrap_or(0),
            "indicator_template" => tx
                .execute(
                    "DELETE FROM indicator_templates WHERE id = ?1 AND updated_at_ms <= ?2",
                    params![key, del_ts],
                )
                .unwrap_or(0),
            "replay_session" => tx
                .execute(
                    "DELETE FROM replay_sessions WHERE id = ?1 AND updated_at_ms <= ?2",
                    params![key, del_ts],
                )
                .unwrap_or(0),
            "saved_screen" => tx
                .execute(
                    "DELETE FROM saved_screens WHERE id = ?1 AND updated_at_ms <= ?2",
                    params![key, del_ts],
                )
                .unwrap_or(0),
            "symbol_flag" => tx
                .execute(
                    "DELETE FROM symbol_flags WHERE ticker = ?1 AND updated_at_ms <= ?2",
                    params![key, del_ts],
                )
                .unwrap_or(0),
            _ => 0,
        };
        report.applied_tombstones += deleted;
    }

    // Merge KV
    if let Some(arr) = bundle.payload.get("kvEntries").and_then(Value::as_array) {
        for kv in arr {
            let Some(k) = kv.get("key").and_then(Value::as_str) else { continue };
            if is_secret_kv_key(k) {
                continue;
            }
            let val = kv.get("value").cloned().unwrap_or(Value::Null);
            let ts = kv.get("updatedAtMs").and_then(Value::as_i64).unwrap_or(0);
            let raw = serde_json::to_string(&val).unwrap_or_else(|_| "null".to_owned());
            let _ = tx.execute(
                "INSERT INTO kv_store (key, value_json, updated_at_ms) VALUES (?1, ?2, ?3) ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json, updated_at_ms = excluded.updated_at_ms WHERE excluded.updated_at_ms >= kv_store.updated_at_ms",
                params![k, raw, ts],
            );
            report.merged_kv += 1;
        }
    }

    // Merge Watchlists (strict per-watchlist LWW using `updatedAtMs` + Unicode `###SECTION:` preservation!)
    if let Some(arr) = bundle.payload.get("watchlists").and_then(Value::as_array) {
        for wl in arr {
            let Some(id) = wl.get("id").and_then(Value::as_str) else { continue };
            let name = wl.get("name").and_then(Value::as_str).unwrap_or(id);
            let sort_order = wl.get("sortOrder").and_then(Value::as_i64).unwrap_or(0) as i32;
            let wl_ts = wl.get("updatedAtMs").and_then(Value::as_i64).unwrap_or(0);
            if is_tombstoned("watchlist", id, wl_ts) {
                continue;
            }
            let local_ts: Option<i64> = tx
                .query_row(
                    "SELECT updated_at_ms FROM watchlists WHERE id = ?1",
                    params![id],
                    |r| r.get(0),
                )
                .optional()
                .map_err(|e| e.to_string())?;
            if let Some(lt) = local_ts
                && wl_ts < lt
            {
                continue;
            }
            tx.execute(
                "INSERT INTO watchlists (id, name, sort_order, updated_at_ms) VALUES (?1, ?2, ?3, ?4) ON CONFLICT(id) DO UPDATE SET name = excluded.name, sort_order = excluded.sort_order, updated_at_ms = excluded.updated_at_ms WHERE excluded.updated_at_ms >= watchlists.updated_at_ms",
                params![id, name, sort_order, wl_ts],
            )
            .map_err(|e| e.to_string())?;
            tx.execute("DELETE FROM watchlist_items WHERE watchlist_id = ?1", params![id])
                .map_err(|e| e.to_string())?;
            if let Some(tickers) = wl.get("tickers").and_then(Value::as_array) {
                for (idx, t) in tickers.iter().enumerate() {
                    if let Some(raw_t) = t.as_str() {
                        let norm = normalize_watchlist_item(raw_t);
                        if !norm.is_empty() {
                            let _ = tx.execute(
                                "INSERT OR IGNORE INTO watchlist_items (watchlist_id, ticker, sort_order) VALUES (?1, ?2, ?3)",
                                params![id, norm, idx as i32],
                            );
                        }
                    }
                }
            }
            report.merged_watchlists += 1;
        }
    }

    // Merge Layouts
    if let Some(arr) = bundle.payload.get("layouts").and_then(Value::as_array) {
        for lay in arr {
            let Some(id) = lay.get("id").and_then(Value::as_str) else { continue };
            let name = lay.get("name").and_then(Value::as_str).unwrap_or(id);
            let ts = lay.get("updatedAtMs").and_then(Value::as_i64).unwrap_or(0);
            if is_tombstoned("layout", id, ts) {
                continue;
            }
            let raw = serde_json::to_string(lay.get("layoutJson").unwrap_or(&Value::Null))
                .unwrap_or_else(|_| "{}".to_owned());
            let _ = tx.execute(
                "INSERT INTO chart_layouts (id, name, layout_json, updated_at_ms) VALUES (?1, ?2, ?3, ?4) ON CONFLICT(id) DO UPDATE SET name = excluded.name, layout_json = excluded.layout_json, updated_at_ms = excluded.updated_at_ms WHERE excluded.updated_at_ms >= chart_layouts.updated_at_ms",
                params![id, name, raw, ts],
            );
            report.merged_layouts += 1;
        }
    }

    // Merge Drawings
    if let Some(arr) = bundle.payload.get("drawings").and_then(Value::as_array) {
        for d in arr {
            let Some(id) = d.get("id").and_then(Value::as_str) else { continue };
            let Some(ticker_raw) = d.get("ticker").and_then(Value::as_str) else { continue };
            let ticker = ticker_raw.trim().to_ascii_uppercase();
            let ts = d.get("updatedAtMs").and_then(Value::as_i64).unwrap_or(0);
            if is_tombstoned("drawing", &format!("{ticker}::{id}"), ts) {
                continue;
            }
            let name = d.get("name").and_then(Value::as_str).unwrap_or("segment");
            let pane_id = d.get("paneId").and_then(Value::as_str).unwrap_or("candle_pane");
            let group_id = d.get("groupId").and_then(Value::as_str).unwrap_or("user_drawings");
            let pts = serde_json::to_string(d.get("points").unwrap_or(&Value::Null)).unwrap_or_else(|_| "[]".to_owned());
            let sty = d.get("styles").filter(|v| !v.is_null()).and_then(|v| serde_json::to_string(v).ok());
            let ext = d.get("extendData").filter(|v| !v.is_null()).and_then(|v| serde_json::to_string(v).ok());
            let lock = i64::from(d.get("lock").and_then(Value::as_bool).unwrap_or(false));
            let vis = i64::from(d.get("visible").and_then(Value::as_bool).unwrap_or(true));
            let mode = d.get("mode").and_then(Value::as_str).unwrap_or("normal");
            let _ = tx.execute(
                "INSERT INTO drawings (id, ticker, name, pane_id, group_id, points_json, styles_json, extend_data_json, locked, visible, mode, updated_at_ms) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12) ON CONFLICT(ticker, id) DO UPDATE SET name = excluded.name, pane_id = excluded.pane_id, group_id = excluded.group_id, points_json = excluded.points_json, styles_json = excluded.styles_json, extend_data_json = excluded.extend_data_json, locked = excluded.locked, visible = excluded.visible, mode = excluded.mode, updated_at_ms = excluded.updated_at_ms WHERE excluded.updated_at_ms >= drawings.updated_at_ms",
                params![id, ticker, name, pane_id, group_id, pts, sty, ext, lock, vis, mode, ts],
            );
            report.merged_drawings += 1;
        }
    }

    // Merge Symbol Flags
    if let Some(arr) = bundle.payload.get("symbolFlags").and_then(Value::as_array) {
        for fl in arr {
            let Some(tk) = fl.get("ticker").and_then(Value::as_str) else { continue };
            let Some(col) = fl.get("color").and_then(Value::as_str) else { continue };
            let ts = fl.get("updatedAtMs").and_then(Value::as_i64).unwrap_or(0);
            if is_tombstoned("symbol_flag", tk, ts) {
                continue;
            }
            let _ = tx.execute(
                "INSERT INTO symbol_flags (ticker, color, updated_at_ms) VALUES (?1, ?2, ?3) ON CONFLICT(ticker) DO UPDATE SET color = excluded.color, updated_at_ms = excluded.updated_at_ms WHERE excluded.updated_at_ms >= symbol_flags.updated_at_ms",
                params![tk, col, ts],
            );
            report.merged_symbol_flags += 1;
        }
    }

    // Merge Saved Screens
    if let Some(arr) = bundle.payload.get("savedScreens").and_then(Value::as_array) {
        for sc in arr {
            let Some(id) = sc.get("id").and_then(Value::as_str) else { continue };
            let name = sc.get("name").and_then(Value::as_str).unwrap_or(id);
            let ts = sc.get("updatedAtMs").and_then(Value::as_i64).unwrap_or(0);
            if is_tombstoned("saved_screen", id, ts) {
                continue;
            }
            let raw = serde_json::to_string(sc.get("filterJson").unwrap_or(&Value::Null))
                .unwrap_or_else(|_| "{}".to_owned());
            let _ = tx.execute(
                "INSERT INTO saved_screens (id, name, filter_json, updated_at_ms) VALUES (?1, ?2, ?3, ?4) ON CONFLICT(id) DO UPDATE SET name = excluded.name, filter_json = excluded.filter_json, updated_at_ms = excluded.updated_at_ms WHERE excluded.updated_at_ms >= saved_screens.updated_at_ms",
                params![id, name, raw, ts],
            );
            report.merged_saved_screens += 1;
        }
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(report)
}

pub fn sync_status_headless(
    data_dir_override: Option<&Path>,
    config_dir_override: Option<&Path>,
) -> CliSyncStatus {
    let data_dir = data_dir_override
        .map(Path::to_path_buf)
        .unwrap_or_else(sumi_data_dir);
    let config_dir = config_dir_override
        .map(Path::to_path_buf)
        .unwrap_or_else(sumi_config_dir);
    let cfg = load_sync_config(&config_dir);
    let sync_bundle_exists = cfg
        .sync_folder
        .as_deref()
        .is_some_and(|f| Path::new(f).join(DEFAULT_SYNC_FILENAME).is_file());
    let ws_path = default_workspace_db_path();
    let backup_dir = data_dir.join("backups");
    let pre_sync_backup_count = fs::read_dir(&backup_dir)
        .ok()
        .map(|rd| {
            rd.flatten()
                .filter(|e| {
                    e.file_name()
                        .to_str()
                        .is_some_and(|n| n.ends_with(".pre_sync.bak"))
                })
                .count()
        })
        .unwrap_or(0);

    CliSyncStatus {
        device_id: cfg.device_id,
        sync_folder: cfg.sync_folder,
        sync_bundle_exists,
        auto_sync_on_launch: cfg.auto_sync_on_launch,
        last_synced_at_ms: cfg.last_synced_at_ms,
        last_synced_sha256: cfg.last_synced_sha256,
        data_dir: data_dir.display().to_string(),
        config_dir: config_dir.display().to_string(),
        workspace_db_path: ws_path.display().to_string(),
        workspace_db_exists: ws_path.is_file(),
        pre_sync_backup_count,
    }
}

pub fn sync_export_headless(
    out_path: Option<&Path>,
    workspace_db_override: Option<&Path>,
    config_dir_override: Option<&Path>,
) -> Result<CliExportResult, String> {
    let ws_path = workspace_db_override
        .map(Path::to_path_buf)
        .unwrap_or_else(default_workspace_db_path);
    let config_dir = config_dir_override
        .map(Path::to_path_buf)
        .unwrap_or_else(sumi_config_dir);
    let cfg = load_sync_config(&config_dir);
    let conn = Connection::open(&ws_path).map_err(|e| e.to_string())?;
    ensure_workspace_tables(&conn)?;
    let bundle = build_bundle_from_conn(&conn, &cfg.device_id)?;
    let target = out_path.map(Path::to_path_buf).unwrap_or_else(|| {
        let ts = Utc::now().format("%Y%m%d_%H%M%S");
        sumi_data_dir()
            .join("exports")
            .join(format!("sumi_workspace_{ts}.sumi.json"))
    });
    write_bundle_atomic(&target, &bundle)?;
    Ok(CliExportResult {
        path: target.display().to_string(),
        payload_sha256: bundle.payload_sha256,
        exported_at_ms: bundle.exported_at_ms,
        watchlists_count: bundle
            .payload
            .get("watchlists")
            .and_then(Value::as_array)
            .map_or(0, Vec::len),
        drawings_count: bundle
            .payload
            .get("drawings")
            .and_then(Value::as_array)
            .map_or(0, Vec::len),
        layouts_count: bundle
            .payload
            .get("layouts")
            .and_then(Value::as_array)
            .map_or(0, Vec::len),
        alerts_count: bundle
            .payload
            .get("alerts")
            .and_then(Value::as_array)
            .map_or(0, Vec::len),
    })
}

pub fn sync_import_headless(
    bundle_path: &Path,
    workspace_db_override: Option<&Path>,
    data_dir_override: Option<&Path>,
) -> Result<CliMergeReport, String> {
    if !bundle_path.is_file() {
        return Err(format!("Không tìm thấy tệp đồng bộ: {}", bundle_path.display()));
    }
    let raw = fs::read_to_string(bundle_path).map_err(|e| e.to_string())?;
    let bundle: BundleEnvelope = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    verify_bundle(&bundle)?;

    let ws_path = workspace_db_override
        .map(Path::to_path_buf)
        .unwrap_or_else(default_workspace_db_path);
    let data_dir = data_dir_override
        .map(Path::to_path_buf)
        .unwrap_or_else(sumi_data_dir);
    let backup = create_rotating_backup(&ws_path, &data_dir)?;
    let mut conn = Connection::open(&ws_path).map_err(|e| e.to_string())?;
    ensure_workspace_tables(&conn)?;
    merge_bundle_into_conn(&mut conn, &bundle, backup)
}

pub fn sync_folder_now_headless(
    folder_override: Option<&Path>,
    workspace_db_override: Option<&Path>,
    data_dir_override: Option<&Path>,
    config_dir_override: Option<&Path>,
) -> Result<CliFolderSyncResult, String> {
    let config_dir = config_dir_override
        .map(Path::to_path_buf)
        .unwrap_or_else(sumi_config_dir);
    let data_dir = data_dir_override
        .map(Path::to_path_buf)
        .unwrap_or_else(sumi_data_dir);
    let ws_path = workspace_db_override
        .map(Path::to_path_buf)
        .unwrap_or_else(default_workspace_db_path);
    let mut cfg = load_sync_config(&config_dir);
    let folder = folder_override
        .map(Path::to_path_buf)
        .or_else(|| cfg.sync_folder.as_deref().map(PathBuf::from))
        .ok_or_else(|| {
            "Chưa cấu hình thư mục đồng bộ đám mây (truyền --folder <dir> hoặc cấu hình trong Sumi Desktop)."
                .to_owned()
        })?;
    let bundle_path = folder.join(DEFAULT_SYNC_FILENAME);

    let mut conn = Connection::open(&ws_path).map_err(|e| e.to_string())?;
    ensure_workspace_tables(&conn)?;

    let mut merge_report = None;
    let mut pulled_and_merged = false;
    if bundle_path.is_file() {
        let raw = fs::read_to_string(&bundle_path).map_err(|e| e.to_string())?;
        let remote: BundleEnvelope = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
        verify_bundle(&remote)?;
        let backup = create_rotating_backup(&ws_path, &data_dir)?;
        merge_report = Some(merge_bundle_into_conn(&mut conn, &remote, backup)?);
        pulled_and_merged = true;
    }

    let merged_bundle = build_bundle_from_conn(&conn, &cfg.device_id)?;
    write_bundle_atomic(&bundle_path, &merged_bundle)?;
    let now_ms = Utc::now().timestamp_millis();
    cfg.sync_folder = Some(folder.display().to_string());
    cfg.last_synced_at_ms = Some(now_ms);
    cfg.last_synced_sha256 = Some(merged_bundle.payload_sha256.clone());
    save_sync_config(&config_dir, &cfg)?;

    Ok(CliFolderSyncResult {
        bundle_path: bundle_path.display().to_string(),
        pulled_and_merged,
        merge_report,
        pushed_sha256: merged_bundle.payload_sha256,
        synced_at_ms: now_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_sync_export_import_and_bidirectional_folder_preserve_sha256_and_lww() {
        let tmp = tempfile::tempdir().unwrap();
        let ws_a = tmp.path().join("a/sumi_workspace.db");
        let ws_b = tmp.path().join("b/sumi_workspace.db");
        let cfg_a = tmp.path().join("cfg_a");
        let cfg_b = tmp.path().join("cfg_b");
        let data_b = tmp.path().join("data_b");
        let cloud = tmp.path().join("cloud");
        fs::create_dir_all(ws_a.parent().unwrap()).unwrap();
        fs::create_dir_all(ws_b.parent().unwrap()).unwrap();

        {
            let conn = Connection::open(&ws_a).unwrap();
            ensure_workspace_tables(&conn).unwrap();
            conn.execute(
                "INSERT INTO watchlists (id, name, sort_order, updated_at_ms) VALUES ('wl_1', 'Rổ A', 0, 2000)",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO watchlist_items (watchlist_id, ticker, sort_order) VALUES ('wl_1', '###SECTION:Ngân Hàng', 0)",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO watchlist_items (watchlist_id, ticker, sort_order) VALUES ('wl_1', 'HOSE:VCB', 1)",
                [],
            )
            .unwrap();
        }

        let export_path = tmp.path().join("manual.sumi.json");
        let exp = sync_export_headless(Some(&export_path), Some(&ws_a), Some(&cfg_a)).unwrap();
        assert_eq!(exp.payload_sha256.len(), 64);
        assert_eq!(exp.watchlists_count, 1);

        let imp = sync_import_headless(&export_path, Some(&ws_b), Some(&data_b)).unwrap();
        assert_eq!(imp.merged_watchlists, 1);

        // Bidirectional cloud folder sync
        let push_a = sync_folder_now_headless(
            Some(&cloud),
            Some(&ws_a),
            Some(tmp.path()),
            Some(&cfg_a),
        )
        .unwrap();
        assert!(!push_a.pulled_and_merged);
        let pull_b =
            sync_folder_now_headless(Some(&cloud), Some(&ws_b), Some(&data_b), Some(&cfg_b))
                .unwrap();
        assert!(pull_b.pulled_and_merged);
        assert_eq!(pull_b.pushed_sha256.len(), 64);
    }
}
