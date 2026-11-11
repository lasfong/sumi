"""replay_session_source

Revision ID: 20260629_0002
Revises: 20260629_0001
Create Date: 2026-06-29

"""
from typing import Sequence, Union

from alembic import op
import sqlalchemy as sa


revision: str = '20260629_0002'
down_revision: Union[str, Sequence[str], None] = '20260629_0001'
branch_labels: Union[str, Sequence[str], None] = None
depends_on: Union[str, Sequence[str], None] = None


def upgrade() -> None:
    op.add_column('replay_sessions', sa.Column('source_type', sa.String(), nullable=True))
    op.add_column('replay_sessions', sa.Column('source_payload', sa.Text(), nullable=True))


def downgrade() -> None:
    op.drop_column('replay_sessions', 'source_payload')
    op.drop_column('replay_sessions', 'source_type')
