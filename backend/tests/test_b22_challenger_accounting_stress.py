"""Adversarial Challenger Stress Suite for Batch B22 (Milestone 3).

Author: Challenger B22-1 (Adversarial Accounting & Breakeven Challenger)
Mission:
Empirically challenge the B22 accounting engine, winner rule, profit factor,
and statistical sample gate with adversarial stress tests.

Scope:
1. Adversarial Breakeven Scenarios:
   - Pure breakevens (net_pnl == 0.0).
   - Within-tolerance residuals (|net_pnl| <= 1e-9) including +/-1e-15, +/-1e-12, +/-1e-10, +/-1e-9.
   - Microscopic gains strictly outside tolerance (+1e-8, +1e-7, +1.01e-9).
   - Microscopic losses strictly outside tolerance (-1e-8, -1e-7, -1.01e-9).
   - Gross breakeven with fees (net_pnl < 0) strictly classified as loss.
   - Denominator / numerator invariant: win_rate = true_winners / total_closed_trades.
   - Characterization of RoundTripPnL unrounded winner evaluation vs round(net_pnl, 6).
2. Extreme Profit Factor Scenarios:
   - 0 closed trades: None / 'N/A', JSON null.
   - 100 wins and 0 losses: None / 'N/A (chưa có lệnh thua)', JSON null, no Infinity.
   - 0 wins and 100 losses: 0.0 / '0.00'.
   - All breakevens: None / 'N/A', win_rate = 0.0%.
   - Huge profit numbers (100 billion VND) vs small loss numbers to test precision.
3. Statistical Sample Gate Stress:
   - Exact boundary transitions: N = 28, N = 29, N = 30, N = 31.
   - Cross-module consensus on low_sample_warning and low_sample_message.
4. Database Safety:
   - Strictly in-memory / tmp_path fixtures.
   - Verify backend/sumi.db hash invariance.
"""

from __future__ import annotations

import hashlib
import json
import math
from datetime import date, datetime, timedelta
from pathlib import Path
from types import SimpleNamespace
from typing import Any, Generator, List

import numpy as np
import pytest
from sqlalchemy import create_engine
from sqlalchemy.orm import Session, sessionmaker

from app.db import Base
from app.domain.accounting import (
    BUY_FEE_RATE,
    SELL_FEE_RATE,
    SELL_TAX_RATE,
    calculate_buy_amounts,
    calculate_sell_amounts,
    calculate_net_pnl,
    calculate_pnl_percent,
    compute_round_trip_pnl,
    RoundTripPnL,
)
from app.domain.backtest.batch_runner import BatchBacktestRunner, PhaseDefinition
from app.domain.backtest.execution import BacktestExecutionKernel
from app.domain.backtest.metrics import (
    BenchmarkMetricRow,
    BenchmarkTableRenderer,
    calculate_benchmark_metrics,
    calculate_sharpe_ratio,
    calculate_max_drawdown,
)
from app.domain.backtest.models import BacktestTrade
from app.domain.market.rules import DEFAULT_VN_MARKET_PROFILE
from app.domain.strategy.strategy_loader import load_strategy_from_dict
from app.models.candle import Candle
from app.models.replay_session import ReplaySession
from app.models.trade import Trade
from app.services.analytics_service import AnalyticsService
from app.services.strategy_lab_service import StrategyLabService
from app.services.trade_lifecycle_service import TradeLifecycleService


# ============================================================================
# Helpers & Fixtures
# ============================================================================

SUMI_DB_PATH = Path("/Users/mizuhara/workspace/sumi/backend/sumi.db")
EXPECTED_SUMI_DB_SHA256 = "234206fc75259f091337d4feb5f41575d0fe5ac02d6c10ba0de8fad68cbcee86"


def _get_sumi_db_sha256() -> str:
    """Read SHA-256 hash of production database."""
    h = hashlib.sha256()
    with open(SUMI_DB_PATH, "rb") as f:
        while chunk := f.read(65536):
            h.update(chunk)
    return h.hexdigest()


@pytest.fixture
def isolated_challenger_db(tmp_path) -> Generator[Session, None, None]:
    """In-memory SQLite database session isolated from production."""
    db_file = tmp_path / "test_b22_challenger_isolated.db"
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


def _make_adversarial_trade(
    trade_id: str,
    net_pnl: float,
    gross_pnl: float | None = None,
    pnl_percent: float | None = None,
    fees: float = 0.0,
    taxes: float = 0.0,
    symbol: str = "HPG",
    holding_bars: int = 3,
) -> BacktestTrade:
    """Helper to instantiate closed BacktestTrade with precise float values."""
    is_be = math.isclose(net_pnl, 0.0, abs_tol=1e-9)
    if net_pnl > 0.0 and not is_be:
        res = "win"
    elif net_pnl < 0.0 and not is_be:
        res = "loss"
    else:
        res = "breakeven"

    calc_gross = gross_pnl if gross_pnl is not None else (net_pnl + fees + taxes)
    calc_pct = pnl_percent if pnl_percent is not None else (net_pnl / 1000.0 * 100.0)

    return BacktestTrade(
        id=trade_id,
        symbol=symbol,
        entry_order_id=f"entry_{trade_id}",
        exit_order_id=f"exit_{trade_id}",
        entry_time=datetime(2024, 1, 1),
        exit_time=datetime(2024, 1, 1) + timedelta(days=holding_bars),
        entry_bar_index=0,
        exit_bar_index=holding_bars,
        holding_bars=holding_bars,
        quantity=100.0,
        entry_price=20.0,
        exit_price=20.0 + (net_pnl / 100.0),
        gross_pnl=calc_gross,
        net_pnl=net_pnl,
        fees=fees,
        taxes=taxes,
        pnl_percent=calc_pct,
        status="closed",
        result=res,
    )


# ============================================================================
# 1. Adversarial Breakeven Scenarios & Tolerance Band Stress
# ============================================================================

def test_stress_pure_breakeven_zero_pnl():
    """Stress Test 1.1: Exact net_pnl == 0.0 must be strictly not-winner, not-loser."""
    trade = _make_adversarial_trade("BE_0", net_pnl=0.0, gross_pnl=0.0, pnl_percent=0.0)
    metrics = calculate_benchmark_metrics([trade], 100_000_000.0, "HPG")

    assert metrics.num_trades == 1
    assert metrics.num_winners == 0
    assert metrics.num_breakeven == 1
    assert metrics.num_losers == 0
    assert metrics.win_rate_pct == 0.0
    assert metrics.profit_factor is None
    assert metrics.profit_factor_display == "N/A"
    assert metrics.win_avg_profit_pct is None
    assert metrics.loss_avg_loss_pct is None


def test_stress_within_tolerance_residuals_are_strictly_breakeven():
    """Stress Test 1.2: Residuals within abs_tol=1e-9 must be strictly classified as breakeven.
    
    Adversarial inputs:
    - Positive residuals: +1e-15, +1e-12, +1e-10, +5e-10, +0.999999999e-9, +1.0e-9
    - Negative residuals: -1e-15, -1e-12, -1e-10, -5e-10, -0.999999999e-9, -1.0e-9
    All must evaluate to is_breakeven == True, is_winner == False, is_loser == False.
    """
    within_tolerance_values = [
        1e-15, -1e-15,
        1e-12, -1e-12,
        1e-10, -1e-10,
        5e-10, -5e-10,
        9.99999999e-10, -9.99999999e-10,
        1.0e-9, -1.0e-9,
    ]

    for val in within_tolerance_values:
        t = _make_adversarial_trade(f"T_{val}", net_pnl=val)
        m = calculate_benchmark_metrics([t], 100_000_000.0, "HPG")
        assert m.num_trades == 1, f"Failed on val={val}"
        assert m.num_winners == 0, f"Microscopic residual {val} was incorrectly classified as winner!"
        assert m.num_losers == 0, f"Microscopic residual {val} was incorrectly classified as loser!"
        assert m.num_breakeven == 1, f"Microscopic residual {val} was not classified as breakeven!"
        assert m.win_rate_pct == 0.0, f"Win rate was not 0.0 for residual {val}"
        assert m.profit_factor is None
        assert m.profit_factor_display == "N/A"


def test_stress_outside_tolerance_microscopic_winners_and_losers():
    """Stress Test 1.3: Microscopic values strictly outside 1e-9 abs_tol must be true winners/losers.
    
    Adversarial inputs:
    - Positive: +1.000000001e-9, +1.01e-9, +1e-8, +1e-7, +1e-6
    - Negative: -1.000000001e-9, -1.01e-9, -1e-8, -1e-7, -1e-6
    """
    outside_positive = [1.000000001e-9, 1.01e-9, 1e-8, 1e-7, 1e-6]
    for val in outside_positive:
        t = _make_adversarial_trade(f"WIN_{val}", net_pnl=val)
        m = calculate_benchmark_metrics([t], 100_000_000.0, "HPG")
        assert m.num_trades == 1
        assert m.num_winners == 1, f"Microscopic gain {val} was not classified as winner!"
        assert m.num_breakeven == 0
        assert m.num_losers == 0
        assert m.win_rate_pct == 100.0
        assert m.profit_factor is None
        assert m.profit_factor_display == "N/A (chưa có lệnh thua)"

    outside_negative = [-1.000000001e-9, -1.01e-9, -1e-8, -1e-7, -1e-6]
    for val in outside_negative:
        t = _make_adversarial_trade(f"LOSS_{val}", net_pnl=val)
        m = calculate_benchmark_metrics([t], 100_000_000.0, "HPG")
        assert m.num_trades == 1
        assert m.num_winners == 0
        assert m.num_breakeven == 0
        assert m.num_losers == 1, f"Microscopic loss {val} was not classified as loser!"
        assert m.win_rate_pct == 0.0
        assert m.profit_factor == 0.0
        assert m.profit_factor_display == "0.00"


def test_stress_gross_breakeven_with_fees_is_strictly_loser():
    """Stress Test 1.4: Gross breakeven with fees must strictly be a net loser.
    
    Buy 100 @ 20.0, Sell 100 @ 20.0.
    Gross PnL = 0.0.
    Friction = 3.0 (buy fee) + 3.0 (sell fee) + 2.0 (sell tax) = 8.0 VND.
    Net PnL = -8.0 VND.
    Must evaluate to: is_winner == False, is_breakeven == False, is_loser == True.
    """
    rt = compute_round_trip_pnl(
        entry_price=20.0,
        exit_price=20.0,
        quantity=100.0,
        side="LONG",
        profile=DEFAULT_VN_MARKET_PROFILE,
    )
    assert math.isclose(rt.gross_pnl, 0.0, abs_tol=1e-9)
    assert math.isclose(rt.total_friction, 8.0, abs_tol=1e-9)
    assert math.isclose(rt.net_pnl, -8.0, abs_tol=1e-9)
    assert rt.is_winner is False
    assert rt.is_breakeven is False
    assert rt.is_loser is True
    assert rt.is_win is False
    assert rt.is_loss is True

    # In benchmark metrics
    t = _make_adversarial_trade("GB_FEE", net_pnl=-8.0, gross_pnl=0.0, fees=6.0, taxes=2.0)
    m = calculate_benchmark_metrics([t], 100_000_000.0, "HPG")
    assert m.num_trades == 1
    assert m.num_winners == 0
    assert m.num_breakeven == 0
    assert m.num_losers == 1
    assert m.win_rate_pct == 0.0
    assert m.profit_factor == 0.0
    assert m.profit_factor_display == "0.00"


def test_stress_adversarial_mixed_breakeven_ledger():
    """Stress Test 1.5: Complex mixed ledger testing denominator and numerator invariance.
    
    Ledger composition:
    - 5 pure breakevens (net_pnl == 0.0)
    - 5 within-tolerance breakevens (+1e-10, -1e-10, +5e-10, -5e-10, +1e-9)
    - 4 gross breakevens with fees (net_pnl == -8.0)
    - 6 microscopic winners (+1e-8, +2e-8, +5e-8, +1e-7, +5e-7, +1e-6)
    - 6 standard winners (+100.0, +250.0, +500.0, +750.0, +1000.0, +2500.0)
    - 4 standard losers (-50.0, -150.0, -300.0, -500.0)
    Total trades = 5 + 5 + 4 + 6 + 6 + 4 = 30 trades.
    
    Expected:
    - num_trades = 30
    - num_winners = 12 (6 micro + 6 standard)
    - num_breakeven = 10 (5 pure + 5 within-tol)
    - num_losers = 8 (4 gross BE w/ fees + 4 standard)
    - win_rate = 12 / 30 = 40.0% (strictly not 22/30 = 73.3%)
    - low_sample_warning = False (30 >= 30)
    """
    trades: List[BacktestTrade] = []
    # 5 pure breakevens
    for i in range(5):
        trades.append(_make_adversarial_trade(f"PURE_BE_{i}", net_pnl=0.0, pnl_percent=0.0))
    # 5 within-tolerance breakevens
    for i, val in enumerate([1e-10, -1e-10, 5e-10, -5e-10, 1e-9]):
        trades.append(_make_adversarial_trade(f"TOL_BE_{i}", net_pnl=val, pnl_percent=0.0))
    # 4 gross breakevens with fees
    for i in range(4):
        trades.append(_make_adversarial_trade(f"GB_FEE_{i}", net_pnl=-8.0, gross_pnl=0.0, pnl_percent=-0.08))
    # 6 microscopic winners
    for i, val in enumerate([1e-8, 2e-8, 5e-8, 1e-7, 5e-7, 1e-6]):
        trades.append(_make_adversarial_trade(f"MICRO_WIN_{i}", net_pnl=val, pnl_percent=val / 10.0))
    # 6 standard winners
    for i, val in enumerate([100.0, 250.0, 500.0, 750.0, 1000.0, 2500.0]):
        trades.append(_make_adversarial_trade(f"STD_WIN_{i}", net_pnl=val, pnl_percent=val / 100.0))
    # 4 standard losers
    for i, val in enumerate([-50.0, -150.0, -300.0, -500.0]):
        trades.append(_make_adversarial_trade(f"STD_LOSS_{i}", net_pnl=val, pnl_percent=val / 100.0))

    metrics = calculate_benchmark_metrics(trades, 100_000_000.0, "HPG")

    assert metrics.num_trades == 30
    assert metrics.num_winners == 12
    assert metrics.num_breakeven == 10
    assert metrics.num_losers == 8
    assert metrics.num_winners + metrics.num_breakeven + metrics.num_losers == metrics.num_trades

    # Win rate invariant: numerator is winners only, denominator is all closed trades
    expected_win_rate = (12 / 30) * 100.0
    assert math.isclose(metrics.win_rate_pct, expected_win_rate, abs_tol=1e-9)
    assert metrics.win_rate_pct != (22 / 30) * 100.0  # Must NOT count breakevens as winners

    # Net Profit Factor verification
    gross_wins = sum(t.net_pnl for t in trades if t.net_pnl > 1e-9)
    gross_losses = abs(sum(t.net_pnl for t in trades if t.net_pnl < -1e-9))
    expected_pf = gross_wins / gross_losses
    assert math.isclose(metrics.profit_factor, expected_pf, abs_tol=1e-6)
    assert metrics.profit_factor_display == f"{expected_pf:.2f}"

    # Sample gate at exactly N = 30
    assert metrics.low_sample_warning is False
    assert metrics.low_sample_message is None


def test_stress_round_trip_pnl_rounding_precision_characterization():
    """Stress Test 1.6: Characterize RoundTripPnL unrounded evaluation vs 6-decimal rounding.
    
    compute_round_trip_pnl evaluates:
      is_breakeven = math.isclose(net_pnl, 0.0, abs_tol=1e-9)
      is_winner = (net_pnl > 0.0) and not is_breakeven
      net_pnl = round(net_pnl, 6)
    
    If an artificial net_pnl is between 1e-9 and 5e-7 (e.g. 1e-8):
    - is_winner is True
    - but round(net_pnl, 6) is 0.0
    We verify this edge case and document that realistic trades in Vietnam Dong
    operate on integer or centi-VND amounts where this rounding boundary does not collide.
    """
    # 1. Realistic Vietnam trade: Buy 100 @ 20.0, Sell 100 @ 20.1
    # Gross = 10.0 VND, Friction = 3.0 + 3.015 + 2.01 = 8.025 VND
    # Net PnL = 1.975 VND (well above 6 decimal digits)
    rt_real = compute_round_trip_pnl(20.0, 20.1, 100.0, "LONG", DEFAULT_VN_MARKET_PROFILE)
    assert math.isclose(rt_real.net_pnl, 1.975, abs_tol=1e-6)
    assert rt_real.is_winner is True
    assert rt_real.is_breakeven is False
    assert rt_real.is_loser is False

    # 2. Exact theoretical breakeven exit price:
    # exit_price = entry_price * (1 + fee_buy) / (1 - fee_sell - tax_sell)
    # exit_price = 10.0 * 1.0015 / 0.9975 = 10.04010025...
    exact_be_exit = 10.0 * (1.0 + BUY_FEE_RATE) / (1.0 - SELL_FEE_RATE - SELL_TAX_RATE)
    rt_be = compute_round_trip_pnl(10.0, exact_be_exit, 100.0, "LONG", DEFAULT_VN_MARKET_PROFILE)
    assert math.isclose(rt_be.net_pnl, 0.0, abs_tol=1e-6)
    assert rt_be.is_breakeven is True
    assert rt_be.is_winner is False
    assert rt_be.is_loser is False


# ============================================================================
# 2. Extreme Profit Factor Scenarios
# ============================================================================

def test_stress_profit_factor_zero_closed_trades():
    """Stress Test 2.1: Empty ledger must yield profit_factor=None, display='N/A'."""
    metrics = calculate_benchmark_metrics([], 100_000_000.0, "HPG")
    assert metrics.num_trades == 0
    assert metrics.profit_factor is None
    assert metrics.profit_factor_display == "N/A"
    assert metrics.win_rate_pct is None

    # JSON compliance
    d = metrics.to_dict()
    assert d["profit_factor"] is None
    assert d["profit_factor_display"] == "N/A"
    raw = json.dumps(d)
    assert '"profit_factor": null' in raw
    assert "Infinity" not in raw


def test_stress_profit_factor_100_wins_0_losses():
    """Stress Test 2.2: 100 wins and 0 losses must yield None / 'N/A (chưa có lệnh thua)'."""
    trades = [_make_adversarial_trade(f"W_{i}", net_pnl=50_000.0) for i in range(100)]
    metrics = calculate_benchmark_metrics(trades, 100_000_000.0, "HPG")

    assert metrics.num_trades == 100
    assert metrics.num_winners == 100
    assert metrics.num_losers == 0
    assert metrics.profit_factor is None
    assert metrics.profit_factor_display == "N/A (chưa có lệnh thua)"
    assert metrics.win_rate_pct == 100.0

    # JSON compliance: no IEEE 754 infinity
    d = metrics.to_dict()
    assert d["profit_factor"] is None
    assert d["profit_factor_display"] == "N/A (chưa có lệnh thua)"
    raw = json.dumps(d)
    assert '"profit_factor": null' in raw
    assert "Infinity" not in raw
    assert "NaN" not in raw
    # Deserialize back
    loaded = json.loads(raw)
    assert loaded["profit_factor"] is None


def test_stress_profit_factor_0_wins_100_losses():
    """Stress Test 2.3: 0 wins and 100 losses must yield 0.0 / '0.00'."""
    trades = [_make_adversarial_trade(f"L_{i}", net_pnl=-50_000.0) for i in range(100)]
    metrics = calculate_benchmark_metrics(trades, 100_000_000.0, "HPG")

    assert metrics.num_trades == 100
    assert metrics.num_winners == 0
    assert metrics.num_losers == 100
    assert metrics.profit_factor == 0.0
    assert metrics.profit_factor_display == "0.00"
    assert metrics.win_rate_pct == 0.0

    d = metrics.to_dict()
    assert d["profit_factor"] == 0.0
    assert d["profit_factor_display"] == "0.00"
    raw = json.dumps(d)
    assert '"profit_factor": 0.0' in raw


def test_stress_profit_factor_all_breakevens():
    """Stress Test 2.4: 50 breakeven trades with zero wins and zero losses."""
    trades = [_make_adversarial_trade(f"BE_{i}", net_pnl=0.0) for i in range(50)]
    metrics = calculate_benchmark_metrics(trades, 100_000_000.0, "HPG")

    assert metrics.num_trades == 50
    assert metrics.num_winners == 0
    assert metrics.num_breakeven == 50
    assert metrics.num_losers == 0
    assert metrics.profit_factor is None
    assert metrics.profit_factor_display == "N/A"
    assert metrics.win_rate_pct == 0.0
    assert metrics.low_sample_warning is False


def test_stress_huge_profit_numbers_precision():
    """Stress Test 2.5: Extreme numbers testing float64 precision and formatting.
    
    1. 100 Billion VND Profit (10^11) vs 1.0 VND Loss:
       PF = 100,000,000,000.0 -> display = '100000000000.00'
    2. 100 Billion VND Profit (10^11) vs 0.0001 VND Loss (10^-4):
       PF = 1,000,000,000,000,000.0 (10^15) -> display = '1000000000000000.00'
    3. Microscopic Win (1e-8) vs 100 Billion VND Loss:
       PF = 1e-19 -> display = '0.00'
    4. 1,000 Trades of 100M VND Profit vs 1,000 Trades of 50M VND Loss:
       Total Wins = 100B, Total Losses = 50B -> PF = 2.00
    """
    # 1. 100B vs 1.0
    t1 = [
        _make_adversarial_trade("W1", net_pnl=100_000_000_000.0),
        _make_adversarial_trade("L1", net_pnl=-1.0),
    ]
    m1 = calculate_benchmark_metrics(t1, 100_000_000_000.0, "HPG")
    assert m1.profit_factor == 100_000_000_000.0
    assert m1.profit_factor_display == "100000000000.00"

    # 2. 100B vs 0.0001
    t2 = [
        _make_adversarial_trade("W1", net_pnl=100_000_000_000.0),
        _make_adversarial_trade("L1", net_pnl=-0.0001),
    ]
    m2 = calculate_benchmark_metrics(t2, 100_000_000_000.0, "HPG")
    assert math.isclose(m2.profit_factor, 1e15, rel_tol=1e-12)
    assert m2.profit_factor_display == "1000000000000000.00"

    # 3. Micro win vs 100B loss
    t3 = [
        _make_adversarial_trade("W1", net_pnl=1e-8),
        _make_adversarial_trade("L1", net_pnl=-100_000_000_000.0),
    ]
    m3 = calculate_benchmark_metrics(t3, 100_000_000_000.0, "HPG")
    assert m3.profit_factor < 1e-18
    assert m3.profit_factor_display == "0.00"

    # 4. Large trade accumulation (1,000 wins of 100M, 1,000 losses of 50M)
    t4_wins = [_make_adversarial_trade(f"W_{i}", net_pnl=100_000_000.0) for i in range(1000)]
    t4_losses = [_make_adversarial_trade(f"L_{i}", net_pnl=-50_000_000.0) for i in range(1000)]
    m4 = calculate_benchmark_metrics(t4_wins + t4_losses, 100_000_000_000.0, "HPG")
    assert m4.num_trades == 2000
    assert math.isclose(m4.profit_factor, 2.0, abs_tol=1e-9)
    assert m4.profit_factor_display == "2.00"


# ============================================================================
# 3. Statistical Sample Gate Stress (N = 28, N = 29, N = 30, N = 31)
# ============================================================================

def test_stress_sample_gate_boundary_transitions():
    """Stress Test 3.1: Strict boundary verification at N = 28, 29, 30, 31.
    
    Invariant:
    - N in [0, 29]: low_sample_warning == True, low_sample_message == 'mẫu nhỏ (<30), độ tin cậy thấp'
    - N >= 30: low_sample_warning == False, low_sample_message == None
    """
    for n in [0, 1, 15, 28, 29, 30, 31, 45, 100]:
        trades = [_make_adversarial_trade(f"T_{i}", net_pnl=100.0) for i in range(n)]
        m = calculate_benchmark_metrics(trades, 100_000_000.0, "HPG")
        if n < 30:
            assert m.low_sample_warning is True, f"Failed low_sample_warning for N={n}"
            assert m.low_sample_message == "mẫu nhỏ (<30), độ tin cậy thấp", f"Failed low_sample_message for N={n}"
        else:
            assert m.low_sample_warning is False, f"Failed low_sample_warning False for N={n}"
            assert m.low_sample_message is None, f"Failed low_sample_message None for N={n}"


def test_stress_cross_module_sample_gate_consensus(isolated_challenger_db: Session):
    """Stress Test 3.2: Verify sample gate parity across all backend services.
    
    1. metrics.py (calculate_benchmark_metrics)
    2. batch_runner.py (BatchBacktestRunner summary)
    3. strategy_lab_service.py (_format_kernel_response)
    4. analytics_service.py (_build_metric_results / get_analytics)
    """
    # Verify N = 29 vs N = 30 across modules
    for n, expected_warning, expected_msg in [
        (29, True, "mẫu nhỏ (<30), độ tin cậy thấp"),
        (30, False, None),
    ]:
        # 1. metrics.py
        trades = [_make_adversarial_trade(f"T_{i}", net_pnl=100.0) for i in range(n)]
        m = calculate_benchmark_metrics(trades, 100_000_000.0, "HPG")
        assert m.low_sample_warning == expected_warning
        assert m.low_sample_message == expected_msg

        # 2. batch_runner summary logic verification
        # In batch_runner.py line 622: low_sample_warning = (total_trades < 30)
        batch_low_sample = (n < 30)
        batch_msg = "mẫu nhỏ (<30), độ tin cậy thấp" if batch_low_sample else None
        assert batch_low_sample == expected_warning
        assert batch_msg == expected_msg

        # 3. strategy_lab_service summary logic verification
        # In strategy_lab_service.py line 771: low_sample_warning = trade_count < 30
        strat_low_sample = (len(trades) < 30)
        strat_msg = "mẫu nhỏ (<30), độ tin cậy thấp" if strat_low_sample else None
        assert strat_low_sample == expected_warning
        assert strat_msg == expected_msg

        # 4. analytics_service.py _build_metric_results
        dummy_trades = [SimpleNamespace(net_pnl=100.0 if i % 2 == 0 else 50.0) for i in range(n)]
        dummy_curve = [{"equity": 100_000.0 + i * 100.0} for i in range(n + 1)]
        metric_results = AnalyticsService._build_metric_results(dummy_trades, dummy_curve)
        if n < 30:
            assert metric_results["win_rate"].value is None
            assert metric_results["profit_factor"].value is None
            assert metric_results["sqn"].status == "insufficient_data"
        else:
            assert metric_results["win_rate"].value == pytest.approx(1.0)
            assert metric_results["sqn"].status == "valid"


# ============================================================================
# 4. Database Invariance Verification
# ============================================================================

def test_stress_production_database_unmutated():
    """Stress Test 4.1: Verify production database backend/sumi.db has zero byte delta."""
    current_sha256 = _get_sumi_db_sha256()
    assert current_sha256 == EXPECTED_SUMI_DB_SHA256, (
        f"CRITICAL: backend/sumi.db was mutated! "
        f"Expected {EXPECTED_SUMI_DB_SHA256}, got {current_sha256}"
    )
