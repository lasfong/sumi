import pytest
from fastapi.testclient import TestClient
from datetime import date
from sqlalchemy import create_engine
from sqlalchemy.orm import sessionmaker
from sqlalchemy.pool import StaticPool
from app.main import app
from app.db import Base
from app.dependencies import get_db
from app.models.candle import Candle
from app.models.replay_session import ReplaySession
from app.domain.enums import SessionMode, DecisionAction

# Use StaticPool so all connections share the same in-memory database
test_engine = create_engine(
    "sqlite:///:memory:",
    connect_args={"check_same_thread": False},
    poolclass=StaticPool,
)
TestingSessionLocal = sessionmaker(autocommit=False, autoflush=False, bind=test_engine)

def override_get_db():
    try:
        db = TestingSessionLocal()
        yield db
    finally:
        db.close()

@pytest.fixture(scope="module")
def client():
    """Create a TestClient with all tables and test data set up."""
    app.dependency_overrides[get_db] = override_get_db
    Base.metadata.create_all(bind=test_engine)

    # Seed dummy candles
    db = TestingSessionLocal()
    for i in range(5):
        candle = Candle(
            symbol="API_TEST",
            timeframe="1D",
            timestamp=date(2023, 11, i+1),
            open=100+i,
            high=102+i,
            low=98+i,
            close=101+i,
            volume=1000,
            adjustment_type="unadjusted"
        )
        db.add(candle)
    db.commit()
    db.close()

    with TestClient(app) as c:
        yield c

    Base.metadata.drop_all(bind=test_engine)
    app.dependency_overrides.clear()

def test_health_check(client):
    response = client.get("/api/health")
    assert response.status_code == 200
    assert response.json() == {"status": "ok", "message": "Sumi API is running"}

def test_list_replay_sessions_excludes_backtest_sessions(client):
    db = TestingSessionLocal()
    backtest_session = ReplaySession(
        symbol="API_TEST",
        timeframe="1D",
        adjustment_type="unadjusted",
        start_date=date(2023, 11, 1),
        end_date=date(2023, 11, 5),
        initial_cash=100000.0,
        current_cash=100000.0,
        current_index=0,
        status="completed",
        mode="backtest",
    )
    db.add(backtest_session)
    db.commit()
    db.close()

    response = client.get("/api/replay/sessions")

    assert response.status_code == 200
    assert all(session["mode"] != "backtest" for session in response.json())

def test_full_replay_lifecycle(client):
    # 1. Create Session (use high initial_cash to cover fees)
    response = client.post(
        "/api/replay/sessions",
        json={
            "symbol": "API_TEST",
            "timeframe": "1D",
            "adjustment_type": "unadjusted",
            "start_date": "2023-11-01",
            "end_date": "2023-11-05",
            "initial_cash": 100000000.0,
            "mode": SessionMode.NORMAL.value,
            "hide_symbol": False,
            "hide_date": False
        }
    )
    assert response.status_code == 200, response.text
    session_data = response.json()
    session_id = session_data["id"]
    assert session_data["current_index"] == 0
    assert session_data["status"] == "active"

    response = client.get(f"/api/replay/sessions/{session_id}")
    assert response.status_code == 200
    assert response.json()["id"] == session_id

    # 2. Get Candles (Index 0 should return 1 candle)
    response = client.get(f"/api/replay/sessions/{session_id}/candles")
    assert response.status_code == 200
    candles = response.json()
    assert len(candles) == 1
    assert candles[0]["timestamp"] == "2023-11-01"

    # 3. Make BUY Decision
    response = client.post(
        f"/api/replay/sessions/{session_id}/decisions",
        json={
            "action": DecisionAction.BUY.value,
            "quantity": 100,
            "confidence_score": 5,
            "reason": "Test buy"
        }
    )
    assert response.status_code == 200, response.text
    decision_data = response.json()
    assert decision_data["action"] == DecisionAction.BUY.value

    # 4. Next Candle (T+1)
    response = client.post(f"/api/replay/sessions/{session_id}/next")
    assert response.status_code == 200
    session_data = response.json()
    assert session_data["current_index"] == 1

    # 4.1 Next Candle (T+2)
    response = client.post(f"/api/replay/sessions/{session_id}/next")
    assert response.status_code == 200
    session_data = response.json()
    assert session_data["current_index"] == 2

    # 5. Get Position
    response = client.get(f"/api/replay/sessions/{session_id}/position")
    assert response.status_code == 200
    positions = response.json()
    assert len(positions) == 1
    assert positions[0]["status"] == "open"

    # 6. Make CLOSE Decision at T+2
    response = client.post(
        f"/api/replay/sessions/{session_id}/decisions",
        json={
            "action": DecisionAction.CLOSE.value,
            "reason": "Test close"
        }
    )
    assert response.status_code == 200

    response = client.get(f"/api/replay/sessions/{session_id}/position")
    assert response.status_code == 200
    assert response.json() == []

    # 7. Check Trades
    response = client.get(f"/api/replay/sessions/{session_id}/trades")
    assert response.status_code == 200
    trades = response.json()
    assert len(trades) == 1
    assert trades[0]["exit_date"] is not None
    assert trades[0]["status"] == "closed"
    assert trades[0]["result"] is not None

def test_no_future_leak_indicators(client):
    # 1. Create a session with plenty of candles but current_index will be 0 initially
    response = client.post(
        "/api/replay/sessions",
        json={
            "symbol": "API_TEST",
            "timeframe": "1D",
            "adjustment_type": "unadjusted",
            "start_date": "2023-11-01",
            "end_date": "2023-11-05",
            "initial_cash": 100000.0,
            "mode": SessionMode.NORMAL.value,
            "hide_symbol": False,
            "hide_date": False
        }
    )
    assert response.status_code == 200
    session_id = response.json()["id"]

    # 2. Advance to index 2
    client.post(f"/api/replay/sessions/{session_id}/next", params={"steps": 2})

    # 3. Request EMA via session-scoped endpoint
    response = client.get(
        f"/api/replay/sessions/{session_id}/indicators",
        params={"indicator": "ema", "length": 2}
    )
    assert response.status_code == 200
    indicator_data = response.json()["data"]
    
    # Check that we only get data up to index 2 (which means 3 candles total: 0, 1, 2)
    assert len(indicator_data) == 3
    
    # 4. Advance 1 more step
    client.post(f"/api/replay/sessions/{session_id}/next")
    
    # 5. Request EMA again
    response = client.get(
        f"/api/replay/sessions/{session_id}/indicators",
        params={"indicator": "ema", "length": 2}
    )
    assert response.status_code == 200
    indicator_data_new = response.json()["data"]
    
    # Check that data grew by 1
    assert len(indicator_data_new) == 4

def test_session_indicators_allow_warmup_without_error(client):
    response = client.post(
        "/api/replay/sessions",
        json={
            "symbol": "API_TEST",
            "timeframe": "1D",
            "adjustment_type": "unadjusted",
            "start_date": "2023-11-01",
            "end_date": "2023-11-05",
            "initial_cash": 100000.0,
            "mode": SessionMode.NORMAL.value,
            "hide_symbol": False,
            "hide_date": False
        }
    )
    assert response.status_code == 200
    session_id = response.json()["id"]

    for params in (
        {"indicator": "ema", "length": 20},
        {"indicator": "rsi", "length": 14},
        {"indicator": "macd", "fast": 12, "slow": 26, "signal": 9},
    ):
        response = client.get(f"/api/replay/sessions/{session_id}/indicators", params=params)

        assert response.status_code == 200, response.text
        data = response.json()["data"]
        assert len(data) == 1
        assert list(data[0].keys()) == ["timestamp"]
