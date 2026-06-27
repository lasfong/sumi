from sqlalchemy import Column, String, Integer, DateTime, ForeignKey
from sqlalchemy.sql import func
from app.db import Base

class EventLog(Base):
    __tablename__ = "event_logs"

    id = Column(Integer, primary_key=True, autoincrement=True)
    session_id = Column(Integer, ForeignKey("replay_sessions.id"), nullable=True)
    event_type = Column(String, nullable=False)
    message = Column(String, nullable=False)
    details = Column(String, nullable=True) # JSON string
    
    created_at = Column(DateTime(timezone=True), default=func.now())
