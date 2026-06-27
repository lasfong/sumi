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
