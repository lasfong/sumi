from sqlalchemy import Column, String, Integer, DateTime, ForeignKey, JSON
from sqlalchemy.sql import func
from app.db import Base

class JournalEntry(Base):
    __tablename__ = "journal_entries"

    id = Column(Integer, primary_key=True, autoincrement=True)
    session_id = Column(Integer, ForeignKey("replay_sessions.id"), nullable=False)
    decision_id = Column(Integer, ForeignKey("decisions.id"), nullable=True)
    trade_id = Column(Integer, ForeignKey("trades.id"), nullable=True)
    
    note_type = Column(String, nullable=False) # e.g. decision, trade_review, session_review, practice_checklist
    content = Column(String, nullable=False)
    tags = Column(String, nullable=True) # comma separated
    setup_type = Column(String, nullable=True)
    market_regime = Column(String, nullable=True)
    confidence_score = Column(Integer, nullable=True)
    emotion = Column(String, nullable=True)
    mistake_tag = Column(String, nullable=True)
    rule_violation = Column(String, nullable=True)
    checklist_snapshot = Column(JSON, nullable=True)
    
    created_at = Column(DateTime(timezone=True), default=func.now())
    updated_at = Column(DateTime(timezone=True), default=func.now(), onupdate=func.now())
