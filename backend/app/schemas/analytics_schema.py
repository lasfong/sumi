from pydantic import BaseModel
from typing import Optional, List

class SetupPerformance(BaseModel):
    setup_type: str
    trades: int
    win_rate: float
    net_pnl: float

class EquityPoint(BaseModel):
    timestamp: str
    equity: float
    drawdown: float

class AnalyticsResponse(BaseModel):
    total_trades: int
    win_rate: float
    total_net_pnl: float
    average_win: float
    average_loss: float
    profit_factor: float
    average_r: Optional[float] = None
    expectancy: Optional[float] = None
    largest_win: Optional[float] = None
    largest_loss: Optional[float] = None
    max_drawdown: Optional[float] = None
    sharpe_ratio: Optional[float] = None
    sortino_ratio: Optional[float] = None
    sqn: Optional[float] = None
    setup_performance: Optional[List[SetupPerformance]] = None
    equity_curve: Optional[List[EquityPoint]] = None
