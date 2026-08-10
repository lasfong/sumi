from pydantic import BaseModel
from typing import List, Optional

class ImportWarning(BaseModel):
    row_index: int
    message: str

class ImportResponse(BaseModel):
    imported_rows: int
    skipped_rows: int
    duplicate_rows: int
    symbols_count: int
    start_date: Optional[str] = None
    end_date: Optional[str] = None
    warnings: List[ImportWarning] = []

class ImportRunItemSchema(BaseModel):
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
    classification: str
    reject_reason: Optional[str] = None

class ImportPreviewResponse(BaseModel):
    run_id: str
    file_name: str
    file_sha256: str
    content_sha256: str
    parser_version: str
    source_type: str
    timeframe: str
    adjustment_type: str
    timezone: str
    status: str
    parsed_count: int
    rejected_count: int
    duplicate_count: int
    conflicting_count: int
    missing_count: int
    out_of_order_count: int
    accepted_count: int
    can_accept: bool
    block_reason: Optional[str] = None
    items: List[ImportRunItemSchema] = []

class ImportAcceptRequest(BaseModel):
    content_sha256: str

class ImportAcceptResponse(BaseModel):
    run_id: str
    status: str
    accepted_count: int
    message: str

class ImportRollbackResponse(BaseModel):
    run_id: str
    status: str
    restored_mutations_count: int
    message: str

class CatalogItemSchema(BaseModel):
    symbol: str
    exchange: Optional[str] = None
    timeframe: str
    adjustment_type: str
    start_date: Optional[str] = None
    end_date: Optional[str] = None
    row_count: int
    last_accepted_at: Optional[str] = None
    provenance_state: str
