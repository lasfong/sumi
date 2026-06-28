from pydantic import BaseModel
from typing import Optional, List

class SetupPerformance(BaseModel):
    setup_type: str
    trades: int
    win_rate: float
    net_pnl: float

class GroupPerformance(BaseModel):
    key: str
    trades: int
    win_rate: float
    net_pnl: float
    average_pnl: float
    best_trade: Optional[float] = None
    worst_trade: Optional[float] = None

class OutlierImpact(BaseModel):
    top_winners_pnl: float
    top_losers_pnl: float
    top_winners_share: Optional[float] = None
    top_losers_share: Optional[float] = None
    median_trade_pnl: Optional[float] = None
    trimmed_expectancy: Optional[float] = None

class EquityPoint(BaseModel):
    timestamp: str
    equity: float
    cash: float
    holdings_value: float
    drawdown: float
    drawdown_pct: float

class DrawdownPeriod(BaseModel):
    start: str
    end: str
    max_drawdown_pct: float

class BenchmarkPoint(BaseModel):
    time: str
    value: float

class TradeDistribution(BaseModel):
    trade_id: int
    symbol: str
    net_pnl: float
    pnl_percent: float
    r_multiple: Optional[float]
    result: str

class AnalyticsResponse(BaseModel):
    total_trades: int
    win_rate: float
    total_net_pnl: float
    average_win: float
    average_loss: float
    profit_factor: Optional[float] = None
    average_r: Optional[float] = None
    expectancy: Optional[float] = None
    largest_win: Optional[float] = None
    largest_loss: Optional[float] = None
    max_drawdown: Optional[float] = None
    max_drawdown_pct: Optional[float] = None
    sharpe_ratio: Optional[float] = None
    sortino_ratio: Optional[float] = None
    sqn: Optional[float] = None
    setup_performance: Optional[List[SetupPerformance]] = None
    equity_curve: Optional[List[EquityPoint]] = None
    drawdown_periods: Optional[List[DrawdownPeriod]] = None
    benchmark_curve: Optional[List[BenchmarkPoint]] = None
    trade_distribution: Optional[List[TradeDistribution]] = None
    symbol_performance: Optional[List[GroupPerformance]] = None
    mistake_performance: Optional[List[GroupPerformance]] = None
    outlier_impact: Optional[OutlierImpact] = None
