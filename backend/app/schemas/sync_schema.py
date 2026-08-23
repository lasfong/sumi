from pydantic import BaseModel
from typing import List, Optional, Dict, Any

class ProviderMetadataSchema(BaseModel):
    provider_id: str
    display_name: str
    is_official: bool
    requires_auth: bool
    supported_timeframes: List[str]
    supported_adjustments: List[str]
    rate_limit_rps: int
    description: str
    auth_fields: List[str]
    is_configured: bool = False


class TestConnectionRequest(BaseModel):
    provider_id: str
    credentials: Optional[Dict[str, Any]] = None


class TestConnectionResponse(BaseModel):
    success: bool
    provider_id: str
    message: str
    latency_ms: Optional[float] = None


class SyncPreviewRequest(BaseModel):
    symbol: str
    start_date: str  # YYYY-MM-DD
    end_date: str    # YYYY-MM-DD
    provider_id: str = "ssi"
    adjustment_type: str = "unadjusted"
    credentials: Optional[Dict[str, Any]] = None


class SyncRunItemSchema(BaseModel):
    row_index: int
    symbol: str
    timeframe: str
    timestamp: str
    adjustment_type: str
    open: Optional[float] = None
    high: Optional[float] = None
    low: Optional[float] = None
    close: Optional[float] = None
    volume: Optional[float] = None
    classification: str  # parsed, duplicate, conflicting, rejected, missing, out_of_order
    reject_reason: Optional[str] = None


class SyncPreviewResponse(BaseModel):
    sync_id: str
    provider_id: str
    symbol: str
    start_date: str
    end_date: str
    timeframe: str
    adjustment_type: str
    status: str
    parsed_count: int
    rejected_count: int
    duplicate_count: int
    conflicting_count: int
    missing_count: int
    out_of_order_count: int
    can_accept: bool
    block_reason: Optional[str] = None
    content_sha256: str
    items: List[SyncRunItemSchema] = []


class SyncExecuteRequest(BaseModel):
    sync_id: str
    content_sha256: str


class SyncExecuteResponse(BaseModel):
    sync_id: str
    status: str
    accepted_count: int
    symbol: str
    timeframe: str
    adjustment_type: str
    duration_ms: float
    message: str
    manifest: Optional[Dict[str, Any]] = None


class SyncRollbackRequest(BaseModel):
    sync_id: str


class SyncRollbackResponse(BaseModel):
    sync_id: str
    status: str
    restored_mutations_count: int
    message: str


class SyncManifestSchema(BaseModel):
    sync_id: str
    created_at: str
    provider_id: str
    symbol: str
    start_date: str
    end_date: str
    timeframe: str
    adjustment_type: str
    status: str
    parsed_count: int
    duplicate_count: int
    conflicting_count: int
    accepted_count: int
    duration_ms: float
    accepted_at: Optional[str] = None
    rolled_back_at: Optional[str] = None
    manifest: Optional[Dict[str, Any]] = None
