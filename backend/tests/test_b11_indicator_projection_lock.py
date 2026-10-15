"""Test Suite for Batch B11: Server-Side Replay Indicator Projection Clamping & Typed MappingResult Contract.

Covers:
- test_g10_replay_ichimoku_future_cloud_clamped_at_current_index
- test_non_replay_chart_preserves_intentional_forward_projection
- test_indicator_meta_warming_and_error_contract_compatibility

All tests use isolated temporary SQLite databases (tmp_path). Production backend/sumi.db is never accessed.
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


@pytest.fixture
def isolated_env(tmp_path) -> Generator[tuple[Session, TestClient, str], None, None]:
    """Provides an isolated SQLite database and TestClient with get_db overridden."""
    db_file = tmp_path / "test_b11_isolated.db"
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
    """Helper to seed daily OHLCV candles."""
    candles = []
    np.random.seed(42)
    close_price = 100.0
    for i in range(count):
        c_date = start_d + timedelta(days=i)
        step = np.random.randn() * 0.8
        close_price = max(20.0, close_price + step)
        c = Candle(
            symbol=symbol,
            timeframe="1D",
            adjustment_type="unadjusted",
            timestamp=datetime.combine(c_date, datetime.min.time()),
            open=float(close_price - 0.2),
            high=float(close_price + 1.5),
            low=float(close_price - 1.5),
            close=float(close_price),
            volume=float(100000 + i * 200),
        )
        db.add(c)
        candles.append(c)
    db.commit()
    return candles


def test_g10_replay_ichimoku_future_cloud_clamped_at_current_index(isolated_env):
    """Verify every returned point across TENKAN, KIJUN, SENKOU_A, SENKOU_B, and CHIKOU

    at current_index = 60 has timestamp <= candles[60].timestamp.
    Projected future cloud rows (+26 bars ahead) must be server-side clamped.
    """
    db, client, _ = isolated_env
    symbol = "ICHI_LOCK"
    start_d = date(2023, 1, 1)
    candles = _seed_candles(db, symbol, start_d, 100)

    # 1. Create replay session and advance to current_index = 60
    session_in = ReplaySessionCreate(
        symbol=symbol,
        start_date=start_d,
        end_date=start_d + timedelta(days=99),
        initial_cash=100_000_000.0,
    )
    session = ReplayService.create_session(db, session_in)
    session_id = session.id

    ReplayService.next_candle(db, session_id, steps=60)

    visible_candles = ReplayService.get_candles(db, session_id)
    assert len(visible_candles) == 61  # index 0..60
    max_visible_ts = visible_candles[60].timestamp
    max_visible_dt = pd.to_datetime(max_visible_ts)

    # 2. Fetch Ichimoku via replay indicator endpoint
    response = client.get(
        f"/api/replay/sessions/{session_id}/indicators",
        params={"indicator": "ichimoku", "tenkan": 9, "kijun": 26, "senkou": 52},
    )
    assert response.status_code == 200
    payload = response.json()

    assert payload["session_id"] == session_id
    assert payload["indicator"] == "ichimoku"

    # 3. Verify meta metadata
    meta = payload["meta"]
    assert meta["is_projected_clamped"] is True
    assert meta["available_bars"] == 61
    assert meta["warmup_bars"] == 52
    assert meta["status"] == "OK"

    data = payload["data"]
    assert len(data) == 61

    # 4. Verify that EVERY returned data point has timestamp <= max_visible_ts
    for item in data:
        item_ts = pd.to_datetime(item["timestamp"])
        assert item_ts <= max_visible_dt, f"Leaked future point: {item['timestamp']} > {max_visible_ts}"
        # Ensure is_projected flag is never True (future projected points dropped completely)
        assert item.get("is_projected") is not True

    # 5. Verify presence of all 5 Ichimoku series columns for visible rows
    for item in data:
        assert "ITS_9" in item  # Tenkan-sen
        assert "IKS_26" in item  # Kijun-sen
        assert "ISA_9" in item  # Senkou Span A
        assert "ISB_26" in item  # Senkou Span B
        assert "ICS_26" in item  # Chikou Span

    last_item = data[-1]
    # At bar 60 (> senkou 52, kijun 26), Tenkan, Kijun, and Span A are calculated
    assert last_item["ITS_9"] is not None
    assert last_item["IKS_26"] is not None
    assert last_item["ISA_9"] is not None

    # Advance to bar 80 (> 52 + 26) so Span B is also calculated
    ReplayService.next_candle(db, session_id, steps=20)
    res_80 = client.get(
        f"/api/replay/sessions/{session_id}/indicators",
        params={"indicator": "ichimoku", "tenkan": 9, "kijun": 26, "senkou": 52},
    )
    assert res_80.status_code == 200
    data_80 = res_80.json()["data"]
    assert len(data_80) == 81
    assert data_80[-1]["ISB_26"] is not None
    assert res_80.json()["meta"]["is_projected_clamped"] is True


def test_non_replay_chart_preserves_intentional_forward_projection():
    """Verify non-Replay full-chart calculation preserves forward-shifted SENKOU_A/SENKOU_B dates.

    When clamp_to_last_candle=False, the Ichimoku cloud projects +26 bars into the future.
    When clamp_to_last_candle=True, the returned dataframe is strictly clamped to the last input bar.
    """
    np.random.seed(42)
    dates = pd.date_range("2023-01-01", periods=120)
    close_prices = 100.0 + np.random.randn(120).cumsum()
    df = pd.DataFrame(
        {
            "open": close_prices - 0.2,
            "high": close_prices + 1.5,
            "low": close_prices - 1.5,
            "close": close_prices,
            "volume": np.random.randint(1000, 5000, 120),
        },
        index=dates,
    )

    # 1. Non-replay path (clamp_to_last_candle=False default)
    res_unclamped = IndicatorEngine.compute(
        df, "ichimoku", clamp_to_last_candle=False, tenkan=9, kijun=26, senkou=52
    )

    # Forward projection must be preserved: 120 + 26 = 146 rows
    assert len(res_unclamped) == len(df) + 26
    assert res_unclamped.index[-1] > df.index[-1]

    # Future 26 bars must have valid Span A & Span B forward cloud values
    future_rows = res_unclamped.loc[res_unclamped.index > df.index[-1]]
    assert len(future_rows) == 26
    assert future_rows["ISA_9"].notna().sum() >= 25
    assert future_rows["ISB_26"].notna().sum() >= 25

    # 2. Replay-mode path (clamp_to_last_candle=True)
    res_clamped = IndicatorEngine.compute(
        df, "ichimoku", clamp_to_last_candle=True, tenkan=9, kijun=26, senkou=52
    )

    # Must be strictly clamped to input dataframe length and date
    assert len(res_clamped) == len(df)
    assert res_clamped.index[-1] == df.index[-1]
    assert "ISA_9" in res_clamped.columns
    assert "ISB_26" in res_clamped.columns
    assert "ITS_9" in res_clamped.columns
    assert "IKS_26" in res_clamped.columns
    assert "ICS_26" in res_clamped.columns


def test_indicator_meta_warming_and_error_contract_compatibility(isolated_env):
    """Verify SMA(50) at current_index = 10 reports EMPTY_WARMING with warmup_bars == 50,

    available_bars == 11, and backward-compatible series payload.
    Also verify transition to OK once sufficient bars are available, and INVALID_SPEC on bad specs.
    """
    db, client, _ = isolated_env
    symbol = "SMA_WARM"
    start_d = date(2023, 1, 1)
    _seed_candles(db, symbol, start_d, 100)

    # 1. Create replay session and advance to current_index = 10 (11 bars visible: 0..10)
    session_in = ReplaySessionCreate(
        symbol=symbol,
        start_date=start_d,
        end_date=start_d + timedelta(days=99),
        initial_cash=100_000_000.0,
    )
    session = ReplayService.create_session(db, session_in)
    session_id = session.id

    client.post(f"/api/replay/sessions/{session_id}/next", params={"steps": 10})

    # 2. Query SMA(50) during warmup (only 11 bars visible)
    res_warming = client.get(
        f"/api/replay/sessions/{session_id}/indicators",
        params={"indicator": "sma", "length": 50},
    )
    assert res_warming.status_code == 200
    payload_warming = res_warming.json()

    meta_warming = payload_warming["meta"]
    assert meta_warming["status"] == "EMPTY_WARMING"
    assert meta_warming["warmup_bars"] == 50
    assert meta_warming["available_bars"] == 11
    assert meta_warming["is_projected_clamped"] is False

    # Data array must be backward-compatible (11 data points)
    data_warming = payload_warming["data"]
    assert len(data_warming) == 11
    for row in data_warming:
        assert "timestamp" in row
        # SMA_50 is either None or omitted during warmup
        assert row.get("SMA_50") is None

    # 3. Advance session past warmup requirement: steps=45 -> current_index=55 (56 bars visible)
    client.post(f"/api/replay/sessions/{session_id}/next", params={"steps": 45})

    res_ok = client.get(
        f"/api/replay/sessions/{session_id}/indicators",
        params={"indicator": "sma", "length": 50},
    )
    assert res_ok.status_code == 200
    payload_ok = res_ok.json()

    meta_ok = payload_ok["meta"]
    assert meta_ok["status"] == "OK"
    assert meta_ok["warmup_bars"] == 50
    assert meta_ok["available_bars"] == 56
    assert meta_ok["is_projected_clamped"] is False

    data_ok = payload_ok["data"]
    assert len(data_ok) == 56
    # Last bar (index 55 >= 50) has a valid numeric value for SMA_50
    assert data_ok[-1]["SMA_50"] is not None
    assert isinstance(data_ok[-1]["SMA_50"], (int, float))

    # 4. Verify INVALID_SPEC handling for unsupported indicator
    res_invalid = client.get(
        f"/api/replay/sessions/{session_id}/indicators",
        params={"indicator": "non_existent_indicator_xyz"},
    )
    assert res_invalid.status_code == 200
    payload_invalid = res_invalid.json()
    assert payload_invalid["meta"]["status"] == "INVALID_SPEC"
    assert payload_invalid["data"] == []
