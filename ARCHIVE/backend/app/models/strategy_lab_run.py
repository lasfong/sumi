from sqlalchemy import Column, DateTime, Integer, String, Text
from sqlalchemy.sql import func

from app.db import Base


class StrategyLabRun(Base):
    __tablename__ = "strategy_lab_runs"

    id = Column(Integer, primary_key=True, autoincrement=True)
    run_type = Column(String, nullable=False)
    label = Column(String, nullable=False)
    request_config = Column(Text, nullable=False)
    result_payload = Column(Text, nullable=False)
    metrics_payload = Column(Text, nullable=True)
    created_at = Column(DateTime(timezone=True), default=func.now(), nullable=False)
