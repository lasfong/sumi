"""scanner_runs

Revision ID: 20260629_0003
Revises: 20260629_0002
Create Date: 2026-06-29

"""
from typing import Sequence, Union

from alembic import op
import sqlalchemy as sa


revision: str = '20260629_0003'
down_revision: Union[str, Sequence[str], None] = '20260629_0002'
branch_labels: Union[str, Sequence[str], None] = None
depends_on: Union[str, Sequence[str], None] = None


def upgrade() -> None:
    op.create_table(
        'scanner_runs',
        sa.Column('id', sa.Integer(), autoincrement=True, nullable=False),
        sa.Column('label', sa.String(), nullable=False),
        sa.Column('status', sa.String(), nullable=False),
        sa.Column('total_results', sa.Integer(), nullable=False),
        sa.Column('request_config', sa.Text(), nullable=False),
        sa.Column('result_payload', sa.Text(), nullable=False),
        sa.Column('created_at', sa.DateTime(timezone=True), nullable=False),
        sa.PrimaryKeyConstraint('id'),
    )


def downgrade() -> None:
    op.drop_table('scanner_runs')
