from sqlalchemy import Column, Integer, String, Text, ForeignKey, DateTime
from sqlalchemy.sql import func
from app.db import Base

class DrawingState(Base):
    __tablename__ = "drawing_states"

    id = Column(Integer, primary_key=True, index=True)
    session_id = Column(Integer, ForeignKey("replay_sessions.id", ondelete="CASCADE"), nullable=False, index=True)
    symbol = Column(String(20), nullable=False, index=True)
    
    # JSON string containing the array of drawings: [{id, type, points, color}]
    state_data = Column(Text, nullable=False, default="[]")

    created_at = Column(DateTime(timezone=True), server_default=func.now())
    updated_at = Column(DateTime(timezone=True), server_default=func.now(), onupdate=func.now())
