//! Deterministic `SplitMix64` Monte Carlo Bootstrap Risk Envelopes (`Milestone R15 / Wave 14`).
//!
//! Pure `f64`/`i64`/`u64`, `wasm32-unknown-unknown`-compatible path-order & return resampling engine
//! with zero external RNG dependencies (`Cargo.toml` untouched), statistical sample-size honesty,
//! permanent bankruptcy
//! absorption, and bar-aligned `p5/p25/p50/p75/p95` equity fan series for `OscillatorPaneId::Equity`.

use serde::{Deserialize, Serialize};

use crate::indicators::SeriesBlockIndex;
use crate::model::Candle;

use super::metrics::{
    BacktestEquityPoint, BacktestTrade, MIN_STATISTICAL_SAMPLE_TRADES, PNL_BREAKEVEN_EPS,
};

/// Default deterministic 64-bit seed (exact 6-digit value displayed bit-for-bit in Tearsheet UI).
pub const DEFAULT_MONTE_CARLO_SEED: u64 = 202_615;

/// Minimum allowed bootstrap simulation count.
pub const MIN_MONTE_CARLO_SIMULATIONS: usize = 100;

/// Maximum allowed bootstrap simulation count (keeps worst-case rebuild `< 2.5 ms`).
pub const MAX_MONTE_CARLO_SIMULATIONS: usize = 5_000;

/// Maximum resampled step horizon per simulated path.
pub const MAX_MONTE_CARLO_HORIZON: usize = 512;

/// Maximum checkpoint knots along `[0..=horizon]` stored for fan envelope interpolation.
pub const MAX_MONTE_CARLO_CHECKPOINTS: usize = 65;

/// Upper bound on simulated equity matching `PriceScale` / drawing domain limits (`1.0e15`).
const MAX_SIMULATED_EQUITY: f64 = 1.0e15;

/// Pure-Rust deterministic `SplitMix64` pseudo-random generator (`wasm32`-safe, zero OS calls).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    #[inline]
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Unbiased Lemire 128-bit multiply-shift index in `0..len` (`len >= 1`).
    #[inline]
    pub fn next_index(&mut self, len: usize) -> usize {
        debug_assert!(len >= 1);
        let r = self.next_u64();
        (((r as u128) * (len as u128)) >> 64) as usize
    }
}

/// Configuration for deterministic Monte Carlo bootstrap resampling.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MonteCarloConfig {
    /// Number of bootstrap paths (`100..=5000`, default `1000`).
    pub simulations: usize,
    /// Explicit deterministic seed (`SplitMix64`).
    pub seed: u64,
    /// Peak-to-trough drawdown threshold (`%`) counted toward `prob_of_ruin_pct` (`5.0..=100.0`, default `50.0`).
    pub ruin_drawdown_pct: f64,
    /// Optional explicit trade/return step horizon (`None` = match realized sample count `N`).
    pub horizon_steps: Option<usize>,
}

impl Default for MonteCarloConfig {
    fn default() -> Self {
        Self {
            simulations: 1_000,
            seed: DEFAULT_MONTE_CARLO_SEED,
            ruin_drawdown_pct: 50.0,
            horizon_steps: None,
        }
    }
}

impl MonteCarloConfig {
    #[must_use]
    pub fn sanitized(self) -> Self {
        let simulations = self
            .simulations
            .clamp(MIN_MONTE_CARLO_SIMULATIONS, MAX_MONTE_CARLO_SIMULATIONS);
        let ruin_drawdown_pct = if self.ruin_drawdown_pct.is_finite() {
            self.ruin_drawdown_pct.clamp(5.0, 100.0)
        } else {
            50.0
        };
        let horizon_steps = self
            .horizon_steps
            .map(|h| h.clamp(1, MAX_MONTE_CARLO_HORIZON));
        Self {
            simulations,
            seed: self.seed,
            ruin_drawdown_pct,
            horizon_steps,
        }
    }
}

/// Statistical sample-size & degeneracy classification (`MetricResultValue.tsx` contract).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MonteCarloValidity {
    /// `sample_count >= 30` (`MIN_STATISTICAL_SAMPLE_TRADES`).
    Valid,
    /// `2 <= sample_count < 30`: envelopes computed with explicit low-sample warning.
    LowSampleWarning,
    /// `sample_count == 1`: zero dispersion (`p5 == p95`), flagged as degenerate single sample.
    SingleSampleDegenerate,
    /// `sample_count == 0` or invalid capital: no metrics or fan bands synthesized.
    InsufficientSamples,
}

impl MonteCarloValidity {
    #[inline]
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Valid => "VALID",
            Self::LowSampleWarning => "LOW_SAMPLE_WARNING",
            Self::SingleSampleDegenerate => "SINGLE_SAMPLE_DEGENERATE",
            Self::InsufficientSamples => "INSUFFICIENT_SAMPLES",
        }
    }

    #[inline]
    #[must_use]
    pub const fn has_envelopes(self) -> bool {
        !matches!(self, Self::InsufficientSamples)
    }
}

/// Quantile summary (`p5`, `p25`, `p50` median, `p75`, `p95`) guaranteed monotonically non-decreasing.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct QuantileEnvelope5 {
    pub p5: f64,
    pub p25: f64,
    pub p50: f64,
    pub p75: f64,
    pub p95: f64,
}

/// Terminal & pathwise tail-risk statistics produced when `validity.has_envelopes() == true`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MonteCarloRiskMetrics {
    pub terminal_equity: QuantileEnvelope5,
    pub terminal_return_pct: QuantileEnvelope5,
    pub max_drawdown_p50_pct: f64,
    pub max_drawdown_var_95_pct: f64,
    pub max_drawdown_cvar_95_pct: f64,
    pub max_drawdown_var_99_pct: f64,
    pub max_drawdown_cvar_99_pct: f64,
    pub prob_of_profit_pct: f64,
    pub prob_of_loss_pct: f64,
    pub prob_of_ruin_pct: f64,
}

/// Bar-aligned `p5/p25/p50/p75/p95` equity fan series projected onto `[0..bar_count)` for
/// `OscillatorPaneId::Equity` (`src/chart.rs`).
#[derive(Debug, Clone)]
pub struct MonteCarloFanSeries {
    pub bar_count: usize,
    pub causal_bar_count: usize,
    pub p5: Vec<Option<f64>>,
    pub p25: Vec<Option<f64>>,
    pub p50: Vec<Option<f64>>,
    pub p75: Vec<Option<f64>>,
    pub p95: Vec<Option<f64>>,
    pub p5_idx: SeriesBlockIndex,
    pub p50_idx: SeriesBlockIndex,
    pub p95_idx: SeriesBlockIndex,
}

/// Complete deterministic Monte Carlo bootstrap report.
#[derive(Debug, Clone)]
pub struct MonteCarloReport {
    pub config: MonteCarloConfig,
    pub initial_cash: f64,
    pub sample_count: usize,
    pub horizon_steps: usize,
    pub validity: MonteCarloValidity,
    pub low_sample_warning: bool,
    pub status_reason: String,
    pub metrics: Option<MonteCarloRiskMetrics>,
    pub checkpoint_envelopes: Vec<QuantileEnvelope5>,
    pub bar_fan: Option<MonteCarloFanSeries>,
}

impl MonteCarloReport {
    fn unavailable(config: MonteCarloConfig, initial_cash: f64, reason: &str) -> Self {
        Self {
            config,
            initial_cash: initial_cash.max(0.0),
            sample_count: 0,
            horizon_steps: 0,
            validity: MonteCarloValidity::InsufficientSamples,
            low_sample_warning: true,
            status_reason: reason.to_owned(),
            metrics: None,
            checkpoint_envelopes: Vec::new(),
            bar_fan: None,
        }
    }
}

/// Extracts sequential on-account-equity fractional returns $r_k = \text{clamp}(\text{net\_pnl}_k / E_{k-1}, -1, 100)$
/// from closed [`BacktestTrade`]s strictly satisfying `exit_timestamp <= cutoff_timestamp` (`DEF-001`).
#[must_use]
pub fn extract_causal_trade_equity_returns(
    initial_cash: f64,
    trades: &[BacktestTrade],
    cutoff_timestamp: Option<i64>,
) -> (Vec<f64>, Option<usize>) {
    if !initial_cash.is_finite() || initial_cash <= PNL_BREAKEVEN_EPS || trades.is_empty() {
        return (Vec::new(), None);
    }

    let mut eq_before = initial_cash;
    let mut returns = Vec::with_capacity(trades.len());
    let mut first_entry_bar: Option<usize> = None;

    for t in trades {
        if let Some(cut) = cutoff_timestamp
            && t.exit_timestamp > cut
        {
            continue;
        }
        if !t.net_pnl.is_finite() {
            continue;
        }
        if eq_before <= PNL_BREAKEVEN_EPS {
            break;
        }
        if first_entry_bar.is_none() {
            first_entry_bar = Some(t.entry_bar_index);
        }
        let r = (t.net_pnl / eq_before).clamp(-1.0, 100.0);
        if r.is_finite() {
            returns.push(r);
        }
        eq_before = (eq_before + t.net_pnl).max(0.0);
    }

    (returns, first_entry_bar)
}

/// Extracts active (non-flat) bar-to-bar fractional returns from a [`BacktestEquityPoint`] curve
/// (used for `MultiAssetPortfolio` equity curves that aggregate multiple constituent symbol streams),
/// strictly gated by `cutoff_timestamp` (`DEF-001`).
#[must_use]
pub fn extract_causal_curve_returns(
    initial_cash: f64,
    curve: &[BacktestEquityPoint],
    cutoff_timestamp: Option<i64>,
) -> (Vec<f64>, Option<usize>) {
    if !initial_cash.is_finite() || initial_cash <= PNL_BREAKEVEN_EPS || curve.is_empty() {
        return (Vec::new(), None);
    }

    let mut prev_eq = initial_cash;
    let mut returns = Vec::new();
    let mut first_active_bar: Option<usize> = None;

    for pt in curve {
        if let Some(cut) = cutoff_timestamp
            && pt.timestamp > cut
        {
            break;
        }
        if !pt.equity.is_finite() {
            continue;
        }
        let curr_eq = pt.equity.max(0.0);
        if prev_eq <= PNL_BREAKEVEN_EPS {
            break;
        }
        let delta = curr_eq - prev_eq;
        if pt.position_qty.abs() > 0.0 || delta.abs() > PNL_BREAKEVEN_EPS {
            if first_active_bar.is_none() {
                first_active_bar = Some(pt.bar_index);
            }
            let r = (delta / prev_eq).clamp(-1.0, 100.0);
            if r.is_finite() {
                returns.push(r);
            }
        }
        prev_eq = curr_eq;
    }

    (returns, first_active_bar)
}

/// Runs deterministic `SplitMix64` bootstrap resampling over closed causal [`BacktestTrade`]s
/// and projects bar-aligned `p5/p25/p50/p75/p95` fan series across `[0..total_bar_count)`.
#[must_use]
pub fn run_monte_carlo_bootstrap(
    initial_cash: f64,
    trades: &[BacktestTrade],
    equity_curve: &[BacktestEquityPoint],
    total_bar_count: usize,
    cutoff_timestamp: Option<i64>,
    config: MonteCarloConfig,
) -> MonteCarloReport {
    let cfg = config.sanitized();
    if !initial_cash.is_finite() || initial_cash <= PNL_BREAKEVEN_EPS {
        return MonteCarloReport::unavailable(cfg, initial_cash, "INVALID_INITIAL_CAPITAL");
    }

    let (returns, first_entry_bar) =
        extract_causal_trade_equity_returns(initial_cash, trades, cutoff_timestamp);
    let curve_start_bar = equity_curve
        .first()
        .map_or(0, |pt| pt.bar_index.min(total_bar_count));
    let causal_bars = causal_equity_bar_count(equity_curve, total_bar_count, cutoff_timestamp);

    run_monte_carlo_from_returns_inner(
        initial_cash,
        &returns,
        curve_start_bar,
        first_entry_bar,
        total_bar_count,
        causal_bars,
        cfg,
    )
}

/// Runs deterministic `SplitMix64` bootstrap resampling over an equity curve's active periodic returns
/// when `equity_curve` points already carry target chart `bar_index` coordinates.
#[must_use]
pub fn run_monte_carlo_from_equity_curve(
    initial_cash: f64,
    equity_curve: &[BacktestEquityPoint],
    total_bar_count: usize,
    cutoff_timestamp: Option<i64>,
    config: MonteCarloConfig,
) -> MonteCarloReport {
    let cfg = config.sanitized();
    if !initial_cash.is_finite() || initial_cash <= PNL_BREAKEVEN_EPS {
        return MonteCarloReport::unavailable(cfg, initial_cash, "INVALID_INITIAL_CAPITAL");
    }

    let (returns, first_active_bar) =
        extract_causal_curve_returns(initial_cash, equity_curve, cutoff_timestamp);
    let curve_start_bar = equity_curve
        .first()
        .map_or(0, |pt| pt.bar_index.min(total_bar_count));
    let causal_bars = causal_equity_bar_count(equity_curve, total_bar_count, cutoff_timestamp);

    run_monte_carlo_from_returns_inner(
        initial_cash,
        &returns,
        curve_start_bar,
        first_active_bar,
        total_bar_count,
        causal_bars,
        cfg,
    )
}

/// Runs deterministic `SplitMix64` bootstrap resampling over a `MultiAssetPortfolio` union-timestamp
/// equity curve while aligning the `p5..p95` fan series onto active `chart_candles` coordinates
/// (`mapped_chart_equity` produced by `map_equity_curve_to_candles`).
#[must_use]
pub fn run_monte_carlo_portfolio_mapped(
    initial_cash: f64,
    portfolio_equity_curve: &[BacktestEquityPoint],
    mapped_chart_equity: &[Option<f64>],
    chart_candles: &[Candle],
    cutoff_timestamp: Option<i64>,
    config: MonteCarloConfig,
) -> MonteCarloReport {
    let cfg = config.sanitized();
    if !initial_cash.is_finite() || initial_cash <= PNL_BREAKEVEN_EPS {
        return MonteCarloReport::unavailable(cfg, initial_cash, "INVALID_INITIAL_CAPITAL");
    }

    let (returns, _) =
        extract_causal_curve_returns(initial_cash, portfolio_equity_curve, cutoff_timestamp);
    let total_bar_count = chart_candles.len();
    let max_causal_chart = match cutoff_timestamp {
        Some(cut) => chart_candles.partition_point(|c| c.timestamp <= cut),
        None => total_bar_count,
    }
    .min(mapped_chart_equity.len())
    .min(total_bar_count);

    let curve_start_bar = (0..max_causal_chart)
        .find(|&i| mapped_chart_equity[i].is_some())
        .unwrap_or(max_causal_chart);
    let causal_bars = (0..max_causal_chart)
        .rev()
        .find(|&i| mapped_chart_equity[i].is_some())
        .map_or(0, |i| i + 1);
    let first_active_bar = (curve_start_bar..causal_bars).find(|&i| {
        mapped_chart_equity[i].is_some_and(|eq| (eq - initial_cash).abs() > PNL_BREAKEVEN_EPS)
    });

    run_monte_carlo_from_returns_inner(
        initial_cash,
        &returns,
        curve_start_bar,
        first_active_bar,
        total_bar_count,
        causal_bars,
        cfg,
    )
}

fn causal_equity_bar_count(
    equity_curve: &[BacktestEquityPoint],
    total_bar_count: usize,
    cutoff_timestamp: Option<i64>,
) -> usize {
    if equity_curve.is_empty() {
        return if cutoff_timestamp.is_some() {
            0
        } else {
            total_bar_count
        };
    }
    let end = match cutoff_timestamp {
        Some(cut) => equity_curve.partition_point(|pt| pt.timestamp <= cut),
        None => equity_curve.len(),
    };
    if end == 0 {
        0
    } else {
        equity_curve[end - 1]
            .bar_index
            .saturating_add(1)
            .min(total_bar_count)
    }
}

fn run_monte_carlo_from_returns_inner(
    initial_cash: f64,
    returns: &[f64],
    curve_start_bar: usize,
    first_active_bar: Option<usize>,
    total_bar_count: usize,
    causal_bar_count: usize,
    cfg: MonteCarloConfig,
) -> MonteCarloReport {
    let k = returns.len();
    if k == 0 {
        return MonteCarloReport::unavailable(cfg, initial_cash, "ZERO_CLOSED_TRADES");
    }

    let horizon = cfg
        .horizon_steps
        .unwrap_or_else(|| k.clamp(1, MAX_MONTE_CARLO_HORIZON));
    let sim_count = cfg.simulations;

    let (validity, low_sample_warning, status_reason) = if k == 1 {
        (
            MonteCarloValidity::SingleSampleDegenerate,
            true,
            "SINGLE_TRADE_ZERO_DISPERSION".to_owned(),
        )
    } else if k < MIN_STATISTICAL_SAMPLE_TRADES {
        (
            MonteCarloValidity::LowSampleWarning,
            true,
            format!("LOW_SAMPLE_N_{k}_LT_{MIN_STATISTICAL_SAMPLE_TRADES}"),
        )
    } else {
        (
            MonteCarloValidity::Valid,
            false,
            format!("VALID_N_{k}_TRADES"),
        )
    };

    // Derive deterministic stream seed mixing user seed, sample count, horizon, and return digest
    let mut mixed_seed = cfg.seed ^ 0xA5A5_5A5A_F0F0_0F0F;
    let mut rng_mixer = SplitMix64::new(mixed_seed);
    mixed_seed ^= rng_mixer.next_u64().wrapping_add(k as u64);
    mixed_seed ^= (horizon as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    for &r in returns.iter().take(64) {
        let bits = if r == 0.0 { 0 } else { r.to_bits() };
        mixed_seed = mixed_seed.rotate_left(13) ^ bits;
    }
    let mut rng = SplitMix64::new(mixed_seed);

    let checkpoint_count = (horizon + 1).clamp(2, MAX_MONTE_CARLO_CHECKPOINTS);
    let mut checkpoint_steps = Vec::with_capacity(checkpoint_count);
    for c in 0..checkpoint_count {
        let step = if c + 1 == checkpoint_count {
            horizon
        } else {
            (c * horizon) / (checkpoint_count - 1)
        };
        if checkpoint_steps.last().copied() != Some(step) {
            checkpoint_steps.push(step);
        }
    }
    let cp_len = checkpoint_steps.len();

    let mut cp_matrix = vec![initial_cash; cp_len * sim_count];
    let mut terminal_equities = Vec::with_capacity(sim_count);
    let mut max_drawdowns = Vec::with_capacity(sim_count);
    let mut profit_paths = 0_usize;
    let mut loss_paths = 0_usize;
    let mut ruin_paths = 0_usize;

    for s in 0..sim_count {
        let mut eq = initial_cash;
        let mut peak = initial_cash;
        let mut max_dd_pct = 0.0_f64;
        let mut bankrupt = false;
        let mut cp_cursor = 0_usize;
        if checkpoint_steps[0] == 0 {
            cp_matrix[s] = initial_cash;
            cp_cursor = 1;
        }

        for step in 1..=horizon {
            let idx = rng.next_index(k);
            if !bankrupt {
                let r = returns[idx];
                let cand = eq * (1.0 + r);
                if !cand.is_finite() || cand <= PNL_BREAKEVEN_EPS {
                    eq = 0.0;
                    bankrupt = true;
                    max_dd_pct = 100.0;
                } else {
                    eq = cand.min(MAX_SIMULATED_EQUITY);
                    if eq >= peak {
                        peak = eq;
                    } else if peak > 0.0 {
                        let dd = ((peak - eq) / peak) * 100.0;
                        if dd > max_dd_pct {
                            max_dd_pct = dd;
                        }
                    }
                }
            }
            while cp_cursor < cp_len && checkpoint_steps[cp_cursor] == step {
                cp_matrix[cp_cursor * sim_count + s] = eq;
                cp_cursor += 1;
            }
        }

        let clamped_dd = max_dd_pct.clamp(0.0, 100.0);
        terminal_equities.push(eq);
        max_drawdowns.push(clamped_dd);

        if eq > initial_cash + PNL_BREAKEVEN_EPS {
            profit_paths += 1;
        } else if eq < initial_cash - PNL_BREAKEVEN_EPS {
            loss_paths += 1;
        }
        if bankrupt || clamped_dd >= cfg.ruin_drawdown_pct {
            ruin_paths += 1;
        }
    }

    terminal_equities.sort_by(f64::total_cmp);
    max_drawdowns.sort_by(f64::total_cmp);

    let term_eq = envelope5_from_sorted(&terminal_equities);
    let to_ret = |eq_v: f64| ((eq_v - initial_cash) / initial_cash) * 100.0;
    let term_ret = QuantileEnvelope5 {
        p5: to_ret(term_eq.p5),
        p25: to_ret(term_eq.p25),
        p50: to_ret(term_eq.p50),
        p75: to_ret(term_eq.p75),
        p95: to_ret(term_eq.p95),
    };

    let dd_p50 = quantile_sorted(&max_drawdowns, 0.50).clamp(0.0, 100.0);
    let dd_var_95 = quantile_sorted(&max_drawdowns, 0.95).clamp(dd_p50, 100.0);
    let dd_var_99 = quantile_sorted(&max_drawdowns, 0.99).clamp(dd_var_95, 100.0);
    let dd_cvar_95 = expected_shortfall_upper_tail(&max_drawdowns, 0.95, dd_var_95);
    let dd_cvar_99 = expected_shortfall_upper_tail(&max_drawdowns, 0.99, dd_var_99);

    let denom = sim_count as f64;
    let metrics = MonteCarloRiskMetrics {
        terminal_equity: term_eq,
        terminal_return_pct: term_ret,
        max_drawdown_p50_pct: dd_p50,
        max_drawdown_var_95_pct: dd_var_95,
        max_drawdown_cvar_95_pct: dd_cvar_95,
        max_drawdown_var_99_pct: dd_var_99,
        max_drawdown_cvar_99_pct: dd_cvar_99,
        prob_of_profit_pct: (profit_paths as f64 / denom) * 100.0,
        prob_of_loss_pct: (loss_paths as f64 / denom) * 100.0,
        prob_of_ruin_pct: (ruin_paths as f64 / denom) * 100.0,
    };

    let mut checkpoint_envelopes = Vec::with_capacity(cp_len);
    for c in 0..cp_len {
        let row = &mut cp_matrix[c * sim_count..(c + 1) * sim_count];
        row.sort_by(f64::total_cmp);
        checkpoint_envelopes.push(envelope5_from_sorted(row));
    }

    let bar_fan = build_bar_aligned_fan(
        initial_cash,
        &checkpoint_envelopes,
        curve_start_bar,
        first_active_bar,
        total_bar_count,
        causal_bar_count,
    );

    MonteCarloReport {
        config: cfg,
        initial_cash,
        sample_count: k,
        horizon_steps: horizon,
        validity,
        low_sample_warning,
        status_reason,
        metrics: Some(metrics),
        checkpoint_envelopes,
        bar_fan,
    }
}

#[inline]
fn quantile_sorted(sorted: &[f64], q: f64) -> f64 {
    debug_assert!(!sorted.is_empty());
    if sorted.len() == 1 {
        return sorted[0];
    }
    let qc = q.clamp(0.0, 1.0);
    let pos = qc * ((sorted.len() - 1) as f64);
    let lo = (pos.floor() as usize).min(sorted.len() - 1);
    let hi = (lo + 1).min(sorted.len() - 1);
    let frac = pos - (lo as f64);
    let v = sorted[lo] + frac * (sorted[hi] - sorted[lo]);
    v.clamp(sorted[lo], sorted[hi])
}

#[inline]
fn envelope5_from_sorted(sorted: &[f64]) -> QuantileEnvelope5 {
    let p5 = quantile_sorted(sorted, 0.05);
    let p25 = quantile_sorted(sorted, 0.25).max(p5);
    let p50 = quantile_sorted(sorted, 0.50).max(p25);
    let p75 = quantile_sorted(sorted, 0.75).max(p50);
    let p95 = quantile_sorted(sorted, 0.95).max(p75);
    QuantileEnvelope5 {
        p5,
        p25,
        p50,
        p75,
        p95,
    }
}

fn expected_shortfall_upper_tail(sorted: &[f64], alpha: f64, var_alpha: f64) -> f64 {
    let n = sorted.len();
    if n == 0 {
        return 0.0;
    }
    let start_idx = ((alpha.clamp(0.0, 1.0) * (n as f64)).floor() as usize).min(n - 1);
    let tail = &sorted[start_idx..];
    let mean = tail.iter().sum::<f64>() / (tail.len() as f64);
    mean.max(var_alpha).clamp(0.0, 100.0)
}

fn build_bar_aligned_fan(
    initial_cash: f64,
    checkpoints: &[QuantileEnvelope5],
    curve_start_bar: usize,
    first_active_bar: Option<usize>,
    total_bar_count: usize,
    causal_bar_count: usize,
) -> Option<MonteCarloFanSeries> {
    let vis_bars = causal_bar_count.min(total_bar_count);
    if total_bar_count == 0
        || vis_bars == 0
        || checkpoints.is_empty()
        || curve_start_bar >= vis_bars
    {
        return None;
    }
    let mut p5 = vec![None; total_bar_count];
    let mut p25 = vec![None; total_bar_count];
    let mut p50 = vec![None; total_bar_count];
    let mut p75 = vec![None; total_bar_count];
    let mut p95 = vec![None; total_bar_count];

    let anchor_bar = first_active_bar
        .unwrap_or(curve_start_bar)
        .max(curve_start_bar)
        .min(vis_bars.saturating_sub(1));
    for i in curve_start_bar..anchor_bar {
        p5[i] = Some(initial_cash);
        p25[i] = Some(initial_cash);
        p50[i] = Some(initial_cash);
        p75[i] = Some(initial_cash);
        p95[i] = Some(initial_cash);
    }

    let span_bars = (vis_bars - 1).saturating_sub(anchor_bar);
    let max_cp = checkpoints.len() - 1;
    for i in anchor_bar..vis_bars {
        let u = if span_bars == 0 || max_cp == 0 {
            1.0
        } else {
            ((i - anchor_bar) as f64) / (span_bars as f64)
        };
        let pos = u.clamp(0.0, 1.0) * (max_cp as f64);
        let lo = (pos.floor() as usize).min(max_cp);
        let hi = (lo + 1).min(max_cp);
        let frac = pos - (lo as f64);
        let a = checkpoints[lo];
        let b = checkpoints[hi];
        let v5 = a.p5 + frac * (b.p5 - a.p5);
        let v25 = (a.p25 + frac * (b.p25 - a.p25)).max(v5);
        let v50 = (a.p50 + frac * (b.p50 - a.p50)).max(v25);
        let v75 = (a.p75 + frac * (b.p75 - a.p75)).max(v50);
        let v95 = (a.p95 + frac * (b.p95 - a.p95)).max(v75);
        p5[i] = Some(v5);
        p25[i] = Some(v25);
        p50[i] = Some(v50);
        p75[i] = Some(v75);
        p95[i] = Some(v95);
    }

    let p5_idx = SeriesBlockIndex::build(&p5);
    let p50_idx = SeriesBlockIndex::build(&p50);
    let p95_idx = SeriesBlockIndex::build(&p95);

    Some(MonteCarloFanSeries {
        bar_count: total_bar_count,
        causal_bar_count: vis_bars,
        p5,
        p25,
        p50,
        p75,
        p95,
        p5_idx,
        p50_idx,
        p95_idx,
    })
}
