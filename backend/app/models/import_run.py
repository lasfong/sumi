import uuid
from datetime import datetime, timezone
from sqlalchemy import Column, String, Float, Integer, DateTime, Boolean, ForeignKey, UniqueConstraint
from app.db import Base

class ImportRun(Base):
    __tablename__ = "import_runs"

    id = Column(String(36), primary_key=True, default=lambda: str(uuid.uuid4()))
    created_at = Column(DateTime(timezone=True), nullable=False, default=lambda: datetime.now(timezone.utc))
    file_name = Column(String, nullable=False)
    file_sha256 = Column(String(64), nullable=False)
    content_sha256 = Column(String(64), nullable=False)
    parser_version = Column(String, nullable=False, default="cafef_v1")
    source_type = Column(String, nullable=False, default="cafef")
    timeframe = Column(String, nullable=False, default="1D")
    adjustment_type = Column(String, nullable=False, default="unadjusted")
    timezone = Column(String, nullable=False, default="Asia/Ho_Chi_Minh")
    status = Column(String, nullable=False, default="previewed")
    
    parsed_count = Column(Integer, default=0, nullable=False)
    rejected_count = Column(Integer, default=0, nullable=False)
    duplicate_count = Column(Integer, default=0, nullable=False)
    conflicting_count = Column(Integer, default=0, nullable=False)
    missing_count = Column(Integer, default=0, nullable=False)
    out_of_order_count = Column(Integer, default=0, nullable=False)
    accepted_count = Column(Integer, default=0, nullable=False)
    
    can_accept = Column(Boolean, default=False, nullable=False)
    block_reason = Column(String, nullable=True)
    accepted_at = Column(DateTime(timezone=True), nullable=True)
    rolled_back_at = Column(DateTime(timezone=True), nullable=True)


class ImportRunItem(Base):
    __tablename__ = "import_run_items"

    id = Column(Integer, primary_key=True, autoincrement=True)
    run_id = Column(String(36), ForeignKey("import_runs.id"), nullable=False, index=True)
    row_index = Column(Integer, nullable=False)
    symbol = Column(String, nullable=False, index=True)
    timeframe = Column(String, default="1D", nullable=False)
    timestamp = Column(DateTime(timezone=True), nullable=False)
    adjustment_type = Column(String, default="unadjusted", nullable=False)
    open = Column(Float, nullable=True)
    high = Column(Float, nullable=True)
    low = Column(Float, nullable=True)
    close = Column(Float, nullable=True)
    volume = Column(Float, nullable=True)
    classification = Column(String, nullable=False)  # parsed, rejected, duplicate, conflicting, missing, out_of_order
    reject_reason = Column(String, nullable=True)


class ImportRunMutation(Base):
    __tablename__ = "import_run_mutations"

    id = Column(Integer, primary_key=True, autoincrement=True)
    run_id = Column(String(36), ForeignKey("import_runs.id"), nullable=False, index=True)
    action = Column(String, nullable=False)  # INSERT, UPDATE, DELETE
    symbol = Column(String, nullable=False)
    timeframe = Column(String, nullable=False)
    timestamp = Column(DateTime(timezone=True), nullable=False)
    adjustment_type = Column(String, nullable=False)
    before_open = Column(Float, nullable=True)
    before_high = Column(Float, nullable=True)
    before_low = Column(Float, nullable=True)
    before_close = Column(Float, nullable=True)
    before_volume = Column(Float, nullable=True)
    after_open = Column(Float, nullable=True)
    after_high = Column(Float, nullable=True)
    after_low = Column(Float, nullable=True)
    after_close = Column(Float, nullable=True)
    after_volume = Column(Float, nullable=True)


class WeeklyCandleProvenance(Base):
    __tablename__ = "weekly_candle_provenance"

    id = Column(Integer, primary_key=True, autoincrement=True)
    symbol = Column(String, nullable=False, index=True)
    adjustment_type = Column(String, default="unadjusted", nullable=False)
    week_start_date = Column(String, nullable=False)  # YYYY-MM-DD
    weekly_timestamp = Column(DateTime(timezone=True), nullable=False)
    rule_version = Column(String, default="VN_TRADING_WEEK_V1", nullable=False)
    daily_member_keys_json = Column(String, nullable=False)
    source_run_ids_json = Column(String, nullable=False)

    __table_args__ = (
        UniqueConstraint('symbol', 'adjustment_type', 'week_start_date', name='uq_weekly_provenance'),
    )
