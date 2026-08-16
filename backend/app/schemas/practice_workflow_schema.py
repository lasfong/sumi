from datetime import datetime
from typing import Optional, Union

from pydantic import BaseModel


class PracticeDecision(BaseModel):
    id: int
    action: str
    candle_index: int
    decision_date: datetime
    price: Optional[float] = None
    quantity: Optional[float] = None
    stop_loss: Optional[float] = None
    target_price: Optional[float] = None
    planned_quantity: Optional[float] = None
    planned_risk: Optional[float] = None
    planned_r: Optional[float] = None
    setup_type: Optional[str] = None
    confidence_score: Optional[int] = None
    market_context: Optional[str] = None
    market_regime: Optional[str] = None
    emotion: Optional[str] = None
    reason: Optional[str] = None
    note: Optional[str] = None
    mistake_tag: Optional[str] = None
    rule_violation: Optional[str] = None
    checklist_snapshot: Optional[Union[dict, str]] = None


class PracticeOrder(BaseModel):
    id: int
    decision_id: int
    side: str
    order_type: str
    requested_price: Optional[float] = None
    quantity: float
    status: str
    decision_index: int
    explanation: str


class PracticeExecution(BaseModel):
    id: int
    order_id: int
    decision_id: int
    side: str
    execution_index: int
    execution_date: datetime
    execution_price: float
    quantity: float
    net_amount: float


class PracticePosition(BaseModel):
    id: int
    symbol: str
    quantity: float
    average_price: float
    total_cost: float
    current_price: float
    realized_pnl: float
    unrealized_pnl: float
    available_quantity: float
    blocked_quantity: float = 0.0
    earliest_release_date: Optional[datetime] = None
    opened_at: datetime


class PracticeTrade(BaseModel):
    id: int
    symbol: str
    entry_date: datetime
    entry_price: float
    quantity: float
    exit_date: Optional[datetime] = None
    exit_price: Optional[float] = None
    net_pnl: Optional[float] = None
    pnl_percent: Optional[float] = None
    initial_stop_loss: Optional[float] = None
    target_price: Optional[float] = None
    initial_risk: Optional[float] = None
    r_multiple: Optional[float] = None
    planned_entry_price: Optional[float] = None
    planned_quantity: Optional[float] = None
    planned_r: Optional[float] = None
    setup_type: Optional[str] = None
    market_regime: Optional[str] = None
    emotion: Optional[str] = None
    mistake_tag: Optional[str] = None
    rule_violation: Optional[str] = None
    entry_drift: Optional[float] = None
    size_variance: Optional[float] = None
    r_variance: Optional[float] = None
    notes: Optional[str] = None
    status: str
    result: str


class PracticeWorkflowSnapshot(BaseModel):
    session_id: int
    symbol: str
    current_index: int
    visible_bar: int
    total_bars: int
    current_date: datetime
    current_price: float
    current_volume: float
    initial_cash: float
    current_cash: float
    available_quantity: float
    blocked_quantity: float = 0.0
    earliest_release_date: Optional[datetime] = None
    latest_activity_index: int
    historical: bool
    can_trade: bool
    trade_block_reason: Optional[str] = None
    decisions: list[PracticeDecision]
    orders: list[PracticeOrder]
    executions: list[PracticeExecution]
    positions: list[PracticePosition]
    trades: list[PracticeTrade]
