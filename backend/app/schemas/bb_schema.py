"""Pydantic schemas for Money Flow Blackbox (BB) API."""

from typing import Dict, List, Optional
from pydantic import BaseModel, Field


class BBSymbolHorizonPointResponse(BaseModel):
    horizon: str
    bb_value: Optional[float] = None
    oib_raw: Optional[float] = None
    raw_numerator: Optional[float] = None
    raw_denominator: Optional[float] = None
    is_warmup: bool
    direction: str
    regime: str
    regime_run_length: int
    value_source: str
    quality: str


class BBSymbolPointResponse(BaseModel):
    date: str
    horizons: Dict[str, BBSymbolHorizonPointResponse]
    daily_pressure: float
    daily_trading_value: float
    daily_value_source: str
    daily_quality: str


class BBSymbolSeriesResponse(BaseModel):
    symbol: str
    flow_method: str = "OHLCV_PROXY"
    methodology_version: str = "bb_v1_ohlcv_proxy"
    as_of: Optional[str] = None
    requested_horizons: List[str] = Field(default_factory=list)
    total_bars: int
    coverage_ratio: float
    points: List[BBSymbolPointResponse] = Field(default_factory=list)


class BBCalculateRequest(BaseModel):
    symbol: str
    as_of: Optional[str] = None
    horizons: Optional[List[str]] = None
    limit: Optional[int] = 500


class MarketBBHorizonPointResponse(BaseModel):
    horizon: str
    market_bb_value: Optional[float] = None
    market_buy_value: float
    market_sell_value: float
    market_total_value: float
    market_oib: Optional[float] = None
    is_warmup: bool
    direction: str
    regime: str
    quality: str


class FlowBreadthHorizonResponse(BaseModel):
    horizon: str
    positive_count: int
    neutral_count: int
    negative_count: int
    total_count: int
    positive_count_ratio: float
    neutral_count_ratio: float
    negative_count_ratio: float
    positive_value: float
    neutral_value: float
    negative_value: float
    total_value: float
    positive_value_ratio: float
    neutral_value_ratio: float
    negative_value_ratio: float
    buffer: float = 0.0


class MarketBBSessionPointResponse(BaseModel):
    date: str
    horizons: Dict[str, MarketBBHorizonPointResponse]
    breadth: Dict[str, FlowBreadthHorizonResponse]
    symbol_coverage: float
    value_coverage: float
    reporting_symbols_count: int
    total_target_count: int
    coverage_status: str


class MarketBBSeriesResponse(BaseModel):
    universe_id: str
    universe_version: str
    universe_mode: str
    is_point_in_time: bool
    publication_status: str
    flow_method: str = "OHLCV_PROXY"
    methodology_version: str = "bb_v1_market_aggregate"
    as_of: Optional[str] = None
    survivor_bias_caveat: Optional[str] = None
    gate_reasons: List[str] = Field(default_factory=list)
    total_sessions: int
    points: List[MarketBBSessionPointResponse] = Field(default_factory=list)


class MarketBBCalculateRequest(BaseModel):
    universe_id: str
    universe_mode: Optional[str] = None
    as_of: Optional[str] = None
    horizons: Optional[List[str]] = None
    breadth_buffer: Optional[float] = 0.0
    limit: Optional[int] = 500

