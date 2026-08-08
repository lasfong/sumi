from datetime import date
import pytest
from sqlalchemy import create_engine
from sqlalchemy.orm import sessionmaker
from sqlalchemy.pool import StaticPool
from fastapi.testclient import TestClient

from app.main import app
from app.db import Base
from app.dependencies import get_db
from app.models.symbol import Symbol
from app.models.candle import Candle
from app.services.readiness_service import ReadinessService


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


@pytest.fixture(autouse=True)
def setup_db():
    Base.metadata.create_all(bind=test_engine)
    app.dependency_overrides[get_db] = override_get_db
    yield
    Base.metadata.drop_all(bind=test_engine)


@pytest.fixture
def client():
    with TestClient(app) as c:
        yield c


def test_readiness_service_unit():
    db = TestingSessionLocal()
    res = ReadinessService.get_readiness(db)
    assert res.status == "empty"
    assert res.total_candles == 0
    assert res.symbols_with_candles == []
    db.close()


def test_readiness_symbol_metadata_with_zero_candles(client):
    db = TestingSessionLocal()
    db.add(Symbol(symbol="ZERO_CANDLE", company_name="Zero Candle Corp", exchange="HOSE", asset_type="equity"))
    db.commit()
    db.close()

    response = client.get("/api/symbols/readiness")
    assert response.status_code == 200
    data = response.json()
    assert data["status"] == "empty"
    assert data["symbols_count"] == 1
    assert data["total_candles"] == 0
    assert "ZERO_CANDLE" not in data["symbols_with_candles"]


def test_readiness_seeded_candles_ready(client):
    db = TestingSessionLocal()
    db.add(Symbol(symbol="READY_SYM", company_name="Ready Sym Corp", exchange="HOSE", asset_type="equity"))
    c1 = Candle(symbol="READY_SYM", timeframe="1D", timestamp=date(2024, 1, 1), open=10.0, high=11.0, low=9.5, close=10.5, volume=1000)
    c2 = Candle(symbol="READY_SYM", timeframe="1D", timestamp=date(2024, 1, 5), open=10.5, high=12.0, low=10.0, close=11.5, volume=1500)
    db.add_all([c1, c2])
    db.commit()
    db.close()

    response = client.get("/api/symbols/readiness")
    assert response.status_code == 200
    data = response.json()
    assert data["status"] == "ready"
    assert "READY_SYM" in data["symbols_with_candles"]
    assert data["total_candles"] == 2
    assert "1D" in data["timeframes"]
    assert data["earliest_timestamp"] == "2024-01-01"
    assert data["latest_timestamp"] == "2024-01-05"
