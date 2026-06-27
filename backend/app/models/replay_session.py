from sqlalchemy import Column, String, Float, Integer, Date, DateTime
from sqlalchemy.sql import func
from app.db import Base

class ReplaySession(Base):
    __tablename__ = "replay_sessions"

    id = Column(Integer, primary_key=True, autoincrement=True)
    symbol = Column(String, nullable=False)
    timeframe = Column(String, default="1D")
    adjustment_type = Column(String, default="unadjusted")
    start_date = Column(Date, nullable=False)
    end_date = Column(Date, nullable=False)
    current_index = Column(Integer, default=0)
    initial_cash = Column(Float, default=100000000.0)
    current_cash = Column(Float, default=100000000.0)
    status = Column(String, default="created") # uses SessionStatus
    mode = Column(String, default="normal") # uses SessionMode
    hide_symbol = Column(Integer, default=0) # SQLite doesn't natively support bools consistently across frameworks sometimes, but boolean is fine, here integer for simplicity (0/1) or boolean
    hide_date = Column(Integer, default=0)
    
    created_at = Column(DateTime(timezone=True), default=func.now())
    updated_at = Column(DateTime(timezone=True), default=func.now(), onupdate=func.now())
