#![warn(clippy::all, rust_2018_idioms)]

pub mod commands;
pub mod deeplink;
pub mod error;
pub mod state;
pub mod sync;
pub mod workspace_db;

use tauri::{Emitter, Manager};

use crate::state::{AppState, resolve_database_paths};

/// Starts the Tauri v2 Supercharts Desktop Workstation runtime.
///
/// # Panics
/// Panics if the Tauri application fails to initialize or bind its native webview window.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let cli_args: Vec<String> = std::env::args().collect();
    let initial_deeplink = deeplink::extract_deeplink_from_args(&cli_args);
    if let Some(ref url) = initial_deeplink
        && deeplink::try_forward_to_running_instance(url, None)
    {
        // Delivered to already-running Sumi instance cleanly; exit secondary process.
        return;
    }

    sumi_data::init_tracing("info");

    tauri::Builder::default()
        .setup(move |app| {
            let app_data_dir = app.path().app_data_dir().ok();
            let (market_db, workspace_db) = resolve_database_paths(app_data_dir);
            let app_state = AppState::initialize(market_db, workspace_db)
                .map_err(|e| std::io::Error::other(e.to_string()))?;
            let data_dir_for_ipc = app_state.data_dir.clone();
            app.manage(app_state);

            deeplink::start_single_instance_ipc_listener(app.handle().clone(), data_dir_for_ipc);

            if let Some(ref raw_url) = initial_deeplink
                && let Some(payload) = deeplink::parse_sumi_deeplink(raw_url)
            {
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(900)).await;
                    let _ = handle.emit(deeplink::DEEPLINK_EVENT_NAME, &payload);
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Market Data & Live Channels
            commands::market::search_symbols,
            commands::market::resolve_symbol,
            commands::market::get_bars,
            commands::market::subscribe_bars,
            commands::market::unsubscribe_bars,
            commands::market::subscribe_quotes,
            commands::market::unsubscribe_quotes,
            commands::market::get_watchlist_snapshots,
            commands::market::get_data_status,
            commands::market::set_offline_mode,
            commands::market::import_cafef_dir,
            commands::market::sync_vn_universe_catalog,
            commands::market::sync_universe_eod,
            commands::market::get_corporate_events,
            commands::market::get_symbol_fundamentals,
            commands::market::get_compare_series,
            // Cross-Device Workspace Sync v1 (Mac <-> Windows PC, .sumi.json + Folder Merge + Doraemi Status)
            commands::sync::sync_get_status,
            commands::sync::sync_export_bundle,
            commands::sync::sync_import_bundle,
            commands::sync::sync_configure_folder,
            commands::sync::sync_folder_now,
            commands::sync::sync_get_doraemi_status,
            // Workspace, Layouts, Drawings, Indicators, Watchlists, Alerts
            commands::workspace::load_workspace_bootstrap,
            commands::workspace::save_workspace_settings,
            commands::workspace::get_drawings,
            commands::workspace::upsert_drawing,
            commands::workspace::delete_drawing,
            commands::workspace::clear_drawings,
            commands::workspace::get_pane_indicators,
            commands::workspace::save_pane_indicators,
            commands::workspace::list_layouts,
            commands::workspace::save_layout,
            commands::workspace::delete_layout,
            commands::workspace::list_watchlists,
            commands::workspace::create_watchlist,
            commands::workspace::delete_watchlist,
            commands::workspace::add_watchlist_symbol,
            commands::workspace::remove_watchlist_symbol,
            commands::workspace::list_alerts,
            commands::workspace::create_alert,
            commands::workspace::update_alert,
            commands::workspace::toggle_alert_status,
            commands::workspace::delete_alert,
            commands::workspace::check_price_alerts,
            commands::workspace::check_bar_alerts,
            commands::workspace::scan_watchlist_alerts,
            commands::workspace::list_alert_logs,
            commands::workspace::clear_alert_logs,
            commands::workspace::get_alert_delivery_config,
            commands::workspace::save_alert_delivery_config,
            commands::workspace::test_alert_delivery,
            commands::workspace::list_indicator_templates,
            commands::workspace::save_indicator_template,
            commands::workspace::delete_indicator_template,
            commands::workspace::open_detached_window,
            commands::workspace::broadcast_sync_event,
            // Causal Bar Replay (+ Save/Resume Sessions v2.2)
            commands::replay::replay_start,
            commands::replay::replay_step,
            commands::replay::replay_jump,
            commands::replay::replay_set_speed,
            commands::replay::replay_stop,
            commands::replay::replay_get_state,
            commands::replay::list_replay_sessions,
            commands::replay::save_replay_session,
            commands::replay::resume_replay_session,
            commands::replay::delete_replay_session,
            // Paper Broker (Vietnam VSD T+2 FIFO & Journal)
            commands::broker::broker_get_state,
            commands::broker::broker_submit_order,
            commands::broker::broker_cancel_order,
            commands::broker::broker_modify_order,
            commands::broker::broker_update_brackets,
            commands::broker::broker_flatten_position,
            commands::broker::broker_reset_account,
            commands::broker::broker_update_journal_note,
            // Strategy Tester (Backtest, Custom 101-Signal Rules, Portfolio, WFO, Monte Carlo, Sweep, Battle, HTML)
            commands::backtest::run_strategy_backtest,
            commands::backtest::run_strategy_portfolio,
            commands::backtest::run_strategy_wfo,
            commands::backtest::run_strategy_monte_carlo,
            commands::backtest::export_backtest_report_html,
            commands::backtest::run_strategy_parameter_sweep,
            commands::backtest::run_strategy_presets_battle,
            // Wave 3 Analytics: Breadth & RRG, Volume Structure, Trade Planner, Synthetic DOM, Journal & Debrief
            commands::analytics::get_market_breadth_and_rrg,
            commands::analytics::compute_volume_structure,
            commands::analytics::compute_trade_plan_cmd,
            commands::analytics::compute_dom_ladder_cmd,
            commands::analytics::broker_get_journal_analytics,
            commands::analytics::compute_replay_debrief_cmd,
            // Stock Screener (123 Signals + MTF 1W/1D/1H + Fundamentals + Saved Screens)
            commands::scanner::get_signal_catalog,
            commands::scanner::run_stock_screener,
            commands::scanner::list_saved_screens,
            commands::scanner::save_screen,
            commands::scanner::delete_screen,
            // AFC — Analog Forecast Cone (Dual-Horizon PIT Probability Fan)
            commands::analog_cone::compute_analog_cone,
            // Yumeko v3.0 Read-Only Top-Down Bridge & KT01 Evidence Exporter (Wave 2)
            commands::yumeko::yumeko_get_bridge_status,
            commands::yumeko::yumeko_get_topdown_context,
            commands::yumeko::yumeko_get_qmv_series,
            commands::yumeko::yumeko_export_kt01_verdict,
            commands::yumeko::yumeko_list_exported_verdicts,
            commands::yumeko::yumeko_verify_research_export,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Sumi Supercharts Tauri application");
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::state::resolve_database_paths;
    use crate::workspace_db::{
        PriceAlertRecord, SavedDrawingRecord, SavedIndicatorSpec, WorkspaceStore,
    };

    #[test]
    fn desktop_sqlite_persistence_full_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let ws_path = dir.path().join("sumi_workspace.db");
        let store = WorkspaceStore::open(&ws_path).unwrap();

        // Settings KV roundtrip
        let settings = json!({
            "theme": "dark",
            "gridLayout": "4",
            "sync": { "symbol": false, "interval": true, "crosshair": true }
        });
        store.set_kv("workspace_settings", &settings).unwrap();
        assert_eq!(store.get_kv("workspace_settings").unwrap(), Some(settings));

        // Layout save & reload
        let saved = store
            .save_layout("ly_1", "VN30 Quad Monitor", &json!({ "gridLayout": "4" }))
            .unwrap();
        assert_eq!(saved.name, "VN30 Quad Monitor");
        assert_eq!(store.list_layouts().unwrap().len(), 1);

        // Multiple custom drawings persisted per ticker
        for (id, name) in [
            ("d1", "longPosition"),
            ("d2", "shortPosition"),
            ("d3", "priceDateRange"),
            ("d4", "fibonacciExtension"),
        ] {
            store
                .upsert_drawing(SavedDrawingRecord {
                    id: id.to_owned(),
                    ticker: "HOSE:FPT".to_owned(),
                    name: name.to_owned(),
                    pane_id: "candle_pane".to_owned(),
                    group_id: "user_drawings".to_owned(),
                    points: json!([{"timestamp": 1700000000000_i64, "value": 120000.0}]),
                    styles: None,
                    extend_data: None,
                    lock: false,
                    visible: true,
                    mode: "weak_magnet".to_owned(),
                    updated_at_ms: 1700000000000,
                })
                .unwrap();
        }
        assert_eq!(store.list_drawings("HOSE:FPT").unwrap().len(), 4);
        store.delete_drawing("HOSE:FPT", "d2").unwrap();
        assert_eq!(store.list_drawings("HOSE:FPT").unwrap().len(), 3);

        // Pane indicators roundtrip
        store
            .save_pane_indicators(
                "pane-0",
                &[
                    SavedIndicatorSpec {
                        id: "i1".to_owned(),
                        name: "ICHIMOKU".to_owned(),
                        pane_id: "candle_pane".to_owned(),
                        is_main: true,
                        calc_params: vec![9.0, 26.0, 52.0],
                        visible: true,
                        styles: None,
                    },
                    SavedIndicatorSpec {
                        id: "i2".to_owned(),
                        name: "STOCHRSI".to_owned(),
                        pane_id: "pane_i2".to_owned(),
                        is_main: false,
                        calc_params: vec![14.0, 14.0, 3.0, 3.0],
                        visible: true,
                        styles: None,
                    },
                ],
            )
            .unwrap();
        assert_eq!(store.get_pane_indicators("pane-0").unwrap().len(), 2);

        // Alerts lifecycle
        store
            .upsert_alert(&PriceAlertRecord {
                id: "al_99".to_owned(),
                ticker: "HOSE:VCB".to_owned(),
                alert_kind: "price".to_owned(),
                condition: "cross_above".to_owned(),
                target_price: 95_000.0,
                interval: Some("1D".to_owned()),
                params_json: None,
                sound_enabled: true,
                message: "VCB break".to_owned(),
                status: "armed".to_owned(),
                frequency: "once".to_owned(),
                cooldown_bars: 0,
                last_fired_bar_ts: None,
                created_at_ms: 1700000000000,
                triggered_at_ms: None,
            })
            .unwrap();
        store
            .set_alert_status("al_99", "triggered", Some(1700000010000))
            .unwrap();
        let alerts = store.list_alerts().unwrap();
        assert_eq!(alerts[0].status, "triggered");
        assert_eq!(alerts[0].triggered_at_ms, Some(1700000010000));

        // Journal annotation roundtrip
        let ann = store
            .upsert_journal_annotation("HOSE:FPT", 1, "Breakout", "", "Tuân thủ SL chuẩn")
            .unwrap();
        assert_eq!(ann.setup_tag, "Breakout");
        assert_eq!(store.list_journal_annotations("HOSE:FPT").unwrap().len(), 1);

        // Database path resolver returns coherent canonical paths
        let (mkt, ws) = resolve_database_paths(Some(dir.path().to_path_buf()));
        assert!(mkt.ends_with("sumi_market.db"));
        assert!(ws.ends_with("sumi_workspace.db"));
    }

    #[test]
    fn non_destructive_legacy_migration_copies_wal_databases_with_sha256_receipt() {
        let tmp = tempfile::tempdir().unwrap();
        let legacy_dir = tmp.path().join("legacy_app_support");
        let target_dir = tmp.path().join("canonical_sumi_data");
        std::fs::create_dir_all(&legacy_dir).unwrap();

        let legacy_mkt_path = legacy_dir.join("sumi_market.db");
        let legacy_ws_path = legacy_dir.join("sumi_workspace.db");
        {
            let mkt = sumi_data::SqliteMarketStore::open(&legacy_mkt_path).unwrap();
            let sym = sumi_data::SymbolKey::new("HOSE", "FPT").unwrap();
            let key = sumi_data::SeriesKey::new(
                &sym,
                sumi_data::Interval::D1,
                sumi_data::Adjustment::Splits,
            );
            mkt.upsert_bars(
                &key,
                &[sumi_data::OhlcvBar::new(
                    1_700_000_000_000,
                    100_000.0,
                    102_000.0,
                    99_000.0,
                    101_000.0,
                    1_200_000.0,
                )],
                "cafef_adjusted",
                Some(true),
            )
            .unwrap();
            let ws = WorkspaceStore::open(&legacy_ws_path).unwrap();
            ws.create_watchlist("wl_legacy_test", "Legacy Test").unwrap();
        }

        let receipt = crate::state::migrate_legacy_databases_into(&target_dir, Some(&legacy_dir))
            .expect("migration receipt emitted");
        assert!(receipt.market_db.is_some());
        assert!(receipt.workspace_db.is_some());
        assert!(legacy_mkt_path.is_file(), "legacy DB must remain untouched");
        assert!(target_dir.join(".migration_v3_receipt.json").is_file());

        let migrated_mkt =
            sumi_data::SqliteMarketStore::open(target_dir.join("sumi_market.db")).unwrap();
        assert_eq!(migrated_mkt.stats().unwrap().total_bars, 1);
        let migrated_ws = WorkspaceStore::open(target_dir.join("sumi_workspace.db")).unwrap();
        assert!(
            migrated_ws
                .list_watchlists()
                .unwrap()
                .iter()
                .any(|w| w.id == "wl_legacy_test")
        );
    }

    #[test]
    fn curated_catalog_contains_vn_equities_for_cold_start_screener() {
        let catalog = crate::commands::market::curated_catalog();
        let vn_stocks: Vec<_> = catalog
            .iter()
            .filter(|c| {
                c.market_type == "stock" && matches!(c.exchange.as_str(), "HOSE" | "HNX" | "UPCOM")
            })
            .collect();
        assert!(vn_stocks.len() >= 15);
        assert!(vn_stocks.iter().any(|c| c.ticker == "HOSE:FPT"));
        assert!(vn_stocks.iter().any(|c| c.ticker == "HNX:SHS"));
        assert!(vn_stocks.iter().any(|c| c.ticker == "UPCOM:ACV"));
    }
}
