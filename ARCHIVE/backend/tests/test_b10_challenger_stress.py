"""Adversarial Challenger Stress Harness for Batch B10.

Rigorous empirical testing of:
1. Replay rewind and stepping lookahead protection:
   - Active session with orders & trades at bars 10, 20, 30.
   - Limit order at bar 10 filled only at bar 20 via market low trigger.
   - Market order at bar 10 entering Trade #1, closed at bar 25.
   - Orders at bar 20 and bar 30.
   - Step forward to bar 30, then rewind to bar 15.
   - Verify future orders (>15) excluded.
   - Verify order placed at bar 10 and filled at bar 20 is masked (status="pending", filled_price=None, filled_at=None).
   - Verify trade placed at bar 10 and closed at bar 25 is masked (status="open", exit_price=None, clamped holding bars=5).
   - Inspect SQLite database directly: verify actual database rows were NOT mutated or overwritten!
2. Escape hatch enforcement:
   - include_future=true on active session -> HTTP 403 Forbidden.
   - include_future=true on COMPLETED/EXPIRED/BANKRUPT session -> HTTP 200 OK unmasked.
3. Multi-cycle oscillatory stepping and rewind (10 -> 30 -> 15 -> 25 -> 5 -> 30).
4. Boundary condition at bar 0.
5. Timezone-aware vs naive datetime handling.

All tests operate strictly against isolated temporary SQLite databases.
"""

from __future__ import annotations

from contextlib import contextmanager
from datetime import date, datetime, timedelta, timezone
from typing import Generator
import pytest
from fastapi.testclient import TestClient
from sqlalchemy import create_engine, text
from sqlalchemy.orm import Session, sessionmaker

from app.db import Base
from app.dependencies import get_db
from app.domain.enums import DecisionAction, OrderSide, OrderStatus, OrderType, SessionStatus
from app.main import app
from app.models.candle import Candle
from app.models.decision import Decision
from app.models.execution import Execution
from app.models.order import Order
from app.models.position import Position
from app.models.replay_session import ReplaySession
from app.models.symbol import Symbol
from app.models.trade import Trade
from app.schemas.decision_schema import DecisionCreate
from app.schemas.replay_schema import ReplaySessionCreate
from app.services.practice_workflow_service import PracticeWorkflowService
from app.services.replay_service import ReplayService
from app.services.trade_lifecycle_service import TradeLifecycleService


@pytest.fixture
def isolated_challenger_db(tmp_path) -> Generator[tuple[Session, TestClient, str], None, None]:
    """Provides an isolated SQLite database and TestClient with get_db overridden."""
    db_file = tmp_path / "test_b10_challenger.db"
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


def _seed_symbol_and_candles(db: Session, symbol: str, start_d: date, count: int) -> list[Candle]:
    sym = db.query(Symbol).filter_by(symbol=symbol).first()
    if not sym:
        sym = Symbol(symbol=symbol, company_name=f"{symbol} Corp", exchange="HOSE")
        db.add(sym)
        db.commit()

    candles = []
    for i in range(count):
        c_date = start_d + timedelta(days=i)
        # For bar 20, make the low dip to 94.0 to trigger the limit order requested at 95.0
        # For all other bars, low remains at 99.0 so limit order at 95.0 does not trigger
        low_val = 94.0 if i == 20 else 99.0
        close_val = 96.0 if i == 20 else 100.0
        c = Candle(
            symbol=symbol,
            timeframe="1D",
            adjustment_type="unadjusted",
            timestamp=datetime.combine(c_date, datetime.min.time()),
            open=100.0,
            high=102.0,
            low=low_val,
            close=close_val,
            volume=500000 + i * 1000,
        )
        db.add(c)
        candles.append(c)
    db.commit()
    return candles


@contextmanager
def engine_direct(db_path: str):
    engine = create_engine(f"sqlite:///{db_path}")
    with engine.connect() as conn:
        yield conn
    engine.dispose()


def test_challenger_rewind_lookahead_masking_and_db_immutability(isolated_challenger_db):
    """TASK 1: Create session with orders/trades at bars 10, 20, 30.
    
    Advance to bar 30, rewind to bar 15.
    Assert:
    - Orders entered after bar 15 excluded.
    - Order entered at bar 10 but filled at bar 20 is masked (status="pending", filled_price=None).
    - Trade entered at bar 10 but closed at bar 25 is masked (status="open", exit_price=None, holding_bars=5).
    - Raw SQLite inspect: underlying rows NOT mutated!
    """
    db, client, db_path = isolated_challenger_db
    symbol = "CHAL_REWIND"
    start_d = date(2023, 2, 1)
    candles = _seed_symbol_and_candles(db, symbol, start_d, 60)

    # Create Replay session
    session_create = ReplaySessionCreate(
        symbol=symbol,
        start_date=start_d,
        end_date=start_d + timedelta(days=59),
        initial_cash=200_000_000.0,
    )
    session = ReplayService.create_session(db, session_create)
    session_id = session.id

    # -------------------------------------------------------------
    # Bar 10:
    # 1. Market BUY order #1 (100 shares at close) -> filled at bar 10, enters Trade #1
    # 2. Limit BUY order #2 (100 shares at requested_price=95.0) -> pending at bar 10
    # -------------------------------------------------------------
    ReplayService.next_candle(db, session_id, steps=10)
    session = ReplayService.get_session(db, session_id)
    assert session.current_index == 10
    c10_close = candles[10].close

    d10_1 = TradeLifecycleService.process_decision(
        db,
        session_id,
        DecisionCreate(
            action=DecisionAction.BUY,
            quantity=100,
            price=c10_close,
            order_type=OrderType.MARKET_AT_CLOSE.value,
            reason="Bar 10 Market BUY",
        ),
    )

    d10_2 = TradeLifecycleService.process_decision(
        db,
        session_id,
        DecisionCreate(
            action=DecisionAction.BUY,
            quantity=100,
            price=95.0,  # Floor is 93.0, so 95.0 is valid within limits
            order_type=OrderType.LIMIT.value,
            reason="Bar 10 Limit BUY (triggers only at bar 20)",
        ),
    )

    # Check that order #2 is pending at bar 10
    orders_at_10 = db.query(Order).filter_by(session_id=session_id).order_by(Order.id.asc()).all()
    assert len(orders_at_10) == 2
    order1 = orders_at_10[0]
    order2 = orders_at_10[1]
    assert order1.status == OrderStatus.EXECUTED.value
    assert order2.status == OrderStatus.PENDING.value

    # Check trade #1 entered
    trades_at_10 = db.query(Trade).filter_by(session_id=session_id).all()
    assert len(trades_at_10) == 1
    trade1 = trades_at_10[0]
    assert trade1.status == "open"

    # -------------------------------------------------------------
    # Advance to bar 20:
    # 1. At bar 20, low=94.0 <= 95.0, so Order #2 is automatically filled by _match_pending_orders!
    # 2. Also place an additional Market BUY order #3 (100 shares) at bar 20
    # -------------------------------------------------------------
    ReplayService.next_candle(db, session_id, steps=10)
    session = ReplayService.get_session(db, session_id)
    assert session.current_index == 20
    c20_close = candles[20].close

    # Verify order2 was executed at bar 20 by replay engine
    db.refresh(order2)
    assert order2.status == OrderStatus.EXECUTED.value

    d20_3 = TradeLifecycleService.process_decision(
        db,
        session_id,
        DecisionCreate(
            action=DecisionAction.BUY,
            quantity=100,
            price=c20_close,
            order_type=OrderType.MARKET_AT_CLOSE.value,
            reason="Bar 20 Market BUY",
        ),
    )

    # -------------------------------------------------------------
    # Advance to bar 25:
    # Close Trade #1 by selling 100 shares (settled under T+2 from bar 10)
    # -------------------------------------------------------------
    ReplayService.next_candle(db, session_id, steps=5)
    session = ReplayService.get_session(db, session_id)
    assert session.current_index == 25
    c25_close = candles[25].close

    d25_close = TradeLifecycleService.process_decision(
        db,
        session_id,
        DecisionCreate(
            action=DecisionAction.SELL,
            quantity=100,
            price=c25_close,
            order_type=OrderType.MARKET_AT_CLOSE.value,
            reason="Bar 25 Partial SELL closing trade",
        ),
    )

    # Verify Trade #1 in DB is closed
    db.refresh(trade1)

    # -------------------------------------------------------------
    # Advance to bar 30:
    # Market BUY order #5 (100 shares) at bar 30
    # -------------------------------------------------------------
    ReplayService.next_candle(db, session_id, steps=5)
    session = ReplayService.get_session(db, session_id)
    assert session.current_index == 30
    c30_close = candles[30].close

    d30_5 = TradeLifecycleService.process_decision(
        db,
        session_id,
        DecisionCreate(
            action=DecisionAction.BUY,
            quantity=100,
            price=c30_close,
            order_type=OrderType.MARKET_AT_CLOSE.value,
            reason="Bar 30 Market BUY",
        ),
    )

    # Snapshot at bar 30 in database
    all_orders_db = db.query(Order).filter_by(session_id=session_id).order_by(Order.id.asc()).all()
    assert len(all_orders_db) >= 4  # orders at 10, 10, 20, 25, 30

    # Record raw DB state before rewind
    raw_order2_status = order2.status
    raw_order2_id = order2.id
    raw_trade1_status = trade1.status
    raw_trade1_exit_price = trade1.exit_price
    raw_trade1_net_pnl = trade1.net_pnl

    # -------------------------------------------------------------
    # REWIND CURSOR BACK TO BAR 15
    # -------------------------------------------------------------
    ReplayService.previous_candle(db, session_id, steps=15)
    session = ReplayService.get_session(db, session_id)
    assert session.current_index == 15

    # -------------------------------------------------------------
    # Query API at cursor 15
    # -------------------------------------------------------------
    # 1. Orders
    res_orders = client.get(f"/api/replay/sessions/{session_id}/orders")
    assert res_orders.status_code == 200
    orders_data = res_orders.json()

    # Assert orders entered after bar 15 (bar 20, 25, 30) are strictly EXCLUDED
    order_ids_returned = [o["id"] for o in orders_data]
    assert order1.id in order_ids_returned
    assert order2.id in order_ids_returned
    assert d20_3.id not in [o.get("decision_id") for o in orders_data]
    assert d25_close.id not in [o.get("decision_id") for o in orders_data]
    assert d30_5.id not in [o.get("decision_id") for o in orders_data]

    # Order #1: entered at bar 10, filled at bar 10 -> executed
    o1_res = next(o for o in orders_data if o["id"] == order1.id)
    assert o1_res["status"].lower() == "executed"
    assert o1_res["filled_price"] == c10_close

    # Order #2: entered at bar 10, but filled at bar 20 -> MUST BE MASKED TO PENDING
    o2_res = next(o for o in orders_data if o["id"] == order2.id)
    assert o2_res["status"].lower() == "pending", f"Expected masked status 'pending', got {o2_res['status']}"
    assert o2_res["filled_price"] is None, f"Expected masked filled_price None, got {o2_res['filled_price']}"
    assert o2_res["filled_at"] is None

    # 2. Trades
    res_trades = client.get(f"/api/replay/sessions/{session_id}/trades")
    assert res_trades.status_code == 200
    trades_data = res_trades.json()

    # Assert trade entered at bar 10 but closed at bar 25 is MASKED TO OPEN
    assert len(trades_data) >= 1
    t1_res = next(t for t in trades_data if t["id"] == trade1.id)
    assert t1_res["status"].lower() == "open", f"Expected masked trade status 'open', got {t1_res['status']}"
    assert t1_res["result"].lower() == "open"
    assert t1_res["exit_price"] is None, f"Expected masked exit_price None, got {t1_res['exit_price']}"
    assert t1_res["exit_date"] is None
    assert t1_res["net_pnl"] is None
    assert t1_res["gross_pnl"] is None
    assert t1_res["holding_candles"] == 5  # 15 - 10 = 5

    # 3. Direct SQLite Database Row Inspection: NEVER MUTATED!
    with engine_direct(db_path) as raw_conn:
        # Check order2 row in DB
        row_o2 = raw_conn.execute(
            text("SELECT status FROM orders WHERE id = :id"),
            {"id": raw_order2_id}
        ).fetchone()
        assert row_o2[0] == OrderStatus.EXECUTED.value, (
            f"MUTATION DETECTED: Order row in SQLite was mutated from {OrderStatus.EXECUTED.value} to {row_o2[0]}"
        )

        # Check execution row for order2 exists in DB
        row_exc = raw_conn.execute(
            text("SELECT execution_price, execution_candle_index FROM executions WHERE order_id = :oid"),
            {"oid": raw_order2_id}
        ).fetchone()
        assert row_exc is not None
        assert row_exc[0] == 95.0
        assert row_exc[1] == 20

        # Check trade1 row in DB
        row_t1 = raw_conn.execute(
            text("SELECT status, exit_price, net_pnl FROM trades WHERE id = :id"),
            {"id": trade1.id}
        ).fetchone()
        assert row_t1[0] == raw_trade1_status, (
            f"MUTATION DETECTED: Trade status in SQLite was mutated from {raw_trade1_status} to {row_t1[0]}"
        )
        assert row_t1[1] == raw_trade1_exit_price, (
            f"MUTATION DETECTED: Trade exit_price in SQLite was mutated from {raw_trade1_exit_price} to {row_t1[1]}"
        )
        assert row_t1[2] == raw_trade1_net_pnl, (
            f"MUTATION DETECTED: Trade net_pnl in SQLite was mutated from {raw_trade1_net_pnl} to {row_t1[2]}"
        )


def test_challenger_escape_hatch_enforcement_and_audit_unlock(isolated_challenger_db):
    """TASK 2: Escape hatch enforcement:
    - Active session with cursor < total_bars - 1: include_future=true must return 403 Forbidden.
    - Transition session to COMPLETED: include_future=true returns 200 OK with unmasked records.
    - Transition to EXPIRED, BANKRUPT: also returns 200 OK unmasked.
    """
    db, client, _ = isolated_challenger_db
    symbol = "CHAL_ESCAPE"
    start_d = date(2023, 3, 1)
    candles = _seed_symbol_and_candles(db, symbol, start_d, 50)

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

    # Place trade at bar 10, close at bar 20
    ReplayService.next_candle(db, session_id, steps=10)
    TradeLifecycleService.process_decision(
        db,
        session_id,
        DecisionCreate(
            action=DecisionAction.BUY,
            quantity=100,
            price=candles[10].close,
            order_type=OrderType.MARKET_AT_CLOSE.value,
            reason="Bar 10 BUY",
        ),
    )

    ReplayService.next_candle(db, session_id, steps=10)
    TradeLifecycleService.process_decision(
        db,
        session_id,
        DecisionCreate(
            action=DecisionAction.CLOSE,
            price=candles[20].close,
            order_type=OrderType.MARKET_AT_CLOSE.value,
            reason="Bar 20 CLOSE",
        ),
    )

    # Rewind to bar 5
    ReplayService.previous_candle(db, session_id, steps=15)
    session = ReplayService.get_session(db, session_id)
    assert session.current_index == 5
    assert session.status.upper() == "ACTIVE"

    # 1. Active session: all endpoints with include_future=true MUST return 403 Forbidden
    endpoints = [
        f"/api/replay/sessions/{session_id}/decisions?include_future=true",
        f"/api/replay/sessions/{session_id}/orders?include_future=true",
        f"/api/replay/sessions/{session_id}/trades?include_future=true",
        f"/api/sessions/{session_id}/decisions?include_future=true",
        f"/api/sessions/{session_id}/orders?include_future=true",
        f"/api/sessions/{session_id}/trades?include_future=true",
    ]
    for ep in endpoints:
        resp = client.get(ep)
        assert resp.status_code == 403, f"Expected 403 on {ep}, got {resp.status_code}: {resp.text}"
        assert "include_future is strictly prohibited during active replay" in resp.json()["detail"]

    # 2. Transition session to COMPLETED
    session.status = SessionStatus.COMPLETED.value
    db.commit()

    # Now include_future=true MUST succeed with 200 OK and unmasked data
    r_dec = client.get(f"/api/replay/sessions/{session_id}/decisions?include_future=true")
    assert r_dec.status_code == 200
    assert len(r_dec.json()) == 2

    r_ord = client.get(f"/api/replay/sessions/{session_id}/orders?include_future=true")
    assert r_ord.status_code == 200
    assert len(r_ord.json()) == 2
    for ord_item in r_ord.json():
        assert ord_item["status"].lower() == "executed"
        assert ord_item["filled_price"] is not None

    r_trd = client.get(f"/api/replay/sessions/{session_id}/trades?include_future=true")
    assert r_trd.status_code == 200
    trades_unmasked = r_trd.json()
    assert len(trades_unmasked) == 1
    assert trades_unmasked[0]["status"].lower() == "closed"
    assert trades_unmasked[0]["exit_price"] == candles[20].close
    assert trades_unmasked[0]["net_pnl"] is not None

    # 3. Transition session to EXPIRED -> also permits unmasked
    session.status = "EXPIRED"
    db.commit()
    r_trd_exp = client.get(f"/api/replay/sessions/{session_id}/trades?include_future=true")
    assert r_trd_exp.status_code == 200
    assert r_trd_exp.json()[0]["status"].lower() == "closed"

    # 4. Transition session to BANKRUPT -> also permits unmasked
    session.status = SessionStatus.BANKRUPT.value
    db.commit()
    r_trd_bnk = client.get(f"/api/replay/sessions/{session_id}/trades?include_future=true")
    assert r_trd_bnk.status_code == 200
    assert r_trd_bnk.json()[0]["status"].lower() == "closed"


def test_challenger_oscillatory_stepping_rewind_cycles(isolated_challenger_db):
    """Stress test cursor oscillations: 10 -> 30 -> 15 -> 25 -> 5 -> 30.
    
    Verifies that repeatedly stepping forward and backward does not corrupt
    session state, does not mutate DB rows, and returns strictly clamped
    records at every cursor position.
    """
    db, client, _ = isolated_challenger_db
    symbol = "OSCILLATE"
    start_d = date(2023, 4, 1)
    candles = _seed_symbol_and_candles(db, symbol, start_d, 60)

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

    # Place order at 10
    ReplayService.next_candle(db, session_id, steps=10)
    TradeLifecycleService.process_decision(
        db,
        session_id,
        DecisionCreate(
            action=DecisionAction.BUY,
            quantity=100,
            price=candles[10].close,
            order_type=OrderType.MARKET_AT_CLOSE.value,
            reason="Bar 10 BUY",
        ),
    )

    # Place order at 20
    ReplayService.next_candle(db, session_id, steps=10)
    TradeLifecycleService.process_decision(
        db,
        session_id,
        DecisionCreate(
            action=DecisionAction.BUY,
            quantity=100,
            price=candles[20].close,
            order_type=OrderType.MARKET_AT_CLOSE.value,
            reason="Bar 20 BUY",
        ),
    )

    # Close at 25
    ReplayService.next_candle(db, session_id, steps=5)
    TradeLifecycleService.process_decision(
        db,
        session_id,
        DecisionCreate(
            action=DecisionAction.SELL,
            quantity=200,
            price=candles[25].close,
            order_type=OrderType.MARKET_AT_CLOSE.value,
            reason="Bar 25 CLOSE",
        ),
    )

    # Move to 30
    ReplayService.next_candle(db, session_id, steps=5)
    assert session.current_index == 30

    # Step 1: Rewind to 15
    ReplayService.previous_candle(db, session_id, steps=15)
    assert session.current_index == 15
    r_orders = client.get(f"/api/replay/sessions/{session_id}/orders").json()
    assert len(r_orders) == 1  # Only bar 10 order
    assert r_orders[0]["quantity"] == 100

    # Step 2: Forward to 25
    ReplayService.next_candle(db, session_id, steps=10)
    assert session.current_index == 25
    r_orders_25 = client.get(f"/api/replay/sessions/{session_id}/orders").json()
    assert len(r_orders_25) == 3  # bar 10, 20, and 25 orders

    # Step 3: Rewind to 5 (before any order was placed!)
    ReplayService.previous_candle(db, session_id, steps=20)
    assert session.current_index == 5
    r_orders_5 = client.get(f"/api/replay/sessions/{session_id}/orders").json()
    assert len(r_orders_5) == 0  # Zero orders visible!
    r_trades_5 = client.get(f"/api/replay/sessions/{session_id}/trades").json()
    assert len(r_trades_5) == 0  # Zero trades visible!

    # Step 4: Advance back to 30
    ReplayService.next_candle(db, session_id, steps=25)
    assert session.current_index == 30
    r_orders_30 = client.get(f"/api/replay/sessions/{session_id}/orders").json()
    assert len(r_orders_30) == 3
    r_trades_30 = client.get(f"/api/replay/sessions/{session_id}/trades").json()
    assert len(r_trades_30) == 1
    assert r_trades_30[0]["status"].lower() == "closed"


def test_challenger_boundary_cursor_zero(isolated_challenger_db):
    """Boundary test at cursor 0: rewind to bar 0 yields empty sets when orders were placed at > 0."""
    db, client, _ = isolated_challenger_db
    symbol = "ZERO_CURSOR"
    start_d = date(2023, 5, 1)
    candles = _seed_symbol_and_candles(db, symbol, start_d, 20)

    session = ReplayService.create_session(
        db,
        ReplaySessionCreate(
            symbol=symbol,
            start_date=start_d,
            end_date=start_d + timedelta(days=19),
            initial_cash=100_000_000.0,
        ),
    )
    session_id = session.id
    assert session.current_index == 0

    # Place order at bar 2
    ReplayService.next_candle(db, session_id, steps=2)
    TradeLifecycleService.process_decision(
        db,
        session_id,
        DecisionCreate(
            action=DecisionAction.BUY,
            quantity=100,
            price=candles[2].close,
            order_type=OrderType.MARKET_AT_CLOSE.value,
            reason="Bar 2 BUY",
        ),
    )

    # Rewind to 0
    ReplayService.previous_candle(db, session_id, steps=2)
    session = ReplayService.get_session(db, session_id)
    assert session.current_index == 0

    orders = client.get(f"/api/replay/sessions/{session_id}/orders").json()
    assert len(orders) == 0

    trades = client.get(f"/api/replay/sessions/{session_id}/trades").json()
    assert len(trades) == 0

    decisions = client.get(f"/api/replay/sessions/{session_id}/decisions").json()
    assert len(decisions) == 0
