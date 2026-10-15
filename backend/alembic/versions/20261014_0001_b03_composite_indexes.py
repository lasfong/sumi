"""b03_composite_indexes

Revision ID: 20261014_0001
Revises: a2d639ecd5d5
Create Date: 2026-10-14 10:00:00.000000

Additive corrective migration adding missing composite and foreign-key indexes
(GAP-DB-06) across market candles, replay graph, and workflow item/mutation tables.
All operations are inspector-guarded and idempotent.
"""
from typing import Sequence, Union
from alembic import op
import sqlalchemy as sa

revision: str = '20261014_0001'
down_revision: Union[str, Sequence[str], None] = 'a2d639ecd5d5'
branch_labels: Union[str, Sequence[str], None] = None
depends_on: Union[str, Sequence[str], None] = None


def _has_table(table_name: str) -> bool:
    bind = op.get_bind()
    inspector = sa.inspect(bind)
    return table_name in inspector.get_table_names()


def _has_column(table_name: str, column_name: str) -> bool:
    bind = op.get_bind()
    inspector = sa.inspect(bind)
    if not _has_table(table_name):
        return False
    return column_name in {column["name"] for column in inspector.get_columns(table_name)}


def _has_index(table_name: str, index_name: str) -> bool:
    bind = op.get_bind()
    inspector = sa.inspect(bind)
    if not _has_table(table_name):
        return False
    return index_name in {idx["name"] for idx in inspector.get_indexes(table_name)}


INDEX_SPECS = [
    # 1. Candles composite index for window queries (GAP-DB-06)
    ("ix_candles_symbol_timeframe_timestamp", "candles", ["symbol", "timeframe", "timestamp"], False),

    # 2. Replay Decisions indexes
    ("ix_decisions_session_id", "decisions", ["session_id"], False),
    ("ix_decisions_session_candle", "decisions", ["session_id", "candle_index"], False),

    # 3. Replay Orders indexes
    ("ix_orders_session_id", "orders", ["session_id"], False),
    ("ix_orders_decision_id", "orders", ["decision_id"], False),

    # 4. Replay Executions indexes
    ("ix_executions_session_id", "executions", ["session_id"], False),
    ("ix_executions_order_id", "executions", ["order_id"], False),
    ("ix_executions_trade_id", "executions", ["trade_id"], False),
    ("ix_executions_session_candle", "executions", ["session_id", "execution_candle_index"], False),

    # 5. Replay Positions indexes
    ("ix_positions_session_id", "positions", ["session_id"], False),
    ("ix_positions_session_symbol", "positions", ["session_id", "symbol"], False),

    # 6. Replay Trades indexes
    ("ix_trades_session_id", "trades", ["session_id"], False),
    ("ix_trades_session_symbol", "trades", ["session_id", "symbol"], False),

    # 7. Replay Journal Entries indexes
    ("ix_journal_entries_session_id", "journal_entries", ["session_id"], False),
    ("ix_journal_entries_decision_id", "journal_entries", ["decision_id"], False),
    ("ix_journal_entries_trade_id", "journal_entries", ["trade_id"], False),

    # 8. Event Logs index
    ("ix_event_logs_session_id", "event_logs", ["session_id"], False),

    # 9. Import Workflow indexes
    ("ix_import_run_items_run_symbol", "import_run_items", ["run_id", "symbol"], False),
    ("ix_import_run_mutations_run_action", "import_run_mutations", ["run_id", "action"], False),

    # 10. Sync Workflow indexes
    ("ix_sync_run_items_sync_symbol", "sync_run_items", ["sync_id", "symbol"], False),
    ("ix_sync_run_mutations_sync_action", "sync_run_mutations", ["sync_id", "action"], False),
]


def upgrade() -> None:
    """Create missing composite and FK indexes idempotently."""
    for index_name, table_name, columns, unique in INDEX_SPECS:
        if _has_table(table_name) and not _has_index(table_name, index_name):
            # Verify all referenced columns exist before creating index
            if all(_has_column(table_name, col) for col in columns):
                op.create_index(index_name, table_name, columns, unique=unique)


def downgrade() -> None:
    """Drop composite and FK indexes idempotently."""
    for index_name, table_name, _, _ in reversed(INDEX_SPECS):
        if _has_table(table_name) and _has_index(table_name, index_name):
            op.drop_index(index_name, table_name=table_name)
