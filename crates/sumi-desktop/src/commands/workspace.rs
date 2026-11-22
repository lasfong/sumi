use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::State;

use crate::error::{DesktopError, DesktopResult};
use crate::state::AppState;
use crate::workspace_db::{
    PriceAlertRecord, SavedDrawingRecord, SavedIndicatorSpec, SavedLayoutRecord, WatchlistGroup,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceBootstrapResponse {
    pub settings: Option<Value>,
    pub layouts: Vec<SavedLayoutRecord>,
    pub watchlists: Vec<WatchlistGroup>,
    pub alerts: Vec<PriceAlertRecord>,
}

#[tauri::command]
pub async fn load_workspace_bootstrap(
    state: State<'_, AppState>,
) -> DesktopResult<WorkspaceBootstrapResponse> {
    let ws = state.workspace.clone();
    tokio::task::spawn_blocking(move || {
        Ok(WorkspaceBootstrapResponse {
            settings: ws.get_kv("workspace_settings")?,
            layouts: ws.list_layouts()?,
            watchlists: ws.list_watchlists()?,
            alerts: ws.list_alerts()?,
        })
    })
    .await
    .map_err(|e| DesktopError::Internal(e.to_string()))?
}

#[tauri::command]
pub async fn save_workspace_settings(
    state: State<'_, AppState>,
    settings: Value,
) -> DesktopResult<()> {
    let ws = state.workspace.clone();
    tokio::task::spawn_blocking(move || ws.set_kv("workspace_settings", &settings))
        .await
        .map_err(|e| DesktopError::Internal(e.to_string()))?
}

#[tauri::command]
pub async fn get_drawings(
    state: State<'_, AppState>,
    ticker: String,
) -> DesktopResult<Vec<SavedDrawingRecord>> {
    let ws = state.workspace.clone();
    tokio::task::spawn_blocking(move || ws.list_drawings(&ticker))
        .await
        .map_err(|e| DesktopError::Internal(e.to_string()))?
}

#[tauri::command]
pub async fn upsert_drawing(
    state: State<'_, AppState>,
    drawing: SavedDrawingRecord,
) -> DesktopResult<()> {
    let ws = state.workspace.clone();
    tokio::task::spawn_blocking(move || ws.upsert_drawing(drawing))
        .await
        .map_err(|e| DesktopError::Internal(e.to_string()))?
}

#[tauri::command]
pub async fn delete_drawing(
    state: State<'_, AppState>,
    ticker: String,
    id: String,
) -> DesktopResult<()> {
    let ws = state.workspace.clone();
    tokio::task::spawn_blocking(move || ws.delete_drawing(&ticker, &id))
        .await
        .map_err(|e| DesktopError::Internal(e.to_string()))?
}

#[tauri::command]
pub async fn clear_drawings(state: State<'_, AppState>, ticker: String) -> DesktopResult<()> {
    let ws = state.workspace.clone();
    tokio::task::spawn_blocking(move || ws.clear_drawings(&ticker))
        .await
        .map_err(|e| DesktopError::Internal(e.to_string()))?
}

#[tauri::command]
pub async fn get_pane_indicators(
    state: State<'_, AppState>,
    slot_id: String,
) -> DesktopResult<Vec<SavedIndicatorSpec>> {
    let ws = state.workspace.clone();
    tokio::task::spawn_blocking(move || ws.get_pane_indicators(&slot_id))
        .await
        .map_err(|e| DesktopError::Internal(e.to_string()))?
}

#[tauri::command]
pub async fn save_pane_indicators(
    state: State<'_, AppState>,
    slot_id: String,
    indicators: Vec<SavedIndicatorSpec>,
) -> DesktopResult<()> {
    let ws = state.workspace.clone();
    tokio::task::spawn_blocking(move || ws.save_pane_indicators(&slot_id, &indicators))
        .await
        .map_err(|e| DesktopError::Internal(e.to_string()))?
}

#[tauri::command]
pub async fn list_layouts(state: State<'_, AppState>) -> DesktopResult<Vec<SavedLayoutRecord>> {
    let ws = state.workspace.clone();
    tokio::task::spawn_blocking(move || ws.list_layouts())
        .await
        .map_err(|e| DesktopError::Internal(e.to_string()))?
}

#[tauri::command]
pub async fn save_layout(
    state: State<'_, AppState>,
    id: String,
    name: String,
    layout_json: Value,
) -> DesktopResult<SavedLayoutRecord> {
    let ws = state.workspace.clone();
    tokio::task::spawn_blocking(move || ws.save_layout(&id, &name, &layout_json))
        .await
        .map_err(|e| DesktopError::Internal(e.to_string()))?
}

#[tauri::command]
pub async fn delete_layout(state: State<'_, AppState>, id: String) -> DesktopResult<()> {
    let ws = state.workspace.clone();
    tokio::task::spawn_blocking(move || ws.delete_layout(&id))
        .await
        .map_err(|e| DesktopError::Internal(e.to_string()))?
}

#[tauri::command]
pub async fn list_watchlists(state: State<'_, AppState>) -> DesktopResult<Vec<WatchlistGroup>> {
    let ws = state.workspace.clone();
    tokio::task::spawn_blocking(move || ws.list_watchlists())
        .await
        .map_err(|e| DesktopError::Internal(e.to_string()))?
}

#[tauri::command]
pub async fn create_watchlist(
    state: State<'_, AppState>,
    id: String,
    name: String,
) -> DesktopResult<WatchlistGroup> {
    let ws = state.workspace.clone();
    tokio::task::spawn_blocking(move || ws.create_watchlist(&id, &name))
        .await
        .map_err(|e| DesktopError::Internal(e.to_string()))?
}

#[tauri::command]
pub async fn delete_watchlist(state: State<'_, AppState>, id: String) -> DesktopResult<()> {
    let ws = state.workspace.clone();
    tokio::task::spawn_blocking(move || ws.delete_watchlist(&id))
        .await
        .map_err(|e| DesktopError::Internal(e.to_string()))?
}

#[tauri::command]
pub async fn add_watchlist_symbol(
    state: State<'_, AppState>,
    watchlist_id: String,
    ticker: String,
) -> DesktopResult<()> {
    let ws = state.workspace.clone();
    tokio::task::spawn_blocking(move || ws.add_watchlist_symbol(&watchlist_id, &ticker))
        .await
        .map_err(|e| DesktopError::Internal(e.to_string()))?
}

#[tauri::command]
pub async fn remove_watchlist_symbol(
    state: State<'_, AppState>,
    watchlist_id: String,
    ticker: String,
) -> DesktopResult<()> {
    let ws = state.workspace.clone();
    tokio::task::spawn_blocking(move || ws.remove_watchlist_symbol(&watchlist_id, &ticker))
        .await
        .map_err(|e| DesktopError::Internal(e.to_string()))?
}

#[tauri::command]
pub async fn list_alerts(state: State<'_, AppState>) -> DesktopResult<Vec<PriceAlertRecord>> {
    let ws = state.workspace.clone();
    tokio::task::spawn_blocking(move || ws.list_alerts())
        .await
        .map_err(|e| DesktopError::Internal(e.to_string()))?
}

#[tauri::command]
pub async fn create_alert(
    state: State<'_, AppState>,
    ticker: String,
    condition: String,
    target_price: f64,
    message: String,
) -> DesktopResult<PriceAlertRecord> {
    if !target_price.is_finite() || target_price <= 0.0 {
        return Err(DesktopError::Validation(
            "Giá mục tiêu cảnh báo phải là số dương hợp lệ".to_owned(),
        ));
    }
    let now_ms = chrono::Utc::now().timestamp_millis();
    let record = PriceAlertRecord {
        id: format!("alert_{now_ms}"),
        ticker: ticker.trim().to_ascii_uppercase(),
        condition,
        target_price,
        message,
        status: "armed".to_owned(),
        created_at_ms: now_ms,
        triggered_at_ms: None,
    };
    let ws = state.workspace.clone();
    let saved = record.clone();
    tokio::task::spawn_blocking(move || ws.upsert_alert(&saved))
        .await
        .map_err(|e| DesktopError::Internal(e.to_string()))??;
    Ok(record)
}

#[tauri::command]
pub async fn toggle_alert_status(
    state: State<'_, AppState>,
    id: String,
    status: String,
) -> DesktopResult<()> {
    let ws = state.workspace.clone();
    tokio::task::spawn_blocking(move || ws.set_alert_status(&id, &status, None))
        .await
        .map_err(|e| DesktopError::Internal(e.to_string()))?
}

#[tauri::command]
pub async fn delete_alert(state: State<'_, AppState>, id: String) -> DesktopResult<()> {
    let ws = state.workspace.clone();
    tokio::task::spawn_blocking(move || ws.delete_alert(&id))
        .await
        .map_err(|e| DesktopError::Internal(e.to_string()))?
}

#[tauri::command]
pub async fn check_price_alerts(
    state: State<'_, AppState>,
    ticker: String,
    prev_price: f64,
    current_price: f64,
) -> DesktopResult<Vec<PriceAlertRecord>> {
    let ws = state.workspace.clone();
    let norm = ticker.trim().to_ascii_uppercase();
    tokio::task::spawn_blocking(move || {
        let alerts = ws.list_alerts()?;
        let now_ms = chrono::Utc::now().timestamp_millis();
        let mut fired = Vec::new();
        for mut a in alerts {
            if a.ticker != norm || a.status != "armed" {
                continue;
            }
            let crossed = match a.condition.as_str() {
                "cross_above" => prev_price < a.target_price && current_price >= a.target_price,
                "cross_below" => prev_price > a.target_price && current_price <= a.target_price,
                _ => {
                    (prev_price < a.target_price && current_price >= a.target_price)
                        || (prev_price > a.target_price && current_price <= a.target_price)
                }
            };
            if crossed {
                a.status = "triggered".to_owned();
                a.triggered_at_ms = Some(now_ms);
                ws.set_alert_status(&a.id, "triggered", Some(now_ms))?;
                fired.push(a);
            }
        }
        Ok(fired)
    })
    .await
    .map_err(|e| DesktopError::Internal(e.to_string()))?
}
