use std::collections::VecDeque;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::data::{DataSourcePreset, Timeframe, WatchlistFeedEntry, aggregate_ohlcv};
use crate::indicators::{
    BollingerOutput, MacdOutput, SeriesBlockIndex, compute_atr, compute_bollinger_slices,
    compute_ema_slices, compute_macd_slices, compute_rsi_slices, compute_sma_slices,
};
use crate::model::Candle;
use crate::scanner::PrecomputedCatalogMatrix;
use crate::trading::{
    MAX_ORDER_NOTIONAL, MAX_ORDER_PRICE, MAX_ORDER_QUANTITY, MIN_ORDER_QUANTITY, OrderSide,
    PositionSide, apply_slippage, compute_commission,
};

use super::metrics::{
    BacktestEquityPoint, BacktestExitReason, BacktestTearsheet, BacktestTrade, RobustnessVerdict,
    UnexecutedSignal, UnfilledReason, compute_tearsheet, evaluate_robustness,
};
use super::strategy::{
    BacktestConfig, PositionSizingMode, StrategyDirectionMode, StrategyPresetKind,
    SweepRankingMetric, build_preset_sweep_configs, evaluate_custom_rules_at_bar,
};

/// Causal execution options gating lookahead during active Bar Replay (`DEF-001`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BacktestRunOptions {
    /// Optional causal Replay frontier timestamp (`candles` beyond this timestamp are strictly excluded).
    pub replay_cutoff_timestamp: Option<i64>,
    /// When `true`, suppresses close-of-bar signal generation on the final unclosed higher-timeframe bucket.
    pub incomplete_tail_bar: bool,
}

/// Active unclosed position held at the end of a backtest window.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenBacktestPosition {
    pub side: PositionSide,
    pub quantity: f64,
    pub entry_bar_index: usize,
    pub entry_timestamp: i64,
    pub entry_price: f64,
    pub entry_fee: f64,
    pub stop_loss: Option<f64>,
    pub take_profit: Option<f64>,
    pub trailing_stop: Option<f64>,
    pub unrealized_pnl: f64,
    pub unrealized_return_pct: f64,
}

/// Complete deterministic output of a single-symbol strategy backtest (`Milestone R09 / Wave 8`).
#[derive(Debug, Clone)]
pub struct BacktestRunResult {
    pub symbol: String,
    pub timeframe: Timeframe,
    pub config: BacktestConfig,
    pub bar_count: usize,
    pub first_timestamp: Option<i64>,
    pub last_timestamp: Option<i64>,
    pub equity_curve: Vec<BacktestEquityPoint>,
    /// Bar-aligned mark-to-market equity values (`len == bar_count`) indexed for `OscillatorPaneId::Equity`.
    pub equity_values: Vec<Option<f64>>,
    pub equity_idx: SeriesBlockIndex,
    /// Bar-aligned underwater drawdown `%` values (`<= 0.0`, `len == bar_count`).
    pub underwater_values: Vec<Option<f64>>,
    pub underwater_idx: SeriesBlockIndex,
    pub trades: Vec<BacktestTrade>,
    pub open_position: Option<OpenBacktestPosition>,
    pub unexecuted_signals: Vec<UnexecutedSignal>,
    pub tearsheet: BacktestTearsheet,
    pub split_bar_index: Option<usize>,
    pub in_sample_tearsheet: Option<BacktestTearsheet>,
    pub out_of_sample_tearsheet: Option<BacktestTearsheet>,
    pub robustness: Option<RobustnessVerdict>,
}

/// Single ranked row inside a deterministic parameter sweep (`run_parameter_sweep`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SweepVariantRow {
    pub rank: usize,
    pub label: String,
    pub config: BacktestConfig,
    pub tearsheet: BacktestTearsheet,
    pub in_sample_return_pct: Option<f64>,
    pub out_of_sample_return_pct: Option<f64>,
    pub robustness: Option<RobustnessVerdict>,
}

/// Complete output of a multi-variant parameter grid sweep on an instrument.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParameterSweepReport {
    pub symbol: String,
    pub timeframe: Timeframe,
    pub preset: StrategyPresetKind,
    pub ranking_metric: SweepRankingMetric,
    pub variants: Vec<SweepVariantRow>,
}

/// Single symbol row inside a multi-symbol universe strategy battle (`run_multi_symbol_battle`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SymbolBattleRow {
    pub rank: usize,
    pub instrument_id: String,
    pub symbol: String,
    pub source_label: String,
    pub timeframe: Timeframe,
    pub bar_count: usize,
    pub tearsheet: BacktestTearsheet,
    pub robustness: Option<RobustnessVerdict>,
}

/// Aggregated multi-symbol backtest comparison across the Watchlist Universe.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MultiSymbolBattleReport {
    pub preset: StrategyPresetKind,
    pub ranking_metric: SweepRankingMetric,
    pub rows: Vec<SymbolBattleRow>,
    pub profitable_symbols: usize,
    pub average_return_pct: f64,
}

/// Single preset row comparing all 6 [`StrategyPresetKind`] strategies on the active symbol.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PresetBattleRow {
    pub rank: usize,
    pub preset: StrategyPresetKind,
    pub param_summary: String,
    pub tearsheet: BacktestTearsheet,
    pub robustness: Option<RobustnessVerdict>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PendingStrategyOrder {
    Entry {
        side: PositionSide,
        signal_bar: usize,
    },
    Exit {
        signal_bar: usize,
        reason: BacktestExitReason,
    },
    Reverse {
        new_side: PositionSide,
        signal_bar: usize,
    },
}

#[derive(Debug, Clone)]
pub(crate) struct ActivePositionState {
    side: PositionSide,
    quantity: f64,
    entry_bar_abs: usize,
    entry_bar_index: usize,
    entry_timestamp: i64,
    pub(crate) entry_price: f64,
    pub(crate) entry_fee: f64,
    pub(crate) entry_sell_tax: f64,
    fixed_stop_loss: Option<f64>,
    take_profit: Option<f64>,
    trailing_stop: Option<f64>,
    initial_risk_amount: Option<f64>,
    highest_since_entry: f64,
    lowest_since_entry: f64,
    mae_pct: f64,
    mfe_pct: f64,
}

#[derive(Debug, Clone)]
pub(crate) struct SimulationCarryState {
    pub(crate) cash: f64,
    pub(crate) peak_equity: f64,
    pub(crate) active_pos: Option<ActivePositionState>,
    pub(crate) pending_order: Option<PendingStrategyOrder>,
    pub(crate) next_trade_id: u64,
}

impl SimulationCarryState {
    #[inline]
    pub(crate) fn new(initial_cash: f64) -> Self {
        Self {
            cash: initial_cash,
            peak_equity: initial_cash,
            active_pos: None,
            pending_order: None,
            next_trade_id: 1,
        }
    }

    #[inline]
    pub(crate) fn open_position_total_entry_fee(&self) -> f64 {
        self.active_pos
            .as_ref()
            .map_or(0.0, |p| p.entry_fee + p.entry_sell_tax)
    }
}

impl ActivePositionState {
    #[inline]
    fn unrealized_pnl(&self, mark_price: f64) -> f64 {
        if !mark_price.is_finite() || self.quantity <= 0.0 {
            return 0.0;
        }
        self.side.sign() * (mark_price - self.entry_price) * self.quantity
    }

    #[inline]
    fn unrealized_return_pct(&self, mark_price: f64) -> f64 {
        if self.entry_price <= 0.0 || !mark_price.is_finite() {
            return 0.0;
        }
        self.side.sign() * ((mark_price - self.entry_price) / self.entry_price) * 100.0
    }

    /// Resolves effective stop level combining fixed `stop_loss` and prior-bar `trailing_stop`.
    #[inline]
    fn effective_stop(&self) -> Option<(f64, BacktestExitReason)> {
        match (self.fixed_stop_loss, self.trailing_stop) {
            (Some(sl), Some(tr)) => match self.side {
                PositionSide::Long => {
                    if tr > sl {
                        Some((tr, BacktestExitReason::TrailingStop))
                    } else {
                        Some((sl, BacktestExitReason::StopLoss))
                    }
                }
                PositionSide::Short => {
                    if tr < sl {
                        Some((tr, BacktestExitReason::TrailingStop))
                    } else {
                        Some((sl, BacktestExitReason::StopLoss))
                    }
                }
            },
            (Some(sl), None) => Some((sl, BacktestExitReason::StopLoss)),
            (None, Some(tr)) => Some((tr, BacktestExitReason::TrailingStop)),
            (None, None) => None,
        }
    }

    #[inline]
    fn observe_price(&mut self, px: f64) {
        if self.entry_price <= 0.0 || !px.is_finite() {
            return;
        }
        let rel_pct = self.side.sign() * ((px - self.entry_price) / self.entry_price) * 100.0;
        if rel_pct.is_finite() {
            self.mae_pct = self.mae_pct.min(rel_pct);
            self.mfe_pct = self.mfe_pct.max(rel_pct);
        }
    }

    #[inline]
    fn update_excursions(&mut self, candle: &Candle) {
        self.observe_price(candle.open);
        self.observe_price(candle.low);
        self.observe_price(candle.high);
        self.observe_price(candle.close);
    }
}

pub(crate) struct PrecomputedFeatures {
    sma_fast: Vec<Option<f64>>,
    ema_slow: Vec<Option<f64>>,
    rsi: Vec<Option<f64>>,
    macd: MacdOutput,
    macd_trend_ema: Vec<Option<f64>>,
    bb: BollingerOutput,
    donchian_entry_high: Vec<Option<f64>>,
    donchian_entry_low: Vec<Option<f64>>,
    donchian_exit_high: Vec<Option<f64>>,
    donchian_exit_low: Vec<Option<f64>>,
    atr: Vec<Option<f64>>,
    catalog_matrix: Option<Arc<PrecomputedCatalogMatrix>>,
}

impl PrecomputedFeatures {
    pub(crate) fn build_with_shared_catalog(
        candles: &[Candle],
        config: &BacktestConfig,
        shared_catalog: Option<Arc<PrecomputedCatalogMatrix>>,
    ) -> Self {
        let len = candles.len();
        let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();
        let p = &config.params;

        let need_atr = config.risk.stop_loss_atr_mult.is_some()
            || config.risk.trailing_stop_atr_mult.is_some()
            || config.preset == StrategyPresetKind::DonchianAtrBreakout;

        let sma_fast = if config.preset == StrategyPresetKind::SmaEmaTrendCross {
            compute_sma_slices(&closes, p.fast_sma)
        } else {
            Vec::new()
        };
        let ema_slow = if config.preset == StrategyPresetKind::SmaEmaTrendCross {
            compute_ema_slices(&closes, p.slow_ema)
        } else {
            Vec::new()
        };
        let rsi = if matches!(
            config.preset,
            StrategyPresetKind::RsiMeanReversion | StrategyPresetKind::RsiBollingerConfluence
        ) {
            compute_rsi_slices(&closes, p.rsi_period)
        } else {
            Vec::new()
        };
        let macd = if config.preset == StrategyPresetKind::MacdMomentum {
            compute_macd_slices(&closes, p.macd_fast, p.macd_slow, p.macd_signal)
        } else {
            MacdOutput::default()
        };
        let macd_trend_ema =
            if config.preset == StrategyPresetKind::MacdMomentum && p.macd_trend_ema >= 2 {
                compute_ema_slices(&closes, p.macd_trend_ema)
            } else {
                Vec::new()
            };
        let bb = if matches!(
            config.preset,
            StrategyPresetKind::BollingerSqueezeBreakout
                | StrategyPresetKind::RsiBollingerConfluence
        ) {
            compute_bollinger_slices(&closes, p.bb_period, p.bb_std_mult)
        } else {
            BollingerOutput::default()
        };

        let (donchian_entry_high, donchian_entry_low, donchian_exit_high, donchian_exit_low) =
            if config.preset == StrategyPresetKind::DonchianAtrBreakout && len > 0 {
                let highs: Vec<f64> = candles.iter().map(|c| c.high).collect();
                let lows: Vec<f64> = candles.iter().map(|c| c.low).collect();
                (
                    compute_prior_window_max(&highs, p.donchian_entry_period),
                    compute_prior_window_min(&lows, p.donchian_entry_period),
                    compute_prior_window_max(&highs, p.donchian_exit_period),
                    compute_prior_window_min(&lows, p.donchian_exit_period),
                )
            } else {
                (Vec::new(), Vec::new(), Vec::new(), Vec::new())
            };

        let atr = if need_atr {
            compute_atr(candles, p.atr_period)
        } else {
            Vec::new()
        };

        let catalog_matrix = if config.preset == StrategyPresetKind::CustomSignalRules && len > 0 {
            match shared_catalog {
                Some(arc) if arc.bar_count == len => Some(arc),
                _ => Some(Arc::new(PrecomputedCatalogMatrix::compute(candles))),
            }
        } else {
            None
        };

        Self {
            sma_fast,
            ema_slow,
            rsi,
            macd,
            macd_trend_ema,
            bb,
            donchian_entry_high,
            donchian_entry_low,
            donchian_exit_high,
            donchian_exit_low,
            atr,
            catalog_matrix,
        }
    }
}

pub(crate) struct WindowSimulationOutput {
    pub(crate) final_cash: f64,
    pub(crate) equity_curve: Vec<BacktestEquityPoint>,
    pub(crate) trades: Vec<BacktestTrade>,
    pub(crate) open_position: Option<OpenBacktestPosition>,
    pub(crate) unexecuted_signals: Vec<UnexecutedSignal>,
}

/// Runs a deterministic, zero-lookahead strategy backtest on `candles`.
#[must_use]
pub fn run_backtest(
    symbol: impl Into<String>,
    timeframe: Timeframe,
    candles: &[Candle],
    config: &BacktestConfig,
    replay_cutoff_timestamp: Option<i64>,
) -> BacktestRunResult {
    run_backtest_with_options(
        symbol,
        timeframe,
        candles,
        config,
        BacktestRunOptions {
            replay_cutoff_timestamp,
            incomplete_tail_bar: false,
        },
    )
}

/// Runs a deterministic strategy backtest with explicit causal [`BacktestRunOptions`].
#[must_use]
pub fn run_backtest_with_options(
    symbol: impl Into<String>,
    timeframe: Timeframe,
    candles: &[Candle],
    config: &BacktestConfig,
    options: BacktestRunOptions,
) -> BacktestRunResult {
    run_backtest_with_shared_catalog(symbol, timeframe, candles, config, options, None)
}

fn run_backtest_with_shared_catalog(
    symbol: impl Into<String>,
    timeframe: Timeframe,
    candles: &[Candle],
    config: &BacktestConfig,
    options: BacktestRunOptions,
    shared_catalog: Option<Arc<PrecomputedCatalogMatrix>>,
) -> BacktestRunResult {
    let symbol = symbol.into();
    let mut clean_cfg = config.clone();
    clean_cfg.sanitize();

    let causal_candles =
        slice_causal_series_for_symbol_and_options(Some(&symbol), candles, timeframe, options);
    if causal_candles.is_empty() {
        return BacktestRunResult {
            symbol,
            timeframe,
            tearsheet: BacktestTearsheet::empty(clean_cfg.initial_cash),
            config: clean_cfg,
            bar_count: 0,
            first_timestamp: None,
            last_timestamp: None,
            equity_curve: Vec::new(),
            equity_values: Vec::new(),
            equity_idx: SeriesBlockIndex::default(),
            underwater_values: Vec::new(),
            underwater_idx: SeriesBlockIndex::default(),
            trades: Vec::new(),
            open_position: None,
            unexecuted_signals: Vec::new(),
            split_bar_index: None,
            in_sample_tearsheet: None,
            out_of_sample_tearsheet: None,
            robustness: None,
        };
    }

    let features =
        PrecomputedFeatures::build_with_shared_catalog(causal_candles, &clean_cfg, shared_catalog);
    let full_sim = simulate_window_with_features(
        causal_candles,
        0,
        causal_candles.len(),
        timeframe,
        &clean_cfg,
        &features,
        options.incomplete_tail_bar,
    );

    let open_entry_fee = full_sim
        .open_position
        .as_ref()
        .map_or(0.0, |pos| pos.entry_fee);
    let tearsheet = compute_tearsheet(
        clean_cfg.initial_cash,
        full_sim.final_cash,
        timeframe,
        &full_sim.equity_curve,
        &full_sim.trades,
        open_entry_fee,
    );

    let mut equity_values = Vec::with_capacity(full_sim.equity_curve.len());
    let mut underwater_values = Vec::with_capacity(full_sim.equity_curve.len());
    for pt in &full_sim.equity_curve {
        equity_values.push(Some(pt.equity));
        underwater_values.push(Some(pt.underwater_pct));
    }
    let equity_idx = SeriesBlockIndex::build(&equity_values);
    let underwater_idx = SeriesBlockIndex::build(&underwater_values);

    // Continuous causal In-Sample / Out-of-Sample (`IS / OOS`) partition directly on `full_sim`
    // (`[CRITICAL-02]`: preserves 100% of cross-boundary trades, `Close[split-1] -> Open[split]` fills,
    // and geometric equity compounding `(1 + IS%) * (1 + OOS%) == (1 + Full%)` matching the Equity pane!).
    let (split_bar_index, in_sample_tearsheet, out_of_sample_tearsheet, robustness) =
        if let Some(split_pct) = clean_cfg
            .in_sample_split_pct
            .filter(|_| causal_candles.len() >= 10)
        {
            let split_idx = ((causal_candles.len() as f64 * split_pct).round() as usize)
                .clamp(4, causal_candles.len() - 4);
            let is_curve = &full_sim.equity_curve[..split_idx];
            let oos_curve = &full_sim.equity_curve[split_idx..];

            let is_final_eq = is_curve
                .last()
                .map_or(clean_cfg.initial_cash, |pt| pt.equity);
            let is_final_cash = is_curve.last().map_or(clean_cfg.initial_cash, |pt| pt.cash);

            let trade_split_pos = full_sim
                .trades
                .partition_point(|t| t.exit_bar_index < split_idx);
            let is_trades = &full_sim.trades[..trade_split_pos];
            let oos_trades = &full_sim.trades[trade_split_pos..];

            let (is_open_fee, oos_open_fee) = match full_sim.open_position.as_ref() {
                Some(pos) if pos.entry_bar_index < split_idx => (pos.entry_fee, 0.0),
                Some(pos) => (0.0, pos.entry_fee),
                None => (0.0, 0.0),
            };

            let is_sheet = compute_tearsheet(
                clean_cfg.initial_cash,
                is_final_cash,
                timeframe,
                is_curve,
                is_trades,
                is_open_fee,
            );
            let oos_sheet = compute_tearsheet(
                is_final_eq,
                full_sim.final_cash,
                timeframe,
                oos_curve,
                oos_trades,
                oos_open_fee,
            );
            let verdict = evaluate_robustness(&is_sheet, &oos_sheet);
            (
                Some(split_idx),
                Some(is_sheet),
                Some(oos_sheet),
                Some(verdict),
            )
        } else {
            (None, None, None, None)
        };

    BacktestRunResult {
        symbol,
        timeframe,
        config: clean_cfg,
        bar_count: causal_candles.len(),
        first_timestamp: causal_candles.first().map(|c| c.timestamp),
        last_timestamp: causal_candles.last().map(|c| c.timestamp),
        equity_curve: full_sim.equity_curve,
        equity_values,
        equity_idx,
        underwater_values,
        underwater_idx,
        trades: full_sim.trades,
        open_position: full_sim.open_position,
        unexecuted_signals: full_sim.unexecuted_signals,
        tearsheet,
        split_bar_index,
        in_sample_tearsheet,
        out_of_sample_tearsheet,
        robustness,
    }
}

/// Evaluates a deterministic 12-variant parameter grid sweep ranked by `ranking_metric`.
#[must_use]
pub fn run_parameter_sweep(
    symbol: impl Into<String>,
    timeframe: Timeframe,
    candles: &[Candle],
    base_config: &BacktestConfig,
    ranking_metric: SweepRankingMetric,
    options: BacktestRunOptions,
) -> ParameterSweepReport {
    let symbol = symbol.into();
    let mut clean_base = base_config.clone();
    clean_base.sanitize();
    let causal_candles =
        slice_causal_series_for_symbol_and_options(Some(&symbol), candles, timeframe, options);

    let grid = build_preset_sweep_configs(&clean_base);
    let shared_catalog = if clean_base.preset == StrategyPresetKind::CustomSignalRules
        && !causal_candles.is_empty()
    {
        Some(Arc::new(PrecomputedCatalogMatrix::compute(causal_candles)))
    } else {
        None
    };
    let mut variants = Vec::with_capacity(grid.len());
    for (label, cfg) in grid {
        let res = run_backtest_with_shared_catalog(
            symbol.clone(),
            timeframe,
            causal_candles,
            &cfg,
            BacktestRunOptions {
                replay_cutoff_timestamp: None,
                incomplete_tail_bar: options.incomplete_tail_bar,
            },
            shared_catalog.clone(),
        );
        variants.push(SweepVariantRow {
            rank: 0,
            label,
            config: res.config,
            in_sample_return_pct: res.in_sample_tearsheet.as_ref().map(|t| t.total_return_pct),
            out_of_sample_return_pct: res
                .out_of_sample_tearsheet
                .as_ref()
                .map(|t| t.total_return_pct),
            robustness: res.robustness,
            tearsheet: res.tearsheet,
        });
    }

    variants.sort_by(|a, b| {
        compare_tearsheets_desc(&a.tearsheet, &b.tearsheet, ranking_metric)
            .then_with(|| a.label.cmp(&b.label))
    });
    for (i, row) in variants.iter_mut().enumerate() {
        row.rank = i + 1;
    }

    ParameterSweepReport {
        symbol,
        timeframe,
        preset: clean_base.preset,
        ranking_metric,
        variants,
    }
}

/// Compares all 6 [`StrategyPresetKind`] presets on `candles` under identical risk/fee assumptions.
#[must_use]
pub fn run_all_presets_battle(
    symbol: impl Into<String>,
    timeframe: Timeframe,
    candles: &[Candle],
    base_config: &BacktestConfig,
    ranking_metric: SweepRankingMetric,
    options: BacktestRunOptions,
) -> Vec<PresetBattleRow> {
    let symbol = symbol.into();
    let causal_candles =
        slice_causal_series_for_symbol_and_options(Some(&symbol), candles, timeframe, options);
    let mut rows = Vec::with_capacity(StrategyPresetKind::ALL.len());

    for preset in StrategyPresetKind::ALL {
        let mut cfg = base_config.clone();
        cfg.preset = preset;
        cfg.sanitize();
        let param_summary = cfg.param_summary();
        let res = run_backtest_with_options(
            symbol.clone(),
            timeframe,
            causal_candles,
            &cfg,
            BacktestRunOptions {
                replay_cutoff_timestamp: None,
                incomplete_tail_bar: options.incomplete_tail_bar,
            },
        );
        rows.push(PresetBattleRow {
            rank: 0,
            preset,
            param_summary,
            tearsheet: res.tearsheet,
            robustness: res.robustness,
        });
    }

    rows.sort_by(|a, b| {
        compare_tearsheets_desc(&a.tearsheet, &b.tearsheet, ranking_metric)
            .then_with(|| a.preset.short_label().cmp(b.preset.short_label()))
    });
    for (i, row) in rows.iter_mut().enumerate() {
        row.rank = i + 1;
    }
    rows
}

/// Returns `true` when a causal replay frontier whose completed base bar closed at `causal_close_ts`
/// (`cursor_open_ts + base_tf.seconds() - 1`) has **not yet** reached the closing boundary of `active_tf`
/// (`[MAJOR-04]`: pure prefix function of `(bar_open_ts, causal_close_ts, base_tf, active_tf)` — never peeks at `idx + 1`!).
#[inline]
#[must_use]
pub fn is_bucket_incomplete_at_close(
    bar_open_ts: i64,
    causal_close_ts: i64,
    base_tf: Timeframe,
    active_tf: Timeframe,
) -> bool {
    is_bucket_incomplete_at_close_for_symbol("", bar_open_ts, causal_close_ts, base_tf, active_tf)
}

/// Exchange-schedule-aware causal bucket-close check (`DEF-001`, `[CRITICAL-R16-01]`):
/// - Pure prefix function of `(symbol, bar_open_ts, causal_close_ts, base_tf, active_tf)` — never peeks at `idx + 1`!
/// - Computes `D1 -> W1` weekday from the `D1` bar's open day (`last_base_open_ts`) so NYSE/SPY `14:30:00` UTC
///   Thursday bars (`close = Fri 14:29:59 UTC`) remain `incomplete = true` until Friday's session!
/// - Recognizes Vietnam exchange intraday session closes (`04:29:59` UTC morning close for `04:00` `1h` bucket,
///   and `07:44:59` UTC afternoon close for `1h`, `4h`, `1D`, and Friday `1W` buckets on `VN30F1M` / `HOSE` / `HNX` / `UPCOM`),
///   while preserving 24/7 UTC boundaries on `CryptoSpotPerp` (`BTCUSDT`).
#[must_use]
pub fn is_bucket_incomplete_at_close_for_symbol(
    symbol: &str,
    bar_open_ts: i64,
    causal_close_ts: i64,
    base_tf: Timeframe,
    active_tf: Timeframe,
) -> bool {
    use crate::journal::ExchangeVenue;

    if active_tf <= base_tf {
        return false;
    }
    let next_instant = causal_close_ts.saturating_add(1);
    if active_tf.bucket_start_timestamp(next_instant)
        != active_tf.bucket_start_timestamp(bar_open_ts)
    {
        return false;
    }

    let symbol_trimmed = symbol.trim();
    let venue = if symbol_trimmed.is_empty() {
        None
    } else {
        Some(ExchangeVenue::infer_from_symbol_and_preset(
            symbol_trimmed,
            DataSourcePreset::CustomFile,
        ))
    };
    let last_base_open_ts =
        causal_close_ts.saturating_sub(base_tf.seconds().max(1).saturating_sub(1));

    if active_tf == Timeframe::W1
        && base_tf == Timeframe::D1
        && venue != Some(ExchangeVenue::CryptoSpotPerp)
    {
        // Use `last_base_open_ts` (NOT `causal_close_ts`) so non-midnight `D1` feeds like `SPY` (`14:30:00` UTC)
        // evaluate Thursday (`dow_mon0 == 3`) as `incomplete = true` and Friday (`dow_mon0 == 4`) as `false`.
        let days_since_epoch = last_base_open_ts.div_euclid(86_400);
        let dow_mon0 = (days_since_epoch + 3).rem_euclid(7);
        return dow_mon0 < 4;
    }

    if base_tf.seconds() < 86_400 {
        let Some(venue) = venue else {
            // No exchange symbol provided (`is_bucket_incomplete_at_close`): enforce pure UTC bucket boundary.
            return true;
        };
        let bar_day = bar_open_ts.div_euclid(86_400);
        let close_day = causal_close_ts.div_euclid(86_400);
        let bar_tod = bar_open_ts.rem_euclid(86_400);
        let last_base_tod = last_base_open_ts.rem_euclid(86_400);
        let tod_close = causal_close_ts.rem_euclid(86_400);

        let is_vn_session = matches!(
            venue,
            ExchangeVenue::Vn30Futures
                | ExchangeVenue::Hose
                | ExchangeVenue::Hnx
                | ExchangeVenue::Upcom
                | ExchangeVenue::VnIndexBenchmark
        );
        if is_vn_session {
            const VN_MORNING_CLOSE_TOD_UTC: i64 = 4 * 3_600 + 29 * 60 + 59; // 04:29:59 UTC (11:30 ICT)
            const VN_AFTERNOON_OPEN_TOD_UTC: i64 = 6 * 3_600; // 06:00:00 UTC (13:00 ICT)
            const VN_AFTERNOON_CLOSE_TOD_UTC: i64 = 7 * 3_600 + 44 * 60 + 59; // 07:44:59 UTC (14:45 ICT)
            const VN_AFTERNOON_CLOSE_END_TOD_UTC: i64 = 8 * 3_600; // 08:00:00 UTC (15:00 ICT)

            // Morning 1h (`04:00..04:29:59` UTC) has zero trading bars during 11:30..13:00 ICT lunch break:
            if active_tf == Timeframe::H1
                && close_day == bar_day
                && bar_tod == 4 * 3_600
                && (VN_MORNING_CLOSE_TOD_UTC..VN_AFTERNOON_OPEN_TOD_UTC).contains(&tod_close)
            {
                return false;
            }

            // Afternoon session close bounded strictly to `07:44:59..08:00:00` UTC (`14:45..15:00` ICT)
            // on buckets opened at or before `07:44:59` UTC (never leaking mid-bucket bars after `08:00` UTC!):
            if close_day == bar_day
                && bar_tod <= VN_AFTERNOON_CLOSE_TOD_UTC
                && (VN_AFTERNOON_CLOSE_TOD_UTC..VN_AFTERNOON_CLOSE_END_TOD_UTC).contains(&tod_close)
                && matches!(active_tf, Timeframe::H1 | Timeframe::H4 | Timeframe::D1)
            {
                return false;
            }
            if active_tf == Timeframe::W1
                && Timeframe::W1.bucket_start_timestamp(causal_close_ts)
                    == Timeframe::W1.bucket_start_timestamp(bar_open_ts)
            {
                let dow_mon0 = (last_base_open_ts.div_euclid(86_400) + 3).rem_euclid(7);
                if dow_mon0 == 4
                    && last_base_tod <= VN_AFTERNOON_CLOSE_TOD_UTC
                    && (VN_AFTERNOON_CLOSE_TOD_UTC..VN_AFTERNOON_CLOSE_END_TOD_UTC)
                        .contains(&tod_close)
                {
                    return false;
                }
            }
        } else if venue == ExchangeVenue::UsEquities {
            const US_REGULAR_CLOSE_TOD_UTC: i64 = 20 * 3_600 + 59 * 60 + 59; // 20:59:59 UTC (16:00 EST)
            const US_REGULAR_CLOSE_END_TOD_UTC: i64 = 21 * 3_600; // 21:00:00 UTC

            if close_day == bar_day
                && bar_tod <= US_REGULAR_CLOSE_TOD_UTC
                && (US_REGULAR_CLOSE_TOD_UTC..US_REGULAR_CLOSE_END_TOD_UTC).contains(&tod_close)
                && matches!(active_tf, Timeframe::H1 | Timeframe::H4 | Timeframe::D1)
            {
                return false;
            }
            if active_tf == Timeframe::W1
                && Timeframe::W1.bucket_start_timestamp(causal_close_ts)
                    == Timeframe::W1.bucket_start_timestamp(bar_open_ts)
            {
                let dow_mon0 = (last_base_open_ts.div_euclid(86_400) + 3).rem_euclid(7);
                if dow_mon0 == 4
                    && last_base_tod <= US_REGULAR_CLOSE_TOD_UTC
                    && (US_REGULAR_CLOSE_TOD_UTC..US_REGULAR_CLOSE_END_TOD_UTC).contains(&tod_close)
                {
                    return false;
                }
            }
        }
    }

    true
}

/// Runs `config` causally across every feed in `feeds` using a bar-open `replay_cutoff_timestamp`.
///
/// For mixed-timeframe universes during active Bar Replay, prefer [`run_multi_symbol_battle_with_close_cutoff`]
/// passing the exact inclusive causal close instant `cursor_open_ts + active_base_tf.seconds() - 1`.
#[must_use]
pub fn run_multi_symbol_battle(
    feeds: &[WatchlistFeedEntry],
    preferred_timeframe: Timeframe,
    config: &BacktestConfig,
    ranking_metric: SweepRankingMetric,
    replay_cutoff_timestamp: Option<i64>,
) -> MultiSymbolBattleReport {
    let inferred_close_cutoff = replay_cutoff_timestamp.map(|ts| {
        if ts.rem_euclid(86_400) == 0 && preferred_timeframe >= Timeframe::D1 {
            ts.saturating_add(86_399)
        } else {
            ts.saturating_add(59)
        }
    });
    run_multi_symbol_battle_with_close_cutoff(
        feeds,
        preferred_timeframe,
        config,
        ranking_metric,
        inferred_close_cutoff,
    )
}

/// Runs `config` causally across every feed in `feeds` strictly gated by `replay_causal_close_timestamp`
/// (`[CRITICAL-01]` `DEF-001` cross-timeframe guard: coarse daily/15m bars whose closing instant exceeds
/// `replay_causal_close_timestamp` are withheld completely so future afternoon/intrabar prices never leak!).
#[must_use]
pub fn run_multi_symbol_battle_with_close_cutoff(
    feeds: &[WatchlistFeedEntry],
    preferred_timeframe: Timeframe,
    config: &BacktestConfig,
    ranking_metric: SweepRankingMetric,
    replay_causal_close_timestamp: Option<i64>,
) -> MultiSymbolBattleReport {
    let mut clean_cfg = config.clone();
    clean_cfg.sanitize();

    let mut rows = Vec::with_capacity(feeds.len());
    for feed in feeds {
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
            (Some(close_cut), Some(last_bar)) => {
                let last_feed_close_ts = closed_base_slice
                    .last()
                    .map_or(close_cut, |c| c.timestamp.saturating_add(base_bar_span));
                is_bucket_incomplete_at_close_for_symbol(
                    &feed.symbol,
                    last_bar.timestamp,
                    close_cut,
                    feed.base_timeframe,
                    effective_tf,
                ) && is_bucket_incomplete_at_close_for_symbol(
                    &feed.symbol,
                    last_bar.timestamp,
                    last_feed_close_ts,
                    feed.base_timeframe,
                    effective_tf,
                )
            }
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
        rows.push(SymbolBattleRow {
            rank: 0,
            instrument_id: feed.instrument_id.clone(),
            symbol: feed.symbol.clone(),
            source_label: feed.preset.short_label().to_owned(),
            timeframe: effective_tf,
            bar_count: res.bar_count,
            tearsheet: res.tearsheet,
            robustness: res.robustness,
        });
    }

    rows.sort_by(|a, b| {
        compare_tearsheets_desc(&a.tearsheet, &b.tearsheet, ranking_metric)
            .then_with(|| a.symbol.cmp(&b.symbol))
            .then_with(|| a.instrument_id.cmp(&b.instrument_id))
    });

    let mut profitable_symbols = 0_usize;
    let mut sum_return_pct = 0.0_f64;
    for (i, row) in rows.iter_mut().enumerate() {
        row.rank = i + 1;
        if row.tearsheet.total_return_pct > 0.0 {
            profitable_symbols += 1;
        }
        sum_return_pct += row.tearsheet.total_return_pct;
    }
    let average_return_pct = if rows.is_empty() {
        0.0
    } else {
        sum_return_pct / (rows.len() as f64)
    };

    MultiSymbolBattleReport {
        preset: clean_cfg.preset,
        ranking_metric,
        rows,
        profitable_symbols,
        average_return_pct,
    }
}

pub(crate) fn simulate_window_with_features(
    candles: &[Candle],
    start_idx: usize,
    end_idx: usize,
    timeframe: Timeframe,
    config: &BacktestConfig,
    features: &PrecomputedFeatures,
    incomplete_tail_bar: bool,
) -> WindowSimulationOutput {
    let (out, _) = simulate_window_with_carry(
        candles,
        start_idx,
        end_idx,
        timeframe,
        config,
        features,
        incomplete_tail_bar,
        None,
        false,
        false,
    );
    out
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn simulate_window_with_carry(
    candles: &[Candle],
    start_idx: usize,
    end_idx: usize,
    timeframe: Timeframe,
    config: &BacktestConfig,
    features: &PrecomputedFeatures,
    incomplete_tail_bar: bool,
    carry_in: Option<SimulationCarryState>,
    allow_handoff_at_end: bool,
    use_absolute_indices: bool,
) -> (WindowSimulationOutput, SimulationCarryState) {
    let end = end_idx.min(candles.len());
    let start = start_idx.min(end);
    let span = end.saturating_sub(start);
    let min_hold_bars = effective_min_holding_bars(config.risk.min_holding_bars, timeframe);

    let init = carry_in.unwrap_or_else(|| SimulationCarryState::new(config.initial_cash));
    let mut cash = init.cash;
    let mut peak_equity = init.peak_equity;
    let mut active_pos: Option<ActivePositionState> = init.active_pos;
    let mut pending_order: Option<PendingStrategyOrder> = init.pending_order;
    let mut next_trade_id = init.next_trade_id;

    let mut equity_curve = Vec::with_capacity(span);
    let mut trades = Vec::new();
    let mut unexecuted_signals = Vec::new();

    for i in start..end {
        let candle = &candles[i];
        let out_bar_idx = if use_absolute_indices { i } else { i - start };

        // STEP A: Execute pending close-of-bar signal from `i - 1` at `Open[i]` (`BT-TIME-001`).
        if let Some(order) = pending_order.take() {
            match order {
                PendingStrategyOrder::Exit {
                    signal_bar: _,
                    reason,
                } => {
                    if let Some(mut pos) = active_pos.take() {
                        let exit_side = pos.side.exit_order_side();
                        let exec_price =
                            apply_slippage(candle.open, exit_side, config.slippage_bps);
                        pos.observe_price(candle.open);
                        pos.observe_price(exec_price);
                        close_position_into_trade(
                            pos,
                            i,
                            out_bar_idx,
                            use_absolute_indices,
                            candle.timestamp,
                            exec_price,
                            reason,
                            config,
                            &mut cash,
                            &mut next_trade_id,
                            &mut trades,
                        );
                    }
                }
                PendingStrategyOrder::Reverse {
                    new_side,
                    signal_bar,
                } => {
                    if let Some(mut pos) = active_pos.take() {
                        let exit_side = pos.side.exit_order_side();
                        let exec_price =
                            apply_slippage(candle.open, exit_side, config.slippage_bps);
                        pos.observe_price(candle.open);
                        pos.observe_price(exec_price);
                        close_position_into_trade(
                            pos,
                            i,
                            out_bar_idx,
                            use_absolute_indices,
                            candle.timestamp,
                            exec_price,
                            BacktestExitReason::SignalReversal,
                            config,
                            &mut cash,
                            &mut next_trade_id,
                            &mut trades,
                        );
                    }
                    if config.direction_mode.allows_side(new_side) {
                        try_open_position_at_open(
                            new_side,
                            signal_bar.saturating_sub(start),
                            i,
                            start,
                            candle,
                            config,
                            features,
                            &mut cash,
                            &mut active_pos,
                            &mut unexecuted_signals,
                        );
                    }
                }
                PendingStrategyOrder::Entry { side, signal_bar } => {
                    if active_pos.is_none() && config.direction_mode.allows_side(side) {
                        try_open_position_at_open(
                            side,
                            signal_bar.saturating_sub(start),
                            i,
                            start,
                            candle,
                            config,
                            features,
                            &mut cash,
                            &mut active_pos,
                            &mut unexecuted_signals,
                        );
                    }
                }
            }
        }

        // STEP B: Evaluate Margin Insolvency (`[CRITICAL-03]`) & Intrabar Protective `SL`, `TP`, `TrailingStop`.
        if let Some(mut pos) = active_pos.take() {
            let bars_held_so_far = i.saturating_sub(pos.entry_bar_abs);
            let is_after_entry_bar = i > pos.entry_bar_abs;
            let exit_side = pos.side.exit_order_side();
            let voluntary_unlocked = bars_held_so_far >= min_hold_bars;
            let eff_stop = if voluntary_unlocked {
                pos.effective_stop()
            } else {
                None
            };
            let tp_opt = if voluntary_unlocked {
                pos.take_profit
            } else {
                None
            };
            let liq_raw_opt = compute_bankruptcy_raw_price(&pos, cash, config);

            // Combine protective `eff_stop` and involuntary `MarginLiquidation` into the tighter adverse stop!
            let effective_adverse_stop: Option<(f64, BacktestExitReason)> =
                match (eff_stop, liq_raw_opt) {
                    (Some((sl_px, sl_reason)), Some(liq_px)) => match pos.side {
                        PositionSide::Long => {
                            if liq_px >= sl_px {
                                Some((liq_px, BacktestExitReason::MarginLiquidation))
                            } else {
                                Some((sl_px, sl_reason))
                            }
                        }
                        PositionSide::Short => {
                            if liq_px <= sl_px {
                                Some((liq_px, BacktestExitReason::MarginLiquidation))
                            } else {
                                Some((sl_px, sl_reason))
                            }
                        }
                    },
                    (Some(s), None) => Some(s),
                    (None, Some(liq_px)) => Some((liq_px, BacktestExitReason::MarginLiquidation)),
                    (None, None) => None,
                };

            // 1. Gap-Open check at `Open[i]` (only when `i > pos.entry_bar_abs`; at `i == entry_bar_abs`, `Open[i]` was the entry fill!).
            let gap_trigger: Option<(f64, BacktestExitReason)> = if is_after_entry_bar {
                let open_exec = apply_slippage(candle.open, exit_side, config.slippage_bps);
                let open_insolvent = net_equity_after_exit(&pos, cash, open_exec, config) <= 0.0;
                match pos.side {
                    PositionSide::Long => {
                        if let Some((_, reason)) =
                            effective_adverse_stop.filter(|(px, _)| candle.open <= *px)
                        {
                            let final_reason = if open_insolvent {
                                BacktestExitReason::MarginLiquidation
                            } else {
                                reason
                            };
                            Some((candle.open, final_reason))
                        } else if open_insolvent {
                            Some((candle.open, final_reason_or_liq()))
                        } else if tp_opt.is_some_and(|px| candle.open >= px) {
                            Some((candle.open, BacktestExitReason::TakeProfit))
                        } else {
                            None
                        }
                    }
                    PositionSide::Short => {
                        if let Some((_, reason)) =
                            effective_adverse_stop.filter(|(px, _)| candle.open >= *px)
                        {
                            let final_reason = if open_insolvent {
                                BacktestExitReason::MarginLiquidation
                            } else {
                                reason
                            };
                            Some((candle.open, final_reason))
                        } else if open_insolvent {
                            Some((candle.open, BacktestExitReason::MarginLiquidation))
                        } else if tp_opt.is_some_and(|px| candle.open <= px) {
                            Some((candle.open, BacktestExitReason::TakeProfit))
                        } else {
                            None
                        }
                    }
                }
            } else {
                None
            };

            if let Some((gap_raw_px, reason)) = gap_trigger {
                let exec_price = apply_slippage(gap_raw_px, exit_side, config.slippage_bps);
                pos.observe_price(candle.open);
                pos.observe_price(exec_price);
                close_position_into_trade(
                    pos,
                    i,
                    out_bar_idx,
                    use_absolute_indices,
                    candle.timestamp,
                    exec_price,
                    reason,
                    config,
                    &mut cash,
                    &mut next_trade_id,
                    &mut trades,
                );
            } else {
                if is_after_entry_bar {
                    pos.observe_price(candle.open);
                }
                // 2. Pessimistic intrabar `[Low, High]` check (`SL`/`Liq` takes strict precedence on dual-touch bars!).
                let intrabar_trigger = match pos.side {
                    PositionSide::Long => {
                        let moved_adversely = is_after_entry_bar || candle.low < candle.open;
                        if moved_adversely
                            && let Some((sl_px, reason)) =
                                effective_adverse_stop.filter(|(px, _)| candle.low <= *px)
                        {
                            Some((sl_px, reason, true))
                        } else {
                            tp_opt
                                .filter(|px| candle.high >= *px)
                                .map(|tp_px| (tp_px, BacktestExitReason::TakeProfit, false))
                        }
                    }
                    PositionSide::Short => {
                        let moved_adversely = is_after_entry_bar || candle.high > candle.open;
                        if moved_adversely
                            && let Some((sl_px, reason)) =
                                effective_adverse_stop.filter(|(px, _)| candle.high >= *px)
                        {
                            Some((sl_px, reason, true))
                        } else {
                            tp_opt
                                .filter(|px| candle.low <= *px)
                                .map(|tp_px| (tp_px, BacktestExitReason::TakeProfit, false))
                        }
                    }
                };

                if let Some((raw_px, reason, is_adverse_exit)) = intrabar_trigger {
                    if !is_adverse_exit {
                        match pos.side {
                            PositionSide::Long => pos.observe_price(candle.low),
                            PositionSide::Short => pos.observe_price(candle.high),
                        }
                    }
                    pos.observe_price(raw_px);
                    let exec_price = apply_slippage(raw_px, exit_side, config.slippage_bps);
                    pos.observe_price(exec_price);
                    let final_reason =
                        if net_equity_after_exit(&pos, cash, exec_price, config) <= 0.0 {
                            BacktestExitReason::MarginLiquidation
                        } else {
                            reason
                        };
                    close_position_into_trade(
                        pos,
                        i,
                        out_bar_idx,
                        use_absolute_indices,
                        candle.timestamp,
                        exec_price,
                        final_reason,
                        config,
                        &mut cash,
                        &mut next_trade_id,
                        &mut trades,
                    );
                } else {
                    pos.update_excursions(candle);
                    ratchet_trailing_stop_post_close(&mut pos, candle, i, config, features);
                    active_pos = Some(pos);
                }
            }
        }

        // STEP C: Record bar-aligned mark-to-market `BacktestEquityPoint` at `Close[i]`.
        let (unrealized, pos_signed_qty) = match active_pos.as_ref() {
            Some(pos) => (
                pos.unrealized_pnl(candle.close),
                pos.side.sign() * pos.quantity,
            ),
            None => (0.0, 0.0),
        };
        let equity = (cash + unrealized).max(0.0);
        if equity >= peak_equity {
            peak_equity = equity;
        }
        let drawdown_abs = (peak_equity - equity).max(0.0);
        let underwater_pct = if peak_equity > 0.0 && equity < peak_equity {
            ((equity - peak_equity) / peak_equity) * 100.0
        } else {
            0.0
        };

        equity_curve.push(BacktestEquityPoint {
            bar_index: out_bar_idx,
            timestamp: candle.timestamp,
            equity,
            cash,
            drawdown_abs,
            underwater_pct,
            position_qty: pos_signed_qty,
        });

        // STEP D: Evaluate causal close-of-bar strategy rules at `Close[i]`.
        let is_last_bar_in_window = i + 1 == end;
        if is_last_bar_in_window && incomplete_tail_bar {
            continue;
        }

        let desired_order = evaluate_close_of_bar_order(
            i,
            min_hold_bars,
            candles,
            config,
            features,
            active_pos.as_ref(),
        );
        if let Some(order) = desired_order {
            if is_last_bar_in_window && !allow_handoff_at_end {
                // `TEST-BT-001`: Final causal bar cannot fill on non-existent bar `end`!
                let (side, is_exit) = match order {
                    PendingStrategyOrder::Entry { side, .. } => (side, false),
                    PendingStrategyOrder::Exit { .. } => (
                        active_pos.as_ref().map_or(PositionSide::Long, |p| p.side),
                        true,
                    ),
                    PendingStrategyOrder::Reverse { new_side, .. } => (new_side, false),
                };
                unexecuted_signals.push(UnexecutedSignal {
                    signal_bar_index: out_bar_idx,
                    timestamp: candle.timestamp,
                    side,
                    is_exit,
                    reason: UnfilledReason::EndOfSeriesCutoff,
                });
            } else {
                pending_order = Some(order);
            }
        }
    }

    let last_close = candles.get(end.saturating_sub(1)).map_or(0.0, |c| c.close);
    let open_position = active_pos.as_ref().map(|pos| OpenBacktestPosition {
        side: pos.side,
        quantity: pos.quantity,
        entry_bar_index: if use_absolute_indices {
            pos.entry_bar_abs
        } else {
            pos.entry_bar_index
        },
        entry_timestamp: pos.entry_timestamp,
        entry_price: pos.entry_price,
        entry_fee: pos.entry_fee + pos.entry_sell_tax,
        stop_loss: pos.fixed_stop_loss,
        take_profit: pos.take_profit,
        trailing_stop: pos.trailing_stop,
        unrealized_pnl: pos.unrealized_pnl(last_close),
        unrealized_return_pct: pos.unrealized_return_pct(last_close),
    });

    let carry_out = SimulationCarryState {
        cash,
        peak_equity,
        active_pos,
        pending_order,
        next_trade_id,
    };

    (
        WindowSimulationOutput {
            final_cash: cash,
            equity_curve,
            trades,
            open_position,
            unexecuted_signals,
        },
        carry_out,
    )
}

#[inline]
const fn final_reason_or_liq() -> BacktestExitReason {
    BacktestExitReason::MarginLiquidation
}

#[allow(clippy::too_many_arguments)]
fn try_open_position_at_open(
    side: PositionSide,
    signal_bar_local: usize,
    fill_bar_abs: usize,
    window_start: usize,
    candle: &Candle,
    config: &BacktestConfig,
    features: &PrecomputedFeatures,
    cash: &mut f64,
    active_pos: &mut Option<ActivePositionState>,
    unexecuted_signals: &mut Vec<UnexecutedSignal>,
) {
    let signal_bar = signal_bar_local;
    let fill_bar = fill_bar_abs;
    if !candle.open.is_finite() || candle.open <= 0.0 || candle.open > MAX_ORDER_PRICE {
        unexecuted_signals.push(UnexecutedSignal {
            signal_bar_index: signal_bar,
            timestamp: candle.timestamp,
            side,
            is_exit: false,
            reason: UnfilledReason::InvalidCandlePrice,
        });
        return;
    }
    if *cash <= 0.0 || !cash.is_finite() {
        unexecuted_signals.push(UnexecutedSignal {
            signal_bar_index: signal_bar,
            timestamp: candle.timestamp,
            side,
            is_exit: false,
            reason: UnfilledReason::InsufficientEquity,
        });
        return;
    }

    let order_side = match side {
        PositionSide::Long => OrderSide::Buy,
        PositionSide::Short => OrderSide::Sell,
    };
    let exec_price = apply_slippage(candle.open, order_side, config.slippage_bps);
    if !exec_price.is_finite() || exec_price <= 0.0 || exec_price > MAX_ORDER_PRICE {
        unexecuted_signals.push(UnexecutedSignal {
            signal_bar_index: signal_bar,
            timestamp: candle.timestamp,
            side,
            is_exit: false,
            reason: UnfilledReason::InvalidCandlePrice,
        });
        return;
    }

    // Strictly causal ATR from `fill_bar - 1` (`Close[fill_bar - 1]`) — never `atr[fill_bar]`!
    let prev_atr = fill_bar
        .checked_sub(1)
        .and_then(|pi| features.atr.get(pi).copied().flatten())
        .filter(|v| v.is_finite() && *v > 0.0);

    // Compute fixed SL and initial trailing stop at `Open[fill_bar]`.
    let pct_sl = config.risk.stop_loss_pct.map(|pct| match side {
        PositionSide::Long => (exec_price * (1.0 - pct * 0.01)).max(1.0e-8),
        PositionSide::Short => exec_price * (1.0 + pct * 0.01),
    });
    let atr_sl = config
        .risk
        .stop_loss_atr_mult
        .zip(prev_atr)
        .map(|(mult, atr_v)| match side {
            PositionSide::Long => (exec_price - mult * atr_v).max(1.0e-8),
            PositionSide::Short => exec_price + mult * atr_v,
        });
    let fixed_stop_loss = match (pct_sl, atr_sl) {
        (Some(a), Some(b)) => Some(match side {
            PositionSide::Long => a.max(b),
            PositionSide::Short => a.min(b),
        }),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    };

    let initial_trail = config
        .risk
        .trailing_stop_atr_mult
        .zip(prev_atr)
        .map(|(mult, atr_v)| match side {
            PositionSide::Long => (exec_price - mult * atr_v).max(1.0e-8),
            PositionSide::Short => exec_price + mult * atr_v,
        });

    let effective_initial_stop = match (fixed_stop_loss, initial_trail) {
        (Some(a), Some(b)) => Some(match side {
            PositionSide::Long => a.max(b),
            PositionSide::Short => a.min(b),
        }),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    };

    let unit_stop_dist = effective_initial_stop
        .map(|sl| (exec_price - sl).abs())
        .filter(|d| d.is_finite() && *d > 1.0e-9);

    let entry_fee_rate = (config.commission_bps
        + if order_side == OrderSide::Sell {
            config.tax_sell_bps
        } else {
            0.0
        })
        * 1.0e-4;
    let max_notional_cap =
        ((*cash * config.max_leverage) / (1.0 + entry_fee_rate)).clamp(0.0, MAX_ORDER_NOTIONAL);

    let desired_notional = match config.sizing {
        PositionSizingMode::PercentEquity(pct) => {
            (*cash * (pct * 0.01) * config.max_leverage).min(max_notional_cap)
        }
        PositionSizingMode::RiskPercentPerTrade(risk_pct) => {
            if let Some(unit_dist) = unit_stop_dist {
                let risk_budget = *cash * (risk_pct * 0.01);
                ((risk_budget / unit_dist) * exec_price).min(max_notional_cap)
            } else {
                (*cash * 0.25 * config.max_leverage).min(max_notional_cap)
            }
        }
        PositionSizingMode::FixedNotional(notional) => notional.min(max_notional_cap),
    };

    let mut quantity = (desired_notional / exec_price).min(MAX_ORDER_QUANTITY);
    if config.lot_step >= 1.0 {
        quantity = (quantity / config.lot_step).floor() * config.lot_step;
    }
    if !quantity.is_finite() || quantity < MIN_ORDER_QUANTITY || quantity * exec_price <= 0.0 {
        unexecuted_signals.push(UnexecutedSignal {
            signal_bar_index: signal_bar,
            timestamp: candle.timestamp,
            side,
            is_exit: false,
            reason: UnfilledReason::ZeroSizedLot,
        });
        return;
    }

    let entry_comm = compute_commission(exec_price, quantity, config.commission_bps);
    let entry_sell_tax = if order_side == OrderSide::Sell {
        compute_commission(exec_price, quantity, config.tax_sell_bps)
    } else {
        0.0
    };
    let total_entry_fee = entry_comm + entry_sell_tax;
    if total_entry_fee >= *cash {
        unexecuted_signals.push(UnexecutedSignal {
            signal_bar_index: signal_bar,
            timestamp: candle.timestamp,
            side,
            is_exit: false,
            reason: UnfilledReason::InsufficientEquity,
        });
        return;
    }

    let pct_tp = config.risk.take_profit_pct.map(|pct| match side {
        PositionSide::Long => exec_price * (1.0 + pct * 0.01),
        PositionSide::Short => (exec_price * (1.0 - pct * 0.01)).max(1.0e-8),
    });
    let rr_tp = config
        .risk
        .take_profit_rr_mult
        .zip(unit_stop_dist)
        .map(|(rr, dist)| match side {
            PositionSide::Long => exec_price + rr * dist,
            PositionSide::Short => (exec_price - rr * dist).max(1.0e-8),
        });
    let take_profit = rr_tp.or(pct_tp);
    let initial_risk_amount = unit_stop_dist.map(|d| d * quantity);

    *cash -= total_entry_fee;
    *active_pos = Some(ActivePositionState {
        side,
        quantity,
        entry_bar_abs: fill_bar,
        entry_bar_index: fill_bar.saturating_sub(window_start),
        entry_timestamp: candle.timestamp,
        entry_price: exec_price,
        entry_fee: entry_comm,
        entry_sell_tax,
        fixed_stop_loss,
        take_profit,
        trailing_stop: initial_trail,
        initial_risk_amount,
        highest_since_entry: exec_price,
        lowest_since_entry: exec_price,
        mae_pct: 0.0,
        mfe_pct: 0.0,
    });
}

#[inline]
fn ratchet_trailing_stop_post_close(
    pos: &mut ActivePositionState,
    candle: &Candle,
    bar_idx: usize,
    config: &BacktestConfig,
    features: &PrecomputedFeatures,
) {
    if candle.high.is_finite() {
        pos.highest_since_entry = pos.highest_since_entry.max(candle.high);
    }
    if candle.low.is_finite() {
        pos.lowest_since_entry = pos.lowest_since_entry.min(candle.low);
    }
    if let Some(trail_mult) = config.risk.trailing_stop_atr_mult
        && let Some(atr_val) = features
            .atr
            .get(bar_idx)
            .copied()
            .flatten()
            .filter(|v| v.is_finite() && *v > 0.0)
    {
        match pos.side {
            PositionSide::Long => {
                let cand = (pos.highest_since_entry - trail_mult * atr_val).max(1.0e-8);
                pos.trailing_stop = Some(pos.trailing_stop.map_or(cand, |prev| prev.max(cand)));
            }
            PositionSide::Short => {
                let cand = pos.lowest_since_entry + trail_mult * atr_val;
                pos.trailing_stop = Some(pos.trailing_stop.map_or(cand, |prev| prev.min(cand)));
            }
        }
    }
}

#[inline]
fn net_equity_after_exit(
    pos: &ActivePositionState,
    cash: f64,
    exec_price: f64,
    config: &BacktestConfig,
) -> f64 {
    let exit_side = pos.side.exit_order_side();
    let exit_fee = compute_commission(exec_price, pos.quantity, config.commission_bps);
    let close_sell_tax = if exit_side == OrderSide::Sell {
        compute_commission(exec_price, pos.quantity, config.tax_sell_bps)
    } else {
        0.0
    };
    let gross_pnl = pos.side.sign() * (exec_price - pos.entry_price) * pos.quantity;
    cash + gross_pnl - exit_fee - close_sell_tax
}

/// Computes the exact raw market price at which net post-exit equity reaches `0.0` (`MarginLiquidation`).
#[inline]
fn compute_bankruptcy_raw_price(
    pos: &ActivePositionState,
    cash: f64,
    config: &BacktestConfig,
) -> Option<f64> {
    if pos.quantity <= 0.0 || !cash.is_finite() {
        return None;
    }
    let exit_side = pos.side.exit_order_side();
    let r_fee = (config.commission_bps
        + if exit_side == OrderSide::Sell {
            config.tax_sell_bps
        } else {
            0.0
        })
        * 1.0e-4;
    let slip = config.slippage_bps * 1.0e-4;

    match pos.side {
        PositionSide::Long => {
            // `cash + (exec_px - entry_px) * qty - exec_px * qty * r_fee == 0`
            // where `exec_px = raw_px * (1 - slip)`.
            let numerator = pos.entry_price * pos.quantity - cash;
            if numerator <= 0.0 {
                // Unleveraged position with sufficient remaining cash cannot go negative at any `raw_px > 0`.
                return None;
            }
            let denom = pos.quantity * (1.0 - r_fee).max(1.0e-6) * (1.0 - slip).max(1.0e-6);
            let raw_px = numerator / denom;
            (raw_px.is_finite() && raw_px > 0.0).then_some(raw_px)
        }
        PositionSide::Short => {
            // `cash + (entry_px - exec_px) * qty - exec_px * qty * r_fee == 0`
            // where `exec_px = raw_px * (1 + slip)`.
            let numerator = cash + pos.entry_price * pos.quantity;
            if numerator <= 0.0 {
                return Some(pos.entry_price);
            }
            let denom = pos.quantity * (1.0 + r_fee) * (1.0 + slip);
            let raw_px = numerator / denom;
            (raw_px.is_finite() && raw_px > 0.0).then_some(raw_px)
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn close_position_into_trade(
    pos: ActivePositionState,
    exit_bar_abs: usize,
    exit_bar_out: usize,
    use_absolute_indices: bool,
    exit_timestamp: i64,
    exec_price: f64,
    exit_reason: BacktestExitReason,
    config: &BacktestConfig,
    cash: &mut f64,
    next_trade_id: &mut u64,
    trades: &mut Vec<BacktestTrade>,
) {
    let exit_side = pos.side.exit_order_side();
    let exit_fee = compute_commission(exec_price, pos.quantity, config.commission_bps);
    let close_sell_tax = if exit_side == OrderSide::Sell {
        compute_commission(exec_price, pos.quantity, config.tax_sell_bps)
    } else {
        0.0
    };
    let total_sell_tax = pos.entry_sell_tax + close_sell_tax;
    let gross_pnl = pos.side.sign() * (exec_price - pos.entry_price) * pos.quantity;
    let equity_before_entry = *cash + pos.entry_fee + pos.entry_sell_tax;
    let raw_new_cash = *cash + gross_pnl - exit_fee - close_sell_tax;

    // Zero-balance protection (`[CRITICAL-03]`): settled cash never drops below `0.0`, while
    // `initial_cash + sum(net_pnl) == final_cash` conservation holds bit-exactly!
    let net_pnl = if raw_new_cash <= 0.0 {
        *cash = 0.0;
        -equity_before_entry
    } else {
        *cash = raw_new_cash;
        gross_pnl - pos.entry_fee - exit_fee - total_sell_tax
    };

    let notional_in = pos.entry_price * pos.quantity;
    let return_pct = if notional_in > 0.0 {
        (net_pnl / notional_in) * 100.0
    } else {
        0.0
    };
    let r_multiple = pos
        .initial_risk_amount
        .filter(|r| r.is_finite() && *r > 1.0e-9)
        .map(|r| net_pnl / r);

    let trade_id = *next_trade_id;
    *next_trade_id = next_trade_id.saturating_add(1);
    let entry_bar_out = if use_absolute_indices {
        pos.entry_bar_abs
    } else {
        pos.entry_bar_index
    };

    trades.push(BacktestTrade {
        trade_id,
        side: pos.side,
        quantity: pos.quantity,
        entry_bar_index: entry_bar_out,
        exit_bar_index: exit_bar_out,
        entry_timestamp: pos.entry_timestamp,
        exit_timestamp,
        entry_price: pos.entry_price,
        exit_price: exec_price,
        bars_held: exit_bar_abs.saturating_sub(pos.entry_bar_abs),
        gross_pnl,
        entry_fee: pos.entry_fee,
        exit_fee,
        exit_tax: total_sell_tax,
        net_pnl,
        return_pct,
        r_multiple,
        mae_pct: pos.mae_pct.min(return_pct).min(0.0),
        mfe_pct: pos.mfe_pct.max(return_pct).max(0.0),
        exit_reason,
    });
}

fn evaluate_close_of_bar_order(
    i: usize,
    min_hold_bars: usize,
    candles: &[Candle],
    config: &BacktestConfig,
    features: &PrecomputedFeatures,
    active_pos: Option<&ActivePositionState>,
) -> Option<PendingStrategyOrder> {
    let (bull_entry, bear_entry, long_exit, short_exit) =
        evaluate_preset_signals_at_bar(i, candles, config, features);

    if let Some(pos) = active_pos {
        // Notice: at `Close[i]`, if the exit fills at `Open[i + 1]`, `exit_bar - entry_bar == (i + 1) - entry_bar_abs`.
        let bars_held_at_next_open = (i + 1).saturating_sub(pos.entry_bar_abs);
        if bars_held_at_next_open < min_hold_bars {
            return None;
        }

        // 1. Check opposite reversal signal first when `LongShort` is active.
        let opposite_entry = match pos.side {
            PositionSide::Long => bear_entry,
            PositionSide::Short => bull_entry,
        };
        let opposite_side = match pos.side {
            PositionSide::Long => PositionSide::Short,
            PositionSide::Short => PositionSide::Long,
        };
        if opposite_entry && config.direction_mode == StrategyDirectionMode::LongShort {
            return Some(PendingStrategyOrder::Reverse {
                new_side: opposite_side,
                signal_bar: i,
            });
        }

        // 2. Check strategy exit rule (`CustomSignalRules` keeps all 4 directional legs strictly isolated
        //    so `short_entry` / `long_entry` never leaks into `LongOnly` / `ShortOnly` exits).
        let rule_exit = if config.preset == StrategyPresetKind::CustomSignalRules {
            match pos.side {
                PositionSide::Long => long_exit,
                PositionSide::Short => short_exit,
            }
        } else {
            match pos.side {
                PositionSide::Long => long_exit || bear_entry,
                PositionSide::Short => short_exit || bull_entry,
            }
        };
        if rule_exit {
            return Some(PendingStrategyOrder::Exit {
                signal_bar: i,
                reason: BacktestExitReason::SignalExit,
            });
        }

        // 3. Check `max_holding_bars` timeout (`bars_held_at_next_open >= max_h`).
        if let Some(max_h) = config.risk.max_holding_bars
            && bars_held_at_next_open >= max_h
        {
            return Some(PendingStrategyOrder::Exit {
                signal_bar: i,
                reason: BacktestExitReason::MaxBarsExit,
            });
        }

        None
    } else {
        if bull_entry && config.direction_mode.allows_side(PositionSide::Long) {
            return Some(PendingStrategyOrder::Entry {
                side: PositionSide::Long,
                signal_bar: i,
            });
        }
        if bear_entry && config.direction_mode.allows_side(PositionSide::Short) {
            return Some(PendingStrategyOrder::Entry {
                side: PositionSide::Short,
                signal_bar: i,
            });
        }
        None
    }
}

fn evaluate_preset_signals_at_bar(
    i: usize,
    candles: &[Candle],
    config: &BacktestConfig,
    features: &PrecomputedFeatures,
) -> (bool, bool, bool, bool) {
    if i == 0 {
        return (false, false, false, false);
    }
    let close = candles[i].close;
    let prev_close = candles[i - 1].close;
    if !close.is_finite() || !prev_close.is_finite() {
        return (false, false, false, false);
    }
    let p = &config.params;

    match config.preset {
        StrategyPresetKind::SmaEmaTrendCross => {
            let (Some(s_prev), Some(s_cur), Some(e_prev), Some(e_cur)) = (
                features.sma_fast.get(i - 1).copied().flatten(),
                features.sma_fast.get(i).copied().flatten(),
                features.ema_slow.get(i - 1).copied().flatten(),
                features.ema_slow.get(i).copied().flatten(),
            ) else {
                return (false, false, false, false);
            };
            let prev_diff = s_prev - e_prev;
            let cur_diff = s_cur - e_cur;
            let bull = prev_diff <= 0.0 && cur_diff > 0.0;
            let bear = prev_diff >= 0.0 && cur_diff < 0.0;
            (bull, bear, bear, bull)
        }
        StrategyPresetKind::RsiMeanReversion => {
            let (Some(r_prev), Some(r_cur)) = (
                features.rsi.get(i - 1).copied().flatten(),
                features.rsi.get(i).copied().flatten(),
            ) else {
                return (false, false, false, false);
            };
            let bull = r_prev <= p.rsi_oversold && r_cur > p.rsi_oversold;
            let bear = r_prev >= p.rsi_overbought && r_cur < p.rsi_overbought;
            let long_exit = r_cur >= p.rsi_exit_midline;
            let short_exit = r_cur <= p.rsi_exit_midline;
            (bull, bear, long_exit, short_exit)
        }
        StrategyPresetKind::MacdMomentum => {
            let (Some(h_prev), Some(h_cur)) = (
                features.macd.histogram.get(i - 1).copied().flatten(),
                features.macd.histogram.get(i).copied().flatten(),
            ) else {
                return (false, false, false, false);
            };
            let (allow_long, allow_short) = if p.macd_trend_ema >= 2 {
                match features.macd_trend_ema.get(i).copied().flatten() {
                    Some(ema_v) => (close > ema_v, close < ema_v),
                    None => (false, false),
                }
            } else {
                (true, true)
            };
            let raw_bull_cross = h_prev <= 0.0 && h_cur > 0.0;
            let raw_bear_cross = h_prev >= 0.0 && h_cur < 0.0;
            (
                raw_bull_cross && allow_long,
                raw_bear_cross && allow_short,
                raw_bear_cross,
                raw_bull_cross,
            )
        }
        StrategyPresetKind::BollingerSqueezeBreakout => {
            let (Some(u_prev), Some(m_prev), Some(l_prev), Some(u_cur), Some(m_cur), Some(l_cur)) = (
                features.bb.upper.get(i - 1).copied().flatten(),
                features.bb.middle.get(i - 1).copied().flatten(),
                features.bb.lower.get(i - 1).copied().flatten(),
                features.bb.upper.get(i).copied().flatten(),
                features.bb.middle.get(i).copied().flatten(),
                features.bb.lower.get(i).copied().flatten(),
            ) else {
                return (false, false, false, false);
            };
            if m_prev <= 0.0 {
                return (false, false, false, false);
            }
            let bw_prev_pct = ((u_prev - l_prev) / m_prev) * 100.0;
            let squeezed = bw_prev_pct <= p.max_squeeze_bw_pct;
            let bull = squeezed && prev_close <= u_prev && close > u_cur;
            let bear = squeezed && prev_close >= l_prev && close < l_cur;
            let long_exit = close < m_cur;
            let short_exit = close > m_cur;
            (bull, bear, long_exit, short_exit)
        }
        StrategyPresetKind::DonchianAtrBreakout => {
            let entry_h = features.donchian_entry_high.get(i).copied().flatten();
            let entry_l = features.donchian_entry_low.get(i).copied().flatten();
            let exit_h = features.donchian_exit_high.get(i).copied().flatten();
            let exit_l = features.donchian_exit_low.get(i).copied().flatten();

            let bull = entry_h.is_some_and(|h| close > h);
            let bear = entry_l.is_some_and(|l| close < l);
            let long_exit = exit_l.is_some_and(|l| close < l);
            let short_exit = exit_h.is_some_and(|h| close > h);
            (bull, bear, long_exit, short_exit)
        }
        StrategyPresetKind::RsiBollingerConfluence => {
            let (Some(r_cur), Some(u_cur), Some(m_cur), Some(l_cur)) = (
                features.rsi.get(i).copied().flatten(),
                features.bb.upper.get(i).copied().flatten(),
                features.bb.middle.get(i).copied().flatten(),
                features.bb.lower.get(i).copied().flatten(),
            ) else {
                return (false, false, false, false);
            };
            let bull = close <= l_cur && r_cur <= p.rsi_confluence_buy;
            let bear = close >= u_cur && r_cur >= p.rsi_confluence_sell;
            let long_exit = close >= m_cur || r_cur >= p.rsi_confluence_sell;
            let short_exit = close <= m_cur || r_cur <= p.rsi_confluence_buy;
            (bull, bear, long_exit, short_exit)
        }
        StrategyPresetKind::CustomSignalRules => {
            let Some(ref matrix) = features.catalog_matrix else {
                return (false, false, false, false);
            };
            evaluate_custom_rules_at_bar(&config.custom_rules, matrix, i)
        }
    }
}

/// $O(N)$ monotonic-deque prior-window maximum: `out[i] = max(values[i - window .. i])` (`TEST-CAUSAL-003`).
fn compute_prior_window_max(values: &[f64], window: usize) -> Vec<Option<f64>> {
    let n = values.len();
    let mut out = vec![None; n];
    if window == 0 || n <= window {
        return out;
    }
    let mut deque: VecDeque<usize> = VecDeque::with_capacity(window + 1);
    for i in 0..n {
        // Evict indices outside `[i - window, i - 1]` BEFORE reading `front()` for `out[i]`.
        if let Some(min_valid) = i.checked_sub(window) {
            while deque.front().is_some_and(|&idx| idx < min_valid) {
                deque.pop_front();
            }
            if let Some(&front_idx) = deque.front()
                && values[front_idx].is_finite()
            {
                out[i] = Some(values[front_idx]);
            }
        }
        // Now insert `i` so it participates starting at `i + 1` (strictly excluding bar `i` from `out[i]`).
        let v = values[i];
        if v.is_finite() {
            while deque.back().is_some_and(|&b_idx| values[b_idx] <= v) {
                deque.pop_back();
            }
            deque.push_back(i);
        }
    }
    out
}

/// $O(N)$ monotonic-deque prior-window minimum: `out[i] = min(values[i - window .. i])` (`TEST-CAUSAL-003`).
fn compute_prior_window_min(values: &[f64], window: usize) -> Vec<Option<f64>> {
    let n = values.len();
    let mut out = vec![None; n];
    if window == 0 || n <= window {
        return out;
    }
    let mut deque: VecDeque<usize> = VecDeque::with_capacity(window + 1);
    for i in 0..n {
        if let Some(min_valid) = i.checked_sub(window) {
            while deque.front().is_some_and(|&idx| idx < min_valid) {
                deque.pop_front();
            }
            if let Some(&front_idx) = deque.front()
                && values[front_idx].is_finite()
            {
                out[i] = Some(values[front_idx]);
            }
        }
        let v = values[i];
        if v.is_finite() {
            while deque.back().is_some_and(|&b_idx| values[b_idx] >= v) {
                deque.pop_back();
            }
            deque.push_back(i);
        }
    }
    out
}

#[inline]
fn effective_min_holding_bars(min_holding_bars: usize, timeframe: Timeframe) -> usize {
    if min_holding_bars == 0 {
        return 0;
    }
    match timeframe {
        Timeframe::W1 => min_holding_bars.div_ceil(5),
        _ => min_holding_bars,
    }
}

/// Normalizes VN spot-specific board lot (`lot_step = 100`), `tax_sell_bps`, and `T+2` lockup
/// when comparing across heterogeneous asset classes (`BTCUSDT`, `SPY`, `VN30F1M`, `VNINDEX`) in
/// `run_multi_symbol_battle` (`[MINOR-01]`).
#[inline]
pub(crate) fn normalize_config_for_universe_feed(
    base: &BacktestConfig,
    preset: DataSourcePreset,
    effective_tf: Timeframe,
) -> BacktestConfig {
    let mut cfg = base.clone();
    match preset {
        DataSourcePreset::Vn30Basket => {
            cfg.risk.min_holding_bars =
                effective_min_holding_bars(cfg.risk.min_holding_bars, effective_tf);
        }
        DataSourcePreset::BtcUsdt15mJson
        | DataSourcePreset::CafeFIndexCsv
        | DataSourcePreset::SyntheticVnindex => {
            cfg.lot_step = 0.0;
            cfg.tax_sell_bps = 0.0;
            cfg.risk.min_holding_bars = 0;
        }
        DataSourcePreset::IntradayVn30F1m | DataSourcePreset::SpyDailyCsv => {
            if cfg.lot_step > 1.0 {
                cfg.lot_step = 1.0;
            }
            cfg.tax_sell_bps = 0.0;
            cfg.risk.min_holding_bars = 0;
        }
        DataSourcePreset::CustomFile => {
            cfg.risk.min_holding_bars =
                effective_min_holding_bars(cfg.risk.min_holding_bars, effective_tf);
        }
    }
    cfg
}

/// Slices `candles` causally up to `options.replay_cutoff_timestamp`, withholding any un-aggregated
/// coarse tail bar whose closing boundary exceeds `cutoff` when `cutoff` falls strictly inside the
/// interior of that bar (`[CRITICAL-01]`), while recognizing exchange-specific session closes
/// (`VN30F1M`/`HOSE`/`HNX`/`UPCOM` `04:29:59` H1 & `07:44:59` H1/H4/D1/W1 UTC, `US` `20:59:59` UTC,
/// and `D1 -> W1` Friday weekly closes) when `symbol` is supplied (`[CRITIC-R17-CRITICAL-01]`).
#[inline]
pub(crate) fn slice_causal_series_for_symbol_and_options<'a>(
    symbol: Option<&str>,
    candles: &'a [Candle],
    timeframe: Timeframe,
    options: BacktestRunOptions,
) -> &'a [Candle] {
    match options.replay_cutoff_timestamp {
        None => candles,
        Some(cutoff) => {
            let mut end = candles.partition_point(|c| c.timestamp <= cutoff);
            if !options.incomplete_tail_bar
                && end > 0
                && let Some(last) = candles.get(end - 1)
            {
                let close_ts = last
                    .timestamp
                    .saturating_add(timeframe.seconds().max(1).saturating_sub(1));
                if cutoff > last.timestamp && cutoff < close_ts {
                    let sym_trimmed = symbol.map(str::trim).filter(|s| !s.is_empty());
                    let is_incomplete = match sym_trimmed {
                        None => true,
                        Some(sym) => {
                            let m1_incomplete = is_bucket_incomplete_at_close_for_symbol(
                                sym,
                                last.timestamp,
                                cutoff,
                                Timeframe::M1,
                                timeframe,
                            );
                            if !m1_incomplete {
                                false
                            } else if timeframe == Timeframe::W1 {
                                let venue =
                                    crate::journal::ExchangeVenue::infer_from_symbol_and_preset(
                                        sym,
                                        DataSourcePreset::CustomFile,
                                    );
                                !(venue != crate::journal::ExchangeVenue::CryptoSpotPerp
                                    && cutoff.saturating_sub(last.timestamp) >= 4 * 86_400 + 86_399)
                            } else {
                                true
                            }
                        }
                    };
                    if is_incomplete {
                        end -= 1;
                    }
                }
            }
            &candles[..end]
        }
    }
}

pub(crate) fn compare_tearsheets_desc(
    a: &BacktestTearsheet,
    b: &BacktestTearsheet,
    metric: SweepRankingMetric,
) -> core::cmp::Ordering {
    match metric {
        SweepRankingMetric::TotalReturnPct => cmp_f64_desc(a.total_return_pct, b.total_return_pct),
        SweepRankingMetric::SharpeRatio => cmp_opt_f64_desc(a.sharpe_ratio, b.sharpe_ratio)
            .then_with(|| cmp_f64_desc(a.total_return_pct, b.total_return_pct)),
        SweepRankingMetric::ProfitFactor => {
            // Note: 0-loss winning strategies (`num_winners > 0 && num_losers == 0`) have `profit_factor == None` ("∞"),
            // which should rank ABOVE finite PF, whereas 0-trade (`num_trades == 0`) `None` ranks BELOW finite PF!
            let pf_score = |t: &BacktestTearsheet| -> Option<f64> {
                if let Some(pf) = t.profit_factor {
                    Some(pf)
                } else if t.num_winners > 0 && t.num_losers == 0 {
                    Some(1.0e9 + t.realized_net_profit.max(0.0))
                } else {
                    None
                }
            };
            cmp_opt_f64_desc(pf_score(a), pf_score(b))
                .then_with(|| cmp_f64_desc(a.total_return_pct, b.total_return_pct))
        }
        SweepRankingMetric::CalmarRatio => cmp_opt_f64_desc(a.calmar_ratio, b.calmar_ratio)
            .then_with(|| cmp_f64_desc(a.total_return_pct, b.total_return_pct)),
    }
}

#[inline]
fn cmp_f64_desc(a: f64, b: f64) -> core::cmp::Ordering {
    match (a.is_finite(), b.is_finite()) {
        (true, true) => b.total_cmp(&a),
        (true, false) => core::cmp::Ordering::Less,
        (false, true) => core::cmp::Ordering::Greater,
        (false, false) => core::cmp::Ordering::Equal,
    }
}

#[inline]
fn cmp_opt_f64_desc(a: Option<f64>, b: Option<f64>) -> core::cmp::Ordering {
    match (a.filter(|v| v.is_finite()), b.filter(|v| v.is_finite())) {
        (Some(va), Some(vb)) => vb.total_cmp(&va),
        (Some(_), None) => core::cmp::Ordering::Less,
        (None, Some(_)) => core::cmp::Ordering::Greater,
        (None, None) => core::cmp::Ordering::Equal,
    }
}
