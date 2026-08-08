from enum import Enum
from pydantic import BaseModel, ConfigDict
from datetime import date, datetime
from typing import Literal, Optional
from app.domain.enums import SessionStatus, SessionMode


class ReplayIntent(str, Enum):
    BLIND_PRACTICE = "blind_practice"
    SIGNAL_REVIEW = "signal_review"


class ScannerSourcePayload(BaseModel):
    symbol: str
    signal_timestamp: datetime
    signal_type: str
    strategy: str
    price: float
    regime: str | None = None
    replay_intent: ReplayIntent = ReplayIntent.BLIND_PRACTICE
    lookback_days: int | None = None
    forward_days: int | None = None


class ReplaySourceSignal(BaseModel):
    timestamp: datetime
    type: str
    strategy: str
    price: float
    regime: str | None = None


class ReplaySourceContext(BaseModel):
    schema_version: Literal[1] = 1
    source_type: str | None = None
    replay_intent: ReplayIntent | None = None
    reveal_at_index: int | None = None
    revealed: bool = False
    signal: ReplaySourceSignal | None = None


class CreateSessionMode(str, Enum):
    NORMAL = "normal"
    RANDOM = "random"
    BLIND_SYMBOL = "blind_symbol"
    BLIND_DATE = "blind_date"


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
    source_type: Optional[str] = None
    source_payload: Optional[str] = None

class ReplaySessionCreate(ReplaySessionBase):
    mode: CreateSessionMode = CreateSessionMode.NORMAL

class ReplaySessionResponse(ReplaySessionBase):
    id: int
    current_index: int
    current_cash: float
    status: SessionStatus
    source_context: ReplaySourceContext
    created_at: datetime
    updated_at: datetime
    model_config = ConfigDict(from_attributes=True)
