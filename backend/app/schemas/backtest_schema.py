"""Pydantic schemas for Backtest requests and batch phase evaluation."""

from typing import Any, Dict, List, Optional
from pydantic import BaseModel, Field, field_validator, model_validator


class PhaseDefinitionRequest(BaseModel):
    """Schema for defining an evaluation phase date range."""
    name: str = Field(..., min_length=1, description="Unique phase identifier name")
    start_date: str = Field(..., description="Start date (YYYY-MM-DD)")
    end_date: str = Field(..., description="End date (YYYY-MM-DD)")
    description: Optional[str] = Field(None, description="Optional phase notes")

    @model_validator(mode="after")
    def validate_dates(self) -> "PhaseDefinitionRequest":
        if self.start_date >= self.end_date:
            raise ValueError(f"Phase '{self.name}': start_date '{self.start_date}' must be strictly before end_date '{self.end_date}'")
        return self


class BacktestRequest(BaseModel):
    """Standard backtest request with optional symbols list and phases."""
    symbol: Optional[str] = None
    symbols: Optional[List[str]] = None
    start_date: Optional[str] = None
    end_date: Optional[str] = None
    phases: Optional[List[PhaseDefinitionRequest]] = None
    initial_cash: float = Field(100_000_000.0, ge=1_000_000.0)
    benchmark_symbol: Optional[str] = "VNINDEX"
    strategy: Dict[str, Any]
    execution_profile: Optional[str] = "vietnam_default_conservative"
    exchange: Optional[str] = "HOSE"

    @model_validator(mode="after")
    def validate_dates_or_phases(self) -> "BacktestRequest":
        if not self.phases and (not self.start_date or not self.end_date):
            raise ValueError("Either phases or both start_date and end_date must be provided")
        return self


class BatchBacktestRequest(BaseModel):
    """Explicit batch backtest request across multiple symbols and phases."""
    symbols: List[str] = Field(..., min_length=1, description="List of ticker symbols")
    phases: List[PhaseDefinitionRequest] = Field(..., min_length=1, description="List of evaluation phases")
    strategy: Dict[str, Any] = Field(..., description="Strategy configuration dictionary")
    initial_cash: float = Field(100_000_000.0, ge=1_000_000.0, description="Independent starting capital per run")
    execution_profile: Optional[str] = "vietnam_default_conservative"
    exchange: Optional[str] = "HOSE"
    benchmark_symbol: Optional[str] = "VNINDEX"
    use_cache: bool = Field(True, description="Enable domain feature caching for acceleration")

    @model_validator(mode="after")
    def validate_unique_phases(self) -> "BatchBacktestRequest":
        names = [p.name for p in self.phases]
        if len(names) != len(set(names)):
            raise ValueError("Duplicate phase names in batch request are prohibited")
        return self


class BenchmarkMetricRowResponse(BaseModel):
    """Schema for exact Master 9 benchmark metrics and auxiliary metrics."""
    ticker: str
    phase_name: Optional[str] = None
    start_date: Optional[str] = None
    end_date: Optional[str] = None
    initial_cash: float
    final_cash: float
    final_equity: float
    net_profit: float
    net_profit_pct: float
    num_trades: int
    avg_profit_loss_pct: Optional[float] = None
    avg_bars_held: Optional[float] = None
    win_rate_pct: Optional[float] = None
    win_avg_profit_pct: Optional[float] = None
    loss_avg_loss_pct: Optional[float] = None
    num_winners: int = 0
    num_losers: int = 0
    num_breakeven: int = 0
    profit_factor: Optional[float] = None
    max_drawdown: float = 0.0
    open_position_quantity: float = 0.0
    open_position_value: float = 0.0


class CrossPhaseDegradationResponse(BaseModel):
    """Schema for cross-phase comparison and performance degradation."""
    ticker: str
    base_phase_name: str
    target_phase_name: str
    net_profit_delta: float
    return_delta_pct: float
    win_rate_delta_pct: Optional[float] = None
    profit_ratio: Optional[float] = None
    degradation_pct: Optional[float] = None
    is_degraded: bool
    drawdown_delta: float


class PhaseMetricMatrixResponse(BaseModel):
    """Schema for 2D phase benchmark matrix and portfolio metrics."""
    phase_names: List[str]
    symbols: List[str]
    rows_by_symbol: Dict[str, Dict[str, BenchmarkMetricRowResponse]]
    portfolio_by_phase: Dict[str, BenchmarkMetricRowResponse]
    cross_phase_degradations: List[CrossPhaseDegradationResponse] = Field(default_factory=list)
    consistency_score: float = 100.0


class BatchPhaseResultResponse(BaseModel):
    """Schema for individual phase simulation output."""
    symbol: str
    phase_name: str
    start_date: str
    end_date: str
    status: str
    total_candles: int
    initial_cash: float
    final_cash: float
    final_equity: float
    net_pnl: float
    net_return_pct: float
    total_trades: int
    open_position_quantity: float
    open_position_value: float
    warnings: List[str] = Field(default_factory=list)
    error_message: Optional[str] = None
    benchmark_metrics: Optional[BenchmarkMetricRowResponse] = None


class BatchTimingMetricsResponse(BaseModel):
    """Timing and cache performance metadata."""
    total_duration_ms: float = 0.0
    feature_compute_ms: float = 0.0
    simulation_ms: float = 0.0
    cache_hits: int = 0
    cache_misses: int = 0


class BatchBacktestResponse(BaseModel):
    """Schema for aggregated batch backtest execution output."""
    status: str
    total_symbols: int
    total_phases: int
    total_runs: int
    feature_compute_count: int
    simulation_run_count: int
    phase_results: List[BatchPhaseResultResponse]
    summary: Dict[str, Any]
    metric_matrix: Optional[PhaseMetricMatrixResponse] = None
    cross_phase_degradations: Optional[List[CrossPhaseDegradationResponse]] = None
    markdown_table: Optional[str] = None
    csv_export: Optional[str] = None
    timing_metrics: Optional[BatchTimingMetricsResponse] = None
