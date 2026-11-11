"""Domain models for in-memory pure backtest execution."""

from dataclasses import dataclass, field
from datetime import datetime
from typing import Any, Dict, List, Optional


@dataclass
class SignalEvent:
    """Represents a strategy signal generated on candle bar T close."""
    bar_index: int
    timestamp: datetime
    symbol: str
    action: str  # BUY, CLOSE, SELL
    price: float  # Close price on signal bar T
    rule_name: Optional[str] = None
    reason: Optional[str] = None


@dataclass
class BacktestOrder:
    """Represents an order placed following a signal."""
    id: str
    symbol: str
    side: str  # BUY, SELL
    order_type: str  # MARKET_NEXT_OPEN, STOP_LOSS, TAKE_PROFIT
    quantity: float
    signal_bar_index: int
    signal_timestamp: datetime
    order_bar_index: int
    order_timestamp: datetime
    requested_price: Optional[float] = None
    status: str = "QUEUED"  # QUEUED, FILLED, REJECTED, UNFILLED_AT_CUTOFF
    rejection_reason: Optional[str] = None


@dataclass
class BacktestExecution:
    """Represents the execution (fill) of an order on bar T+1 open."""
    id: str
    order_id: str
    symbol: str
    side: str  # BUY, SELL
    quantity: float
    price: float  # Open price on fill bar T+1
    gross_amount: float
    fee: float
    tax: float
    net_amount: float
    signal_timestamp: datetime
    signal_bar_index: int
    order_timestamp: datetime
    order_bar_index: int
    fill_timestamp: datetime
    fill_bar_index: int


@dataclass
class BacktestPosition:
    """Tracks current active holding for a symbol."""
    symbol: str
    quantity: float = 0.0
    average_price: float = 0.0
    current_price: float = 0.0
    unrealized_pnl: float = 0.0
    realized_pnl: float = 0.0
    cost_basis: float = 0.0
    last_buy_bar_index: Optional[int] = None
    last_buy_timestamp: Optional[datetime] = None


@dataclass
class BacktestTrade:
    """Represents a round-trip completed or currently open trade."""
    id: str
    symbol: str
    entry_order_id: str
    exit_order_id: Optional[str] = None
    entry_time: datetime = field(default_factory=datetime.utcnow)
    exit_time: Optional[datetime] = None
    entry_bar_index: int = 0
    exit_bar_index: Optional[int] = None
    quantity: float = 0.0
    entry_price: float = 0.0
    exit_price: Optional[float] = None
    gross_pnl: float = 0.0
    net_pnl: float = 0.0
    fees: float = 0.0
    taxes: float = 0.0
    pnl_percent: float = 0.0
    holding_bars: int = 0
    status: str = "open"  # open, closed
    result: str = "open"  # win, loss, breakeven, open
    exit_reason: Optional[str] = None


@dataclass
class BacktestLedger:
    """In-memory accounting ledger for backtest simulation."""
    initial_cash: float
    current_cash: float
    portfolio_value: float
    peak_portfolio_value: float
    max_drawdown: float = 0.0
    orders: List[BacktestOrder] = field(default_factory=list)
    executions: List[BacktestExecution] = field(default_factory=list)
    trades: List[BacktestTrade] = field(default_factory=list)
    equity_curve: List[Dict[str, Any]] = field(default_factory=list)
    unexecuted_signals: List[Dict[str, Any]] = field(default_factory=list)
    warnings: List[str] = field(default_factory=list)


@dataclass
class BacktestKernelResult:
    """Result of pure in-memory backtest execution."""
    ledger: BacktestLedger
    symbol: str
    timeframe: str
    start_date: datetime
    end_date: datetime
    total_candles: int
    last_analyzed_index: int
    execution_timing: str = "daily signal generated on bar T close, executed at bar T+1 open (no same-bar close fills)"
    price_basis: str = "OHLC close"
    open_position: Optional[BacktestPosition] = None
    execution_profile: Optional[str] = None
    market_rule_version: Optional[str] = None
    calendar_version: Optional[str] = None
    has_unsellable_open_position: bool = False

    @property
    def final_cash(self) -> float:
        return self.ledger.current_cash

    @property
    def final_equity(self) -> float:
        return self.ledger.portfolio_value

    @property
    def final_position(self) -> BacktestPosition:
        if self.open_position is not None:
            return self.open_position
        return BacktestPosition(symbol=self.symbol)

    @property
    def trades(self) -> List[BacktestTrade]:
        return self.ledger.trades

    @property
    def warnings(self) -> List[str]:
        return self.ledger.warnings

