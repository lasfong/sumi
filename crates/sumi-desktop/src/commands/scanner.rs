use serde::{Deserialize, Serialize};
use sumi_core::data::Timeframe;
use sumi_core::model::Candle;
use sumi_core::scanner::{
    ConfluenceWeightProfile, MarketRegime, PrecomputedCatalogMatrix, SIGNAL_CATALOG,
    SignalCategory, SignalDirection, SignalOutputType, catalog_entry_index, evaluate_symbol_scan,
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
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StockScreenerInput {
    /// `"ALL"` | `"HOSE"` | `"HNX"` | `"UPCOM"` | `"WATCHLIST"`
    pub exchange_filter: String,
    /// Optional explicit tickers (used when `exchange_filter == "WATCHLIST"`).
    pub tickers: Option<Vec<String>>,
    /// Optional filter by a specific `SIGNAL_CATALOG` entry id (from the 101 signals!).
    pub signal_id: Option<String>,
    /// Optional signal category filter (`"all"`, `"health"`, `"flow"`, `"divergence"`, ...).
    pub category_filter: Option<String>,
    /// Optional regime filter (`"Uptrend"`, `"Downtrend"`, `"Pullback"`, `"Recovery"`, `"SidewaysSqueeze"`, `"Neutral"`).
    pub regime_filter: Option<String>,
    /// Optional text query matching ticker or company name.
    pub search_query: Option<String>,
    /// Limit maximum scanned symbols (default 180 for instant < 200ms UI response).
    pub max_symbols: Option<usize>,
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
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StockScreenerResponse {
    pub total_catalog_signals: usize,
    pub scanned_symbols: usize,
    pub matched_symbols: usize,
    pub elapsed_ms: u64,
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
    let max_symbols = input.max_symbols.unwrap_or(180).clamp(10, 500);
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
            let ex_arg = match ex_upper_clone.as_str() {
                "HOSE" | "HNX" | "UPCOM" => Some(ex_upper_clone.as_str()),
                _ => None,
            };
            let hits =
                store_candidates.search_local_symbols(&q_upper_clone, ex_arg, max_symbols)?;
            let mut local: Vec<(SymbolKey, String)> = hits
                .into_iter()
                .filter(|h| h.market_type != "index")
                .filter_map(|h| SymbolKey::parse(&h.ticker).ok().map(|k| (k, h.description)))
                .collect();

            if local.is_empty() {
                local = curated_catalog()
                    .into_iter()
                    .filter(|h| {
                        h.market_type == "stock"
                            && matches!(h.exchange.as_str(), "HOSE" | "HNX" | "UPCOM")
                            && ex_arg.is_none_or(|ex| h.exchange == ex)
                    })
                    .filter_map(|h| SymbolKey::parse(&h.ticker).ok().map(|k| (k, h.description)))
                    .collect();
            }
            Ok(local)
        }
    })
    .await
    .map_err(|e| DesktopError::Internal(e.to_string()))??;

    // On a cold-start database (or uncached watchlist items), hydrate up to 18 candidate daily series
    // online so the 101-signal Screener works immediately out-of-the-box before CafeF CSV import.
    if !state.is_offline() && !candidates.is_empty() {
        let store_check = store.clone();
        let candidates_check = candidates.clone();
        let uncached: Vec<SymbolKey> = tokio::task::spawn_blocking(move || {
            candidates_check
                .into_iter()
                .map(|(k, _)| k)
                .filter(|k| {
                    let series = SeriesKey::new(k, Interval::D1, Adjustment::Splits);
                    store_check
                        .query_bars(&series, BarQueryMode::Init, 55)
                        .map(|r| r.bars.len() < 55)
                        .unwrap_or(true)
                })
                .take(18)
                .collect()
        })
        .await
        .unwrap_or_default();

        if !uncached.is_empty() {
            let svc = state.active_market_service();
            for key in uncached {
                let series = SeriesKey::new(&key, Interval::D1, Adjustment::Splits);
                let _ = svc.get_bars(&BarsRequest::init(series, 220)).await;
            }
        }
    }

    tokio::task::spawn_blocking(move || {
        let started = std::time::Instant::now();
        let regime_target = input.regime_filter.as_deref().and_then(parse_regime_name);
        let category_target = input
            .category_filter
            .as_deref()
            .and_then(parse_category_key);
        let signal_idx = input
            .signal_id
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .and_then(catalog_entry_index);

        let mut scanned_symbols = 0_usize;
        let mut rows: Vec<ScreenerRowDto> = Vec::new();

        for (sym_key, description) in candidates.into_iter().take(max_symbols) {
            if !q_upper.is_empty()
                && !sym_key.symbol.contains(&q_upper)
                && !sym_key.ticker().contains(&q_upper)
                && !description.to_ascii_uppercase().contains(&q_upper)
            {
                continue;
            }
            let series = SeriesKey::new(&sym_key, Interval::D1, Adjustment::Splits);
            let Ok(res) = store.query_bars(&series, BarQueryMode::Init, 220) else {
                continue;
            };
            if res.bars.len() < 55 {
                continue;
            }
            scanned_symbols += 1;

            let candles: Vec<Candle> = res
                .bars
                .iter()
                .map(|b| b.to_core_candle(Interval::D1))
                .collect();
            let last_idx = candles.len() - 1;

            let ticker_str = sym_key.ticker();
            let Some(scan_row) = evaluate_symbol_scan(
                &ticker_str,
                &sym_key.symbol,
                &sym_key.exchange,
                Timeframe::D1,
                &candles,
                None,
                5,
            ) else {
                continue;
            };

            if let Some(target_reg) = regime_target
                && scan_row.regime != target_reg
            {
                continue;
            }

            let matrix = PrecomputedCatalogMatrix::compute(&candles);
            let profile = ConfluenceWeightProfile::Balanced;
            let health_score = matrix
                .health_score_at(last_idx, profile)
                .ok()
                .filter(|v| v.is_finite());
            let (bullish_signals, bearish_signals) = matrix.counts_at(last_idx, profile);

            let mut selected_signal_value: Option<String> = None;
            if let Some(c_idx) = signal_idx {
                let entry = &SIGNAL_CATALOG[c_idx];
                let pt = matrix.point_at_with_profile(c_idx, last_idx, profile);
                let matched = match pt {
                    Some(p) if p.quality.is_valid() => match entry.output_type {
                        SignalOutputType::Bool => {
                            if p.bool_value == Some(true) {
                                selected_signal_value = Some(
                                    p.direction
                                        .map_or("ACTIVE", SignalDirection::short_label)
                                        .to_owned(),
                                );
                                true
                            } else {
                                false
                            }
                        }
                        SignalOutputType::Float => {
                            if let Some(fv) = p.float_value.filter(|v| v.is_finite()) {
                                selected_signal_value = Some(format!("{fv:.2}"));
                                true
                            } else {
                                false
                            }
                        }
                        SignalOutputType::Enum => {
                            selected_signal_value =
                                Some(matrix.regime_at(last_idx).full_label().to_owned());
                            true
                        }
                    },
                    _ => false,
                };
                if !matched {
                    continue;
                }
            }

            // Collect top active boolean signals from all 101 catalog entries (filtered by category if selected).
            let mut matched_signals: Vec<String> = Vec::new();
            for (idx, entry) in SIGNAL_CATALOG.iter().enumerate() {
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

            if category_target.is_some() && signal_idx.is_none() && matched_signals.is_empty() {
                continue;
            }

            rows.push(ScreenerRowDto {
                ticker: sym_key.ticker(),
                exchange: sym_key.exchange,
                symbol: sym_key.symbol,
                description,
                last_close: scan_row.last_close,
                change_pct: scan_row.change_pct,
                last_volume: scan_row.last_volume,
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
            });
        }

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
    fn signal_catalog_exposes_all_101_signals() {
        let items = get_signal_catalog();
        assert_eq!(items.len(), 101);
        assert!(items.iter().any(|i| !i.label_vi.is_empty()));
    }
}
