use serde::{Deserialize, Serialize};

use crate::data::{DataSourcePreset, Timeframe, WatchlistFeedEntry, aggregate_ohlcv};
use crate::indicators::SeriesBlockIndex;
use crate::model::Candle;

use super::engine::{
    BacktestRunOptions, PrecomputedFeatures, SimulationCarryState, compare_tearsheets_desc,
    is_bucket_incomplete_at_close, normalize_config_for_universe_feed, run_backtest_with_options,
    simulate_window_with_carry, simulate_window_with_features, slice_causal_series_for_options,
};
use super::metrics::{BacktestEquityPoint, BacktestTearsheet, BacktestTrade, compute_tearsheet};
use super::strategy::{
    BacktestConfig, StrategyPresetKind, SweepRankingMetric, build_preset_sweep_configs,
};

/// Minimum bars per Out-of-Sample fold required to run rolling Walk-Forward validation.
pub const MIN_OOS_BARS_PER_FOLD: usize = 12;

/// Minimum bars per In-Sample training window required to optimize parameters.
pub const MIN_IS_BARS_PER_FOLD: usize = 24;

/// Configuration for `K`-fold Rolling or Anchored Walk-Forward Optimization (`Milestone R10`).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WalkForwardConfig {
    /// Number of contiguous disjoint Out-of-Sample folds (`2..=8`, default `4`).
    pub fold_count: usize,
    /// Target In-Sample ratio relative to `IS + 1 OOS` fold (`0.50..=0.85`, default `0.70`).
    pub in_sample_ratio: f64,
    /// When `true`, `IS[k]` expands from bar `0` (`[0 .. oos_start_k)`); when `false`, rolls `[oos_start_k - L_is .. oos_start_k)`.
    pub anchored: bool,
    /// Objective metric maximized strictly inside `IS[k]`.
    pub ranking_metric: SweepRankingMetric,
}

impl Default for WalkForwardConfig {
    fn default() -> Self {
        Self {
            fold_count: 4,
            in_sample_ratio: 0.70,
            anchored: false,
            ranking_metric: SweepRankingMetric::SharpeRatio,
        }
    }
}

impl WalkForwardConfig {
    pub fn sanitize(&mut self) {
        self.fold_count = self.fold_count.clamp(2, 8);
        if !self.in_sample_ratio.is_finite() {
            self.in_sample_ratio = 0.70;
        } else {
            self.in_sample_ratio = self.in_sample_ratio.clamp(0.50, 0.85);
        }
    }
}

/// Out-of-Sample Walk-Forward robustness classification (`Pardo WFE` + fold consistency).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WalkForwardVerdict {
    RobustOutOfSample,
    ModerateDecay,
    OverfitRegimeDecay,
    InsufficientData,
}

impl WalkForwardVerdict {
    #[inline]
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::RobustOutOfSample => "WFO ROBUST",
            Self::ModerateDecay => "MODERATE DECAY",
            Self::OverfitRegimeDecay => "OVERFIT / DECAY",
            Self::InsufficientData => "INSUFFICIENT BARS",
        }
    }
}

/// Per-fold `IS -> OOS` optimization and blind causal test summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WalkForwardFoldResult {
    pub fold_index: usize,
    pub is_start_bar: usize,
    pub is_end_bar: usize,
    pub oos_start_bar: usize,
    pub oos_end_bar: usize,
    pub is_start_timestamp: i64,
    pub is_end_timestamp: i64,
    pub oos_start_timestamp: i64,
    pub oos_end_timestamp: i64,
    pub winning_label: String,
    pub winning_config: BacktestConfig,
    pub is_tearsheet: BacktestTearsheet,
    pub oos_tearsheet: BacktestTearsheet,
}

/// Complete output of `K`-fold Rolling or Anchored Walk-Forward Validation (`Milestone R10`).
#[derive(Debug, Clone)]
pub struct WalkForwardReport {
    pub symbol: String,
    pub timeframe: Timeframe,
    pub preset: StrategyPresetKind,
    pub wf_config: WalkForwardConfig,
    pub total_bars: usize,
    pub first_oos_bar_index: Option<usize>,
    pub folds: Vec<WalkForwardFoldResult>,
    pub stitched_oos_curve: Vec<BacktestEquityPoint>,
    pub stitched_oos_trades: Vec<BacktestTrade>,
    pub stitched_oos_tearsheet: BacktestTearsheet,
    /// Bar-aligned stitched `OOS` equity values (`None` on `0 .. first_oos_bar_index`, `Some` on `OOS`).
    pub stitched_equity_values: Vec<Option<f64>>,
    pub stitched_equity_idx: SeriesBlockIndex,
    pub stitched_underwater_values: Vec<Option<f64>>,
    pub stitched_underwater_idx: SeriesBlockIndex,
    pub profitable_folds: usize,
    /// Pardo Walk-Forward Efficiency (`(OOS return per bar / mean IS return per bar) * 100%`) when mean IS return > 0.
    pub wfe_pct: Option<f64>,
    /// Percentage of adjacent fold transitions that selected the identical parameter set (`0..=100%`).
    pub param_stability_pct: f64,
    pub verdict: WalkForwardVerdict,
}

/// Runs deterministic `K`-fold Rolling or Anchored Walk-Forward Optimization strictly gated by `options`.
///
/// Guarantees zero lookahead:
/// 1. Input candles are sliced causally via `slice_causal_series_for_options` (`DEF-001`).
/// 2. On every fold `k`, parameter selection evaluates `IS[k] = [is_start_k .. oos_start_k)` exclusively;
///    future bars `>= oos_start_k` never influence `winning_config`.
/// 3. Disjoint `OOS[k] = [oos_start_k .. oos_end_k)` segments compound capital continuously into `stitched_oos_curve`
///    with global peak-to-trough drawdown tracking.
#[must_use]
pub fn run_walk_forward_validation(
    symbol: impl Into<String>,
    timeframe: Timeframe,
    candles: &[Candle],
    base_config: &BacktestConfig,
    wf_config: WalkForwardConfig,
    options: BacktestRunOptions,
) -> WalkForwardReport {
    let symbol = symbol.into();
    let mut clean_base = base_config.clone();
    clean_base.sanitize();
    let mut clean_wf = wf_config;
    clean_wf.sanitize();

    let causal_candles = slice_causal_series_for_options(candles, timeframe, options);
    let n = causal_candles.len();

    let alpha = clean_wf.in_sample_ratio / (1.0 - clean_wf.in_sample_ratio);
    let raw_oos_len = (n as f64 / (alpha + clean_wf.fold_count as f64)).floor() as usize;
    let first_oos_start = n.saturating_sub(clean_wf.fold_count.saturating_mul(raw_oos_len));

    if raw_oos_len < MIN_OOS_BARS_PER_FOLD || first_oos_start < MIN_IS_BARS_PER_FOLD {
        let empty_sheet = compute_tearsheet(
            clean_base.initial_cash,
            clean_base.initial_cash,
            timeframe,
            &[],
            &[],
            0.0,
        );
        return WalkForwardReport {
            symbol,
            timeframe,
            preset: clean_base.preset,
            wf_config: clean_wf,
            total_bars: n,
            first_oos_bar_index: None,
            folds: Vec::new(),
            stitched_oos_curve: Vec::new(),
            stitched_oos_trades: Vec::new(),
            stitched_oos_tearsheet: empty_sheet,
            stitched_equity_values: vec![None; n],
            stitched_equity_idx: SeriesBlockIndex::default(),
            stitched_underwater_values: vec![None; n],
            stitched_underwater_idx: SeriesBlockIndex::default(),
            profitable_folds: 0,
            wfe_pct: None,
            param_stability_pct: 0.0,
            verdict: WalkForwardVerdict::InsufficientData,
        };
    }

    let is_window_len = first_oos_start;
    let grid = build_preset_sweep_configs(&clean_base);
    // Precompute strictly prefix-causal indicator vectors once per grid variant across `causal_candles`.
    let grid_with_features: Vec<(String, BacktestConfig, PrecomputedFeatures)> = grid
        .into_iter()
        .map(|(lbl, cfg)| {
            let feats = PrecomputedFeatures::build(causal_candles, &cfg);
            (lbl, cfg, feats)
        })
        .collect();

    let mut folds = Vec::with_capacity(clean_wf.fold_count);
    let mut stitched_oos_curve: Vec<BacktestEquityPoint> = Vec::with_capacity(n - first_oos_start);
    let mut stitched_oos_trades = Vec::new();
    let mut carry = SimulationCarryState::new(clean_base.initial_cash);

    let mut is_geom_rates = Vec::with_capacity(clean_wf.fold_count);
    let mut profitable_folds = 0_usize;

    for k in 0..clean_wf.fold_count {
        let oos_start = first_oos_start + k * raw_oos_len;
        let oos_end = if k + 1 == clean_wf.fold_count {
            n
        } else {
            oos_start + raw_oos_len
        };
        let is_start = if clean_wf.anchored {
            0
        } else {
            oos_start.saturating_sub(is_window_len)
        };
        let is_end = oos_start;

        // 1. Optimize strictly inside `IS[k] = [is_start .. is_end)` (`incomplete_tail = false`).
        let mut best_idx = 0_usize;
        let mut best_is_sheet: Option<BacktestTearsheet> = None;

        for (v_idx, (lbl, cfg, feats)) in grid_with_features.iter().enumerate() {
            let sim = simulate_window_with_features(
                causal_candles,
                is_start,
                is_end,
                timeframe,
                cfg,
                feats,
                false,
            );
            let open_fee = sim.open_position.as_ref().map_or(0.0, |p| p.entry_fee);
            let sheet = compute_tearsheet(
                cfg.initial_cash,
                sim.final_cash,
                timeframe,
                &sim.equity_curve,
                &sim.trades,
                open_fee,
            );
            let take = match &best_is_sheet {
                None => true,
                Some(prev_best) => {
                    compare_tearsheets_desc(&sheet, prev_best, clean_wf.ranking_metric)
                        .then_with(|| lbl.cmp(&grid_with_features[best_idx].0))
                        .is_lt()
                }
            };
            if take {
                best_idx = v_idx;
                best_is_sheet = Some(sheet);
            }
        }

        let (win_label, win_cfg, win_feats) = &grid_with_features[best_idx];
        let is_tearsheet = best_is_sheet.unwrap_or_else(|| {
            compute_tearsheet(
                clean_base.initial_cash,
                clean_base.initial_cash,
                timeframe,
                &[],
                &[],
                0.0,
            )
        });
        let is_bars_k = is_end.saturating_sub(is_start).max(1);
        let is_ratio_k = if is_tearsheet.initial_cash > 0.0 {
            (is_tearsheet.final_equity / is_tearsheet.initial_cash).max(0.0)
        } else {
            1.0
        };
        let is_geom_k = if is_ratio_k > 0.0 {
            is_ratio_k.powf(1.0 / (is_bars_k as f64)) - 1.0
        } else {
            -1.0
        };
        is_geom_rates.push(is_geom_k);

        // 2. Blind causal test on `OOS[k] = [oos_start .. oos_end)` carrying `SimulationCarryState`
        //    (`cash`, `active_pos` incl. `VN T+2` `min_holding_bars`, `pending_order`, `peak_equity`,
        //    and `next_trade_id`) seamlessly across fold boundaries (`C-02`).
        let is_final_fold = k + 1 == clean_wf.fold_count;
        let oos_incomplete = is_final_fold && options.incomplete_tail_bar;
        let fold_start_equity = if k == 0 {
            clean_base.initial_cash
        } else {
            stitched_oos_curve
                .last()
                .map_or(clean_base.initial_cash, |pt| pt.equity)
                .max(0.0)
        };

        let oos_tearsheet = if fold_start_equity <= 0.0 && carry.active_pos.is_none() {
            carry.cash = 0.0;
            carry.pending_order = None;
            for (local_i, c) in causal_candles[oos_start..oos_end].iter().enumerate() {
                let uw_pct = if carry.peak_equity > 0.0 { -100.0 } else { 0.0 };
                stitched_oos_curve.push(BacktestEquityPoint {
                    bar_index: oos_start + local_i,
                    timestamp: c.timestamp,
                    equity: 0.0,
                    cash: 0.0,
                    drawdown_abs: carry.peak_equity.max(0.0),
                    underwater_pct: uw_pct,
                    position_qty: 0.0,
                });
            }
            compute_tearsheet(0.0, 0.0, timeframe, &[], &[], 0.0)
        } else {
            let (oos_sim, next_carry) = simulate_window_with_carry(
                causal_candles,
                oos_start,
                oos_end,
                timeframe,
                win_cfg,
                win_feats,
                oos_incomplete,
                Some(carry),
                !is_final_fold,
                true,
            );
            carry = next_carry;

            let fold_open_fee = oos_sim
                .open_position
                .as_ref()
                .filter(|p| p.entry_bar_index >= oos_start)
                .map_or(0.0, |p| p.entry_fee);
            let sheet = compute_tearsheet(
                fold_start_equity.max(1.0e-9),
                oos_sim.final_cash,
                timeframe,
                &oos_sim.equity_curve,
                &oos_sim.trades,
                fold_open_fee,
            );

            if sheet.total_return_pct > 0.0 {
                profitable_folds += 1;
            }

            stitched_oos_curve.extend_from_slice(&oos_sim.equity_curve);
            stitched_oos_trades.extend(oos_sim.trades);
            sheet
        };

        folds.push(WalkForwardFoldResult {
            fold_index: k + 1,
            is_start_bar: is_start,
            is_end_bar: is_end,
            oos_start_bar: oos_start,
            oos_end_bar: oos_end,
            is_start_timestamp: causal_candles[is_start].timestamp,
            is_end_timestamp: causal_candles[is_end - 1].timestamp,
            oos_start_timestamp: causal_candles[oos_start].timestamp,
            oos_end_timestamp: causal_candles[oos_end - 1].timestamp,
            winning_label: win_label.clone(),
            winning_config: win_cfg.clone(),
            is_tearsheet,
            oos_tearsheet,
        });
    }

    let final_stitched_cash = carry.cash;
    let final_open_fee = carry.open_position_total_entry_fee();
    let stitched_oos_tearsheet = compute_tearsheet(
        clean_base.initial_cash,
        final_stitched_cash,
        timeframe,
        &stitched_oos_curve,
        &stitched_oos_trades,
        final_open_fee,
    );

    let mut stitched_equity_values = vec![None; n];
    let mut stitched_underwater_values = vec![None; n];
    for pt in &stitched_oos_curve {
        if pt.bar_index < n {
            stitched_equity_values[pt.bar_index] = Some(pt.equity);
            stitched_underwater_values[pt.bar_index] = Some(pt.underwater_pct);
        }
    }
    let stitched_equity_idx = SeriesBlockIndex::build(&stitched_equity_values);
    let stitched_underwater_idx = SeriesBlockIndex::build(&stitched_underwater_values);

    let total_oos_bars = n.saturating_sub(first_oos_start);
    let is_rate_per_bar = if !is_geom_rates.is_empty() {
        is_geom_rates.iter().sum::<f64>() / (is_geom_rates.len() as f64)
    } else {
        0.0
    };
    let oos_rate_per_bar = if total_oos_bars > 0 && clean_base.initial_cash > 0.0 {
        let oos_ratio = (stitched_oos_tearsheet.final_equity / clean_base.initial_cash).max(0.0);
        if oos_ratio > 0.0 {
            oos_ratio.powf(1.0 / (total_oos_bars as f64)) - 1.0
        } else {
            -1.0
        }
    } else {
        0.0
    };
    let wfe_pct = (is_rate_per_bar > 1.0e-9).then(|| (oos_rate_per_bar / is_rate_per_bar) * 100.0);

    let param_stability_pct = if folds.len() >= 2 {
        let transitions = folds.len() - 1;
        let same_cnt = folds
            .windows(2)
            .filter(|pair| pair[0].winning_label == pair[1].winning_label)
            .count();
        (same_cnt as f64 / transitions as f64) * 100.0
    } else {
        100.0
    };

    let min_robust_trades = folds.len().max(4);
    let verdict = if is_rate_per_bar <= 1.0e-9 || stitched_oos_tearsheet.total_return_pct <= 0.0 {
        WalkForwardVerdict::OverfitRegimeDecay
    } else if stitched_oos_trades.len() >= min_robust_trades
        && profitable_folds * 2 >= folds.len()
        && wfe_pct.is_some_and(|w| w >= 50.0)
    {
        WalkForwardVerdict::RobustOutOfSample
    } else {
        WalkForwardVerdict::ModerateDecay
    };

    WalkForwardReport {
        symbol,
        timeframe,
        preset: clean_base.preset,
        wf_config: clean_wf,
        total_bars: n,
        first_oos_bar_index: Some(first_oos_start),
        folds,
        stitched_oos_curve,
        stitched_oos_trades,
        stitched_oos_tearsheet,
        stitched_equity_values,
        stitched_equity_idx,
        stitched_underwater_values,
        stitched_underwater_idx,
        profitable_folds,
        wfe_pct,
        param_stability_pct,
        verdict,
    }
}

/// Capital allocation rule across active portfolio constituents (`Milestone R10`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum PortfolioWeightingMode {
    #[default]
    EqualWeight,
    CausalInverseVolatility {
        lookback_bars: usize,
    },
}

impl PortfolioWeightingMode {
    #[inline]
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::EqualWeight => "Equal Weight (1/N)",
            Self::CausalInverseVolatility { .. } => "Causal Inv-Vol",
        }
    }
}

/// Universe selection filter for multi-asset portfolio simulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum PortfolioUniverseFilter {
    /// Vietnam `VN30` cash equity basket (`8` HOSE blue-chips, homogeneous VND currency & `D1`/`W1` session grid).
    #[default]
    Vn30EquitiesOnly,
    /// All eligible watchlist feeds in the universe.
    AllUniverseFeeds,
}

impl PortfolioUniverseFilter {
    #[inline]
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::Vn30EquitiesOnly => "VN30 Basket",
            Self::AllUniverseFeeds => "All 13 Feeds",
        }
    }
}

/// Per-constituent attribution row inside [`PortfolioBacktestReport`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortfolioConstituentAttribution {
    pub symbol: String,
    pub instrument_id: String,
    pub timeframe: Timeframe,
    pub final_weight_pct: f64,
    pub standalone_return_pct: f64,
    pub standalone_max_dd_pct: f64,
    pub weighted_return_contribution_pct: f64,
    pub num_trades: usize,
}

/// Complete causal multi-asset portfolio simulation report (`Milestone R10`).
#[derive(Debug, Clone)]
pub struct PortfolioBacktestReport {
    pub preset: StrategyPresetKind,
    pub universe_filter: PortfolioUniverseFilter,
    pub weighting: PortfolioWeightingMode,
    pub timeframe: Timeframe,
    pub initial_capital: f64,
    pub portfolio_equity_curve: Vec<BacktestEquityPoint>,
    pub portfolio_tearsheet: BacktestTearsheet,
    pub avg_constituent_max_dd_pct: f64,
    /// Relative max-drawdown reduction vs average standalone constituent (`((avg_dd - port_dd) / avg_dd) * 100%`).
    pub diversification_dd_reduction_pct: f64,
    pub constituents: Vec<PortfolioConstituentAttribution>,
}

#[inline]
fn constituent_annualization_root(timeframe: Timeframe) -> f64 {
    let bars_per_year: f64 = match timeframe {
        Timeframe::M1 => 98_280.0,
        Timeframe::M5 => 19_656.0,
        Timeframe::M15 => 6_552.0,
        Timeframe::H1 => 1_638.0,
        Timeframe::H4 => 1_512.0,
        Timeframe::D1 => 252.0,
        Timeframe::W1 => 52.0,
    };
    bars_per_year.sqrt()
}

struct PreparedConstituent {
    symbol: String,
    instrument_id: String,
    timeframe: Timeframe,
    first_ts: i64,
    /// Strictly increasing causal bar-close instants (`bar_open_ts + tf.seconds() - 1`, clamped to `replay_causal_close_timestamp`).
    timestamps: Vec<i64>,
    wealth: Vec<f64>,
    /// Genuine constituent-frequency bar-to-bar returns (`1:1` with `timestamps`, zero-dilution-free).
    obs_returns: Vec<f64>,
    annualization_root: f64,
    standalone_return_pct: f64,
    standalone_max_dd_pct: f64,
    num_trades: usize,
}

/// Simulates a combined multi-asset portfolio across `feeds`, strictly gated by
/// `replay_causal_close_timestamp` (`DEF-001`), stamping constituent equity at causal bar-close
/// instants (`C-01`) and using strictly past-only annualized inverse-volatility or equal weights.
#[must_use]
pub fn run_portfolio_backtest(
    feeds: &[WatchlistFeedEntry],
    preferred_timeframe: Timeframe,
    config: &BacktestConfig,
    universe_filter: PortfolioUniverseFilter,
    weighting: PortfolioWeightingMode,
    replay_causal_close_timestamp: Option<i64>,
) -> PortfolioBacktestReport {
    let mut clean_cfg = config.clone();
    clean_cfg.sanitize();
    let initial_capital = clean_cfg.initial_cash;

    let mut prepared = Vec::new();
    let mut all_timestamps = Vec::new();

    for feed in feeds {
        if universe_filter == PortfolioUniverseFilter::Vn30EquitiesOnly
            && feed.preset != DataSourcePreset::Vn30Basket
        {
            continue;
        }
        let effective_tf = if feed.base_timeframe.can_aggregate_to(preferred_timeframe) {
            preferred_timeframe
        } else {
            feed.base_timeframe
        };
        let base_bar_span = feed.base_timeframe.seconds().max(1).saturating_sub(1);
        let closed_base_slice: &[Candle] = match replay_causal_close_timestamp {
            None => &feed.candles,
            Some(close_cut) => {
                let end = feed
                    .candles
                    .partition_point(|c| c.timestamp.saturating_add(base_bar_span) <= close_cut);
                &feed.candles[..end]
            }
        };
        if closed_base_slice.is_empty() {
            continue;
        }

        let series_cow: Vec<Candle>;
        let series_ref: &[Candle] = if effective_tf == feed.base_timeframe {
            closed_base_slice
        } else {
            match aggregate_ohlcv(closed_base_slice, feed.base_timeframe, effective_tf) {
                Ok(v) => {
                    series_cow = v;
                    &series_cow
                }
                Err(_) => continue,
            }
        };
        if series_ref.is_empty() {
            continue;
        }

        let incomplete_tail = match (replay_causal_close_timestamp, series_ref.last()) {
            (Some(close_cut), Some(last_bar)) => is_bucket_incomplete_at_close(
                last_bar.timestamp,
                close_cut,
                feed.base_timeframe,
                effective_tf,
            ),
            _ => false,
        };

        let feed_cfg = normalize_config_for_universe_feed(&clean_cfg, feed.preset, effective_tf);
        let res = run_backtest_with_options(
            feed.symbol.clone(),
            effective_tf,
            series_ref,
            &feed_cfg,
            BacktestRunOptions {
                replay_cutoff_timestamp: None,
                incomplete_tail_bar: incomplete_tail,
            },
        );
        if res.equity_curve.is_empty() {
            continue;
        }
        let c0 = feed_cfg.initial_cash.max(1.0);
        let bar_span = effective_tf.seconds().max(1).saturating_sub(1);
        let mut ts_vec: Vec<i64> = Vec::with_capacity(res.equity_curve.len());
        let mut w_vec: Vec<f64> = Vec::with_capacity(res.equity_curve.len());
        for pt in &res.equity_curve {
            let mut close_ts = pt.timestamp.saturating_add(bar_span);
            if let Some(cut) = replay_causal_close_timestamp {
                close_ts = close_ts.min(cut);
            }
            let w_val = (pt.equity / c0).max(0.0);
            if ts_vec.last() == Some(&close_ts) {
                if let Some(last_w) = w_vec.last_mut() {
                    *last_w = w_val;
                }
            } else {
                ts_vec.push(close_ts);
                w_vec.push(w_val);
                all_timestamps.push(close_ts);
            }
        }
        if ts_vec.is_empty() {
            continue;
        }
        let mut obs_returns = Vec::with_capacity(w_vec.len());
        let mut prev_obs_w = 1.0_f64;
        for &cur_obs_w in &w_vec {
            let r = if prev_obs_w > 1.0e-12 {
                (cur_obs_w / prev_obs_w) - 1.0
            } else {
                0.0
            };
            obs_returns.push(if r.is_finite() { r } else { 0.0 });
            prev_obs_w = cur_obs_w;
        }

        prepared.push(PreparedConstituent {
            symbol: feed.symbol.clone(),
            instrument_id: feed.instrument_id.clone(),
            timeframe: effective_tf,
            first_ts: ts_vec[0],
            timestamps: ts_vec,
            wealth: w_vec,
            obs_returns,
            annualization_root: constituent_annualization_root(effective_tf),
            standalone_return_pct: res.tearsheet.total_return_pct,
            standalone_max_dd_pct: res.tearsheet.max_drawdown_pct,
            num_trades: res.tearsheet.num_trades,
        });
    }

    all_timestamps.sort_unstable();
    all_timestamps.dedup();

    if prepared.is_empty() || all_timestamps.is_empty() {
        let empty_sheet = compute_tearsheet(
            initial_capital,
            initial_capital,
            preferred_timeframe,
            &[],
            &[],
            0.0,
        );
        return PortfolioBacktestReport {
            preset: clean_cfg.preset,
            universe_filter,
            weighting,
            timeframe: preferred_timeframe,
            initial_capital,
            portfolio_equity_curve: Vec::new(),
            portfolio_tearsheet: empty_sheet,
            avg_constituent_max_dd_pct: 0.0,
            diversification_dd_reduction_pct: 0.0,
            constituents: Vec::new(),
        };
    }

    let grid_tf = prepared
        .iter()
        .map(|c| c.timeframe)
        .min_by_key(|tf| tf.seconds())
        .unwrap_or(preferred_timeframe);

    let s_cnt = prepared.len();
    let m_cnt = all_timestamps.len();

    // Forward-fill constituent wealth onto `all_timestamps` using strictly past/current causal bar-closes `<= t_m`.
    let mut step_returns: Vec<Vec<f64>> = vec![vec![0.0; m_cnt]; s_cnt];
    for (j, c) in prepared.iter().enumerate() {
        let mut prev_w = 1.0_f64;
        let mut cursor = 0_usize;
        for (m, &t_m) in all_timestamps.iter().enumerate() {
            while cursor < c.timestamps.len() && c.timestamps[cursor] <= t_m {
                cursor += 1;
            }
            let cur_w = if cursor == 0 {
                1.0
            } else {
                c.wealth[cursor - 1]
            };
            let ret = if prev_w > 1.0e-12 {
                (cur_w / prev_w) - 1.0
            } else {
                0.0
            };
            step_returns[j][m] = if ret.is_finite() { ret } else { 0.0 };
            prev_w = cur_w;
        }
    }

    let mut port_equity = initial_capital;
    let mut peak_equity = initial_capital;
    let mut portfolio_equity_curve = Vec::with_capacity(m_cnt);
    let mut dollar_pnl = vec![0.0_f64; s_cnt];
    let mut final_weights = vec![0.0_f64; s_cnt];
    let mut weights_buf = vec![0.0_f64; s_cnt];

    for (m, &t_m) in all_timestamps.iter().enumerate() {
        // Constituent `j` is eligible for allocation at step `m` iff its first bar closed by `t_{m-1}` (or `t_0` at `m == 0`).
        let elig_ts = if m == 0 { t_m } else { all_timestamps[m - 1] };
        weights_buf.fill(0.0);

        let mut active_indices = Vec::with_capacity(s_cnt);
        for (j, c) in prepared.iter().enumerate() {
            if c.first_ts <= elig_ts {
                active_indices.push(j);
            }
        }
        if active_indices.is_empty() {
            for j in 0..s_cnt {
                active_indices.push(j);
            }
        }

        match weighting {
            PortfolioWeightingMode::EqualWeight => {
                let w = 1.0 / (active_indices.len() as f64);
                for &j in &active_indices {
                    weights_buf[j] = w;
                }
            }
            PortfolioWeightingMode::CausalInverseVolatility { lookback_bars } => {
                let lb = lookback_bars.clamp(5, 252);
                // Evaluate strictly on genuine past bar-close observations `obs_ts <= elig_ts` (`< t_m` for `m >= 1`),
                // annualized by `c.annualization_root` so mixed-frequency (`1m`/`15m`/`1D`) volatilities never suffer zero-padding dilution (`C-01`).
                let mut inv_vol_sum = 0.0_f64;
                let mut all_have_min_obs = m >= 1;
                if all_have_min_obs {
                    for &j in &active_indices {
                        let c = &prepared[j];
                        let obs_end = c.timestamps.partition_point(|&ts| ts <= elig_ts);
                        let obs_start = obs_end.saturating_sub(lb);
                        let slice = &c.obs_returns[obs_start..obs_end];
                        if slice.len() < 5 {
                            all_have_min_obs = false;
                            break;
                        }
                        let mean = slice.iter().sum::<f64>() / (slice.len() as f64);
                        let var = slice
                            .iter()
                            .map(|r| {
                                let d = *r - mean;
                                d * d
                            })
                            .sum::<f64>()
                            / ((slice.len() - 1).max(1) as f64);
                        let ann_vol = (var.sqrt() * c.annualization_root).max(0.02);
                        let raw_w = 1.0 / ann_vol;
                        weights_buf[j] = raw_w;
                        inv_vol_sum += raw_w;
                    }
                }
                if all_have_min_obs && inv_vol_sum > 1.0e-12 {
                    for &j in &active_indices {
                        weights_buf[j] /= inv_vol_sum;
                    }
                } else {
                    weights_buf.fill(0.0);
                    let w = 1.0 / (active_indices.len() as f64);
                    for &j in &active_indices {
                        weights_buf[j] = w;
                    }
                }
            }
        }

        let prev_port_eq = port_equity;
        let mut port_ret = 0.0_f64;
        for j in 0..s_cnt {
            let wj = weights_buf[j];
            let rj = step_returns[j][m];
            let weighted_r = wj * rj;
            port_ret += weighted_r;
            dollar_pnl[j] += prev_port_eq * weighted_r;
        }
        if m + 1 == m_cnt {
            final_weights.copy_from_slice(&weights_buf);
        }

        port_equity = (port_equity * (1.0 + port_ret)).max(0.0);
        if port_equity >= peak_equity {
            peak_equity = port_equity;
        }
        let dd_abs = (peak_equity - port_equity).max(0.0);
        let uw_pct = if peak_equity > 0.0 && port_equity < peak_equity {
            ((port_equity - peak_equity) / peak_equity) * 100.0
        } else {
            0.0
        };

        portfolio_equity_curve.push(BacktestEquityPoint {
            bar_index: m,
            timestamp: t_m,
            equity: port_equity,
            cash: port_equity,
            drawdown_abs: dd_abs,
            underwater_pct: uw_pct,
            position_qty: 0.0,
        });
    }

    let portfolio_tearsheet = compute_tearsheet(
        initial_capital,
        port_equity,
        grid_tf,
        &portfolio_equity_curve,
        &[],
        0.0,
    );

    let avg_constituent_max_dd_pct = prepared
        .iter()
        .map(|c| c.standalone_max_dd_pct)
        .sum::<f64>()
        / (s_cnt as f64);
    let diversification_dd_reduction_pct = if avg_constituent_max_dd_pct > 1.0e-9 {
        ((avg_constituent_max_dd_pct - portfolio_tearsheet.max_drawdown_pct)
            / avg_constituent_max_dd_pct)
            * 100.0
    } else {
        0.0
    };

    let denom_cap = initial_capital.max(1.0e-9);
    let mut constituents: Vec<PortfolioConstituentAttribution> = prepared
        .into_iter()
        .enumerate()
        .map(|(j, c)| PortfolioConstituentAttribution {
            symbol: c.symbol,
            instrument_id: c.instrument_id,
            timeframe: c.timeframe,
            final_weight_pct: final_weights[j] * 100.0,
            standalone_return_pct: c.standalone_return_pct,
            standalone_max_dd_pct: c.standalone_max_dd_pct,
            weighted_return_contribution_pct: (dollar_pnl[j] / denom_cap) * 100.0,
            num_trades: c.num_trades,
        })
        .collect();
    constituents.sort_by(|a, b| {
        b.weighted_return_contribution_pct
            .total_cmp(&a.weighted_return_contribution_pct)
            .then_with(|| a.symbol.cmp(&b.symbol))
    });

    PortfolioBacktestReport {
        preset: clean_cfg.preset,
        universe_filter,
        weighting,
        timeframe: grid_tf,
        initial_capital,
        portfolio_equity_curve,
        portfolio_tearsheet,
        avg_constituent_max_dd_pct,
        diversification_dd_reduction_pct,
        constituents,
    }
}

/// Projects an arbitrary timestamp-ordered [`BacktestEquityPoint`] curve (such as a multi-asset
/// [`PortfolioBacktestReport`] curve stamped at causal bar-close instants) onto `chart_candles`
/// via causal bar-close forward-fill (`pt.timestamp <= bar_close_ts`) so `OscillatorPaneId::Equity`
/// renders it synchronized with the active chart's `TimeScale` (`M-03`).
#[must_use]
pub fn map_equity_curve_to_candles(
    curve: &[BacktestEquityPoint],
    chart_candles: &[Candle],
    chart_timeframe: Timeframe,
    replay_causal_close_timestamp: Option<i64>,
) -> (
    Vec<Option<f64>>,
    SeriesBlockIndex,
    Vec<Option<f64>>,
    SeriesBlockIndex,
) {
    let n = chart_candles.len();
    if curve.is_empty() || n == 0 {
        return (
            vec![None; n],
            SeriesBlockIndex::default(),
            vec![None; n],
            SeriesBlockIndex::default(),
        );
    }
    let first_ts = curve[0].timestamp;
    let bar_span = chart_timeframe.seconds().max(1).saturating_sub(1);
    let mut eq_vals = Vec::with_capacity(n);
    let mut uw_vals = Vec::with_capacity(n);
    for (i, c) in chart_candles.iter().enumerate() {
        let mut bar_end_ts = c.timestamp.saturating_add(bar_span);
        if let Some(next_c) = chart_candles.get(i + 1)
            && next_c.timestamp > c.timestamp
        {
            bar_end_ts = bar_end_ts.min(next_c.timestamp.saturating_sub(1));
        }
        if let Some(cut) = replay_causal_close_timestamp {
            bar_end_ts = bar_end_ts.min(cut);
        }
        if bar_end_ts < first_ts {
            eq_vals.push(None);
            uw_vals.push(None);
        } else {
            let idx = curve.partition_point(|pt| pt.timestamp <= bar_end_ts);
            if idx == 0 {
                eq_vals.push(None);
                uw_vals.push(None);
            } else {
                let pt = &curve[idx - 1];
                eq_vals.push(Some(pt.equity));
                uw_vals.push(Some(pt.underwater_pct));
            }
        }
    }
    let eq_idx = SeriesBlockIndex::build(&eq_vals);
    let uw_idx = SeriesBlockIndex::build(&uw_vals);
    (eq_vals, eq_idx, uw_vals, uw_idx)
}
