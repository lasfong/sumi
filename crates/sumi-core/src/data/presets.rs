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

/// Canonical tickers included in [`DataSourcePreset::Vn30Basket`] (`Milestone R08`).
pub const VN30_BASKET_SYMBOLS: [&str; 8] = ["FPT", "VCB", "HPG", "SSI", "MWG", "TCB", "ACB", "VIC"];

/// Built-in workstation data feeds selectable from the top toolbar (`Milestones R06–R08`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum DataSourcePreset {
    #[default]
    SyntheticVnindex,
    IntradayVn30F1m,
    CafeFIndexCsv,
    Vn30Basket,
    SpyDailyCsv,
    BtcUsdt15mJson,
    CustomFile,
}

impl DataSourcePreset {
    pub const TOOLBAR_PRESETS: [Self; 6] = [
        Self::SyntheticVnindex,
        Self::IntradayVn30F1m,
        Self::CafeFIndexCsv,
        Self::Vn30Basket,
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
            Self::Vn30Basket => "VN30 Basket",
            Self::SpyDailyCsv => "SPY CSV",
            Self::BtcUsdt15mJson => "BTC JSON",
            Self::CustomFile => "File",
        }
    }
}

/// One preloaded instrument feed in the workstation's multi-symbol Watchlist & Scanner universe.
#[derive(Debug, Clone)]
pub struct WatchlistFeedEntry {
    pub instrument_id: String,
    pub preset: DataSourcePreset,
    pub symbol: String,
    pub base_timeframe: Timeframe,
    pub candles: Vec<Candle>,
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
                quarantined_row_count: 0,
                filtered_other_symbol_rows: 0,
            }
        }
        DataSourcePreset::IntradayVn30F1m => generate_intraday_vn30f_1m_report(5_400),
        DataSourcePreset::CafeFIndexCsv => load_cafef_preset(symbol_override),
        DataSourcePreset::Vn30Basket => generate_vn30_basket_report(symbol_override),
        DataSourcePreset::SpyDailyCsv => load_spy_preset(),
        DataSourcePreset::BtcUsdt15mJson => load_btcusdt_15m_json_preset(),
    }
}

/// Loads all `13` bundled multi-asset instruments (`VNINDEX`, `HNX-INDEX`, `VN30F1M`, `SPY`,
/// `BTCUSDT`, plus all `8` `VN30` equities) for the workstation Watchlist & Technical Scanner.
#[must_use]
pub fn load_bundled_watchlist_universe() -> Vec<WatchlistFeedEntry> {
    let mut entries = Vec::with_capacity(13);
    let push_report =
        |entries: &mut Vec<WatchlistFeedEntry>, preset: DataSourcePreset, rep: DataLoadReport| {
            let sym_upper = rep.symbol.trim().to_ascii_uppercase();
            let instrument_id = format!("Preset:{}:{sym_upper}", preset.short_label());
            entries.push(WatchlistFeedEntry {
                instrument_id,
                preset,
                symbol: rep.symbol,
                base_timeframe: rep.base_timeframe,
                candles: rep.candles,
            });
        };

    for sym in ["VNINDEX", "HNX-INDEX"] {
        push_report(
            &mut entries,
            DataSourcePreset::CafeFIndexCsv,
            load_cafef_preset(Some(sym)),
        );
    }
    push_report(
        &mut entries,
        DataSourcePreset::IntradayVn30F1m,
        generate_intraday_vn30f_1m_report(5_400),
    );
    push_report(
        &mut entries,
        DataSourcePreset::SpyDailyCsv,
        load_spy_preset(),
    );
    push_report(
        &mut entries,
        DataSourcePreset::BtcUsdt15mJson,
        load_btcusdt_15m_json_preset(),
    );
    for sym in VN30_BASKET_SYMBOLS {
        push_report(
            &mut entries,
            DataSourcePreset::Vn30Basket,
            generate_vn30_basket_report(Some(sym)),
        );
    }
    entries
}

/// Generates a deterministic `180`-session daily OHLCV series for one of the 8 [`VN30_BASKET_SYMBOLS`],
/// spanning uptrend, pullback, oversold recovery, Bollinger squeeze breakout, golden cross, and downtrend regimes.
#[must_use]
pub fn generate_vn30_basket_report(symbol_override: Option<&str>) -> DataLoadReport {
    let requested = symbol_override
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("FPT");
    let matched_idx = VN30_BASKET_SYMBOLS
        .iter()
        .position(|s| s.eq_ignore_ascii_case(requested))
        .unwrap_or(0);
    let symbol = VN30_BASKET_SYMBOLS[matched_idx].to_owned();
    let total_bars = 180_usize;
    let mut candles = Vec::with_capacity(total_bars);
    let mut day_cursor = days_from_civil(2025, 11, 3); // Monday 2025-11-03

    let base_price = match matched_idx {
        0 => 126.0, // FPT: strong tech uptrend + final-bar 2.8x volume breakout
        1 => 89.5,  // VCB: tight squeeze followed by upper Bollinger breakout
        2 => 26.5,  // HPG: uptrend followed by orderly 6% pullback above EMA(50)
        3 => 32.0,  // SSI: momentum wave with fresh bullish MACD histogram cross
        4 => 61.0,  // MWG: sharp selloff into RSI < 30 followed by RSI > 30 recovery bounce
        5 => 23.4,  // TCB: base-to-uptrend SMA(20)/EMA(50) golden cross
        6 => 25.6,  // ACB: ultra-tight sideways Bollinger consolidation (< 4% bandwidth)
        _ => 48.5,  // VIC: sustained distribution downtrend
    };
    let mut prev_close = base_price;

    while candles.len() < total_bars {
        if !crate::journal::is_vn_exchange_trading_day_days(day_cursor) {
            day_cursor += 1;
            continue;
        }
        let i = candles.len();
        let t = i as f64;
        let ts = day_cursor * 86_400;

        let (delta_pct, range_pct, vol_mult) = match matched_idx {
            0 => {
                // FPT: steady positive drift + strong volume expansion on last bar
                let drift = 0.0034 + (t * 0.17).sin() * 0.0042;
                if i == total_bars - 1 {
                    (0.024, 0.014, 2.85)
                } else {
                    (drift, 0.009, 1.0 + (t * 0.31).sin().abs() * 0.25)
                }
            }
            1 => {
                // VCB: tight low-volatility band in bars 140..178, then decisive upper-BB breakout on final bar
                if (140..total_bars - 1).contains(&i) {
                    ((t * 0.55).sin() * 0.0022, 0.0035, 0.92)
                } else if i == total_bars - 1 {
                    (0.026, 0.011, 2.35)
                } else {
                    (0.0012 + (t * 0.14).cos() * 0.0045, 0.007, 1.05)
                }
            }
            2 => {
                // HPG: strong uptrend through bar 164, then controlled ~5.5% pullback holding EMA50
                if i < 165 {
                    (0.0038 + (t * 0.19).sin() * 0.0035, 0.010, 1.1)
                } else {
                    (-0.0038 + (t * 0.4).cos() * 0.001, 0.008, 0.85)
                }
            }
            3 => {
                // SSI: dip bars 155..175 then sharp 4-bar turn triggering fresh MACD Bull Cross
                if (155..175).contains(&i) {
                    (-0.0055, 0.009, 0.95)
                } else if i >= 175 {
                    (0.0145, 0.012, 1.65)
                } else {
                    (0.0018 + (t * 0.21).sin() * 0.006, 0.010, 1.05)
                }
            }
            4 => {
                // MWG: steep 14-bar slide pushing RSI(14) < 26, then strong rebound bar crossing RSI > 30
                if (163..total_bars - 1).contains(&i) {
                    (-0.0135, 0.009, 1.15)
                } else if i == total_bars - 1 {
                    (0.038, 0.015, 2.40)
                } else {
                    (0.0008 + (t * 0.16).sin() * 0.005, 0.009, 1.0)
                }
            }
            5 => {
                // TCB: gentle dip bars 110..158 then steady acceleration so SMA(20) crosses above EMA(50)
                if (110..158).contains(&i) {
                    (-0.0032 + (t * 0.25).sin() * 0.002, 0.007, 0.9)
                } else if i >= 158 {
                    (0.0062, 0.008, 1.35)
                } else {
                    (0.0010 + (t * 0.18).cos() * 0.003, 0.007, 1.0)
                }
            }
            6 => {
                // ACB: super tight sideways range (< 4% BB width)
                (
                    (t * 0.37).sin() * 0.0018,
                    0.0030,
                    0.95 + (t * 0.29).cos().abs() * 0.15,
                )
            }
            _ => {
                // VIC: steady distribution downtrend
                (-0.0036 + (t * 0.22).cos() * 0.0032, 0.009, 1.10)
            }
        };

        let gap_pct = ((t * 0.63 + matched_idx as f64).cos()) * 0.0012;
        let open = round2((prev_close * (1.0 + gap_pct)).max(5.0));
        let close = round2((open * (1.0 + delta_pct)).max(5.0));
        let hi_pad = (open.max(close) * range_pct * 0.55).max(0.05);
        let lo_pad = (open.min(close) * range_pct * 0.50).max(0.05);
        let high = round2(open.max(close) + hi_pad);
        let low = round2((open.min(close) - lo_pad).max(1.0));
        let base_vol = 2_400_000.0 + (matched_idx as f64) * 450_000.0;
        let volume = (base_vol * vol_mult).round();

        candles.push(Candle {
            timestamp: ts,
            date_label: Timeframe::D1.format_candle_label(ts),
            open,
            high,
            low,
            close,
            volume,
        });
        prev_close = close;
        day_cursor += 1;
    }

    DataLoadReport {
        symbol,
        available_symbols: VN30_BASKET_SYMBOLS.iter().map(|&s| s.to_owned()).collect(),
        base_timeframe: Timeframe::D1,
        raw_row_count: total_bars * VN30_BASKET_SYMBOLS.len(),
        deduplicated_count: 0,
        repaired_envelope_count: 0,
        quarantined_row_count: 0,
        filtered_other_symbol_rows: total_bars * (VN30_BASKET_SYMBOLS.len() - 1),
        candles,
    }
}

/// Generates a deterministic `1m` intraday futures dataset (`VN30F1M`, `255` 1m bars/session:
/// `02:00..04:29 UTC` morning + `06:00..07:44 UTC` afternoon across Mon..Fri sessions)
/// supporting upward rollup across every timeframe `1m -> 5m -> 15m -> 1h -> 4h -> 1D -> 1W`.
#[must_use]
pub fn generate_intraday_vn30f_1m_report(target_bars: usize) -> DataLoadReport {
    let total = target_bars.max(60);
    let mut candles = Vec::with_capacity(total);
    // Start at `2026-09-01` and skip official `VIETNAM_HOLIDAYS_V1` holidays (`2026-09-01`, `2026-09-02`
    // National Day) & weekends so intraday VN30F1M opens cleanly on Thursday `2026-09-03` (`BUG-025`).
    let mut day_cursor = days_from_civil(2026, 9, 1);
    let mut prev_close = 1_285.0_f64;

    while candles.len() < total {
        if !crate::journal::is_vn_exchange_trading_day_days(day_cursor) {
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
        quarantined_row_count: 0,
        filtered_other_symbol_rows: 0,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn workstation_data_roots() -> Vec<std::path::PathBuf> {
    let mut roots = Vec::new();
    if let Ok(env_dir) = std::env::var("SUMI_DATA_DIR") {
        let trimmed = env_dir.trim();
        if !trimmed.is_empty() {
            roots.push(std::path::PathBuf::from(trimmed));
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        roots.push(cwd);
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(macos_dir) = exe.parent()
    {
        if let Some(contents_dir) = macos_dir.parent() {
            roots.push(contents_dir.join("Resources"));
        }
        let mut cursor = Some(macos_dir);
        while let Some(dir) = cursor {
            if dir.join("data").is_dir() || dir.join("Cargo.toml").is_file() {
                roots.push(dir.to_path_buf());
                break;
            }
            cursor = dir.parent();
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        roots.push(
            std::path::PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("Sumi Workstation"),
        );
    }
    if let Ok(appdata) = std::env::var("APPDATA") {
        let trimmed = appdata.trim();
        if !trimmed.is_empty() {
            roots.push(std::path::PathBuf::from(trimmed).join("Sumi Workstation"));
        }
    }
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    if let Some(workspace_root) = manifest_dir.parent().and_then(|p| p.parent()) {
        roots.push(workspace_root.to_path_buf());
    }
    roots.push(manifest_dir);
    roots
}

#[cfg(not(target_arch = "wasm32"))]
fn read_repo_fixture(rel_path: &str) -> Option<String> {
    for root in workstation_data_roots() {
        let candidate = root.join(rel_path);
        if let Ok(text) = std::fs::read_to_string(&candidate) {
            return Some(text);
        }
    }
    None
}

#[cfg(not(target_arch = "wasm32"))]
fn read_latest_real_cafef_index_csv() -> Option<String> {
    // 1. Prefer full official `data/raw/cafef/CafeF.INDEX*.csv` (e.g. `CafeF.INDEX.Upto06112026.csv`,
    //    `2000–2026`, clean without the `-34.5%` spike in archived `CafeF.INDEX.clean.csv`, `BUG-006`/`BUG-022`).
    for rel in [
        "data/raw/cafef/CafeF.INDEX.Upto06112026.csv",
        "data/raw/cafef/CafeF.INDEX.csv",
    ] {
        if let Some(txt) = read_repo_fixture(rel) {
            return Some(txt);
        }
    }
    for root in workstation_data_roots() {
        let base = root.join("data/raw/cafef");
        if let Ok(entries) = std::fs::read_dir(&base) {
            let mut candidates: Vec<std::path::PathBuf> = entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| {
                    p.is_file()
                        && p.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
                            let up = n.to_ascii_uppercase();
                            up.contains("CAFEF.INDEX") && up.ends_with(".CSV")
                        })
                })
                .collect();
            candidates.sort();
            if let Some(latest) = candidates.pop()
                && let Ok(txt) = std::fs::read_to_string(latest)
            {
                return Some(txt);
            }
        }
    }
    None
}

fn load_cafef_preset(symbol_override: Option<&str>) -> DataLoadReport {
    let opts = DataLoadOptions {
        symbol_filter: Some(symbol_override.unwrap_or("VNINDEX").to_owned()),
        default_symbol: Some("VNINDEX".to_owned()),
        invalid_row_policy: super::InvalidRowPolicy::QuarantineRow,
        ..DataLoadOptions::default()
    };

    #[cfg(not(target_arch = "wasm32"))]
    if let Some(csv_text) = read_latest_real_cafef_index_csv()
        && let Ok(rep) = load_ohlcv_csv(&csv_text, &opts)
    {
        return rep;
    }

    #[cfg(not(target_arch = "wasm32"))]
    if let Some(csv_text) = read_repo_fixture("assets/fixtures/CafeF.INDEX.clean.csv")
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
    if let Some(csv_text) = read_repo_fixture("assets/fixtures/SPY.csv")
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
    for d in 0..280 {
        let day = start_day + d;
        if !crate::journal::is_vn_exchange_trading_day_days(day) {
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
    for d in 0..280 {
        let day = start_day + d;
        let (y, m, dm) = super::calendar::civil_from_days(day);
        if weekday_from_days(day) >= 5 || super::audit::is_us_nyse_holiday_ymd(y, m, dm) {
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

/// Resolves an overlapping benchmark series for Relative Strength (`RS`) and Main-Pane Rebased Comparison.
///
/// Prefers an exact match from `universe` whenever it contains at least `2` candles overlapping
/// `[ref_first_ts - 35 days, ref_last_ts + 1 day]`. For macro benchmarks (`VNINDEX`, `HNX-INDEX`, `SPY`)
/// compared against instruments whose preset dates fall outside the historical archive sample (such as
/// `VN30` basket `2025-11..2026-07` or `VN30F1M` `2026-09`), generates a deterministic daily benchmark
/// series covering the exact target window so causal bar-close alignment operates cleanly.
#[must_use]
pub fn resolve_benchmark_series_for_window(
    benchmark_symbol: &str,
    ref_first_ts: i64,
    ref_last_ts: i64,
    universe: &[WatchlistFeedEntry],
) -> Option<(String, Timeframe, Vec<Candle>)> {
    let target = benchmark_symbol.trim();
    if target.is_empty() || ref_last_ts < ref_first_ts {
        return None;
    }

    let win_lo = ref_first_ts.saturating_sub(35 * 86_400);
    let win_hi = ref_last_ts.saturating_add(86_400);
    let recent_floor_ts = ref_last_ts.saturating_sub(14 * 86_400);

    if let Some(entry) = universe
        .iter()
        .filter(|e| e.symbol.eq_ignore_ascii_case(target))
        .filter_map(|e| {
            let overlap_count = e
                .candles
                .iter()
                .filter(|c| c.timestamp >= win_lo && c.timestamp <= win_hi)
                .count();
            let reaches_recent_tail = e
                .candles
                .last()
                .is_some_and(|c| c.timestamp >= recent_floor_ts);
            // Prefer real overlapping historical index data (`overlap_count >= 20` or recent tail match)
            // over silently falling back to a synthetic sine wave (`BUG-006`).
            (overlap_count >= 2 && (reaches_recent_tail || overlap_count >= 20)).then_some((
                reaches_recent_tail,
                overlap_count,
                e,
            ))
        })
        .max_by_key(|(reaches_tail, overlap_count, _)| (*reaches_tail, *overlap_count))
        .map(|(_, _, e)| e)
    {
        return Some((
            entry.symbol.clone(),
            entry.base_timeframe,
            entry.candles.clone(),
        ));
    }

    let upper = target.to_ascii_uppercase();
    let (canonical, base_price, wave_amp, drift_per_bar) = match upper.as_str() {
        "VNINDEX" | "VN-INDEX" => ("VNINDEX [SYNTH]", 1_265.0, 5.8, 0.42),
        "HNX-INDEX" | "HNXINDEX" => ("HNX-INDEX [SYNTH]", 232.0, 1.6, 0.08),
        "SPY" => ("SPY [SYNTH]", 588.0, 2.9, 0.22),
        _ => return None,
    };

    let start_day = (ref_first_ts.saturating_sub(45 * 86_400)).div_euclid(86_400);
    let end_day = (ref_last_ts.saturating_add(7 * 86_400)).div_euclid(86_400);
    let mut candles = Vec::new();
    let mut prev_close = base_price;

    for day in start_day..=end_day {
        if weekday_from_days(day) >= 5 {
            continue;
        }
        let idx = candles.len() as f64;
        let ts = day * 86_400;
        let open = round2((prev_close + (idx * 0.29).cos() * (wave_amp * 0.18)).max(10.0));
        let delta = drift_per_bar
            + (idx * 0.17).sin() * (wave_amp * 0.72)
            + (idx * 0.43).cos() * (wave_amp * 0.35);
        let close = round2((open + delta).max(10.0));
        let hi_wick = (wave_amp * 0.55) + (idx * 0.61).sin().abs() * (wave_amp * 0.45);
        let lo_wick = (wave_amp * 0.50) + (idx * 0.53).cos().abs() * (wave_amp * 0.42);
        let high = round2(open.max(close) + hi_wick);
        let low = round2((open.min(close) - lo_wick).max(5.0));
        let volume = ((580_000_000.0_f64) * (0.85 + (idx * 0.23).sin().abs() * 0.35)).round();

        candles.push(Candle {
            timestamp: ts,
            date_label: Timeframe::D1.format_candle_label(ts),
            open,
            high,
            low,
            close,
            volume,
        });
        prev_close = close;
    }

    (candles.len() >= 2).then_some((canonical.to_owned(), Timeframe::D1, candles))
}
