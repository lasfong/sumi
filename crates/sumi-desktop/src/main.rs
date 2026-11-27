// Prevents additional console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--mcp-serve") {
        if let Err(err) = sumi_cli::mcp::run_stdio_mcp_server() {
            eprintln!("sumi-mcp error: {err}");
            std::process::exit(1);
        }
        return;
    }
    sumi_desktop_lib::run();
}
