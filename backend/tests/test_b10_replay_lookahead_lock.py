"""Test Suite for Batch B10: Replay Ledger Zero-Lookahead, Future-State Masking, and Practice Horizon Clamping.

Covers:
- test_g10_active_replay_filters_future_records_and_masks_future_fills_and_exits
- test_g10_active_replay_rejects_unrestricted_include_future_escape_hatch
- test_g10_post_session_audit_eligibility_allows_completed_session
- test_practice_horizon_clamped_to_end_date_and_utc_normalized

All tests use isolated temporary SQLite databases (tmp_path). Production backend/sumi.db is never accessed.
"""

from __future__ import annotations

from datetime import date, datetime, timedelta, timezone
from typing import Generator
import pytest
from fastapi.testclient import TestClient
from sqlalchemy import create_engine
from sqlalchemy.orm import Session, sessionmaker

from app.db import Base
from app.dependencies import get_db
from app.domain.enums import DecisionAction, SessionStatus
from app.main import app
from app.models.candle import Candle
from app.models.decision import Decision
from app.models.order import Order
from app.models.replay_session import ReplaySession
from app.models.trade import Trade
from app.schemas.decision_schema import DecisionCreate
from app.schemas.replay_schema import ReplaySessionCreate
from app.services.practice_workflow_service import PracticeWorkflowService
from app.services.replay_service import ReplayService
from app.services.trade_lifecycle_service import TradeLifecycleService


@pytest.fixture
def isolated_env(tmp_path) -> Generator[tuple[Session, TestClient, str], None, None]:
    """Provides an isolated SQLite database and TestClient with get_db overridden."""
    db_file = tmp_path / "test_b10_isolated.db"
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


def test_g10_active_replay_filters_future_records_and_masks_future_fills_and_exits(isolated_env):
    """Verify that stepping backward filters future records and masks future fills/exits without mutating SQLite rows."""
    db, client, _ = isolated_env
    symbol = "REWIND_LOCK"
    start_d = date(2023, 1, 1)
    candles = _seed_candles(db, symbol, start_d, 100)

    # Create active replay session
    session_in = ReplaySessionCreate(
        symbol=symbol,
        start_date=start_d,
        end_date=start_d + timedelta(days=99),
        initial_cash=100_000_000.0,
    )
    session = ReplayService.create_session(db, session_in)
    session_id = session.id

    # Advance to bar 15 and place BUY order of 1,000 shares
    ReplayService.next_candle(db, session_id, steps=15)
    c15_close = candles[15].close
    d1 = TradeLifecycleService.process_decision(
        db,
        session_id,
        DecisionCreate(
            action=DecisionAction.BUY,
            quantity=1000,
            price=c15_close,
            order_type="MARKET_AT_CLOSE",
            reason="Bar 15 BUY",
        ),
    )

    # Advance to bar 30 and CLOSE the position
    ReplayService.next_candle(db, session_id, steps=15)
    c30_close = candles[30].close
    d2 = TradeLifecycleService.process_decision(
        db,
        session_id,
        DecisionCreate(
            action=DecisionAction.CLOSE,
            price=c30_close,
            order_type="MARKET_AT_CLOSE",
            reason="Bar 30 CLOSE",
        ),
    )

    # Advance to bar 45 and place second BUY order of 500 shares
    ReplayService.next_candle(db, session_id, steps=15)
    c45_close = candles[45].close
    d3 = TradeLifecycleService.process_decision(
        db,
        session_id,
        DecisionCreate(
            action=DecisionAction.BUY,
            quantity=500,
            price=c45_close,
            order_type="MARKET_AT_CLOSE",
            reason="Bar 45 BUY",
        ),
    )

    # Verify at bar 45: 3 decisions, 3 orders, 2 trades
    trades_db = db.query(Trade).filter_by(session_id=session_id).all()
    assert len(trades_db) == 2
    trade1_id = trades_db[0].id
    assert trades_db[0].status == "closed"
    assert trades_db[0].exit_price is not None
    assert trades_db[0].net_pnl is not None

    # REWIND back to bar 20
    ReplayService.previous_candle(db, session_id, steps=25)
    session = ReplayService.get_session(db, session_id)
    assert session.current_index == 20

    # 1. Assert Decisions Endpoint
    res_dec = client.get(f"/api/replay/sessions/{session_id}/decisions")
    assert res_dec.status_code == 200
    decisions = res_dec.json()
    assert len(decisions) == 1
    assert decisions[0]["candle_index"] == 15
    assert decisions[0]["reason"] == "Bar 15 BUY"

    # Also test via /api/sessions/... prefix
    res_dec_compat = client.get(f"/api/sessions/{session_id}/decisions")
    assert res_dec_compat.status_code == 200
    assert len(res_dec_compat.json()) == 1

    # 2. Assert Orders Endpoint
    res_ord = client.get(f"/api/replay/sessions/{session_id}/orders")
    assert res_ord.status_code == 200
    orders = res_ord.json()
    # Bar 45 order and bar 30 SELL order are both excluded because creation bar > 20
    assert len(orders) == 1
    assert orders[0]["side"] == "BUY"
    assert orders[0]["quantity"] == 1000
    assert orders[0]["status"].lower() == "executed"
    assert orders[0]["filled_price"] == c15_close

    # 3. Assert Trades Endpoint
    res_trd = client.get(f"/api/replay/sessions/{session_id}/trades")
    assert res_trd.status_code == 200
    trades = res_trd.json()
    # Trade #2 (bar 45) is excluded; Trade #1 (bar 15) is present with MASKED state
    assert len(trades) == 1
    t1 = trades[0]
    assert t1["id"] == trade1_id
    assert t1["status"].lower() == "open"
    assert t1["result"].lower() == "open"
    assert t1["exit_price"] is None
    assert t1["exit_date"] is None
    assert t1["gross_pnl"] is None
    assert t1["net_pnl"] is None
    assert t1["pnl_percent"] is None
    assert t1["r_multiple"] is None
    assert t1["holding_candles"] == 5  # 20 - 15

    # 4. Assert Database Invariance: Underlying DB records must NEVER be mutated on rewind!
    db_trade = db.query(Trade).filter_by(id=trade1_id).first()
    assert db_trade.status == "closed"
    assert db_trade.exit_price == c30_close
    assert db_trade.net_pnl is not None


def test_g10_active_replay_rejects_unrestricted_include_future_escape_hatch(isolated_env):
    """Verify that include_future=True is strictly rejected with HTTP 403 during active mid-replay."""
    db, client, _ = isolated_env
    symbol = "ESCAPE_TEST"
    start_d = date(2023, 1, 1)
    _seed_candles(db, symbol, start_d, 100)

    session = ReplayService.create_session(
        db,
        ReplaySessionCreate(
            symbol=symbol,
            start_date=start_d,
            end_date=start_d + timedelta(days=99),
            initial_cash=100_000_000.0,
        ),
    )
    session_id = session.id
    ReplayService.next_candle(db, session_id, steps=20)
    session = ReplayService.get_session(db, session_id)
    assert session.status.upper() == "ACTIVE"
    assert session.current_index == 20

    # 1. Decisions with include_future=true
    r1 = client.get(f"/api/replay/sessions/{session_id}/decisions?include_future=true")
    assert r1.status_code == 403
    assert "include_future is strictly prohibited during active replay" in r1.json()["detail"]

    # 2. Orders with include_future=true
    r2 = client.get(f"/api/replay/sessions/{session_id}/orders?include_future=true")
    assert r2.status_code == 403
    assert "include_future is strictly prohibited during active replay" in r2.json()["detail"]

    # 3. Trades with include_future=true
    r3 = client.get(f"/api/replay/sessions/{session_id}/trades?include_future=true")
    assert r3.status_code == 403
    assert "include_future is strictly prohibited during active replay" in r3.json()["detail"]

    # 4. Verify same rejection on /api/sessions/ prefix
    r4 = client.get(f"/api/sessions/{session_id}/decisions?include_future=true")
    assert r4.status_code == 403


def test_g10_post_session_audit_eligibility_allows_completed_session(isolated_env):
    """Verify that include_future=True returns unmasked audit ledger when session is completed or at final bar."""
    db, client, _ = isolated_env
    symbol = "AUDIT_TEST"
    start_d = date(2023, 1, 1)
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

    # Place buy at bar 10, sell at bar 20
    ReplayService.next_candle(db, session_id, steps=10)
    TradeLifecycleService.process_decision(
        db,
        session_id,
        DecisionCreate(
            action=DecisionAction.BUY,
            quantity=1000,
            price=candles[10].close,
            order_type="MARKET_AT_CLOSE",
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
            order_type="MARKET_AT_CLOSE",
            reason="Bar 20 CLOSE",
        ),
    )

    # Rewind to bar 5
    ReplayService.previous_candle(db, session_id, steps=15)
    session = ReplayService.get_session(db, session_id)
    assert session.current_index == 5

    # Mark session as completed (post-session audit)
    session.status = SessionStatus.COMPLETED.value
    db.commit()

    # Now include_future=true is permitted and returns full unmasked ledger!
    r_dec = client.get(f"/api/replay/sessions/{session_id}/decisions?include_future=true")
    assert r_dec.status_code == 200
    assert len(r_dec.json()) == 2

    r_ord = client.get(f"/api/replay/sessions/{session_id}/orders?include_future=true")
    assert r_ord.status_code == 200
    assert len(r_ord.json()) == 2
    for order in r_ord.json():
        assert order["status"].lower() == "executed"
        assert order["filled_price"] is not None

    r_trd = client.get(f"/api/replay/sessions/{session_id}/trades?include_future=true")
    assert r_trd.status_code == 200
    trades = r_trd.json()
    assert len(trades) == 1
    assert trades[0]["status"].lower() == "closed"
    assert trades[0]["exit_price"] == candles[20].close
    assert trades[0]["net_pnl"] is not None

    # Conversely, if include_future=False is passed even on completed session at bar 5,
    # it defaults to masking/filtering up to bar 5
    r_trd_masked = client.get(f"/api/replay/sessions/{session_id}/trades?include_future=false")
    assert r_trd_masked.status_code == 200
    assert len(r_trd_masked.json()) == 0  # entered at bar 10 > 5, so omitted!


def test_practice_horizon_clamped_to_end_date_and_utc_normalized(isolated_env):
    """Verify total_bars strictly clamped to session [start_date, end_date], release date clamped, and UTC datetimes normalized."""
    db, client, _ = isolated_env
    symbol = "CLAMP_SYM"
    start_d = date(2023, 1, 1)

    # Seed 200 daily candles (Jan 1, 2023 to Jul 19, 2023)
    _seed_candles(db, symbol, start_d, 200)

    # Create session spanning Jan 1, 2023 to Apr 30, 2023 (exactly 120 days: 31 + 28 + 31 + 30 = 120)
    # Use timezone-aware UTC datetime strings in session create to verify normalization
    start_iso = "2023-01-01T00:00:00+00:00"
    end_iso = "2023-04-30T00:00:00+00:00"

    session = ReplayService.create_session(
        db,
        ReplaySessionCreate(
            symbol=symbol,
            start_date=start_iso,
            end_date=end_iso,
            initial_cash=100_000_000.0,
        ),
    )
    session_id = session.id

    # 1. Verify snapshot total_bars is strictly 120 (clamped to end_date, NOT 200)
    snapshot = PracticeWorkflowService.get_snapshot(db, session_id)
    assert snapshot.total_bars == 120

    # 2. Advance to the final bar of the session (bar 119)
    ReplayService.next_candle(db, session_id, steps=119)
    session = ReplayService.get_session(db, session_id)
    assert session.current_index == 119

    # 3. Submit BUY order at bar 119
    TradeLifecycleService.process_decision(
        db,
        session_id,
        DecisionCreate(
            action=DecisionAction.BUY,
            quantity=100,
            price=219.0,
            order_type="MARKET_AT_CLOSE",
            reason="Bar 119 BUY",
        ),
    )

    # In T+2 settlement, release bar is 119 + 2 = 121 (which is outside the 120-bar session horizon)
    snapshot_119 = PracticeWorkflowService.get_snapshot(db, session_id)
    assert len(snapshot_119.positions) == 1
    # earliest_release_date must be None because bar 121 is outside the session horizon
    assert snapshot_119.positions[0].earliest_release_date is None

    # 4. Verify that timezone-aware string comparisons in API endpoints run without TypeError
    res_snap = client.get(f"/api/replay/sessions/{session_id}/practice-state")
    assert res_snap.status_code == 200
    assert res_snap.json()["total_bars"] == 120
