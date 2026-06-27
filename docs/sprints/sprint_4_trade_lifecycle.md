# Sprint 4 — Trade Lifecycle Hardening

> **Mục tiêu:** Fee/Tax/PnL tính đúng cho mọi trade. trade.result phản ánh đúng win/loss/breakeven.  
> **Thời gian:** 2–3 ngày  
> **Phụ thuộc:** Sprint 2 (backend tests green)  

> [!IMPORTANT]
> Sprint này sửa logic tài chính cốt lõi. Mọi con số phải ĐÚNG. Dùng phép tính tay để verify, không tin code mù quáng.

---

## S4-T1: Fix Trade Result Assignment

**Estimate:** 1–2 giờ

### Vấn đề

Trong `_execute_sell`, khi close position, code chỉ set `trade.result = 'breakeven'` trong nhánh `else` của `initial_risk`. Không set `win` / `loss` chuẩn dựa trên `net_pnl`.

### File sửa

- `backend/app/services/trade_lifecycle_service.py`

### Hướng dẫn

**Bước 1:** Tìm function `_execute_sell` (hoặc tương tự) trong `trade_lifecycle_service.py`.

**Bước 2:** Tìm chỗ set `trade.result`. Hiện tại có thể chỉ set trong nhánh `initial_risk`.

**Bước 3:** Thêm logic set result NGOÀI nhánh initial_risk, dựa trên net_pnl:

```python
# SAU khi tính trade.net_pnl (bất kể có initial_risk hay không):
if trade.net_pnl > 0:
    trade.result = "win"
elif trade.net_pnl < 0:
    trade.result = "loss"
else:
    trade.result = "breakeven"
```

> [!WARNING]
> Đặt logic này SAU khi đã tính xong `trade.net_pnl`. Nếu đặt trước → sai.

**Bước 4:** Viết 3 unit tests:

```python
def test_close_with_profit_result_is_win(db_session):
    """Mua 100, bán 110 → result = win"""
    # Setup: BUY ở giá 100, advance T+2, SELL ở giá 110
    # Assert: trade.result == "win"
    # Assert: trade.net_pnl > 0

def test_close_with_loss_result_is_loss(db_session):
    """Mua 100, bán 90 → result = loss"""
    # Setup: BUY ở giá 100, advance T+2, SELL ở giá 90  
    # Assert: trade.result == "loss"
    # Assert: trade.net_pnl < 0

def test_close_breakeven_result_is_breakeven(db_session):
    """Mua 100, bán 100 → result = breakeven (nếu fee làm lỗ nhẹ thì vẫn loss)"""
    # Lưu ý: nếu có fee, bán cùng giá vẫn lỗ vì fee
    # → Nếu fee > 0, thì trade.result sẽ là "loss", không phải "breakeven"
    # → Test cần reflect đúng reality
```

### Lệnh kiểm tra

```bash
python -m pytest -v app/tests/test_trade_lifecycle.py -k "result"
```

### Definition of Done

| Tiêu chí | Cách kiểm tra |
|----------|---------------|
| trade.result = "win" khi net_pnl > 0 | Test pass |
| trade.result = "loss" khi net_pnl < 0 | Test pass |
| trade.result = "breakeven" khi net_pnl = 0 | Test pass |
| Logic nằm NGOÀI nhánh initial_risk | Đọc code confirm |

---

## S4-T2: Populate Fee/Tax Trong Execution Records

**Estimate:** 1–2 giờ

### Vấn đề

Model `Execution` có fields `fee`, `tax`, `slippage` nhưng khi tạo Execution, code không truyền giá trị → default 0.

### File sửa

- `backend/app/services/trade_lifecycle_service.py`

### Công thức phí

| Loại | Công thức | Giá trị mặc định |
|------|-----------|-------------------|
| Phí mua | `gross_amount × 0.0015` | 0.15% |
| Phí bán | `gross_amount × 0.0015` | 0.15% |
| Thuế bán | `gross_amount × 0.001` | 0.1% |
| Phí mua + thuế mua | fee only (không thuế khi mua) | |
| Phí bán + thuế bán | `fee + tax = 0.25%` | |

### Hướng dẫn

**Bước 1:** Tìm chỗ tạo `Execution` cho BUY:

```python
# TRƯỚC (thiếu fee/tax):
execution = Execution(
    order_id=order.id,
    session_id=session.id,
    symbol=symbol,
    execution_date=candle_date,
    execution_price=price,
    quantity=quantity,
    gross_amount=gross_amount,
    net_amount=net_amount,
)

# SAU (có fee/tax):
fee = gross_amount * 0.0015  # 0.15% phí mua
tax = 0.0  # Không có thuế khi mua
net_amount = gross_amount + fee  # Tổng tiền phải trả = giá trị + phí

execution = Execution(
    order_id=order.id,
    session_id=session.id,
    symbol=symbol,
    execution_date=candle_date,
    execution_price=price,
    quantity=quantity,
    fee=fee,
    tax=tax,
    slippage=0.0,
    gross_amount=gross_amount,
    net_amount=net_amount,
)
```

**Bước 2:** Tìm chỗ tạo `Execution` cho SELL:

```python
fee = gross_amount * 0.0015  # 0.15% phí bán
tax = gross_amount * 0.001   # 0.1% thuế bán
net_amount = gross_amount - fee - tax  # Tiền thực nhận = giá trị - phí - thuế

execution = Execution(
    order_id=order.id,
    session_id=session.id,
    symbol=symbol,
    execution_date=candle_date,
    execution_price=price,
    quantity=quantity,
    fee=fee,
    tax=tax,
    slippage=0.0,
    gross_amount=gross_amount,
    net_amount=net_amount,
)
```

**Bước 3:** Viết test:

```python
def test_buy_execution_has_correct_fee(db_session):
    """BUY 1000 cổ giá 100 → fee = 1000*100*0.0015 = 150"""
    # BUY
    # Query execution
    assert execution.fee == pytest.approx(150.0, rel=0.01)
    assert execution.tax == 0.0
    assert execution.gross_amount == 100000.0  # 1000 * 100
    assert execution.net_amount == pytest.approx(100150.0, rel=0.01)  # 100000 + 150

def test_sell_execution_has_correct_fee_and_tax(db_session):
    """SELL 1000 cổ giá 110 → fee=165, tax=110"""
    # SELL
    # gross = 1000 * 110 = 110000
    # fee = 110000 * 0.0015 = 165
    # tax = 110000 * 0.001 = 110
    assert execution.fee == pytest.approx(165.0, rel=0.01)
    assert execution.tax == pytest.approx(110.0, rel=0.01)
    assert execution.net_amount == pytest.approx(110000 - 165 - 110, rel=0.01)
```

### Definition of Done

| Tiêu chí | Cách kiểm tra |
|----------|---------------|
| BUY execution có fee > 0 | Test pass |
| BUY execution tax = 0 | Test pass |
| SELL execution fee > 0 | Test pass |
| SELL execution tax > 0 | Test pass |
| net_amount phản ánh phí/thuế | Test pass |

---

## S4-T3: Fix Net PnL Calculation

**Estimate:** 2–3 giờ

### Vấn đề

Hiện `trade.net_pnl = position.realized_pnl` — chưa tính đến phí/thuế.

### Công thức đúng

```text
buy_cash_out  = BUY quantity × BUY price × (1 + buy_fee_rate)
sell_cash_in  = SELL quantity × SELL price × (1 - sell_fee_rate - sell_tax_rate)
net_pnl       = sell_cash_in - buy_cash_out
```

### Ví dụ cụ thể

| Thông số | Giá trị |
|----------|---------|
| Mua 1000 cổ giá 100 | |
| buy_gross | 1000 × 100 = 100,000 |
| buy_fee | 100,000 × 0.0015 = 150 |
| buy_cash_out | 100,000 + 150 = **100,150** |
| | |
| Bán 1000 cổ giá 110 | |
| sell_gross | 1000 × 110 = 110,000 |
| sell_fee | 110,000 × 0.0015 = 165 |
| sell_tax | 110,000 × 0.001 = 110 |
| sell_cash_in | 110,000 - 165 - 110 = **109,725** |
| | |
| **net_pnl** | 109,725 - 100,150 = **9,575** |
| **gross_pnl** | (110 - 100) × 1000 = **10,000** |
| **pnl_percent** | 9,575 / 100,150 × 100 = **9.56%** |

### File sửa

- `backend/app/services/trade_lifecycle_service.py`

### Hướng dẫn

**Bước 1:** Tìm chỗ tính `trade.net_pnl` khi close:

```python
# TRƯỚC (sai):
trade.net_pnl = position.realized_pnl

# SAU (đúng):
# Cách 1: Dùng execution net_amount (đơn giản nhất cho full close)
buy_executions = [e for e in session_executions 
                  if e.symbol == symbol and e.order.side == "BUY"]
sell_executions = [e for e in session_executions 
                   if e.symbol == symbol and e.order.side == "SELL"]

buy_cash_out = sum(e.net_amount for e in buy_executions)
sell_cash_in = sum(e.net_amount for e in sell_executions)
trade.net_pnl = sell_cash_in - buy_cash_out

# Cách 2: Nếu execution chưa link đủ, tính trực tiếp
trade.gross_pnl = (exit_price - entry_price) * quantity
total_buy_fee = entry_price * quantity * 0.0015
total_sell_cost = exit_price * quantity * (0.0015 + 0.001)
trade.net_pnl = trade.gross_pnl - total_buy_fee - total_sell_cost
```

**Bước 2:** Cập nhật `trade.pnl_percent`:

```python
trade.pnl_percent = (trade.net_pnl / buy_cash_out) * 100
```

### Test với số cụ thể

```python
def test_net_pnl_includes_fees_and_tax(db_session):
    """
    BUY 1000 cổ giá 100.
    SELL 1000 cổ giá 110.
    Expected net_pnl = 9575.0
    """
    # Setup: BUY, advance T+2, SELL
    # ...
    
    trade = get_closed_trade(db_session, session.id)
    
    # Gross PnL (không fee): (110-100)*1000 = 10000
    assert trade.gross_pnl == pytest.approx(10000.0, rel=0.01)
    
    # Net PnL (sau fee/tax): 9575
    assert trade.net_pnl == pytest.approx(9575.0, rel=0.01)
    
    # PnL percent: 9575 / 100150 * 100 ≈ 9.56%
    assert trade.pnl_percent == pytest.approx(9.56, abs=0.1)
```

### Definition of Done

| Tiêu chí | Cách kiểm tra |
|----------|---------------|
| net_pnl = sell_cash_in - buy_cash_out | Test with exact numbers pass |
| gross_pnl = (exit - entry) × qty | Test pass |
| pnl_percent dựa trên chi phí thực | Test pass |
| Không division by zero | Edge case test |

---

## S4-T4: Comprehensive E2E Trade Lifecycle Test

**Estimate:** 2–3 giờ

### Mục tiêu

Một test duy nhất cover toàn bộ luồng: BUY → advance T+2 → SELL → verify tất cả entities.

### Test code

```python
def test_full_trade_lifecycle_e2e(db_session):
    """
    E2E test cho toàn bộ trade lifecycle:
    
    1. Seed 10 candles giá tăng 100→109
    2. Create session, initial_cash=100,000,000
    3. BUY 1000 cổ tại candle index 0, giá 100
    4. Verify: Decision, Order, Execution, Position, Trade entities
    5. Advance 2 steps (T+2)
    6. SELL 1000 cổ tại candle index 2, giá 102
    7. Verify: All entities updated correctly
    8. Verify: PnL calculations correct
    """
    symbol = "E2E_TEST"
    initial_cash = 100_000_000.0
    
    # ===== SETUP =====
    # Seed 10 candles: price 100, 101, 102, ...
    for i in range(10):
        create_candle(db_session, symbol, 
                      trade_date=date(2024,1,1) + timedelta(days=i),
                      open=100.0+i, high=101.0+i, low=99.0+i, close=100.0+i,
                      volume=1000000)
    
    session = create_session(db_session, symbol, initial_cash=initial_cash)
    
    # ===== BUY =====
    buy_price = 100.0
    buy_qty = 1000
    buy_response = execute_decision(db_session, session, 
                                     action="BUY", price=buy_price, quantity=buy_qty)
    
    # Verify Decision
    decision = get_latest_decision(db_session, session.id)
    assert decision.action == "BUY"
    assert decision.candle_index == 0
    
    # Verify Order
    order = get_latest_order(db_session, session.id)
    assert order.side == "BUY"
    assert order.status == "executed"
    
    # Verify Execution
    execution = get_latest_execution(db_session, session.id)
    buy_gross = buy_price * buy_qty  # 100,000
    buy_fee = buy_gross * 0.0015     # 150
    assert execution.gross_amount == pytest.approx(buy_gross)
    assert execution.fee == pytest.approx(buy_fee)
    assert execution.tax == 0.0
    assert execution.net_amount == pytest.approx(buy_gross + buy_fee)  # 100,150
    
    # Verify Position
    position = get_position(db_session, session.id, symbol)
    assert position.status == "open"
    assert position.quantity == buy_qty
    assert position.average_price == pytest.approx(buy_price)
    
    # Verify Trade
    trade = get_trade(db_session, session.id, symbol)
    assert trade.status == "open"
    assert trade.entry_price == pytest.approx(buy_price)
    
    # Verify Cash
    assert session.current_cash == pytest.approx(initial_cash - buy_gross - buy_fee)
    
    # ===== ADVANCE T+2 =====
    advance_session(db_session, session, steps=2)
    assert session.current_index == 2
    
    # ===== SELL =====
    sell_price = 102.0  # Candle index 2 close price
    sell_response = execute_decision(db_session, session,
                                      action="CLOSE", price=sell_price, quantity=buy_qty)
    
    # Verify Sell Execution
    sell_execution = get_latest_execution(db_session, session.id)
    sell_gross = sell_price * buy_qty  # 102,000
    sell_fee = sell_gross * 0.0015     # 153
    sell_tax = sell_gross * 0.001      # 102
    assert sell_execution.fee == pytest.approx(sell_fee)
    assert sell_execution.tax == pytest.approx(sell_tax)
    assert sell_execution.net_amount == pytest.approx(sell_gross - sell_fee - sell_tax)  # 101,745
    
    # Verify Position Closed
    position = get_position(db_session, session.id, symbol)
    assert position.status == "closed"
    assert position.quantity == 0
    
    # Verify Trade Closed
    trade = get_trade(db_session, session.id, symbol)
    assert trade.status == "closed"
    assert trade.result == "win"
    assert trade.exit_price == pytest.approx(sell_price)
    
    # Net PnL = sell_cash_in - buy_cash_out
    # = 101,745 - 100,150 = 1,595
    assert trade.net_pnl == pytest.approx(1595.0, rel=0.02)
    
    # Verify Final Cash
    expected_cash = initial_cash - (buy_gross + buy_fee) + (sell_gross - sell_fee - sell_tax)
    assert session.current_cash == pytest.approx(expected_cash)
```

### Definition of Done

| Tiêu chí | Cách kiểm tra |
|----------|---------------|
| Test pass | `pytest -v ...::test_full_trade_lifecycle_e2e` PASSED |
| Tất cả entities được verify | Decision, Order, Execution, Position, Trade |
| PnL tính đúng với số cụ thể | Assert với pytest.approx |
| Cash cuối cùng đúng | initial_cash - buy_cost + sell_revenue |

---

## 🏁 Definition of Done — Sprint 4 Tổng Thể

```bash
cd backend
python -m pytest -q app/tests
```

| # | Tiêu chí | Kết quả |
|---|----------|---------|
| 1 | trade.result = win/loss/breakeven chính xác | 3 tests pass |
| 2 | Execution.fee, Execution.tax populated | Tests pass |
| 3 | net_pnl = sell_cash_in - buy_cash_out | Test với số cụ thể pass |
| 4 | E2E lifecycle test pass | Full test pass |
| 5 | All backend tests pass | `pytest -q` exit 0 |
