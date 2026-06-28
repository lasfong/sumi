# Implementation Playbook - Sumi Professional Completion Plan

## 1. Cách dùng tài liệu này

Tài liệu này là kế hoạch triển khai chi tiết cho dev follow từng bước.

Nguyên tắc:

- Làm theo thứ tự phase.
- Không nhảy phase khi phase trước chưa pass Definition of Done.
- Mỗi task phải có code, test và xác nhận bằng command.
- Không sửa lung tung ngoài phạm vi task.
- Không thêm feature mới khi build/test đang đỏ.
- Không tin "chạy được trên máy em" nếu không có command output.

Mục tiêu trước mắt không phải làm thật nhiều feature. Mục tiêu là biến code hiện tại thành nền tảng đáng tin, rồi mới phát triển tiếp.

## 2. Chuẩn làm việc bắt buộc

### 2.1. Trước khi code

Dev phải chạy:

```bash
git status --short
```

Ghi lại file nào đang dirty.

Nếu task đụng vào file đã có thay đổi của người khác, phải đọc file kỹ trước khi sửa.

### 2.2. Sau mỗi task

Dev phải chạy tối thiểu:

```bash
cd frontend
npm.cmd run build
```

Nếu task backend:

```bash
cd backend
python -m pytest -q app/tests
```

Nếu task full-stack:

```bash
cd backend
python -m pytest -q app/tests

cd ../frontend
npm.cmd run build
```

### 2.3. Format báo cáo task

Mỗi task khi báo cáo phải có:

```text
Task:
Files changed:
What changed:
Tests run:
Result:
Known limitations:
```

Không báo "done" nếu chưa chạy test/build tương ứng.

## 3. Phase 0 - Environment và baseline

Mục tiêu: có môi trường dev chạy được test/build.

### Task 0.1 - Kiểm tra runtime

Chạy:

```bash
node -v
npm -v
python --version
```

Nếu Windows không có `python`, thử:

```bash
py --version
```

Kết quả mong muốn:

- Node 18+
- Python 3.10+
- npm chạy được

Nếu thiếu Python, cài Python trước khi làm backend.

### Task 0.2 - Tạo backend venv

Trong `backend`:

```bash
python -m venv .venv
.\.venv\Scripts\activate
python -m pip install --upgrade pip
python -m pip install -r requirements.txt
```

Kiểm tra:

```bash
python -m pytest --version
python -c "import fastapi, sqlalchemy, pandas; print('ok')"
```

Definition of Done:

- Venv tạo thành công.
- Install requirements thành công.
- `pytest --version` chạy được.

### Task 0.3 - Cài frontend dependencies

Trong `frontend`:

```bash
npm.cmd install
npm.cmd run build
```

Hiện tại build fail là expected. Ghi lại lỗi trước khi sửa.

Definition of Done:

- Dependencies cài được.
- Có log lỗi build hiện tại.

## 4. Phase 1 - Làm frontend build xanh

Mục tiêu: `npm.cmd run build` pass.

Không làm feature mới trong phase này.

### Task 1.1 - Sửa type DrawingType

File:

- `frontend/src/components/chart/CandleChart.tsx`
- `frontend/src/components/chart/DrawingToolbar.tsx`

Vấn đề:

- `DrawingToolbar` dùng `cursor | trendline | horizontal | fibonacci`.
- `CandleChart` khai báo `line | trendline | ray`.
- `CandleChart` code lại xử lý `horizontal/fibonacci`.

Việc cần làm:

1. Tạo một type thống nhất:

```ts
export type DrawingType = 'cursor' | 'trendline' | 'horizontal' | 'fibonacci';
```

2. Dùng type này nhất quán ở cả hai component.
3. `DrawingLine.type` không nên là `cursor`, vì cursor không tạo drawing. Có thể dùng:

```ts
export type PersistedDrawingType = 'trendline' | 'horizontal' | 'fibonacci';
```

4. Nếu chưa muốn tách type, tối thiểu đảm bảo `DrawingLine.type` không gây lỗi compile.

Test:

```bash
cd frontend
npm.cmd run build
```

### Task 1.2 - Destructure đầy đủ props trong CandleChart

File:

- `frontend/src/components/chart/CandleChart.tsx`

Vấn đề:

Component hiện:

```ts
export const CandleChart = forwardRef<CandleChartRef, CandleChartProps>(({ data, volumeData, markers }, ref) => {
```

Nhưng bên trong dùng:

- `activeTool`
- `drawings`
- `onDrawingComplete`

Việc cần làm:

Sửa thành:

```ts
export const CandleChart = forwardRef<CandleChartRef, CandleChartProps>(
  ({ data, volumeData, markers, drawings = [], activeTool = 'cursor', onDrawingComplete }, ref) => {
```

Nhớ đóng ngoặc component đúng.

Test:

```bash
cd frontend
npm.cmd run build
```

### Task 1.3 - Fix BarPrice/null typing

File:

- `frontend/src/components/chart/CandleChart.tsx`

Vấn đề:

`coordinateToPrice()` có thể trả `BarPrice | null`, trong khi code muốn `number`.

Việc cần làm:

1. Sau khi gọi `coordinateToPrice`, check null ngay.
2. Convert rõ:

```ts
const rawPrice = mainSeries.coordinateToPrice(param.point.y);
if (rawPrice == null) return;
let price = Number(rawPrice);
```

3. Hàm `getSnappedPrice()` nên nhận và trả `number`.
4. Khi lấy OHLC từ `param.seriesData`, convert `Number(p)`.

Test:

```bash
cd frontend
npm.cmd run build
```

### Task 1.4 - Fix lỗi children của MultiChartLayout

File:

- `frontend/src/components/layout/MultiChartLayout.tsx`
- `frontend/src/pages/ReplayPage.tsx`

Vấn đề:

Build báo:

```text
children prop expects type ReactNode[] but only a single child was provided
```

Việc cần làm:

1. Mở `MultiChartLayout.tsx`.
2. Nếu prop đang khai báo `children: React.ReactNode[]`, đổi thành:

```ts
children: React.ReactNode;
```

3. Nếu component thật sự cần array, dùng `React.Children.toArray(children)` bên trong.

Test:

```bash
cd frontend
npm.cmd run build
```

### Task 1.5 - Dọn unused imports gây lỗi strict

File:

- `frontend/src/pages/ReplayPage.tsx`

Vấn đề:

`LineData` import nhưng không dùng.

Việc cần làm:

- Xóa import không dùng.
- Chạy build lại.

Definition of Done Phase 1:

```bash
cd frontend
npm.cmd run build
```

Pass 100%.

## 5. Phase 2 - Backend test baseline

Mục tiêu: backend tests chạy được và phản ánh đúng rule hiện tại.

### Task 2.1 - Chạy toàn bộ backend tests

Trong `backend`:

```bash
.\.venv\Scripts\activate
python -m pytest -q app/tests
```

Nếu fail, không sửa ngay tất cả. Phân loại fail:

- Fail vì environment/import.
- Fail vì test cũ trái rule mới.
- Fail vì bug thật.

Ghi lại danh sách test fail.

### Task 2.2 - Sửa test T+2 cũ

Files:

- `backend/app/tests/test_trade_lifecycle.py`
- `backend/app/tests/test_api_integration.py`

Vấn đề:

Test hiện mua ở T rồi close ở T+1, nhưng rule T+2 yêu cầu reject.

Việc cần làm:

Tạo 2 test riêng:

1. `test_sell_t_plus_1_is_rejected`
   - Seed ít nhất 3 candles.
   - Create session.
   - BUY ở index 0.
   - Next 1 lần, current_index = 1.
   - SELL/CLOSE.
   - Expect HTTP 400 hoặc service raise `HTTPException`.
   - Message chứa `T+2`.

2. `test_sell_t_plus_2_is_allowed`
   - BUY ở index 0.
   - Next 2 lần, current_index = 2.
   - SELL/CLOSE.
   - Expect success.
   - Position closed.
   - Trade closed.

Chú ý:

- Nếu test service-level, dùng `pytest.raises(HTTPException)`.
- Nếu test API-level, check `response.status_code == 400`.

### Task 2.3 - Sửa trade.result khi đóng lệnh thường

File:

- `backend/app/services/trade_lifecycle_service.py`

Vấn đề:

Trong `_execute_sell`, khi close position, code chỉ set `trade.result = 'breakeven'` trong nhánh `else` của initial risk. Không set win/loss chuẩn.

Việc cần làm:

Sau khi tính `trade.net_pnl`, set:

```python
if trade.net_pnl > 0:
    trade.result = "win"
elif trade.net_pnl < 0:
    trade.result = "loss"
else:
    trade.result = "breakeven"
```

Đặt logic này ngoài nhánh `initial_risk`.

Test cần có:

- Close lời => result `win`.
- Close lỗ => result `loss`.
- Close hòa => result `breakeven`.

### Task 2.4 - Lưu fee/tax vào Execution

File:

- `backend/app/services/trade_lifecycle_service.py`

Vấn đề:

Model `Execution` có field:

- `fee`
- `tax`
- `slippage`

Nhưng khi tạo Execution, code không truyền `fee` và `tax`.

Việc cần làm:

Ở BUY:

```python
execution = Execution(
    ...,
    fee=fee,
    tax=0.0,
    gross_amount=gross_amount,
    net_amount=net_amount,
)
```

Ở SELL:

```python
execution = Execution(
    ...,
    fee=fee,
    tax=tax,
    gross_amount=gross_amount,
    net_amount=net_amount,
)
```

Test:

- BUY execution fee = gross * 0.0015.
- SELL execution fee = gross * 0.0015.
- SELL execution tax = gross * 0.001.

### Task 2.5 - Tính net_pnl đúng sau phí/thuế

Vấn đề:

Hiện `trade.net_pnl = position.realized_pnl`, không phản ánh phí/thuế.

Cách làm đơn giản cho MVP:

1. Khi close trade, lấy tất cả executions của trade/session/symbol.
2. Tính:

```text
buy_cash_out = sum(net_amount của BUY executions)
sell_cash_in = sum(net_amount của SELL executions)
net_pnl = sell_cash_in - buy_cash_out
```

Nhưng vì model Execution chưa liên kết trực tiếp trade_id, có thể dùng session/symbol và order side trong giai đoạn đầu.

Yêu cầu:

- Không được làm sai partial sell.
- Nếu chưa xử lý partial sell chuẩn, ghi rõ limitation và test full close trước.

Test:

- BUY 100 cổ giá 100, fee 0.15%.
- SELL 100 cổ giá 110, fee 0.15%, tax 0.1%.
- Expected net PnL:

```text
buy_out = 100 * 100 * 1.0015
sell_in = 100 * 110 * (1 - 0.0015 - 0.001)
net = sell_in - buy_out
```

Definition of Done Phase 2:

```bash
cd backend
python -m pytest -q app/tests
```

Pass hoặc chỉ còn fail đã ghi rõ là ngoài phạm vi.

## 6. Phase 3 - No-future-leak cho indicators

Mục tiêu: frontend không bao giờ nhận indicator data của tương lai trong replay mode.

### Task 3.1 - Thiết kế API indicator theo session

Tạo endpoint mới:

```http
GET /api/replay/sessions/{session_id}/indicators
```

Query:

```text
indicator=sma|ema|rsi|...
timeframe=1D
length=20
...
```

Luồng xử lý:

1. Load session.
2. Dùng `ReplayService.get_candles(db, session_id, target_timeframe)`.
3. Convert visible candles thành DataFrame.
4. Compute indicator.
5. Trả indicator values chỉ đến current candle.

Không được query trực tiếp theo symbol/limit trong replay mode.

### Task 3.2 - Giữ API cũ cho non-replay nếu cần

API cũ `/api/indicators/{symbol}` có thể giữ, nhưng frontend replay không được dùng nó.

Thêm comment rõ:

```python
# This endpoint is for non-replay exploratory use only.
# Replay mode must use session-scoped indicator endpoint.
```

### Task 3.3 - Sửa frontend gọi session indicator API

Files:

- `frontend/src/api/indicatorsApi.ts`
- `frontend/src/pages/ReplayPage.tsx`

Việc cần làm:

- Thêm function `getSessionIndicatorData(sessionId, indicator, params)`.
- `ReplayPage` dùng function mới.
- Xóa logic cache full future data theo symbol.
- Indicator data trả về từ backend đã an toàn, frontend không cần filter để chống leak. Frontend có thể filter thêm để defensive, nhưng không được phụ thuộc vào đó.

### Task 3.4 - Test no-future-leak indicator

Backend test:

1. Seed 30 candles tăng dần.
2. Create session current_index = 10.
3. Gọi EMA20 session endpoint.
4. Assert response không có timestamp > candle index 10.
5. Advance to index 11.
6. Assert response dài hơn đúng 1 candle hoặc timestamp cuối là candle index 11.

Definition of Done Phase 3:

- Test no-future-leak candles pass.
- Test no-future-leak indicators pass.
- Replay frontend không gọi `/api/indicators/{symbol}` nữa.

## 7. Phase 4 - Manual Limit Order và trần/sàn

Mục tiêu: TC-004 hoạt động thật trong manual trading flow.

### Task 4.1 - Mở rộng schema DecisionCreate

File:

- `backend/app/schemas/decision_schema.py`

Hiện có `order_type`, `price`.

Việc cần làm:

- Validate `order_type` bằng enum thay vì string tự do nếu có thể.
- Với LIMIT, `price` bắt buộc.

Rule:

```text
MARKET_AT_CLOSE: price optional, dùng close hiện tại.
MARKET_NEXT_OPEN: không execute ngay, pending đến next candle.
LIMIT: price required.
```

Nếu chưa làm MARKET_NEXT_OPEN trong phase này, reject với message rõ.

### Task 4.2 - Thêm trạng thái pending order

Hiện manual flow execute ngay.

Cần hỗ trợ:

- `created`
- `pending`
- `executed`
- `rejected`
- `cancelled`

Limit order khi đặt hợp lệ sẽ là `pending`.

### Task 4.3 - Validate price limits khi tạo LIMIT

Files:

- `backend/app/services/trade_lifecycle_service.py`
- `backend/app/domain/engine/market_constraints.py`
- `backend/app/models/symbol.py`

Luồng:

1. Lấy symbol của session.
2. Lấy exchange từ bảng `symbols`.
3. Lấy previous candle close làm reference price.
4. Gọi `MarketConstraints.is_price_within_limits(price, exchange, reference_price)`.
5. Nếu fail:
   - Tạo order status `rejected`, hoặc raise HTTP 400 tùy quyết định.
   - Nên lưu rejected order để audit.

Chú ý:

- Nếu không có previous candle, có thể dùng current close hoặc reject với message rõ.
- Nếu không có exchange, default HOSE chỉ là tạm thời. Nên warning.

### Task 4.4 - Khớp pending LIMIT khi Next Candle

File:

- `backend/app/services/replay_service.py`
- Có thể tạo service mới `order_matching_service.py`.

Khi `next_candle()` advance:

1. Lấy candle mới.
2. Lấy pending orders của session/symbol.
3. Với BUY LIMIT:

```text
if candle.low <= limit_price <= candle.high:
    execute buy at min(candle.open, limit_price) nếu gap favorable
```

4. Với SELL LIMIT:

```text
if candle.low <= limit_price <= candle.high:
    execute sell at max(candle.open, limit_price) nếu gap favorable
```

5. Sau khi execute, order status = executed.

### Task 4.5 - Frontend Order Panel cho Limit

File:

- `frontend/src/components/replay/TradeControls.tsx`

Thêm:

- Select order type: Market at close / Limit.
- Nếu Limit, show input limit price.
- Submit `order_type` và `price`.

UI tối thiểu:

```text
Order Type [Market at close | Limit]
Limit Price [input number] nếu Limit
Quantity
Stop Loss
Target
Reason
```

### Task 4.6 - Tests TC-004

Test cases:

1. HOSE previous close 100.
2. BUY LIMIT 111.
3. Expected reject vì vượt +7%.

Thêm test hợp lệ:

1. HOSE previous close 100.
2. BUY LIMIT 105.
3. Order pending.
4. Next candle high/low chứa 105.
5. Order executed.

Definition of Done Phase 4:

- TC-004 pass.
- Manual UI đặt được Limit.
- Pending order khớp khi next candle.

## 8. Phase 5 - Analytics chuẩn hóa

Mục tiêu: analytics phản ánh đúng tài chính, không còn demo hard-code.

### Task 5.1 - Lấy session initial cash

File:

- `backend/app/services/analytics_service.py`

Việc cần làm:

- Query `ReplaySession`.
- Dùng `session.initial_cash`.
- Không hard-code `100000.0`.

Test:

- Session initial_cash = 100_000_000.
- Equity curve điểm đầu hoặc baseline phải dựa trên 100_000_000.

### Task 5.2 - Tạo equity curve chuẩn

Cách đơn giản cho manual replay:

1. Lấy all candles trong session date range.
2. Lấy executions theo ngày.
3. Replay lại ledger:
   - cash starts at initial_cash.
   - BUY trừ net_amount.
   - SELL cộng net_amount.
   - holdings cập nhật quantity/average.
4. Mỗi candle date:
   - mark-to-market holdings bằng close.
   - equity = cash + holdings market value.

Response point:

```json
{
  "timestamp": "2024-01-02",
  "equity": 101000000,
  "drawdown": 0,
  "drawdown_pct": 0
}
```

Nếu chưa muốn đổi schema lớn, thêm field optional `drawdown_pct`.

### Task 5.3 - Max drawdown đúng

Công thức:

```text
peak = max(equity[0..t])
drawdown_pct = (peak - equity[t]) / peak
max_drawdown_pct = max(drawdown_pct)
```

Lưu:

- max_drawdown_amount
- max_drawdown_pct

Nếu schema hiện chỉ có `max_drawdown`, quyết định:

- Giữ `max_drawdown` là amount.
- Thêm `max_drawdown_pct`.

### Task 5.4 - Drawdown duration

Tính:

- Start khi equity giảm dưới peak.
- Duration tăng mỗi candle cho đến khi equity vượt lại peak.
- Max duration.
- Series drawdown để chart histogram/bar.

Response thêm:

```json
"drawdown_periods": [
  {
    "start": "2024-01-05",
    "end": "2024-02-01",
    "duration_bars": 18,
    "max_drawdown_pct": 0.12
  }
]
```

### Task 5.5 - Sharpe Ratio đúng hơn

Từ equity curve:

```text
daily_return[t] = equity[t] / equity[t-1] - 1
```

Risk-free annual mặc định:

```text
0.045
```

Daily risk-free:

```text
rf_daily = (1 + rf_annual) ** (1 / 252) - 1
```

Sharpe:

```text
mean(excess_returns) / std(excess_returns) * sqrt(252)
```

Nếu std = 0:

- Sharpe = 0 hoặc null.
- Không được division by zero.

### Task 5.6 - Profit Factor và expectancy

Profit Factor:

```text
gross_profit = sum(net_pnl > 0)
gross_loss = abs(sum(net_pnl < 0))
profit_factor = gross_profit / gross_loss
```

Nếu không có loss:

- Trả `null` hoặc string không được vì schema float.
- Nên trả `None` và frontend hiển thị `N/A`.

Expectancy:

```text
win_rate * average_win + loss_rate * average_loss
```

Trong đó average_loss nên là số âm hoặc dùng công thức rõ. Chọn một convention và test.

Khuyến nghị:

- `average_loss` trả số âm.
- `expectancy = win_rate * avg_win + loss_rate * avg_loss`.

Nếu giữ `average_loss` dương như hiện tại, phải tính:

```text
expectancy = win_rate * average_win - loss_rate * average_loss
```

### Task 5.7 - Benchmark VNINDEX

Backend:

1. Load candles của `VNINDEX` cùng date range.
2. Normalize:

```text
benchmark_value = benchmark_close / first_benchmark_close * initial_cash
```

3. Return series:

```json
"benchmark_curve": [
  {"timestamp": "...", "value": 100000000}
]
```

Frontend:

- `EquityChart` vẽ thêm line benchmark màu khác.

### Task 5.8 - Trade distribution histogram

Backend có thể trả bins:

```json
"trade_distribution": [
  {"bin_start": -10, "bin_end": -5, "count": 2},
  {"bin_start": -5, "bin_end": 0, "count": 3}
]
```

Hoặc frontend tự bin từ trades.

Khuyến nghị cho dev junior:

- Backend trả raw closed trades.
- Frontend bin đơn giản theo `net_pnl` hoặc `r_multiple`.
- Sau đó mới optimize.

Definition of Done Phase 5:

- Analytics không hard-code capital.
- Max drawdown amount và pct đúng.
- Sharpe không division by zero.
- Equity curve có benchmark VNINDEX nếu data tồn tại.
- Có chart trade distribution.
- Unit tests cho các metric chính.

## 9. Phase 6 - Data schema và migration

Mục tiêu: database được quản lý chuyên nghiệp.

### Task 6.1 - Quyết định scope schema

Phải chọn một trong hai:

Option A - Theo `PRODUCT_SPEC.md`:

- Local-first SQLite.
- `symbols`, `candles`, replay/journal/trade.
- Không bắt buộc TimescaleDB.

Option B - Theo `SPEC.md`:

- PostgreSQL/TimescaleDB.
- `dim_tickers`, `fact_stock_ohlcv_daily`.
- `basic_close`, adjusted OHLC bắt buộc.

Khuyến nghị hiện tại: chọn Option A để hoàn thiện MVP trước.

Ghi quyết định vào:

- `docs/DECISIONS.md`

### Task 6.2 - Tạo Alembic migration thật

Hiện `alembic/versions` trống.

Việc cần làm:

```bash
cd backend
alembic revision --autogenerate -m "initial schema"
```

Kiểm tra file migration:

- Có create table đầy đủ.
- Có unique constraint cho candles.
- Có indexes cần thiết.

Sau đó test:

```bash
alembic upgrade head
```

### Task 6.3 - Bỏ create_all khỏi production startup

File:

- `backend/app/main.py`

Hiện:

```python
Base.metadata.create_all(bind=engine)
```

Việc cần làm:

- Không tự create table trong app startup production.
- Có thể giữ trong script dev riêng:
  - `backend/scripts/init_db.py`

Nếu muốn local tiện, dùng env:

```python
if settings.AUTO_CREATE_TABLES:
    Base.metadata.create_all(bind=engine)
```

Default nên là false cho production, true cho local demo chỉ khi docs ghi rõ.

### Task 6.4 - Exchange data cho symbol

Để trần/sàn đúng, cần `Symbol.exchange`.

Nếu CafeF filename có sàn, parse từ filename:

- `CafeF.HSX...` => HOSE
- `CafeF.HNX...` => HNX
- `CafeF.UPCOM...` => UPCOM

Nếu không biết, default null và UI báo "missing exchange".

Test:

- Import file HSX => symbols exchange HOSE.
- Import file HNX => HNX.
- Import file UPCOM => UPCOM.

Definition of Done Phase 6:

- Có migration file.
- Fresh DB chạy được bằng Alembic.
- App không phụ thuộc vào create_all lẫn lộn.
- Symbol có exchange khi import CafeF theo file.

## 10. Phase 7 - Algorithmic Backtest Lab

Chỉ bắt đầu phase này sau khi Phase 1-6 đạt DoD.

Mục tiêu: có runner backend trước, UI code editor sau.

### Task 7.1 - Thiết kế domain model

Models đề xuất:

```text
strategy_artifacts
backtest_jobs
backtest_results
backtest_equity_points
backtest_orders
backtest_executions
```

Tối thiểu:

`strategy_artifacts`:

- id
- name
- source_code
- created_at

`backtest_jobs`:

- id
- strategy_artifact_id
- symbol
- timeframe
- start_date
- end_date
- initial_cash
- status: pending/running/succeeded/failed/cancelled
- error_message
- created_at
- started_at
- finished_at

`backtest_results`:

- id
- job_id
- total_return
- final_equity
- max_drawdown_pct
- sharpe_ratio
- win_rate
- profit_factor
- total_trades

### Task 7.2 - Runner MVP không cần Celery trước

Dev junior không nên thêm Celery/Redis ngay.

Làm MVP runner sync trước:

```http
POST /api/backtests/run
```

Request:

```json
{
  "strategy_code": "...",
  "symbol": "FPT",
  "start_date": "2022-01-01",
  "end_date": "2024-12-31",
  "initial_cash": 100000000
}
```

Response:

```json
{
  "job_id": 1,
  "status": "succeeded",
  "metrics": {}
}
```

Khi sync runner ổn, mới chuyển sang background job.

### Task 7.3 - Strategy API an toàn tối thiểu

Không chạy code user trực tiếp trong app process lâu dài nếu có thể.

MVP có thể chạy trong subprocess với timeout:

- Write strategy temp file.
- Run `python strategy_runner.py`.
- Timeout 30-120 giây.
- Capture stdout/stderr.
- Return error rõ.

Cấm:

- Network access trong strategy.
- File write ngoài temp dir.
- Infinite loop không timeout.

Nếu chưa sandbox đầy đủ, ghi rõ "local trusted code only".

### Task 7.4 - BaseStrategy contract

Chuẩn hóa API:

```python
class BaseStrategy:
    def init(self):
        pass

    def next(self, i, row):
        pass

    def buy(self, quantity, order_type="MKT", price=None):
        pass

    def sell(self, quantity, order_type="MKT", price=None):
        pass
```

Yêu cầu:

- Strategy không tự sửa cash trực tiếp.
- Strategy chỉ phát order.
- Broker xử lý cash, fee, T+2, trần/sàn.

### Task 7.5 - Sample strategy

Tạo sample:

```python
class Strategy(BaseStrategy):
    def init(self):
        self.sma20 = self.data["close"].rolling(20).mean()
        self.sma50 = self.data["close"].rolling(50).mean()

    def next(self, i, row):
        if i < 50:
            return
        if self.sma20.iloc[i] > self.sma50.iloc[i] and not self.position:
            self.buy(quantity=100)
        elif self.sma20.iloc[i] < self.sma50.iloc[i] and self.position:
            self.sell(quantity=self.position.quantity)
```

Test:

- Runner load được sample.
- Có trades.
- Có metrics.

### Task 7.6 - UI strategy lab sau cùng

Frontend page:

- Strategy list
- Code editor textarea trước, Monaco sau
- Symbol/date/cash inputs
- Run button
- Job result panel
- Equity chart
- Trade table

Không cần Monaco ngay nếu làm chậm. Textarea đủ cho MVP.

Definition of Done Phase 7:

- Chạy được sample strategy.
- Có result persisted.
- Có metrics dùng chung với AnalyticsService.
- Có timeout/error handling.
- UI hiển thị kết quả.

## 11. Final Acceptance Checklist

Project chỉ được coi là hoàn thiện bản chuyên nghiệp đầu tiên khi:

- Frontend build pass.
- Backend tests pass.
- Import CafeF CSV/TXT/ZIP pass.
- Replay candles không leak future.
- Replay indicators không leak future.
- BUY mở position/trade đúng.
- T+1 SELL bị reject.
- T+2 SELL pass.
- Fee/tax/net PnL đúng.
- LIMIT order pending/executed/rejected đúng.
- Trần/sàn theo HOSE/HNX/UPCOM đúng.
- Force liquidation có test.
- Analytics equity/drawdown/sharpe/profit factor đúng.
- VNINDEX benchmark hiển thị nếu có data.
- Trade distribution hiển thị.
- Alembic migration tồn tại và chạy được fresh DB.
- README cập nhật hướng dẫn chạy/test thật.
- Không có tuyên bố hoàn thành nếu không có test output.

## 12. Thứ tự tuyệt đối không được đảo

Không làm:

- Strategy editor trước khi frontend build xanh.
- Celery/Redis trước khi sync backtest runner chạy đúng.
- UI đẹp trước khi trade lifecycle đúng.
- Indicator nhiều hơn trước khi no-future-leak indicator được khóa.
- Analytics chart nhiều hơn trước khi công thức đúng.

Thứ tự đúng:

```text
Build xanh
-> Test backend xanh
-> No future leak tuyệt đối
-> Trade lifecycle đúng
-> Market constraints
-> Analytics đúng
-> Migration/schema sạch
-> Algorithmic engine
-> UI polish
```

