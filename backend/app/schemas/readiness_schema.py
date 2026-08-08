from pydantic import BaseModel
from typing import List, Optional


class DataReadinessResponse(BaseModel):
    status: str  # 'ready' | 'empty' | 'partial'
    symbols_count: int
    symbols_with_candles: List[str]
    timeframes: List[str]
    total_candles: int
    earliest_timestamp: Optional[str] = None
    latest_timestamp: Optional[str] = None
