//! Pure `f64`/`i64`, `wasm32-unknown-unknown`-compatible Deterministic Strategy Backtest Lab (`Milestone R09 / Wave 8`).
//!
//! Enforces strict causal execution (`Signal at Close[T] -> Fill at Open[T+1]`, `TEST-BT-001` final-bar
//! `EndOfSeriesCutoff`, `DEF-001` Bar Replay cutoff gating, pessimistic intrabar `SL`-before-`TP`,
//! and post-close trailing-stop ratcheting) with zero OS, wall-clock, or UI dependencies.

pub mod engine;
pub mod metrics;
pub mod monte_carlo;
pub mod strategy;
#[cfg(test)]
mod tests;
pub mod walk_forward;

pub use engine::{
    BacktestRunOptions, BacktestRunResult, MultiSymbolBattleReport, OpenBacktestPosition,
    ParameterSweepReport, PresetBattleRow, SweepVariantRow, SymbolBattleRow,
    is_bucket_incomplete_at_close, run_all_presets_battle, run_backtest, run_backtest_with_options,
    run_multi_symbol_battle, run_multi_symbol_battle_with_close_cutoff, run_parameter_sweep,
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
pub use strategy::{
    BacktestConfig, MAX_SWEEP_VARIANTS, PositionSizingMode, StopTakeConfig, StrategyDirectionMode,
    StrategyParams, StrategyPresetKind, SweepRankingMetric, build_preset_sweep_configs,
};
pub use walk_forward::{
    MIN_IS_BARS_PER_FOLD, MIN_OOS_BARS_PER_FOLD, PortfolioBacktestReport,
    PortfolioConstituentAttribution, PortfolioUniverseFilter, PortfolioWeightingMode,
    WalkForwardConfig, WalkForwardFoldResult, WalkForwardReport, WalkForwardVerdict,
    map_equity_curve_to_candles, run_portfolio_backtest, run_walk_forward_validation,
};
