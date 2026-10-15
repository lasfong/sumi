"""
Empirical Challenger Test Suite — SettlementLockDetail & Exception Serialization Contracts
Milestone: Sumi V3 Train 02 Batch B21 (Challenger B21-R2-2)
"""

import json
from datetime import date, datetime, timedelta
from typing import Generator
import pytest
from fastapi import FastAPI, HTTPException
from fastapi.encoders import jsonable_encoder
from fastapi.testclient import TestClient
from sqlalchemy import create_engine
from sqlalchemy.orm import Session, sessionmaker

from app.db import Base
from app.domain.market.settlement import SettlementLockDetail
from app.models.candle import Candle
from app.domain.enums import DecisionAction
from app.schemas.replay_schema import ReplaySessionCreate
from app.schemas.decision_schema import DecisionCreate
from app.services.replay_service import ReplayService
from app.services.trade_lifecycle_service import TradeLifecycleService


@pytest.fixture
def isolated_db(tmp_path) -> Generator[Session, None, None]:
    """Isolated SQLite database session for challenger tests."""
    db_file = tmp_path / "test_b21_contract.db"
    engine = create_engine(
        f"sqlite:///{db_file}",
        connect_args={"check_same_thread": False},
    )
    Base.metadata.create_all(bind=engine)
    TestingSessionLocal = sessionmaker(autocommit=False, autoflush=False, bind=engine)
    session = TestingSessionLocal()
    try:
        yield session
    finally:
        session.close()
        engine.dispose()



def create_sample_settlement_detail(
    available_qty: float = 0.0,
    blocked_qty: float = 200.0,
    release_date_str: str = "2024-06-03",
    settlement_label: str = "T+1 · về sau 1 phiên",
) -> SettlementLockDetail:
    """Helper to generate a canonical SettlementLockDetail matching TradeLifecycleService."""
    return SettlementLockDetail(
        error_code="LOT_SETTLEMENT_LOCKED",
        status="REJECTED_SETTLEMENT_LOCK",
        message=(
            f"Cannot sell: T+2 constraint. Shares locked under {settlement_label}. "
            f"Available: {available_qty:g}, Blocked: {blocked_qty:g}, Earliest release date: {release_date_str}"
        ),
        available_qty=available_qty,
        available_quantity=available_qty,
        blocked_qty=blocked_qty,
        blocked_quantity=blocked_qty,
        available_on=release_date_str,
        earliest_release_date=release_date_str,
        settlement_label=settlement_label,
    )


class TestSettlementLockDetailDictionaryContracts:
    """Requirement 1: Verify dictionary contracts."""

    def test_isinstance_dict(self):
        detail = create_sample_settlement_detail()
        assert isinstance(detail, dict), "SettlementLockDetail must be an instance of dict"
        assert issubclass(SettlementLockDetail, dict), "SettlementLockDetail must subclass dict"

    def test_error_code_and_status(self):
        detail = create_sample_settlement_detail()
        assert "error_code" in detail
        assert detail["error_code"] == "LOT_SETTLEMENT_LOCKED"
        assert "status" in detail
        assert detail["status"] == "REJECTED_SETTLEMENT_LOCK"

    def test_available_quantity_aliases_present_and_equal(self):
        detail = create_sample_settlement_detail(available_qty=50.0)
        assert "available_qty" in detail
        assert "available_quantity" in detail
        assert detail["available_qty"] == 50.0
        assert detail["available_quantity"] == 50.0
        assert detail["available_qty"] == detail["available_quantity"]

    def test_blocked_quantity_aliases_present_and_equal(self):
        detail = create_sample_settlement_detail(blocked_qty=200.0)
        assert "blocked_qty" in detail
        assert "blocked_quantity" in detail
        assert detail["blocked_qty"] == 200.0
        assert detail["blocked_quantity"] == 200.0
        assert detail["blocked_qty"] == detail["blocked_quantity"]

    def test_release_date_aliases_present_and_equal(self):
        detail = create_sample_settlement_detail(release_date_str="2024-06-03")
        assert "available_on" in detail
        assert "earliest_release_date" in detail
        assert detail["available_on"] == "2024-06-03"
        assert detail["earliest_release_date"] == "2024-06-03"
        assert detail["available_on"] == detail["earliest_release_date"]

    def test_settlement_label_present(self):
        detail = create_sample_settlement_detail()
        assert "settlement_label" in detail
        assert detail["settlement_label"] == "T+1 · về sau 1 phiên"


class TestSettlementLockDetailLegacyStringContracts:
    """Requirement 2: Verify legacy string compatibility assertions."""

    def test_t2_constraint_substring_in_detail(self):
        detail = create_sample_settlement_detail()
        assert "T+2 constraint" in detail, "'T+2 constraint' must be detectable via `in detail`"

    def test_cannot_sell_t2_constraint_in_str_detail(self):
        detail = create_sample_settlement_detail()
        assert "Cannot sell: T+2 constraint" in str(detail), (
            "'Cannot sell: T+2 constraint' must be in str(detail)"
        )

    def test_cannot_sell_t2_constraint_directly_in_detail(self):
        detail = create_sample_settlement_detail()
        assert "Cannot sell: T+2 constraint" in detail, (
            "'Cannot sell: T+2 constraint' must also be directly detectable via `in detail`"
        )

    def test_available_0_in_detail(self):
        detail = create_sample_settlement_detail(available_qty=0.0)
        assert "Available: 0" in detail, "'Available: 0' must be detectable via `in detail`"

    def test_blocked_200_in_detail(self):
        detail = create_sample_settlement_detail(blocked_qty=200.0)
        assert "Blocked: 200" in detail, "'Blocked: 200' must be detectable via `in detail`"

    def test_string_convenience_methods(self):
        detail = create_sample_settlement_detail()
        # Equality with message or str
        assert detail == detail["message"]
        assert detail == str(detail)
        # startswith, endswith, lower
        assert detail.startswith("Cannot sell")
        assert detail.lower().startswith("cannot sell")
        assert detail.endswith("2024-06-03")


class TestSettlementLockDetailFastAPISerialization:
    """Requirement 3: Verify FastAPI serialization."""

    def test_jsonable_encoder_preserves_all_keys_and_values(self):
        detail = create_sample_settlement_detail(available_qty=100.0, blocked_qty=300.0, release_date_str="2024-07-15")
        encoded = jsonable_encoder(detail)
        assert isinstance(encoded, dict)
        assert encoded["error_code"] == "LOT_SETTLEMENT_LOCKED"
        assert encoded["status"] == "REJECTED_SETTLEMENT_LOCK"
        assert encoded["available_qty"] == 100.0
        assert encoded["available_quantity"] == 100.0
        assert encoded["blocked_qty"] == 300.0
        assert encoded["blocked_quantity"] == 300.0
        assert encoded["available_on"] == "2024-07-15"
        assert encoded["earliest_release_date"] == "2024-07-15"
        assert encoded["settlement_label"] == "T+1 · về sau 1 phiên"

    def test_json_dumps_roundtrip(self):
        detail = create_sample_settlement_detail()
        raw_json = json.dumps(jsonable_encoder(detail))
        parsed = json.loads(raw_json)
        assert parsed["error_code"] == "LOT_SETTLEMENT_LOCKED"
        assert parsed["available_quantity"] == 0.0
        assert parsed["blocked_quantity"] == 200.0

    def test_jsonable_encoder_with_raw_date_objects(self):
        detail = SettlementLockDetail(
            error_code="LOT_SETTLEMENT_LOCKED",
            status="REJECTED_SETTLEMENT_LOCK",
            message="Locked",
            available_qty=0,
            blocked_qty=100,
            available_on=date(2024, 6, 3),
            earliest_release_date=date(2024, 6, 3),
        )
        encoded = jsonable_encoder(detail)
        assert encoded["available_on"] == "2024-06-03"
        assert encoded["earliest_release_date"] == "2024-06-03"

    def test_fastapi_test_client_http_exception_serialization(self):
        """Simulate live FastAPI app returning HTTPException(400, detail=detail)."""
        app = FastAPI()

        @app.post("/test-settlement-rejection")
        def route_reject():
            raise HTTPException(
                status_code=400,
                detail=create_sample_settlement_detail(available_qty=0.0, blocked_qty=200.0),
            )

        client = TestClient(app)
        response = client.post("/test-settlement-rejection")
        assert response.status_code == 400
        body = response.json()
        assert "detail" in body
        resp_detail = body["detail"]
        assert isinstance(resp_detail, dict)
        assert resp_detail["error_code"] == "LOT_SETTLEMENT_LOCKED"
        assert resp_detail["status"] == "REJECTED_SETTLEMENT_LOCK"
        assert resp_detail["available_qty"] == 0.0
        assert resp_detail["available_quantity"] == 0.0
        assert resp_detail["blocked_qty"] == 200.0
        assert resp_detail["blocked_quantity"] == 200.0
        assert resp_detail["available_on"] == "2024-06-03"
        assert resp_detail["earliest_release_date"] == "2024-06-03"
        assert resp_detail["settlement_label"] == "T+1 · về sau 1 phiên"


class TestSettlementLockDetailAliasAutoPopulation:
    """Stress test: alias synchronization when only one variant is provided in __init__."""

    def test_init_with_only_available_qty(self):
        detail = SettlementLockDetail(available_qty=150.0)
        assert detail["available_qty"] == 150.0
        assert detail["available_quantity"] == 150.0

    def test_init_with_only_available_quantity(self):
        detail = SettlementLockDetail(available_quantity=250.0)
        assert detail["available_qty"] == 250.0
        assert detail["available_quantity"] == 250.0

    def test_init_with_only_blocked_qty(self):
        detail = SettlementLockDetail(blocked_qty=500.0)
        assert detail["blocked_qty"] == 500.0
        assert detail["blocked_quantity"] == 500.0

    def test_init_with_only_blocked_quantity(self):
        detail = SettlementLockDetail(blocked_quantity=600.0)
        assert detail["blocked_qty"] == 600.0
        assert detail["blocked_quantity"] == 600.0

    def test_init_with_only_available_on(self):
        detail = SettlementLockDetail(available_on="2024-08-01")
        assert detail["available_on"] == "2024-08-01"
        assert detail["earliest_release_date"] == "2024-08-01"

    def test_init_with_only_earliest_release_date(self):
        detail = SettlementLockDetail(earliest_release_date="2024-08-02")
        assert detail["available_on"] == "2024-08-02"
        assert detail["earliest_release_date"] == "2024-08-02"


class TestSettlementLockDetailAdversarialBoundaries:
    """Adversarial probe of dictionary and string edge cases."""

    def test_membership_non_string_key(self):
        detail = create_sample_settlement_detail()
        assert 123 not in detail
        assert None not in detail
        assert () not in detail

    def test_membership_absent_substring(self):
        detail = create_sample_settlement_detail()
        assert "NOT_IN_ANY_STRING_OR_KEY" not in detail

    def test_membership_empty_string(self):
        detail = create_sample_settlement_detail()
        # Empty string is in any string
        assert "" in detail

    def test_missing_message_key_graceful_string_methods(self):
        detail = SettlementLockDetail(error_code="TEST")
        # message is not set; should return "" rather than crash
        assert detail.lower() == ""
        assert detail.startswith("") is True
        assert detail.startswith("Cannot") is False
        assert detail.endswith("") is True

    def test_dict_methods_integrity(self):
        detail = create_sample_settlement_detail()
        keys = list(detail.keys())
        assert "error_code" in keys
        assert "status" in keys
        assert "available_quantity" in keys
        assert "blocked_quantity" in keys
        assert "earliest_release_date" in keys

        # items() and values()
        items = dict(detail.items())
        assert items["error_code"] == "LOT_SETTLEMENT_LOCKED"

        # copy()
        copied = detail.copy()
        assert isinstance(copied, dict)
        assert copied == detail


class TestLiveTradeLifecycleRejectionContract:
    """Empirical end-to-end probe of TradeLifecycleService HTTPException detail payload."""

    def test_lifecycle_rejection_payload_strictly_satisfies_all_contracts(self, isolated_db: Session):
        db = isolated_db
        symbol = "ADV_REJECT"

        # 5 daily candles starting on Friday 2024-05-31
        start_d = date(2024, 5, 31)
        for i in range(5):
            db.add(Candle(
                symbol=symbol, timeframe="1D", timestamp=start_d + timedelta(days=i),
                open=100.0, high=102.0, low=98.0, close=100.0, volume=1000,
                adjustment_type="unadjusted"
            ))
        db.commit()

        session = ReplayService.create_session(
            db,
            ReplaySessionCreate(
                symbol=symbol,
                start_date=start_d,
                end_date=start_d + timedelta(days=4),
                initial_cash=100_000_000.0,
            )
        )

        # Bar 0 (T0): Buy 200 shares
        TradeLifecycleService.process_decision(
            db, session.id, DecisionCreate(action=DecisionAction.BUY, quantity=200)
        )

        # Step to Bar 1 (T1)
        ReplayService.next_candle(db, session.id)

        # Attempt to sell 100 shares at T1 -> Must raise HTTPException
        with pytest.raises(HTTPException) as excinfo:
            TradeLifecycleService.process_decision(
                db, session.id, DecisionCreate(action=DecisionAction.SELL, quantity=100)
            )

        assert excinfo.value.status_code == 400
        detail = excinfo.value.detail

        # 1. Dictionary contracts
        assert isinstance(detail, dict), "detail must be dict"
        assert isinstance(detail, SettlementLockDetail), "detail must be SettlementLockDetail"
        assert "error_code" in detail and detail["error_code"] == "LOT_SETTLEMENT_LOCKED"
        assert "status" in detail and detail["status"] == "REJECTED_SETTLEMENT_LOCK"
        assert "available_qty" in detail and "available_quantity" in detail
        assert detail["available_qty"] == detail["available_quantity"] == 0.0
        assert "blocked_qty" in detail and "blocked_quantity" in detail
        assert detail["blocked_qty"] == detail["blocked_quantity"] == 200.0
        assert "available_on" in detail and "earliest_release_date" in detail
        assert detail["available_on"] == detail["earliest_release_date"]
        assert detail["settlement_label"] == "T+1 · về sau 1 phiên"

        # 2. Legacy string compatibility assertions
        assert "T+2 constraint" in detail
        assert "Cannot sell: T+2 constraint" in str(detail)
        assert "Available: 0" in detail
        assert "Blocked: 200" in detail

        # 3. FastAPI serialization
        encoded = jsonable_encoder(detail)
        assert isinstance(encoded, dict)
        assert encoded["error_code"] == "LOT_SETTLEMENT_LOCKED"
        assert encoded["status"] == "REJECTED_SETTLEMENT_LOCK"
        assert encoded["available_quantity"] == 0.0
        assert encoded["blocked_quantity"] == 200.0
        assert encoded["settlement_label"] == "T+1 · về sau 1 phiên"
        serialized = json.dumps(encoded)
        assert "LOT_SETTLEMENT_LOCKED" in serialized
