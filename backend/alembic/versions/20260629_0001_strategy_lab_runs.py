"""strategy_lab_runs

Revision ID: 20260629_0001
Revises: cdf80254e9dc
Create Date: 2026-06-29

"""
from typing import Sequence, Union

from alembic import op
import sqlalchemy as sa


revision: str = '20260629_0001'
down_revision: Union[str, Sequence[str], None] = 'cdf80254e9dc'
branch_labels: Union[str, Sequence[str], None] = None
depends_on: Union[str, Sequence[str], None] = None


def upgrade() -> None:
    op.create_table(
        'strategy_lab_runs',
        sa.Column('id', sa.Integer(), autoincrement=True, nullable=False),
        sa.Column('run_type', sa.String(), nullable=False),
        sa.Column('label', sa.String(), nullable=False),
        sa.Column('request_config', sa.Text(), nullable=False),
        sa.Column('result_payload', sa.Text(), nullable=False),
        sa.Column('metrics_payload', sa.Text(), nullable=True),
        sa.Column('created_at', sa.DateTime(timezone=True), nullable=False),
        sa.PrimaryKeyConstraint('id'),
    )


def downgrade() -> None:
    op.drop_table('strategy_lab_runs')
