"""Pydantic schemas for Signal Registry and Replay Signal Calculation."""

from typing import Any, Dict, List, Optional, Union
from pydantic import BaseModel, Field, field_validator, model_validator


class SignalDefinitionResponse(BaseModel):
    """Schema for a single registered signal definition."""
    name: str
    version: str
    category: str
    label_vi: str
    description: str
    output_type: str
    parameters_schema: Dict[str, Any]
    default_parameters: Dict[str, Any]
    dependencies: List[str]
    warmup_bars: int
    causal_delay_bars: int
    status: str
    ast_alias: str


class SignalRegistryResponse(BaseModel):
    """Schema for signal registry listing response."""
    signals: List[SignalDefinitionResponse]


class SignalCalculationItemRequest(BaseModel):
    """Request payload for a single signal calculation."""
    name: str = Field(..., description="Canonical signal registry name (e.g. volume.spike)")
    version: Optional[str] = Field(None, description="Signal version (e.g. 1.0.0)")
    params: Dict[str, Any] = Field(default_factory=dict, description="Configured parameter overrides")

    model_config = {"extra": "forbid"}

    @field_validator("params")
    @classmethod
    def validate_params_not_none(cls, v: Any) -> Dict[str, Any]:
        if v is None:
            raise ValueError("params cannot be null")
        return v


class SignalCalculationRequest(BaseModel):
    """Request payload for replay-scoped signal calculation."""
    signals: List[SignalCalculationItemRequest] = Field(
        ...,
        min_length=1,
        max_length=2,
        description="Explicit list of 1 to 2 signals to calculate",
    )

    model_config = {"extra": "forbid"}

    @model_validator(mode="after")
    def validate_unique_signals(self) -> "SignalCalculationRequest":
        names = [item.name for item in self.signals]
        if len(names) != len(set(names)):
            raise ValueError("Duplicate signal name in calculation request is prohibited")
        return self


class SignalOutputPointResponse(BaseModel):
    """Schema for a single calculated signal point in time."""
    bar_index: int
    timestamp: str
    output_type: str
    value: Optional[Union[bool, float, str]] = None
    quality: str
    reasons: List[str] = Field(default_factory=list)
    baseline: Optional[float] = None
    current_volume: Optional[float] = None
    relative_volume: Optional[float] = None
    threshold: Optional[float] = None
    availability_event: str
    available_at_index: int
    available_at_timestamp: str


class SignalCalculationSeriesResponse(BaseModel):
    """Schema for a calculated signal series result."""
    signal_name: str
    signal_version: str
    resolved_params: Dict[str, Any]
    params_hash: str
    points: List[SignalOutputPointResponse]


class SignalCalculationResponse(BaseModel):
    """Full response schema for replay signal calculation."""
    session_id: int
    observed_current_index: int
    timeframe: str
    results: List[SignalCalculationSeriesResponse]
