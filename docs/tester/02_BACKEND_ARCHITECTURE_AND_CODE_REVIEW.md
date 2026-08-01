# Giai đoạn 2: Review Kiến trúc Backend & Domain Engine (Sumi V3 RC)

> **Ngày thực hiện:** 25/07/2026
>
> **Phạm vi kiểm tra:** Lớp Domain (`backend/app/domain/`), Services (`backend/app/services/`), API (`backend/app/api/`)
> **Trạng thái:** Hoàn thành Giai đoạn 2 của Kế hoạch Nghiên cứu (`docs/tester/SUMI_PROJECT_RESEARCH_AND_REVIEW_PLAN.md`)

---

## I. TỔNG QUAN KIẾN TRÚC BACKEND

Backend của Sumi được xây dựng trên **FastAPI**, **SQLAlchemy**, **Pandas**, và **SQLite**. Kiến trúcBackend tuân thủ nghiêm ngặt nguyên tắc **Phân tách trách nhiệm (Separation of Concerns)**:
- **FastAPI Routes (`app/api/`):** Chỉ đóng vai trò Controller tiếp nhận HTTP/WebSocket request, validate input và gọi Service tương ứng. Không chứa business logic.
- **Service Layer (`app/services/`):** Chịu trách nhiệm quản lý nghiệp vụ chính (Replay session, vòng đời giao dịch, quy trình thực hành practice workflow, backtest, dọn rác session).
- **Domain Engine (`app/domain/`):** Nơi chứa các thuật toán cốt lõi (tính toán chỉ báo kỹ thuật `IndicatorEngine`, đánh giá luật chiến lược `StrategyRuleEvaluator`, khớp lệnh `Broker`, và hạch toán kế toán `accounting.py`).

```text
+-------------------------------------------------------------------------+
|                           FastAPI API Layer                             |
|    /api/replay  |  /api/backtest  |  /api/indicators  |  /api/ws/...   |
+-------------------------------------------------------------------------+
                                     |
                                     v
+-------------------------------------------------------------------------+
|                             Service Layer                               |
|   ReplayService | TradeLifecycleService | PracticeWorkflowService ...  |
+-------------------------------------------------------------------------+
                                     |
                                     v
+-------------------------------------------------------------------------+
|                             Domain Engine                               |
|   IndicatorEngine | StrategyRuleEvaluator | Broker | Accounting       |
+-------------------------------------------------------------------------+
```

---

## II. ĐÁNH GIÁ CHI TIẾT CÁC MODULE LỚP DOMAIN (`app/domain/`)

### 1. `IndicatorEngine` (`backend/app/domain/engine/indicator_engine.py`)
- **Vai trò:** Là Nguồn sự thật (Source of Truth) chịu trách nhiệm tính toán toàn bộ các chỉ báo kỹ thuật trong hệ thống.
- **Danh mục chỉ báo supported:**
  - Trend: `sma`, `ema`, `macd`
  - Oscillators: `rsi`, `stoch`, `cci`
  - Volatility: `bbands`, `atr`
  - Others: `adx`, `ichimoku`, `volume_sma`
- **Xử lý đặc biệt:** Đã tự code lại thuật toán tính toán chỉ báo **CCI (Commodity Channel Index)** chính quy nhằm khắc phục lỗi thứ tự độ ưu tiên (precedence defect) tồn tại trong thư viện `pandas-ta==0.4.71b0`.
- **Đánh giá:** Rất vững chắc. Kết quả tính toán đảm bảo tính nhất quán (parity) giữa đường hiển thị Replay và đường chạy chiến lược tự động.

### 2. `StrategyIndicatorAdapter` (`backend/app/domain/engine/strategy_indicator_adapter.py`)
- **Vai trò:** Cầu nối giữa Backtest/Scanner Engine và `IndicatorEngine`.
- **Đặc điểm:** Chuyển đổi các yêu cầu chỉ báo dạng danh sách tham số thành dữ liệu Pandas Series/DataFrame sẵn sàng cho bộ đánh giá luật chiến lược (`StrategyRuleEvaluator`).

### 3. `StrategyRuleEvaluator` (`backend/app/domain/strategy/strategy_rule_evaluator.py`)
- **Vai trò:** Bộ đánh giá tín hiệu giao dịch an toàn (declarative rule evaluation).
- **Đặc điểm:** Loại bỏ hoàn toàn việc sử dụng hàm nguy hiểm `eval()` của Python. Mọi điều kiện chiến lược (Vd: `rsi < 30`, `sma_20 cross_above sma_50`) được parse và so sánh khai báo an toàn.
- **Sử dụng chung:** Cả `BacktestService` và `ScannerService` cùng gọi qua module này, giúp tránh việc lệch logic tín hiệu giữa 2 tính năng.

### 4. `Accounting` (`backend/app/domain/accounting.py`) & `Broker` (`broker.py`)
- **Vai trò:** Quản lý tài khoản, kiểm soát quy tắc T+2 của thị trường chứng khoán Việt Nam, xử lý phí giao dịch, thuế và hạch toán PnL.
- **Đặc điểm:** Hỗ trợ đầy đủ các loại lệnh Market (MP), Limit (LO), Stop-loss và mô phỏng chính xác số dư tiền khả dụng/tiền chờ về.

---

## III. ĐÁNH GIÁ CHI TIẾT CÁC SERVICES NỔI BẬT (`app/services/`)

### 1. `ReplayService` (`backend/app/services/replay_service.py`)
- **Bảo toàn Quy tắc No-Future-Leak:** Khi frontend yêu cầu nến qua `get_candles()`, service truy vấn với điều kiện `limit = session.current_index + 1`. Điều này đảm bảo **tuyệt đối không bao giờ gửi nến tương lai về client**.
- **Quản lý phiên:** Hỗ trợ tạo phiên mới, tua tiến/lùi (next/prev), nhảy nến (jump), và khôi phục trạng thái làm việc (Resume).

### 2. `TradeLifecycleService` (`backend/app/services/trade_lifecycle_service.py`)
- **Vòng đời lệnh hoàn chỉnh:** Tiếp nhận lệnh từ người dùng -> kiểm tra sức mua/sức bán -> đưa vào trạng thái Pending -> khớp lệnh khi giá nến thỏa mãn -> cập nhật vị thế (Position) -> tính PnL khi đóng vị thế.
- **Hỗ trợ T+2:** Đảm bảo cổ phiếu mua ngày T chỉ trở thành cổ phiếu khả dụng để bán vào ngày T+2 theo quy định VN.

### 3. `PracticeWorkflowService` (`backend/app/services/practice_workflow_service.py`) & `JournalService`
- **Tích hợp thực hành V3:** Quản lý checklist kỷ luật trước khi vào lệnh, ghi chép lý do vào lệnh/thoát lệnh, đánh giá tâm lý giao dịch, và đồng bộ nhật ký với từng bước Replay.

### 4. `BacktestCleanupService` (`backend/app/services/backtest_cleanup_service.py`)
- **Dọn rác phiên tự động:** Cung cấp API `POST /api/backtest/cleanup-sessions` giúp xóa sạch các phiên backtest tạm thời cùng các bản ghi con (orders, executions, positions, journal...) nhằm tránh làm phình to CSDL SQLite local.
- **Cách ly an toàn:** Luôn kiểm tra điều kiện `ReplaySession.mode == "backtest"` để đảm bảo **không bao giờ xóa nhầm các phiên Manual Replay của người dùng**.

---

## IV. ĐÁNH GIÁ LỚP API VÀ DỮ LIỆU (`app/api/`, `models/`, `schemas/`)

- **FastAPI Endpoint Cleanliness:** Các file trong `app/api/` (như `replay.py`, `backtest.py`, `indicators.py`, `ws_replay.py`) tuân thủ đúng chuẩn RESTful API. Logic được chuyển toàn bộ vào Service layer.
- **WebSocket Replay (`ws_replay.py`):** Hỗ trợ truyền dữ liệu nến khi bật chế độ Autoplay với định dạng JSON chuẩn hóa múi giờ (timezone-independent daily candle serialization).
- **SQLite Database Integrity:** CSDL local được tổ chức qua SQLAlchemy ORM và Alembic migrations.

---

## V. KẾT LUẬN GIAI ĐOẠN 2

Mã nguồn Backend của Sumi V3 đạt chất lượng cao:
- **Đúng kiến trúc:** Phân lớp rõ ràng, không bị lẫn lộn giữa API route và Business logic.
- **Đảm bảo tính chính xác nghiệp vụ:** Giải quyết triệt để lỗi của thư viện bên thứ 3 (như CCI trong `pandas-ta`), áp dụng đúng luật T+2 của Việt Nam và quy tắc không rò rỉ nến tương lai (`no-future-leak`).
- **An toàn dữ liệu:** Xử lý cleanup phiên backtest rác có rào chắn bảo vệ phiên replay thủ công.

**Bước tiếp theo:** Chuyển sang **Giai đoạn 3: Review Kiến trúc Frontend & Component Lifecycle** để rà soát mã nguồn React/TypeScript tại `frontend/src/`.
