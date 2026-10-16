"""Batch B50 — Trading Practice UX Backend Verification.

Covers:
- GAP-U-02: Symmetric LONG and SHORT position sizing & validation in trade_planning.py
- GAP-U-04 (part): Pending LIMIT order cancellation via DELETE /api/replay/sessions/{id}/orders/{order_id}
"""

from datetime import datetime, timedelta
import pytest
from fastapi.testclient import TestClient
from sqlalchemy import create_engine
from sqlalchemy.orm import sessionmaker
from sqlalchemy.pool import StaticPool

import app.models  # noqa: F401
from app.db import Base
from app.dependencies import get_db
from app.domain.trade_planning import TradePlanInput, calculate_position_size
from app.main import app
from app.models.candle import Candle
from app.models.replay_session import ReplaySession

_TEST_ENGINE = create_engine(
    "sqlite:///:memory:",
    connect_args={"check_same_thread": False},
    poolclass=StaticPool,
)
_TestingSessionLocal = sessionmaker(autocommit=False, autoflush=False, bind=_TEST_ENGINE)


def _override_get_db():
    db = _TestingSessionLocal()
    try:
        yield db
    finally:
        db.close()


@pytest.fixture(scope="function")
def db_session():
    Base.metadata.drop_all(bind=_TEST_ENGINE)
    Base.metadata.create_all(bind=_TEST_ENGINE)
    db = _TestingSessionLocal()
    try:
        yield db
    finally:
        db.close()
        Base.metadata.drop_all(bind=_TEST_ENGINE)


@pytest.fixture(scope="function")
def client(db_session):
    app.dependency_overrides[get_db] = _override_get_db
    with TestClient(app) as c:
        yield c
    app.dependency_overrides.clear()


def test_b50_symmetric_long_and_short_position_sizing():
    """Verify calculate_position_size supports both LONG and SHORT risk geometry."""
    long_plan = calculate_position_size(
        equity=100_000_000,
        available_cash=100_000_000,
        plan=TradePlanInput(
            risk_percent=1.0,
            entry_price=50_000,
            stop_loss=47_500,
            target_price=55_000,
            direction="LONG",
        ),
    )
    assert long_plan.direction == "LONG"
    assert long_plan.risk_per_share == 2_500
    assert long_plan.reward_per_share == 5_000
    assert long_plan.expected_r_multiple == 2.0
    assert long_plan.planned_quantity == 400
    assert long_plan.estimated_net_profit > 0

    short_plan = calculate_position_size(
        equity=100_000_000,
        available_cash=100_000_000,
        plan=TradePlanInput(
            risk_percent=1.0,
            entry_price=50_000,
            stop_loss=52_500,
            target_price=45_000,
            direction="SHORT",
        ),
    )
    assert short_plan.direction == "SHORT"
    assert short_plan.risk_per_share == 2_500
    assert short_plan.reward_per_share == 5_000
    assert short_plan.expected_r_multiple == 2.0
    assert short_plan.planned_quantity == 400
    assert short_plan.estimated_net_profit > 0

    with pytest.raises(ValueError, match="stop loss must be strictly above entry price"):
        calculate_position_size(
            equity=100_000_000,
            available_cash=100_000_000,
            plan=TradePlanInput(
                risk_percent=1.0,
                entry_price=50_000,
                stop_loss=48_000,
                target_price=45_000,
                direction="SHORT",
            ),
        )


def test_b50_cancel_pending_limit_order(client: TestClient, db_session):
    """Verify pending LIMIT order can be cancelled via DELETE /api/replay/sessions/{id}/orders/{order_id}."""
    base_dt = datetime(2025, 2, 3)
    for i in range(10):
        db_session.add(
            Candle(
                symbol="B50_SYM",
                timeframe="1D",
                timestamp=base_dt + timedelta(days=i),
                open=100.0,
                high=105.0,
                low=95.0,
                close=100.0,
                volume=500_000,
                source="vnstock_kbs",
                adjustment_type="unadjusted",
            )
        )
    session = ReplaySession(
        symbol="B50_SYM",
        timeframe="1D",
        adjustment_type="unadjusted",
        start_date=base_dt,
        end_date=base_dt + timedelta(days=15),
        current_index=2,
        initial_cash=100_000_000.0,
        current_cash=100_000_000.0,
        status="active",
        mode="normal",
    )
    db_session.add(session)
    db_session.commit()
    db_session.refresh(session)

    # Create a pending LIMIT BUY order within HOSE +/-7% price limits (ref=100.0, limit=98.0)
    res_dec = client.post(
        f"/api/replay/sessions/{session.id}/decisions",
        json={
            "action": "BUY",
            "order_type": "LIMIT",
            "price": 98.0,
            "quantity": 200,
            "stop_loss": 93.0,
            "target_price": 108.0,
        },
    )
    assert res_dec.status_code == 200

    res_state = client.get(f"/api/replay/sessions/{session.id}/practice-state")
    assert res_state.status_code == 200
    orders = res_state.json()["orders"]
    assert len(orders) == 1
    assert orders[0]["status"] == "pending"
    order_id = orders[0]["id"]

    # Cancel the pending LIMIT order
    res_cancel = client.delete(f"/api/replay/sessions/{session.id}/orders/{order_id}")
    assert res_cancel.status_code == 200
    assert res_cancel.json()["status"] == "cancelled"

    # Verify practice-state now shows order as cancelled
    res_state_after = client.get(f"/api/replay/sessions/{session.id}/practice-state")
    assert res_state_after.status_code == 200
    assert res_state_after.json()["orders"][0]["status"] == "cancelled"

    # Second cancellation attempt fails with 400
    res_cancel_again = client.delete(f"/api/replay/sessions/{session.id}/orders/{order_id}")
    assert res_cancel_again.status_code == 400

    # Create a MARKET_NEXT_OPEN BUY order with taxonomy + SL/TP and step 1 bar to verify Trade retains metadata (DEF-W5-05)
    res_buy = client.post(
        f"/api/replay/sessions/{session.id}/decisions",
        json={
            "action": "BUY",
            "order_type": "MARKET_NEXT_OPEN",
            "price": 100.0,
            "quantity": 200,
            "stop_loss": 95.0,
            "target_price": 110.0,
            "setup_type": "Breakout",
            "market_regime": "Bull Trend",
            "emotion": "Calm / Disciplined",
            "reason": "Breakout with volume",
        },
    )
    assert res_buy.status_code == 200
    res_next = client.post(f"/api/replay/sessions/{session.id}/next?steps=1")
    assert res_next.status_code == 200
    res_filled_state = client.get(f"/api/replay/sessions/{session.id}/practice-state")
    assert res_filled_state.status_code == 200
    trades = res_filled_state.json()["trades"]
    assert len(trades) == 1
    assert trades[0]["setup_type"] == "Breakout"
    assert trades[0]["market_regime"] == "Bull Trend"
    assert trades[0]["emotion"] == "Calm / Disciplined"
    assert trades[0]["initial_stop_loss"] == 95.0
    assert trades[0]["target_price"] == 110.0
    assert trades[0]["planned_entry_price"] == 100.0
    assert trades[0]["planned_r"] == 2.0
