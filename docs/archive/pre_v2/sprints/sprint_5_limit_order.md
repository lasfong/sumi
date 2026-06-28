# Sprint 5 — Limit Order & Market Constraints

> **Mục tiêu:** TC-004 pass. Manual UI đặt được Limit order. Trần/sàn theo HOSE/HNX/UPCOM hoạt động.  
> **Thời gian:** 3–4 ngày  
> **Phụ thuộc:** Sprint 4 (trade lifecycle đúng)  

> [!IMPORTANT]
> Quy tắc biên độ thị trường Việt Nam:
> - **HOSE:** ±7% so với giá tham chiếu (giá close phiên trước)
> - **HNX:** ±10%
> - **UPCOM:** ±15%
> 
> Lệnh Limit Order đặt ngoài biên độ → REJECTED.

---

## S5-T1: Expand Schema DecisionCreate

**Estimate:** 1–2 giờ

### File sửa

- `backend/app/schemas/decision_schema.py`

### Hướng dẫn

**Bước 1:** Tìm schema `DecisionCreate` (hoặc tương tự). Thêm validation cho `order_type`:

```python
from enum import Enum

class OrderTypeEnum(str, Enum):
    MARKET_AT_CLOSE = "MARKET_AT_CLOSE"
    LIMIT = "LIMIT"
    # Các loại khác để sau:
    # MARKET_NEXT_OPEN = "MARKET_NEXT_OPEN"

class DecisionCreate(BaseModel):
    action: str  # BUY, SELL, CLOSE, etc.
    price: Optional[float] = None
    quantity: Optional[int] = None
    order_type: str = "MARKET_AT_CLOSE"
    stop_loss: Optional[float] = None
    target_price: Optional[float] = None
    setup_type: Optional[str] = None
    confidence_score: Optional[int] = None
    market_context: Optional[str] = None
    reason: Optional[str] = None
    note: Optional[str] = None
    mistake_tag: Optional[str] = None

    @validator("price")
    def price_required_for_limit(cls, v, values):
        """LIMIT order phải có price."""
        if values.get("order_type") == "LIMIT" and v is None:
            raise ValueError("Limit order requires a price")
        return v
    
    @validator("order_type")
    def validate_order_type(cls, v):
        """Chỉ chấp nhận order_type hợp lệ."""
        allowed = ["MARKET_AT_CLOSE", "LIMIT"]
        if v not in allowed:
            raise ValueError(f"order_type must be one of: {allowed}")
        return v
```

### Definition of Done

| Tiêu chí | Cách kiểm tra |
|----------|---------------|
| LIMIT order không có price → validation error | Unit test |
| MARKET_AT_CLOSE không cần price | Unit test |
| order_type không hợp lệ → error | Unit test |

---

## S5-T2: Thêm Trạng Thái Order Lifecycle

**Estimate:** 1–2 giờ

### Hiện tại

Manual flow luôn execute ngay → order status luôn là `executed`.

### Cần thêm

```text
created   → Mới tạo
pending   → Limit order chờ khớp
executed  → Đã khớp
rejected  → Bị từ chối (ngoài biên độ, thiếu tiền, etc.)
cancelled → Người dùng hủy
expired   → Hết hạn (cuối phiên)
```

### File sửa

- `backend/app/models/order.py` — kiểm tra status field
- `backend/app/services/trade_lifecycle_service.py` — logic tạo order

### Hướng dẫn

```python
# Khi tạo MARKET_AT_CLOSE order → execute ngay:
order.status = "executed"

# Khi tạo LIMIT order hợp lệ → pending:
order.status = "pending"

# Khi LIMIT order bị reject (ngoài biên độ):
order.status = "rejected"
```

### Definition of Done

| Tiêu chí | Cách kiểm tra |
|----------|---------------|
| MARKET_AT_CLOSE → status "executed" | Test |
| LIMIT hợp lệ → status "pending" | Test |
| LIMIT ngoài biên độ → status "rejected" | Test |

---

## S5-T3: Validate Price Limits (Trần/Sàn)

**Estimate:** 3–4 giờ

### Logic

```text
1. Lấy symbol → exchange
2. Lấy previous candle close → reference price
3. Tính ceiling/floor:
   - HOSE: ceiling = ref × 1.07, floor = ref × 0.93
   - HNX:  ceiling = ref × 1.10, floor = ref × 0.90
   - UPCOM: ceiling = ref × 1.15, floor = ref × 0.85
4. Nếu limit_price > ceiling hoặc limit_price < floor → REJECT
```

### Files sửa

- `backend/app/services/trade_lifecycle_service.py`
- `backend/app/domain/engine/market_constraints.py` (nếu có, hoặc tạo mới)

### Hướng dẫn

**Bước 1:** Tạo/cập nhật market constraints helper:

```python
# backend/app/domain/engine/market_constraints.py

EXCHANGE_LIMITS = {
    "HOSE": 0.07,   # ±7%
    "HNX": 0.10,    # ±10%
    "UPCOM": 0.15,  # ±15%
}

def get_price_limits(reference_price: float, exchange: str) -> tuple:
    """
    Trả ceiling và floor price dựa trên reference price và exchange.
    
    Args:
        reference_price: Giá đóng cửa phiên trước
        exchange: HOSE, HNX, hoặc UPCOM
    
    Returns:
        (floor_price, ceiling_price)
    """
    limit_pct = EXCHANGE_LIMITS.get(exchange, 0.07)  # Default HOSE nếu không biết
    ceiling = reference_price * (1 + limit_pct)
    floor = reference_price * (1 - limit_pct)
    return (floor, ceiling)


def is_price_within_limits(price: float, reference_price: float, exchange: str) -> bool:
    """Kiểm tra giá có nằm trong biên độ trần/sàn không."""
    floor, ceiling = get_price_limits(reference_price, exchange)
    return floor <= price <= ceiling
```

**Bước 2:** Trong `trade_lifecycle_service.py`, khi tạo LIMIT order:

```python
async def _create_limit_order(self, session, decision, price, quantity):
    # 1. Lấy exchange
    symbol_record = db.query(Symbol).filter(Symbol.symbol == session.symbol).first()
    exchange = symbol_record.exchange if symbol_record and symbol_record.exchange else "HOSE"
    
    # 2. Lấy reference price (previous candle close)
    # current_index là candle hiện tại, reference = candle trước đó
    candles = self.replay_service.get_candles(db, session.id)
    if session.current_index > 0:
        reference_price = candles[session.current_index - 1].close
    else:
        reference_price = candles[0].close  # Dùng candle đầu tiên nếu index = 0
    
    # 3. Validate price limits
    from app.domain.engine.market_constraints import is_price_within_limits, get_price_limits
    
    if not is_price_within_limits(price, reference_price, exchange):
        floor, ceiling = get_price_limits(reference_price, exchange)
        order = Order(
            status="rejected",
            # ... other fields
        )
        db.add(order)
        raise HTTPException(
            status_code=400,
            detail=f"Giá {price} nằm ngoài biên độ {exchange}. "
                   f"Trần: {ceiling:.2f}, Sàn: {floor:.2f}"
        )
    
    # 4. Tạo pending order
    order = Order(
        status="pending",
        order_type="LIMIT",
        requested_price=price,
        # ... other fields
    )
    db.add(order)
    return order
```

### Definition of Done

| Tiêu chí | Cách kiểm tra |
|----------|---------------|
| HOSE ref=100, limit=111 → rejected (>107) | Test pass |
| HOSE ref=100, limit=105 → pending | Test pass |
| HNX ref=100, limit=112 → rejected (>110) | Test pass |
| UPCOM ref=100, limit=114 → pending (<115) | Test pass |

---

## S5-T4: Khớp Pending LIMIT Order Khi Next Candle

**Estimate:** 3–4 giờ

### Logic

Khi `next_candle()` advance, sau khi update session.current_index:

```text
1. Lấy candle mới vừa được reveal
2. Lấy tất cả pending orders của session
3. Với mỗi pending order:
   a. BUY LIMIT: nếu candle.low <= limit_price <= candle.high → execute
   b. SELL LIMIT: nếu candle.low <= limit_price <= candle.high → execute
4. Execute = tạo Execution, cập nhật Position/Trade, cập nhật Cash
```

### File sửa

- `backend/app/services/replay_service.py` (thêm vào `next_candle`)
- Hoặc tạo `backend/app/services/order_matching_service.py`

### Hướng dẫn

```python
# Trong replay_service.py, trong method next_candle:

async def next_candle(self, db, session_id, steps=1):
    # ... existing logic to advance current_index ...
    
    # SAU khi advance, check pending orders
    await self._match_pending_orders(db, session)


async def _match_pending_orders(self, db, session):
    """Kiểm tra và khớp các pending limit orders."""
    from app.models.order import Order
    
    # Lấy candle hiện tại (vừa được reveal)
    candles = self.get_candles(db, session.id)
    current_candle = candles[session.current_index]
    
    # Lấy pending orders
    pending_orders = db.query(Order).filter(
        Order.session_id == session.id,
        Order.status == "pending"
    ).all()
    
    for order in pending_orders:
        if self._should_fill_order(order, current_candle):
            # Determine execution price
            if order.side == "BUY":
                # Fill at limit price or candle open (whichever is worse for buyer)
                exec_price = min(order.requested_price, current_candle.high)
                if current_candle.open <= order.requested_price:
                    exec_price = current_candle.open  # Gap down favorable
            else:
                exec_price = max(order.requested_price, current_candle.low)
                if current_candle.open >= order.requested_price:
                    exec_price = current_candle.open  # Gap up favorable
            
            # Execute the order
            await self.trade_lifecycle.execute_pending_order(
                db, session, order, exec_price
            )

def _should_fill_order(self, order, candle) -> bool:
    """Kiểm tra candle có trigger limit order không."""
    return candle.low <= order.requested_price <= candle.high
```

### Definition of Done

| Tiêu chí | Cách kiểm tra |
|----------|---------------|
| BUY LIMIT pending → next candle chứa price → executed | Test pass |
| SELL LIMIT pending → next candle chứa price → executed | Test pass |
| Pending order ở candle không chứa price → vẫn pending | Test pass |
| Executed order cập nhật Position/Trade/Cash | Test pass |

---

## S5-T5: Frontend Order Panel Cho Limit

**Estimate:** 2–3 giờ

### File sửa

- `frontend/src/components/replay/TradeControls.tsx`

### Hướng dẫn

Thêm UI elements:

```typescript
// Trong TradeControls component, thêm:

// 1. Select Order Type
<select value={orderType} onChange={e => setOrderType(e.target.value)}>
  <option value="MARKET_AT_CLOSE">Market at Close</option>
  <option value="LIMIT">Limit Order</option>
</select>

// 2. Limit Price Input (chỉ hiện khi orderType === "LIMIT")
{orderType === "LIMIT" && (
  <input
    type="number"
    placeholder="Nhập giá limit"
    value={limitPrice}
    onChange={e => setLimitPrice(Number(e.target.value))}
    step="100"
  />
)}

// 3. Submit gửi order_type và price
const handleSubmit = () => {
  const payload = {
    action: currentAction,
    order_type: orderType,
    price: orderType === "LIMIT" ? limitPrice : undefined,
    quantity: quantity,
    // ... other fields
  };
  submitDecision(payload);
};
```

### Definition of Done

| Tiêu chí | Cách kiểm tra |
|----------|---------------|
| UI có dropdown chọn order type | Visual check |
| Chọn Limit → hiện input giá | Visual check |
| Submit gửi đúng order_type và price | Network tab check |
| Frontend build pass | `npm run build` |

---

## S5-T6: Tests TC-004

**Estimate:** 2–3 giờ

### Test cases

```python
def test_tc004_hose_limit_above_ceiling_rejected(db_session):
    """
    TC-004: HOSE previous close = 100.
    BUY LIMIT price = 111 (> 107 ceiling).
    Expected: REJECTED.
    """
    # Seed candle close = 100, exchange = HOSE
    # Create session, advance to index 1 (so previous close = 100)
    # Submit BUY LIMIT price=111
    # Assert: HTTP 400, order status = rejected

def test_tc004_hose_limit_within_band_pending(db_session):
    """
    HOSE previous close = 100.
    BUY LIMIT price = 105 (< 107 ceiling).
    Expected: PENDING, then fills when candle contains 105.
    """
    # Seed candles: index 0 close=100, index 1 low=104 high=106
    # Create session
    # Advance to index 1  
    # Submit BUY LIMIT price=105
    # Assert: order status = pending
    # Advance to index 2 (candle contains 105)
    # Assert: order status = executed
    # Assert: position opened

def test_tc004_hnx_limit_above_ceiling_rejected(db_session):
    """HNX ±10%: ref=100, limit=112 → rejected"""
    # Similar to above but exchange=HNX, limit=112 > 110

def test_tc004_upcom_limit_within_band(db_session):
    """UPCOM ±15%: ref=100, limit=114 → pending (< 115)"""
    # exchange=UPCOM, limit=114
```

### Definition of Done

| Tiêu chí | Cách kiểm tra |
|----------|---------------|
| TC-004 HOSE reject pass | pytest pass |
| TC-004 HOSE pending+fill pass | pytest pass |
| TC-004 HNX reject pass | pytest pass |
| TC-004 UPCOM pass | pytest pass |

---

## 🏁 Definition of Done — Sprint 5 Tổng Thể

| # | Tiêu chí | Kết quả |
|---|----------|---------|
| 1 | LIMIT order schema validation | Pass |
| 2 | Order status lifecycle (pending/executed/rejected) | Pass |
| 3 | Price limit validation HOSE ±7% | Pass |
| 4 | Price limit validation HNX ±10% | Pass |
| 5 | Price limit validation UPCOM ±15% | Pass |
| 6 | Pending order matches on next candle | Pass |
| 7 | Frontend UI cho Limit order | Build pass |
| 8 | TC-004 all tests pass | `pytest` pass |
| 9 | All backend tests pass | `pytest -q` exit 0 |
| 10 | Frontend build pass | `npm run build` exit 0 |
