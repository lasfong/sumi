from pydantic import BaseModel, ConfigDict
from datetime import datetime

class ExecutionResponse(BaseModel):
    id: int
    order_id: int
    session_id: int
    symbol: str
    execution_date: datetime
    execution_price: float
    quantity: float
    fee: float
    tax: float
    slippage: float
    gross_amount: float
    net_amount: float
    created_at: datetime
    model_config = ConfigDict(from_attributes=True)
