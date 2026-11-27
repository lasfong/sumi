use serde::{Deserialize, Serialize};

use crate::data::Timeframe;
use crate::trading::PositionSide;

/// Tolerance around `0.0` net PnL classifying a round-trip trade as breakeven (`DEC-05`).
pub const PNL_BREAKEVEN_EPS: f64 = 1.0e-9;

/// Minimum closed-trade sample size before lifting the low-sample statistical warning (`N >= 30`).
pub const MIN_STATISTICAL_SAMPLE_TRADES: usize = 30;

/// Seconds per Julian year (`365.25 * 86_400.0`).
const SECONDS_PER_YEAR: f64 = 31_557_600.0;

/// Minimum elapsed calendar span (`7` days) required before extrapolating `CAGR %`.
const MIN_CAGR_YEARS: f64 = 7.0 / 365.25;

/// Minimum elapsed calendar span (`30` days) required before switching from nominal
/// `Timeframe` annualization (`252` equity daily bars, etc.) to empirical interval density.
const MIN_EMPIRICAL_ANNUALIZATION_YEARS: f64 = 30.0 / 365.25;

/// Reason a backtest position closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BacktestExitReason {
    SignalExit,
    SignalReversal,
    StopLoss,
    TakeProfit,
    TrailingStop,
    MaxBarsExit,
    MarginLiquidation,
    /// Point-in-time (`PIT`) constituent exit / delisting settlement on final active bar close.
    UniverseRemoval,
}

impl BacktestExitReason {
    #[inline]
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::SignalExit => "Signal",
            Self::SignalReversal => "Reverse",
            Self::StopLoss => "SL",
            Self::TakeProfit => "TP",
            Self::TrailingStop => "Trail",
            Self::MaxBarsExit => "MaxBars",
            Self::MarginLiquidation => "Liq",
            Self::UniverseRemoval => "PITExit",
        }
    }
}

/// Completed causal round-trip trade record emitted by `src/backtest/`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BacktestTrade {
    pub trade_id: u64,
    pub side: PositionSide,
    pub quantity: f64,
    pub entry_bar_index: usize,
    pub exit_bar_index: usize,
    pub entry_timestamp: i64,
    pub exit_timestamp: i64,
    pub entry_price: f64,
    pub exit_price: f64,
    pub bars_held: usize,
    pub gross_pnl: f64,
    /// Entry brokerage commission (`commission_bps`).
    pub entry_fee: f64,
    /// Exit brokerage commission (`commission_bps`).
    pub exit_fee: f64,
    /// Sell-side securities transaction tax (`tax_sell_bps`, charged on `Long` Sell exit or `Short` Sell entry).
    pub exit_tax: f64,
    pub net_pnl: f64,
    pub return_pct: f64,
    pub r_multiple: Option<f64>,
    /// Maximum Adverse Excursion (`<= 0.0%` worst unrealized excursion relative to entry).
    pub mae_pct: f64,
    /// Maximum Favorable Excursion (`>= 0.0%` peak unrealized excursion relative to entry).
    pub mfe_pct: f64,
    pub exit_reason: BacktestExitReason,
}

/// Bar-aligned mark-to-market equity and drawdown sample at causal bar close `i`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BacktestEquityPoint {
    pub bar_index: usize,
    pub timestamp: i64,
    pub equity: f64,
    pub cash: f64,
    /// Non-negative currency drawdown from running peak equity (`peak_equity - equity >= 0.0`).
    pub drawdown_abs: f64,
    /// Non-positive signed underwater drawdown percentage (`<= 0.0%`).
    pub underwater_pct: f64,
    /// Signed open position quantity (`+qty` Long, `-qty` Short, `0.0` Flat).
    pub position_qty: f64,
}

/// Why a causal strategy signal could not fill on the subsequent bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UnfilledReason {
    /// Signal occurred at final causal bar `N - 1`; bar `N` does not exist (`TEST-BT-001`).
    EndOfSeriesCutoff,
    /// Account equity/cash was `<= 0.0` at `Open[i]`.
    InsufficientEquity,
    /// Computed quantity rounded below `MIN_ORDER_QUANTITY` or discrete `lot_step`.
    ZeroSizedLot,
    /// Target open price was non-finite or `<= 0.0`.
    InvalidCandlePrice,
    /// Session opened and stayed pinned at statutory price limit (`trắng bên bán` on ceiling / `trắng bên mua` on floor).
    LimitLocked,
    /// 20-session average liquidity was below `min_entry_avg_vol_20` threshold.
    IlliquidBar,
}

impl UnfilledReason {
    #[inline]
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::EndOfSeriesCutoff => "Cutoff(N-1)",
            Self::InsufficientEquity => "NoEquity",
            Self::ZeroSizedLot => "MinLot",
            Self::InvalidCandlePrice => "BadPrice",
            Self::LimitLocked => "LimitLock",
            Self::IlliquidBar => "LowLiq",
        }
    }
}

/// Record of a causal signal that did not convert into an execution fill.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnexecutedSignal {
    pub signal_bar_index: usize,
    pub timestamp: i64,
    pub side: PositionSide,
    pub is_exit: bool,
    pub reason: UnfilledReason,
}

/// Out-of-Sample (`OOS`) generalization verdict comparing `IS` and `OOS` tearsheets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RobustnessVerdict {
    Robust,
    Stable,
    Degraded,
    Overfitted,
    LowSample,
    Unprofitable,
}

impl RobustnessVerdict {
    #[inline]
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Robust => "ROBUST",
            Self::Stable => "STABLE",
            Self::Degraded => "DEGRADED",
            Self::Overfitted => "OVERFITTED",
            Self::LowSample => "LOW SAMPLE",
            Self::Unprofitable => "UNPROFITABLE",
        }
    }
}

/// Institutional quantitative performance tearsheet for a strategy run or phase slice.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BacktestTearsheet {
    pub initial_cash: f64,
    /// Settled cash balance (`initial_cash + realized_net_profit - open_entry_charges`, excluding unrealized open PnL).
    pub final_cash: f64,
    pub final_equity: f64,
    /// Sum of `net_pnl` across all **closed** round-trip trades.
    pub realized_net_profit: f64,
    /// Mark-to-market total net profit (`final_equity - initial_cash`, includes open unrealized PnL).
    pub total_return_abs: f64,
    pub total_return_pct: f64,
    pub cagr_pct: Option<f64>,
    pub max_drawdown_abs: f64,
    /// Positive peak-to-trough percentage drawdown (`[0.0, 100.0+]`).
    pub max_drawdown_pct: f64,
    pub max_drawdown_duration_bars: usize,
    pub sharpe_ratio: Option<f64>,
    pub sortino_ratio: Option<f64>,
    pub calmar_ratio: Option<f64>,
    pub num_trades: usize,
    pub num_winners: usize,
    pub num_losers: usize,
    pub num_breakeven: usize,
    pub win_rate_pct: f64,
    pub profit_factor: Option<f64>,
    pub expectancy_net: Option<f64>,
    pub avg_trade_return_pct: f64,
    pub avg_win_pct: f64,
    /// Average losing trade return percentage (preserves negative sign `< 0.0`).
    pub avg_loss_pct: f64,
    pub avg_bars_held: f64,
    pub avg_r_multiple: Option<f64>,
    pub exposure_pct: f64,
    pub total_fees_and_tax: f64,
    pub low_sample_warning: bool,
}

impl BacktestTearsheet {
    #[must_use]
    pub fn empty(initial_cash: f64) -> Self {
        Self {
            initial_cash,
            final_cash: initial_cash,
            final_equity: initial_cash,
            realized_net_profit: 0.0,
            total_return_abs: 0.0,
            total_return_pct: 0.0,
            cagr_pct: None,
            max_drawdown_abs: 0.0,
            max_drawdown_pct: 0.0,
            max_drawdown_duration_bars: 0,
            sharpe_ratio: None,
            sortino_ratio: None,
            calmar_ratio: None,
            num_trades: 0,
            num_winners: 0,
            num_losers: 0,
            num_breakeven: 0,
            win_rate_pct: 0.0,
            profit_factor: None,
            expectancy_net: None,
            avg_trade_return_pct: 0.0,
            avg_win_pct: 0.0,
            avg_loss_pct: 0.0,
            avg_bars_held: 0.0,
            avg_r_multiple: None,
            exposure_pct: 0.0,
            total_fees_and_tax: 0.0,
            low_sample_warning: true,
        }
    }
}

/// Computes a full [`BacktestTearsheet`] from bar-aligned equity samples and closed trades.
#[must_use]
pub fn compute_tearsheet(
    initial_cash: f64,
    final_cash: f64,
    timeframe: Timeframe,
    equity_slice: &[BacktestEquityPoint],
    trades: &[BacktestTrade],
    open_entry_fee: f64,
) -> BacktestTearsheet {
    if equity_slice.is_empty() || initial_cash <= 0.0 || !initial_cash.is_finite() {
        return BacktestTearsheet::empty(initial_cash.max(0.0));
    }

    let final_equity = equity_slice
        .last()
        .map_or(initial_cash, |pt| pt.equity)
        .max(0.0);
    let total_return_abs = final_equity - initial_cash;
    let total_return_pct = (total_return_abs / initial_cash) * 100.0;

    // Recompute peak-to-trough drawdown strictly inside `equity_slice` (supports IS/OOS sub-windows!).
    let mut peak = initial_cash;
    let mut max_dd_abs = 0.0_f64;
    let mut max_dd_pct = 0.0_f64;
    let mut cur_dd_bars = 0_usize;
    let mut max_dd_bars = 0_usize;
    let mut bars_in_market = 0_usize;

    for pt in equity_slice {
        if pt.position_qty.abs() > 0.0 {
            bars_in_market += 1;
        }
        if pt.equity >= peak {
            peak = pt.equity;
            cur_dd_bars = 0;
        } else if peak > 0.0 {
            let dd_abs = peak - pt.equity;
            let dd_pct = (dd_abs / peak) * 100.0;
            max_dd_abs = max_dd_abs.max(dd_abs);
            max_dd_pct = max_dd_pct.max(dd_pct);
            cur_dd_bars += 1;
            max_dd_bars = max_dd_bars.max(cur_dd_bars);
        }
    }

    let exposure_pct = (bars_in_market as f64 / equity_slice.len() as f64) * 100.0;

    // Elapsed calendar years & CAGR guard (`>= 7` days prevents intraday powf overflow).
    // Include one bar width (`timeframe.seconds()`) in total calendar span so e.g. 7 `D1` bars = 7 full days,
    // while keeping `interval_years` (`N - 1` inter-bar intervals) exact for empirical bar density.
    let interval_secs = equity_slice
        .last()
        .zip(equity_slice.first())
        .map_or(0_i64, |(last, first)| {
            last.timestamp.saturating_sub(first.timestamp)
        })
        .max(0);
    let elapsed_secs = interval_secs.saturating_add(timeframe.seconds().max(0)) as f64;
    let elapsed_years = elapsed_secs / SECONDS_PER_YEAR;
    let interval_years = (interval_secs as f64) / SECONDS_PER_YEAR;

    let cagr_pct = if elapsed_years >= MIN_CAGR_YEARS {
        if final_equity <= 0.0 {
            Some(-100.0)
        } else {
            let ratio = final_equity / initial_cash;
            let val = (ratio.powf(1.0 / elapsed_years) - 1.0) * 100.0;
            val.is_finite().then_some(val)
        }
    } else {
        None
    };

    // Bar-to-bar return series for Sharpe & Sortino (`r_0` includes bar 0 relative to `initial_cash`).
    let mut bar_returns = Vec::with_capacity(equity_slice.len());
    let mut prev_eq = initial_cash;
    for pt in equity_slice {
        if pt.equity.is_finite() {
            let eq_clamped = pt.equity.max(0.0);
            if prev_eq > 0.0 {
                let r = (eq_clamped - prev_eq) / prev_eq;
                if r.is_finite() {
                    bar_returns.push(r);
                }
            } else if eq_clamped == 0.0 {
                // Post-liquidation flat bankrupt bar contributes `0.0` return to sample size `N`.
                bar_returns.push(0.0);
            }
            prev_eq = eq_clamped;
        }
    }

    let n_intervals = equity_slice.len().saturating_sub(1);
    let bars_per_year = effective_bars_per_year(timeframe, n_intervals, interval_years);
    let (sharpe_ratio, sortino_ratio) = compute_sharpe_and_sortino(&bar_returns, bars_per_year);
    let calmar_ratio = match cagr_pct {
        Some(cagr) if max_dd_pct > 1.0e-9 => {
            let calmar = cagr / max_dd_pct;
            calmar.is_finite().then_some(calmar)
        }
        _ => None,
    };

    // Closed round-trip trade statistics (`DEC-05`: breakevens stay in `num_trades` denominator).
    let num_trades = trades.len();
    let mut num_winners = 0_usize;
    let mut num_losers = 0_usize;
    let mut num_breakeven = 0_usize;
    let mut realized_net_profit = 0.0_f64;
    let mut gross_win_net = 0.0_f64;
    let mut gross_loss_abs = 0.0_f64;
    let mut sum_ret_pct = 0.0_f64;
    let mut sum_win_pct = 0.0_f64;
    let mut sum_loss_pct = 0.0_f64;
    let mut sum_bars_held = 0_usize;
    let mut sum_r = 0.0_f64;
    let mut count_r = 0_usize;
    let mut closed_fees_and_tax = 0.0_f64;

    for t in trades {
        realized_net_profit += t.net_pnl;
        closed_fees_and_tax += t.entry_fee + t.exit_fee + t.exit_tax;
        sum_ret_pct += t.return_pct;
        sum_bars_held += t.bars_held;
        if let Some(r) = t.r_multiple.filter(|r| r.is_finite()) {
            sum_r += r;
            count_r += 1;
        }
        if t.net_pnl > PNL_BREAKEVEN_EPS {
            num_winners += 1;
            gross_win_net += t.net_pnl;
            sum_win_pct += t.return_pct;
        } else if t.net_pnl < -PNL_BREAKEVEN_EPS {
            num_losers += 1;
            gross_loss_abs += t.net_pnl.abs();
            sum_loss_pct += t.return_pct;
        } else {
            num_breakeven += 1;
        }
    }

    let win_rate_pct = if num_trades > 0 {
        (num_winners as f64 / num_trades as f64) * 100.0
    } else {
        0.0
    };
    let profit_factor = if gross_loss_abs > PNL_BREAKEVEN_EPS {
        let pf = gross_win_net / gross_loss_abs;
        pf.is_finite().then_some(pf)
    } else {
        None
    };
    let expectancy_net = (num_trades > 0).then(|| realized_net_profit / num_trades as f64);
    let avg_trade_return_pct = if num_trades > 0 {
        sum_ret_pct / num_trades as f64
    } else {
        0.0
    };
    let avg_win_pct = if num_winners > 0 {
        sum_win_pct / num_winners as f64
    } else {
        0.0
    };
    let avg_loss_pct = if num_losers > 0 {
        sum_loss_pct / num_losers as f64
    } else {
        0.0
    };
    let avg_bars_held = if num_trades > 0 {
        sum_bars_held as f64 / num_trades as f64
    } else {
        0.0
    };
    let avg_r_multiple = (count_r > 0).then(|| sum_r / count_r as f64);

    BacktestTearsheet {
        initial_cash,
        final_cash,
        final_equity,
        realized_net_profit,
        total_return_abs,
        total_return_pct,
        cagr_pct,
        max_drawdown_abs: max_dd_abs,
        max_drawdown_pct: max_dd_pct,
        max_drawdown_duration_bars: max_dd_bars,
        sharpe_ratio,
        sortino_ratio,
        calmar_ratio,
        num_trades,
        num_winners,
        num_losers,
        num_breakeven,
        win_rate_pct,
        profit_factor,
        expectancy_net,
        avg_trade_return_pct,
        avg_win_pct,
        avg_loss_pct,
        avg_bars_held,
        avg_r_multiple,
        exposure_pct,
        total_fees_and_tax: closed_fees_and_tax + open_entry_fee.max(0.0),
        low_sample_warning: num_trades < MIN_STATISTICAL_SAMPLE_TRADES,
    }
}

/// Evaluates Out-of-Sample (`OOS`) generalization stability relative to In-Sample (`IS`).
#[must_use]
pub fn evaluate_robustness(
    is_sheet: &BacktestTearsheet,
    oos_sheet: &BacktestTearsheet,
) -> RobustnessVerdict {
    if is_sheet.num_trades < 3 || oos_sheet.num_trades < 2 {
        return RobustnessVerdict::LowSample;
    }
    if is_sheet.total_return_pct <= 0.0 && oos_sheet.total_return_pct <= 0.0 {
        return RobustnessVerdict::Unprofitable;
    }
    if is_sheet.total_return_pct <= 0.0 {
        // Strategy lost money in In-Sample training (`<= 0%`): NEVER classify as `Robust` or `Stable`!
        let combined_net = is_sheet.total_return_abs + oos_sheet.total_return_abs;
        return if combined_net <= 0.0 {
            RobustnessVerdict::Unprofitable
        } else {
            RobustnessVerdict::Degraded
        };
    }
    if oos_sheet.total_return_pct < 0.0
        || (is_sheet.total_return_pct >= 5.0 && oos_sheet.total_return_pct <= 0.0)
    {
        return RobustnessVerdict::Overfitted;
    }
    if oos_sheet.total_return_pct > 0.0 {
        match (is_sheet.sharpe_ratio, oos_sheet.sharpe_ratio) {
            (Some(is_sh), Some(oos_sh)) if is_sh > 0.0 && oos_sh > 0.0 => {
                if oos_sh >= is_sh * 0.65 {
                    RobustnessVerdict::Robust
                } else if oos_sh >= is_sh * 0.30 {
                    RobustnessVerdict::Stable
                } else {
                    RobustnessVerdict::Degraded
                }
            }
            _ => {
                if oos_sheet.max_drawdown_pct <= is_sheet.max_drawdown_pct * 1.5 + 2.0 {
                    RobustnessVerdict::Stable
                } else {
                    RobustnessVerdict::Degraded
                }
            }
        }
    } else {
        RobustnessVerdict::Degraded
    }
}

#[inline]
fn effective_bars_per_year(timeframe: Timeframe, n_intervals: usize, elapsed_years: f64) -> f64 {
    if n_intervals >= 10 && elapsed_years >= MIN_EMPIRICAL_ANNUALIZATION_YEARS {
        ((n_intervals as f64) / elapsed_years).clamp(1.0, 525_960.0)
    } else {
        match timeframe {
            Timeframe::M1 => 98_280.0,
            Timeframe::M5 => 19_656.0,
            Timeframe::M15 => 6_552.0,
            Timeframe::H1 => 1_638.0,
            Timeframe::H4 => 409.5,
            Timeframe::D1 => 252.0,
            Timeframe::W1 => 52.0,
        }
    }
}

fn compute_sharpe_and_sortino(returns: &[f64], bars_per_year: f64) -> (Option<f64>, Option<f64>) {
    let n = returns.len();
    if n < 2 || !bars_per_year.is_finite() || bars_per_year <= 0.0 {
        return (None, None);
    }

    let mean = returns.iter().sum::<f64>() / (n as f64);
    if !mean.is_finite() {
        return (None, None);
    }

    let mut var_sum = 0.0_f64;
    let mut downside_sq_sum = 0.0_f64;
    for &r in returns {
        let diff = r - mean;
        var_sum += diff * diff;
        if r < 0.0 {
            downside_sq_sum += r * r;
        }
    }

    let ann_root = bars_per_year.sqrt();
    let sample_std = (var_sum / ((n - 1) as f64)).sqrt();
    let sharpe = if sample_std > 1.0e-12 && sample_std.is_finite() {
        let s = (mean / sample_std) * ann_root;
        s.is_finite().then_some(s)
    } else {
        None
    };

    let downside_dev = (downside_sq_sum / (n as f64)).sqrt();
    let sortino = if downside_dev > 1.0e-12 && downside_dev.is_finite() {
        let so = (mean / downside_dev) * ann_root;
        so.is_finite().then_some(so)
    } else {
        None
    };

    (sharpe, sortino)
}
