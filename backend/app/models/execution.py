from sqlalchemy import Column, String, Float, Integer, DateTime, ForeignKey
from sqlalchemy.sql import func
from app.db import Base

class Execution(Base):
    __tablename__ = "executions"

    id = Column(Integer, primary_key=True, autoincrement=True)
    order_id = Column(Integer, ForeignKey("orders.id"), nullable=False)
    trade_id = Column(Integer, ForeignKey("trades.id"), nullable=True)
    session_id = Column(Integer, ForeignKey("replay_sessions.id"), nullable=False)
    symbol = Column(String, nullable=False)
    execution_date = Column(DateTime(timezone=True), nullable=False)
    execution_price = Column(Float, nullable=False)
    quantity = Column(Float, nullable=False)
    fee = Column(Float, default=0.0)
    tax = Column(Float, default=0.0)
    slippage = Column(Float, default=0.0)
    gross_amount = Column(Float, nullable=False)
    net_amount = Column(Float, nullable=False)
    
    created_at = Column(DateTime(timezone=True), default=func.now())
