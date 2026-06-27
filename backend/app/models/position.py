from sqlalchemy import Column, String, Float, Integer, DateTime, ForeignKey
from app.db import Base

class Position(Base):
    __tablename__ = "positions"

    id = Column(Integer, primary_key=True, autoincrement=True)
    session_id = Column(Integer, ForeignKey("replay_sessions.id"), nullable=False)
    symbol = Column(String, nullable=False)
    quantity = Column(Float, nullable=False, default=0.0)
    average_price = Column(Float, nullable=False, default=0.0)
    total_cost = Column(Float, nullable=False, default=0.0)
    realized_pnl = Column(Float, nullable=False, default=0.0)
    unrealized_pnl = Column(Float, nullable=False, default=0.0)
    status = Column(String, default="open") # open, closed
    
    opened_at = Column(DateTime(timezone=True), nullable=False)
    closed_at = Column(DateTime(timezone=True), nullable=True)
