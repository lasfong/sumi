"""Empirical Stress & Boundary Test Suite for B20 Execution Kernel.

Conducted by Challenger B20-2 (Stress & Boundary Challenger).
Tests boundary and corner cases:
1. Micro-datasets: 0-bar, 1-bar, 2-bar, 3-bar datasets with signals on bar 0, 1, or 2.
2. Extreme gap-up sizing: 1000% gap up, near-zero price, board lot 100 enforcement, no fractional shares, solvency.
3. Simultaneous SL & TP on same bar: Conservative Stop Loss precedence over Take Profit in next_open and intraday_touch modes.
4. Final bar simultaneous breach: Position open at T, bar N-1 breaches SL (and both SL+TP). Order queued at cutoff, zero access to bar N.
"""

from __future__ import annotations

import math
from datetime import date, datetime, timedelta
from types import SimpleNamespace
from typing import Generator
import numpy as np
import pandas as pd
import pytest

from app.domain.backtest.execution import BacktestExecutionKernel
from app.domain.backtest.models import BacktestKernelResult


def make_test_strategy(
    name="stress_strat",
    stop_loss=None,
    take_profit=None,
    sizing_method="fixed_quantity",
    sizing_qty=100,
    sizing_pct=100.0,
    stop_loss_price=None,
    take_profit_price=None,
):
    rm = None
    if stop_loss is not None or take_profit is not None or stop_loss_price is not None or take_profit_price is not None:
        rm = SimpleNamespace(
            stop_loss_percent=stop_loss,
            take_profit_percent=take_profit,
            stop_loss_price=stop_loss_price,
            take_profit_price=take_profit_price,
        )

    return SimpleNamespace(
        name=name,
        indicators=[
            {"id": "fast_ma", "kind": "sma", "params": {"length": 5}},
            {"id": "slow_ma", "kind": "sma", "params": {"length": 10}},
        ],
        entry_rules=[{"dsl": {"gt": ["fast_ma", "slow_ma"]}}],
        exit_rules=[{"dsl": {"lt": ["fast_ma", "slow_ma"]}}],
        position_sizing=SimpleNamespace(
            method=sizing_method,
            quantity=sizing_qty,
            percent=sizing_pct,
            model_dump=lambda: {"method": sizing_method, "quantity": sizing_qty, "percent": sizing_pct},
        ),
        risk_management=rm,
    )


def make_bars(prices: list[dict], base_date=date(2024, 1, 1)) -> pd.DataFrame:
    records = []
    for i, p in enumerate(prices):
        records.append({
            "timestamp": datetime.combine(base_date + timedelta(days=i), datetime.min.time()),
            "open": float(p.get("open", 10000.0)),
            "high": float(p.get("high", 10500.0)),
            "low": float(p.get("low", 9500.0)),
            "close": float(p.get("close", 10000.0)),
            "volume": float(p.get("volume", 100_000)),
        })
    return pd.DataFrame(records)


# ============================================================================
# 1. Micro-datasets: 0-bar, 1-bar, 2-bar, 3-bar
# ============================================================================

class TestMicroDatasets:
    def test_0_bar_dataset_raises_graceful_value_error(self):
        """0-bar DataFrame must raise a clear ValueError, not IndexError or AttributeError."""
        empty_df = pd.DataFrame(columns=["timestamp", "open", "high", "low", "close", "volume"])
        indicator_values = {"fast_ma": np.array([]), "slow_ma": np.array([])}
        strategy = make_test_strategy()
        kernel = BacktestExecutionKernel()

        with pytest.raises(ValueError, match="Cannot run backtest on empty DataFrame"):
            kernel.run(
                df=empty_df,
                indicator_values=indicator_values,
                strategy=strategy,
                symbol="MICRO_0",
                initial_cash=100_000_000.0,
            )

    def test_1_bar_dataset_zero_lookahead_no_signals(self):
        """1-bar DataFrame: Bar 0 has no preceding bar to evaluate signals (i >= 1 is False).
        Must complete gracefully without IndexError, 0 orders, 0 trades, cash intact.
        """
        df = make_bars([{"open": 10000.0, "high": 10500.0, "low": 9500.0, "close": 10000.0}])
        indicator_values = {"fast_ma": np.array([30.0]), "slow_ma": np.array([20.0])}
        strategy = make_test_strategy()
        kernel = BacktestExecutionKernel()

        result = kernel.run(
            df=df,
            indicator_values=indicator_values,
            strategy=strategy,
            symbol="MICRO_1",
            initial_cash=100_000_000.0,
        )

        assert isinstance(result, BacktestKernelResult)
        assert result.total_candles == 1
        assert result.last_analyzed_index == 0
        assert len(result.ledger.orders) == 0
        assert len(result.ledger.trades) == 0
        assert result.ledger.current_cash == 100_000_000.0
        assert len(result.ledger.equity_curve) == 1

    def test_2_bar_dataset_signal_on_bar_1_is_cutoff(self):
        """2-bar DataFrame: Signal on bar 1 (the final bar N-1) must be marked UNFILLED_AT_CUTOFF.
        Zero trades executed, zero cash delta, no index out of range.
        """
        df = make_bars([
            {"open": 10000.0, "high": 10500.0, "low": 9500.0, "close": 10000.0},
            {"open": 10200.0, "high": 10600.0, "low": 10000.0, "close": 10500.0},
        ])
        # Bar 0: fast <= slow, Bar 1: fast > slow (crossover at bar 1 close)
        indicator_values = {
            "fast_ma": np.array([10.0, 30.0]),
            "slow_ma": np.array([20.0, 20.0]),
        }
        strategy = make_test_strategy(sizing_method="fixed_quantity", sizing_qty=100)
        kernel = BacktestExecutionKernel()

        result = kernel.run(
            df=df,
            indicator_values=indicator_values,
            strategy=strategy,
            symbol="MICRO_2",
            initial_cash=50_000_000.0,
        )

        assert result.total_candles == 2
        assert result.last_analyzed_index == 1
        assert len(result.ledger.executions) == 0
        assert len(result.ledger.trades) == 0
        assert result.ledger.current_cash == 50_000_000.0

        cutoff_orders = [o for o in result.ledger.orders if o.status == "UNFILLED_AT_CUTOFF"]
        assert len(cutoff_orders) == 1
        assert cutoff_orders[0].signal_bar_index == 1
        assert cutoff_orders[0].order_type == "MARKET_NEXT_OPEN"
        assert len(result.ledger.unexecuted_signals) == 1
        assert result.ledger.unexecuted_signals[0]["reason"] == "UNFILLED_AT_CUTOFF"

    def test_3_bar_dataset_signal_on_bar_1_fills_on_bar_2_and_closes_or_cutoff(self):
        """3-bar DataFrame: Signal on bar 1 fills on bar 2 Open.
        If an exit signal is generated on bar 2 close (final bar), exit is UNFILLED_AT_CUTOFF.
        Position remains open at end of backtest.
        """
        df = make_bars([
            {"open": 10000.0, "high": 10500.0, "low": 9500.0, "close": 10000.0},
            {"open": 10200.0, "high": 10600.0, "low": 10000.0, "close": 10500.0},
            {"open": 10600.0, "high": 11000.0, "low": 10400.0, "close": 10800.0},
        ])
        # Entry signal at bar 1, exit signal at bar 2
        indicator_values = {
            "fast_ma": np.array([10.0, 30.0, 5.0]),
            "slow_ma": np.array([20.0, 20.0, 20.0]),
        }
        # In explicit profile mode or min_holding_bars=0 to test exit evaluation
        strategy = make_test_strategy(sizing_method="fixed_quantity", sizing_qty=100)
        kernel = BacktestExecutionKernel(min_holding_bars=0)

        result = kernel.run(
            df=df,
            indicator_values=indicator_values,
            strategy=strategy,
            symbol="MICRO_3",
            initial_cash=50_000_000.0,
        )

        assert result.total_candles == 3
        # Bar 1 entry filled at Bar 2 Open
        assert len(result.ledger.executions) == 1
        assert result.ledger.executions[0].fill_bar_index == 2
        assert result.ledger.executions[0].price == 10600.0

        # Bar 2 exit signal is on final bar -> UNFILLED_AT_CUTOFF
        cutoff_orders = [o for o in result.ledger.orders if o.status == "UNFILLED_AT_CUTOFF"]
        assert len(cutoff_orders) == 1
        assert cutoff_orders[0].side == "SELL"
        assert cutoff_orders[0].signal_bar_index == 2

        # Position remains open, not closed
        assert len([t for t in result.ledger.trades if t.status == "open"]) == 1
        assert len([t for t in result.ledger.trades if t.status == "closed"]) == 0
        assert result.open_position.quantity == 100.0


# ============================================================================
# 2. Extreme gap-up sizing (1000% gap up, near zero price, lot_size=100)
# ============================================================================

class TestExtremeGapUpAndPricingSizing:
    def test_extreme_gap_up_1000_percent_fill_estimate_sizing(self):
        """Test sizing_basis='fill_estimate' when Open[T+1] gaps up by 1000% (11x).
        Verify:
        - lot_size=100 is strictly respected (quantity % 100 == 0).
        - No fractional shares.
        - Cash balance is strictly non-negative.
        - Quantity is causally recomputed downwards based on Open[T+1].
        """
        initial_cash = 100_000_000.0
        # Bar 1 close = 10,000. Bar 2 open = 110,000 (+1000% gap up)
        df = make_bars([
            {"open": 10000.0, "high": 10500.0, "low": 9500.0, "close": 10000.0},
            {"open": 10000.0, "high": 10500.0, "low": 9500.0, "close": 10000.0},
            {"open": 110000.0, "high": 115000.0, "low": 108000.0, "close": 112000.0},
            {"open": 112000.0, "high": 115000.0, "low": 108000.0, "close": 112000.0},
        ])
        indicator_values = {
            "fast_ma": np.array([10.0, 30.0, 30.0, 30.0]),
            "slow_ma": np.array([20.0, 20.0, 20.0, 20.0]),
        }
        # 100% equity sizing
        strategy = make_test_strategy(sizing_method="percent_equity", sizing_pct=100.0)
        kernel = BacktestExecutionKernel(sizing_basis="fill_estimate")

        result = kernel.run(
            df=df,
            indicator_values=indicator_values,
            strategy=strategy,
            symbol="GAP_1000",
            initial_cash=initial_cash,
        )

        assert len(result.ledger.executions) == 1
        exec_0 = result.ledger.executions[0]
        assert exec_0.price == 110000.0

        # At Bar 1 close (10,000): provisional qty was 9,900 shares
        order_0 = result.ledger.orders[0]
        assert order_0.provisional_quantity == 9900.0

        # At Bar 2 open (110,000):
        # cost_per_share = 110,000 * 1.0015 = 110,165
        # 100_000_000 / 110,165 = 907.72 -> 900 shares
        assert exec_0.quantity == 900.0
        assert exec_0.quantity % 100 == 0
        assert float(exec_0.quantity).is_integer()

        # Cash balance is strictly non-negative
        assert result.ledger.current_cash >= 0.0
        assert exec_0.net_amount <= initial_cash
        assert math.isclose(
            result.ledger.current_cash + exec_0.net_amount,
            initial_cash,
            abs_tol=1e-5,
        )

    def test_extreme_gap_up_1000_percent_insufficient_for_1_lot(self):
        """When 1000% gap up makes even 1 board lot (100 shares) unaffordable,
        order must be rejected and cash must remain 100% intact.
        """
        initial_cash = 5_000_000.0  # 5 million VND
        # Bar 1 close = 10,000 (100 shares = ~1,001,500 <= 5M -> queued)
        # Bar 2 open = 110,000 (100 shares = ~11,016,500 > 5M -> cannot afford)
        df = make_bars([
            {"open": 10000.0, "high": 10500.0, "low": 9500.0, "close": 10000.0},
            {"open": 10000.0, "high": 10500.0, "low": 9500.0, "close": 10000.0},
            {"open": 110000.0, "high": 115000.0, "low": 108000.0, "close": 112000.0},
        ])
        indicator_values = {
            "fast_ma": np.array([10.0, 30.0, 30.0]),
            "slow_ma": np.array([20.0, 20.0, 20.0]),
        }
        strategy = make_test_strategy(sizing_method="fixed_quantity", sizing_qty=100)
        kernel = BacktestExecutionKernel(sizing_basis="fill_estimate")

        result = kernel.run(
            df=df,
            indicator_values=indicator_values,
            strategy=strategy,
            symbol="GAP_UNAFFORDABLE",
            initial_cash=initial_cash,
        )

        assert len(result.ledger.executions) == 0
        assert result.ledger.current_cash == initial_cash
        rejected = [o for o in result.ledger.orders if o.status == "REJECTED"]
        assert len(rejected) == 1
        assert "insufficient cash" in rejected[0].rejection_reason.lower()

    def test_near_zero_price_lot_sizing(self):
        """Near-zero price (e.g. 0.001 VND):
        Verify that lot_size=100 is respected, no fractional shares occur,
        and cash balance remains strictly non-negative.
        """
        initial_cash = 1_000_000.0
        df = make_bars([
            {"open": 0.001, "high": 0.002, "low": 0.0005, "close": 0.001},
            {"open": 0.001, "high": 0.002, "low": 0.0005, "close": 0.001},
            {"open": 0.001, "high": 0.002, "low": 0.0005, "close": 0.001},
        ])
        indicator_values = {
            "fast_ma": np.array([10.0, 30.0, 30.0]),
            "slow_ma": np.array([20.0, 20.0, 20.0]),
        }
        strategy = make_test_strategy(sizing_method="percent_equity", sizing_pct=100.0)
        kernel = BacktestExecutionKernel(sizing_basis="fill_estimate")

        result = kernel.run(
            df=df,
            indicator_values=indicator_values,
            strategy=strategy,
            symbol="NEAR_ZERO",
            initial_cash=initial_cash,
        )

        assert len(result.ledger.executions) == 1
        exec_0 = result.ledger.executions[0]
        assert exec_0.quantity % 100 == 0
        assert float(exec_0.quantity).is_integer()
        assert exec_0.net_amount <= initial_cash
        assert result.ledger.current_cash >= 0.0


# ============================================================================
# 3. Simultaneous SL & TP on the same bar: Stop Loss precedence
# ============================================================================

class TestSimultaneousStopLossAndTakeProfit:
    def test_simultaneous_sl_and_tp_next_open_mode(self):
        """Under sl_tp_execution_mode='next_open':
        When Bar T+2 range breaches both Stop Loss AND Take Profit on the same bar,
        Stop Loss MUST strictly take precedence over Take Profit in conservative fashion.
        Order must queue at Bar T+2 with exit_reason='STOP_LOSS' and fill at Open[T+3].
        """
        df = make_bars([
            {"open": 10000.0, "high": 10500.0, "low": 9500.0, "close": 10000.0},
            {"open": 10000.0, "high": 10500.0, "low": 9500.0, "close": 10000.0},
            {"open": 10000.0, "high": 10200.0, "low": 9800.0, "close": 10000.0},  # Filled at 10000
            # Bar 3: Low 9000 breaches SL 9500 AND High 11000 breaches TP 10500 simultaneously!
            {"open": 10000.0, "high": 11000.0, "low": 9000.0, "close": 10000.0},
            # Bar 4: Next-open fill bar
            {"open": 9800.0, "high": 10200.0, "low": 9600.0, "close": 9800.0},
        ])
        # Entry signal on bar 1 close -> fills bar 2 open
        indicator_values = {
            "fast_ma": np.array([10.0, 30.0, 30.0, 30.0, 30.0]),
            "slow_ma": np.array([20.0, 20.0, 20.0, 20.0, 20.0]),
        }
        strategy = make_test_strategy(
            stop_loss=5.0,  # SL = 9500
            take_profit=5.0,  # TP = 10500
            stop_loss_price=9500.0,
            take_profit_price=10500.0,
            sizing_method="fixed_quantity",
            sizing_qty=100,
        )
        strategy.exit_rules = []

        kernel = BacktestExecutionKernel(min_holding_bars=0, sl_tp_execution_mode="next_open")
        result = kernel.run(
            df=df,
            indicator_values=indicator_values,
            strategy=strategy,
            symbol="SL_TP_SIMUL",
            initial_cash=10_000_000.0,
        )

        closed = [t for t in result.ledger.trades if t.status == "closed"]
        assert len(closed) == 1
        trade = closed[0]
        # Must be STOP_LOSS, NOT TAKE_PROFIT
        assert trade.exit_reason == "STOP_LOSS"
        assert trade.exit_bar_index == 4  # Filled on bar 4 Open
        assert trade.exit_price == 9800.0  # Open[4]

        # Verify queued order was STOP_LOSS
        sell_orders = [o for o in result.ledger.orders if o.side == "SELL"]
        assert len(sell_orders) == 1
        assert sell_orders[0].exit_reason == "STOP_LOSS"
        assert sell_orders[0].requested_price == 9500.0  # SL trigger price

    def test_simultaneous_sl_and_tp_intraday_touch_mode(self):
        """Under sl_tp_execution_mode='intraday_touch':
        When same bar breaches both SL and TP, Stop Loss takes precedence,
        filling at the SL price (or worse if gap), NOT Take Profit.
        """
        df = make_bars([
            {"open": 10000.0, "high": 10500.0, "low": 9500.0, "close": 10000.0},
            {"open": 10000.0, "high": 10500.0, "low": 9500.0, "close": 10000.0},
            {"open": 10000.0, "high": 10200.0, "low": 9800.0, "close": 10000.0},  # Filled at 10000
            # Bar 3: Low 9000 breaches SL 9500 AND High 11000 breaches TP 10500
            {"open": 10000.0, "high": 11000.0, "low": 9000.0, "close": 10000.0},
        ])
        indicator_values = {
            "fast_ma": np.array([10.0, 30.0, 30.0, 30.0]),
            "slow_ma": np.array([20.0, 20.0, 20.0, 20.0]),
        }
        strategy = make_test_strategy(
            stop_loss=5.0,
            take_profit=5.0,
            stop_loss_price=9500.0,
            take_profit_price=10500.0,
            sizing_method="fixed_quantity",
            sizing_qty=100,
        )
        strategy.exit_rules = []

        kernel = BacktestExecutionKernel(min_holding_bars=0, sl_tp_execution_mode="intraday_touch")
        result = kernel.run(
            df=df,
            indicator_values=indicator_values,
            strategy=strategy,
            symbol="SL_TP_INTRADAY",
            initial_cash=10_000_000.0,
        )

        closed = [t for t in result.ledger.trades if t.status == "closed"]
        assert len(closed) == 1
        trade = closed[0]
        assert trade.exit_reason == "STOP_LOSS"
        assert trade.exit_bar_index == 3
        assert trade.exit_price == 9500.0  # Filled at SL trigger price


# ============================================================================
# 4. Final bar simultaneous breach: Cutoff, zero access to bar N
# ============================================================================

class TestFinalBarSimultaneousBreach:
    def test_final_bar_sl_and_tp_breach_next_open_cutoff(self):
        """Position open at Bar 2. Bar N-1 (Bar 3) breaches both SL and TP.
        Under default next_open:
        - Must record UNFILLED_AT_CUTOFF for the SELL order with exit_reason='STOP_LOSS'.
        - Trade remains OPEN (not closed).
        - Must NOT attempt to index into non-existent Bar 4 (total_candles=4, indices 0..3).
        - Zero unhandled exceptions.
        """
        total_bars = 4
        df = make_bars([
            {"open": 10000.0, "high": 10500.0, "low": 9500.0, "close": 10000.0},
            {"open": 10000.0, "high": 10500.0, "low": 9500.0, "close": 10000.0},  # Signal at bar 1 close
            {"open": 10000.0, "high": 10200.0, "low": 9800.0, "close": 10000.0},  # Entry fill at bar 2 open
            # Bar 3 (final bar N-1): Low breaches SL 9500 AND High breaches TP 10500
            {"open": 10000.0, "high": 11000.0, "low": 9000.0, "close": 9800.0},
        ])
        assert len(df) == total_bars

        indicator_values = {
            "fast_ma": np.array([10.0, 30.0, 30.0, 30.0]),
            "slow_ma": np.array([20.0, 20.0, 20.0, 20.0]),
        }
        strategy = make_test_strategy(
            stop_loss=5.0,
            take_profit=5.0,
            stop_loss_price=9500.0,
            take_profit_price=10500.0,
            sizing_method="fixed_quantity",
            sizing_qty=100,
        )
        strategy.exit_rules = []

        kernel = BacktestExecutionKernel(min_holding_bars=0, sl_tp_execution_mode="next_open")
        result = kernel.run(
            df=df,
            indicator_values=indicator_values,
            strategy=strategy,
            symbol="FINAL_BREACH",
            initial_cash=10_000_000.0,
        )

        assert result.total_candles == total_bars
        assert result.last_analyzed_index == total_bars - 1

        # Trade must remain OPEN because exit cannot execute after final bar
        closed_trades = [t for t in result.ledger.trades if t.status == "closed"]
        assert len(closed_trades) == 0

        open_trades = [t for t in result.ledger.trades if t.status == "open"]
        assert len(open_trades) == 1
        assert open_trades[0].quantity == 100.0

        # Cutoff order must exist with STOP_LOSS reason
        cutoff_orders = [o for o in result.ledger.orders if o.status == "UNFILLED_AT_CUTOFF"]
        assert len(cutoff_orders) == 1
        assert cutoff_orders[0].side == "SELL"
        assert cutoff_orders[0].exit_reason == "STOP_LOSS"
        assert cutoff_orders[0].signal_bar_index == total_bars - 1
        assert "STOP_LOSS breach on final bar" in cutoff_orders[0].rejection_reason

        # Unexecuted signal recorded
        assert len(result.ledger.unexecuted_signals) == 1
        assert result.ledger.unexecuted_signals[0]["action"] == "SELL"
        assert result.ledger.unexecuted_signals[0]["reason"] == "UNFILLED_AT_CUTOFF"

        # Position is open and accurately tracked
        assert result.open_position.quantity == 100.0
        assert len(result.ledger.equity_curve) == total_bars


# ============================================================================
# 5. Additional Adversarial Edge Cases
# ============================================================================

class TestAdversarialEdgeCases:
    def test_t2_settlement_locks_sl_tp_until_settled(self):
        """Under Vietnam T+2 settlement, SL/TP cannot trigger on bar of entry or T+1.
        It can only trigger once shares become sellable (T+2).
        """
        base_date = date(2024, 1, 1)  # Monday
        # Bar 0: Mon (Jan 1)
        # Bar 1: Tue (Jan 2) -> signal at close
        # Bar 2: Wed (Jan 3) -> fill at open. Low breaches SL, but locked T+2!
        # Bar 3: Thu (Jan 4) -> Low breaches SL, but still locked (T+1 session)!
        # Bar 4: Fri (Jan 5) -> Settled! Low breaches SL -> Exit triggered!
        # Bar 5: Mon (Jan 8) -> Exit fills at Open under next_open.
        df = pd.DataFrame([
            {"timestamp": datetime.combine(date(2024, 1, 1), datetime.min.time()), "open": 10000.0, "high": 10500.0, "low": 9500.0, "close": 10000.0, "volume": 100000},
            {"timestamp": datetime.combine(date(2024, 1, 2), datetime.min.time()), "open": 10000.0, "high": 10500.0, "low": 9500.0, "close": 10000.0, "volume": 100000},
            {"timestamp": datetime.combine(date(2024, 1, 3), datetime.min.time()), "open": 10000.0, "high": 10500.0, "low": 9000.0, "close": 9800.0, "volume": 100000},  # Low 9000 < SL 9500
            {"timestamp": datetime.combine(date(2024, 1, 4), datetime.min.time()), "open": 9800.0, "high": 10000.0, "low": 9000.0, "close": 9500.0, "volume": 100000},   # Low 9000 < SL 9500
            {"timestamp": datetime.combine(date(2024, 1, 5), datetime.min.time()), "open": 9500.0, "high": 9700.0, "low": 9000.0, "close": 9200.0, "volume": 100000},    # Low 9000 < SL 9500 -> triggers here!
            {"timestamp": datetime.combine(date(2024, 1, 8), datetime.min.time()), "open": 9100.0, "high": 9300.0, "low": 8900.0, "close": 9000.0, "volume": 100000},    # Fills here!
        ])
        indicator_values = {
            "fast_ma": np.array([10.0, 30.0, 30.0, 30.0, 30.0, 30.0]),
            "slow_ma": np.array([20.0, 20.0, 20.0, 20.0, 20.0, 20.0]),
        }
        strategy = make_test_strategy(
            stop_loss=5.0,
            stop_loss_price=9500.0,
            sizing_method="fixed_quantity",
            sizing_qty=100,
        )
        strategy.exit_rules = []

        # Use conservative profile with T+2 settlement enabled
        kernel = BacktestExecutionKernel(min_holding_bars=2, sl_tp_execution_mode="next_open")
        result = kernel.run(
            df=df,
            indicator_values=indicator_values,
            strategy=strategy,
            symbol="T2_SL_LOCK",
            initial_cash=10_000_000.0,
        )

        closed = [t for t in result.ledger.trades if t.status == "closed"]
        assert len(closed) == 1
        trade = closed[0]
        # Must NOT exit on bar 2 or 3; must trigger on bar 4 and fill on bar 5
        assert trade.entry_bar_index == 2
        assert trade.exit_bar_index == 5
        assert trade.exit_price == 9100.0
        assert trade.exit_reason == "STOP_LOSS"

    def test_extreme_gap_down_90_percent_sizing(self):
        """When Open[T+1] drops by 90%, fill_estimate sizes safely within cash."""
        initial_cash = 100_000_000.0
        df = make_bars([
            {"open": 10000.0, "high": 10500.0, "low": 9500.0, "close": 10000.0},
            {"open": 10000.0, "high": 10500.0, "low": 9500.0, "close": 10000.0},
            {"open": 1000.0, "high": 1200.0, "low": 900.0, "close": 1000.0},  # -90% drop
            {"open": 1000.0, "high": 1200.0, "low": 900.0, "close": 1000.0},
        ])
        indicator_values = {
            "fast_ma": np.array([10.0, 30.0, 30.0, 30.0]),
            "slow_ma": np.array([20.0, 20.0, 20.0, 20.0]),
        }
        strategy = make_test_strategy(sizing_method="percent_equity", sizing_pct=100.0)
        kernel = BacktestExecutionKernel(sizing_basis="fill_estimate")

        result = kernel.run(
            df=df,
            indicator_values=indicator_values,
            strategy=strategy,
            symbol="GAP_DOWN",
            initial_cash=initial_cash,
        )

        assert len(result.ledger.executions) == 1
        exec_0 = result.ledger.executions[0]
        assert exec_0.price == 1000.0
        assert exec_0.quantity % 100 == 0
        assert exec_0.net_amount <= initial_cash
        assert result.ledger.current_cash >= 0.0

    def test_zero_cash_handling(self):
        """Zero initial cash: orders must be rejected gracefully with 0 executions."""
        df = make_bars([
            {"open": 10000.0, "high": 10500.0, "low": 9500.0, "close": 10000.0},
            {"open": 10000.0, "high": 10500.0, "low": 9500.0, "close": 10000.0},
            {"open": 10000.0, "high": 10500.0, "low": 9500.0, "close": 10000.0},
        ])
        indicator_values = {
            "fast_ma": np.array([10.0, 30.0, 30.0]),
            "slow_ma": np.array([20.0, 20.0, 20.0]),
        }
        strategy = make_test_strategy(sizing_method="percent_equity", sizing_pct=100.0)
        kernel = BacktestExecutionKernel(sizing_basis="fill_estimate")

        result = kernel.run(
            df=df,
            indicator_values=indicator_values,
            strategy=strategy,
            symbol="ZERO_CASH",
            initial_cash=0.0,
        )

        assert len(result.ledger.executions) == 0
        assert result.ledger.current_cash == 0.0
        # Order should be rejected
        assert len(result.ledger.orders) >= 1
        assert result.ledger.orders[0].status == "REJECTED"

    def test_extreme_gap_up_with_slippage_maintains_solvency(self):
        """Gap-up with slippage: net cost including slippage and fee must not breach available cash."""
        initial_cash = 10_000_000.0
        slippage = 0.02  # 2% slippage
        fee = 0.0015
        df = make_bars([
            {"open": 10000.0, "high": 10500.0, "low": 9500.0, "close": 10000.0},
            {"open": 10000.0, "high": 10500.0, "low": 9500.0, "close": 10000.0},
            {"open": 50000.0, "high": 52000.0, "low": 48000.0, "close": 50000.0},  # +400% gap up
        ])
        indicator_values = {
            "fast_ma": np.array([10.0, 30.0, 30.0]),
            "slow_ma": np.array([20.0, 20.0, 20.0]),
        }
        strategy = make_test_strategy(sizing_method="percent_equity", sizing_pct=100.0)
        kernel = BacktestExecutionKernel(
            fee_rate=fee,
            slippage_rate=slippage,
            sizing_basis="fill_estimate",
        )

        result = kernel.run(
            df=df,
            indicator_values=indicator_values,
            strategy=strategy,
            symbol="SLIPPAGE_SOLVENCY",
            initial_cash=initial_cash,
        )

        assert len(result.ledger.executions) == 1
        exec_0 = result.ledger.executions[0]
        # Fill price = 50000 * 1.02 = 51000
        assert exec_0.price == 51000.0
        assert exec_0.quantity % 100 == 0
        assert exec_0.net_amount <= initial_cash
        assert result.ledger.current_cash >= 0.0

    def test_zero_open_price_raises_zerodivisionerror_demonstrating_missing_guard(self):
        """When open_price == 0.0 on fill bar, kernel guards against non-positive fill prices,
        cleanly rejecting the order without raising ZeroDivisionError.
        """
        df = make_bars([
            {"open": 10000.0, "high": 10500.0, "low": 9500.0, "close": 10000.0},
            {"open": 10000.0, "high": 10500.0, "low": 9500.0, "close": 10000.0},
            {"open": 0.0, "high": 100.0, "low": 0.0, "close": 0.0},
        ])
        indicator_values = {
            "fast_ma": np.array([10.0, 30.0, 30.0]),
            "slow_ma": np.array([20.0, 20.0, 20.0]),
        }
        strategy = make_test_strategy()
        kernel = BacktestExecutionKernel()

        result = kernel.run(
            df=df,
            indicator_values=indicator_values,
            strategy=strategy,
            symbol="ZERO_OPEN",
            initial_cash=100_000_000.0,
        )

        assert len(result.ledger.executions) == 0
        assert result.ledger.current_cash == 100_000_000.0
        rejected_orders = [o for o in result.ledger.orders if o.status == "REJECTED"]
        assert len(rejected_orders) == 1
        assert "non-positive" in rejected_orders[0].rejection_reason.lower()

    def test_contract_gap_stop_loss_pct_attribute_name_discrepancy(self):
        """Verifies contract resolution:
        StrategyConfig from strategy_schema.py provides 'stop_loss_pct',
        and execution.py recognizes 'stop_loss_pct', successfully triggering
        the stop loss exit.
        """
        strat_with_pct = SimpleNamespace(
            name="test_pct",
            entry_rules=[{"dsl": {"gt": ["fast_ma", "slow_ma"]}}],
            exit_rules=[],
            position_sizing=SimpleNamespace(method="fixed_quantity", quantity=100),
            risk_management=SimpleNamespace(stop_loss_pct=5.0),  # Schema field name
        )
        df = make_bars([
            {"open": 10000.0, "high": 10500.0, "low": 9500.0, "close": 10000.0},
            {"open": 10000.0, "high": 10500.0, "low": 9500.0, "close": 10000.0},
            {"open": 10000.0, "high": 10200.0, "low": 9800.0, "close": 10000.0},
            {"open": 9800.0, "high": 9900.0, "low": 8000.0, "close": 8200.0},  # Breaches 5% SL (9500)
            {"open": 8200.0, "high": 8500.0, "low": 8000.0, "close": 8300.0},
        ])
        indicator_values = {
            "fast_ma": np.array([10.0, 30.0, 30.0, 30.0, 30.0]),
            "slow_ma": np.array([20.0, 20.0, 20.0, 20.0, 20.0]),
        }
        kernel = BacktestExecutionKernel(min_holding_bars=0, sl_tp_execution_mode="next_open")
        result = kernel.run(
            df=df,
            indicator_values=indicator_values,
            strategy=strat_with_pct,
            symbol="PCT_NAME_GAP",
            initial_cash=10_000_000.0,
        )
        # stop_loss_pct is recognized and triggers stop loss exit
        closed_trades = [t for t in result.ledger.trades if t.status == "closed"]
        assert len(closed_trades) == 1
        trade = closed_trades[0]
        assert trade.exit_reason == "STOP_LOSS"
        assert trade.exit_price == 8200.0
        assert trade.exit_bar_index == 4


