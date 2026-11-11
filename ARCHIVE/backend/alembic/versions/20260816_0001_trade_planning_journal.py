"""trade_planning_and_journal

Revision ID: 20260816_0001
Revises: 20260810_0001
Create Date: 2026-08-16

"""
from typing import Sequence, Union
from alembic import op
import sqlalchemy as sa

revision: str = '20260816_0001'
down_revision: Union[str, Sequence[str], None] = '20260810_0001'
branch_labels: Union[str, Sequence[str], None] = None
depends_on: Union[str, Sequence[str], None] = None


def _has_column(table_name: str, column_name: str) -> bool:
    bind = op.get_bind()
    inspector = sa.inspect(bind)
    columns = [col['name'] for col in inspector.get_columns(table_name)]
    return column_name in columns


def upgrade() -> None:
    # 1. Decisions table
    decision_cols = [
        ('stop_loss', sa.Float(), True),
        ('target_price', sa.Float(), True),
        ('planned_quantity', sa.Float(), True),
        ('planned_risk', sa.Float(), True),
        ('planned_r', sa.Float(), True),
        ('market_regime', sa.String(), True),
        ('emotion', sa.String(), True),
        ('rule_violation', sa.String(), True),
        ('checklist_snapshot', sa.String(), True),
    ]
    for col_name, col_type, nullable in decision_cols:
        if not _has_column('decisions', col_name):
            op.add_column('decisions', sa.Column(col_name, col_type, nullable=nullable))

    # 2. Trades table
    trade_cols = [
        ('planned_entry_price', sa.Float(), True),
        ('planned_quantity', sa.Float(), True),
        ('planned_r', sa.Float(), True),
        ('market_regime', sa.String(), True),
        ('emotion', sa.String(), True),
        ('rule_violation', sa.String(), True),
        ('notes', sa.String(), True),
    ]
    for col_name, col_type, nullable in trade_cols:
        if not _has_column('trades', col_name):
            op.add_column('trades', sa.Column(col_name, col_type, nullable=nullable))

    # 3. Journal entries table
    journal_cols = [
        ('setup_type', sa.String(), True),
        ('market_regime', sa.String(), True),
        ('confidence_score', sa.Integer(), True),
        ('emotion', sa.String(), True),
        ('mistake_tag', sa.String(), True),
        ('rule_violation', sa.String(), True),
        ('checklist_snapshot', sa.JSON(), True),
    ]
    for col_name, col_type, nullable in journal_cols:
        if not _has_column('journal_entries', col_name):
            op.add_column('journal_entries', sa.Column(col_name, col_type, nullable=nullable))


def downgrade() -> None:
    # In SQLite, drop column is not supported in older SQLite versions, but with batch mode we can handle or pass
    with op.batch_alter_table('decisions') as batch_op:
        for col_name in ['stop_loss', 'target_price', 'planned_quantity', 'planned_risk', 'planned_r', 'market_regime', 'emotion', 'rule_violation', 'checklist_snapshot']:
            if _has_column('decisions', col_name):
                batch_op.drop_column(col_name)

    with op.batch_alter_table('trades') as batch_op:
        for col_name in ['planned_entry_price', 'planned_quantity', 'planned_r', 'market_regime', 'emotion', 'rule_violation', 'notes']:
            if _has_column('trades', col_name):
                batch_op.drop_column(col_name)

    with op.batch_alter_table('journal_entries') as batch_op:
        for col_name in ['setup_type', 'market_regime', 'confidence_score', 'emotion', 'mistake_tag', 'rule_violation', 'checklist_snapshot']:
            if _has_column('journal_entries', col_name):
                batch_op.drop_column(col_name)
