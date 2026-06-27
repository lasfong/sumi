from pydantic import BaseModel, ConfigDict
from datetime import datetime
from typing import Optional

class TradeResponse(BaseModel):
    id: int
    session_id: int
    symbol: str
    entry_date: datetime
    entry_price: float
    exit_date: Optional[datetime] = None
    exit_price: Optional[float] = None
    quantity: float
    gross_pnl: Optional[float] = None
    net_pnl: Optional[float] = None
    pnl_percent: Optional[float] = None
    initial_stop_loss: Optional[float] = None
    target_price: Optional[float] = None
    initial_risk: Optional[float] = None
    r_multiple: Optional[float] = None
    holding_candles: Optional[int] = None
    holding_days: Optional[int] = None
    status: Optional[str] = None
    result: Optional[str] = None
    setup_type: Optional[str] = None
    mistake_tag: Optional[str] = None
    model_config = ConfigDict(from_attributes=True)
