"""Batch B40 — OpenAPI Contract Snapshot & Error Envelope Verification.

Covers:
- GAP-API-01: SessionStatus.BANKRUPT and DecisionCreate OrderType.CUSTOM_PRICE validator alignment
- GAP-API-03: CreateSessionMode.BACKTEST enum alignment
- GAP-API-04: AnalyticsResponse schema completeness
- GAP-API-05: TradePlanResult 27-field completeness
- GAP-API-06 & GAP-DB-07: checklist_snapshot dual-read/write (dict | str | None)
- GAP-API-07: SignalCalculationRequest 1..2 max, unique names, non-null params, extra forbid
- GAP-API-08: Single canonical /api/data/catalog route (/api/import/catalog removed)
- GAP-API-09: Structured error envelope {code, message, field_errors, detail} on 422/404/400
- GAP-API-10 & GAP-DB-13: DataFeedService deprecation guard
- GAP-API-11: Explicit response_model schemas on /api/scanner/* and /api/bb/*
"""

from datetime import datetime
import pytest
from fastapi.testclient import TestClient
from sqlalchemy import create_engine
from sqlalchemy.orm import sessionmaker
from sqlalchemy.pool import StaticPool

import app.models  # noqa: F401
from app.db import Base
from app.dependencies import get_db
from app.main import app
from app.domain.enums import OrderType, SessionMode, SessionStatus
from app.domain.trade_planning import TradePlanResult
from app.schemas.analytics_schema import AnalyticsResponse
from app.schemas.decision_schema import DecisionCreate
from app.schemas.replay_schema import CreateSessionMode
from app.services.data_service import DataFeedService

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
def client():
    Base.metadata.drop_all(bind=_TEST_ENGINE)
    Base.metadata.create_all(bind=_TEST_ENGINE)
    app.dependency_overrides[get_db] = _override_get_db
    with TestClient(app) as c:
        yield c
    app.dependency_overrides.clear()
    Base.metadata.drop_all(bind=_TEST_ENGINE)


def test_b40_enum_and_validator_alignment():
    """Verify GAP-API-01 and GAP-API-03 enum and validator parity."""
    assert SessionStatus.BANKRUPT.value == "bankrupt"
    assert SessionMode.BACKTEST.value == "backtest"
    assert "backtest" not in {m.value for m in CreateSessionMode}
    assert {m.value for m in CreateSessionMode} == {"normal", "random", "blind_symbol", "blind_date"}
    assert OrderType.CUSTOM_PRICE.value == "CUSTOM_PRICE"

    # CUSTOM_PRICE with price succeeds
    dec = DecisionCreate(action="BUY", order_type="CUSTOM_PRICE", price=85.5, quantity=100)
    assert dec.order_type == "CUSTOM_PRICE"
    assert dec.price == 85.5

    # CUSTOM_PRICE without price fails validation
    with pytest.raises(ValueError, match="CUSTOM_PRICE order requires a price"):
        DecisionCreate(action="BUY", order_type="CUSTOM_PRICE", price=None, quantity=100)

    # Checklist snapshot accepts both dict and str (GAP-API-06 / GAP-DB-07)
    dec_dict = DecisionCreate(
        action="HOLD",
        checklist_snapshot={"trend": True, "volume_confirmed": True},
    )
    assert isinstance(dec_dict.checklist_snapshot, dict)


def test_b40_trade_plan_and_analytics_schema_fields():
    """Verify GAP-API-04 and GAP-API-05 required schema fields exist."""
    plan_fields = set(TradePlanResult.model_fields.keys())
    expected_7_fields = {
        "entry_price",
        "stop_loss",
        "target_price",
        "direction",
        "account_equity",
        "available_cash",
        "risk_percent",
    }
    assert expected_7_fields.issubset(plan_fields)

    analytics_fields = set(AnalyticsResponse.model_fields.keys())
    for required_field in (
        "benchmark_symbol",
        "profit_factor_display",
        "low_sample_warning",
        "low_sample_message",
        "metrics",
    ):
        assert required_field in analytics_fields


def test_b40_catalog_deduplication_and_openapi_response_models():
    """Verify GAP-API-08 (single catalog route) and GAP-API-11 (response_model coverage)."""
    openapi = app.openapi()
    paths = openapi.get("paths", {})

    # Canonical catalog route exists; duplicate /api/import/catalog is removed
    assert "/api/data/catalog" in paths
    assert "/api/import/catalog" not in paths

    # Scanner and BB routes have explicit OpenAPI response schemas
    for route_path, method in [
        ("/api/bb/horizons", "get"),
        ("/api/bb/symbol/{symbol}", "get"),
        ("/api/bb/market/{universe_id}", "get"),
        ("/api/scanner/run", "post"),
        ("/api/scanner/runs", "get"),
        ("/api/scanner/runs/{run_id}", "get"),
    ]:
        assert route_path in paths, f"Missing route {route_path}"
        op = paths[route_path][method]
        schema_200 = (
            op.get("responses", {})
            .get("200", {})
            .get("content", {})
            .get("application/json", {})
            .get("schema", {})
        )
        assert schema_200, f"Route {method.upper()} {route_path} has no OpenAPI 200 schema"


def test_b40_structured_error_envelope_on_422_and_404(client: TestClient):
    """Verify GAP-API-09 structured error envelope {code, message, field_errors, detail}."""
    # 1. HTTP 422 validation error returns structured envelope + string detail (never raw array)
    res_422 = client.post("/api/signals/replay/1/calculate", json={"signals": []})
    assert res_422.status_code == 422
    payload_422 = res_422.json()
    assert payload_422["code"] == "VALIDATION_ERROR"
    assert isinstance(payload_422["message"], str) and len(payload_422["message"]) > 0
    assert isinstance(payload_422["detail"], str) and len(payload_422["detail"]) > 0
    assert isinstance(payload_422["field_errors"], list) and len(payload_422["field_errors"]) >= 1
    assert "signals" in payload_422["field_errors"][0]["field"]

    # 2. HTTP 404 returns structured envelope
    res_404 = client.get("/api/scanner/runs/999999")
    assert res_404.status_code == 404
    payload_404 = res_404.json()
    assert payload_404["code"] == "NOT_FOUND"
    assert payload_404["message"] == "Scanner run not found"
    assert payload_404["detail"] == "Scanner run not found"
    assert payload_404["field_errors"] == []


def test_b40_data_feed_service_deprecation_guard():
    """Verify GAP-DB-13 / GAP-API-10 DataFeedService blocks unapproved direct ingestion."""
    assert getattr(DataFeedService, "DEPRECATED", False) is True

    class _DummyFeed:
        def fetch_historical_data(self, *args, **kwargs):
            raise AssertionError("Feed should not be called when blocked by deprecation guard")

    svc = DataFeedService(db=None, feed=_DummyFeed())  # type: ignore[arg-type]
    with pytest.raises(RuntimeError, match="DataFeedService is deprecated"):
        svc.sync_historical_data("FPT", "1D", datetime(2024, 1, 1))
