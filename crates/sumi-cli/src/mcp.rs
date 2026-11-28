//! JSON-RPC 2.0 stdio MCP Server (`sumi-mcp`, `protocolVersion = "2025-03-26"`).
//!
//! Exposes 5 quantitative `KT01` workstation tools to `agy acp`, Yumeko agents, Claude, and Codex:
//!   1. `sumi_kt01_analyze_symbol`
//!   2. `sumi_mtf_screen`
//!   3. `sumi_run_backtest`
//!   4. `sumi_market_breadth_rrg`
//!   5. `sumi_export_kt01_verdict`

use std::io::{self, BufRead, Write};
use std::path::Path;

use serde_json::{Value, json};

use crate::engine::{
    BacktestCliParams, ScreenCliParams, analyze_kt01_headless, compute_breadth_rrg_headless,
    export_kt01_headless, run_backtest_headless, run_mtf_screen_headless,
};

pub const MCP_PROTOCOL_VERSION: &str = "2025-03-26";
pub const MCP_SERVER_NAME: &str = "sumi-mcp";

/// Starts a blocking line-delimited JSON-RPC 2.0 MCP server on stdin/stdout.
pub fn run_stdio_mcp_server() -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    for line_res in stdin.lock().lines() {
        let line = line_res?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(resp) = handle_jsonrpc_line(trimmed, None, None) {
            writeln!(out, "{resp}")?;
            out.flush()?;
        }
    }
    Ok(())
}

/// Processes a single JSON-RPC 2.0 request line and returns `Some(response_json)` for requests
/// (or `None` for notifications without an `id`).
pub fn handle_jsonrpc_line(
    line: &str,
    market_db_override: Option<&Path>,
    data_dir_override: Option<&Path>,
) -> Option<String> {
    let req: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(err) => {
            return Some(
                json!({
                    "jsonrpc": "2.0",
                    "id": Value::Null,
                    "error": { "code": -32700, "message": format!("Parse error: {err}") }
                })
                .to_string(),
            );
        }
    };

    let id = req.get("id").cloned();
    let method = req.get("method").and_then(Value::as_str).unwrap_or("");
    let params = req.get("params").cloned().unwrap_or_else(|| json!({}));

    // Notifications (no id) require no response
    if id.is_none() && method.starts_with("notifications/") {
        return None;
    }
    let req_id = id.unwrap_or(Value::Null);

    let response = match method {
        "initialize" => json!({
            "jsonrpc": "2.0",
            "id": req_id,
            "result": {
                "protocolVersion": MCP_PROTOCOL_VERSION,
                "capabilities": {
                    "tools": { "listChanged": false }
                },
                "serverInfo": {
                    "name": MCP_SERVER_NAME,
                    "version": env!("CARGO_PKG_VERSION")
                }
            }
        }),
        "ping" => json!({
            "jsonrpc": "2.0",
            "id": req_id,
            "result": {}
        }),
        "tools/list" => json!({
            "jsonrpc": "2.0",
            "id": req_id,
            "result": {
                "tools": mcp_tool_definitions()
            }
        }),
        "tools/call" => {
            let name = params.get("name").and_then(Value::as_str).unwrap_or("");
            let args = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            match dispatch_tool(name, &args, market_db_override, data_dir_override) {
                Ok(payload) => json!({
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {
                        "content": [
                            {
                                "type": "text",
                                "text": serde_json::to_string_pretty(&payload).unwrap_or_default()
                            }
                        ],
                        "structuredContent": payload,
                        "isError": false
                    }
                }),
                Err(msg) => json!({
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {
                        "content": [{ "type": "text", "text": msg }],
                        "isError": true
                    }
                }),
            }
        }
        other => json!({
            "jsonrpc": "2.0",
            "id": req_id,
            "error": {
                "code": -32601,
                "message": format!("Method not found: {other}")
            }
        }),
    };

    Some(response.to_string())
}

pub fn mcp_tool_definitions() -> Value {
    json!([
        {
            "name": "sumi_kt01_analyze_symbol",
            "description": "Đánh giá cổng xác nhận kỹ thuật KT01 (1W/1D/1H + QMV Blackbox thật vs OHLCV Proxy + mức SL/TP1/TP2 chuẩn hóa VND & kVND, advisory_only = true).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "ticker": { "type": "string", "description": "Mã cổ phiếu (VD: HOSE:FPT hoặc FPT)" },
                    "as_of": { "type": "string", "description": "Ngày chốt Point-in-Time tùy chọn (YYYY-MM-DD)" }
                },
                "required": ["ticker"]
            }
        },
        {
            "name": "sumi_mtf_screen",
            "description": "Quét đa khung thời gian (1W + 1D) trên bộ 123 tín hiệu kỹ thuật QMV kết hợp Hộp đen QMV thật (`qmv_real`) & định giá DN01 (`P/E`, `ROE`) theo mốc `as_of`.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "scope": { "type": "string", "description": "ALL_VN | QMV60 | QMV30 | HOSE | HNX | UPCOM", "default": "QMV60" },
                    "signal_ids": { "type": "array", "items": { "type": "string" } },
                    "min_health_score": { "type": "number" },
                    "as_of": { "type": "string", "description": "Ngày chốt Point-in-Time tùy chọn (YYYY-MM-DD)" },
                    "weekly_regime": { "type": "string", "description": "uptrend | downtrend" },
                    "only_money_in": { "type": "boolean", "default": false, "description": "Chỉ lọc mã có Dòng tiền Vào hôm nay trên Hộp đen QMV thật" },
                    "min_tmc": { "type": "number", "description": "Ngưỡng tối thiểu TMC từ Hộp đen QMV thật" },
                    "max_pe": { "type": "number", "description": "Trần P/E tối đa (DN01 / cache)" },
                    "min_roe": { "type": "number", "description": "Sàn ROE % tối thiểu (DN01 / cache)" },
                    "limit": { "type": "integer", "default": 25 }
                }
            }
        },
        {
            "name": "sumi_run_backtest",
            "description": "Kiểm định chiến lược định lượng QMV (qmv_s1..qmv_s6, sma_ema, rsi_mean_rev, macd, bb_squeeze, donchian_atr) tuân thủ luật T+2 và biên độ trần/sàn Việt Nam.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "ticker": { "type": "string" },
                    "preset": { "type": "string", "default": "qmv_s1" },
                    "interval": { "type": "string", "default": "1D" },
                    "initial_cash": { "type": "number", "default": 500000000 },
                    "from_date": { "type": "string" },
                    "to_date": { "type": "string" }
                },
                "required": ["ticker"]
            }
        },
        {
            "name": "sumi_market_breadth_rrg",
            "description": "Tính độ rộng thị trường (Adv/Dec, %>MA20, %>EMA50, VN30 buy-flow) và tọa độ 4 góc phần tư RRG (Leading/Weakening/Lagging/Improving) theo mốc as_of.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "as_of": { "type": "string", "description": "Ngày chốt Point-in-Time tùy chọn (YYYY-MM-DD)" }
                }
            }
        },
        {
            "name": "sumi_export_kt01_verdict",
            "description": "Xuất gói bằng chứng kỹ thuật KT01GateVerdictV1 + TechnicalEvidenceBundleV1 (.json + .md có ký SHA-256, advisory_only = true) sang kho Sumi và tùy chọn dropzone Yumeko.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "ticker": { "type": "string" },
                    "as_of": { "type": "string" },
                    "copy_to_yumeko_dropzone": { "type": "boolean", "default": true }
                },
                "required": ["ticker"]
            }
        }
    ])
}

fn dispatch_tool(
    name: &str,
    args: &Value,
    market_db_override: Option<&Path>,
    data_dir_override: Option<&Path>,
) -> Result<Value, String> {
    match name {
        "sumi_kt01_analyze_symbol" => {
            let ticker = args
                .get("ticker")
                .or_else(|| args.get("symbol"))
                .and_then(Value::as_str)
                .ok_or_else(|| "Thiếu tham số bắt buộc `ticker`".to_owned())?;
            let as_of = args.get("as_of").and_then(Value::as_str);
            let out = analyze_kt01_headless(ticker, as_of, market_db_override, None)?;
            serde_json::to_value(out).map_err(|e| e.to_string())
        }
        "sumi_mtf_screen" => {
            let scope = args
                .get("scope")
                .and_then(Value::as_str)
                .unwrap_or("QMV60")
                .to_owned();
            let signal_ids = args
                .get("signal_ids")
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(ToOwned::to_owned))
                        .collect()
                })
                .unwrap_or_default();
            let min_health_score = args.get("min_health_score").and_then(Value::as_f64);
            let as_of = args
                .get("as_of")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned);
            let weekly_regime = args
                .get("weekly_regime")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned);
            let only_money_in = args
                .get("only_money_in")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let min_tmc = args.get("min_tmc").and_then(Value::as_f64);
            let max_pe = args.get("max_pe").and_then(Value::as_f64);
            let min_roe = args.get("min_roe").and_then(Value::as_f64);
            let limit = args
                .get("limit")
                .and_then(Value::as_u64)
                .map_or(25, |n| n as usize);
            let res = run_mtf_screen_headless(
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
                market_db_override,
            )?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "sumi_run_backtest" => {
            let ticker = args
                .get("ticker")
                .or_else(|| args.get("symbol"))
                .and_then(Value::as_str)
                .ok_or_else(|| "Thiếu tham số bắt buộc `ticker`".to_owned())?
                .to_owned();
            let preset = args
                .get("preset")
                .and_then(Value::as_str)
                .unwrap_or("qmv_s1")
                .to_owned();
            let interval = args
                .get("interval")
                .and_then(Value::as_str)
                .unwrap_or("1D")
                .to_owned();
            let initial_cash = args
                .get("initial_cash")
                .and_then(Value::as_f64)
                .unwrap_or(500_000_000.0);
            let from_date = args
                .get("from_date")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned);
            let to_date = args
                .get("to_date")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned);
            let res = run_backtest_headless(
                &BacktestCliParams {
                    ticker,
                    preset,
                    interval,
                    initial_cash,
                    from_date,
                    to_date,
                },
                market_db_override,
            )?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "sumi_market_breadth_rrg" => {
            let as_of = args.get("as_of").and_then(Value::as_str);
            let snap = compute_breadth_rrg_headless(as_of, market_db_override)?;
            serde_json::to_value(snap).map_err(|e| e.to_string())
        }
        "sumi_export_kt01_verdict" => {
            let ticker = args
                .get("ticker")
                .or_else(|| args.get("symbol"))
                .and_then(Value::as_str)
                .ok_or_else(|| "Thiếu tham số bắt buộc `ticker`".to_owned())?;
            let as_of = args.get("as_of").and_then(Value::as_str);
            let copy_drop = args
                .get("copy_to_yumeko_dropzone")
                .and_then(Value::as_bool)
                .unwrap_or(true);
            let receipt = export_kt01_headless(
                ticker,
                as_of,
                copy_drop,
                market_db_override,
                data_dir_override,
            )?;
            serde_json::to_value(receipt).map_err(|e| e.to_string())
        }
        other => Err(format!("Unknown Sumi MCP tool: {other}")),
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use sumi_data::{Adjustment, Interval, OhlcvBar, SeriesKey, SqliteMarketStore, SymbolKey};

    use super::*;

    #[test]
    fn mcp_jsonrpc_initialize_tools_list_and_kt01_call_succeed() {
        let tmp = tempfile::tempdir().unwrap();
        let db_path = tmp.path().join("sumi_market.db");
        let store = SqliteMarketStore::open(&db_path).unwrap();
        let sym = SymbolKey::new("HOSE", "FPT").unwrap();
        let base_ms = NaiveDate::from_ymd_opt(2026, 6, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp_millis();
        let bars: Vec<OhlcvBar> = (0..80_i64)
            .map(|i| {
                let p = 120_000.0 + (i as f64) * 200.0;
                OhlcvBar::new(
                    base_ms + i * 86_400_000,
                    p - 300.0,
                    p + 700.0,
                    p - 700.0,
                    p + 100.0,
                    1_500_000.0,
                )
            })
            .collect();
        store
            .upsert_bars(
                &SeriesKey::new(&sym, Interval::D1, Adjustment::Splits),
                &bars,
                "seed",
                Some(true),
            )
            .unwrap();

        // 1. initialize
        let init_resp: Value = serde_json::from_str(
            &handle_jsonrpc_line(
                r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
                Some(&db_path),
                Some(tmp.path()),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(init_resp["result"]["protocolVersion"], MCP_PROTOCOL_VERSION);
        assert_eq!(init_resp["result"]["serverInfo"]["name"], MCP_SERVER_NAME);

        // 2. tools/list
        let list_resp: Value = serde_json::from_str(
            &handle_jsonrpc_line(
                r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#,
                Some(&db_path),
                Some(tmp.path()),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(list_resp["result"]["tools"].as_array().unwrap().len(), 5);

        // 3. tools/call -> sumi_kt01_analyze_symbol
        let call_resp: Value = serde_json::from_str(
            &handle_jsonrpc_line(
                r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"sumi_kt01_analyze_symbol","arguments":{"ticker":"HOSE:FPT"}}}"#,
                Some(&db_path),
                Some(tmp.path()),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(call_resp["result"]["isError"], false);
        assert_eq!(call_resp["result"]["structuredContent"]["verdict"]["advisoryOnly"], true);

        // 4. tools/call -> sumi_export_kt01_verdict (no dropzone copy in test)
        let exp_resp: Value = serde_json::from_str(
            &handle_jsonrpc_line(
                r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"sumi_export_kt01_verdict","arguments":{"ticker":"HOSE:FPT","copy_to_yumeko_dropzone":false}}}"#,
                Some(&db_path),
                Some(tmp.path()),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(exp_resp["result"]["isError"], false);
        let json_p = exp_resp["result"]["structuredContent"]["jsonPath"]
            .as_str()
            .unwrap();
        assert!(std::path::Path::new(json_p).is_file());
    }
}
