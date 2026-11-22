use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use sumi_data::{
    Adjustment, BarQueryMode, BarsRequest, CafeFImportConfig, CafeFImportSummary, CafeFImporter,
    DataSourceOrigin, Interval, LiveQuote, OhlcvBar, ResolvedSymbol, SeriesKey, StoreStats,
    SymbolKey, SymbolSearchResult, VietnamPriceBand,
};
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
    }
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
pub async fn get_bars(
    state: State<'_, AppState>,
    ticker: String,
    interval: String,
    mode: String,
    timestamp: Option<i64>,
    limit: Option<usize>,
    adjustment: Option<String>,
) -> DesktopResult<BarsPageResponse> {
    let sym = SymbolKey::parse(&ticker)?;
    let iv: Interval = interval.parse()?;
    let adj: Adjustment = adjustment
        .as_deref()
        .map(str::parse)
        .transpose()?
        .unwrap_or(Adjustment::Splits);
    let page_limit = limit.unwrap_or(300).clamp(1, 5_000);

    // Causal Bar Replay guard (DEF-001): if Replay is active on this ticker+interval, never leak future bars!
    {
        let replay_guard = state.replay.lock().await;
        if let Some(rt) = replay_guard.as_ref()
            && rt.controller.is_active()
            && rt.ticker.eq_ignore_ascii_case(&sym.ticker())
            && rt.interval == iv
        {
            let cutoff_idx = rt
                .controller
                .base_cursor_index
                .min(rt.base_bars.len().saturating_sub(1));
            let visible = &rt.base_bars[..=cutoff_idx];
            let bars: Vec<OhlcvBar> = match mode.to_ascii_lowercase().as_str() {
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
            let vn_band = compute_vn_band_from_bars(&sym_info, visible);
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

    let (page, needs_revalidation) = match svc.get_bars_stale_while_revalidate(&req).await {
        Ok(swr) => (swr.page, swr.needs_revalidation),
        Err(err) => {
            warn!(ticker = %sym, %interval, error = %err, "get_bars SWR failed; falling back to instant cache");
            (svc.get_bars_instant(&req).await?, false)
        }
    };

    // Non-blocking Stale-While-Revalidate: spawn background upstream refresh & push delta bars
    // smoothly over `bar_updates_tx` so subscribed charts update without blocking symbol switches!
    if needs_revalidation && !state.is_offline() {
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

    // Compute Vietnam price band against latest D1 bars for consistent daily ceiling/floor lines.
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
    let vn_band = compute_vn_band_from_bars(&sym_info, &d1_bars);

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
    on_bar: Channel<LiveBarPayload>,
) -> DesktopResult<bool> {
    // Tear down any existing stream for this slot first so subscriptions never leak.
    unsubscribe_bars_inner(&state, &stream_id).await;

    if state.is_offline() {
        return Ok(false);
    }

    let sym = SymbolKey::parse(&ticker)?;
    let iv: Interval = interval.parse()?;
    let series = SeriesKey::new(&sym, iv, Adjustment::Splits);
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

    let mut guard = state.bar_streams.lock().await;
    guard.insert(
        stream_id,
        ActiveBarStream {
            cancel_tx,
            forwarder,
        },
    );
    Ok(true)
}

async fn unsubscribe_bars_inner(state: &AppState, stream_id: &str) {
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

    if state.is_offline() || tickers.is_empty() {
        return Ok(false);
    }

    let keys: Vec<SymbolKey> = tickers
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

            let (last_price, prev_close, change, change_percent, volume, timestamp_ms) =
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
                            Some(chg),
                            Some(pct),
                            Some(last.volume),
                            Some(last.timestamp),
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
                        Some(only.timestamp),
                    ),
                    [] => (None, None, None, None, None, None),
                };

            let vn_band = compute_vn_band_from_bars(&sym_info, &bars);
            out.push(WatchlistQuoteSnapshot {
                ticker: sym_info.ticker.clone(),
                exchange: sym_info.exchange.clone(),
                symbol: sym_info.symbol.clone(),
                description: sym_info.description.clone(),
                last_price,
                prev_close,
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

#[tauri::command]
pub async fn get_data_status(state: State<'_, AppState>) -> DesktopResult<DataStatusResponse> {
    let stats = state.active_market_service().store_stats().await?;
    Ok(DataStatusResponse {
        stats,
        offline_mode: state.is_offline(),
    })
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
    let stats = state.active_market_service().store_stats().await?;
    Ok(DataStatusResponse {
        stats,
        offline_mode: state.is_offline(),
    })
}

#[tauri::command]
pub async fn import_cafef_dir(
    state: State<'_, AppState>,
    dir: String,
) -> DesktopResult<CafeFImportSummary> {
    let store = state.market_store.clone();
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
        let importer = CafeFImporter::new(CafeFImportConfig::default());
        importer
            .import_dir(resolved_dir, &store)
            .map_err(DesktopError::from)
    })
    .await
    .map_err(|e| DesktopError::Internal(e.to_string()))??;
    state.market_service.invalidate_ram_cache();
    Ok(summary)
}
