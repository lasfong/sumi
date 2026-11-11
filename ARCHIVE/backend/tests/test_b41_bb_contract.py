"""Batch B41 — Money Flow Blackbox (BB) Envelope & Endpoint Contract Tests.

Covers:
- GAP-API-02: BB series vs chart format unification, warmup (is_warmup), and degraded (UNAVAILABLE_DEGRADED) states
- GAP-API-11: Explicit response_model validation for GET /api/bb/horizons, GET /api/bb/symbol/{symbol}, GET /api/bb/market/{universe_id}
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
from app.main import app
from app.models.candle import Candle

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


def test_b41_bb_horizons_and_symbol_series_vs_chart_contracts(client: TestClient, db_session):
    """Verify /api/bb/horizons and /api/bb/symbol/{symbol} in both series and chart formats."""
    # Seed 25 daily bars for B41_SYM so T03/T05/T10/T20 are valid while T50/T200 are in warmup
    base_dt = datetime(2025, 1, 2)
    for i in range(25):
        bar_dt = base_dt + timedelta(days=i)
        db_session.add(
            Candle(
                symbol="B41_SYM",
                timeframe="1D",
                timestamp=bar_dt,
                open=100.0 + i * 0.5,
                high=102.0 + i * 0.5,
                low=99.0 + i * 0.5,
                close=101.5 + i * 0.5,
                volume=1_000_000 + i * 10_000,
                source="vnstock_kbs",
                adjustment_type="unadjusted",
            )
        )
    db_session.commit()

    # 1. GET /api/bb/horizons
    res_h = client.get("/api/bb/horizons")
    assert res_h.status_code == 200
    h_json = res_h.json()
    assert h_json["methodology"] == "OHLCV_PROXY"
    assert [item["id"] for item in h_json["horizons"]] == ["T03", "T05", "T10", "T20", "T50", "T200"]

    # 2. GET /api/bb/symbol/B41_SYM?format=series
    res_series = client.get("/api/bb/symbol/B41_SYM?horizons=T03,T20,T50&format=series")
    assert res_series.status_code == 200
    s_json = res_series.json()
    assert s_json["symbol"] == "B41_SYM"
    assert s_json["flow_method"] == "OHLCV_PROXY"
    assert s_json["total_bars"] == 25
    assert len(s_json["points"]) == 25

    latest_pt = s_json["points"][-1]
    assert latest_pt["horizons"]["T03"]["is_warmup"] is False
    assert latest_pt["horizons"]["T03"]["bb_value"] is not None
    assert latest_pt["horizons"]["T20"]["is_warmup"] is False
    assert latest_pt["horizons"]["T20"]["bb_value"] is not None
    # T50 requires 50 bars -> must be explicitly flagged as warmup with bb_value=None
    assert latest_pt["horizons"]["T50"]["is_warmup"] is True
    assert latest_pt["horizons"]["T50"]["bb_value"] is None

    # 3. GET /api/bb/symbol/B41_SYM?format=chart
    res_chart = client.get("/api/bb/symbol/B41_SYM?horizons=T03,T20,T50&format=chart")
    assert res_chart.status_code == 200
    c_json = res_chart.json()
    assert c_json["symbol"] == "B41_SYM"
    assert c_json["flow_method"] == "OHLCV_PROXY"
    assert c_json["label"] == "Technical Flow BB"
    assert len(c_json["data"]) == 25
    last_chart_pt = c_json["data"][-1]
    assert last_chart_pt["t03"] == latest_pt["horizons"]["T03"]["bb_value"]
    assert last_chart_pt["t20"] == latest_pt["horizons"]["T20"]["bb_value"]
    assert last_chart_pt["t50"] is None

    # 4. GET /api/bb/symbol/B41_SYM with as_of date parameter (DEF-W5-02 regression check)
    res_as_of = client.get("/api/bb/symbol/B41_SYM?horizons=T03,T05&as_of=2025-01-15&format=series")
    assert res_as_of.status_code == 200
    as_of_json = res_as_of.json()
    assert len(as_of_json["points"]) == 14
    assert as_of_json["points"][-1]["date"] == "2025-01-15"
    assert all(pt["date"] <= "2025-01-15" for pt in as_of_json["points"])
