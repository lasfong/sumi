from pydantic import BaseModel, ConfigDict
from typing import Optional

class SymbolBase(BaseModel):
    symbol: str
    exchange: Optional[str] = None
    company_name: Optional[str] = None
    sector: Optional[str] = None
    industry: Optional[str] = None
    asset_type: str = "stock"
    is_active: bool = True

class SymbolCreate(SymbolBase):
    pass

class SymbolResponse(SymbolBase):
    model_config = ConfigDict(from_attributes=True)
