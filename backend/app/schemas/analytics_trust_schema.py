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
    execution_timing: str = "daily signal generated on bar T close, executed at bar T+1 open (no same-bar close fills)"
    price_basis: str = "OHLC close"
    fees: Dict[str, float] = Field(default_factory=dict)
    taxes: Dict[str, float] = Field(default_factory=dict)
    slippage: Dict[str, Any] = Field(default_factory=lambda: {"model": "none", "rate": 0.0})
    liquidity: str = "no volume/liquidity constraint beyond available cash and position"
    position_sizing: Dict[str, Any] = Field(default_factory=dict)
    settlement: str = "Vietnam cash-equity T+2 sell settlement"
    execution_profile: str = "vietnam_default_conservative"
    market_rule_version: str = "VN_EQUITY_DAILY_V1"
    calendar_version: str = "VN_CALENDAR_2020_2026_V1"
    settlement_details: Dict[str, Any] = Field(default_factory=lambda: {
        "regime": "T+1.5_afternoon",
        "conservative_1d_daily": True,
        "sellable_session": "T+2 Close or T+3 Open",
    })
    price_bands: Dict[str, float] = Field(default_factory=lambda: {
        "HOSE": 0.07,
        "HNX": 0.10,
        "UPCOM": 0.15,
    })
    board_lot: int = 100
    locked_limit_policy: str = "conservative_reject"


class RunManifest(BaseModel):
    strategy_name: str
    strategy_version: str
    strategy_parameters: Dict[str, Any] = Field(default_factory=dict)
    data_identity: str
    assumptions_identity: str
    engine_version: str = "sumi-backtest-v1"
    run_timestamp: datetime
    input_hash: str
