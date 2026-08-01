# Giai đoạn 1: Mã hóa & Phân tích Hệ thống Tài liệu Chuẩn Sumi V3

> **Ngày thực hiện:** 25/07/2026
>
> **Người thực hiện:** Antigravity AI (Chuyên viên Review & Đánh giá Dự án)
> **Trạng thái:** Hoàn thành Giai đoạn 1 của Kế hoạch Nghiên cứu (`docs/tester/SUMI_PROJECT_RESEARCH_AND_REVIEW_PLAN.md`)

---

## I. TỔNG QUAN TÀI LIỆU CHUẨN (CANONICAL V3 DOCUMENTS)

Toàn bộ quá trình phát triển phiên bản **Sumi V3** được điều hành bởi 7 tài liệu chuẩn cốt lõi (Canonical Documents). Các tài liệu V2 trước đây chỉ mang tính chất tham chiếu lịch sử (Historical evidence), không còn là hợp đồng nghiệm thu chính cho V3.

| STT | Tài liệu | Vai trò / Nội dung cốt lõi |
| :--- | :--- | :--- |
| 1 | `docs/PRODUCT_V3_PLAN_2026-07-15.md` | Định hướng sản phẩm V3, mục tiêu nâng cấp không phải là clone TradingView mà là tạo ra một Workstation tập luyện phân tích kỹ thuật (TA) nhất quán, tin cậy cho thị trường Việt Nam. |
| 2 | `docs/PRODUCT_ACCEPTANCE_CRITERIA_V3.md` | Hợp đồng nghiệm thu V3 với 254+ mã điều kiện (Acceptance IDs) bao quát 5 nhóm: Global Quality (G-*), Replay Integrity (R-*), Indicator Manager (I-*), Drawing System (D-*), và Trading Practice (T-*). |
| 3 | `docs/ARCHITECTURE_DECISION_001_REPLAY_UI_REBUILD.md` | Quyết định ADR-001 lựa chọn phương án "Controlled Frontend Rebuild": Giữ nguyên Lightweight Charts v5 và backend engines, xây dựng lại toàn bộ lớp Replay UI, Indicator Manager và Drawing Subsystem. |
| 4 | `docs/DEVELOPMENT_OPERATING_MODEL.md` | Mô hình vận hành phát triển theo Bounded Batches (Từng lô độc lập), phân định rõ vai trò Reviewer/Orchestrator và DEV task. |
| 5 | `docs/PROJECT_REVIEW_REPORT_2026-07-15.md` | Báo cáo đánh giá hiện trạng dự án trước V3, vạch ra các điểm hạn chế của bản RC2 (thiếu Indicator Manager, công cụ vẽ ở dạng prototype đơn sơ). |
| 6 | `AGENTS.md` | Bộ quy tắc vận hành dự án bắt buộc đối với AI Agent và Developer (Bảo toàn invariants, DB isolation, no telemetry). |
| 7 | `PLANS.md` | Quy chuẩn định dạng tài liệu kế hoạch thực thi (ExecPlan). |

---

## II. QUYẾT ĐỊNH KIẾN TRÚC ADR-001 (REPLAY UI REBUILD)

### 1. Bối cảnh
Trước V3, file `ReplayPage.tsx` gánh vác quá nhiều trách nhiệm (595+ dòng code), vừa quản lý state Replay, vừa kết nối WebSocket, vừa vẽ chỉ báo và lưu công cụ vẽ. Các công cụ vẽ cũ chỉ là những đường giá/chuỗi điểm đơn sơ (`LineSeries` cơ bản), không có mô hình tương tác chuyên nghiệp (không thể chọn, di chuyển, chỉnh sửa anchor point, hoặc undo/redo).

### 2. Định hướng "Controlled Rebuild"
Hệ thống quyết định phân tách ranh giới rõ ràng:

```text
+-----------------------------------------------------------------------+
|                           Replay Page / Route                         |
+-----------------------------------------------------------------------+
                                    |
                                    v
+-----------------------------------------------------------------------+
|                     Replay Workspace Controller                       |
|   - Quản lý Application State, Session Lifecycle, Order/Trade state   |
+-----------------------------------------------------------------------+
                                    |
        +---------------------------+---------------------------+
        |                                                       |
        v                                                       v
+-------------------------------+               +-------------------------------+
|     ChartWorkspace Facade     |               |      PracticeRail / Journal   |
|   - Lightweight Charts v5.2   |               |   - Bảng đặt lệnh, PnL        |
|   - Pane & Series Managers    |               |   - Nhật ký quyết định        |
|   - Indicator Renderer Reg    |               |   - Checklist giao dịch       |
|   - Sumi Drawing Provider     |               +-------------------------------+
+-------------------------------+
```

### 3. Phân định Giữ lại vs Sửa đổi:
- **GIỮ NGUYÊN (KEEP):**
  - Lightweight Charts v5.2 làm engine render đồ họa chính.
  - Backend FastAPI, SQLAlchemy, SQLite, và `IndicatorEngine` làm nguồn sự thật (Source of Truth) cho dữ liệu nến và chỉ báo.
  - Luồng giao dịch (Trade lifecycle), tính toán PnL, T+2 settlement, và các quy tắc khớp lệnh.
  - Không gửi dữ liệu người dùng ra bên ngoài (Local-first).
- **XÂY DỰNG LẠI (REBUILD):**
  - Giao diện Replay Workspace và bố cục phân cấp thông tin.
  - Hệ thống **Indicator Manager** (thêm/sửa/xóa/ẩn/hiện/sắp xếp chỉ báo, quản lý pane chuyên biệt cho RSI, MACD, CCI).
  - Hệ thống **Drawing Provider Adapter** (hỗ trợ đầy đủ Cursor, Horizontal Line, Trendline, Ray, Rectangle, Fibonacci Retracement, Text/Note với khả năng chọn, kéo thả, snap nam châm, undo/redo).
  - Bộ kiểm thử tự động hóa UAT tích hợp Playwright để chụp ảnh màn hình và kiểm tra console errors.

---

## III. BẮT BÚỢC TUÂN THỦ NGUYÊN TẮC (SYSTEM INVARIANTS)

Trong bất kỳ hoàn cảnh nào, các nguyên tắc hệ thống sau đây phải được bảo toàn:
1. **Không rò rỉ nến tương lai (No Future Leak):** API Replay chỉ được trả dữ liệu nến tính từ nến bắt đầu đến `current_index`. Tuyệt đối không trả toàn bộ nến về frontend rồi cắt trên browser.
2. **Backend Authoritative Indicator:** Mọi giá trị Indicator hiển thị hoặc dùng trong Backtest/Scanner phải do backend `IndicatorEngine` tính toán. Frontend chỉ chịu trách nhiệm render (màu sắc, độ rộng, nhãn).
3. **Cách ly CSDL khi Test:** Tất cả bài test tự động và script UAT phải sử dụng CSDL tạm thời (VD: `sqlite:////tmp/sumi-test.db`). Tuyệt đối không ghi đè hoặc làm biến đổi `backend/sumi.db`.
4. **Local-first Security:** Ứng dụng hoạt động hoàn toàn offline/local. Không tích hợp các dịch vụ theo dõi (telemetry) hoặc gửi dữ liệu giao dịch của người dùng tới server bên thứ 3.

---

## IV. ĐÁNH GIÁ MA TRẬN TIÊU CHUẨN NGHIỆM THU (ACCEPTANCE MATRIX V3)

V3 được nghiệm thu dựa trên 5 nhóm tiêu chí nghiêm ngặt:

1. **Global Quality (G-01 -> G-05):** Mọi bài test backend, frontend, lint, build và product UAT đều phải PASS với 0 lỗi console/page error.
2. **Replay Integrity (R-01 -> R-05):** Tua nến (Next/Prev/Autoplay) đồng bộ chính xác giữa biểu đồ, lệnh, vị thế, markers và chỉ báo; tính năng khôi phục phiên (Reload/Resume) hoạt động hoàn hảo.
3. **Indicator Manager (I-01 -> I-13):** Cho phép xem danh sách chỉ báo đang bật, thêm mới từ registry, sửa tham số, ẩn/hiện, xóa đơn lẻ. Các chỉ báo phân vùng (MACD, RSI, CCI, Volume) hiển thị tiêu đề, thang đo chuẩn và các đường tham chiếu (30/50/70 cho RSI, -100/0/100 cho CCI).
4. **Drawing System (D-01 -> D-11):** Hỗ trợ đầy đủ các công cụ TA tiêu chuẩn, có trạng thái chọn (selection bounds), chỉnh sửa điểm neo, di chuyển, xóa bằng phím Delete/Backspace, Undo/Redo, Snap nam châm, và lưu trữ dạng JSON có phiên bản (versioned schema).
5. **Trading Practice Workflow (T-01 -> T-05):** Đảm bảo diện tích biểu đồ không bị thu hẹp quá mức; các bảng đặt lệnh, nhật ký, checklist truy cập dễ dàng mà không làm mất context biểu đồ; người dùng có thể thực hiện một phiên tập luyện 30 phút mượt mà.

---

## V. KẾT LUẬN GIAI ĐOẠN 1

Tài liệu thiết kế của Sumi V3 rất chặt chẽ, đầy đủ và định hình rõ ranh giới giữa Backend (xử lý logic/dữ liệu) và Frontend (xử lý giao diện/tương tác). Dự án đã đóng gói thành công các lứa phát triển Batch 0 -> 5 và đạt mốc V3 Release Candidate.

**Bước tiếp theo:** Chuyển sang **Giai đoạn 2: Review Kiến trúc Backend & Domain Engine** để kiểm tra mã nguồn thực tế tại `backend/app/`.
