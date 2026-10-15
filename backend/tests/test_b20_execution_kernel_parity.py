"""Test Suite for Milestone 1 (Batch B20): Canonical Open[T+1] Execution Kernel Parity.

Covers:
- G-01: Basic Long win with fees and tax (exact penny-level hand-calc: net_pnl == 90.7225)
- G-03: Short symmetry arithmetic verification and zero-lookahead next-open separation
- G-04: Next-open SL/TP (SL breach on bar T+1 fills at Open[T+2], backward compatibility for intraday touch, cutoff at N-1)
- G-05: Cutoff boundary at bar N-1 -> UNFILLED_AT_CUTOFF (0 cash delta, trade counts unchanged)
- G-11: Causal position sizing on fill_estimate with board lot 100 and budget constraint (gap-up recalculation and rejection)
- G-12: Zero divergence across Single Backtest, BatchRunner, and Parameter Sweep (Sweep == Batch == Single)

All tests operate in-memory or on isolated temporary SQLite databases (tmp_path).
Production backend/sumi.db is never accessed or mutated.
"""

from __future__ import annotations

import math
from datetime import date, datetime, timedelta
from types import SimpleNamespace
from typing import Generator
import numpy as np
import pandas as pd
import pytest
from sqlalchemy import create_engine
from sqlalchemy.orm import Session, sessionmaker

from app.db import Base
from app.domain.backtest.batch_runner import BatchBacktestRunner, PhaseDefinition
from app.domain.backtest.execution import BacktestExecutionKernel
from app.domain.market.provider import MarketRuleProvider
from app.domain.strategy.strategy_loader import load_strategy_from_dict
from app.models.candle import Candle
from app.services.backtest_service import BacktestService
from app.services.strategy_lab_service import StrategyLabService


@pytest.fixture
def isolated_db(tmp_path) -> Generator[Session, None, None]:
    """Provides an isolated SQLite database session using tmp_path."""
    db_file = tmp_path / "test_b20_parity.db"
    engine = create_engine(
        f"sqlite:///{db_file}",
        connect_args={"check_same_thread": False},
    )
    Base.metadata.create_all(bind=engine)
    TestingSessionLocal = sessionmaker(autocommit=False, autoflush=False, bind=engine)
    session = TestingSessionLocal()
    try:
        yield session
    finally:
        session.close()
        engine.dispose()


def make_strategy_ns(
    name="test_strat",
    stop_loss=None,
    take_profit=None,
    sizing_method="fixed_quantity",
    sizing_qty=100,
    sizing_pct=100.0,
):
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
        risk_management=SimpleNamespace(
            stop_loss_percent=stop_loss,
            take_profit_percent=take_profit,
        ) if (stop_loss or take_profit) else None,
    )


# ============================================================================
# G-01: Basic Long Win Hand-Calculation Parity
# ============================================================================

def test_g01_execution_parity_hand_calc():
    """Verify Golden Case G-01 exact hand-calculation with fees and tax.

    Specs:
    - 15 candles.
    - Signal at Bar 5 close (10.0) -> Fill at Bar 6 Open (10.1).
    - Exit signal at Bar 10 close (11.0) -> Fill at Bar 11 Open (11.05).
    - Qty = 100 shares.
    - Buy gross = 10.1 * 100 = 1010.0
    - Buy fee (0.15%) = 1.515
    - Sell gross = 11.05 * 100 = 1105.0
    - Sell fee (0.15%) = 1.6575
    - Sell tax (0.10%) = 1.105
    - Gross PnL = 95.0
    - Net PnL = 95.0 - (1.515 + 1.6575 + 1.105) = 90.7225
    - Winner (net_pnl > 0), holding bars = 11 - 6 = 5.
    """
    base_date = date(2024, 1, 1)
    n = 15
    candles = []
    for i in range(n):
        candles.append({
            "timestamp": datetime.combine(base_date + timedelta(days=i), datetime.min.time()),
            "open": 10.0,
            "high": 12.0,
            "low": 9.0,
            "close": 10.0,
            "volume": 500_000,
        })
    df = pd.DataFrame(candles)
    # Set exact prices for G-01
    df.loc[5, "close"] = 10.0
    df.loc[6, "open"] = 10.1
    df.loc[10, "close"] = 11.0
    df.loc[11, "open"] = 11.05

    # Strategy cross: fast > slow on bar 5, fast < slow on bar 10
    fast_ma = np.array([10.0] * 5 + [25.0] * 5 + [5.0] * 5)
    slow_ma = np.array([15.0] * n)
    indicator_values = {"fast_ma": fast_ma, "slow_ma": slow_ma}

    strategy = make_strategy_ns(sizing_method="fixed_quantity", sizing_qty=100)
    kernel = BacktestExecutionKernel(
        fee_rate=0.0015,
        tax_rate=0.0010,
        slippage_rate=0.0,
        min_holding_bars=2,
    )
    result = kernel.run(
        df=df,
        indicator_values=indicator_values,
        strategy=strategy,
        symbol="TCB",
        initial_cash=100_000.0,
    )

    closed_trades = [t for t in result.ledger.trades if t.status == "closed"]
    assert len(closed_trades) == 1
    trade = closed_trades[0]

    assert trade.entry_bar_index == 6
    assert trade.entry_price == 10.1
    assert trade.exit_bar_index == 11
    assert trade.exit_price == 11.05
    assert trade.quantity == 100.0
    assert trade.gross_pnl == pytest.approx(95.0, rel=1e-6)
    assert trade.net_pnl == pytest.approx(90.7225, rel=1e-6)
    assert trade.result == "win"
    assert trade.holding_bars == 5


# ============================================================================
# G-03: SHORT Symmetry & Zero-Lookahead Next-Open Separation
# ============================================================================

def test_g03_short_symmetry_arithmetic():
    """Verify Golden Case G-03 hand-calculation arithmetic for short symmetry.

    Short entry fill = 14.95, cover fill = 13.05, qty = 100.
    Gross = (14.95 - 13.05) * 100 = 190.0.
    Fee entry = 14.95 * 100 * 0.0015 = 2.2425.
    Fee exit = 13.05 * 100 * 0.0015 = 1.9575.
    Tax = 14.95 * 100 * 0.0010 = 1.495.
    Total costs = 5.695.
    Net PnL = 190.0 - 5.695 = 184.305 > 0 (winner).
    """
    entry_price = 14.95
    cover_price = 13.05
    qty = 100.0
    fee_rate = 0.0015
    tax_rate = 0.0010

    gross_pnl = (entry_price - cover_price) * qty
    fee_entry = round(entry_price * qty * fee_rate, 4)
    fee_exit = round(cover_price * qty * fee_rate, 4)
    tax_entry = round(entry_price * qty * tax_rate, 4)
    total_costs = fee_entry + fee_exit + tax_entry
    net_pnl = gross_pnl - total_costs

    assert gross_pnl == pytest.approx(190.0, rel=1e-6)
    assert fee_entry == pytest.approx(2.2425, rel=1e-6)
    assert fee_exit == pytest.approx(1.9575, rel=1e-6)
    assert tax_entry == pytest.approx(1.4950, rel=1e-6)
    assert total_costs == pytest.approx(5.695, rel=1e-6)
    assert net_pnl == pytest.approx(184.305, rel=1e-6)
    assert net_pnl > 0


def test_g03_zero_lookahead_next_open_separation():
    """Verify strict temporal separation between Bar T Close evaluation and Bar T+1 Open fill.

    Signal on Bar 4 Close (20.0). Overnight gap: Bar 5 Open = 23.0 (+15%).
    Order must queue at Bar 4 Close and fill at Bar 5 Open = 23.0, proving zero lookahead.
    """
    base_date = date(2024, 1, 1)
    n = 10
    candles = []
    for i in range(n):
        candles.append({
            "timestamp": datetime.combine(base_date + timedelta(days=i), datetime.min.time()),
            "open": 20.0,
            "high": 21.0,
            "low": 19.0,
            "close": 20.0,
            "volume": 500_000,
        })
    df = pd.DataFrame(candles)
    df.loc[4, "close"] = 20.0
    df.loc[5, "open"] = 23.0  # +15% overnight gap

    fast_ma = np.array([10.0] * 4 + [30.0] * 6)
    slow_ma = np.array([20.0] * n)
    indicator_values = {"fast_ma": fast_ma, "slow_ma": slow_ma}

    strategy = make_strategy_ns(sizing_method="fixed_quantity", sizing_qty=100)
    kernel = BacktestExecutionKernel()
    result = kernel.run(
        df=df,
        indicator_values=indicator_values,
        strategy=strategy,
        symbol="GAP_SYM",
        initial_cash=100_000.0,
    )

    assert len(result.ledger.executions) >= 1
    exec_0 = result.ledger.executions[0]
    assert exec_0.signal_bar_index == 4
    assert exec_0.order_bar_index == 4
    assert exec_0.fill_bar_index == 5
    assert exec_0.price == 23.0
    assert exec_0.price != df.loc[4, "close"]


# ============================================================================
# G-04: Next-Open SL/TP Breach & Intraday Touch Backward Compatibility
# ============================================================================

def test_g04_sl_next_open_execution():
    """Verify Golden Case G-04 under default sl_tp_execution_mode='next_open'.

    Position opened at Open[T+1] = 10100.0. SL configured at 9500.0.
    Bar T+2: Low = 9300.0 breaches SL.
    Bar T+3: Open = 9200.0.
    Exit fills at Open[T+3] = 9200.0, NOT 9500.0!
    """
    base_date = date(2024, 1, 1)
    candles = [
        {"timestamp": datetime.combine(base_date, datetime.min.time()), "open": 10000.0, "high": 10500.0, "low": 9500.0, "close": 10000.0, "volume": 100_000},
        {"timestamp": datetime.combine(base_date + timedelta(days=1), datetime.min.time()), "open": 10000.0, "high": 10500.0, "low": 9500.0, "close": 10000.0, "volume": 100_000},
        {"timestamp": datetime.combine(base_date + timedelta(days=2), datetime.min.time()), "open": 10100.0, "high": 10500.0, "low": 9800.0, "close": 10000.0, "volume": 100_000},
        {"timestamp": datetime.combine(base_date + timedelta(days=3), datetime.min.time()), "open": 10000.0, "high": 10500.0, "low": 9300.0, "close": 9400.0, "volume": 100_000},  # Low 9300 breaches SL 9500
        {"timestamp": datetime.combine(base_date + timedelta(days=4), datetime.min.time()), "open": 9200.0, "high": 9500.0, "low": 9000.0, "close": 9300.0, "volume": 100_000},   # Fills here at 9200
        {"timestamp": datetime.combine(base_date + timedelta(days=5), datetime.min.time()), "open": 9300.0, "high": 9500.0, "low": 9000.0, "close": 9300.0, "volume": 100_000},
    ]
    df = pd.DataFrame(candles)

    # Signal on bar 1 close -> fills bar 2 open (10100.0)
    fast_ma = np.array([10.0, 30.0, 30.0, 30.0, 30.0, 30.0])
    slow_ma = np.array([20.0] * 6)
    indicator_values = {"fast_ma": fast_ma, "slow_ma": slow_ma}

    strategy = make_strategy_ns(stop_loss=5.9406, sizing_method="fixed_quantity", sizing_qty=100)
    strategy.exit_rules = []  # Rely exclusively on SL/TP for exits
    strategy.risk_management.stop_loss_price = 9500.0

    kernel = BacktestExecutionKernel(min_holding_bars=0, sl_tp_execution_mode="next_open")
    result = kernel.run(
        df=df,
        indicator_values=indicator_values,
        strategy=strategy,
        symbol="SL_TEST",
        initial_cash=10_000_000.0,
    )

    closed = [t for t in result.ledger.trades if t.status == "closed"]
    assert len(closed) == 1
    trade = closed[0]
    assert trade.entry_bar_index == 2
    assert trade.entry_price == 10100.0
    assert trade.exit_bar_index == 4  # Filled at bar 4 Open (T+2 after entry at bar 2)
    assert trade.exit_price == 9200.0     # Filled at Open[4], NOT 9500.0!
    assert trade.exit_reason == "STOP_LOSS"


def test_g04_intraday_touch_backward_compatibility():
    """Verify that explicitly setting sl_tp_execution_mode='intraday_touch' fills at SL price level on bar T+1."""
    base_date = date(2024, 1, 1)
    candles = [
        {"timestamp": datetime.combine(base_date, datetime.min.time()), "open": 10000.0, "high": 10500.0, "low": 9500.0, "close": 10000.0, "volume": 100_000},
        {"timestamp": datetime.combine(base_date + timedelta(days=1), datetime.min.time()), "open": 10000.0, "high": 10500.0, "low": 9500.0, "close": 10000.0, "volume": 100_000},
        {"timestamp": datetime.combine(base_date + timedelta(days=2), datetime.min.time()), "open": 10100.0, "high": 10500.0, "low": 9800.0, "close": 10000.0, "volume": 100_000},
        {"timestamp": datetime.combine(base_date + timedelta(days=3), datetime.min.time()), "open": 10000.0, "high": 10500.0, "low": 9300.0, "close": 9400.0, "volume": 100_000},
        {"timestamp": datetime.combine(base_date + timedelta(days=4), datetime.min.time()), "open": 9200.0, "high": 9500.0, "low": 9000.0, "close": 9300.0, "volume": 100_000},
    ]
    df = pd.DataFrame(candles)

    # Signal only on bar 1 close -> fills bar 2 open (10100.0)
    fast_ma = np.array([10.0, 30.0, 10.0, 10.0, 10.0])
    slow_ma = np.array([20.0] * 5)
    indicator_values = {"fast_ma": fast_ma, "slow_ma": slow_ma}

    strategy = make_strategy_ns(stop_loss=5.9406, sizing_method="fixed_quantity", sizing_qty=100)
    strategy.exit_rules = []  # Rely exclusively on SL/TP for exits
    strategy.risk_management.stop_loss_price = 9500.0

    kernel = BacktestExecutionKernel(min_holding_bars=0, sl_tp_execution_mode="intraday_touch")
    result = kernel.run(
        df=df,
        indicator_values=indicator_values,
        strategy=strategy,
        symbol="SL_TOUCH",
        initial_cash=10_000_000.0,
    )

    closed = [t for t in result.ledger.trades if t.status == "closed"]
    assert len(closed) == 1
    trade = closed[0]
    assert trade.exit_bar_index == 3   # Same bar as breach
    assert trade.exit_price == 9500.0     # Filled at SL trigger price


def test_g04_next_open_sl_tp_final_bar_cutoff():
    """Verify that if an SL breach occurs on the final bar N-1 under next_open, the exit order remains UNFILLED_AT_CUTOFF."""
    base_date = date(2024, 1, 1)
    candles = [
        {"timestamp": datetime.combine(base_date, datetime.min.time()), "open": 10000.0, "high": 10500.0, "low": 9500.0, "close": 10000.0, "volume": 100_000},
        {"timestamp": datetime.combine(base_date + timedelta(days=1), datetime.min.time()), "open": 10000.0, "high": 10500.0, "low": 9500.0, "close": 10000.0, "volume": 100_000},
        {"timestamp": datetime.combine(base_date + timedelta(days=2), datetime.min.time()), "open": 10100.0, "high": 10500.0, "low": 9800.0, "close": 10000.0, "volume": 100_000},
        {"timestamp": datetime.combine(base_date + timedelta(days=3), datetime.min.time()), "open": 10000.0, "high": 10500.0, "low": 9300.0, "close": 9400.0, "volume": 100_000},  # SL breach on final bar N-1
    ]
    df = pd.DataFrame(candles)

    fast_ma = np.array([10.0, 30.0, 30.0, 30.0])
    slow_ma = np.array([20.0] * 4)
    indicator_values = {"fast_ma": fast_ma, "slow_ma": slow_ma}

    strategy = make_strategy_ns(stop_loss=5.9406, sizing_method="fixed_quantity", sizing_qty=100)
    strategy.risk_management.stop_loss_price = 9500.0

    kernel = BacktestExecutionKernel(min_holding_bars=0, sl_tp_execution_mode="next_open")
    result = kernel.run(
        df=df,
        indicator_values=indicator_values,
        strategy=strategy,
        symbol="SL_CUTOFF",
        initial_cash=10_000_000.0,
    )

    closed = [t for t in result.ledger.trades if t.status == "closed"]
    assert len(closed) == 0  # Cannot fill after final bar
    open_trades = [t for t in result.ledger.trades if t.status == "open"]
    assert len(open_trades) == 1
    cutoff_orders = [o for o in result.ledger.orders if o.status == "UNFILLED_AT_CUTOFF"]
    assert len(cutoff_orders) == 1
    assert cutoff_orders[0].side == "SELL"


def test_g04_sl_pct_schema_with_load_strategy_from_dict():
    """Verify that a strategy loaded via load_strategy_from_dict with risk_management={'stop_loss_pct': 10.0}
    correctly activates the Stop Loss in BacktestExecutionKernel and exits at Open[T+2].
    """
    base_date = date(2024, 1, 1)
    candles = [
        {"timestamp": datetime.combine(base_date, datetime.min.time()), "open": 100.0, "high": 102.0, "low": 98.0, "close": 100.0, "volume": 100_000},
        {"timestamp": datetime.combine(base_date + timedelta(days=1), datetime.min.time()), "open": 100.0, "high": 102.0, "low": 98.0, "close": 100.0, "volume": 100_000},  # Signal on bar 1 close
        {"timestamp": datetime.combine(base_date + timedelta(days=2), datetime.min.time()), "open": 98.0, "high": 99.0, "low": 88.0, "close": 89.0, "volume": 100_000},   # Entry at Open=98.0; Low=88.0 breaches SL (88.2)
        {"timestamp": datetime.combine(base_date + timedelta(days=3), datetime.min.time()), "open": 65.0, "high": 68.0, "low": 62.0, "close": 66.0, "volume": 100_000},   # Exit fills at Open=65.0 (T+2 after entry at bar 2 open / queued at bar 2 close)
        {"timestamp": datetime.combine(base_date + timedelta(days=4), datetime.min.time()), "open": 66.0, "high": 70.0, "low": 65.0, "close": 68.0, "volume": 100_000},
    ]
    strat = load_strategy_from_dict({
        "name": "test_sl_pct",
        "indicators": [{"name": "sma_fast", "type": "sma", "length": 5}, {"name": "sma_slow", "type": "sma", "length": 10}],
        "entry_rules": [{"condition": "sma_fast > sma_slow"}],
        "exit_rules": [],
        "position_sizing": {"method": "fixed_quantity", "quantity": 100},
        "risk_management": {"stop_loss_pct": 10.0},
    })
    kernel = BacktestExecutionKernel(min_holding_bars=0, sl_tp_execution_mode="next_open")
    res = kernel.run(
        df=pd.DataFrame(candles),
        indicator_values={"sma_fast": np.array([30.0] * 5), "sma_slow": np.array([20.0] * 5)},
        strategy=strat,
        symbol="SL_PCT_TEST",
        initial_cash=1_000_000.0,
    )
    closed = [t for t in res.ledger.trades if t.status == "closed"]
    assert len(closed) == 1
    trade = closed[0]
    assert trade.entry_bar_index == 2
    assert trade.entry_price == 98.0
    assert trade.exit_bar_index == 3
    assert trade.exit_price == 65.0
    assert trade.exit_reason == "STOP_LOSS"


# ============================================================================
# G-05: Cutoff Boundary at Bar N-1
# ============================================================================

def test_g05_cutoff_boundary_final_bar():
    """Verify Golden Case G-05: Signal generated on last bar N-1 produces UNFILLED_AT_CUTOFF with zero side-effects."""
    base_date = date(2024, 1, 1)
    n = 10
    candles = []
    for i in range(n):
        candles.append({
            "timestamp": datetime.combine(base_date + timedelta(days=i), datetime.min.time()),
            "open": 50.0 + i,
            "high": 52.0 + i,
            "low": 48.0 + i,
            "close": 50.5 + i,
            "volume": 100_000,
        })
    df = pd.DataFrame(candles)

    # Cross triggers ONLY on final bar (n - 1 = 9)
    fast_ma = np.array([10.0] * 9 + [30.0])
    slow_ma = np.array([20.0] * 10)
    indicator_values = {"fast_ma": fast_ma, "slow_ma": slow_ma}

    strategy = make_strategy_ns(sizing_method="fixed_quantity", sizing_qty=100)
    kernel = BacktestExecutionKernel()
    result = kernel.run(
        df=df,
        indicator_values=indicator_values,
        strategy=strategy,
        symbol="CUTOFF_SYM",
        initial_cash=100_000_000.0,
    )

    assert len(result.ledger.executions) == 0
    assert len(result.ledger.trades) == 0
    assert result.ledger.current_cash == 100_000_000.0

    cutoff_orders = [o for o in result.ledger.orders if o.status == "UNFILLED_AT_CUTOFF"]
    assert len(cutoff_orders) == 1
    assert cutoff_orders[0].signal_bar_index == 9
    assert cutoff_orders[0].order_type == "MARKET_NEXT_OPEN"


# ============================================================================
# G-11: Causal Position Sizing on Fill Estimate
# ============================================================================

def test_g11_causal_position_sizing_fill_estimate():
    """Verify Golden Case G-11: Causal position sizing on fill price Open[T+1] with board lot 100."""
    base_date = date(2024, 1, 1)
    candles = [
        {"timestamp": datetime.combine(base_date, datetime.min.time()), "open": 10000.0, "high": 10500.0, "low": 9500.0, "close": 10000.0, "volume": 100_000},
        {"timestamp": datetime.combine(base_date + timedelta(days=1), datetime.min.time()), "open": 10000.0, "high": 10500.0, "low": 9500.0, "close": 10000.0, "volume": 100_000},
        {"timestamp": datetime.combine(base_date + timedelta(days=2), datetime.min.time()), "open": 12000.0, "high": 12500.0, "low": 11800.0, "close": 12200.0, "volume": 100_000},  # Gaps up to 12000
        {"timestamp": datetime.combine(base_date + timedelta(days=3), datetime.min.time()), "open": 12200.0, "high": 12500.0, "low": 11800.0, "close": 12200.0, "volume": 100_000},
    ]
    df = pd.DataFrame(candles)

    # Signal on bar 1 close (close = 10,000)
    fast_ma = np.array([10.0, 30.0, 30.0, 30.0])
    slow_ma = np.array([20.0] * 4)
    indicator_values = {"fast_ma": fast_ma, "slow_ma": slow_ma}

    initial_cash = 100_000_000.0
    strategy = make_strategy_ns(sizing_method="percent_equity", sizing_pct=100.0)

    kernel = BacktestExecutionKernel(sizing_basis="fill_estimate")
    result = kernel.run(
        df=df,
        indicator_values=indicator_values,
        strategy=strategy,
        symbol="SIZING_SYM",
        initial_cash=initial_cash,
    )

    assert len(result.ledger.executions) >= 1
    exec_0 = result.ledger.executions[0]
    assert exec_0.price == 12000.0

    # Max affordable at 12000: cost_per_share = 12000 * 1.0015 = 12018
    # 100_000_000 / 12018 = 8320.85 -> 8300 shares (board lot 100)
    assert exec_0.quantity == 8300.0
    assert exec_0.quantity % 100 == 0

    # Strict solvency check
    assert exec_0.net_amount <= initial_cash
    assert result.ledger.current_cash >= 0.0

    # Provisional quantity at bar 1 close (close 10000): 100_000_000 / (10000 * 1.0015) = 9985 -> 9900 shares
    order_0 = result.ledger.orders[0]
    assert order_0.sizing_basis == "fill_estimate"
    assert order_0.provisional_quantity == 9900.0
    assert order_0.quantity == 8300.0  # Recalculated to actual executable qty at fill


def test_g11_sizing_budget_rejection_on_gap_up():
    """Verify that if an overnight gap up makes even 1 board lot unaffordable, the order is cleanly rejected."""
    base_date = date(2024, 1, 1)
    candles = [
        {"timestamp": datetime.combine(base_date, datetime.min.time()), "open": 9500.0, "high": 9600.0, "low": 9400.0, "close": 9500.0, "volume": 100_000},
        {"timestamp": datetime.combine(base_date + timedelta(days=1), datetime.min.time()), "open": 9500.0, "high": 9600.0, "low": 9400.0, "close": 9500.0, "volume": 100_000},
        {"timestamp": datetime.combine(base_date + timedelta(days=2), datetime.min.time()), "open": 11000.0, "high": 11500.0, "low": 10800.0, "close": 11200.0, "volume": 100_000},
    ]
    df = pd.DataFrame(candles)

    fast_ma = np.array([10.0, 30.0, 30.0])
    slow_ma = np.array([20.0] * 3)
    indicator_values = {"fast_ma": fast_ma, "slow_ma": slow_ma}

    # Cash = 1,000,000.
    # At Bar 1 close (9500): 100 shares = 951,425 <= 1,000,000. Order queued!
    # At Bar 2 open (11000): 100 shares = 1,101,650 > 1,000,000. Order must reject!
    strategy = make_strategy_ns(sizing_method="fixed_quantity", sizing_qty=100)
    kernel = BacktestExecutionKernel(sizing_basis="fill_estimate")
    result = kernel.run(
        df=df,
        indicator_values=indicator_values,
        strategy=strategy,
        symbol="GAP_REJECT",
        initial_cash=1_000_000.0,
    )

    assert len(result.ledger.executions) == 0
    rejected_orders = [o for o in result.ledger.orders if o.status == "REJECTED"]
    assert len(rejected_orders) == 1
    assert "insufficient cash" in rejected_orders[0].rejection_reason.lower()
    assert result.ledger.current_cash == 1_000_000.0


# ============================================================================
# G-12: Complete Parity: Single == Batch == Sweep
# ============================================================================

@pytest.mark.asyncio
async def test_g12_complete_parity_single_batch_sweep(isolated_db):
    """Verify Golden Case G-12: Zero divergence between Single Backtest, BatchRunner, and Parameter Sweep.

    Given identical strategy rules, candles, fees, and dates:
    - Single Backtest (BacktestService._run_single_symbol_backtest)
    - Batch Runner (BatchBacktestRunner.run_batch)
    - Parameter Sweep (StrategyLabService.run_parameter_sweep)
    All produce identical trade counts, entry/exit prices, net PnL, win rate, and final equity.
    """
    symbol = "PARITY_SYM"
    base_date = date(2024, 1, 1)
    num_days = 60

    # Seed deterministic candles into isolated SQLite database
    for i in range(num_days):
        day = base_date + timedelta(days=i)
        # Periodic price curve creating clean crossovers
        price = 50.0 + math.sin(i / 5.0) * 10.0 + (i * 0.2)
        price = round(price, 2)
        isolated_db.add(Candle(
            symbol=symbol,
            timeframe="1D",
            timestamp=day,
            open=price,
            high=price + 1.0,
            low=price - 1.0,
            close=price,
            volume=500_000,
        ))
    isolated_db.commit()

    start_date_str = str(base_date)
    end_date_str = str(base_date + timedelta(days=num_days))
    initial_cash = 100_000_000.0

    strategy_dict = {
        "name": "Parity Test Strategy",
        "indicators": [
            {"name": "sma_fast", "type": "sma", "length": 5},
            {"name": "sma_slow", "type": "sma", "length": 15},
        ],
        "entry_rules": [{"condition": "sma_fast > sma_slow"}],
        "exit_rules": [{"condition": "sma_fast < sma_slow"}],
        "position_sizing": {"method": "fixed_quantity", "quantity": 1000},
    }

    # 1. Run Single Backtest
    single_config = {
        "symbol": symbol,
        "start_date": start_date_str,
        "end_date": end_date_str,
        "initial_cash": initial_cash,
        "strategy": strategy_dict,
    }
    single_res = await BacktestService().run_backtest(isolated_db, single_config)
    assert single_res["status"] == "succeeded"
    single_analytics = single_res["analytics"]
    single_trades_count = single_analytics["total_trades"]
    single_net_pnl = single_analytics["total_net_pnl"]
    single_win_rate = single_analytics["win_rate"]

    # 2. Run Batch Backtest
    strategy_obj = load_strategy_from_dict(strategy_dict)
    phase = PhaseDefinition(name="ParityPhase", start_date=start_date_str, end_date=end_date_str)

    def candle_provider(sym: str, start: str, end: str) -> pd.DataFrame:
        candles = (
            isolated_db.query(Candle)
            .filter(Candle.symbol == sym)
            .order_by(Candle.timestamp.asc())
            .all()
        )
        return pd.DataFrame([{
            "timestamp": pd.Timestamp(c.timestamp),
            "open": float(c.open),
            "high": float(c.high),
            "low": float(c.low),
            "close": float(c.close),
            "volume": float(c.volume),
        } for c in candles])

    batch_res = BatchBacktestRunner.run_batch(
        strategy=strategy_obj,
        symbols=[symbol],
        phases=[phase],
        candle_provider=candle_provider,
        initial_cash_per_run=initial_cash,
    )
    assert len(batch_res.phase_results) == 1
    batch_run = batch_res.phase_results[0]
    assert batch_run.status == "succeeded"
    batch_trades_count = batch_run.total_trades
    batch_net_pnl = batch_run.net_pnl
    batch_win_rate = batch_run.benchmark_metrics.win_rate_pct / 100.0

    # 3. Run Parameter Sweep
    sweep_config = {
        "symbol": symbol,
        "start_date": start_date_str,
        "end_date": end_date_str,
        "initial_cash": initial_cash,
        "strategy": strategy_dict,
        "sweep": [
            {"path": "indicators[0].length", "values": [5]},
        ],
    }
    sweep_res = await StrategyLabService().run_parameter_sweep(isolated_db, sweep_config)
    assert sweep_res["status"] == "succeeded"
    variant = sweep_res["variants"][0]
    sweep_metrics = variant["metrics"]
    sweep_trades_count = sweep_metrics["total_trades"]
    sweep_net_pnl = sweep_metrics["net_pnl"]
    sweep_win_rate = sweep_metrics["win_rate"]

    # 4. Strict Zero Divergence Parity Assertions
    assert single_trades_count == batch_trades_count == sweep_trades_count
    assert single_trades_count > 0  # Must have actual executed trades to verify parity

    assert math.isclose(single_net_pnl, batch_net_pnl, rel_tol=1e-5, abs_tol=1e-2)
    assert math.isclose(single_net_pnl, sweep_net_pnl, rel_tol=1e-5, abs_tol=1e-2)

    assert math.isclose(single_win_rate, batch_win_rate, rel_tol=1e-5, abs_tol=1e-4)
    assert math.isclose(single_win_rate, sweep_win_rate, rel_tol=1e-5, abs_tol=1e-4)
