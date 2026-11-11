import asyncio
import json
from datetime import datetime, timedelta

import pytest
from fastapi.testclient import TestClient
from sqlalchemy import create_engine
from sqlalchemy.orm import sessionmaker
from sqlalchemy.pool import StaticPool

from app.api.scanner import ScannerReplaySessionRequest
from app.api.ws_replay import ConnectionManager
from app.db import Base
from app.dependencies import get_db
from app.main import app
from app.models.candle import Candle
from app.models.replay_session import ReplaySession
from app.services.replay_service import ReplayService


test_engine = create_engine(
    "sqlite:///:memory:",
    connect_args={"check_same_thread": False},
    poolclass=StaticPool,
)
TestingSessionLocal = sessionmaker(autocommit=False, autoflush=False, bind=test_engine)


def override_get_db():
    db = TestingSessionLocal()
    try:
        yield db
    finally:
        db.close()


@pytest.fixture()
def integrity_client():
    app.dependency_overrides[get_db] = override_get_db
    Base.metadata.create_all(bind=test_engine)
    db = TestingSessionLocal()
    start = datetime(2024, 1, 1)
    for index in range(21):
        price = 100 + index
        db.add(Candle(
            symbol="PRO00",
            timeframe="1D",
            adjustment_type="unadjusted",
            timestamp=start + timedelta(days=index),
            open=price,
            high=price + 1,
            low=price - 1,
            close=price + 0.5,
            volume=1_000_000 + index,
        ))
    db.commit()
    db.close()
    try:
        with TestClient(app) as client:
            yield client
    finally:
        app.dependency_overrides.clear()
        Base.metadata.drop_all(bind=test_engine)


def scanner_request(**overrides):
    request = {
        "symbol": "PRO00",
        "signal_timestamp": "2024-01-11T00:00:00",
        "signal_type": "entry",
        "strategy": "Integrity Strategy",
        "price": 110.5,
        "regime": "bullish",
        "lookback_days": 10,
        "forward_days": 10,
    }
    request.update(overrides)
    return request


def assert_no_raw_future_fields(payload):
    serialized = json.dumps(payload, sort_keys=True)
    assert payload["source_payload"] is None
    assert "Integrity Strategy" not in serialized
    assert "110.5" not in serialized
    assert "bullish" not in serialized
    assert "2024-01-11T00:00:00" not in serialized


def test_scanner_request_defaults_to_blind_and_rejects_unknown_intent():
    assert ScannerReplaySessionRequest(**scanner_request()).replay_intent.value == "blind_practice"
    with pytest.raises(ValueError):
        ScannerReplaySessionRequest(**scanner_request(replay_intent="preview"))


def test_blind_create_get_list_advance_boundary_rewind_and_resume(integrity_client):
    created_response = integrity_client.post("/api/scanner/replay-session", json=scanner_request())
    assert created_response.status_code == 200
    created = created_response.json()["session"]
    assert created["current_index"] == 0
    assert created["source_context"] == {
        "schema_version": 1,
        "source_type": "scanner_signal",
        "replay_intent": "blind_practice",
        "reveal_at_index": 10,
        "revealed": False,
        "signal": None,
    }
    assert_no_raw_future_fields(created)
    session_id = created["id"]

    fetched = integrity_client.get(f"/api/replay/sessions/{session_id}").json()
    listed = next(item for item in integrity_client.get("/api/replay/sessions").json() if item["id"] == session_id)
    assert fetched == listed
    assert_no_raw_future_fields(fetched)

    before = integrity_client.post(f"/api/replay/sessions/{session_id}/next", params={"steps": 9}).json()
    assert before["current_index"] == 9
    assert before["source_context"]["revealed"] is False
    assert before["source_context"]["signal"] is None
    assert_no_raw_future_fields(before)

    boundary = integrity_client.post(f"/api/replay/sessions/{session_id}/next").json()
    assert boundary["current_index"] == 10
    assert boundary["source_context"]["revealed"] is True
    assert boundary["source_context"]["signal"] == {
        "timestamp": "2024-01-11T00:00:00",
        "type": "entry",
        "strategy": "Integrity Strategy",
        "price": 110.5,
        "regime": "bullish",
    }
    assert boundary["source_payload"] is None

    after = integrity_client.post(f"/api/replay/sessions/{session_id}/next").json()
    assert after["source_context"] == boundary["source_context"]

    rewound = integrity_client.post(f"/api/replay/sessions/{session_id}/previous", params={"steps": 2}).json()
    assert rewound["current_index"] == 9
    assert rewound["source_context"]["signal"] is None
    assert_no_raw_future_fields(rewound)

    resumed = integrity_client.get(f"/api/replay/sessions/{session_id}").json()
    assert resumed["source_context"] == rewound["source_context"]


def test_signal_review_starts_revealed_at_actual_signal_candle(integrity_client):
    response = integrity_client.post(
        "/api/scanner/replay-session",
        json=scanner_request(replay_intent="signal_review"),
    )
    assert response.status_code == 200
    session = response.json()["session"]
    assert session["current_index"] == 10
    assert session["source_payload"] is None
    assert session["source_context"]["replay_intent"] == "signal_review"
    assert session["source_context"]["revealed"] is True
    assert session["source_context"]["signal"]["strategy"] == "Integrity Strategy"
    candles = integrity_client.get(f"/api/replay/sessions/{session['id']}/candles").json()
    assert len(candles) == 11
    assert candles[-1]["timestamp"].startswith("2024-01-11")


def test_scanner_creation_rejects_signal_without_matching_session_candle(integrity_client):
    response = integrity_client.post(
        "/api/scanner/replay-session",
        json=scanner_request(signal_timestamp="2024-01-11T12:00:00"),
    )
    assert response.status_code == 400
    assert response.json()["detail"] == "Selected signal does not match a candle in the replay window"


def test_legacy_malformed_and_non_scanner_compatibility(integrity_client):
    db = TestingSessionLocal()
    common = {
        "symbol": "PRO00",
        "timeframe": "1D",
        "adjustment_type": "unadjusted",
        "start_date": datetime(2024, 1, 1).date(),
        "end_date": datetime(2024, 1, 21).date(),
        "status": "active",
        "mode": "normal",
    }
    legacy = ReplaySession(
        **common,
        source_type="scanner_signal",
        source_payload=json.dumps({
            "symbol": "PRO00",
            "signal_timestamp": "2024-01-11T00:00:00",
            "signal_type": "entry",
            "strategy": "Legacy Strategy",
            "price": 110.5,
            "regime": None,
        }),
    )
    malformed = ReplaySession(**common, source_type="scanner_signal", source_payload='{"signal_timestamp":')
    incomplete = ReplaySession(
        **common,
        source_type="scanner_signal",
        source_payload=json.dumps({"signal_timestamp": "2024-01-11T00:00:00", "price": 110.5}),
    )
    non_scanner = ReplaySession(**common, source_type="manual_note", source_payload='{"note":"local"}')
    db.add_all([legacy, malformed, incomplete, non_scanner])
    db.commit()
    ids = [legacy.id, malformed.id, incomplete.id, non_scanner.id]
    db.close()

    legacy_payload = integrity_client.get(f"/api/replay/sessions/{ids[0]}").json()
    assert legacy_payload["source_context"]["replay_intent"] == "blind_practice"
    assert legacy_payload["source_context"]["signal"] is None
    assert legacy_payload["source_payload"] is None

    for session_id in ids[1:3]:
        payload = integrity_client.get(f"/api/replay/sessions/{session_id}").json()
        assert payload["source_payload"] is None
        assert payload["source_context"]["revealed"] is False
        assert payload["source_context"]["signal"] is None

    compatible = integrity_client.get(f"/api/replay/sessions/{ids[3]}").json()
    assert compatible["source_payload"] == '{"note":"local"}'
    assert compatible["source_context"]["source_type"] == "manual_note"
    assert compatible["source_context"]["replay_intent"] is None


def test_generic_scanner_session_create_route_is_sanitized(integrity_client):
    payload = {
        "symbol": "PRO00",
        "start_date": "2024-01-01",
        "end_date": "2024-01-21",
        "source_type": "scanner_signal",
        "source_payload": json.dumps({
            "symbol": "PRO00",
            "signal_timestamp": "2024-01-11T00:00:00",
            "signal_type": "entry",
            "strategy": "Integrity Strategy",
            "price": 110.5,
            "regime": "bullish",
        }),
    }
    response = integrity_client.post("/api/replay/sessions", json=payload)
    assert response.status_code == 200
    assert_no_raw_future_fields(response.json())


def test_websocket_message_contains_only_sanitized_source_context():
    class FakeSocket:
        def __init__(self):
            self.messages = []

        async def send_json(self, payload):
            self.messages.append(payload)

    manager = ConnectionManager()
    socket = FakeSocket()
    manager.active_connections[7] = socket
    context = {
        "schema_version": 1,
        "source_type": "scanner_signal",
        "replay_intent": "blind_practice",
        "reveal_at_index": 10,
        "revealed": False,
        "signal": None,
    }
    asyncio.run(manager.send_candle_update(7, {"time": 1, "close": 100}, context))
    assert socket.messages == [{
        "type": "new_candle",
        "data": {"time": 1, "close": 100},
        "source_context": context,
    }]
    assert "source_payload" not in json.dumps(socket.messages)
