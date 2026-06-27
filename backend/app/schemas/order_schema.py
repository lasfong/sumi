from pydantic import BaseModel, ConfigDict
from datetime import datetime
from typing import Optional
from app.domain.enums import OrderSide, OrderType, OrderStatus

class OrderBase(BaseModel):
    side: OrderSide
    order_type: OrderType
    requested_price: Optional[float] = None
    quantity: float
    capital_percent: Optional[float] = None

class OrderCreate(OrderBase):
    pass

class OrderResponse(OrderBase):
    id: int
    session_id: int
    decision_id: int
    symbol: str
    status: OrderStatus
    created_at: datetime
    model_config = ConfigDict(from_attributes=True)
