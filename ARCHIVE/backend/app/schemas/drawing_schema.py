from pydantic import BaseModel, ConfigDict
from typing import Any

class DrawingStateUpdate(BaseModel):
    state_data: str # JSON string of drawings

class DrawingStateResponse(BaseModel):
    id: int
    session_id: int
    symbol: str
    state_data: str

    model_config = ConfigDict(from_attributes=True)
