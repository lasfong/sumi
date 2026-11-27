use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sumi_core::alerts::{CrossDirection, evaluate_4_waypoint_cross};
use sumi_core::indicators::{compute_ema_slices, compute_macd_slices, compute_rsi_slices};
use sumi_core::model::Candle;
use sumi_core::scanner::{
    ConfluenceWeightProfile, PrecomputedCatalogMatrix, SIGNAL_CATALOG, SignalOutputType,
    catalog_entry_index,
};
use sumi_data::{
    Adjustment, BarQueryMode, Interval, OhlcvBar, SeriesKey, SymbolKey, sumi_config_dir,
    write_private_file_0600,
};
use tauri::State;

use crate::error::{DesktopError, DesktopResult};
use crate::state::AppState;
use crate::workspace_db::{
    AlertTriggerLogRecord, IndicatorTemplateRecord, PriceAlertRecord, SavedDrawingRecord,
    SavedIndicatorSpec, SavedLayoutRecord, SavedReplaySessionRecord, WatchlistGroup,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceBootstrapResponse {
    pub settings: Option<Value>,
    pub layouts: Vec<SavedLayoutRecord>,
    pub watchlists: Vec<WatchlistGroup>,
    pub alerts: Vec<PriceAlertRecord>,
    pub indicator_templates: Vec<IndicatorTemplateRecord>,
    pub replay_sessions: Vec<SavedReplaySessionRecord>,
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
            indicator_templates: ws.list_indicator_templates()?,
            replay_sessions: ws.list_replay_sessions()?,
        })
    })
    .await
    .map_err(|e| DesktopError::Internal(e.to_string()))?
}

// ─── Indicator Templates (v2.2) ──────────────────────────────────────────────

#[tauri::command]
pub async fn list_indicator_templates(
    state: State<'_, AppState>,
) -> DesktopResult<Vec<IndicatorTemplateRecord>> {
    let ws = state.workspace.clone();
    tokio::task::spawn_blocking(move || ws.list_indicator_templates())
        .await
        .map_err(|e| DesktopError::Internal(e.to_string()))?
}

#[tauri::command]
pub async fn save_indicator_template(
    state: State<'_, AppState>,
    id: Option<String>,
    name: String,
    description: Option<String>,
    indicators: Vec<SavedIndicatorSpec>,
) -> DesktopResult<IndicatorTemplateRecord> {
    let clean_name = name.trim();
    if clean_name.is_empty() {
        return Err(DesktopError::Validation(
            "Tên mẫu chỉ báo không được để trống".to_owned(),
        ));
    }
    if indicators.is_empty() {
        return Err(DesktopError::Validation(
            "Biểu đồ hiện chưa có chỉ báo nào để lưu thành mẫu".to_owned(),
        ));
    }
    let now_ms = chrono::Utc::now().timestamp_millis();
    let tpl_id = id
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| format!("tpl_{now_ms}"));
    let record = IndicatorTemplateRecord {
        id: tpl_id,
        name: clean_name.to_owned(),
        description: description.unwrap_or_default().trim().to_owned(),
        indicators,
        updated_at_ms: now_ms,
    };
    let ws = state.workspace.clone();
    tokio::task::spawn_blocking(move || ws.save_indicator_template(&record))
        .await
        .map_err(|e| DesktopError::Internal(e.to_string()))?
}

#[tauri::command]
pub async fn delete_indicator_template(
    state: State<'_, AppState>,
    id: String,
) -> DesktopResult<()> {
    let ws = state.workspace.clone();
    tokio::task::spawn_blocking(move || ws.delete_indicator_template(&id))
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
    let trimmed_name = name.trim().to_owned();
    if trimmed_name.is_empty() {
        return Err(DesktopError::Validation(
            "Tên danh mục theo dõi không được để trống".to_owned(),
        ));
    }
    let trimmed_id = id.trim().to_owned();
    if trimmed_id.is_empty() {
        return Err(DesktopError::Validation(
            "Mã định danh danh mục không được để trống".to_owned(),
        ));
    }
    let ws = state.workspace.clone();
    tokio::task::spawn_blocking(move || ws.create_watchlist(&trimmed_id, &trimmed_name))
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
#[allow(clippy::too_many_arguments)]
pub async fn create_alert(
    state: State<'_, AppState>,
    ticker: String,
    condition: String,
    target_price: f64,
    message: String,
    alert_kind: Option<String>,
    interval: Option<String>,
    params_json: Option<Value>,
    sound_enabled: Option<bool>,
    frequency: Option<String>,
    cooldown_bars: Option<i32>,
) -> DesktopResult<PriceAlertRecord> {
    let kind = alert_kind
        .as_deref()
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| matches!(s.as_str(), "price" | "drawing" | "indicator"))
        .unwrap_or_else(|| "price".to_owned());

    if kind == "price" && (!target_price.is_finite() || target_price <= 0.0) {
        return Err(DesktopError::Validation(
            "Giá mục tiêu cảnh báo phải là số dương hợp lệ".to_owned(),
        ));
    }
    if !target_price.is_finite() {
        return Err(DesktopError::Validation(
            "Ngưỡng cảnh báo không hợp lệ".to_owned(),
        ));
    }
    let freq = frequency
        .as_deref()
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| matches!(s.as_str(), "once" | "once_per_bar_close"))
        .unwrap_or_else(|| "once".to_owned());
    let now_ms = chrono::Utc::now().timestamp_millis();
    let seq = state
        .alert_seq
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let record = PriceAlertRecord {
        id: format!("alert_{now_ms}_{seq}"),
        ticker: ticker.trim().to_ascii_uppercase(),
        alert_kind: kind,
        condition,
        target_price,
        interval,
        params_json,
        sound_enabled: sound_enabled.unwrap_or(true),
        message,
        status: "armed".to_owned(),
        frequency: freq,
        cooldown_bars: cooldown_bars.unwrap_or(0).clamp(0, 500),
        last_fired_bar_ts: None,
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

/// Updates an existing alert (condition, target, message, frequency, cooldown, params, sound) and re-arms it.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn update_alert(
    state: State<'_, AppState>,
    id: String,
    condition: Option<String>,
    target_price: Option<f64>,
    message: Option<String>,
    interval: Option<String>,
    params_json: Option<Value>,
    sound_enabled: Option<bool>,
    frequency: Option<String>,
    cooldown_bars: Option<i32>,
    rearm: Option<bool>,
) -> DesktopResult<PriceAlertRecord> {
    let ws = state.workspace.clone();
    tokio::task::spawn_blocking(move || {
        let mut alert = ws
            .list_alerts()?
            .into_iter()
            .find(|a| a.id == id)
            .ok_or_else(|| DesktopError::Validation(format!("Không tìm thấy cảnh báo {id}")))?;
        if let Some(c) = condition {
            alert.condition = c;
        }
        if let Some(tp) = target_price {
            if !tp.is_finite() {
                return Err(DesktopError::Validation("Ngưỡng giá không hợp lệ".to_owned()));
            }
            alert.target_price = tp;
        }
        if let Some(m) = message {
            alert.message = m;
        }
        if let Some(iv) = interval {
            alert.interval = Some(iv);
        }
        if let Some(p) = params_json {
            alert.params_json = Some(p);
        }
        if let Some(snd) = sound_enabled {
            alert.sound_enabled = snd;
        }
        if let Some(f) = frequency
            && matches!(f.as_str(), "once" | "once_per_bar_close")
        {
            alert.frequency = f;
        }
        if let Some(cd) = cooldown_bars {
            alert.cooldown_bars = cd.clamp(0, 500);
        }
        if rearm.unwrap_or(false) {
            alert.status = "armed".to_owned();
        }
        ws.upsert_alert(&alert)?;
        Ok(alert)
    })
    .await
    .map_err(|e| DesktopError::Internal(e.to_string()))?
}

#[tauri::command]
pub async fn list_alert_logs(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> DesktopResult<Vec<AlertTriggerLogRecord>> {
    let ws = state.workspace.clone();
    let lim = limit.unwrap_or(100);
    tokio::task::spawn_blocking(move || ws.list_alert_trigger_logs(lim))
        .await
        .map_err(|e| DesktopError::Internal(e.to_string()))?
}

#[tauri::command]
pub async fn clear_alert_logs(state: State<'_, AppState>) -> DesktopResult<()> {
    let ws = state.workspace.clone();
    tokio::task::spawn_blocking(move || ws.clear_alert_trigger_logs())
        .await
        .map_err(|e| DesktopError::Internal(e.to_string()))?
}

// ─── Private Alert Notification Delivery (`~/.config/sumi/alerts_secrets.json` 0600) ───

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertDeliveryConfig {
    #[serde(default = "default_true_flag")]
    pub os_notification_enabled: bool,
    #[serde(default)]
    pub telegram_bot_token: String,
    #[serde(default)]
    pub telegram_chat_id: String,
    #[serde(default)]
    pub webhook_url: String,
    #[serde(default = "default_true_flag")]
    pub watchlist_background_scan_enabled: bool,
}

fn default_true_flag() -> bool {
    true
}

impl Default for AlertDeliveryConfig {
    fn default() -> Self {
        Self {
            os_notification_enabled: true,
            telegram_bot_token: String::new(),
            telegram_chat_id: String::new(),
            webhook_url: String::new(),
            watchlist_background_scan_enabled: true,
        }
    }
}

fn alerts_secrets_path() -> PathBuf {
    sumi_config_dir().join("alerts_secrets.json")
}

pub(crate) fn load_alert_delivery_config_from(path: &std::path::Path) -> AlertDeliveryConfig {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str::<AlertDeliveryConfig>(&s).ok())
        .unwrap_or_default()
}

pub(crate) fn save_alert_delivery_config_to(
    path: &std::path::Path,
    cfg: &AlertDeliveryConfig,
) -> DesktopResult<()> {
    let bytes = serde_json::to_vec_pretty(cfg)?;
    write_private_file_0600(path, &bytes).map_err(|e| DesktopError::Internal(e.to_string()))
}

#[tauri::command]
pub async fn get_alert_delivery_config() -> DesktopResult<AlertDeliveryConfig> {
    tokio::task::spawn_blocking(|| load_alert_delivery_config_from(&alerts_secrets_path()))
        .await
        .map_err(|e| DesktopError::Internal(e.to_string()))
}

#[tauri::command]
pub async fn save_alert_delivery_config(
    config: AlertDeliveryConfig,
) -> DesktopResult<AlertDeliveryConfig> {
    let out = config.clone();
    tokio::task::spawn_blocking(move || {
        save_alert_delivery_config_to(&alerts_secrets_path(), &config)
    })
    .await
    .map_err(|e| DesktopError::Internal(e.to_string()))??;
    Ok(out)
}

async fn dispatch_external_alert_channels(cfg: &AlertDeliveryConfig, alert: &PriceAlertRecord) -> Vec<&'static str> {
    let mut channels = vec!["in_app"];
    if cfg.os_notification_enabled {
        channels.push("os");
    }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(6))
        .build()
        .ok();
    let Some(client) = client else {
        return channels;
    };

    let token = cfg.telegram_bot_token.trim();
    let chat_id = cfg.telegram_chat_id.trim();
    if !token.is_empty() && !chat_id.is_empty() {
        let text = format!(
            "🔔 Sumi Alert · {}\n{}\nMốc: {} ({})",
            alert.ticker, alert.message, alert.target_price, alert.condition
        );
        let url = format!("https://api.telegram.org/bot{token}/sendMessage");
        if client
            .post(&url)
            .json(&serde_json::json!({ "chat_id": chat_id, "text": text }))
            .send()
            .await
            .is_ok_and(|r| r.status().is_success())
        {
            channels.push("telegram");
        }
    }

    let webhook = cfg.webhook_url.trim();
    if webhook.starts_with("http://") || webhook.starts_with("https://") {
        let payload = serde_json::json!({
            "source": "sumi_supercharts",
            "alertId": alert.id,
            "ticker": alert.ticker,
            "alertKind": alert.alert_kind,
            "condition": alert.condition,
            "targetPrice": alert.target_price,
            "message": alert.message,
            "triggeredAtMs": alert.triggered_at_ms,
        });
        if client
            .post(webhook)
            .json(&payload)
            .send()
            .await
            .is_ok_and(|r| r.status().is_success())
        {
            channels.push("webhook");
        }
    }
    channels
}

#[tauri::command]
pub async fn test_alert_delivery() -> DesktopResult<Vec<String>> {
    let cfg = load_alert_delivery_config_from(&alerts_secrets_path());
    let sample = PriceAlertRecord {
        id: "test_alert".to_owned(),
        ticker: "HOSE:FPT".to_owned(),
        alert_kind: "price".to_owned(),
        condition: "cross_above".to_owned(),
        target_price: 130_000.0,
        interval: Some("1D".to_owned()),
        params_json: None,
        sound_enabled: true,
        message: "Kiểm tra kết nối thông báo từ Sumi Supercharts v3.0".to_owned(),
        status: "triggered".to_owned(),
        frequency: "once".to_owned(),
        cooldown_bars: 0,
        last_fired_bar_ts: None,
        created_at_ms: chrono::Utc::now().timestamp_millis(),
        triggered_at_ms: Some(chrono::Utc::now().timestamp_millis()),
    };
    let channels = dispatch_external_alert_channels(&cfg, &sample).await;
    Ok(channels.into_iter().map(str::to_owned).collect())
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

/// Evaluates the exact line price at `timestamp_ms` for a drawing-line alert (`horizontal`, `segment`, `rayLine`, `straightLine`).
pub(crate) fn evaluate_drawing_line_price_at(
    alert: &PriceAlertRecord,
    timestamp_ms: i64,
) -> Option<f64> {
    let params = alert.params_json.as_ref()?;
    let pts = params.get("points")?.as_array()?;
    let drawing_name = params
        .get("drawingName")
        .and_then(Value::as_str)
        .unwrap_or("segment");

    let p0_val = pts.first()?.get("value").and_then(Value::as_f64)?;
    let p0_ts = pts.first()?.get("timestamp").and_then(Value::as_i64);

    if pts.len() < 2
        || matches!(
            drawing_name,
            "horizontalStraightLine" | "horizontalRayLine" | "horizontalSegment"
        )
    {
        if drawing_name == "horizontalRayLine"
            && let Some(t0) = p0_ts
            && timestamp_ms < t0
        {
            return None;
        }
        return Some(p0_val);
    }

    let p1_val = pts.get(1)?.get("value").and_then(Value::as_f64)?;
    let t0 = p0_ts?;
    let t1 = pts.get(1)?.get("timestamp").and_then(Value::as_i64)?;
    if t1 == t0 {
        return Some(p0_val);
    }
    match drawing_name {
        "segment" => {
            let (lo, hi) = if t0 <= t1 { (t0, t1) } else { (t1, t0) };
            if timestamp_ms < lo || timestamp_ms > hi {
                return None;
            }
        }
        "rayLine" if (t1 > t0 && timestamp_ms < t0) || (t1 < t0 && timestamp_ms > t0) => {
            return None;
        }
        _ => {} // straightLine: infinite extension
    }
    let ratio = (timestamp_ms as f64 - t0 as f64) / (t1 as f64 - t0 as f64);
    let val = p0_val + (p1_val - p0_val) * ratio;
    val.is_finite().then_some(val)
}

/// Evaluates an `"indicator"` alert (`RSI`, `MACD`, `EMA`) strictly causally over `bars` (ending at current bar).
pub(crate) fn evaluate_indicator_alert_on_bars(
    alert: &PriceAlertRecord,
    bars: &[OhlcvBar],
) -> bool {
    if bars.len() < 3 {
        return false;
    }
    let Some(params) = alert.params_json.as_ref() else {
        return false;
    };
    let ind = params
        .get("indicator")
        .and_then(Value::as_str)
        .unwrap_or("RSI")
        .to_ascii_uppercase();
    let closes: Vec<f64> = bars.iter().map(|b| b.close).collect();
    let n = closes.len();

    match ind.as_str() {
        "RSI" => {
            let period = params
                .get("period")
                .and_then(Value::as_u64)
                .unwrap_or(14)
                .clamp(2, 100) as usize;
            let threshold = params
                .get("threshold")
                .and_then(Value::as_f64)
                .unwrap_or(alert.target_price);
            let rsi = compute_rsi_slices(&closes, period);
            let (Some(prev), Some(curr)) = (rsi[n - 2], rsi[n - 1]) else {
                return false;
            };
            match alert.condition.as_str() {
                "cross_above" => prev < threshold && curr >= threshold,
                "cross_below" => prev > threshold && curr <= threshold,
                _ => (prev < threshold && curr >= threshold) || (prev > threshold && curr <= threshold),
            }
        }
        "MACD" => {
            let fast = params.get("fast").and_then(Value::as_u64).unwrap_or(12).clamp(2, 60) as usize;
            let slow = params.get("slow").and_then(Value::as_u64).unwrap_or(26).clamp(3, 120) as usize;
            let sig = params.get("signal").and_then(Value::as_u64).unwrap_or(9).clamp(1, 50) as usize;
            let out = compute_macd_slices(&closes, fast, slow, sig);
            let (Some(prev_h), Some(curr_h)) = (out.histogram[n - 2], out.histogram[n - 1]) else {
                return false;
            };
            match alert.condition.as_str() {
                "cross_above" => prev_h <= 0.0 && curr_h > 0.0,
                "cross_below" => prev_h >= 0.0 && curr_h < 0.0,
                _ => (prev_h <= 0.0 && curr_h > 0.0) || (prev_h >= 0.0 && curr_h < 0.0),
            }
        }
        "EMA" => {
            let period = params
                .get("period")
                .and_then(Value::as_u64)
                .unwrap_or(20)
                .clamp(2, 250) as usize;
            let ema = compute_ema_slices(&closes, period);
            let (Some(prev_ema), Some(curr_ema)) = (ema[n - 2], ema[n - 1]) else {
                return false;
            };
            let prev_c = closes[n - 2];
            let curr_c = closes[n - 1];
            match alert.condition.as_str() {
                "cross_above" => prev_c <= prev_ema && curr_c > curr_ema,
                "cross_below" => prev_c >= prev_ema && curr_c < curr_ema,
                _ => {
                    (prev_c <= prev_ema && curr_c > curr_ema)
                        || (prev_c >= prev_ema && curr_c < curr_ema)
                }
            }
        }
        // Wave 3: Full 123 QMV SIGNAL_CATALOG causal trigger (`sumi-core::scanner::PrecomputedCatalogMatrix`)
        "SIGNAL" => {
            let Some(catalog_id) = params.get("catalogId").and_then(Value::as_str) else {
                return false;
            };
            let Some(entry_idx) = catalog_entry_index(catalog_id.trim()) else {
                return false;
            };
            let entry = &SIGNAL_CATALOG[entry_idx];
            let iv = alert
                .interval
                .as_deref()
                .and_then(|s| s.parse::<Interval>().ok())
                .unwrap_or(Interval::D1);
            let candles: Vec<Candle> = bars.iter().map(|b| b.to_core_candle(iv)).collect();
            let matrix = PrecomputedCatalogMatrix::compute(&candles);
            let profile = ConfluenceWeightProfile::Balanced;
            let curr_pt = matrix.point_at_with_profile(entry_idx, n - 1, profile);
            let prev_pt = matrix.point_at_with_profile(entry_idx, n - 2, profile);
            match entry.output_type {
                SignalOutputType::Bool => {
                    let prev_on = prev_pt.is_some_and(|p| p.quality.is_valid() && p.bool_value == Some(true));
                    let curr_on = curr_pt.is_some_and(|p| p.quality.is_valid() && p.bool_value == Some(true));
                    !prev_on && curr_on
                }
                SignalOutputType::Float => {
                    let threshold = params
                        .get("threshold")
                        .and_then(Value::as_f64)
                        .unwrap_or(alert.target_price);
                    let (Some(pv), Some(cv)) = (
                        prev_pt.filter(|p| p.quality.is_valid()).and_then(|p| p.float_value),
                        curr_pt.filter(|p| p.quality.is_valid()).and_then(|p| p.float_value),
                    ) else {
                        return false;
                    };
                    match alert.condition.as_str() {
                        "cross_above" => pv < threshold && cv >= threshold,
                        "cross_below" => pv > threshold && cv <= threshold,
                        _ => (pv < threshold && cv >= threshold) || (pv > threshold && cv <= threshold),
                    }
                }
                SignalOutputType::Enum => {
                    let prev_d = prev_pt.filter(|p| p.quality.is_valid()).and_then(|p| p.direction);
                    let curr_d = curr_pt.filter(|p| p.quality.is_valid()).and_then(|p| p.direction);
                    prev_d != curr_d && curr_d.is_some()
                }
            }
        }
        // Wave 3: 4-Family Technical Health Confluence Threshold crossing (`[-100, +100]`)
        "CONFLUENCE" => {
            let iv = alert
                .interval
                .as_deref()
                .and_then(|s| s.parse::<Interval>().ok())
                .unwrap_or(Interval::D1);
            let candles: Vec<Candle> = bars.iter().map(|b| b.to_core_candle(iv)).collect();
            let matrix = PrecomputedCatalogMatrix::compute(&candles);
            let profile = match params
                .get("profile")
                .and_then(Value::as_str)
                .map(|s| s.to_ascii_lowercase())
                .as_deref()
            {
                Some("trend" | "trend_focused") => ConfluenceWeightProfile::TrendFocused,
                _ => ConfluenceWeightProfile::Balanced,
            };
            let threshold = params
                .get("threshold")
                .and_then(Value::as_f64)
                .unwrap_or(alert.target_price);
            let Ok(prev_h) = matrix.health_score_at(n - 2, profile) else {
                return false;
            };
            let Ok(curr_h) = matrix.health_score_at(n - 1, profile) else {
                return false;
            };
            if !prev_h.is_finite() || !curr_h.is_finite() {
                return false;
            }
            match alert.condition.as_str() {
                "cross_above" => prev_h < threshold && curr_h >= threshold,
                "cross_below" => prev_h > threshold && curr_h <= threshold,
                _ => (prev_h < threshold && curr_h >= threshold) || (prev_h > threshold && curr_h <= threshold),
            }
        }
        _ => false,
    }
}

fn parse_cross_dir(cond: &str) -> CrossDirection {
    match cond {
        "cross_above" => CrossDirection::CrossAbove,
        "cross_below" => CrossDirection::CrossBelow,
        _ => CrossDirection::CrossAny,
    }
}

fn alert_cooldown_elapsed(alert: &PriceAlertRecord, bar_ts_ms: i64, causal_bars: &[OhlcvBar]) -> bool {
    let Some(last_ts) = alert.last_fired_bar_ts else {
        return true;
    };
    if bar_ts_ms <= last_ts {
        return false;
    }
    let cd = alert.cooldown_bars.max(0) as usize;
    if cd == 0 {
        return true;
    }
    let bars_since = causal_bars.iter().filter(|b| b.timestamp > last_ts && b.timestamp <= bar_ts_ms).count();
    bars_since > cd
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
    let fired = tokio::task::spawn_blocking(move || -> DesktopResult<Vec<PriceAlertRecord>> {
        let alerts = ws.list_alerts()?;
        let now_ms = chrono::Utc::now().timestamp_millis();
        let mut fired = Vec::new();
        for mut a in alerts {
            if a.ticker != norm || a.status != "armed" {
                continue;
            }
            let target = match a.alert_kind.as_str() {
                "price" => Some(a.target_price),
                "drawing" => evaluate_drawing_line_price_at(&a, now_ms),
                _ => None, // indicator alerts require bar history -> evaluated by `check_bar_alerts` / Replay
            };
            let Some(level) = target else {
                continue;
            };
            let crossed = match a.condition.as_str() {
                "cross_above" => prev_price < level && current_price >= level,
                "cross_below" => prev_price > level && current_price <= level,
                _ => (prev_price < level && current_price >= level)
                    || (prev_price > level && current_price <= level),
            };
            if crossed {
                let next_status = if a.frequency == "once_per_bar_close" {
                    "armed"
                } else {
                    "triggered"
                };
                a.status = next_status.to_owned();
                a.triggered_at_ms = Some(now_ms);
                a.last_fired_bar_ts = Some(now_ms);
                let _ = ws.record_alert_fire(&a, next_status, now_ms, current_price, "in_app");
                fired.push(a);
            }
        }
        Ok(fired)
    })
    .await
    .map_err(|e| DesktopError::Internal(e.to_string()))??;

    // Async non-blocking Telegram / Webhook delivery
    if !fired.is_empty() {
        let cfg = load_alert_delivery_config_from(&alerts_secrets_path());
        if !cfg.telegram_bot_token.trim().is_empty() || !cfg.webhook_url.trim().is_empty() {
            let to_send = fired.clone();
            tauri::async_runtime::spawn(async move {
                for al in &to_send {
                    let _ = dispatch_external_alert_channels(&cfg, al).await;
                }
            });
        }
    }
    Ok(fired)
}

/// Evaluates all armed alerts across every ticker in Watchlists (`Watchlist Alerts` background scan, Wave 3).
#[tauri::command]
pub async fn scan_watchlist_alerts(
    state: State<'_, AppState>,
) -> DesktopResult<Vec<PriceAlertRecord>> {
    let ws = state.workspace.clone();
    let store = state.market_store.clone();
    let fired = tokio::task::spawn_blocking(move || {
        let alerts = ws.list_alerts()?;
        let armed: Vec<PriceAlertRecord> = alerts
            .into_iter()
            .filter(|a| a.status == "armed")
            .collect();
        let mut seen_pairs = std::collections::BTreeSet::new();
        for a in &armed {
            let iv_str = a.interval.as_deref().unwrap_or("1D").to_owned();
            seen_pairs.insert((a.ticker.clone(), iv_str));
        }
        let mut all_fired = Vec::new();
        for (ticker, iv_str) in seen_pairs {
            let Ok(sym) = SymbolKey::parse(&ticker) else {
                continue;
            };
            let Ok(iv) = iv_str.parse::<Interval>() else {
                continue;
            };
            let Ok(page) = store.query_bars(
                &SeriesKey::new(&sym, iv, Adjustment::Splits),
                BarQueryMode::Init,
                260,
            ) else {
                continue;
            };
            if page.bars.len() < 2 {
                continue;
            }
            let bar = page.bars[page.bars.len() - 1];
            let prev_close = page.bars[page.bars.len() - 2].close;
            let hits = evaluate_bar_alerts_slice(&ws, &ticker, prev_close, &bar, &page.bars);
            all_fired.extend(hits);
        }
        Ok::<Vec<PriceAlertRecord>, DesktopError>(all_fired)
    })
    .await
    .map_err(|e| DesktopError::Internal(e.to_string()))??;

    if !fired.is_empty() {
        let cfg = load_alert_delivery_config_from(&alerts_secrets_path());
        if !cfg.telegram_bot_token.trim().is_empty() || !cfg.webhook_url.trim().is_empty() {
            let to_send = fired.clone();
            tauri::async_runtime::spawn(async move {
                for al in &to_send {
                    let _ = dispatch_external_alert_channels(&cfg, al).await;
                }
            });
        }
    }
    Ok(fired)
}

/// Evaluates armed price, drawing-line, AND indicator (`RSI`/`MACD`/`EMA`) alerts on live bar arrival (v2.2).
#[tauri::command]
pub async fn check_bar_alerts(
    state: State<'_, AppState>,
    ticker: String,
    interval: String,
) -> DesktopResult<Vec<PriceAlertRecord>> {
    let ws = state.workspace.clone();
    let store = state.market_store.clone();
    let sym = SymbolKey::parse(&ticker)?;
    let iv: Interval = interval.parse()?;
    let norm = sym.ticker();
    let iv_str = iv.as_str().to_owned();

    tokio::task::spawn_blocking(move || {
        let alerts = ws.list_alerts()?;
        let relevant: Vec<PriceAlertRecord> = alerts
            .into_iter()
            .filter(|a| {
                a.ticker == norm
                    && a.status == "armed"
                    && a.interval
                        .as_deref()
                        .is_none_or(|a_iv| a_iv.eq_ignore_ascii_case(&iv_str))
            })
            .collect();
        if relevant.is_empty() {
            return Ok(Vec::new());
        }
        let series = SeriesKey::new(&sym, iv, Adjustment::Splits);
        let page = store.query_bars(&series, BarQueryMode::Init, 260)?;
        if page.bars.len() < 2 {
            return Ok(Vec::new());
        }
        let bar = page.bars[page.bars.len() - 1];
        let prev_close = page.bars[page.bars.len() - 2].close;
        Ok(evaluate_bar_alerts_slice(&ws, &norm, prev_close, &bar, &page.bars))
    })
    .await
    .map_err(|e| DesktopError::Internal(e.to_string()))?
}

/// Shared causal evaluator for Price, Drawing-Line & Indicator alerts across both Live bars and Bar Replay (`v2.2`).
pub(crate) fn evaluate_bar_alerts_slice(
    ws: &crate::workspace_db::WorkspaceStore,
    ticker: &str,
    prev_close: f64,
    bar: &OhlcvBar,
    causal_bars: &[OhlcvBar],
) -> Vec<PriceAlertRecord> {
    let Ok(alerts) = ws.list_alerts() else {
        return Vec::new();
    };
    let norm = ticker.trim().to_ascii_uppercase();
    let mut fired = Vec::new();
    for mut a in alerts {
        if a.ticker != norm || a.status != "armed" {
            continue;
        }
        if !alert_cooldown_elapsed(&a, bar.timestamp, causal_bars) {
            continue;
        }
        let core_bar = bar.to_core_candle(Interval::D1);
        let dir = parse_cross_dir(&a.condition);
        let crossed = match a.alert_kind.as_str() {
            "drawing" => {
                if let Some(target) = evaluate_drawing_line_price_at(&a, bar.timestamp) {
                    evaluate_4_waypoint_cross(Some(prev_close), &core_bar, target, dir).is_some()
                } else {
                    false
                }
            }
            "indicator" => evaluate_indicator_alert_on_bars(&a, causal_bars),
            _ => evaluate_4_waypoint_cross(Some(prev_close), &core_bar, a.target_price, dir).is_some(),
        };
        if crossed {
            let next_status = if a.frequency == "once_per_bar_close" {
                "armed"
            } else {
                "triggered"
            };
            a.status = next_status.to_owned();
            a.triggered_at_ms = Some(bar.timestamp);
            a.last_fired_bar_ts = Some(bar.timestamp);
            let _ = ws.record_alert_fire(&a, next_status, bar.timestamp, bar.close, "in_app");
            fired.push(a);
        }
    }
    fired
}

/// Backward-compatible wrapper called from `replay_step`.
pub(crate) fn evaluate_replay_bar_alerts(
    ws: &crate::workspace_db::WorkspaceStore,
    ticker: &str,
    prev_close: f64,
    bar: &OhlcvBar,
    causal_bars: &[OhlcvBar],
) -> Vec<PriceAlertRecord> {
    evaluate_bar_alerts_slice(ws, ticker, prev_close, bar, causal_bars)
}

/// Opens a synchronized detached OS window for multi-monitor setups (`BUG-043`).
#[tauri::command]
pub async fn open_detached_window(
    app: tauri::AppHandle,
    ticker: String,
    interval: String,
) -> DesktopResult<String> {
    use tauri::{WebviewUrl, WebviewWindowBuilder};

    let now_ms = chrono::Utc::now().timestamp_millis();
    let label = format!("chart_{now_ms}");
    let clean_ticker = ticker.trim().to_ascii_uppercase();
    let clean_iv = interval.trim().to_owned();
    let url = format!(
        "index.html?detached=1&ticker={}&interval={}",
        clean_ticker.replace(':', "%3A"),
        clean_iv
    );

    WebviewWindowBuilder::new(&app, &label, WebviewUrl::App(url.into()))
        .title(format!("Sumi Supercharts — {clean_ticker} ({clean_iv})"))
        .inner_size(1180.0, 760.0)
        .min_inner_size(720.0, 480.0)
        .resizable(true)
        .build()
        .map_err(|e| DesktopError::Internal(format!("Không thể mở cửa sổ mới: {e}")))?;

    Ok(label)
}

/// Broadcasts a real-time sync event (Replay step/cut, Drawing update, Symbol change) across all OS windows (`BUG-043`).
#[tauri::command]
pub async fn broadcast_sync_event(
    app: tauri::AppHandle,
    event_name: String,
    payload: serde_json::Value,
) -> DesktopResult<()> {
    use tauri::Emitter;
    let allowed = matches!(
        event_name.as_str(),
        "sumi:sync_replay" | "sumi:sync_drawing" | "sumi:sync_symbol" | "sumi:sync_range"
    );
    if !allowed {
        return Err(DesktopError::Validation(format!(
            "Sự kiện đồng bộ không hợp lệ: {event_name}"
        )));
    }
    app.emit(&event_name, payload)
        .map_err(|e| DesktopError::Internal(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace_db::WorkspaceStore;
    use serde_json::json;

    #[test]
    fn drawing_trendline_and_indicator_alerts_evaluate_causally() {
        let ws = WorkspaceStore::open_in_memory().unwrap();
        let t0 = 1_700_000_000_000_i64;
        let step = 86_400_000_i64;

        // Sloped rayLine rising from (t0, 100.0) to (t0 + 10*step, 110.0) => at t0 + 20*step price == 120.0
        ws.upsert_alert(&PriceAlertRecord {
            id: "al_draw".to_owned(),
            ticker: "HOSE:FPT".to_owned(),
            alert_kind: "drawing".to_owned(),
            condition: "cross_above".to_owned(),
            target_price: 110.0,
            interval: Some("1D".to_owned()),
            params_json: Some(json!({
                "drawingId": "d1",
                "drawingName": "rayLine",
                "points": [
                    { "timestamp": t0, "value": 100.0 },
                    { "timestamp": t0 + 10 * step, "value": 110.0 }
                ]
            })),
            sound_enabled: true,
            message: "Break trendline".to_owned(),
            status: "armed".to_owned(),
            frequency: "once".to_owned(),
            cooldown_bars: 0,
            last_fired_bar_ts: None,
            created_at_ms: t0,
            triggered_at_ms: None,
        })
        .unwrap();

        let bar_at_20 = OhlcvBar::new(t0 + 20 * step, 118.0, 121.0, 117.0, 120.5, 1_000.0);
        let fired = evaluate_bar_alerts_slice(&ws, "HOSE:FPT", 118.0, &bar_at_20, &[bar_at_20]);
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].id, "al_draw");

        // RSI(14) crossing above 70 on strong uptrend bars
        ws.upsert_alert(&PriceAlertRecord {
            id: "al_rsi".to_owned(),
            ticker: "HOSE:FPT".to_owned(),
            alert_kind: "indicator".to_owned(),
            condition: "cross_above".to_owned(),
            target_price: 65.0,
            interval: Some("1D".to_owned()),
            params_json: Some(json!({ "indicator": "RSI", "period": 14, "threshold": 65.0 })),
            sound_enabled: true,
            message: "RSI > 65".to_owned(),
            status: "armed".to_owned(),
            frequency: "once_per_bar_close".to_owned(),
            cooldown_bars: 2,
            last_fired_bar_ts: None,
            created_at_ms: t0,
            triggered_at_ms: None,
        })
        .unwrap();

        let mut bars: Vec<OhlcvBar> = (0..25)
            .map(|i| {
                let c = if i % 2 == 0 { 100.0 } else { 100.5 };
                OhlcvBar::new(t0 + i * step, c, c + 1.0, c - 1.0, c, 1_000.0)
            })
            .collect();
        bars.push(OhlcvBar::new(t0 + 25 * step, 101.0, 115.0, 100.5, 114.0, 2_000.0));
        let last = *bars.last().unwrap();
        let fired_ind = evaluate_bar_alerts_slice(&ws, "HOSE:FPT", 100.5, &last, &bars);
        assert_eq!(fired_ind.len(), 1);
        assert_eq!(fired_ind[0].id, "al_rsi");
        // `once_per_bar_close` remains armed AND logs trigger history!
        assert_eq!(fired_ind[0].status, "armed");
        assert!(ws.list_alert_trigger_logs(10).unwrap().len() >= 2);
        // Same bar must NOT re-fire due to cooldown
        let repeat = evaluate_bar_alerts_slice(&ws, "HOSE:FPT", 100.5, &last, &bars);
        assert!(repeat.is_empty());
    }
}
