from pydantic import BaseModel, ConfigDict, model_validator
from datetime import datetime
from typing import Optional
from app.domain.enums import DecisionAction

class DecisionBase(BaseModel):
    action: DecisionAction
    price: Optional[float] = None
    quantity: Optional[float] = None
    order_type: str = "MARKET_AT_CLOSE"
    stop_loss: Optional[float] = None
    target_price: Optional[float] = None
    confidence_score: Optional[int] = None
    setup_type: Optional[str] = None
    market_context: Optional[str] = None
    reason: Optional[str] = None
    note: Optional[str] = None
    mistake_tag: Optional[str] = None

class DecisionCreate(DecisionBase):
    @model_validator(mode='after')
    def check_limit_order(self):
        if self.order_type not in ["MARKET_AT_CLOSE", "LIMIT"]:
            raise ValueError("order_type must be MARKET_AT_CLOSE or LIMIT")
        if self.order_type == "LIMIT" and self.price is None:
            raise ValueError("Limit order requires a price")
        return self

class DecisionResponse(DecisionBase):
    id: int
    session_id: int
    symbol: str
    decision_date: datetime
    candle_index: int
    created_at: datetime
    model_config = ConfigDict(from_attributes=True)
