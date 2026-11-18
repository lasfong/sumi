//! Multi-Strategy `0%` Equity Comparison Overlay & Institutional Advanced Tearsheet Analytics
//! (`OutlierImpact`, `DrawdownPeriod`s, UTC `Year` & Causal Entry-Signal `MarketRegime` Breakdowns,
//! `MultiPhaseBatch` Degradation Matrix, and Deterministic `SplitMix64` `RunManifest` — `Milestone R17 / Wave 16`).

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::data::{
    DataSourcePreset, Timeframe, WatchlistFeedEntry, decompose_utc,
    resolve_benchmark_series_for_window,
};
use crate::indicators::SeriesBlockIndex;
use crate::journal::ExchangeVenue;
use crate::model::Candle;
use crate::scanner::{MarketRegime, PrecomputedCatalogMatrix, compute_regime_series};
use crate::viewport::workspace_template::{escape_str_into, fmt_f64, splitmix64_hash_bytes};

use super::engine::{
    BacktestRunOptions, BacktestRunResult, PrecomputedFeatures, run_backtest_with_options,
    simulate_window_with_features, slice_causal_series_for_symbol_and_options,
};
use super::metrics::{
    BacktestEquityPoint, BacktestTearsheet, BacktestTrade, PNL_BREAKEVEN_EPS, compute_tearsheet,
};
use super::strategy::{BacktestConfig, StrategyDirectionMode, StrategyPresetKind};

pub const RUN_MANIFEST_SCHEMA_VERSION: u32 = 1;

// ============================================================================
// 1. Multi-Strategy `0%` Rebased Equity Comparison Overlay (`A4`)
// ============================================================================

/// Kind of curve rendered inside [`MultiStrategyComparisonReport`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComparisonCurveKind {
    StrategyPreset(StrategyPresetKind),
    AssetBuyAndHold,
    MarketBenchmark,
}

/// One UTC-timestamp-aligned `0.00%`-rebased cumulative return curve (`len == chart_bar_count`)
/// indexed with [`SeriesBlockIndex`] for $O(1)$ viewport autoscale and conflated rendering.
#[derive(Debug, Clone)]
pub struct MultiStrategyCurveSeries {
    pub kind: ComparisonCurveKind,
    pub label: String,
    pub is_active_preset: bool,
    pub return_pct_values: Vec<Option<f64>>,
    pub block_index: SeriesBlockIndex,
    pub final_return_pct: Option<f64>,
    pub max_drawdown_pct: f64,
    pub num_trades: usize,
    pub sharpe_ratio: Option<f64>,
}

/// Complete multi-strategy comparison across all `7` [`StrategyPresetKind::ALL`] presets,
/// `Asset Buy & Hold`, and causal `Benchmark` (`VNINDEX` / `SPY`) aligned strictly by bar-close
/// UTC timestamps (`Milestone R17 / A4` — rectifies `MultiStrategyEquityChart.tsx:94-99`).
#[derive(Debug, Clone)]
pub struct MultiStrategyComparisonReport {
    pub symbol: String,
    pub benchmark_label: Option<String>,
    pub timeframe: Timeframe,
    pub chart_bar_count: usize,
    pub curves: Vec<MultiStrategyCurveSeries>,
}

impl MultiStrategyComparisonReport {
    /// Computes combined `(min_pct, max_pct)` over `range` in $O(\text{curves} \times \text{blocks})$ time.
    #[must_use]
    pub fn range_bounds(&self, range: std::ops::Range<usize>) -> Option<(f64, f64)> {
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for c in &self.curves {
            let total = c.return_pct_values.len();
            let clamped = range.start.min(total)..range.end.min(total);
            if let Some((c_lo, c_hi)) = c.block_index.range_bounds(&c.return_pct_values, clamped) {
                if c_lo.is_finite() {
                    lo = lo.min(c_lo);
                }
                if c_hi.is_finite() {
                    hi = hi.max(c_hi);
                }
            }
        }
        (lo.is_finite() && hi.is_finite() && lo <= hi).then_some((lo, hi))
    }
}

#[inline]
fn compute_chart_bar_end_timestamps(
    chart_candles: &[Candle],
    chart_timeframe: Timeframe,
    replay_causal_close_timestamp: Option<i64>,
) -> Vec<i64> {
    let n = chart_candles.len();
    let bar_span = chart_timeframe.seconds().max(1).saturating_sub(1);
    let mut out = Vec::with_capacity(n);
    for (i, c) in chart_candles.iter().enumerate() {
        let mut end_ts = c.timestamp.saturating_add(bar_span);
        if let Some(next_c) = chart_candles.get(i + 1)
            && next_c.timestamp > c.timestamp
        {
            end_ts = end_ts.min(next_c.timestamp.saturating_sub(1));
        }
        if let Some(cut) = replay_causal_close_timestamp {
            end_ts = end_ts.min(cut);
        }
        out.push(end_ts);
    }
    out
}

/// Executes all `7` [`StrategyPresetKind::ALL`] presets + `Asset Buy & Hold` + `Benchmark`
/// (`VNINDEX` for Vietnam venues, `SPY` otherwise) and projects every curve onto `chart_candles`
/// strictly by **causal bar-close UTC timestamp** normalized to `0.00%` (`Milestone R17 / A4`).
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn run_multi_strategy_comparison(
    symbol: impl Into<String>,
    data_source: DataSourcePreset,
    timeframe: Timeframe,
    chart_candles: &[Candle],
    base_config: &BacktestConfig,
    universe: &[WatchlistFeedEntry],
    replay_causal_close_timestamp: Option<i64>,
    incomplete_tail_bar: bool,
) -> MultiStrategyComparisonReport {
    let symbol = symbol.into();
    let n = chart_candles.len();
    if n == 0 {
        return MultiStrategyComparisonReport {
            symbol,
            benchmark_label: None,
            timeframe,
            chart_bar_count: 0,
            curves: Vec::new(),
        };
    }

    let mut clean_base = base_config.clone();
    clean_base.sanitize();
    let active_preset = clean_base.preset;

    // Causal candle prefix gated identically to `run_backtest_with_options` (`[CRITIC-R17-CRITICAL-01]`:
    // withholds unclosed mid-bar coarse tails when `incomplete_tail_bar == false` and recognizes exchange session closes).
    let causal_candles = slice_causal_series_for_symbol_and_options(
        Some(&symbol),
        chart_candles,
        timeframe,
        BacktestRunOptions {
            replay_cutoff_timestamp: replay_causal_close_timestamp,
            incomplete_tail_bar,
        },
    );
    let causal_end = causal_candles.len();
    let chart_end_ts =
        compute_chart_bar_end_timestamps(chart_candles, timeframe, replay_causal_close_timestamp);
    let causal_end_ts =
        compute_chart_bar_end_timestamps(causal_candles, timeframe, replay_causal_close_timestamp);

    let shared_catalog = if !causal_candles.is_empty() {
        Some(Arc::new(PrecomputedCatalogMatrix::compute(causal_candles)))
    } else {
        None
    };

    let mut curves = Vec::with_capacity(StrategyPresetKind::ALL.len() + 2);
    let init_cash = clean_base.initial_cash.max(1.0);

    // 1. All 7 `StrategyPresetKind::ALL` presets projected by causal bar-close UTC timestamp.
    for preset in StrategyPresetKind::ALL {
        let mut cfg = clean_base.clone();
        cfg.preset = preset;
        cfg.sanitize();

        let (stamped_pct, max_dd, num_trades, sharpe) = if causal_candles.is_empty() {
            (Vec::new(), 0.0, 0, None)
        } else {
            let feats = PrecomputedFeatures::build_with_shared_catalog(
                causal_candles,
                &cfg,
                if preset == StrategyPresetKind::CustomSignalRules {
                    shared_catalog.clone()
                } else {
                    None
                },
            );
            let sim = simulate_window_with_features(
                causal_candles,
                0,
                causal_candles.len(),
                timeframe,
                &cfg,
                &feats,
                incomplete_tail_bar,
            );
            let open_fee = sim.open_position.as_ref().map_or(0.0, |p| p.entry_fee);
            let ts = compute_tearsheet(
                cfg.initial_cash,
                sim.final_cash,
                timeframe,
                &sim.equity_curve,
                &sim.trades,
                open_fee,
            );
            let pts: Vec<(i64, f64)> = sim
                .equity_curve
                .iter()
                .enumerate()
                .map(|(k, pt)| {
                    let close_ts = causal_end_ts.get(k).copied().unwrap_or(pt.timestamp);
                    let pct = ((pt.equity / init_cash) - 1.0) * 100.0;
                    (close_ts, if pct.is_finite() { pct } else { 0.0 })
                })
                .collect();
            (pts, ts.max_drawdown_pct, ts.num_trades, ts.sharpe_ratio)
        };

        let mut vals = Vec::with_capacity(n);
        for (i, &bar_end) in chart_end_ts.iter().enumerate() {
            if i >= causal_end || stamped_pct.is_empty() {
                vals.push(None);
            } else {
                let pos = stamped_pct.partition_point(|(ts, _)| *ts <= bar_end);
                if pos == 0 {
                    vals.push(None);
                } else {
                    vals.push(Some(stamped_pct[pos - 1].1));
                }
            }
        }
        let final_return_pct = vals.iter().rev().find_map(|v| *v);
        let block_index = SeriesBlockIndex::build(&vals);
        curves.push(MultiStrategyCurveSeries {
            kind: ComparisonCurveKind::StrategyPreset(preset),
            label: preset.short_label().to_owned(),
            is_active_preset: preset == active_preset,
            return_pct_values: vals,
            block_index,
            final_return_pct,
            max_drawdown_pct: max_dd,
            num_trades,
            sharpe_ratio: sharpe,
        });
    }

    // 2. Asset Buy & Hold (`0.00%` at `causal_candles[0].close`).
    if let Some(first_c) = causal_candles.first()
        && first_c.close.is_finite()
        && first_c.close > 0.0
    {
        let base_px = first_c.close;
        let mut bh_vals = Vec::with_capacity(n);
        let mut peak = base_px;
        let mut max_dd = 0.0_f64;
        for (i, c) in chart_candles.iter().enumerate() {
            if i >= causal_end || !c.close.is_finite() || c.close <= 0.0 {
                bh_vals.push(None);
            } else {
                peak = peak.max(c.close);
                if peak > 0.0 && c.close < peak {
                    max_dd = max_dd.max(((peak - c.close) / peak) * 100.0);
                }
                bh_vals.push(Some(((c.close / base_px) - 1.0) * 100.0));
            }
        }
        let final_return_pct = bh_vals.iter().rev().find_map(|v| *v);
        let block_index = SeriesBlockIndex::build(&bh_vals);
        curves.push(MultiStrategyCurveSeries {
            kind: ComparisonCurveKind::AssetBuyAndHold,
            label: format!("B&H ({})", symbol.trim()),
            is_active_preset: false,
            return_pct_values: bh_vals,
            block_index,
            final_return_pct,
            max_drawdown_pct: max_dd,
            num_trades: 1,
            sharpe_ratio: None,
        });
    }

    // 3. Causal Market Benchmark (`VNINDEX` for Vietnam venues, `SPY` for US/Global/Crypto),
    // strictly projected by completed benchmark bar close `bm_ts + bm_span <= bar_end_ts` (`DEF-001`).
    let venue = ExchangeVenue::infer_from_symbol_and_preset(&symbol, data_source);
    let target_bm = match venue {
        ExchangeVenue::Hose
        | ExchangeVenue::Hnx
        | ExchangeVenue::Upcom
        | ExchangeVenue::Vn30Futures
        | ExchangeVenue::VnIndexBenchmark => "VNINDEX",
        ExchangeVenue::UsEquities | ExchangeVenue::CryptoSpotPerp => "SPY",
    };

    let mut benchmark_label = None;
    if let (Some(first_c), Some(last_c)) = (causal_candles.first(), causal_candles.last())
        && !symbol.trim().eq_ignore_ascii_case(target_bm)
        && let Some((bm_sym, bm_tf, bm_candles)) = resolve_benchmark_series_for_window(
            target_bm,
            first_c.timestamp,
            last_c.timestamp,
            universe,
        )
    {
        let bm_span = bm_tf.seconds().max(1).saturating_sub(1);
        let bm_closed: Vec<(i64, f64)> = bm_candles
            .iter()
            .filter(|b| b.close.is_finite() && b.close > 0.0)
            .map(|b| (b.timestamp.saturating_add(bm_span), b.close))
            .collect();

        let mut base_bm_px: Option<f64> = None;
        let mut peak_bm = 0.0_f64;
        let mut max_dd_bm = 0.0_f64;
        let mut bm_vals = Vec::with_capacity(n);

        for (i, &bar_end) in chart_end_ts.iter().enumerate() {
            if i >= causal_end || bm_closed.is_empty() {
                bm_vals.push(None);
                continue;
            }
            let pos = bm_closed.partition_point(|(close_ts, _)| *close_ts <= bar_end);
            if pos == 0 {
                bm_vals.push(None);
            } else {
                let px = bm_closed[pos - 1].1;
                let ref_px = *base_bm_px.get_or_insert(px);
                peak_bm = peak_bm.max(px);
                if peak_bm > 0.0 && px < peak_bm {
                    max_dd_bm = max_dd_bm.max(((peak_bm - px) / peak_bm) * 100.0);
                }
                bm_vals.push(Some(((px / ref_px) - 1.0) * 100.0));
            }
        }

        if bm_vals.iter().any(Option::is_some) {
            let final_return_pct = bm_vals.iter().rev().find_map(|v| *v);
            let block_index = SeriesBlockIndex::build(&bm_vals);
            benchmark_label = Some(bm_sym.clone());
            curves.push(MultiStrategyCurveSeries {
                kind: ComparisonCurveKind::MarketBenchmark,
                label: format!("Bench ({bm_sym})"),
                is_active_preset: false,
                return_pct_values: bm_vals,
                block_index,
                final_return_pct,
                max_drawdown_pct: max_dd_bm,
                num_trades: 0,
                sharpe_ratio: None,
            });
        }
    }

    MultiStrategyComparisonReport {
        symbol,
        benchmark_label,
        timeframe,
        chart_bar_count: n,
        curves,
    }
}

// ============================================================================
// 2. Outlier Impact, Drawdown Periods, Year & Entry-Regime Breakdowns (`A5`)
// ============================================================================

/// Tail-dependency & outlier concentration diagnostics (`Milestone R17 / A5` — fixes
/// `ARCHIVE/backend/app/services/analytics_service.py:421` denominator & `N < 3` trim bugs).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutlierImpactReport {
    pub num_trades: usize,
    pub total_net_pnl: f64,
    pub gross_profit: f64,
    pub gross_loss_abs: f64,
    pub full_expectancy: Option<f64>,
    pub top_3_winners_pnl: f64,
    /// `Some((top_3_winners_pnl / gross_profit) * 100.0)` iff `gross_profit > 1e-9` (strictly `gross_profit`, never `gross_movement`).
    pub top_3_winners_share_of_gross_profit_pct: Option<f64>,
    pub net_pnl_ex_top_3_winners: f64,
    pub expectancy_ex_top_3_winners: Option<f64>,
    pub top_3_losers_abs: f64,
    /// `Some((top_3_losers_abs / gross_loss_abs) * 100.0)` iff `gross_loss_abs > 1e-9`.
    pub top_3_losers_share_of_gross_loss_pct: Option<f64>,
    /// Symmetric `10%` tail-trimmed expectancy (`k = (N / 10).max(1)` removed from each tail for `N >= 3`; `None` for `N < 3`).
    pub trimmed_expectancy_10pct: Option<f64>,
    /// `true` when `N >= 5`, strategy is net profitable, yet removing top 3 winners turns net PnL `<= 0.0`.
    pub outlier_dependent_warning: bool,
}

#[must_use]
pub fn compute_outlier_impact(trades: &[BacktestTrade]) -> OutlierImpactReport {
    let n = trades.len();
    let mut total_net_pnl = 0.0_f64;
    let mut gross_profit = 0.0_f64;
    let mut gross_loss_abs = 0.0_f64;
    let mut winners = Vec::new();
    let mut losers_abs = Vec::new();
    let mut all_pnls = Vec::with_capacity(n);

    for t in trades {
        if !t.net_pnl.is_finite() {
            continue;
        }
        total_net_pnl += t.net_pnl;
        all_pnls.push(t.net_pnl);
        if t.net_pnl > PNL_BREAKEVEN_EPS {
            gross_profit += t.net_pnl;
            winners.push(t.net_pnl);
        } else if t.net_pnl < -PNL_BREAKEVEN_EPS {
            let abs_l = t.net_pnl.abs();
            gross_loss_abs += abs_l;
            losers_abs.push(abs_l);
        }
    }

    winners.sort_by(|a, b| b.total_cmp(a));
    losers_abs.sort_by(|a, b| b.total_cmp(a));
    all_pnls.sort_by(|a, b| a.total_cmp(b));

    let valid_n = all_pnls.len();
    let full_expectancy = (valid_n > 0).then_some(total_net_pnl / (valid_n as f64));

    let top_3_winners_pnl: f64 = winners.iter().take(3).sum();
    let top_3_winners_share_of_gross_profit_pct = if gross_profit > PNL_BREAKEVEN_EPS {
        Some((top_3_winners_pnl / gross_profit) * 100.0)
    } else {
        None
    };
    let net_pnl_ex_top_3_winners = total_net_pnl - top_3_winners_pnl;
    let expectancy_ex_top_3_winners = if valid_n > 3 {
        Some(net_pnl_ex_top_3_winners / ((valid_n - 3) as f64))
    } else {
        None
    };

    let top_3_losers_abs: f64 = losers_abs.iter().take(3).sum();
    let top_3_losers_share_of_gross_loss_pct = if gross_loss_abs > PNL_BREAKEVEN_EPS {
        Some((top_3_losers_abs / gross_loss_abs) * 100.0)
    } else {
        None
    };

    let trimmed_expectancy_10pct = if valid_n >= 3 {
        let k = (valid_n / 10).max(1);
        let mid = &all_pnls[k..valid_n - k];
        (!mid.is_empty()).then(|| mid.iter().sum::<f64>() / (mid.len() as f64))
    } else {
        None
    };

    let outlier_dependent_warning = valid_n >= 5
        && total_net_pnl > PNL_BREAKEVEN_EPS
        && net_pnl_ex_top_3_winners <= PNL_BREAKEVEN_EPS;

    OutlierImpactReport {
        num_trades: valid_n,
        total_net_pnl,
        gross_profit,
        gross_loss_abs,
        full_expectancy,
        top_3_winners_pnl,
        top_3_winners_share_of_gross_profit_pct,
        net_pnl_ex_top_3_winners,
        expectancy_ex_top_3_winners,
        top_3_losers_abs,
        top_3_losers_share_of_gross_loss_pct,
        trimmed_expectancy_10pct,
        outlier_dependent_warning,
    }
}

/// Discrete peak-to-trough-to-recovery underwater drawdown episode (`Milestone R17 / A5`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DrawdownPeriod {
    pub peak_bar_index: usize,
    pub trough_bar_index: usize,
    /// `None` when the strategy remains underwater at the end of the causal window.
    pub recovery_bar_index: Option<usize>,
    pub peak_timestamp: i64,
    pub trough_timestamp: i64,
    pub recovery_timestamp: Option<i64>,
    pub peak_equity: f64,
    pub trough_equity: f64,
    pub max_drawdown_abs: f64,
    pub max_drawdown_pct: f64,
    pub duration_bars: usize,
    pub recovery_bars: Option<usize>,
}

/// Extracts discrete underwater drawdown episodes sorted by `max_drawdown_pct` descending.
#[must_use]
pub fn extract_drawdown_periods(
    initial_cash: f64,
    equity_curve: &[BacktestEquityPoint],
) -> Vec<DrawdownPeriod> {
    let Some(first) = equity_curve.first() else {
        return Vec::new();
    };
    let mut peak_eq = initial_cash.max(0.0);
    let mut peak_idx = first.bar_index;
    let mut peak_ts = first.timestamp;

    let mut active: Option<DrawdownPeriod> = None;
    let mut completed = Vec::new();

    for pt in equity_curve {
        if !pt.equity.is_finite() {
            continue;
        }
        if pt.equity >= peak_eq - PNL_BREAKEVEN_EPS {
            if let Some(mut ep) = active.take() {
                ep.recovery_bar_index = Some(pt.bar_index);
                ep.recovery_timestamp = Some(pt.timestamp);
                ep.duration_bars = pt.bar_index.saturating_sub(ep.peak_bar_index).max(1);
                ep.recovery_bars = Some(pt.bar_index.saturating_sub(ep.trough_bar_index));
                completed.push(ep);
            }
            if pt.equity >= peak_eq {
                peak_eq = pt.equity;
                peak_idx = pt.bar_index;
                peak_ts = pt.timestamp;
            }
        } else if peak_eq > 0.0 {
            let dd_abs = (peak_eq - pt.equity).max(0.0);
            let dd_pct = (dd_abs / peak_eq) * 100.0;
            match active.as_mut() {
                None => {
                    active = Some(DrawdownPeriod {
                        peak_bar_index: peak_idx,
                        trough_bar_index: pt.bar_index,
                        recovery_bar_index: None,
                        peak_timestamp: peak_ts,
                        trough_timestamp: pt.timestamp,
                        recovery_timestamp: None,
                        peak_equity: peak_eq,
                        trough_equity: pt.equity,
                        max_drawdown_abs: dd_abs,
                        max_drawdown_pct: dd_pct,
                        duration_bars: pt.bar_index.saturating_sub(peak_idx).max(1),
                        recovery_bars: None,
                    });
                }
                Some(ep) => {
                    ep.duration_bars = pt.bar_index.saturating_sub(ep.peak_bar_index).max(1);
                    if dd_abs > ep.max_drawdown_abs {
                        ep.trough_bar_index = pt.bar_index;
                        ep.trough_timestamp = pt.timestamp;
                        ep.trough_equity = pt.equity;
                        ep.max_drawdown_abs = dd_abs;
                        ep.max_drawdown_pct = dd_pct;
                    }
                }
            }
        }
    }

    if let Some(mut unrecovered) = active.take() {
        if let Some(last) = equity_curve.last() {
            unrecovered.duration_bars = last
                .bar_index
                .saturating_sub(unrecovered.peak_bar_index)
                .max(1);
        }
        completed.push(unrecovered);
    }

    completed.sort_by(|a, b| {
        b.max_drawdown_pct
            .total_cmp(&a.max_drawdown_pct)
            .then_with(|| a.peak_timestamp.cmp(&b.peak_timestamp))
    });
    completed
}

/// UTC calendar-year performance attribution row (`Milestone R17 / A5`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct YearPerformanceSlice {
    pub year: i32,
    pub bar_count: usize,
    pub equity_return_pct: f64,
    pub max_drawdown_pct: f64,
    pub trade_count: usize,
    pub win_count: usize,
    pub loss_count: usize,
    pub win_rate_pct: f64,
    pub realized_net_pnl: f64,
    pub profit_factor: Option<f64>,
    pub avg_r_multiple: Option<f64>,
}

/// Computes pure UTC proleptic Gregorian calendar-year slices (`decompose_utc`, zero local-TZ drift).
#[must_use]
pub fn compute_yearly_breakdown(
    initial_cash: f64,
    equity_curve: &[BacktestEquityPoint],
    trades: &[BacktestTrade],
) -> Vec<YearPerformanceSlice> {
    if equity_curve.is_empty() && trades.is_empty() {
        return Vec::new();
    }

    #[derive(Default)]
    struct YearAcc {
        bar_count: usize,
        start_equity: Option<f64>,
        end_equity: f64,
        peak_equity: f64,
        max_dd_pct: f64,
        trade_count: usize,
        win_count: usize,
        loss_count: usize,
        net_pnl: f64,
        gross_wins: f64,
        gross_losses: f64,
        r_sum: f64,
        r_count: usize,
    }

    let mut map: BTreeMap<i32, YearAcc> = BTreeMap::new();
    let mut prev_eq = initial_cash.max(1.0);

    for pt in equity_curve {
        if !pt.equity.is_finite() {
            continue;
        }
        let yr = decompose_utc(pt.timestamp).year;
        let acc = map.entry(yr).or_default();
        if acc.start_equity.is_none() {
            acc.start_equity = Some(prev_eq);
            acc.peak_equity = prev_eq.max(pt.equity);
        } else if pt.equity > acc.peak_equity {
            acc.peak_equity = pt.equity;
        }
        if acc.peak_equity > 0.0 && pt.equity < acc.peak_equity {
            acc.max_dd_pct = acc
                .max_dd_pct
                .max(((acc.peak_equity - pt.equity) / acc.peak_equity) * 100.0);
        }
        acc.bar_count += 1;
        acc.end_equity = pt.equity;
        prev_eq = pt.equity.max(0.0);
    }

    for tr in trades {
        let yr = decompose_utc(tr.exit_timestamp).year;
        let acc = map.entry(yr).or_default();
        acc.trade_count += 1;
        acc.net_pnl += tr.net_pnl;
        if tr.net_pnl > PNL_BREAKEVEN_EPS {
            acc.win_count += 1;
            acc.gross_wins += tr.net_pnl;
        } else if tr.net_pnl < -PNL_BREAKEVEN_EPS {
            acc.loss_count += 1;
            acc.gross_losses += tr.net_pnl.abs();
        }
        if let Some(r) = tr.r_multiple
            && r.is_finite()
        {
            acc.r_sum += r;
            acc.r_count += 1;
        }
    }

    map.into_iter()
        .map(|(year, acc)| {
            let start_eq = acc.start_equity.unwrap_or(initial_cash).max(1.0e-9);
            let equity_return_pct = if acc.bar_count > 0 {
                ((acc.end_equity - start_eq) / start_eq) * 100.0
            } else if initial_cash > 0.0 {
                (acc.net_pnl / initial_cash) * 100.0
            } else {
                0.0
            };
            YearPerformanceSlice {
                year,
                bar_count: acc.bar_count,
                equity_return_pct,
                max_drawdown_pct: acc.max_dd_pct,
                trade_count: acc.trade_count,
                win_count: acc.win_count,
                loss_count: acc.loss_count,
                win_rate_pct: if acc.trade_count > 0 {
                    (acc.win_count as f64 / acc.trade_count as f64) * 100.0
                } else {
                    0.0
                },
                realized_net_pnl: acc.net_pnl,
                profit_factor: (acc.gross_losses > PNL_BREAKEVEN_EPS)
                    .then_some(acc.gross_wins / acc.gross_losses),
                avg_r_multiple: (acc.r_count > 0).then_some(acc.r_sum / (acc.r_count as f64)),
            }
        })
        .collect()
}

/// Causal entry-signal [`MarketRegime`] performance breakdown row (`Milestone R17 / A5` — fixes
/// `ARCHIVE/backend/app/services/backtest_service.py:583` exit-bar regime lookahead contamination).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegimePerformanceSlice {
    pub regime: MarketRegime,
    pub bar_count: usize,
    pub trade_count: usize,
    pub win_count: usize,
    pub loss_count: usize,
    pub win_rate_pct: f64,
    pub net_pnl: f64,
    pub avg_return_pct: f64,
    pub profit_factor: Option<f64>,
    pub avg_r_multiple: Option<f64>,
}

/// Classifies every closed trade strictly by the causal [`MarketRegime`] at its **signal bar**
/// `signal_bar = trade.entry_bar_index.saturating_sub(1)` (`Close[T] -> Open[T + 1]` contract,
/// never inspecting `Close[entry_bar_index]` or `exit_bar_index`).
#[must_use]
pub fn compute_entry_regime_breakdown(
    causal_candles: &[Candle],
    trades: &[BacktestTrade],
) -> Vec<RegimePerformanceSlice> {
    if causal_candles.is_empty() {
        return Vec::new();
    }
    let regimes = compute_regime_series(causal_candles);
    compute_entry_regime_breakdown_with_precomputed(&regimes, trades)
}

#[must_use]
pub fn compute_entry_regime_breakdown_with_precomputed(
    regimes: &[MarketRegime],
    trades: &[BacktestTrade],
) -> Vec<RegimePerformanceSlice> {
    if regimes.is_empty() {
        return Vec::new();
    }

    #[derive(Default)]
    struct RegAcc {
        bar_count: usize,
        trade_count: usize,
        win_count: usize,
        loss_count: usize,
        net_pnl: f64,
        ret_pct_sum: f64,
        gross_wins: f64,
        gross_losses: f64,
        r_sum: f64,
        r_count: usize,
    }

    let mut map: HashMap<MarketRegime, RegAcc> = HashMap::new();
    for &r in regimes {
        map.entry(r).or_default().bar_count += 1;
    }

    for tr in trades {
        let signal_bar = tr.entry_bar_index.saturating_sub(1).min(regimes.len() - 1);
        let reg = regimes[signal_bar];
        let acc = map.entry(reg).or_default();
        acc.trade_count += 1;
        acc.net_pnl += tr.net_pnl;
        if tr.return_pct.is_finite() {
            acc.ret_pct_sum += tr.return_pct;
        }
        if tr.net_pnl > PNL_BREAKEVEN_EPS {
            acc.win_count += 1;
            acc.gross_wins += tr.net_pnl;
        } else if tr.net_pnl < -PNL_BREAKEVEN_EPS {
            acc.loss_count += 1;
            acc.gross_losses += tr.net_pnl.abs();
        }
        if let Some(r) = tr.r_multiple
            && r.is_finite()
        {
            acc.r_sum += r;
            acc.r_count += 1;
        }
    }

    let ordered_regimes = [
        MarketRegime::Uptrend,
        MarketRegime::Pullback,
        MarketRegime::SidewaysSqueeze,
        MarketRegime::Recovery,
        MarketRegime::Downtrend,
        MarketRegime::Neutral,
        MarketRegime::InsufficientHistory,
    ];

    let mut out = Vec::new();
    for reg in ordered_regimes {
        if let Some(acc) = map.remove(&reg)
            && (acc.trade_count > 0 || acc.bar_count > 0)
        {
            out.push(RegimePerformanceSlice {
                regime: reg,
                bar_count: acc.bar_count,
                trade_count: acc.trade_count,
                win_count: acc.win_count,
                loss_count: acc.loss_count,
                win_rate_pct: if acc.trade_count > 0 {
                    (acc.win_count as f64 / acc.trade_count as f64) * 100.0
                } else {
                    0.0
                },
                net_pnl: acc.net_pnl,
                avg_return_pct: if acc.trade_count > 0 {
                    acc.ret_pct_sum / (acc.trade_count as f64)
                } else {
                    0.0
                },
                profit_factor: (acc.gross_losses > PNL_BREAKEVEN_EPS)
                    .then_some(acc.gross_wins / acc.gross_losses),
                avg_r_multiple: (acc.r_count > 0).then_some(acc.r_sum / (acc.r_count as f64)),
            });
        }
    }
    out
}

// ============================================================================
// 3. `MultiPhaseBatch` Degradation Matrix & Deterministic `RunManifest` (`A5`)
// ============================================================================

/// Sub-window segmentation mode for [`run_multi_phase_batch`] (`MultiPhaseBatchPanel.tsx` replacement).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum MultiPhaseSplitMode {
    Halves,
    #[default]
    Tertiles,
    AutoMacroRegimes,
}

impl MultiPhaseSplitMode {
    pub const ALL: [Self; 3] = [Self::Halves, Self::Tertiles, Self::AutoMacroRegimes];

    #[inline]
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::Halves => "2 Halves",
            Self::Tertiles => "3 Tertiles",
            Self::AutoMacroRegimes => "Macro Regimes",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhaseWindowTearsheet {
    pub phase_index: usize,
    pub label: String,
    pub start_bar: usize,
    pub end_bar: usize,
    pub start_timestamp: i64,
    pub end_timestamp: i64,
    pub tearsheet: BacktestTearsheet,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MultiPhaseBatchReport {
    pub mode: MultiPhaseSplitMode,
    pub phases: Vec<PhaseWindowTearsheet>,
    pub profitable_phases: usize,
    pub consistency_pct: f64,
    pub best_phase_return_pct: f64,
    pub worst_phase_return_pct: f64,
    pub return_dispersion_pct: f64,
    pub late_vs_early_return_delta_pct: f64,
}

fn build_macro_regime_windows(
    regimes: &[MarketRegime],
    min_bars: usize,
) -> Vec<(String, usize, usize)> {
    let n = regimes.len();
    if n == 0 {
        return Vec::new();
    }
    let bucket_of = |r: MarketRegime| -> &'static str {
        match r {
            MarketRegime::Uptrend | MarketRegime::Pullback => "Bull Phase",
            MarketRegime::Downtrend | MarketRegime::Recovery => "Bear Phase",
            MarketRegime::SidewaysSqueeze
            | MarketRegime::Neutral
            | MarketRegime::InsufficientHistory => "Range/Base",
        }
    };

    let mut raw: Vec<(&'static str, usize, usize)> = Vec::new();
    for (i, &r) in regimes.iter().enumerate() {
        let lbl = bucket_of(r);
        match raw.last_mut() {
            Some((prev_lbl, _, end)) if *prev_lbl == lbl => {
                *end = i + 1;
            }
            _ => raw.push((lbl, i, i + 1)),
        }
    }

    // Coalesce short segments (`< min_bars`) into the preceding or following window, and cap to <= 6 windows.
    let mut merged: Vec<(&'static str, usize, usize)> = Vec::new();
    for (lbl, s, e) in raw {
        if let Some((_, _, prev_e)) = merged.last_mut()
            && e.saturating_sub(s) < min_bars
        {
            *prev_e = e;
        } else {
            merged.push((lbl, s, e));
        }
    }
    if merged.len() >= 2 && (merged[0].2 - merged[0].1) < min_bars {
        let first_s = merged[0].1;
        merged.remove(0);
        merged[0].1 = first_s;
    }
    while merged.len() > 6 {
        // Merge shortest adjacent pair
        let mut best_idx = 0_usize;
        let mut best_span = usize::MAX;
        for i in 0..merged.len() - 1 {
            let span = merged[i + 1].2 - merged[i].1;
            if span < best_span {
                best_span = span;
                best_idx = i;
            }
        }
        let end_merged = merged[best_idx + 1].2;
        merged.remove(best_idx + 1);
        merged[best_idx].2 = end_merged;
    }

    merged
        .into_iter()
        .enumerate()
        .map(|(k, (lbl, s, e))| (format!("P{} {}", k + 1, lbl), s, e))
        .collect()
}

/// Evaluates `base_config` across contiguous sub-windows (`Halves`, `Tertiles`, or `AutoMacroRegimes`)
/// with **single-pass `PrecomputedFeatures` reuse** and independent per-phase initial capital.
#[must_use]
pub fn run_multi_phase_batch(
    timeframe: Timeframe,
    candles: &[Candle],
    base_config: &BacktestConfig,
    mode: MultiPhaseSplitMode,
    options: BacktestRunOptions,
) -> MultiPhaseBatchReport {
    run_multi_phase_batch_for_symbol(None, timeframe, candles, base_config, mode, options)
}

/// Symbol-aware variant of [`run_multi_phase_batch`] recognizing exchange session closes (`VN30F1M`/`HOSE`/`US`/`W1`)
/// identically to [`run_backtest_with_options`].
#[must_use]
pub fn run_multi_phase_batch_for_symbol(
    symbol: Option<&str>,
    timeframe: Timeframe,
    candles: &[Candle],
    base_config: &BacktestConfig,
    mode: MultiPhaseSplitMode,
    options: BacktestRunOptions,
) -> MultiPhaseBatchReport {
    let mut clean_cfg = base_config.clone();
    clean_cfg.sanitize();
    let causal_candles =
        slice_causal_series_for_symbol_and_options(symbol, candles, timeframe, options);
    let regimes = compute_regime_series(causal_candles);
    run_multi_phase_batch_internal(
        timeframe,
        causal_candles,
        &regimes,
        &clean_cfg,
        mode,
        options.incomplete_tail_bar,
    )
}

fn run_multi_phase_batch_internal(
    timeframe: Timeframe,
    causal_candles: &[Candle],
    regimes: &[MarketRegime],
    clean_cfg: &BacktestConfig,
    mode: MultiPhaseSplitMode,
    incomplete_tail_bar: bool,
) -> MultiPhaseBatchReport {
    let n = causal_candles.len();
    if n < 8 {
        return MultiPhaseBatchReport {
            mode,
            phases: Vec::new(),
            profitable_phases: 0,
            consistency_pct: 0.0,
            best_phase_return_pct: 0.0,
            worst_phase_return_pct: 0.0,
            return_dispersion_pct: 0.0,
            late_vs_early_return_delta_pct: 0.0,
        };
    }

    let windows: Vec<(String, usize, usize)> = match mode {
        MultiPhaseSplitMode::Halves => {
            let mid = n / 2;
            vec![
                ("H1 (Early 50%)".to_owned(), 0, mid),
                ("H2 (Late 50%)".to_owned(), mid, n),
            ]
        }
        MultiPhaseSplitMode::Tertiles => {
            let t1 = n / 3;
            let t2 = (2 * n) / 3;
            vec![
                ("T1 (Early 33%)".to_owned(), 0, t1),
                ("T2 (Mid 33%)".to_owned(), t1, t2),
                ("T3 (Late 34%)".to_owned(), t2, n),
            ]
        }
        MultiPhaseSplitMode::AutoMacroRegimes => {
            let min_bars = (n / 5).clamp(8, 40);
            let built = build_macro_regime_windows(regimes, min_bars);
            if built.len() >= 2 {
                built
            } else {
                let mid = n / 2;
                vec![
                    ("P1 Early".to_owned(), 0, mid),
                    ("P2 Late".to_owned(), mid, n),
                ]
            }
        }
    };

    let shared_catalog = if clean_cfg.preset == StrategyPresetKind::CustomSignalRules {
        Some(Arc::new(PrecomputedCatalogMatrix::compute(causal_candles)))
    } else {
        None
    };
    let features =
        PrecomputedFeatures::build_with_shared_catalog(causal_candles, clean_cfg, shared_catalog);

    let mut phases = Vec::with_capacity(windows.len());
    for (idx, (label, s, e)) in windows.into_iter().enumerate() {
        if e <= s {
            continue;
        }
        let is_tail = e == n && incomplete_tail_bar;
        let sim = simulate_window_with_features(
            causal_candles,
            s,
            e,
            timeframe,
            clean_cfg,
            &features,
            is_tail,
        );
        let open_fee = sim.open_position.as_ref().map_or(0.0, |p| p.entry_fee);
        let tearsheet = compute_tearsheet(
            clean_cfg.initial_cash,
            sim.final_cash,
            timeframe,
            &sim.equity_curve,
            &sim.trades,
            open_fee,
        );
        phases.push(PhaseWindowTearsheet {
            phase_index: idx + 1,
            label,
            start_bar: s,
            end_bar: e,
            start_timestamp: causal_candles[s].timestamp,
            end_timestamp: causal_candles[e - 1].timestamp,
            tearsheet,
        });
    }

    if phases.is_empty() {
        return MultiPhaseBatchReport {
            mode,
            phases,
            profitable_phases: 0,
            consistency_pct: 0.0,
            best_phase_return_pct: 0.0,
            worst_phase_return_pct: 0.0,
            return_dispersion_pct: 0.0,
            late_vs_early_return_delta_pct: 0.0,
        };
    }

    let profitable_phases = phases
        .iter()
        .filter(|p| p.tearsheet.total_return_pct > PNL_BREAKEVEN_EPS)
        .count();
    let consistency_pct = (profitable_phases as f64 / phases.len() as f64) * 100.0;
    let best_phase_return_pct = phases
        .iter()
        .map(|p| p.tearsheet.total_return_pct)
        .fold(f64::NEG_INFINITY, f64::max);
    let worst_phase_return_pct = phases
        .iter()
        .map(|p| p.tearsheet.total_return_pct)
        .fold(f64::INFINITY, f64::min);
    let return_dispersion_pct = (best_phase_return_pct - worst_phase_return_pct).max(0.0);
    let late_vs_early_return_delta_pct = phases
        .last()
        .zip(phases.first())
        .map_or(0.0, |(late, early)| {
            late.tearsheet.total_return_pct - early.tearsheet.total_return_pct
        });

    MultiPhaseBatchReport {
        mode,
        phases,
        profitable_phases,
        consistency_pct,
        best_phase_return_pct,
        worst_phase_return_pct,
        return_dispersion_pct,
        late_vs_early_return_delta_pct,
    }
}

/// Deterministic wall-clock-free backtest reproducibility manifest (`Milestone R17 / A5` —
/// rectifies `ARCHIVE/backend/app/services/backtest_service.py:558` `datetime.now()` non-determinism).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunManifest {
    pub schema_version: u32,
    pub symbol: String,
    pub timeframe: Timeframe,
    pub bar_count: usize,
    pub first_timestamp: Option<i64>,
    pub last_timestamp: Option<i64>,
    pub replay_cutoff_timestamp: Option<i64>,
    pub incomplete_tail_bar: bool,
    pub preset: StrategyPresetKind,
    pub direction_mode: StrategyDirectionMode,
    pub param_summary: String,
    pub initial_cash: f64,
    pub commission_bps: f64,
    pub tax_sell_bps: f64,
    pub slippage_bps: f64,
    pub num_trades: usize,
    pub final_equity: f64,
    pub total_return_pct: f64,
    pub max_drawdown_pct: f64,
    pub manifest_digest: u64,
}

impl RunManifest {
    #[must_use]
    pub fn build(result: &BacktestRunResult, options: BacktestRunOptions) -> Self {
        let cfg = &result.config;
        let ts = &result.tearsheet;
        let param_summary = cfg.param_summary();

        // Canonical byte buffer over all deterministic inputs + outputs (zero wall-clock timestamps!).
        let mut canonical = String::with_capacity(512);
        let _ = write!(
            canonical,
            "v={RUN_MANIFEST_SCHEMA_VERSION}|sym={}|tf={:?}|n={}|t0={:?}|t1={:?}|cut={:?}|inc={}|preset={:?}|dir={:?}|sizing={:?}|lev={}|lot={}|split={:?}|params={:?}|risk={:?}|custom={:?}|cash={}|comm={}|tax={}|slip={}|trades={}|eq={}|ret={}|dd={}",
            result.symbol,
            result.timeframe,
            result.bar_count,
            result.first_timestamp,
            result.last_timestamp,
            options.replay_cutoff_timestamp,
            options.incomplete_tail_bar,
            cfg.preset,
            cfg.direction_mode,
            cfg.sizing,
            cfg.max_leverage.to_bits(),
            cfg.lot_step.to_bits(),
            cfg.in_sample_split_pct.map(f64::to_bits),
            cfg.params,
            cfg.risk,
            cfg.custom_rules,
            cfg.initial_cash.to_bits(),
            cfg.commission_bps.to_bits(),
            cfg.tax_sell_bps.to_bits(),
            cfg.slippage_bps.to_bits(),
            ts.num_trades,
            ts.final_equity.to_bits(),
            ts.total_return_pct.to_bits(),
            ts.max_drawdown_pct.to_bits(),
        );
        let manifest_digest = splitmix64_hash_bytes(canonical.as_bytes());

        Self {
            schema_version: RUN_MANIFEST_SCHEMA_VERSION,
            symbol: result.symbol.clone(),
            timeframe: result.timeframe,
            bar_count: result.bar_count,
            first_timestamp: result.first_timestamp,
            last_timestamp: result.last_timestamp,
            replay_cutoff_timestamp: options.replay_cutoff_timestamp,
            incomplete_tail_bar: options.incomplete_tail_bar,
            preset: cfg.preset,
            direction_mode: cfg.direction_mode,
            param_summary,
            initial_cash: cfg.initial_cash,
            commission_bps: cfg.commission_bps,
            tax_sell_bps: cfg.tax_sell_bps,
            slippage_bps: cfg.slippage_bps,
            num_trades: ts.num_trades,
            final_equity: ts.final_equity,
            total_return_pct: ts.total_return_pct,
            max_drawdown_pct: ts.max_drawdown_pct,
            manifest_digest,
        }
    }

    #[inline]
    #[must_use]
    pub fn short_digest_tag(&self) -> String {
        format!("#{:016x}", self.manifest_digest)
    }

    #[must_use]
    pub fn to_json_pretty(&self) -> String {
        let mut out = String::with_capacity(640);
        out.push_str("{\n");
        let _ = writeln!(out, "  \"schema_version\": {},", self.schema_version);
        out.push_str("  \"symbol\": ");
        escape_str_into(&mut out, &self.symbol);
        let _ = writeln!(out, ",\n  \"timeframe\": \"{:?}\",", self.timeframe);
        let _ = writeln!(out, "  \"bar_count\": {},", self.bar_count);
        match self.first_timestamp {
            Some(t) => {
                let _ = writeln!(out, "  \"first_timestamp\": {t},");
            }
            None => out.push_str("  \"first_timestamp\": null,\n"),
        }
        match self.last_timestamp {
            Some(t) => {
                let _ = writeln!(out, "  \"last_timestamp\": {t},");
            }
            None => out.push_str("  \"last_timestamp\": null,\n"),
        }
        match self.replay_cutoff_timestamp {
            Some(t) => {
                let _ = writeln!(out, "  \"replay_cutoff_timestamp\": {t},");
            }
            None => out.push_str("  \"replay_cutoff_timestamp\": null,\n"),
        }
        let _ = writeln!(
            out,
            "  \"incomplete_tail_bar\": {},",
            self.incomplete_tail_bar
        );
        let _ = writeln!(out, "  \"preset\": \"{:?}\",", self.preset);
        let _ = writeln!(out, "  \"direction_mode\": \"{:?}\",", self.direction_mode);
        out.push_str("  \"param_summary\": ");
        escape_str_into(&mut out, &self.param_summary);
        let _ = writeln!(
            out,
            ",\n  \"initial_cash\": {},",
            fmt_f64(self.initial_cash)
        );
        let _ = writeln!(
            out,
            "  \"commission_bps\": {},",
            fmt_f64(self.commission_bps)
        );
        let _ = writeln!(out, "  \"tax_sell_bps\": {},", fmt_f64(self.tax_sell_bps));
        let _ = writeln!(out, "  \"slippage_bps\": {},", fmt_f64(self.slippage_bps));
        let _ = writeln!(out, "  \"num_trades\": {},", self.num_trades);
        let _ = writeln!(out, "  \"final_equity\": {},", fmt_f64(self.final_equity));
        let _ = writeln!(
            out,
            "  \"total_return_pct\": {},",
            fmt_f64(self.total_return_pct)
        );
        let _ = writeln!(
            out,
            "  \"max_drawdown_pct\": {},",
            fmt_f64(self.max_drawdown_pct)
        );
        let _ = writeln!(
            out,
            "  \"manifest_digest_hex\": \"{:016x}\"",
            self.manifest_digest
        );
        out.push('}');
        out
    }

    #[must_use]
    pub fn to_ron_pretty(&self) -> String {
        let mut out = String::with_capacity(640);
        out.push_str("(\n");
        let _ = writeln!(out, "    schema_version: {},", self.schema_version);
        out.push_str("    symbol: ");
        escape_str_into(&mut out, &self.symbol);
        let _ = writeln!(out, ",\n    timeframe: {:?},", self.timeframe);
        let _ = writeln!(out, "    bar_count: {},", self.bar_count);
        let _ = writeln!(out, "    first_timestamp: {:?},", self.first_timestamp);
        let _ = writeln!(out, "    last_timestamp: {:?},", self.last_timestamp);
        let _ = writeln!(
            out,
            "    replay_cutoff_timestamp: {:?},",
            self.replay_cutoff_timestamp
        );
        let _ = writeln!(
            out,
            "    incomplete_tail_bar: {},",
            self.incomplete_tail_bar
        );
        let _ = writeln!(out, "    preset: {:?},", self.preset);
        let _ = writeln!(out, "    direction_mode: {:?},", self.direction_mode);
        out.push_str("    param_summary: ");
        escape_str_into(&mut out, &self.param_summary);
        let _ = writeln!(out, ",\n    initial_cash: {},", fmt_f64(self.initial_cash));
        let _ = writeln!(out, "    commission_bps: {},", fmt_f64(self.commission_bps));
        let _ = writeln!(out, "    tax_sell_bps: {},", fmt_f64(self.tax_sell_bps));
        let _ = writeln!(out, "    slippage_bps: {},", fmt_f64(self.slippage_bps));
        let _ = writeln!(out, "    num_trades: {},", self.num_trades);
        let _ = writeln!(out, "    final_equity: {},", fmt_f64(self.final_equity));
        let _ = writeln!(
            out,
            "    total_return_pct: {},",
            fmt_f64(self.total_return_pct)
        );
        let _ = writeln!(
            out,
            "    max_drawdown_pct: {},",
            fmt_f64(self.max_drawdown_pct)
        );
        let _ = writeln!(
            out,
            "    manifest_digest_hex: \"{:016x}\",",
            self.manifest_digest
        );
        out.push(')');
        out
    }
}

/// Complete `A5` institutional tearsheet analytics bundle cached alongside [`BacktestRunResult`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdvancedTearsheetAnalytics {
    pub outlier_impact: OutlierImpactReport,
    pub drawdown_periods: Vec<DrawdownPeriod>,
    pub yearly_slices: Vec<YearPerformanceSlice>,
    pub entry_regime_slices: Vec<RegimePerformanceSlice>,
    pub multi_phase: MultiPhaseBatchReport,
    pub manifest: RunManifest,
}

#[must_use]
pub fn compute_advanced_tearsheet_analytics(
    candles: &[Candle],
    result: &BacktestRunResult,
    phase_mode: MultiPhaseSplitMode,
    options: BacktestRunOptions,
) -> AdvancedTearsheetAnalytics {
    let causal_candles = slice_causal_series_for_symbol_and_options(
        Some(&result.symbol),
        candles,
        result.timeframe,
        options,
    );
    let regimes = compute_regime_series(causal_candles);

    let outlier_impact = compute_outlier_impact(&result.trades);
    let drawdown_periods =
        extract_drawdown_periods(result.config.initial_cash, &result.equity_curve);
    let yearly_slices = compute_yearly_breakdown(
        result.config.initial_cash,
        &result.equity_curve,
        &result.trades,
    );
    let entry_regime_slices =
        compute_entry_regime_breakdown_with_precomputed(&regimes, &result.trades);
    let multi_phase = run_multi_phase_batch_internal(
        result.timeframe,
        causal_candles,
        &regimes,
        &result.config,
        phase_mode,
        options.incomplete_tail_bar,
    );
    let manifest = RunManifest::build(result, options);

    // Silence unused-import warning when `run_backtest_with_options` is only referenced via doc links.
    let _ = run_backtest_with_options
        as fn(
            String,
            Timeframe,
            &[Candle],
            &BacktestConfig,
            BacktestRunOptions,
        ) -> BacktestRunResult;

    AdvancedTearsheetAnalytics {
        outlier_impact,
        drawdown_periods,
        yearly_slices,
        entry_regime_slices,
        multi_phase,
        manifest,
    }
}
