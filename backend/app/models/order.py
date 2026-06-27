from sqlalchemy import Column, String, Float, Integer, DateTime, ForeignKey
from sqlalchemy.sql import func
from app.db import Base

class Order(Base):
    __tablename__ = "orders"

    id = Column(Integer, primary_key=True, autoincrement=True)
    session_id = Column(Integer, ForeignKey("replay_sessions.id"), nullable=False)
    decision_id = Column(Integer, ForeignKey("decisions.id"), nullable=False)
    symbol = Column(String, nullable=False)
    side = Column(String, nullable=False) # BUY, SELL
    order_type = Column(String, nullable=False) # MARKET_AT_CLOSE, MARKET_NEXT_OPEN, LIMIT
    requested_price = Column(Float, nullable=True)
    quantity = Column(Float, nullable=False)
    capital_percent = Column(Float, nullable=True)
    status = Column(String, default="created") # created, executed, cancelled, rejected
    
    created_at = Column(DateTime(timezone=True), default=func.now())
