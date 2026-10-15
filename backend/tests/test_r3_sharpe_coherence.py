"""Deterministic Cross-Service Sharpe Risk-Free-Rate Coherence Test Suite (R3).

Authoritative Acceptance Requirement R3:
- Align default risk-free-rate across AnalyticsService and Backtest Domain Metrics to 0.0.
- Align with Golden Case G-10 definition: annualized daily return Sharpe ratio
  (rf = 0.0, sample std ddof = 1, sqrt(252)).
- Preserve the distinction between pure numerical Sharpe float in metrics.py (N >= 2)
  and presentation-level trust gating in AnalyticsService:
  - When N < 30, MetricResult.status == "insufficient_data", MetricResult.value is None,
    and AnalyticsResponse.sharpe_ratio is None.
  - When N >= 30 and sample variance > 0, MetricResult.status == "valid" and value is
    rounded to 4 decimals.
- Deterministic cross-service parity test with >= 40 nonconstant daily returns:
  Assert that metrics.calculate_sharpe_ratio,
  metrics.calculate_benchmark_metrics().sharpe_ratio,
  AnalyticsService._calculate_sharpe_ratio, and
  AnalyticsService.get_analytics().sharpe_ratio achieve bit-exact 4-decimal parity.
"""

from __future__ import annotations

from datetime import date, datetime, timedelta
import math
from typing import Generator, List

import numpy as np
import pytest
from sqlalchemy import create_engine
from sqlalchemy.orm import Session, sessionmaker

from app.db import Base
from app.domain.backtest.metrics import (
    calculate_benchmark_metrics,
    calculate_daily_returns_from_equity_curve,
    calculate_sharpe_ratio,
)
from app.models.candle import Candle
from app.models.decision import Decision
from app.models.execution import Execution
from app.models.order import Order
from app.models.replay_session import ReplaySession
from app.services.analytics_service import AnalyticsService


@pytest.fixture
def isolated_db(tmp_path) -> Generator[Session, None, None]:
    """Isolated SQLite database session guaranteeing backend/sumi.db invariance."""
    db_file = tmp_path / "test_r3_sharpe_coherence.db"
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


def _build_deterministic_returns() -> List[float]:
    """Return 45 deterministic, nonconstant daily returns (N >= 40)."""
    return [
        0.0125, -0.0084, 0.0052, 0.0151, -0.0033,
        0.0076, -0.0112, 0.0028, 0.0094, -0.0041,
        0.0182, -0.0065, 0.0019, 0.0105, -0.0153,
        0.0087, -0.0022, 0.0141, -0.0073, 0.0035,
        -0.0051, 0.0164, -0.0092, 0.0048, 0.0117,
        -0.0123, 0.0069, -0.0015, 0.0204, -0.0081,
        0.0038, -0.0047, 0.0132, -0.0101, 0.0056,
        0.0083, -0.0062, 0.0175, -0.0039, 0.0021,
        -0.0078, 0.0159, -0.0054, 0.0091, -0.0025,
    ]


def _setup_session_with_returns(
    db: Session,
    returns: List[float],
    symbol: str = "HPG",
    initial_cash: float = 100_000.0,
    start_dt: datetime = datetime(2024, 1, 1, 0, 0, 0),
) -> ReplaySession:
    """Set up an isolated session whose candles and execution produce exact target returns."""
    num_returns = len(returns)
    end_dt = start_dt + timedelta(days=num_returns)
    session = ReplaySession(
        symbol=symbol,
        timeframe="1D",
        adjustment_type="unadjusted",
        start_date=start_dt.date(),
        end_date=end_dt.date(),
        current_index=num_returns,
        initial_cash=initial_cash,
        current_cash=0.0,
        status="active",
        mode="normal",
    )
    db.add(session)
    db.flush()

    decision = Decision(
        session_id=session.id,
        symbol=symbol,
        decision_date=start_dt,
        candle_index=0,
        action="BUY",
        price=100.0,
        planned_quantity=1000.0,
    )
    db.add(decision)
    db.flush()

    order = Order(
        session_id=session.id,
        decision_id=decision.id,
        symbol=symbol,
        side="BUY",
        order_type="MARKET",
        quantity=1000.0,
        requested_price=100.0,
        status="executed",
    )
    db.add(order)
    db.flush()

    execution = Execution(
        order_id=order.id,
        session_id=session.id,
        symbol=symbol,
        execution_date=start_dt,
        execution_price=100.0,
        quantity=1000.0,
        gross_amount=initial_cash,
        net_amount=initial_cash,
    )
    db.add(execution)

    # Generate sequential candles from compound returns
    current_equity = initial_cash
    candle_0 = Candle(
        symbol=symbol,
        timeframe="1D",
        adjustment_type="unadjusted",
        timestamp=start_dt,
        open=100.0,
        high=101.0,
        low=99.0,
        close=100.0,
        volume=100_000.0,
    )
    db.add(candle_0)

    for i, r in enumerate(returns):
        current_equity *= (1.0 + r)
        close_price = round(current_equity / 1000.0, 4)
        c = Candle(
            symbol=symbol,
            timeframe="1D",
            adjustment_type="unadjusted",
            timestamp=start_dt + timedelta(days=i + 1),
            open=close_price,
            high=close_price * 1.01,
            low=close_price * 0.99,
            close=close_price,
            volume=100_000.0,
        )
        db.add(c)

    db.commit()
    return session


def test_r3_sharpe_cross_service_bit_exact_parity_n45(isolated_db: Session):
    """Verify bit-exact 4-decimal Sharpe parity across 4 services/methods with N=45 >= 40."""
    returns = _build_deterministic_returns()
    assert len(returns) == 45 >= 40

    session = _setup_session_with_returns(isolated_db, returns, symbol="HPG")
    report = AnalyticsService.get_analytics(isolated_db, session.id)

    # 1. AnalyticsService API presentation value
    val_api = report.sharpe_ratio
    metric_res = report.metrics["sharpe_ratio"]

    assert metric_res.status == "valid"
    assert metric_res.sample_size == 45
    assert metric_res.value == val_api
    assert isinstance(val_api, float)

    # 2. AnalyticsService internal static helper
    val_helper = AnalyticsService._calculate_sharpe_ratio(report.equity_curve)

    # 3. Domain metrics calculate_sharpe_ratio
    val_metrics = calculate_sharpe_ratio(report.equity_curve)

    # 4. Domain metrics calculate_benchmark_metrics
    bench_row = calculate_benchmark_metrics(
        trades=[],
        initial_cash=session.initial_cash,
        ticker="HPG",
        equity_curve=report.equity_curve,
    )
    val_benchmark = bench_row.sharpe_ratio

    # Manual oracle verification (Golden Case G-10 definition: rf=0.0, ddof=1, sqrt(252))
    equity_values = [p["equity"] if isinstance(p, dict) else p.equity for p in report.equity_curve]
    daily_returns = [
        (equity_values[i] - equity_values[i - 1]) / equity_values[i - 1]
        for i in range(1, len(equity_values))
    ]
    r_arr = np.array(daily_returns, dtype=float)
    oracle_sharpe = round(float((np.mean(r_arr) / np.std(r_arr, ddof=1)) * math.sqrt(252.0)), 4)

    # Bit-exact 4-decimal parity check
    assert val_api == oracle_sharpe
    assert metric_res.value == oracle_sharpe
    assert val_helper == oracle_sharpe
    assert val_metrics == oracle_sharpe
    assert val_benchmark == oracle_sharpe


def test_r3_insufficient_sample_state_preservation_n25(isolated_db: Session):
    """Verify trust gating when N < 30 (N=25 returns, 26 candles).

    Presentation layer (AnalyticsService.get_analytics):
    - status == "insufficient_data"
    - value is None
    - response.sharpe_ratio is None
    - explanatory reason present

    Numerical layer (metrics.py & _calculate_sharpe_ratio):
    - returns genuine non-zero float for N >= 2
    """
    returns_short = _build_deterministic_returns()[:25]
    assert len(returns_short) == 25 < 30

    session_short = _setup_session_with_returns(
        isolated_db, returns_short, symbol="VNM", start_dt=datetime(2024, 3, 1, 0, 0, 0)
    )
    report_short = AnalyticsService.get_analytics(isolated_db, session_short.id)

    # Presentation layer gating
    assert report_short.sharpe_ratio is None
    metric_short = report_short.metrics["sharpe_ratio"]
    assert metric_short.status == "insufficient_data"
    assert metric_short.value is None
    assert metric_short.sample_size == 25
    assert "at least 30 daily equity returns" in metric_short.reason

    # Mathematical layer returns numerical float (N=25 >= 2)
    curve_short = report_short.equity_curve
    pure_sharpe = calculate_sharpe_ratio(curve_short)
    helper_sharpe = AnalyticsService._calculate_sharpe_ratio(curve_short)
    bench_row = calculate_benchmark_metrics(
        trades=[],
        initial_cash=session_short.initial_cash,
        ticker="VNM",
        equity_curve=curve_short,
    )

    assert isinstance(pure_sharpe, float)
    assert pure_sharpe != 0.0
    assert pure_sharpe == helper_sharpe
    assert pure_sharpe == bench_row.sharpe_ratio


def test_r3_explicit_rf_annual_coherence(isolated_db: Session):
    """Verify bit-exact 4-decimal parity when explicit rf_annual=0.045 is provided."""
    returns = _build_deterministic_returns()
    session = _setup_session_with_returns(
        isolated_db, returns, symbol="FPT", start_dt=datetime(2024, 6, 1, 0, 0, 0)
    )

    rf_annual = 0.045
    report = AnalyticsService.get_analytics(isolated_db, session.id, rf_annual=rf_annual)
    curve = report.equity_curve

    val_api = report.sharpe_ratio
    val_metric = report.metrics["sharpe_ratio"].value
    val_helper = AnalyticsService._calculate_sharpe_ratio(curve, rf_annual=rf_annual)
    val_pure = calculate_sharpe_ratio(curve, rf_annual=rf_annual)
    val_bench = calculate_benchmark_metrics(
        trades=[],
        initial_cash=session.initial_cash,
        ticker="FPT",
        equity_curve=curve,
        rf_annual=rf_annual,
    ).sharpe_ratio

    # All four services/methods must achieve bit-exact parity with explicit rf_annual
    assert val_api == val_metric == val_helper == val_pure == val_bench

    # Excess returns under rf=0.045 must be lower than under rf=0.0
    val_pure_rf0 = calculate_sharpe_ratio(curve, rf_annual=0.0)
    assert val_pure < val_pure_rf0


def test_r3_zero_variance_flat_returns(isolated_db: Session):
    """Verify zero-variance handling when N >= 30:

    - Numerical Sharpe returns 0.0.
    - AnalyticsService reports status='not_applicable' and value=None.
    """
    flat_returns = [0.0] * 35
    session_flat = _setup_session_with_returns(
        isolated_db, flat_returns, symbol="VIC", start_dt=datetime(2024, 8, 1, 0, 0, 0)
    )
    report_flat = AnalyticsService.get_analytics(isolated_db, session_flat.id)

    # Presentation layer
    assert report_flat.sharpe_ratio is None
    metric_flat = report_flat.metrics["sharpe_ratio"]
    assert metric_flat.status == "not_applicable"
    assert metric_flat.value is None
    assert metric_flat.sample_size == 35
    assert "zero variance" in metric_flat.reason

    # Pure numerical layer
    assert calculate_sharpe_ratio(report_flat.equity_curve) == 0.0
    assert AnalyticsService._calculate_sharpe_ratio(report_flat.equity_curve) == 0.0
