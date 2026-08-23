"""data_sync_runs

Revision ID: 20260816_0002
Revises: 20260816_0001
Create Date: 2026-08-16

"""
from typing import Sequence, Union
from alembic import op
import sqlalchemy as sa

revision: str = '20260816_0002'
down_revision: Union[str, Sequence[str], None] = '20260816_0001'
branch_labels: Union[str, Sequence[str], None] = None
depends_on: Union[str, Sequence[str], None] = None

def _has_table(table_name: str) -> bool:
    bind = op.get_bind()
    inspector = sa.inspect(bind)
    return table_name in inspector.get_table_names()

def upgrade() -> None:
    if not _has_table('sync_runs'):
        op.create_table(
            'sync_runs',
            sa.Column('id', sa.String(length=36), primary_key=True),
            sa.Column('created_at', sa.DateTime(timezone=True), nullable=False),
            sa.Column('provider_id', sa.String(), nullable=False, server_default='ssi'),
            sa.Column('symbol', sa.String(), nullable=False),
            sa.Column('start_date', sa.String(), nullable=False),
            sa.Column('end_date', sa.String(), nullable=False),
            sa.Column('timeframe', sa.String(), nullable=False, server_default='1D'),
            sa.Column('adjustment_type', sa.String(), nullable=False, server_default='unadjusted'),
            sa.Column('status', sa.String(), nullable=False, server_default='previewed'),
            sa.Column('can_accept', sa.Boolean(), nullable=False, server_default=sa.text('0')),
            sa.Column('block_reason', sa.String(), nullable=True),
            sa.Column('parsed_count', sa.Integer(), nullable=False, server_default='0'),
            sa.Column('rejected_count', sa.Integer(), nullable=False, server_default='0'),
            sa.Column('duplicate_count', sa.Integer(), nullable=False, server_default='0'),
            sa.Column('conflicting_count', sa.Integer(), nullable=False, server_default='0'),
            sa.Column('missing_count', sa.Integer(), nullable=False, server_default='0'),
            sa.Column('out_of_order_count', sa.Integer(), nullable=False, server_default='0'),
            sa.Column('accepted_count', sa.Integer(), nullable=False, server_default='0'),
            sa.Column('content_sha256', sa.String(length=64), nullable=False),
            sa.Column('duration_ms', sa.Float(), nullable=False, server_default='0.0'),
            sa.Column('accepted_at', sa.DateTime(timezone=True), nullable=True),
            sa.Column('rolled_back_at', sa.DateTime(timezone=True), nullable=True),
            sa.Column('manifest_json', sa.JSON(), nullable=True),
        )
        op.create_index('ix_sync_runs_symbol', 'sync_runs', ['symbol'])

    if not _has_table('sync_run_items'):
        op.create_table(
            'sync_run_items',
            sa.Column('id', sa.Integer(), primary_key=True, autoincrement=True),
            sa.Column('sync_id', sa.String(length=36), sa.ForeignKey('sync_runs.id'), nullable=False),
            sa.Column('row_index', sa.Integer(), nullable=False),
            sa.Column('symbol', sa.String(), nullable=False),
            sa.Column('timeframe', sa.String(), nullable=False, server_default='1D'),
            sa.Column('timestamp', sa.DateTime(timezone=True), nullable=False),
            sa.Column('adjustment_type', sa.String(), nullable=False, server_default='unadjusted'),
            sa.Column('open', sa.Float(), nullable=True),
            sa.Column('high', sa.Float(), nullable=True),
            sa.Column('low', sa.Float(), nullable=True),
            sa.Column('close', sa.Float(), nullable=True),
            sa.Column('volume', sa.Float(), nullable=True),
            sa.Column('classification', sa.String(), nullable=False),
            sa.Column('reject_reason', sa.String(), nullable=True),
        )
        op.create_index('ix_sync_run_items_sync_id', 'sync_run_items', ['sync_id'])
        op.create_index('ix_sync_run_items_symbol', 'sync_run_items', ['symbol'])

    if not _has_table('sync_run_mutations'):
        op.create_table(
            'sync_run_mutations',
            sa.Column('id', sa.Integer(), primary_key=True, autoincrement=True),
            sa.Column('sync_id', sa.String(length=36), sa.ForeignKey('sync_runs.id'), nullable=False),
            sa.Column('action', sa.String(), nullable=False),
            sa.Column('symbol', sa.String(), nullable=False),
            sa.Column('timeframe', sa.String(), nullable=False),
            sa.Column('timestamp', sa.DateTime(timezone=True), nullable=False),
            sa.Column('adjustment_type', sa.String(), nullable=False),
            sa.Column('before_open', sa.Float(), nullable=True),
            sa.Column('before_high', sa.Float(), nullable=True),
            sa.Column('before_low', sa.Float(), nullable=True),
            sa.Column('before_close', sa.Float(), nullable=True),
            sa.Column('before_volume', sa.Float(), nullable=True),
            sa.Column('after_open', sa.Float(), nullable=True),
            sa.Column('after_high', sa.Float(), nullable=True),
            sa.Column('after_low', sa.Float(), nullable=True),
            sa.Column('after_close', sa.Float(), nullable=True),
            sa.Column('after_volume', sa.Float(), nullable=True),
        )
        op.create_index('ix_sync_run_mutations_sync_id', 'sync_run_mutations', ['sync_id'])

def downgrade() -> None:
    if _has_table('sync_run_mutations'):
        op.drop_table('sync_run_mutations')
    if _has_table('sync_run_items'):
        op.drop_table('sync_run_items')
    if _has_table('sync_runs'):
        op.drop_table('sync_runs')
