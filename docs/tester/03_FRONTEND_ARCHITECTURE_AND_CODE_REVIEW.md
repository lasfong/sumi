# Giai đoạn 3: Review Kiến trúc Frontend & Component Lifecycle (Sumi V3 RC)

> **Ngày thực hiện:** 25/07/2026
>
> **Phạm vi kiểm tra:** Lớp Replay Workspace (`frontend/src/components/replay/`), Charting Subsystem (`frontend/src/components/chart/`), Feature Domain Modules (`frontend/src/features/`)
> **Trạng thái:** Hoàn thành Giai đoạn 3 của Kế hoạch Nghiên cứu (`docs/tester/SUMI_PROJECT_RESEARCH_AND_REVIEW_PLAN.md`)

---

## I. TỔNG QUAN KIẾN TRÚC FRONTEND V3 (ADR-001 IMPLEMENTATION)

Đợt nâng cấp **Sumi V3** đã thực hiện tái cấu trúc controlled rebuild đối với lớp Replay Frontend. Từ một file `ReplayPage.tsx` đơn lẻ chứa quá nhiều trách nhiệm ở V2, V3 đã tách thành kiến trúc phân lớp đa tầng rõ ràng:

```text
                                  ReplayPage.tsx
                                        |
                                        v
                          ReplayWorkspaceController.tsx
                      (Application State & Logic Controller)
                                        |
        +-------------------------------+-------------------------------+
        |                               |                               |
        v                               v                               v
ReplayWorkspace.tsx             Drawing Subsystem              Indicator Subsystem
 (Layout & Sub-panels)      (SumiPrimitiveDrawingProvider)    (IndicatorRequestCoordinator)
  ├── PracticeRail                   ├── Selection & Handles           ├── IndicatorManager
  ├── PracticeJournal                ├── Geometry & Magnet             ├── PaneManager
  ├── PositionPanel                  ├── Undo/Redo History             ├── SeriesManager
  └── TradeControls                  └── Versioned Schema              └── RenderRegistry
```

---

## II. ĐÁNH GIÁ CHI TIẾT CÁC THÀNH PHẦN CORE

### 1. Replay Workspace Subsystem (`frontend/src/components/replay/`)
- **`ReplayWorkspaceController.tsx` (24KB):** Đóng vai trò làm Controller trung tâm của không gian Replay. Quản lý trạng thái phiên, nến hiện tại, vị thế, danh sách lệnh chờ, lịch sử giao dịch và tích hợp chính sách phím tắt toàn cục (`globalShortcutPolicy.ts`).
- **`ReplayWorkspace.tsx` (15KB):** Chịu trách nhiệm render bố cục giao diện (Layout Facade). Đảm bảo phân chia tỷ lệ hiển thị giữa Biểu đồ chính và các bảng phụ (Trade controls, Position, Journal) hợp lý trên độ phân giải màn hình tiêu chuẩn **1440×1000** và **1280×800**.
- **`PracticeRail.tsx` & `PracticeJournal.tsx`:** Tích hợp quy trình tập luyện thực hành (Practice Workflow). Đạt tiêu chuẩn truy nhập ARIA (Tab/Tabpanel), cho phép mở nhật ký, đánh giá checklist trước khi vào lệnh mà không làm mất bối cảnh (context) biểu đồ.

---

### 2. Charting & Lightweight Charts v5 Integration (`frontend/src/components/chart/`)
- **`CandleChart.tsx` & `workspaceTypes.ts`:** Facade kết nối trực tiếp với engine render `lightweight-charts@5.2.0`.
- **`PaneManager.ts`:** Gọi API chính thức `chart.addPane(true)` của v5 để tạo các phân vùng độc lập cho Oscillators. Quản lý chiều cao tối thiểu, tính giãn cách và tự động xóa pane khi chỉ báo cuối cùng bị gỡ bỏ.
- **`SeriesManager.ts`:** Quản lý vòng đời của Nến, Volume, các đường chỉ báo (LineSeries, HistogramSeries). Đồng bộ duy nhất một trục thời gian (Time scale) chung cho tất cả các pane.
- **`IndicatorRenderRegistry.ts`:** Định nghĩa quy tắc vẽ cho từng loại chỉ báo từ dữ liệu backend (Vd: cấu hình đường MACD, tín hiệu signal, cột histogram; thang đo RSI 0-100 với đường 30/50/70; CCI với đường -100/0/100).

---

### 3. Professional Indicator Manager (`frontend/src/components/chart/` & `features/indicators/`)
- **`IndicatorManager.tsx` & `IndicatorPaneChrome.tsx`:** Cung cấp giao diện quản lý chỉ báo chuyên nghiệp:
  - Danh sách chỉ báo đang bật (Active list) luôn hiển thị rõ ràng.
  - Hỗ trợ xem, sửa tham số (Modal settings), ẩn/hiện (Toggle visibility), và xóa từng chỉ báo đơn lẻ bằng nút bấm trực quan.
  - Hiển thị tiêu đề (Pane title), thông số huyền thoại (Legend) và giá trị nến hiện tại ở góc pane.
- **`IndicatorRequestCoordinator.ts`:** Bộ điều phối request thông minh. Tự động hủy bỏ các request chỉ báo bị lỗi thời khi người dùng chuyển phiên/tua nến nhanh, loại bỏ các request trùng lặp và loại bỏ lỗi rác ngoài console.

---

### 4. Advanced Drawing System Subsystem (`frontend/src/features/drawings/`)
- **`SumiPrimitiveDrawingProvider.ts` (23KB):** Hiện thực hóa hoàn hảo interface `DrawingProvider` bằng cách sử dụng các Primitives chính quy của Lightweight Charts.
- **Danh mục 7 công cụ TA tiêu chuẩn:**
  1. Cursor / Select Tool
  2. Horizontal Line
  3. Trendline
  4. Ray
  5. Rectangle
  6. Fibonacci Retracement (đầy đủ các cấp độ 0, 0.236, 0.382, 0.5, 0.618, 0.786, 1.0 với nhãn % trực quan)
  7. Text / Note
- **Tính năng nâng cao:**
  - **Mô hình Tương tác (Interaction Model):** Cho phép chọn hình vẽ (Selection bounds), hiển thị các điểm neo (Anchor handles), kéo di chuyển hoặc sửa điểm neo.
  - **Nam châm (Magnet Snapping - `drawingMagnet.ts`):** Tự động hít điểm vẽ vào các mức OHLC của nến gần nhất.
  - **Undo/Redo History (`DrawingCommandHistory.ts`):** Hỗ trợ hoàn tác và làm lại mọi thao tác vẽ/sửa/xóa.
  - **Lưu trữ chuẩn hóa (`DrawingRepository.ts`):** Lưu trữ theo JSON Schema phiên bản v1 (`sumi-drawing-document-v1.schema.json`), đảm bảo khả năng khôi phục trọn vẹn sau khi reload trang mà không phụ thuộc vào cấu trúc riêng của thư viện bên thứ 3.

---

## III. ĐÁNH GIÁ TÍNH TRUY CẬP VÀ PHÍM TẮT (ACCESSIBILITY & SHORTCUTS)

- **`useModalFocus.ts`:** Đảm bảo khi mở các cửa sổ modal (như cài đặt chỉ báo, ghi nhật ký), con trỏ focus được giữ gọn bên trong modal, hỗ trợ phím `Escape` để đóng và trả lại focus cho nút mở trước đó.
- **`globalShortcutPolicy.ts`:** Phân tách thông minh giữa phím tắt ứng dụng (Space để tua nến, Delete để xóa hình vẽ, v.v.) và việc gõ văn bản trong các thẻ input/textarea của Journal, tránh tình trạng vừa gõ chữ vừa lỡ tay nhảy nến Replay.

---

## IV. KẾT LUẬN GIAI ĐOẠN 3

Mã nguồn Frontend Sumi V3 là một bước tiến vượt bậc so với bản V2:
- **Tương tác chuyên nghiệp:** Hệ thống công cụ vẽ (Drawing) và quản lý chỉ báo (Indicator Manager) đạt đẳng cấp của các công cụ phân tích kỹ thuật thương mại chuyên nghiệp.
- **Trải nghiệm mượt mà:** Khắc phục triệt để các lỗi đè nhãn, rò rỉ dữ liệu request, giật lag khi tua nến.
- **Tuân thủ thiết kế:** Tách biệt hoàn toàn giữa UI Component, Feature Domain State và Rendering Engine Adapter.

**Bước tiếp theo:** Chuyển sang **Giai đoạn 4: Phân tích Quy trình Kiểm thử (Testing & Quality Assurance Inspection)**.
