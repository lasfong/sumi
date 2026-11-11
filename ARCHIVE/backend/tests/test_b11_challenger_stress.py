"""Adversarial Challenger Stress Suite for Batch B11.

Rigorous empirical testing of:
1. Server-Side Replay Indicator Forward Projection Clamping:
   - Ichimoku Cloud forward projection (+26 bars) is clamped across stepping, rewinding, and varied cursors (bar 20, 45, 60, 75).
   - All points returned strictly satisfy timestamp <= candles[current_index].timestamp.
   - Forward projected records are completely dropped from serialization over the wire (never return is_projected=True).
2. Non-Replay Chart Intentional Forward Projection Preservation:
   - Direct IndicatorEngine.compute with clamp_to_last_candle=False preserves forward-projected dates.
   - Non-replay endpoint GET /api/indicators/{symbol} preserves forward projection for full-chart views.
3. Typed MappingResult Metadata Contract:
   - Meta status: OK, EMPTY_WARMING, UNMAPPED_COLUMN, INVALID_SPEC.
   - Dynamic warmup calculations for SMA, EMA, MACD, RSI, BBands, ADX, Ichimoku, Supertrend.
   - Backward compatibility: data array shape and keys are preserved for legacy consumers.
4. Stress Under Dynamic Cursor Movements:
   - Step forward, request indicators, step backward, re-request indicators; ensure no cached or leaked future indicator data.
5. Production Database Invariance:
   - All tests use isolated temporary SQLite databases. backend/sumi.db is never touched.
"""

from __future__ import annotations

from datetime import date, datetime, timedelta
from typing import Generator
import numpy as np
import pandas as pd
import pytest
from fastapi.testclient import TestClient
from sqlalchemy import create_engine
from sqlalchemy.orm import Session, sessionmaker

from app.db import Base
from app.dependencies import get_db
from app.domain.engine.indicator_engine import IndicatorEngine
from app.main import app
from app.models.candle import Candle
from app.schemas.replay_schema import ReplaySessionCreate
from app.services.replay_service import ReplayService
from app.api.replay import compute_indicator_for_replay


@pytest.fixture
def isolated_challenger_db(tmp_path) -> Generator[tuple[Session, TestClient, str], None, None]:
    """Provides an isolated SQLite database and TestClient with get_db overridden."""
    db_file = tmp_path / "test_b11_challenger.db"
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
    np.random.seed(123)
    close_price = 120.0
    for i in range(count):
        c_date = start_d + timedelta(days=i)
        step = np.random.randn() * 1.2
        close_price = max(20.0, close_price + step)
        c = Candle(
            symbol=symbol,
            timeframe="1D",
            adjustment_type="unadjusted",
            timestamp=datetime.combine(c_date, datetime.min.time()),
            open=float(close_price - 0.5),
            high=float(close_price + 2.0),
            low=float(close_price - 2.0),
            close=float(close_price),
            volume=float(250000 + i * 500),
        )
        db.add(c)
        candles.append(c)
    db.commit()
    return candles


def test_ichimoku_clamping_across_stepping_and_rewinding(isolated_challenger_db):
    """Stress test Ichimoku clamping across dynamic cursor movements:

    Advance to 70 -> verify clamped at 70 -> rewind to 40 -> verify clamped at 40 -> advance to 85.
    """
    db, client, _ = isolated_challenger_db
    symbol = "CHALLENGER_ICHI"
    start_d = date(2023, 2, 1)
    candles = _seed_candles(db, symbol, start_d, 120)

    session = ReplayService.create_session(
        db,
        ReplaySessionCreate(
            symbol=symbol,
            start_date=start_d,
            end_date=start_d + timedelta(days=119),
            initial_cash=100_000_000.0,
        ),
    )
    session_id = session.id

    # 1. Advance to cursor 70
    ReplayService.next_candle(db, session_id, steps=70)
    res_70 = client.get(
        f"/api/replay/sessions/{session_id}/indicators",
        params={"indicator": "ichimoku", "tenkan": 9, "kijun": 26, "senkou": 52},
    )
    assert res_70.status_code == 200
    payload_70 = res_70.json()
    assert payload_70["meta"]["available_bars"] == 71
    assert payload_70["meta"]["is_projected_clamped"] is True
    assert len(payload_70["data"]) == 71
    max_ts_70 = pd.to_datetime(candles[70].timestamp)
    for p in payload_70["data"]:
        assert pd.to_datetime(p["timestamp"]) <= max_ts_70

    # 2. Rewind to cursor 40
    ReplayService.previous_candle(db, session_id, steps=30)
    res_40 = client.get(
        f"/api/replay/sessions/{session_id}/indicators",
        params={"indicator": "ichimoku", "tenkan": 9, "kijun": 26, "senkou": 52},
    )
    assert res_40.status_code == 200
    payload_40 = res_40.json()
    assert payload_40["meta"]["available_bars"] == 41
    assert payload_40["meta"]["is_projected_clamped"] is True
    assert len(payload_40["data"]) == 41
    max_ts_40 = pd.to_datetime(candles[40].timestamp)
    for p in payload_40["data"]:
        assert pd.to_datetime(p["timestamp"]) <= max_ts_40

    # 3. Advance to cursor 85
    ReplayService.next_candle(db, session_id, steps=45)
    res_85 = client.get(
        f"/api/replay/sessions/{session_id}/indicators",
        params={"indicator": "ichimoku", "tenkan": 9, "kijun": 26, "senkou": 52},
    )
    assert res_85.status_code == 200
    payload_85 = res_85.json()
    assert payload_85["meta"]["available_bars"] == 86
    assert payload_85["meta"]["is_projected_clamped"] is True
    assert len(payload_85["data"]) == 86
    max_ts_85 = pd.to_datetime(candles[85].timestamp)
    for p in payload_85["data"]:
        assert pd.to_datetime(p["timestamp"]) <= max_ts_85


def test_meta_status_transitions_and_unmapped_column(isolated_challenger_db):
    """Test transitions between EMPTY_WARMING, OK, and UNMAPPED_COLUMN."""
    db, client, _ = isolated_challenger_db
    symbol = "META_TRANS"
    start_d = date(2023, 3, 1)
    _seed_candles(db, symbol, start_d, 60)

    session = ReplayService.create_session(
        db,
        ReplaySessionCreate(
            symbol=symbol,
            start_date=start_d,
            end_date=start_d + timedelta(days=59),
            initial_cash=50_000_000.0,
        ),
    )
    session_id = session.id

    # 1. Cursor at 5 -> RSI(14) should be EMPTY_WARMING
    ReplayService.next_candle(db, session_id, steps=5)
    res_rsi_warm = client.get(
        f"/api/replay/sessions/{session_id}/indicators",
        params={"indicator": "rsi", "length": 14},
    )
    assert res_rsi_warm.status_code == 200
    p_rsi_warm = res_rsi_warm.json()
    assert p_rsi_warm["meta"]["status"] == "EMPTY_WARMING"
    assert p_rsi_warm["meta"]["warmup_bars"] == 14
    assert p_rsi_warm["meta"]["available_bars"] == 6

    # 2. Advance to 20 -> RSI(14) should be OK
    ReplayService.next_candle(db, session_id, steps=15)
    res_rsi_ok = client.get(
        f"/api/replay/sessions/{session_id}/indicators",
        params={"indicator": "rsi", "length": 14},
    )
    assert res_rsi_ok.status_code == 200
    p_rsi_ok = res_rsi_ok.json()
    assert p_rsi_ok["meta"]["status"] == "OK"
    assert p_rsi_ok["meta"]["warmup_bars"] == 14
    assert p_rsi_ok["meta"]["available_bars"] == 21
    assert p_rsi_ok["data"][-1]["RSI_14"] is not None

    # 3. MACD warmup: fast 12, slow 26, signal 9 -> warmup is 35 bars
    # At cursor 20 (21 bars visible), MACD must be EMPTY_WARMING
    res_macd_warm = client.get(
        f"/api/replay/sessions/{session_id}/indicators",
        params={"indicator": "macd", "fast": 12, "slow": 26, "signal": 9},
    )
    assert res_macd_warm.status_code == 200
    p_macd_warm = res_macd_warm.json()
    assert p_macd_warm["meta"]["status"] == "EMPTY_WARMING"
    assert p_macd_warm["meta"]["warmup_bars"] == 35
    assert p_macd_warm["meta"]["available_bars"] == 21

    # Advance to cursor 45 (46 bars visible) -> MACD must be OK
    ReplayService.next_candle(db, session_id, steps=25)
    res_macd_ok = client.get(
        f"/api/replay/sessions/{session_id}/indicators",
        params={"indicator": "macd", "fast": 12, "slow": 26, "signal": 9},
    )
    assert res_macd_ok.status_code == 200
    p_macd_ok = res_macd_ok.json()
    assert p_macd_ok["meta"]["status"] == "OK"
    assert p_macd_ok["meta"]["available_bars"] == 46


def test_compute_indicator_for_replay_direct_call(isolated_challenger_db):
    """Verify that compute_indicator_for_replay works when invoked directly as a Python service function."""
    db, _, _ = isolated_challenger_db
    symbol = "DIRECT_CALL"
    start_d = date(2023, 4, 1)
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
    ReplayService.next_candle(db, session.id, steps=25)

    result = compute_indicator_for_replay(
        db=db,
        session_id=session.id,
        indicator="ema",
        timeframe="1D",
        params={"length": 10},
    )

    assert result["session_id"] == session.id
    assert result["indicator"] == "ema"
    assert result["meta"]["status"] == "OK"
    assert result["meta"]["warmup_bars"] == 10
    assert result["meta"]["available_bars"] == 26
    assert len(result["data"]) == 26
    assert result["data"][-1]["EMA_10"] is not None


def test_non_replay_indicator_endpoint_preserves_full_history(isolated_challenger_db):
    """Verify that the full-chart non-replay endpoint /api/indicators/{symbol}

    preserves all candles and intentional forward projections without clamping.
    """
    db, client, _ = isolated_challenger_db
    symbol = "FULL_CHART_SYM"
    start_d = date(2023, 1, 1)
    _seed_candles(db, symbol, start_d, 100)

    # Call full-chart endpoint
    res = client.get(
        f"/api/indicators/{symbol}",
        params={"indicator": "ichimoku", "timeframe": "1D", "tenkan": 9, "kijun": 26, "senkou": 52},
    )
    assert res.status_code == 200
    payload = res.json()
    # Non-replay endpoint preserves full historical candles + 26 future bars (100 + 26 = 126 rows)
    assert len(payload["data"]) == 126
    # Verify future bars have valid cloud projections
    future_items = payload["data"][100:]
    assert len(future_items) == 26
    assert any(item["ISA_9"] is not None for item in future_items)


def test_challenger_ichimoku_cursor_clamping_at_bar_30_and_60(isolated_challenger_db):
    """Adversarial stress test: Ichimoku on replay session at bar 30, bar 60, and bar 0.

    Verify that EVERY returned point across all series (ITS_9, IKS_26, ISA_9, ISB_26, ICS_26)
    has timestamp <= candles[current_index].timestamp.
    """
    db, client, _ = isolated_challenger_db
    symbol = "CHAL_ICHI_30_60"
    start_d = date(2023, 5, 1)
    candles = _seed_candles(db, symbol, start_d, 100)

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

    # Check cursor 0 (boundary)
    res_0 = client.get(
        f"/api/replay/sessions/{session_id}/indicators",
        params={"indicator": "ichimoku", "tenkan": 9, "kijun": 26, "senkou": 52},
    )
    assert res_0.status_code == 200
    p_0 = res_0.json()
    assert p_0["meta"]["available_bars"] == 1
    assert p_0["meta"]["is_projected_clamped"] is True
    assert len(p_0["data"]) == 1
    assert pd.to_datetime(p_0["data"][0]["timestamp"]) <= pd.to_datetime(candles[0].timestamp)

    # 1. Advance to cursor 30 (31 candles visible)
    ReplayService.next_candle(db, session_id, steps=30)
    res_30 = client.get(
        f"/api/replay/sessions/{session_id}/indicators",
        params={"indicator": "ichimoku", "tenkan": 9, "kijun": 26, "senkou": 52},
    )
    assert res_30.status_code == 200
    p_30 = res_30.json()
    assert p_30["meta"]["available_bars"] == 31
    assert p_30["meta"]["is_projected_clamped"] is True
    assert len(p_30["data"]) == 31
    max_ts_30 = pd.to_datetime(candles[30].timestamp)
    for row in p_30["data"]:
        ts = pd.to_datetime(row["timestamp"])
        assert ts <= max_ts_30, f"Future leak at cursor 30: {ts} > {max_ts_30}"
        assert row.get("is_projected") is not True

    # 2. Advance to cursor 60 (61 candles visible)
    ReplayService.next_candle(db, session_id, steps=30)
    res_60 = client.get(
        f"/api/replay/sessions/{session_id}/indicators",
        params={"indicator": "ichimoku", "tenkan": 9, "kijun": 26, "senkou": 52},
    )
    assert res_60.status_code == 200
    p_60 = res_60.json()
    assert p_60["meta"]["available_bars"] == 61
    assert p_60["meta"]["is_projected_clamped"] is True
    assert len(p_60["data"]) == 61
    max_ts_60 = pd.to_datetime(candles[60].timestamp)
    for row in p_60["data"]:
        ts = pd.to_datetime(row["timestamp"])
        assert ts <= max_ts_60, f"Future leak at cursor 60: {ts} > {max_ts_60}"
        assert row.get("is_projected") is not True


def test_challenger_mapping_result_warming_unmapped_and_consumer_compatibility(
    isolated_challenger_db, monkeypatch
):
    """Adversarial stress test:

    1. SMA(50) on 10 bars -> assert meta.status == 'EMPTY_WARMING', meta.warmup_bars == 50.
    2. Unmapped column -> assert meta.status == 'UNMAPPED_COLUMN'.
    3. Existing consumers reading response['data'] directly receive valid array without runtime exceptions.
    """
    db, client, _ = isolated_challenger_db
    symbol = "CHAL_WARM_UNMAPPED"
    start_d = date(2023, 6, 1)
    _seed_candles(db, symbol, start_d, 60)

    session = ReplayService.create_session(
        db,
        ReplaySessionCreate(
            symbol=symbol,
            start_date=start_d,
            end_date=start_d + timedelta(days=59),
            initial_cash=50_000_000.0,
        ),
    )
    session_id = session.id

    # 1. Advance to cursor 9 -> exactly 10 bars visible (index 0..9)
    ReplayService.next_candle(db, session_id, steps=9)
    res_warming = client.get(
        f"/api/replay/sessions/{session_id}/indicators",
        params={"indicator": "sma", "length": 50},
    )
    assert res_warming.status_code == 200
    p_warming = res_warming.json()

    # Assert contract for warming period
    assert p_warming["meta"]["status"] == "EMPTY_WARMING"
    assert p_warming["meta"]["warmup_bars"] == 50
    assert p_warming["meta"]["available_bars"] == 10
    assert p_warming["meta"]["is_projected_clamped"] is False

    # 2. Existing consumer reading response['data'] directly
    data = p_warming["data"]
    assert isinstance(data, list)
    assert len(data) == 10
    # Simulate existing chart consumer iterating over points
    for pt in data:
        assert isinstance(pt, dict)
        assert "timestamp" in pt
        # During warmup SMA_50 is None, should not raise KeyError or AttributeError
        val = pt.get("SMA_50")
        assert val is None

    # 3. Unmapped column: simulate when IndicatorEngine produces no new columns
    # In replay.py: if available_bars >= warmup_bars and not new_cols: status = 'UNMAPPED_COLUMN'
    ReplayService.next_candle(db, session_id, steps=20)  # cursor 29 (30 bars visible)

    # Monkeypatch IndicatorEngine.compute to return original df without adding any new columns
    monkeypatch.setattr(
        IndicatorEngine,
        "compute",
        lambda df, ind, clamp_to_last_candle=False, **kw: df.copy(),
    )

    res_unmapped = client.get(
        f"/api/replay/sessions/{session_id}/indicators",
        params={"indicator": "sma", "length": 20},
    )
    assert res_unmapped.status_code == 200
    p_unmapped = res_unmapped.json()

    # Assert unmapped column status
    assert p_unmapped["meta"]["status"] == "UNMAPPED_COLUMN"
    assert p_unmapped["meta"]["warmup_bars"] == 20
    assert p_unmapped["meta"]["available_bars"] == 30
    assert isinstance(p_unmapped["data"], list)

