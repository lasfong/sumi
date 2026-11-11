"""Integration tests for Signal Registry and Replay Calculation APIs.

Tests API endpoints using temporary in-memory database fixture.
Verifies acceptance oracles P1-OR-08 and P1-OR-09.
"""

from datetime import date, datetime
import pytest

from app.domain.enums import SessionMode, SessionStatus
from app.models.candle import Candle
from app.models.replay_session import ReplaySession


def _seed_session_and_candles(db, symbol="SSI", timeframe="1D", num_candles=5, current_index=3):
    """Seed a test replay session with daily candles."""
    candles = []
    for i in range(num_candles):
        c = Candle(
            symbol=symbol,
            timeframe=timeframe,
            adjustment_type="adjusted",
            timestamp=datetime(2026, 1, i + 1),
            open=100.0 + i,
            high=105.0 + i,
            low=95.0 + i,
            close=102.0 + i,
            volume=1000.0 * (i + 1),
        )
        db.add(c)
        candles.append(c)
    db.commit()

    session = ReplaySession(
        symbol=symbol,
        timeframe=timeframe,
        adjustment_type="adjusted",
        start_date=date(2026, 1, 1),
        end_date=date(2026, 1, num_candles),
        current_index=current_index,
        initial_cash=100_000_000,
        current_cash=100_000_000,
        status=SessionStatus.ACTIVE.value,
        mode=SessionMode.NORMAL.value,
        hide_symbol=False,
        hide_date=False,
    )
    db.add(session)
    db.commit()
    db.refresh(session)
    return session


def test_get_signal_registry(client):
    """Verify GET /api/signals/registry returns definitions and schemas."""
    response = client.get("/api/signals/registry")
    assert response.status_code == 200
    data = response.json()
    assert "signals" in data
    signals = {s["name"]: s for s in data["signals"]}

    assert "volume.relative_volume" in signals
    rvol = signals["volume.relative_volume"]
    assert rvol["output_type"] == "float"
    assert rvol["ast_alias"] == "volume__relative_volume"
    assert "period" in rvol["parameters_schema"]

    assert "volume.spike" in signals
    spike = signals["volume.spike"]
    assert spike["output_type"] == "bool"
    assert spike["ast_alias"] == "volume__spike"
    assert "multiplier" in spike["parameters_schema"]
    assert "period" in spike["parameters_schema"]

    mult_schema = spike["parameters_schema"]["multiplier"]
    assert mult_schema.get("exclusiveMinimum") == 0
    assert mult_schema.get("maximum") == 100
    assert "min" not in mult_schema
    assert "0.01" not in str(mult_schema)
    assert "0.1" not in str(mult_schema)


def test_calculate_signals_session_not_found(client):
    """Verify 404 is returned if replay session does not exist."""
    payload = {
        "signals": [
            {"name": "volume.spike", "params": {"period": 20, "multiplier": 2.0}}
        ]
    }
    response = client.post("/api/signals/replay/999999/calculate", json=payload)
    assert response.status_code == 404


def test_calculate_signals_unsupported_timeframe(client, db_session):
    """Verify 422 is returned if session timeframe is not 1D."""
    session = _seed_session_and_candles(db_session, symbol="FPT", timeframe="1H", num_candles=5, current_index=2)
    payload = {
        "signals": [
            {"name": "volume.spike", "params": {"period": 20, "multiplier": 2.0}}
        ]
    }
    response = client.post(f"/api/signals/replay/{session.id}/calculate", json=payload)
    assert response.status_code == 422
    assert "Only '1D' is supported" in response.json()["detail"]


def test_calculate_signals_payload_validation_errors(client, db_session):
    """Verify 422 rejection on empty list, duplicate names, unknown signals, extra fields."""
    session = _seed_session_and_candles(db_session, symbol="SSI", timeframe="1D", num_candles=5, current_index=3)

    # Empty list
    res_empty = client.post(f"/api/signals/replay/{session.id}/calculate", json={"signals": []})
    assert res_empty.status_code == 422

    # Duplicate signal name
    res_dup = client.post(
        f"/api/signals/replay/{session.id}/calculate",
        json={
            "signals": [
                {"name": "volume.spike", "params": {"period": 20}},
                {"name": "volume.spike", "params": {"period": 10}},
            ]
        },
    )
    assert res_dup.status_code == 422

    # More than 2 signals
    res_over = client.post(
        f"/api/signals/replay/{session.id}/calculate",
        json={
            "signals": [
                {"name": "volume.spike"},
                {"name": "volume.relative_volume"},
                {"name": "volume.spike"},
            ]
        },
    )
    assert res_over.status_code == 422

    # Unknown signal
    res_unknown = client.post(
        f"/api/signals/replay/{session.id}/calculate",
        json={"signals": [{"name": "unknown.signal"}]},
    )
    assert res_unknown.status_code == 422

    # Extra top-level fields (forbidden)
    res_extra = client.post(
        f"/api/signals/replay/{session.id}/calculate",
        json={"signals": [{"name": "volume.spike"}], "extra_field": "hacked"},
    )
    assert res_extra.status_code == 422

    # Invalid parameter type (bool for int)
    res_bool = client.post(
        f"/api/signals/replay/{session.id}/calculate",
        json={"signals": [{"name": "volume.spike", "params": {"period": True}}]},
    )
    assert res_bool.status_code == 422


def test_p1_or_09_replay_boundary_and_rewind(client, db_session):
    """P1-OR-09: Replay index capping and rewind behavior.
    
    Session has 5 bars; current_index = 3.
    Output only contains bars 0..3; bar 4 is absent.
    Rewind to index 2; output only contains bars 0..2.
    """
    session = _seed_session_and_candles(db_session, symbol="SSI", timeframe="1D", num_candles=5, current_index=3)

    payload = {
        "signals": [
            {"name": "volume.spike", "params": {"period": 2, "multiplier": 1.5}}
        ]
    }

    # First call at current_index = 3
    res = client.post(f"/api/signals/replay/{session.id}/calculate", json=payload)
    assert res.status_code == 200
    data = res.json()

    assert data["session_id"] == session.id
    assert data["observed_current_index"] == 3
    assert len(data["results"]) == 1

    series = data["results"][0]
    assert series["signal_name"] == "volume.spike"
    assert len(series["points"]) == 4  # exactly indices 0, 1, 2, 3

    indexes = [p["bar_index"] for p in series["points"]]
    assert indexes == [0, 1, 2, 3]
    assert 4 not in indexes

    # Verify availability metadata
    for pt in series["points"]:
        assert pt["availability_event"] == "BAR_CLOSE"
        assert pt["available_at_index"] == pt["bar_index"]

    # Now rewind session to current_index = 2
    session.current_index = 2
    db_session.commit()

    res_rewind = client.post(f"/api/signals/replay/{session.id}/calculate", json=payload)
    assert res_rewind.status_code == 200
    data_rewind = res_rewind.json()

    assert data_rewind["observed_current_index"] == 2
    points_rewind = data_rewind["results"][0]["points"]
    assert len(points_rewind) == 3
    assert [p["bar_index"] for p in points_rewind] == [0, 1, 2]
    assert 3 not in [p["bar_index"] for p in points_rewind]


def test_calculate_signals_rejects_params_null_and_defaults_omitted(client, db_session):
    """Verify explicit params: null is rejected with 422, while omitted params still resolves defaults."""
    session = _seed_session_and_candles(db_session, symbol="SSI", timeframe="1D", num_candles=5, current_index=3)

    # 1. Explicit params: null MUST return HTTP 422
    res_null = client.post(
        f"/api/signals/replay/{session.id}/calculate",
        json={"signals": [{"name": "volume.spike", "params": None}]},
    )
    assert res_null.status_code == 422

    # 2. Omitted params resolves defaults
    res_omitted = client.post(
        f"/api/signals/replay/{session.id}/calculate",
        json={"signals": [{"name": "volume.spike"}]},
    )
    assert res_omitted.status_code == 200
    data_omitted = res_omitted.json()
    assert len(data_omitted["results"]) == 1
    result_series = data_omitted["results"][0]
    assert result_series["resolved_params"] == {"period": 20, "multiplier": 2.0}

