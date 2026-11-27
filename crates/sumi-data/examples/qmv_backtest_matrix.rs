//! Automated Multi-Combination QMV Backtest & Walk-Forward + Monte Carlo Research Harness (Stage 4).
//!
//! Runs strictly causal Vietnam microstructure backtests (`HOSE ±7%`, `HNX ±10%`, `UPCOM ±15%`,
//! locked ceiling/floor rules, `10/50/100đ` tick ladder, lot `100`, `T+3` pre-`2022-08-29` / `T+2` post,
//! `0.15%` fee + `0.10%` sell tax + `0.05%` slippage) over real historical daily bars in `sumi_market.db`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use rusqlite::Connection;
use serde::Serialize;
use sumi_core::backtest::{
    BacktestConfig, BacktestRunOptions, DEFAULT_MONTE_CARLO_SEED, MonteCarloConfig,
    PortfolioUniverseFilter, PortfolioWeightingMode, PositionSizingMode, StopTakeConfig,
    StrategyDirectionMode, StrategyParams, StrategyPresetKind, SweepRankingMetric,
    WalkForwardConfig, WalkForwardVerdict, run_backtest_with_options,
    run_monte_carlo_from_equity_curve, run_portfolio_backtest, run_walk_forward_validation,
};
use sumi_core::data::{DataSourcePreset, Timeframe, WatchlistFeedEntry};
use sumi_core::model::Candle;
use sumi_core::scanner::qmv_universe::{
    QmvLiquidityTier, QmvMoneyTier, QmvPriceTier, QmvSector, QmvSectorGroup, QmvSymbolProfile,
    is_vn_exchange, lookup_qmv_profile,
};
use sumi_core::scanner::{MarketRegime, compute_regime_series};
use sumi_data::{Interval, OhlcvBar};

const START_2016_MS: i64 = 1_451_606_400_000; // 2016-01-01T00:00:00Z
const MIN_BARS_FOR_WFO: usize = 260;

#[derive(Clone)]
struct SymbolSeries {
    ticker: String,
    symbol: String,
    candles: Vec<Candle>,
    profile: QmvSymbolProfile,
    median_vol20: f64,
    price_tier: QmvPriceTier,
    liq_tier: QmvLiquidityTier,
}

#[derive(Clone)]
struct TradeSliceRecord {
    return_pct: f64,
    mae_pct: f64,
    vnindex_regime: &'static str, // "BULL" | "SIDEWAYS" | "BEAR"
}

#[derive(Clone)]
struct SymbolStrategyEval {
    is_return_pct: f64,
    is_sharpe: f64,
    wfo_oos_return_pct: f64,
    wfo_oos_cagr_pct: f64,
    wfo_oos_max_dd_pct: f64,
    wfo_oos_sharpe: f64,
    wfo_oos_sortino: f64,
    wfo_oos_trades: usize,
    wfo_wfe_pct: f64,
    wfo_robust: bool,
    last_winning_label: String,
    limit_locked_skips: usize,
    illiquid_skips: usize,
    oos_trade_records: Vec<TradeSliceRecord>,
    ungated_trade_records: Vec<TradeSliceRecord>,
}

#[derive(Debug, Clone, Serialize)]
struct StrategyGroupSummary {
    preset_code: String,
    preset_label: String,
    symbol_count: usize,
    total_wfo_oos_trades: usize,
    low_sample_warning: bool,
    mean_is_return_pct: f64,
    mean_is_sharpe: f64,
    mean_wfo_oos_return_pct: f64,
    median_wfo_oos_return_pct: f64,
    mean_wfo_oos_cagr_pct: f64,
    mean_wfo_oos_max_dd_pct: f64,
    mean_wfo_oos_sharpe: f64,
    mean_wfo_oos_sortino: f64,
    pooled_oos_win_rate_pct: f64,
    pooled_oos_profit_factor: f64,
    pooled_avg_trade_return_pct: f64,
    mean_wfe_pct: f64,
    pct_symbols_oos_profitable: f64,
    pct_symbols_wfo_robust: f64,
    dominant_winning_variant: String,
    portfolio_eq_return_pct: Option<f64>,
    portfolio_eq_cagr_pct: Option<f64>,
    portfolio_eq_max_dd_pct: Option<f64>,
    portfolio_eq_sharpe: Option<f64>,
    portfolio_invvol_return_pct: Option<f64>,
    portfolio_invvol_cagr_pct: Option<f64>,
    portfolio_invvol_max_dd_pct: Option<f64>,
    portfolio_invvol_sharpe: Option<f64>,
    mc_prob_profit_pct: Option<f64>,
    mc_prob_ruin_35_pct: Option<f64>,
    mc_p50_return_pct: Option<f64>,
    mc_var95_dd_pct: Option<f64>,
    mc_cvar95_dd_pct: Option<f64>,
    composite_rank_score: f64,
}

#[derive(Debug, Clone, Serialize)]
struct GroupReport {
    group_id: String,
    dimension: String,
    group_label: String,
    symbol_count: usize,
    sample_tickers: Vec<String>,
    winning_preset_code: String,
    winning_preset_label: String,
    winning_variant: String,
    strategies: Vec<StrategyGroupSummary>,
}

#[derive(Debug, Clone, Serialize)]
struct RegimeSliceSummary {
    preset_code: String,
    preset_label: String,
    regime: String,
    gated_trades: usize,
    gated_win_rate_pct: f64,
    gated_avg_return_pct: f64,
    gated_profit_factor: f64,
    ungated_trades: usize,
    ungated_win_rate_pct: f64,
    ungated_avg_return_pct: f64,
    ungated_profit_factor: f64,
    ungated_avg_mae_pct: f64,
}

#[derive(Debug, Clone, Serialize)]
struct AblationRow {
    experiment: String,
    variant_label: String,
    group_label: String,
    symbol_count: usize,
    total_trades: usize,
    mean_return_pct: f64,
    median_return_pct: f64,
    mean_cagr_pct: f64,
    mean_max_dd_pct: f64,
    mean_sharpe: f64,
    win_rate_pct: f64,
    profit_factor: f64,
}

#[derive(Debug, Serialize)]
struct FullMatrixOutput {
    generated_at: String,
    db_path: String,
    vn_total_1d_symbols: usize,
    wfo_eligible_symbols: usize,
    total_1d_bars_loaded: usize,
    vnindex_bars: usize,
    vnindex_bull_days: usize,
    vnindex_sideways_days: usize,
    vnindex_bear_days: usize,
    total_limit_locked_skips: usize,
    total_illiquid_skips: usize,
    elapsed_secs: f64,
    overall_universe: GroupReport,
    stock_type_and_money_groups: Vec<GroupReport>,
    sector_groups_and_sectors: Vec<GroupReport>,
    liquidity_groups: Vec<GroupReport>,
    vnindex_regime_matrix: Vec<RegimeSliceSummary>,
    ablation_experiments: Vec<AblationRow>,
}

const EVAL_PRESETS: [(&str, StrategyPresetKind); 8] = [
    ("QMV_S1", StrategyPresetKind::QmvSupportVpa),
    ("QMV_S2", StrategyPresetKind::QmvSafeBottomDiv),
    ("QMV_S3", StrategyPresetKind::QmvIchimokuV7),
    ("QMV_S4", StrategyPresetKind::QmvDualCycleFlow),
    ("QMV_S5", StrategyPresetKind::QmvCongestionBreakout),
    ("QMV_S6", StrategyPresetKind::QmvMultiGateConfluence),
    ("BASE_SMA_EMA", StrategyPresetKind::SmaEmaTrendCross),
    ("BASE_BB_SQZ", StrategyPresetKind::BollingerSqueezeBreakout),
];

type GroupPred = Box<dyn Fn(&SymbolSeries) -> bool>;

fn median_f64(mut vals: Vec<f64>) -> f64 {
    vals.retain(|v| v.is_finite());
    if vals.is_empty() {
        return 0.0;
    }
    vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = vals.len() / 2;
    if vals.len().is_multiple_of(2) {
        0.5 * (vals[mid - 1] + vals[mid])
    } else {
        vals[mid]
    }
}

fn mean_f64(vals: &[f64]) -> f64 {
    let mut sum = 0.0;
    let mut cnt = 0usize;
    for &v in vals {
        if v.is_finite() {
            sum += v;
            cnt += 1;
        }
    }
    if cnt > 0 { sum / cnt as f64 } else { 0.0 }
}

fn build_qmv_base_config(
    preset: StrategyPresetKind,
    non_downtrend_days: Option<Arc<BTreeSet<i64>>>,
) -> BacktestConfig {
    let is_qmv = StrategyPresetKind::QMV_PLAYBOOKS.contains(&preset);
    let params = StrategyParams {
        qmv_use_vnindex_gate: is_qmv && non_downtrend_days.is_some(),
        ..StrategyParams::default()
    };

    let mut cfg = BacktestConfig {
        preset,
        direction_mode: StrategyDirectionMode::LongOnly,
        params,
        custom_rules: Default::default(),
        sizing: PositionSizingMode::PercentEquity(95.0),
        risk: StopTakeConfig {
            stop_loss_pct: Some(if is_qmv { 8.0 } else { 7.0 }),
            take_profit_pct: if preset == StrategyPresetKind::QmvSafeBottomDiv {
                Some(18.0)
            } else {
                None
            },
            min_holding_bars: 2,
            support_stop_lookback: if is_qmv { Some(20) } else { None },
            qmv_wave_trailing_2_3: is_qmv,
            ..StopTakeConfig::default()
        },
        initial_cash: 500_000_000.0,
        commission_bps: 15.0,
        tax_sell_bps: 10.0,
        slippage_bps: 5.0,
        max_leverage: 1.0,
        lot_step: 100.0,
        in_sample_split_pct: Some(0.70),
        enforce_vn_microstructure: true,
        min_entry_avg_vol_20: 10_000.0,
        benchmark_non_downtrend_days: if is_qmv { non_downtrend_days } else { None },
    };
    cfg.sanitize();
    cfg
}

type VnindexRegimeInfo = (
    Arc<BTreeSet<i64>>,
    BTreeMap<i64, &'static str>,
    (usize, usize, usize),
);

fn classify_vnindex_day_regimes(vnindex_candles: &[Candle]) -> VnindexRegimeInfo {
    let regimes = compute_regime_series(vnindex_candles);
    let mut non_down = BTreeSet::new();
    let mut day_map = BTreeMap::new();
    let mut bull = 0usize;
    let mut side = 0usize;
    let mut bear = 0usize;

    for (c, reg) in vnindex_candles.iter().zip(regimes) {
        let day = c.timestamp.div_euclid(86_400);
        if reg != MarketRegime::Downtrend && reg != MarketRegime::InsufficientHistory {
            non_down.insert(day);
        }
        let bucket = match reg {
            MarketRegime::Uptrend | MarketRegime::Recovery => {
                if c.timestamp * 1000 >= START_2016_MS {
                    bull += 1;
                }
                "BULL"
            }
            MarketRegime::Downtrend => {
                if c.timestamp * 1000 >= START_2016_MS {
                    bear += 1;
                }
                "BEAR"
            }
            _ => {
                if c.timestamp * 1000 >= START_2016_MS {
                    side += 1;
                }
                "SIDEWAYS"
            }
        };
        day_map.insert(day, bucket);
    }
    (Arc::new(non_down), day_map, (bull, side, bear))
}

fn lookup_day_regime(day_map: &BTreeMap<i64, &'static str>, entry_ts_sec: i64) -> &'static str {
    let signal_day = entry_ts_sec.div_euclid(86_400) - 1;
    day_map
        .range(..=signal_day)
        .next_back()
        .map(|(_, &v)| v)
        .unwrap_or("SIDEWAYS")
}

fn load_all_vn_series(db_path: &Path) -> (Vec<SymbolSeries>, Vec<Candle>, usize, usize) {
    let conn = Connection::open(db_path).expect("open sumi_market.db");
    let mut stmt = conn
        .prepare(
            r#"
            SELECT exchange, symbol, timestamp_ms, open, high, low, close, volume
            FROM bars
            WHERE interval = '1D' AND adjustment = 'splits'
              AND exchange IN ('HOSE', 'HNX', 'UPCOM')
            ORDER BY exchange ASC, symbol ASC, timestamp_ms ASC
            "#,
        )
        .expect("prepare bars query");

    let mut grouped: BTreeMap<(String, String), Vec<Candle>> = BTreeMap::new();
    let mut total_bars = 0usize;
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, f64>(3)?,
                r.get::<_, f64>(4)?,
                r.get::<_, f64>(5)?,
                r.get::<_, f64>(6)?,
                r.get::<_, f64>(7)?,
            ))
        })
        .expect("query bars");

    for item in rows {
        let (ex, sym, ts_ms, o, h, l, c, v) = item.expect("bar row");
        if !is_vn_exchange(&ex) {
            continue;
        }
        total_bars += 1;
        let bar = OhlcvBar::new(ts_ms, o, h, l, c, v);
        let candle = bar.to_core_candle(Interval::D1);
        grouped.entry((ex, sym)).or_default().push(candle);
    }

    let total_vn_symbols = grouped.len();
    let mut vnindex_candles = Vec::new();
    let mut out = Vec::new();

    for ((ex, sym), all_candles) in grouped {
        if sym == "VNINDEX" {
            vnindex_candles = all_candles;
            continue;
        }
        if matches!(
            sym.as_str(),
            "VN30" | "HNXINDEX" | "UPINDEX" | "VNXALL" | "VN30F1!" | "VN301!"
        ) {
            continue;
        }
        let candles: Vec<Candle> = all_candles
            .into_iter()
            .filter(|c| c.timestamp * 1000 >= START_2016_MS)
            .collect();
        if candles.len() < MIN_BARS_FOR_WFO {
            continue;
        }
        let profile = lookup_qmv_profile(&sym).copied().unwrap_or(QmvSymbolProfile {
            symbol: "OTHER",
            sector_group: QmvSectorGroup::Group2OperatingCycle,
            sector: QmvSector::OtherCyclical,
            money_tier: QmvMoneyTier::Dt3,
            in_qmv30: false,
            in_qmv60: false,
            in_yumeko_core10: false,
            exact_yumeko_tier: false,
        });
        if profile.money_tier == QmvMoneyTier::IndexOrOther {
            continue;
        }

        let tail_start = candles.len().saturating_sub(250);
        let tail = &candles[tail_start..];
        let med_close = median_f64(tail.iter().map(|c| c.close).collect());
        let med_vol = median_f64(tail.iter().map(|c| c.volume).collect());
        let price_tier = QmvPriceTier::from_close_price(med_close);
        let liq_tier = QmvLiquidityTier::from_avg_vol_20(med_vol);

        out.push(SymbolSeries {
            ticker: format!("{ex}:{sym}"),
            symbol: sym,
            candles,
            profile,
            median_vol20: med_vol,
            price_tier,
            liq_tier,
        });
    }

    (out, vnindex_candles, total_vn_symbols, total_bars)
}

fn evaluate_symbol_all_strategies(
    s: &SymbolSeries,
    non_downtrend_days: &Arc<BTreeSet<i64>>,
    vnindex_day_regimes: &BTreeMap<i64, &'static str>,
) -> Vec<SymbolStrategyEval> {
    let wf_cfg = WalkForwardConfig {
        fold_count: 5,
        in_sample_ratio: 0.70,
        anchored: false,
        ranking_metric: SweepRankingMetric::SharpeRatio,
    };

    let mut evals = Vec::with_capacity(EVAL_PRESETS.len());
    for &(_code, preset) in &EVAL_PRESETS {
        let cfg = build_qmv_base_config(preset, Some(non_downtrend_days.clone()));
        let bt = run_backtest_with_options(
            &s.ticker,
            Timeframe::D1,
            &s.candles,
            &cfg,
            BacktestRunOptions {
                replay_cutoff_timestamp: None,
                incomplete_tail_bar: false,
            },
        );
        let limit_locked_skips = bt
            .unexecuted_signals
            .iter()
            .filter(|u| matches!(u.reason, sumi_core::backtest::UnfilledReason::LimitLocked))
            .count();
        let illiquid_skips = bt
            .unexecuted_signals
            .iter()
            .filter(|u| matches!(u.reason, sumi_core::backtest::UnfilledReason::IlliquidBar))
            .count();

        let ungated_cfg = build_qmv_base_config(preset, None);
        let bt_ungated = run_backtest_with_options(
            &s.ticker,
            Timeframe::D1,
            &s.candles,
            &ungated_cfg,
            BacktestRunOptions {
                replay_cutoff_timestamp: None,
                incomplete_tail_bar: false,
            },
        );

        let wfo = run_walk_forward_validation(
            &s.ticker,
            Timeframe::D1,
            &s.candles,
            &cfg,
            wf_cfg,
            BacktestRunOptions {
                replay_cutoff_timestamp: None,
                incomplete_tail_bar: false,
            },
        );

        let is_ts = bt.in_sample_tearsheet.as_ref().unwrap_or(&bt.tearsheet);
        let st_ts = &wfo.stitched_oos_tearsheet;

        let oos_trade_records: Vec<TradeSliceRecord> = wfo
            .stitched_oos_trades
            .iter()
            .map(|tr| TradeSliceRecord {
                return_pct: tr.return_pct,
                mae_pct: tr.mae_pct,
                vnindex_regime: lookup_day_regime(vnindex_day_regimes, tr.entry_timestamp),
            })
            .collect();

        let oos_cut_idx = (s.candles.len() as f64 * 0.70) as usize;
        let ungated_trade_records: Vec<TradeSliceRecord> = bt_ungated
            .trades
            .iter()
            .filter(|tr| tr.entry_bar_index >= oos_cut_idx)
            .map(|tr| TradeSliceRecord {
                return_pct: tr.return_pct,
                mae_pct: tr.mae_pct,
                vnindex_regime: lookup_day_regime(vnindex_day_regimes, tr.entry_timestamp),
            })
            .collect();

        let last_winning_label = wfo
            .folds
            .last()
            .map(|f| f.winning_label.clone())
            .unwrap_or_else(|| "Default".to_owned());

        evals.push(SymbolStrategyEval {
            is_return_pct: is_ts.total_return_pct,
            is_sharpe: is_ts.sharpe_ratio.unwrap_or(0.0),
            wfo_oos_return_pct: st_ts.total_return_pct,
            wfo_oos_cagr_pct: st_ts.cagr_pct.unwrap_or(0.0),
            wfo_oos_max_dd_pct: st_ts.max_drawdown_pct,
            wfo_oos_sharpe: st_ts.sharpe_ratio.unwrap_or(0.0),
            wfo_oos_sortino: st_ts.sortino_ratio.unwrap_or(0.0),
            wfo_oos_trades: st_ts.num_trades,
            wfo_wfe_pct: wfo.wfe_pct.unwrap_or(0.0).clamp(-100.0, 250.0),
            wfo_robust: matches!(
                wfo.verdict,
                WalkForwardVerdict::RobustOutOfSample | WalkForwardVerdict::ModerateDecay
            ) && st_ts.total_return_pct > 0.0,
            last_winning_label,
            limit_locked_skips,
            illiquid_skips,
            oos_trade_records,
            ungated_trade_records,
        });
    }
    evals
}

fn summarize_group(
    group_id: &str,
    dimension: &str,
    group_label: &str,
    members: &[&SymbolSeries],
    evals_by_ticker: &BTreeMap<String, Vec<SymbolStrategyEval>>,
    non_downtrend_days: &Arc<BTreeSet<i64>>,
    run_portfolio: bool,
) -> GroupReport {
    let mut sample_tickers: Vec<String> =
        members.iter().take(12).map(|m| m.symbol.clone()).collect();
    sample_tickers.sort();

    let mut pf_members: Vec<&SymbolSeries> = members.to_vec();
    pf_members.sort_by(|a, b| {
        b.median_vol20
            .partial_cmp(&a.median_vol20)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    pf_members.truncate(45);

    let feeds: Vec<WatchlistFeedEntry> = if run_portfolio && pf_members.len() >= 2 {
        pf_members
            .iter()
            .map(|m| WatchlistFeedEntry {
                symbol: m.symbol.clone(),
                instrument_id: m.ticker.clone(),
                preset: DataSourcePreset::Vn30Basket,
                base_timeframe: Timeframe::D1,
                candles: m.candles.clone(),
            })
            .collect()
    } else {
        Vec::new()
    };

    let mut strategies = Vec::with_capacity(EVAL_PRESETS.len());
    for (p_idx, &(code, preset)) in EVAL_PRESETS.iter().enumerate() {
        let mut sym_evals = Vec::with_capacity(members.len());
        for m in members {
            if let Some(ev_list) = evals_by_ticker.get(&m.ticker) {
                sym_evals.push(&ev_list[p_idx]);
            }
        }
        let n_sym = sym_evals.len().max(1);
        let total_oos_trades: usize = sym_evals.iter().map(|e| e.wfo_oos_trades).sum();

        let mut wins = 0usize;
        let mut gross_win_pct = 0.0;
        let mut gross_loss_pct = 0.0;
        let mut variant_counts: BTreeMap<String, usize> = BTreeMap::new();

        for e in &sym_evals {
            *variant_counts.entry(e.last_winning_label.clone()).or_insert(0) += 1;
            for tr in &e.oos_trade_records {
                if tr.return_pct > 0.0 {
                    wins += 1;
                    gross_win_pct += tr.return_pct;
                } else if tr.return_pct < 0.0 {
                    gross_loss_pct += tr.return_pct.abs();
                }
            }
        }

        let dominant_winning_variant = variant_counts
            .into_iter()
            .max_by_key(|(_, c)| *c)
            .map(|(k, _)| k)
            .unwrap_or_else(|| "Default".to_owned());

        let pooled_win_rate = if total_oos_trades > 0 {
            (wins as f64 / total_oos_trades as f64) * 100.0
        } else {
            0.0
        };
        let pooled_pf = if gross_loss_pct > 1e-6 {
            (gross_win_pct / gross_loss_pct).min(15.0)
        } else if gross_win_pct > 0.0 {
            5.0
        } else {
            0.0
        };
        let pooled_avg_ret = if total_oos_trades > 0 {
            (gross_win_pct - gross_loss_pct) / total_oos_trades as f64
        } else {
            0.0
        };

        let mean_is_ret = mean_f64(&sym_evals.iter().map(|e| e.is_return_pct).collect::<Vec<_>>());
        let mean_is_sh = mean_f64(&sym_evals.iter().map(|e| e.is_sharpe).collect::<Vec<_>>());
        let oos_rets: Vec<f64> = sym_evals.iter().map(|e| e.wfo_oos_return_pct).collect();
        let mean_oos_ret = mean_f64(&oos_rets);
        let med_oos_ret = median_f64(oos_rets);
        let mean_oos_cagr = mean_f64(
            &sym_evals
                .iter()
                .map(|e| e.wfo_oos_cagr_pct)
                .collect::<Vec<_>>(),
        );
        let mean_oos_dd = mean_f64(
            &sym_evals
                .iter()
                .map(|e| e.wfo_oos_max_dd_pct)
                .collect::<Vec<_>>(),
        );
        let mean_oos_sh = mean_f64(
            &sym_evals
                .iter()
                .map(|e| e.wfo_oos_sharpe)
                .collect::<Vec<_>>(),
        );
        let mean_oos_so = mean_f64(
            &sym_evals
                .iter()
                .map(|e| e.wfo_oos_sortino)
                .collect::<Vec<_>>(),
        );
        let mean_wfe = mean_f64(&sym_evals.iter().map(|e| e.wfo_wfe_pct).collect::<Vec<_>>());

        let pct_prof = (sym_evals
            .iter()
            .filter(|e| e.wfo_oos_return_pct > 0.0)
            .count() as f64
            / n_sym as f64)
            * 100.0;
        let pct_rob =
            (sym_evals.iter().filter(|e| e.wfo_robust).count() as f64 / n_sym as f64) * 100.0;

        let (mut pf_eq_ret, mut pf_eq_cagr, mut pf_eq_dd, mut pf_eq_sh) = (None, None, None, None);
        let (mut pf_iv_ret, mut pf_iv_cagr, mut pf_iv_dd, mut pf_iv_sh) = (None, None, None, None);
        let mut mc_prob = None;
        let mut mc_ruin = None;
        let mut mc_p50 = None;
        let mut mc_var95 = None;
        let mut mc_cvar95 = None;

        if feeds.len() >= 2 {
            let mut cfg = build_qmv_base_config(preset, Some(non_downtrend_days.clone()));
            cfg.initial_cash = 1_000_000_000.0;
            let rep_eq = run_portfolio_backtest(
                &feeds,
                Timeframe::D1,
                &cfg,
                PortfolioUniverseFilter::AllUniverseFeeds,
                PortfolioWeightingMode::EqualWeight,
                None,
            );
            pf_eq_ret = Some(rep_eq.portfolio_tearsheet.total_return_pct);
            pf_eq_cagr = rep_eq.portfolio_tearsheet.cagr_pct;
            pf_eq_dd = Some(rep_eq.portfolio_tearsheet.max_drawdown_pct);
            pf_eq_sh = rep_eq.portfolio_tearsheet.sharpe_ratio;

            let rep_iv = run_portfolio_backtest(
                &feeds,
                Timeframe::D1,
                &cfg,
                PortfolioUniverseFilter::AllUniverseFeeds,
                PortfolioWeightingMode::CausalInverseVolatility { lookback_bars: 20 },
                None,
            );
            pf_iv_ret = Some(rep_iv.portfolio_tearsheet.total_return_pct);
            pf_iv_cagr = rep_iv.portfolio_tearsheet.cagr_pct;
            pf_iv_dd = Some(rep_iv.portfolio_tearsheet.max_drawdown_pct);
            pf_iv_sh = rep_iv.portfolio_tearsheet.sharpe_ratio;

            let curve_len = rep_iv.portfolio_equity_curve.len();
            let oos_curve_start = ((curve_len as f64) * 0.70) as usize;
            let oos_curve = if curve_len > oos_curve_start + 30 {
                &rep_iv.portfolio_equity_curve[oos_curve_start..]
            } else {
                &rep_iv.portfolio_equity_curve[..]
            };
            if oos_curve.len() >= 25 {
                let mc_init = oos_curve[0].equity.max(1.0);
                let mc_cfg = MonteCarloConfig {
                    simulations: 2_000,
                    seed: DEFAULT_MONTE_CARLO_SEED ^ (p_idx as u64 * 0x9E37_79B9),
                    ruin_drawdown_pct: 35.0,
                    horizon_steps: Some(250.min(oos_curve.len())),
                };
                let mc = run_monte_carlo_from_equity_curve(
                    mc_init,
                    oos_curve,
                    oos_curve.len(),
                    None,
                    mc_cfg,
                );
                if let Some(m) = mc.metrics {
                    mc_prob = Some(m.prob_of_profit_pct);
                    mc_ruin = Some(m.prob_of_ruin_pct);
                    mc_p50 = Some(m.terminal_return_pct.p50);
                    mc_var95 = Some(m.max_drawdown_var_95_pct);
                    mc_cvar95 = Some(m.max_drawdown_cvar_95_pct);
                }
            }
        }

        let low_sample = total_oos_trades < 30;
        let sample_penalty = if low_sample { -25.0 } else { 0.0 };
        let pf_sh_bonus = pf_iv_sh.or(pf_eq_sh).unwrap_or(mean_oos_sh) * 18.0;
        let composite_rank_score = mean_oos_sh * 28.0
            + pf_sh_bonus
            + (pooled_pf - 1.0).clamp(-1.0, 3.0) * 16.0
            + (pct_prof - 50.0) * 0.45
            + (med_oos_ret * 0.22)
            - (mean_oos_dd * 0.40)
            + sample_penalty;

        strategies.push(StrategyGroupSummary {
            preset_code: code.to_owned(),
            preset_label: preset.full_label().to_owned(),
            symbol_count: members.len(),
            total_wfo_oos_trades: total_oos_trades,
            low_sample_warning: low_sample,
            mean_is_return_pct: mean_is_ret,
            mean_is_sharpe: mean_is_sh,
            mean_wfo_oos_return_pct: mean_oos_ret,
            median_wfo_oos_return_pct: med_oos_ret,
            mean_wfo_oos_cagr_pct: mean_oos_cagr,
            mean_wfo_oos_max_dd_pct: mean_oos_dd,
            mean_wfo_oos_sharpe: mean_oos_sh,
            mean_wfo_oos_sortino: mean_oos_so,
            pooled_oos_win_rate_pct: pooled_win_rate,
            pooled_oos_profit_factor: pooled_pf,
            pooled_avg_trade_return_pct: pooled_avg_ret,
            mean_wfe_pct: mean_wfe,
            pct_symbols_oos_profitable: pct_prof,
            pct_symbols_wfo_robust: pct_rob,
            dominant_winning_variant,
            portfolio_eq_return_pct: pf_eq_ret,
            portfolio_eq_cagr_pct: pf_eq_cagr,
            portfolio_eq_max_dd_pct: pf_eq_dd,
            portfolio_eq_sharpe: pf_eq_sh,
            portfolio_invvol_return_pct: pf_iv_ret,
            portfolio_invvol_cagr_pct: pf_iv_cagr,
            portfolio_invvol_max_dd_pct: pf_iv_dd,
            portfolio_invvol_sharpe: pf_iv_sh,
            mc_prob_profit_pct: mc_prob,
            mc_prob_ruin_35_pct: mc_ruin,
            mc_p50_return_pct: mc_p50,
            mc_var95_dd_pct: mc_var95,
            mc_cvar95_dd_pct: mc_cvar95,
            composite_rank_score,
        });
    }

    strategies.sort_by(|a, b| {
        b.composite_rank_score
            .partial_cmp(&a.composite_rank_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let winner = strategies.first().cloned().expect("at least 1 strategy");
    GroupReport {
        group_id: group_id.to_owned(),
        dimension: dimension.to_owned(),
        group_label: group_label.to_owned(),
        symbol_count: members.len(),
        sample_tickers,
        winning_preset_code: winner.preset_code,
        winning_preset_label: winner.preset_label,
        winning_variant: winner.dominant_winning_variant,
        strategies,
    }
}

fn run_single_ablation(
    experiment: &str,
    variant_label: &str,
    group_label: &str,
    members: &[&SymbolSeries],
    cfg: &BacktestConfig,
) -> AblationRow {
    let mut rets = Vec::with_capacity(members.len());
    let mut cagrs = Vec::with_capacity(members.len());
    let mut dds = Vec::with_capacity(members.len());
    let mut sharpes = Vec::with_capacity(members.len());
    let mut tot_trades = 0usize;
    let mut wins = 0usize;
    let mut g_win = 0.0;
    let mut g_loss = 0.0;

    for m in members {
        let bt = run_backtest_with_options(
            &m.ticker,
            Timeframe::D1,
            &m.candles,
            cfg,
            BacktestRunOptions {
                replay_cutoff_timestamp: None,
                incomplete_tail_bar: false,
            },
        );
        let oos = bt.out_of_sample_tearsheet.as_ref().unwrap_or(&bt.tearsheet);
        rets.push(oos.total_return_pct);
        cagrs.push(oos.cagr_pct.unwrap_or(0.0));
        dds.push(oos.max_drawdown_pct);
        sharpes.push(oos.sharpe_ratio.unwrap_or(0.0));

        let cut = (m.candles.len() as f64 * 0.70) as usize;
        for tr in bt.trades.iter().filter(|t| t.entry_bar_index >= cut) {
            tot_trades += 1;
            if tr.return_pct > 0.0 {
                wins += 1;
                g_win += tr.return_pct;
            } else if tr.return_pct < 0.0 {
                g_loss += tr.return_pct.abs();
            }
        }
    }

    let pf = if g_loss > 1e-6 {
        (g_win / g_loss).min(15.0)
    } else if g_win > 0.0 {
        5.0
    } else {
        0.0
    };

    AblationRow {
        experiment: experiment.to_owned(),
        variant_label: variant_label.to_owned(),
        group_label: group_label.to_owned(),
        symbol_count: members.len(),
        total_trades: tot_trades,
        mean_return_pct: mean_f64(&rets),
        median_return_pct: median_f64(rets),
        mean_cagr_pct: mean_f64(&cagrs),
        mean_max_dd_pct: mean_f64(&dds),
        mean_sharpe: mean_f64(&sharpes),
        win_rate_pct: if tot_trades > 0 {
            (wins as f64 / tot_trades as f64) * 100.0
        } else {
            0.0
        },
        profit_factor: pf,
    }
}

fn main() {
    let started = Instant::now();
    let home = std::env::var("HOME").expect("HOME env");
    let db_path = PathBuf::from(&home)
        .join("Library/Application Support/com.sumi.supercharts/sumi_market.db");
    let out_json_path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp/qmv_backtest_results.json"));

    println!("=== QMV STAGE 4 AUTOMATED MULTI-GROUP BACKTEST MATRIX ===");
    println!("Database: {}", db_path.display());

    let (series_list, vnindex_candles, total_vn_symbols, total_bars) = load_all_vn_series(&db_path);
    let (non_downtrend_days, vnindex_day_regimes, (bull_days, side_days, bear_days)) =
        classify_vnindex_day_regimes(&vnindex_candles);

    println!(
        "Loaded {} WFO-eligible VN equity symbols (from {} total VN symbols, {} 1D bars; VNINDEX={} bars [Bull={}, Side={}, Bear={}]).",
        series_list.len(),
        total_vn_symbols,
        total_bars,
        vnindex_candles.len(),
        bull_days,
        side_days,
        bear_days
    );

    let n_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(8)
        .clamp(2, 16);
    let chunk_size = series_list.len().div_ceil(n_threads);

    let mut evals_by_ticker: BTreeMap<String, Vec<SymbolStrategyEval>> = BTreeMap::new();
    std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for chunk in series_list.chunks(chunk_size) {
            let nd = non_downtrend_days.clone();
            let reg_map = &vnindex_day_regimes;
            handles.push(scope.spawn(move || {
                let mut local = Vec::with_capacity(chunk.len());
                for s in chunk {
                    let ev = evaluate_symbol_all_strategies(s, &nd, reg_map);
                    local.push((s.ticker.clone(), ev));
                }
                local
            }));
        }
        for h in handles {
            for (tk, ev) in h.join().expect("worker thread") {
                evals_by_ticker.insert(tk, ev);
            }
        }
    });

    println!(
        "Completed 5-Fold WFO ({} symbols × {} strategies = {} WFO pipelines) in {:.2}s.",
        series_list.len(),
        EVAL_PRESETS.len(),
        series_list.len() * EVAL_PRESETS.len(),
        started.elapsed().as_secs_f64()
    );

    let mut total_limit_locked_skips = 0usize;
    let mut total_illiquid_skips = 0usize;
    for evs in evals_by_ticker.values() {
        for e in evs {
            total_limit_locked_skips += e.limit_locked_skips;
            total_illiquid_skips += e.illiquid_skips;
        }
    }

    let all_refs: Vec<&SymbolSeries> = series_list.iter().collect();
    let overall_universe = summarize_group(
        "ALL_VN",
        "UNIVERSE",
        "Toàn vũ trụ Cổ phiếu VN đủ lịch sử (≥260 phiên)",
        &all_refs,
        &evals_by_ticker,
        &non_downtrend_days,
        true,
    );

    let stock_group_specs: Vec<(&str, &str, GroupPred)> = vec![
        (
            "QMV30",
            "Rổ QMV30 — 30 Cổ phiếu Trụ cột",
            Box::new(|s| s.profile.in_qmv30),
        ),
        (
            "QMV60",
            "Rổ QMV60 — 60 Cổ phiếu Trọng điểm",
            Box::new(|s| s.profile.in_qmv60),
        ),
        (
            "YUMEKO_CORE10",
            "Danh mục Tham chiếu Yumeko Core 10",
            Box::new(|s| s.profile.in_yumeko_core10),
        ),
        (
            "DT1",
            "Dòng tiền Loại 1 (DT1 · Tạo lập / Large-cap)",
            Box::new(|s| s.profile.money_tier == QmvMoneyTier::Dt1),
        ),
        (
            "DT2",
            "Dòng tiền Loại 2 (DT2 · Mid-cap Cơ bản)",
            Box::new(|s| s.profile.money_tier == QmvMoneyTier::Dt2),
        ),
        (
            "DT3",
            "Dòng tiền Loại 3 (DT3 · Small-cap / Đầu cơ)",
            Box::new(|s| s.profile.money_tier == QmvMoneyTier::Dt3),
        ),
        (
            "PRICE_50K_PLUS",
            "Phân tầng Thị giá Cao (≥ 50,000đ)",
            Box::new(|s| s.price_tier == QmvPriceTier::High50kPlus),
        ),
        (
            "PRICE_10K_50K",
            "Phân tầng Thị giá Vừa (10,000 – 50,000đ)",
            Box::new(|s| {
                matches!(
                    s.price_tier,
                    QmvPriceTier::Low10To30k | QmvPriceTier::Mid30To50k
                )
            }),
        ),
        (
            "PRICE_PENNY_UNDER_10K",
            "Cổ phiếu Trà đá (< 10,000đ)",
            Box::new(|s| s.price_tier == QmvPriceTier::PennyUnder10k),
        ),
    ];

    let mut stock_type_and_money_groups = Vec::new();
    for (gid, glabel, pred) in &stock_group_specs {
        let members: Vec<&SymbolSeries> = series_list.iter().filter(|s| pred(s)).collect();
        if !members.is_empty() {
            stock_type_and_money_groups.push(summarize_group(
                gid,
                "STOCK_TYPE_AND_MONEY_FLOW",
                glabel,
                &members,
                &evals_by_ticker,
                &non_downtrend_days,
                true,
            ));
        }
    }

    let mut sector_groups_and_sectors = Vec::new();
    for grp in [
        QmvSectorGroup::Group1FinancialLeverage,
        QmvSectorGroup::Group2OperatingCycle,
        QmvSectorGroup::Group3Defensive,
    ] {
        let members: Vec<&SymbolSeries> = series_list
            .iter()
            .filter(|s| s.profile.sector_group == grp)
            .collect();
        if !members.is_empty() {
            sector_groups_and_sectors.push(summarize_group(
                grp.code(),
                "SECTOR_GROUP",
                grp.label_vi(),
                &members,
                &evals_by_ticker,
                &non_downtrend_days,
                true,
            ));
        }
    }
    for sec in QmvSector::ALL_EQUITY {
        if sec == QmvSector::OtherCyclical {
            continue;
        }
        let members: Vec<&SymbolSeries> = series_list
            .iter()
            .filter(|s| s.profile.sector == sec)
            .collect();
        if !members.is_empty() {
            sector_groups_and_sectors.push(summarize_group(
                sec.code(),
                "SECTOR",
                sec.label_vi(),
                &members,
                &evals_by_ticker,
                &non_downtrend_days,
                true,
            ));
        }
    }

    let liq_specs = [
        (
            "LIQ_HIGH_1M_PLUS",
            "Thanh khoản Lớn (ADTV20 ≥ 1,000,000 cp/phiên)",
            QmvLiquidityTier::High1MPlus,
        ),
        (
            "LIQ_STD_50K_1M",
            "Thanh khoản Vừa chuẩn QMV V7 (50,000 – 1,000,000 cp/phiên)",
            QmvLiquidityTier::Standard50kTo1M,
        ),
        (
            "LIQ_SMALL_10K_50K",
            "Thanh khoản Nhỏ (10,000 – 50,000 cp/phiên)",
            QmvLiquidityTier::Small10kTo50k,
        ),
    ];
    let mut liquidity_groups = Vec::new();
    for (gid, glabel, tier) in liq_specs {
        let members: Vec<&SymbolSeries> =
            series_list.iter().filter(|s| s.liq_tier == tier).collect();
        if !members.is_empty() {
            liquidity_groups.push(summarize_group(
                gid,
                "LIQUIDITY_TIER",
                glabel,
                &members,
                &evals_by_ticker,
                &non_downtrend_days,
                true,
            ));
        }
    }

    let mut vnindex_regime_matrix = Vec::new();
    for (p_idx, &(code, preset)) in EVAL_PRESETS.iter().enumerate() {
        for regime_name in ["BULL", "SIDEWAYS", "BEAR"] {
            let mut g_cnt = 0usize;
            let mut g_wins = 0usize;
            let mut g_pos = 0.0;
            let mut g_neg = 0.0;

            let mut u_cnt = 0usize;
            let mut u_wins = 0usize;
            let mut u_pos = 0.0;
            let mut u_neg = 0.0;
            let mut u_mae_sum = 0.0;

            for evs in evals_by_ticker.values() {
                let e = &evs[p_idx];
                for tr in e
                    .oos_trade_records
                    .iter()
                    .filter(|t| t.vnindex_regime == regime_name)
                {
                    g_cnt += 1;
                    if tr.return_pct > 0.0 {
                        g_wins += 1;
                        g_pos += tr.return_pct;
                    } else if tr.return_pct < 0.0 {
                        g_neg += tr.return_pct.abs();
                    }
                }
                for tr in e
                    .ungated_trade_records
                    .iter()
                    .filter(|t| t.vnindex_regime == regime_name)
                {
                    u_cnt += 1;
                    u_mae_sum += tr.mae_pct;
                    if tr.return_pct > 0.0 {
                        u_wins += 1;
                        u_pos += tr.return_pct;
                    } else if tr.return_pct < 0.0 {
                        u_neg += tr.return_pct.abs();
                    }
                }
            }

            vnindex_regime_matrix.push(RegimeSliceSummary {
                preset_code: code.to_owned(),
                preset_label: preset.full_label().to_owned(),
                regime: regime_name.to_owned(),
                gated_trades: g_cnt,
                gated_win_rate_pct: if g_cnt > 0 {
                    (g_wins as f64 / g_cnt as f64) * 100.0
                } else {
                    0.0
                },
                gated_avg_return_pct: if g_cnt > 0 {
                    (g_pos - g_neg) / g_cnt as f64
                } else {
                    0.0
                },
                gated_profit_factor: if g_neg > 1e-6 {
                    (g_pos / g_neg).min(15.0)
                } else if g_pos > 0.0 {
                    5.0
                } else {
                    0.0
                },
                ungated_trades: u_cnt,
                ungated_win_rate_pct: if u_cnt > 0 {
                    (u_wins as f64 / u_cnt as f64) * 100.0
                } else {
                    0.0
                },
                ungated_avg_return_pct: if u_cnt > 0 {
                    (u_pos - u_neg) / u_cnt as f64
                } else {
                    0.0
                },
                ungated_profit_factor: if u_neg > 1e-6 {
                    (u_pos / u_neg).min(15.0)
                } else if u_pos > 0.0 {
                    5.0
                } else {
                    0.0
                },
                ungated_avg_mae_pct: if u_cnt > 0 {
                    u_mae_sum / u_cnt as f64
                } else {
                    0.0
                },
            });
        }
    }

    let mut ablation_experiments = Vec::new();
    let qmv60_members: Vec<&SymbolSeries> =
        series_list.iter().filter(|s| s.profile.in_qmv60).collect();

    for (variant_id, label) in [
        (0u8, "Ichimoku Chuẩn QMV V7 (6-18-36, dịch 18)"),
        (1u8, "Ichimoku Ngắn hạn QMV (3-6-18, dịch 18)"),
        (2u8, "Ichimoku Cổ điển Nhật (9-26-52, dịch 26)"),
    ] {
        let mut cfg = build_qmv_base_config(
            StrategyPresetKind::QmvIchimokuV7,
            Some(non_downtrend_days.clone()),
        );
        cfg.params.qmv_ichi_variant = variant_id;
        ablation_experiments.push(run_single_ablation(
            "ABLATION_ICHIMOKU_PARAMS",
            label,
            "Rổ QMV60 (OOS 30%)",
            &qmv60_members,
            &cfg,
        ));
    }

    let dt1_m: Vec<&SymbolSeries> = series_list
        .iter()
        .filter(|s| s.profile.money_tier == QmvMoneyTier::Dt1)
        .collect();
    let dt2_m: Vec<&SymbolSeries> = series_list
        .iter()
        .filter(|s| s.profile.money_tier == QmvMoneyTier::Dt2)
        .collect();
    let dt3_m: Vec<&SymbolSeries> = series_list
        .iter()
        .filter(|s| s.profile.money_tier == QmvMoneyTier::Dt3)
        .collect();
    let penny_m: Vec<&SymbolSeries> = series_list
        .iter()
        .filter(|s| s.price_tier == QmvPriceTier::PennyUnder10k)
        .collect();
    let s2_cfg = build_qmv_base_config(
        StrategyPresetKind::QmvSafeBottomDiv,
        Some(non_downtrend_days.clone()),
    );
    for (glabel, mem) in [
        ("DT1 — Cổ phiếu Tạo lập / Dẫn dắt", &dt1_m),
        ("DT2 — Cổ phiếu Mid-cap Cơ bản", &dt2_m),
        ("DT3 — Cổ phiếu Đầu cơ / Nhỏ", &dt3_m),
        ("Trà đá (< 10,000đ)", &penny_m),
    ] {
        ablation_experiments.push(run_single_ablation(
            "ABLATION_S2_BOTTOM_FISHING_BY_TIER",
            "QMV S2 — Bắt Đáy Phân Kỳ",
            glabel,
            mem,
            &s2_cfg,
        ));
    }

    {
        let full_qmv_cfg = build_qmv_base_config(
            StrategyPresetKind::QmvMultiGateConfluence,
            Some(non_downtrend_days.clone()),
        );
        ablation_experiments.push(run_single_ablation(
            "ABLATION_QMV_RISK_DISCIPLINE",
            "Đầy đủ Kỷ luật QMV (Cổng VNINDEX + SL Đáy 20p + Giữ 2/3 sóng)",
            "Rổ QMV60 (OOS 30%)",
            &qmv60_members,
            &full_qmv_cfg,
        ));

        let mut fixed_sl_cfg = full_qmv_cfg.clone();
        fixed_sl_cfg.risk.support_stop_lookback = None;
        fixed_sl_cfg.risk.qmv_wave_trailing_2_3 = false;
        fixed_sl_cfg.risk.stop_loss_pct = Some(6.0);
        fixed_sl_cfg.risk.take_profit_pct = Some(15.0);
        ablation_experiments.push(run_single_ablation(
            "ABLATION_QMV_RISK_DISCIPLINE",
            "SL Cố định 6% + TP 15% (Tắt SL Đáy & Tắt Giữ 2/3 sóng)",
            "Rổ QMV60 (OOS 30%)",
            &qmv60_members,
            &fixed_sl_cfg,
        ));

        let ungated_s6_cfg =
            build_qmv_base_config(StrategyPresetKind::QmvMultiGateConfluence, None);
        ablation_experiments.push(run_single_ablation(
            "ABLATION_QMV_RISK_DISCIPLINE",
            "Tắt Rào chắn HOSE:VNINDEX (Mua cả trong Downtrend)",
            "Rổ QMV60 (OOS 30%)",
            &qmv60_members,
            &ungated_s6_cfg,
        ));
    }

    let elapsed_secs = started.elapsed().as_secs_f64();

    println!("\n--- WINNING QMV PLAYBOOK PER STOCK TYPE / MONEY TIER ---");
    for g in &stock_type_and_money_groups {
        let top = &g.strategies[0];
        println!(
            "{:<42} | n={:<3} | Winner: {:<8} ({}) | OOS Mean={:+.1}% Med={:+.1}% Sharpe={:.2} PF={:.2} Win={:.1}% MC_Prob={:.1}%",
            g.group_label,
            g.symbol_count,
            g.winning_preset_code,
            g.winning_variant,
            top.mean_wfo_oos_return_pct,
            top.median_wfo_oos_return_pct,
            top.mean_wfo_oos_sharpe,
            top.pooled_oos_profit_factor,
            top.pooled_oos_win_rate_pct,
            top.mc_prob_profit_pct.unwrap_or(0.0),
        );
    }

    println!("\n--- WINNING QMV PLAYBOOK PER SECTOR GROUP & SECTOR ---");
    for g in &sector_groups_and_sectors {
        let top = &g.strategies[0];
        println!(
            "{:<42} | n={:<3} | Winner: {:<8} ({}) | OOS Mean={:+.1}% PF_IV={:+.1}% Sharpe={:.2} PF={:.2} Trades={}{}",
            g.group_label,
            g.symbol_count,
            g.winning_preset_code,
            g.winning_variant,
            top.mean_wfo_oos_return_pct,
            top.portfolio_invvol_return_pct.unwrap_or(0.0),
            top.mean_wfo_oos_sharpe,
            top.pooled_oos_profit_factor,
            top.total_wfo_oos_trades,
            if top.low_sample_warning {
                " [LOW-SAMPLE]"
            } else {
                ""
            },
        );
    }

    println!("\n--- ABLATION EXPERIMENTS ---");
    for ab in &ablation_experiments {
        println!(
            "[{}] {:<56} | {:<32} | n={:<3} trades={:<4} | OOS Mean={:+.2}% Med={:+.2}% DD=-{:.2}% Sharpe={:.2} PF={:.2}",
            ab.experiment,
            ab.variant_label,
            ab.group_label,
            ab.symbol_count,
            ab.total_trades,
            ab.mean_return_pct,
            ab.median_return_pct,
            ab.mean_max_dd_pct,
            ab.mean_sharpe,
            ab.profit_factor
        );
    }

    let output = FullMatrixOutput {
        generated_at: chrono::Utc::now().to_rfc3339(),
        db_path: db_path.display().to_string(),
        vn_total_1d_symbols: total_vn_symbols,
        wfo_eligible_symbols: series_list.len(),
        total_1d_bars_loaded: total_bars,
        vnindex_bars: vnindex_candles.len(),
        vnindex_bull_days: bull_days,
        vnindex_sideways_days: side_days,
        vnindex_bear_days: bear_days,
        total_limit_locked_skips,
        total_illiquid_skips,
        elapsed_secs,
        overall_universe,
        stock_type_and_money_groups,
        sector_groups_and_sectors,
        liquidity_groups,
        vnindex_regime_matrix,
        ablation_experiments,
    };

    if let Some(parent) = out_json_path.parent() {
        std::fs::create_dir_all(parent).expect("create output dir");
    }
    let json_bytes = serde_json::to_vec_pretty(&output).expect("serialize matrix JSON");
    std::fs::write(&out_json_path, json_bytes).expect("write matrix JSON");
    println!(
        "\nSaved full empirical JSON report to {} (total {:.2}s).",
        out_json_path.display(),
        elapsed_secs
    );
}
