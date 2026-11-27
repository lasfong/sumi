//! Pure `f64`/`i64`, `wasm32-unknown-unknown`-compatible Deterministic Strategy Backtest Lab (`Milestone R09 / Wave 8`).
//!
//! Enforces strict causal execution (`Signal at Close[T] -> Fill at Open[T+1]`, `TEST-BT-001` final-bar
//! `EndOfSeriesCutoff`, `DEF-001` Bar Replay cutoff gating, pessimistic intrabar `SL`-before-`TP`,
//! and post-close trailing-stop ratcheting) with zero OS, wall-clock, or UI dependencies.

pub mod analytics;
pub mod engine;
pub mod metrics;
pub mod monte_carlo;
pub mod report_html;
pub mod strategy;
#[cfg(test)]
mod tests;
pub mod walk_forward;

pub use analytics::{
    AdvancedTearsheetAnalytics, ComparisonCurveKind, DrawdownPeriod, MultiPhaseBatchReport,
    MultiPhaseSplitMode, MultiStrategyComparisonReport, MultiStrategyCurveSeries,
    OutlierImpactReport, RUN_MANIFEST_SCHEMA_VERSION, RegimePerformanceSlice, RunManifest,
    YearPerformanceSlice, compute_advanced_tearsheet_analytics, compute_entry_regime_breakdown,
    compute_entry_regime_breakdown_with_precomputed, compute_outlier_impact,
    compute_yearly_breakdown, extract_drawdown_periods, run_multi_phase_batch,
    run_multi_phase_batch_for_symbol, run_multi_strategy_comparison,
};

pub use engine::{
    BacktestRunOptions, BacktestRunResult, MultiSymbolBattleReport, OpenBacktestPosition,
    ParameterSweepReport, PresetBattleRow, SweepVariantRow, SymbolBattleRow,
    is_bucket_incomplete_at_close, is_bucket_incomplete_at_close_for_symbol,
    run_all_presets_battle, run_backtest, run_backtest_with_magnifier, run_backtest_with_options,
    run_multi_symbol_battle, run_multi_symbol_battle_with_close_cutoff,
    run_multi_symbol_battle_with_universe, run_parameter_sweep, run_pit_episodes_backtest,
};
pub use metrics::{
    BacktestEquityPoint, BacktestExitReason, BacktestTearsheet, BacktestTrade,
    MIN_STATISTICAL_SAMPLE_TRADES, PNL_BREAKEVEN_EPS, RobustnessVerdict, UnexecutedSignal,
    UnfilledReason, compute_tearsheet, evaluate_robustness,
};
pub use monte_carlo::{
    DEFAULT_MONTE_CARLO_SEED, MAX_MONTE_CARLO_CHECKPOINTS, MAX_MONTE_CARLO_HORIZON,
    MAX_MONTE_CARLO_SIMULATIONS, MIN_MONTE_CARLO_SIMULATIONS, MonteCarloConfig,
    MonteCarloFanSeries, MonteCarloReport, MonteCarloRiskMetrics, MonteCarloValidity,
    QuantileEnvelope5, SplitMix64, extract_causal_curve_returns,
    extract_causal_trade_equity_returns, run_monte_carlo_bootstrap,
    run_monte_carlo_from_equity_curve, run_monte_carlo_portfolio_mapped,
};
pub use report_html::{escape_html, render_backtest_report_html};
pub use strategy::{
    BacktestConfig, ConfluenceThresholdGate, CustomSignalRuleSpec, DirectionalRuleLeg, FloatCmpOp,
    LogicalCombinator, MAX_CLAUSES_PER_GROUP, MAX_RULE_GROUPS, MAX_RULE_NAME_BYTES,
    MAX_SWEEP_VARIANTS, PositionSizingMode, RegimeBitmask, SignalClause, SignalClauseGroup,
    SignalPredicate, StopTakeConfig, StrategyDirectionMode, StrategyParams, StrategyPresetKind,
    SweepRankingMetric, build_preset_sweep_configs, count_custom_rule_matches,
    evaluate_custom_rules_at_bar, evaluate_rule_leg_at, evaluate_signal_clause_at,
};
pub use walk_forward::{
    MIN_IS_BARS_PER_FOLD, MIN_OOS_BARS_PER_FOLD, PortfolioBacktestReport,
    PortfolioConstituentAttribution, PortfolioUniverseFilter, PortfolioWeightingMode,
    WalkForwardConfig, WalkForwardFoldResult, WalkForwardReport, WalkForwardVerdict,
    map_equity_curve_to_candles, run_portfolio_backtest, run_portfolio_backtest_with_universe,
    run_walk_forward_validation,
};
