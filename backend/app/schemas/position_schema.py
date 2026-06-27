from pydantic import BaseModel, ConfigDict
from datetime import datetime
from typing import Optional
from app.domain.enums import PositionStatus

class PositionResponse(BaseModel):
    id: int
    session_id: int
    symbol: str
    quantity: float
    average_price: float
    total_cost: float
    realized_pnl: float
    unrealized_pnl: float
    status: PositionStatus
    opened_at: datetime
    closed_at: Optional[datetime] = None
    model_config = ConfigDict(from_attributes=True)
