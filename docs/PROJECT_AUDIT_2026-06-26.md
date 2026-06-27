# Project Audit - Sumi / VN Replay Trading Lab

Ngày đánh giá: 2026-06-26

## 1. Mục tiêu tài liệu

Tài liệu này ghi lại đánh giá nhanh nhưng có căn cứ về tình trạng hiện tại của dự án Sumi sau khi đối chiếu:

- `docs/SPEC.md`
- `docs/PRODUCT_SPEC.md`
- `docs/DECISIONS.md`
- README
- Code backend/frontend/tests hiện có trong repository

Mục tiêu là trả lời câu hỏi: báo cáo của dev trước đó có đúng không, phần nào đã làm thật, phần nào mới là skeleton, phần nào còn rủi ro.

## 2. Kết luận tổng quan

Báo cáo của dev đúng một phần, nhưng hơi tô hồng.

Dự án đã có nền tảng backend/frontend tương đối nhiều, không phải prototype một file. Tuy nhiên chưa thể gọi là core hoàn thiện vì:

- Frontend hiện build fail bằng `npm.cmd run build`.
- Backend tests chưa chạy được trong môi trường kiểm tra vì máy hiện không có Python runtime trên PATH.
- Một số test hiện tại có vẻ trái với rule T+2 mới được thêm.
- Một số tính năng chỉ tồn tại ở domain skeleton, chưa được nối vào API/UI thật.
- Analytics đã có màn hình và metric, nhưng công thức còn sơ khai và chưa đúng chuẩn tài chính trong SPEC.
- No-future-leak mới được đảm bảo cho candles chính, chưa đảm bảo cho indicator API.

Ước lượng trạng thái hiện tại: khoảng 60-70% của MVP foundation trong `PRODUCT_SPEC.md`, nhưng còn xa so với toàn bộ `SPEC.md`.

## 3. Điểm đã làm tốt

### 3.1. Cấu trúc dự án

Backend đã chia tương đối đúng:

- `app/models`
- `app/schemas`
- `app/api`
- `app/services`
- `app/domain`
- `app/tests`

Frontend cũng có phân lớp:

- `src/api`
- `src/components`
- `src/pages`
- `src/store`
- `src/types`

Điều này phù hợp với nguyên tắc trong `PRODUCT_SPEC.md`: business logic không nên nằm trực tiếp trong route handler.

### 3.2. Replay service

`ReplayService.get_candles()` đã lấy candles theo `session.current_index + 1`.

Bằng chứng:

- `backend/app/services/replay_service.py`
- Logic chính: query candles theo session, sort tăng dần, `limit = session.current_index + 1`.

Đây là hướng đúng cho rule quan trọng nhất: backend không trả candles tương lai cho frontend.

### 3.3. Import CafeF

Đã có importer hỗ trợ:

- CSV
- TXT
- ZIP chứa CSV/TXT
- Validate OHLCV cơ bản
- Deduplicate theo symbol/timestamp
- Upsert vào SQLite

Bằng chứng:

- `backend/app/services/cafef_importer.py`
- `backend/app/services/data_quality_service.py`
- `backend/app/api/import_data.py`

Mức độ: đủ tốt cho MVP đầu tiên, chưa đủ cho bản theo toàn bộ SPEC.

### 3.4. Trading lifecycle

Đã có các entity chính:

- Decision
- Order
- Execution
- Position
- Trade

Đã có xử lý:

- BUY mở/tăng position
- SELL/REDUCE/CLOSE đóng hoặc giảm position
- Cash trong session được cập nhật
- Có phí mua 0.15%
- Có phí bán 0.15% và thuế bán 0.1%
- Có rule T+2 dựa trên `candle_index`

Bằng chứng:

- `backend/app/services/trade_lifecycle_service.py`
- `backend/app/models/decision.py`
- `backend/app/models/order.py`
- `backend/app/models/execution.py`
- `backend/app/models/position.py`
- `backend/app/models/trade.py`

### 3.5. Replay frontend

Đã có nhiều phần UI:

- Session setup
- Chart bằng lightweight-charts
- WebSocket auto-play
- Next/Previous
- Indicator selector
- Drawing toolbar
- Order panel
- Position panel
- Decision journal

Bằng chứng:

- `frontend/src/pages/ReplayPage.tsx`
- `frontend/src/components/chart/CandleChart.tsx`
- `frontend/src/components/chart/DrawingToolbar.tsx`
- `frontend/src/components/replay/TradeControls.tsx`

### 3.6. Analytics

Không đúng nếu nói hệ thống "mới chỉ có sổ nhật ký lệnh cơ bản".

Hiện đã có:

- Analytics API
- Analytics page
- Equity/drawdown chart
- Win rate
- Total net PnL
- Average win/loss
- Profit factor
- Average R
- Expectancy
- Max drawdown
- Sharpe
- Sortino
- SQN
- Setup performance

Bằng chứng:

- `backend/app/services/analytics_service.py`
- `backend/app/api/analytics.py`
- `frontend/src/pages/AnalyticsPage.tsx`
- `frontend/src/components/analytics/EquityChart.tsx`

Nhưng cách tính hiện tại chưa chuẩn, xem phần rủi ro.

## 4. Vấn đề nghiêm trọng

### 4.1. Frontend hiện build fail

Lệnh đã chạy:

```bash
cd frontend
npm.cmd run build
```

Kết quả: fail TypeScript.

Các lỗi chính:

- `CandleChart.tsx` không destructure props `activeTool`, `drawings`, `onDrawingComplete`.
- Type `DrawingType` trong `CandleChart.tsx` không khớp với `DrawingToolbar.tsx`.
- Code xử lý `horizontal` và `fibonacci`, nhưng type lại khai báo `line | trendline | ray`.
- Có lỗi type `BarPrice | null` gán vào `number`.
- `ReplayPage.tsx` truyền `activeTool="cursor"` nhưng `CandleChart` không chấp nhận.

Kết luận: phần frontend chưa đạt trạng thái bàn giao chuyên nghiệp.

### 4.2. Backend tests chưa được xác minh

Môi trường hiện tại không có:

- `pytest`
- `python`
- `py`

Do đó chưa thể xác nhận báo cáo "pass 100% test E2E".

Ngoài ra, khi đọc test thấy có dấu hiệu test cũ không còn đúng:

- `backend/app/tests/test_trade_lifecycle.py` mua ở ngày 1, next sang ngày 2 rồi CLOSE ngay.
- `backend/app/tests/test_api_integration.py` cũng kỳ vọng CLOSE ngay sau một lần next vẫn status 200.
- Nhưng code mới đã chặn T+2 bằng `Decision.candle_index <= session.current_index - 2`.

Nếu rule T+2 là bắt buộc, các test trên phải được sửa thành:

- Bán ở T+1: reject.
- Bán ở T+2: pass.

### 4.3. No-future-leak chưa được khóa hoàn toàn

Candles chính trong replay đã ổn.

Nhưng indicator endpoint hiện lấy trực tiếp theo symbol:

- `GET /api/indicators/{symbol}?indicator=...&limit=500`
- Query theo `Candle.symbol`, `timeframe`, rồi `limit`.
- Không gắn với `session_id`.
- Không biết `current_index`.

Frontend sau đó tự filter indicator data theo `currentCandle`.

Điều này vi phạm nguyên tắc trong `PRODUCT_SPEC.md`: frontend không được nhận dữ liệu tương lai rồi tự slice.

Kết luận: cần chuyển indicator trong replay sang endpoint session-scoped.

### 4.4. Limit Order và trần/sàn chưa được nối vào manual trading

Hiện enum có `LIMIT`.

Domain broker có skeleton xử lý:

- Market order
- Limit order
- Stop order
- Price limits bằng `MarketConstraints`

Nhưng manual flow trong `TradeLifecycleService` luôn tạo:

```python
order_type=OrderType.MARKET_AT_CLOSE.value
```

Tức là UI/API gửi `order_type` cũng chưa được thực thi thật.

Kết luận: TC-004 chưa hoàn thiện trong sản phẩm thật.

### 4.5. Analytics có nhưng chưa chuẩn

Các vấn đề chính:

- Equity curve hard-code `current_equity = 100000.0`, không dùng `session.initial_cash`.
- Max drawdown hiện tính trên cumulative PnL theo trade, không phải drawdown % của equity curve theo thời gian.
- Sharpe hiện tính theo PnL từng trade, chưa phải return series chuẩn.
- Risk-free rate VN trong SPEC chưa được dùng.
- Chưa có benchmark VNINDEX overlay thật.
- Chưa có trade distribution histogram.
- `trade.net_pnl` trong close thường chưa trừ phí/thuế đúng cách, chỉ dùng `position.realized_pnl`.

Kết luận: analytics hiện là bản demo tốt, chưa đạt chuẩn trong SPEC.

### 4.6. Algorithmic backtest mới là skeleton

Đã có:

- `BaseStrategy`
- `EventDispatcher`
- `BrokerSimulation`
- `Portfolio`
- `EngineOrder`

Nhưng chưa có:

- API upload strategy
- Code editor
- Backtest job
- Worker
- Result persistence
- Strategy sandbox
- UI xem kết quả backtest
- Optimization thật

Trong `strategy.py`, method `optimize()` còn ghi rõ đây là stub.

Kết luận: báo cáo dev rằng phân hệ này chưa xong là đúng.

### 4.7. Migration chưa sạch

`backend/alembic` có cấu hình nhưng thư mục `versions` không có migration `.py`.

`backend/app/main.py` vẫn gọi:

```python
Base.metadata.create_all(bind=engine)
```

Điều này tiện cho demo SQLite, nhưng chưa phải cách quản lý schema chuyên nghiệp.

### 4.8. Data schema chưa khớp hoàn toàn SPEC

Theo `SPEC.md`, bảng candle cần có:

- adjusted OHLC
- `basic_close`
- index `(ticker, date desc)`
- dim tickers có exchange

Hiện code:

- `Candle` không có `basic_close`.
- `Symbol.exchange` nullable và importer chưa điền exchange chắc chắn.
- `volume` là Float, trong spec là BIGINT.
- Dùng `symbol/timestamp` thay vì `ticker/date`, điều này ổn nếu đã chọn `PRODUCT_SPEC.md`, nhưng cần ghi rõ.

Kết luận: code đang theo `PRODUCT_SPEC.md` local-first hơn là theo toàn bộ `SPEC.md`.

## 5. Đánh giá báo cáo của dev

### 5.1. "Đã hoàn thiện Manual Bar Replay bằng WebSocket, smooth scrolling, magnet tool"

Đúng một phần.

Có WebSocket auto-play, chart update, drawing và magnet snap logic.

Nhưng frontend build fail nên không thể coi là hoàn thiện.

Ngoài ra indicator API còn có kênh leak future data.

### 5.2. "Trade Engine đã pass 100% E2E, có T+2"

Chưa đủ căn cứ.

Code có T+2, nhưng test hiện đọc được có vẻ chưa cập nhật theo T+2.

Không chạy được test trong môi trường hiện tại vì thiếu Python runtime.

### 5.3. "Phân hệ Algorithmic Engine chưa có"

Đúng về mặt sản phẩm.

Có skeleton kỹ thuật, nhưng chưa usable.

### 5.4. "Analytics mới chỉ có nhật ký lệnh cơ bản"

Không hoàn toàn đúng.

Đã có Analytics page và service tính nhiều metric. Nhưng các công thức cần làm lại để đạt chuẩn.

### 5.5. "Limit Order, trần/sàn, force liquidate còn thiếu"

Đúng một phần.

- Limit/trần/sàn: có ở domain skeleton, chưa nối manual app.
- Force liquidate: đã có code cơ bản, nhưng cần test và polish.

## 6. Ưu tiên hành động

Thứ tự nên làm:

1. Làm app build/test được.
2. Khóa no-future-leak cho indicator.
3. Sửa trade lifecycle/T+2/tests/PnL.
4. Nối Limit Order và trần/sàn vào manual flow.
5. Làm lại analytics theo chuẩn.
6. Dọn migration/schema.
7. Sau đó mới làm Algorithmic Engine.

Không nên nhảy ngay vào Strategy Backtest Lab khi core manual replay vẫn build fail và test chưa rõ.

## 7. Definition of Done cho giai đoạn hardening

Giai đoạn hardening chỉ được coi là xong khi:

- `npm.cmd run build` pass.
- Backend test suite pass trong venv chuẩn.
- Test no-future-leak bao gồm cả candles và indicators.
- T+1 sell bị reject.
- T+2 sell pass.
- LIMIT order ngoài biên độ bị reject.
- Analytics dùng `session.initial_cash`, không hard-code capital.
- README/docs cập nhật đúng giới hạn hiện tại.
- Không còn tuyên bố "pass 100%" nếu chưa có log test thật.

