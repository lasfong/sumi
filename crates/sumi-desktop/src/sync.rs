//! Cross-Device Workspace Sync Engine (`Mac <-> Windows PC <-> Linux`, Wave 1).
//!
//! Architecture (Decision #1):
//! 1. **Tier 1 (Immediate, 0 server dependency):** Deterministic, canonical-JSON `SHA-256`-verified
//!    `SumiWorkspaceBundleV1` (`.sumi.json`) with bidirectional per-record Last-Write-Wins (LWW) +
//!    Tombstone merge against any user-selected Cloud Sync Folder (`iCloud Drive`, `OneDrive`,
//!    `Dropbox`, `Syncthing`, `Git`), protected by a 5-copy rotating `.pre_sync.bak` backup!
//! 2. **Tier 2 (Pre-Wired Future Cloud Sync):** `DoraemiSyncTransport` implementing RFC 7636
//!    OAuth 2.1 + PKCE (`S256`) and `/sync/v1/{status,push,pull}` wire compatibility (targets
//!    `entity_kind = "sumi_workspace_bundle"` once `DORAEMI_MIGRATION_0006_PROPOSAL.sql` is applied).
//!
//! Security invariant: Secrets (`alerts_secrets.json`, OAuth tokens, Telegram tokens) are NEVER
//! embedded in `SumiWorkspaceBundleV1`.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::Utc;
use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sumi_data::{
    sha256_bytes_hex, sqlite_backup_atomic, sumi_config_dir, sumi_data_dir,
    write_private_file_0600, yumeko_config_dir,
};
use uuid::Uuid;

use crate::error::{DesktopError, DesktopResult};
use crate::workspace_db::{
    IndicatorTemplateRecord, JournalAnnotationRecord, KvRecord, PaneIndicatorsRecord,
    PriceAlertRecord, SavedDrawingRecord, SavedLayoutRecord, SavedReplaySessionRecord,
    SavedScreenRecord, SymbolFlagRecord, SyncTombstoneRecord, WatchlistGroup, WorkspaceStore,
};

pub const SUMI_WORKSPACE_BUNDLE_SCHEMA_V1: &str = "sumi_workspace_bundle_v1";
pub const DEFAULT_SYNC_FILENAME: &str = "sumi_workspace_sync.sumi.json";
const MAX_PRE_SYNC_BACKUPS: usize = 5;

/// Persistent sync preferences stored in `~/.config/sumi/sync_config.json` (or `%APPDATA%\sumi\`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncConfig {
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

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            device_id: default_device_id(),
            sync_folder: None,
            auto_sync_on_launch: false,
            last_synced_at_ms: None,
            last_synced_sha256: None,
        }
    }
}

#[must_use]
pub fn default_device_id() -> String {
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
    format!("{os}-{clean}")
}

pub fn load_sync_config(config_dir: &Path) -> SyncConfig {
    let path = config_dir.join("sync_config.json");
    if let Ok(raw) = fs::read_to_string(&path)
        && let Ok(parsed) = serde_json::from_str::<SyncConfig>(&raw)
    {
        return parsed;
    }
    SyncConfig::default()
}

pub fn save_sync_config(config_dir: &Path, cfg: &SyncConfig) -> DesktopResult<()> {
    let path = config_dir.join("sync_config.json");
    let bytes = serde_json::to_vec_pretty(cfg)?;
    write_private_file_0600(&path, &bytes)?;
    Ok(())
}

/// Canonical JSON serializer (recursive sorted object keys) guaranteeing cross-platform
/// bit-identical `SHA-256` across macOS, Windows, and Linux.
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

pub fn canonical_sha256_value(val: &Value) -> DesktopResult<String> {
    let canon = canonicalize_json(val);
    let bytes = serde_json::to_vec(&canon)?;
    Ok(sha256_bytes_hex(&bytes))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SumiWorkspacePayloadV1 {
    pub kv_entries: Vec<KvRecord>,
    pub watchlists: Vec<WatchlistGroup>,
    pub layouts: Vec<SavedLayoutRecord>,
    pub pane_indicators: Vec<PaneIndicatorsRecord>,
    pub indicator_templates: Vec<IndicatorTemplateRecord>,
    pub drawings: Vec<SavedDrawingRecord>,
    pub alerts: Vec<PriceAlertRecord>,
    pub journal_annotations: Vec<JournalAnnotationRecord>,
    pub replay_sessions: Vec<SavedReplaySessionRecord>,
    #[serde(default)]
    pub saved_screens: Vec<SavedScreenRecord>,
    #[serde(default)]
    pub symbol_flags: Vec<SymbolFlagRecord>,
    pub tombstones: Vec<SyncTombstoneRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SumiWorkspaceBundleV1 {
    pub schema_version: String,
    pub device_id: String,
    pub exported_at_ms: i64,
    pub payload_sha256: String,
    pub payload: SumiWorkspacePayloadV1,
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

impl SumiWorkspaceBundleV1 {
    pub fn build_from_store(store: &WorkspaceStore, device_id: &str) -> DesktopResult<Self> {
        // Filter out any secret/ephemeral keys so credentials never enter sync bundles.
        let kv_entries: Vec<KvRecord> = store
            .list_all_kv()?
            .into_iter()
            .filter(|kv| !is_secret_kv_key(&kv.key))
            .collect();

        let payload = SumiWorkspacePayloadV1 {
            kv_entries,
            watchlists: store.list_watchlists()?,
            layouts: store.list_layouts()?,
            pane_indicators: store.list_all_pane_indicators()?,
            indicator_templates: store.list_indicator_templates()?,
            drawings: store.list_all_drawings()?,
            alerts: store.list_alerts()?,
            journal_annotations: store.list_all_journal_annotations()?,
            replay_sessions: store.list_replay_sessions()?,
            saved_screens: store.list_saved_screens()?,
            symbol_flags: store.list_symbol_flags()?,
            tombstones: store.list_tombstones()?,
        };
        let payload_val = serde_json::to_value(&payload)?;
        let payload_sha256 = canonical_sha256_value(&payload_val)?;
        Ok(Self {
            schema_version: SUMI_WORKSPACE_BUNDLE_SCHEMA_V1.to_owned(),
            device_id: device_id.to_owned(),
            exported_at_ms: Utc::now().timestamp_millis(),
            payload_sha256,
            payload,
        })
    }

    /// Fail-closed SHA-256 verification before touching any local SQLite state.
    pub fn verify_integrity(&self) -> DesktopResult<()> {
        if self.schema_version != SUMI_WORKSPACE_BUNDLE_SCHEMA_V1 {
            return Err(DesktopError::Validation(format!(
                "Phiên bản gói đồng bộ không hợp lệ: {}",
                self.schema_version
            )));
        }
        let payload_val = serde_json::to_value(&self.payload)?;
        let computed = canonical_sha256_value(&payload_val)?;
        if computed != self.payload_sha256.trim().to_ascii_lowercase() {
            return Err(DesktopError::Validation(format!(
                "Lỗi kiểm chứng toàn vẹn SHA-256 gói đồng bộ (mong đợi {}, thực tế {computed})",
                self.payload_sha256
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceMergeReport {
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
    #[serde(default)]
    pub merged_saved_screens: usize,
    #[serde(default)]
    pub merged_symbol_flags: usize,
    pub applied_tombstones: usize,
}

/// Rotates pre-sync backups in `<data_dir>/backups/`, keeping at most `MAX_PRE_SYNC_BACKUPS` (5) copies.
pub fn create_rotating_pre_sync_backup(
    workspace: &WorkspaceStore,
    data_dir: &Path,
) -> DesktopResult<Option<PathBuf>> {
    let src = workspace.path();
    if !src.is_file() {
        return Ok(None);
    }
    let backup_dir = data_dir.join("backups");
    fs::create_dir_all(&backup_dir)?;
    let ts = Utc::now().format("%Y%m%d_%H%M%S_%3f");
    let dst = backup_dir.join(format!("sumi_workspace_{ts}.pre_sync.bak"));
    sqlite_backup_atomic(src, &dst)?;

    // Prune oldest backups beyond MAX_PRE_SYNC_BACKUPS
    let mut entries: Vec<(std::time::SystemTime, PathBuf)> = fs::read_dir(&backup_dir)?
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            let name = p.file_name()?.to_str()?;
            if !name.ends_with(".pre_sync.bak") {
                return None;
            }
            let modified = e.metadata().ok()?.modified().ok()?;
            Some((modified, p))
        })
        .collect();
    entries.sort_by_key(|a| std::cmp::Reverse(a.0));
    for (_, old_path) in entries.into_iter().skip(MAX_PRE_SYNC_BACKUPS) {
        let _ = fs::remove_file(old_path);
    }
    Ok(Some(dst))
}

/// Merges an incoming verified `SumiWorkspaceBundleV1` into `workspace` using per-record
/// Last-Write-Wins (LWW) + Tombstone deletion semantics (deletions never resurrect!).
pub fn merge_bundle_into_store(
    workspace: &WorkspaceStore,
    bundle: &SumiWorkspaceBundleV1,
    data_dir: &Path,
) -> DesktopResult<WorkspaceMergeReport> {
    bundle.verify_integrity()?;
    let backup_path = create_rotating_pre_sync_backup(workspace, data_dir)?;

    let mut report = WorkspaceMergeReport {
        bundle_sha256: bundle.payload_sha256.clone(),
        source_device_id: bundle.device_id.clone(),
        backup_path: backup_path.map(|p| p.display().to_string()),
        ..WorkspaceMergeReport::default()
    };

    // 1. Merge tombstones first (taking max deleted_at_ms per entity)
    let mut tomb_map: HashMap<(String, String), i64> = HashMap::new();
    for t in workspace.list_tombstones()? {
        tomb_map.insert((t.entity_kind, t.entity_key), t.deleted_at_ms);
    }
    for t in &bundle.payload.tombstones {
        workspace.upsert_tombstone(t)?;
        let entry = tomb_map
            .entry((t.entity_kind.clone(), t.entity_key.clone()))
            .or_insert(0);
        if t.deleted_at_ms > *entry {
            *entry = t.deleted_at_ms;
        }
    }

    let is_tombstoned = |kind: &str, key: &str, record_ts: i64| -> bool {
        tomb_map
            .get(&(kind.to_owned(), key.to_owned()))
            .is_some_and(|&del_ts| del_ts >= record_ts)
    };

    // Apply incoming tombstones to delete stale local records cleanly
    for ((kind, key), del_ts) in &tomb_map {
        match kind.as_str() {
            "drawing" => {
                if let Some((ticker, id)) = key.split_once("::") {
                    let local = workspace.list_drawings(ticker)?;
                    if local.iter().any(|d| d.id == id && d.updated_at_ms <= *del_ts) {
                        workspace.delete_drawing(ticker, id)?;
                        report.applied_tombstones += 1;
                    }
                }
            }
            "layout" => {
                let local = workspace.list_layouts()?;
                if local.iter().any(|l| l.id == *key && l.updated_at_ms <= *del_ts) {
                    workspace.delete_layout(key)?;
                    report.applied_tombstones += 1;
                }
            }
            "watchlist" => {
                let local = workspace.list_watchlists()?;
                let recreated_newer_in_bundle = bundle
                    .payload
                    .watchlists
                    .iter()
                    .any(|w| w.id == *key && w.updated_at_ms > *del_ts);
                if !recreated_newer_in_bundle
                    && local
                        .iter()
                        .any(|w| w.id == *key && w.updated_at_ms <= *del_ts)
                {
                    workspace.delete_watchlist(key)?;
                    report.applied_tombstones += 1;
                }
            }
            "alert" => {
                let local = workspace.list_alerts()?;
                if local.iter().any(|a| a.id == *key && a.created_at_ms <= *del_ts) {
                    workspace.delete_alert(key)?;
                    report.applied_tombstones += 1;
                }
            }
            "indicator_template" => {
                let local = workspace.list_indicator_templates()?;
                if local.iter().any(|t| t.id == *key && t.updated_at_ms <= *del_ts) {
                    workspace.delete_indicator_template(key)?;
                    report.applied_tombstones += 1;
                }
            }
            "replay_session" => {
                if let Some(local) = workspace.get_replay_session(key)?
                    && local.updated_at_ms <= *del_ts
                {
                    workspace.delete_replay_session(key)?;
                    report.applied_tombstones += 1;
                }
            }
            "saved_screen" => {
                let local = workspace.list_saved_screens()?;
                if local.iter().any(|s| s.id == *key && s.updated_at_ms <= *del_ts) {
                    workspace.delete_screen(key)?;
                    report.applied_tombstones += 1;
                }
            }
            "symbol_flag" => {
                let local = workspace.list_symbol_flags()?;
                if local
                    .iter()
                    .any(|f| f.ticker == *key && f.updated_at_ms <= *del_ts)
                {
                    workspace.clear_symbol_flag(key)?;
                    report.applied_tombstones += 1;
                }
            }
            _ => {}
        }
    }

    // 2. Merge KV entries (excluding secrets)
    for kv in &bundle.payload.kv_entries {
        if is_secret_kv_key(&kv.key) {
            continue;
        }
        workspace.set_kv_with_ts(&kv.key, &kv.value, kv.updated_at_ms)?;
        report.merged_kv += 1;
    }

    // 3. Merge Watchlists (strict per-watchlist LWW using `wl.updated_at_ms`, NEVER `bundle.exported_at_ms`!)
    let existing_watchlists: HashMap<String, i64> = workspace
        .list_watchlists()?
        .into_iter()
        .map(|w| (w.id, w.updated_at_ms))
        .collect();
    for wl in &bundle.payload.watchlists {
        if is_tombstoned("watchlist", &wl.id, wl.updated_at_ms) {
            continue;
        }
        let local_ts = existing_watchlists.get(&wl.id).copied();
        if local_ts.is_none_or(|lt| wl.updated_at_ms >= lt)
            && workspace.upsert_watchlist_full(wl, wl.updated_at_ms)?
        {
            report.merged_watchlists += 1;
        }
    }

    // 4. Merge Layouts
    for lay in &bundle.payload.layouts {
        if is_tombstoned("layout", &lay.id, lay.updated_at_ms) {
            continue;
        }
        workspace.save_layout_with_ts(lay)?;
        report.merged_layouts += 1;
    }

    // 5. Merge Pane Indicators
    for pi in &bundle.payload.pane_indicators {
        workspace.save_pane_indicators_with_ts(&pi.slot_id, &pi.indicators, pi.updated_at_ms)?;
        report.merged_pane_indicators += 1;
    }

    // 6. Merge Indicator Templates
    for tpl in &bundle.payload.indicator_templates {
        if is_tombstoned("indicator_template", &tpl.id, tpl.updated_at_ms) {
            continue;
        }
        workspace.save_indicator_template_with_ts(tpl)?;
        report.merged_templates += 1;
    }

    // 7. Merge Drawings (LWW per `(ticker, id)`)
    let existing_drawings: HashMap<(String, String), i64> = workspace
        .list_all_drawings()?
        .into_iter()
        .map(|d| ((d.ticker, d.id), d.updated_at_ms))
        .collect();
    for d in &bundle.payload.drawings {
        let t_key = format!("{}::{}", d.ticker.trim().to_ascii_uppercase(), d.id);
        if is_tombstoned("drawing", &t_key, d.updated_at_ms) {
            continue;
        }
        let local_ts = existing_drawings
            .get(&(d.ticker.trim().to_ascii_uppercase(), d.id.clone()))
            .copied()
            .unwrap_or(i64::MIN);
        if d.updated_at_ms >= local_ts {
            workspace.upsert_drawing(d.clone())?;
            report.merged_drawings += 1;
        }
    }

    // 8. Merge Alerts
    let existing_alerts: HashMap<String, i64> = workspace
        .list_alerts()?
        .into_iter()
        .map(|a| (a.id, a.triggered_at_ms.unwrap_or(a.created_at_ms)))
        .collect();
    for a in &bundle.payload.alerts {
        let a_ts = a.triggered_at_ms.unwrap_or(a.created_at_ms);
        if is_tombstoned("alert", &a.id, a_ts) {
            continue;
        }
        let local_ts = existing_alerts.get(&a.id).copied().unwrap_or(i64::MIN);
        if a_ts >= local_ts {
            workspace.upsert_alert(a)?;
            report.merged_alerts += 1;
        }
    }

    // 9. Merge Journal Annotations
    for j in &bundle.payload.journal_annotations {
        workspace.upsert_journal_annotation_with_ts(j)?;
        report.merged_journal += 1;
    }

    // 10. Merge Replay Sessions
    for r in &bundle.payload.replay_sessions {
        if is_tombstoned("replay_session", &r.id, r.updated_at_ms) {
            continue;
        }
        workspace.save_replay_session_with_ts(r)?;
        report.merged_replay_sessions += 1;
    }

    // 11. Merge Saved Screener Presets (Wave 4)
    for sc in &bundle.payload.saved_screens {
        if is_tombstoned("saved_screen", &sc.id, sc.updated_at_ms) {
            continue;
        }
        workspace.save_screen_with_ts(sc)?;
        report.merged_saved_screens += 1;
    }

    // 12. Merge Symbol Color Flags (Wave 5)
    for fl in &bundle.payload.symbol_flags {
        if is_tombstoned("symbol_flag", &fl.ticker, fl.updated_at_ms) {
            continue;
        }
        workspace.set_symbol_flag_with_ts(fl)?;
        report.merged_symbol_flags += 1;
    }

    Ok(report)
}

/// Atomic file write (`tmp -> sync_all -> rename`) so iCloud / OneDrive / Dropbox never observe a half-written JSON.
pub fn write_bundle_atomic(path: &Path, bundle: &SumiWorkspaceBundleV1) -> DesktopResult<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension(format!(
        "sumi.tmp.{}",
        Utc::now().timestamp_nanos_opt().unwrap_or(0)
    ));
    let bytes = serde_json::to_vec_pretty(bundle)?;
    {
        let mut f = fs::File::create(&tmp)?;
        std::io::Write::write_all(&mut f, &bytes)?;
        f.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    Ok(())
}

pub fn read_bundle_verified(path: &Path) -> DesktopResult<SumiWorkspaceBundleV1> {
    let raw = fs::read_to_string(path)?;
    let bundle: SumiWorkspaceBundleV1 = serde_json::from_str(&raw)?;
    bundle.verify_integrity()?;
    Ok(bundle)
}

/// Abstract bidirectional sync transport (implemented by `LocalFolderSyncTransport` today
/// and pre-wired for `DoraemiSyncTransport` once Migration 0006 is applied).
pub trait SyncTransport {
    fn transport_name(&self) -> &'static str;
    fn read_remote_bundle(&self) -> DesktopResult<Option<SumiWorkspaceBundleV1>>;
    fn write_remote_bundle(&self, bundle: &SumiWorkspaceBundleV1) -> DesktopResult<String>;
}

/// Immediate zero-server Cloud Folder transport (`iCloud Drive`, `OneDrive`, `Dropbox`, `Syncthing`, `Git`).
pub struct LocalFolderSyncTransport {
    pub folder: PathBuf,
}

impl LocalFolderSyncTransport {
    #[must_use]
    pub fn bundle_path(&self) -> PathBuf {
        self.folder.join(DEFAULT_SYNC_FILENAME)
    }
}

impl SyncTransport for LocalFolderSyncTransport {
    fn transport_name(&self) -> &'static str {
        "local_cloud_folder"
    }

    fn read_remote_bundle(&self) -> DesktopResult<Option<SumiWorkspaceBundleV1>> {
        let p = self.bundle_path();
        if !p.is_file() {
            return Ok(None);
        }
        Ok(Some(read_bundle_verified(&p)?))
    }

    fn write_remote_bundle(&self, bundle: &SumiWorkspaceBundleV1) -> DesktopResult<String> {
        let p = self.bundle_path();
        write_bundle_atomic(&p, bundle)?;
        Ok(p.display().to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BidirectionalFolderSyncResult {
    pub bundle_path: String,
    pub pulled_and_merged: bool,
    pub merge_report: Option<WorkspaceMergeReport>,
    pub pushed_sha256: String,
    pub synced_at_ms: i64,
}

/// Performs full bidirectional sync (`Pull -> Verify SHA-256 -> Backup -> LWW Merge -> Export -> Atomic Push`)
/// against `folder` and updates `sync_config.json`.
pub fn sync_bidirectional_folder(
    workspace: &WorkspaceStore,
    data_dir: &Path,
    config_dir: &Path,
    folder: &Path,
) -> DesktopResult<BidirectionalFolderSyncResult> {
    let mut cfg = load_sync_config(config_dir);
    let transport = LocalFolderSyncTransport {
        folder: folder.to_path_buf(),
    };

    let mut merge_report = None;
    let mut pulled_and_merged = false;
    if let Some(remote_bundle) = transport.read_remote_bundle()? {
        merge_report = Some(merge_bundle_into_store(workspace, &remote_bundle, data_dir)?);
        pulled_and_merged = true;
    }

    let merged_local = SumiWorkspaceBundleV1::build_from_store(workspace, &cfg.device_id)?;
    let bundle_path = transport.write_remote_bundle(&merged_local)?;
    let now_ms = Utc::now().timestamp_millis();

    cfg.sync_folder = Some(folder.display().to_string());
    cfg.last_synced_at_ms = Some(now_ms);
    cfg.last_synced_sha256 = Some(merged_local.payload_sha256.clone());
    save_sync_config(config_dir, &cfg)?;

    Ok(BidirectionalFolderSyncResult {
        bundle_path,
        pulled_and_merged,
        merge_report,
        pushed_sha256: merged_local.payload_sha256,
        synced_at_ms: now_ms,
    })
}

/// Runs bidirectional Cloud Folder sync automatically on application startup when
/// `sync_config.json` has `auto_sync_on_launch = true` and a valid mounted folder (`W6-02`).
/// Never crashes startup if the cloud drive is temporarily unavailable.
pub fn maybe_run_auto_sync_on_launch(
    workspace: &WorkspaceStore,
    data_dir: &Path,
    config_dir: &Path,
) -> Option<BidirectionalFolderSyncResult> {
    let cfg = load_sync_config(config_dir);
    if !cfg.auto_sync_on_launch {
        return None;
    }
    let folder_str = cfg.sync_folder.as_deref()?.trim();
    if folder_str.is_empty() {
        return None;
    }
    let folder = PathBuf::from(folder_str);
    if !folder.is_dir() {
        tracing::warn!(
            folder = %folder.display(),
            "auto_sync_on_launch skipped: cloud sync folder is currently unavailable"
        );
        return None;
    }
    match sync_bidirectional_folder(workspace, data_dir, config_dir, &folder) {
        Ok(res) => {
            tracing::info!(
                bundle = %res.bundle_path,
                pulled = res.pulled_and_merged,
                sha256 = %res.pushed_sha256,
                "auto_sync_on_launch completed cleanly"
            );
            Some(res)
        }
        Err(err) => {
            tracing::warn!(
                folder = %folder.display(),
                error = %err,
                "auto_sync_on_launch failed (non-fatal)"
            );
            None
        }
    }
}

// ============================================================================
// Tier 2 Pre-Wired Transport: Doraemi OAuth 2.1 + PKCE (`S256`) `/sync/v1`
// ============================================================================

pub const DORAEMI_SUMI_WORKSPACE_ENTITY_KIND: &str = "sumi_workspace_bundle";
pub const DORAEMI_SUMI_KT01_VERDICT_ENTITY_KIND: &str = "sumi_kt01_verdict";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PkcePair {
    pub code_verifier: String,
    pub code_challenge: String,
    pub state: String,
}

/// Generates RFC 7636 S256 PKCE verifier, challenge, and OAuth state matching Yumeko's Doraemi flow.
#[must_use]
pub fn generate_doraemi_pkce_pair() -> PkcePair {
    let raw = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let code_verifier = URL_SAFE_NO_PAD.encode(raw.as_bytes());
    let mut hasher = Sha256::new();
    hasher.update(code_verifier.as_bytes());
    let code_challenge = URL_SAFE_NO_PAD.encode(hasher.finalize());
    let state = Uuid::new_v4().simple().to_string();
    PkcePair {
        code_verifier,
        code_challenge,
        state,
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DoraemiSyncCredentials {
    pub base_url: String,
    #[serde(default)]
    pub device_id: String,
    #[serde(default)]
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub expires_at: Option<String>,
    #[serde(default)]
    pub granted_scopes: Vec<String>,
}

impl std::fmt::Debug for DoraemiSyncCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DoraemiSyncCredentials")
            .field("base_url", &self.base_url)
            .field("device_id", &self.device_id)
            .field("access_token", &"[REDACTED]")
            .field(
                "refresh_token",
                &self.refresh_token.as_ref().map(|_| "[REDACTED]"),
            )
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DoraemiTransportStatusDto {
    pub configured: bool,
    pub source_file: Option<String>,
    pub base_url: Option<String>,
    pub migration_0006_required_note: String,
    pub ready_for_cloud_push: bool,
}

/// Inspects local Doraemi OAuth 2.1 credentials (0 network calls, zero secret disclosure),
/// checking `~/.config/sumi/doraemi_sync.json` first and falling back read-only to
/// `~/.config/yumeko/doraemi_sync.json` so Sumi immediately recognizes Yumeko's Doraemi setup.
#[must_use]
pub fn inspect_doraemi_transport_status() -> DoraemiTransportStatusDto {
    let sumi_path = sumi_config_dir().join("doraemi_sync.json");
    let yumeko_path = yumeko_config_dir().join("doraemi_sync.json");
    let candidate = if sumi_path.is_file() {
        Some(sumi_path)
    } else if yumeko_path.is_file() {
        Some(yumeko_path)
    } else {
        None
    };

    let note = "DoraemiSyncTransport (OAuth 2.1 + PKCE) đã cài sẵn trong Sumi. Khi áp dụng docs/architecture/DORAEMI_MIGRATION_0006_PROPOSAL.sql trên server Doraemi, Sumi có thể đẩy trực tiếp entity_kind='sumi_workspace_bundle' & 'sumi_kt01_verdict' lên /sync/v1.".to_owned();

    if let Some(p) = candidate
        && let Ok(raw) = fs::read_to_string(&p)
        && let Ok(val) = serde_json::from_str::<Value>(&raw)
    {
        let base_url = val
            .get("base_url")
            .and_then(Value::as_str)
            .map(str::to_owned);
        let has_token = val
            .get("access_token")
            .and_then(Value::as_str)
            .is_some_and(|s| !s.trim().is_empty())
            || val.get("ciphertext").is_some(); // Yumeko encrypted vault format
        return DoraemiTransportStatusDto {
            configured: base_url.is_some() || has_token,
            source_file: Some(p.display().to_string()),
            base_url,
            migration_0006_required_note: note,
            ready_for_cloud_push: false,
        };
    }

    DoraemiTransportStatusDto {
        configured: false,
        source_file: None,
        base_url: None,
        migration_0006_required_note: note,
        ready_for_cloud_push: false,
    }
}

/// Pre-wired async HTTP client for Doraemi `/sync/v1/{status,push,pull}` (`rustls-tls`, zero redirects).
#[derive(Clone)]
pub struct DoraemiSyncTransport {
    base_url: String,
    bearer_token: String,
    http: reqwest::Client,
}

impl DoraemiSyncTransport {
    pub fn new(base_url: &str, bearer_token: &str, timeout: Duration) -> DesktopResult<Self> {
        let normalized = base_url.trim().trim_end_matches('/').to_owned();
        if !(normalized.starts_with("http://") || normalized.starts_with("https://")) {
            return Err(DesktopError::Validation(
                "URL Doraemi phải bắt đầu bằng http:// hoặc https://".to_owned(),
            ));
        }
        let token = bearer_token.trim().to_owned();
        if token.is_empty() {
            return Err(DesktopError::Validation(
                "Thiếu Bearer token Doraemi OAuth 2.1".to_owned(),
            ));
        }
        let http = reqwest::Client::builder()
            .timeout(timeout)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| DesktopError::Internal(e.to_string()))?;
        Ok(Self {
            base_url: normalized,
            bearer_token: token,
            http,
        })
    }

    fn auth_headers(&self) -> DesktopResult<HeaderMap> {
        let mut h = HeaderMap::new();
        h.insert(ACCEPT, HeaderValue::from_static("application/json"));
        h.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        let val = HeaderValue::from_str(&format!("Bearer {}", self.bearer_token))
            .map_err(|_| DesktopError::Validation("Bearer token chứa ký tự không hợp lệ".to_owned()))?;
        h.insert(AUTHORIZATION, val);
        Ok(h)
    }

    /// Probes `GET /sync/v1/status` on Doraemi to inspect remote head revision.
    pub async fn probe_status(&self) -> DesktopResult<Value> {
        let url = format!("{}/sync/v1/status", self.base_url);
        let resp = self
            .http
            .get(&url)
            .headers(self.auth_headers()?)
            .send()
            .await
            .map_err(|e| DesktopError::Internal(format!("doraemi_sync_status_error: {e}")))?;
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| DesktopError::Internal(e.to_string()))?;
        if !status.is_success() {
            return Err(DesktopError::Internal(format!(
                "Doraemi /sync/v1/status HTTP {}: {}",
                status.as_u16(),
                text.chars().take(200).collect::<String>()
            )));
        }
        Ok(serde_json::from_str(&text)?)
    }

    /// Pushes a verified `SumiWorkspaceBundleV1` as `entity_kind = "sumi_workspace_bundle"`
    /// (requires `DORAEMI_MIGRATION_0006_PROPOSAL.sql` to be applied on Doraemi).
    pub async fn push_workspace_bundle(
        &self,
        device_id: &str,
        bundle: &SumiWorkspaceBundleV1,
    ) -> DesktopResult<Value> {
        bundle.verify_integrity()?;
        let url = format!("{}/sync/v1/push", self.base_url);
        let payload_json = serde_json::to_value(bundle)?;
        let payload_sha256 = canonical_sha256_value(&payload_json)?;
        let body = serde_json::json!({
            "device_id": device_id,
            "items": [{
                "entity_kind": DORAEMI_SUMI_WORKSPACE_ENTITY_KIND,
                "entity_key": "workspace:default",
                "as_of": Utc::now().format("%Y-%m-%d").to_string(),
                "payload_json": payload_json,
                "payload_sha256": payload_sha256,
            }]
        });
        let resp = self
            .http
            .post(&url)
            .headers(self.auth_headers()?)
            .json(&body)
            .send()
            .await
            .map_err(|e| DesktopError::Internal(format!("doraemi_sync_push_error: {e}")))?;
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| DesktopError::Internal(e.to_string()))?;
        if !status.is_success() {
            return Err(DesktopError::Internal(format!(
                "Doraemi /sync/v1/push HTTP {} (kiểm tra Migration 0006): {}",
                status.as_u16(),
                text.chars().take(240).collect::<String>()
            )));
        }
        Ok(serde_json::from_str(&text)?)
    }

    /// Pulls `entity_kind = "sumi_workspace_bundle"` items from Doraemi `/sync/v1/pull`.
    pub async fn pull_workspace_bundles(&self, since_seq: i64) -> DesktopResult<Value> {
        let mut parsed = url::Url::parse(&format!("{}/sync/v1/pull", self.base_url))
            .map_err(|e| DesktopError::Validation(e.to_string()))?;
        parsed
            .query_pairs_mut()
            .append_pair("since_seq", &since_seq.max(0).to_string())
            .append_pair("limit", "20")
            .append_pair("entity_kind", DORAEMI_SUMI_WORKSPACE_ENTITY_KIND);
        let resp = self
            .http
            .get(parsed)
            .headers(self.auth_headers()?)
            .send()
            .await
            .map_err(|e| DesktopError::Internal(format!("doraemi_sync_pull_error: {e}")))?;
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| DesktopError::Internal(e.to_string()))?;
        if !status.is_success() {
            return Err(DesktopError::Internal(format!(
                "Doraemi /sync/v1/pull HTTP {}: {}",
                status.as_u16(),
                text.chars().take(240).collect::<String>()
            )));
        }
        Ok(serde_json::from_str(&text)?)
    }
}

/// Returns current status summary for the UI Sync & Data modal.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatusDto {
    pub device_id: String,
    pub sync_folder: Option<String>,
    pub sync_bundle_exists: bool,
    pub auto_sync_on_launch: bool,
    pub last_synced_at_ms: Option<i64>,
    pub last_synced_sha256: Option<String>,
    pub data_dir: String,
    pub config_dir: String,
    pub pre_sync_backup_count: usize,
    pub doraemi_transport: DoraemiTransportStatusDto,
}

pub fn get_sync_status(data_dir: &Path, config_dir: &Path) -> SyncStatusDto {
    let cfg = load_sync_config(config_dir);
    let sync_bundle_exists = cfg
        .sync_folder
        .as_deref()
        .map(|f| Path::new(f).join(DEFAULT_SYNC_FILENAME).is_file())
        .unwrap_or(false);
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
    SyncStatusDto {
        device_id: cfg.device_id,
        sync_folder: cfg.sync_folder,
        sync_bundle_exists,
        auto_sync_on_launch: cfg.auto_sync_on_launch,
        last_synced_at_ms: cfg.last_synced_at_ms,
        last_synced_sha256: cfg.last_synced_sha256,
        data_dir: data_dir.display().to_string(),
        config_dir: config_dir.display().to_string(),
        pre_sync_backup_count,
        doraemi_transport: inspect_doraemi_transport_status(),
    }
}

/// Default path for 1-click bundle export (`<SUMI_DATA_DIR>/exports/sumi_workspace_<date>.sumi.json`).
#[must_use]
pub fn default_export_bundle_path() -> PathBuf {
    let ts = Utc::now().format("%Y%m%d_%H%M%S");
    sumi_data_dir()
        .join("exports")
        .join(format!("sumi_workspace_{ts}.sumi.json"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn mac_to_clean_slate_windows_sync_preserves_all_entities_and_propagates_tombstones() {
        let tmp = tempfile::tempdir().unwrap();
        let mac_data = tmp.path().join("mac_data");
        let mac_cfg = tmp.path().join("mac_cfg");
        let win_data = tmp.path().join("win_data");
        let win_cfg = tmp.path().join("win_cfg");
        let cloud_folder = tmp.path().join("icloud_sync");

        let mac_ws = WorkspaceStore::open(mac_data.join("sumi_workspace.db")).unwrap();
        let win_ws = WorkspaceStore::open(win_data.join("sumi_workspace.db")).unwrap();

        // Mac creates custom watchlist, drawing, layout, alert, journal note, and replay session
        mac_ws.create_watchlist("wl_custom", "Săn Cổ Phiếu QMV").unwrap();
        mac_ws.add_watchlist_symbol("wl_custom", "HOSE:FPT").unwrap();
        mac_ws
            .upsert_drawing(SavedDrawingRecord {
                id: "draw_avwap_1".to_owned(),
                ticker: "HOSE:FPT".to_owned(),
                name: "segment".to_owned(),
                pane_id: "candle_pane".to_owned(),
                group_id: "user_drawings".to_owned(),
                points: json!([{"timestamp": 1_700_000_000_000_i64, "value": 125_000.0}]),
                styles: None,
                extend_data: None,
                lock: false,
                visible: true,
                mode: "weak_magnet".to_owned(),
                updated_at_ms: Utc::now().timestamp_millis() - 5_000,
            })
            .unwrap();
        mac_ws
            .save_layout("ly_mtf", "Quad MTF", &json!({"gridLayout": "4"}))
            .unwrap();
        mac_ws
            .upsert_journal_annotation("HOSE:FPT", 7, "Pullback", "", "Tuân thủ SL chuẩn")
            .unwrap();

        // Mac pushes to Cloud Sync Folder
        let mac_sync = sync_bidirectional_folder(&mac_ws, &mac_data, &mac_cfg, &cloud_folder).unwrap();
        assert!(!mac_sync.pulled_and_merged);
        assert_eq!(mac_sync.pushed_sha256.len(), 64);

        // Clean-Slate Windows PC pulls & merges from Cloud Sync Folder
        let win_sync = sync_bidirectional_folder(&win_ws, &win_data, &win_cfg, &cloud_folder).unwrap();
        assert!(win_sync.pulled_and_merged);
        assert!(win_ws.list_watchlists().unwrap().iter().any(|w| w.id == "wl_custom"));
        assert_eq!(win_ws.list_drawings("HOSE:FPT").unwrap().len(), 1);
        assert_eq!(win_ws.list_layouts().unwrap().len(), 1);
        assert_eq!(win_ws.list_journal_annotations("HOSE:FPT").unwrap().len(), 1);

        // Windows deletes `draw_avwap_1` -> tombstone propagates back to Mac without resurrection!
        win_ws.delete_drawing("HOSE:FPT", "draw_avwap_1").unwrap();
        sync_bidirectional_folder(&win_ws, &win_data, &win_cfg, &cloud_folder).unwrap();

        let mac_sync_2 = sync_bidirectional_folder(&mac_ws, &mac_data, &mac_cfg, &cloud_folder).unwrap();
        assert!(mac_sync_2.pulled_and_merged);
        assert!(
            mac_ws.list_drawings("HOSE:FPT").unwrap().is_empty(),
            "Deleted drawing on Windows must NOT resurrect on Mac!"
        );
    }

    #[test]
    fn tampered_bundle_fails_closed_on_sha256_mismatch_and_rotates_5_backups() {
        let tmp = tempfile::tempdir().unwrap();
        let data_dir = tmp.path().join("data");
        let ws = WorkspaceStore::open(data_dir.join("sumi_workspace.db")).unwrap();

        let mut bundle = SumiWorkspaceBundleV1::build_from_store(&ws, "mac-test").unwrap();
        assert!(bundle.verify_integrity().is_ok());

        // Tamper with payload without updating SHA-256 -> must fail closed!
        bundle.payload.layouts.push(SavedLayoutRecord {
            id: "evil".to_owned(),
            name: "Tampered".to_owned(),
            layout_json: json!({}),
            updated_at_ms: 999,
        });
        assert!(bundle.verify_integrity().is_err());

        // Verify backup rotation keeps at most 5 `.pre_sync.bak` files
        for _ in 0..7 {
            std::thread::sleep(std::time::Duration::from_millis(5));
            let _ = create_rotating_pre_sync_backup(&ws, &data_dir).unwrap();
        }
        let status = get_sync_status(&data_dir, tmp.path());
        assert_eq!(status.pre_sync_backup_count, MAX_PRE_SYNC_BACKUPS);

        // PKCE S256 pair sanity
        let pkce = generate_doraemi_pkce_pair();
        assert!(pkce.code_verifier.len() >= 43);
        assert!(!pkce.code_challenge.is_empty());
        assert!(!pkce.state.is_empty());
    }

    #[test]
    fn wave6_watchlist_lww_preserves_machine_a_edits_and_unicode_section_headers() {
        let tmp = tempfile::tempdir().unwrap();
        let mac_data = tmp.path().join("mac_data");
        let mac_cfg = tmp.path().join("mac_cfg");
        let win_data = tmp.path().join("win_data");
        let win_cfg = tmp.path().join("win_cfg");
        let cloud_folder = tmp.path().join("cloud_sync");

        let mac_ws = WorkspaceStore::open(mac_data.join("sumi_workspace.db")).unwrap();
        let win_ws = WorkspaceStore::open(win_data.join("sumi_workspace.db")).unwrap();

        // T1: Initial shared watchlist synced to both Machine A (Mac) and Machine B (Windows)
        mac_ws.create_watchlist("wl_focus", "Rổ Trọng Điểm").unwrap();
        mac_ws.add_watchlist_symbol("wl_focus", "###SECTION:Ngân Hàng").unwrap();
        mac_ws.add_watchlist_symbol("wl_focus", "HOSE:VCB").unwrap();
        sync_bidirectional_folder(&mac_ws, &mac_data, &mac_cfg, &cloud_folder).unwrap();
        sync_bidirectional_folder(&win_ws, &win_data, &win_cfg, &cloud_folder).unwrap();

        // Verify Unicode section header casing survived sync intact on Machine B
        let win_wl_t1 = win_ws
            .list_watchlists()
            .unwrap()
            .into_iter()
            .find(|w| w.id == "wl_focus")
            .unwrap();
        assert_eq!(
            win_wl_t1.tickers,
            vec!["###SECTION:Ngân Hàng".to_owned(), "HOSE:VCB".to_owned()]
        );

        // T2: Machine A adds HOSE:MBB to watchlist (newer watchlist updated_at_ms)
        std::thread::sleep(std::time::Duration::from_millis(12));
        mac_ws.add_watchlist_symbol("wl_focus", "HOSE:MBB").unwrap();

        // T3 > T2: Machine B ONLY draws a trendline (does NOT touch watchlist), then exports/pushes!
        std::thread::sleep(std::time::Duration::from_millis(12));
        win_ws
            .upsert_drawing(SavedDrawingRecord {
                id: "draw_win_only".to_owned(),
                ticker: "HOSE:VCB".to_owned(),
                name: "segment".to_owned(),
                pane_id: "candle_pane".to_owned(),
                group_id: "user_drawings".to_owned(),
                points: json!([{"timestamp": 1_700_000_000_000_i64, "value": 92_000.0}]),
                styles: None,
                extend_data: None,
                lock: false,
                visible: true,
                mode: "normal".to_owned(),
                updated_at_ms: Utc::now().timestamp_millis(),
            })
            .unwrap();
        // Machine B pushes at T3 with stale watchlist updated_at_ms == T1 < T2
        sync_bidirectional_folder(&win_ws, &win_data, &win_cfg, &cloud_folder).unwrap();

        // Machine A pulls Machine B's bundle (exported_at_ms == T3) -> MUST NOT clobber HOSE:MBB!
        sync_bidirectional_folder(&mac_ws, &mac_data, &mac_cfg, &cloud_folder).unwrap();
        let mac_wl_after = mac_ws
            .list_watchlists()
            .unwrap()
            .into_iter()
            .find(|w| w.id == "wl_focus")
            .unwrap();
        assert_eq!(
            mac_wl_after.tickers,
            vec![
                "###SECTION:Ngân Hàng".to_owned(),
                "HOSE:VCB".to_owned(),
                "HOSE:MBB".to_owned()
            ],
            "Machine B's older watchlist must NOT overwrite Machine A's newer watchlist edit!"
        );
        assert_eq!(
            mac_ws.list_drawings("HOSE:VCB").unwrap().len(),
            1,
            "Machine A still receives Machine B's newer drawing!"
        );

        // Verify `auto_sync_on_launch` executes on Machine B and pulls HOSE:MBB automatically
        let mut win_sync_cfg = load_sync_config(&win_cfg);
        win_sync_cfg.auto_sync_on_launch = true;
        win_sync_cfg.sync_folder = Some(cloud_folder.display().to_string());
        save_sync_config(&win_cfg, &win_sync_cfg).unwrap();

        let auto_res = maybe_run_auto_sync_on_launch(&win_ws, &win_data, &win_cfg);
        assert!(auto_res.is_some());
        let win_wl_after = win_ws
            .list_watchlists()
            .unwrap()
            .into_iter()
            .find(|w| w.id == "wl_focus")
            .unwrap();
        assert!(win_wl_after.tickers.contains(&"HOSE:MBB".to_owned()));

        // Verify removing a Unicode section header works cleanly
        mac_ws
            .remove_watchlist_symbol("wl_focus", "###SECTION:Ngân Hàng")
            .unwrap();
        let after_rm = mac_ws
            .list_watchlists()
            .unwrap()
            .into_iter()
            .find(|w| w.id == "wl_focus")
            .unwrap();
        assert!(!after_rm.tickers.iter().any(|t| t.starts_with("###SECTION:")));

        // Verify sensitive KV keys are stripped from sync bundles
        mac_ws
            .set_kv("oauth_session_cookie", &json!({"api_key": "secret"}))
            .unwrap();
        let bundle = SumiWorkspaceBundleV1::build_from_store(&mac_ws, "mac-test").unwrap();
        assert!(
            !bundle
                .payload
                .kv_entries
                .iter()
                .any(|kv| kv.key == "oauth_session_cookie")
        );
    }
}
