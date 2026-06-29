"""execution_trade_id_drift

Revision ID: 20260629_0004
Revises: 20260629_0003
Create Date: 2026-06-29

"""
from typing import Sequence, Union

from alembic import op
import sqlalchemy as sa


revision: str = '20260629_0004'
down_revision: Union[str, Sequence[str], None] = '20260629_0003'
branch_labels: Union[str, Sequence[str], None] = None
depends_on: Union[str, Sequence[str], None] = None


def _has_column(table_name: str, column_name: str) -> bool:
    bind = op.get_bind()
    inspector = sa.inspect(bind)
    return column_name in {column["name"] for column in inspector.get_columns(table_name)}


def upgrade() -> None:
    if not _has_column('executions', 'trade_id'):
        op.add_column('executions', sa.Column('trade_id', sa.Integer(), nullable=True))


def downgrade() -> None:
    if _has_column('executions', 'trade_id'):
        op.drop_column('executions', 'trade_id')
