# Sprint 8 — Algorithmic Backtest Engine MVP

> **Mục tiêu:** User có thể chạy backtest cho strategy đơn giản (MA crossover), nhận kết quả đúng, dùng lại toàn bộ hạ tầng trade lifecycle.  
> **Thời gian:** 4–5 ngày  
> **Phụ thuộc:** Sprint 3 (no leak) + Sprint 4 (lifecycle) + Sprint 6 (analytics)

> [!IMPORTANT]
> Engine backtest PHẢI reuse trade lifecycle service. KHÔNG copy-paste logic BUY/SELL/fee/tax. Nếu trade lifecycle có bug → backtest cũng bug. Đó là intentional — fix 1 chỗ là fix hết.

---

## S8-T1: Define Strategy YAML Schema

**Estimate:** 1–2 giờ

### Mục tiêu

Định nghĩa format YAML cho strategy để user viết strategy dễ dàng.

### File tạo

- `backend/app/domain/strategy/strategy_schema.py`
- `backend/app/domain/strategy/examples/ma_crossover.yaml`

### YAML format mẫu

```yaml
# ma_crossover.yaml
name: "MA Crossover Simple"
version: "1.0"
description: "Mua khi SMA ngắn cắt lên SMA dài, bán khi ngược lại"

# Indicators cần tính
indicators:
  - name: sma_short
    type: sma
    length: 10
  - name: sma_long
    type: sma
    length: 50

# Signals / Rules
entry_rules:
  # Tất cả conditions phải TRUE → BUY signal
  - condition: "sma_short > sma_long"
    description: "SMA ngắn cắt lên SMA dài"
  - condition: "previous_sma_short <= previous_sma_long"
    description: "Crossover: phiên trước SMA ngắn chưa vượt SMA dài"

exit_rules:
  - condition: "sma_short < sma_long"
    description: "SMA ngắn cắt xuống SMA dài"

# Position sizing
position_sizing:
  method: "fixed_quantity"
  quantity: 1000
  # Alternatives:
  # method: "percent_equity"
  # percent: 10  # 10% equity

# Risk management (optional)
risk_management:
  stop_loss_pct: null  # Không dùng stop loss
  take_profit_pct: null
```

### Schema Python

```python
# backend/app/domain/strategy/strategy_schema.py
from pydantic import BaseModel
from typing import Optional, List

class IndicatorConfig(BaseModel):
    name: str
    type: str  # sma, ema, rsi, etc.
    length: int

class PositionSizing(BaseModel):
    method: str = "fixed_quantity"
    quantity: Optional[int] = None
    percent: Optional[float] = None

class RiskManagement(BaseModel):
    stop_loss_pct: Optional[float] = None
    take_profit_pct: Optional[float] = None

class StrategyConfig(BaseModel):
    name: str
    version: str = "1.0"
    description: str = ""
    indicators: List[IndicatorConfig]
    entry_rules: List[dict]
    exit_rules: List[dict]
    position_sizing: PositionSizing
    risk_management: Optional[RiskManagement] = None
```

### Definition of Done

| Tiêu chí | Cách kiểm tra |
|----------|---------------|
| YAML file tồn tại | `ls examples/` |
| Schema Python parse được YAML | Unit test |
| YAML đọc lên → StrategyConfig object | Test |

---

## S8-T2: Parse & Load Strategies

**Estimate:** 2–3 giờ

### File tạo

- `backend/app/domain/strategy/strategy_loader.py`

### Hướng dẫn

```python
# backend/app/domain/strategy/strategy_loader.py
import yaml
from pathlib import Path
from .strategy_schema import StrategyConfig

def load_strategy_from_yaml(path: str) -> StrategyConfig:
    """Load strategy config từ YAML file."""
    with open(path, 'r', encoding='utf-8') as f:
        data = yaml.safe_load(f)
    return StrategyConfig(**data)

def load_strategy_from_dict(data: dict) -> StrategyConfig:
    """Load strategy config từ dict (API input)."""
    return StrategyConfig(**data)

def list_available_strategies(directory: str = None) -> list:
    """Liệt kê strategies có sẵn trong directory."""
    if directory is None:
        directory = str(Path(__file__).parent / "examples")
    
    strategies = []
    for f in Path(directory).glob("*.yaml"):
        try:
            config = load_strategy_from_yaml(str(f))
            strategies.append({
                "filename": f.name,
                "name": config.name,
                "description": config.description,
            })
        except Exception:
            pass  # Skip invalid files
    
    return strategies
```

### Definition of Done

| Tiêu chí | Cách kiểm tra |
|----------|---------------|
| load_strategy_from_yaml hoạt động | Test with sample YAML |
| Invalid YAML → error rõ ràng | Test exception |
| list_available_strategies trả list | Test |

---

## S8-T3: Implement run_backtest Engine

**Estimate:** 6–8 giờ

### Logic chính

```text
1. Load strategy config
2. Load candles cho symbol + date range
3. Precompute indicators
4. Iterate từng candle:
   a. Evaluate signals dựa trên indicator values
   b. Nếu entry signal TRUE + chưa có position → BUY
   c. Nếu exit signal TRUE + có position → SELL (check T+2!)
   d. Check stop_loss / take_profit
5. Dùng trade_lifecycle_service cho BUY/SELL
6. Trả analytics kết quả
```

### File tạo

- `backend/app/services/backtest_service.py`

### Code mẫu

```python
# backend/app/services/backtest_service.py
import pandas as pd
import numpy as np
from app.domain.strategy.strategy_loader import load_strategy_from_dict
from app.services.trade_lifecycle_service import TradeLifecycleService
from app.services.analytics_service import AnalyticsService

class BacktestService:
    def __init__(self):
        self.trade_service = TradeLifecycleService()
        self.analytics_service = AnalyticsService()
    
    async def run_backtest(self, db, config: dict) -> dict:
        """
        Chạy backtest cho 1 strategy trên 1 symbol.
        
        Args:
            config: {
                "symbol": "VNM",
                "start_date": "2023-01-01",
                "end_date": "2024-01-01",
                "initial_cash": 100000000,
                "strategy": { ... strategy config ... }
            }
        """
        strategy = load_strategy_from_dict(config["strategy"])
        
        # 1. Create virtual session
        session = self._create_backtest_session(db, config)
        
        # 2. Load ALL candles for symbol (backtest dùng full history, ko phải replay)
        candles = self._load_candles(db, config["symbol"], 
                                      config["start_date"], config["end_date"])
        
        if len(candles) == 0:
            return {"error": "No candles found", "total_candles": 0}
        
        # 3. Precompute indicators
        df = self._candles_to_dataframe(candles)
        indicator_values = self._compute_indicators(df, strategy.indicators)
        
        # 4. Iterate candles
        has_position = False
        buy_candle_index = None
        
        for i in range(1, len(df)):  # Start from 1 to have previous values
            session.current_index = i
            
            current = self._get_indicator_snapshot(indicator_values, i)
            previous = self._get_indicator_snapshot(indicator_values, i - 1)
            
            candle = candles[i]
            close_price = float(df.iloc[i]["close"])
            
            if not has_position:
                # Check entry signals
                if self._evaluate_rules(strategy.entry_rules, current, previous):
                    # BUY
                    try:
                        quantity = self._calculate_quantity(
                            strategy.position_sizing, session, close_price
                        )
                        await self.trade_service.execute_decision(
                            db, session, action="BUY",
                            price=close_price, quantity=quantity
                        )
                        has_position = True
                        buy_candle_index = i
                    except Exception:
                        pass  # Insufficient cash, etc.
            
            else:
                # Check T+2 first
                if i - buy_candle_index < 2:
                    continue  # Can't sell yet
                
                # Check exit signals
                if self._evaluate_rules(strategy.exit_rules, current, previous):
                    # SELL
                    try:
                        await self.trade_service.execute_decision(
                            db, session, action="CLOSE",
                            price=close_price, quantity=None  # Close all
                        )
                        has_position = False
                        buy_candle_index = None
                    except Exception:
                        pass
        
        # 5. Return analytics
        analytics = await self.analytics_service.get_session_analytics(db, session.id)
        
        return {
            "session_id": session.id,
            "strategy": strategy.name,
            "symbol": config["symbol"],
            "total_candles": len(candles),
            "analytics": analytics,
        }
    
    def _evaluate_rules(self, rules, current, previous) -> bool:
        """
        Evaluate rule conditions.
        Tất cả rules phải TRUE → signal TRUE.
        """
        for rule in rules:
            condition = rule["condition"]
            # Simple evaluation (cho MVP):
            # Replace indicator names với values
            expr = condition
            for key, val in current.items():
                expr = expr.replace(key, str(val))
            for key, val in previous.items():
                expr = expr.replace(f"previous_{key}", str(val))
            
            try:
                if not eval(expr):  # ⚠️ eval cho MVP, sẽ refactor sau
                    return False
            except Exception:
                return False
        
        return True
```

> [!WARNING]
> **`eval()` KHÔNG AN TOÀN** cho production. Đây chỉ là MVP. Sprint tương lai sẽ thay bằng safe expression parser. Ghi TODO rõ trong code.

### Definition of Done

| Tiêu chí | Cách kiểm tra |
|----------|---------------|
| run_backtest hoạt động | Test with MA crossover |
| Dùng trade_lifecycle_service cho BUY/SELL | Đọc code confirm |
| T+2 rule được tuân thủ trong backtest | Test: BUY at i=5, check no SELL at i=6 |
| Fee/Tax tính đúng | Check trades qua analytics |
| Analytics returned | Response có analytics data |

---

## S8-T4: Backtest API Endpoint

**Estimate:** 1–2 giờ

### File sửa

- `backend/app/api/backtest.py` (tạo mới)
- `backend/app/main.py` (register router)

### Hướng dẫn

```python
# backend/app/api/backtest.py
from fastapi import APIRouter, Depends
from sqlalchemy.orm import Session
from app.db import get_db
from app.services.backtest_service import BacktestService

router = APIRouter(prefix="/api/backtest", tags=["backtest"])
backtest_service = BacktestService()

@router.post("/run")
async def run_backtest(config: dict, db: Session = Depends(get_db)):
    """
    Chạy backtest cho strategy.
    
    Request body:
    {
        "symbol": "VNM",
        "start_date": "2023-01-01", 
        "end_date": "2024-01-01",
        "initial_cash": 100000000,
        "strategy": { ... }
    }
    """
    result = await backtest_service.run_backtest(db, config)
    return result

@router.get("/strategies")
async def list_strategies():
    """Liệt kê strategies có sẵn."""
    from app.domain.strategy.strategy_loader import list_available_strategies
    return list_available_strategies()
```

### Đăng ký router

```python
# backend/app/main.py
from app.api.backtest import router as backtest_router
app.include_router(backtest_router)
```

### Definition of Done

| Tiêu chí | Cách kiểm tra |
|----------|---------------|
| POST /api/backtest/run hoạt động | curl test |
| GET /api/backtest/strategies trả list | curl test |
| Swagger docs hiện endpoints | Open /docs |

---

## S8-T5: Tạo MA Crossover Strategy Mẫu

**Estimate:** 1 giờ

### File tạo

- `backend/app/domain/strategy/examples/ma_crossover.yaml`

### Nội dung

```yaml
name: "MA Crossover Simple"
version: "1.0"
description: |
  Strategy giao dịch đơn giản dựa trên MA crossover:
  - BUY: SMA(10) cắt lên SMA(50)
  - SELL: SMA(10) cắt xuống SMA(50)
  - Position sizing: fixed 1000 cổ
  - Không dùng stop loss / take profit

indicators:
  - name: sma_short
    type: sma
    length: 10
  - name: sma_long
    type: sma
    length: 50

entry_rules:
  - condition: "sma_short > sma_long"
    description: "SMA 10 > SMA 50"
  - condition: "previous_sma_short <= previous_sma_long"
    description: "Crossover xảy ra ở candle này"

exit_rules:
  - condition: "sma_short < sma_long"
    description: "SMA 10 < SMA 50 (death cross)"

position_sizing:
  method: fixed_quantity
  quantity: 1000
```

### Definition of Done

| Tiêu chí | Cách kiểm tra |
|----------|---------------|
| File YAML tồn tại | `cat` file |
| Parse thành công | Python test |
| Logic crossover đúng | Review conditions |

---

## S8-T6: Test E2E Backtest

**Estimate:** 3–4 giờ

### File tạo

- `backend/app/tests/test_backtest.py`

### Test scenario

```python
def test_backtest_ma_crossover_e2e(db_session):
    """
    Test MA crossover backtest:
    
    1. Seed 100 candles với pattern rõ:
       - Candles 0-30: giá tăng 100→130 (SMA10 > SMA50 ≈ Golden Cross ở ~candle 50)
       - Candles 30-60: giá giảm 130→100
       - Candles 60-90: giá tăng 100→130
    2. Run backtest với MA(10,50)
    3. Verify ít nhất 1 trade closed
    4. Verify analytics returned
    5. Verify no trade at T+1
    """
    symbol = "BACKTEST_TEST"
    
    # Seed candles: up-down-up pattern
    import math
    base_date = date(2024, 1, 1)
    for i in range(100):
        # Sine wave pattern for clear MA crossovers
        price = 100 + 30 * math.sin(i * 0.06)
        create_candle(
            db_session, symbol,
            trade_date=base_date + timedelta(days=i),
            open=price - 0.5, high=price + 1, low=price - 1, close=price,
            volume=1000000
        )
    
    # Run backtest
    config = {
        "symbol": symbol,
        "start_date": str(base_date),
        "end_date": str(base_date + timedelta(days=99)),
        "initial_cash": 100_000_000,
        "strategy": {
            "name": "MA Crossover Test",
            "indicators": [
                {"name": "sma_short", "type": "sma", "length": 10},
                {"name": "sma_long", "type": "sma", "length": 30},  # 30 thay vì 50 để có crossover trong 100 candles
            ],
            "entry_rules": [
                {"condition": "sma_short > sma_long"},
                {"condition": "previous_sma_short <= previous_sma_long"},
            ],
            "exit_rules": [
                {"condition": "sma_short < sma_long"},
            ],
            "position_sizing": {
                "method": "fixed_quantity",
                "quantity": 1000,
            },
        }
    }
    
    result = await backtest_service.run_backtest(db_session, config)
    
    # Verify
    assert result["total_candles"] == 100
    assert result["analytics"] is not None
    
    # Có ít nhất 1 trade
    trades = get_trades(db_session, result["session_id"])
    assert len(trades) >= 1
    
    # Trades closed đúng
    closed_trades = [t for t in trades if t.status == "closed"]
    for trade in closed_trades:
        assert trade.result in ["win", "loss", "breakeven"]
        assert trade.net_pnl is not None
        assert trade.entry_price is not None
        assert trade.exit_price is not None
```

### Definition of Done

| Tiêu chí | Cách kiểm tra |
|----------|---------------|
| Test pass | pytest PASSED |
| Ít nhất 1 trade closed | Assert trong test |
| Analytics returned | Assert trong test |
| No exception / crash | Test completes cleanly |
| T+2 respected | Review trade indices |

---

## 🏁 Definition of Done — Sprint 8 Tổng Thể

| # | Tiêu chí | Cách kiểm tra |
|---|----------|---------------|
| 1 | Strategy YAML schema defined | File + Python schema tồn tại |
| 2 | YAML loader hoạt động | Unit test |
| 3 | run_backtest engine hoạt động | E2E test pass |
| 4 | API POST /api/backtest/run | curl test |
| 5 | MA crossover strategy mẫu | YAML file + test |
| 6 | Reuses trade_lifecycle_service | Đọc code confirm |
| 7 | T+2 rule in backtest | Test verify |
| 8 | Analytics included in result | Assert |
| 9 | All backend tests pass | `pytest -q` exit 0 |
| 10 | Frontend build pass | `npm run build` exit 0 |

> [!CAUTION]
> **Sprint 8 kết thúc MVP core.** Sau sprint này, dự án chuyển sang phase Enhancement (xem FUTURE_ROADMAP.md): Multi-timeframe, Advanced Position Sizing, Risk Budgeting, Strategy Optimization.
