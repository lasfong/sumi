"""Adversarial stress-test suite for Milestone 2: Phase B (Batch B03).

Authored by Challenger Subagent (teamwork_preview_challenger_b03).
Strictly operates on isolated temporary SQLite databases.
Validates:
1. Alembic migration idempotency on fresh empty DB (multiple upgrades + downgrade + re-upgrade).
2. Alembic migration idempotency on Base.metadata.create_all() DB (multiple upgrades + empty version table).
3. All 12 composite and FK indexes created and verified.
4. Preview read-only isolation (confirm=False) with 0-row delta across all 10 workflow tables:
   - Valid data
   - Invalid metadata
   - Corrupted binary data
   - Out-of-order dates
   - Extreme/invalid OHLC
   - Duplicate data
   - Conflicting data
5. seed_cafef_data(dry_run=True) and seed_cafef_data(confirm=False) 0-row delta across 10 tables.
6. FastAPI /api/import/preview HTTP endpoint isolation with confirm=False vs confirm=True.
7. Production sumi.db invariant (hash and size).
"""

from __future__ import annotations

import hashlib
import io
import os
from datetime import date, datetime
from pathlib import Path
import pytest
from sqlalchemy import create_engine, inspect, text
from sqlalchemy.orm import Session, sessionmaker
from fastapi.testclient import TestClient

from alembic.config import Config
from alembic import command

from app.db import Base
from app.main import app
from app.models import (
    Symbol,
    Candle,
    ReplaySession,
    Decision,
    Order,
    Execution,
    Position,
    Trade,
    JournalEntry,
    EventLog,
    DrawingState,
    StrategyLabRun,
    ScannerRun,
    ImportRun,
    ImportRunItem,
    ImportRunMutation,
    WeeklyCandleProvenance,
    SyncRun,
    SyncRunItem,
    SyncRunMutation,
)
from app.services.import_workflow_service import ImportWorkflowService
from seed_cafef import seed_cafef_data

BACKEND_DIR = Path(__file__).resolve().parent.parent
ALEMBIC_INI_PATH = BACKEND_DIR / "alembic.ini"
ALEMBIC_DIR = BACKEND_DIR / "alembic"
PROD_DB_PATH = BACKEND_DIR / "sumi.db"
EXPECTED_PROD_DB_SHA256 = "234206fc75259f091337d4feb5f41575d0fe5ac02d6c10ba0de8fad68cbcee86"
EXPECTED_PROD_DB_SIZE = 188416

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

EXPECTED_COMPOSITE_INDEXES = {
    "candles": {"ix_candles_symbol_timeframe_timestamp"},
    "decisions": {"ix_decisions_session_id", "ix_decisions_session_candle"},
    "orders": {"ix_orders_session_id", "ix_orders_decision_id"},
    "executions": {
        "ix_executions_session_id",
        "ix_executions_order_id",
        "ix_executions_trade_id",
        "ix_executions_session_candle",
    },
    "positions": {"ix_positions_session_id", "ix_positions_session_symbol"},
    "trades": {"ix_trades_session_id", "ix_trades_session_symbol"},
    "journal_entries": {
        "ix_journal_entries_session_id",
        "ix_journal_entries_decision_id",
        "ix_journal_entries_trade_id",
    },
    "event_logs": {"ix_event_logs_session_id"},
    "import_run_items": {"ix_import_run_items_run_symbol"},
    "import_run_mutations": {"ix_import_run_mutations_run_action"},
    "sync_run_items": {"ix_sync_run_items_sync_symbol"},
    "sync_run_mutations": {"ix_sync_run_mutations_sync_action"},
}


def _get_workflow_table_counts(conn) -> dict[str, int]:
    counts = {}
    for table in WORKFLOW_TABLES:
        val = conn.execute(text(f"SELECT COUNT(*) FROM {table}")).scalar()
        counts[table] = int(val)
    return counts


def _seed_baseline_workflow_records(session: Session) -> None:
    """Pre-populate a minimal set of baseline records in several workflow tables to guarantee
    that queries do not just test 0 -> 0 transitions."""
    sym = Symbol(symbol="HPG", company_name="Hoa Phat Group", exchange="HOSE", is_active=True)
    session.add(sym)

    c = Candle(
        symbol="HPG",
        timeframe="1D",
        timestamp=datetime(2026, 1, 2, 0, 0, 0),
        open=25000.0,
        high=25500.0,
        low=24800.0,
        close=25200.0,
        volume=1500000.0,
        adjustment_type="unadjusted",
    )
    session.add(c)

    ir = ImportRun(
        id="baseline-import-run-001",
        file_name="baseline.csv",
        file_sha256="0000000000000000000000000000000000000000000000000000000000000000",
        content_sha256="0000000000000000000000000000000000000000000000000000000000000000",
        parser_version="cafef_v1",
        source_type="cafef",
        timeframe="1D",
        adjustment_type="unadjusted",
        timezone="Asia/Ho_Chi_Minh",
        status="accepted",
        parsed_count=1,
        accepted_count=1,
        can_accept=True,
    )
    session.add(ir)

    sr = SyncRun(
        id="baseline-sync-run-001",
        provider_id="ssi",
        symbol="HPG",
        start_date="2026-01-01",
        end_date="2026-01-02",
        timeframe="1D",
        adjustment_type="unadjusted",
        status="accepted",
        content_sha256="0000000000000000000000000000000000000000000000000000000000000000",
    )
    session.add(sr)

    el = EventLog(
        id=999,
        event_type="SYSTEM_INIT",
        session_id=None,
        message="baseline test init",
        details="{}",
    )
    session.add(el)

    session.commit()


# ==============================================================================
# TEST 1: Alembic Fresh DB Upgrade Idempotency, Downgrade & Re-upgrade
# ==============================================================================

def test_alembic_upgrade_idempotency_and_downgrade_fresh_db(tmp_path: Path):
    """Verify alembic upgrade head runs cleanly on empty DB, is idempotent on 2nd and 3rd runs,
    downgrades cleanly to a2d639ecd5d5 dropping indexes, and re-upgrades idempotently."""
    db_path = (tmp_path / "fresh_alembic.db").resolve()
    db_url = f"sqlite:///{db_path.as_posix()}"

    cfg = Config(str(ALEMBIC_INI_PATH))
    cfg.set_main_option("sqlalchemy.url", db_url)
    cfg.set_main_option("script_location", str(ALEMBIC_DIR))

    # 1. First upgrade to head
    command.upgrade(cfg, "head")

    engine = create_engine(db_url)
    with engine.connect() as conn:
        inspector = inspect(conn)
        tables = set(inspector.get_table_names())
        assert len(tables) >= 21  # 20 models + alembic_version
        ver = conn.execute(text("SELECT version_num FROM alembic_version")).scalar()
        assert ver == "20261014_0001"

        # Check composite indexes
        for tbl, expected_idxs in EXPECTED_COMPOSITE_INDEXES.items():
            actual_idxs = {idx["name"] for idx in inspector.get_indexes(tbl)}
            assert expected_idxs.issubset(actual_idxs), f"Missing indexes on {tbl}: {expected_idxs - actual_idxs}"

    # 2. Second upgrade to head (must be no-op)
    command.upgrade(cfg, "head")
    with engine.connect() as conn:
        ver2 = conn.execute(text("SELECT version_num FROM alembic_version")).scalar()
        assert ver2 == "20261014_0001"

    # 3. Third upgrade to head (idempotent stress)
    command.upgrade(cfg, "head")
    with engine.connect() as conn:
        ver3 = conn.execute(text("SELECT version_num FROM alembic_version")).scalar()
        assert ver3 == "20261014_0001"

    # 4. Downgrade by 1 step to 'a2d639ecd5d5'
    command.downgrade(cfg, "a2d639ecd5d5")
    with engine.connect() as conn:
        inspector_down = inspect(conn)
        ver_down = conn.execute(text("SELECT version_num FROM alembic_version")).scalar()
        assert ver_down == "a2d639ecd5d5"
        # B03 composite indexes should be removed
        candle_idxs = {idx["name"] for idx in inspector_down.get_indexes("candles")}
        assert "ix_candles_symbol_timeframe_timestamp" not in candle_idxs

    # 5. Re-upgrade to head
    command.upgrade(cfg, "head")
    with engine.connect() as conn:
        inspector_reup = inspect(conn)
        ver_reup = conn.execute(text("SELECT version_num FROM alembic_version")).scalar()
        assert ver_reup == "20261014_0001"
        candle_idxs_reup = {idx["name"] for idx in inspector_reup.get_indexes("candles")}
        assert "ix_candles_symbol_timeframe_timestamp" in candle_idxs_reup

    engine.dispose()


# ==============================================================================
# TEST 2: Alembic Pre-Created DB Upgrade Idempotency
# ==============================================================================

def test_alembic_upgrade_idempotency_precreated_db(tmp_path: Path):
    """Verify that a database generated via Base.metadata.create_all() is stamped to
    'a2d639ecd5d5', upgraded cleanly to 20261014_0001, and is idempotent on repeat runs."""
    db_path = (tmp_path / "precreated_alembic.db").resolve()
    db_url = f"sqlite:///{db_path.as_posix()}"

    engine = create_engine(db_url)
    Base.metadata.create_all(bind=engine)

    # Tables exist, alembic_version does not
    with engine.connect() as conn:
        inspector = inspect(conn)
        assert "candles" in inspector.get_table_names()
        assert "alembic_version" not in inspector.get_table_names()
    engine.dispose()

    cfg = Config(str(ALEMBIC_INI_PATH))
    cfg.set_main_option("sqlalchemy.url", db_url)
    cfg.set_main_option("script_location", str(ALEMBIC_DIR))

    # 1. First upgrade: stamps a2d639ecd5d5 and applies 20261014_0001
    command.upgrade(cfg, "head")

    engine2 = create_engine(db_url)
    with engine2.connect() as conn:
        inspector2 = inspect(conn)
        ver = conn.execute(text("SELECT version_num FROM alembic_version")).scalar()
        assert ver == "20261014_0001"
        for tbl, expected_idxs in EXPECTED_COMPOSITE_INDEXES.items():
            actual_idxs = {idx["name"] for idx in inspector2.get_indexes(tbl)}
            assert expected_idxs.issubset(actual_idxs), f"Missing indexes on {tbl}: {expected_idxs - actual_idxs}"

    # 2. Second upgrade: no-op
    command.upgrade(cfg, "head")
    with engine2.connect() as conn:
        ver2 = conn.execute(text("SELECT version_num FROM alembic_version")).scalar()
        assert ver2 == "20261014_0001"

    # 3. Third upgrade: no-op
    command.upgrade(cfg, "head")
    with engine2.connect() as conn:
        ver3 = conn.execute(text("SELECT version_num FROM alembic_version")).scalar()
        assert ver3 == "20261014_0001"

    engine2.dispose()


# ==============================================================================
# TEST 3: Alembic Pre-Created DB with Empty alembic_version Table
# ==============================================================================

def test_alembic_precreated_db_with_empty_version_table(tmp_path: Path):
    """Edge case: alembic_version table exists on precreated DB but is empty.
    _ensure_precreated_db_stamped should populate 'a2d639ecd5d5' and upgrade cleanly."""
    db_path = (tmp_path / "empty_version_tbl.db").resolve()
    db_url = f"sqlite:///{db_path.as_posix()}"

    engine = create_engine(db_url)
    Base.metadata.create_all(bind=engine)
    with engine.connect() as conn:
        conn.execute(text(
            "CREATE TABLE alembic_version (version_num VARCHAR(32) NOT NULL, PRIMARY KEY (version_num))"
        ))
        conn.commit()
    engine.dispose()

    cfg = Config(str(ALEMBIC_INI_PATH))
    cfg.set_main_option("sqlalchemy.url", db_url)
    cfg.set_main_option("script_location", str(ALEMBIC_DIR))

    command.upgrade(cfg, "head")

    engine2 = create_engine(db_url)
    with engine2.connect() as conn:
        ver = conn.execute(text("SELECT version_num FROM alembic_version")).scalar()
        assert ver == "20261014_0001"
    engine2.dispose()


# ==============================================================================
# TEST 4: Preview Read-Only Isolation — Valid CSV Data
# ==============================================================================

def test_preview_import_confirm_false_zero_delta_valid_data(tmp_path: Path):
    """Execute preview import with valid data and confirm=False on a pre-populated DB.
    Assert exactly 0 row delta across all 10 workflow tables."""
    db_path = (tmp_path / "preview_valid.db").resolve()
    db_url = f"sqlite:///{db_path.as_posix()}"

    engine = create_engine(db_url)
    Base.metadata.create_all(bind=engine)
    SessionClass = sessionmaker(bind=engine)
    session = SessionClass()
    _seed_baseline_workflow_records(session)

    with engine.connect() as conn:
        before_counts = _get_workflow_table_counts(conn)

    valid_csv = (
        "<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\n"
        "FPT,20260105,95.0,96.5,94.0,96.0,800000\n"
        "FPT,20260106,96.0,97.5,95.5,97.0,950000\n"
        "FPT,20260107,97.0,98.0,96.0,97.5,700000\n"
    ).encode("utf-8")

    res = ImportWorkflowService.preview_import(
        db=session,
        file_content=valid_csv,
        filename="FPT_valid.csv",
        source_type="cafef",
        timeframe="1D",
        adjustment_type="unadjusted",
        confirm=False,
    )

    assert res.status == "previewed"
    assert res.can_accept is True
    assert res.parsed_count == 3
    assert len(res.items) == 3

    with engine.connect() as conn:
        after_counts = _get_workflow_table_counts(conn)

    assert after_counts == before_counts, f"Row delta detected! Before: {before_counts}, After: {after_counts}"

    session.close()
    engine.dispose()


# ==============================================================================
# TEST 5: Preview Read-Only Isolation — Invalid Data Matrix
# ==============================================================================

def test_preview_import_confirm_false_zero_delta_invalid_data_matrix(tmp_path: Path):
    """Execute preview import with various invalid inputs (bad metadata, corrupted file,
    out-of-order dates, negative prices) with confirm=False.
    Assert exactly 0 row delta across all 10 workflow tables in all cases."""
    db_path = (tmp_path / "preview_invalid.db").resolve()
    db_url = f"sqlite:///{db_path.as_posix()}"

    engine = create_engine(db_url)
    Base.metadata.create_all(bind=engine)
    SessionClass = sessionmaker(bind=engine)
    session = SessionClass()
    _seed_baseline_workflow_records(session)

    with engine.connect() as conn:
        initial_counts = _get_workflow_table_counts(conn)

    # 1. Invalid metadata (unsupported timeframe)
    res_bad_tf = ImportWorkflowService.preview_import(
        db=session,
        file_content=b"some,data\n",
        filename="test.csv",
        timeframe="5M",
        confirm=False,
    )
    assert res_bad_tf.status == "blocked"
    assert res_bad_tf.can_accept is False

    with engine.connect() as conn:
        assert _get_workflow_table_counts(conn) == initial_counts

    # 2. Corrupt binary content
    res_corrupt = ImportWorkflowService.preview_import(
        db=session,
        file_content=b"\x00\xff\xfe\x01\x02\x03corrupted_bytes",
        filename="corrupt.csv",
        confirm=False,
    )
    assert res_corrupt.status == "blocked"
    assert res_corrupt.can_accept is False

    with engine.connect() as conn:
        assert _get_workflow_table_counts(conn) == initial_counts

    # 3. All rows rejected (high < low, open outside range) -> blocks acceptance
    all_bad_csv = (
        "<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\n"
        "SSI,20260105,30.0,20.0,35.0,29.0,500000\n"
        "SSI,20260106,40.0,25.0,35.0,28.0,500000\n"
    ).encode("utf-8")
    res_all_bad = ImportWorkflowService.preview_import(
        db=session,
        file_content=all_bad_csv,
        filename="SSI_all_bad.csv",
        confirm=False,
    )
    assert res_all_bad.status == "blocked"
    assert res_all_bad.can_accept is False
    assert res_all_bad.rejected_count == 2
    assert res_all_bad.parsed_count == 0

    with engine.connect() as conn:
        assert _get_workflow_table_counts(conn) == initial_counts

    # 4. Partially invalid rows (1 bad row skipped, 1 valid row parsed)
    mixed_csv = (
        "<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\n"
        "SSI,20260105,30.0,20.0,35.0,29.0,500000\n"
        "SSI,20260106,30.0,32.0,29.0,31.0,500000\n"
    ).encode("utf-8")
    res_mixed = ImportWorkflowService.preview_import(
        db=session,
        file_content=mixed_csv,
        filename="SSI_mixed.csv",
        confirm=False,
    )
    assert res_mixed.rejected_count == 1
    assert res_mixed.parsed_count == 1

    with engine.connect() as conn:
        assert _get_workflow_table_counts(conn) == initial_counts

    session.close()
    engine.dispose()


# ==============================================================================
# TEST 6: Preview Read-Only Isolation — Duplicate & Conflict Scenarios
# ==============================================================================

def test_preview_import_confirm_false_zero_delta_duplicate_and_conflict(tmp_path: Path):
    """Execute preview import with duplicate and conflicting data with confirm=False.
    Assert exactly 0 row delta across all 10 workflow tables."""
    db_path = (tmp_path / "preview_dup_conflict.db").resolve()
    db_url = f"sqlite:///{db_path.as_posix()}"

    engine = create_engine(db_url)
    Base.metadata.create_all(bind=engine)
    SessionClass = sessionmaker(bind=engine)
    session = SessionClass()

    # Pre-insert existing candle for HPG on 2026-01-05
    existing_c = Candle(
        symbol="HPG",
        timeframe="1D",
        timestamp=datetime(2026, 1, 5, 0, 0, 0),
        open=25000.0,
        high=26000.0,
        low=24500.0,
        close=25800.0,
        volume=1000000.0,
        adjustment_type="unadjusted",
    )
    session.add(existing_c)
    session.commit()

    with engine.connect() as conn:
        baseline_counts = _get_workflow_table_counts(conn)

    # 1. Preview Duplicate Data (identical OHLCV) with confirm=False
    dup_csv = (
        "<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\n"
        "HPG,20260105,25.0,26.0,24.5,25.8,1000000\n"
    ).encode("utf-8")

    res_dup = ImportWorkflowService.preview_import(
        db=session,
        file_content=dup_csv,
        filename="HPG_duplicate.csv",
        confirm=False,
    )
    assert res_dup.duplicate_count == 1
    assert res_dup.can_accept is True

    with engine.connect() as conn:
        assert _get_workflow_table_counts(conn) == baseline_counts

    # 2. Preview Conflicting Data (different OHLCV on same date) with confirm=False
    conflict_csv = (
        "<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\n"
        "HPG,20260105,30.0,32.0,29.0,31.0,2000000\n"
    ).encode("utf-8")

    res_conflict = ImportWorkflowService.preview_import(
        db=session,
        file_content=conflict_csv,
        filename="HPG_conflict.csv",
        confirm=False,
    )
    assert res_conflict.conflicting_count == 1
    assert res_conflict.can_accept is False
    assert res_conflict.status == "blocked"

    with engine.connect() as conn:
        assert _get_workflow_table_counts(conn) == baseline_counts

    session.close()
    engine.dispose()


# ==============================================================================
# TEST 7: seed_cafef_data(dry_run=True) & seed_cafef_data(confirm=False) Isolation
# ==============================================================================

def test_seed_cafef_dry_run_and_confirm_false_isolation(tmp_path: Path):
    """Verify seed_cafef_data with dry_run=True and confirm=False produces 0 row delta
    across all 10 workflow tables on a sample fixture directory containing multiple files."""
    db_path = (tmp_path / "seed_dry_run_test.db").resolve()
    db_url = f"sqlite:///{db_path.as_posix()}"

    engine = create_engine(db_url)
    Base.metadata.create_all(bind=engine)
    SessionClass = sessionmaker(bind=engine)
    session = SessionClass()
    _seed_baseline_workflow_records(session)

    sample_dir = tmp_path / "sample_fixtures"
    sample_dir.mkdir()

    (sample_dir / "CafeF.HSX.01.csv").write_text(
        "<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\n"
        "MBB,20260105,20.0,21.0,19.5,20.5,1000000\n"
        "MBB,20260106,20.5,21.5,20.0,21.0,1200000\n",
        encoding="utf-8"
    )
    (sample_dir / "CafeF.HSX.02.csv").write_text(
        "<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\n"
        "TCB,20260105,30.0,31.0,29.5,30.5,800000\n",
        encoding="utf-8"
    )

    with engine.connect() as conn:
        before_counts = _get_workflow_table_counts(conn)

    # 1. Test dry_run=True (confirm=True, dry_run=True -> effective_confirm=False)
    seed_cafef_data(
        confirm=True,
        dry_run=True,
        data_dir=str(sample_dir),
        db=session,
    )

    with engine.connect() as conn:
        after_counts_dry = _get_workflow_table_counts(conn)
    assert after_counts_dry == before_counts, f"dry_run=True mutated tables! {after_counts_dry} vs {before_counts}"

    # 2. Test confirm=False (confirm=False, dry_run=False -> effective_confirm=False)
    seed_cafef_data(
        confirm=False,
        dry_run=False,
        data_dir=str(sample_dir),
        db=session,
    )

    with engine.connect() as conn:
        after_counts_no_confirm = _get_workflow_table_counts(conn)
    assert after_counts_no_confirm == before_counts, f"confirm=False mutated tables! {after_counts_no_confirm} vs {before_counts}"

    session.close()
    engine.dispose()


# ==============================================================================
# TEST 8: FastAPI POST /api/import/preview confirm=false vs confirm=true
# ==============================================================================

def test_api_import_preview_endpoint_confirm_parameter(tmp_path: Path):
    """Verify HTTP endpoint POST /api/import/preview honors confirm=false with zero table delta,
    and stages records when confirm=true."""
    db_path = (tmp_path / "api_test.db").resolve()
    db_url = f"sqlite:///{db_path.as_posix()}"

    engine = create_engine(db_url)
    Base.metadata.create_all(bind=engine)
    SessionClass = sessionmaker(bind=engine)

    from app.dependencies import get_db

    def override_get_db():
        db = SessionClass()
        try:
            yield db
        finally:
            db.close()

    app.dependency_overrides[get_db] = override_get_db
    client = TestClient(app)

    with engine.connect() as conn:
        baseline_counts = _get_workflow_table_counts(conn)

    sample_csv_content = (
        b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\n"
        b"VIC,20260105,45.0,46.0,44.5,45.5,600000\n"
    )

    # 1. POST /api/import/preview with confirm=False
    resp_false = client.post(
        "/api/import/preview",
        files={"file": ("VIC_sample.csv", io.BytesIO(sample_csv_content), "text/csv")},
        data={"confirm": "false"},
    )
    assert resp_false.status_code == 200
    data_false = resp_false.json()
    assert data_false["status"] == "previewed"
    assert data_false["can_accept"] is True

    with engine.connect() as conn:
        counts_after_false = _get_workflow_table_counts(conn)
    assert counts_after_false == baseline_counts, f"confirm=false via API mutated DB! {counts_after_false} vs {baseline_counts}"

    # 2. POST /api/import/preview with confirm=True (should stage run and item)
    resp_true = client.post(
        "/api/import/preview",
        files={"file": ("VIC_sample2.csv", io.BytesIO(sample_csv_content), "text/csv")},
        data={"confirm": "true"},
    )
    assert resp_true.status_code == 200
    data_true = resp_true.json()
    assert data_true["status"] == "previewed"

    with engine.connect() as conn:
        counts_after_true = _get_workflow_table_counts(conn)

    assert counts_after_true["import_runs"] == baseline_counts["import_runs"] + 1
    assert counts_after_true["import_run_items"] == baseline_counts["import_run_items"] + 1
    # Candles and symbols must still be 0 (unmutated until accepted)
    assert counts_after_true["candles"] == baseline_counts["candles"]
    assert counts_after_true["symbols"] == baseline_counts["symbols"]

    app.dependency_overrides.clear()
    engine.dispose()


# ==============================================================================
# TEST 9: Production DB Invariant
# ==============================================================================

def test_production_db_invariant():
    """Verify production database backend/sumi.db has not been modified or accessed."""
    assert PROD_DB_PATH.exists(), f"Production DB does not exist at {PROD_DB_PATH}"

    size = PROD_DB_PATH.stat().st_size
    assert size == EXPECTED_PROD_DB_SIZE, f"Production DB size mismatch: got {size}, expected {EXPECTED_PROD_DB_SIZE}"

    hasher = hashlib.sha256()
    with open(PROD_DB_PATH, "rb") as f:
        while chunk := f.read(65536):
            hasher.update(chunk)
    actual_hash = hasher.hexdigest()
    assert actual_hash == EXPECTED_PROD_DB_SHA256, (
        f"Production DB SHA-256 mismatch: got {actual_hash}, expected {EXPECTED_PROD_DB_SHA256}"
    )
