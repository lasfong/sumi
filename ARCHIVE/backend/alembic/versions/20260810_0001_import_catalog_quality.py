"""import_catalog_quality

Revision ID: 20260810_0001
Revises: 20260629_0004
Create Date: 2026-08-10

"""
from typing import Sequence, Union
from alembic import op
import sqlalchemy as sa

revision: str = '20260810_0001'
down_revision: Union[str, Sequence[str], None] = '20260629_0004'
branch_labels: Union[str, Sequence[str], None] = None
depends_on: Union[str, Sequence[str], None] = None

def _has_table(table_name: str) -> bool:
    bind = op.get_bind()
    inspector = sa.inspect(bind)
    return table_name in inspector.get_table_names()

def upgrade() -> None:
    if not _has_table('import_runs'):
        op.create_table(
            'import_runs',
            sa.Column('id', sa.String(length=36), primary_key=True),
            sa.Column('created_at', sa.DateTime(timezone=True), nullable=False),
            sa.Column('file_name', sa.String(), nullable=False),
            sa.Column('file_sha256', sa.String(length=64), nullable=False),
            sa.Column('content_sha256', sa.String(length=64), nullable=False),
            sa.Column('parser_version', sa.String(), nullable=False, server_default='cafef_v1'),
            sa.Column('source_type', sa.String(), nullable=False, server_default='cafef'),
            sa.Column('timeframe', sa.String(), nullable=False, server_default='1D'),
            sa.Column('adjustment_type', sa.String(), nullable=False, server_default='unadjusted'),
            sa.Column('timezone', sa.String(), nullable=False, server_default='Asia/Ho_Chi_Minh'),
            sa.Column('status', sa.String(), nullable=False, server_default='previewed'),
            sa.Column('parsed_count', sa.Integer(), nullable=False, server_default='0'),
            sa.Column('rejected_count', sa.Integer(), nullable=False, server_default='0'),
            sa.Column('duplicate_count', sa.Integer(), nullable=False, server_default='0'),
            sa.Column('conflicting_count', sa.Integer(), nullable=False, server_default='0'),
            sa.Column('missing_count', sa.Integer(), nullable=False, server_default='0'),
            sa.Column('out_of_order_count', sa.Integer(), nullable=False, server_default='0'),
            sa.Column('accepted_count', sa.Integer(), nullable=False, server_default='0'),
            sa.Column('can_accept', sa.Boolean(), nullable=False, server_default=sa.text('0')),
            sa.Column('block_reason', sa.String(), nullable=True),
            sa.Column('accepted_at', sa.DateTime(timezone=True), nullable=True),
            sa.Column('rolled_back_at', sa.DateTime(timezone=True), nullable=True),
        )

    if not _has_table('import_run_items'):
        op.create_table(
            'import_run_items',
            sa.Column('id', sa.Integer(), primary_key=True, autoincrement=True),
            sa.Column('run_id', sa.String(length=36), sa.ForeignKey('import_runs.id'), nullable=False),
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
        op.create_index('ix_import_run_items_run_id', 'import_run_items', ['run_id'])
        op.create_index('ix_import_run_items_symbol', 'import_run_items', ['symbol'])

    if not _has_table('import_run_mutations'):
        op.create_table(
            'import_run_mutations',
            sa.Column('id', sa.Integer(), primary_key=True, autoincrement=True),
            sa.Column('run_id', sa.String(length=36), sa.ForeignKey('import_runs.id'), nullable=False),
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
        op.create_index('ix_import_run_mutations_run_id', 'import_run_mutations', ['run_id'])

    if not _has_table('weekly_candle_provenance'):
        op.create_table(
            'weekly_candle_provenance',
            sa.Column('id', sa.Integer(), primary_key=True, autoincrement=True),
            sa.Column('symbol', sa.String(), nullable=False),
            sa.Column('adjustment_type', sa.String(), nullable=False, server_default='unadjusted'),
            sa.Column('week_start_date', sa.String(), nullable=False),
            sa.Column('weekly_timestamp', sa.DateTime(timezone=True), nullable=False),
            sa.Column('rule_version', sa.String(), nullable=False, server_default='VN_TRADING_WEEK_V1'),
            sa.Column('daily_member_keys_json', sa.String(), nullable=False),
            sa.Column('source_run_ids_json', sa.String(), nullable=False),
            sa.UniqueConstraint('symbol', 'adjustment_type', 'week_start_date', name='uq_weekly_provenance')
        )
        op.create_index('ix_weekly_candle_provenance_symbol', 'weekly_candle_provenance', ['symbol'])

def downgrade() -> None:
    if _has_table('weekly_candle_provenance'):
        op.drop_table('weekly_candle_provenance')
    if _has_table('import_run_mutations'):
        op.drop_table('import_run_mutations')
    if _has_table('import_run_items'):
        op.drop_table('import_run_items')
    if _has_table('import_runs'):
        op.drop_table('import_runs')
