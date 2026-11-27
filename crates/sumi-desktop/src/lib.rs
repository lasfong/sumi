#![warn(clippy::all, rust_2018_idioms)]

pub mod commands;
pub mod error;
pub mod state;
pub mod workspace_db;

use tauri::Manager;

use crate::state::{AppState, resolve_database_paths};

/// Starts the Tauri v2 Supercharts Desktop Workstation runtime.
///
/// # Panics
/// Panics if the Tauri application fails to initialize or bind its native webview window.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    sumi_data::init_tracing("info");

    tauri::Builder::default()
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir().ok();
            let (market_db, workspace_db) = resolve_database_paths(app_data_dir);
            let app_state = AppState::initialize(market_db, workspace_db)
                .map_err(|e| std::io::Error::other(e.to_string()))?;
            app.manage(app_state);
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
            commands::workspace::toggle_alert_status,
            commands::workspace::delete_alert,
            commands::workspace::check_price_alerts,
            commands::workspace::check_bar_alerts,
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
            // Strategy Tester (Backtest, Custom 101-Signal Rules, Portfolio, WFO, Monte Carlo)
            commands::backtest::run_strategy_backtest,
            commands::backtest::run_strategy_portfolio,
            commands::backtest::run_strategy_wfo,
            commands::backtest::run_strategy_monte_carlo,
            // Stock Screener (101 Signals)
            commands::scanner::get_signal_catalog,
            commands::scanner::run_stock_screener,
            // AFC — Analog Forecast Cone (Dual-Horizon PIT Probability Fan)
            commands::analog_cone::compute_analog_cone,
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

        // Database path resolver returns coherent paths
        let (mkt, ws) = resolve_database_paths(Some(dir.path().to_path_buf()));
        assert!(mkt.ends_with("sumi_market.db"));
        assert!(ws.ends_with("sumi_workspace.db"));
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
