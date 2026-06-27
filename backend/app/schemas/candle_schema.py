from pydantic import BaseModel, ConfigDict
from datetime import date
from typing import Optional

class CandleBase(BaseModel):
    symbol: str
    timeframe: str = "1D"
    timestamp: date
    open: float
    high: float
    low: float
    close: float
    volume: float
    source: Optional[str] = None
    adjustment_type: str = "unadjusted"

class CandleCreate(CandleBase):
    pass

class CandleResponse(CandleBase):
    id: int
    model_config = ConfigDict(from_attributes=True)
