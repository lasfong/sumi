use std::collections::HashMap;

use chrono::{Datelike, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sumi_core::data::Timeframe;
use sumi_core::model::Candle;
use sumi_core::scanner::{
    BBHorizon, ConfluenceWeightProfile, MarketRegime, PrecomputedCatalogMatrix, QmvMoneyTier,
    QmvPriceTier, QmvSector, QmvSectorGroup, SIGNAL_CATALOG, SignalCategory, SignalDirection,
    SignalOutputType, catalog_entry_index, evaluate_symbol_scan, is_qmv_screener_signal,
    lookup_qmv_profile,
};
use sumi_data::{Adjustment, BarQueryMode, BarsRequest, Interval, SeriesKey, SymbolKey};
use sumi_yumeko_bridge::YumekoReadOnlyBridge;
use tauri::State;

use crate::commands::market::curated_catalog;
use crate::error::{DesktopError, DesktopResult};
use crate::state::AppState;
use crate::workspace_db::SavedScreenRecord;

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
    // === Wave 4: Multi-Timeframe (1W/1D/1H), Fundamental (DN01/TV) & Yumeko QMV Blackbox Filters ===
    /// Optional weekly (`1W` resampled from `1D`) regime filter (`"Uptrend"`, `"Pullback"`, `"Recovery"`, ...)
    #[serde(default)]
    pub weekly_regime_filter: Option<String>,
    /// Stage-2 funnel: evaluate `1H` timing on top `<= 25` Stage-1 survivors (`true`/`false`)
    #[serde(default)]
    pub enable_mtf_1h: Option<bool>,
    #[serde(default)]
    pub max_pe: Option<f64>,
    #[serde(default)]
    pub max_pb: Option<f64>,
    #[serde(default)]
    pub min_roe: Option<f64>,
    #[serde(default)]
    pub min_market_cap_bn: Option<f64>,
    #[serde(default)]
    pub only_money_in_today: Option<bool>,
    #[serde(default)]
    pub min_qmv_tmc: Option<f64>,
    #[serde(default)]
    pub min_qmv_dspi: Option<f64>,
    #[serde(default)]
    pub only_qmv_reversal_t_plus: Option<bool>,
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
    // Wave 4 MTF + Fundamentals + Yumeko QMV Real columns
    pub weekly_regime: String,
    pub weekly_regime_short: String,
    pub hourly_timing: Option<String>,
    pub mtf_aligned: bool,
    pub pe: Option<f64>,
    pub pb: Option<f64>,
    pub roe: Option<f64>,
    pub market_cap_bn: Option<f64>,
    pub qmv_real_tmc: Option<f64>,
    pub qmv_real_dspi: Option<f64>,
    pub qmv_real_money_in_today: bool,
    pub qmv_real_provenance: bool,
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

/// Strictly causal `1D -> 1W` resampling grouped by ISO week in Vietnam time (`UTC+7`).
fn resample_daily_to_weekly(daily: &[Candle]) -> Vec<Candle> {
    let mut weekly: Vec<Candle> = Vec::new();
    let mut current_week: Option<(i32, u32)> = None;
    for c in daily {
        let ict_secs = c.timestamp.saturating_add(7 * 3600);
        let Some(dt) = Utc.timestamp_opt(ict_secs, 0).single() else {
            continue;
        };
        let iso = dt.iso_week();
        let wk = (iso.year(), iso.week());
        if current_week == Some(wk)
            && let Some(last) = weekly.last_mut()
        {
            if c.high > last.high {
                last.high = c.high;
            }
            if c.low < last.low {
                last.low = c.low;
            }
            last.close = c.close;
            last.volume += c.volume.max(0.0);
        } else {
            current_week = Some(wk);
            weekly.push(Candle {
                timestamp: c.timestamp,
                open: c.open,
                high: c.high,
                low: c.low,
                close: c.close,
                volume: c.volume.max(0.0),
                date_label: c.date_label.clone(),
            });
        }
    }
    weekly
}

fn is_bullish_or_pullback_regime(r: MarketRegime) -> bool {
    matches!(
        r,
        MarketRegime::Uptrend | MarketRegime::Pullback | MarketRegime::Recovery
    )
}

fn classify_hourly_timing(h1_candles: &[Candle]) -> Option<String> {
    if h1_candles.len() < 30 {
        return None;
    }
    let last = h1_candles.last()?;
    let n = h1_candles.len();
    let sma20: f64 = h1_candles[n - 20..].iter().map(|c| c.close).sum::<f64>() / 20.0;
    let sma10: f64 = h1_candles[n - 10..].iter().map(|c| c.close).sum::<f64>() / 10.0;
    let prev_close = h1_candles[n - 2].close;
    if last.close >= sma20 && sma10 >= sma20 {
        Some("BULL_1H".to_owned())
    } else if last.close >= sma20 * 0.985 && last.close >= prev_close {
        Some("PULLBACK_1H".to_owned())
    } else {
        Some("BEAR_1H".to_owned())
    }
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

    let enable_mtf_1h = input.enable_mtf_1h.unwrap_or(false);
    let store_stage1 = store.clone();
    let mut response = tokio::task::spawn_blocking(move || {
        let started = std::time::Instant::now();
        let regime_target = input.regime_filter.as_deref().and_then(parse_regime_name);
        let weekly_regime_target = input
            .weekly_regime_filter
            .as_deref()
            .and_then(parse_regime_name);
        let category_target = input
            .category_filter
            .as_deref()
            .and_then(parse_category_key);

        // Bulk-load cached fundamentals (`sumi_market.db`) + read-only Yumeko DN01 & QMV Blackbox maps once!
        let local_funda_map = store_stage1.list_fundamentals_map().unwrap_or_default();
        let bridge = YumekoReadOnlyBridge::discover_default();
        let yumeko_val_map = bridge.read_latest_valuation_map(None).unwrap_or_default();
        let yumeko_qmv_map = bridge.read_latest_qmv_screener_map(None).unwrap_or_default();

        let max_pe = input.max_pe.filter(|v| v.is_finite() && *v > 0.0);
        let max_pb = input.max_pb.filter(|v| v.is_finite() && *v > 0.0);
        let min_roe = input.min_roe.filter(|v| v.is_finite());
        let min_mcap_bn = input.min_market_cap_bn.filter(|v| v.is_finite() && *v > 0.0);
        let only_money_in_today = input.only_money_in_today.unwrap_or(false);
        let min_qmv_tmc = input.min_qmv_tmc.filter(|v| v.is_finite());
        let min_qmv_dspi = input.min_qmv_dspi.filter(|v| v.is_finite());
        let only_qmv_rev_t_plus = input.only_qmv_reversal_t_plus.unwrap_or(false);

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
            let Ok(res) = store_stage1.query_bars(&series, BarQueryMode::Init, 300) else {
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
        let local_funda_ref = &local_funda_map;
        let yumeko_val_ref = &yumeko_val_map;
        let yumeko_qmv_ref = &yumeko_qmv_map;

        let mut rows: Vec<ScreenerRowDto> = std::thread::scope(|s| {
            let mut handles = Vec::new();
            for chunk in loaded_series.chunks(chunk_size) {
                handles.push(s.spawn(move || {
                    let mut local_rows = Vec::new();
                    for (sym_key, description, candles) in chunk {
                        let last_idx = candles.len() - 1;
                        let ticker_str = sym_key.ticker();
                        let sym_up = sym_key.symbol.to_ascii_uppercase();

                        // Fundamental & Yumeko QMV Blackbox early filters before heavy catalog scan!
                        let y_val = yumeko_val_ref.get(&sym_up);
                        let l_funda = local_funda_ref.get(&sym_up);
                        let y_qmv = yumeko_qmv_ref.get(&sym_up);

                        let pe = y_val
                            .and_then(|v| v.pe)
                            .or_else(|| l_funda.and_then(|f| f.pe_ttm))
                            .filter(|v| v.is_finite() && *v > 0.0);
                        let pb = y_val
                            .and_then(|v| v.pb)
                            .or_else(|| l_funda.and_then(|f| f.pb))
                            .filter(|v| v.is_finite() && *v > 0.0);
                        let roe = y_val
                            .and_then(|v| v.roe)
                            .or_else(|| l_funda.and_then(|f| f.roe))
                            .filter(|v| v.is_finite());
                        let market_cap_bn = l_funda
                            .and_then(|f| f.market_cap_basic)
                            .filter(|v| v.is_finite() && *v > 0.0)
                            .map(|v| v / 1_000_000_000.0);

                        if let Some(cap) = max_pe
                            && pe.is_none_or(|v| v > cap)
                        {
                            continue;
                        }
                        if let Some(cap) = max_pb
                            && pb.is_none_or(|v| v > cap)
                        {
                            continue;
                        }
                        if let Some(lo) = min_roe
                            && roe.is_none_or(|v| v < lo)
                        {
                            continue;
                        }
                        if let Some(lo) = min_mcap_bn
                            && market_cap_bn.is_none_or(|v| v < lo)
                        {
                            continue;
                        }
                        if only_money_in_today && !y_qmv.is_some_and(|q| q.money_in_today) {
                            continue;
                        }
                        if only_qmv_rev_t_plus && !y_qmv.is_some_and(|q| q.reversal_up_t_plus) {
                            continue;
                        }
                        if let Some(lo) = min_qmv_tmc
                            && y_qmv.and_then(|q| q.tmc).is_none_or(|v| v < lo)
                        {
                            continue;
                        }
                        if let Some(lo) = min_qmv_dspi
                            && y_qmv.and_then(|q| q.dspi).is_none_or(|v| v < lo)
                        {
                            continue;
                        }

                        // Weekly (`1W`) causal resampled regime check
                        let weekly_candles = resample_daily_to_weekly(candles);
                        let weekly_reg = if weekly_candles.len() >= 26 {
                            evaluate_symbol_scan(
                                &ticker_str,
                                &sym_key.symbol,
                                &sym_key.exchange,
                                Timeframe::W1,
                                &weekly_candles,
                                None,
                                5,
                            )
                            .map_or(MarketRegime::Neutral, |r| r.regime)
                        } else {
                            MarketRegime::Neutral
                        };
                        if let Some(target_w) = weekly_regime_target
                            && weekly_reg != target_w
                        {
                            continue;
                        }

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

                        let mtf_w_d = is_bullish_or_pullback_regime(weekly_reg)
                            && is_bullish_or_pullback_regime(scan_row.regime);

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
                            weekly_regime: weekly_reg.full_label().to_owned(),
                            weekly_regime_short: weekly_reg.short_label().to_owned(),
                            hourly_timing: None,
                            mtf_aligned: mtf_w_d,
                            pe,
                            pb,
                            roe,
                            market_cap_bn,
                            qmv_real_tmc: y_qmv.and_then(|q| q.tmc),
                            qmv_real_dspi: y_qmv.and_then(|q| q.dspi),
                            qmv_real_money_in_today: y_qmv.is_some_and(|q| q.money_in_today),
                            qmv_real_provenance: y_qmv.is_some(),
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
        Ok::<_, DesktopError>(StockScreenerResponse {
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
    .map_err(|e| DesktopError::Internal(e.to_string()))??;

    // Stage-2 MTF Funnel (Wave 4): evaluate `1H` timing on top <= 25 Stage-1 survivors!
    if enable_mtf_1h && !response.rows.is_empty() {
        let top_n = response.rows.len().min(25);
        let top_keys: Vec<SymbolKey> = response.rows[..top_n]
            .iter()
            .filter_map(|r| SymbolKey::parse(&r.ticker).ok())
            .collect();

        // Hydrate up to 12 missing `1H` series online if online
        if !state.is_offline() {
            let store_h1_chk = store.clone();
            let keys_chk = top_keys.clone();
            let missing_h1: Vec<SymbolKey> = tokio::task::spawn_blocking(move || {
                keys_chk
                    .into_iter()
                    .filter(|k| {
                        let sk = SeriesKey::new(k, Interval::H1, Adjustment::Splits);
                        store_h1_chk
                            .series_meta(&sk)
                            .ok()
                            .flatten()
                            .is_none_or(|m| m.bar_count < 30)
                    })
                    .take(12)
                    .collect()
            })
            .await
            .unwrap_or_default();

            if !missing_h1.is_empty() {
                let svc = state.active_market_service();
                for key in missing_h1 {
                    let sk = SeriesKey::new(&key, Interval::H1, Adjustment::Splits);
                    let _ = svc.get_bars(&BarsRequest::init(sk, 120)).await;
                }
            }
        }

        let store_h1 = store.clone();
        let timing_map: HashMap<String, String> = tokio::task::spawn_blocking(move || {
            let mut out = HashMap::new();
            for key in top_keys {
                let sk = SeriesKey::new(&key, Interval::H1, Adjustment::Splits);
                if let Ok(res) = store_h1.query_bars(&sk, BarQueryMode::Init, 120)
                    && res.bars.len() >= 30
                {
                    let h1_candles: Vec<Candle> = res
                        .bars
                        .iter()
                        .map(|b| b.to_core_candle(Interval::H1))
                        .collect();
                    if let Some(label) = classify_hourly_timing(&h1_candles) {
                        out.insert(key.ticker(), label);
                    }
                }
            }
            out
        })
        .await
        .unwrap_or_default();

        for row in response.rows.iter_mut().take(top_n) {
            if let Some(h1_label) = timing_map.get(&row.ticker) {
                row.hourly_timing = Some(h1_label.clone());
                let h1_ok = h1_label == "BULL_1H" || h1_label == "PULLBACK_1H";
                row.mtf_aligned = row.mtf_aligned && h1_ok;
            }
        }
    }

    Ok(response)
}

#[tauri::command]
pub fn list_saved_screens(state: State<'_, AppState>) -> DesktopResult<Vec<SavedScreenRecord>> {
    state.workspace.list_saved_screens()
}

#[tauri::command]
pub fn save_screen(
    state: State<'_, AppState>,
    id: Option<String>,
    name: String,
    filter_json: Value,
) -> DesktopResult<SavedScreenRecord> {
    let clean_name = name.trim();
    if clean_name.is_empty() {
        return Err(DesktopError::Validation(
            "Tên bộ lọc lưu trữ không được để trống.".to_owned(),
        ));
    }
    let rec_id = id
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| format!("scr_{}", uuid::Uuid::new_v4().simple()));
    state.workspace.save_screen(&rec_id, clean_name, &filter_json)
}

#[tauri::command]
pub fn delete_screen(state: State<'_, AppState>, id: String) -> DesktopResult<()> {
    state.workspace.delete_screen(id.trim())
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

    #[test]
    fn resample_daily_to_weekly_groups_iso_weeks_causally() {
        // 10 consecutive weekdays (Mon..Fri x 2 weeks starting 2026-01-05)
        let mut daily = Vec::new();
        let base_ts = chrono::NaiveDate::from_ymd_opt(2026, 1, 5)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp();
        for w in 0..2 {
            for d in 0..5 {
                let ts = base_ts + (w * 7 + d) * 86_400;
                let base_p = 100_000.0 + (w * 10 + d) as f64 * 1_000.0;
                daily.push(Candle {
                    timestamp: ts,
                    open: base_p,
                    high: base_p + 2_000.0,
                    low: base_p - 1_000.0,
                    close: base_p + 500.0,
                    volume: 100_000.0,
                    date_label: String::new(),
                });
            }
        }
        let weekly = resample_daily_to_weekly(&daily);
        assert_eq!(weekly.len(), 2);
        assert_eq!(weekly[0].open, daily[0].open);
        assert_eq!(weekly[0].close, daily[4].close);
        assert_eq!(weekly[0].high, daily[4].high);
        assert_eq!(weekly[0].low, daily[0].low);
        assert!((weekly[0].volume - 500_000.0).abs() < 1e-6);
        assert_eq!(weekly[1].open, daily[5].open);
        assert_eq!(weekly[1].close, daily[9].close);
    }
}
