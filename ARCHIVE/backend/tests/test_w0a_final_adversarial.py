"""Adversarial Acceptance and Stress Suite for Sumi V3 W0-A Final Remediation.

Empirical verification suite authored by challenger_w0a (Agent D — Independent Adversarial Verifier)
verifying remediations for:
- Finding R1: Rollback Fail-Closed (Tests 1-7)
- Finding R2: Multiple Quarantine Candidates (Tests 8-14)
- Finding R3: Production DB Safeguard Baseline (Tests 15-17)

All tests operate strictly against isolated temporary databases and assert zero delta on production db.
"""

from __future__ import annotations

import hashlib
import os
import sqlite3
from datetime import date, datetime, timezone
from pathlib import Path
from unittest.mock import patch
import pytest
from fastapi import HTTPException
from sqlalchemy import create_engine, select
from sqlalchemy.orm import Session

from app.db import Base
from app.models.candle import Candle
from app.models.symbol import Symbol
from app.models.import_run import ImportRun, ImportRunMutation
from app.models.sync_run import SyncRun, SyncRunMutation
from app.services.data_providers.base_provider import (
    MarketDataProviderAdapter,
    ProviderCandleDTO,
    ProviderMetadata,
)
from app.services.data_providers.provider_registry import ProviderRegistry
from app.services.import_workflow_service import ImportWorkflowService
from app.services.sync_workflow_service import SyncWorkflowService
from app.services.weekly_aggregator import WeeklyAggregator
from tests_safety import (
    PROD_DB_PATH,
    capture_production_database_baseline,
    get_session_database_baseline,
    install_database_safeguard,
    is_session_baseline_locked,
    verify_production_database_integrity,
)


# ==============================================================================
# FIXTURES & HELPERS
# ==============================================================================

@pytest.fixture
def adv_db(tmp_path: Path):
    """Isolated file database per test."""
    db_file = tmp_path / "adv_w0a_test.db"
    engine = create_engine(f"sqlite:///{db_file.as_posix()}", echo=False)
    Base.metadata.create_all(bind=engine)
    session = Session(bind=engine)
    try:
        yield session
    finally:
        session.close()
        engine.dispose()


class W0AMockProvider(MarketDataProviderAdapter):
    def __init__(self, metadata: ProviderMetadata, candles: list[ProviderCandleDTO]):
        self._metadata = metadata
        self._candles = candles

    def get_metadata(self) -> ProviderMetadata:
        return self._metadata

    def test_connection(self, credentials=None) -> bool:
        return True

    def fetch_daily_candles(
        self,
        symbol: str,
        start_date: date,
        end_date: date,
        adjustment_type: str = "unadjusted",
        credentials=None,
    ) -> list[ProviderCandleDTO]:
        return self._candles

    def fetch_benchmark_indices(
        self,
        benchmark: str,
        start_date: date,
        end_date: date,
        credentials=None,
    ) -> list[ProviderCandleDTO]:
        return []


def _snapshot_candles(db: Session) -> tuple[int, str]:
    """Capture total count and deterministic SHA-256 hash of all candle rows in isolated session."""
    rows = db.execute(
        select(
            Candle.symbol,
            Candle.timeframe,
            Candle.timestamp,
            Candle.open,
            Candle.high,
            Candle.low,
            Candle.close,
            Candle.volume,
            Candle.source,
        )
        .execution_options(include_quarantine=True)
        .order_by(Candle.symbol, Candle.timestamp)
    ).all()
    count = len(rows)
    content = repr(rows).encode("utf-8")
    return count, hashlib.sha256(content).hexdigest()


# ==============================================================================
# FINDING R1: ROLLBACK FAIL-CLOSED TESTS (1 - 7)
# ==============================================================================

def test_01_malformed_provenance_timestamp_fails_closed(adv_db: Session):
    """Test 1: Malformed provenance timestamp raises HTTP 400 and run remains accepted."""
    adv_db.add(Symbol(symbol="HPG", is_active=True))
    dt = datetime(2026, 8, 3, 0, 0, 0)
    candle = Candle(
        symbol="HPG",
        timeframe="1D",
        timestamp=dt,
        adjustment_type="unadjusted",
        open=20.0,
        high=21.0,
        low=19.0,
        close=20.5,
        volume=1000.0,
        source="run_imp_malformed",
    )
    adv_db.add(candle)
    run = ImportRun(
        id="run_imp_malformed",
        file_name="test.csv",
        file_sha256="dummy",
        content_sha256="dummy",
        status="accepted",
        accepted_at=datetime.now(timezone.utc),
    )
    adv_db.add(run)
    mutation = ImportRunMutation(
        run_id="run_imp_malformed",
        symbol="HPG",
        timeframe="1D",
        timestamp=dt,
        adjustment_type="unadjusted",
        action="UPDATE:quarantine:seed|not-a-valid-timestamp-xyz",
        before_open=10.0,
        before_high=11.0,
        before_low=9.0,
        before_close=10.5,
        before_volume=500.0,
    )
    adv_db.add(mutation)
    adv_db.commit()

    with pytest.raises(HTTPException) as exc_info:
        ImportWorkflowService.rollback_import(adv_db, "run_imp_malformed")

    assert exc_info.value.status_code == 400
    assert "Malformed provenance timestamp" in exc_info.value.detail or "không hợp lệ" in exc_info.value.detail

    # Verify run remains accepted (never marked rolled_back)
    r = adv_db.execute(select(ImportRun).filter_by(id="run_imp_malformed")).scalar_one()
    assert r.status == "accepted"

    # Verify candle in DB was NOT mutated
    c = adv_db.execute(select(Candle).filter_by(symbol="HPG", timeframe="1D")).scalar_one()
    assert c.source == "run_imp_malformed"
    assert c.close == 20.5


def test_02_missing_restoration_target_candle_rolls_back(adv_db: Session):
    """Test 2: Missing restoration target candle raises HTTP 400 and transaction rolls back."""
    adv_db.add(Symbol(symbol="VNM", is_active=True))
    dt = datetime(2026, 8, 3, 0, 0, 0)
    run = ImportRun(
        id="run_imp_missing_target",
        file_name="missing.csv",
        file_sha256="dummy",
        content_sha256="dummy",
        status="accepted",
        accepted_at=datetime.now(timezone.utc),
    )
    adv_db.add(run)
    mutation = ImportRunMutation(
        run_id="run_imp_missing_target",
        symbol="VNM",
        timeframe="1D",
        timestamp=dt,
        adjustment_type="unadjusted",
        action="UPDATE:quarantine:seed|2026-08-03T00:00:00",
        before_open=50.0,
        before_high=51.0,
        before_low=49.0,
        before_close=50.5,
        before_volume=500.0,
    )
    adv_db.add(mutation)
    adv_db.commit()

    # Note: Target candle does NOT exist in adv_db
    with pytest.raises(HTTPException) as exc_info:
        ImportWorkflowService.rollback_import(adv_db, "run_imp_missing_target")

    assert exc_info.value.status_code == 400
    assert "Missing rollback target" in exc_info.value.detail or "Không tìm thấy" in exc_info.value.detail

    r = adv_db.execute(select(ImportRun).filter_by(id="run_imp_missing_target")).scalar_one()
    assert r.status == "accepted"


def test_03_ambiguous_restoration_target_candle_on_date_fallback(adv_db: Session):
    """Test 3: Ambiguous restoration target candle on date fallback raises HTTP 400 and zero candles mutated."""
    adv_db.add(Symbol(symbol="TCB", is_active=True))
    target_dt = datetime(2026, 8, 3, 0, 0, 0)

    # Date fallback scenario: target 00:00:00 is missing, but 2 candles exist on that same calendar date
    c1 = Candle(
        symbol="TCB",
        timeframe="1D",
        timestamp=datetime(2026, 8, 3, 9, 30, 0),
        adjustment_type="unadjusted",
        open=20.0,
        high=21.0,
        low=19.0,
        close=20.5,
        volume=100.0,
        source="quarantine:cand1",
    )
    c2 = Candle(
        symbol="TCB",
        timeframe="1D",
        timestamp=datetime(2026, 8, 3, 14, 0, 0),
        adjustment_type="unadjusted",
        open=21.0,
        high=22.0,
        low=20.0,
        close=21.5,
        volume=100.0,
        source="quarantine:cand2",
    )
    adv_db.add_all([c1, c2])

    run = ImportRun(
        id="run_imp_ambig_target",
        file_name="ambig.csv",
        file_sha256="dummy",
        content_sha256="dummy",
        status="accepted",
        accepted_at=datetime.now(timezone.utc),
    )
    adv_db.add(run)
    mutation = ImportRunMutation(
        run_id="run_imp_ambig_target",
        symbol="TCB",
        timeframe="1D",
        timestamp=target_dt,
        adjustment_type="unadjusted",
        action="UPDATE:quarantine:old|2026-08-03T00:00:00",
        before_open=15.0,
        before_high=16.0,
        before_low=14.0,
        before_close=15.5,
        before_volume=1000.0,
    )
    adv_db.add(mutation)
    adv_db.commit()

    with pytest.raises(HTTPException) as exc_info:
        ImportWorkflowService.rollback_import(adv_db, "run_imp_ambig_target")

    assert exc_info.value.status_code == 400
    assert "Ambiguous rollback target" in exc_info.value.detail or "nhiều cây nến trùng ngày" in exc_info.value.detail

    # Run remains accepted
    r = adv_db.execute(select(ImportRun).filter_by(id="run_imp_ambig_target")).scalar_one()
    assert r.status == "accepted"

    # Zero candles mutated
    candles = adv_db.execute(
        select(Candle).execution_options(include_quarantine=True).filter_by(symbol="TCB")
    ).scalars().all()
    assert len(candles) == 2
    assert {c.source for c in candles} == {"quarantine:cand1", "quarantine:cand2"}


def test_04_legacy_update_mutation_compatibility_without_colons_or_pipes(adv_db: Session):
    """Test 4: Legacy UPDATE mutation compatibility without colons/pipes achieves successful restoration."""
    adv_db.add(Symbol(symbol="FPT", is_active=True))
    dt = datetime(2026, 8, 3, 0, 0, 0)
    c = Candle(
        symbol="FPT",
        timeframe="1D",
        timestamp=dt,
        adjustment_type="unadjusted",
        open=95.0,
        high=98.0,
        low=94.0,
        close=97.0,
        volume=20000.0,
        source="run_legacy",
    )
    adv_db.add(c)
    run = ImportRun(
        id="run_legacy",
        file_name="legacy.csv",
        file_sha256="dummy",
        content_sha256="dummy",
        status="accepted",
        accepted_at=datetime.now(timezone.utc),
    )
    adv_db.add(run)

    # Legacy mutation: action="UPDATE" without colons/pipes
    mut1 = ImportRunMutation(
        run_id="run_legacy",
        symbol="FPT",
        timeframe="1D",
        timestamp=dt,
        adjustment_type="unadjusted",
        action="UPDATE",
        before_open=85.0,
        before_high=88.0,
        before_low=84.0,
        before_close=86.0,
        before_volume=10000.0,
    )
    adv_db.add(mut1)
    adv_db.commit()

    resp = ImportWorkflowService.rollback_import(adv_db, "run_legacy")
    assert resp.status == "rolled_back"
    assert resp.restored_mutations_count == 1

    restored = adv_db.execute(select(Candle).filter_by(symbol="FPT", timeframe="1D")).scalar_one()
    assert restored.open == 85.0
    assert restored.close == 86.0
    assert restored.volume == 10000.0

    # Test Sync legacy action="UPDATE:legacy_source" without pipe
    adv_db.add(Symbol(symbol="MSN", is_active=True))
    c2 = Candle(
        symbol="MSN",
        timeframe="1D",
        timestamp=dt,
        adjustment_type="unadjusted",
        open=75.0,
        high=78.0,
        low=74.0,
        close=77.0,
        volume=20000.0,
        source="sync_legacy",
    )
    adv_db.add(c2)
    sync_run = SyncRun(
        id="sync_legacy",
        provider_id="mock",
        symbol="MSN",
        start_date="2026-08-01",
        end_date="2026-08-05",
        content_sha256="dummy",
        status="accepted",
        accepted_at=datetime.now(timezone.utc),
    )
    adv_db.add(sync_run)
    mut2 = SyncRunMutation(
        sync_id="sync_legacy",
        symbol="MSN",
        timeframe="1D",
        timestamp=dt,
        adjustment_type="unadjusted",
        action="UPDATE:legacy_source_name",
        before_open=65.0,
        before_high=68.0,
        before_low=64.0,
        before_close=66.0,
        before_volume=5000.0,
    )
    adv_db.add(mut2)
    adv_db.commit()

    sync_resp = SyncWorkflowService.rollback_sync(adv_db, "sync_legacy")
    assert sync_resp.status == "rolled_back"
    assert sync_resp.restored_mutations_count == 1

    c2_restored = adv_db.execute(select(Candle).filter_by(symbol="MSN", timeframe="1D")).scalar_one()
    assert c2_restored.source == "legacy_source_name"
    assert c2_restored.close == 66.0


def test_05_source_containing_pipes_and_colons_survives_or_safely_rejected(adv_db: Session):
    """Test 5: Source containing '|' or ':' in name is properly parsed or safely rejected without silent corruption."""
    adv_db.add(Symbol(symbol="SSI", is_active=True))
    dt = datetime(2026, 8, 3, 0, 0, 0)
    complex_src = "quarantine:feed:corp|dept:eng|tag:v1:sub|uuid-abc"
    q_candle = Candle(
        symbol="SSI",
        timeframe="1D",
        timestamp=dt,
        adjustment_type="unadjusted",
        open=30.0,
        high=31.0,
        low=29.0,
        close=30.5,
        volume=100.0,
        source=complex_src,
    )
    adv_db.add(q_candle)
    adv_db.commit()

    csv_content = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nSSI,20260803,35.0,36.0,34.0,35.5,5000\n"
    prev = ImportWorkflowService.preview_import(
        db=adv_db,
        file_content=csv_content,
        filename="complex_pipe.csv",
        source_type="cafef",
        adjustment_type="unadjusted",
    )
    ImportWorkflowService.accept_import(adv_db, prev.run_id, prev.content_sha256)

    # Rollback must correctly parse multi-pipe source
    rb = ImportWorkflowService.rollback_import(adv_db, prev.run_id)
    assert rb.status == "rolled_back"

    restored = adv_db.execute(
        select(Candle).execution_options(include_quarantine=True).filter_by(symbol="SSI", timeframe="1D")
    ).scalar_one()
    assert restored.source == complex_src
    assert restored.close == 30.5
    assert restored.timestamp == dt

    # Adversarial test: trailing pipe with empty timestamp fails closed
    run_bad = ImportRun(
        id="run_bad_pipe",
        file_name="bad.csv",
        file_sha256="dummy",
        content_sha256="dummy",
        status="accepted",
        accepted_at=datetime.now(timezone.utc),
    )
    adv_db.add(run_bad)
    mut_bad = ImportRunMutation(
        run_id="run_bad_pipe",
        symbol="SSI",
        timeframe="1D",
        timestamp=dt,
        adjustment_type="unadjusted",
        action="UPDATE:some_source|   ",  # whitespace only timestamp
        before_open=30.0,
        before_high=31.0,
        before_low=29.0,
        before_close=30.5,
        before_volume=100.0,
    )
    adv_db.add(mut_bad)
    adv_db.commit()

    with pytest.raises(HTTPException) as exc_info:
        ImportWorkflowService.rollback_import(adv_db, "run_bad_pipe")
    assert exc_info.value.status_code == 400
    assert "Empty provenance timestamp" in exc_info.value.detail or "bị trống" in exc_info.value.detail


def test_06_fault_injection_during_rollback_rolls_back_entire_db_transaction(adv_db: Session):
    """Test 6: Fault injection during rollback (e.g. exception during weekly derivation) asserts full DB rollback."""
    adv_db.add(Symbol(symbol="MBB", is_active=True))
    dt = datetime(2026, 8, 3, 0, 0, 0)
    q_candle = Candle(
        symbol="MBB",
        timeframe="1D",
        timestamp=dt,
        adjustment_type="unadjusted",
        open=20.0,
        high=21.0,
        low=19.0,
        close=20.5,
        volume=100.0,
        source="quarantine:mbb:1",
    )
    adv_db.add(q_candle)
    adv_db.commit()

    csv_content = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nMBB,20260803,25.0,26.0,24.0,25.5,5000\n"
    prev = ImportWorkflowService.preview_import(
        db=adv_db,
        file_content=csv_content,
        filename="mbb.csv",
        source_type="cafef",
        adjustment_type="unadjusted",
    )
    ImportWorkflowService.accept_import(adv_db, prev.run_id, prev.content_sha256)

    # Injected failure in WeeklyAggregator during rollback
    with patch.object(
        WeeklyAggregator,
        "derive_weekly_candles",
        side_effect=RuntimeError("Simulated catastrophic crash during weekly aggregation"),
    ):
        with pytest.raises(RuntimeError, match="Simulated catastrophic crash"):
            ImportWorkflowService.rollback_import(adv_db, prev.run_id)

    # Verify complete DB transaction rollback
    run_check = adv_db.execute(select(ImportRun).filter_by(id=prev.run_id)).scalar_one()
    assert run_check.status == "accepted"

    c_check = adv_db.execute(select(Candle).filter_by(symbol="MBB", timeframe="1D")).scalar_one()
    assert c_check.source == prev.run_id
    assert c_check.close == 25500.0


def test_07_successful_rollback_after_prior_failed_attempt(adv_db: Session):
    """Test 7: Successful rollback after prior failed attempt (when root issue is corrected)."""
    adv_db.add(Symbol(symbol="VRE", is_active=True))
    dt = datetime(2026, 8, 3, 0, 0, 0)
    q_candle = Candle(
        symbol="VRE",
        timeframe="1D",
        timestamp=dt,
        adjustment_type="unadjusted",
        open=22.0,
        high=23.0,
        low=21.0,
        close=22.5,
        volume=100.0,
        source="quarantine:vre:1",
    )
    adv_db.add(q_candle)
    adv_db.commit()

    csv_content = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nVRE,20260803,30.0,31.0,29.0,30.5,5000\n"
    prev = ImportWorkflowService.preview_import(
        db=adv_db,
        file_content=csv_content,
        filename="vre.csv",
        source_type="cafef",
        adjustment_type="unadjusted",
    )
    ImportWorkflowService.accept_import(adv_db, prev.run_id, prev.content_sha256)

    # Attempt 1 fails due to transient fault
    with patch.object(
        WeeklyAggregator,
        "derive_weekly_candles",
        side_effect=RuntimeError("Transient lock error"),
    ):
        with pytest.raises(RuntimeError):
            ImportWorkflowService.rollback_import(adv_db, prev.run_id)

    # Status remains accepted
    r1 = adv_db.execute(select(ImportRun).filter_by(id=prev.run_id)).scalar_one()
    assert r1.status == "accepted"

    # Attempt 2 succeeds cleanly once issue is cleared
    res = ImportWorkflowService.rollback_import(adv_db, prev.run_id)
    assert res.status == "rolled_back"
    assert res.restored_mutations_count == 1

    r2 = adv_db.execute(select(ImportRun).filter_by(id=prev.run_id)).scalar_one()
    assert r2.status == "rolled_back"

    restored = adv_db.execute(
        select(Candle).execution_options(include_quarantine=True).filter_by(symbol="VRE", timeframe="1D")
    ).scalar_one()
    assert restored.source == "quarantine:vre:1"
    assert restored.close == 22.5


# ==============================================================================
# FINDING R2: MULTIPLE QUARANTINE CANDIDATES TESTS (8 - 14)
# ==============================================================================

def test_08_two_quarantine_candles_same_date_in_import_fails_closed(adv_db: Session):
    """Test 8: Two quarantine candles on one date with different timestamps in Import asserts HTTP 400 and DB unchanged."""
    adv_db.add(Symbol(symbol="AAA", is_active=True))
    q1 = Candle(
        symbol="AAA",
        timeframe="1D",
        timestamp=datetime(2026, 8, 3, 0, 0, 0),
        adjustment_type="unadjusted",
        open=10.0,
        high=11.0,
        low=9.0,
        close=10.5,
        volume=100.0,
        source="quarantine:feed:a1",
    )
    q2 = Candle(
        symbol="AAA",
        timeframe="1D",
        timestamp=datetime(2026, 8, 3, 9, 30, 0),
        adjustment_type="unadjusted",
        open=11.0,
        high=12.0,
        low=10.0,
        close=11.5,
        volume=200.0,
        source="quarantine:feed:a2",
    )
    adv_db.add_all([q1, q2])
    adv_db.commit()

    count_before, hash_before = _snapshot_candles(adv_db)
    assert count_before == 2

    csv_content = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nAAA,20260803,15.0,16.0,14.0,15.5,5000\n"
    prev = ImportWorkflowService.preview_import(
        db=adv_db,
        file_content=csv_content,
        filename="aaa.csv",
        source_type="cafef",
        adjustment_type="unadjusted",
    )
    assert prev.can_accept is True

    with pytest.raises(HTTPException) as exc_info:
        ImportWorkflowService.accept_import(adv_db, prev.run_id, prev.content_sha256)

    assert exc_info.value.status_code == 400
    assert "Ambiguous quarantine candidates detected" in exc_info.value.detail or "nến cách ly" in exc_info.value.detail

    # DB unchanged
    count_after, hash_after = _snapshot_candles(adv_db)
    assert count_after == count_before
    assert hash_after == hash_before


def test_09_two_quarantine_candles_same_date_in_sync_fails_closed(adv_db: Session):
    """Test 9: Two quarantine candles on one date with different timestamps in Sync asserts HTTP 400 and DB unchanged."""
    adv_db.add(Symbol(symbol="BBB", is_active=True))
    q1 = Candle(
        symbol="BBB",
        timeframe="1D",
        timestamp=datetime(2026, 8, 3, 0, 0, 0),
        adjustment_type="unadjusted",
        open=20.0,
        high=21.0,
        low=19.0,
        close=20.5,
        volume=100.0,
        source="quarantine:sync:b1",
    )
    q2 = Candle(
        symbol="BBB",
        timeframe="1D",
        timestamp=datetime(2026, 8, 3, 14, 45, 0),
        adjustment_type="unadjusted",
        open=21.0,
        high=22.0,
        low=20.0,
        close=21.5,
        volume=200.0,
        source="quarantine:sync:b2",
    )
    adv_db.add_all([q1, q2])
    adv_db.commit()

    count_before, hash_before = _snapshot_candles(adv_db)
    assert count_before == 2

    sync_provider = W0AMockProvider(
        metadata=ProviderMetadata(
            provider_id="real_b_prov",
            display_name="Real Provider B",
            is_official=True,
            requires_auth=False,
            is_synthetic=False,
            description="Real provider",
        ),
        candles=[
            ProviderCandleDTO(
                symbol="BBB",
                timeframe="1D",
                timestamp=date(2026, 8, 3),
                open=25.0,
                high=26.0,
                low=24.0,
                close=25.5,
                volume=5000.0,
                adjustment_type="unadjusted",
            )
        ],
    )
    ProviderRegistry.reset_instance()
    ProviderRegistry.get_instance().register_provider(sync_provider)

    from app.schemas.sync_schema import SyncPreviewRequest
    prev = SyncWorkflowService.generate_sync_preview(
        db=adv_db,
        request=SyncPreviewRequest(
            symbol="BBB",
            start_date="2026-08-03",
            end_date="2026-08-03",
            provider_id="real_b_prov",
            adjustment_type="unadjusted",
        ),
    )
    assert prev.can_accept is True

    with pytest.raises(HTTPException) as exc_info:
        SyncWorkflowService.execute_sync(adv_db, prev.sync_id, prev.content_sha256)

    assert exc_info.value.status_code == 400
    assert "Ambiguous quarantine candidates detected" in exc_info.value.detail or "nến cách ly" in exc_info.value.detail

    # DB unchanged
    count_after, hash_after = _snapshot_candles(adv_db)
    assert count_after == count_before
    assert hash_after == hash_before


def test_10_same_day_midnight_non_midnight_collision_fails_closed(adv_db: Session):
    """Test 10: Same-day midnight/non-midnight collision asserts HTTP 400 and DB unchanged without UNIQUE error."""
    adv_db.add(Symbol(symbol="COL", is_active=True))
    q_midnight = Candle(
        symbol="COL",
        timeframe="1D",
        timestamp=datetime(2026, 8, 3, 0, 0, 0),
        adjustment_type="unadjusted",
        open=50.0,
        high=52.0,
        low=49.0,
        close=51.0,
        volume=100.0,
        source="quarantine:mid",
    )
    q_non_midnight = Candle(
        symbol="COL",
        timeframe="1D",
        timestamp=datetime(2026, 8, 3, 11, 23, 45),
        adjustment_type="unadjusted",
        open=50.5,
        high=52.5,
        low=49.5,
        close=51.5,
        volume=200.0,
        source="quarantine:non_mid",
    )
    adv_db.add_all([q_midnight, q_non_midnight])
    adv_db.commit()

    count_before, hash_before = _snapshot_candles(adv_db)

    csv_content = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nCOL,20260803,60.0,62.0,59.0,61.0,1000\n"
    prev = ImportWorkflowService.preview_import(
        db=adv_db,
        file_content=csv_content,
        filename="collision.csv",
        source_type="cafef",
        adjustment_type="unadjusted",
    )
    with pytest.raises(HTTPException) as exc_info:
        ImportWorkflowService.accept_import(adv_db, prev.run_id, prev.content_sha256)

    assert exc_info.value.status_code == 400
    assert "Ambiguous quarantine candidates detected: 2" in exc_info.value.detail

    count_after, hash_after = _snapshot_candles(adv_db)
    assert count_after == count_before
    assert hash_after == hash_before


def test_11_one_unambiguous_quarantine_candle_promotes_and_rolls_back(adv_db: Session):
    """Test 11: One valid unambiguous quarantine candle asserts successful Option A promotion and exact rollback."""
    adv_db.add(Symbol(symbol="CCC", is_active=True))
    dt = datetime(2026, 8, 3, 0, 0, 0)
    q_candle = Candle(
        symbol="CCC",
        timeframe="1D",
        timestamp=dt,
        adjustment_type="unadjusted",
        open=40.0,
        high=41.0,
        low=39.0,
        close=40.5,
        volume=300.0,
        source="quarantine:unique_seed",
    )
    adv_db.add(q_candle)
    adv_db.commit()
    orig_candle_id = q_candle.id

    csv_content = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nCCC,20260803,50.0,51.0,49.0,50.5,10000\n"
    prev = ImportWorkflowService.preview_import(
        db=adv_db,
        file_content=csv_content,
        filename="ccc.csv",
        source_type="cafef",
        adjustment_type="unadjusted",
    )
    ImportWorkflowService.accept_import(adv_db, prev.run_id, prev.content_sha256)

    # Option A promotion: same ID, updated values, source=run_id
    promoted = adv_db.execute(select(Candle).filter_by(symbol="CCC", timeframe="1D")).scalar_one()
    assert promoted.id == orig_candle_id
    assert promoted.source == prev.run_id
    assert promoted.close == 50500.0

    # Mutation logged
    mut = adv_db.execute(select(ImportRunMutation).filter_by(run_id=prev.run_id)).scalar_one()
    assert mut.action == f"UPDATE:quarantine:unique_seed|{dt.isoformat()}"

    # Rollback
    rb_res = ImportWorkflowService.rollback_import(adv_db, prev.run_id)
    assert rb_res.status == "rolled_back"

    restored = adv_db.execute(
        select(Candle).execution_options(include_quarantine=True).filter_by(symbol="CCC", timeframe="1D")
    ).scalar_one()
    assert restored.id == orig_candle_id
    assert restored.source == "quarantine:unique_seed"
    assert restored.close == 40.5
    assert restored.timestamp == dt


def test_12_canonical_already_present_takes_precedence_over_quarantine(adv_db: Session):
    """Test 12: Canonical already present when quarantine exists asserts canonical precedence."""
    adv_db.add(Symbol(symbol="GAS", is_active=True))
    dt = datetime(2026, 8, 3, 0, 0, 0)
    c_canon = Candle(
        symbol="GAS",
        timeframe="1D",
        timestamp=dt,
        adjustment_type="unadjusted",
        open=80000.0,
        high=82000.0,
        low=79000.0,
        close=81000.0,
        volume=50000.0,
        source="canonical_previous_run",
    )
    c_quar = Candle(
        symbol="GAS",
        timeframe="1D",
        timestamp=datetime(2026, 8, 3, 10, 0, 0),
        adjustment_type="unadjusted",
        open=10.0,
        high=11.0,
        low=9.0,
        close=10.5,
        volume=100.0,
        source="quarantine:orphan",
    )
    adv_db.add_all([c_canon, c_quar])
    adv_db.commit()

    # Incoming CafeF CSV with conflicting price for GAS on 2026-08-03
    csv_content = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nGAS,20260803,90.0,92.0,89.0,91.0,10000\n"
    prev = ImportWorkflowService.preview_import(
        db=adv_db,
        file_content=csv_content,
        filename="gas_conflict.csv",
        source_type="cafef",
        adjustment_type="unadjusted",
    )
    # Canonical row takes precedence: conflicting preview prevents accept
    assert prev.conflicting_count == 1
    assert prev.can_accept is False

    # Canonical candle remains unaltered
    c_check = adv_db.execute(select(Candle).filter_by(symbol="GAS", timeframe="1D")).scalar_one()
    assert c_check.source == "canonical_previous_run"
    assert c_check.close == 81000.0


def test_13_rollback_after_promotion_restores_quarantine_values_and_timestamp(adv_db: Session):
    """Test 13: Rollback after successful promotion asserts quarantine restored with original values and timestamps."""
    adv_db.add(Symbol(symbol="KDH", is_active=True))
    orig_ts = datetime(2026, 8, 3, 9, 45, 12)
    q_candle = Candle(
        symbol="KDH",
        timeframe="1D",
        timestamp=orig_ts,
        adjustment_type="unadjusted",
        open=31.0,
        high=32.0,
        low=30.0,
        close=31.5,
        volume=777.0,
        source="quarantine:kdh:non_midnight",
    )
    adv_db.add(q_candle)
    adv_db.commit()

    csv_content = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nKDH,20260803,35.0,36.0,34.0,35.5,8888\n"
    prev = ImportWorkflowService.preview_import(
        db=adv_db,
        file_content=csv_content,
        filename="kdh.csv",
        source_type="cafef",
        adjustment_type="unadjusted",
    )
    ImportWorkflowService.accept_import(adv_db, prev.run_id, prev.content_sha256)

    # In-place promotion normalized timestamp to midnight
    promoted = adv_db.execute(select(Candle).filter_by(symbol="KDH", timeframe="1D")).scalar_one()
    assert promoted.timestamp == datetime(2026, 8, 3, 0, 0, 0)
    assert promoted.close == 35500.0

    # Rollback restores exact original timestamp and values
    rb_res = ImportWorkflowService.rollback_import(adv_db, prev.run_id)
    assert rb_res.status == "rolled_back"

    restored = adv_db.execute(
        select(Candle).execution_options(include_quarantine=True).filter_by(symbol="KDH", timeframe="1D")
    ).scalar_one()
    assert restored.source == "quarantine:kdh:non_midnight"
    assert restored.timestamp == orig_ts
    assert restored.open == 31.0
    assert restored.high == 32.0
    assert restored.low == 30.0
    assert restored.close == 31.5
    assert restored.volume == 777.0


def test_14_database_state_unchanged_after_rejected_ambiguous_promotion(adv_db: Session):
    """Test 14: Database state unchanged after rejected ambiguous promotion (verify candle count and hashes)."""
    adv_db.add(Symbol(symbol="STB", is_active=True))
    q1 = Candle(
        symbol="STB",
        timeframe="1D",
        timestamp=datetime(2026, 8, 3, 8, 0, 0),
        adjustment_type="unadjusted",
        open=28.0,
        high=29.0,
        low=27.0,
        close=28.5,
        volume=100.0,
        source="quarantine:stb:1",
    )
    q2 = Candle(
        symbol="STB",
        timeframe="1D",
        timestamp=datetime(2026, 8, 3, 13, 0, 0),
        adjustment_type="unadjusted",
        open=28.5,
        high=29.5,
        low=27.5,
        close=29.0,
        volume=200.0,
        source="quarantine:stb:2",
    )
    adv_db.add_all([q1, q2])
    adv_db.commit()

    count_before, hash_before = _snapshot_candles(adv_db)
    assert count_before == 2

    csv_content = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nSTB,20260803,32.0,33.0,31.0,32.5,9999\n"
    prev = ImportWorkflowService.preview_import(
        db=adv_db,
        file_content=csv_content,
        filename="stb_ambig.csv",
        source_type="cafef",
        adjustment_type="unadjusted",
    )
    with pytest.raises(HTTPException) as exc_info:
        ImportWorkflowService.accept_import(adv_db, prev.run_id, prev.content_sha256)
    assert exc_info.value.status_code == 400

    count_after, hash_after = _snapshot_candles(adv_db)
    assert count_after == count_before
    assert hash_after == hash_before


# ==============================================================================
# FINDING R3: PRODUCTION DB SAFEGUARD BASELINE TESTS (15 - 17)
# ==============================================================================

def test_15_dynamic_baseline_capture_and_zero_delta_verification():
    """Test 15: Verify dynamic baseline capture and zero-delta verification when database is untouched."""
    exists, sha256_val, size_val = capture_production_database_baseline()
    assert exists is True
    assert isinstance(sha256_val, str)
    assert len(sha256_val) == 64
    assert size_val > 0

    # Zero-delta verification against run-local baseline returns True
    is_valid, curr_sha, curr_size = verify_production_database_integrity()
    assert is_valid is True
    assert curr_sha == sha256_val
    assert curr_size == size_val

    # Tampered baseline returns False
    tampered_baseline = (True, "0000000000000000000000000000000000000000000000000000000000000000", size_val)
    is_valid_tampered, _, _ = verify_production_database_integrity(baseline=tampered_baseline)
    assert is_valid_tampered is False


def test_16_os_open_write_flags_abort_for_prod_and_aux_files():
    """Test 16: Verify os.open write flags (O_WRONLY, O_RDWR, O_CREAT, O_TRUNC) to production DB or aux files abort with RuntimeError."""
    install_database_safeguard()

    # Mutating flags on production db file
    write_flags_list = [
        os.O_WRONLY,
        os.O_RDWR,
        os.O_CREAT,
        os.O_TRUNC,
        os.O_WRONLY | os.O_CREAT,
    ]
    for flag in write_flags_list:
        with pytest.raises(RuntimeError, match=r"FAIL-CLOSED SAFEGUARD"):
            os.open(str(PROD_DB_PATH), flag)

    # Mutating flags on auxiliary files
    for suffix in ["-wal", "-shm", "-journal"]:
        aux_path = str(PROD_DB_PATH) + suffix
        for flag in write_flags_list:
            with pytest.raises(RuntimeError, match=r"FAIL-CLOSED SAFEGUARD"):
                os.open(aux_path, flag)


def test_17_legitimate_temp_database_access_permitted(tmp_path: Path):
    """Test 17: Verify legitimate temp database access is permitted without restriction."""
    install_database_safeguard()
    temp_db_path = tmp_path / "legit_test.db"

    # os.open on temporary file with write flags succeeds
    fd = os.open(str(temp_db_path), os.O_RDWR | os.O_CREAT)
    assert fd >= 0
    os.close(fd)

    # sqlite3.connect on temporary database succeeds
    conn = sqlite3.connect(str(temp_db_path))
    cursor = conn.cursor()
    cursor.execute("CREATE TABLE test_table (id INTEGER PRIMARY KEY, val TEXT);")
    cursor.execute("INSERT INTO test_table (val) VALUES ('hello_safeguard');")
    conn.commit()

    cursor.execute("SELECT val FROM test_table WHERE id = 1;")
    row = cursor.fetchone()
    assert row is not None
    assert row[0] == "hello_safeguard"
    conn.close()


def test_18_repeated_baseline_capture_cannot_overwrite_session_or_hide_mismatch():
    """Test 18: Repeated calls to capture_production_database_baseline cannot overwrite session baseline or hide mismatch."""
    assert is_session_baseline_locked() is True
    b_initial = capture_production_database_baseline()
    assert b_initial[0] is True
    assert isinstance(b_initial[1], str) and len(b_initial[1]) == 64

    # Repeated capture call returns identical locked baseline
    b_repeat = capture_production_database_baseline()
    assert b_repeat == b_initial

    # Simulate tampered database hash on disk during test execution
    tampered_hash = "abcdef0123456789" * 4
    tampered_size = b_initial[2] + 8192
    with patch("tests_safety.compute_production_database_hash", return_value=(tampered_hash, tampered_size)):
        # Calling capture_production_database_baseline does NOT overwrite session baseline
        b_mocked = capture_production_database_baseline()
        assert b_mocked == b_initial
        assert b_mocked[1] != tampered_hash

        # verify_production_database_integrity detects simulated mismatch against locked baseline
        is_valid, curr_sha, curr_size = verify_production_database_integrity()
        assert is_valid is False
        assert curr_sha == tampered_hash
        assert curr_size == tampered_size

    # Verify session baseline is still unmutated
    assert get_session_database_baseline() == b_initial

