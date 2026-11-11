"""Test Suite for Batch B31: WebSocket Sequencing, Heartbeat, Reconnect & State Resynchronization.

Covers ADR-WS-001 and WEBSOCKET_TEST_MATRIX.md (WS-01..WS-10 backend contracts):
- WS-01 / WS-02: sync/resume and ping/pong state echo with authoritative current_index and source_context
- WS-03: Malformed JSON and invalid/unsupported actions emit error frames without dropping connection
- WS-04: Speed clamping (negative, zero, extreme values, speed_multiplier) with clamped echo
- WS-05: Dynamic set_speed updates active speed in place and echoes each change with monotonic seq
- WS-09: Stepping/playing to final candle (N-1) emits completed frame with END_OF_DATA reason
- WS-10: Two tabs connected to the same session maintain independent monotonic seq counters without cross-talk

All tests use an isolated temporary SQLite database (tmp_path). Production backend/sumi.db is never accessed.
"""

from __future__ import annotations

from datetime import date, datetime, timedelta, timezone
from typing import Generator
import pytest
from fastapi.testclient import TestClient
from sqlalchemy import create_engine
from sqlalchemy.orm import Session, sessionmaker

from app.api.ws_replay import manager, clamp_speed_ms, MIN_SPEED_MS, MAX_SPEED_MS, DEFAULT_SPEED_MS
from app.db import Base, SessionLocal
from app.dependencies import get_db
from app.main import app
from app.models.candle import Candle
from app.schemas.replay_schema import ReplaySessionCreate
from app.services.replay_service import ReplayService


@pytest.fixture
def isolated_ws_env(tmp_path) -> Generator[tuple[Session, TestClient, int], None, None]:
    """Provides an isolated temporary SQLite database, TestClient, and seeded 4-bar ReplaySession."""
    db_file = tmp_path / "test_b31_ws_isolated.db"
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
    original_factory = manager.session_factory
    manager.session_factory = TestingSessionLocal

    db = TestingSessionLocal()
    start_d = date(2024, 1, 2)
    for i in range(4):
        c_date = start_d + timedelta(days=i)
        db.add(
            Candle(
                symbol="FPT",
                timeframe="1D",
                adjustment_type="unadjusted",
                timestamp=datetime(c_date.year, c_date.month, c_date.day, tzinfo=timezone.utc),
                open=100.0 + i,
                high=102.0 + i,
                low=99.0 + i,
                close=101.0 + i,
                volume=100000 + i * 1000,
                source="vnstock",
            )
        )
    db.commit()

    replay_session = ReplayService.create_session(
        db,
        ReplaySessionCreate(
            symbol="FPT",
            timeframe="1D",
            start_date=datetime(2024, 1, 2, tzinfo=timezone.utc),
            end_date=datetime(2024, 1, 10, tzinfo=timezone.utc),
            initial_cash=100_000_000.0,
        ),
    )
    session_id = replay_session.id
    db.commit()
    db.close()

    with TestClient(app) as client:
        yield TestingSessionLocal, client, session_id

    manager.session_factory = original_factory
    app.dependency_overrides.pop(get_db, None)


def test_ws04_clamp_speed_helper_unit():
    assert clamp_speed_ms(None) == (DEFAULT_SPEED_MS, False)
    assert clamp_speed_ms(1000) == (1000, False)
    assert clamp_speed_ms(-500) == (MIN_SPEED_MS, True)
    assert clamp_speed_ms(0) == (MIN_SPEED_MS, True)
    assert clamp_speed_ms(999999) == (MAX_SPEED_MS, True)
    assert clamp_speed_ms("invalid") == (DEFAULT_SPEED_MS, True)
    assert clamp_speed_ms(None, 2.0) == (500, False)
    assert clamp_speed_ms(None, 100.0) == (MIN_SPEED_MS, True)
    assert clamp_speed_ms(None, -1.0) == (MAX_SPEED_MS, True)


def test_ws03_malformed_json_and_invalid_action_emit_error_frame_without_disconnect(isolated_ws_env):
    _, client, session_id = isolated_ws_env

    with client.websocket_connect(f"/api/ws/replay/{session_id}") as ws:
        # 1. Send garbage JSON
        ws.send_text("{not-valid-json!!!")
        frame1 = ws.receive_json()
        assert frame1["type"] == "error"
        assert frame1["seq"] == 1
        assert frame1["session_id"] == session_id
        assert frame1["payload"]["code"] == "INVALID_JSON"

        # 2. Send non-object JSON
        ws.send_text("[1, 2, 3]")
        frame2 = ws.receive_json()
        assert frame2["type"] == "error"
        assert frame2["seq"] == 2
        assert frame2["payload"]["code"] == "INVALID_PAYLOAD"

        # 3. Send unknown action
        ws.send_json({"action": "explode_everything"})
        frame3 = ws.receive_json()
        assert frame3["type"] == "error"
        assert frame3["seq"] == 3
        assert frame3["payload"]["code"] == "INVALID_ACTION"

        # 4. Verify connection is still alive and processes valid ping
        ws.send_json({"action": "ping", "ts": 123456})
        frame4 = ws.receive_json()
        assert frame4["type"] == "pong"
        assert frame4["seq"] == 4
        assert frame4["payload"]["ts"] == 123456


def test_ws04_ws05_speed_clamping_and_continuous_set_speed_echo(isolated_ws_env):
    _, client, session_id = isolated_ws_env

    with client.websocket_connect(f"/api/ws/replay/{session_id}") as ws:
        # Negative speed -> clamped to 100ms
        ws.send_json({"action": "set_speed", "speed": -250})
        f1 = ws.receive_json()
        assert f1["type"] == "playback_state"
        assert f1["seq"] == 1
        assert f1["speed_ms"] == 100
        assert f1["payload"]["clamped"] is True

        # Excessive speed -> clamped to 4000ms
        ws.send_json({"action": "set_speed", "speed": 50000})
        f2 = ws.receive_json()
        assert f2["type"] == "playback_state"
        assert f2["seq"] == 2
        assert f2["speed_ms"] == 4000
        assert f2["payload"]["clamped"] is True

        # Rapid set_speed sequence (WS-05): last speed wins, every command echoed monotonically
        for expected_seq, speed in enumerate([800, 500, 250, 1000], start=3):
            ws.send_json({"action": "set_speed", "speed": speed})
            echo = ws.receive_json()
            assert echo["type"] == "playback_state"
            assert echo["seq"] == expected_seq
            assert echo["speed_ms"] == speed
            assert echo["payload"]["clamped"] is False


def test_ws01_ws09_sync_step_and_completed_end_of_data_frame(isolated_ws_env):
    _, client, session_id = isolated_ws_env

    with client.websocket_connect(f"/api/ws/replay/{session_id}") as ws:
        # Initial sync handshake (WS-01 / WS-02)
        ws.send_json({"action": "sync"})
        sync_frame = ws.receive_json()
        assert sync_frame["type"] == "playback_state"
        assert sync_frame["seq"] == 1
        assert sync_frame["current_index"] == 0
        assert sync_frame["playback_state"] == "PAUSED"
        assert sync_frame["payload"]["candle"]["close"] == 101.0

        # Step 1 -> index 1
        ws.send_json({"action": "next"})
        c1 = ws.receive_json()
        assert c1["type"] == "new_candle"
        assert c1["seq"] == 2
        assert c1["current_index"] == 1
        assert c1["data"]["close"] == 102.0

        # Step 2 -> index 2
        ws.send_json({"action": "next"})
        c2 = ws.receive_json()
        assert c2["type"] == "new_candle"
        assert c2["seq"] == 3
        assert c2["current_index"] == 2
        assert c2["data"]["close"] == 103.0

        # Step 3 -> index 3 (N-1 = final bar of 4-bar dataset) -> emits new_candle AND completed!
        ws.send_json({"action": "next"})
        c3 = ws.receive_json()
        assert c3["type"] == "new_candle"
        assert c3["seq"] == 4
        assert c3["current_index"] == 3
        assert c3["data"]["close"] == 104.0

        comp = ws.receive_json()
        assert comp["type"] == "completed"
        assert comp["seq"] == 5
        assert comp["current_index"] == 3
        assert comp["playback_state"] == "COMPLETED"
        assert comp["payload"]["reason"] == "END_OF_DATA"

        # Attempting to start when already COMPLETED immediately emits completed frame
        ws.send_json({"action": "start", "speed": 500})
        comp_again = ws.receive_json()
        assert comp_again["type"] == "completed"
        assert comp_again["seq"] == 6
        assert comp_again["playback_state"] == "COMPLETED"


def test_ws10_two_tabs_same_session_have_independent_seq_without_crosstalk(isolated_ws_env):
    _, client, session_id = isolated_ws_env

    with client.websocket_connect(f"/api/ws/replay/{session_id}") as ws_tab1:
        with client.websocket_connect(f"/api/ws/replay/{session_id}") as ws_tab2:
            # Tab 1 sends 3 commands -> seq 1, 2, 3
            ws_tab1.send_json({"action": "sync"})
            t1_f1 = ws_tab1.receive_json()
            ws_tab1.send_json({"action": "set_speed", "speed": 500})
            t1_f2 = ws_tab1.receive_json()
            ws_tab1.send_json({"action": "ping", "ts": 111})
            t1_f3 = ws_tab1.receive_json()

            assert [t1_f1["seq"], t1_f2["seq"], t1_f3["seq"]] == [1, 2, 3]

            # Tab 2 sends its first command -> its seq starts independently at 1!
            ws_tab2.send_json({"action": "sync"})
            t2_f1 = ws_tab2.receive_json()
            ws_tab2.send_json({"action": "ping", "ts": 222})
            t2_f2 = ws_tab2.receive_json()

            assert [t2_f1["seq"], t2_f2["seq"]] == [1, 2]
            assert t2_f1["speed_ms"] == DEFAULT_SPEED_MS
            assert t1_f2["speed_ms"] == 500


def test_ws01_connection_termination_and_new_connection_resync(isolated_ws_env):
    _, client, session_id = isolated_ws_env

    # Connection 1: sync + step 2 bars (seq reaches 3, current_index reaches 2)
    with client.websocket_connect(f"/api/ws/replay/{session_id}") as ws1:
        ws1.send_json({"action": "sync"})
        f1 = ws1.receive_json()
        assert f1["seq"] == 1
        assert f1["current_index"] == 0

        ws1.send_json({"action": "next"})
        f2 = ws1.receive_json()
        assert f2["seq"] == 2
        assert f2["current_index"] == 1

        ws1.send_json({"action": "next"})
        f3 = ws1.receive_json()
        assert f3["seq"] == 3
        assert f3["current_index"] == 2

    # Connection 1 is now closed. Establish brand-new Connection 2 on the same session:
    with client.websocket_connect(f"/api/ws/replay/{session_id}") as ws2:
        # New connection seq counter starts fresh at 1 and resynchronizes at current_index=2
        ws2.send_json({"action": "sync"})
        r1 = ws2.receive_json()
        assert r1["type"] == "playback_state"
        assert r1["seq"] == 1
        assert r1["current_index"] == 2
        assert r1["payload"]["candle"]["close"] == 103.0

        # Queued set_speed + step to final bar (index 3)
        ws2.send_json({"action": "set_speed", "speed": 250})
        r2 = ws2.receive_json()
        assert r2["type"] == "playback_state"
        assert r2["seq"] == 2
        assert r2["speed_ms"] == 250
        assert r2["payload"]["clamped"] is False

        ws2.send_json({"action": "next"})
        r3 = ws2.receive_json()
        assert r3["type"] == "new_candle"
        assert r3["seq"] == 3
        assert r3["current_index"] == 3
        assert r3["data"]["close"] == 104.0

        r4 = ws2.receive_json()
        assert r4["type"] == "completed"
        assert r4["seq"] == 4
        assert r4["current_index"] == 3
        assert r4["playback_state"] == "COMPLETED"
