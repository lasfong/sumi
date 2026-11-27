use std::path::PathBuf;

use sumi_data::{Adjustment, BarQueryMode, Interval, SeriesKey, SymbolKey};
use sumi_yumeko_bridge::{
    ExportedVerdictListItem, ExportedVerdictReceipt, Kt01AfcSummary, QmvRealChartSeriesDto,
    ResearchExportVerificationDto, TopDownSymbolContextDto, YumekoBridgeStatus,
    YumekoReadOnlyBridge, evaluate_kt01_gate, export_kt01_verdict_to_disk, list_exported_verdicts,
};
use tauri::State;

use crate::commands::analog_cone::compute_analog_cone;
use crate::error::{DesktopError, DesktopResult};
use crate::state::AppState;

/// Inspects read-only Yumeko v3.0 SQLite availability, latest `as_of` dates, and cluster counts.
#[tauri::command]
pub async fn yumeko_get_bridge_status() -> DesktopResult<YumekoBridgeStatus> {
    tokio::task::spawn_blocking(|| {
        let bridge = YumekoReadOnlyBridge::discover_default();
        bridge
            .get_status()
            .map_err(|e| DesktopError::Internal(e.to_string()))
    })
    .await
    .map_err(|e| DesktopError::Internal(e.to_string()))?
}

/// Loads full Top-Down context (`VM01 -> HP01 -> DN01 -> DM01`) for `ticker` at optional PIT `as_of`.
#[tauri::command]
pub async fn yumeko_get_topdown_context(
    ticker: String,
    as_of: Option<String>,
) -> DesktopResult<TopDownSymbolContextDto> {
    tokio::task::spawn_blocking(move || {
        let bridge = YumekoReadOnlyBridge::discover_default();
        bridge
            .load_topdown_context(&ticker, as_of.as_deref())
            .map_err(|e| DesktopError::Internal(e.to_string()))
    })
    .await
    .map_err(|e| DesktopError::Internal(e.to_string()))?
}

/// Loads real QMV Blackbox chart series (`QMV_STOCK_BB`, `QMV_ACTIVE_MONEY`, `QMV_FOREIGN_FLOW`)
/// marked with explicit `sourceProvenance` (`"qmv_real"` vs `"local_proxy"`).
#[tauri::command]
pub async fn yumeko_get_qmv_series(
    ticker: String,
    as_of: Option<String>,
) -> DesktopResult<QmvRealChartSeriesDto> {
    tokio::task::spawn_blocking(move || {
        let bridge = YumekoReadOnlyBridge::discover_default();
        bridge
            .load_qmv_stock_series(&ticker, as_of.as_deref())
            .map_err(|e| DesktopError::Internal(e.to_string()))
    })
    .await
    .map_err(|e| DesktopError::Internal(e.to_string()))?
}

/// Evaluates the full `KT01` Confirmation Gate (`1W / 1D / 1H` + QMV Blackbox + AFC + SL/TP)
/// and exports a SHA-256 pinned `KT01GateVerdictV1` + `TechnicalEvidenceBundleV1` (`.json` + `.md`).
#[tauri::command]
pub async fn yumeko_export_kt01_verdict(
    state: State<'_, AppState>,
    ticker: String,
    as_of: Option<String>,
    copy_to_yumeko_dropzone: Option<bool>,
) -> DesktopResult<ExportedVerdictReceipt> {
    let sym_key = SymbolKey::parse(&ticker)?;
    let canonical_ticker = sym_key.ticker();
    let as_of_cutoff_ms = as_of
        .as_deref()
        .and_then(|s| chrono::NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d").ok())
        .and_then(|d| d.and_hms_opt(23, 59, 59))
        .map(|dt| dt.and_utc().timestamp_millis());

    // Optional AFC summary (non-fatal if bars insufficient)
    let afc_summary: Option<Kt01AfcSummary> = compute_analog_cone(
        state.clone(),
        canonical_ticker.clone(),
        "1D".to_owned(),
        Some(10),
        Some(100),
        Some(true),
        as_of_cutoff_ms,
    )
    .await
    .ok()
    .and_then(|resp| {
        resp.forecast.map(|f| {
            let last_step = f.steps.last();
            Kt01AfcSummary {
                horizon_bars: resp.horizon,
                win_rate_pct: f.win_rate_pct,
                expected_p50_pct: f.expected_return_p50_pct,
                p10_pct: last_step.map_or(0.0, |s| s.p10_ret_pct),
                p90_pct: last_step.map_or(0.0, |s| s.p90_ret_pct),
                prob_tail_drop_6_pct: f.prob_tail_drop_6pct,
            }
        })
    });

    let store = state.market_store.clone();
    let data_dir = state.data_dir.clone();
    let want_dropzone = copy_to_yumeko_dropzone.unwrap_or(true);

    tokio::task::spawn_blocking(move || {
        let bridge = YumekoReadOnlyBridge::discover_default();
        let topdown = bridge
            .load_topdown_context(&canonical_ticker, as_of.as_deref())
            .ok();

        let d1_key = SeriesKey::new(&sym_key, Interval::D1, Adjustment::Splits);
        let mut daily_bars = store
            .query_bars(&d1_key, BarQueryMode::Init, 600)
            .map(|r| r.bars)
            .unwrap_or_default();

        // Decision #3: If Yumeko PIT `as_of` is active or local DB has sparse bars, merge `dnse_real` bars!
        if let Ok(Some(dnse)) = bridge.load_dnse_ohlcv_series(&canonical_ticker, as_of.as_deref())
            && !dnse.sumi_bars.is_empty()
        {
            let first_dnse = dnse.sumi_bars.first().unwrap().timestamp;
            if as_of.is_some() || daily_bars.is_empty() {
                let mut stitched: Vec<_> = daily_bars
                    .into_iter()
                    .filter(|b| b.timestamp < first_dnse)
                    .collect();
                stitched.extend(dnse.sumi_bars);
                daily_bars = stitched;
            }
        }

        let h1_key = SeriesKey::new(&sym_key, Interval::H1, Adjustment::Splits);
        let hourly_bars = store
            .query_bars(&h1_key, BarQueryMode::Init, 200)
            .ok()
            .map(|r| r.bars)
            .filter(|v| v.len() >= 20);

        let verdict = evaluate_kt01_gate(
            &canonical_ticker,
            &daily_bars,
            hourly_bars.as_deref(),
            afc_summary,
            topdown.as_ref(),
            as_of.as_deref(),
        )
        .map_err(|e| DesktopError::Validation(e.to_string()))?;

        export_kt01_verdict_to_disk(&verdict, Some(&data_dir), want_dropzone)
            .map_err(|e| DesktopError::Internal(e.to_string()))
    })
    .await
    .map_err(|e| DesktopError::Internal(e.to_string()))?
}

/// Lists recently exported KT01 verdict bundles from `<SUMI_DATA_DIR>/kt01_verdicts/`.
#[tauri::command]
pub async fn yumeko_list_exported_verdicts(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> DesktopResult<Vec<ExportedVerdictListItem>> {
    let data_dir = state.data_dir.clone();
    let lim = limit.unwrap_or(20);
    tokio::task::spawn_blocking(move || {
        list_exported_verdicts(Some(&data_dir), lim)
            .map_err(|e| DesktopError::Internal(e.to_string()))
    })
    .await
    .map_err(|e| DesktopError::Internal(e.to_string()))?
}

/// Verifies a `yumeko research export` folder (`manifest.json` + SHA-256 of exported PIT tables).
#[tauri::command]
pub async fn yumeko_verify_research_export(
    dir: String,
) -> DesktopResult<ResearchExportVerificationDto> {
    tokio::task::spawn_blocking(move || {
        YumekoReadOnlyBridge::verify_research_export_dir(&PathBuf::from(dir.trim()))
            .map_err(|e| DesktopError::Internal(e.to_string()))
    })
    .await
    .map_err(|e| DesktopError::Internal(e.to_string()))?
}
