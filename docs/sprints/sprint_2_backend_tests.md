# Sprint 2 — Backend Tests Green

> **Mục tiêu:** `python -m pytest -q app/tests` pass — backend tests phản ánh đúng rule hiện tại.  
> **Thời gian:** 2–3 ngày  
> **Phụ thuộc:** Sprint 0 (venv hoạt động)  
> **Nguyên tắc:** KHÔNG thêm feature mới. CHỈ sửa tests và logic liên quan.

> [!IMPORTANT]
> Rule T+2: Cổ phiếu mua ở candle index T chỉ được bán ở candle index T+2 trở đi. Đây là rule bắt buộc của thị trường chứng khoán Việt Nam. Tất cả test phải tuân thủ rule này.

---

## S2-T1: Chạy Toàn Bộ Backend Tests & Phân Loại Lỗi

**Estimate:** 1–2 giờ

### Mục tiêu

Chạy toàn bộ test suite, liệt kê test nào pass/fail, và phân loại nguyên nhân fail.

### Hướng dẫn từng bước

**Bước 1:** Activate venv và chạy tests:

```bash
cd E:\Workspace\sumi\backend
.\.venv\Scripts\activate
python -m pytest -v app/tests
```

> [!TIP]
> Flag `-v` hiện chi tiết từng test. Flag `-q` chỉ hiện tóm tắt. Lần đầu dùng `-v` để thấy rõ.

**Bước 2:** Ghi lại output. Phân loại mỗi test fail vào 1 trong 3 nhóm:

| Nhóm | Ý nghĩa | Ví dụ |
|------|---------|-------|
| **ENV** | Fail vì environment/import | ModuleNotFoundError, database không kết nối được |
| **RULE** | Test cũ trái rule mới (T+2) | Test expect SELL at T+1 thành công, nhưng rule T+2 bắt reject |
| **BUG** | Bug thật trong code | Logic tính PnL sai, fee không được tính |

**Bước 3:** Tạo bảng phân loại:

```text
| Test Name                        | Status | Nhóm  | Ghi chú                          |
|----------------------------------|--------|-------|----------------------------------|
| test_buy_opens_position          | PASS   | -     |                                  |
| test_close_position_after_buy    | FAIL   | RULE  | SELL ở T+1, cần T+2              |
| test_import_csv                  | PASS   | -     |                                  |
| ...                              |        |       |                                  |
```

### Definition of Done

| Tiêu chí | Cách kiểm tra |
|----------|---------------|
| Đã chạy `pytest -v` | Có output đầy đủ |
| Mỗi test fail đều được phân loại | Có bảng phân loại |
| Biết tổng số pass/fail | Ghi lại con số |

### Sai lầm thường gặp

- ❌ Thấy test fail → sửa ngay mà không hiểu nguyên nhân
- ❌ Không ghi lại output → sau không nhớ test nào fail
- ❌ Bỏ qua lỗi ENV (import error) → nó sẽ ảnh hưởng đến các test khác

---

## S2-T2: Tạo Test — Bán T+1 Bị Reject

**Estimate:** 2–3 giờ

### Mục tiêu

Viết test chứng minh: mua ở candle index 0, tiến 1 bước đến index 1, bán → hệ thống phải REJECT.

### File tạo/sửa

- `backend/app/tests/test_trade_lifecycle.py` (thêm test mới)

### Hướng dẫn từng bước

**Bước 1:** Mở file `test_trade_lifecycle.py`, thêm test mới:

```python
def test_sell_t_plus_1_is_rejected(db_session):
    """
    Mua ở candle index 0, next 1 lần đến index 1.
    Bán ở index 1 → phải bị reject vì vi phạm T+2.
    
    Rule: cổ phiếu mua ở T chỉ được bán ở T+2.
    T=0, T+1=1, T+2=2 → bán sớm nhất ở index 2.
    """
    # 1. Seed ít nhất 5 candles cho symbol
    symbol = "TEST"
    candles = create_test_candles(db_session, symbol, count=5, start_price=100.0)
    
    # 2. Tạo replay session
    session = create_test_session(db_session, symbol, initial_cash=100_000_000)
    
    # 3. BUY ở index 0
    buy_result = execute_buy(db_session, session, price=100.0, quantity=1000)
    assert buy_result is not None  # BUY thành công
    
    # 4. Next 1 lần → current_index = 1
    advance_session(db_session, session, steps=1)
    assert session.current_index == 1
    
    # 5. Thử SELL/CLOSE → expect FAIL
    from fastapi import HTTPException
    import pytest
    
    with pytest.raises(HTTPException) as exc_info:
        execute_sell(db_session, session, quantity=1000)
    
    # 6. Verify lỗi là về T+2
    assert exc_info.value.status_code == 400
    assert "T+2" in str(exc_info.value.detail) or "chưa về" in str(exc_info.value.detail)
```

> [!WARNING]
> Code trên là pseudo-code. Bạn cần dùng đúng function names từ codebase hiện tại.
> Đọc file `conftest.py` và các test hiện có để biết helper functions nào đã có.

**Bước 2:** Kiểm tra cách test hiện tại setup data. Mở `conftest.py`:

```bash
# Xem conftest.py để biết fixtures có sẵn
cat backend/app/tests/conftest.py
```

**Bước 3:** Adapt test theo cấu trúc thực tế. Ví dụ nếu test dùng API client:

```python
def test_sell_t_plus_1_is_rejected(client, db_session):
    """Bán ở T+1 phải bị reject"""
    # Setup: seed candles, create session, buy
    # ...
    
    # Advance 1 step
    response = client.post(f"/api/replay/sessions/{session_id}/next")
    assert response.status_code == 200
    
    # Try to sell
    response = client.post(
        f"/api/replay/sessions/{session_id}/decisions",
        json={
            "action": "CLOSE",
            "price": 105.0,
            "quantity": 1000,
            "order_type": "MARKET_AT_CLOSE"
        }
    )
    
    # Must be rejected
    assert response.status_code == 400
    assert "T+2" in response.json().get("detail", "")
```

### Lệnh kiểm tra

```bash
cd backend
python -m pytest -v app/tests/test_trade_lifecycle.py::test_sell_t_plus_1_is_rejected
```

### Definition of Done

| Tiêu chí | Cách kiểm tra |
|----------|---------------|
| Test tồn tại | File có function `test_sell_t_plus_1_is_rejected` |
| Test PASS | `pytest -v ...::test_sell_t_plus_1_is_rejected` hiện PASSED |
| Test thực sự check T+2 | Đọc code, confirm expect 400 + message T+2 |

---

## S2-T3: Tạo Test — Bán T+2 Được Phép

**Estimate:** 2–3 giờ

### Mục tiêu

Viết test chứng minh: mua ở candle index 0, tiến 2 bước đến index 2, bán → thành công.

### File tạo/sửa

- `backend/app/tests/test_trade_lifecycle.py`

### Hướng dẫn từng bước

```python
def test_sell_t_plus_2_is_allowed(db_session):
    """
    Mua ở candle index 0, next 2 lần đến index 2.
    Bán ở index 2 → phải thành công.
    
    Verify:
    - Order executed
    - Position closed
    - Trade closed với result và PnL
    """
    # 1. Seed candles
    symbol = "TEST"
    candles = create_test_candles(db_session, symbol, count=5, start_price=100.0)
    
    # 2. Create session
    session = create_test_session(db_session, symbol, initial_cash=100_000_000)
    
    # 3. BUY ở index 0
    buy_result = execute_buy(db_session, session, price=100.0, quantity=1000)
    
    # 4. Next 2 lần → current_index = 2
    advance_session(db_session, session, steps=1)  # index = 1
    advance_session(db_session, session, steps=1)  # index = 2
    assert session.current_index == 2
    
    # 5. SELL ở index 2 → expect SUCCESS
    sell_result = execute_sell(db_session, session, quantity=1000)
    
    # 6. Verify
    # Position phải đóng
    position = get_position(db_session, session.id, symbol)
    assert position.status == "closed"
    assert position.quantity == 0
    
    # Trade phải đóng
    trade = get_trade(db_session, session.id, symbol)
    assert trade.status == "closed"
    assert trade.result in ["win", "loss", "breakeven"]
    assert trade.exit_price is not None
    assert trade.net_pnl is not None
```

### Lệnh kiểm tra

```bash
python -m pytest -v app/tests/test_trade_lifecycle.py::test_sell_t_plus_2_is_allowed
```

### Definition of Done

| Tiêu chí | Cách kiểm tra |
|----------|---------------|
| Test tồn tại | Function `test_sell_t_plus_2_is_allowed` |
| Test PASS | pytest hiện PASSED |
| Position closed | assert position.status == "closed" |
| Trade closed | assert trade.status == "closed" |

---

## S2-T4: Sửa Các Test Cũ Conflict Với T+2

**Estimate:** 2–3 giờ

### Mục tiêu

Tìm và sửa tất cả test cũ mà BUY rồi SELL/CLOSE ở T+1 nhưng expect thành công.

### Files liên quan

- `backend/app/tests/test_trade_lifecycle.py`
- `backend/app/tests/test_api_integration.py`

### Hướng dẫn từng bước

**Bước 1:** Grep tìm test có pattern "BUY rồi CLOSE/SELL ngay":

```bash
cd backend
grep -n "CLOSE\|SELL\|close\|sell" app/tests/test_trade_lifecycle.py
grep -n "CLOSE\|SELL\|close\|sell" app/tests/test_api_integration.py
```

**Bước 2:** Với mỗi test tìm được, đọc logic:
- Nếu test BUY ở index X rồi SELL ở index X hoặc X+1 và expect success → **SAI**, cần sửa.
- Nếu test BUY ở index X rồi SELL ở index X+2 trở lên → OK.

**Bước 3:** Sửa các test sai bằng cách thêm `advance_session` đủ 2 bước trước khi SELL:

```python
# TRƯỚC (sai - sell ở T+1):
buy(session, ...)
advance(session, 1)  # index = 1  
sell(session, ...)   # Expect success ← SAI vì T+2

# SAU (đúng - sell ở T+2):
buy(session, ...)
advance(session, 1)  # index = 1
advance(session, 1)  # index = 2
sell(session, ...)   # Expect success ← ĐÚNG vì T+2
```

**Bước 4:** Chạy lại tất cả tests đã sửa:

```bash
python -m pytest -v app/tests/test_trade_lifecycle.py app/tests/test_api_integration.py
```

### Definition of Done

| Tiêu chí | Cách kiểm tra |
|----------|---------------|
| Không còn test SELL ở T+1 expect success | Grep + đọc code |
| Tất cả test SELL đều ở T+2 trở lên | Đọc code confirm |
| Tests đã sửa pass | pytest pass |

### Sai lầm thường gặp

- ❌ Xóa test thay vì sửa → mất coverage
- ❌ Comment test ra → test suite fake pass
- ❌ Sửa logic T+2 trong service để test cũ pass → SAI HƯỚNG, T+2 là rule bất di bất dịch

---

## S2-T5: Fix Remaining Test Failures

**Estimate:** 2–4 giờ

### Mục tiêu

Sửa tất cả test failure còn lại sau S2-T2/T3/T4. Nếu test nào ngoài scope, ghi rõ reason.

### Hướng dẫn

**Bước 1:** Chạy full test suite:

```bash
python -m pytest -v app/tests
```

**Bước 2:** Với mỗi test fail:

| Nếu... | Thì... |
|---------|--------|
| Import error → thư viện thiếu | `pip install <library>` |
| Database error → schema sai | Kiểm tra model, có thể cần seed data |
| Assertion error → logic sai | Debug: print giá trị thực vs expected |
| Timeout → test chậm | Thêm timeout hoặc optimize |

**Bước 3:** Nếu có test THẬT SỰ ngoài scope (ví dụ test cho feature chưa implement), đánh dấu skip:

```python
@pytest.mark.skip(reason="Feature chưa implement - Sprint 5: Limit Order")
def test_limit_order_rejected_above_ceiling():
    pass
```

> [!WARNING]
> **KHÔNG** skip test chỉ vì lười sửa. Chỉ skip khi test thuộc feature chưa trong scope.

### Lệnh kiểm tra CUỐI CÙNG

```bash
cd backend
python -m pytest -q app/tests
```

### Definition of Done

| Tiêu chí | Cách kiểm tra |
|----------|---------------|
| pytest exit code 0 | Chạy xong không lỗi |
| Tất cả test pass hoặc skip có reason | Output không có FAILED |
| Skip tests có ghi rõ lý do | Đọc code confirm |

---

## 🏁 Definition of Done — Sprint 2 Tổng Thể

```bash
cd E:\Workspace\sumi\backend
.\.venv\Scripts\activate
python -m pytest -q app/tests
```

| # | Tiêu chí | Kết quả bắt buộc |
|---|----------|-------------------|
| 1 | pytest exit code | 0 |
| 2 | Test SELL ở T+1 | REJECTED (test pass) |
| 3 | Test SELL ở T+2 | SUCCESS (test pass) |
| 4 | Không test nào conflict T+2 rule | Đã sửa hết |
| 5 | Mỗi test fail ngoài scope có @skip + reason | Đã đánh dấu |

> [!CAUTION]
> **Sprint 2 chỉ hoàn thành khi paste được output `pytest -q app/tests` ALL PASS vào báo cáo.**
