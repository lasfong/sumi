"""Adversarial Challenger Stress Suite for Batch B20 (Milestone 1).

Author: Challenger B20-1 (Adversarial Parity & Timing Challenger)
Scope:
1. Timing Verification:
   - Anti-lookahead differential testing: identical history <= T with wildly divergent T+1 bars must yield 100% identical signal and provisional sizing at Bar T close.
   - Reject adversarial rule definitions attempting to peek ahead at future bars (next_open, future variables).
   - Invariance of Bar T+1 open fill price against extreme High/Low[T+1] swings.
   - Strict UNFILLED_AT_CUTOFF behavior on final bar N-1.
2. Gap-Down SL Test:
   - Severe overnight gap-down after SL breach: Low[T+1] <= SL triggers exit queued at T+1, filling at severely lower Open[T+2].
   - Full slippage accounting: gap-down open fill vs artificial fill at SL level.
   - Comparison of next_open vs intraday_touch to quantify the elimination of optimistic fill bias.
3. Multi-Variant Sweep vs Single Parity:
   - Grid sweep across 9 variants (3 fast SMAs x 3 slow SMAs) comparing StrategyLabService.run_parameter_sweep vs BacktestService.run_backtest.
   - 100% exact numerical match across total_trades, net_pnl, win_rate, and profit_factor for every variant.
   - Sweep cancellation manager state verification.
4. Defect Characterization:
   - Confirmed contract mismatch: StrategyConfig.risk_management uses stop_loss_pct / take_profit_pct, but BacktestExecutionKernel only inspects stop_loss_percent / stop_loss, causing silent failure of SL/TP on all strategies loaded from dict/schema.
5. Database Safety:
   - All tests use isolated in-memory or tmp_path SQLite fixtures. Production backend/sumi.db is never touched.
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
from app.domain.accounting import BUY_FEE_RATE, SELL_TAX_RATE
from app.domain.backtest.execution import BacktestExecutionKernel
from app.domain.strategy.rule_evaluator import RuleEvaluationError
from app.domain.strategy.strategy_loader import load_strategy_from_dict
from app.domain.strategy.strategy_rule_evaluator import StrategyRuleEvaluator
from app.models.candle import Candle
from app.services.backtest_service import BacktestService
from app.services.strategy_lab_service import StrategyLabService, SweepCancellationManager


@pytest.fixture
def isolated_challenger_db(tmp_path) -> Generator[Session, None, None]:
    """Provides an isolated SQLite database session using tmp_path."""
    db_file = tmp_path / "test_b20_challenger.db"
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


def _make_strategy(
    name: str = "adversarial_strategy",
    fast_len: int = 5,
    slow_len: int = 15,
    stop_loss: float | None = None,
    take_profit: float | None = None,
    sizing_method: str = "fixed_quantity",
    sizing_qty: int = 100,
    sizing_pct: float = 100.0,
) -> dict:
    strat = {
        "name": name,
        "indicators": [
            {"name": "sma_fast", "type": "sma", "length": fast_len},
            {"name": "sma_slow", "type": "sma", "length": slow_len},
        ],
        "entry_rules": [{"condition": "sma_fast > sma_slow"}],
        "exit_rules": [{"condition": "sma_fast < sma_slow"}],
        "position_sizing": {
            "method": sizing_method,
            "quantity": sizing_qty,
            "percent": sizing_pct,
        },
    }
    if stop_loss is not None or take_profit is not None:
        strat["risk_management"] = {
            "stop_loss_pct": stop_loss,
            "take_profit_pct": take_profit,
        }
    return strat


def _make_strategy_ns(
    name: str = "ns_strategy",
    stop_loss_percent: float | None = None,
    stop_loss_price: float | None = None,
    take_profit_percent: float | None = None,
    take_profit_price: float | None = None,
    sizing_method: str = "fixed_quantity",
    sizing_qty: int = 100,
    sizing_pct: float = 100.0,
) -> SimpleNamespace:
    """Helper creating a strategy object compatible with BacktestExecutionKernel's getattr expectations."""
    rm = None
    if any(x is not None for x in (stop_loss_percent, stop_loss_price, take_profit_percent, take_profit_price)):
        rm = SimpleNamespace(
            stop_loss_percent=stop_loss_percent,
            stop_loss_price=stop_loss_price,
            take_profit_percent=take_profit_percent,
            take_profit_price=take_profit_price,
        )
    return SimpleNamespace(
        name=name,
        indicators=[
            {"id": "sma_fast", "kind": "sma", "params": {"length": 5}},
            {"id": "sma_slow", "kind": "sma", "params": {"length": 10}},
        ],
        entry_rules=[{"dsl": {"gt": ["sma_fast", "sma_slow"]}}],
        exit_rules=[{"dsl": {"lt": ["sma_fast", "sma_slow"]}}],
        position_sizing=SimpleNamespace(
            method=sizing_method,
            quantity=sizing_qty,
            percent=sizing_pct,
            model_dump=lambda: {"method": sizing_method, "quantity": sizing_qty, "percent": sizing_pct},
        ),
        risk_management=rm,
    )


# ==============================================================================
# SECTION 1: Timing Verification & Anti-Lookahead
# ==============================================================================

def test_adv_timing_anti_lookahead_future_bar_differential():
    """Adversarial Anti-Lookahead Test:

    Verify that two market datasets with IDENTICAL history through Bar T (close)
    produce 100% IDENTICAL signals, orders, and provisional quantities at Bar T close,
    even when Bar T+1 diverges dramatically (e.g. normal open vs +200% gap-up with extreme volatility).
    """
    base_date = date(2024, 1, 1)
    n = 10
    t = 5

    # Base candle sequence
    candles_common = []
    for i in range(t + 1):
        candles_common.append({
            "timestamp": datetime.combine(base_date + timedelta(days=i), datetime.min.time()),
            "open": 50.0 + i,
            "high": 52.0 + i,
            "low": 48.0 + i,
            "close": 50.0 + i,
            "volume": 100_000,
        })

    # Dataset A: Normal progression for bars > t
    candles_a = list(candles_common)
    for i in range(t + 1, n):
        candles_a.append({
            "timestamp": datetime.combine(base_date + timedelta(days=i), datetime.min.time()),
            "open": 52.0,
            "high": 55.0,
            "low": 50.0,
            "close": 53.0,
            "volume": 100_000,
        })
    df_a = pd.DataFrame(candles_a)

    # Dataset B: Wildly divergent future bar T+1 (Open = 150.0, High = 500.0, Low = 10.0, Close = 300.0)
    candles_b = list(candles_common)
    candles_b.append({
        "timestamp": datetime.combine(base_date + timedelta(days=t + 1), datetime.min.time()),
        "open": 150.0,
        "high": 500.0,
        "low": 10.0,
        "close": 300.0,
        "volume": 999_000,
    })
    for i in range(t + 2, n):
        candles_b.append({
            "timestamp": datetime.combine(base_date + timedelta(days=i), datetime.min.time()),
            "open": 300.0,
            "high": 310.0,
            "low": 290.0,
            "close": 305.0,
            "volume": 100_000,
        })
    df_b = pd.DataFrame(candles_b)

    # Indicator values engineered to produce cross ONLY on bar 5
    fast_ma_a = np.array([20.0] * 5 + [80.0] + [80.0] * 4)
    slow_ma_a = np.array([40.0] * n)

    fast_ma_b = np.array([20.0] * 5 + [80.0] + [80.0] * 4)
    slow_ma_b = np.array([40.0] * n)

    strat_dict = _make_strategy(sizing_method="percent_equity", sizing_pct=100.0)
    strat_obj = load_strategy_from_dict(strat_dict)

    kernel_a = BacktestExecutionKernel(sizing_basis="fill_estimate")
    res_a = kernel_a.run(
        df=df_a,
        indicator_values={"sma_fast": fast_ma_a, "sma_slow": slow_ma_a},
        strategy=strat_obj,
        symbol="SYM_A",
        initial_cash=100_000_000.0,
    )

    kernel_b = BacktestExecutionKernel(sizing_basis="fill_estimate")
    res_b = kernel_b.run(
        df=df_b,
        indicator_values={"sma_fast": fast_ma_b, "sma_slow": slow_ma_b},
        strategy=strat_obj,
        symbol="SYM_B",
        initial_cash=100_000_000.0,
    )

    # Inspect the orders created at Bar T (bar_index = 5) close
    order_a = [o for o in res_a.ledger.orders if o.signal_bar_index == t][0]
    order_b = [o for o in res_b.ledger.orders if o.signal_bar_index == t][0]

    # Verify zero lookahead at signal generation time
    assert order_a.signal_bar_index == order_b.signal_bar_index == t
    assert order_a.order_bar_index == order_b.order_bar_index == t
    assert order_a.requested_price == order_b.requested_price == df_a.loc[t, "close"]
    assert order_a.provisional_quantity == order_b.provisional_quantity
    assert order_a.status == "FILLED"
    assert order_b.status == "FILLED"

    # Now verify that at fill time (Bar T+1), execution is causal:
    # res_a filled at df_a Open[T+1] (52.0)
    # res_b filled at df_b Open[T+1] (150.0)
    exec_a = res_a.ledger.executions[0]
    exec_b = res_b.ledger.executions[0]

    assert exec_a.fill_bar_index == t + 1
    assert exec_b.fill_bar_index == t + 1
    assert exec_a.price == 52.0
    assert exec_b.price == 150.0

    assert exec_a.quantity == 1_920_100.0
    assert exec_b.quantity == 665_600.0
    assert exec_a.quantity != exec_b.quantity


def test_adv_timing_adversarial_peeking_rule_dsl_rejected():
    """Adversarial Strategy Validation Test:

    Verify that strategies attempting to declare lookahead DSL rules or peeking variables
    are caught and rejected by StrategyRuleEvaluator.validate_strategy_rules.
    """
    allowed_indicators = {"sma_fast", "sma_slow", "open", "high", "low", "close", "volume"}

    # Attempt 1: Referencing next_open in condition string
    strat_peek_cond = {
        "name": "peeking_condition",
        "indicators": [{"name": "sma_fast", "type": "sma"}, {"name": "sma_slow", "type": "sma"}],
        "entry_rules": [{"condition": "next_open > sma_slow"}],
        "exit_rules": [],
        "position_sizing": {"method": "fixed_quantity", "quantity": 100},
    }
    strat_obj_1 = load_strategy_from_dict(strat_peek_cond)
    with pytest.raises(RuleEvaluationError, match=r"Unknown identifier\(s\): next_open"):
        StrategyRuleEvaluator.validate_strategy_rules(strat_obj_1, allowed_indicators)

    # Attempt 2: Referencing future variable in inline DSL
    strat_peek_dsl = {
        "name": "peeking_dsl",
        "indicators": [{"name": "sma_fast", "type": "sma"}, {"name": "sma_slow", "type": "sma"}],
        "entry_rules": [{"gt": ["future_close", "sma_slow"]}],
        "exit_rules": [],
        "position_sizing": {"method": "fixed_quantity", "quantity": 100},
    }
    strat_obj_2 = load_strategy_from_dict(strat_peek_dsl)
    with pytest.raises(RuleEvaluationError, match=r"Unknown identifier\(s\): future_close"):
        StrategyRuleEvaluator.validate_strategy_rules(strat_obj_2, allowed_indicators)


def test_adv_timing_high_low_t1_does_not_affect_open_t1_fill_price():
    """Verify that extreme High/Low swings on Bar T+1 do NOT corrupt the Open[T+1] fill price."""
    base_date = date(2024, 1, 1)
    candles = [
        {"timestamp": datetime.combine(base_date, datetime.min.time()), "open": 50.0, "high": 52.0, "low": 48.0, "close": 50.0, "volume": 100_000},
        {"timestamp": datetime.combine(base_date + timedelta(days=1), datetime.min.time()), "open": 50.0, "high": 52.0, "low": 48.0, "close": 50.0, "volume": 100_000},
        # Bar 2 is T+1: Open is 55.0, but High is 1000.0 and Low is 1.0!
        {"timestamp": datetime.combine(base_date + timedelta(days=2), datetime.min.time()), "open": 55.0, "high": 1000.0, "low": 1.0, "close": 58.0, "volume": 100_000},
        {"timestamp": datetime.combine(base_date + timedelta(days=3), datetime.min.time()), "open": 58.0, "high": 60.0, "low": 56.0, "close": 58.0, "volume": 100_000},
    ]
    df = pd.DataFrame(candles)

    fast_ma = np.array([10.0, 30.0, 30.0, 30.0])
    slow_ma = np.array([20.0] * 4)

    strat = load_strategy_from_dict(_make_strategy(sizing_method="fixed_quantity", sizing_qty=100))
    kernel = BacktestExecutionKernel(slippage_rate=0.0)
    res = kernel.run(
        df=df,
        indicator_values={"sma_fast": fast_ma, "sma_slow": slow_ma},
        strategy=strat,
        symbol="VOLATILITY_TEST",
        initial_cash=100_000.0,
    )

    assert len(res.ledger.executions) == 1
    exec_0 = res.ledger.executions[0]
    assert exec_0.fill_bar_index == 2
    assert exec_0.price == 55.0
    assert exec_0.price != 1000.0
    assert exec_0.price != 1.0


def test_adv_timing_final_bar_n_minus_1_cutoff_no_execution_leak():
    """Verify that an entry signal triggered on the final bar N-1 is strictly UNFILLED_AT_CUTOFF."""
    base_date = date(2024, 1, 1)
    candles = [
        {"timestamp": datetime.combine(base_date + timedelta(days=i), datetime.min.time()), "open": 50.0, "high": 52.0, "low": 48.0, "close": 50.0, "volume": 100_000}
        for i in range(5)
    ]
    df = pd.DataFrame(candles)

    # Signal fires ONLY on final bar 4
    fast_ma = np.array([10.0, 10.0, 10.0, 10.0, 30.0])
    slow_ma = np.array([20.0] * 5)

    strat = load_strategy_from_dict(_make_strategy(sizing_method="fixed_quantity", sizing_qty=100))
    kernel = BacktestExecutionKernel()
    res = kernel.run(
        df=df,
        indicator_values={"sma_fast": fast_ma, "sma_slow": slow_ma},
        strategy=strat,
        symbol="CUTOFF_TEST",
        initial_cash=100_000.0,
    )

    assert len(res.ledger.executions) == 0
    assert len(res.ledger.trades) == 0
    assert res.ledger.current_cash == 100_000.0
    cutoff_orders = [o for o in res.ledger.orders if o.status == "UNFILLED_AT_CUTOFF"]
    assert len(cutoff_orders) == 1
    assert cutoff_orders[0].signal_bar_index == 4


# ==============================================================================
# SECTION 2: Gap-Down SL Test & Slippage Realism under next_open
# ==============================================================================

def test_adv_gap_down_sl_severe_overnight_slippage():
    """Adversarial Gap-Down Stop-Loss Test:

    Scenario:
    - Signal fires on Bar 1 Close.
    - Position entered at Bar 2 Open = 100.0, Qty = 100 shares.
    - Stop-Loss configured at 90.0 (-10.0%).
    - Bar 3: High = 99.0, Low = 88.0 (breaches 90.0 SL), Close = 89.0.
    - Bar 4: Catastrophic overnight gap-down! Open = 65.0 (-27.8% below SL price of 90.0!).
    - Verification under sl_tp_execution_mode="next_open":
      * The exit order is queued at Bar 3 Close.
      * The fill occurs at Bar 4 Open = 65.0, NOT 90.0!
      * Gross loss is (65.0 - 100.0) * 100 = -3500.0 (reflecting realistic gap slippage).
      * An artificial engine filling at 90.0 would understate loss by $2500 (71.4% error).
    """
    base_date = date(2024, 1, 1)
    candles = [
        {"timestamp": datetime.combine(base_date, datetime.min.time()), "open": 100.0, "high": 102.0, "low": 98.0, "close": 100.0, "volume": 100_000},
        {"timestamp": datetime.combine(base_date + timedelta(days=1), datetime.min.time()), "open": 100.0, "high": 102.0, "low": 98.0, "close": 100.0, "volume": 100_000},  # Signal on bar 1 close
        {"timestamp": datetime.combine(base_date + timedelta(days=2), datetime.min.time()), "open": 100.0, "high": 102.0, "low": 98.0, "close": 100.0, "volume": 100_000},  # Fill at 100.0
        {"timestamp": datetime.combine(base_date + timedelta(days=3), datetime.min.time()), "open": 98.0, "high": 99.0, "low": 88.0, "close": 89.0, "volume": 100_000},    # Low 88.0 breaches SL 90.0
        {"timestamp": datetime.combine(base_date + timedelta(days=4), datetime.min.time()), "open": 65.0, "high": 68.0, "low": 62.0, "close": 66.0, "volume": 100_000},    # Gaps down to 65.0!
        {"timestamp": datetime.combine(base_date + timedelta(days=5), datetime.min.time()), "open": 66.0, "high": 70.0, "low": 65.0, "close": 68.0, "volume": 100_000},
    ]
    df = pd.DataFrame(candles)

    # Signal triggers on bar 1 close:
    fast_ma = np.array([10.0, 30.0, 10.0, 10.0, 10.0, 10.0])
    slow_ma = np.array([20.0] * 6)

    strat_obj = _make_strategy_ns(stop_loss_percent=10.0, stop_loss_price=90.0, sizing_method="fixed_quantity", sizing_qty=100)
    strat_obj.exit_rules = []  # Rely exclusively on SL

    # Execute under next_open mode
    kernel = BacktestExecutionKernel(min_holding_bars=0, sl_tp_execution_mode="next_open")
    res = kernel.run(
        df=df,
        indicator_values={"sma_fast": fast_ma, "sma_slow": slow_ma},
        strategy=strat_obj,
        symbol="GAP_DOWN_SYM",
        initial_cash=1_000_000.0,
    )

    closed = [t for t in res.ledger.trades if t.status == "closed"]
    assert len(closed) == 1
    trade = closed[0]

    assert trade.entry_bar_index == 2
    assert trade.entry_price == 100.0
    assert trade.exit_bar_index == 4       # Filled on Bar 4 Open (T+1 after breach bar 3)
    assert trade.exit_price == 65.0        # Filled at severe gap-down Open, NOT 90.0!
    assert trade.exit_reason == "STOP_LOSS"
    assert trade.gross_pnl == pytest.approx(-3500.0, rel=1e-6)
    assert trade.result == "loss"

    # Buy fee = 100.0 * 100 * 0.0015 = 15.0
    # Sell fee = 65.0 * 100 * 0.0015 = 9.75
    # Sell tax = 65.0 * 100 * 0.0010 = 6.50
    # Total costs = 31.25
    # Net PnL = -3500.0 - 31.25 = -3531.25
    assert trade.fees == pytest.approx(24.75, rel=1e-6)
    assert trade.taxes == pytest.approx(6.50, rel=1e-6)
    assert trade.net_pnl == pytest.approx(-3531.25, rel=1e-6)


def test_adv_gap_down_sl_with_slippage_rate():
    """Verify that when slippage_rate > 0 is configured, the gap-down exit price absorbs both gap and slippage."""
    base_date = date(2024, 1, 1)
    candles = [
        {"timestamp": datetime.combine(base_date, datetime.min.time()), "open": 100.0, "high": 102.0, "low": 98.0, "close": 100.0, "volume": 100_000},
        {"timestamp": datetime.combine(base_date + timedelta(days=1), datetime.min.time()), "open": 100.0, "high": 102.0, "low": 98.0, "close": 100.0, "volume": 100_000},
        {"timestamp": datetime.combine(base_date + timedelta(days=2), datetime.min.time()), "open": 100.0, "high": 102.0, "low": 98.0, "close": 100.0, "volume": 100_000},
        {"timestamp": datetime.combine(base_date + timedelta(days=3), datetime.min.time()), "open": 98.0, "high": 99.0, "low": 88.0, "close": 89.0, "volume": 100_000},
        {"timestamp": datetime.combine(base_date + timedelta(days=4), datetime.min.time()), "open": 65.0, "high": 68.0, "low": 62.0, "close": 66.0, "volume": 100_000},
    ]
    df = pd.DataFrame(candles)

    fast_ma = np.array([10.0, 30.0, 10.0, 10.0, 10.0])
    slow_ma = np.array([20.0] * 5)

    strat_obj = _make_strategy_ns(stop_loss_percent=10.0, stop_loss_price=90.0, sizing_method="fixed_quantity", sizing_qty=100)
    strat_obj.exit_rules = []

    # 1% slippage on exits
    kernel = BacktestExecutionKernel(min_holding_bars=0, slippage_rate=0.01, sl_tp_execution_mode="next_open")
    res = kernel.run(
        df=df,
        indicator_values={"sma_fast": fast_ma, "sma_slow": slow_ma},
        strategy=strat_obj,
        symbol="SLIPPAGE_SYM",
        initial_cash=1_000_000.0,
    )

    trade = res.ledger.trades[0]
    expected_exit = 65.0 * (1.0 - 0.01)  # 64.35
    expected_entry = 100.0 * (1.0 + 0.01)  # 101.0
    assert trade.exit_price == pytest.approx(expected_exit, rel=1e-6)
    assert trade.gross_pnl == pytest.approx((expected_exit - expected_entry) * 100, rel=1e-6)


def test_adv_gap_down_sl_comparison_next_open_vs_intraday_touch():
    """Empirically compare next_open vs intraday_touch on the identical gap-down dataset.

    Proves that next_open eliminates the artificial, unfillable fill price at the SL touch level.
    """
    base_date = date(2024, 1, 1)
    candles = [
        {"timestamp": datetime.combine(base_date, datetime.min.time()), "open": 100.0, "high": 102.0, "low": 98.0, "close": 100.0, "volume": 100_000},
        {"timestamp": datetime.combine(base_date + timedelta(days=1), datetime.min.time()), "open": 100.0, "high": 102.0, "low": 98.0, "close": 100.0, "volume": 100_000},
        {"timestamp": datetime.combine(base_date + timedelta(days=2), datetime.min.time()), "open": 100.0, "high": 102.0, "low": 98.0, "close": 100.0, "volume": 100_000},
        {"timestamp": datetime.combine(base_date + timedelta(days=3), datetime.min.time()), "open": 98.0, "high": 99.0, "low": 88.0, "close": 89.0, "volume": 100_000},
        {"timestamp": datetime.combine(base_date + timedelta(days=4), datetime.min.time()), "open": 65.0, "high": 68.0, "low": 62.0, "close": 66.0, "volume": 100_000},
    ]
    df = pd.DataFrame(candles)

    fast_ma = np.array([10.0, 30.0, 10.0, 10.0, 10.0])
    slow_ma = np.array([20.0] * 5)

    strat_obj = _make_strategy_ns(stop_loss_percent=10.0, stop_loss_price=90.0, sizing_method="fixed_quantity", sizing_qty=100)
    strat_obj.exit_rules = []

    # 1. next_open
    kernel_no = BacktestExecutionKernel(min_holding_bars=0, sl_tp_execution_mode="next_open")
    res_no = kernel_no.run(
        df=df, indicator_values={"sma_fast": fast_ma, "sma_slow": slow_ma},
        strategy=strat_obj, symbol="COMP_SYM", initial_cash=1_000_000.0,
    )
    trade_no = res_no.ledger.trades[0]

    # 2. intraday_touch
    kernel_it = BacktestExecutionKernel(min_holding_bars=0, sl_tp_execution_mode="intraday_touch")
    res_it = kernel_it.run(
        df=df, indicator_values={"sma_fast": fast_ma, "sma_slow": slow_ma},
        strategy=strat_obj, symbol="COMP_SYM", initial_cash=1_000_000.0,
    )
    trade_it = res_it.ledger.trades[0]

    # Verification:
    # next_open filled at Bar 4 Open = 65.0
    assert trade_no.exit_bar_index == 4
    assert trade_no.exit_price == 65.0
    assert trade_no.net_pnl == pytest.approx(-3531.25, rel=1e-5)

    # intraday_touch filled at Bar 3 SL touch = 90.0 (ignoring the overnight collapse!)
    assert trade_it.exit_bar_index == 3
    assert trade_it.exit_price == 90.0
    assert trade_it.net_pnl == pytest.approx(-1037.5, rel=1e-5)

    # The optimistic bias delta is over 2,490 VND per share!
    pnl_delta = trade_it.net_pnl - trade_no.net_pnl
    assert pnl_delta > 2490.0


# ==============================================================================
# SECTION 3: Multi-Variant Parameter Sweep vs Single Backtest Parity
# ==============================================================================

@pytest.mark.asyncio
async def test_adv_multi_variant_sweep_vs_single_exact_parity(isolated_challenger_db: Session):
    """Adversarial Parity Test across Multiple Variants:

    Construct a 3x3 parameter sweep (9 variants total):
    - sma_fast lengths: [3, 5, 7]
    - sma_slow lengths: [10, 15, 20]

    Execute run_parameter_sweep and compare EVERY variant against a separate
    run_backtest call on the same isolated database.
    Verify 100% exact numerical match across all 9 variants!
    """
    symbol = "MULTI_PARITY"
    base_date = date(2024, 1, 1)
    num_days = 70

    # Seed deterministic price trajectory with clear oscillating crossovers
    for i in range(num_days):
        day = base_date + timedelta(days=i)
        price = 60.0 + math.sin(i / 4.0) * 8.0 + (i * 0.15)
        price = round(price, 2)
        isolated_challenger_db.add(Candle(
            symbol=symbol,
            timeframe="1D",
            timestamp=day,
            open=price,
            high=price + 1.2,
            low=price - 1.2,
            close=price,
            volume=250_000,
        ))
    isolated_challenger_db.commit()

    start_date_str = str(base_date)
    end_date_str = str(base_date + timedelta(days=num_days))
    initial_cash = 100_000_000.0

    base_strat = {
        "name": "MultiVariantTest",
        "indicators": [
            {"name": "fast_ma", "type": "sma", "length": 5},
            {"name": "slow_ma", "type": "sma", "length": 15},
        ],
        "entry_rules": [{"condition": "fast_ma > slow_ma"}],
        "exit_rules": [{"condition": "fast_ma < slow_ma"}],
        "position_sizing": {"method": "fixed_quantity", "quantity": 1000},
    }

    fast_lens = [3, 5, 7]
    slow_lens = [10, 15, 20]

    sweep_config = {
        "symbol": symbol,
        "start_date": start_date_str,
        "end_date": end_date_str,
        "initial_cash": initial_cash,
        "strategy": base_strat,
        "sweep": [
            {"path": "indicators[0].length", "values": fast_lens},
            {"path": "indicators[1].length", "values": slow_lens},
        ],
    }

    strategy_lab_svc = StrategyLabService()
    backtest_svc = BacktestService()

    # 1. Run the Multi-Variant Parameter Sweep
    sweep_result = await strategy_lab_svc.run_parameter_sweep(isolated_challenger_db, sweep_config)
    assert sweep_result["status"] == "succeeded"
    assert sweep_result["total_variants"] == len(fast_lens) * len(slow_lens) == 9

    # 2. Iterate through every single variant and verify parity against single backtest
    variants_checked = 0
    total_trades_accum = 0

    for variant in sweep_result["variants"]:
        params = variant["parameters"]
        v_fast = params["indicators[0].length"]
        v_slow = params["indicators[1].length"]

        # Build equivalent single backtest strategy
        single_strat = {
            "name": f"Single_{v_fast}_{v_slow}",
            "indicators": [
                {"name": "fast_ma", "type": "sma", "length": v_fast},
                {"name": "slow_ma", "type": "sma", "length": v_slow},
            ],
            "entry_rules": [{"condition": "fast_ma > slow_ma"}],
            "exit_rules": [{"condition": "fast_ma < slow_ma"}],
            "position_sizing": {"method": "fixed_quantity", "quantity": 1000},
        }

        single_config = {
            "symbol": symbol,
            "start_date": start_date_str,
            "end_date": end_date_str,
            "initial_cash": initial_cash,
            "strategy": single_strat,
        }

        single_result = await backtest_svc.run_backtest(isolated_challenger_db, single_config)
        assert single_result["status"] == "succeeded"
        s_analytics = single_result["analytics"]

        s_trades = s_analytics["total_trades"]
        s_net_pnl = s_analytics["total_net_pnl"]
        s_win_rate = s_analytics["win_rate"]
        s_pf = s_analytics["profit_factor"]

        v_metrics = variant["metrics"]
        v_trades = v_metrics["total_trades"]
        v_net_pnl = v_metrics["net_pnl"]
        v_win_rate = v_metrics["win_rate"]
        v_pf = v_metrics["profit_factor"]

        # Exact parity checks
        assert s_trades == v_trades, f"Trade count mismatch for ({v_fast}, {v_slow}): single={s_trades}, sweep={v_trades}"
        assert math.isclose(s_net_pnl, v_net_pnl, rel_tol=1e-5, abs_tol=1e-2), (
            f"Net PnL mismatch for ({v_fast}, {v_slow}): single={s_net_pnl}, sweep={v_net_pnl}"
        )
        assert math.isclose(s_win_rate, v_win_rate, rel_tol=1e-5, abs_tol=1e-4), (
            f"Win rate mismatch for ({v_fast}, {v_slow}): single={s_win_rate}, sweep={v_win_rate}"
        )
        if s_pf is not None and v_pf is not None:
            assert math.isclose(s_pf, v_pf, rel_tol=1e-4, abs_tol=1e-2)
        else:
            assert s_pf is None and v_pf is None

        variants_checked += 1
        total_trades_accum += s_trades

    assert variants_checked == 9
    assert total_trades_accum > 0, "Expected non-zero total trades across all variants"


def test_adv_sweep_cancellation_manager_integrity():
    """Verify that SweepCancellationManager correctly registers, cancels, checks, and cleans up."""
    test_id = "test_sweep_cancel_123"
    assert not SweepCancellationManager.is_cancelled(test_id)

    SweepCancellationManager.register_sweep(test_id)
    assert not SweepCancellationManager.is_cancelled(test_id)

    SweepCancellationManager.cancel_sweep(test_id)
    assert SweepCancellationManager.is_cancelled(test_id)

    SweepCancellationManager.cleanup_sweep(test_id)
    assert not SweepCancellationManager.is_cancelled(test_id)


# ==============================================================================
# SECTION 4: Defect Remediation Verification — StrategyConfig stop_loss_pct Contract Parity
# ==============================================================================

def test_adv_schema_match_strategy_config_stop_loss_pct_verified():
    """Empirical Verification of Remediated Schema Contract:

    When a strategy is loaded from dict/JSON using the canonical StrategyConfig schema:
    `strat = load_strategy_from_dict({"risk_management": {"stop_loss_pct": 5.0}})`
    The Pydantic model produces `strat.risk_management.stop_loss_pct = 5.0`.

    `BacktestExecutionKernel` now properly inspects `stop_loss_pct` and `take_profit_pct`
    on `strategy.risk_management` (both dict and model representations).

    Verification:
    - Position entered on Bar 2 at 100.0.
    - Bar 3 low=80.0 breaches 5% SL (sl_price=95.0), queuing an exit at Bar 3 close.
    - Exit executes at Bar 4 Open = 65.0 under next_open mode.
    - Trade closes successfully with exit_reason='STOP_LOSS' and exit_price=65.0.
    """
    base_date = date(2024, 1, 1)
    candles = [
        {"timestamp": datetime.combine(base_date, datetime.min.time()), "open": 100.0, "high": 102.0, "low": 98.0, "close": 100.0, "volume": 100_000},
        {"timestamp": datetime.combine(base_date + timedelta(days=1), datetime.min.time()), "open": 100.0, "high": 102.0, "low": 98.0, "close": 100.0, "volume": 100_000},  # Signal on bar 1 close
        {"timestamp": datetime.combine(base_date + timedelta(days=2), datetime.min.time()), "open": 100.0, "high": 102.0, "low": 98.0, "close": 100.0, "volume": 100_000},  # Filled at 100.0
        {"timestamp": datetime.combine(base_date + timedelta(days=3), datetime.min.time()), "open": 90.0, "high": 92.0, "low": 80.0, "close": 85.0, "volume": 100_000},     # 15% drop breaches 5% SL
        {"timestamp": datetime.combine(base_date + timedelta(days=4), datetime.min.time()), "open": 65.0, "high": 75.0, "low": 60.0, "close": 65.0, "volume": 100_000},     # Fills at Open[T+2]=65.0
        {"timestamp": datetime.combine(base_date + timedelta(days=5), datetime.min.time()), "open": 50.0, "high": 55.0, "low": 45.0, "close": 50.0, "volume": 100_000},
    ]
    df = pd.DataFrame(candles)

    fast_ma = np.array([10.0, 30.0, 10.0, 10.0, 10.0, 10.0])
    slow_ma = np.array([20.0] * 6)

    strat_dict = {
        "name": "schema_mismatch_test",
        "indicators": [{"name": "sma_fast", "type": "sma"}, {"name": "sma_slow", "type": "sma"}],
        "entry_rules": [{"condition": "sma_fast > sma_slow"}],
        "exit_rules": [],
        "position_sizing": {"method": "fixed_quantity", "quantity": 100},
        "risk_management": {"stop_loss_pct": 5.0},
    }
    strat_obj = load_strategy_from_dict(strat_dict)

    assert hasattr(strat_obj.risk_management, "stop_loss_pct")
    assert strat_obj.risk_management.stop_loss_pct == 5.0

    kernel = BacktestExecutionKernel(min_holding_bars=0, sl_tp_execution_mode="next_open")
    res = kernel.run(
        df=df,
        indicator_values={"sma_fast": fast_ma, "sma_slow": slow_ma},
        strategy=strat_obj,
        symbol="DEFECT_SYM",
        initial_cash=100_000.0,
    )

    closed_trades = [t for t in res.ledger.trades if t.status == "closed"]
    open_trades = [t for t in res.ledger.trades if t.status == "open"]

    assert len(closed_trades) == 1, "Remediation verified: trade closed on stop loss"
    assert len(open_trades) == 0, "No trades remain open"
    assert closed_trades[0].exit_reason == "STOP_LOSS"
    assert closed_trades[0].exit_price == 65.0


# ==============================================================================
# SECTION 5: Database Safety & Invariance
# ==============================================================================

def test_adv_database_safety_invariance():
    """Verify that backend/sumi.db SHA-256 remains strictly untouched."""
    import hashlib
    from pathlib import Path

    prod_db = Path("/Users/mizuhara/workspace/sumi/backend/sumi.db")
    assert prod_db.exists(), "Production DB must exist"

    hasher = hashlib.sha256()
    with open(prod_db, "rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            hasher.update(chunk)

    expected_hash = "234206fc75259f091337d4feb5f41575d0fe5ac02d6c10ba0de8fad68cbcee86"
    assert hasher.hexdigest() == expected_hash
    assert prod_db.stat().st_size == 188416
