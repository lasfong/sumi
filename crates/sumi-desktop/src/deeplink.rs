//! Deep-Link (`sumi://inspect?symbol=FPT&as_of=2026-11-26&tf=1D`) parser and
//! Authenticated Single-Instance Loopback IPC (`<SUMI_DATA_DIR>/runtime.lock` `0600`).
//!
//! Allows upstream **Yumeko.app** (or terminal / macOS LaunchServices / Windows `HKCU\...\sumi`)
//! to focus an already-running Sumi Desktop window and switch directly to the requested symbol,
//! timeframe, and Point-in-Time `as_of` date without spawning duplicate instances.

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sumi_data::{infer_vn_exchange_for_ticker, sumi_data_dir, write_private_file_0600};
use tauri::{AppHandle, Emitter, Manager};
use url::Url;
use uuid::Uuid;

pub const DEEPLINK_EVENT_NAME: &str = "sumi:deeplink";
const RUNTIME_LOCK_FILE: &str = "runtime.lock";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeepLinkPayload {
    pub raw_url: String,
    pub action: String,
    pub ticker: String,
    pub symbol: String,
    pub as_of: Option<String>,
    pub interval: Option<String>,
    pub tab: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RuntimeLockRecord {
    pid: u32,
    port: u16,
    nonce: String,
    started_at_ms: i64,
}

/// Parses `sumi://inspect?symbol=FPT&as_of=2026-11-26&tf=1D` (or `HOSE:FPT`, `interval=...`, `tab=...`).
#[must_use]
pub fn parse_sumi_deeplink(raw: &str) -> Option<DeepLinkPayload> {
    let trimmed = raw.trim();
    if !trimmed.to_ascii_lowercase().starts_with("sumi://") {
        return None;
    }
    let parsed = Url::parse(trimmed).ok()?;
    let action = parsed
        .host_str()
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_else(|| "inspect".to_owned());

    let mut raw_sym: Option<String> = None;
    let mut as_of: Option<String> = None;
    let mut interval: Option<String> = None;
    let mut tab: Option<String> = None;

    for (k, v) in parsed.query_pairs() {
        let val = v.trim();
        if val.is_empty() {
            continue;
        }
        match k.to_ascii_lowercase().as_str() {
            "symbol" | "ticker" | "sym" => raw_sym = Some(val.to_ascii_uppercase()),
            "as_of" | "asof" | "date" => {
                if val.len() == 10 && chrono::NaiveDate::parse_from_str(val, "%Y-%m-%d").is_ok() {
                    as_of = Some(val.to_owned());
                }
            }
            "tf" | "interval" | "timeframe" => interval = Some(normalize_interval_param(val)),
            "tab" | "panel" => tab = Some(val.to_ascii_lowercase()),
            _ => {}
        }
    }

    // Fallback: `sumi://inspect/FPT`
    if raw_sym.is_none() {
        let seg = parsed.path().trim_matches('/');
        if !seg.is_empty() {
            raw_sym = Some(seg.to_ascii_uppercase());
        }
    }

    let raw_sym = raw_sym?;
    let (ticker, symbol) = if let Some((ex, sym)) = raw_sym.split_once(':') {
        (format!("{ex}:{sym}"), sym.to_owned())
    } else {
        let is_idx = matches!(
            raw_sym.as_str(),
            "VNINDEX" | "VN30" | "HNXINDEX" | "HNX30" | "UPCOMINDEX" | "VNXALL"
        );
        let ex = infer_vn_exchange_for_ticker(&raw_sym, is_idx);
        (format!("{ex}:{raw_sym}"), raw_sym)
    };

    Some(DeepLinkPayload {
        raw_url: trimmed.to_owned(),
        action,
        ticker,
        symbol,
        as_of,
        interval,
        tab,
    })
}

fn normalize_interval_param(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "1m" | "m1" => "1m".to_owned(),
        "5m" | "m5" => "5m".to_owned(),
        "15m" | "m15" => "15m".to_owned(),
        "30m" | "m30" => "30m".to_owned(),
        "1h" | "h1" | "60m" => "1h".to_owned(),
        "4h" | "h4" | "240m" => "4h".to_owned(),
        "1w" | "w1" | "w" => "1W".to_owned(),
        "1mo" | "mn1" | "month" => "1M".to_owned(),
        _ if raw.trim() == "1M" => "1M".to_owned(),
        _ => "1D".to_owned(),
    }
}

/// Scans process CLI arguments for a `sumi://...` URI.
#[must_use]
pub fn extract_deeplink_from_args(args: &[String]) -> Option<String> {
    args.iter()
        .skip(1)
        .find(|a| a.trim().to_ascii_lowercase().starts_with("sumi://"))
        .cloned()
}

fn lock_path_for(data_dir: &Path) -> PathBuf {
    data_dir.join(RUNTIME_LOCK_FILE)
}

/// Attempts to forward a `sumi://` URI to an already-running Sumi Desktop instance over loopback.
/// Returns `true` if delivered and acknowledged (`200 OK`), meaning the secondary process can exit.
pub fn try_forward_to_running_instance(raw_url: &str, data_dir: Option<&Path>) -> bool {
    let dir = data_dir.map(Path::to_path_buf).unwrap_or_else(sumi_data_dir);
    let lp = lock_path_for(&dir);
    let Ok(raw) = fs::read_to_string(&lp) else {
        return false;
    };
    let Ok(rec) = serde_json::from_str::<RuntimeLockRecord>(&raw) else {
        return false;
    };
    let addr = SocketAddr::from(([127, 0, 0, 1], rec.port));
    let Ok(mut stream) = TcpStream::connect_timeout(&addr, Duration::from_millis(450)) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_millis(800)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(800)));

    let body = serde_json::json!({ "url": raw_url }).to_string();
    let req = format!(
        "POST /deeplink HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nX-Sumi-Nonce: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        rec.port,
        rec.nonce,
        body.len(),
        body
    );
    if stream.write_all(req.as_bytes()).is_err() {
        return false;
    }
    let mut resp = String::new();
    let _ = stream.read_to_string(&mut resp);
    resp.starts_with("HTTP/1.1 200")
}

/// Starts the authenticated loopback listener on `127.0.0.1:0`, writes `runtime.lock` (`0600`),
/// and forwards valid `sumi://` invocations to the Tauri main window as `"sumi:deeplink"`.
pub fn start_single_instance_ipc_listener(app: AppHandle, data_dir: PathBuf) {
    let Ok(listener) = TcpListener::bind("127.0.0.1:0") else {
        return;
    };
    let Ok(local_addr) = listener.local_addr() else {
        return;
    };
    let nonce = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let lock = RuntimeLockRecord {
        pid: std::process::id(),
        port: local_addr.port(),
        nonce: nonce.clone(),
        started_at_ms: chrono::Utc::now().timestamp_millis(),
    };
    if let Ok(bytes) = serde_json::to_vec_pretty(&lock) {
        let _ = write_private_file_0600(&lock_path_for(&data_dir), &bytes);
    }

    std::thread::Builder::new()
        .name("sumi-ipc-loopback".to_owned())
        .spawn(move || {
            for stream in listener.incoming().flatten() {
                handle_ipc_connection(stream, &nonce, &app);
            }
        })
        .ok();
}

fn handle_ipc_connection(mut stream: TcpStream, expected_nonce: &str, app: &AppHandle) {
    let _ = stream.set_read_timeout(Some(Duration::from_millis(800)));
    let mut reader = BufReader::new(match stream.try_clone() {
        Ok(s) => s,
        Err(_) => return,
    });

    let mut req_line = String::new();
    if reader.read_line(&mut req_line).is_err() {
        return;
    }
    let mut content_len: usize = 0;
    let mut provided_nonce = String::new();
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    break;
                }
                if let Some((k, v)) = trimmed.split_once(':') {
                    if k.eq_ignore_ascii_case("content-length") {
                        content_len = v.trim().parse().unwrap_or(0).min(8_192);
                    } else if k.eq_ignore_ascii_case("x-sumi-nonce") {
                        provided_nonce = v.trim().to_owned();
                    }
                }
            }
        }
    }

    if provided_nonce != expected_nonce {
        let _ = stream.write_all(b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\n\r\n");
        return;
    }

    let mut body = vec![0_u8; content_len];
    if content_len > 0 && reader.read_exact(&mut body).is_err() {
        let _ = stream.write_all(b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n");
        return;
    }

    let url_str = serde_json::from_slice::<serde_json::Value>(&body)
        .ok()
        .and_then(|v| v.get("url").and_then(|u| u.as_str()).map(ToOwned::to_owned));

    if let Some(raw_url) = url_str
        && let Some(payload) = parse_sumi_deeplink(&raw_url)
    {
        let _ = app.emit(DEEPLINK_EVENT_NAME, &payload);
        if let Some(win) = app.get_webview_window("main") {
            let _ = win.unminimize();
            let _ = win.show();
            let _ = win.set_focus();
        }
        let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nOK");
    } else {
        let _ = stream.write_all(b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sumi_inspect_deeplinks_with_exchange_inference_and_as_of() {
        let p = parse_sumi_deeplink("sumi://inspect?symbol=fpt&as_of=2026-11-26&tf=1h&tab=yumeko")
            .expect("valid deeplink");
        assert_eq!(p.action, "inspect");
        assert_eq!(p.ticker, "HOSE:FPT");
        assert_eq!(p.symbol, "FPT");
        assert_eq!(p.as_of.as_deref(), Some("2026-11-26"));
        assert_eq!(p.interval.as_deref(), Some("1h"));
        assert_eq!(p.tab.as_deref(), Some("yumeko"));

        let p2 = parse_sumi_deeplink("sumi://inspect?ticker=HNX:PVS&tf=1W").unwrap();
        assert_eq!(p2.ticker, "HNX:PVS");
        assert_eq!(p2.interval.as_deref(), Some("1W"));
        assert!(p2.as_of.is_none());

        assert!(parse_sumi_deeplink("https://example.com").is_none());
    }
}
