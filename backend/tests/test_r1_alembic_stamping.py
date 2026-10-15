"""Test suite for Requirement R1: Safe Alembic Pre-Created Database Stamping.

Verifies:
1. Fresh schema (empty DB): Alembic executes full migration chain from scratch to head without stamping.
2. Complete pre-created schema: _ensure_precreated_db_stamped validates all 20 historical tables
   and critical historical columns, stamps revision 'a2d639ecd5d5', and allows subsequent additive
   migrations to apply cleanly to head.
3. Deliberately partial schema (only 'candles' table exists): Fails closed with an actionable
   RuntimeError diagnostic detailing missing required tables; does not stamp revision.
4. Partial schema missing a table (e.g., 19 of 20 tables exist): Fails closed with actionable
   RuntimeError diagnostic specifying the missing table.
5. Partial schema missing critical historical columns:
   - executions missing 'execution_candle_index'
   - executions missing 'trade_id'
   - replay_sessions missing 'source_type'
   - decisions missing 'stop_loss'
   - trades missing 'planned_entry_price'
   Each case fails closed with an actionable RuntimeError diagnostic naming the table and missing column.
6. Idempotency: Calling _ensure_precreated_db_stamped on an already-migrated database does not
   overwrite existing alembic_version.
7. Verification that backend/sumi.db is strictly untouched.

All database operations strictly execute on isolated temporary databases (tmp_path).
"""
import hashlib
import importlib.util
from pathlib import Path
import pytest
from sqlalchemy import create_engine, inspect, text

from alembic.config import Config
from alembic import command

from app.db import Base

BACKEND_DIR = Path(__file__).resolve().parent.parent
ALEMBIC_INI_PATH = BACKEND_DIR / "alembic.ini"
ALEMBIC_DIR = BACKEND_DIR / "alembic"
SUMI_DB_PATH = BACKEND_DIR / "sumi.db"
EXPECTED_SUMI_DB_SHA256 = "234206fc75259f091337d4feb5f41575d0fe5ac02d6c10ba0de8fad68cbcee86"

EXPECTED_20_TABLES = frozenset({
    "candles",
    "symbols",
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
})


def _load_env_module():
    env_path = ALEMBIC_DIR / "env.py"
    spec = importlib.util.spec_from_file_location("sumi_alembic_env", env_path)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


env_mod = _load_env_module()
REQUIRED_HISTORICAL_TABLES = env_mod.REQUIRED_HISTORICAL_TABLES
REQUIRED_HISTORICAL_COLUMNS = env_mod.REQUIRED_HISTORICAL_COLUMNS
_ensure_precreated_db_stamped = env_mod._ensure_precreated_db_stamped


def _build_alembic_config(db_url: str) -> Config:
    cfg = Config(str(ALEMBIC_INI_PATH))
    cfg.set_main_option("sqlalchemy.url", db_url)
    cfg.set_main_option("script_location", str(ALEMBIC_DIR))
    return cfg


def test_constants_and_module_exports():
    """Verify REQUIRED_HISTORICAL_TABLES and REQUIRED_HISTORICAL_COLUMNS specifications."""
    assert REQUIRED_HISTORICAL_TABLES == EXPECTED_20_TABLES
    assert len(REQUIRED_HISTORICAL_TABLES) == 20
    assert isinstance(REQUIRED_HISTORICAL_TABLES, frozenset)

    assert "executions" in REQUIRED_HISTORICAL_COLUMNS
    assert "execution_candle_index" in REQUIRED_HISTORICAL_COLUMNS["executions"]
    assert "trade_id" in REQUIRED_HISTORICAL_COLUMNS["executions"]

    assert "replay_sessions" in REQUIRED_HISTORICAL_COLUMNS
    assert "source_type" in REQUIRED_HISTORICAL_COLUMNS["replay_sessions"]

    assert "decisions" in REQUIRED_HISTORICAL_COLUMNS
    assert "stop_loss" in REQUIRED_HISTORICAL_COLUMNS["decisions"]

    assert "trades" in REQUIRED_HISTORICAL_COLUMNS
    assert "planned_entry_price" in REQUIRED_HISTORICAL_COLUMNS["trades"]


def test_fresh_schema_empty_db_migrates_cleanly_from_scratch(tmp_path):
    """Scenario 1: Fresh empty database allows Alembic to run all migrations from scratch to head."""
    db_path = (tmp_path / "fresh_db.db").resolve()
    db_url = f"sqlite:///{db_path.as_posix()}"

    cfg = _build_alembic_config(db_url)
    command.upgrade(cfg, "head")

    engine = create_engine(db_url)
    with engine.connect() as conn:
        inspector = inspect(conn)
        tables = set(inspector.get_table_names())
        assert EXPECTED_20_TABLES.issubset(tables), f"Missing tables after fresh upgrade: {EXPECTED_20_TABLES - tables}"
        assert "alembic_version" in tables

        ver = conn.execute(text("SELECT version_num FROM alembic_version")).scalar()
        assert ver == "20261014_0001", f"Expected head revision '20261014_0001', got: '{ver}'"

        # Check that composite indexes created by head revision exist
        candle_idx = {idx["name"] for idx in inspector.get_indexes("candles")}
        assert "ix_candles_symbol_timeframe_timestamp" in candle_idx
    engine.dispose()


def test_complete_precreated_schema_stamps_and_applies_subsequent_migrations(tmp_path):
    """Scenario 2: Complete pre-created DB is verified, stamped with 'a2d639ecd5d5', and upgraded to head."""
    db_path = (tmp_path / "complete_precreated.db").resolve()
    db_url = f"sqlite:///{db_path.as_posix()}"

    # Pre-create all 20 tables via Base.metadata.create_all()
    engine = create_engine(db_url)
    Base.metadata.create_all(bind=engine)

    # Before migration: all 20 tables exist, alembic_version does not exist
    with engine.connect() as conn:
        inspector_before = inspect(conn)
        tables_before = set(inspector_before.get_table_names())
        assert EXPECTED_20_TABLES.issubset(tables_before)
        assert "alembic_version" not in tables_before
    engine.dispose()

    # Run alembic upgrade head
    cfg = _build_alembic_config(db_url)
    command.upgrade(cfg, "head")

    # After migration: stamped with a2d639ecd5d5 and subsequently upgraded to 20261014_0001
    engine_post = create_engine(db_url)
    with engine_post.connect() as conn:
        inspector_after = inspect(conn)
        tables_after = set(inspector_after.get_table_names())
        assert EXPECTED_20_TABLES.issubset(tables_after)
        assert "alembic_version" in tables_after

        ver = conn.execute(text("SELECT version_num FROM alembic_version")).scalar()
        assert ver == "20261014_0001", f"Expected head revision '20261014_0001', got: '{ver}'"

        # Verify composite indexes from 20261014_0001 applied cleanly
        candle_indexes = {idx["name"] for idx in inspector_after.get_indexes("candles")}
        assert "ix_candles_symbol_timeframe_timestamp" in candle_indexes

        exec_indexes = {idx["name"] for idx in inspector_after.get_indexes("executions")}
        assert "ix_executions_session_candle" in exec_indexes
    engine_post.dispose()


def test_partial_schema_only_candles_fails_closed_with_actionable_diagnostic(tmp_path):
    """Scenario 3: Defect reproduction — a database with only 'candles' must fail closed with RuntimeError."""
    db_path = (tmp_path / "only_candles.db").resolve()
    db_url = f"sqlite:///{db_path.as_posix()}"

    engine = create_engine(db_url)
    Base.metadata.tables["candles"].create(bind=engine)

    with engine.connect() as conn:
        inspector = inspect(conn)
        assert set(inspector.get_table_names()) == {"candles"}
    engine.dispose()

    cfg = _build_alembic_config(db_url)

    with pytest.raises(RuntimeError) as exc_info:
        command.upgrade(cfg, "head")

    err_msg = str(exc_info.value)
    assert "partially initialized but incomplete" in err_msg
    assert "missing 19 required historical tables" in err_msg
    assert "Cannot safely stamp 'a2d639ecd5d5' on a partial schema" in err_msg
    assert "symbols" in err_msg
    assert "executions" in err_msg

    # Verify alembic_version was NOT created or stamped
    engine_check = create_engine(db_url)
    with engine_check.connect() as conn:
        inspector_check = inspect(conn)
        assert "alembic_version" not in inspector_check.get_table_names()
    engine_check.dispose()


def test_partial_schema_missing_single_table_fails_closed(tmp_path):
    """Scenario 4: A database with 19 of 20 tables fails closed identifying the missing table."""
    db_path = (tmp_path / "missing_one_table.db").resolve()
    db_url = f"sqlite:///{db_path.as_posix()}"

    # Create 19 tables, omitting 'sync_run_mutations'
    tables_19 = [tbl for name, tbl in Base.metadata.tables.items() if name != "sync_run_mutations"]
    engine = create_engine(db_url)
    Base.metadata.create_all(bind=engine, tables=tables_19)

    with engine.connect() as conn:
        inspector = inspect(conn)
        table_names = set(inspector.get_table_names())
        assert "sync_run_mutations" not in table_names
        assert len(table_names) == 19
    engine.dispose()

    cfg = _build_alembic_config(db_url)

    with pytest.raises(RuntimeError) as exc_info:
        command.upgrade(cfg, "head")

    err_msg = str(exc_info.value)
    assert "partially initialized but incomplete" in err_msg
    assert "missing 1 required historical tables: ['sync_run_mutations']" in err_msg

    # Verify alembic_version was NOT created or stamped
    engine_check = create_engine(db_url)
    with engine_check.connect() as conn:
        inspector_check = inspect(conn)
        assert "alembic_version" not in inspector_check.get_table_names()
    engine_check.dispose()


def test_partial_schema_missing_execution_candle_index_column_fails_closed(tmp_path):
    """Scenario 5a: All 20 tables exist, but 'executions' lacks 'execution_candle_index' -> fails closed."""
    db_path = (tmp_path / "missing_exec_candle_idx.db").resolve()
    db_url = f"sqlite:///{db_path.as_posix()}"

    # Create 19 tables via metadata
    tables_without_executions = [tbl for name, tbl in Base.metadata.tables.items() if name != "executions"]
    engine = create_engine(db_url)
    Base.metadata.create_all(bind=engine, tables=tables_without_executions)

    # Manually create executions without 'execution_candle_index'
    with engine.connect() as conn:
        conn.execute(text("""
            CREATE TABLE executions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                order_id INTEGER NOT NULL,
                trade_id INTEGER,
                session_id INTEGER NOT NULL,
                symbol VARCHAR NOT NULL,
                execution_date DATETIME NOT NULL,
                execution_price FLOAT NOT NULL,
                quantity FLOAT NOT NULL,
                fee FLOAT DEFAULT 0.0,
                tax FLOAT DEFAULT 0.0,
                slippage FLOAT DEFAULT 0.0,
                gross_amount FLOAT NOT NULL,
                net_amount FLOAT NOT NULL,
                created_at DATETIME
            )
        """))
        conn.commit()

        inspector = inspect(conn)
        user_tables = {t for t in inspector.get_table_names() if not t.startswith("sqlite_")}
        assert EXPECTED_20_TABLES == user_tables
        exec_cols = {c["name"] for c in inspector.get_columns("executions")}
        assert "execution_candle_index" not in exec_cols
        assert "trade_id" in exec_cols
    engine.dispose()

    cfg = _build_alembic_config(db_url)

    with pytest.raises(RuntimeError) as exc_info:
        command.upgrade(cfg, "head")

    err_msg = str(exc_info.value)
    assert "missing required historical columns" in err_msg
    assert "executions: ['execution_candle_index']" in err_msg

    # Verify alembic_version was NOT created or stamped
    engine_check = create_engine(db_url)
    with engine_check.connect() as conn:
        inspector_check = inspect(conn)
        assert "alembic_version" not in inspector_check.get_table_names()
    engine_check.dispose()


def test_partial_schema_missing_execution_trade_id_column_fails_closed(tmp_path):
    """Scenario 5b: 'executions' lacks 'trade_id' -> fails closed."""
    db_path = (tmp_path / "missing_trade_id.db").resolve()
    db_url = f"sqlite:///{db_path.as_posix()}"

    tables_without_executions = [tbl for name, tbl in Base.metadata.tables.items() if name != "executions"]
    engine = create_engine(db_url)
    Base.metadata.create_all(bind=engine, tables=tables_without_executions)

    with engine.connect() as conn:
        conn.execute(text("""
            CREATE TABLE executions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                order_id INTEGER NOT NULL,
                session_id INTEGER NOT NULL,
                symbol VARCHAR NOT NULL,
                execution_date DATETIME NOT NULL,
                execution_price FLOAT NOT NULL,
                quantity FLOAT NOT NULL,
                fee FLOAT DEFAULT 0.0,
                tax FLOAT DEFAULT 0.0,
                slippage FLOAT DEFAULT 0.0,
                execution_candle_index INTEGER,
                gross_amount FLOAT NOT NULL,
                net_amount FLOAT NOT NULL,
                created_at DATETIME
            )
        """))
        conn.commit()
    engine.dispose()

    cfg = _build_alembic_config(db_url)

    with pytest.raises(RuntimeError) as exc_info:
        command.upgrade(cfg, "head")

    err_msg = str(exc_info.value)
    assert "missing required historical columns" in err_msg
    assert "executions: ['trade_id']" in err_msg


def test_partial_schema_missing_replay_session_source_type_column_fails_closed(tmp_path):
    """Scenario 5c: 'replay_sessions' lacks 'source_type' -> fails closed."""
    db_path = (tmp_path / "missing_source_type.db").resolve()
    db_url = f"sqlite:///{db_path.as_posix()}"

    tables_without_sessions = [tbl for name, tbl in Base.metadata.tables.items() if name != "replay_sessions"]
    engine = create_engine(db_url)
    Base.metadata.create_all(bind=engine, tables=tables_without_sessions)

    with engine.connect() as conn:
        conn.execute(text("""
            CREATE TABLE replay_sessions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                symbol VARCHAR NOT NULL,
                timeframe VARCHAR DEFAULT '1D',
                adjustment_type VARCHAR DEFAULT 'unadjusted',
                start_date DATE NOT NULL,
                end_date DATE NOT NULL,
                current_index INTEGER DEFAULT 0,
                initial_cash FLOAT DEFAULT 100000000.0,
                current_cash FLOAT DEFAULT 100000000.0,
                status VARCHAR DEFAULT 'created',
                mode VARCHAR DEFAULT 'normal',
                hide_symbol INTEGER DEFAULT 0,
                hide_date INTEGER DEFAULT 0,
                created_at DATETIME,
                updated_at DATETIME
            )
        """))
        conn.commit()
    engine.dispose()

    cfg = _build_alembic_config(db_url)

    with pytest.raises(RuntimeError) as exc_info:
        command.upgrade(cfg, "head")

    err_msg = str(exc_info.value)
    assert "missing required historical columns" in err_msg
    assert "replay_sessions: ['source_type']" in err_msg


def test_partial_schema_missing_decisions_stop_loss_column_fails_closed(tmp_path):
    """Scenario 5d: 'decisions' lacks 'stop_loss' -> fails closed."""
    db_path = (tmp_path / "missing_stop_loss.db").resolve()
    db_url = f"sqlite:///{db_path.as_posix()}"

    tables_without_decisions = [tbl for name, tbl in Base.metadata.tables.items() if name != "decisions"]
    engine = create_engine(db_url)
    Base.metadata.create_all(bind=engine, tables=tables_without_decisions)

    with engine.connect() as conn:
        conn.execute(text("""
            CREATE TABLE decisions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id INTEGER NOT NULL,
                symbol VARCHAR NOT NULL,
                decision_date DATETIME NOT NULL,
                candle_index INTEGER NOT NULL,
                action VARCHAR NOT NULL,
                price FLOAT,
                created_at DATETIME
            )
        """))
        conn.commit()
    engine.dispose()

    cfg = _build_alembic_config(db_url)

    with pytest.raises(RuntimeError) as exc_info:
        command.upgrade(cfg, "head")

    err_msg = str(exc_info.value)
    assert "missing required historical columns" in err_msg
    assert "decisions: ['stop_loss']" in err_msg


def test_partial_schema_missing_trades_planned_entry_price_column_fails_closed(tmp_path):
    """Scenario 5e: 'trades' lacks 'planned_entry_price' -> fails closed."""
    db_path = (tmp_path / "missing_planned_entry_price.db").resolve()
    db_url = f"sqlite:///{db_path.as_posix()}"

    tables_without_trades = [tbl for name, tbl in Base.metadata.tables.items() if name != "trades"]
    engine = create_engine(db_url)
    Base.metadata.create_all(bind=engine, tables=tables_without_trades)

    with engine.connect() as conn:
        conn.execute(text("""
            CREATE TABLE trades (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id INTEGER NOT NULL,
                symbol VARCHAR NOT NULL,
                entry_date DATETIME NOT NULL,
                entry_price FLOAT NOT NULL,
                quantity FLOAT NOT NULL,
                created_at DATETIME,
                updated_at DATETIME
            )
        """))
        conn.commit()
    engine.dispose()

    cfg = _build_alembic_config(db_url)

    with pytest.raises(RuntimeError) as exc_info:
        command.upgrade(cfg, "head")

    err_msg = str(exc_info.value)
    assert "missing required historical columns" in err_msg
    assert "trades: ['planned_entry_price']" in err_msg


def test_idempotency_and_preservation_of_applied_migrations(tmp_path):
    """Scenario 6: Already-migrated database is not overwritten; direct stamping is idempotent."""
    db_path = (tmp_path / "idempotency.db").resolve()
    db_url = f"sqlite:///{db_path.as_posix()}"

    engine = create_engine(db_url)
    Base.metadata.create_all(bind=engine)

    with engine.connect() as conn:
        # Pre-create alembic_version pointing to an earlier revision 'cdf80254e9dc'
        conn.execute(text(
            "CREATE TABLE alembic_version (version_num VARCHAR(32) NOT NULL, "
            "CONSTRAINT alembic_version_pkc PRIMARY KEY (version_num))"
        ))
        conn.execute(text("INSERT INTO alembic_version (version_num) VALUES ('cdf80254e9dc')"))
        conn.commit()

        # Direct call to _ensure_precreated_db_stamped must NOT overwrite 'cdf80254e9dc'
        _ensure_precreated_db_stamped(conn)

        ver = conn.execute(text("SELECT version_num FROM alembic_version")).scalar()
        assert ver == "cdf80254e9dc", f"Existing migration version was overwritten! Expected cdf80254e9dc, got {ver}"
    engine.dispose()

    # Second check: Clean stamping on a database without alembic_version
    db_path_2 = (tmp_path / "idempotency_2.db").resolve()
    db_url_2 = f"sqlite:///{db_path_2.as_posix()}"
    engine_2 = create_engine(db_url_2)
    Base.metadata.create_all(bind=engine_2)

    with engine_2.connect() as conn:
        _ensure_precreated_db_stamped(conn)
        ver_1 = conn.execute(text("SELECT version_num FROM alembic_version")).scalar()
        assert ver_1 == "a2d639ecd5d5"

        # Calling again should be a clean no-op
        _ensure_precreated_db_stamped(conn)
        ver_2 = conn.execute(text("SELECT version_num FROM alembic_version")).scalar()
        assert ver_2 == "a2d639ecd5d5"
    engine_2.dispose()


def test_backend_sumi_db_invariance():
    """Verify that backend/sumi.db exists and its SHA256 checksum is bit-for-bit unchanged."""
    assert SUMI_DB_PATH.exists(), f"sumi.db not found at {SUMI_DB_PATH}"
    content = SUMI_DB_PATH.read_bytes()
    computed_hash = hashlib.sha256(content).hexdigest()
    assert computed_hash == EXPECTED_SUMI_DB_SHA256, (
        f"INVARIANT VIOLATION: backend/sumi.db checksum changed! "
        f"Expected {EXPECTED_SUMI_DB_SHA256}, got {computed_hash}"
    )
