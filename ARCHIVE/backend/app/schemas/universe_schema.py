"""Pydantic schemas for managed universe APIs."""

from __future__ import annotations

from typing import Any, List, Optional
from pydantic import BaseModel, Field


class UniverseMemberSchema(BaseModel):
    """Schema for a constituent in a universe."""
    symbol: str
    exchange: str
    company_name: Optional[str] = None
    sector: Optional[str] = None
    effective_from: Optional[str] = None
    effective_to: Optional[str] = None
    status: str
    inclusion_reason: str = ""
    weight: float = 1.0
    metadata: dict[str, Any] = Field(default_factory=dict)


class UniverseSummarySchema(BaseModel):
    """Summary schema for a registered universe definition."""
    universe_id: str
    version: str
    name: str
    status: str
    default_mode: str
    effective_from: Optional[str] = None
    effective_to: Optional[str] = None
    selection_purpose: str = ""
    source_evidence: str = ""
    total_member_count: int
    active_member_count: int
    content_hash: str


class UniverseDetailSchema(UniverseSummarySchema):
    """Full detail schema including selection rules and member list."""
    selection_rules: List[str] = Field(default_factory=list)
    members: List[UniverseMemberSchema] = Field(default_factory=list)
    metadata: dict[str, Any] = Field(default_factory=dict)


class UniverseResolutionSchema(BaseModel):
    """Schema for the result of universe resolution."""
    universe_id: str
    version: str
    as_of: str
    mode: str
    is_point_in_time: bool
    active_symbols: List[str]
    excluded_symbols: List[str]
    active_count: int
    total_target_count: int
    content_hash: str
    survivor_bias_caveat: Optional[str] = None


class UniverseCoverageRequestSchema(BaseModel):
    """Request payload for checking coverage against a universe."""
    symbols: List[str] = Field(..., description="Available symbol tickers observed on the as_of date.")
    threshold: float = Field(0.80, ge=0.0, le=1.0, description="Coverage satisfaction threshold ratio.")


class UniverseCoverageReportSchema(BaseModel):
    """Coverage report schema."""
    universe_id: str
    version: str
    as_of: str
    target_count: int
    available_count: int
    missing_count: int
    coverage_ratio: float
    missing_symbols: List[str]
    status: str
    threshold: float
