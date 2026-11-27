use serde::{Deserialize, Serialize};
use sumi_core::data::Timeframe;
use sumi_core::model::Candle;
use sumi_core::scanner::{
    BBHorizon, ConfluenceWeightProfile, MarketRegime, PrecomputedCatalogMatrix, QmvMoneyTier,
    QmvPriceTier, QmvSector, QmvSectorGroup, SIGNAL_CATALOG, SignalCategory, SignalDirection,
    SignalOutputType, catalog_entry_index, evaluate_symbol_scan, is_qmv_screener_signal,
    lookup_qmv_profile,
};
use sumi_data::{Adjustment, BarQueryMode, BarsRequest, Interval, SeriesKey, SymbolKey};
use tauri::State;

use crate::commands::market::curated_catalog;
use crate::error::{DesktopError, DesktopResult};
use crate::state::AppState;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignalCatalogItemDto {
    pub id: String,
    pub ast_alias: String,
    pub label_en: String,
    pub label_vi: String,
    pub description: String,
    pub category: String,
    pub category_label: String,
    pub output_type: String,
    pub status: String,
    pub warmup_bars: usize,
    pub is_qmv_core: bool,
    pub is_ohlcv_proxy: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StockScreenerInput {
    /// `"ALL"` | `"ALL_VN"` | `"HOSE"` | `"HNX"` | `"UPCOM"` | `"QMV30"` | `"QMV60"` | `"DT1"` | `"DT2"` | `"DT3"` | `"WATCHLIST"`
    pub exchange_filter: String,
    /// Optional explicit tickers (used when `exchange_filter == "WATCHLIST"`).
    pub tickers: Option<Vec<String>>,
    /// Optional filter by a single `SIGNAL_CATALOG` entry id (legacy compatibility).
    pub signal_id: Option<String>,
    /// Multi-condition QMV Screener `AND` signal filter (`signal_ids` — all selected signals must fire!).
    #[serde(default)]
    pub signal_ids: Vec<String>,
    /// Optional QMV basket filter (`"QMV30"` | `"QMV60"` | `"CORE10"`).
    #[serde(default)]
    pub qmv_basket: Option<String>,
    /// Optional QMV sector group (`"GROUP_1"` | `"GROUP_2"` | `"GROUP_3"`).
    #[serde(default)]
    pub sector_group_filter: Option<String>,
    /// Optional QMV sector (`"BANKING"`, `"BROKERAGE"`, `"REAL_ESTATE"`, `"CONSTRUCTION_STEEL"`, ...).
    #[serde(default)]
    pub sector_filter: Option<String>,
    /// Optional QMV money tier (`"DT1"` | `"DT2"` | `"DT3"`).
    #[serde(default)]
    pub money_tier_filter: Option<String>,
    /// Optional QMV price tier (`"UNDER_10K"` | `"10K_30K"` | `"30K_50K"` | `"OVER_50K"`).
    #[serde(default)]
    pub price_tier_filter: Option<String>,
    /// Optional minimum 20-session average volume (`>= 50,000` QMV V7 liquidity gate).
    #[serde(default)]
    pub min_avg_vol_20: Option<f64>,
    /// Optional signal category filter (`"all"`, `"health"`, `"flow"`, `"divergence"`, ...).
    pub category_filter: Option<String>,
    /// Optional regime filter (`"Uptrend"`, `"Downtrend"`, `"Pullback"`, `"Recovery"`, `"SidewaysSqueeze"`, `"Neutral"`).
    pub regime_filter: Option<String>,
    /// Optional text query matching ticker or company name.
    pub search_query: Option<String>,
    /// Limit maximum scanned symbols (default 500 to cover all 475 Vietnam symbols).
    pub max_symbols: Option<usize>,
    /// Multi-condition numeric filters (v2.2): RSI min/max, RVOL min, Change% min/max, MACD sign, Health Score min.
    #[serde(default)]
    pub min_rsi: Option<f64>,
    #[serde(default)]
    pub max_rsi: Option<f64>,
    #[serde(default)]
    pub min_rvol: Option<f64>,
    #[serde(default)]
    pub min_change_pct: Option<f64>,
    #[serde(default)]
    pub max_change_pct: Option<f64>,
    /// `"positive"` | `"negative"` | `"all"`
    #[serde(default)]
    pub macd_filter: Option<String>,
    #[serde(default)]
    pub min_health_score: Option<f64>,
    /// Optional explicit opt-in to refresh up to 60 stale symbols before running the scan.
    #[serde(default)]
    pub auto_refresh_stale: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenerRowDto {
    pub ticker: String,
    pub exchange: String,
    pub symbol: String,
    pub description: String,
    pub last_close: f64,
    pub change_pct: f64,
    pub last_volume: f64,
    pub avg_vol_20: Option<f64>,
    pub rvol_20: Option<f64>,
    pub rsi_14: Option<f64>,
    pub macd_hist: Option<f64>,
    pub atr_pct: Option<f64>,
    pub regime: String,
    pub regime_short: String,
    pub health_score: Option<f64>,
    pub bullish_signals: u8,
    pub bearish_signals: u8,
    pub matched_signals: Vec<String>,
    pub selected_signal_value: Option<String>,
    pub last_timestamp_ms: i64,
    pub qmv_sector_group: String,
    pub qmv_sector: String,
    pub qmv_sector_label: String,
    pub money_tier: String,
    pub in_qmv30: bool,
    pub in_qmv60: bool,
    pub price_tier: String,
    pub qmv_flow_z50_t05: Option<f64>,
    pub qmv_flow_z50_t20: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StockScreenerResponse {
    pub total_catalog_signals: usize,
    pub scanned_symbols: usize,
    pub matched_symbols: usize,
    pub elapsed_ms: u64,
    pub expected_session_date: String,
    pub fresh_symbols_count: usize,
    pub stale_symbols_count: usize,
    pub median_cached_date: Option<String>,
    pub rows: Vec<ScreenerRowDto>,
}

#[tauri::command]
pub fn get_signal_catalog() -> Vec<SignalCatalogItemDto> {
    SIGNAL_CATALOG
        .iter()
        .map(|entry| SignalCatalogItemDto {
            id: entry.id.to_owned(),
            ast_alias: entry.ast_alias.to_owned(),
            label_en: entry.label_en.to_owned(),
            label_vi: entry.label_vi.to_owned(),
            description: entry.description.to_owned(),
            category: entry.category.key().to_owned(),
            category_label: entry.category.full_label().to_owned(),
            output_type: entry.output_type.short_label().to_owned(),
            status: entry.status.short_label().to_owned(),
            warmup_bars: entry.warmup_bars,
            is_qmv_core: is_qmv_screener_signal(entry.id),
            is_ohlcv_proxy: entry.is_ohlcv_proxy,
        })
        .collect()
}

fn parse_regime_name(name: &str) -> Option<MarketRegime> {
    match name.trim().to_ascii_lowercase().as_str() {
        "uptrend" | "up" => Some(MarketRegime::Uptrend),
        "downtrend" | "dn" => Some(MarketRegime::Downtrend),
        "pullback" | "pb" => Some(MarketRegime::Pullback),
        "recovery" | "rec" => Some(MarketRegime::Recovery),
        "sidewayssqueeze" | "squeeze" | "sqz" => Some(MarketRegime::SidewaysSqueeze),
        "neutral" | "neu" => Some(MarketRegime::Neutral),
        _ => None,
    }
}

fn parse_category_key(key: &str) -> Option<SignalCategory> {
    SignalCategory::ALL_TABS
        .iter()
        .copied()
        .find(|c| c.key().eq_ignore_ascii_case(key.trim()))
        .filter(|c| *c != SignalCategory::All)
}

#[tauri::command]
pub async fn run_stock_screener(
    state: State<'_, AppState>,
    input: StockScreenerInput,
) -> DesktopResult<StockScreenerResponse> {
    let store = state.market_store.clone();
    let max_symbols = input.max_symbols.unwrap_or(500).clamp(10, 1000);
    let ex_upper = input.exchange_filter.trim().to_ascii_uppercase();
    let q_upper = input
        .search_query
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_ascii_uppercase();

    // Gather candidate symbols from explicit watchlist or local SQLite market universe
    // (falling back to the curated Vietnam equity basket on a fresh cold-start DB before CafeF import).
    let store_candidates = store.clone();
    let ex_upper_clone = ex_upper.clone();
    let q_upper_clone = q_upper.clone();
    let explicit_tickers = input.tickers.clone();
    let candidates: Vec<(SymbolKey, String)> = tokio::task::spawn_blocking(move || {
        if ex_upper_clone == "WATCHLIST" {
            Ok::<_, DesktopError>(
                explicit_tickers
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|t| SymbolKey::parse(&t).ok())
                    .map(|k| {
                        let desc = store_candidates
                            .get_symbol(&k)
                            .ok()
                            .flatten()
                            .map_or_else(|| k.symbol.clone(), |(s, _)| s.description);
                        (k, desc)
                    })
                    .collect(),
            )
        } else {
            let (ex_arg, allowed_exchanges): (Option<&str>, Option<&[&str]>) =
                match ex_upper_clone.as_str() {
                    "HOSE" | "HNX" | "UPCOM" => (Some(ex_upper_clone.as_str()), None),
                    "CRYPTO" | "BINANCE" => (
                        None,
                        Some(&["BINANCE", "OKX", "BYBIT", "COINBASE", "KRAKEN"][..]),
                    ),
                    "US" | "NASDAQ" | "NYSE" => (None, Some(&["NASDAQ", "NYSE", "AMEX"][..])),
                    // QMV Screener default ("ALL", "ALL_VN", "QMV30", "QMV60", "DT1..3"): strictly Vietnam equities!
                    _ => (None, Some(&["HOSE", "HNX", "UPCOM"][..])),
                };
            let sql_limit = 5000;
            let hits =
                store_candidates.search_local_symbols(&q_upper_clone, ex_arg, sql_limit)?;
            let mut local: Vec<(SymbolKey, String)> = hits
                .into_iter()
                .filter(|h| {
                    if h.market_type == "index" || h.market_type == "futures" || h.symbol.ends_with("F1!") {
                        return false;
                    }
                    if let Some(allowed) = allowed_exchanges {
                        allowed.contains(&h.exchange.as_str())
                    } else if let Some(ex) = ex_arg {
                        h.exchange == ex
                    } else {
                        matches!(h.exchange.as_str(), "HOSE" | "HNX" | "UPCOM")
                    }
                })
                .filter_map(|h| SymbolKey::parse(&h.ticker).ok().map(|k| (k, h.description)))
                .collect();

            if local.is_empty() {
                local = curated_catalog()
                    .into_iter()
                    .filter(|h| {
                        if h.market_type == "index" || h.market_type == "futures" {
                            return false;
                        }
                        let matches_ex = if let Some(allowed) = allowed_exchanges {
                            allowed.contains(&h.exchange.as_str())
                        } else if let Some(ex) = ex_arg {
                            h.exchange == ex
                        } else {
                            matches!(h.exchange.as_str(), "HOSE" | "HNX" | "UPCOM")
                        };
                        let matches_q = q_upper_clone.is_empty()
                            || h.symbol.contains(&q_upper_clone)
                            || h.ticker.contains(&q_upper_clone)
                            || h.description.to_ascii_uppercase().contains(&q_upper_clone);
                        matches_ex && matches_q
                    })
                    .filter_map(|h| SymbolKey::parse(&h.ticker).ok().map(|k| (k, h.description)))
                    .collect();
            }
            Ok(local)
        }
    })
    .await
    .map_err(|e| DesktopError::Internal(e.to_string()))??;

    let now_ms = chrono::Utc::now().timestamp_millis();
    let (expected_session_date, expected_session_ms) =
        sumi_data::latest_expected_vn_trading_session_ms(now_ms);

    // Fix Stale EOD Trap (v3.0 Wave 1):
    // 1. On a cold-start database (<15 cached series) or Watchlist filter, hydrate missing symbols online.
    // 2. When `auto_refresh_stale == Some(true)` (explicit user click), refresh up to 60 stale candidate
    //    symbols whose cached `1D` latest_ms < `expected_session_ms` without ever stalling a 475-symbol scan.
    if !state.is_offline() && !candidates.is_empty() {
        let store_check = store.clone();
        let candidates_check = candidates.clone();
        let force_hydrate = ex_upper == "WATCHLIST";
        let want_refresh_stale = input.auto_refresh_stale.unwrap_or(false);
        let (cached_count, uncached, stale_list): (usize, Vec<SymbolKey>, Vec<SymbolKey>) =
            tokio::task::spawn_blocking(move || {
                let mut cached = 0usize;
                let mut missing = Vec::new();
                let mut stale = Vec::new();
                for (k, _) in candidates_check {
                    let series = SeriesKey::new(&k, Interval::D1, Adjustment::Splits);
                    let meta = store_check.series_meta(&series).ok().flatten();
                    let has_enough = meta.as_ref().is_some_and(|m| m.bar_count >= 55);
                    if has_enough {
                        cached += 1;
                        if want_refresh_stale
                            && stale.len() < 60
                            && meta.is_some_and(|m| m.latest_ms < expected_session_ms)
                        {
                            stale.push(k);
                        }
                    } else if missing.len() < 18 {
                        missing.push(k);
                    }
                }
                (cached, missing, stale)
            })
            .await
            .unwrap_or_default();

        if !uncached.is_empty() && (force_hydrate || cached_count < 15) {
            let svc = state.active_market_service();
            for key in uncached {
                let series = SeriesKey::new(&key, Interval::D1, Adjustment::Splits);
                let _ = svc.get_bars(&BarsRequest::init(series, 220)).await;
            }
        }
        if want_refresh_stale && !stale_list.is_empty() {
            let svc = state.active_market_service();
            let _ = svc
                .sync_batch(&stale_list, Interval::D1, Adjustment::Splits, 120)
                .await;
        }
    }

    tokio::task::spawn_blocking(move || {
        let started = std::time::Instant::now();
        let regime_target = input.regime_filter.as_deref().and_then(parse_regime_name);
        let category_target = input
            .category_filter
            .as_deref()
            .and_then(parse_category_key);

        let mut required_signal_indices: Vec<usize> = Vec::new();
        if let Some(c_idx) = input
            .signal_id
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .and_then(catalog_entry_index)
        {
            required_signal_indices.push(c_idx);
        }
        for sid in &input.signal_ids {
            if let Some(c_idx) = catalog_entry_index(sid.trim())
                && !required_signal_indices.contains(&c_idx)
            {
                required_signal_indices.push(c_idx);
            }
        }

        let basket_target = input
            .qmv_basket
            .as_deref()
            .map(|s| s.trim().to_ascii_uppercase())
            .filter(|s| !s.is_empty() && s != "ALL")
            .or_else(|| {
                matches!(ex_upper.as_str(), "QMV30" | "QMV60" | "CORE10").then(|| ex_upper.clone())
            });
        let sector_group_target = input
            .sector_group_filter
            .as_deref()
            .and_then(QmvSectorGroup::parse_str);
        let sector_target = input
            .sector_filter
            .as_deref()
            .and_then(QmvSector::parse_str);
        let money_tier_target = input
            .money_tier_filter
            .as_deref()
            .and_then(QmvMoneyTier::parse_str)
            .or_else(|| QmvMoneyTier::parse_str(&ex_upper).filter(|t| *t != QmvMoneyTier::IndexOrOther));
        let price_tier_target = input
            .price_tier_filter
            .as_deref()
            .and_then(QmvPriceTier::parse_str);
        let min_avg_vol_20 = input.min_avg_vol_20.filter(|v| v.is_finite() && *v > 0.0);

        // Phase 1: Batch-load candidate OHLCV series sequentially from SQLite (<25ms for 475 symbols).
        let mut loaded_series: Vec<(SymbolKey, String, Vec<Candle>)> = Vec::new();
        for (sym_key, description) in candidates.into_iter().take(max_symbols) {
            if !q_upper.is_empty()
                && !sym_key.symbol.contains(&q_upper)
                && !sym_key.ticker().contains(&q_upper)
                && !description.to_ascii_uppercase().contains(&q_upper)
            {
                continue;
            }
            // Fast QMV static taxonomy pre-filter BEFORE touching SQLite bars!
            let prof_opt = lookup_qmv_profile(&sym_key.symbol);
            if let Some(ref b) = basket_target {
                let ok = match b.as_str() {
                    "QMV30" => prof_opt.is_some_and(|p| p.in_qmv30),
                    "QMV60" => prof_opt.is_some_and(|p| p.in_qmv60),
                    "CORE10" => prof_opt.is_some_and(|p| p.in_yumeko_core10),
                    _ => true,
                };
                if !ok {
                    continue;
                }
            }
            if let Some(sg) = sector_group_target
                && prof_opt.is_none_or(|p| p.sector_group != sg)
            {
                continue;
            }
            if let Some(sec) = sector_target
                && prof_opt.is_none_or(|p| p.sector != sec)
            {
                continue;
            }
            if let Some(mt) = money_tier_target
                && prof_opt.is_none_or(|p| p.money_tier != mt)
            {
                continue;
            }

            let series = SeriesKey::new(&sym_key, Interval::D1, Adjustment::Splits);
            let Ok(res) = store.query_bars(&series, BarQueryMode::Init, 300) else {
                continue;
            };
            if res.bars.len() < 55 {
                continue;
            }
            let candles: Vec<Candle> = res
                .bars
                .iter()
                .map(|b| b.to_core_candle(Interval::D1))
                .collect();
            loaded_series.push((sym_key, description, candles));
        }

        let scanned_symbols = loaded_series.len();
        let mut latest_ts_list: Vec<i64> = loaded_series
            .iter()
            .filter_map(|(_, _, cs)| cs.last().map(|c| c.timestamp.saturating_mul(1_000)))
            .collect();
        latest_ts_list.sort_unstable();
        let fresh_symbols_count = latest_ts_list
            .iter()
            .filter(|&&ts| ts >= expected_session_ms)
            .count();
        let stale_symbols_count = scanned_symbols.saturating_sub(fresh_symbols_count);
        let median_cached_date = latest_ts_list
            .get(latest_ts_list.len() / 2)
            .map(|&ts| sumi_data::format_ymd_utc(ts));

        let min_rsi = input.min_rsi.filter(|v| v.is_finite());
        let max_rsi = input.max_rsi.filter(|v| v.is_finite());
        let min_rvol = input.min_rvol.filter(|v| v.is_finite());
        let min_change_pct = input.min_change_pct.filter(|v| v.is_finite());
        let max_change_pct = input.max_change_pct.filter(|v| v.is_finite());
        let min_health_score = input.min_health_score.filter(|v| v.is_finite());
        let macd_sign: Option<i8> = match input
            .macd_filter
            .as_deref()
            .map(|s| s.trim().to_ascii_lowercase())
            .as_deref()
        {
            Some("positive" | "bull" | ">0") => Some(1),
            Some("negative" | "bear" | "<0") => Some(-1),
            _ => None,
        };

        // Phase 2: Parallel multi-core evaluation of all catalog signals across CPU cores (`BUG-023` speedup!).
        let num_workers = std::thread::available_parallelism()
            .map(std::num::NonZeroUsize::get)
            .unwrap_or(4)
            .min(scanned_symbols.max(1));
        let chunk_size = scanned_symbols.div_ceil(num_workers).max(1);
        let profile = ConfluenceWeightProfile::Balanced;
        let req_indices_ref = &required_signal_indices;

        let mut rows: Vec<ScreenerRowDto> = std::thread::scope(|s| {
            let mut handles = Vec::new();
            for chunk in loaded_series.chunks(chunk_size) {
                handles.push(s.spawn(move || {
                    let mut local_rows = Vec::new();
                    for (sym_key, description, candles) in chunk {
                        let last_idx = candles.len() - 1;
                        let ticker_str = sym_key.ticker();
                        let Some(scan_row) = evaluate_symbol_scan(
                            &ticker_str,
                            &sym_key.symbol,
                            &sym_key.exchange,
                            Timeframe::D1,
                            candles,
                            None,
                            5,
                        ) else {
                            continue;
                        };

                        let p_tier = QmvPriceTier::from_close_price(scan_row.last_close);
                        if let Some(target_pt) = price_tier_target
                            && p_tier != target_pt
                        {
                            continue;
                        }

                        // Compute causal 20-session average volume [last_idx-20 .. last_idx)
                        let avg_vol_20: Option<f64> = if last_idx >= 20 {
                            let v_sum: f64 = candles[last_idx - 20..last_idx]
                                .iter()
                                .map(|b| if b.volume.is_finite() { b.volume.max(0.0) } else { 0.0 })
                                .sum();
                            Some(v_sum / 20.0)
                        } else {
                            None
                        };
                        if let Some(min_v20) = min_avg_vol_20
                            && avg_vol_20.is_none_or(|v| v < min_v20)
                        {
                            continue;
                        }

                        if let Some(target_reg) = regime_target
                            && scan_row.regime != target_reg
                        {
                            continue;
                        }

                        // Multi-condition numeric filters (v2.2) — fast early-out before catalog matrix!
                        if let Some(lo) = min_rsi
                            && scan_row.rsi_14.is_none_or(|v| v < lo)
                        {
                            continue;
                        }
                        if let Some(hi) = max_rsi
                            && scan_row.rsi_14.is_none_or(|v| v > hi)
                        {
                            continue;
                        }
                        if let Some(lo) = min_rvol
                            && scan_row.rvol_20.is_none_or(|v| v < lo)
                        {
                            continue;
                        }
                        if let Some(lo) = min_change_pct
                            && scan_row.change_pct < lo
                        {
                            continue;
                        }
                        if let Some(hi) = max_change_pct
                            && scan_row.change_pct > hi
                        {
                            continue;
                        }
                        if let Some(sign) = macd_sign {
                            let ok = scan_row
                                .macd_hist
                                .is_some_and(|h| if sign > 0 { h > 0.0 } else { h < 0.0 });
                            if !ok {
                                continue;
                            }
                        }

                        let matrix = PrecomputedCatalogMatrix::compute(candles);
                        let health_score = matrix
                            .health_score_at(last_idx, profile)
                            .ok()
                            .filter(|v| v.is_finite());
                        if let Some(lo) = min_health_score
                            && health_score.is_none_or(|v| v < lo)
                        {
                            continue;
                        }
                        let (bullish_signals, bearish_signals) =
                            matrix.counts_at(last_idx, profile);

                        // Multi-signal `AND` intersection across all selected QMV Screener signals!
                        let mut selected_signal_value: Option<String> = None;
                        let mut all_req_matched = true;
                        for (pos, &c_idx) in req_indices_ref.iter().enumerate() {
                            let entry = &SIGNAL_CATALOG[c_idx];
                            let pt = matrix.point_at_with_profile(c_idx, last_idx, profile);
                            let matched = match pt {
                                Some(p) if p.quality.is_valid() => match entry.output_type {
                                    SignalOutputType::Bool => {
                                        if p.bool_value == Some(true) {
                                            if pos == 0 {
                                                selected_signal_value = Some(
                                                    p.direction
                                                        .map_or("ACTIVE", SignalDirection::short_label)
                                                        .to_owned(),
                                                );
                                            }
                                            true
                                        } else {
                                            false
                                        }
                                    }
                                    SignalOutputType::Float => {
                                        if let Some(fv) = p.float_value.filter(|v| v.is_finite()) {
                                            if pos == 0 {
                                                selected_signal_value = Some(format!("{fv:.1}"));
                                            }
                                            true
                                        } else {
                                            false
                                        }
                                    }
                                    SignalOutputType::Enum => {
                                        if pos == 0 {
                                            selected_signal_value = Some(
                                                matrix.regime_at(last_idx).full_label().to_owned(),
                                            );
                                        }
                                        true
                                    }
                                },
                                _ => false,
                            };
                            if !matched {
                                all_req_matched = false;
                                break;
                            }
                        }
                        if !all_req_matched {
                            continue;
                        }

                        let mut matched_signals: Vec<String> = Vec::new();
                        // Prioritize QMV core signals in the matched badge strip!
                        for (idx, entry) in SIGNAL_CATALOG.iter().enumerate().rev() {
                            if !is_qmv_screener_signal(entry.id) {
                                continue;
                            }
                            if let Some(cat) = category_target
                                && entry.category != cat
                            {
                                continue;
                            }
                            if entry.output_type != SignalOutputType::Bool {
                                continue;
                            }
                            if let Some(pt) = matrix.point_at_with_profile(idx, last_idx, profile)
                                && pt.quality.is_valid()
                                && pt.bool_value == Some(true)
                            {
                                matched_signals.push(entry.label_vi.to_owned());
                                if matched_signals.len() >= 6 {
                                    break;
                                }
                            }
                        }

                        if category_target.is_some()
                            && req_indices_ref.is_empty()
                            && matched_signals.is_empty()
                        {
                            continue;
                        }

                        let prof = lookup_qmv_profile(&sym_key.symbol);
                        let pbb = sumi_core::scanner::compute_proxy_bb_series(candles);
                        let qmv_flow_z50_t05 = pbb.z50_at(last_idx, BBHorizon::T05);
                        let qmv_flow_z50_t20 = pbb.z50_at(last_idx, BBHorizon::T20);

                        local_rows.push(ScreenerRowDto {
                            ticker: ticker_str,
                            exchange: sym_key.exchange.clone(),
                            symbol: sym_key.symbol.clone(),
                            description: description.clone(),
                            last_close: scan_row.last_close,
                            change_pct: scan_row.change_pct,
                            last_volume: scan_row.last_volume,
                            avg_vol_20,
                            rvol_20: scan_row.rvol_20,
                            rsi_14: scan_row.rsi_14,
                            macd_hist: scan_row.macd_hist,
                            atr_pct: scan_row.atr_pct,
                            regime: scan_row.regime.full_label().to_owned(),
                            regime_short: scan_row.regime.short_label().to_owned(),
                            health_score,
                            bullish_signals,
                            bearish_signals,
                            matched_signals,
                            selected_signal_value,
                            last_timestamp_ms: scan_row.last_timestamp.saturating_mul(1_000),
                            qmv_sector_group: prof
                                .map_or("GROUP_2", |p| p.sector_group.code())
                                .to_owned(),
                            qmv_sector: prof
                                .map_or("OTHER_CYCLICAL", |p| p.sector.code())
                                .to_owned(),
                            qmv_sector_label: prof
                                .map_or("Khác", |p| p.sector.label_vi())
                                .to_owned(),
                            money_tier: prof.map_or("DT3", |p| p.money_tier.code()).to_owned(),
                            in_qmv30: prof.is_some_and(|p| p.in_qmv30),
                            in_qmv60: prof.is_some_and(|p| p.in_qmv60),
                            price_tier: p_tier.code().to_owned(),
                            qmv_flow_z50_t05,
                            qmv_flow_z50_t20,
                        });
                    }
                    local_rows
                }));
            }
            let mut all = Vec::new();
            for h in handles {
                if let Ok(part) = h.join() {
                    all.extend(part);
                }
            }
            all
        });

        // Default sort: highest Technical Health score descending, then change %.
        rows.sort_by(|a, b| {
            let ha = a.health_score.unwrap_or(-999.0);
            let hb = b.health_score.unwrap_or(-999.0);
            hb.total_cmp(&ha)
                .then_with(|| b.change_pct.total_cmp(&a.change_pct))
        });

        let matched_symbols = rows.len();
        Ok(StockScreenerResponse {
            total_catalog_signals: SIGNAL_CATALOG.len(),
            scanned_symbols,
            matched_symbols,
            elapsed_ms: started.elapsed().as_millis() as u64,
            expected_session_date,
            fresh_symbols_count,
            stale_symbols_count,
            median_cached_date,
            rows,
        })
    })
    .await
    .map_err(|e| DesktopError::Internal(e.to_string()))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signal_catalog_exposes_all_123_signals_and_qmv_core_filter() {
        let items = get_signal_catalog();
        assert_eq!(items.len(), 123);
        assert!(items.iter().any(|i| !i.label_vi.is_empty()));
        assert!(items.iter().any(|i| i.id == "qmv.ichimoku_v7_bullish" && i.is_qmv_core));
        assert!(items.iter().any(|i| i.id == "technical.psar_flip_bull" && !i.is_qmv_core));
    }
}
