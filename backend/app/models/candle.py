from sqlalchemy import Column, String, Float, Integer, DateTime, UniqueConstraint
from app.db import Base

class Candle(Base):
    __tablename__ = "candles"
    
    id = Column(Integer, primary_key=True, autoincrement=True)
    symbol = Column(String, index=True, nullable=False)
    timeframe = Column(String, default="1D", nullable=False)
    timestamp = Column(DateTime(timezone=True), index=True, nullable=False)
    open = Column(Float, nullable=False)
    high = Column(Float, nullable=False)
    low = Column(Float, nullable=False)
    close = Column(Float, nullable=False)
    volume = Column(Float, nullable=False)
    source = Column(String, nullable=True)
    adjustment_type = Column(String, default="unadjusted")

    __table_args__ = (
        UniqueConstraint('symbol', 'timeframe', 'timestamp', 'adjustment_type', name='uq_candle'),
    )
