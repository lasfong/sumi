use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::error::{DesktopError, DesktopResult};
use crate::state::AppState;
use crate::sync::{
    BidirectionalFolderSyncResult, DoraemiTransportStatusDto, SumiWorkspaceBundleV1, SyncStatusDto,
    WorkspaceMergeReport, default_export_bundle_path, get_sync_status,
    inspect_doraemi_transport_status, load_sync_config, merge_bundle_into_store,
    read_bundle_verified, save_sync_config, sync_bidirectional_folder, write_bundle_atomic,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportBundleResponse {
    pub path: String,
    pub payload_sha256: String,
    pub exported_at_ms: i64,
    pub watchlists_count: usize,
    pub drawings_count: usize,
    pub layouts_count: usize,
    pub alerts_count: usize,
}

#[tauri::command]
pub async fn sync_get_status(state: State<'_, AppState>) -> DesktopResult<SyncStatusDto> {
    let data_dir = state.data_dir.clone();
    let config_dir = state.config_dir.clone();
    tokio::task::spawn_blocking(move || get_sync_status(&data_dir, &config_dir))
        .await
        .map_err(|e| DesktopError::Internal(e.to_string()))
}

#[tauri::command]
pub async fn sync_export_bundle(
    state: State<'_, AppState>,
    out_path: Option<String>,
) -> DesktopResult<ExportBundleResponse> {
    let ws = state.workspace.clone();
    let config_dir = state.config_dir.clone();
    tokio::task::spawn_blocking(move || {
        let cfg = load_sync_config(&config_dir);
        let bundle = SumiWorkspaceBundleV1::build_from_store(&ws, &cfg.device_id)?;
        let target = out_path
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(default_export_bundle_path);
        write_bundle_atomic(&target, &bundle)?;
        Ok(ExportBundleResponse {
            path: target.display().to_string(),
            payload_sha256: bundle.payload_sha256,
            exported_at_ms: bundle.exported_at_ms,
            watchlists_count: bundle.payload.watchlists.len(),
            drawings_count: bundle.payload.drawings.len(),
            layouts_count: bundle.payload.layouts.len(),
            alerts_count: bundle.payload.alerts.len(),
        })
    })
    .await
    .map_err(|e| DesktopError::Internal(e.to_string()))?
}

#[tauri::command]
pub async fn sync_import_bundle(
    state: State<'_, AppState>,
    bundle_path: String,
) -> DesktopResult<WorkspaceMergeReport> {
    let ws = state.workspace.clone();
    let data_dir = state.data_dir.clone();
    tokio::task::spawn_blocking(move || {
        let path = PathBuf::from(bundle_path.trim());
        if !path.is_file() {
            return Err(DesktopError::Validation(format!(
                "Không tìm thấy tệp gói đồng bộ: {}",
                path.display()
            )));
        }
        let bundle = read_bundle_verified(&path)?;
        merge_bundle_into_store(&ws, &bundle, &data_dir)
    })
    .await
    .map_err(|e| DesktopError::Internal(e.to_string()))?
}

#[tauri::command]
pub async fn sync_configure_folder(
    state: State<'_, AppState>,
    folder_path: Option<String>,
    device_id: Option<String>,
    auto_sync_on_launch: Option<bool>,
) -> DesktopResult<SyncStatusDto> {
    let data_dir = state.data_dir.clone();
    let config_dir = state.config_dir.clone();
    tokio::task::spawn_blocking(move || {
        let mut cfg = load_sync_config(&config_dir);
        if let Some(f) = folder_path {
            let trimmed = f.trim();
            cfg.sync_folder = if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_owned())
            };
        }
        if let Some(dev) = device_id
            && !dev.trim().is_empty()
        {
            cfg.device_id = dev.trim().to_owned();
        }
        if let Some(auto) = auto_sync_on_launch {
            cfg.auto_sync_on_launch = auto;
        }
        save_sync_config(&config_dir, &cfg)?;
        Ok(get_sync_status(&data_dir, &config_dir))
    })
    .await
    .map_err(|e| DesktopError::Internal(e.to_string()))?
}

#[tauri::command]
pub async fn sync_folder_now(
    state: State<'_, AppState>,
    folder_path: Option<String>,
) -> DesktopResult<BidirectionalFolderSyncResult> {
    let ws = state.workspace.clone();
    let data_dir = state.data_dir.clone();
    let config_dir = state.config_dir.clone();
    tokio::task::spawn_blocking(move || {
        let cfg = load_sync_config(&config_dir);
        let folder = folder_path
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .or_else(|| cfg.sync_folder.map(PathBuf::from))
            .ok_or_else(|| {
                DesktopError::Validation(
                    "Chưa cấu hình thư mục đồng bộ đám mây (iCloud / OneDrive / Dropbox / Syncthing)."
                        .to_owned(),
                )
            })?;
        sync_bidirectional_folder(&ws, &data_dir, &config_dir, &folder)
    })
    .await
    .map_err(|e| DesktopError::Internal(e.to_string()))?
}

#[tauri::command]
pub async fn sync_get_doraemi_status() -> DesktopResult<DoraemiTransportStatusDto> {
    Ok(inspect_doraemi_transport_status())
}
