//! Bundled workstation OHLCV presets exercising synthetic benchmarks, intraday `1m` futures,
//! real multi-symbol CafeF CSV (`HNX-INDEX` + `VNINDEX`), real 5-column NYSE `SPY.csv`,
//! and 24/7 weekend-inclusive `BTCUSDT` `15m` JSON klines (`Milestone R06`).

use std::fmt::Write as _;

use serde::{Deserialize, Serialize};

use crate::model::Candle;

use super::calendar::{days_from_civil, weekday_from_days};
use super::csv_loader::load_ohlcv_csv;
use super::json_loader::load_ohlcv_json;
use super::synthetic::generate_synthetic_ohlcv;
use super::timeframe::Timeframe;
use super::{DataLoadOptions, DataLoadReport};

/// Built-in workstation data feeds selectable from the top toolbar (`Milestone R06`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum DataSourcePreset {
    #[default]
    SyntheticVnindex,
    IntradayVn30F1m,
    CafeFIndexCsv,
    SpyDailyCsv,
    BtcUsdt15mJson,
    CustomFile,
}

impl DataSourcePreset {
    pub const TOOLBAR_PRESETS: [Self; 5] = [
        Self::SyntheticVnindex,
        Self::IntradayVn30F1m,
        Self::CafeFIndexCsv,
        Self::SpyDailyCsv,
        Self::BtcUsdt15mJson,
    ];

    #[inline]
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::SyntheticVnindex => "VNINDEX Synth",
            Self::IntradayVn30F1m => "VN30F1M 1m",
            Self::CafeFIndexCsv => "CafeF CSV",
            Self::SpyDailyCsv => "SPY CSV",
            Self::BtcUsdt15mJson => "BTC JSON",
            Self::CustomFile => "File",
        }
    }
}

/// Loads one of the built-in workstation presets into a [`DataLoadReport`].
#[must_use]
pub fn load_preset_dataset(
    preset: DataSourcePreset,
    synthetic_bar_count: usize,
    symbol_override: Option<&str>,
) -> DataLoadReport {
    match preset {
        DataSourcePreset::SyntheticVnindex | DataSourcePreset::CustomFile => {
            let count = synthetic_bar_count.max(10);
            let candles = generate_synthetic_ohlcv(count);
            DataLoadReport {
                symbol: "VNINDEX".to_owned(),
                available_symbols: vec!["VNINDEX".to_owned()],
                base_timeframe: Timeframe::D1,
                candles,
                raw_row_count: count,
                deduplicated_count: 0,
                repaired_envelope_count: 0,
                filtered_other_symbol_rows: 0,
            }
        }
        DataSourcePreset::IntradayVn30F1m => generate_intraday_vn30f_1m_report(5_400),
        DataSourcePreset::CafeFIndexCsv => load_cafef_preset(symbol_override),
        DataSourcePreset::SpyDailyCsv => load_spy_preset(),
        DataSourcePreset::BtcUsdt15mJson => load_btcusdt_15m_json_preset(),
    }
}

/// Generates a deterministic `1m` intraday futures dataset (`VN30F1M`, `255` 1m bars/session:
/// `02:00..04:29 UTC` morning + `06:00..07:44 UTC` afternoon across Mon..Fri sessions)
/// supporting upward rollup across every timeframe `1m -> 5m -> 15m -> 1h -> 4h -> 1D -> 1W`.
#[must_use]
pub fn generate_intraday_vn30f_1m_report(target_bars: usize) -> DataLoadReport {
    let total = target_bars.max(60);
    let mut candles = Vec::with_capacity(total);
    let mut day_cursor = days_from_civil(2026, 9, 1); // Tuesday 2026-09-01
    let mut prev_close = 1_285.0_f64;

    while candles.len() < total {
        if weekday_from_days(day_cursor) >= 5 {
            day_cursor += 1;
            continue;
        }
        let day_epoch = day_cursor * 86_400;
        // Morning session: 02:00..04:30 UTC (150 minutes) + Afternoon session: 06:00..07:45 UTC (105 minutes)
        for session_minute in (120..270).chain(360..465) {
            if candles.len() >= total {
                break;
            }
            let i = candles.len();
            let t = i as f64;
            let ts = day_epoch + i64::from(session_minute) * 60;

            let trend = if (i / 320) % 2 == 0 { 0.045 } else { -0.038 };
            let wave = (t * 0.09).sin() * 0.62 + (t * 0.27).cos() * 0.34;
            let gap = if session_minute == 120 || session_minute == 360 {
                ((t * 0.41).cos() * 1.35) - ((t * 0.19).sin() * 0.85)
            } else {
                (t * 0.73).cos() * 0.08
            };

            let open = round2((prev_close + gap).max(950.0));
            let close = round2((open + trend + wave * 0.42).max(950.0));
            let body_top = open.max(close);
            let body_bot = open.min(close);
            let hi_wick = 0.20 + (t * 0.61).sin().abs() * 0.85;
            let lo_wick = 0.20 + (t * 0.53).cos().abs() * 0.80;
            let high = round2(body_top + hi_wick);
            let low = round2((body_bot - lo_wick).max(900.0));
            let volume =
                (1_850.0 + (close - open).abs() * 2_400.0 + ((t * 0.17).sin().abs() * 1_600.0))
                    .round();

            candles.push(Candle {
                timestamp: ts,
                date_label: Timeframe::M1.format_candle_label(ts),
                open,
                high,
                low,
                close,
                volume,
            });
            prev_close = close;
        }
        day_cursor += 1;
    }

    DataLoadReport {
        symbol: "VN30F1M".to_owned(),
        available_symbols: vec!["VN30F1M".to_owned()],
        base_timeframe: Timeframe::M1,
        candles,
        raw_row_count: total,
        deduplicated_count: 0,
        repaired_envelope_count: 0,
        filtered_other_symbol_rows: 0,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn read_repo_fixture(rel_path: &str) -> Option<String> {
    if let Ok(text) = std::fs::read_to_string(rel_path) {
        return Some(text);
    }
    let manifest_joined = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(rel_path);
    std::fs::read_to_string(manifest_joined).ok()
}

fn load_cafef_preset(symbol_override: Option<&str>) -> DataLoadReport {
    let opts = DataLoadOptions {
        symbol_filter: Some(symbol_override.unwrap_or("VNINDEX").to_owned()),
        default_symbol: Some("VNINDEX".to_owned()),
        ..DataLoadOptions::default()
    };

    #[cfg(not(target_arch = "wasm32"))]
    if let Some(csv_text) = read_repo_fixture("ARCHIVE/test-results/p0b/data/CafeF.INDEX.clean.csv")
        && let Ok(rep) = load_ohlcv_csv(&csv_text, &opts)
    {
        return rep;
    }

    let fallback_csv = build_embedded_cafef_csv();
    load_ohlcv_csv(&fallback_csv, &opts).expect("valid embedded CafeF CSV")
}

fn load_spy_preset() -> DataLoadReport {
    let opts = DataLoadOptions {
        default_symbol: Some("SPY".to_owned()),
        ..DataLoadOptions::default()
    };

    #[cfg(not(target_arch = "wasm32"))]
    if let Some(csv_text) =
        read_repo_fixture("ARCHIVE/research_repos/lightweight-charts-drawing/demo/SPY.csv")
        && let Ok(rep) = load_ohlcv_csv(&csv_text, &opts)
    {
        return rep;
    }

    let fallback_csv = build_embedded_spy_csv();
    load_ohlcv_csv(&fallback_csv, &opts).expect("valid embedded SPY CSV")
}

fn load_btcusdt_15m_json_preset() -> DataLoadReport {
    let json_doc = build_btcusdt_15m_json_document(1_344);
    load_ohlcv_json(&json_doc, &DataLoadOptions::default()).expect("valid BTCUSDT 15m JSON")
}

/// Builds a deterministic 24/7 (weekend-inclusive) `BTCUSDT` `15m` JSON kline envelope document
/// using 13-digit Unix millisecond timestamps.
#[must_use]
pub fn build_btcusdt_15m_json_document(bar_count: usize) -> String {
    let total = bar_count.max(32);
    // Anchor at Friday 2026-10-02 00:00:00 UTC so bars span two full Sat/Sun crypto weekends!
    let start_sec = days_from_civil(2026, 10, 2) * 86_400;
    let mut prev_close = 64_250.0_f64;
    let mut out = String::with_capacity(total * 68 + 128);
    out.push_str("{\"symbol\":\"BTCUSDT\",\"timeframe\":\"15m\",\"candles\":[");

    for i in 0..total {
        if i > 0 {
            out.push(',');
        }
        let t = i as f64;
        let ts_ms = (start_sec + (i as i64) * 900) * 1_000;
        let drift = if (i / 192) % 2 == 0 { 14.5 } else { -11.2 };
        let osc = (t * 0.13).sin() * 92.0 + (t * 0.31).cos() * 54.0;
        let open = round2((prev_close + (t * 0.71).cos() * 12.0).max(10_000.0));
        let close = round2((open + drift + osc * 0.48).max(10_000.0));
        let hi = round2(open.max(close) + 28.0 + (t * 0.47).sin().abs() * 95.0);
        let lo = round2((open.min(close) - 26.0 - (t * 0.39).cos().abs() * 88.0).max(9_500.0));
        let vol = round2(42.5 + (close - open).abs() * 0.85 + (t * 0.21).sin().abs() * 65.0);
        let _ = write!(
            out,
            "[{ts_ms},{open:.2},{hi:.2},{lo:.2},{close:.2},{vol:.2}]"
        );
        prev_close = close;
    }
    out.push_str("]}");
    out
}

fn build_embedded_cafef_csv() -> String {
    let mut s = String::from("<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\r\n");
    let start_day = days_from_civil(2025, 1, 2);
    let mut vn_close = 1_255.0_f64;
    let mut hnx_close = 228.0_f64;
    for d in 0..260 {
        let day = start_day + d;
        if weekday_from_days(day) >= 5 {
            continue;
        }
        let dt = super::calendar::decompose_utc(day * 86_400);
        let ymd = format!("{:04}{:02}{:02}", dt.year, dt.month, dt.day);
        let t = d as f64;
        let hnx_o = round2(hnx_close);
        let hnx_c = round2(hnx_o + (t * 0.23).sin() * 1.8);
        let hnx_h = round2(hnx_o.max(hnx_c) + 1.4);
        let hnx_l = round2(hnx_o.min(hnx_c) - 1.3);
        let _ = writeln!(
            s,
            "HNX-INDEX,{ymd},{hnx_o:.2},{hnx_h:.2},{hnx_l:.2},{hnx_c:.2},48500000\r"
        );
        hnx_close = hnx_c;

        let vn_o = round2(vn_close);
        let vn_c = round2(vn_o + (t * 0.19).cos() * 6.4 + 0.45);
        let vn_h = round2(vn_o.max(vn_c) + 5.2);
        let vn_l = round2(vn_o.min(vn_c) - 4.8);
        let _ = writeln!(
            s,
            "VNINDEX,{ymd},{vn_o:.2},{vn_h:.2},{vn_l:.2},{vn_c:.2},615000000\r"
        );
        vn_close = vn_c;
    }
    s
}

fn build_embedded_spy_csv() -> String {
    let mut s = String::from("time,open,high,low,close\n");
    let start_day = days_from_civil(2025, 1, 2);
    let mut close = 586.0_f64;
    for d in 0..260 {
        let day = start_day + d;
        if weekday_from_days(day) >= 5 {
            continue;
        }
        let ts = day * 86_400 + 14 * 3_600 + 30 * 60;
        let t = d as f64;
        let o = round2(close);
        let c = round2(o + (t * 0.21).sin() * 3.2 + 0.25);
        let h = round2(o.max(c) + 2.4);
        let l = round2(o.min(c) - 2.1);
        let _ = writeln!(s, "{ts},{o:.2},{h:.2},{l:.2},{c:.2}");
        close = c;
    }
    s
}

#[inline]
fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}
