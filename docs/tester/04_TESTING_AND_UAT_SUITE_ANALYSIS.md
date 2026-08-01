# Giai đoạn 4: Phân tích Quy trình Kiểm thử & Test Harness (Sumi V3 RC)

> **Ngày thực hiện:** 25/07/2026
>
> **Phạm vi kiểm tra:** Backend Pytest (`backend/app/tests/`), Frontend Jest/Vitest (`frontend/src/**/__tests__/`), Automated Product UAT (`scripts/product-uat.mjs`, `verify-product.sh`)
> **Trạng thái:** Hoàn thành Giai đoạn 4 của Kế hoạch Nghiên cứu (`docs/tester/SUMI_PROJECT_RESEARCH_AND_REVIEW_PLAN.md`)

---

## I. TỔNG QUAN HỆ THỐNG KIỂM THỬ SUMI V3

Dự án Sumi áp dụng nguyên tắc **"Bằng chứng kiểm thử là cơ quan thẩm quyền duy nhất" (Evidence-backed quality authority)**. Một tính năng không được coi là hoàn thành chỉ vì code chạy thông qua unit test; nó bắt buộc phải vượt qua các bài kiểm thử UAT thực tế trên trình duyệt với ảnh chụp màn hình và log bằng chứng được niêm phong.

Hệ thống kiểm thử gồm 3 tầng khép kín:

```text
+-------------------------------------------------------------------------+
|                  Tầng 1: Fast Technical Gate                            |
|    ./scripts/verify-v2.sh  (Backend pytest + Frontend lint/build)       |
+-------------------------------------------------------------------------+
                                     |
                                     v
+-------------------------------------------------------------------------+
|                  Tầng 2: Integration & Parity Suite                     |
|    Pytest (test_indicator_parity, test_replay_no_future_leak, ...)      |
|    Frontend Unit Tests (DrawingProvider, IndicatorCoordinator)          |
+-------------------------------------------------------------------------+
                                     |
                                     v
+-------------------------------------------------------------------------+
|                  Tầng 3: Automated Product UAT Suite                    |
|    ./scripts/verify-product.sh -> scripts/product-uat.mjs (Playwright)  |
|    - 277 Assertion Checks | 1440x1000 Screenshot Evidence              |
|    - Sealed Manifest Bundle (SHA-256 Verified)                          |
+-------------------------------------------------------------------------+
```

---

## II. PHÂN TÍCH BACKEND TEST SUITE (`backend/app/tests/`)

Bộ test backend bao gồm **20 file test** chuyên biệt phủ toàn bộ logic nghiệp vụ:

| Tên file test | Phạm vi & Mục tiêu kiểm thử |
| :--- | :--- |
| `test_replay_no_future_leak.py` | Khóa chặn quy tắc không rò rỉ nến tương lai. Đảm bảo API chỉ trả dữ liệu đến `current_index`. |
| `test_indicator_parity_e2e.py` | Kiểm tra tính nhất quán (parity) giữa chỉ báo do `IndicatorEngine` tính toán và dữ liệu xuất qua `StrategyIndicatorAdapter`. |
| `test_trade_lifecycle.py` (19.9KB) | Kiểm thử toàn bộ vòng đời lệnh: Đặt lệnh, hủy lệnh, khớp lệnh theo nến, kiểm tra quy tắc T+2, tính PnL và margin. |
| `test_practice_workflow.py` (19.0KB) | Kiểm thử quy trình thực hành giao dịch: Checklist trước lệnh, nhật ký quyết định, liên kết dấu mốc Replay. |
| `test_backtest_cleanup.py` | Đảm bảo tính năng dọn dẹp phiên rác hoạt động chính xác và cách ly hoàn toàn các phiên Manual Replay. |
| `test_accounting.py` | Kiểm tra tính toán sổ sách kế toán, PnL đã thực hiện (realized) và chưa thực hiện (unrealized). |
| `test_ws_replay.py` | Kiểm tra kết nối WebSocket Replay truyền tải nến thời gian thực chuẩn timezone. |

---

## III. PHÂN TÍCH FRONTEND TEST SUITE (`frontend/src/**/__tests__/`)

Bộ test frontend tập trung kiểm thử các module quản lý state và tương tác phức tạp:
1. **`SumiPrimitiveDrawingProvider.test.ts`:** Kiểm tra việc tạo 7 loại công cụ vẽ, tính toán hình học (geometry), snapping nam châm, hoàn tác (undo/redo), và chuyển đổi JSON schema.
2. **`IndicatorRequestCoordinator.test.ts`:** Kiểm tra cơ chế hủy request lỗi thời, loại bỏ request trùng lặp và xử lý các lỗi mạng.
3. **`PaneManager.test.ts` & `SeriesManager.test.ts`:** Kiểm tra việc khởi tạo/hủy phân vùng v5, thêm/xóa series đồ họa mà không bị rò rỉ bộ nhớ (memory leak).
4. **`DrawingInspector.test.tsx` & `PracticeWorkflow.test.tsx`:** Kiểm tra khả năng tương tác phím tắt, ARIA tab navigation và hiển thị modal.

---

## IV. PHÂN TÍCH HỆ THỐNG AUTOMATED PRODUCT UAT (`scripts/product-uat.mjs`)

Đây là trái tim của quy trình kiểm định chất lượng Sumi V3:
- **Kịch bản thực thi (`product-uat.mjs` - 180KB):** Tự động khởi chạy một instance browser (qua Playwright), tương tác với giao diện Replay như một trader thực thụ trong hơn **30 phút (1,800 giây)**.
- **277 Điểm kiểm định (Assertion IDs):** Bao phủ toàn bộ các yêu cầu trong `PRODUCT_ACCEPTANCE_CRITERIA_V3.md`:
  - **G-01 -> G-05:** Không xuất hiện bất kỳ lỗi Javascript Console hay lỗi mạng 500 nào.
  - **R-01 -> R-05:** Nến không bị rò rỉ; khôi phục đúng trạng thái khi tua nến hoặc F5 reload.
  - **I-01 -> I-13:** Thêm/sửa/ẩn/xóa chỉ báo thành công; MACD, RSI, CCI hiển thị đúng thang đo.
  - **D-01 -> D-11:** Vẽ thử 50 hình vẽ (Horizontal, Trendline, Ray, Rectangle, Fibonacci, Text), kiểm tra kéo di chuyển, nam châm, undo/redo và reload không bị mất.
  - **T-01 -> T-05:** Đặt lệnh Mua/Bán, kiểm tra T+2, lưu nhật ký giao dịch mượt mà.
- **Bằng chứng Niêm phong (Sealed Evidence Bundle):**
  - Kết quả kiểm thử được xuất ra gói niêm phong tại `test-results/batch5-hardening/YYYY-MM-DD.../manifest.json`.
  - Toàn bộ file kết quả (`results.json`), CSDL khôi phục (`restored.db`), và ảnh chụp màn hình UI (**1440×1000**) đều được tính mã băm **SHA-256** để đảm bảo tính minh bạch, không thể làm giả.

---

## V. KẾT LUẬN GIAI ĐOẠN 4

Hệ thống test harness và UAT tự động của Sumi V3 cực kỳ chặt chẽ:
- **Tự động hóa hoàn toàn:** Chỉ cần chạy lệnh `./scripts/verify-product.sh` là toàn bộ quy trình kiểm thử từ backend, frontend đến browser UAT đều được tự động hóa.
- **Minh bạch và Đáng tin cậy:** Cơ chế niêm phong bằng chứng bằng SHA-256 ngăn chặn mọi hành vi báo cáo khống ("fake pass").

**Bước tiếp theo:** Chuyển sang **Giai đoạn 5: Tổng hợp Ma trận Đánh giá & Báo cáo Tổng kết** để lập báo cáo đánh giá cuối cùng về dự án Sumi V3 Release Candidate.
