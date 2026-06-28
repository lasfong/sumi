# QA Acceptance Review - Sumi

Ngày kiểm tra: 2026-06-28

Vai trò kiểm tra: QA / QC / Tester / Release Gatekeeper

## 1. Kết luận nghiệm thu

**Kết luận: Chưa đủ điều kiện nhận bàn giao production-quality.**

Dự án đã tiến bộ rõ so với audit trước:

- Frontend production build đã pass.
- Backend pytest suite đã pass 21 tests.
- Frontend vitest suite đã pass 2 tests.
- Đã có session-scoped indicator API.
- Đã có thêm limit order, market constraints, migration file và backtest MVP.

Tuy nhiên vẫn còn nhiều blocker trước khi nhận bàn giao:

- Frontend lint fail 48 errors.
- Backtest API có rủi ro security nghiêm trọng do dùng `eval()` trên input API.
- Trade PnL sai khi có nhiều round-trip trades cùng session/symbol.
- Backend analytics contract lệch với frontend chart contract.
- Alembic migration không vận hành được từ requirements hiện tại vì thiếu Alembic CLI/dependency.
- Test coverage còn mỏng so với phạm vi feature đã thêm.

Trạng thái khuyến nghị: **Reject handover, return to development with blocking fixes.**

## 2. Lệnh kiểm tra đã chạy

### 2.1. Runtime

```text
node -v
Result: v24.14.0

npm.cmd -v
Result: 11.9.0

python --version
Result: command not found

py --version
Result: No installed Python found
```

Backend chỉ chạy được bằng virtualenv nội bộ:

```text
backend/.venv/Scripts/python.exe
```

Trong sandbox thường bị `Access is denied`; phải chạy escalated mới kiểm thử được.

### 2.2. Frontend build

Command:

```bash
cd frontend
npm.cmd run build
```

Result:

```text
PASS
vite build completed
Warning: chunk larger than 500 kB
```

### 2.3. Frontend lint

Command:

```bash
cd frontend
npm.cmd run lint
```

Result:

```text
FAIL
48 errors, 1 warning
```

Nhóm lỗi chính:

- `no-explicit-any`
- `react-hooks/set-state-in-effect`
- unused variables
- empty catch blocks
- missing hook dependency

### 2.4. Frontend tests

Command:

```bash
cd frontend
npm.cmd test
```

Result:

```text
PASS
1 test file passed
2 tests passed
```

Nhận xét: test frontend hiện rất mỏng, mới cover BacktestPage happy path.

### 2.5. Backend tests

Command:

```bash
cd backend
.\.venv\Scripts\python.exe -m pytest -q app\tests
```

Result:

```text
PASS
21 passed, 3 warnings
```

Warnings:

- FastAPI `@app.on_event("startup")` deprecated.
- Starlette/TestClient warning.

### 2.6. Alembic

Commands:

```bash
.\.venv\Scripts\python.exe -m alembic history
.\.venv\Scripts\alembic.exe history
```

Result:

```text
FAIL
No module named alembic.__main__
alembic.exe not found
```

Nguyên nhân trực tiếp:

- `backend/requirements.txt` không có `alembic`.
- Venv không có `alembic.exe`.

## 3. Blocking Findings

### P0-1. Backtest API có thể thực thi code tùy ý qua `eval()`

File:

- `backend/app/services/backtest_service.py`

Bằng chứng:

- `_evaluate_rules()` nhận condition từ strategy config.
- Condition đến từ API `/api/backtest/run`.
- Code gọi `eval(expr)` trực tiếp.
- API backend bật CORS `allow_origins=["*"]`.

Impact:

- Người dùng hoặc trang web độc hại có thể gửi strategy condition chứa Python expression nguy hiểm.
- Vì app local-first thường chạy trên máy cá nhân, đây là rủi ro local code execution.

Yêu cầu sửa:

- Xóa `eval()`.
- Dùng parser an toàn cho rule expression.
- Chỉ cho phép operator whitelist:
  - `>`
  - `<`
  - `>=`
  - `<=`
  - `==`
  - `and`
  - `or`
  - parentheses nếu parser hỗ trợ.
- Chỉ cho phép identifiers thuộc indicator snapshot.
- Thêm test security: condition chứa `__import__`, `open`, `os`, `subprocess` phải bị reject.

Điều kiện pass:

- Backtest runner không còn dùng `eval`.
- Test malicious expression pass.

### P0-2. Trade net PnL sai khi có nhiều trades trong cùng session/symbol

File:

- `backend/app/services/trade_lifecycle_service.py`

Bằng chứng:

Khi close trade, code lấy:

```python
buy_executions = all BUY executions by session_id + symbol
sell_executions = all SELL executions by session_id + symbol
trade.net_pnl = sell_cash_in - buy_cash_out
```

Impact:

- Trade thứ 2 sẽ bị tính lẫn cashflow của trade thứ 1.
- Analytics tổng PnL, win rate, profit factor, expectancy, R-multiple đều có thể sai.

Yêu cầu sửa:

- Execution hoặc Order phải liên kết được với `trade_id`, hoặc service phải xác định execution thuộc vòng đời trade hiện tại.
- Với MVP, ít nhất phải có test:
  - Buy/Sell round trip 1 lời.
  - Buy/Sell round trip 2 lỗ.
  - Mỗi trade có net_pnl riêng đúng.
  - Total analytics = sum từng trade, không double count.

Điều kiện pass:

- Test multi-round-trip cùng session/symbol pass.

### P0-3. Backend analytics response lệch contract frontend chart

Files:

- Backend: `backend/app/services/analytics_service.py`
- Backend schema: `backend/app/schemas/analytics_schema.py`
- Frontend chart: `frontend/src/components/analytics/EquityChart.tsx`
- Frontend type: `frontend/src/types/analytics.ts`

Bằng chứng:

Backend `equity_curve` item trả:

```json
{
  "time": "...",
  "equity": 100000000,
  "cash": 90000000,
  "holdings_value": 10000000
}
```

Frontend `EquityChart` lại đọc:

```ts
d.timestamp
d.drawdown
```

Impact:

- Chart equity/drawdown có thể sort date sai hoặc không render đúng runtime.
- TypeScript build không bắt được vì backend schema dùng `List[dict]` và frontend type chưa cập nhật fields mới.

Yêu cầu sửa:

- Chuẩn hóa `EquityPoint` ở backend có field rõ:
  - `timestamp`
  - `equity`
  - `cash`
  - `holdings_value`
  - `drawdown`
  - `drawdown_pct`
- Frontend type phải match backend.
- `EquityChart` dùng cùng contract.
- Thêm frontend test hoặc API contract fixture test.

Điều kiện pass:

- Analytics page và Backtest page hiển thị equity chart với real backend-shaped payload.

### P1-1. Frontend lint fail, chưa đạt chất lượng bàn giao

Command:

```bash
npm.cmd run lint
```

Result:

```text
48 errors, 1 warning
```

Impact:

- Code chưa đạt TypeScript/React quality gate.
- Nhiều `any` che mất lỗi contract thật.
- Empty catch blocks làm mất lỗi runtime quan trọng.

Yêu cầu sửa:

- Zero lint error trước khi bàn giao.
- Không tắt rule hàng loạt để qua cổng.

### P1-2. Alembic migration chưa vận hành được

Files:

- `backend/requirements.txt`
- `backend/alembic.ini`
- `backend/alembic/env.py`
- `backend/alembic/versions/cdf80254e9dc_initial_schema.py`

Impact:

- Có migration file nhưng môi trường không chạy được migration CLI.
- Không thể nghiệm thu fresh database setup bằng Alembic.

Yêu cầu sửa:

- Thêm `alembic` vào requirements.
- Reinstall venv.
- Chạy:

```bash
alembic history
alembic upgrade head
alembic current
```

- Test trên DB rỗng.

### P1-3. Config dev/prod chưa an toàn

File:

- `backend/app/config.py`

Bằng chứng:

```python
DEBUG = True
AUTO_CREATE_TABLES = True
```

Impact:

- App vẫn auto-create tables mặc định.
- Không phù hợp release handover.

Yêu cầu sửa:

- Default `DEBUG=False`, `AUTO_CREATE_TABLES=False`.
- Local dev bật qua `.env.example`.
- README ghi rõ cách init DB.

### P1-4. CORS mở toàn bộ

File:

- `backend/app/main.py`

Bằng chứng:

```python
allow_origins=["*"]
```

Impact:

- Với backtest `eval()` hiện tại, rủi ro tăng mạnh.
- Ngay cả local app cũng nên giới hạn origin frontend.

Yêu cầu sửa:

- Configurable allowed origins.
- Default chỉ cho `http://localhost:5173` và `http://127.0.0.1:5173`.

### P1-5. Backtest service swallow exceptions

File:

- `backend/app/services/backtest_service.py`

Bằng chứng:

```python
except Exception:
    traceback.print_exc()
```

Impact:

- Backtest có thể fail trade execution nhưng API vẫn trả analytics, làm người dùng tin kết quả sai.

Yêu cầu sửa:

- Job result phải có status `failed` nếu execution lỗi không recoverable.
- Lưu errors theo bar index.
- Test case strategy invalid phải trả error rõ.

## 4. Non-blocking Findings

### P2-1. Test coverage frontend quá mỏng

Hiện chỉ có:

- `BacktestPage.test.tsx`
- 2 tests

Thiếu:

- ReplayPage interaction.
- TradeControls limit order validation.
- Analytics chart contract.
- ImportPage upload handling.
- Error states.

### P2-2. Backend tests chưa đủ acceptance matrix

Backend có 21 tests pass, nhưng cần thêm:

- Multi-trade same session/symbol.
- Partial sell/add/reduce.
- Pending limit order cancel/expiry.
- Limit sell order.
- Market next open nếu enum còn tồn tại.
- Indicator no-future-leak endpoint test qua API.
- Backtest malicious condition reject.
- Backtest no data / no trades / invalid strategy.
- Analytics benchmark curve integration.

### P2-3. FastAPI startup dùng deprecated `on_event`

File:

- `backend/app/main.py`

Yêu cầu:

- Chuyển sang lifespan handler trước release.

### P2-4. Bundle frontend hơi lớn

Build warning:

```text
Some chunks are larger than 500 kB after minification
```

Không block MVP, nhưng nên code split Backtest/Analytics pages sau.

## 5. Quality Gate Đề Xuất

### Gate A - Developer Done

Mỗi PR/task chỉ được merge khi:

```bash
cd backend
.\.venv\Scripts\python.exe -m pytest -q app\tests

cd ../frontend
npm.cmd run build
npm.cmd run lint
npm.cmd test
```

Tất cả phải pass.

### Gate B - QA Acceptance

QA chỉ nhận test khi:

- Dev cung cấp command output.
- Có migration chạy được trên DB rỗng.
- Có seed/sample data tối thiểu.
- Có test evidence cho TC-001 đến TC-005.
- Có danh sách known limitations.

### Gate C - Business UAT

Business/user nghiệm thu theo luồng:

1. Import CafeF sample.
2. List symbols.
3. Create replay session.
4. Verify chart chỉ hiện candle visible.
5. Add EMA/RSI, verify indicator không leak future.
6. BUY market.
7. SELL ở T+1 bị reject.
8. SELL ở T+2 pass.
9. LIMIT ngoài biên độ bị reject.
10. LIMIT trong biên độ pending rồi khớp khi next candle.
11. Analytics hiển thị equity, drawdown, trade list.
12. Backtest sample strategy chạy được.
13. Backtest invalid/malicious strategy bị reject an toàn.

### Gate D - Handover

Chỉ nhận bàn giao code khi có đủ:

- README chạy local chính xác.
- `.env.example`.
- Fresh install guide.
- Fresh DB migration guide.
- Test command output.
- QA acceptance checklist signed off.
- Known limitations rõ ràng.
- Không còn P0/P1 open.

## 6. Kế Hoạch Nghiệm Thu Chi Tiết

### Bước 1 - Static Quality

Commands:

```bash
git status --short
npm.cmd run lint
npm.cmd run build
npm.cmd test
python -m pytest -q app/tests
```

Exit criteria:

- No lint errors.
- All tests pass.
- No generated/cache files staged.

### Bước 2 - Database/Migration

Commands:

```bash
alembic history
alembic upgrade head
alembic current
```

Manual check:

- Fresh DB has all tables.
- No startup `create_all` required in non-dev mode.

### Bước 3 - API Contract Smoke

Use FastAPI TestClient or HTTP smoke:

- `GET /api/health`
- `POST /api/import/cafef`
- `GET /api/symbols`
- `POST /api/replay/sessions`
- `GET /api/replay/sessions/{id}/candles`
- `GET /api/replay/sessions/{id}/indicators`
- `POST /api/replay/sessions/{id}/decisions`
- `GET /api/replay/sessions/{id}/analytics`
- `POST /api/backtest/run`

Exit criteria:

- All endpoints return expected status and response schema.

### Bước 4 - Replay Acceptance

Dataset:

- One symbol with 10 candles.
- One VNINDEX benchmark series.

Scenarios:

- No future candle at index 0, 1, 5.
- Indicator no future at index 5.
- Previous does not corrupt existing decisions.
- Auto-play advances and updates chart.

### Bước 5 - Trading Acceptance

Scenarios:

- BUY market opens position.
- T+1 SELL rejected.
- T+2 SELL closes.
- Net PnL includes buy fee, sell fee, sell tax.
- Two separate trades in same session calculate independently.
- ADD then partial REDUCE works.
- CLOSE ALL closes current open quantity only.

### Bước 6 - Limit Order Acceptance

Scenarios:

- HOSE +8% limit buy rejected.
- HNX +11% limit buy rejected.
- UPCOM +16% limit buy rejected.
- Valid limit buy pending.
- Pending limit buy fills when candle range touches price.
- Pending limit buy does not fill when price is not touched.
- Sell limit follows T+2 availability.

### Bước 7 - Analytics Acceptance

Scenarios:

- Equity curve starts at `initial_cash`.
- Equity curve updates after buy/sell executions.
- Drawdown amount and pct correct.
- Drawdown duration correct.
- Sharpe uses equity returns.
- Profit factor correct.
- Trade distribution renders.
- VNINDEX benchmark overlays if data exists.

### Bước 8 - Backtest Acceptance

Scenarios:

- Sample MA crossover produces deterministic result.
- No data returns clear error.
- Invalid strategy returns clear error.
- Malicious condition is rejected.
- No silent exception swallowing.
- Backtest result analytics uses same calculations as manual replay.

## 7. Handover Decision

Current decision:

```text
REJECT HANDOVER
```

Minimum fixes before re-review:

1. Remove unsafe `eval()` from backtest service.
2. Fix per-trade net PnL for multiple trades.
3. Align analytics backend/frontend contract.
4. Make `npm.cmd run lint` pass.
5. Add Alembic dependency and verify migration commands.
6. Add missing tests for the above blockers.

