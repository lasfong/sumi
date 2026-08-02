from datetime import datetime
from typing import Any, Dict, List, Literal, Optional

from pydantic import BaseModel, Field


MetricStatus = Literal["valid", "insufficient_data", "not_applicable"]


class MetricResult(BaseModel):
    value: Optional[float] = None
    status: MetricStatus
    sample_size: int = 0
    period: Optional[str] = None
    reason: Optional[str] = None


class DataCoverage(BaseModel):
    requested_start: str
    requested_end: str
    actual_start: Optional[str] = None
    actual_end: Optional[str] = None
    symbols_requested: List[str] = Field(default_factory=list)
    symbols_covered: List[str] = Field(default_factory=list)
    candle_count: int = 0
    warmup_candles: int = 0
    gaps: List[str] = Field(default_factory=list)
    excluded_data: List[str] = Field(default_factory=list)


class ExecutionAssumptions(BaseModel):
    execution_timing: str = "signal evaluated on close; market order executed at close"
    price_basis: str = "OHLC close"
    fees: Dict[str, float] = Field(default_factory=dict)
    taxes: Dict[str, float] = Field(default_factory=dict)
    slippage: Dict[str, Any] = Field(default_factory=lambda: {"model": "none", "rate": 0.0})
    liquidity: str = "no volume/liquidity constraint beyond available cash and position"
    position_sizing: Dict[str, Any] = Field(default_factory=dict)
    settlement: str = "Vietnam cash-equity T+2 sell settlement"


class RunManifest(BaseModel):
    strategy_name: str
    strategy_version: str
    strategy_parameters: Dict[str, Any] = Field(default_factory=dict)
    data_identity: str
    assumptions_identity: str
    engine_version: str = "sumi-backtest-v1"
    run_timestamp: datetime
    input_hash: str
