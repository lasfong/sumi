"""R2 Regression Suite: Authoritative Settlement Calendar Enforcement.

Validates requirement R2 from External GPT-6 Architect Review:
1. T+2 morning rejection vs permitted afternoon settlement phase.
2. Weekend boundary enforcement (orders spanning Friday to Tuesday).
3. Holiday boundary enforcement (Hung Kings, Reunification, Labor Day calendar closures).
4. Tri-service agreement: Backtest Kernel, Practice, and Trade Lifecycle evaluate
   sellability identically for identical lots and dates.
5. Strict prevention of bar-count heuristics overriding calendar closures.

Database safety:
All tests run against isolated temporary SQLite databases.
Production backend/sumi.db is strictly untouched.
"""

from __future__ import annotations

from datetime import date, datetime
from types import SimpleNamespace
from typing import Generator
import numpy as np
import pandas as pd
import pytest
from fastapi import HTTPException
from sqlalchemy import create_engine
from sqlalchemy.orm import Session, sessionmaker

from app.db import Base
from app.domain.backtest.execution import BacktestExecutionKernel, RejectedSettlementLock
from app.domain.market.calendar import TradingCalendar
from app.domain.market.rules import DEFAULT_VN_MARKET_PROFILE
from app.domain.market.settlement import HoldingLot, SettlementEngine
from app.models.candle import Candle
from app.models.decision import Decision
from app.models.execution import Execution
from app.models.order import Order
from app.models.position import Position
from app.models.replay_session import ReplaySession
from app.schemas.decision_schema import DecisionCreate
from app.services.practice_workflow_service import PracticeWorkflowService
from app.services.trade_lifecycle_service import TradeLifecycleService


@pytest.fixture
def isolated_db(tmp_path) -> Generator[Session, None, None]:
    """Isolated SQLite database session for deterministic tests."""
    db_file = tmp_path / "test_r2.db"
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


# ---------------------------------------------------------------------------
# 1. T+2 Morning Rejection vs Permitted Afternoon Settlement Phase
# ---------------------------------------------------------------------------


def test_r2_t2_morning_rejection_vs_afternoon_settlement(isolated_db: Session):
    """Under conservative mode (T+1.5 ADR-SETTLE-001):
    - Day T+2 morning Open: SELL must be rejected (shares in transit until afternoon).
    - Day T+2 afternoon / Close: SELL is permitted (shares settled in afternoon session).
    """
    db = isolated_db
    engine = SettlementEngine(conservative_mode=True)

    # Trading days: Monday 2024-03-04 (T), Tuesday 2024-03-05 (T+1), Wednesday 2024-03-06 (T+2)
    t0_date = date(2024, 3, 4)  # Monday
    t1_date = date(2024, 3, 5)  # Tuesday
    t2_date = date(2024, 3, 6)  # Wednesday
    t3_date = date(2024, 3, 7)  # Thursday

    # Engine level verification
    assert engine.is_sellable(t0_date, t0_date, current_session_phase="open") is False
    assert engine.is_sellable(t0_date, t0_date, current_session_phase="close") is False
    assert engine.is_sellable(t0_date, t1_date, current_session_phase="open") is False
    assert engine.is_sellable(t0_date, t1_date, current_session_phase="close") is False

    # Day T+2 morning Open is strictly REJECTED
    assert engine.is_sellable(t0_date, t2_date, current_session_phase="open") is False
    assert engine.is_sellable(t0_date, t2_date, current_session_phase="intraday") is False

    # Day T+2 afternoon / Close is strictly PERMITTED
    assert engine.is_sellable(t0_date, t2_date, current_session_phase="close") is True
    assert engine.is_sellable(t0_date, t2_date, current_session_phase="afternoon") is True
    assert engine.is_sellable(t0_date, t2_date, current_session_phase="atc") is True

    # Day T+3 is fully sellable across all phases
    assert engine.is_sellable(t0_date, t3_date, current_session_phase="open") is True
    assert engine.is_sellable(t0_date, t3_date, current_session_phase="close") is True

    # DB Service verification: TradeLifecycleService
    c1 = Candle(symbol="TCB", timeframe="1D", timestamp=datetime(2024, 3, 4), open=40.0, high=41.0, low=39.0, close=40.5, volume=1000)
    c2 = Candle(symbol="TCB", timeframe="1D", timestamp=datetime(2024, 3, 5), open=40.5, high=41.5, low=40.0, close=41.0, volume=1200)
    c3 = Candle(symbol="TCB", timeframe="1D", timestamp=datetime(2024, 3, 6), open=41.0, high=42.0, low=40.5, close=41.5, volume=1500)
    db.add_all([c1, c2, c3])
    db.commit()

    session = ReplaySession(
        symbol="TCB",
        timeframe="1D",
        start_date=datetime(2024, 3, 4),
        end_date=datetime(2024, 3, 6),
        current_index=2,  # Wednesday 2024-03-06 (Day T+2)
        initial_cash=100_000_000.0,
        current_cash=96_000_000.0,
    )
    db.add(session)
    db.commit()

    # Buy executed at bar 0 (Monday 2024-03-04)
    dec_buy = Decision(session_id=session.id, symbol="TCB", action="BUY", candle_index=0, decision_date=datetime(2024, 3, 4), price=40.0, planned_quantity=100)
    db.add(dec_buy)
    db.flush()
    order_buy = Order(session_id=session.id, decision_id=dec_buy.id, symbol="TCB", side="BUY", order_type="MARKET_NEXT_OPEN", quantity=100, status="executed")
    db.add(order_buy)
    db.flush()
    exc_buy = Execution(session_id=session.id, order_id=order_buy.id, symbol="TCB", execution_candle_index=0, execution_date=datetime(2024, 3, 4), execution_price=40.0, quantity=100, gross_amount=4000.0, net_amount=4006.0)
    db.add(exc_buy)
    pos = Position(session_id=session.id, symbol="TCB", quantity=100.0, average_price=40.0, total_cost=4000.0, opened_at=datetime(2024, 3, 4), status="open")
    db.add(pos)
    db.commit()

    # 1. Attempt SELL at T+2 morning Open (MARKET_NEXT_OPEN) -> Must raise 400 LOT_SETTLEMENT_LOCKED
    sell_dec_open = Decision(session_id=session.id, symbol="TCB", action="SELL", candle_index=2, decision_date=datetime(2024, 3, 6), price=41.0, planned_quantity=100)
    db.add(sell_dec_open)
    db.commit()

    dec_in_open = DecisionCreate(action="SELL", candle_index=2, price=41.0, quantity=100, order_type="MARKET_NEXT_OPEN")
    with pytest.raises(HTTPException) as exc_info:
        TradeLifecycleService._execute_sell(
            db=db,
            session=session,
            decision_id=sell_dec_open.id,
            decision_in=dec_in_open,
            symbol="TCB",
            date=datetime(2024, 3, 6),
            price=41.0,
            qty=100.0,
        )
    assert exc_info.value.status_code == 400
    assert exc_info.value.detail["error_code"] == "LOT_SETTLEMENT_LOCKED"
    assert exc_info.value.detail["status"] == "REJECTED_SETTLEMENT_LOCK"
    assert exc_info.value.detail["available_qty"] == 0.0
    assert exc_info.value.detail["blocked_qty"] == 100.0

    # 2. Attempt SELL at T+2 afternoon Close (MARKET_AT_CLOSE) -> Must succeed
    sell_dec_close = Decision(session_id=session.id, symbol="TCB", action="SELL", candle_index=2, decision_date=datetime(2024, 3, 6), price=41.5, planned_quantity=100)
    db.add(sell_dec_close)
    db.commit()

    dec_in_close = DecisionCreate(action="SELL", candle_index=2, price=41.5, quantity=100, order_type="MARKET_AT_CLOSE")
    TradeLifecycleService._execute_sell(
        db=db,
        session=session,
        decision_id=sell_dec_close.id,
        decision_in=dec_in_close,
        symbol="TCB",
        date=datetime(2024, 3, 6),
        price=41.5,
        qty=100.0,
    )
    db.commit()

    # Position is now fully closed
    pos_after = db.query(Position).filter_by(session_id=session.id).first()
    assert pos_after.quantity == 0.0
    assert pos_after.status == "closed"


# ---------------------------------------------------------------------------
# 2. Weekend Boundary Enforcement (Orders Spanning Friday to Tuesday)
# ---------------------------------------------------------------------------


def test_r2_weekend_boundary_enforcement(isolated_db: Session):
    """Buy on Friday 2024-03-01:
    - Saturday 2024-03-02 & Sunday 2024-03-03 are weekend non-trading days.
    - Monday 2024-03-04 is Day T+1 trading day: strictly UNSETTLED / locked.
    - Tuesday 2024-03-05 is Day T+2 trading day: Morning Open locked, Afternoon Close sellable.
    - Wednesday 2024-03-06 is Day T+3 trading day: Morning Open sellable.
    """
    db = isolated_db
    engine = SettlementEngine()

    buy_date = date(2024, 3, 1)  # Friday

    # Calendar validations
    assert engine.calendar.is_trading_day(date(2024, 3, 1)) is True
    assert engine.calendar.is_trading_day(date(2024, 3, 2)) is False  # Saturday
    assert engine.calendar.is_trading_day(date(2024, 3, 3)) is False  # Sunday
    assert engine.calendar.is_trading_day(date(2024, 3, 4)) is True   # Monday (T+1)
    assert engine.calendar.is_trading_day(date(2024, 3, 5)) is True   # Tuesday (T+2)

    # Monday T+1 is unsellable across all phases
    assert engine.is_sellable(buy_date, date(2024, 3, 4), current_session_phase="open") is False
    assert engine.is_sellable(buy_date, date(2024, 3, 4), current_session_phase="close") is False

    # Tuesday T+2: Morning Open is locked, Afternoon Close is sellable
    assert engine.is_sellable(buy_date, date(2024, 3, 5), current_session_phase="open") is False
    assert engine.is_sellable(buy_date, date(2024, 3, 5), current_session_phase="close") is True

    # Wednesday T+3 is sellable at Open
    assert engine.is_sellable(buy_date, date(2024, 3, 6), current_session_phase="open") is True

    # Exact calculate_sellable_date is Tuesday 2024-03-05
    assert engine.calculate_sellable_date(buy_date) == date(2024, 3, 5)

    # BacktestExecutionKernel test across weekend
    dates = [
        datetime(2024, 3, 1),  # Bar 0: Fri (Buy fill at Open)
        datetime(2024, 3, 4),  # Bar 1: Mon (T+1 trading day)
        datetime(2024, 3, 5),  # Bar 2: Tue (T+2 trading day)
        datetime(2024, 3, 6),  # Bar 3: Wed (T+3 trading day)
    ]
    df = pd.DataFrame({
        "timestamp": dates,
        "open": [10.0, 10.5, 11.0, 11.5],
        "high": [10.5, 11.0, 11.5, 12.0],
        "low": [9.5, 10.0, 10.5, 11.0],
        "close": [10.0, 10.8, 11.2, 11.8],
        "volume": [1000, 1000, 1000, 1000],
    }, index=dates)

    # Signal on Bar 0 close queues SELL for Bar 1 (Monday) Open -> MUST REJECT
    strategy = SimpleNamespace(
        name="weekend_test",
        indicators=[],
        entry_rules=[],
        exit_rules=[{"dsl": {"gt": ["close", 0]}}],  # Triggers exit immediately
        position_sizing=SimpleNamespace(method="fixed_quantity", quantity=100),
        risk_management=None,
    )
    indicator_values = {"close": np.array([10.0, 10.8, 11.2, 11.8])}

    kernel = BacktestExecutionKernel(profile=DEFAULT_VN_MARKET_PROFILE)
    result = kernel.run(
        df=df,
        indicator_values=indicator_values,
        strategy=strategy,
        symbol="VCB",
        initial_cash=100_000.0,
    )

    # No bar-count override allowed premature fill on Monday
    monday_fills = [e for e in result.ledger.executions if e.side == "SELL" and e.fill_bar_index == 1]
    assert len(monday_fills) == 0


# ---------------------------------------------------------------------------
# 3. Holiday Boundary Enforcement (Hung Kings & Tet / May Day Closures)
# ---------------------------------------------------------------------------


def test_r2_holiday_boundary_enforcement_hung_kings():
    """Hung Kings Holiday (2024-04-18 Thursday):
    - Wednesday 2024-04-17 buy (Day T).
    - Thursday 2024-04-18 is Hung Kings holiday (closed).
    - Friday 2024-04-19 is Day T+1 trading session -> locked!
    - Monday 2024-04-22 is Day T+2 trading session -> locked at Open, sellable at Close!
    - Tuesday 2024-04-23 is Day T+3 trading session -> sellable at Open!
    """
    engine = SettlementEngine()
    buy_date = date(2024, 4, 17)  # Wednesday

    # Thursday holiday
    assert engine.calendar.is_trading_day(date(2024, 4, 18)) is False
    # Friday T+1
    assert engine.calendar.is_trading_day(date(2024, 4, 19)) is True
    assert engine.is_sellable(buy_date, date(2024, 4, 19), current_session_phase="open") is False
    assert engine.is_sellable(buy_date, date(2024, 4, 19), current_session_phase="close") is False

    # Monday T+2
    assert engine.calendar.is_trading_day(date(2024, 4, 22)) is True
    assert engine.is_sellable(buy_date, date(2024, 4, 22), current_session_phase="open") is False
    assert engine.is_sellable(buy_date, date(2024, 4, 22), current_session_phase="close") is True

    # Tuesday T+3
    assert engine.is_sellable(buy_date, date(2024, 4, 23), current_session_phase="open") is True
    assert engine.calculate_sellable_date(buy_date) == date(2024, 4, 22)


def test_r2_holiday_boundary_enforcement_reunification_and_labor_day():
    """Reunification / May Day 5-day holiday window:
    - Friday 2024-04-26 buy (Day T).
    - Monday 2024-04-29 (swap day), Tuesday 2024-04-30 (Reunification), Wednesday 2024-05-01 (Labor Day) are closed.
    - Thursday 2024-05-02 is Day T+1 trading day -> locked!
    - Friday 2024-05-03 is Day T+2 trading day -> locked at Open, sellable at Close!
    - Saturday 2024-05-04 compensatory workday does NOT trade.
    - Monday 2024-05-06 is Day T+3 trading day -> sellable at Open!
    """
    engine = SettlementEngine()
    buy_date = date(2024, 4, 26)  # Friday

    # Holiday closures
    assert engine.calendar.is_trading_day(date(2024, 4, 29)) is False
    assert engine.calendar.is_trading_day(date(2024, 4, 30)) is False
    assert engine.calendar.is_trading_day(date(2024, 5, 1)) is False

    # Thursday T+1
    assert engine.calendar.is_trading_day(date(2024, 5, 2)) is True
    assert engine.is_sellable(buy_date, date(2024, 5, 2), current_session_phase="open") is False
    assert engine.is_sellable(buy_date, date(2024, 5, 2), current_session_phase="close") is False

    # Friday T+2
    assert engine.calendar.is_trading_day(date(2024, 5, 3)) is True
    assert engine.is_sellable(buy_date, date(2024, 5, 3), current_session_phase="open") is False
    assert engine.is_sellable(buy_date, date(2024, 5, 3), current_session_phase="close") is True

    # Compensatory Saturday
    assert engine.calendar.is_trading_day(date(2024, 5, 4)) is False

    # Monday T+3
    assert engine.calendar.is_trading_day(date(2024, 5, 6)) is True
    assert engine.is_sellable(buy_date, date(2024, 5, 6), current_session_phase="open") is True
    assert engine.calculate_sellable_date(buy_date) == date(2024, 5, 3)


# ---------------------------------------------------------------------------
# 4. Tri-Service Agreement: Backtest Kernel, Practice & Trade Lifecycle
# ---------------------------------------------------------------------------


def test_r2_tri_service_sellability_agreement(isolated_db: Session):
    """Backtest Kernel, Practice Workflow Service, and Trade Lifecycle Service
    must evaluate sellability identically for identical lots and dates across
    Day T, Day T+1, Day T+2 Open, Day T+2 Close, and Day T+3 Open.
    """
    db = isolated_db
    profile = DEFAULT_VN_MARKET_PROFILE
    engine = profile.settlement_engine

    # Trading days: Monday 2024-03-04 (T), Tuesday 2024-03-05 (T+1), Wednesday 2024-03-06 (T+2), Thursday 2024-03-07 (T+3)
    t0 = datetime(2024, 3, 4)
    t1 = datetime(2024, 3, 5)
    t2 = datetime(2024, 3, 6)
    t3 = datetime(2024, 3, 7)

    c0 = Candle(symbol="FPT", timeframe="1D", timestamp=t0, open=100.0, high=102.0, low=99.0, close=101.0, volume=1000)
    c1 = Candle(symbol="FPT", timeframe="1D", timestamp=t1, open=101.0, high=103.0, low=100.0, close=102.0, volume=1000)
    c2 = Candle(symbol="FPT", timeframe="1D", timestamp=t2, open=102.0, high=104.0, low=101.0, close=103.0, volume=1000)
    c3 = Candle(symbol="FPT", timeframe="1D", timestamp=t3, open=103.0, high=105.0, low=102.0, close=104.0, volume=1000)
    db.add_all([c0, c1, c2, c3])
    db.commit()

    session = ReplaySession(
        symbol="FPT",
        timeframe="1D",
        start_date=t0,
        end_date=t3,
        current_index=0,
        initial_cash=100_000_000.0,
        current_cash=90_000_000.0,
    )
    db.add(session)
    db.commit()

    # Buy on Day T (bar 0)
    dec_buy = Decision(session_id=session.id, symbol="FPT", action="BUY", candle_index=0, decision_date=t0, price=100.0, planned_quantity=100)
    db.add(dec_buy)
    db.flush()
    order_buy = Order(session_id=session.id, decision_id=dec_buy.id, symbol="FPT", side="BUY", order_type="MARKET_NEXT_OPEN", quantity=100, status="executed")
    db.add(order_buy)
    db.flush()
    exc_buy = Execution(session_id=session.id, order_id=order_buy.id, symbol="FPT", execution_candle_index=0, execution_date=t0, execution_price=100.0, quantity=100, gross_amount=10000.0, net_amount=10015.0)
    db.add(exc_buy)
    pos = Position(session_id=session.id, symbol="FPT", quantity=100.0, average_price=100.0, total_cost=10000.0, opened_at=t0, status="open")
    db.add(pos)
    db.commit()

    lot = HoldingLot(
        quantity=100.0,
        acquisition_date=t0.date(),
        price=100.0,
        symbol="FPT",
        sellable_date=engine.calculate_sellable_date(t0.date()),
    )

    # -------------------------------------------------------------
    # Evaluate at Day T (t0) Close
    # -------------------------------------------------------------
    session.current_index = 0
    snap_t0 = PracticeWorkflowService.get_snapshot(db, session.id)
    kernel_qty_t0 = engine.get_sellable_quantity([lot], t0.date(), current_session_phase="close")
    assert snap_t0.available_quantity == 0.0
    assert snap_t0.blocked_quantity == 100.0
    assert kernel_qty_t0 == 0.0

    # -------------------------------------------------------------
    # Evaluate at Day T+1 (t1) Close
    # -------------------------------------------------------------
    session.current_index = 1
    snap_t1 = PracticeWorkflowService.get_snapshot(db, session.id)
    kernel_qty_t1 = engine.get_sellable_quantity([lot], t1.date(), current_session_phase="close")
    assert snap_t1.available_quantity == 0.0
    assert snap_t1.blocked_quantity == 100.0
    assert kernel_qty_t1 == 0.0

    # -------------------------------------------------------------
    # Evaluate at Day T+2 (t2) Open phase
    # -------------------------------------------------------------
    kernel_qty_t2_open = engine.get_sellable_quantity([lot], t2.date(), current_session_phase="open")
    assert kernel_qty_t2_open == 0.0  # Kernel: 0 sellable at morning Open

    # TradeLifecycle: must reject SELL at Open
    dec_sell_t2_open = Decision(session_id=session.id, symbol="FPT", action="SELL", candle_index=2, decision_date=t2, price=103.0, planned_quantity=100)
    db.add(dec_sell_t2_open)
    db.commit()
    with pytest.raises(HTTPException) as exc_t2_open:
        TradeLifecycleService._execute_sell(
            db=db,
            session=session,
            decision_id=dec_sell_t2_open.id,
            decision_in=DecisionCreate(action="SELL", candle_index=2, price=103.0, quantity=100, order_type="MARKET_NEXT_OPEN"),
            symbol="FPT",
            date=t2,
            price=103.0,
            qty=100.0,
        )
    assert exc_t2_open.value.status_code == 400
    assert exc_t2_open.value.detail["available_qty"] == 0.0

    # -------------------------------------------------------------
    # Evaluate at Day T+2 (t2) Close phase
    # -------------------------------------------------------------
    session.current_index = 2
    snap_t2_close = PracticeWorkflowService.get_snapshot(db, session.id)
    kernel_qty_t2_close = engine.get_sellable_quantity([lot], t2.date(), current_session_phase="close")
    # Both Practice snapshot and Kernel agree: 100 shares available at Close!
    assert snap_t2_close.available_quantity == 100.0
    assert snap_t2_close.blocked_quantity == 0.0
    assert kernel_qty_t2_close == 100.0

    # TradeLifecycle: must permit SELL at Close
    TradeLifecycleService._execute_sell(
        db=db,
        session=session,
        decision_id=dec_sell_t2_open.id,
        decision_in=DecisionCreate(action="SELL", candle_index=2, price=103.0, quantity=100, order_type="MARKET_AT_CLOSE"),
        symbol="FPT",
        date=t2,
        price=103.0,
        qty=100.0,
    )
    db.commit()

    pos_after = db.query(Position).filter_by(session_id=session.id).first()
    assert pos_after.quantity == 0.0


# ---------------------------------------------------------------------------
# 5. Multi-Lot FIFO Settlement & Heuristic Bypass Prevention
# ---------------------------------------------------------------------------


def test_r2_multi_lot_fifo_settlement_and_sl_tp():
    """Verify multi-lot FIFO allocation respects distinct acquisition dates and settlement phases.
    Lot 1: 100 shares bought on Monday 2024-03-04.
    Lot 2: 100 shares bought on Tuesday 2024-03-05.
    On Wednesday 2024-03-06 (T+2 for Lot 1, T+1 for Lot 2):
    - Open phase: 0 shares sellable.
    - Close phase: 100 shares sellable (Lot 1).
    - Allocating 100 shares leaves Lot 2 intact.
    On Thursday 2024-03-07 (T+3 for Lot 1, T+2 for Lot 2):
    - Open phase: 0 shares of Lot 2 sellable.
    - Close phase: 100 shares of Lot 2 sellable.
    """
    engine = SettlementEngine(conservative_mode=True)
    t0 = date(2024, 3, 4)  # Mon
    t1 = date(2024, 3, 5)  # Tue
    t2 = date(2024, 3, 6)  # Wed
    t3 = date(2024, 3, 7)  # Thu

    lot1 = HoldingLot(quantity=100.0, acquisition_date=t0, symbol="SSI")
    lot2 = HoldingLot(quantity=100.0, acquisition_date=t1, symbol="SSI")
    lots = [lot1, lot2]

    # Wednesday Open: 0 sellable
    assert engine.get_sellable_quantity(lots, t2, current_session_phase="open") == 0.0

    # Wednesday Close: exactly 100 sellable (Lot 1)
    assert engine.get_sellable_quantity(lots, t2, current_session_phase="close") == 100.0

    # Allocate 100 on Wednesday Close
    sold_qty, remaining = engine.allocate_sell(lots, 100.0, t2, current_session_phase="close")
    assert sold_qty == 100.0
    assert len(remaining) == 1
    assert remaining[0].acquisition_date == t1
    assert remaining[0].quantity == 100.0

    # Thursday Open: Lot 2 is on its Day T+2 morning -> 0 sellable
    assert engine.get_sellable_quantity(remaining, t3, current_session_phase="open") == 0.0

    # Thursday Close: Lot 2 is settled -> 100 sellable
    assert engine.get_sellable_quantity(remaining, t3, current_session_phase="close") == 100.0


def test_r2_prevent_heuristic_bar_count_bypass(isolated_db: Session):
    """Verify that bar counts cannot trick SettlementEngine or TradeLifecycleService
    into early unlocks across non-trading periods or short intervals.
    """
    db = isolated_db

    # Suppose a replay session has candles on Friday 2024-03-01 and Monday 2024-03-04
    c1 = Candle(symbol="MBB", timeframe="1D", timestamp=datetime(2024, 3, 1), open=20.0, high=21.0, low=19.5, close=20.5, volume=1000)
    c2 = Candle(symbol="MBB", timeframe="1D", timestamp=datetime(2024, 3, 4), open=20.5, high=21.5, low=20.0, close=21.0, volume=1000)
    db.add_all([c1, c2])
    db.commit()

    session = ReplaySession(
        symbol="MBB",
        timeframe="1D",
        start_date=datetime(2024, 3, 1),
        end_date=datetime(2024, 3, 4),
        current_index=1,  # Monday (bar 1, only 1 trading day elapsed)
        initial_cash=50_000_000.0,
        current_cash=48_000_000.0,
    )
    db.add(session)
    db.commit()

    dec_buy = Decision(session_id=session.id, symbol="MBB", action="BUY", candle_index=0, decision_date=datetime(2024, 3, 1), price=20.0, planned_quantity=100)
    db.add(dec_buy)
    db.flush()
    order_buy = Order(session_id=session.id, decision_id=dec_buy.id, symbol="MBB", side="BUY", order_type="MARKET_NEXT_OPEN", quantity=100, status="executed")
    db.add(order_buy)
    db.flush()
    exc_buy = Execution(session_id=session.id, order_id=order_buy.id, symbol="MBB", execution_candle_index=0, execution_date=datetime(2024, 3, 1), execution_price=20.0, quantity=100, gross_amount=2000.0, net_amount=2003.0)
    db.add(exc_buy)
    pos = Position(session_id=session.id, symbol="MBB", quantity=100.0, average_price=20.0, total_cost=2000.0, opened_at=datetime(2024, 3, 1), status="open")
    db.add(pos)
    db.commit()

    dec_sell = Decision(session_id=session.id, symbol="MBB", action="SELL", candle_index=1, decision_date=datetime(2024, 3, 4), price=21.0, planned_quantity=100)
    db.add(dec_sell)
    db.commit()

    with pytest.raises(HTTPException) as excinfo:
        TradeLifecycleService._execute_sell(
            db=db,
            session=session,
            decision_id=dec_sell.id,
            decision_in=DecisionCreate(action="SELL", candle_index=1, price=21.0, quantity=100, order_type="MARKET_AT_CLOSE"),
            symbol="MBB",
            date=datetime(2024, 3, 4),
            price=21.0,
            qty=100.0,
        )
    assert excinfo.value.status_code == 400
    assert excinfo.value.detail["error_code"] == "LOT_SETTLEMENT_LOCKED"
    assert excinfo.value.detail["earliest_release_date"] == "2024-03-05"
