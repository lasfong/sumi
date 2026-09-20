"""Pure domain backtest execution models and kernel.

Implements next-event causal execution (signal on Close[T] fills at Open[T+1])
with zero intermediate database side effects.
"""

from app.domain.backtest.models import (
    BacktestExecution,
    BacktestKernelResult,
    BacktestLedger,
    BacktestOrder,
    BacktestPosition,
    BacktestTrade,
    SignalEvent,
)
from app.domain.backtest.execution import BacktestExecutionKernel
from app.domain.backtest.persistence_adapter import BacktestPersistenceAdapter

__all__ = [
    "SignalEvent",
    "BacktestOrder",
    "BacktestExecution",
    "BacktestPosition",
    "BacktestTrade",
    "BacktestLedger",
    "BacktestKernelResult",
    "BacktestExecutionKernel",
    "BacktestPersistenceAdapter",
]
