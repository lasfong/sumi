# Sprint 3 — No-Future-Leak Lock (Indicators)

> **Mục tiêu:** Frontend KHÔNG BAO GIỜ nhận indicator data của tương lai trong replay mode.  
> **Thời gian:** 2–3 ngày  
> **Phụ thuộc:** Sprint 1 (frontend build green) + Sprint 2 (backend tests green)  

> [!CAUTION]
> Đây là nguyên tắc quan trọng nhất của dự án: **KHÔNG LEAK DỮ LIỆU TƯƠNG LAI.**
> 
> Candles chính đã được bảo vệ. Nhưng indicator API hiện tại query trực tiếp theo symbol, không qua session → frontend có thể nhận indicator data cho candles tương lai. Đây là lỗ hổng nghiêm trọng.

---

## S3-T1: Tạo Session-Scoped Indicator API

**Estimate:** 4–6 giờ

### Vấn đề hiện tại

API hiện tại:
```http
GET /api/indicators/{symbol}?indicator=ema&length=20&limit=500
```
- Query trực tiếp theo symbol → trả TẤT CẢ dữ liệu, kể cả tương lai.
- Không biết session nào, không biết current_index.

### Giải pháp

Tạo endpoint mới:
```http
GET /api/replay/sessions/{session_id}/indicators?indicator=ema&length=20
```
- Load session → biết current_index
- Lấy visible candles (giống ReplayService.get_candles)
- Compute indicator chỉ trên visible candles
- Trả kết quả

### Files tạo/sửa

- `backend/app/api/replay.py` hoặc tạo `backend/app/api/indicators.py` (thêm endpoint)
- `backend/app/services/replay_service.py` (có thể thêm method)

### Hướng dẫn từng bước

**Bước 1:** Thêm endpoint trong file API replay:

```python
# backend/app/api/replay.py (hoặc file phù hợp)

@router.get("/sessions/{session_id}/indicators")
async def get_session_indicators(
    session_id: int,
    indicator: str,  # "sma", "ema", "rsi", etc.
    length: int = 20,
    db: Session = Depends(get_db)
):
    """
    Trả indicator data chỉ cho visible candles của session.
    KHÔNG ĐƯỢC trả data cho candles tương lai.
    
    Luồng xử lý:
    1. Load session → lấy current_index
    2. Lấy visible candles (giống get_candles)
    3. Compute indicator trên visible candles
    4. Trả kết quả
    """
    # 1. Load session
    session = db.query(ReplaySession).filter(ReplaySession.id == session_id).first()
    if not session:
        raise HTTPException(status_code=404, detail="Session not found")
    
    # 2. Lấy visible candles (chỉ đến current_index)
    from app.services.replay_service import ReplayService
    replay_service = ReplayService()
    candles = replay_service.get_candles(db, session_id, target_timeframe="1D")
    
    # 3. Convert thành DataFrame
    import pandas as pd
    df = pd.DataFrame([{
        "time": c.trade_date.isoformat() if hasattr(c, 'trade_date') else c["time"],
        "open": float(c.open if hasattr(c, 'open') else c["open"]),
        "high": float(c.high if hasattr(c, 'high') else c["high"]),
        "low": float(c.low if hasattr(c, 'low') else c["low"]),
        "close": float(c.close if hasattr(c, 'close') else c["close"]),
        "volume": float(c.volume if hasattr(c, 'volume') else c["volume"]),
    } for c in candles])
    
    if df.empty:
        return {"indicator": indicator, "length": length, "data": []}
    
    # 4. Compute indicator
    data = compute_indicator(df, indicator, length)
    
    # 5. Trả kết quả
    return {
        "session_id": session_id,
        "indicator": indicator,
        "length": length,
        "current_index": session.current_index,
        "data": data
    }


def compute_indicator(df: pd.DataFrame, indicator: str, length: int) -> list:
    """Tính indicator trên DataFrame visible candles."""
    close = df["close"]
    
    if indicator.lower() == "sma":
        result = close.rolling(window=length).mean()
    elif indicator.lower() == "ema":
        result = close.ewm(span=length, adjust=False).mean()
    elif indicator.lower() == "rsi":
        delta = close.diff()
        gain = delta.clip(lower=0).rolling(window=length).mean()
        loss = (-delta.clip(upper=0)).rolling(window=length).mean()
        rs = gain / loss
        result = 100 - (100 / (1 + rs))
    else:
        raise HTTPException(status_code=400, detail=f"Unknown indicator: {indicator}")
    
    # Convert thành list of dicts, bỏ NaN
    data = []
    for i, val in enumerate(result):
        if pd.notna(val):
            data.append({
                "time": df.iloc[i]["time"],
                "value": round(float(val), 2)
            })
    
    return data
```

> [!IMPORTANT]
> Logic quan trọng: **dùng `replay_service.get_candles()`** để lấy candles — function này đã đảm bảo chỉ trả visible candles đến current_index. KHÔNG query candle trực tiếp bằng symbol.

**Bước 2:** Register endpoint trong router (nếu cần).

**Bước 3:** Test thủ công:

```bash
# Start backend
cd backend
uvicorn app.main:app --reload

# Trong browser hoặc curl:
# 1. Tạo session
# 2. Gọi endpoint mới
curl http://localhost:8000/api/replay/sessions/1/indicators?indicator=sma&length=20
```

### Definition of Done

| Tiêu chí | Cách kiểm tra |
|----------|---------------|
| Endpoint `/sessions/{id}/indicators` tồn tại | Swagger docs hoặc curl |
| Chỉ trả data cho visible candles | So sánh response length với current_index |
| Hỗ trợ SMA, EMA, RSI | Test cả 3 indicator |
| Error handling cho indicator không hợp lệ | Gửi indicator=xyz → 400 |

---

## S3-T2: Đánh Dấu API Cũ Chỉ Dùng Ngoài Replay

**Estimate:** 15 phút

### File sửa

- `backend/app/api/indicators.py` (hoặc file chứa API indicator cũ)

### Hướng dẫn

Thêm comment và docstring cảnh báo:

```python
@router.get("/indicators/{symbol}")
async def get_indicators(symbol: str, ...):
    """
    ⚠️ WARNING: API này CHỈ dùng cho exploratory/non-replay use.
    
    Trong Replay mode, PHẢI dùng endpoint session-scoped:
    GET /api/replay/sessions/{session_id}/indicators
    
    API này query theo symbol và KHÔNG lọc theo session current_index,
    nên sẽ trả data cho cả candles tương lai → vi phạm no-future-leak rule.
    """
    # ... code hiện tại giữ nguyên
```

### Definition of Done

| Tiêu chí | Cách kiểm tra |
|----------|---------------|
| Comment cảnh báo có trong code | Đọc file |
| Docstring mô tả rõ giới hạn | Swagger docs hiện cảnh báo |

---

## S3-T3: Sửa Frontend Gọi Session Indicator API

**Estimate:** 3–4 giờ

### Files sửa

- `frontend/src/api/indicatorsApi.ts` (hoặc file tương tự)
- `frontend/src/pages/ReplayPage.tsx`

### Hướng dẫn từng bước

**Bước 1:** Thêm function mới trong API client:

```typescript
// frontend/src/api/indicatorsApi.ts

export async function getSessionIndicatorData(
  sessionId: number,
  indicator: string,
  params: { length?: number } = {}
) {
  const queryParams = new URLSearchParams({
    indicator,
    ...(params.length ? { length: String(params.length) } : {})
  });
  
  const response = await apiClient.get(
    `/api/replay/sessions/${sessionId}/indicators?${queryParams}`
  );
  return response.data;
}
```

**Bước 2:** Trong `ReplayPage.tsx`, tìm chỗ gọi indicator API cũ và thay bằng API mới:

```typescript
// TRƯỚC (sai — gọi theo symbol):
const indicatorData = await getIndicatorData(symbol, "ema", { length: 20, limit: 500 });

// SAU (đúng — gọi theo session):
const indicatorData = await getSessionIndicatorData(sessionId, "ema", { length: 20 });
```

**Bước 3:** Xóa logic cache full future data. Tìm và xóa bất kỳ code nào:
- Cache toàn bộ indicator data rồi slice theo current candle
- Filter indicator data ở frontend theo timestamp

```typescript
// XÓA logic kiểu này:
const filtered = allIndicatorData.filter(d => d.time <= currentCandle.time);
// Vì backend đã chỉ trả visible data, không cần filter nữa.
```

**Bước 4:** Build kiểm tra:

```bash
cd frontend
npm run build
```

### Definition of Done

| Tiêu chí | Cách kiểm tra |
|----------|---------------|
| Frontend gọi `/sessions/{id}/indicators` | Grep code, không còn call API cũ trong ReplayPage |
| Không cache full future data | Grep code, không còn logic slice/filter indicator |
| Build pass | `npm run build` exit 0 |

### Sai lầm thường gặp

- ❌ Giữ cả API cũ và mới → confusing, dễ dùng sai
- ❌ Frontend vẫn filter client-side mà nghĩ đã an toàn → sai, data đã đến frontend rồi
- ❌ Quên truyền sessionId → call fail

---

## S3-T4: Test No-Future-Leak Cho Indicators

**Estimate:** 2–3 giờ

### File tạo

- `backend/app/tests/test_indicator_no_leak.py` (file mới)

### Hướng dẫn từng bước

```python
# backend/app/tests/test_indicator_no_leak.py

import pytest
from datetime import date, timedelta

def test_indicator_no_future_leak(client, db_session):
    """
    Seed 30 candles, tạo session current_index=10.
    Gọi EMA20 session endpoint.
    Assert: không có data point nào có timestamp > candle tại index 10.
    """
    symbol = "LEAK_TEST"
    
    # 1. Seed 30 candles với giá tăng dần
    base_date = date(2024, 1, 1)
    for i in range(30):
        create_candle(
            db_session,
            symbol=symbol,
            trade_date=base_date + timedelta(days=i),
            open=100.0 + i,
            high=101.0 + i,
            low=99.0 + i,
            close=100.5 + i,
            volume=1000000
        )
    
    # 2. Tạo session
    response = client.post("/api/replay/sessions", json={
        "symbol": symbol,
        "timeframe": "1D",
        "start_date": str(base_date),
        "end_date": str(base_date + timedelta(days=29)),
        "initial_cash": 100000000
    })
    session_id = response.json()["id"]
    
    # 3. Advance đến index 10
    for _ in range(10):
        client.post(f"/api/replay/sessions/{session_id}/next")
    
    # 4. Gọi indicator endpoint
    response = client.get(
        f"/api/replay/sessions/{session_id}/indicators",
        params={"indicator": "ema", "length": 5}
    )
    assert response.status_code == 200
    indicator_data = response.json()["data"]
    
    # 5. Assert: không có data sau candle index 10
    max_visible_date = str(base_date + timedelta(days=10))
    for point in indicator_data:
        assert point["time"] <= max_visible_date, \
            f"LEAK DETECTED: indicator point {point['time']} > visible date {max_visible_date}"
    
    # 6. Advance thêm 1 bước
    client.post(f"/api/replay/sessions/{session_id}/next")
    
    # 7. Gọi lại → response phải dài hơn
    response2 = client.get(
        f"/api/replay/sessions/{session_id}/indicators",
        params={"indicator": "ema", "length": 5}
    )
    indicator_data_2 = response2.json()["data"]
    
    # Sau khi advance, response có thể bằng hoặc dài hơn (tùy EMA warmup)
    # Nhưng chắc chắn timestamp cuối <= candle index 11
    max_visible_date_2 = str(base_date + timedelta(days=11))
    for point in indicator_data_2:
        assert point["time"] <= max_visible_date_2
```

### Lệnh kiểm tra

```bash
python -m pytest -v app/tests/test_indicator_no_leak.py
```

### Definition of Done

| Tiêu chí | Cách kiểm tra |
|----------|---------------|
| Test file tồn tại | `ls app/tests/test_indicator_no_leak.py` |
| Test PASS | pytest hiện PASSED |
| Test thực sự check timestamp boundary | Đọc code, confirm assert |
| Test check sau advance | Đọc code, confirm advance + re-check |

---

## S3-T5: Full Regression

**Estimate:** 1 giờ

### Mục tiêu

Đảm bảo thay đổi Sprint 3 không phá Sprint 1 và Sprint 2.

### Hướng dẫn

```bash
# Backend tests
cd backend
python -m pytest -q app/tests

# Frontend build
cd ../frontend
npm run build
```

**Cả hai phải pass.**

### Definition of Done

| Tiêu chí | Cách kiểm tra |
|----------|---------------|
| Backend tests pass | pytest output ALL PASS |
| Frontend build pass | npm run build exit 0 |
| No-future-leak candles test pass | Existing test still green |
| No-future-leak indicators test pass | New test green |

---

## 🏁 Definition of Done — Sprint 3 Tổng Thể

| # | Tiêu chí | Lệnh kiểm tra | Kết quả |
|---|----------|---------------|---------|
| 1 | Session indicator endpoint hoạt động | curl test | 200 + data |
| 2 | Endpoint chỉ trả visible data | Test no_leak PASS |
| 3 | Frontend dùng session API | Grep code confirm |
| 4 | Frontend không cache future data | Grep code confirm |
| 5 | API cũ có warning comment | Đọc code |
| 6 | Backend tests pass | `pytest -q` pass |
| 7 | Frontend build pass | `npm run build` pass |

> [!CAUTION]
> **Nếu indicator leak test FAIL, Sprint 3 KHÔNG HOÀN THÀNH.** Đây là nguyên tắc bất di bất dịch của dự án.
