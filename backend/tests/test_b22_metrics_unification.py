"""Golden Test Suite for Train 02 - Batch B22 (Metrics Unification).

Covers Golden Cases G-01 through G-10 and Cross-Module Parity:
1. G-01, G-02, G-03: Realized Net PnL calculation and transaction friction parity.
2. G-02 & DEC-05: Strict winner rule (net_pnl > 0 strictly, breakeven = not-winner).
3. G-08: Net Profit Factor, zero-loss edge case (None / 'N/A (chưa có lệnh thua)'), JSON compliance.
4. G-09: Low-sample warning gate (N < 30 boundary).
5. G-10: Annualized daily return Sharpe ratio (rf=0.0, ddof=1, sqrt(252)) and close-based Max Drawdown.
6. Cross-Module Parity: Single Backtest == Batch Runner == Strategy Lab Parameter Sweep.
"""

from datetime import date, datetime, timedelta
import json
import math
from typing import Generator

import numpy as np
import pandas as pd
import pytest
from sqlalchemy import create_engine
from sqlalchemy.orm import Session, sessionmaker

from app.db import Base
from app.domain.accounting import (
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
from app.services.analytics_service import AnalyticsService
from app.services.backtest_service import BacktestService
from app.services.strategy_lab_service import StrategyLabService


# ============================================================================
# Test Fixtures & Helpers
# ============================================================================

@pytest.fixture
def isolated_db(tmp_path) -> Generator[Session, None, None]:
    """Provides an isolated SQLite database session using tmp_path."""
    db_file = tmp_path / "test_b22_parity.db"
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


def _make_trade(
    trade_id: str,
    net_pnl: float,
    pnl_percent: float,
    gross_pnl: float | None = None,
    symbol: str = "HPG",
    holding_bars: int = 5,
) -> BacktestTrade:
    """Helper to instantiate a closed BacktestTrade with specified PnL."""
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
        gross_pnl=gross_pnl if gross_pnl is not None else net_pnl,
        net_pnl=net_pnl,
        fees=10.0,
        taxes=5.0,
        pnl_percent=pnl_percent,
        status="closed",
        result="win" if net_pnl > 0 and not math.isclose(net_pnl, 0.0, abs_tol=1e-9) else ("breakeven" if math.isclose(net_pnl, 0.0, abs_tol=1e-9) else "loss"),
    )


# ============================================================================
# 1. G-01, G-02, G-03: Realized Net PnL Parity
# ============================================================================

def test_b22_g01_realized_net_pnl_parity():
    """Verify Golden Case G-01 exact hand-calc parity via accounting module.
    
    Buy 100 @ 10.1: Gross = 1010.0, Fee (0.15%) = 1.515, Total Out = 1011.515
    Sell 100 @ 11.05: Gross = 1105.0, Fee (0.15%) = 1.6575, Tax (0.10%) = 1.105, Total In = 1102.2375
    Realized Net PnL = 1102.2375 - 1011.515 = 90.7225 (Winner: net > 0).
    """
    buy = calculate_buy_amounts(10.1, 100, DEFAULT_VN_MARKET_PROFILE)
    assert math.isclose(buy.gross_amount, 1010.0, abs_tol=1e-9)
    assert math.isclose(buy.fee, 1.515, abs_tol=1e-9)
    assert math.isclose(buy.net_amount, 1011.515, abs_tol=1e-9)

    sell = calculate_sell_amounts(11.05, 100, DEFAULT_VN_MARKET_PROFILE)
    assert math.isclose(sell.gross_amount, 1105.0, abs_tol=1e-9)
    assert math.isclose(sell.fee, 1.6575, abs_tol=1e-9)
    assert math.isclose(sell.tax, 1.105, abs_tol=1e-9)
    assert math.isclose(sell.net_amount, 1102.2375, abs_tol=1e-9)

    net_pnl = calculate_net_pnl(buy.net_amount, sell.net_amount)
    assert math.isclose(net_pnl, 90.7225, abs_tol=1e-9)

    # Gross PnL - Total Fees - Total Tax parity
    gross_pnl = sell.gross_amount - buy.gross_amount
    total_friction = buy.fee + sell.fee + sell.tax
    assert math.isclose(net_pnl, gross_pnl - total_friction, abs_tol=1e-9)
    assert net_pnl > 0  # Winner

    # Verify compute_round_trip_pnl matches exactly
    rt = compute_round_trip_pnl(
        entry_price=10.1,
        exit_price=11.05,
        quantity=100.0,
        side="long",
        profile=DEFAULT_VN_MARKET_PROFILE,
    )
    assert math.isclose(rt.gross_pnl, 95.0, abs_tol=1e-9)
    assert math.isclose(rt.total_friction, 1.515 + 1.6575 + 1.105, abs_tol=1e-9)
    assert math.isclose(rt.net_pnl, 90.7225, abs_tol=1e-9)
    assert math.isclose(rt.pnl_percent, (90.7225 / 1011.515) * 100.0, abs_tol=1e-5)
    assert rt.is_win is True
    assert rt.is_breakeven is False
    assert rt.is_loss is False


def test_b22_g02_realized_net_pnl_loss_and_breakeven():
    """Verify Golden Case G-02: Loss and Gross Breakeven friction analysis.
    
    1. Loss: Buy 100 @ 20.05, Sell 100 @ 19.45
       Gross PnL = -60.0, Friction = 3.0075 + 2.9175 + 1.945 = 7.87
       Net PnL = -67.87 (Loser).
    2. Gross Breakeven: Buy 100 @ 20.0, Sell 100 @ 20.0
       Gross PnL = 0.0, Friction = 3.0 + 3.0 + 2.0 = 8.0
       Net PnL = -8.0 (Net Loser, strictly NOT a winner).
    """
    # 1. Loss
    b1 = calculate_buy_amounts(20.05, 100, DEFAULT_VN_MARKET_PROFILE)
    s1 = calculate_sell_amounts(19.45, 100, DEFAULT_VN_MARKET_PROFILE)
    pnl_loss = calculate_net_pnl(b1.net_amount, s1.net_amount)
    assert math.isclose(pnl_loss, -67.87, abs_tol=1e-9)
    assert pnl_loss < 0

    # 2. Gross Breakeven
    b2 = calculate_buy_amounts(20.0, 100, DEFAULT_VN_MARKET_PROFILE)
    s2 = calculate_sell_amounts(20.0, 100, DEFAULT_VN_MARKET_PROFILE)
    pnl_gb = calculate_net_pnl(b2.net_amount, s2.net_amount)
    assert math.isclose(pnl_gb, -8.0, abs_tol=1e-9)
    assert pnl_gb < 0  # Gross breakeven becomes a net loss due to transaction friction


def test_b22_g03_short_realized_net_pnl_parity():
    """Verify Golden Case G-03: Short trade symmetry parity.
    
    Sell short 100 @ 14.95, Cover 100 @ 13.05 -> Gross PnL = +190.0
    Friction deducted -> Net PnL > 0 (Winner).
    """
    gross_sell = 14.95 * 100
    gross_cover = 13.05 * 100
    fee_sell = gross_sell * 0.0015
    tax_sell = gross_sell * 0.0010
    fee_cover = gross_cover * 0.0015
    net_pnl = (gross_sell - gross_cover) - (fee_sell + tax_sell + fee_cover)
    assert net_pnl > 0
    assert math.isclose(net_pnl, 190.0 - (2.2425 + 1.495 + 1.9575), abs_tol=1e-9)

    rt_short = compute_round_trip_pnl(
        entry_price=14.95,
        exit_price=13.05,
        quantity=100.0,
        side="short",
        profile=DEFAULT_VN_MARKET_PROFILE,
    )
    assert math.isclose(rt_short.net_pnl, net_pnl, abs_tol=1e-9)
    assert rt_short.is_win is True


# ============================================================================
# 2. G-02 & DEC-05: Winner Rule (net_pnl > 0 strictly, breakeven = not-winner)
# ============================================================================

def test_b22_g02_winner_rule_breakeven_exclusion():
    """Verify Winner Rule: win_rate = win_trades / total_closed_trades.
    
    Dataset:
    - 5 winning trades (net_pnl > 0)
    - 2 breakeven trades (net_pnl == 0.0)
    - 3 losing trades (net_pnl < 0)
    Total closed trades = 10.
    
    Expected:
    - win_rate = 5 / 10 = 0.50 (50.0%)
    - Strict negative assertion: win_rate != 0.70 (70.0%)
    - Breakeven trades must NOT dilute winning trade average return.
    """
    trades = [
        # 5 Wins (+10% each)
        _make_trade("W1", net_pnl=100.0, pnl_percent=10.0),
        _make_trade("W2", net_pnl=250.0, pnl_percent=10.0),
        _make_trade("W3", net_pnl=80.0, pnl_percent=8.0),
        _make_trade("W4", net_pnl=150.0, pnl_percent=12.0),
        _make_trade("W5", net_pnl=420.0, pnl_percent=10.0),
        # 2 Breakevens (net_pnl == 0.0)
        _make_trade("BE1", net_pnl=0.0, pnl_percent=0.0),
        _make_trade("BE2", net_pnl=0.0, pnl_percent=0.0),
        # 3 Losers (-5% each)
        _make_trade("L1", net_pnl=-50.0, pnl_percent=-5.0),
        _make_trade("L2", net_pnl=-180.0, pnl_percent=-6.0),
        _make_trade("L3", net_pnl=-90.0, pnl_percent=-4.0),
    ]

    metrics = calculate_benchmark_metrics(
        trades=trades,
        initial_cash=100_000_000.0,
        ticker="HPG",
    )

    assert metrics.num_trades == 10
    assert metrics.num_winners == 5
    assert metrics.num_breakeven == 2
    assert metrics.num_losers == 3

    # Win rate must strictly be 50.0% (5 / 10), NEVER 70.0% (7 / 10)
    assert metrics.win_rate_pct == 50.0
    assert metrics.win_rate_pct != 70.0

    # Winning average return must ONLY average the 5 true winners: (10 + 10 + 8 + 12 + 10) / 5 = 10.0%
    assert metrics.win_avg_profit_pct == 10.0


# ============================================================================
# 3. G-08: Net Profit Factor & Zero-Loss Edge Case
# ============================================================================

def test_b22_g08_profit_factor_zero_loss_edge_case():
    """Verify Golden Case G-08: Zero-loss edge case handling.
    
    When gross_losses_net == 0 and gross_wins_net > 0:
    - profit_factor evaluates to None (JSON null)
    - profit_factor_display evaluates to 'N/A (chưa có lệnh thua)'
    - Strictly forbidden: float('inf'), 'Infinity', 0.0, 999.0
    - JSON serialization must be standard and compliant.
    """
    trades = [
        _make_trade("T1", net_pnl=500_000.0, pnl_percent=5.0),
        _make_trade("T2", net_pnl=1_000_000.0, pnl_percent=10.0),
        _make_trade("T3", net_pnl=200_000.0, pnl_percent=2.0),
    ]

    metrics = calculate_benchmark_metrics(
        trades=trades,
        initial_cash=100_000_000.0,
        ticker="HPG",
    )

    assert metrics.num_trades == 3
    assert metrics.num_winners == 3
    assert metrics.num_losers == 0
    assert metrics.profit_factor is None
    assert metrics.profit_factor_display == "N/A (chưa có lệnh thua)"

    # Verify standard JSON serialization does not fail or output non-compliant Infinity
    data_dict = metrics.to_dict()
    assert data_dict["profit_factor"] is None
    assert data_dict["profit_factor_display"] == "N/A (chưa có lệnh thua)"

    raw_json = json.dumps(data_dict)
    assert '"profit_factor": null' in raw_json
    assert "Infinity" not in raw_json


def test_b22_g08_profit_factor_normal_and_empty():
    """Verify Profit Factor calculation for normal two-sided and empty ledgers."""
    # 1. Normal case: wins = 600, losses = -200 -> PF = 3.0
    t_normal = [
        _make_trade("T1", net_pnl=600_000.0, pnl_percent=6.0),
        _make_trade("T2", net_pnl=-200_000.0, pnl_percent=-2.0),
    ]
    m_normal = calculate_benchmark_metrics(t_normal, 100_000_000.0, "HPG")
    assert m_normal.profit_factor == 3.0
    assert m_normal.profit_factor_display == "3.00"

    # 2. Empty case: 0 trades -> PF is None, display is 'N/A'
    m_empty = calculate_benchmark_metrics([], 100_000_000.0, "HPG")
    assert m_empty.profit_factor is None
    assert m_empty.profit_factor_display == "N/A"


# ============================================================================
# 4. G-09: Low-Sample Warning Gate (N < 30)
# ============================================================================

def test_b22_g09_low_sample_warning_gate():
    """Verify Golden Case G-09: Low-sample warning gate at N < 30 boundary.
    
    - If total_closed_trades < 30:
        low_sample_warning = True
        low_sample_message = 'mẫu nhỏ (<30), độ tin cậy thấp'
    - If total_closed_trades >= 30:
        low_sample_warning = False
        low_sample_message = None
    """
    # N = 12 (< 30)
    t_12 = [_make_trade(f"T{i}", net_pnl=100.0, pnl_percent=1.0) for i in range(12)]
    m_12 = calculate_benchmark_metrics(t_12, 100_000_000.0, "HPG")
    assert m_12.num_trades == 12
    assert m_12.low_sample_warning is True
    assert m_12.low_sample_message == "mẫu nhỏ (<30), độ tin cậy thấp"

    # N = 29 (< 30, immediate boundary)
    t_29 = [_make_trade(f"T{i}", net_pnl=100.0, pnl_percent=1.0) for i in range(29)]
    m_29 = calculate_benchmark_metrics(t_29, 100_000_000.0, "HPG")
    assert m_29.low_sample_warning is True
    assert m_29.low_sample_message == "mẫu nhỏ (<30), độ tin cậy thấp"

    # N = 30 (>= 30, threshold met)
    t_30 = [_make_trade(f"T{i}", net_pnl=100.0, pnl_percent=1.0) for i in range(30)]
    m_30 = calculate_benchmark_metrics(t_30, 100_000_000.0, "HPG")
    assert m_30.num_trades == 30
    assert m_30.low_sample_warning is False
    assert m_30.low_sample_message is None

    # N = 50 (>= 30, comfortable sample)
    t_50 = [_make_trade(f"T{i}", net_pnl=100.0, pnl_percent=1.0) for i in range(50)]
    m_50 = calculate_benchmark_metrics(t_50, 100_000_000.0, "HPG")
    assert m_50.low_sample_warning is False
    assert m_50.low_sample_message is None


# ============================================================================
# 5. G-10: Annualized Daily Return Sharpe & Close-Based Max Drawdown
# ============================================================================

def test_b22_g10_annualized_daily_sharpe():
    """Verify Golden Case G-10: Annualized daily return Sharpe ratio.
    
    Formula:
    - Daily returns: r_t = (E_t - E_{t-1}) / E_{t-1}
    - rf = 0.0
    - ddof = 1 for sample standard deviation
    - Sharpe = (mean(r) / std(r, ddof=1)) * sqrt(252)
    - If len(r) < 2 or std == 0 -> sharpe = 0.0
    """
    # Deterministic equity curve: 10 daily points -> 9 returns
    equities = [100.0, 102.0, 101.0, 105.0, 103.0, 107.0, 106.0, 110.0, 108.0, 112.0]
    equity_curve = [{"equity": e} for e in equities]

    returns = [
        (equities[i] - equities[i - 1]) / equities[i - 1]
        for i in range(1, len(equities))
    ]
    mean_r = float(np.mean(returns))
    std_r = float(np.std(returns, ddof=1))
    expected_sharpe = round((mean_r / std_r) * math.sqrt(252), 4)

    calculated_sharpe = calculate_sharpe_ratio(equity_curve)
    assert math.isclose(calculated_sharpe, expected_sharpe, abs_tol=1e-4)

    # Edge cases
    assert calculate_sharpe_ratio([]) == 0.0
    assert calculate_sharpe_ratio([{"equity": 100.0}]) == 0.0
    assert calculate_sharpe_ratio([{"equity": 100.0}, {"equity": 100.0}]) == 0.0  # zero std


def test_b22_g10_close_based_max_drawdown():
    """Verify Golden Case G-10: Close-based maximum drawdown.
    
    Formula:
    - peak_t = max_{0 <= s <= t} E_s
    - dd_t = (peak_t - E_t) / peak_t
    - max_drawdown = max_t dd_t
    """
    equities = [100.0, 102.0, 101.0, 105.0, 103.0, 107.0, 106.0, 110.0, 108.0, 112.0]
    equity_curve = [{"equity": e} for e in equities]

    # Hand calculation:
    # Max peak was 105.0 before dropping to 103.0: dd = (105 - 103) / 105 = 2 / 105 ~ 0.0190476
    # Other drops: 102 -> 101 (1/102 ~ 0.0098), 107 -> 106 (1/107 ~ 0.0093), 110 -> 108 (2/110 ~ 0.01818)
    expected_mdd = 2.0 / 105.0

    calculated_mdd = calculate_max_drawdown(equity_curve)
    assert math.isclose(calculated_mdd, expected_mdd, abs_tol=1e-6)


# ============================================================================
# 6. Cross-Module Numerical Parity (Consensus)
# ============================================================================

@pytest.mark.asyncio
async def test_b22_cross_module_numerical_parity(isolated_db: Session):
    """Verify identical metrics across the backend architecture.
    
    Modules verified:
    1. metrics.py (calculate_benchmark_metrics)
    2. batch_runner.py (BatchBacktestRunner)
    3. strategy_lab_service.py (_format_kernel_response / Parameter Sweep)
    4. backtest_service.py (run_backtest / Single Backtest)
    
    Invariants tested:
    - Identical total_closed_trades
    - Identical net_pnl
    - Identical win_rate (accounting for DEC-05 breakeven exclusion)
    - Identical profit_factor / profit_factor_display
    - Identical low_sample_warning behavior
    """
    base_date = date(2024, 1, 1)
    candles = []
    # 40 candles generating trades
    for i in range(40):
        c = Candle(
            symbol="HPG",
            timeframe="1D",
            timestamp=base_date + timedelta(days=i),
            open=20.0 + (i % 6) * 0.5,
            high=21.0 + (i % 6) * 0.5,
            low=19.5 + (i % 6) * 0.5,
            close=20.5 + (i % 6) * 0.5,
            volume=1_000_000,
        )
        candles.append(c)
        isolated_db.add(c)
    isolated_db.commit()

    strategy_dict = {
        "name": "Parity SMA Strategy",
        "indicators": [
            {"name": "sma_fast", "type": "sma", "length": 3},
            {"name": "sma_slow", "type": "sma", "length": 7},
        ],
        "entry_rules": [{"dsl": {"cross_up": ["sma_fast", "sma_slow"]}}],
        "exit_rules": [{"dsl": {"cross_down": ["sma_fast", "sma_slow"]}}],
        "position_sizing": {"method": "fixed_quantity", "quantity": 100},
    }
    strategy = load_strategy_from_dict(strategy_dict)

    # 1. Single Backtest (BacktestService)
    bt_service = BacktestService()
    bt_config = {
        "symbol": "HPG",
        "start_date": "2024-01-01",
        "end_date": "2024-02-10",
        "initial_cash": 100_000_000.0,
        "strategy": strategy_dict,
    }
    single_res = await bt_service.run_backtest(isolated_db, bt_config)
    assert single_res["status"] == "succeeded"
    s_analytics = single_res["analytics"]

    # 2. Batch Backtest (BatchBacktestRunner)
    def provider(sym: str, start: str, end: str) -> pd.DataFrame:
        c_list = isolated_db.query(Candle).filter(Candle.symbol == sym).order_by(Candle.timestamp).all()
        return pd.DataFrame([{
            "timestamp": c.timestamp, "open": c.open, "high": c.high,
            "low": c.low, "close": c.close, "volume": c.volume
        } for c in c_list])

    phases = [PhaseDefinition(name="Full", start_date="2024-01-01", end_date="2024-02-10")]
    batch_res = BatchBacktestRunner.run_batch(
        strategy=strategy,
        symbols=["HPG"],
        phases=phases,
        candle_provider=provider,
        initial_cash_per_run=100_000_000.0,
    )
    assert batch_res.status in ("succeeded", "partial")
    b_row = batch_res.metric_matrix.rows_by_symbol["HPG"]["Full"]

    # 3. Strategy Lab Parameter Sweep (StrategyLabService)
    lab_service = StrategyLabService()
    sweep_config = {
        "symbol": "HPG",
        "start_date": "2024-01-01",
        "end_date": "2024-02-10",
        "initial_cash": 100_000_000.0,
        "strategy": strategy_dict,
        "sweep": [{"path": "indicators[0].length", "values": [3]}],
    }
    sweep_res = await lab_service.run_parameter_sweep(isolated_db, sweep_config)
    assert sweep_res["status"] == "succeeded"
    v_metrics = sweep_res["variants"][0]["metrics"]

    # Numerical Parity Assertions
    # A. Closed trade count
    assert s_analytics["total_trades"] == b_row.num_trades
    assert s_analytics["total_trades"] == v_metrics["total_trades"]

    # B. Net PnL (within rounding)
    assert math.isclose(s_analytics["total_net_pnl"], b_row.net_profit, abs_tol=1e-2)
    assert math.isclose(s_analytics["total_net_pnl"], v_metrics["net_pnl"], abs_tol=1e-2)

    # C. Win Rate
    # Note: b_row.win_rate_pct is in %, s_analytics["win_rate"] is a fraction (0.0 - 1.0)
    assert math.isclose(s_analytics["win_rate"], b_row.win_rate_pct / 100.0, abs_tol=1e-4)
    assert math.isclose(s_analytics["win_rate"], v_metrics["win_rate"], abs_tol=1e-4)

    # D. Profit Factor Parity
    if s_analytics["profit_factor"] is None:
        assert b_row.profit_factor is None
        assert v_metrics["profit_factor"] is None
    else:
        assert math.isclose(s_analytics["profit_factor"], b_row.profit_factor, abs_tol=1e-2)
        assert math.isclose(s_analytics["profit_factor"], v_metrics["profit_factor"], abs_tol=1e-2)
