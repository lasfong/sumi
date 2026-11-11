import uuid
from datetime import datetime, timezone
from sqlalchemy import Column, String, Float, Integer, DateTime, Boolean, ForeignKey, UniqueConstraint, JSON
from app.db import Base

class SyncRun(Base):
    __tablename__ = "sync_runs"

    id = Column(String(36), primary_key=True, default=lambda: str(uuid.uuid4()))
    created_at = Column(DateTime(timezone=True), nullable=False, default=lambda: datetime.now(timezone.utc))
    provider_id = Column(String, nullable=False, default="ssi")
    symbol = Column(String, nullable=False, index=True)
    start_date = Column(String, nullable=False)
    end_date = Column(String, nullable=False)
    timeframe = Column(String, nullable=False, default="1D")
    adjustment_type = Column(String, nullable=False, default="unadjusted")
    status = Column(String, nullable=False, default="previewed")  # previewed, accepted, blocked, rolled_back, noop, failed
    
    can_accept = Column(Boolean, default=False, nullable=False)
    block_reason = Column(String, nullable=True)
    
    parsed_count = Column(Integer, default=0, nullable=False)
    rejected_count = Column(Integer, default=0, nullable=False)
    duplicate_count = Column(Integer, default=0, nullable=False)
    conflicting_count = Column(Integer, default=0, nullable=False)
    missing_count = Column(Integer, default=0, nullable=False)
    out_of_order_count = Column(Integer, default=0, nullable=False)
    accepted_count = Column(Integer, default=0, nullable=False)
    
    content_sha256 = Column(String(64), nullable=False)
    duration_ms = Column(Float, default=0.0, nullable=False)
    
    accepted_at = Column(DateTime(timezone=True), nullable=True)
    rolled_back_at = Column(DateTime(timezone=True), nullable=True)
    manifest_json = Column(JSON, nullable=True)


class SyncRunItem(Base):
    __tablename__ = "sync_run_items"

    id = Column(Integer, primary_key=True, autoincrement=True)
    sync_id = Column(String(36), ForeignKey("sync_runs.id"), nullable=False, index=True)
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


class SyncRunMutation(Base):
    __tablename__ = "sync_run_mutations"

    id = Column(Integer, primary_key=True, autoincrement=True)
    sync_id = Column(String(36), ForeignKey("sync_runs.id"), nullable=False, index=True)
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
