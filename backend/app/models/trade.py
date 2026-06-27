from sqlalchemy import Column, String, Float, Integer, DateTime, ForeignKey
from sqlalchemy.sql import func
from app.db import Base

class Trade(Base):
    __tablename__ = "trades"

    id = Column(Integer, primary_key=True, autoincrement=True)
    session_id = Column(Integer, ForeignKey("replay_sessions.id"), nullable=False)
    symbol = Column(String, nullable=False)
    
    entry_date = Column(DateTime(timezone=True), nullable=False)
    entry_price = Column(Float, nullable=False)
    exit_date = Column(DateTime(timezone=True), nullable=True)
    exit_price = Column(Float, nullable=True)
    quantity = Column(Float, nullable=False)
    
    gross_pnl = Column(Float, nullable=True)
    net_pnl = Column(Float, nullable=True)
    pnl_percent = Column(Float, nullable=True)
    
    initial_stop_loss = Column(Float, nullable=True)
    target_price = Column(Float, nullable=True)
    initial_risk = Column(Float, nullable=True)
    r_multiple = Column(Float, nullable=True)
    
    holding_candles = Column(Integer, nullable=True)
    holding_days = Column(Integer, nullable=True)
    
    status = Column(String, default='open')
    result = Column(String, default='open')
    setup_type = Column(String, nullable=True)
    mistake_tag = Column(String, nullable=True)
    
    created_at = Column(DateTime(timezone=True), default=func.now())
    updated_at = Column(DateTime(timezone=True), default=func.now(), onupdate=func.now())
