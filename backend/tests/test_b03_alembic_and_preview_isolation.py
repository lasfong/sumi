"""Test suite for Milestone 2: Phase B (Batch B03).

Verifies:
1. Exact 20 models exported by app.models are registered in Alembic env.py (GAP-DB-05).
2. Alembic upgrade head succeeds on both empty and pre-created SQLite databases without
   rewriting historical migrations (GAP-DB-06, GAP-DB-10).
3. Import preview with confirm=False produces 0 row delta across all 10 workflow tables (GAP-DB-14).
4. seed_cafef_data with dry_run=True produces 0 row delta across all 10 workflow tables.
5. confirm=True preserves full staging, accept, weekly synthesis, and rollback pipeline.

All database operations strictly execute on isolated temporary databases (tmp_path).
"""
import os
from pathlib import Path
import pytest
from sqlalchemy import create_engine, inspect, text
from sqlalchemy.orm import sessionmaker

from alembic.config import Config
from alembic import command

from app.db import Base
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

EXPECTED_20_TABLES = {
    "symbols",
    "candles",
    "replay_sessions",
    "decisions",
    "orders",
    "executions",
    "positions",
    "trades",
    "journal_entries",
    "event_logs",
    "drawing_states",
    "strategy_lab_runs",
    "scanner_runs",
    "import_runs",
    "import_run_items",
    "import_run_mutations",
    "weekly_candle_provenance",
    "sync_runs",
    "sync_run_items",
    "sync_run_mutations",
}


def _get_table_counts(conn) -> dict[str, int]:
    counts = {}
    for table_name in WORKFLOW_TABLES:
        val = conn.execute(text(f"SELECT COUNT(*) FROM {table_name}")).scalar()
        counts[table_name] = int(val)
    return counts


def test_alembic_env_registers_exact_20_expected_models():
    """GAP-DB-05: Verify Alembic env.py imports and registers exact 20 expected models."""
    import ast

    # 1. Statically parse backend/alembic/env.py to verify exact 20 model imports
    env_file = ALEMBIC_DIR / "env.py"
    env_code = env_file.read_text(encoding="utf-8")
    tree = ast.parse(env_code)
    imported_models = set()
    for node in ast.walk(tree):
        if isinstance(node, ast.ImportFrom) and node.module == "app.models":
            for alias in node.names:
                imported_models.add(alias.name)

    expected_model_names = {
        "Symbol",
        "Candle",
        "ReplaySession",
        "Decision",
        "Order",
        "Execution",
        "Position",
        "Trade",
        "JournalEntry",
        "EventLog",
        "DrawingState",
        "StrategyLabRun",
        "ScannerRun",
        "ImportRun",
        "ImportRunItem",
        "ImportRunMutation",
        "WeeklyCandleProvenance",
        "SyncRun",
        "SyncRunItem",
        "SyncRunMutation",
    }
    assert imported_models == expected_model_names, (
        f"Mismatch in env.py imported models: {expected_model_names ^ imported_models}"
    )

    # 2. Inspect Base.metadata registered tables
    registered_tables = set(Base.metadata.tables.keys())
    assert EXPECTED_20_TABLES.issubset(registered_tables), (
        f"Missing tables in Base.metadata: {EXPECTED_20_TABLES - registered_tables}"
    )

    # 3. Verify exactly 20 distinct ORM model classes are exported in __all__
    from app import models
    exported_models = [getattr(models, name) for name in models.__all__]
    assert len(exported_models) == 20
    assert len(EXPECTED_20_TABLES) == 20


def test_alembic_upgrade_head_on_empty_and_precreated_sqlite_db(tmp_path):
    """GAP-DB-06, GAP-DB-10: Verify migration chain runs on both empty and pre-created DBs."""
    # -------------------------------------------------------------
    # Scenario A: Clean migration from empty DB to head
    # -------------------------------------------------------------
    empty_db_path = (tmp_path / "empty_migration.db").resolve()
    empty_db_url = f"sqlite:///{empty_db_path.as_posix()}"

    cfg_empty = Config(str(ALEMBIC_INI_PATH))
    cfg_empty.set_main_option("sqlalchemy.url", empty_db_url)
    cfg_empty.set_main_option("script_location", str(ALEMBIC_DIR))

    command.upgrade(cfg_empty, "head")

    engine_empty = create_engine(empty_db_url)
    with engine_empty.connect() as conn:
        inspector = inspect(conn)
        tables = set(inspector.get_table_names())
        assert EXPECTED_20_TABLES.issubset(tables), f"Tables missing after fresh upgrade: {EXPECTED_20_TABLES - tables}"
        assert "alembic_version" in tables

        ver = conn.execute(text("SELECT version_num FROM alembic_version")).scalar()
        assert ver == "20261014_0001", f"Expected head revision '20261014_0001', got: '{ver}'"

        # Check composite indexes created by 20261014_0001
        candle_indexes = {idx["name"] for idx in inspector.get_indexes("candles")}
        assert "ix_candles_symbol_timeframe_timestamp" in candle_indexes

        decision_indexes = {idx["name"] for idx in inspector.get_indexes("decisions")}
        assert "ix_decisions_session_id" in decision_indexes
        assert "ix_decisions_session_candle" in decision_indexes

        exec_indexes = {idx["name"] for idx in inspector.get_indexes("executions")}
        assert "ix_executions_session_id" in exec_indexes
        assert "ix_executions_session_candle" in exec_indexes

        import_item_indexes = {idx["name"] for idx in inspector.get_indexes("import_run_items")}
        assert "ix_import_run_items_run_symbol" in import_item_indexes

        sync_item_indexes = {idx["name"] for idx in inspector.get_indexes("sync_run_items")}
        assert "ix_sync_run_items_sync_symbol" in sync_item_indexes
    engine_empty.dispose()

    # -------------------------------------------------------------
    # Scenario B: Migration on database pre-created via Base.metadata.create_all()
    # -------------------------------------------------------------
    precreated_db_path = (tmp_path / "precreated_migration.db").resolve()
    precreated_db_url = f"sqlite:///{precreated_db_path.as_posix()}"

    engine_pre = create_engine(precreated_db_url)
    Base.metadata.create_all(bind=engine_pre)

    # Verify tables exist but alembic_version does not yet exist
    with engine_pre.connect() as conn:
        inspector_before = inspect(conn)
        assert "candles" in inspector_before.get_table_names()
        assert "alembic_version" not in inspector_before.get_table_names()
    engine_pre.dispose()

    cfg_pre = Config(str(ALEMBIC_INI_PATH))
    cfg_pre.set_main_option("sqlalchemy.url", precreated_db_url)
    cfg_pre.set_main_option("script_location", str(ALEMBIC_DIR))

    # Should stamp baseline a2d639ecd5d5 then apply 20261014_0001 cleanly
    command.upgrade(cfg_pre, "head")

    engine_post = create_engine(precreated_db_url)
    with engine_post.connect() as conn:
        inspector_after = inspect(conn)
        tables_after = set(inspector_after.get_table_names())
        assert EXPECTED_20_TABLES.issubset(tables_after)
        assert "alembic_version" in tables_after

        ver_after = conn.execute(text("SELECT version_num FROM alembic_version")).scalar()
        assert ver_after == "20261014_0001", f"Expected stamped head revision '20261014_0001', got: '{ver_after}'"

        candle_indexes = {idx["name"] for idx in inspector_after.get_indexes("candles")}
        assert "ix_candles_symbol_timeframe_timestamp" in candle_indexes

        exec_indexes = {idx["name"] for idx in inspector_after.get_indexes("executions")}
        assert "ix_executions_session_candle" in exec_indexes
    engine_post.dispose()


def test_import_preview_confirm_false_is_read_only_across_all_workflow_tables(tmp_path):
    """GAP-DB-14: preview_import(confirm=False) must produce 0 row delta across all 10 workflow tables."""
    db_path = (tmp_path / "preview_readonly.db").resolve()
    db_url = f"sqlite:///{db_path.as_posix()}"

    engine = create_engine(db_url)
    Base.metadata.create_all(bind=engine)
    Session = sessionmaker(bind=engine)
    session = Session()

    sample_csv = (
        "<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\n"
        "HPG,20260105,25.0,26.0,24.5,25.8,1000000\n"
        "HPG,20260106,25.8,26.5,25.5,26.2,1200000\n"
    ).encode("utf-8")

    with engine.connect() as conn:
        before_counts = _get_table_counts(conn)

    # 1. Preview with confirm=False (valid CSV)
    res_valid = ImportWorkflowService.preview_import(
        db=session,
        file_content=sample_csv,
        filename="HPG_valid.csv",
        confirm=False,
    )
    assert res_valid.status == "previewed"
    assert res_valid.can_accept is True
    assert res_valid.parsed_count == 2
    assert len(res_valid.items) == 2

    with engine.connect() as conn:
        after_counts_1 = _get_table_counts(conn)
    assert after_counts_1 == before_counts, f"Row count changed after valid preview: {after_counts_1} vs {before_counts}"

    # 2. Preview with confirm=False (invalid metadata)
    res_bad_meta = ImportWorkflowService.preview_import(
        db=session,
        file_content=sample_csv,
        filename="HPG_bad_meta.csv",
        timeframe="INVALID_TF",
        confirm=False,
    )
    assert res_bad_meta.status == "blocked"
    assert res_bad_meta.can_accept is False

    with engine.connect() as conn:
        after_counts_2 = _get_table_counts(conn)
    assert after_counts_2 == before_counts, f"Row count changed after metadata error preview: {after_counts_2} vs {before_counts}"

    # 3. Preview with confirm=False (corrupt unparseable file)
    res_corrupt = ImportWorkflowService.preview_import(
        db=session,
        file_content=b"NOT_A_VALID_CSV\x00\xff\xfe",
        filename="corrupt.csv",
        confirm=False,
    )
    assert res_corrupt.status == "blocked"
    assert res_corrupt.can_accept is False

    with engine.connect() as conn:
        after_counts_3 = _get_table_counts(conn)
    assert after_counts_3 == before_counts, f"Row count changed after corrupt file preview: {after_counts_3} vs {before_counts}"

    session.close()
    engine.dispose()


def test_seed_cafef_dry_run_is_read_only_across_all_workflow_tables(tmp_path):
    """GAP-DB-14: seed_cafef_data(dry_run=True) must produce 0 row delta across all 10 workflow tables."""
    db_path = (tmp_path / "seed_dry_run.db").resolve()
    db_url = f"sqlite:///{db_path.as_posix()}"

    engine = create_engine(db_url)
    Base.metadata.create_all(bind=engine)
    Session = sessionmaker(bind=engine)
    session = Session()

    # Create temporary fixture sample directory
    sample_dir = tmp_path / "cafef_sample"
    sample_dir.mkdir()

    sample_csv_path = sample_dir / "CafeF.HSX.Sample.csv"
    sample_csv_path.write_text(
        "<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\n"
        "VNM,20260105,70.0,71.0,69.5,70.5,500000\n"
        "VNM,20260106,70.5,72.0,70.0,71.8,650000\n",
        encoding="utf-8"
    )

    with engine.connect() as conn:
        before_counts = _get_table_counts(conn)

    # Call seed_cafef_data with dry_run=True
    seed_cafef_data(
        confirm=False,
        dry_run=True,
        data_dir=str(sample_dir),
        db=session
    )

    with engine.connect() as conn:
        after_counts = _get_table_counts(conn)

    assert after_counts == before_counts, f"seed_cafef_data(dry_run=True) modified database rows! {after_counts} vs {before_counts}"

    session.close()
    engine.dispose()


def test_import_preview_confirm_true_and_accept_workflow_preserved(tmp_path):
    """Verify confirm=True preserves full staging, accept, weekly synthesis, and rollback pipeline."""
    db_path = (tmp_path / "preview_confirm_true.db").resolve()
    db_url = f"sqlite:///{db_path.as_posix()}"

    engine = create_engine(db_url)
    Base.metadata.create_all(bind=engine)
    Session = sessionmaker(bind=engine)
    session = Session()

    sample_csv = (
        "<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\n"
        "FPT,20260105,90.0,92.0,89.5,91.5,800000\n"
        "FPT,20260106,91.5,93.0,91.0,92.5,950000\n"
    ).encode("utf-8")

    # 1. Preview with confirm=True (stages ImportRun and items)
    preview = ImportWorkflowService.preview_import(
        db=session,
        file_content=sample_csv,
        filename="FPT_confirm_true.csv",
        confirm=True,
    )
    assert preview.can_accept is True

    # ImportRun and ImportRunItem should exist in DB
    staged_run = session.query(ImportRun).filter(ImportRun.id == preview.run_id).first()
    assert staged_run is not None
    assert staged_run.status == "previewed"

    staged_items = session.query(ImportRunItem).filter(ImportRunItem.run_id == preview.run_id).all()
    assert len(staged_items) == 2

    # 2. Accept import
    accept_res = ImportWorkflowService.accept_import(
        db=session,
        run_id=preview.run_id,
        content_sha256=preview.content_sha256
    )
    assert accept_res.status == "accepted"
    assert accept_res.accepted_count == 2

    # Check 1D candles exist in DB (scaled 1000x for equities: 90.0 -> 90000.0)
    fpt_candles = session.query(Candle).filter(Candle.symbol == "FPT", Candle.timeframe == "1D").all()
    assert len(fpt_candles) == 2

    # Check 1W weekly synthesized candle exists
    fpt_weekly = session.query(Candle).filter(Candle.symbol == "FPT", Candle.timeframe == "1W").all()
    assert len(fpt_weekly) == 1

    # 3. Rollback import
    rollback_res = ImportWorkflowService.rollback_import(db=session, run_id=preview.run_id)
    assert rollback_res.status == "rolled_back"

    # Candles should be reverted
    fpt_candles_after = session.query(Candle).filter(Candle.symbol == "FPT", Candle.timeframe == "1D").all()
    assert len(fpt_candles_after) == 0

    session.close()
    engine.dispose()
