from pydantic import BaseModel, ConfigDict
from datetime import date, datetime
from typing import Optional
from app.domain.enums import SessionStatus, SessionMode

class ReplaySessionBase(BaseModel):
    symbol: str
    timeframe: str = "1D"
    adjustment_type: str = "unadjusted"
    start_date: date
    end_date: date
    initial_cash: float = 100000000.0
    mode: SessionMode = SessionMode.NORMAL
    hide_symbol: int = 0
    hide_date: int = 0

class ReplaySessionCreate(ReplaySessionBase):
    pass

class ReplaySessionResponse(ReplaySessionBase):
    id: int
    current_index: int
    current_cash: float
    status: SessionStatus
    created_at: datetime
    updated_at: datetime
    model_config = ConfigDict(from_attributes=True)
