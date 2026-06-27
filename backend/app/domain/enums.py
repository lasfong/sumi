from enum import Enum

class AssetType(str, Enum):
    STOCK = "stock"
    INDEX = "index"
    ETF = "etf"
    FUTURE = "future"
    UNKNOWN = "unknown"

class SessionStatus(str, Enum):
    CREATED = "created"
    ACTIVE = "active"
    PAUSED = "paused"
    COMPLETED = "completed"
    ARCHIVED = "archived"
    BANKRUPT = "bankrupt"

class SessionMode(str, Enum):
    NORMAL = "normal"
    RANDOM = "random"
    BLIND_SYMBOL = "blind_symbol"
    BLIND_DATE = "blind_date"

class DecisionAction(str, Enum):
    BUY = "BUY"
    SELL = "SELL"
    HOLD = "HOLD"
    SKIP = "SKIP"
    ADD = "ADD"
    REDUCE = "REDUCE"
    CLOSE = "CLOSE"
    CUT_LOSS = "CUT_LOSS"
    TAKE_PROFIT = "TAKE_PROFIT"

class OrderSide(str, Enum):
    BUY = "BUY"
    SELL = "SELL"

class OrderType(str, Enum):
    MARKET_AT_CLOSE = "MARKET_AT_CLOSE"
    MARKET_NEXT_OPEN = "MARKET_NEXT_OPEN"
    LIMIT = "LIMIT"
    CUSTOM_PRICE = "CUSTOM_PRICE"

class OrderStatus(str, Enum):
    CREATED = "created"
    EXECUTED = "executed"
    CANCELLED = "cancelled"
    REJECTED = "rejected"

class PositionStatus(str, Enum):
    OPEN = "open"
    CLOSED = "closed"

class TradeStatus(str, Enum):
    OPEN = "open"
    CLOSED = "closed"

class TradeResult(str, Enum):
    WIN = "win"
    LOSS = "loss"
    BREAKEVEN = "breakeven"
    OPEN = "open"
