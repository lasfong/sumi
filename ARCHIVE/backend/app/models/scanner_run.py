from sqlalchemy import Column, DateTime, Integer, String, Text
from sqlalchemy.sql import func

from app.db import Base


class ScannerRun(Base):
    __tablename__ = "scanner_runs"

    id = Column(Integer, primary_key=True, autoincrement=True)
    label = Column(String, nullable=False)
    status = Column(String, nullable=False)
    total_results = Column(Integer, default=0, nullable=False)
    request_config = Column(Text, nullable=False)
    result_payload = Column(Text, nullable=False)
    created_at = Column(DateTime(timezone=True), default=func.now(), nullable=False)
