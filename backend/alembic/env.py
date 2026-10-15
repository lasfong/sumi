from logging.config import fileConfig

from sqlalchemy import engine_from_config
from sqlalchemy import pool

from alembic import context
import sys
import os
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from app.config import settings, verify_database_path_security
from app.db import Base
# Import all 20 models to ensure they are registered with Base.metadata (GAP-DB-05)
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

# this is the Alembic Config object, which provides
# access to the values within the .ini file in use.
try:
    config = context.config
except AttributeError:
    config = None

# Interpret the config file for Python logging.
# This line sets up loggers basically.
if config is not None:
    if config.config_file_name is not None:
        fileConfig(config.config_file_name)

    target_url = config.get_main_option("sqlalchemy.url")
    if not target_url or target_url == "driver://user:pass@localhost/dbname":
        target_url = settings.DATABASE_URL
    verify_database_path_security(target_url)
    config.set_main_option("sqlalchemy.url", target_url)

# add your model's MetaData object here
# for 'autogenerate' support
target_metadata = Base.metadata

# other values from the config, defined by the needs of env.py,
# can be acquired:
# my_important_option = config.get_main_option("my_important_option")
# ... etc.


def run_migrations_offline() -> None:
    """Run migrations in 'offline' mode.

    This configures the context with just a URL
    and not an Engine, though an Engine is acceptable
    here as well.  By skipping the Engine creation
    we don't even need a DBAPI to be available.

    Calls to context.execute() here emit the given string to the
    script output.

    """
    url = config.get_main_option("sqlalchemy.url")
    context.configure(
        url=url,
        target_metadata=target_metadata,
        literal_binds=True,
        dialect_opts={"paramstyle": "named"},
    )

    with context.begin_transaction():
        context.run_migrations()


REQUIRED_HISTORICAL_TABLES = frozenset({
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

REQUIRED_HISTORICAL_COLUMNS = {
    "executions": {"execution_candle_index", "trade_id"},
    "replay_sessions": {"source_type"},
    "decisions": {"stop_loss"},
    "trades": {"planned_entry_price"},
}


def _ensure_precreated_db_stamped(connection) -> None:
    """Validate pre-created historical database schema before stamping revision 'a2d639ecd5d5'.

    Rules:
    - If database is fresh/empty: do nothing; Alembic runs full migration chain.
    - If alembic_version has existing rows: do nothing; Alembic is active.
    - If user tables exist but do not satisfy the complete 20-table historical schema
      and key columns for 'a2d639ecd5d5': FAIL CLOSED with an actionable diagnostic.
    - If all 20 tables and required columns exist: stamp 'a2d639ecd5d5' into alembic_version.
    """
    from sqlalchemy import inspect, text

    inspector = inspect(connection)
    all_tables = set(inspector.get_table_names())
    user_tables = {t for t in all_tables if not t.startswith("sqlite_") and t != "alembic_version"}

    # Fresh empty database: let migrations run from scratch
    if not user_tables:
        return

    # Check if alembic_version already has recorded migrations
    if "alembic_version" in all_tables:
        rows = connection.execute(text("SELECT version_num FROM alembic_version")).fetchall()
        if rows:
            return

    # Database has tables but no alembic_version rows -> Verify historical completeness
    missing_tables = REQUIRED_HISTORICAL_TABLES - user_tables
    if missing_tables:
        raise RuntimeError(
            f"Database schema is partially initialized but incomplete for stamping revision 'a2d639ecd5d5'. "
            f"Found {len(user_tables)} existing tables, but missing {len(missing_tables)} required historical tables: "
            f"{sorted(missing_tables)}. "
            f"Cannot safely stamp 'a2d639ecd5d5' on a partial schema. "
            f"Database must either be completely empty or contain the complete historical schema."
        )

    # Verify critical historical columns
    missing_columns_by_table = {}
    for table_name, req_cols in REQUIRED_HISTORICAL_COLUMNS.items():
        existing_cols = {col["name"] for col in inspector.get_columns(table_name)}
        missing_cols = req_cols - existing_cols
        if missing_cols:
            missing_columns_by_table[table_name] = sorted(missing_cols)

    if missing_columns_by_table:
        formatted_missing = "; ".join(
            f"{table}: {cols}" for table, cols in sorted(missing_columns_by_table.items())
        )
        raise RuntimeError(
            f"Database schema is missing required historical columns for revision 'a2d639ecd5d5': "
            f"{formatted_missing}. Cannot safely stamp 'a2d639ecd5d5' on an incomplete schema."
        )

    # Schema is complete: stamp baseline revision 'a2d639ecd5d5'
    if "alembic_version" not in all_tables:
        connection.execute(text(
            "CREATE TABLE alembic_version (version_num VARCHAR(32) NOT NULL, "
            "CONSTRAINT alembic_version_pkc PRIMARY KEY (version_num))"
        ))
    connection.execute(text("INSERT INTO alembic_version (version_num) VALUES ('a2d639ecd5d5')"))
    connection.commit()


def run_migrations_online() -> None:
    """Run migrations in 'online' mode.

    In this scenario we need to create an Engine
    and associate a connection with the context.

    """
    connectable = engine_from_config(
        config.get_section(config.config_ini_section, {}),
        prefix="sqlalchemy.",
        poolclass=pool.NullPool,
    )

    with connectable.connect() as connection:
        _ensure_precreated_db_stamped(connection)
        context.configure(
            connection=connection, target_metadata=target_metadata
        )

        with context.begin_transaction():
            context.run_migrations()
        connection.commit()


if config is not None and hasattr(context, "is_offline_mode"):
    if context.is_offline_mode():
        run_migrations_offline()
    else:
        run_migrations_online()
