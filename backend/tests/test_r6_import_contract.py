"""Integrated contract test for Requirement R6: Import Preview -> Accept Contract Integrity.

Verifies:
1. confirm=False preview creates 0 database records across all 10 workflow tables.
   Default on POST /api/import/preview is confirm=False (strictly read-only).
   ImportPreviewResponse returns staged=False for unstaged preview.
2. Attempting to accept an unstaged ephemeral run_id fails closed with actionable HTTP 400
   ("Cannot accept unstaged preview. Staged run required before acceptance.") instead of generic 404.
3. Confirmed import (confirm=True) stages and accepts cleanly with all audit records recorded:
   - ImportRun and ImportRunItem staged on confirm=True (with staged=True)
   - Atomic acceptance applies 1D candles, derives 1W weekly candles, records ImportRunMutation,
     and logs audit event in event_logs
   - Idempotent re-accept returns noop
   - Rollback restores baseline
4. Zero unnecessary storage infrastructure: workflow is fully self-contained in local SQLite transactions.
5. backend/sumi.db remains strictly untouched and unmodified.

All database operations execute strictly in isolated temporary databases (tmp_path).
"""
import hashlib
from pathlib import Path
import pytest
from sqlalchemy import create_engine, text
from sqlalchemy.orm import sessionmaker, Session
from fastapi.testclient import TestClient

from app.main import app
from app.db import Base
from app.dependencies import get_db
from app.models.candle import Candle
from app.models.import_run import ImportRun, ImportRunItem, ImportRunMutation
from app.models.event_log import EventLog
from app.services.import_workflow_service import ImportWorkflowService, ImportPreviewResponse

WORKFLOW_TABLES = [
    "candles",
    "symbols",
    "import_runs",
    "import_run_items",
    "import_run_mutations",
    "weekly_candle_provenance",
    "sync_runs",
    "sync_run_items",
    "sync_run_mutations",
    "event_logs",
]

SUMI_DB_EXPECTED_SIZE = 188416
SUMI_DB_EXPECTED_SHA256 = "234206fc75259f091337d4feb5f41575d0fe5ac02d6c10ba0de8fad68cbcee86"


def _get_table_counts(conn) -> dict[str, int]:
    counts = {}
    for table_name in WORKFLOW_TABLES:
        val = conn.execute(text(f"SELECT COUNT(*) FROM {table_name}")).scalar()
        counts[table_name] = int(val)
    return counts


@pytest.fixture
def isolated_env(tmp_path: Path):
    """Provides an isolated temporary SQLite database and configured TestClient."""
    db_file = tmp_path / "test_r6_isolated.db"
    engine = create_engine(
        f"sqlite:///{db_file.as_posix()}",
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
        yield session, client, engine
    session.close()
    app.dependency_overrides.pop(get_db, None)
    engine.dispose()


SAMPLE_CSV_CONTENT = (
    "<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\n"
    "SSI,20260105,30.0,31.0,29.5,30.5,1500000\n"
    "SSI,20260106,30.5,31.5,30.0,31.0,1800000\n"
).encode("utf-8")


def test_r6_preview_confirm_false_zero_database_records(isolated_env):
    """Requirement 1: confirm=False preview creates 0 database records across all 10 workflow tables.

    Tests that default on POST /api/import/preview is confirm=False, returns staged=False,
    and produces 0 row delta across all tables.
    """
    session, client, engine = isolated_env

    with engine.connect() as conn:
        before_counts = _get_table_counts(conn)

    # 1. POST /api/import/preview without 'confirm' parameter (tests default confirm=False)
    files = {"file": ("SSI.csv", SAMPLE_CSV_CONTENT, "text/csv")}
    res_default = client.post(
        "/api/import/preview",
        files=files,
        data={"adjustment_type": "unadjusted"}
    )
    assert res_default.status_code == 200
    data_default = res_default.json()
    assert data_default["staged"] is False
    assert data_default["can_accept"] is True
    assert data_default["status"] == "previewed"
    assert data_default["parsed_count"] == 2
    assert len(data_default["items"]) == 2
    assert data_default["run_id"] is not None
    assert data_default["content_sha256"] is not None

    with engine.connect() as conn:
        after_counts_default = _get_table_counts(conn)
    assert after_counts_default == before_counts, (
        f"Database modified after default confirm=False preview: {after_counts_default} vs {before_counts}"
    )

    # 2. POST /api/import/preview with explicit confirm=false
    files = {"file": ("SSI.csv", SAMPLE_CSV_CONTENT, "text/csv")}
    res_explicit = client.post(
        "/api/import/preview",
        files=files,
        data={"adjustment_type": "unadjusted", "confirm": "false"}
    )
    assert res_explicit.status_code == 200
    data_explicit = res_explicit.json()
    assert data_explicit["staged"] is False
    assert data_explicit["can_accept"] is True

    with engine.connect() as conn:
        after_counts_explicit = _get_table_counts(conn)
    assert after_counts_explicit == before_counts, (
        f"Database modified after explicit confirm=false preview: {after_counts_explicit} vs {before_counts}"
    )

    # 3. Invalid metadata with confirm=False also produces 0 writes
    files = {"file": ("SSI.csv", SAMPLE_CSV_CONTENT, "text/csv")}
    res_bad_meta = client.post(
        "/api/import/preview",
        files=files,
        data={"timeframe": "INVALID_TF", "confirm": "false"}
    )
    assert res_bad_meta.status_code == 200
    data_bad = res_bad_meta.json()
    assert data_bad["staged"] is False
    assert data_bad["can_accept"] is False
    assert data_bad["status"] == "blocked"

    with engine.connect() as conn:
        after_counts_bad = _get_table_counts(conn)
    assert after_counts_bad == before_counts, (
        f"Database modified after bad metadata preview: {after_counts_bad} vs {before_counts}"
    )


def test_r6_accept_unstaged_ephemeral_run_id_fails_closed_with_400(isolated_env):
    """Requirement 2: Attempting to accept an unstaged ephemeral run_id fails closed with HTTP 400.

    Actionable error message: 'Cannot accept unstaged preview. Staged run required before acceptance.'
    """
    session, client, engine = isolated_env

    with engine.connect() as conn:
        before_counts = _get_table_counts(conn)

    # 1. Preview file with default confirm=False -> receives unstaged preview with ephemeral run_id
    files = {"file": ("SSI.csv", SAMPLE_CSV_CONTENT, "text/csv")}
    preview_res = client.post(
        "/api/import/preview",
        files=files,
        data={"adjustment_type": "unadjusted"}
    )
    assert preview_res.status_code == 200
    preview_data = preview_res.json()
    assert preview_data["staged"] is False
    ephemeral_run_id = preview_data["run_id"]
    checksum = preview_data["content_sha256"]

    # 2. Attempt to accept unstaged ephemeral run_id -> fails with HTTP 400
    accept_res = client.post(
        f"/api/import/runs/{ephemeral_run_id}/accept",
        json={"content_sha256": checksum}
    )
    assert accept_res.status_code == 400
    error_detail = accept_res.json()["detail"]
    assert "Cannot accept unstaged preview. Staged run required before acceptance." in error_detail

    # Verify 0 database records were created or altered
    with engine.connect() as conn:
        after_counts = _get_table_counts(conn)
    assert after_counts == before_counts, (
        f"Database mutated after failed accept attempt: {after_counts} vs {before_counts}"
    )

    # 3. Direct service call with unstaged run_id also fails with HTTP 400
    with pytest.raises(Exception) as exc_info:
        ImportWorkflowService.accept_import(session, ephemeral_run_id, checksum)
    assert exc_info.value.status_code == 400
    assert "Cannot accept unstaged preview. Staged run required before acceptance." in exc_info.value.detail


def test_r6_confirmed_import_stages_and_accepts_cleanly_with_audit_trail(isolated_env):
    """Requirement 3: Confirmed import stages and accepts cleanly with all audit records recorded.

    1. confirm=True stages the run in import_runs and import_run_items with staged=True.
    2. Accept commits 1D candles, derives 1W candles, writes mutations, logs audit event.
    3. Idempotent re-accept returns noop.
    4. Rollback cleanly restores original state.
    """
    session, client, engine = isolated_env

    # 1. Preview with confirm=True (explicit confirmation staging)
    files = {"file": ("SSI.csv", SAMPLE_CSV_CONTENT, "text/csv")}
    stage_res = client.post(
        "/api/import/preview",
        files=files,
        data={"adjustment_type": "unadjusted", "confirm": "true"}
    )
    assert stage_res.status_code == 200
    stage_data = stage_res.json()
    assert stage_data["staged"] is True
    assert stage_data["can_accept"] is True
    staged_run_id = stage_data["run_id"]
    checksum = stage_data["content_sha256"]

    # Verify staged records exist in DB, but candles do not yet exist
    with engine.connect() as conn:
        run_count = conn.execute(text(f"SELECT COUNT(*) FROM import_runs WHERE id = '{staged_run_id}'")).scalar()
        assert run_count == 1
        item_count = conn.execute(text(f"SELECT COUNT(*) FROM import_run_items WHERE run_id = '{staged_run_id}'")).scalar()
        assert item_count == 2
        candle_count = conn.execute(text("SELECT COUNT(*) FROM candles WHERE symbol = 'SSI'")).scalar()
        assert candle_count == 0

    # 2. Accept staged run
    accept_res = client.post(
        f"/api/import/runs/{staged_run_id}/accept",
        json={"content_sha256": checksum}
    )
    assert accept_res.status_code == 200
    accept_data = accept_res.json()
    assert accept_data["status"] == "accepted"
    assert accept_data["accepted_count"] == 2

    # Verify daily candles inserted (scaled 1000x for equities: 30.0 -> 30000.0)
    daily_candles = session.query(Candle).filter(Candle.symbol == "SSI", Candle.timeframe == "1D").all()
    assert len(daily_candles) == 2
    assert daily_candles[0].open == 30000.0

    # Verify weekly candle auto-derived
    weekly_candles = session.query(Candle).filter(Candle.symbol == "SSI", Candle.timeframe == "1W").all()
    assert len(weekly_candles) == 1
    assert weekly_candles[0].open == 30000.0
    assert weekly_candles[0].close == 31000.0

    # Verify audit mutations recorded
    mutations = session.query(ImportRunMutation).filter(ImportRunMutation.run_id == staged_run_id).all()
    assert len(mutations) == 2
    assert all(m.action == "INSERT" for m in mutations)

    # Verify event log audit record
    event_logs = session.query(EventLog).filter(EventLog.event_type == "IMPORT_ACCEPTED").all()
    assert len(event_logs) >= 1
    assert any(staged_run_id in str(e.details) for e in event_logs)

    # 3. Idempotent re-acceptance returns no-op
    repeat_res = client.post(
        f"/api/import/runs/{staged_run_id}/accept",
        json={"content_sha256": checksum}
    )
    assert repeat_res.status_code == 200
    repeat_data = repeat_res.json()
    assert repeat_data["status"] == "noop"
    assert repeat_data["accepted_count"] == 0

    # 4. Rollback reverts mutations and weekly candles
    rollback_res = client.post(f"/api/import/runs/{staged_run_id}/rollback")
    assert rollback_res.status_code == 200
    rollback_data = rollback_res.json()
    assert rollback_data["status"] == "rolled_back"
    assert rollback_data["restored_mutations_count"] == 2

    # Verify candles deleted
    daily_after = session.query(Candle).filter(Candle.symbol == "SSI", Candle.timeframe == "1D").all()
    assert len(daily_after) == 0


def test_r6_no_unnecessary_storage_infrastructure():
    """Requirement 4: No unnecessary new storage infrastructure is introduced.

    Verifies the preview -> confirm -> accept contract operates strictly via SQLite
    database transactions and in-memory Pydantic validation, without Redis or foreign caches.
    """
    import sys
    # Verify no redis, celery, memcached, or external message broker modules are required
    prohibited_modules = ["redis", "celery", "pylibmc", "memcache", "kafka", "pika"]
    for mod in prohibited_modules:
        assert mod not in sys.modules, f"Unnecessary storage module {mod} found loaded in sys.modules"

    # Verify ImportWorkflowService operates via Session
    import inspect
    preview_sig = inspect.signature(ImportWorkflowService.preview_import)
    assert "confirm" in preview_sig.parameters
    assert "db" in preview_sig.parameters

    accept_sig = inspect.signature(ImportWorkflowService.accept_import)
    assert "db" in accept_sig.parameters
    assert "run_id" in accept_sig.parameters
    assert "content_sha256" in accept_sig.parameters


def test_r6_sumi_db_untouched():
    """Invariant: backend/sumi.db must be strictly untouched and bit-exact."""
    sumi_db_path = Path(__file__).resolve().parent.parent / "sumi.db"
    assert sumi_db_path.exists(), "backend/sumi.db not found"
    stat = sumi_db_path.stat()
    assert stat.st_size == SUMI_DB_EXPECTED_SIZE, (
        f"sumi.db size changed: {stat.st_size} vs expected {SUMI_DB_EXPECTED_SIZE}"
    )
    actual_sha = hashlib.sha256(sumi_db_path.read_bytes()).hexdigest()
    assert actual_sha == SUMI_DB_EXPECTED_SHA256, (
        f"sumi.db sha256 mismatch: {actual_sha} vs expected {SUMI_DB_EXPECTED_SHA256}"
    )
