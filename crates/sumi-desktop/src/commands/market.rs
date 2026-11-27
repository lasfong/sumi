use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use sumi_data::{
    Adjustment, BarQueryMode, BarsRequest, CafeFImportConfig, CafeFImportSummary, CafeFImporter,
    CorporateEventMarker, DataSourceOrigin, Interval, LiveQuote, OhlcvBar, ParsedExpression,
    ResolvedSymbol, SeriesKey, StoreStats, SymbolKey, SymbolSearchResult, VietnamPriceBand,
    VnFreshnessSummary, compute_rebased_comparison_series, evaluate_expression_ohlcv,
    is_synthetic_expression, latest_expected_vn_trading_session_ms, parse_expression,
    yumeko_data_dir,
};
use sumi_yumeko_bridge::{ValuationQuarterDto, YumekoReadOnlyBridge};
use tauri::State;
use tauri::ipc::Channel;
use tracing::{debug, warn};

use crate::error::{DesktopError, DesktopResult};
use crate::state::{ActiveBarStream, ActiveQuoteStream, AppState};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VnBandLevels {
    pub venue: String,
    pub label: String,
    pub reference: f64,
    pub ceiling: f64,
    pub floor: f64,
    pub tick_size: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedSymbolResponse {
    pub symbol_info: ResolvedSymbol,
    pub last_price: Option<f64>,
    pub prev_close: Option<f64>,
    pub change: Option<f64>,
    pub change_percent: Option<f64>,
    pub volume: Option<f64>,
    pub vn_band: Option<VnBandLevels>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BarsPageResponse {
    pub bars: Vec<OhlcvBar>,
    pub has_older: bool,
    pub has_newer: bool,
    pub source: DataSourceOrigin,
    pub symbol_info: Option<ResolvedSymbol>,
    pub vn_band: Option<VnBandLevels>,
    pub replay_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveBarPayload {
    pub ticker: String,
    pub interval: String,
    pub bar: OhlcvBar,
    pub is_update: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchlistQuoteSnapshot {
    pub ticker: String,
    pub exchange: String,
    pub symbol: String,
    pub description: String,
    pub last_price: Option<f64>,
    pub prev_close: Option<f64>,
    pub open: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub bid: Option<f64>,
    pub ask: Option<f64>,
    pub change: Option<f64>,
    pub change_percent: Option<f64>,
    pub volume: Option<f64>,
    pub price_precision: u32,
    pub vn_band: Option<VnBandLevels>,
    pub timestamp_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataStatusResponse {
    pub stats: StoreStats,
    pub offline_mode: bool,
    pub freshness: VnFreshnessSummary,
    pub data_dir: String,
    pub config_dir: String,
    pub yumeko_db_detected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncProgressPayload {
    pub scope: String,
    pub completed: usize,
    pub total: usize,
    pub current_ticker: String,
    pub updated_symbols: usize,
    pub updated_bars: usize,
    pub failed_symbols: usize,
    pub done: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VnUniverseSyncSummary {
    pub discovered_symbols: usize,
    pub upserted_symbols: usize,
    pub elapsed_ms: u64,
}

pub(crate) fn curated_catalog() -> Vec<SymbolSearchResult> {
    let entries: &[(&str, &str, &str, &str, &str, &str)] = &[
        ("HOSE", "FPT", "Công ty Cổ phần FPT", "stock", "VND", "VN"),
        (
            "HOSE",
            "VCB",
            "Ngân hàng TMCP Ngoại thương Việt Nam (Vietcombank)",
            "stock",
            "VND",
            "VN",
        ),
        ("HOSE", "HPG", "Tập đoàn Hòa Phát", "stock", "VND", "VN"),
        (
            "HOSE",
            "MWG",
            "Đầu tư Thế Giới Di Động",
            "stock",
            "VND",
            "VN",
        ),
        (
            "HOSE",
            "TCB",
            "Ngân hàng TMCP Kỹ thương Việt Nam (Techcombank)",
            "stock",
            "VND",
            "VN",
        ),
        (
            "HOSE",
            "MBB",
            "Ngân hàng TMCP Quân đội (MBBank)",
            "stock",
            "VND",
            "VN",
        ),
        ("HOSE", "SSI", "Chứng khoán SSI", "stock", "VND", "VN"),
        ("HOSE", "VHM", "Vinhomes JSC", "stock", "VND", "VN"),
        (
            "HOSE",
            "VNM",
            "Sữa Việt Nam (Vinamilk)",
            "stock",
            "VND",
            "VN",
        ),
        ("HOSE", "VIC", "Tập đoàn Vingroup", "stock", "VND", "VN"),
        ("HOSE", "ACB", "Ngân hàng TMCP Á Châu", "stock", "VND", "VN"),
        ("HOSE", "MSN", "Tập đoàn Masan", "stock", "VND", "VN"),
        (
            "HOSE",
            "VNINDEX",
            "Chỉ số VN-Index (HOSE)",
            "index",
            "VND",
            "VN",
        ),
        ("HOSE", "VN30", "Chỉ số VN30 (HOSE)", "index", "VND", "VN"),
        (
            "HNX",
            "VN30F1!",
            "Hợp đồng tương lai VN30 kỳ hạn gần nhất",
            "futures",
            "VND",
            "VN",
        ),
        ("HNX", "HNXINDEX", "Chỉ số HNX-Index", "index", "VND", "VN"),
        (
            "HNX",
            "SHS",
            "Chứng khoán Sài Gòn - Hà Nội",
            "stock",
            "VND",
            "VN",
        ),
        (
            "HNX",
            "PVS",
            "Dịch vụ Kỹ thuật Dầu khí Việt Nam",
            "stock",
            "VND",
            "VN",
        ),
        ("HNX", "CEO", "Tập đoàn C.E.O", "stock", "VND", "VN"),
        (
            "UPCOM",
            "ACV",
            "Tổng Công ty Cảng hàng không Việt Nam",
            "stock",
            "VND",
            "VN",
        ),
        ("UPCOM", "BSR", "Lọc hóa dầu Bình Sơn", "stock", "VND", "VN"),
        ("UPCOM", "MCH", "Hàng tiêu dùng Masan", "stock", "VND", "VN"),
        (
            "BINANCE",
            "BTCUSDT",
            "Bitcoin / TetherUS",
            "crypto",
            "USDT",
            "",
        ),
        (
            "BINANCE",
            "ETHUSDT",
            "Ethereum / TetherUS",
            "crypto",
            "USDT",
            "",
        ),
        (
            "BINANCE",
            "SOLUSDT",
            "Solana / TetherUS",
            "crypto",
            "USDT",
            "",
        ),
        ("BINANCE", "BNBUSDT", "BNB / TetherUS", "crypto", "USDT", ""),
        ("BINANCE", "XRPUSDT", "XRP / TetherUS", "crypto", "USDT", ""),
        ("NASDAQ", "AAPL", "Apple Inc.", "stock", "USD", "US"),
        ("NASDAQ", "NVDA", "NVIDIA Corporation", "stock", "USD", "US"),
        (
            "NASDAQ",
            "MSFT",
            "Microsoft Corporation",
            "stock",
            "USD",
            "US",
        ),
        ("AMEX", "SPY", "SPDR S&P 500 ETF Trust", "fund", "USD", "US"),
        (
            "FX_IDC",
            "USDVND",
            "US Dollar / Vietnamese Dong",
            "forex",
            "VND",
            "",
        ),
        ("OANDA", "XAUUSD", "Gold / US Dollar Spot", "cfd", "USD", ""),
    ];
    entries
        .iter()
        .map(|(ex, sym, desc, mt, curr, country)| SymbolSearchResult {
            ticker: format!("{ex}:{sym}"),
            exchange: (*ex).to_owned(),
            prefix: (*ex).to_owned(),
            symbol: (*sym).to_owned(),
            description: (*desc).to_owned(),
            market_type: (*mt).to_owned(),
            currency_code: (*curr).to_owned(),
            country_code: (*country).to_owned(),
            provider_id: "sumi_catalog".to_owned(),
            type_specs: Vec::new(),
        })
        .collect()
}

pub(crate) fn compute_vn_band_from_bars(
    symbol: &ResolvedSymbol,
    bars: &[OhlcvBar],
) -> Option<VnBandLevels> {
    let band = symbol.vn_price_band.clone().or_else(|| {
        VietnamPriceBand::infer(&symbol.exchange, &symbol.symbol, &symbol.market_type)
    })?;
    let reference = if bars.len() >= 2 {
        bars[bars.len() - 2].close
    } else {
        bars.last()?.close
    };
    if !reference.is_finite() || reference <= 0.0 {
        return None;
    }
    let (floor, ceiling) = band.ceiling_floor_for_reference(reference)?;
    let tick_size = symbol.tick_size_at(reference);
    Some(VnBandLevels {
        venue: band.venue_label,
        label: band.label,
        reference,
        ceiling,
        floor,
        tick_size,
    })
}

pub(crate) fn synthesize_fallback_symbol(key: &SymbolKey) -> ResolvedSymbol {
    let ex = key.exchange.as_str();
    if matches!(ex, "HOSE" | "HNX" | "UPCOM") {
        let is_index = key.symbol.contains("INDEX") || key.symbol == "VN30";
        return ResolvedSymbol::from_cafef_vn_ticker(&key.exchange, &key.symbol, is_index);
    }
    let is_crypto = matches!(ex, "BINANCE" | "OKX" | "BYBIT" | "COINBASE");
    let is_fx = matches!(ex, "FX_IDC" | "OANDA");
    let price_precision = if key.symbol == "USDVND" { 0 } else { 2 };
    ResolvedSymbol {
        ticker: key.ticker(),
        exchange: key.exchange.clone(),
        listed_exchange: key.exchange.clone(),
        symbol: key.symbol.clone(),
        description: format!("{} ({})", key.symbol, key.exchange),
        market_type: if is_crypto {
            "crypto".to_owned()
        } else if is_fx {
            "forex".to_owned()
        } else {
            "stock".to_owned()
        },
        currency: if is_crypto {
            "USDT".to_owned()
        } else {
            "USD".to_owned()
        },
        timezone: if is_crypto {
            "Etc/UTC".to_owned()
        } else {
            "America/New_York".to_owned()
        },
        session: if is_crypto {
            "24x7".to_owned()
        } else {
            "0930-1600".to_owned()
        },
        price_precision,
        volume_precision: if is_crypto { 4 } else { 0 },
        pricescale: 100,
        minmov: 1,
        minmove2: 0,
        tick_size: Some(0.01),
        variable_tick_size: String::new(),
        variable_tick_table: None,
        point_value: 1.0,
        delay_secs: 0,
        isin: String::new(),
        vn_price_band: None,
        exchange_venue: None,
        sector: None,
        industry: None,
        market_cap_basic: None,
        price_earnings_ttm: None,
        last_annual_eps: None,
        total_shares_outstanding: None,
        total_revenue: None,
        earnings_release_date: None,
        founded: None,
        business_description: None,
    }
}

pub(crate) fn synthesize_expression_symbol(parsed: &ParsedExpression) -> ResolvedSymbol {
    let is_ratio = parsed.canonical.contains('/');
    let precision = if is_ratio { 4 } else { 2 };
    let pricescale = if is_ratio { 10_000 } else { 100 };
    ResolvedSymbol {
        ticker: parsed.canonical.clone(),
        exchange: "EXPR".to_owned(),
        listed_exchange: "EXPR".to_owned(),
        symbol: parsed.canonical.clone(),
        description: format!("Biểu đồ Công thức Tổng hợp: {}", parsed.canonical),
        market_type: "spread".to_owned(),
        currency: if is_ratio { "RATIO".to_owned() } else { "PTS".to_owned() },
        timezone: "Asia/Ho_Chi_Minh".to_owned(),
        session: "0900-1500".to_owned(),
        price_precision: precision,
        volume_precision: 0,
        pricescale,
        minmov: 1,
        minmove2: 0,
        tick_size: Some(1.0 / (pricescale as f64)),
        variable_tick_size: String::new(),
        variable_tick_table: None,
        point_value: 1.0,
        delay_secs: 0,
        isin: String::new(),
        vn_price_band: None,
        exchange_venue: None,
        sector: None,
        industry: None,
        market_cap_basic: None,
        price_earnings_ttm: None,
        last_annual_eps: None,
        total_shares_outstanding: None,
        total_revenue: None,
        earnings_release_date: None,
        founded: None,
        business_description: None,
    }
}

async fn load_expression_bars(
    state: &AppState,
    parsed: &ParsedExpression,
    iv: Interval,
    adj: Adjustment,
    limit: usize,
    cutoff_ms: Option<i64>,
) -> DesktopResult<Vec<OhlcvBar>> {
    let svc = state.active_market_service();
    let fetch_limit = limit.clamp(200, 3_000);
    let mut series_map: HashMap<SymbolKey, Vec<OhlcvBar>> = HashMap::new();
    for sym in &parsed.symbols {
        let sk = SeriesKey::new(sym, iv, adj);
        let req = BarsRequest::init(sk, fetch_limit);
        let mut bars = match svc.get_bars_instant(&req).await {
            Ok(p) if !p.bars.is_empty() => p.bars,
            _ if !state.is_offline() => svc
                .get_bars(&req)
                .await
                .map(|p| p.bars)
                .unwrap_or_default(),
            _ => Vec::new(),
        };
        if let Some(cut) = cutoff_ms {
            bars.retain(|b| b.timestamp <= cut);
        }
        series_map.insert(sym.clone(), bars);
    }
    Ok(evaluate_expression_ohlcv(parsed, &series_map))
}

fn matches_category(item: &SymbolSearchResult, category: Option<&str>) -> bool {
    let Some(cat) = category.map(|c| c.trim().to_ascii_lowercase()) else {
        return true;
    };
    if cat.is_empty() || cat == "all" {
        return true;
    }
    let ex = item.exchange.to_ascii_uppercase();
    let mt = item.market_type.to_ascii_lowercase();
    match cat.as_str() {
        "hose" => ex == "HOSE",
        "hnx" => ex == "HNX" && mt != "futures" && !item.symbol.starts_with("VN30F"),
        "upcom" => ex == "UPCOM",
        "derivatives" | "futures" => {
            mt == "futures" || item.symbol.starts_with("VN30F") || ex == "CME"
        }
        "crypto" => {
            mt == "crypto"
                || matches!(
                    ex.as_str(),
                    "BINANCE" | "OKX" | "BYBIT" | "COINBASE" | "KRAKEN"
                )
        }
        "global" | "international" => {
            !matches!(ex.as_str(), "HOSE" | "HNX" | "UPCOM")
                && mt != "crypto"
                && !matches!(
                    ex.as_str(),
                    "BINANCE" | "OKX" | "BYBIT" | "COINBASE" | "KRAKEN"
                )
        }
        _ => true,
    }
}

#[tauri::command]
pub async fn search_symbols(
    state: State<'_, AppState>,
    query: String,
    category: Option<String>,
    limit: Option<usize>,
) -> DesktopResult<Vec<SymbolSearchResult>> {
    let max_items = limit.unwrap_or(40).clamp(1, 100);
    let q_trimmed = query.trim().to_owned();
    let q_upper = q_trimmed.to_ascii_uppercase();

    let exchange_filter = match category.as_deref().map(str::to_ascii_lowercase).as_deref() {
        Some("hose") => Some("HOSE"),
        Some("hnx") => Some("HNX"),
        Some("upcom") => Some("UPCOM"),
        Some("crypto") => Some("BINANCE"),
        _ => None,
    };

    let mut combined: Vec<SymbolSearchResult> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    // 1. Curated hits matching query + category (ensures Vietnamese descriptions & global/crypto quick picks).
    for item in curated_catalog() {
        if !matches_category(&item, category.as_deref()) {
            continue;
        }
        let hit = q_upper.is_empty()
            || item.symbol.contains(&q_upper)
            || item.ticker.contains(&q_upper)
            || item.description.to_ascii_uppercase().contains(&q_upper);
        if hit && seen.insert(item.ticker.clone()) {
            combined.push(item);
        }
    }

    // 2. Local SQLite symbols (473+ CafeF Vietnamese equities/indices + cached remote symbols).
    let store = state.market_store.clone();
    let q_for_db = q_trimmed.clone();
    let ex_for_db = exchange_filter.map(str::to_owned);
    let local_hits = tokio::task::spawn_blocking(move || {
        store.search_local_symbols(&q_for_db, ex_for_db.as_deref(), max_items * 2)
    })
    .await
    .map_err(|e| DesktopError::Internal(e.to_string()))??;

    for item in local_hits {
        if matches_category(&item, category.as_deref()) && seen.insert(item.ticker.clone()) {
            combined.push(item);
        }
    }

    // 3. Online search when query is non-empty and online mode is active.
    if !q_trimmed.is_empty() && !state.is_offline() && combined.len() < max_items {
        let svc = state.active_market_service();
        if let Ok(remote) = svc
            .search_symbols(&q_trimmed, exchange_filter, max_items)
            .await
        {
            for item in remote {
                if matches_category(&item, category.as_deref()) && seen.insert(item.ticker.clone())
                {
                    combined.push(item);
                }
            }
        }
    }

    // Rank exact symbol match first, then prefix match, then ticker.
    if !q_upper.is_empty() {
        combined.sort_by(|a, b| {
            let rank = |item: &SymbolSearchResult| -> u8 {
                if item.symbol == q_upper {
                    0
                } else if item.symbol.starts_with(&q_upper) {
                    1
                } else if item.ticker.starts_with(&q_upper) {
                    2
                } else {
                    3
                }
            };
            rank(a).cmp(&rank(b)).then_with(|| a.symbol.cmp(&b.symbol))
        });
    }

    combined.truncate(max_items);
    Ok(combined)
}

#[tauri::command]
pub async fn resolve_symbol(
    state: State<'_, AppState>,
    ticker: String,
) -> DesktopResult<ResolvedSymbolResponse> {
    if is_synthetic_expression(&ticker) {
        let parsed = parse_expression(&ticker)?;
        let resolved = synthesize_expression_symbol(&parsed);
        let bars = load_expression_bars(&state, &parsed, Interval::D1, Adjustment::Splits, 60, None).await?;
        let (last_price, prev_close, change, change_percent, volume) = match bars.as_slice() {
            [.., prev, last] => {
                let chg = last.close - prev.close;
                let pct = if prev.close.abs() > 1e-9 {
                    (chg / prev.close) * 100.0
                } else {
                    0.0
                };
                (Some(last.close), Some(prev.close), Some(chg), Some(pct), Some(last.volume))
            }
            [only] => (Some(only.close), Some(only.open), Some(only.close - only.open), Some(0.0), Some(only.volume)),
            [] => (None, None, None, None, None),
        };
        return Ok(ResolvedSymbolResponse {
            symbol_info: resolved,
            last_price,
            prev_close,
            change,
            change_percent,
            volume,
            vn_band: None,
        });
    }

    let key = SymbolKey::parse(&ticker)?;
    let svc = state.active_market_service();
    let resolved = match svc.resolve_symbol(&key).await {
        Ok(info) => info,
        Err(_) => synthesize_fallback_symbol(&key),
    };

    // Query latest 2 daily bars from cache for instant reference price & VN ceiling/floor band.
    let series_d1 = SeriesKey::new(&key, Interval::D1, Adjustment::Splits);
    let latest_bars = svc
        .get_bars_instant(&BarsRequest::init(series_d1, 2))
        .await
        .map(|p| p.bars)
        .unwrap_or_default();

    let (last_price, prev_close, change, change_percent, volume) = match latest_bars.as_slice() {
        [.., prev, last] => {
            let chg = last.close - prev.close;
            let pct = if prev.close.abs() > 1e-9 {
                (chg / prev.close) * 100.0
            } else {
                0.0
            };
            (
                Some(last.close),
                Some(prev.close),
                Some(chg),
                Some(pct),
                Some(last.volume),
            )
        }
        [only] => (
            Some(only.close),
            Some(only.open),
            Some(only.close - only.open),
            if only.open.abs() > 1e-9 {
                Some(((only.close - only.open) / only.open) * 100.0)
            } else {
                Some(0.0)
            },
            Some(only.volume),
        ),
        [] => (None, None, None, None, None),
    };

    let vn_band = compute_vn_band_from_bars(&resolved, &latest_bars);
    Ok(ResolvedSymbolResponse {
        symbol_info: resolved,
        last_price,
        prev_close,
        change,
        change_percent,
        volume,
        vn_band,
    })
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn get_bars(
    state: State<'_, AppState>,
    ticker: String,
    interval: String,
    mode: String,
    timestamp: Option<i64>,
    limit: Option<usize>,
    adjustment: Option<String>,
    as_of_date: Option<String>,
) -> DesktopResult<BarsPageResponse> {
    let iv: Interval = interval.parse()?;
    let adj: Adjustment = adjustment
        .as_deref()
        .map(str::parse)
        .transpose()?
        .unwrap_or(Adjustment::Splits);
    let page_limit = limit.unwrap_or(300).clamp(1, 5_000);

    // Synthetic arithmetic ratio/spread expressions (`HOSE:SSI / HOSE:VNINDEX`, `HNX:VN30F1! - HOSE:VN30`, etc.)
    if is_synthetic_expression(&ticker) {
        let parsed = parse_expression(&ticker)?;
        let pit_cut_ms = as_of_date
            .as_deref()
            .map(str::trim)
            .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
            .and_then(|d| d.and_hms_opt(23, 59, 59))
            .map(|dt| dt.and_utc().timestamp_millis());
        let replay_cut_ms = {
            let guard = state.replay.lock().await;
            guard
                .as_ref()
                .filter(|rt| rt.controller.is_active())
                .and_then(|rt| rt.controller.cutoff_timestamp(&rt.base_candles))
                .map(|ts| ts.saturating_mul(1_000))
        };
        let eff_cut = match (pit_cut_ms, replay_cut_ms) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
        let all_bars =
            load_expression_bars(&state, &parsed, iv, adj, page_limit.max(800), eff_cut).await?;
        let mode_lower = mode.to_ascii_lowercase();
        let bars: Vec<OhlcvBar> = match mode_lower.as_str() {
            "forward" => {
                let before = timestamp.unwrap_or(i64::MAX);
                let older: Vec<OhlcvBar> = all_bars
                    .iter()
                    .copied()
                    .filter(|b| b.timestamp < before)
                    .collect();
                let start = older.len().saturating_sub(page_limit);
                older[start..].to_vec()
            }
            "backward" => {
                let after = timestamp.unwrap_or(i64::MIN);
                all_bars
                    .iter()
                    .copied()
                    .filter(|b| b.timestamp > after)
                    .take(page_limit)
                    .collect()
            }
            _ => {
                let start = all_bars.len().saturating_sub(page_limit);
                all_bars[start..].to_vec()
            }
        };
        let has_older = bars
            .first()
            .and_then(|first| all_bars.first().map(|v| first.timestamp > v.timestamp))
            .unwrap_or(false);
        let has_newer = bars
            .last()
            .and_then(|last| all_bars.last().map(|v| last.timestamp < v.timestamp))
            .unwrap_or(false);
        return Ok(BarsPageResponse {
            bars,
            has_older,
            has_newer,
            source: DataSourceOrigin::CacheHit,
            symbol_info: Some(synthesize_expression_symbol(&parsed)),
            vn_band: None,
            replay_active: replay_cut_ms.is_some(),
        });
    }

    let sym = SymbolKey::parse(&ticker)?;

    // Causal Multi-Timeframe Bar Replay guard (`DEF-001`, `BUG-040`):
    // If Replay is active on this ticker (on ANY timeframe!), materialize causally up to cutoff
    // so higher-TF panes (e.g. `4h` alongside `1h`) NEVER leak future candles!
    {
        let mut replay_guard = state.replay.lock().await;
        if let Some(rt) = replay_guard.as_mut()
            && rt.controller.is_active()
            && rt.ticker.eq_ignore_ascii_case(&sym.ticker())
        {
            if !rt.tracked_intervals.contains(&iv) {
                rt.tracked_intervals.push(iv);
                rt.tracked_intervals.sort_by_key(|i| i.duration_ms());
            }
            let visible =
                crate::commands::replay::materialize_replay_bars(rt, iv, &state.market_store);
            let mode_lower = mode.to_ascii_lowercase();
            let bars: Vec<OhlcvBar> = match mode_lower.as_str() {
                "forward" => {
                    let before = timestamp.unwrap_or(i64::MAX);
                    let older: Vec<OhlcvBar> = visible
                        .iter()
                        .copied()
                        .filter(|b| b.timestamp < before)
                        .collect();
                    let start = older.len().saturating_sub(page_limit);
                    older[start..].to_vec()
                }
                "backward" => Vec::new(),
                _ => {
                    let start = visible.len().saturating_sub(page_limit);
                    visible[start..].to_vec()
                }
            };
            let has_older = bars
                .first()
                .and_then(|first| visible.first().map(|v| first.timestamp > v.timestamp))
                .unwrap_or(false);
            let sym_info = state
                .market_store
                .get_symbol(&sym)
                .ok()
                .flatten()
                .map(|(s, _)| s)
                .unwrap_or_else(|| synthesize_fallback_symbol(&sym));
            let vn_band = if mode_lower == "init" {
                compute_vn_band_from_bars(&sym_info, &visible)
            } else {
                None
            };
            return Ok(BarsPageResponse {
                bars,
                has_older,
                has_newer: false,
                source: DataSourceOrigin::CacheHit,
                symbol_info: Some(sym_info),
                vn_band,
                replay_active: true,
            });
        }
    }

    let series = SeriesKey::new(&sym, iv, adj);
    let query_mode = match mode.to_ascii_lowercase().as_str() {
        "forward" => BarQueryMode::Forward {
            before_ms: timestamp.ok_or_else(|| {
                DesktopError::Validation("forward pagination requires timestamp".to_owned())
            })?,
        },
        "backward" => BarQueryMode::Backward {
            after_ms: timestamp.ok_or_else(|| {
                DesktopError::Validation("backward pagination requires timestamp".to_owned())
            })?,
        },
        _ => BarQueryMode::Init,
    };

    let svc = state.active_market_service();
    let req = BarsRequest {
        key: series.clone(),
        mode: query_mode,
        limit: page_limit,
    };

    let (mut page, needs_revalidation) = match svc.get_bars_stale_while_revalidate(&req).await {
        Ok(swr) => (swr.page, swr.needs_revalidation),
        Err(err) => {
            warn!(ticker = %sym, %interval, error = %err, "get_bars SWR failed; falling back to instant cache");
            (svc.get_bars_instant(&req).await?, false)
        }
    };

    // Decision #3 — OHLCV Priority & PIT Lock Mode (`dnse_real` + `sumi_market.db` long-history stitch):
    // - When `as_of_date` is locked (Yumeko PIT mode) on `1D` (`splits`): prioritize verified `dnse_real`
    //   bars from Yumeko (`kVND -> VND` via Scale Guard), stitching older `sumi_market.db` bars prior to
    //   the DNSE window so MA200/AFC stay intact, and clip strictly `<= as_of_date`!
    // - In normal mode (`as_of_date = None`), if `dnse_real` has fresher `1D` bars than local cache (or offline),
    //   transparently append those fresher bars.
    let pit_as_of = as_of_date
        .as_deref()
        .map(str::trim)
        .filter(|s| s.len() == 10 && chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").is_ok())
        .map(ToOwned::to_owned);
    if iv == Interval::D1 && adj == Adjustment::Splits && matches!(query_mode, BarQueryMode::Init) {
        let ticker_clone = sym.ticker();
        let pit_clone = pit_as_of.clone();
        let dnse_opt = tokio::task::spawn_blocking(move || {
            let bridge = sumi_yumeko_bridge::YumekoReadOnlyBridge::discover_default();
            bridge
                .load_dnse_ohlcv_series(&ticker_clone, pit_clone.as_deref())
                .ok()
                .flatten()
        })
        .await
        .ok()
        .flatten();

        if let Some(dnse) = dnse_opt
            && !dnse.sumi_bars.is_empty()
        {
            if pit_as_of.is_some() {
                let first_dnse_ts = dnse.sumi_bars.first().unwrap().timestamp;
                let mut stitched: Vec<OhlcvBar> = page
                    .bars
                    .into_iter()
                    .filter(|b| b.timestamp < first_dnse_ts)
                    .collect();
                stitched.extend(dnse.sumi_bars);
                let start = stitched.len().saturating_sub(page_limit);
                page.bars = stitched[start..].to_vec();
            } else if let Some(last_dnse) = dnse.sumi_bars.last()
                && page
                    .bars
                    .last()
                    .is_none_or(|b| last_dnse.timestamp > b.timestamp)
            {
                let last_local = page.bars.last().map_or(i64::MIN, |b| b.timestamp);
                page.bars.extend(
                    dnse.sumi_bars
                        .into_iter()
                        .filter(|b| b.timestamp > last_local),
                );
                let start = page.bars.len().saturating_sub(page_limit);
                if start > 0 {
                    page.bars.drain(..start);
                }
            }
        }
    }

    if let Some(ref cut_str) = pit_as_of
        && let Ok(cut_date) = chrono::NaiveDate::parse_from_str(cut_str, "%Y-%m-%d")
        && let Some(cut_dt) = cut_date.and_hms_opt(23, 59, 59)
    {
        let cut_ms = cut_dt.and_utc().timestamp_millis();
        page.bars.retain(|b| b.timestamp <= cut_ms);
    }

    // Non-blocking Stale-While-Revalidate: skip background live ticks while PIT `as_of_date` is locked!
    if needs_revalidation && !state.is_offline() && pit_as_of.is_none() {
        let bg_svc = svc.clone();
        let bg_series = series.clone();
        let bg_tx = state.bar_updates_tx.clone();
        let ticker_norm = sym.ticker();
        let iv_str = iv.as_str().to_owned();
        tokio::spawn(async move {
            match bg_svc.revalidate_series(&bg_series, page_limit).await {
                Ok(delta_bars) => {
                    for bar in delta_bars {
                        let _ = bg_tx.send(LiveBarPayload {
                            ticker: ticker_norm.clone(),
                            interval: iv_str.clone(),
                            bar,
                            is_update: true,
                        });
                    }
                }
                Err(err) => {
                    debug!(series = %bg_series, error = %err, "background SWR revalidation skipped");
                }
            }
        });
    }

    let sym_info = page
        .symbol_info
        .clone()
        .unwrap_or_else(|| synthesize_fallback_symbol(&sym));

    // Compute Vietnam price band ONLY for `init` loads (`BUG-005`: forward historical pagination must
    // never overwrite today's live Ceiling/Reference/Floor band!).
    let vn_band = if matches!(query_mode, BarQueryMode::Init) {
        let d1_bars = if iv == Interval::D1 && !page.bars.is_empty() {
            page.bars.clone()
        } else {
            svc.get_bars_instant(&BarsRequest::init(
                SeriesKey::new(&sym, Interval::D1, adj),
                2,
            ))
            .await
            .map(|p| p.bars)
            .unwrap_or_default()
        };
        compute_vn_band_from_bars(&sym_info, &d1_bars)
    } else {
        None
    };

    Ok(BarsPageResponse {
        bars: page.bars,
        has_older: page.has_older,
        has_newer: page.has_newer,
        source: page.source,
        symbol_info: Some(sym_info),
        vn_band,
        replay_active: false,
    })
}

#[tauri::command]
pub async fn subscribe_bars(
    state: State<'_, AppState>,
    stream_id: String,
    ticker: String,
    interval: String,
    adjustment: Option<String>,
    on_bar: Channel<LiveBarPayload>,
) -> DesktopResult<bool> {
    // Bump monotonic generation and tear down any existing stream for this slot (`BUG-012` race guard!).
    let my_gen = {
        let mut gens = state.bar_stream_generations.lock().await;
        let entry = gens.entry(stream_id.clone()).or_insert(0);
        *entry = entry.saturating_add(1);
        *entry
    };
    let prev_stream = {
        let mut guard = state.bar_streams.lock().await;
        guard.remove(&stream_id)
    };
    if let Some(stream) = prev_stream {
        let _ = stream.cancel_tx.send(());
        stream.forwarder.abort();
    }

    if state.is_offline() || is_synthetic_expression(&ticker) {
        return Ok(false);
    }

    let sym = SymbolKey::parse(&ticker)?;
    let iv: Interval = interval.parse()?;
    let adj: Adjustment = adjustment
        .as_deref()
        .map(str::parse)
        .transpose()?
        .unwrap_or(Adjustment::Splits);
    let series = SeriesKey::new(&sym, iv, adj);
    let svc = state.active_market_service();

    let (cancel_tx, mut cancel_rx) = tokio::sync::oneshot::channel::<()>();
    let ticker_norm = sym.ticker();
    let iv_str = iv.as_str().to_owned();
    let mut swr_rx = state.bar_updates_tx.subscribe();
    let stream_id_log = stream_id.clone();

    let forwarder = tokio::spawn(async move {
        // Connect upstream TradingView WS in the background while immediately forwarding any
        // Stale-While-Revalidate delta bars so `subscribe_bars` returns in < 0.2ms!
        let connect_fut = svc.subscribe_bars_with_archival(series.clone());
        tokio::pin!(connect_fut);

        let mut live_pair: Option<(sumi_data::BarSubscription, tokio::task::JoinHandle<()>)> = None;

        // Phase 1: Wait for either WS connection, SWR update, or cancellation.
        loop {
            tokio::select! {
                _ = &mut cancel_rx => {
                    debug!(%ticker_norm, %iv_str, "bar stream cancelled before WS handshake completed");
                    return;
                }
                swr_ev = swr_rx.recv() => {
                    match swr_ev {
                        Ok(payload)
                            if payload.ticker.eq_ignore_ascii_case(&ticker_norm)
                                && payload.interval == iv_str =>
                        {
                            if on_bar.send(payload).is_err() {
                                return;
                            }
                        }
                        Ok(_) | Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    }
                }
                ws_res = &mut connect_fut => {
                    match ws_res {
                        Ok(pair) => {
                            live_pair = Some(pair);
                        }
                        Err(err) => {
                            warn!(stream_id = %stream_id_log, %ticker_norm, %iv_str, error = %err, "live WS bar subscription unavailable; keeping SWR bar channel active");
                        }
                    }
                    break;
                }
            }
        }

        // Phase 2: Forward both live WS ticks and SWR delta updates until cancelled.
        if let Some((mut bar_sub, archiver)) = live_pair {
            loop {
                tokio::select! {
                    _ = &mut cancel_rx => {
                        debug!(%ticker_norm, %iv_str, "bar stream cancelled by frontend");
                        break;
                    }
                    swr_ev = swr_rx.recv() => {
                        match swr_ev {
                            Ok(payload)
                                if payload.ticker.eq_ignore_ascii_case(&ticker_norm)
                                    && payload.interval == iv_str =>
                            {
                                if on_bar.send(payload).is_err() {
                                    break;
                                }
                            }
                            Ok(_) | Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                            Err(tokio::sync::broadcast::error::RecvError::Closed) => {}
                        }
                    }
                    maybe_ev = bar_sub.recv() => {
                        let Some(ev) = maybe_ev else { break; };
                        let payload = LiveBarPayload {
                            ticker: ticker_norm.clone(),
                            interval: iv_str.clone(),
                            bar: ev.bar,
                            is_update: ev.is_update,
                        };
                        if on_bar.send(payload).is_err() {
                            break;
                        }
                    }
                }
            }
            bar_sub.unsubscribe().await;
            archiver.abort();
        } else {
            // Keep forwarding background SWR updates even when live WS failed.
            loop {
                tokio::select! {
                    _ = &mut cancel_rx => break,
                    swr_ev = swr_rx.recv() => {
                        match swr_ev {
                            Ok(payload)
                                if payload.ticker.eq_ignore_ascii_case(&ticker_norm)
                                    && payload.interval == iv_str =>
                            {
                                if on_bar.send(payload).is_err() {
                                    break;
                                }
                            }
                            Ok(_) | Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                            Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                        }
                    }
                }
            }
        }
    });

    // Re-check monotonic generation under lock so an interleaved `unsubscribe_bars` never leaves a zombie stream (`BUG-012`).
    let still_current = {
        let gens = state.bar_stream_generations.lock().await;
        gens.get(&stream_id).copied() == Some(my_gen)
    };
    if !still_current {
        let _ = cancel_tx.send(());
        forwarder.abort();
        return Ok(false);
    }

    let mut guard = state.bar_streams.lock().await;
    if let Some(old) = guard.insert(
        stream_id,
        ActiveBarStream {
            cancel_tx,
            forwarder,
        },
    ) {
        let _ = old.cancel_tx.send(());
        old.forwarder.abort();
    }
    Ok(true)
}

async fn unsubscribe_bars_inner(state: &AppState, stream_id: &str) {
    {
        let mut gens = state.bar_stream_generations.lock().await;
        let entry = gens.entry(stream_id.to_owned()).or_insert(0);
        *entry = entry.saturating_add(1);
    }
    let existing = {
        let mut guard = state.bar_streams.lock().await;
        guard.remove(stream_id)
    };
    if let Some(stream) = existing {
        let _ = stream.cancel_tx.send(());
        stream.forwarder.abort();
    }
}

#[tauri::command]
pub async fn unsubscribe_bars(state: State<'_, AppState>, stream_id: String) -> DesktopResult<()> {
    unsubscribe_bars_inner(&state, &stream_id).await;
    Ok(())
}

#[tauri::command]
pub async fn subscribe_quotes(
    state: State<'_, AppState>,
    tickers: Vec<String>,
    on_quote: Channel<LiveQuote>,
) -> DesktopResult<bool> {
    unsubscribe_quotes_inner(&state).await;

    if state.is_offline() {
        return Ok(false);
    }

    // Union requested Watchlist tickers with all `armed` alert tickers so alerts on symbols
    // outside the currently viewed Watchlist still receive live quotes and fire (`BUG-017`)!
    let mut merged_tickers = tickers;
    if let Ok(armed_alerts) = state.workspace.list_alerts() {
        for a in armed_alerts {
            if a.status == "armed"
                && !merged_tickers
                    .iter()
                    .any(|t| t.eq_ignore_ascii_case(&a.ticker))
            {
                merged_tickers.push(a.ticker);
            }
        }
    }

    if merged_tickers.is_empty() {
        return Ok(false);
    }

    let keys: Vec<SymbolKey> = merged_tickers
        .iter()
        .filter_map(|t| SymbolKey::parse(t).ok())
        .collect();
    if keys.is_empty() {
        return Ok(false);
    }

    let svc = state.active_market_service();
    let mut quote_sub = match svc.subscribe_quotes(keys).await {
        Ok(sub) => sub,
        Err(err) => {
            warn!(error = %err, "live quote stream unavailable");
            return Ok(false);
        }
    };

    let (cancel_tx, mut cancel_rx) = tokio::sync::oneshot::channel::<()>();
    let forwarder = tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = &mut cancel_rx => break,
                maybe_q = quote_sub.recv() => {
                    let Some(q) = maybe_q else { break; };
                    if on_quote.send(q).is_err() {
                        break;
                    }
                }
            }
        }
        quote_sub.unsubscribe().await;
    });

    let mut guard = state.quote_stream.lock().await;
    *guard = Some(ActiveQuoteStream {
        cancel_tx,
        forwarder,
    });
    Ok(true)
}

async fn unsubscribe_quotes_inner(state: &AppState) {
    let existing = {
        let mut guard = state.quote_stream.lock().await;
        guard.take()
    };
    if let Some(stream) = existing {
        let _ = stream.cancel_tx.send(());
        stream.forwarder.abort();
    }
}

#[tauri::command]
pub async fn unsubscribe_quotes(state: State<'_, AppState>) -> DesktopResult<()> {
    unsubscribe_quotes_inner(&state).await;
    Ok(())
}

#[tauri::command]
pub async fn get_watchlist_snapshots(
    state: State<'_, AppState>,
    tickers: Vec<String>,
) -> DesktopResult<Vec<WatchlistQuoteSnapshot>> {
    let keys: Vec<SymbolKey> = tickers
        .iter()
        .filter_map(|raw| SymbolKey::parse(raw).ok())
        .collect();

    // On a fresh cold-start machine (before CafeF import), hydrate missing daily series online
    // (bounded to 16 symbols per batch, protected by sumi-data's rate limiter) so Watchlist shows
    // real prices & VN ceiling/floor bands out-of-the-box even outside trading hours.
    if !state.is_offline() && !keys.is_empty() {
        let store_check = state.market_store.clone();
        let keys_check = keys.clone();
        let missing: Vec<SymbolKey> = tokio::task::spawn_blocking(move || {
            keys_check
                .into_iter()
                .filter(|key| {
                    let series = SeriesKey::new(key, Interval::D1, Adjustment::Splits);
                    store_check
                        .query_bars(&series, BarQueryMode::Init, 2)
                        .map(|r| r.bars.is_empty())
                        .unwrap_or(true)
                })
                .take(16)
                .collect()
        })
        .await
        .unwrap_or_default();

        if !missing.is_empty() {
            let svc = state.active_market_service();
            for key in missing {
                let series = SeriesKey::new(&key, Interval::D1, Adjustment::Splits);
                let _ = svc.get_bars(&BarsRequest::init(series, 30)).await;
            }
        }
    }

    let store = state.market_store.clone();
    tokio::task::spawn_blocking(move || {
        let mut out = Vec::with_capacity(keys.len());
        for key in keys {
            let sym_info = store
                .get_symbol(&key)
                .ok()
                .flatten()
                .map(|(s, _)| s)
                .unwrap_or_else(|| synthesize_fallback_symbol(&key));

            let series = SeriesKey::new(&key, Interval::D1, Adjustment::Splits);
            let bars = store
                .query_bars(&series, BarQueryMode::Init, 2)
                .map(|r| r.bars)
                .unwrap_or_default();

            let (last_price, prev_close, open, high, low, change, change_percent, volume, timestamp_ms) =
                match bars.as_slice() {
                    [.., prev, last] => {
                        let chg = last.close - prev.close;
                        let pct = if prev.close.abs() > 1e-9 {
                            (chg / prev.close) * 100.0
                        } else {
                            0.0
                        };
                        (
                            Some(last.close),
                            Some(prev.close),
                            Some(last.open),
                            Some(last.high),
                            Some(last.low),
                            Some(chg),
                            Some(pct),
                            Some(last.volume),
                            Some(last.timestamp),
                        )
                    }
                    [only] => (
                        Some(only.close),
                        Some(only.open),
                        Some(only.open),
                        Some(only.high),
                        Some(only.low),
                        Some(only.close - only.open),
                        if only.open.abs() > 1e-9 {
                            Some(((only.close - only.open) / only.open) * 100.0)
                        } else {
                            Some(0.0)
                        },
                        Some(only.volume),
                        Some(only.timestamp),
                    ),
                    [] => (None, None, None, None, None, None, None, None, None),
                };

            let vn_band = compute_vn_band_from_bars(&sym_info, &bars);
            out.push(WatchlistQuoteSnapshot {
                ticker: sym_info.ticker.clone(),
                exchange: sym_info.exchange.clone(),
                symbol: sym_info.symbol.clone(),
                description: sym_info.description.clone(),
                last_price,
                prev_close,
                open,
                high,
                low,
                bid: None,
                ask: None,
                change,
                change_percent,
                volume,
                price_precision: sym_info.price_precision,
                vn_band,
                timestamp_ms,
            });
        }
        Ok(out)
    })
    .await
    .map_err(|e| DesktopError::Internal(e.to_string()))?
}

async fn build_data_status(state: &AppState) -> DesktopResult<DataStatusResponse> {
    let stats = state.active_market_service().store_stats().await?;
    let store = state.market_store.clone();
    let now_ms = chrono::Utc::now().timestamp_millis();
    let freshness = tokio::task::spawn_blocking(move || store.vn_freshness_summary(now_ms))
        .await
        .map_err(|e| DesktopError::Internal(e.to_string()))??;
    let yumeko_db_detected = yumeko_data_dir().join("yumeko-app.sqlite").is_file();
    Ok(DataStatusResponse {
        stats,
        offline_mode: state.is_offline(),
        freshness,
        data_dir: state.data_dir.display().to_string(),
        config_dir: state.config_dir.display().to_string(),
        yumeko_db_detected,
    })
}

#[tauri::command]
pub async fn get_data_status(state: State<'_, AppState>) -> DesktopResult<DataStatusResponse> {
    build_data_status(&state).await
}

#[tauri::command]
pub async fn set_offline_mode(
    state: State<'_, AppState>,
    offline: bool,
) -> DesktopResult<DataStatusResponse> {
    state.set_offline(offline);
    if offline {
        unsubscribe_quotes_inner(&state).await;
    }
    build_data_status(&state).await
}

#[tauri::command]
pub async fn import_cafef_dir(
    state: State<'_, AppState>,
    dir: String,
    include_raw: Option<bool>,
) -> DesktopResult<CafeFImportSummary> {
    let store = state.market_store.clone();
    let want_raw = include_raw.unwrap_or(true);
    let summary = tokio::task::spawn_blocking(move || {
        let raw_path = std::path::PathBuf::from(dir.trim());
        let resolved_dir = if raw_path.exists() {
            raw_path
        } else {
            crate::state::workspace_ancestor_dirs()
                .into_iter()
                .map(|root| root.join(&raw_path))
                .find(|cand| cand.exists())
                .unwrap_or(raw_path)
        };
        let cfg = CafeFImportConfig {
            include_raw: want_raw,
            ..CafeFImportConfig::default()
        };
        let importer = CafeFImporter::new(cfg);
        importer
            .import_dir(resolved_dir, &store)
            .map_err(DesktopError::from)
    })
    .await
    .map_err(|e| DesktopError::Internal(e.to_string()))??;
    state.market_service.invalidate_ram_cache();
    state.invalidate_analog_memory_bank();
    Ok(summary)
}

/// Hydrates the full Vietnam listed market catalog (`~1,600+` HOSE/HNX/UPCOM symbols) via
/// `tradingview-rs::list_symbols` so Clean-Slate machines & Screener cover the entire VN market.
#[tauri::command]
pub async fn sync_vn_universe_catalog(
    state: State<'_, AppState>,
) -> DesktopResult<VnUniverseSyncSummary> {
    if state.is_offline() {
        return Err(DesktopError::Validation(
            "Không thể đồng bộ danh mục mã VN khi đang bật chế độ Offline.".to_owned(),
        ));
    }
    let started = std::time::Instant::now();
    let symbols = state
        .market_service
        .provider()
        .list_vn_universe_symbols()
        .await?;
    let discovered = symbols.len();
    let store = state.market_store.clone();
    let upserted = tokio::task::spawn_blocking(move || store.upsert_symbols_bulk(&symbols))
        .await
        .map_err(|e| DesktopError::Internal(e.to_string()))??;
    Ok(VnUniverseSyncSummary {
        discovered_symbols: discovered,
        upserted_symbols: upserted,
        elapsed_ms: started.elapsed().as_millis() as u64,
    })
}

/// Incrementally gap-fills stale Vietnam `1D` (`splits`) EOD bars across the requested scope
/// (`"QMV30"`, `"QMV60"`, `"WATCHLIST"`, `"HOSE"`, `"HNX"`, `"UPCOM"`, `"ALL_VN"`) while streaming
/// real-time progress over a Tauri IPC Channel (`Eliminates the Stale EOD Trap`).
#[tauri::command]
pub async fn sync_universe_eod(
    state: State<'_, AppState>,
    scope: String,
    tickers: Option<Vec<String>>,
    max_symbols: Option<usize>,
    on_progress: Channel<SyncProgressPayload>,
) -> DesktopResult<SyncProgressPayload> {
    if state.is_offline() {
        return Err(DesktopError::Validation(
            "Không thể đồng bộ nến EOD khi đang bật chế độ Offline.".to_owned(),
        ));
    }

    let scope_upper = scope.trim().to_ascii_uppercase();
    let now_ms = chrono::Utc::now().timestamp_millis();
    let (_, expected_ms) = latest_expected_vn_trading_session_ms(now_ms);

    let explicit_keys: Option<Vec<SymbolKey>> = match scope_upper.as_str() {
        "WATCHLIST" => Some(
            tickers
                .unwrap_or_default()
                .iter()
                .filter_map(|t| SymbolKey::parse(t).ok())
                .collect(),
        ),
        "QMV30" => Some(
            sumi_core::scanner::qmv_universe::QMV30_SYMBOLS
                .iter()
                .filter_map(|s| {
                    let ex = sumi_data::infer_vn_exchange_for_ticker(s, false);
                    SymbolKey::new(ex, s).ok()
                })
                .collect(),
        ),
        "QMV60" => Some(
            sumi_core::scanner::qmv_universe::QMV60_SYMBOLS
                .iter()
                .filter_map(|s| {
                    let ex = sumi_data::infer_vn_exchange_for_ticker(s, false);
                    SymbolKey::new(ex, s).ok()
                })
                .collect(),
        ),
        "HOSE" | "HNX" | "UPCOM" => {
            let ex_filter = scope_upper.clone();
            let store = state.market_store.clone();
            let found = tokio::task::spawn_blocking(move || {
                store.search_local_symbols("", Some(&ex_filter), 2_000)
            })
            .await
            .map_err(|e| DesktopError::Internal(e.to_string()))??;
            Some(
                found
                    .into_iter()
                    .filter_map(|h| SymbolKey::parse(&h.ticker).ok())
                    .collect(),
            )
        }
        _ => None, // ALL_VN cached symbols
    };

    let store = state.market_store.clone();
    let mut stale_list = tokio::task::spawn_blocking(move || {
        store.list_stale_vn_symbols(explicit_keys.as_deref(), expected_ms)
    })
    .await
    .map_err(|e| DesktopError::Internal(e.to_string()))??;

    let cap = max_symbols.unwrap_or(500).clamp(1, 2_000);
    stale_list.truncate(cap);

    let total = stale_list.len();
    let mut updated_symbols = 0_usize;
    let mut updated_bars = 0_usize;
    let mut failed_symbols = 0_usize;

    let svc = state.active_market_service();
    for (idx, (sym_key, _latest)) in stale_list.into_iter().enumerate() {
        let series = SeriesKey::new(&sym_key, Interval::D1, Adjustment::Splits);
        match svc.revalidate_series(&series, 300).await {
            Ok(delta) => {
                updated_symbols += 1;
                updated_bars += delta.len();
            }
            Err(err) => {
                debug!(ticker = %sym_key, error = %err, "EOD sync failed for symbol");
                failed_symbols += 1;
            }
        }
        let payload = SyncProgressPayload {
            scope: scope_upper.clone(),
            completed: idx + 1,
            total,
            current_ticker: sym_key.ticker(),
            updated_symbols,
            updated_bars,
            failed_symbols,
            done: false,
        };
        let _ = on_progress.send(payload);
    }

    if updated_bars > 0 {
        state.invalidate_analog_memory_bank();
    }

    let final_payload = SyncProgressPayload {
        scope: scope_upper,
        completed: total,
        total,
        current_ticker: String::new(),
        updated_symbols,
        updated_bars,
        failed_symbols,
        done: true,
    };
    let _ = on_progress.send(final_payload.clone());
    Ok(final_payload)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SymbolFundamentalsResponse {
    pub ticker: String,
    pub symbol: String,
    pub exchange: String,
    pub source: String,
    pub sector: Option<String>,
    pub industry: Option<String>,
    pub market_cap_basic: Option<f64>,
    pub pe: Option<f64>,
    pub pb: Option<f64>,
    pub ps: Option<f64>,
    pub ev_ebitda: Option<f64>,
    pub roe: Option<f64>,
    pub roa: Option<f64>,
    pub eps: Option<f64>,
    pub bvps: Option<f64>,
    pub debt_to_equity: Option<f64>,
    pub total_revenue: Option<f64>,
    pub total_shares_outstanding: Option<f64>,
    pub earnings_release_date: Option<i64>,
    pub business_description: Option<String>,
    pub quarterly_valuations: Vec<ValuationQuarterDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComparePointDto {
    pub timestamp: i64,
    pub pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompareSeriesItemDto {
    pub ticker: String,
    pub symbol: String,
    pub points: Vec<ComparePointDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompareSeriesResponse {
    pub base_ticker: String,
    pub series: Vec<CompareSeriesItemDto>,
}

/// Returns auto-detected corporate action & earnings markers (`D` Dividend, `S` Split/Bonus, `E` Earnings/BCTC).
#[tauri::command]
pub async fn get_corporate_events(
    state: State<'_, AppState>,
    ticker: String,
) -> DesktopResult<Vec<CorporateEventMarker>> {
    if is_synthetic_expression(&ticker) {
        return Ok(Vec::new());
    }
    let sym = SymbolKey::parse(&ticker)?;
    let store = state.market_store.clone();
    tokio::task::spawn_blocking(move || {
        let mut events = store.compute_corporate_events(&sym)?;
        let mut seen_e_dates: HashSet<String> = HashSet::new();

        // Merge `E` (Earnings / BCTC) markers from read-only Yumeko DN01 snapshots
        let bridge = YumekoReadOnlyBridge::discover_default();
        if let Ok(ctx) = bridge.load_topdown_context(&sym.ticker(), None) {
            for q in ctx.dn01_valuations {
                let Some(date_str) = q.content_date.as_deref().map(str::trim).filter(|s| s.len() >= 10) else {
                    continue;
                };
                let iso = &date_str[..10];
                if !seen_e_dates.insert(iso.to_owned()) {
                    continue;
                }
                if let Ok(d) = chrono::NaiveDate::parse_from_str(iso, "%Y-%m-%d")
                    && let Some(dt) = d.and_hms_opt(0, 0, 0)
                {
                    let eps_str = q.eps.map(|v| format!("EPS {v:.0}")).unwrap_or_default();
                    let pe_str = q.pe.map(|v| format!("P/E {v:.1}x")).unwrap_or_default();
                    let detail = format!("BCTC {} ({}) — {} {}", q.period, iso, eps_str, pe_str)
                        .trim()
                        .to_owned();
                    events.push(CorporateEventMarker {
                        timestamp_ms: dt.and_utc().timestamp_millis(),
                        date: iso.to_owned(),
                        kind: "E".to_owned(),
                        label: format!("BCTC {}", q.period),
                        factor: q.eps,
                        detail,
                    });
                }
            }
        }

        // Fallback/supplement: TradingView cached `earnings_release_date`
        if let Ok(Some(snap)) = store.get_fundamental_snapshot(&sym)
            && let Some(ts_sec) = snap.earnings_release_date
            && ts_sec > 0
        {
            let ts_ms = if ts_sec > 10_000_000_000 {
                ts_sec
            } else {
                ts_sec.saturating_mul(1_000)
            };
            let iso = sumi_data::format_ymd_utc(ts_ms);
            if seen_e_dates.insert(iso.clone()) {
                events.push(CorporateEventMarker {
                    timestamp_ms: ts_ms,
                    date: iso.clone(),
                    kind: "E".to_owned(),
                    label: "Kỳ BCTC".to_owned(),
                    factor: snap.eps,
                    detail: format!("Ngày công bố KQKD dự kiến/gần nhất: {iso}"),
                });
            }
        }

        events.sort_by_key(|e| e.timestamp_ms);
        Ok(events)
    })
    .await
    .map_err(|e| DesktopError::Internal(e.to_string()))?
}

/// Returns unified fundamental metrics & 8-quarter DN01 time series for `ticker`:
/// Priority 1: Read-only Yumeko DN01 (`symbol_valuation_snapshots`),
/// Priority 2: Local `sumi_market.db` `fundamentals` / `ResolvedSymbol` cache.
#[tauri::command]
pub async fn get_symbol_fundamentals(
    state: State<'_, AppState>,
    ticker: String,
    as_of: Option<String>,
) -> DesktopResult<SymbolFundamentalsResponse> {
    if is_synthetic_expression(&ticker) {
        return Ok(SymbolFundamentalsResponse {
            ticker: ticker.clone(),
            symbol: ticker,
            exchange: "EXPR".to_owned(),
            source: "none".to_owned(),
            sector: None,
            industry: None,
            market_cap_basic: None,
            pe: None,
            pb: None,
            ps: None,
            ev_ebitda: None,
            roe: None,
            roa: None,
            eps: None,
            bvps: None,
            debt_to_equity: None,
            total_revenue: None,
            total_shares_outstanding: None,
            earnings_release_date: None,
            business_description: None,
            quarterly_valuations: Vec::new(),
        });
    }
    let sym = SymbolKey::parse(&ticker)?;
    let store = state.market_store.clone();
    tokio::task::spawn_blocking(move || {
        let local_snap = store.get_fundamental_snapshot(&sym).ok().flatten();
        let sym_cached = store.get_symbol(&sym).ok().flatten().map(|(s, _)| s);
        let qmv_prof = sumi_core::scanner::lookup_qmv_profile(&sym.symbol);

        let bridge = YumekoReadOnlyBridge::discover_default();
        let ctx_opt = bridge.load_topdown_context(&sym.ticker(), as_of.as_deref()).ok();
        let valuations = ctx_opt
            .as_ref()
            .map(|c| c.dn01_valuations.clone())
            .unwrap_or_default();
        let latest_q = valuations.first();

        let source = if latest_q.is_some() {
            "yumeko_dn01".to_owned()
        } else if local_snap.is_some() {
            "tradingview_cache".to_owned()
        } else {
            "none".to_owned()
        };

        let sector = local_snap
            .as_ref()
            .and_then(|s| s.sector.clone())
            .or_else(|| sym_cached.as_ref().and_then(|s| s.sector.clone()))
            .or_else(|| qmv_prof.map(|p| p.sector.label_vi().to_owned()));
        let industry = local_snap
            .as_ref()
            .and_then(|s| s.industry.clone())
            .or_else(|| sym_cached.as_ref().and_then(|s| s.industry.clone()));
        let market_cap_basic = local_snap
            .as_ref()
            .and_then(|s| s.market_cap_basic)
            .or_else(|| sym_cached.as_ref().and_then(|s| s.market_cap_basic));
        let pe = latest_q
            .and_then(|q| q.pe)
            .or_else(|| local_snap.as_ref().and_then(|s| s.pe_ttm))
            .or_else(|| sym_cached.as_ref().and_then(|s| s.price_earnings_ttm));
        let pb = latest_q
            .and_then(|q| q.pb)
            .or_else(|| local_snap.as_ref().and_then(|s| s.pb));
        let roe = latest_q
            .and_then(|q| q.roe)
            .or_else(|| local_snap.as_ref().and_then(|s| s.roe));
        let eps = latest_q
            .and_then(|q| q.eps)
            .or_else(|| local_snap.as_ref().and_then(|s| s.eps))
            .or_else(|| sym_cached.as_ref().and_then(|s| s.last_annual_eps));

        Ok(SymbolFundamentalsResponse {
            ticker: sym.ticker(),
            symbol: sym.symbol.clone(),
            exchange: sym.exchange.clone(),
            source,
            sector,
            industry,
            market_cap_basic,
            pe,
            pb,
            ps: latest_q.and_then(|q| q.ps),
            ev_ebitda: latest_q.and_then(|q| q.ev_ebitda),
            roe,
            roa: latest_q.and_then(|q| q.roa),
            eps,
            bvps: latest_q.and_then(|q| q.bvps),
            debt_to_equity: latest_q.and_then(|q| q.debt_to_equity),
            total_revenue: local_snap
                .as_ref()
                .and_then(|s| s.total_revenue)
                .or_else(|| sym_cached.as_ref().and_then(|s| s.total_revenue)),
            total_shares_outstanding: local_snap
                .as_ref()
                .and_then(|s| s.total_shares_outstanding)
                .or_else(|| sym_cached.as_ref().and_then(|s| s.total_shares_outstanding)),
            earnings_release_date: local_snap
                .as_ref()
                .and_then(|s| s.earnings_release_date)
                .or_else(|| sym_cached.as_ref().and_then(|s| s.earnings_release_date)),
            business_description: local_snap
                .as_ref()
                .and_then(|s| s.business_description.clone())
                .or_else(|| sym_cached.as_ref().and_then(|s| s.business_description.clone())),
            quarterly_valuations: valuations,
        })
    })
    .await
    .map_err(|e| DesktopError::Internal(e.to_string()))?
}

/// Computes strictly causal `0.00%`-rebased multi-symbol comparison curves aligned to `base_ticker`
/// (supports plain symbols, indices, and synthetic expressions; honors active Replay & PIT `as_of_date`).
#[tauri::command]
pub async fn get_compare_series(
    state: State<'_, AppState>,
    base_ticker: String,
    compare_tickers: Vec<String>,
    interval: String,
    limit: Option<usize>,
    as_of_date: Option<String>,
) -> DesktopResult<CompareSeriesResponse> {
    let iv: Interval = interval.parse()?;
    let adj = Adjustment::Splits;
    let bar_limit = limit.unwrap_or(400).clamp(30, 3_000);

    let pit_cut_ms = as_of_date
        .as_deref()
        .map(str::trim)
        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .and_then(|d| d.and_hms_opt(23, 59, 59))
        .map(|dt| dt.and_utc().timestamp_millis());
    let replay_cut_ms = {
        let guard = state.replay.lock().await;
        guard
            .as_ref()
            .filter(|rt| rt.controller.is_active())
            .and_then(|rt| rt.controller.cutoff_timestamp(&rt.base_candles))
            .map(|ts| ts.saturating_mul(1_000))
    };
    let eff_cut = match (pit_cut_ms, replay_cut_ms) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    };

    let load_one = |raw: String| {
        let svc = state.active_market_service();
        let offline = state.is_offline();
        async move {
            if is_synthetic_expression(&raw) {
                let Ok(parsed) = parse_expression(&raw) else {
                    return Vec::new();
                };
                let mut map = HashMap::new();
                for s in &parsed.symbols {
                    let req = BarsRequest::init(SeriesKey::new(s, iv, adj), bar_limit);
                    let mut bars = match svc.get_bars_instant(&req).await {
                        Ok(p) if !p.bars.is_empty() => p.bars,
                        _ if !offline => svc.get_bars(&req).await.map(|p| p.bars).unwrap_or_default(),
                        _ => Vec::new(),
                    };
                    if let Some(cut) = eff_cut {
                        bars.retain(|b| b.timestamp <= cut);
                    }
                    map.insert(s.clone(), bars);
                }
                return evaluate_expression_ohlcv(&parsed, &map);
            }
            let Ok(sk) = SymbolKey::parse(&raw) else {
                return Vec::new();
            };
            let req = BarsRequest::init(SeriesKey::new(&sk, iv, adj), bar_limit);
            let mut bars = match svc.get_bars_instant(&req).await {
                Ok(p) if !p.bars.is_empty() => p.bars,
                _ if !offline => svc.get_bars(&req).await.map(|p| p.bars).unwrap_or_default(),
                _ => Vec::new(),
            };
            if let Some(cut) = eff_cut {
                bars.retain(|b| b.timestamp <= cut);
            }
            bars
        }
    };

    let base_bars = load_one(base_ticker.clone()).await;
    let mut series = Vec::new();
    for raw_cmp in compare_tickers.into_iter().take(6) {
        let trimmed = raw_cmp.trim().to_owned();
        if trimmed.is_empty() {
            continue;
        }
        let cmp_bars = load_one(trimmed.clone()).await;
        let pts: Vec<ComparePointDto> = compute_rebased_comparison_series(&base_bars, &cmp_bars)
            .into_iter()
            .map(|(timestamp, pct)| ComparePointDto { timestamp, pct })
            .collect();
        let sym_label = SymbolKey::parse(&trimmed)
            .map(|k| k.symbol)
            .unwrap_or_else(|_| trimmed.clone());
        series.push(CompareSeriesItemDto {
            ticker: trimmed,
            symbol: sym_label,
            points: pts,
        });
    }

    Ok(CompareSeriesResponse {
        base_ticker,
        series,
    })
}
