use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use sumi_core::data::{DataSourcePreset, format_utc_date};
use sumi_core::indicators::{
    AnalogBankConfig, AnalogConeConfig, AnalogConeError, AnalogConeForecast, AnalogMemoryBank,
    AnalogSymbolInput, AnalogWeightProfile, DEFAULT_ANALOG_HORIZON, DEFAULT_ANALOG_TOP_K,
    DEFAULT_MIN_MEDIAN_TURNOVER_VND, MAX_ANALOG_HORIZON,
};
use sumi_core::journal::{ExchangeVenue, VnPriceUnit};
use sumi_core::model::Candle;
use sumi_data::{Adjustment, BarQueryMode, BarsRequest, Interval, OhlcvBar, SeriesKey, SymbolKey};
use tauri::State;
use tracing::info;

use crate::commands::market::{curated_catalog, synthesize_fallback_symbol};
use crate::error::{DesktopError, DesktopResult};
use crate::state::{AppState, CachedAnalogMemoryBank};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalogConeStepDto {
    pub step: usize,
    pub projected_timestamp_ms: i64,
    pub p10: f64,
    pub p25: f64,
    pub p50: f64,
    pub p75: f64,
    pub p90: f64,
    pub p10_ret_pct: f64,
    pub p25_ret_pct: f64,
    pub p50_ret_pct: f64,
    pub p75_ret_pct: f64,
    pub p90_ret_pct: f64,
    pub cumulative_floor: Option<f64>,
    pub cumulative_ceiling: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalogMatchCaseDto {
    pub ticker: String,
    pub symbol: String,
    pub entry_timestamp_ms: i64,
    pub entry_date: String,
    pub similarity_pct: f64,
    pub distance: f64,
    pub weight: f64,
    pub realized_t5_ret_pct: Option<f64>,
    pub realized_t10_ret_pct: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalogConeForecastDto {
    pub steps: Vec<AnalogConeStepDto>,
    pub win_rate_pct: f64,
    pub prob_gain_gt_3pct: f64,
    pub prob_tail_drop_6pct: f64,
    pub skew_ratio: f64,
    pub expected_return_p50_pct: f64,
    pub expected_return_mean_pct: f64,
    pub query_yz20_vol: f64,
    pub pit_candidate_count: usize,
    pub fast_candidate_count: usize,
    pub top_analogs: Vec<AnalogMatchCaseDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalogConeResponseDto {
    pub ticker: String,
    pub exchange: String,
    pub symbol: String,
    pub venue: String,
    pub interval: String,
    pub supported_interval: bool,
    pub replay_locked: bool,
    pub as_of_timestamp_ms: Option<i64>,
    pub as_of_date: Option<String>,
    pub anchor_close: Option<f64>,
    pub horizon: usize,
    pub top_k: usize,
    pub weight_profile: String,
    pub query_latency_us: u64,
    pub status_message: Option<String>,
    pub forecast: Option<AnalogConeForecastDto>,
}

impl AnalogConeResponseDto {
    fn from_forecast(
        sym: &SymbolKey,
        venue: ExchangeVenue,
        interval: Interval,
        replay_locked: bool,
        fc: AnalogConeForecast,
        query_latency_us: u64,
    ) -> Self {
        let steps = fc
            .steps
            .into_iter()
            .map(|s| AnalogConeStepDto {
                step: s.step,
                projected_timestamp_ms: s.projected_timestamp.saturating_mul(1_000),
                p10: s.p10,
                p25: s.p25,
                p50: s.p50,
                p75: s.p75,
                p90: s.p90,
                p10_ret_pct: s.p10_ret_pct,
                p25_ret_pct: s.p25_ret_pct,
                p50_ret_pct: s.p50_ret_pct,
                p75_ret_pct: s.p75_ret_pct,
                p90_ret_pct: s.p90_ret_pct,
                cumulative_floor: s.cumulative_floor,
                cumulative_ceiling: s.cumulative_ceiling,
            })
            .collect();

        let top_analogs = fc
            .top_analogs
            .into_iter()
            .map(|a| AnalogMatchCaseDto {
                ticker: a.ticker,
                symbol: a.symbol,
                entry_timestamp_ms: a.entry_timestamp.saturating_mul(1_000),
                entry_date: a.entry_date,
                similarity_pct: a.similarity_pct,
                distance: a.distance,
                weight: a.weight,
                realized_t5_ret_pct: a.realized_t5_ret_pct,
                realized_t10_ret_pct: a.realized_t10_ret_pct,
            })
            .collect();

        Self {
            ticker: sym.ticker(),
            exchange: sym.exchange.clone(),
            symbol: sym.symbol.clone(),
            venue: venue.short_label().to_owned(),
            interval: interval.as_str().to_owned(),
            supported_interval: true,
            replay_locked,
            as_of_timestamp_ms: Some(fc.as_of_timestamp.saturating_mul(1_000)),
            as_of_date: Some(fc.as_of_date),
            anchor_close: Some(fc.anchor_close),
            horizon: fc.horizon,
            top_k: fc.top_k,
            weight_profile: fc.weight_profile.as_str().to_owned(),
            query_latency_us,
            status_message: None,
            forecast: Some(AnalogConeForecastDto {
                steps,
                win_rate_pct: fc.win_rate_pct,
                prob_gain_gt_3pct: fc.prob_gain_gt_3pct,
                prob_tail_drop_6pct: fc.prob_tail_drop_6pct,
                skew_ratio: fc.skew_ratio,
                expected_return_p50_pct: fc.expected_return_p50_pct,
                expected_return_mean_pct: fc.expected_return_mean_pct,
                query_yz20_vol: fc.query_yz20_vol,
                pit_candidate_count: fc.pit_candidate_count,
                fast_candidate_count: fc.fast_candidate_count,
                top_analogs,
            }),
        }
    }
}

fn map_exchange_to_venue(exchange: &str, symbol: &str, store: &sumi_data::SqliteMarketStore) -> ExchangeVenue {
    let Ok(key) = SymbolKey::new(exchange, symbol) else {
        return ExchangeVenue::Hose;
    };
    if let Ok(Some((resolved, _))) = store.get_symbol(&key)
        && let Some(v) = resolved.exchange_venue
    {
        return v;
    }
    let fallback = synthesize_fallback_symbol(&key);
    fallback.exchange_venue.unwrap_or_else(|| {
        ExchangeVenue::infer_from_symbol_and_preset(&key.ticker(), DataSourcePreset::CustomFile)
    })
}

/// Lazily initializes (or returns the cached `Arc<AnalogMemoryBank>`) from `SqliteMarketStore`.
pub(crate) async fn get_or_build_analog_bank(state: &AppState) -> DesktopResult<Arc<AnalogMemoryBank>> {
    let target_epoch = state.analog_cache_epoch.load(Ordering::Acquire);
    if let Ok(guard) = state.analog_bank.read()
        && let Some(ref cached) = *guard
        && cached.built_epoch == target_epoch
        && !cached.bank.is_empty()
    {
        return Ok(Arc::clone(&cached.bank));
    }

    let _build_guard = state.analog_build_lock.lock().await;
    let target_epoch = state.analog_cache_epoch.load(Ordering::Acquire);
    if let Ok(guard) = state.analog_bank.read()
        && let Some(ref cached) = *guard
        && cached.built_epoch == target_epoch
        && !cached.bank.is_empty()
    {
        return Ok(Arc::clone(&cached.bank));
    }

    // If cold-start DB has virtually no daily VN bars yet and we are online, hydrate core VN basket + VNINDEX.
    if !state.is_offline() {
        let store_probe = state.market_store.clone();
        let vn_bar_total = tokio::task::spawn_blocking(move || {
            store_probe
                .stats()
                .map(|s| s.total_bars)
                .unwrap_or(0)
        })
        .await
        .unwrap_or(0);

        if vn_bar_total < 1_000 {
            let svc = state.active_market_service();
            if let Ok(vn_key) = SymbolKey::new("HOSE", "VNINDEX") {
                let _ = svc
                    .get_bars(&BarsRequest::init(
                        SeriesKey::new(&vn_key, Interval::D1, Adjustment::Splits),
                        600,
                    ))
                    .await;
            }
            for item in curated_catalog() {
                if item.market_type == "stock"
                    && matches!(item.exchange.as_str(), "HOSE" | "HNX" | "UPCOM")
                    && let Ok(k) = SymbolKey::parse(&item.ticker)
                {
                    let _ = svc
                        .get_bars(&BarsRequest::init(
                            SeriesKey::new(&k, Interval::D1, Adjustment::Splits),
                            600,
                        ))
                        .await;
                }
            }
        }
    }

    let store = state.market_store.clone();
    let (bank, total_bars, build_ms) = tokio::task::spawn_blocking(move || -> DesktopResult<(Arc<AnalogMemoryBank>, usize, u128)> {
        let t0 = Instant::now();
        let dump = store.load_vn_daily_universe_bars()?;
        let (equity_series, vnindex_bars) = (dump.equities, dump.vnindex);
        let total_bars = store.stats().map(|s| s.total_bars).unwrap_or(0);

        let mut inputs = Vec::with_capacity(equity_series.len());
        for (sym_key, bars) in equity_series {
            if bars.len() <= 66 {
                continue;
            }
            let venue = match sym_key.exchange.as_str() {
                "HNX" => ExchangeVenue::Hnx,
                "UPCOM" => ExchangeVenue::Upcom,
                _ => ExchangeVenue::Hose,
            };
            let candles: Vec<Candle> = bars
                .iter()
                .map(|b| b.to_core_candle(Interval::D1))
                .collect();
            inputs.push(AnalogSymbolInput {
                ticker: sym_key.ticker(),
                symbol: sym_key.symbol,
                venue,
                candles,
            });
        }

        let vnindex_candles: Option<Vec<Candle>> = vnindex_bars.map(|bars| {
            bars.iter()
                .map(|b| b.to_core_candle(Interval::D1))
                .collect()
        });

        let built = AnalogMemoryBank::build(
            &inputs,
            vnindex_candles.as_deref(),
            AnalogBankConfig {
                min_median_turnover_vnd: DEFAULT_MIN_MEDIAN_TURNOVER_VND,
                price_unit: VnPriceUnit::RawVnd,
            },
        );
        let elapsed_ms = t0.elapsed().as_millis();
        Ok((Arc::new(built), total_bars, elapsed_ms))
    })
    .await
    .map_err(|e| DesktopError::Internal(e.to_string()))??;

    info!(
        symbols = bank.symbol_count(),
        records = bank.len(),
        has_vnindex = bank.has_market_context(),
        elapsed_ms = build_ms,
        "initialized PIT AnalogMemoryBank for AFC"
    );

    if let Ok(mut guard) = state.analog_bank.write() {
        *guard = Some(CachedAnalogMemoryBank {
            bank: Arc::clone(&bank),
            built_epoch: target_epoch,
            built_total_bars: total_bars,
        });
    }

    Ok(bank)
}

/// Computes the Dual-Horizon Point-in-Time `AFC — Analog Forecast Cone` for `ticker`.
///
/// - Enforces timeframe applicability (`Interval::D1` only; returns clean status message on sub-daily/weekly frames).
/// - Automatically locks `as_of_timestamp` to the active `ReplayRuntime` cursor (`DEF-001`) so
///   **Bar Replay / Blind Practice** updates the probability cone causally bar-by-bar with zero future leakage.
#[tauri::command]
pub async fn compute_analog_cone(
    state: State<'_, AppState>,
    ticker: String,
    interval: String,
    horizon: Option<usize>,
    top_k: Option<usize>,
    use_market_context: Option<bool>,
    as_of_timestamp_ms: Option<i64>,
) -> DesktopResult<AnalogConeResponseDto> {
    compute_analog_cone_inner(
        &state,
        &ticker,
        &interval,
        horizon,
        top_k,
        use_market_context,
        as_of_timestamp_ms,
    )
    .await
}

pub(crate) async fn compute_analog_cone_inner(
    state: &AppState,
    ticker: &str,
    interval: &str,
    horizon: Option<usize>,
    top_k: Option<usize>,
    use_market_context: Option<bool>,
    as_of_timestamp_ms: Option<i64>,
) -> DesktopResult<AnalogConeResponseDto> {
    let sym = SymbolKey::parse(ticker)?;
    let iv: Interval = interval.parse()?;
    let hz = horizon.unwrap_or(DEFAULT_ANALOG_HORIZON).clamp(1, MAX_ANALOG_HORIZON);
    let k = top_k.unwrap_or(DEFAULT_ANALOG_TOP_K).clamp(10, 250);
    let want_mkt = use_market_context.unwrap_or(true);
    let venue = map_exchange_to_venue(&sym.exchange, &sym.symbol, &state.market_store);
    let requested_profile = if want_mkt {
        AnalogWeightProfile::MarketAware
    } else {
        AnalogWeightProfile::EndogenousOnly
    };

    // 1. Timeframe applicability guard (`AFC` is empirically calibrated strictly on `1D` sessions).
    if iv != Interval::D1 {
        return Ok(AnalogConeResponseDto {
            ticker: sym.ticker(),
            exchange: sym.exchange,
            symbol: sym.symbol,
            venue: venue.short_label().to_owned(),
            interval: iv.as_str().to_owned(),
            supported_interval: false,
            replay_locked: false,
            as_of_timestamp_ms: None,
            as_of_date: None,
            anchor_close: None,
            horizon: hz,
            top_k: k,
            weight_profile: requested_profile.as_str().to_owned(),
            query_latency_us: 0,
            status_message: Some("AFC hiệu lực trên khung 1D".to_owned()),
            forecast: None,
        });
    }

    // 2. Strict Bar Replay / Blind Practice PIT Lock (`DEF-001`):
    // Never allow `as_of_timestamp` or `query_bars` to look past `ReplayRuntime` cursor!
    let (replay_locked, mut query_bars, effective_cut_ms): (bool, Vec<OhlcvBar>, Option<i64>) = {
        let rg = state.replay.lock().await;
        if let Some(rt) = rg.as_ref()
            && rt.controller.is_active()
            && rt.ticker.eq_ignore_ascii_case(&sym.ticker())
        {
            let cut_idx = rt
                .controller
                .base_cursor_index
                .min(rt.base_bars.len().saturating_sub(1));
            let replay_cut_ms = rt.base_bars.get(cut_idx).map(|b| b.timestamp);
            let locked_cut_ms = match (as_of_timestamp_ms, replay_cut_ms) {
                (Some(req_ms), Some(rep_ms)) => Some(req_ms.min(rep_ms)),
                (None, rep_ms) => rep_ms,
                (req_ms, None) => req_ms,
            };
            let materialized =
                crate::commands::replay::materialize_replay_bars(rt, Interval::D1, &state.market_store);
            (true, materialized, locked_cut_ms)
        } else {
            (false, Vec::new(), as_of_timestamp_ms)
        }
    };

    if !replay_locked {
        let series = SeriesKey::new(&sym, Interval::D1, Adjustment::Splits);
        if let Some(cut_ms) = effective_cut_ms {
            let store = state.market_store.clone();
            let series_clone = series.clone();
            query_bars = tokio::task::spawn_blocking(move || {
                store
                    .query_bars(
                        &series_clone,
                        BarQueryMode::Forward {
                            before_ms: cut_ms.saturating_add(1),
                        },
                        420,
                    )
                    .map(|r| r.bars)
            })
            .await
            .map_err(|e| DesktopError::Internal(e.to_string()))??;
        } else {
            let svc = state.active_market_service();
            query_bars = svc
                .get_bars_instant(&BarsRequest::init(series.clone(), 420))
                .await
                .map(|p| p.bars)
                .unwrap_or_default();
            if query_bars.len() <= 66 && !state.is_offline() {
                query_bars = svc
                    .get_bars(&BarsRequest::init(series, 420))
                    .await
                    .map(|p| p.bars)
                    .unwrap_or_default();
            }
        }
    } else if let Some(cut_ms) = effective_cut_ms {
        query_bars.retain(|b| b.timestamp <= cut_ms);
    }

    let bank = get_or_build_analog_bank(state).await?;

    let sym_clone = sym.clone();
    let cut_secs = effective_cut_ms.map(|ms| ms.div_euclid(1_000));
    let price_unit = if venue.is_vn_cash_equity() {
        VnPriceUnit::RawVnd
    } else {
        VnPriceUnit::Auto
    };

    tokio::task::spawn_blocking(move || {
        let query_candles: Vec<Candle> = query_bars
            .iter()
            .map(|b| b.to_core_candle(Interval::D1))
            .collect();

        let t0 = Instant::now();
        let res = bank.query(
            &sym_clone.ticker(),
            venue,
            &query_candles,
            cut_secs,
            AnalogConeConfig {
                horizon: hz,
                top_k: k,
                weight_profile: requested_profile,
                price_unit,
            },
        );
        let latency_us = t0.elapsed().as_micros() as u64;

        match res {
            Ok(fc) => Ok(AnalogConeResponseDto::from_forecast(
                &sym_clone,
                venue,
                iv,
                replay_locked,
                fc,
                latency_us,
            )),
            Err(
                err @ (AnalogConeError::InsufficientQueryHistory { .. }
                | AnalogConeError::InsufficientPitAnalogs { .. }
                | AnalogConeError::InvalidQueryAnchorPrice),
            ) => {
                let last_c = query_candles.last();
                Ok(AnalogConeResponseDto {
                    ticker: sym_clone.ticker(),
                    exchange: sym_clone.exchange,
                    symbol: sym_clone.symbol,
                    venue: venue.short_label().to_owned(),
                    interval: iv.as_str().to_owned(),
                    supported_interval: true,
                    replay_locked,
                    as_of_timestamp_ms: last_c.map(|c| c.timestamp.saturating_mul(1_000)),
                    as_of_date: last_c.map(|c| format_utc_date(c.timestamp)),
                    anchor_close: last_c.map(|c| c.close),
                    horizon: hz,
                    top_k: k,
                    weight_profile: requested_profile.as_str().to_owned(),
                    query_latency_us: latency_us,
                    status_message: Some(err.to_string()),
                    forecast: None,
                })
            }
        }
    })
    .await
    .map_err(|e| DesktopError::Internal(e.to_string()))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use sumi_core::replay::ReplayController;
    use sumi_data::ResolvedSymbol;

    use crate::state::ReplayRuntime;

    fn seed_test_store(app_state: &AppState) {
        let tickers = [
            ("HOSE", "FPT", 95_000.0, 2_500_000.0, 11_u64),
            ("HOSE", "VCB", 88_000.0, 1_800_000.0, 22_u64),
            ("HOSE", "HPG", 28_000.0, 12_000_000.0, 33_u64),
            ("HNX", "SHS", 15_500.0, 7_500_000.0, 44_u64),
            ("UPCOM", "ACV", 102_000.0, 800_000.0, 55_u64),
        ];
        let start_ms = 1_640_995_200_000_i64; // 2022-01-01 UTC
        let count = 360_usize;

        for (ex, sym, base_p, base_v, mut seed) in tickers {
            let resolved = ResolvedSymbol::from_cafef_vn_ticker(ex, sym, false);
            app_state.market_store.upsert_symbol(&resolved).unwrap();
            let sk = SymbolKey::new(ex, sym).unwrap();
            let series = SeriesKey::new(&sk, Interval::D1, Adjustment::Splits);

            let mut p = base_p;
            let mut bars = Vec::with_capacity(count);
            for i in 0..count {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                let u = ((seed >> 11) as f64) / ((1_u64 << 53) as f64);
                let next_p = ((p * (1.0 + (u - 0.49) * 0.04)) / 100.0).round() * 100.0;
                let hi = next_p.max(p) + 200.0;
                let lo = (next_p.min(p) - 200.0).max(1_000.0);
                bars.push(OhlcvBar::new(
                    start_ms + (i as i64) * 86_400_000,
                    p,
                    hi,
                    lo,
                    next_p,
                    base_v,
                ));
                p = next_p;
            }
            app_state
                .market_store
                .upsert_bars(&series, &bars, "test", Some(true))
                .unwrap();
        }

        // Seed VNINDEX
        let vn_sym = ResolvedSymbol::from_cafef_vn_ticker("HOSE", "VNINDEX", true);
        app_state.market_store.upsert_symbol(&vn_sym).unwrap();
        let vn_sk = SymbolKey::new("HOSE", "VNINDEX").unwrap();
        let vn_series = SeriesKey::new(&vn_sk, Interval::D1, Adjustment::Splits);
        let vn_bars: Vec<OhlcvBar> = (0..count)
            .map(|i| {
                let base = 1_200.0 + (i as f64) * 0.4;
                OhlcvBar::new(
                    start_ms + (i as i64) * 86_400_000,
                    base,
                    base + 8.0,
                    base - 7.0,
                    base + 1.5,
                    650_000_000.0,
                )
            })
            .collect();
        app_state
            .market_store
            .upsert_bars(&vn_series, &vn_bars, "test", Some(true))
            .unwrap();
    }

    #[tokio::test]
    async fn compute_analog_cone_locks_replay_cursor_and_guards_non_1d_frames() {
        let dir = tempfile::tempdir().unwrap();
        let mkt_db = dir.path().join("test_mkt.db");
        let ws_db = dir.path().join("test_ws.db");
        let state = AppState::initialize(mkt_db, ws_db).unwrap();
        state.set_offline(true);
        seed_test_store(&state);

        // 1. Non-1D frame gracefully returns `supported_interval = false` + Vietnamese hint
        let intraday = compute_analog_cone_inner(&state, "HOSE:FPT", "1h", None, None, None, None)
            .await
            .unwrap();
        assert!(!intraday.supported_interval);
        assert!(intraday.forecast.is_none());
        assert_eq!(
            intraday.status_message.as_deref(),
            Some("AFC hiệu lực trên khung 1D")
        );

        // 2. Live 1D query succeeds
        let live_res = compute_analog_cone_inner(&state, "HOSE:FPT", "1D", Some(10), Some(60), Some(true), None)
            .await
            .unwrap();
        assert!(live_res.supported_interval);
        assert!(!live_res.replay_locked);
        assert!(live_res.forecast.is_some());

        // 3. Activate Bar Replay at bar 240 -> must lock `as_of_timestamp_ms` strictly to bar 240
        // even if caller attempts to pass `as_of_timestamp_ms = i64::MAX`!
        let fpt_sk = SymbolKey::new("HOSE", "FPT").unwrap();
        let fpt_bars = state
            .market_store
            .query_bars(
                &SeriesKey::new(&fpt_sk, Interval::D1, Adjustment::Splits),
                BarQueryMode::Init,
                500,
            )
            .unwrap()
            .bars;
        let fpt_candles: Vec<Candle> = fpt_bars
            .iter()
            .map(|b| b.to_core_candle(Interval::D1))
            .collect();
        let mut ctrl = ReplayController::new();
        ctrl.start_at_base_index(&fpt_candles, 240);
        let expected_cut_ms = fpt_bars[240].timestamp;

        {
            let mut rg = state.replay.lock().await;
            *rg = Some(ReplayRuntime {
                ticker: "HOSE:FPT".to_owned(),
                interval: Interval::D1,
                tracked_intervals: vec![Interval::D1],
                base_bars: fpt_bars,
                base_candles: fpt_candles,
                controller: ctrl,
                speed_multiplier: 1.0,
                blind_drill: None,
            });
        }

        let replay_res = compute_analog_cone_inner(
            &state,
            "HOSE:FPT",
            "1D",
            Some(10),
            Some(60),
            Some(true),
            Some(i64::MAX),
        )
        .await
        .unwrap();
        assert!(replay_res.replay_locked);
        assert_eq!(replay_res.as_of_timestamp_ms, Some(expected_cut_ms));
        let fc = replay_res.forecast.expect("replay forecast present");
        assert_eq!(fc.steps.len(), 10);
        for analog in &fc.top_analogs {
            // Every historical analog's T+10 must have settled ON OR BEFORE `expected_cut_ms`!
            assert!(analog.entry_timestamp_ms < expected_cut_ms);
        }
    }

    #[test]
    fn real_vn_market_db_query_latency_under_2ms_readonly() {
        let maybe_db = crate::state::workspace_ancestor_dirs()
            .into_iter()
            .map(|root| root.join("data/cache/sumi_market.db"))
            .find(|p| p.exists());
        let Some(db_path) = maybe_db else {
            return;
        };

        // Strictly read-only immutable connection — NEVER mutates `data/cache/sumi_market.db`!
        let conn = rusqlite::Connection::open_with_flags(
            &db_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .unwrap();

        let mut stmt = conn
            .prepare(
                r#"
                SELECT exchange, symbol, timestamp_ms, open, high, low, close, volume
                FROM bars
                WHERE exchange IN ('HOSE', 'HNX', 'UPCOM')
                  AND interval = '1D'
                  AND adjustment = 'splits'
                ORDER BY exchange ASC, symbol ASC, timestamp_ms ASC
                "#,
            )
            .unwrap();

        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    Candle {
                        timestamp: r.get::<_, i64>(2)?.div_euclid(1_000),
                        date_label: String::new(),
                        open: r.get(3)?,
                        high: r.get(4)?,
                        low: r.get(5)?,
                        close: r.get(6)?,
                        volume: r.get(7)?,
                    },
                ))
            })
            .unwrap();

        let mut inputs: Vec<AnalogSymbolInput> = Vec::new();
        let mut vnindex: Option<Vec<Candle>> = None;
        let mut cur_key: Option<(String, String)> = None;
        let mut cur_candles: Vec<Candle> = Vec::new();

        let flush = |k: Option<(String, String)>,
                     cs: Vec<Candle>,
                     out: &mut Vec<AnalogSymbolInput>,
                     vn_out: &mut Option<Vec<Candle>>| {
            let Some((ex, sym)) = k else { return };
            if sym == "VNINDEX" {
                *vn_out = Some(cs);
                return;
            }
            if sym.contains("INDEX") || sym.starts_with("VN30") || sym.ends_with("F1!") {
                return;
            }
            let venue = match ex.as_str() {
                "HNX" => ExchangeVenue::Hnx,
                "UPCOM" => ExchangeVenue::Upcom,
                _ => ExchangeVenue::Hose,
            };
            out.push(AnalogSymbolInput {
                ticker: format!("{ex}:{sym}"),
                symbol: sym,
                venue,
                candles: cs,
            });
        };

        for item in rows {
            let (ex, sym, c) = item.unwrap();
            if cur_key.as_ref().is_none_or(|(ce, cs)| ce != &ex || cs != &sym) {
                let prev = cur_key.replace((ex, sym));
                flush(prev, std::mem::take(&mut cur_candles), &mut inputs, &mut vnindex);
            }
            cur_candles.push(c);
        }
        flush(cur_key, cur_candles, &mut inputs, &mut vnindex);

        if inputs.len() < 20 {
            return;
        }

        let bank = AnalogMemoryBank::build(
            &inputs,
            vnindex.as_deref(),
            AnalogBankConfig {
                min_median_turnover_vnd: DEFAULT_MIN_MEDIAN_TURNOVER_VND,
                price_unit: VnPriceUnit::RawVnd,
            },
        );
        let fpt = inputs
            .iter()
            .find(|s| s.ticker == "HOSE:FPT")
            .unwrap_or(&inputs[0]);
        let cfg = AnalogConeConfig {
            price_unit: VnPriceUnit::RawVnd,
            ..AnalogConeConfig::default()
        };

        // Warmup once, then measure median latency across 9 queries on the full VN memory bank.
        let _ = bank
            .query(&fpt.ticker, fpt.venue, &fpt.candles, None, cfg)
            .unwrap();
        let mut samples_us = [0_u128; 9];
        for slot in &mut samples_us {
            let t0 = Instant::now();
            let fc = bank
                .query(&fpt.ticker, fpt.venue, &fpt.candles, None, cfg)
                .unwrap();
            *slot = t0.elapsed().as_micros();
            assert_eq!(fc.steps.len(), 10);
        }
        samples_us.sort_unstable();
        let median_us = samples_us[4];
        println!("[AFC Real VN Market DB Benchmark] records={} median_latency_us={median_us}", bank.len());
        if !cfg!(debug_assertions) {
            assert!(
                median_us < 2_000,
                "release query latency {median_us} µs exceeded 2,000 µs (< 2 ms) gate (records={})",
                bank.len()
            );
        } else {
            assert!(
                median_us < 60_000,
                "unoptimized debug query latency {median_us} µs too high (records={})",
                bank.len()
            );
        }
    }
}
