# Sprint 6 — Analytics Standardization

> **Mục tiêu:** Analytics phản ánh đúng tiêu chuẩn tài chính. Không hard-code, không division-by-zero.  
> **Thời gian:** 4–5 ngày  
> **Phụ thuộc:** Sprint 4 (trade lifecycle đúng)

> [!IMPORTANT]
> Mọi công thức tài chính trong sprint này đều có gốc toán học rõ ràng. Dev phải tính tay verify trước khi tin output code.

---

## S6-T1: Dùng session.initial_cash Thay Vì Hard-Code

**Estimate:** 30 phút

### Vấn đề

`analytics_service.py` hard-code `current_equity = 100000.0` thay vì dùng `session.initial_cash`.

### File sửa

- `backend/app/services/analytics_service.py`

### Hướng dẫn

Tìm và thay tất cả hard-coded values:

```python
# TRƯỚC (sai):
current_equity = 100000.0
# hoặc
initial_capital = 100000

# SAU (đúng):
session = db.query(ReplaySession).filter(ReplaySession.id == session_id).first()
initial_capital = float(session.initial_cash)
```

### Test

```python
def test_analytics_uses_session_initial_cash(db_session):
    """Session với initial_cash=50,000,000 → analytics dùng 50M, không phải 100K."""
    session = create_session(db_session, "TEST", initial_cash=50_000_000)
    # ... thêm trades ...
    analytics = get_analytics(db_session, session.id)
    # Equity curve point đầu tiên phải gần 50M
    assert analytics["equity_curve"][0]["equity"] == pytest.approx(50_000_000, rel=0.01)
```

### Definition of Done

| Tiêu chí | Cách kiểm tra |
|----------|---------------|
| Grep "100000" trong analytics_service.py → không còn hard-code | grep |
| Test với initial_cash khác 100M pass | pytest |

---

## S6-T2: Equity Curve Chuẩn

**Estimate:** 4–6 giờ

### Công thức

Equity tại mỗi candle date:
$$\text{Equity}_t = \text{Cash}_t + \sum_{h \in \text{Holdings}} (\text{qty}_h \times \text{Close}_t)$$

### Luồng xử lý

```text
1. Bắt đầu: cash = initial_cash, holdings = {}
2. Với mỗi candle trong session (từ đầu đến current_index):
   a. Áp dụng executions xảy ra ở candle này:
      - BUY: cash -= net_amount, holdings[symbol] += qty
      - SELL: cash += net_amount, holdings[symbol] -= qty
   b. Mark-to-market: holdings_value = sum(qty * candle.close)
   c. equity = cash + holdings_value
   d. Ghi lại point: {date, equity, cash, holdings_value}
```

### Code mẫu

```python
async def build_equity_curve(self, db, session_id):
    session = db.query(ReplaySession).filter_by(id=session_id).first()
    initial_cash = float(session.initial_cash)
    
    # Lấy candles visible
    candles = self.replay_service.get_candles(db, session_id)
    
    # Lấy executions sorted by date
    executions = db.query(Execution).filter_by(session_id=session_id)\
        .order_by(Execution.execution_date).all()
    
    cash = initial_cash
    holdings = {}  # {symbol: {qty, avg_price}}
    equity_curve = []
    
    exec_index = 0
    
    for i, candle in enumerate(candles):
        candle_date = candle.trade_date if hasattr(candle, 'trade_date') else candle['time']
        
        # Áp dụng executions tại candle này
        while exec_index < len(executions):
            ex = executions[exec_index]
            ex_date = ex.execution_date
            if str(ex_date) > str(candle_date):
                break
            
            order = db.query(Order).filter_by(id=ex.order_id).first()
            if order.side == "BUY":
                cash -= float(ex.net_amount)
                sym = ex.symbol
                if sym not in holdings:
                    holdings[sym] = {"qty": 0, "avg_price": 0}
                holdings[sym]["qty"] += ex.quantity
            elif order.side == "SELL":
                cash += float(ex.net_amount)
                sym = ex.symbol
                if sym in holdings:
                    holdings[sym]["qty"] -= ex.quantity
                    if holdings[sym]["qty"] <= 0:
                        del holdings[sym]
            
            exec_index += 1
        
        # Mark-to-market
        close_price = float(candle.close if hasattr(candle, 'close') else candle['close'])
        holdings_value = sum(
            h["qty"] * close_price 
            for h in holdings.values()
        )
        
        equity = cash + holdings_value
        
        equity_curve.append({
            "time": str(candle_date),
            "equity": round(equity, 2),
            "cash": round(cash, 2),
            "holdings_value": round(holdings_value, 2),
        })
    
    return equity_curve
```

### Definition of Done

| Tiêu chí | Cách kiểm tra |
|----------|---------------|
| Equity curve point đầu = initial_cash | Test |
| Sau BUY, cash giảm | Test |
| Sau SELL, cash tăng | Test |
| Holdings mark-to-market dùng candle close | Test |
| Equity = cash + holdings_value | Test |

---

## S6-T3: Max Drawdown Đúng

**Estimate:** 2 giờ

### Công thức

$$\text{Peak}_t = \max(\text{Equity}_0, \text{Equity}_1, ..., \text{Equity}_t)$$

$$\text{Drawdown\%}_t = \frac{\text{Peak}_t - \text{Equity}_t}{\text{Peak}_t} \times 100$$

$$\text{MaxDrawdown\%} = \max(\text{Drawdown\%}_t)$$

$$\text{MaxDrawdownAmount} = \max(\text{Peak}_t - \text{Equity}_t)$$

### Code mẫu

```python
def calculate_max_drawdown(equity_curve: list) -> dict:
    if not equity_curve:
        return {"max_drawdown_amount": 0, "max_drawdown_pct": 0}
    
    peak = equity_curve[0]["equity"]
    max_dd_amount = 0
    max_dd_pct = 0
    
    for point in equity_curve:
        equity = point["equity"]
        if equity > peak:
            peak = equity
        
        dd_amount = peak - equity
        dd_pct = (dd_amount / peak * 100) if peak > 0 else 0
        
        if dd_amount > max_dd_amount:
            max_dd_amount = dd_amount
        if dd_pct > max_dd_pct:
            max_dd_pct = dd_pct
    
    return {
        "max_drawdown_amount": round(max_dd_amount, 2),
        "max_drawdown_pct": round(max_dd_pct, 2),
    }
```

### Test

```python
def test_max_drawdown():
    curve = [
        {"equity": 100}, {"equity": 110}, {"equity": 105},  # DD từ 110 xuống 105
        {"equity": 120}, {"equity": 96},  # DD từ 120 xuống 96 = 24, 20%
        {"equity": 115}
    ]
    result = calculate_max_drawdown(curve)
    assert result["max_drawdown_amount"] == 24  # 120 - 96
    assert result["max_drawdown_pct"] == pytest.approx(20.0)  # 24/120*100
```

---

## S6-T4: Drawdown Duration

**Estimate:** 2 giờ

### Công thức

Drawdown period bắt đầu khi equity giảm dưới peak, kết thúc khi equity vượt lại peak (hoặc đến cuối data).

### Code mẫu

```python
def calculate_drawdown_periods(equity_curve: list) -> list:
    periods = []
    peak = equity_curve[0]["equity"]
    in_drawdown = False
    dd_start = None
    dd_max_pct = 0
    
    for point in equity_curve:
        equity = point["equity"]
        
        if equity >= peak:
            if in_drawdown:
                # Kết thúc drawdown period
                periods.append({
                    "start": dd_start,
                    "end": point["time"],
                    "max_drawdown_pct": round(dd_max_pct, 2),
                })
                in_drawdown = False
                dd_max_pct = 0
            peak = equity
        else:
            if not in_drawdown:
                dd_start = point["time"]
                in_drawdown = True
            dd_pct = (peak - equity) / peak * 100
            dd_max_pct = max(dd_max_pct, dd_pct)
    
    # Nếu đang trong drawdown ở cuối data
    if in_drawdown:
        periods.append({
            "start": dd_start,
            "end": equity_curve[-1]["time"],
            "max_drawdown_pct": round(dd_max_pct, 2),
        })
    
    return periods
```

### Definition of Done

| Tiêu chí | Cách kiểm tra |
|----------|---------------|
| Drawdown periods detected correctly | Test with known curve |
| Ongoing drawdown included | Test where last point is below peak |

---

## S6-T5: Sharpe Ratio Đúng

**Estimate:** 2 giờ

### Công thức

$$R_t = \frac{\text{Equity}_t}{\text{Equity}_{t-1}} - 1$$

$$R_f^{daily} = (1 + 0.045)^{1/252} - 1 \approx 0.0001754$$

$$\text{Excess}_t = R_t - R_f^{daily}$$

$$\text{Sharpe} = \frac{\text{mean}(\text{Excess})}{\text{std}(\text{Excess})} \times \sqrt{252}$$

### Code mẫu

```python
import numpy as np

def calculate_sharpe_ratio(equity_curve: list, rf_annual: float = 0.045) -> float:
    if len(equity_curve) < 2:
        return None
    
    equities = [p["equity"] for p in equity_curve]
    
    # Daily returns
    returns = []
    for i in range(1, len(equities)):
        if equities[i-1] > 0:
            returns.append(equities[i] / equities[i-1] - 1)
    
    if not returns:
        return None
    
    returns = np.array(returns)
    
    # Daily risk-free rate
    rf_daily = (1 + rf_annual) ** (1/252) - 1
    
    # Excess returns
    excess = returns - rf_daily
    
    # Sharpe
    std = np.std(excess, ddof=1)  # Sample std
    if std == 0:
        return 0.0  # Không có biến động → Sharpe = 0
    
    sharpe = (np.mean(excess) / std) * np.sqrt(252)
    return round(float(sharpe), 4)
```

> [!WARNING]
> Nếu `std == 0` (không có biến động), trả `0.0` hoặc `None`. **KHÔNG BAO GIỜ** division by zero.

### Definition of Done

| Tiêu chí | Cách kiểm tra |
|----------|---------------|
| Sharpe dùng daily returns từ equity curve | Đọc code |
| Risk-free rate = 4.5% VN | Đọc code |
| std = 0 → không crash | Test edge case |
| Annualized bằng √252 | Đọc code |

---

## S6-T6: Profit Factor & Expectancy

**Estimate:** 1–2 giờ

### Profit Factor

```python
def calculate_profit_factor(trades: list) -> float:
    gross_profit = sum(t.net_pnl for t in trades if t.net_pnl > 0)
    gross_loss = abs(sum(t.net_pnl for t in trades if t.net_pnl < 0))
    
    if gross_loss == 0:
        return None  # Không có trade thua → không tính được
    
    return round(gross_profit / gross_loss, 4)
```

### Expectancy

```python
def calculate_expectancy(trades: list) -> float:
    if not trades:
        return None
    
    wins = [t for t in trades if t.net_pnl > 0]
    losses = [t for t in trades if t.net_pnl < 0]
    
    win_rate = len(wins) / len(trades)
    loss_rate = len(losses) / len(trades)
    
    avg_win = sum(t.net_pnl for t in wins) / len(wins) if wins else 0
    avg_loss = sum(t.net_pnl for t in losses) / len(losses) if losses else 0
    # avg_loss là số ÂM
    
    expectancy = win_rate * avg_win + loss_rate * avg_loss
    return round(expectancy, 2)
```

### Edge cases cần test

- Tất cả trades thắng → Profit Factor = None
- Tất cả trades thua → Profit Factor = 0
- Không có trades → Return None
- Chỉ 1 trade → Vẫn tính được

---

## S6-T7: VNINDEX Benchmark

**Estimate:** 2–3 giờ

### Logic

```text
1. Load VNINDEX candles cho cùng date range
2. Normalize: benchmark_value = close / first_close * initial_cash
3. Return series để frontend vẽ thêm line
```

### Code mẫu

```python
def get_benchmark_curve(self, db, session_id):
    session = db.query(ReplaySession).filter_by(id=session_id).first()
    initial_cash = float(session.initial_cash)
    
    # Lấy VNINDEX candles cùng date range
    vnindex_candles = db.query(Candle).filter(
        Candle.symbol == "VNINDEX",
        Candle.trade_date >= session.start_date,
        Candle.trade_date <= session.end_date
    ).order_by(Candle.trade_date).all()
    
    if not vnindex_candles:
        return None  # Không có data VNINDEX
    
    first_close = float(vnindex_candles[0].close)
    
    return [{
        "time": str(c.trade_date),
        "value": round(float(c.close) / first_close * initial_cash, 2)
    } for c in vnindex_candles]
```

### Definition of Done

| Tiêu chí | Cách kiểm tra |
|----------|---------------|
| Benchmark point đầu = initial_cash | Test |
| Normalize đúng | Test với known values |
| Không có VNINDEX data → return None | Test |

---

## S6-T8: Trade Distribution

**Estimate:** 1 giờ

### Logic đơn giản

Backend trả raw closed trades, frontend tự bin thành histogram.

```python
def get_trade_distribution(self, db, session_id):
    trades = db.query(Trade).filter(
        Trade.session_id == session_id,
        Trade.status == "closed"
    ).all()
    
    return [{
        "trade_id": t.id,
        "symbol": t.symbol,
        "net_pnl": float(t.net_pnl) if t.net_pnl else 0,
        "pnl_percent": float(t.pnl_percent) if t.pnl_percent else 0,
        "r_multiple": float(t.r_multiple) if t.r_multiple else None,
        "result": t.result,
    } for t in trades]
```

---

## 🏁 Definition of Done — Sprint 6 Tổng Thể

| # | Tiêu chí | Cách kiểm tra |
|---|----------|---------------|
| 1 | Không hard-code initial_cash | grep "100000" → 0 matches |
| 2 | Equity curve đúng | Test with known scenario |
| 3 | Max drawdown amount + pct đúng | Test pass |
| 4 | Drawdown periods detected | Test pass |
| 5 | Sharpe dùng equity returns, no div/0 | Test pass |
| 6 | Profit Factor, Expectancy đúng | Test edge cases pass |
| 7 | VNINDEX benchmark nếu có data | Test pass |
| 8 | Trade distribution data available | API test |
| 9 | All backend tests pass | `pytest -q` exit 0 |
