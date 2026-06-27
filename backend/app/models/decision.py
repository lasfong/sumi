from sqlalchemy import Column, String, Float, Integer, DateTime, ForeignKey
from sqlalchemy.sql import func
from app.db import Base

class Decision(Base):
    __tablename__ = "decisions"

    id = Column(Integer, primary_key=True, autoincrement=True)
    session_id = Column(Integer, ForeignKey("replay_sessions.id"), nullable=False)
    symbol = Column(String, nullable=False)
    decision_date = Column(DateTime(timezone=True), nullable=False)
    candle_index = Column(Integer, nullable=False)
    action = Column(String, nullable=False) # BUY, SELL, HOLD, etc.
    price = Column(Float, nullable=True)
    confidence_score = Column(Integer, nullable=True)
    setup_type = Column(String, nullable=True)
    market_context = Column(String, nullable=True)
    reason = Column(String, nullable=True)
    note = Column(String, nullable=True)
    mistake_tag = Column(String, nullable=True)
    
    created_at = Column(DateTime(timezone=True), default=func.now())
