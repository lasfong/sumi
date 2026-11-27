//! Headless Sumi Supercharts v3.0 CLI & MCP Server (`sumi`).
//!
//! Usage:
//!   sumi mcp | --mcp-serve
//!   sumi kt01 --symbol HOSE:FPT [--as-of YYYY-MM-DD] [--export] [--json]
//!   sumi screen [--scope ALL_VN|QMV60|QMV30] [--signal <id>] [--min-health <f64>] [--limit <N>]
//!   sumi backtest --symbol HOSE:FPT [--preset qmv_s1] [--interval 1D] [--cash 500000000]
//!   sumi breadth [--as-of YYYY-MM-DD]

use std::process::ExitCode;

use sumi_cli::engine::{
    BacktestCliParams, ScreenCliParams, analyze_kt01_headless, compute_breadth_rrg_headless,
    export_kt01_headless, run_backtest_headless, run_mtf_screen_headless,
};
use sumi_cli::mcp::run_stdio_mcp_server;
use sumi_yumeko_bridge::render_kt01_markdown;

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
    let limit = flag_val(args, &["--limit"])
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(25);

    match run_mtf_screen_headless(
        &ScreenCliParams {
            scope,
            signal_ids,
            min_health_score,
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
           sumi kt01 --symbol <TICKER> [--as-of YYYY-MM-DD]      Evaluate KT01 Confirmation Gate\n\
                     [--export] [--json] [--no-dropzone]\n\
           sumi screen [--scope ALL_VN|QMV60|QMV30]              Run MTF 1W+1D 123-signal screener\n\
                       [--signal id1,id2] [--min-health 20] [--limit 25]\n\
           sumi backtest --symbol <TICKER> [--preset qmv_s1]     Run VN T+2 quantitative backtest\n\
                         [--interval 1D] [--cash 500000000]\n\
           sumi breadth [--as-of YYYY-MM-DD]                     Compute Market Breadth & 4-Quadrant RRG",
        env!("CARGO_PKG_VERSION")
    );
}
