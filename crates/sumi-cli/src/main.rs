//! Headless Sumi Supercharts v3.0 CLI & MCP Server (`sumi`).
//!
//! Usage:
//!   sumi mcp | --mcp-serve
//!   sumi bridge status [--json]
//!   sumi sync [--status | --now [--folder <dir>] | --export <path> | --import <path>] [--json]
//!   sumi kt01 --symbol HOSE:FPT [--as-of YYYY-MM-DD] [--export] [--json]
//!   sumi screen [--scope ALL_VN|QMV60|QMV30] [--signal <id>] [--min-health <f64>]
//!               [--only-money-in] [--min-tmc <f64>] [--max-pe <f64>] [--min-roe <f64>]
//!               [--weekly-regime uptrend|downtrend] [--as-of YYYY-MM-DD] [--limit <N>]
//!   sumi backtest --symbol HOSE:FPT [--preset qmv_s1] [--interval 1D] [--cash 500000000]
//!   sumi breadth [--as-of YYYY-MM-DD]

use std::path::PathBuf;
use std::process::ExitCode;

use sumi_cli::engine::{
    BacktestCliParams, ScreenCliParams, analyze_kt01_headless, compute_breadth_rrg_headless,
    export_kt01_headless, run_backtest_headless, run_mtf_screen_headless,
};
use sumi_cli::mcp::run_stdio_mcp_server;
use sumi_cli::sync_cmd::{
    sync_export_headless, sync_folder_now_headless, sync_import_headless, sync_status_headless,
};
use sumi_yumeko_bridge::{YumekoReadOnlyBridge, render_kt01_markdown};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() <= 1 {
        print_help();
        return ExitCode::SUCCESS;
    }

    match args[1].as_str() {
        "-h" | "--help" | "help" => {
            print_help();
            ExitCode::SUCCESS
        }
        "-V" | "--version" | "version" => {
            println!("sumi {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        "mcp" | "--mcp-serve" => {
            if let Err(err) = run_stdio_mcp_server() {
                eprintln!("sumi-mcp error: {err}");
                return ExitCode::FAILURE;
            }
            ExitCode::SUCCESS
        }
        "bridge" => run_bridge_cmd(&args[2..]),
        "sync" => run_sync_cmd(&args[2..]),
        "kt01" => run_kt01_cmd(&args[2..]),
        "screen" => run_screen_cmd(&args[2..]),
        "backtest" => run_backtest_cmd(&args[2..]),
        "breadth" => run_breadth_cmd(&args[2..]),
        other => {
            eprintln!("Unknown subcommand: {other}\n");
            print_help();
            ExitCode::FAILURE
        }
    }
}

fn run_bridge_cmd(args: &[String]) -> ExitCode {
    let sub = args.first().map(String::as_str).unwrap_or("status");
    if sub != "status" && !sub.starts_with("--") {
        eprintln!("Unknown bridge subcommand: {sub} (expected: `sumi bridge status [--json]`)");
        return ExitCode::FAILURE;
    }
    let want_json = has_flag(args, &["--json"]);
    let bridge = match flag_val(args, &["--yumeko-db"]) {
        Some(p) => YumekoReadOnlyBridge::new(PathBuf::from(p), None),
        None => YumekoReadOnlyBridge::discover_default(),
    };
    let status = match bridge.get_status() {
        Ok(s) => s,
        Err(err) => {
            eprintln!("Yumeko bridge status error: {err}");
            return ExitCode::FAILURE;
        }
    };
    if want_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&status).unwrap_or_default()
        );
    } else {
        println!(
            "Yumeko Bridge (READ-ONLY): {}",
            if status.available {
                "CONNECTED"
            } else {
                "OFFLINE / CLEAN-SLATE"
            }
        );
        println!("  SQLite path:       {}", status.sqlite_path);
        if let Some(qmv_path) = status.qmv_shared_path.as_deref() {
            println!("  QMV shared path:   {qmv_path}");
        }
        println!(
            "  QMV symbols:       tech={} | active_money={} | foreign_flow={}",
            status.qmv_technical_symbols,
            status.qmv_active_money_symbols,
            status.qmv_foreign_flow_symbols
        );
        println!(
            "  DN01 / DNSE / VM:  valuations={} | dnse_history={} | macro_obs={} | portfolios={}",
            status.valuation_symbols,
            status.dnse_history_symbols,
            status.macro_observations_count,
            status.portfolio_snapshots_count
        );
        println!(
            "  Latest as_of:      {}",
            status
                .latest_as_of_dates
                .first()
                .map(String::as_str)
                .unwrap_or("—")
        );
        if !status.latest_as_of_dates.is_empty() {
            let preview: Vec<&str> = status
                .latest_as_of_dates
                .iter()
                .take(8)
                .map(String::as_str)
                .collect();
            println!("  Available as_of:   {}", preview.join(", "));
        }
        if !status.note.is_empty() {
            println!("  Note:              {}", status.note);
        }
    }
    ExitCode::SUCCESS
}

fn run_sync_cmd(args: &[String]) -> ExitCode {
    let want_json = has_flag(args, &["--json"]);
    if let Some(out_str) = flag_val(args, &["--export"]) {
        let p = PathBuf::from(out_str);
        return match sync_export_headless(Some(&p), None, None) {
            Ok(res) => {
                println!("{}", serde_json::to_string_pretty(&res).unwrap_or_default());
                ExitCode::SUCCESS
            }
            Err(err) => {
                eprintln!("Sync export error: {err}");
                ExitCode::FAILURE
            }
        };
    }
    if let Some(in_str) = flag_val(args, &["--import"]) {
        let p = PathBuf::from(in_str);
        return match sync_import_headless(&p, None, None) {
            Ok(res) => {
                println!("{}", serde_json::to_string_pretty(&res).unwrap_or_default());
                ExitCode::SUCCESS
            }
            Err(err) => {
                eprintln!("Sync import error: {err}");
                ExitCode::FAILURE
            }
        };
    }
    if has_flag(args, &["--now"]) {
        let folder = flag_val(args, &["--folder"]).map(PathBuf::from);
        return match sync_folder_now_headless(folder.as_deref(), None, None, None) {
            Ok(res) => {
                println!("{}", serde_json::to_string_pretty(&res).unwrap_or_default());
                ExitCode::SUCCESS
            }
            Err(err) => {
                eprintln!("Sync folder error: {err}");
                ExitCode::FAILURE
            }
        };
    }

    let status = sync_status_headless(None, None);
    if want_json {
        println!("{}", serde_json::to_string_pretty(&status).unwrap_or_default());
    } else {
        println!("Sumi Workspace Sync Status");
        println!("  Device ID:         {}", status.device_id);
        println!(
            "  Sync folder:       {}",
            status.sync_folder.as_deref().unwrap_or("(chưa cấu hình)")
        );
        println!("  Bundle exists:     {}", status.sync_bundle_exists);
        println!("  Auto-sync launch:  {}", status.auto_sync_on_launch);
        println!(
            "  Last SHA-256:      {}",
            status.last_synced_sha256.as_deref().unwrap_or("—")
        );
        println!("  Workspace DB:      {} (exists={})", status.workspace_db_path, status.workspace_db_exists);
        println!("  Pre-sync backups:  {}", status.pre_sync_backup_count);
    }
    ExitCode::SUCCESS
}

fn flag_val(args: &[String], names: &[&str]) -> Option<String> {
    let mut i = 0;
    while i < args.len() {
        if names.iter().any(|n| args[i] == *n) && i + 1 < args.len() {
            return Some(args[i + 1].clone());
        }
        i += 1;
    }
    None
}

fn has_flag(args: &[String], names: &[&str]) -> bool {
    args.iter().any(|a| names.iter().any(|n| a == n))
}

fn run_kt01_cmd(args: &[String]) -> ExitCode {
    let symbol = flag_val(args, &["--symbol", "-s", "--ticker"])
        .unwrap_or_else(|| "HOSE:FPT".to_owned());
    let as_of = flag_val(args, &["--as-of", "--as_of"]);
    let want_export = has_flag(args, &["--export", "-e"]);
    let want_json = has_flag(args, &["--json"]);
    let copy_dropzone = !has_flag(args, &["--no-dropzone"]);

    if want_export {
        match export_kt01_headless(&symbol, as_of.as_deref(), copy_dropzone, None, None) {
            Ok(receipt) => {
                if want_json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&receipt).unwrap_or_default()
                    );
                } else {
                    println!("{}", receipt.markdown_summary);
                    println!("JSON: {}", receipt.json_path);
                    println!("MD:   {}", receipt.markdown_path);
                    if let Some(dz) = receipt.yumeko_dropzone_path {
                        println!("Yumeko dropzone: {dz}");
                    }
                }
                ExitCode::SUCCESS
            }
            Err(err) => {
                eprintln!("KT01 export error: {err}");
                ExitCode::FAILURE
            }
        }
    } else {
        match analyze_kt01_headless(&symbol, as_of.as_deref(), None, None) {
            Ok(out) => {
                if want_json {
                    println!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
                } else {
                    println!("{}", render_kt01_markdown(&out.verdict));
                }
                ExitCode::SUCCESS
            }
            Err(err) => {
                eprintln!("KT01 analyze error: {err}");
                ExitCode::FAILURE
            }
        }
    }
}

fn run_screen_cmd(args: &[String]) -> ExitCode {
    let scope = flag_val(args, &["--scope", "--exchange"]).unwrap_or_else(|| "ALL_VN".to_owned());
    let signal_ids: Vec<String> = flag_val(args, &["--signal", "--signals"])
        .map(|s| {
            s.split(',')
                .map(|x| x.trim().to_owned())
                .filter(|x| !x.is_empty())
                .collect()
        })
        .unwrap_or_default();
    let min_health_score = flag_val(args, &["--min-health"]).and_then(|s| s.parse::<f64>().ok());
    let as_of = flag_val(args, &["--as-of", "--as_of"]);
    let weekly_regime = flag_val(args, &["--weekly-regime"]);
    let only_money_in = has_flag(args, &["--only-money-in"]);
    let min_tmc = flag_val(args, &["--min-tmc"]).and_then(|s| s.parse::<f64>().ok());
    let max_pe = flag_val(args, &["--max-pe"]).and_then(|s| s.parse::<f64>().ok());
    let min_roe = flag_val(args, &["--min-roe"]).and_then(|s| s.parse::<f64>().ok());
    let limit = flag_val(args, &["--limit"])
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(25);

    match run_mtf_screen_headless(
        &ScreenCliParams {
            scope,
            signal_ids,
            min_health_score,
            as_of,
            weekly_regime,
            only_money_in,
            min_tmc,
            max_pe,
            min_roe,
            limit,
        },
        None,
    ) {
        Ok(res) => {
            println!("{}", serde_json::to_string_pretty(&res).unwrap_or_default());
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("Screen error: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run_backtest_cmd(args: &[String]) -> ExitCode {
    let ticker = flag_val(args, &["--symbol", "-s", "--ticker"])
        .unwrap_or_else(|| "HOSE:FPT".to_owned());
    let preset = flag_val(args, &["--preset", "-p"]).unwrap_or_else(|| "qmv_s1".to_owned());
    let interval = flag_val(args, &["--interval", "--tf"]).unwrap_or_else(|| "1D".to_owned());
    let initial_cash = flag_val(args, &["--cash"])
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(500_000_000.0);

    match run_backtest_headless(
        &BacktestCliParams {
            ticker,
            preset,
            interval,
            initial_cash,
            from_date: flag_val(args, &["--from"]),
            to_date: flag_val(args, &["--to"]),
        },
        None,
    ) {
        Ok(res) => {
            println!("{}", serde_json::to_string_pretty(&res).unwrap_or_default());
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("Backtest error: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run_breadth_cmd(args: &[String]) -> ExitCode {
    let as_of = flag_val(args, &["--as-of", "--as_of"]);
    match compute_breadth_rrg_headless(as_of.as_deref(), None) {
        Ok(snap) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&snap).unwrap_or_default()
            );
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("Breadth/RRG error: {err}");
            ExitCode::FAILURE
        }
    }
}

fn print_help() {
    println!(
        "Sumi Supercharts v{} — Headless QMV KT01 CLI & MCP Server\n\
         \n\
         COMMANDS:\n\
           sumi mcp | --mcp-serve                                Start JSON-RPC 2.0 stdio MCP server (5 tools)\n\
           sumi bridge status [--json]                           Inspect read-only Yumeko SQLite bridge & as_of dates\n\
           sumi sync [--status | --now [--folder <dir>]          Cross-device Workspace Sync (.sumi.json LWW + SHA-256)\n\
                     | --export <path> | --import <path>] [--json]\n\
           sumi kt01 --symbol <TICKER> [--as-of YYYY-MM-DD]      Evaluate KT01 Confirmation Gate\n\
                     [--export] [--json] [--no-dropzone]\n\
           sumi screen [--scope ALL_VN|QMV60|QMV30]              Run MTF 1W+1D 123-signal + QMV Blackbox + DN01 screener\n\
                       [--signal id1,id2] [--min-health 20] [--only-money-in]\n\
                       [--min-tmc <f64>] [--max-pe <f64>] [--min-roe <f64>]\n\
                       [--weekly-regime uptrend|downtrend] [--as-of YYYY-MM-DD] [--limit 25]\n\
           sumi backtest --symbol <TICKER> [--preset qmv_s1]     Run VN T+2 quantitative backtest\n\
                         [--interval 1D] [--cash 500000000]\n\
           sumi breadth [--as-of YYYY-MM-DD]                     Compute Market Breadth & 4-Quadrant RRG",
        env!("CARGO_PKG_VERSION")
    );
}
