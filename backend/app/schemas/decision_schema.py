from pydantic import BaseModel, ConfigDict
from datetime import datetime
from typing import Optional
from app.domain.enums import DecisionAction

class DecisionBase(BaseModel):
    action: DecisionAction
    price: Optional[float] = None
    quantity: Optional[float] = None
    order_type: Optional[str] = None  # MARKET_AT_CLOSE, CUSTOM_PRICE, etc.
    stop_loss: Optional[float] = None
    target_price: Optional[float] = None
    confidence_score: Optional[int] = None
    setup_type: Optional[str] = None
    market_context: Optional[str] = None
    reason: Optional[str] = None
    note: Optional[str] = None
    mistake_tag: Optional[str] = None

class DecisionCreate(DecisionBase):
    pass

class DecisionResponse(DecisionBase):
    id: int
    session_id: int
    symbol: str
    decision_date: datetime
    candle_index: int
    created_at: datetime
    model_config = ConfigDict(from_attributes=True)
