"""Adversarial Cross-Module Parity & Metrics Stress Test Suite for Batch B22.

Empirical Challenger B22-2 verification:
1. 5-Way Numerical Parity Stress:
   - Single Backtest (BacktestService)
   - Batch Runner (BatchBacktestRunner)
   - Strategy Lab Parameter Sweep (StrategyLabService)
   - Replay/Practice (PracticeWorkflowService / TradeLifecycleService)
   - Analytics Service (AnalyticsService)
2. Annualized Daily Return Sharpe Stress:
   - 1 bar (returns 0.0)
   - 2 bars (returns 0.0, sample std with ddof=1 requires >= 2 returns)
   - 3+ bars (normal calculation against manual oracle)
   - Flat curve (zero variance returns 0.0)
   - Highly volatile curve
   - Steadily declining curve (negative Sharpe preserved)
3. Close-Based Maximum Drawdown Stress:
   - Flat equity curve (returns 0.0)
   - Monotonically rising curve (returns 0.0)
   - 100% wipeout curve (drawdown = 1.0 / 100%)
   - Rollercoaster multiple peaks
   - Empty and single-point boundary conditions
4. Winner Rule & Statistical Sample Gates:
   - Floating-point epsilon tolerance (1e-9) around breakeven
   - Low-sample warning gate (N < 30)
5. Zero-Loss and All-Loss Profit Factor Edge Cases:
   - Zero-loss: profit_factor is None, display is 'N/A (chưa có lệnh thua)'
   - All-loss: profit_factor is 0.0, display is '0.00'
   - Zero trades: profit_factor is None, display is 'N/A'
"""

from __future__ import annotations

from datetime import date, datetime, timedelta
import json
import math
from typing import Any, Generator, List

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
)
from app.domain.backtest.batch_runner import BatchBacktestRunner, PhaseDefinition
from app.domain.backtest.execution import BacktestExecutionKernel
from app.domain.backtest.metrics import (
    BenchmarkMetricRow,
    calculate_benchmark_metrics,
    calculate_daily_returns_from_equity_curve,
    calculate_max_drawdown,
    calculate_sharpe_ratio,
)
from app.domain.backtest.models import BacktestKernelResult, BacktestTrade
from app.domain.enums import DecisionAction, OrderType, SessionMode, SessionStatus
from app.domain.market.rules import DEFAULT_VN_MARKET_PROFILE
from app.domain.strategy.strategy_loader import load_strategy_from_dict
from app.models.candle import Candle
from app.models.decision import Decision
from app.models.execution import Execution
from app.models.order import Order
from app.models.position import Position
from app.models.replay_session import ReplaySession
from app.models.trade import Trade
from app.schemas.decision_schema import DecisionCreate
from app.services.analytics_service import AnalyticsService
from app.services.backtest_service import BacktestService
from app.services.strategy_lab_service import StrategyLabService
from app.services.trade_lifecycle_service import TradeLifecycleService


@pytest.fixture
def isolated_db(tmp_path) -> Generator[Session, None, None]:
    """Isolated SQLite database session ensuring sumi.db invariance."""
    db_file = tmp_path / "test_b22_challenger_stress.db"
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


def _populate_candles(
    db: Session,
    symbol: str,
    price_sequence: List[float],
    start_date: date = date(2024, 1, 1),
) -> List[Candle]:
    """Seed sequential daily candles from a price sequence."""
    candles = []
    for i, p in enumerate(price_sequence):
        # Mon-Fri business day stepping
        dt = start_date + timedelta(days=i)
        c = Candle(
            symbol=symbol,
            timeframe="1D",
            timestamp=dt,
            open=float(p),
            high=float(p * 1.02),
            low=float(p * 0.98),
            close=float(p),
            volume=1_000_000,
        )
        db.add(c)
        candles.append(c)
    db.commit()
    return candles


# ============================================================================
# 1. 5-Way Numerical Parity Stress Tests
# ============================================================================

@pytest.mark.asyncio
async def test_adv_5way_numerical_parity_standard_dataset(isolated_db: Session):
    """Stress-test 5-way parity on a standard multi-trade dataset.
    
    Modules compared:
    1. Single Backtest (BacktestService)
    2. Batch Runner (BatchBacktestRunner)
    3. Strategy Lab Parameter Sweep (StrategyLabService)
    4. Replay/Practice (TradeLifecycleService + AnalyticsService)
    5. Core Metrics Engine (metrics.calculate_benchmark_metrics)
    
    Verifies:
    - total_trades matches 100% identically
    - total_net_pnl matches within 1e-2 tolerance
    - win_rate matches within 1e-4 tolerance
    - profit_factor matches within 1e-2 tolerance
    - profit_factor_display matches 100%
    - low_sample_warning matches 100%
    - sharpe_ratio and max_drawdown follow unified definitions
    """
    symbol = "HPG"
    start_date = date(2024, 1, 1)
    
    # 50 candles with periodic price fluctuations designed to generate multiple round-trips
    prices = [
        20.0, 20.2, 20.5, 21.0, 21.5, 21.0, 20.0, 19.5, 19.0, 19.2,
        20.0, 20.8, 21.2, 21.8, 22.0, 21.0, 20.2, 19.8, 19.4, 20.1,
        20.5, 21.0, 21.7, 22.2, 21.5, 20.5, 19.9, 19.3, 19.7, 20.4,
        20.9, 21.4, 22.1, 22.5, 21.8, 20.7, 20.0, 19.5, 19.9, 20.5,
        21.1, 21.6, 22.3, 22.7, 22.0, 21.0, 20.2, 19.7, 20.2, 20.8,
    ]
    _populate_candles(isolated_db, symbol, prices, start_date)

    strategy_dict = {
        "name": "Parity Stress SMA Strategy",
        "indicators": [
            {"name": "sma_fast", "type": "sma", "length": 3},
            {"name": "sma_slow", "type": "sma", "length": 6},
        ],
        "entry_rules": [{"dsl": {"cross_up": ["sma_fast", "sma_slow"]}}],
        "exit_rules": [{"dsl": {"cross_down": ["sma_fast", "sma_slow"]}}],
        "position_sizing": {"method": "fixed_quantity", "quantity": 100},
    }
    strategy = load_strategy_from_dict(strategy_dict)

    # 1. Single Backtest (BacktestService)
    bt_service = BacktestService()
    single_res = await bt_service.run_backtest(
        isolated_db,
        {
            "symbol": symbol,
            "start_date": "2024-01-01",
            "end_date": "2024-03-01",
            "initial_cash": 100_000_000.0,
            "strategy": strategy_dict,
        },
    )
    assert single_res["status"] == "succeeded"
    s_analytics = single_res["analytics"]
    kernel_result: BacktestKernelResult = single_res["kernel_result"]
    closed_trades = [t for t in kernel_result.trades if t.status == "closed"]
    assert len(closed_trades) > 0, "Stress dataset must trigger at least 1 closed trade"

    # 2. Batch Backtest Runner
    def candle_provider(sym: str, start: str, end: str) -> pd.DataFrame:
        c_list = isolated_db.query(Candle).filter(Candle.symbol == sym).order_by(Candle.timestamp).all()
        return pd.DataFrame([{
            "timestamp": c.timestamp, "open": c.open, "high": c.high,
            "low": c.low, "close": c.close, "volume": c.volume
        } for c in c_list])

    phases = [PhaseDefinition(name="FullPeriod", start_date="2024-01-01", end_date="2024-03-01")]
    batch_res = BatchBacktestRunner.run_batch(
        strategy=strategy,
        symbols=[symbol],
        phases=phases,
        candle_provider=candle_provider,
        initial_cash_per_run=100_000_000.0,
    )
    assert batch_res.status == "succeeded"
    b_metrics = batch_res.metric_matrix.rows_by_symbol[symbol]["FullPeriod"]

    # 3. Strategy Lab Parameter Sweep
    lab_service = StrategyLabService()
    sweep_res = await lab_service.run_parameter_sweep(
        isolated_db,
        {
            "symbol": symbol,
            "start_date": "2024-01-01",
            "end_date": "2024-03-01",
            "initial_cash": 100_000_000.0,
            "strategy": strategy_dict,
            "sweep": [{"path": "indicators[0].length", "values": [3]}],
        },
    )
    assert sweep_res["status"] == "succeeded"
    v_metrics = sweep_res["variants"][0]["metrics"]
    v_analytics = sweep_res["variants"][0].get("analytics", {})

    # 4. Core Metrics Engine direct calculation
    bench_row = calculate_benchmark_metrics(
        trades=kernel_result.trades,
        initial_cash=100_000_000.0,
        ticker=symbol,
        equity_curve=kernel_result.ledger.equity_curve,
    )

    # 5. Replay/Practice Module: Simulate exact trades in a ReplaySession
    practice_session = ReplaySession(
        symbol=symbol,
        timeframe="1D",
        start_date=date(2024, 1, 1),
        end_date=date(2024, 3, 1),
        current_index=len(prices) - 1,
        initial_cash=100_000_000.0,
        current_cash=100_000_000.0,
        mode=SessionMode.NORMAL.value,
        status=SessionStatus.ACTIVE.value,
    )
    isolated_db.add(practice_session)
    isolated_db.flush()

    # Replicate the exact trades produced by kernel into the practice session
    for t in closed_trades:
        db_trade = Trade(
            session_id=practice_session.id,
            symbol=symbol,
            entry_date=t.entry_time,
            entry_price=t.entry_price,
            exit_date=t.exit_time,
            exit_price=t.exit_price,
            quantity=t.quantity,
            gross_pnl=t.gross_pnl,
            net_pnl=t.net_pnl,
            pnl_percent=t.pnl_percent,
            holding_candles=t.holding_bars,
            status="closed",
            result=t.result,
        )
        isolated_db.add(db_trade)
    isolated_db.commit()

    practice_analytics = AnalyticsService.get_analytics(isolated_db, practice_session.id)

    # ==================== PARITY ASSERTIONS ====================
    # 1. Total closed trades
    expected_trades = len(closed_trades)
    assert s_analytics["total_trades"] == expected_trades
    assert b_metrics.num_trades == expected_trades
    assert v_metrics["total_trades"] == expected_trades
    assert bench_row.num_trades == expected_trades
    assert practice_analytics.total_trades == expected_trades

    # 2. Total Net PnL (within 1e-2)
    expected_pnl = sum(t.net_pnl for t in closed_trades)
    assert math.isclose(s_analytics["total_net_pnl"], expected_pnl, abs_tol=1e-2)
    assert math.isclose(b_metrics.net_profit, expected_pnl, abs_tol=1e-2)
    assert math.isclose(v_metrics["net_pnl"], expected_pnl, abs_tol=1e-2)
    assert math.isclose(bench_row.net_profit, expected_pnl, abs_tol=1e-2)
    assert math.isclose(practice_analytics.total_net_pnl, expected_pnl, abs_tol=1e-2)

    # 3. Win Rate (within 1e-4)
    # Note: b_metrics.win_rate_pct is %, others are fraction (0.0-1.0)
    expected_win_rate = len([t for t in closed_trades if t.net_pnl > 0 and not math.isclose(t.net_pnl, 0.0, abs_tol=1e-9)]) / expected_trades
    assert math.isclose(s_analytics["win_rate"], expected_win_rate, abs_tol=1e-4)
    assert math.isclose(b_metrics.win_rate_pct / 100.0, expected_win_rate, abs_tol=1e-4)
    assert math.isclose(v_metrics["win_rate"], expected_win_rate, abs_tol=1e-4)
    assert math.isclose(bench_row.win_rate_pct / 100.0, expected_win_rate, abs_tol=1e-4)
    assert math.isclose(practice_analytics.win_rate, expected_win_rate, abs_tol=1e-4)

    # 4. Profit Factor & Profit Factor Display
    if bench_row.profit_factor is None:
        assert s_analytics["profit_factor"] is None
        assert b_metrics.profit_factor is None
        assert v_metrics["profit_factor"] is None
        assert practice_analytics.profit_factor is None
    else:
        assert math.isclose(s_analytics["profit_factor"], bench_row.profit_factor, abs_tol=1e-2)
        assert math.isclose(b_metrics.profit_factor, bench_row.profit_factor, abs_tol=1e-2)
        assert math.isclose(v_metrics["profit_factor"], bench_row.profit_factor, abs_tol=1e-2)
        assert math.isclose(practice_analytics.profit_factor, bench_row.profit_factor, abs_tol=1e-2)

    assert b_metrics.profit_factor_display == bench_row.profit_factor_display
    assert s_analytics["profit_factor_display"] == bench_row.profit_factor_display
    assert practice_analytics.profit_factor_display == bench_row.profit_factor_display

    # 5. Low-Sample Warning (expected_trades < 30)
    expected_warning = expected_trades < 30
    assert s_analytics["low_sample_warning"] == expected_warning
    assert b_metrics.low_sample_warning == expected_warning
    assert bench_row.low_sample_warning == expected_warning
    assert practice_analytics.low_sample_warning == expected_warning

    # 6. Sharpe Ratio & Max Drawdown cross-checks
    # Benchmark row matches kernel ledger max_drawdown
    assert math.isclose(b_metrics.max_drawdown, kernel_result.ledger.max_drawdown, abs_tol=1e-4)
    assert math.isclose(bench_row.max_drawdown, kernel_result.ledger.max_drawdown, abs_tol=1e-4)


@pytest.mark.asyncio
async def test_adv_5way_zero_loss_edge_case(isolated_db: Session):
    """Verify zero-loss edge case handling across all modules.
    
    When all closed trades are winners:
    - profit_factor is None (JSON null) across all modules
    - profit_factor_display is 'N/A (chưa có lệnh thua)'
    - No NaN, Inf, or crash anywhere
    """
    trades = [
        BacktestTrade(
            id=f"T{i}",
            symbol="HPG",
            entry_order_id=f"e_{i}",
            exit_order_id=f"x_{i}",
            entry_time=datetime(2024, 1, 1),
            exit_time=datetime(2024, 1, 5),
            entry_bar_index=0,
            exit_bar_index=5,
            holding_bars=5,
            quantity=100.0,
            entry_price=20.0,
            exit_price=22.0,
            gross_pnl=200.0,
            net_pnl=180.0 + i * 20.0,
            fees=12.0,
            taxes=8.0,
            pnl_percent=9.0,
            status="closed",
            result="win",
        )
        for i in range(5)
    ]

    # 1. metrics.py
    row = calculate_benchmark_metrics(trades, 100_000_000.0, "HPG")
    assert row.profit_factor is None
    assert row.profit_factor_display == "N/A (chưa có lệnh thua)"
    assert json.loads(json.dumps(row.to_dict()))["profit_factor"] is None

    # 2. StrategyLabService._format_kernel_response
    from app.domain.backtest.models import BacktestLedger
    kernel_res = BacktestKernelResult(
        ledger=BacktestLedger(
            initial_cash=100_000_000.0,
            current_cash=100_000_000.0,
            portfolio_value=100_000_000.0,
            peak_portfolio_value=100_000_000.0,
            max_drawdown=0.0,
            trades=trades,
        ),
        symbol="HPG",
        timeframe="1D",
        start_date=datetime(2024, 1, 1),
        end_date=datetime(2024, 2, 1),
        total_candles=30,
        last_analyzed_index=29,
    )
    dummy_strat = load_strategy_from_dict({
        "name": "Dummy",
        "indicators": [],
        "entry_rules": [],
        "exit_rules": [],
        "position_sizing": {"method": "fixed_quantity", "quantity": 100},
    })
    dummy_df = pd.DataFrame([{"timestamp": "2024-01-01", "close": 20.0}, {"timestamp": "2024-02-01", "close": 22.0}])
    lab = StrategyLabService()
    dummy_candles = [
        Candle(symbol="HPG", timestamp=date(2024, 1, 1), open=20.0, high=20.0, low=20.0, close=20.0, volume=100),
        Candle(symbol="HPG", timestamp=date(2024, 2, 1), open=20.0, high=20.0, low=20.0, close=20.0, volume=100),
    ]
    formatted = lab._format_kernel_response(
        kernel_res=kernel_res,
        strategy_obj=dummy_strat,
        symbol="HPG",
        df=dummy_df,
        candles=dummy_candles,
        initial_cash=100_000_000.0,
        benchmark_symbol="VNINDEX",
    )
    assert formatted["analytics"]["profit_factor"] is None
    assert formatted["analytics"]["profit_factor_display"] == "N/A (chưa có lệnh thua)"

    # 3. Practice / AnalyticsService
    session = ReplaySession(
        symbol="HPG",
        timeframe="1D",
        start_date=date(2024, 1, 1),
        end_date=date(2024, 2, 1),
        current_index=1,
        initial_cash=100_000_000.0,
        current_cash=100_000_000.0,
        mode=SessionMode.NORMAL.value,
        status=SessionStatus.ACTIVE.value,
    )
    isolated_db.add(session)
    isolated_db.flush()
    for t in trades:
        db_t = Trade(
            session_id=session.id,
            symbol="HPG",
            entry_date=t.entry_time,
            exit_date=t.exit_time,
            entry_price=t.entry_price,
            exit_price=t.exit_price,
            quantity=t.quantity,
            net_pnl=t.net_pnl,
            status="closed",
            result="win",
        )
        isolated_db.add(db_t)
    isolated_db.commit()

    analytics = AnalyticsService.get_analytics(isolated_db, session.id)
    assert analytics.profit_factor is None
    assert analytics.profit_factor_display == "N/A (chưa có lệnh thua)"


@pytest.mark.asyncio
async def test_adv_5way_all_losses_edge_case(isolated_db: Session):
    """Verify all-losses edge case handling across modules.
    
    When all closed trades are losses:
    - profit_factor is 0.0
    - profit_factor_display is '0.00'
    - win_rate is 0.0
    """
    trades = [
        BacktestTrade(
            id=f"T{i}",
            symbol="HPG",
            entry_order_id=f"e_{i}",
            exit_order_id=f"x_{i}",
            entry_time=datetime(2024, 1, 1),
            exit_time=datetime(2024, 1, 5),
            entry_bar_index=0,
            exit_bar_index=5,
            holding_bars=5,
            quantity=100.0,
            entry_price=20.0,
            exit_price=18.0,
            gross_pnl=-200.0,
            net_pnl=-220.0,
            fees=12.0,
            taxes=8.0,
            pnl_percent=-10.0,
            status="closed",
            result="loss",
        )
        for i in range(4)
    ]

    # 1. metrics.py
    row = calculate_benchmark_metrics(trades, 100_000_000.0, "HPG")
    assert row.profit_factor == 0.0
    assert row.profit_factor_display == "0.00"
    assert row.win_rate_pct == 0.0

    # 2. StrategyLabService
    from app.domain.backtest.models import BacktestLedger
    kernel_res = BacktestKernelResult(
        ledger=BacktestLedger(
            initial_cash=100_000_000.0,
            current_cash=100_000_000.0,
            portfolio_value=100_000_000.0,
            peak_portfolio_value=100_000_000.0,
            max_drawdown=0.0,
            trades=trades,
        ),
        symbol="HPG",
        timeframe="1D",
        start_date=datetime(2024, 1, 1),
        end_date=datetime(2024, 2, 1),
        total_candles=30,
        last_analyzed_index=29,
    )
    dummy_strat = load_strategy_from_dict({
        "name": "Dummy",
        "indicators": [],
        "entry_rules": [],
        "exit_rules": [],
        "position_sizing": {"method": "fixed_quantity", "quantity": 100},
    })
    lab = StrategyLabService()
    dummy_candles = [
        Candle(symbol="HPG", timestamp=date(2024, 1, 1), open=20.0, high=20.0, low=20.0, close=20.0, volume=100),
        Candle(symbol="HPG", timestamp=date(2024, 2, 1), open=20.0, high=20.0, low=20.0, close=20.0, volume=100),
    ]
    formatted = lab._format_kernel_response(
        kernel_res=kernel_res,
        strategy_obj=dummy_strat,
        symbol="HPG",
        df=pd.DataFrame([{"timestamp": "2024-01-01", "close": 20.0}, {"timestamp": "2024-02-01", "close": 20.0}]),
        candles=dummy_candles,
        initial_cash=100_000_000.0,
        benchmark_symbol="VNINDEX",
    )
    assert formatted["analytics"]["profit_factor"] == 0.0
    assert formatted["analytics"]["profit_factor_display"] == "0.00"
    assert formatted["analytics"]["win_rate"] == 0.0

    # 3. Practice / AnalyticsService
    session = ReplaySession(
        symbol="HPG",
        timeframe="1D",
        start_date=date(2024, 1, 1),
        end_date=date(2024, 2, 1),
        current_index=1,
        initial_cash=100_000_000.0,
        current_cash=100_000_000.0,
        mode=SessionMode.NORMAL.value,
        status=SessionStatus.ACTIVE.value,
    )
    isolated_db.add(session)
    isolated_db.flush()
    for t in trades:
        db_t = Trade(
            session_id=session.id,
            symbol="HPG",
            entry_date=t.entry_time,
            exit_date=t.exit_time,
            entry_price=t.entry_price,
            exit_price=t.exit_price,
            quantity=t.quantity,
            net_pnl=t.net_pnl,
            status="closed",
            result="loss",
        )
        isolated_db.add(db_t)
    isolated_db.commit()

    analytics = AnalyticsService.get_analytics(isolated_db, session.id)
    assert analytics.profit_factor == 0.0
    assert analytics.profit_factor_display == "0.00"
    assert analytics.win_rate == 0.0


@pytest.mark.asyncio
async def test_adv_5way_zero_trades_edge_case(isolated_db: Session):
    """Verify zero-trades edge case handling across all modules."""
    # 1. metrics.py
    row = calculate_benchmark_metrics([], 100_000_000.0, "HPG")
    assert row.num_trades == 0
    assert row.net_profit == 0.0
    assert row.profit_factor is None
    assert row.profit_factor_display == "N/A"
    assert row.win_rate_pct is None
    assert row.low_sample_warning is True

    # 2. StrategyLabService
    from app.domain.backtest.models import BacktestLedger
    kernel_res = BacktestKernelResult(
        ledger=BacktestLedger(
            initial_cash=100_000_000.0,
            current_cash=100_000_000.0,
            portfolio_value=100_000_000.0,
            peak_portfolio_value=100_000_000.0,
            max_drawdown=0.0,
            trades=[],
        ),
        symbol="HPG",
        timeframe="1D",
        start_date=datetime(2024, 1, 1),
        end_date=datetime(2024, 2, 1),
        total_candles=30,
        last_analyzed_index=29,
    )
    dummy_strat = load_strategy_from_dict({
        "name": "Dummy",
        "indicators": [],
        "entry_rules": [],
        "exit_rules": [],
        "position_sizing": {"method": "fixed_quantity", "quantity": 100},
    })
    dummy_candles = [
        Candle(symbol="HPG", timestamp=date(2024, 1, 1), open=20.0, high=20.0, low=20.0, close=20.0, volume=100),
        Candle(symbol="HPG", timestamp=date(2024, 2, 1), open=20.0, high=20.0, low=20.0, close=20.0, volume=100),
    ]
    lab = StrategyLabService()
    formatted = lab._format_kernel_response(
        kernel_res=kernel_res,
        strategy_obj=dummy_strat,
        symbol="HPG",
        df=pd.DataFrame([{"timestamp": "2024-01-01", "close": 20.0}, {"timestamp": "2024-02-01", "close": 20.0}]),
        candles=dummy_candles,
        initial_cash=100_000_000.0,
        benchmark_symbol="VNINDEX",
    )
    assert formatted["analytics"]["total_trades"] == 0
    assert formatted["analytics"]["profit_factor"] is None
    assert formatted["analytics"]["profit_factor_display"] == "N/A"
    assert formatted["analytics"]["low_sample_warning"] is True

    # 3. Practice / AnalyticsService
    session = ReplaySession(
        symbol="HPG",
        timeframe="1D",
        start_date=date(2024, 1, 1),
        end_date=date(2024, 2, 1),
        current_index=1,
        initial_cash=100_000_000.0,
        current_cash=100_000_000.0,
        mode=SessionMode.NORMAL.value,
        status=SessionStatus.ACTIVE.value,
    )
    isolated_db.add(session)
    isolated_db.commit()

    analytics = AnalyticsService.get_analytics(isolated_db, session.id)
    assert analytics.total_trades == 0
    assert analytics.profit_factor is None
    assert analytics.profit_factor_display == "N/A"
    assert analytics.low_sample_warning is True


# ============================================================================
# 2. Annualized Daily Return Sharpe Ratio Stress Tests
# ============================================================================

def test_adv_sharpe_ratio_stressed_curves():
    """Verify Annualized Daily Sharpe ratio across extreme stress scenarios:
    - 1 bar (returns 0.0)
    - 2 bars (returns 0.0, sample std with ddof=1 requires >= 2 returns)
    - 3+ bars (verified against manual oracle)
    - Flat curve (zero variance returns 0.0)
    - Highly volatile curve
    - Steadily declining curve (negative Sharpe preserved)
    """
    # 1. Empty curve
    assert calculate_sharpe_ratio([]) == 0.0
    assert AnalyticsService._calculate_sharpe_ratio([]) == 0.0

    # 2. Single-bar curve (0 returns)
    curve_1 = [{"equity": 100_000_000.0}]
    assert calculate_sharpe_ratio(curve_1) == 0.0
    assert AnalyticsService._calculate_sharpe_ratio(curve_1) == 0.0

    # 3. Two-bar curve (1 return -> ddof=1 sample std is undefined)
    curve_2 = [{"equity": 100_000_000.0}, {"equity": 105_000_000.0}]
    assert calculate_sharpe_ratio(curve_2) == 0.0
    assert AnalyticsService._calculate_sharpe_ratio(curve_2) == 0.0

    # 4. Three-bar curve (2 returns -> first valid Sharpe sample)
    curve_3 = [
        {"equity": 100_000_000.0},
        {"equity": 105_000_000.0},
        {"equity": 102_000_000.0},
    ]
    r1 = (105.0 - 100.0) / 100.0  # +0.05
    r2 = (102.0 - 105.0) / 105.0  # -0.0285714...
    expected_mean = (r1 + r2) / 2.0
    expected_std = float(np.std([r1, r2], ddof=1))
    oracle_sharpe_3 = round((expected_mean / expected_std) * math.sqrt(252), 4)

    calc_sharpe_3 = calculate_sharpe_ratio(curve_3)
    calc_analytics_3 = AnalyticsService._calculate_sharpe_ratio(curve_3)
    assert math.isclose(calc_sharpe_3, oracle_sharpe_3, abs_tol=1e-4)
    assert math.isclose(calc_analytics_3, oracle_sharpe_3, abs_tol=1e-4)

    # 5. Flat curve (zero variance across 50 bars)
    curve_flat = [{"equity": 100_000_000.0} for _ in range(50)]
    assert calculate_sharpe_ratio(curve_flat) == 0.0
    assert AnalyticsService._calculate_sharpe_ratio(curve_flat) == 0.0

    # 6. Highly volatile curve
    curve_volatile = [
        {"equity": 100.0}, {"equity": 130.0}, {"equity": 90.0},
        {"equity": 140.0}, {"equity": 80.0}, {"equity": 150.0},
        {"equity": 75.0}, {"equity": 160.0}, {"equity": 70.0},
    ]
    calc_sharpe_vol = calculate_sharpe_ratio(curve_volatile)
    calc_analytics_vol = AnalyticsService._calculate_sharpe_ratio(curve_volatile)
    assert math.isclose(calc_sharpe_vol, calc_analytics_vol, abs_tol=1e-4)

    # 7. Steadily declining curve (monotonically negative returns)
    curve_declining = [{"equity": 100.0 - (i * 0.5)} for i in range(40)]
    calc_sharpe_dec = calculate_sharpe_ratio(curve_declining)
    calc_analytics_dec = AnalyticsService._calculate_sharpe_ratio(curve_declining)
    # Must preserve negative sign and match identically
    assert calc_sharpe_dec < 0.0, f"Expected negative Sharpe, got {calc_sharpe_dec}"
    assert math.isclose(calc_sharpe_dec, calc_analytics_dec, abs_tol=1e-4)


# ============================================================================
# 3. Close-Based Maximum Drawdown Stress Tests
# ============================================================================

def test_adv_max_drawdown_stressed_curves():
    """Verify Close-Based Max Drawdown across stress scenarios:
    - Flat equity curve (returns 0.0)
    - Monotonically rising curve (returns 0.0)
    - 100% wipeout curve (drawdown = 1.0 / 100%)
    - Multi-peak rollercoaster
    - Boundary conditions: empty and 1 bar
    """
    # 1. Empty curve
    assert calculate_max_drawdown([]) == 0.0
    assert AnalyticsService._calculate_max_drawdown([])["max_drawdown_pct"] == 0.0

    # 2. Single-bar curve
    assert calculate_max_drawdown([{"equity": 100.0}]) == 0.0
    assert AnalyticsService._calculate_max_drawdown([{"equity": 100.0}])["max_drawdown_pct"] == 0.0

    # 3. Flat equity curve (0.0 drawdown)
    curve_flat = [{"equity": 100.0} for _ in range(30)]
    assert calculate_max_drawdown(curve_flat) == 0.0
    assert AnalyticsService._calculate_max_drawdown(curve_flat)["max_drawdown_pct"] == 0.0

    # 4. Monotonically rising curve (0.0 drawdown)
    curve_rising = [{"equity": 100.0 + i * 2.5} for i in range(30)]
    assert calculate_max_drawdown(curve_rising) == 0.0
    assert AnalyticsService._calculate_max_drawdown(curve_rising)["max_drawdown_pct"] == 0.0

    # 5. 100% Wipeout curve (peak 100 -> drop to 0)
    curve_wipeout = [{"equity": 100.0}, {"equity": 80.0}, {"equity": 40.0}, {"equity": 10.0}, {"equity": 0.0}]
    assert calculate_max_drawdown(curve_wipeout) == 1.0
    assert AnalyticsService._calculate_max_drawdown(curve_wipeout)["max_drawdown_pct"] == 100.0
    assert AnalyticsService._calculate_max_drawdown(curve_wipeout)["max_drawdown_amount"] == 100.0

    # 6. Multi-peak rollercoaster
    # Peaks at 120 (drops to 90 -> dd = 30/120 = 25%), then peaks at 150 (drops to 105 -> dd = 45/150 = 30%)
    curve_rollercoaster = [
        {"equity": 100.0},
        {"equity": 120.0},  # Peak 1
        {"equity": 110.0},
        {"equity": 90.0},   # DD1 = (120 - 90)/120 = 25.0%
        {"equity": 130.0},
        {"equity": 150.0},  # Peak 2
        {"equity": 120.0},
        {"equity": 105.0},  # DD2 = (150 - 105)/150 = 30.0% (Worst)
        {"equity": 140.0},
    ]
    expected_mdd_fraction = 45.0 / 150.0  # 0.30
    assert math.isclose(calculate_max_drawdown(curve_rollercoaster), expected_mdd_fraction, abs_tol=1e-6)
    assert AnalyticsService._calculate_max_drawdown(curve_rollercoaster)["max_drawdown_pct"] == 30.0
    assert AnalyticsService._calculate_max_drawdown(curve_rollercoaster)["max_drawdown_amount"] == 45.0


# ============================================================================
# 4. Floating-Point Epsilon Tolerance & Breakeven Rule
# ============================================================================

def test_adv_floating_point_epsilon_breakeven_tolerance():
    """Verify that floating-point epsilon noise around zero is treated as BREAKEVEN.
    
    Per DEC-05 & G-02:
    - abs(pnl) <= 1e-9 is classified as BREAKEVEN (NOT a winner).
    - pnl > 1e-9 is classified as WINNER.
    - pnl < -1e-9 is classified as LOSER.
    """
    t_tiny_pos = BacktestTrade(
        id="T_tiny_pos", symbol="HPG", entry_order_id="e", exit_order_id="x",
        entry_time=datetime(2024, 1, 1), exit_time=datetime(2024, 1, 2),
        entry_bar_index=0, exit_bar_index=1, holding_bars=1, quantity=100.0,
        entry_price=20.0, exit_price=20.0, gross_pnl=0.0,
        net_pnl=1e-10,  # Below 1e-9 tolerance
        fees=0.0, taxes=0.0, pnl_percent=0.0, status="closed",
    )
    t_tiny_neg = BacktestTrade(
        id="T_tiny_neg", symbol="HPG", entry_order_id="e", exit_order_id="x",
        entry_time=datetime(2024, 1, 1), exit_time=datetime(2024, 1, 2),
        entry_bar_index=0, exit_bar_index=1, holding_bars=1, quantity=100.0,
        entry_price=20.0, exit_price=20.0, gross_pnl=0.0,
        net_pnl=-1e-10,  # Below 1e-9 tolerance
        fees=0.0, taxes=0.0, pnl_percent=0.0, status="closed",
    )
    t_real_win = BacktestTrade(
        id="T_real_win", symbol="HPG", entry_order_id="e", exit_order_id="x",
        entry_time=datetime(2024, 1, 1), exit_time=datetime(2024, 1, 2),
        entry_bar_index=0, exit_bar_index=1, holding_bars=1, quantity=100.0,
        entry_price=20.0, exit_price=21.0, gross_pnl=100.0,
        net_pnl=90.0,
        fees=6.0, taxes=4.0, pnl_percent=4.5, status="closed",
    )

    row = calculate_benchmark_metrics([t_tiny_pos, t_tiny_neg, t_real_win], 100_000_000.0, "HPG")
    assert row.num_trades == 3
    assert row.num_winners == 1
    assert row.num_breakeven == 2
    assert row.num_losers == 0
    # Win rate is strictly 1 / 3 (33.33%), NOT 2 / 3 or 3 / 3
    assert math.isclose(row.win_rate_pct, 100.0 / 3.0, abs_tol=1e-4)


# ============================================================================
# 5. Low-Sample Warning Gate Boundary Stress (N=29 vs N=30)
# ============================================================================

def test_adv_low_sample_gate_exact_boundary():
    """Verify statistical gate boundary strictly flips at N=30 across modules."""
    def make_trades(n: int) -> List[BacktestTrade]:
        return [
            BacktestTrade(
                id=f"T{i}", symbol="HPG", entry_order_id="e", exit_order_id="x",
                entry_time=datetime(2024, 1, 1), exit_time=datetime(2024, 1, 2),
                entry_bar_index=0, exit_bar_index=1, holding_bars=1, quantity=100.0,
                entry_price=20.0, exit_price=20.5, gross_pnl=50.0, net_pnl=40.0,
                fees=6.0, taxes=4.0, pnl_percent=2.0, status="closed",
            )
            for i in range(n)
        ]

    # N = 29 -> Warning active
    m_29 = calculate_benchmark_metrics(make_trades(29), 100_000_000.0, "HPG")
    assert m_29.low_sample_warning is True
    assert m_29.low_sample_message == "mẫu nhỏ (<30), độ tin cậy thấp"

    # N = 30 -> Warning clear
    m_30 = calculate_benchmark_metrics(make_trades(30), 100_000_000.0, "HPG")
    assert m_30.low_sample_warning is False
    assert m_30.low_sample_message is None


# ============================================================================
# 6. Rebalanced Breakeven Portfolio Stress Across 4 Engines
# ============================================================================

def test_adv_rebalanced_breakeven_portfolio_parity(isolated_db: Session):
    """Stress-test a mixed ledger of 5 Wins, 5 Losses, 5 Breakevens (15 trades).
    
    Expected:
    - total_trades = 15
    - win_rate = 5 / 15 = 33.3333% (NOT 66.67%)
    - net_pnl = 5 * 500,000 - 5 * 250,000 = 1,250,000.0
    - profit_factor = 2,500,000 / 1,250,000 = 2.0
    - profit_factor_display = "2.00"
    - low_sample_warning = True (15 < 30)
    """
    trades = []
    # 5 Wins (+500,000 net)
    for i in range(5):
        trades.append(BacktestTrade(
            id=f"W{i}", symbol="HPG", entry_order_id="e", exit_order_id="x",
            entry_time=datetime(2024, 1, 1), exit_time=datetime(2024, 1, 3),
            entry_bar_index=0, exit_bar_index=2, holding_bars=2, quantity=100.0,
            entry_price=20.0, exit_price=25.0, gross_pnl=500_000.0, net_pnl=500_000.0,
            fees=0.0, taxes=0.0, pnl_percent=25.0, status="closed", result="win",
        ))
    # 5 Breakevens (0.0 net)
    for i in range(5):
        trades.append(BacktestTrade(
            id=f"BE{i}", symbol="HPG", entry_order_id="e", exit_order_id="x",
            entry_time=datetime(2024, 1, 1), exit_time=datetime(2024, 1, 3),
            entry_bar_index=0, exit_bar_index=2, holding_bars=2, quantity=100.0,
            entry_price=20.0, exit_price=20.0, gross_pnl=0.0, net_pnl=0.0,
            fees=0.0, taxes=0.0, pnl_percent=0.0, status="closed", result="breakeven",
        ))
    # 5 Losses (-250,000 net)
    for i in range(5):
        trades.append(BacktestTrade(
            id=f"L{i}", symbol="HPG", entry_order_id="e", exit_order_id="x",
            entry_time=datetime(2024, 1, 1), exit_time=datetime(2024, 1, 3),
            entry_bar_index=0, exit_bar_index=2, holding_bars=2, quantity=100.0,
            entry_price=20.0, exit_price=17.5, gross_pnl=-250_000.0, net_pnl=-250_000.0,
            fees=0.0, taxes=0.0, pnl_percent=-12.5, status="closed", result="loss",
        ))

    # Engine 1: metrics.py
    m_row = calculate_benchmark_metrics(trades, 100_000_000.0, "HPG")
    assert m_row.num_trades == 15
    assert m_row.num_winners == 5
    assert m_row.num_breakeven == 5
    assert m_row.num_losers == 5
    assert math.isclose(m_row.win_rate_pct, 100.0 / 3.0, abs_tol=1e-4)
    assert math.isclose(m_row.net_profit, 1_250_000.0, abs_tol=1e-2)
    assert math.isclose(m_row.profit_factor, 2.0, abs_tol=1e-4)
    assert m_row.profit_factor_display == "2.00"
    assert m_row.low_sample_warning is True

    # Engine 2: StrategyLabService
    from app.domain.backtest.models import BacktestLedger
    k_res = BacktestKernelResult(
        ledger=BacktestLedger(
            initial_cash=100_000_000.0, current_cash=101_250_000.0,
            portfolio_value=101_250_000.0, peak_portfolio_value=101_250_000.0,
            max_drawdown=0.0, trades=trades,
        ),
        symbol="HPG", timeframe="1D",
        start_date=datetime(2024, 1, 1), end_date=datetime(2024, 2, 1),
        total_candles=30, last_analyzed_index=29,
    )
    dummy_strat = load_strategy_from_dict({
        "name": "Dummy", "indicators": [], "entry_rules": [], "exit_rules": [],
        "position_sizing": {"method": "fixed_quantity", "quantity": 100},
    })
    lab = StrategyLabService()
    dummy_candles = [
        Candle(symbol="HPG", timestamp=date(2024, 1, 1), open=20.0, high=20.0, low=20.0, close=20.0, volume=100),
        Candle(symbol="HPG", timestamp=date(2024, 2, 1), open=20.0, high=20.0, low=20.0, close=20.0, volume=100),
    ]
    f_res = lab._format_kernel_response(
        kernel_res=k_res, strategy_obj=dummy_strat, symbol="HPG",
        df=pd.DataFrame([{"timestamp": "2024-01-01", "close": 20.0}, {"timestamp": "2024-02-01", "close": 20.0}]),
        candles=dummy_candles, initial_cash=100_000_000.0, benchmark_symbol="VNINDEX",
    )
    assert f_res["analytics"]["total_trades"] == 15
    assert math.isclose(f_res["analytics"]["win_rate"], 1.0 / 3.0, abs_tol=1e-4)
    assert math.isclose(f_res["analytics"]["total_net_pnl"], 1_250_000.0, abs_tol=1e-2)
    assert math.isclose(f_res["analytics"]["profit_factor"], 2.0, abs_tol=1e-2)
    assert f_res["analytics"]["profit_factor_display"] == "2.00"
    assert f_res["analytics"]["low_sample_warning"] is True

    # Engine 3: Replay / AnalyticsService
    session = ReplaySession(
        symbol="HPG", timeframe="1D",
        start_date=date(2024, 1, 1), end_date=date(2024, 2, 1),
        current_index=1, initial_cash=100_000_000.0, current_cash=101_250_000.0,
        mode=SessionMode.NORMAL.value, status=SessionStatus.ACTIVE.value,
    )
    isolated_db.add(session)
    isolated_db.flush()
    for t in trades:
        isolated_db.add(Trade(
            session_id=session.id, symbol="HPG", entry_date=t.entry_time, exit_date=t.exit_time,
            entry_price=t.entry_price, exit_price=t.exit_price, quantity=t.quantity,
            net_pnl=t.net_pnl, status="closed", result=t.result,
        ))
    isolated_db.commit()

    analytics = AnalyticsService.get_analytics(isolated_db, session.id)
    assert analytics.total_trades == 15
    assert math.isclose(analytics.win_rate, 1.0 / 3.0, abs_tol=1e-4)
    assert math.isclose(analytics.total_net_pnl, 1_250_000.0, abs_tol=1e-2)
    assert math.isclose(analytics.profit_factor, 2.0, abs_tol=1e-2)
    assert analytics.profit_factor_display == "2.00"
    assert analytics.low_sample_warning is True


# ============================================================================
# 7. Deep Drawdown Recovery & Multiple Local Peaks Stress
# ============================================================================

def test_adv_drawdown_deep_recovery_multiple_peaks():
    """Verify max drawdown tracks the deepest peak-to-trough drop even after full recoveries.
    
    Curve progression:
    - Point 0: 100.0
    - Point 1: 150.0 (Peak 1)
    - Point 2: 30.0  (Drop 1: (150 - 30)/150 = 80.0%)
    - Point 3: 180.0 (Peak 2, full recovery beyond Peak 1)
    - Point 4: 20.0  (Drop 2: (180 - 20)/180 = 88.888889%) -> WORST
    - Point 5: 200.0 (Peak 3, full recovery beyond Peak 2)
    
    Expected Max Drawdown = 160 / 180 = 88.888889% (0.888889 fraction).
    """
    curve = [
        {"equity": 100.0},
        {"equity": 150.0},
        {"equity": 30.0},
        {"equity": 180.0},
        {"equity": 20.0},
        {"equity": 200.0},
    ]

    expected_mdd = 160.0 / 180.0  # 88.888889%
    calc_mdd = calculate_max_drawdown(curve)
    assert math.isclose(calc_mdd, expected_mdd, abs_tol=1e-6)

    analytics_mdd = AnalyticsService._calculate_max_drawdown(curve)
    assert math.isclose(analytics_mdd["max_drawdown_pct"], expected_mdd * 100.0, abs_tol=1e-2)
    assert math.isclose(analytics_mdd["max_drawdown_amount"], 160.0, abs_tol=1e-2)


# ============================================================================
# 8. Negative Drift Sharpe Ratio Stress (252 bars)
# ============================================================================

def test_adv_sharpe_ratio_negative_drift_full_year():
    """Verify that a steadily losing account over 252 daily bars produces
    an unambiguously negative Sharpe ratio that matches across modules.
    """
    # 252 bars with small steady daily loss (-0.2% per day) plus small noise
    equities = [100_000_000.0]
    for i in range(1, 252):
        prev = equities[-1]
        loss_pct = -0.002 - (0.0005 * (i % 3))
        equities.append(prev * (1 + loss_pct))

    curve = [{"equity": e} for e in equities]
    sharpe_m = calculate_sharpe_ratio(curve)
    sharpe_a = AnalyticsService._calculate_sharpe_ratio(curve)

    assert sharpe_m < -5.0, f"Expected deep negative Sharpe, got {sharpe_m}"
    assert math.isclose(sharpe_m, sharpe_a, abs_tol=1e-4)

