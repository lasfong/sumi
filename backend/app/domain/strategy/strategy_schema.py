from pydantic import BaseModel
from typing import Optional, List, Dict, Any

class IndicatorConfig(BaseModel):
    name: str
    type: str  # sma, ema, rsi, etc.
    length: int

class PositionSizing(BaseModel):
    method: str = "fixed_quantity"
    quantity: Optional[int] = None
    percent: Optional[float] = None

class RiskManagement(BaseModel):
    stop_loss_pct: Optional[float] = None
    take_profit_pct: Optional[float] = None

class StrategyConfig(BaseModel):
    name: str
    version: str = "1.0"
    description: str = ""
    indicators: List[IndicatorConfig]
    entry_rules: List[Dict[str, Any]]
    exit_rules: List[Dict[str, Any]]
    position_sizing: PositionSizing
    risk_management: Optional[RiskManagement] = None
