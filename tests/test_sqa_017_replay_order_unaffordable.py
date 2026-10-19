import sys
from pathlib import Path
from datetime import date, datetime, timedelta, timezone

# Add backend directory to sys.path
_backend_dir = Path(__file__).resolve().parent.parent / "backend"
if str(_backend_dir) not in sys.path:
    sys.path.insert(0, str(_backend_dir))

from tests_safety import install_database_safeguard
install_database_safeguard()

import pytest
from fastapi import HTTPException
from sqlalchemy import create_engine
from sqlalchemy.orm import sessionmaker
from sqlalchemy.pool import StaticPool

from app.db import Base
import app.models  # noqa: F401
from app.models.candle import Candle
from app.models.symbol import Symbol
from app.models.order import Order
from app.models.position import Position
from app.models.trade import Trade
from app.models.event_log import EventLog
from app.models.replay_session import ReplaySession
from app.services.replay_service import ReplayService
from app.services.trade_lifecycle_service import TradeLifecycleService
from app.services.practice_workflow_service import PracticeWorkflowService
from app.schemas.replay_schema import ReplaySessionCreate
from app.schemas.decision_schema import DecisionCreate
from app.domain.enums import DecisionAction, OrderType, OrderStatus, OrderSide
from app.api.decisions import get_orders, clear_order_rejection_registry
from app.domain.market.calendar import TradingCalendar


@pytest.fixture(autouse=True)
def clean_registry():
    clear_order_rejection_registry()
    yield
    clear_order_rejection_registry()


@pytest.fixture
def test_db():
    engine = create_engine(
        "sqlite:///:memory:",
        connect_args={"check_same_thread": False},
        poolclass=StaticPool,
    )
    Base.metadata.create_all(bind=engine)
    SessionClass = sessionmaker(autocommit=False, autoflush=False, bind=engine)
    db = SessionClass()
    try:
        yield db
    finally:
        db.close()
        Base.metadata.drop_all(bind=engine)


def _seed_candles(db, symbol: str = "FPT", count: int = 15, base_price: float = 90_000.0):
    sym = Symbol(
        symbol=symbol,
        exchange="HOSE",
        company_name=f"{symbol} Corp",
        sector="Technology",
        industry="Software",
        asset_type="stock",
        is_active=True,
    )
    db.merge(sym)
    db.flush()

    cal = TradingCalendar()
    cur = date(2024, 1, 2)
    added = 0
    while added < count:
        if cal.is_trading_day(cur):
            db.add(
                Candle(
                    symbol=symbol,
                    timeframe="1D",
                    timestamp=datetime(cur.year, cur.month, cur.day, tzinfo=timezone.utc),
                    open=base_price,
                    high=base_price + 2_000.0,
                    low=base_price - 1_000.0,
                    close=base_price + 1_000.0,
                    volume=1_000_000 + added * 10_000,
                    source="cafef",
                    adjustment_type="unadjusted",
                )
            )
            added += 1
        cur += timedelta(days=1)
    db.commit()


def test_unaffordable_market_next_open_buy_order_does_not_wedge_replay(test_db):
    """SQA-017: A pending MARKET_NEXT_OPEN BUY with insufficient cash must transition to REJECTED

    without raising HTTPException(400) during candle advance, without deducting cash or creating
    phantom positions/trades, and must allow subsequent replay stepping without wedging.
    """
    _seed_candles(test_db, "FPT", count=10, base_price=90_000.0)

    # 1. Create replay session with 100M VND starting cash
    session_in = ReplaySessionCreate(
        symbol="FPT",
        timeframe="1D",
        start_date=date(2024, 1, 2),
        end_date=date(2024, 1, 30),
        initial_cash=100_000_000.0,
    )
    session = ReplayService.create_session(test_db, session_in)
    assert session.current_index == 0
    assert session.current_cash == 100_000_000.0

    # 2. Submit an unaffordable MARKET_NEXT_OPEN BUY (10,000 shares @ 90k ~= 900M VND)
    decision_in = DecisionCreate(
        action=DecisionAction.BUY,
        order_type=OrderType.MARKET_NEXT_OPEN.value,
        quantity=10_000.0,
        price=90_000.0,
        note="Unaffordable aggressive order",
    )
    decision = TradeLifecycleService.process_decision(test_db, session.id, decision_in)
    assert decision.id is not None

    pending_orders = test_db.query(Order).filter(
        Order.session_id == session.id,
        Order.status == OrderStatus.PENDING.value,
    ).all()
    assert len(pending_orders) == 1
    order = pending_orders[0]
    assert order.order_type == OrderType.MARKET_NEXT_OPEN.value
    assert order.quantity == 10_000.0

    # 3. Advance to the next candle (index 1) - must NOT throw HTTPException(400) or wedge replay!
    next_session = ReplayService.next_candle(test_db, session.id, steps=1)
    assert next_session.current_index == 1
    assert next_session.current_cash == 100_000_000.0  # Zero cash deducted

    # 4. Verify order transitioned to REJECTED cleanly
    test_db.refresh(order)
    assert order.status == OrderStatus.REJECTED.value

    # 5. Verify no phantom position or false trade
    open_positions = test_db.query(Position).filter(
        Position.session_id == session.id,
        Position.status == "open",
    ).all()
    assert len(open_positions) == 0

    trades = test_db.query(Trade).filter(Trade.session_id == session.id).all()
    assert len(trades) == 0

    # 6. Verify EventLog contains ORDER_REJECTED
    rejection_logs = test_db.query(EventLog).filter(
        EventLog.session_id == session.id,
        EventLog.event_type == "ORDER_REJECTED",
    ).all()
    assert len(rejection_logs) >= 1
    assert "insufficient cash" in rejection_logs[0].message.lower()

    # 7. Advance replay further: subsequent stepping must work completely normally!
    step2_session = ReplayService.next_candle(test_db, session.id, steps=1)
    assert step2_session.current_index == 2

    step3_session = ReplayService.next_candle(test_db, session.id, steps=1)
    assert step3_session.current_index == 3


def test_unaffordable_limit_buy_order_does_not_wedge_replay(test_db):
    """SQA-017: A pending LIMIT BUY that becomes executed when candle price crosses limit

    must be rejected cleanly if cash is insufficient, without wedging replay stepping.
    """
    _seed_candles(test_db, "FPT", count=10, base_price=90_000.0)

    session_in = ReplaySessionCreate(
        symbol="FPT",
        timeframe="1D",
        start_date=date(2024, 1, 2),
        end_date=date(2024, 1, 30),
        initial_cash=50_000_000.0,
    )
    session = ReplayService.create_session(test_db, session_in)
    assert session.current_cash == 50_000_000.0

    # Submit LIMIT order for 89,500 VND x 5,000 shares = ~447.5M VND (> 50M cash)
    decision_in = DecisionCreate(
        action=DecisionAction.BUY,
        order_type=OrderType.LIMIT.value,
        quantity=5_000.0,
        price=89_500.0,
    )
    TradeLifecycleService.process_decision(test_db, session.id, decision_in)

    # Next candle has low=89,000 <= 89,500, triggering the limit order
    next_session = ReplayService.next_candle(test_db, session.id, steps=1)
    assert next_session.current_index == 1
    assert next_session.current_cash == 50_000_000.0

    order = test_db.query(Order).filter(Order.session_id == session.id).first()
    assert order.status == OrderStatus.REJECTED.value

    # Subsequent steps proceed normally
    step2 = ReplayService.next_candle(test_db, session.id, steps=1)
    assert step2.current_index == 2


def test_replay_cursor_rewind_future_rejection_isolation(test_db):
    """SQA-017 / R4: When rewinding replay cursor before the rejection candle index,

    the rejected order must be masked as PENDING so future rejection status does not leak.
    When rewinding before order creation, the order must be masked entirely.
    """
    _seed_candles(test_db, "FPT", count=10, base_price=90_000.0)

    session_in = ReplaySessionCreate(
        symbol="FPT",
        timeframe="1D",
        start_date=date(2024, 1, 2),
        end_date=date(2024, 1, 30),
        initial_cash=100_000_000.0,
    )
    session = ReplayService.create_session(test_db, session_in)

    # Step to index 1 first
    ReplayService.next_candle(test_db, session.id, steps=1)
    test_db.refresh(session)
    assert session.current_index == 1

    # At index 1, place MARKET_NEXT_OPEN BUY for 10,000 shares
    decision_in = DecisionCreate(
        action=DecisionAction.BUY,
        order_type=OrderType.MARKET_NEXT_OPEN.value,
        quantity=10_000.0,
        price=90_000.0,
    )
    TradeLifecycleService.process_decision(test_db, session.id, decision_in)

    # Step to index 2: order is processed at index 2 and rejected for insufficient cash
    ReplayService.next_candle(test_db, session.id, steps=1)
    test_db.refresh(session)
    assert session.current_index == 2

    orders_at_idx2 = get_orders(session.id, db=test_db)
    assert len(orders_at_idx2) == 1
    assert orders_at_idx2[0].status == OrderStatus.REJECTED
    snap_at_idx2 = PracticeWorkflowService.get_snapshot(test_db, session.id)
    assert len(snap_at_idx2.orders) == 1
    assert snap_at_idx2.orders[0].status == "rejected"

    # Rewind cursor to index 1 (order created at 1, but rejected at 2)
    ReplayService.previous_candle(test_db, session.id, steps=1)
    test_db.refresh(session)
    assert session.current_index == 1

    # At index 1, rejection occurred in the future (bar 2). Must be masked as PENDING in BOTH /orders and /practice-state!
    orders_at_idx1 = get_orders(session.id, db=test_db)
    assert len(orders_at_idx1) == 1
    assert orders_at_idx1[0].status == OrderStatus.PENDING
    snap_at_idx1 = PracticeWorkflowService.get_snapshot(test_db, session.id)
    assert len(snap_at_idx1.orders) == 1
    assert snap_at_idx1.orders[0].status == "pending"

    # Rewind cursor to index 0 (order not yet created at bar 0)
    ReplayService.previous_candle(test_db, session.id, steps=1)
    test_db.refresh(session)
    assert session.current_index == 0

    # At index 0, order must not exist at all (future order isolation)
    orders_at_idx0 = get_orders(session.id, db=test_db)
    assert len(orders_at_idx0) == 0
    snap_at_idx0 = PracticeWorkflowService.get_snapshot(test_db, session.id)
    assert len(snap_at_idx0.orders) == 0


def test_sqa017_next_open_gap_unaffordable_causal_projection_parity(test_db):
    """Mandatory reproduction for SQA-017:
    - Initial cash 97,700,000 VND.
    - BUY accepted at submission close (idx=2, close=96,000 VND, qty=1,000 -> 96,144,000 VND <= 97.7M).
    - BUY becomes unaffordable at next-open (idx=3, open=98,500 VND, qty=1,000 -> 98,647,750 VND > 97.7M).
    - Durable rejection at idx=3.
    - Rewind to idx=2 -> both /orders and /practice-state project 'pending', durable DB remains 'rejected'.
    - Reload at idx=2 (clearing runtime registry) -> both /orders and /practice-state still project 'pending'.
    - Advance again to idx=3 -> both /orders and /practice-state project 'rejected', no phantom fill or duplicate rejection.
    """
    from app.services.practice_workflow_service import PracticeWorkflowService
    from app.models.execution import Execution

    sym = Symbol(
        symbol="FPT_GAP",
        exchange="HOSE",
        company_name="FPT Gap Test",
        sector="Technology",
        industry="Software",
        asset_type="stock",
        is_active=True,
    )
    test_db.merge(sym)
    test_db.flush()

    # Seed 6 deterministic candles where idx=2 close is 96,000 and idx=3 open gaps up to 98,500
    candle_specs = [
        (date(2024, 1, 2), 95_000.0, 96_000.0, 94_500.0, 95_500.0),
        (date(2024, 1, 3), 95_500.0, 96_500.0, 95_000.0, 96_000.0),
        (date(2024, 1, 4), 96_000.0, 96_500.0, 95_500.0, 96_000.0),  # idx=2: close=96,000 (affordable)
        (date(2024, 1, 5), 98_500.0, 99_500.0, 98_000.0, 99_000.0),  # idx=3: open=98,500 (unaffordable gap!)
        (date(2024, 1, 8), 99_000.0, 100_000.0, 98_500.0, 99_500.0),
        (date(2024, 1, 9), 99_500.0, 100_500.0, 99_000.0, 100_000.0),
    ]
    for c_date, o, h, l, c in candle_specs:
        test_db.add(
            Candle(
                symbol="FPT_GAP",
                timeframe="1D",
                timestamp=datetime(c_date.year, c_date.month, c_date.day, tzinfo=timezone.utc),
                open=o,
                high=h,
                low=l,
                close=c,
                volume=1_500_000,
                source="cafef",
                adjustment_type="unadjusted",
            )
        )
    test_db.commit()

    session = ReplayService.create_session(
        test_db,
        ReplaySessionCreate(
            symbol="FPT_GAP",
            timeframe="1D",
            start_date=date(2024, 1, 2),
            end_date=date(2024, 1, 15),
            initial_cash=97_700_000.0,
        ),
    )

    # Step to idx=2
    ReplayService.next_candle(test_db, session.id, steps=2)
    test_db.refresh(session)
    assert session.current_index == 2
    assert session.current_cash == 97_700_000.0

    # Submit MARKET_NEXT_OPEN BUY at idx=2 (1,000 shares @ 96,000 close)
    TradeLifecycleService.process_decision(
        test_db,
        session.id,
        DecisionCreate(
            action=DecisionAction.BUY,
            order_type=OrderType.MARKET_NEXT_OPEN.value,
            quantity=1_000.0,
            price=96_000.0,
            reason="Affordable at idx=2 close, unaffordable at idx=3 open gap",
        ),
    )

    # At idx=2 before stepping: both /orders and /practice-state show pending
    assert get_orders(session.id, db=test_db)[0].status == OrderStatus.PENDING
    assert PracticeWorkflowService.get_snapshot(test_db, session.id).orders[0].status == "pending"

    # Step to idx=3: open is 98,500 -> required cash 98,647,750 > 97,700,000 -> durably rejected at idx=3
    ReplayService.next_candle(test_db, session.id, steps=1)
    test_db.refresh(session)
    assert session.current_index == 3
    assert session.current_cash == 97_700_000.0

    db_order = test_db.query(Order).filter(Order.session_id == session.id).one()
    assert db_order.status == OrderStatus.REJECTED.value
    assert get_orders(session.id, db=test_db)[0].status == OrderStatus.REJECTED
    snap_idx3 = PracticeWorkflowService.get_snapshot(test_db, session.id)
    assert snap_idx3.orders[0].status == "rejected"
    assert snap_idx3.positions == []
    assert snap_idx3.trades == []
    assert snap_idx3.executions == []

    rejection_events_count = (
        test_db.query(EventLog)
        .filter(EventLog.session_id == session.id, EventLog.event_type == "ORDER_REJECTED")
        .count()
    )
    assert rejection_events_count == 1

    # Rewind to idx=2
    ReplayService.previous_candle(test_db, session.id, steps=1)
    test_db.refresh(session)
    assert session.current_index == 2

    # At idx=2: BOTH /orders and /practice-state MUST project 'pending', while durable DB remains 'rejected'
    assert get_orders(session.id, db=test_db)[0].status == OrderStatus.PENDING
    snap_rewound_idx2 = PracticeWorkflowService.get_snapshot(test_db, session.id)
    assert snap_rewound_idx2.orders[0].status == "pending"
    assert snap_rewound_idx2.historical is True
    assert snap_rewound_idx2.latest_activity_index == 3
    test_db.refresh(db_order)
    assert db_order.status == OrderStatus.REJECTED.value

    # Simulate session reload at idx=2 (clear in-memory rejection registry; rely on durable EventLog)
    clear_order_rejection_registry()
    assert get_orders(session.id, db=test_db)[0].status == OrderStatus.PENDING
    snap_reloaded_idx2 = PracticeWorkflowService.get_snapshot(test_db, session.id)
    assert snap_reloaded_idx2.orders[0].status == "pending"

    # Advance again to idx=3: projected status returns to 'rejected' with zero duplicate fills or events
    ReplayService.next_candle(test_db, session.id, steps=1)
    test_db.refresh(session)
    assert session.current_index == 3
    assert session.current_cash == 97_700_000.0
    assert get_orders(session.id, db=test_db)[0].status == OrderStatus.REJECTED
    snap_readvanced_idx3 = PracticeWorkflowService.get_snapshot(test_db, session.id)
    assert snap_readvanced_idx3.orders[0].status == "rejected"
    assert snap_readvanced_idx3.historical is False
    assert test_db.query(Execution).filter(Execution.session_id == session.id).count() == 0
    assert test_db.query(Position).filter(Position.session_id == session.id).count() == 0
    assert test_db.query(Trade).filter(Trade.session_id == session.id).count() == 0
    assert (
        test_db.query(EventLog)
        .filter(EventLog.session_id == session.id, EventLog.event_type == "ORDER_REJECTED")
        .count()
        == rejection_events_count
    )


def test_immediate_unaffordable_buy_raises_400(test_db):
    """Direct immediate orders with insufficient cash must still raise HTTPException(400)."""
    _seed_candles(test_db, "FPT", count=5, base_price=90_000.0)

    session_in = ReplaySessionCreate(
        symbol="FPT",
        timeframe="1D",
        start_date=date(2024, 1, 2),
        end_date=date(2024, 1, 20),
        initial_cash=10_000_000.0,
    )
    session = ReplayService.create_session(test_db, session_in)

    # Immediate MARKET_AT_CLOSE order exceeding cash
    decision_in = DecisionCreate(
        action=DecisionAction.BUY,
        order_type=OrderType.MARKET_AT_CLOSE.value,
        quantity=5_000.0,
        price=90_000.0,
    )
    with pytest.raises(HTTPException) as exc_info:
        TradeLifecycleService.process_decision(test_db, session.id, decision_in)

    assert exc_info.value.status_code == 400
    assert "insufficient cash" in str(exc_info.value.detail).lower()
