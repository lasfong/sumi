"""Test Suite for Requirement R4: Replay Cursor Future-State Isolation.

Requirements covered:
1. GET /sessions/{session_id}/orders:
   - Eliminate future cancellation leakage: an order created before or at cursor K,
     but cancelled after cursor K, is masked to OrderStatus.PENDING at cursor K.
   - Preserves existing future execution masking (is_future_fill).
   - When cursor K advances to or past the cancellation bar, order returns unmasked status ("cancelled").
2. GET /sessions/{session_id}/position:
   - Eliminate future position leakage: do not query raw Position.status == "open" directly from SQLite.
   - When inspecting at cursor K, project positions at cursor K using PracticeWorkflowService.get_snapshot.
   - If a position was opened at bar 10 and closed at bar 25, when rewound to bar 15 it returns
     the OPEN position at bar 15, NOT an empty list.
   - If a position was opened in the future (bar > K), it is not visible at cursor K.
   - Calculates unrealized PnL dynamically based on the candle close at cursor K.
   - Underlying historical database rows in SQLite are NEVER mutated.
3. Post-session audit eligibility:
   - include_future=True is forbidden (HTTP 403) during active replay before the final bar.
   - include_future=True is permitted once the session is terminal (COMPLETED, EXPIRED, BANKRUPT)
     or at the final bar of the session.
4. Database immutability:
   - backend/sumi.db is never accessed or mutated; all tests use isolated temporary SQLite databases.
"""

from __future__ import annotations

import json
from datetime import date, datetime, timedelta, timezone
import hashlib
from pathlib import Path
from typing import Generator
import pytest
from fastapi.testclient import TestClient
from sqlalchemy import create_engine
from sqlalchemy.orm import Session, sessionmaker

from app.db import Base
from app.dependencies import get_db
from app.domain.enums import DecisionAction, OrderSide, OrderStatus, OrderType, PositionStatus, SessionStatus
from app.main import app
from app.models.candle import Candle
from app.models.decision import Decision
from app.models.event_log import EventLog
from app.models.order import Order
from app.models.position import Position
from app.models.replay_session import ReplaySession
from app.models.trade import Trade
from app.schemas.decision_schema import DecisionCreate
from app.schemas.replay_schema import ReplaySessionCreate
from app.services.practice_workflow_service import PracticeWorkflowService
from app.services.replay_service import ReplayService
from app.services.trade_lifecycle_service import TradeLifecycleService
from app.api.decisions import register_order_cancellation, clear_order_cancellation_registry


@pytest.fixture
def isolated_env(tmp_path) -> Generator[tuple[Session, TestClient, str], None, None]:
    """Provides an isolated temporary SQLite database and TestClient with get_db overridden."""
    clear_order_cancellation_registry()
    db_file = tmp_path / "test_r4_isolated.db"
    engine = create_engine(
        f"sqlite:///{db_file}",
        connect_args={"check_same_thread": False},
    )
    Base.metadata.create_all(bind=engine)
    TestingSessionLocal = sessionmaker(autocommit=False, autoflush=False, bind=engine)

    def override_get_db():
        db = TestingSessionLocal()
        try:
            yield db
        finally:
            db.close()

    app.dependency_overrides[get_db] = override_get_db
    session = TestingSessionLocal()
    with TestClient(app) as client:
        yield session, client, str(db_file)
    session.close()
    app.dependency_overrides.pop(get_db, None)
    clear_order_cancellation_registry()


def _seed_candles(db: Session, symbol: str, start_d: date, count: int) -> list[Candle]:
    candles = []
    for i in range(count):
        c_date = start_d + timedelta(days=i)
        c = Candle(
            symbol=symbol,
            timeframe="1D",
            adjustment_type="unadjusted",
            timestamp=datetime.combine(c_date, datetime.min.time()),
            open=100.0 + i,
            high=105.0 + i,
            low=95.0 + i,
            close=100.0 + i,
            volume=100000 + i * 100,
        )
        db.add(c)
        candles.append(c)
    db.commit()
    return candles


def test_r4_order_future_cancellation_masking_via_candle_index(isolated_env):
    """Verify that an order created before cursor K and cancelled after K is returned with status PENDING at K."""
    db, client, _ = isolated_env
    symbol = "ORD_CANCEL_IDX"
    start_d = date(2024, 1, 1)
    candles = _seed_candles(db, symbol, start_d, 50)

    session = ReplayService.create_session(
        db,
        ReplaySessionCreate(
            symbol=symbol,
            start_date=start_d,
            end_date=start_d + timedelta(days=49),
            initial_cash=100_000_000.0,
        ),
    )
    session_id = session.id

    # Advance to bar 10
    ReplayService.next_candle(db, session_id, steps=10)
    session = ReplayService.get_session(db, session_id)
    assert session.current_index == 10

    # Create a decision and order at bar 10
    decision = Decision(
        session_id=session_id,
        symbol=symbol,
        decision_date=candles[10].timestamp,
        candle_index=10,
        action="BUY",
        price=candles[10].close,
        reason="Limit buy placed at bar 10",
    )
    db.add(decision)
    db.flush()

    # Order created at bar 10, but cancelled at bar 20
    order = Order(
        session_id=session_id,
        decision_id=decision.id,
        symbol=symbol,
        side=OrderSide.BUY.value,
        order_type=OrderType.LIMIT.value,
        requested_price=90.0,
        quantity=500.0,
        status="cancelled",
        created_at=candles[10].timestamp,
    )
    db.add(order)
    db.flush()
    order_id = order.id
    order.cancellation_candle_index = 20
    register_order_cancellation(order_id, candle_index=20)
    db.commit()

    # 1. Inspect at cursor K = 5 (before order creation bar 10)
    ReplayService.previous_candle(db, session_id, steps=5)
    assert ReplayService.get_session(db, session_id).current_index == 5
    res_k5 = client.get(f"/api/replay/sessions/{session_id}/orders")
    assert res_k5.status_code == 200
    assert len(res_k5.json()) == 0  # Order not yet created at bar 5

    # 2. Inspect at cursor K = 15 (after creation bar 10, but before cancellation bar 20)
    ReplayService.next_candle(db, session_id, steps=10)
    assert ReplayService.get_session(db, session_id).current_index == 15
    res_k15 = client.get(f"/api/replay/sessions/{session_id}/orders")
    assert res_k15.status_code == 200
    orders_k15 = res_k15.json()
    assert len(orders_k15) == 1
    ord_k15 = orders_k15[0]
    assert ord_k15["id"] == order_id
    # CRITICAL: Status must be masked to 'pending', NOT leaking future 'cancelled' status!
    assert ord_k15["status"].lower() == "pending"
    assert ord_k15["filled_price"] is None
    assert ord_k15["filled_at"] is None

    # Verify database immutability: underlying SQLite row status MUST STILL BE 'cancelled'
    db_order = db.query(Order).filter(Order.id == order_id).first()
    assert db_order.status == "cancelled"

    # 3. Inspect at cursor K = 20 (cancellation occurs at bar 20)
    ReplayService.next_candle(db, session_id, steps=5)
    assert ReplayService.get_session(db, session_id).current_index == 20
    res_k20 = client.get(f"/api/replay/sessions/{session_id}/orders")
    assert res_k20.status_code == 200
    orders_k20 = res_k20.json()
    assert len(orders_k20) == 1
    # At cursor 20, cancellation has occurred -> unmasked 'cancelled'
    assert orders_k20[0]["status"].lower() == "cancelled"

    # 4. Inspect at cursor K = 25 (after cancellation)
    ReplayService.next_candle(db, session_id, steps=5)
    assert ReplayService.get_session(db, session_id).current_index == 25
    res_k25 = client.get(f"/api/replay/sessions/{session_id}/orders")
    assert res_k25.status_code == 200
    assert res_k25.json()[0]["status"].lower() == "cancelled"


def test_r4_order_future_cancellation_masking_via_timestamp_and_event_log(isolated_env):
    """Verify that order future cancellation masking works with cancelled_at timestamp and EventLog."""
    db, client, _ = isolated_env
    symbol = "ORD_CANCEL_TS"
    start_d = date(2024, 1, 1)
    candles = _seed_candles(db, symbol, start_d, 50)

    session = ReplayService.create_session(
        db,
        ReplaySessionCreate(
            symbol=symbol,
            start_date=start_d,
            end_date=start_d + timedelta(days=49),
            initial_cash=100_000_000.0,
        ),
    )
    session_id = session.id

    # Place decision and order at bar 8
    decision = Decision(
        session_id=session_id,
        symbol=symbol,
        decision_date=candles[8].timestamp,
        candle_index=8,
        action="BUY",
        price=candles[8].close,
        reason="Order placed at bar 8",
    )
    db.add(decision)
    db.flush()

    order = Order(
        session_id=session_id,
        decision_id=decision.id,
        symbol=symbol,
        side=OrderSide.BUY.value,
        order_type=OrderType.LIMIT.value,
        requested_price=85.0,
        quantity=300.0,
        status="cancelled",
        created_at=candles[8].timestamp,
    )
    db.add(order)
    db.flush()
    order_id = order.id
    order.cancelled_at = candles[22].timestamp
    register_order_cancellation(order_id, cancelled_at=candles[22].timestamp)

    # Also add an EventLog recording cancellation at candle 22
    ev = EventLog(
        session_id=session_id,
        event_type="order_cancelled",
        message=f"Order {order_id} was cancelled by user",
        details=json.dumps({"order_id": order_id, "reason": "User cancelled", "candle_index": 22}),
        created_at=candles[22].timestamp,
    )
    db.add(ev)
    db.commit()

    # Move replay to bar 15 (between creation at 8 and cancel at 22)
    ReplayService.next_candle(db, session_id, steps=15)
    assert ReplayService.get_session(db, session_id).current_index == 15

    res = client.get(f"/api/replay/sessions/{session_id}/orders")
    assert res.status_code == 200
    orders = res.json()
    assert len(orders) == 1
    # Must be masked to pending at bar 15
    assert orders[0]["status"].lower() == "pending"

    # Advance to bar 22
    ReplayService.next_candle(db, session_id, steps=7)
    assert ReplayService.get_session(db, session_id).current_index == 22

    res_22 = client.get(f"/api/replay/sessions/{session_id}/orders")
    assert res_22.status_code == 200
    assert res_22.json()[0]["status"].lower() == "cancelled"


def test_r4_order_future_cancellation_masking_via_cancel_decision(isolated_env):
    """Verify that order future cancellation masking resolves cancellation from a CANCEL decision record."""
    db, client, _ = isolated_env
    symbol = "ORD_CANCEL_DEC"
    start_d = date(2024, 1, 1)
    candles = _seed_candles(db, symbol, start_d, 50)

    session = ReplayService.create_session(
        db,
        ReplaySessionCreate(
            symbol=symbol,
            start_date=start_d,
            end_date=start_d + timedelta(days=49),
            initial_cash=100_000_000.0,
        ),
    )
    session_id = session.id

    # Place decision and order at bar 5
    d1 = Decision(
        session_id=session_id,
        symbol=symbol,
        decision_date=candles[5].timestamp,
        candle_index=5,
        action="BUY",
        price=candles[5].close,
        reason="Order placed at bar 5",
    )
    db.add(d1)
    db.flush()

    order = Order(
        session_id=session_id,
        decision_id=d1.id,
        symbol=symbol,
        side=OrderSide.BUY.value,
        order_type=OrderType.LIMIT.value,
        requested_price=70.0,
        quantity=200.0,
        status="cancelled",
        created_at=candles[5].timestamp,
    )
    db.add(order)
    db.flush()

    # Cancel decision at bar 18 referencing the order
    d_cancel = Decision(
        session_id=session_id,
        symbol=symbol,
        decision_date=candles[18].timestamp,
        candle_index=18,
        action="CANCEL",
        reason=f"Cancel order {order.id}",
        note=f"Order {order.id} cancelled at bar 18",
    )
    db.add(d_cancel)
    db.commit()

    # At bar 12 (before cancel bar 18): order status must be masked to pending
    ReplayService.next_candle(db, session_id, steps=12)
    assert ReplayService.get_session(db, session_id).current_index == 12

    res_12 = client.get(f"/api/replay/sessions/{session_id}/orders")
    assert res_12.status_code == 200
    assert res_12.json()[0]["status"].lower() == "pending"

    # At bar 18: unmasked to cancelled
    ReplayService.next_candle(db, session_id, steps=6)
    assert ReplayService.get_session(db, session_id).current_index == 18

    res_18 = client.get(f"/api/replay/sessions/{session_id}/orders")
    assert res_18.status_code == 200
    assert res_18.json()[0]["status"].lower() == "cancelled"


def test_r4_position_rewind_isolation_reflects_open_state_without_db_mutation(isolated_env):
    """Verify that /sessions/{id}/position reflects open state when closed after cursor K, without mutating persisted rows."""
    db, client, _ = isolated_env
    symbol = "POS_REWIND_TEST"
    start_d = date(2024, 1, 1)
    candles = _seed_candles(db, symbol, start_d, 60)

    session = ReplayService.create_session(
        db,
        ReplaySessionCreate(
            symbol=symbol,
            start_date=start_d,
            end_date=start_d + timedelta(days=59),
            initial_cash=100_000_000.0,
        ),
    )
    session_id = session.id

    # 1. Step to bar 10 and BUY 1,000 shares
    ReplayService.next_candle(db, session_id, steps=10)
    c10_close = candles[10].close
    TradeLifecycleService.process_decision(
        db,
        session_id,
        DecisionCreate(
            action=DecisionAction.BUY,
            quantity=1000,
            price=c10_close,
            order_type="MARKET_AT_CLOSE",
            reason="Bar 10 BUY 1000 shares",
        ),
    )

    # 2. Advance to bar 25 and CLOSE the entire position
    ReplayService.next_candle(db, session_id, steps=15)
    session = ReplayService.get_session(db, session_id)
    assert session.current_index == 25
    c25_close = candles[25].close
    TradeLifecycleService.process_decision(
        db,
        session_id,
        DecisionCreate(
            action=DecisionAction.CLOSE,
            price=c25_close,
            order_type="MARKET_AT_CLOSE",
            reason="Bar 25 CLOSE",
        ),
    )

    # 3. Verify at bar 25: position is closed -> endpoint returns empty list []
    res_pos_25 = client.get(f"/api/replay/sessions/{session_id}/position")
    assert res_pos_25.status_code == 200
    assert res_pos_25.json() == []

    # Verify that underlying DB row in positions is indeed marked 'closed' with quantity 0
    db_pos = db.query(Position).filter(Position.session_id == session_id).first()
    assert db_pos is not None
    assert db_pos.status == "closed"
    assert db_pos.quantity == 0.0
    assert db_pos.closed_at is not None

    # 4. REWIND to bar 15 (between BUY at 10 and CLOSE at 25)
    ReplayService.previous_candle(db, session_id, steps=10)
    session = ReplayService.get_session(db, session_id)
    assert session.current_index == 15

    # 5. Query /sessions/{id}/position at cursor 15:
    # Must project the position as OPEN at cursor 15!
    res_pos_15 = client.get(f"/api/replay/sessions/{session_id}/position")
    assert res_pos_15.status_code == 200
    positions_15 = res_pos_15.json()
    assert len(positions_15) == 1
    p15 = positions_15[0]
    assert p15["symbol"] == symbol
    assert p15["quantity"] == 1000.0
    assert p15["average_price"] == c10_close
    assert p15["total_cost"] == 1000.0 * c10_close
    assert p15["status"] == "open"
    assert p15["closed_at"] is None
    # Unrealized PnL dynamically computed from candle 15 close
    c15_close = candles[15].close
    expected_unrealized_pnl = (c15_close - c10_close) * 1000.0
    assert abs(p15["unrealized_pnl"] - expected_unrealized_pnl) < 1e-4

    # Also test via /api/sessions/... route prefix
    res_compat = client.get(f"/api/sessions/{session_id}/position")
    assert res_compat.status_code == 200
    assert len(res_compat.json()) == 1

    # 6. INVARIANT CHECK: Underlying database row in SQLite MUST NEVER BE MUTATED by rewind!
    db_pos_after_rewind = db.query(Position).filter(Position.session_id == session_id).first()
    assert db_pos_after_rewind.status == "closed"
    assert db_pos_after_rewind.quantity == 0.0
    assert db_pos_after_rewind.closed_at is not None

    # 7. REWIND to bar 5 (before BUY at bar 10)
    ReplayService.previous_candle(db, session_id, steps=10)
    session = ReplayService.get_session(db, session_id)
    assert session.current_index == 5

    # Position opened at bar 10 MUST NOT be visible at cursor 5!
    res_pos_5 = client.get(f"/api/replay/sessions/{session_id}/position")
    assert res_pos_5.status_code == 200
    assert res_pos_5.json() == []


def test_r4_post_session_audit_eligibility_enforcement(isolated_env):
    """Verify include_future=True is rejected during active replay and permitted only for terminal/final-bar sessions."""
    db, client, _ = isolated_env
    symbol = "AUDIT_ELIG"
    start_d = date(2024, 1, 1)
    candles = _seed_candles(db, symbol, start_d, 40)

    session = ReplayService.create_session(
        db,
        ReplaySessionCreate(
            symbol=symbol,
            start_date=start_d,
            end_date=start_d + timedelta(days=39),
            initial_cash=100_000_000.0,
        ),
    )
    session_id = session.id

    # Advance to bar 15 (active replay mid-session)
    ReplayService.next_candle(db, session_id, steps=15)
    session = ReplayService.get_session(db, session_id)
    assert session.status.upper() == "ACTIVE"
    assert session.current_index == 15

    # Place buy at bar 15
    TradeLifecycleService.process_decision(
        db,
        session_id,
        DecisionCreate(
            action=DecisionAction.BUY,
            quantity=500,
            price=candles[15].close,
            order_type="MARKET_AT_CLOSE",
            reason="Bar 15 BUY",
        ),
    )

    # 1. During active replay, include_future=True is strictly forbidden (HTTP 403)
    r_pos = client.get(f"/api/replay/sessions/{session_id}/position?include_future=true")
    assert r_pos.status_code == 403
    assert "include_future is strictly prohibited during active replay" in r_pos.json()["detail"]

    r_ord = client.get(f"/api/replay/sessions/{session_id}/orders?include_future=true")
    assert r_ord.status_code == 403

    r_dec = client.get(f"/api/replay/sessions/{session_id}/decisions?include_future=true")
    assert r_dec.status_code == 403

    r_trd = client.get(f"/api/replay/sessions/{session_id}/trades?include_future=true")
    assert r_trd.status_code == 403

    # 2. Advance to final bar (bar 39)
    ReplayService.next_candle(db, session_id, steps=24)
    session = ReplayService.get_session(db, session_id)
    assert session.current_index == 39

    # At final bar, include_future=True is permitted even while status is ACTIVE
    r_pos_final = client.get(f"/api/replay/sessions/{session_id}/position?include_future=true")
    assert r_pos_final.status_code == 200

    r_ord_final = client.get(f"/api/replay/sessions/{session_id}/orders?include_future=true")
    assert r_ord_final.status_code == 200

    # 3. Rewind back to bar 5 and mark session terminal: COMPLETED
    ReplayService.previous_candle(db, session_id, steps=34)
    session = ReplayService.get_session(db, session_id)
    assert session.current_index == 5
    session.status = SessionStatus.COMPLETED.value
    db.commit()

    # Even though current_index is 5, COMPLETED status permits full unmasked post-session audit!
    r_pos_completed = client.get(f"/api/replay/sessions/{session_id}/position?include_future=true")
    assert r_pos_completed.status_code == 200
    assert len(r_pos_completed.json()) == 1  # Unmasked position from bar 15 is returned!

    r_ord_completed = client.get(f"/api/replay/sessions/{session_id}/orders?include_future=true")
    assert r_ord_completed.status_code == 200
    assert len(r_ord_completed.json()) == 1

    # But without include_future=true, cursor 5 still properly masks future data
    r_pos_masked = client.get(f"/api/replay/sessions/{session_id}/position?include_future=false")
    assert r_pos_masked.status_code == 200
    assert r_pos_masked.json() == []  # Not visible at cursor 5


def test_r4_direct_db_fixture_lookahead_isolation(isolated_env):
    """Verify fallback lookahead projection on raw database rows without execution records."""
    db, client, _ = isolated_env
    symbol = "RAW_DB_TEST"
    start_d = date(2024, 1, 1)
    candles = _seed_candles(db, symbol, start_d, 40)

    session = ReplayService.create_session(
        db,
        ReplaySessionCreate(
            symbol=symbol,
            start_date=start_d,
            end_date=start_d + timedelta(days=39),
            initial_cash=100_000_000.0,
        ),
    )
    session_id = session.id

    # Create raw Position row in SQLite (opened at bar 10, closed at bar 25)
    pos = Position(
        session_id=session_id,
        symbol=symbol,
        quantity=0.0,
        average_price=110.0,
        total_cost=0.0,
        realized_pnl=5000.0,
        unrealized_pnl=0.0,
        status="closed",
        opened_at=candles[10].timestamp,
        closed_at=candles[25].timestamp,
    )
    # Also add a Trade so fallback can determine original trade quantity
    trade = Trade(
        session_id=session_id,
        symbol=symbol,
        entry_date=candles[10].timestamp,
        entry_price=110.0,
        quantity=500.0,
        exit_date=candles[25].timestamp,
        exit_price=120.0,
        status="closed",
    )
    db.add(pos)
    db.add(trade)
    db.commit()

    # 1. At bar 5 (before open at 10): returns []
    ReplayService.next_candle(db, session_id, steps=5)
    res_5 = client.get(f"/api/replay/sessions/{session_id}/position")
    assert res_5.status_code == 200
    assert res_5.json() == []

    # 2. At bar 15 (between open 10 and close 25): returns open position
    ReplayService.next_candle(db, session_id, steps=10)
    assert ReplayService.get_session(db, session_id).current_index == 15
    res_15 = client.get(f"/api/replay/sessions/{session_id}/position")
    assert res_15.status_code == 200
    positions = res_15.json()
    assert len(positions) == 1
    assert positions[0]["status"] == "open"
    assert positions[0]["quantity"] == 500.0
    assert positions[0]["closed_at"] is None

    # 3. At bar 30 (after close at 25): returns []
    ReplayService.next_candle(db, session_id, steps=15)
    assert ReplayService.get_session(db, session_id).current_index == 30
    res_30 = client.get(f"/api/replay/sessions/{session_id}/position")
    assert res_30.status_code == 200
    assert res_30.json() == []


def test_r4_database_immutability_invariant():
    """Verify backend/sumi.db was never modified or corrupted."""
    db_path = Path("backend/sumi.db")
    assert db_path.exists(), "backend/sumi.db must exist"
    assert db_path.stat().st_size == 188416, f"Expected size 188416, got {db_path.stat().st_size}"
    actual_hash = hashlib.sha256(db_path.read_bytes()).hexdigest()
    expected_hash = "234206fc75259f091337d4feb5f41575d0fe5ac02d6c10ba0de8fad68cbcee86"
    assert actual_hash == expected_hash, f"Hash mismatch: {actual_hash} != {expected_hash}"
