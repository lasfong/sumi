"""Unit and integration tests for pure in-memory next-event backtest kernel.

Validates:
- BT-TIME-001: Signal on Close[T] fills at Open[T+1], never same-bar Close[T].
- BT-TIME-002: Signal, order, and fill timestamps are strictly separated.
- TEST-BT-001: Signal on final cutoff bar N-1 is captured as UNFILLED_AT_CUTOFF.
- NFR-DET-001: Pure in-memory calculation with zero DB interactions during simulation.
- Vietnam lot sizing (100 shares) and graceful insufficient cash rejection.
- Simultaneous SL/TP precedence (Stop Loss executed first).
"""

from datetime import date, datetime, timedelta
from types import SimpleNamespace
import numpy as np
import pandas as pd
import pytest

from app.domain.backtest.execution import BacktestExecutionKernel
from app.domain.backtest.models import BacktestKernelResult


def make_sample_strategy(stop_loss=None, take_profit=None, sizing_method="fixed_quantity", sizing_qty=100):
    return SimpleNamespace(
        name="test_cross_strategy",
        indicators=[
            {"id": "fast_ma", "kind": "sma", "params": {"length": 5}},
            {"id": "slow_ma", "kind": "sma", "params": {"length": 10}},
        ],
        entry_rules=[{
            "dsl": {"gt": ["fast_ma", "slow_ma"]},
        }],
        exit_rules=[{
            "dsl": {"lt": ["fast_ma", "slow_ma"]},
        }],
        position_sizing=SimpleNamespace(
            method=sizing_method,
            quantity=sizing_qty,
            percent=100.0,
            model_dump=lambda: {"method": sizing_method, "quantity": sizing_qty},
        ),
        risk_management=SimpleNamespace(
            stop_loss_percent=stop_loss,
            take_profit_percent=take_profit,
        ) if (stop_loss or take_profit) else None,
    )


def test_signal_close_fills_at_next_open():
    """Verify that a strategy signal evaluated at Close[T] fills at Open[T+1]."""
    base_date = date(2024, 1, 1)
    n = 20
    # Construct candles where on bar 5, fast_ma crosses above slow_ma
    candles = []
    for i in range(n):
        open_val = 100.0 + i * 2.0
        high_val = open_val + 2.0
        low_val = open_val - 2.0
        close_val = open_val + 1.0
        candles.append({
            "timestamp": datetime.combine(base_date + timedelta(days=i), datetime.min.time()),
            "open": open_val,
            "high": high_val,
            "low": low_val,
            "close": close_val,
            "volume": 500_000,
        })
    df = pd.DataFrame(candles)

    # Indicator snapshots: fast crosses above slow at bar 5
    # (i.e. at bar 4: fast <= slow; at bar 5: fast > slow)
    fast_ma = np.array([10.0] * 5 + [25.0] * 15)
    slow_ma = np.array([20.0] * n)
    indicator_values = {
        "fast_ma": fast_ma,
        "slow_ma": slow_ma,
    }

    strategy = make_sample_strategy()
    kernel = BacktestExecutionKernel()
    result = kernel.run(
        df=df,
        indicator_values=indicator_values,
        strategy=strategy,
        symbol="VNM",
        initial_cash=100_000_000,
    )

    assert len(result.ledger.executions) >= 1
    first_exec = result.ledger.executions[0]

    # Signal was evaluated at bar 5 Close
    assert first_exec.signal_bar_index == 5
    assert first_exec.signal_timestamp == df.iloc[5]["timestamp"]

    # Fill MUST occur at bar 6 Open (BT-TIME-001)
    assert first_exec.fill_bar_index == 6
    assert first_exec.fill_timestamp == df.iloc[6]["timestamp"]
    assert first_exec.price == pytest.approx(df.iloc[6]["open"])
    assert first_exec.price != df.iloc[5]["close"]  # Proves no same-bar close fill!


def test_final_bar_signal_unfilled_cutoff():
    """Verify that a signal on the last available bar is recorded as UNFILLED_AT_CUTOFF."""
    base_date = date(2024, 1, 1)
    n = 10
    candles = [{
        "timestamp": datetime.combine(base_date + timedelta(days=i), datetime.min.time()),
        "open": 50.0 + i,
        "high": 52.0 + i,
        "low": 48.0 + i,
        "close": 50.5 + i,
        "volume": 100_000,
    } for i in range(n)]
    df = pd.DataFrame(candles)

    # Trigger cross only on the final bar (n - 1)
    fast_ma = np.array([10.0] * (n - 1) + [30.0])
    slow_ma = np.array([20.0] * n)
    indicator_values = {"fast_ma": fast_ma, "slow_ma": slow_ma}

    strategy = make_sample_strategy()
    kernel = BacktestExecutionKernel()
    result = kernel.run(
        df=df,
        indicator_values=indicator_values,
        strategy=strategy,
        symbol="FPT",
        initial_cash=50_000_000,
    )

    # No fill should have occurred because there is no bar n
    assert len(result.ledger.executions) == 0

    # Order must exist with UNFILLED_AT_CUTOFF status
    cutoff_orders = [o for o in result.ledger.orders if o.status == "UNFILLED_AT_CUTOFF"]
    assert len(cutoff_orders) == 1
    assert cutoff_orders[0].signal_bar_index == n - 1

    # Unexecuted signals ledger list
    assert len(result.ledger.unexecuted_signals) == 1
    assert result.ledger.unexecuted_signals[0]["reason"] == "UNFILLED_AT_CUTOFF"


def test_zero_db_interaction_during_simulation():
    """Verify that the execution kernel is pure in-memory and operates without DB sessions."""
    base_date = date(2024, 1, 1)
    candles = [{
        "timestamp": datetime.combine(base_date + timedelta(days=i), datetime.min.time()),
        "open": 20.0 + i,
        "high": 21.0 + i,
        "low": 19.0 + i,
        "close": 20.2 + i,
        "volume": 50_000,
    } for i in range(15)]
    df = pd.DataFrame(candles)

    indicator_values = {
        "fast_ma": np.array([10.0] * 5 + [30.0] * 5 + [5.0] * 5),
        "slow_ma": np.array([20.0] * 15),
    }

    strategy = make_sample_strategy()
    kernel = BacktestExecutionKernel()

    # Call run without any DB session or mocks - proves pure in-memory decoupling
    result = kernel.run(
        df=df,
        indicator_values=indicator_values,
        strategy=strategy,
        symbol="HPG",
        initial_cash=20_000_000,
    )

    assert result.symbol == "HPG"
    assert result.ledger.portfolio_value > 0
    assert len(result.ledger.equity_curve) == 15


def test_vietnam_lot_sizing_and_insufficient_cash():
    """Verify that quantity is clipped to 100-share lots and rejected if cash < 100 shares."""
    base_date = date(2024, 1, 1)
    # Price is 50,000 VND. 100 shares = 5,000,000 VND
    candles = [{
        "timestamp": datetime.combine(base_date + timedelta(days=i), datetime.min.time()),
        "open": 50_000.0,
        "high": 51_000.0,
        "low": 49_000.0,
        "close": 50_000.0,
        "volume": 100_000,
    } for i in range(10)]
    df = pd.DataFrame(candles)

    fast_ma = np.array([10.0] * 3 + [30.0] * 7)
    slow_ma = np.array([20.0] * 10)
    indicator_values = {"fast_ma": fast_ma, "slow_ma": slow_ma}

    # Case A: Cash is 3,000,000 VND (less than 100 shares * 50,000 = 5,000,000)
    strategy = make_sample_strategy(sizing_method="percent_equity", sizing_qty=100)
    kernel = BacktestExecutionKernel()
    result_insufficient = kernel.run(
        df=df,
        indicator_values=indicator_values,
        strategy=strategy,
        symbol="SSI",
        initial_cash=3_000_000,
    )

    assert len(result_insufficient.ledger.executions) == 0
    assert any("rejected" in w.lower() for w in result_insufficient.ledger.warnings)

    # Case B: Cash is 18,000,000 VND (can buy 300 shares = 15,000,000, not 400 shares = 20,000,000)
    result_sufficient = kernel.run(
        df=df,
        indicator_values=indicator_values,
        strategy=strategy,
        symbol="SSI",
        initial_cash=18_000_000,
    )
    assert len(result_sufficient.ledger.executions) >= 1
    # Quantity must be a multiple of 100
    qty = result_sufficient.ledger.executions[0].quantity
    assert qty % 100 == 0
    assert qty == 300.0


def test_simultaneous_sl_tp_precedence():
    """Verify conservative Stop Loss precedence when both SL and TP are hit on the same bar."""
    base_date = date(2024, 1, 1)
    candles = [
        # Bar 0: initial
        {"timestamp": datetime.combine(base_date, datetime.min.time()), "open": 100.0, "high": 101.0, "low": 99.0, "close": 100.0, "volume": 100_000},
        # Bar 1: Entry signal on Close
        {"timestamp": datetime.combine(base_date + timedelta(days=1), datetime.min.time()), "open": 100.0, "high": 101.0, "low": 99.0, "close": 102.0, "volume": 100_000},
        # Bar 2: Buy fills at Open 102.0
        {"timestamp": datetime.combine(base_date + timedelta(days=2), datetime.min.time()), "open": 102.0, "high": 103.0, "low": 101.0, "close": 102.5, "volume": 100_000},
        # Bar 3: Holding (T+1)
        {"timestamp": datetime.combine(base_date + timedelta(days=3), datetime.min.time()), "open": 102.5, "high": 103.0, "low": 102.0, "close": 102.8, "volume": 100_000},
        # Bar 4: (T+2) Wild bar with massive range: Low drops to 80 (hits 5% SL at 96.9) and High surges to 120 (hits 5% TP at 107.1)
        {"timestamp": datetime.combine(base_date + timedelta(days=4), datetime.min.time()), "open": 102.8, "high": 120.0, "low": 80.0, "close": 90.0, "volume": 100_000},
        # Bar 5: End
        {"timestamp": datetime.combine(base_date + timedelta(days=5), datetime.min.time()), "open": 90.0, "high": 91.0, "low": 89.0, "close": 90.0, "volume": 100_000},
    ]
    df = pd.DataFrame(candles)

    fast_ma = np.array([10.0, 30.0, 30.0, 30.0, 30.0, 30.0])
    slow_ma = np.array([20.0] * 6)
    indicator_values = {"fast_ma": fast_ma, "slow_ma": slow_ma}

    strategy = make_sample_strategy(stop_loss=5.0, take_profit=5.0)
    kernel = BacktestExecutionKernel()
    result = kernel.run(
        df=df,
        indicator_values=indicator_values,
        strategy=strategy,
        symbol="VIC",
        initial_cash=50_000_000,
    )

    # Trade must be closed with STOP_LOSS, not TAKE_PROFIT
    closed_trades = [t for t in result.ledger.trades if t.status == "closed"]
    assert len(closed_trades) == 1
    assert closed_trades[0].exit_reason == "STOP_LOSS"
    assert closed_trades[0].result == "loss"
