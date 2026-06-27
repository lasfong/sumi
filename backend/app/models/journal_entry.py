from sqlalchemy import Column, String, Integer, DateTime, ForeignKey
from sqlalchemy.sql import func
from app.db import Base

class JournalEntry(Base):
    __tablename__ = "journal_entries"

    id = Column(Integer, primary_key=True, autoincrement=True)
    session_id = Column(Integer, ForeignKey("replay_sessions.id"), nullable=False)
    decision_id = Column(Integer, ForeignKey("decisions.id"), nullable=True)
    trade_id = Column(Integer, ForeignKey("trades.id"), nullable=True)
    
    note_type = Column(String, nullable=False) # e.g. decision, trade_review, session_review
    content = Column(String, nullable=False)
    tags = Column(String, nullable=True) # comma separated
    
    created_at = Column(DateTime(timezone=True), default=func.now())
    updated_at = Column(DateTime(timezone=True), default=func.now(), onupdate=func.now())
