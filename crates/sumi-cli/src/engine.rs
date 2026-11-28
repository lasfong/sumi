//! Headless quantitative analysis engine shared by `sumi` CLI and `sumi-mcp` server.
//! Reads `sumi_market.db` (plus read-only `yumeko-app.sqlite` via `sumi-yumeko-bridge`).

use std::path::{Path, PathBuf};

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sumi_core::backtest::{
    BacktestConfig, BacktestRunOptions, BacktestTearsheet, StopTakeConfig, StrategyPresetKind,
    run_backtest_with_options,
};
use sumi_core::data::{Timeframe, aggregate_ohlcv};
use sumi_core::model::Candle;
use sumi_core::scanner::breadth::{
    BreadthFeedInput, MarketBreadthSnapshot, compute_market_breadth_and_rrg,
};
use sumi_core::scanner::{
    ConfluenceWeightProfile, PrecomputedCatalogMatrix, SIGNAL_CATALOG, SignalOutputType,
    catalog_entry_index, evaluate_symbol_scan, is_qmv_screener_signal, lookup_qmv_profile,
};
use sumi_data::{
    Adjustment, BarQueryMode, Interval, OhlcvBar, SeriesKey, SqliteMarketStore, SymbolKey,
    infer_vn_exchange_for_ticker, sumi_data_dir,
};
use sumi_yumeko_bridge::{
    ExportedVerdictReceipt, KT01GateVerdictV1, TopDownSymbolContextDto, YumekoReadOnlyBridge,
    evaluate_kt01_gate, export_kt01_verdict_to_disk,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Kt01AnalyzeOutput {
    pub verdict: KT01GateVerdictV1,
    pub topdown_context: TopDownSymbolContextDto,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenCliParams {
    pub scope: String,
    #[serde(default)]
    pub signal_ids: Vec<String>,
    #[serde(default)]
    pub min_health_score: Option<f64>,
    #[serde(default)]
    pub as_of: Option<String>,
    #[serde(default)]
    pub weekly_regime: Option<String>,
    #[serde(default)]
    pub only_money_in: bool,
    #[serde(default)]
    pub min_tmc: Option<f64>,
    #[serde(default)]
    pub max_pe: Option<f64>,
    #[serde(default)]
    pub min_roe: Option<f64>,
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenRowOut {
    pub ticker: String,
    pub symbol: String,
    pub last_close: f64,
    pub change_pct: f64,
    pub rvol_20: Option<f64>,
    pub rsi_14: Option<f64>,
    pub regime: String,
    pub weekly_trend: String,
    pub health_score: Option<f64>,
    pub matched_signals: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pe: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pb: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roe: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qmv_real_tmc: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qmv_real_dspi: Option<f64>,
    pub qmv_real_money_in_today: bool,
    pub qmv_real_provenance: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenCliResponse {
    pub scope: String,
    pub scanned_symbols: usize,
    pub matched_symbols: usize,
    pub rows: Vec<ScreenRowOut>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BacktestCliParams {
    pub ticker: String,
    pub preset: String,
    pub interval: String,
    pub initial_cash: f64,
    pub from_date: Option<String>,
    pub to_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BacktestCliResponse {
    pub ticker: String,
    pub interval: String,
    pub preset: String,
    pub bar_count: usize,
    pub advisory_only: bool,
    pub tearsheet: BacktestTearsheet,
}

/// Resolves canonical `sumi_market.db` path (honoring `SUMI_MARKET_DB` or `sumi_data_dir()`).
#[must_use]
pub fn default_market_db_path() -> PathBuf {
    if let Ok(explicit) = std::env::var("SUMI_MARKET_DB")
        && !explicit.trim().is_empty()
    {
        return PathBuf::from(explicit.trim());
    }
    let canonical = sumi_data_dir().join("sumi_market.db");
    if canonical.is_file() {
        return canonical;
    }
    // Check legacy macOS location read-only fallback if user runs CLI before desktop migration
    for legacy in sumi_data::legacy_sumi_app_data_dirs() {
        let p = legacy.join("sumi_market.db");
        if p.is_file() {
            return p;
        }
    }
    canonical
}

fn parse_canonical_symbol(raw: &str) -> Result<SymbolKey, String> {
    let trimmed = raw.trim().to_ascii_uppercase();
    if trimmed.contains(':') {
        SymbolKey::parse(&trimmed).map_err(|e| e.to_string())
    } else {
        let is_idx = matches!(
            trimmed.as_str(),
            "VNINDEX" | "VN30" | "HNXINDEX" | "HNX30" | "UPCOMINDEX" | "VNXALL"
        );
        let ex = infer_vn_exchange_for_ticker(&trimmed, is_idx);
        SymbolKey::new(ex, &trimmed).map_err(|e| e.to_string())
    }
}

fn load_daily_bars_with_dnse_stitch(
    store: Option<&SqliteMarketStore>,
    bridge: &YumekoReadOnlyBridge,
    sym: &SymbolKey,
    as_of: Option<&str>,
) -> Vec<OhlcvBar> {
    let cutoff_ms = as_of.and_then(sumi_yumeko_bridge::vn_as_of_end_of_day_ms);
    let query_mode = match cutoff_ms {
        Some(cut) => BarQueryMode::Forward {
            before_ms: cut.saturating_add(1),
        },
        None => BarQueryMode::Init,
    };
    let mut bars = store
        .and_then(|s| {
            s.query_bars(
                &SeriesKey::new(sym, Interval::D1, Adjustment::Splits),
                query_mode,
                600,
            )
            .ok()
        })
        .map(|r| r.bars)
        .unwrap_or_default();

    if let Ok(Some(dnse)) = bridge.load_dnse_ohlcv_series(&sym.ticker(), as_of)
        && !dnse.sumi_bars.is_empty()
    {
        let first_dnse = dnse.sumi_bars.first().unwrap().timestamp;
        if as_of.is_some() || bars.is_empty() {
            let mut stitched: Vec<OhlcvBar> = bars
                .into_iter()
                .filter(|b| b.timestamp < first_dnse)
                .collect();
            stitched.extend(dnse.sumi_bars);
            bars = stitched;
        } else if let Some(last_dnse) = dnse.sumi_bars.last()
            && bars.last().is_none_or(|b| last_dnse.timestamp > b.timestamp)
        {
            let last_local = bars.last().map_or(i64::MIN, |b| b.timestamp);
            bars.extend(
                dnse.sumi_bars
                    .into_iter()
                    .filter(|b| b.timestamp > last_local),
            );
        }
    }
    bars
}

pub fn analyze_kt01_headless(
    ticker_raw: &str,
    as_of: Option<&str>,
    market_db_override: Option<&Path>,
    yumeko_db_override: Option<&Path>,
) -> Result<Kt01AnalyzeOutput, String> {
    let sym = parse_canonical_symbol(ticker_raw)?;
    let market_path = market_db_override
        .map(Path::to_path_buf)
        .unwrap_or_else(default_market_db_path);
    let store = market_path
        .is_file()
        .then(|| SqliteMarketStore::open(&market_path).ok())
        .flatten();
    let bridge = match yumeko_db_override {
        Some(p) => YumekoReadOnlyBridge::new(p.to_path_buf(), None),
        None => YumekoReadOnlyBridge::discover_default(),
    };

    let topdown = bridge
        .load_topdown_context(&sym.ticker(), as_of)
        .map_err(|e| e.to_string())?;
    let daily_bars = load_daily_bars_with_dnse_stitch(store.as_ref(), &bridge, &sym, as_of);
    let h1_query_mode = match as_of.and_then(sumi_yumeko_bridge::vn_as_of_end_of_day_ms) {
        Some(cut) => BarQueryMode::Forward {
            before_ms: cut.saturating_add(1),
        },
        None => BarQueryMode::Init,
    };
    let hourly_bars = store
        .as_ref()
        .and_then(|s| {
            s.query_bars(
                &SeriesKey::new(&sym, Interval::H1, Adjustment::Splits),
                h1_query_mode,
                200,
            )
            .ok()
        })
        .map(|r| r.bars)
        .filter(|v| v.len() >= 20);

    let verdict = evaluate_kt01_gate(
        &sym.ticker(),
        &daily_bars,
        hourly_bars.as_deref(),
        None,
        Some(&topdown),
        as_of,
    )
    .map_err(|e| e.to_string())?;

    Ok(Kt01AnalyzeOutput {
        verdict,
        topdown_context: topdown,
    })
}

pub fn export_kt01_headless(
    ticker_raw: &str,
    as_of: Option<&str>,
    copy_to_yumeko_dropzone: bool,
    market_db_override: Option<&Path>,
    data_dir_override: Option<&Path>,
) -> Result<ExportedVerdictReceipt, String> {
    let analysis = analyze_kt01_headless(ticker_raw, as_of, market_db_override, None)?;
    export_kt01_verdict_to_disk(&analysis.verdict, data_dir_override, copy_to_yumeko_dropzone)
        .map_err(|e| e.to_string())
}

pub fn run_mtf_screen_headless(
    params: &ScreenCliParams,
    market_db_override: Option<&Path>,
) -> Result<ScreenCliResponse, String> {
    let market_path = market_db_override
        .map(Path::to_path_buf)
        .unwrap_or_else(default_market_db_path);
    if !market_path.is_file() {
        return Ok(ScreenCliResponse {
            scope: params.scope.clone(),
            scanned_symbols: 0,
            matched_symbols: 0,
            rows: Vec::new(),
        });
    }
    let store = SqliteMarketStore::open(&market_path).map_err(|e| e.to_string())?;
    let local_funda_map = store.list_fundamentals_map().unwrap_or_default();
    let bridge = YumekoReadOnlyBridge::discover_default();
    let yumeko_val_map = bridge
        .read_latest_valuation_map(params.as_of.as_deref())
        .unwrap_or_default();
    let yumeko_qmv_map = bridge
        .read_latest_qmv_screener_map(params.as_of.as_deref())
        .unwrap_or_default();
    let cutoff_ms = params
        .as_of
        .as_deref()
        .and_then(sumi_yumeko_bridge::vn_as_of_end_of_day_ms);
    let query_mode = match cutoff_ms {
        Some(cut) => BarQueryMode::Forward {
            before_ms: cut.saturating_add(1),
        },
        None => BarQueryMode::Init,
    };
    let weekly_target = params
        .weekly_regime
        .as_deref()
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty());

    let scope_upper = params.scope.trim().to_ascii_uppercase();
    let ex_filter = match scope_upper.as_str() {
        "HOSE" | "HNX" | "UPCOM" => Some(scope_upper.as_str()),
        _ => None,
    };
    let hits = store
        .search_local_symbols("", ex_filter, 600)
        .map_err(|e| e.to_string())?;

    let req_indices: Vec<usize> = params
        .signal_ids
        .iter()
        .filter_map(|id| catalog_entry_index(id.trim()))
        .collect();
    let profile = ConfluenceWeightProfile::Balanced;

    let mut scanned = 0usize;
    let mut rows = Vec::new();
    for h in hits {
        if h.market_type == "index" || h.market_type == "futures" {
            continue;
        }
        if !matches!(h.exchange.as_str(), "HOSE" | "HNX" | "UPCOM") {
            continue;
        }
        if scope_upper == "QMV30" && !lookup_qmv_profile(&h.symbol).is_some_and(|p| p.in_qmv30) {
            continue;
        }
        if scope_upper == "QMV60" && !lookup_qmv_profile(&h.symbol).is_some_and(|p| p.in_qmv60) {
            continue;
        }
        let Ok(sym) = SymbolKey::parse(&h.ticker) else {
            continue;
        };

        // Early QMV Blackbox (`qmv_real`) & Fundamental (`DN01` / TV cache) filters (`W6-10`)
        let sym_up = sym.symbol.to_ascii_uppercase();
        let y_val = yumeko_val_map.get(&sym_up);
        let l_funda = local_funda_map.get(&sym_up);
        let y_qmv = yumeko_qmv_map.get(&sym_up);

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

        if params.only_money_in && !y_qmv.is_some_and(|q| q.money_in_today) {
            continue;
        }
        if let Some(lo_tmc) = params.min_tmc
            && y_qmv.and_then(|q| q.tmc).is_none_or(|v| v < lo_tmc)
        {
            continue;
        }
        if let Some(cap_pe) = params.max_pe
            && pe.is_none_or(|v| v > cap_pe)
        {
            continue;
        }
        if let Some(lo_roe) = params.min_roe
            && roe.is_none_or(|v| v < lo_roe)
        {
            continue;
        }

        let Ok(page) = store.query_bars(
            &SeriesKey::new(&sym, Interval::D1, Adjustment::Splits),
            query_mode,
            300,
        ) else {
            continue;
        };
        if page.bars.len() < 55 {
            continue;
        }
        scanned += 1;
        let candles: Vec<Candle> = page
            .bars
            .iter()
            .map(|b| b.to_core_candle(Interval::D1))
            .collect();
        let last_idx = candles.len() - 1;
        let Some(scan) =
            evaluate_symbol_scan(&h.ticker, &sym.symbol, &sym.exchange, Timeframe::D1, &candles, None, 5)
        else {
            continue;
        };
        let matrix = PrecomputedCatalogMatrix::compute(&candles);
        let health = matrix
            .health_score_at(last_idx, profile)
            .ok()
            .filter(|v| v.is_finite());
        if let Some(min_h) = params.min_health_score
            && health.is_none_or(|v| v < min_h)
        {
            continue;
        }
        let mut all_ok = true;
        for &c_idx in &req_indices {
            let entry = &SIGNAL_CATALOG[c_idx];
            let matched = matrix
                .point_at_with_profile(c_idx, last_idx, profile)
                .is_some_and(|p| {
                    p.quality.is_valid()
                        && match entry.output_type {
                            SignalOutputType::Bool => p.bool_value == Some(true),
                            SignalOutputType::Float => p.float_value.is_some_and(f64::is_finite),
                            SignalOutputType::Enum => true,
                        }
                });
            if !matched {
                all_ok = false;
                break;
            }
        }
        if !all_ok {
            continue;
        }

        let (weekly_trend, weekly_regime_slug) = aggregate_ohlcv(&candles, Timeframe::D1, Timeframe::W1)
            .ok()
            .and_then(|w| {
                if w.len() < 10 {
                    return None;
                }
                let last_c = w.last()?.close;
                let ma10: f64 = w[w.len() - 10..].iter().map(|b| b.close).sum::<f64>() / 10.0;
                if last_c >= ma10 {
                    Some(("1W Tăng/Tích lũy".to_owned(), "uptrend"))
                } else {
                    Some(("1W Điều chỉnh".to_owned(), "downtrend"))
                }
            })
            .unwrap_or_else(|| ("—".to_owned(), "neutral"));

        if let Some(ref target_w) = weekly_target {
            let matched_w = match target_w.as_str() {
                "uptrend" | "bull" | "tang" => weekly_regime_slug == "uptrend",
                "downtrend" | "bear" | "giam" => weekly_regime_slug == "downtrend",
                _ => true,
            };
            if !matched_w {
                continue;
            }
        }

        let mut matched_signals = Vec::new();
        for (idx, entry) in SIGNAL_CATALOG.iter().enumerate().rev() {
            if !is_qmv_screener_signal(entry.id) || entry.output_type != SignalOutputType::Bool {
                continue;
            }
            if let Some(pt) = matrix.point_at_with_profile(idx, last_idx, profile)
                && pt.quality.is_valid()
                && pt.bool_value == Some(true)
            {
                matched_signals.push(entry.label_vi.to_owned());
                if matched_signals.len() >= 5 {
                    break;
                }
            }
        }

        rows.push(ScreenRowOut {
            ticker: h.ticker,
            symbol: sym.symbol,
            last_close: scan.last_close,
            change_pct: (scan.change_pct * 100.0).round() / 100.0,
            rvol_20: scan.rvol_20.map(|v| (v * 100.0).round() / 100.0),
            rsi_14: scan.rsi_14.map(|v| (v * 10.0).round() / 10.0),
            regime: scan.regime.full_label().to_owned(),
            weekly_trend,
            health_score: health.map(|v| (v * 10.0).round() / 10.0),
            matched_signals,
            pe,
            pb,
            roe,
            qmv_real_tmc: y_qmv.and_then(|q| q.tmc),
            qmv_real_dspi: y_qmv.and_then(|q| q.dspi),
            qmv_real_money_in_today: y_qmv.is_some_and(|q| q.money_in_today),
            qmv_real_provenance: y_qmv.is_some(),
        });
    }

    rows.sort_by(|a, b| {
        b.health_score
            .unwrap_or(-999.0)
            .total_cmp(&a.health_score.unwrap_or(-999.0))
    });
    let matched_symbols = rows.len();
    rows.truncate(params.limit.clamp(1, 200));

    Ok(ScreenCliResponse {
        scope: params.scope.clone(),
        scanned_symbols: scanned,
        matched_symbols,
        rows,
    })
}

pub fn run_backtest_headless(
    params: &BacktestCliParams,
    market_db_override: Option<&Path>,
) -> Result<BacktestCliResponse, String> {
    let sym = parse_canonical_symbol(&params.ticker)?;
    let iv: Interval = params.interval.parse().map_err(|e: sumi_data::DataError| e.to_string())?;
    let tf = iv.to_core_timeframe().unwrap_or(Timeframe::D1);
    let market_path = market_db_override
        .map(Path::to_path_buf)
        .unwrap_or_else(default_market_db_path);
    let store = SqliteMarketStore::open(&market_path).map_err(|e| e.to_string())?;
    let mut bars = store
        .query_bars(&SeriesKey::new(&sym, iv, Adjustment::Splits), BarQueryMode::Init, 2_500)
        .map_err(|e| e.to_string())?
        .bars;

    let from_ms = params
        .from_date
        .as_deref()
        .and_then(|s| NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d").ok())
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|dt| dt.and_utc().timestamp_millis());
    let to_ms = params
        .to_date
        .as_deref()
        .and_then(|s| NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d").ok())
        .and_then(|d| d.and_hms_opt(23, 59, 59))
        .map(|dt| dt.and_utc().timestamp_millis());
    if from_ms.is_some() || to_ms.is_some() {
        bars.retain(|b| {
            from_ms.is_none_or(|f| b.timestamp >= f) && to_ms.is_none_or(|t| b.timestamp <= t)
        });
    }
    if bars.len() < 30 {
        return Err(format!(
            "Không đủ dữ liệu backtest cho {} (có {} nến, cần >= 30)",
            sym.ticker(),
            bars.len()
        ));
    }

    let preset = match params.preset.trim().to_ascii_lowercase().as_str() {
        "qmv_s1" => StrategyPresetKind::QmvSupportVpa,
        "qmv_s2" => StrategyPresetKind::QmvSafeBottomDiv,
        "qmv_s3" => StrategyPresetKind::QmvIchimokuV7,
        "qmv_s4" => StrategyPresetKind::QmvDualCycleFlow,
        "qmv_s5" => StrategyPresetKind::QmvCongestionBreakout,
        "qmv_s6" => StrategyPresetKind::QmvMultiGateConfluence,
        "rsi_mean_rev" | "rsi" => StrategyPresetKind::RsiMeanReversion,
        "macd" => StrategyPresetKind::MacdMomentum,
        "bb_squeeze" | "bb" => StrategyPresetKind::BollingerSqueezeBreakout,
        "donchian_atr" | "donchian" => StrategyPresetKind::DonchianAtrBreakout,
        "rsi_bb" => StrategyPresetKind::RsiBollingerConfluence,
        _ => StrategyPresetKind::SmaEmaTrendCross,
    };

    let risk = StopTakeConfig {
        min_holding_bars: if iv == Interval::D1
            && matches!(sym.exchange.as_str(), "HOSE" | "HNX" | "UPCOM")
        {
            2
        } else {
            0
        },
        ..StopTakeConfig::default()
    };

    let mut cfg = BacktestConfig {
        preset,
        risk,
        initial_cash: params.initial_cash.max(10_000_000.0),
        commission_bps: 15.0,
        tax_sell_bps: 10.0,
        slippage_bps: 2.0,
        lot_step: if matches!(sym.exchange.as_str(), "HOSE" | "HNX" | "UPCOM") {
            100.0
        } else {
            1.0
        },
        enforce_vn_microstructure: matches!(sym.exchange.as_str(), "HOSE" | "HNX" | "UPCOM"),
        ..BacktestConfig::default()
    };
    cfg.sanitize();

    let candles: Vec<Candle> = bars.iter().map(|b| b.to_core_candle(iv)).collect();
    let report = run_backtest_with_options(
        sym.ticker(),
        tf,
        &candles,
        &cfg,
        BacktestRunOptions::default(),
    );

    Ok(BacktestCliResponse {
        ticker: report.symbol,
        interval: iv.as_str().to_owned(),
        preset: report.config.preset.full_label().to_owned(),
        bar_count: report.bar_count,
        advisory_only: true,
        tearsheet: report.tearsheet,
    })
}

pub fn compute_breadth_rrg_headless(
    as_of: Option<&str>,
    market_db_override: Option<&Path>,
) -> Result<MarketBreadthSnapshot, String> {
    let market_path = market_db_override
        .map(Path::to_path_buf)
        .unwrap_or_else(default_market_db_path);
    if !market_path.is_file() {
        return Ok(MarketBreadthSnapshot::default());
    }
    let store = SqliteMarketStore::open(&market_path).map_err(|e| e.to_string())?;
    let cutoff_ts = as_of
        .and_then(|s| NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d").ok())
        .and_then(|d| d.and_hms_opt(23, 59, 59))
        .map(|dt| dt.and_utc().timestamp());

    let bench_sym = SymbolKey::new("HOSE", "VNINDEX").map_err(|e| e.to_string())?;
    let bench_bars = store
        .query_bars(
            &SeriesKey::new(&bench_sym, Interval::D1, Adjustment::Splits),
            BarQueryMode::Init,
            400,
        )
        .map(|r| r.bars)
        .unwrap_or_default();
    let bench_candles: Vec<Candle> = bench_bars
        .iter()
        .map(|b| b.to_core_candle(Interval::D1))
        .collect();

    let hits = store
        .search_local_symbols("", None, 500)
        .map_err(|e| e.to_string())?;
    let mut owned: Vec<(String, String, String, Vec<Candle>)> = Vec::new();
    for h in hits {
        if h.market_type == "index"
            || h.market_type == "futures"
            || !matches!(h.exchange.as_str(), "HOSE" | "HNX" | "UPCOM")
        {
            continue;
        }
        let Ok(sym) = SymbolKey::parse(&h.ticker) else {
            continue;
        };
        let Ok(res) = store.query_bars(
            &SeriesKey::new(&sym, Interval::D1, Adjustment::Splits),
            BarQueryMode::Init,
            260,
        ) else {
            continue;
        };
        if res.bars.len() < 40 {
            continue;
        }
        let sector_lbl = lookup_qmv_profile(&sym.symbol)
            .map_or("Khác", |p| p.sector.label_vi())
            .to_owned();
        let candles: Vec<Candle> = res
            .bars
            .iter()
            .map(|b| b.to_core_candle(Interval::D1))
            .collect();
        owned.push((h.ticker, sym.symbol, sector_lbl, candles));
    }

    let feeds: Vec<BreadthFeedInput<'_>> = owned
        .iter()
        .map(|(ticker, sym, sector, cs)| BreadthFeedInput {
            instrument_id: ticker.as_str(),
            symbol: sym.as_str(),
            preset_label: sector.as_str(),
            timeframe: Timeframe::D1,
            candles: cs.as_slice(),
            exclude_incomplete_tail: false,
        })
        .collect();

    Ok(compute_market_breadth_and_rrg(
        &feeds,
        &bench_candles,
        Timeframe::D1,
        "VNINDEX",
        cutoff_ts,
    ))
}
