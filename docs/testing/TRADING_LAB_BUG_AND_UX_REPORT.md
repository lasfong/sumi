# TRADING LAB BUG & UX AUDIT REPORT (Replay Workspace)

Tài liệu này tổng hợp kết quả Audit chuyên sâu dựa trên mã nguồn Frontend/Backend và phân tích tiêu chuẩn UI/UX của Trading Lab, đối chiếu với các sản phẩm chuyên nghiệp như TradingView.

## 1. Hiệu năng Render Chart & Hiện tượng Giật/Lag (Stuttering & Flashing)
- **Mức độ nghiêm trọng**: P1 (Lỗi UX/Hiệu năng nghiêm trọng, gây trải nghiệm tệ)
- **Steps to Reproduce**: 
  - Thêm 2-3 chỉ báo (SMA, MACD, Bollinger Bands) vào chart.
  - Bật "Auto-Play" hoặc liên tục bấm Next (Spacebar).
- **Current Behavior**: 
  - Mỗi khi nhảy nến, `ReplayWorkspaceController` trigger Fetch API `getSessionIndicatorData` để tính toán lại TOÀN BỘ dữ liệu lịch sử cho các indicator. 
  - Trong `SeriesManager.ts`, mỗi lần có data indicator mới, hệ thống thực hiện thao tác **xóa bỏ (tear-down) series cũ (`this.chart.removeSeries()`) và tạo mới lại (`this.chart.addSeries()`)**. Quá trình rebuild liên tục này gây ra hiện tượng chớp màn hình (flashing) và giật/lag cực mạnh.
- **Expected Behavior**: 
  - Biểu đồ chạy mượt mà. Update nến mới chỉ nên push data point mới vào biểu đồ.
- **Technical Suggestion**: 
  - **Frontend**: Trong `SeriesManager.ts`, kiểm tra nếu `seriesKey` đã tồn tại thì gọi hàm `series.setData(data)` (hoặc `series.update()`) thay vì xóa và tạo lại.
  - **Backend/Flow**: Tối ưu hóa luồng WebSocket để trả về delta-point của indicator cho nến mới thay vì tính lại toàn bộ lịch sử qua REST API cho mỗi nến.

## 2. Lỗi Kéo/Thả (Drag & Drop) vào Khoảng trống Tương lai (Future Whitespace)
- **Mức độ nghiêm trọng**: P1 (Blocker cho Phân tích Kỹ thuật)
- **Steps to Reproduce**: 
  - Chọn công cụ Trendline hoặc Risk-Reward.
  - Cố gắng click hoặc kéo điểm neo (anchor) thứ 2 vào vùng trống bên phải (tương lai) của nến hiện tại.
- **Current Behavior**: 
  - Không thể neo/vẽ vào vùng tương lai. Hàm `coordinateToTime(x)` của Lightweight Charts trả về `null` cho các vùng chưa có nến. Hàm `anchorAt` trong `SumiPrimitiveDrawingProvider.ts` kiểm tra `if (!time) return null;`, dẫn đến từ chối thao tác vẽ.
- **Expected Behavior**: 
  - Giống TradingView, user có thể vẽ Trendline, thiết lập Điểm chốt lời (Target) kéo dài ra tương lai (ví dụ 1 tháng sau).
- **Technical Suggestion**: 
  - Khởi tạo Data cho Chart cần chèn thêm các "nến ảo" (có thể trống dữ liệu) vào tương lai để time-scale kéo dài ra, hoặc tính toán nội suy thời gian tương lai thông qua logic khoảng cách business days.

## 3. Chỉ báo Ichimoku Cloud - Thiếu Mây Kumo và Bị cắt Tương lai
- **Mức độ nghiêm trọng**: P1 (Lỗi nghiệp vụ hiển thị Indicator)
- **Steps to Reproduce**: Bật chỉ báo Ichimoku Cloud.
- **Current Behavior**: 
  - **Mây Kumo không tô màu**: `IndicatorRenderRegistry.ts` định nghĩa Span A và Span B là 2 `LineSeries`. Lightweight Charts không tự động tô nền giữa 2 line, làm mây Kumo chỉ có 2 đường viền.
  - **Mất mây tương lai**: Đặc trưng của Ichimoku là Span A/B chìa ra 26 nến về tương lai. Tuy nhiên, endpoint `/sessions/{session_id}/indicators` dùng `ReplayService.get_candles` (chỉ lấy đến nến hiện tại). Kết quả là đám mây bị cắt cụt.
- **Technical Suggestion**: 
  - Dùng Custom Plugin cho Lightweight Charts để render polygon/fill-area cho Span A và B.
  - Thiết kế API hỗ trợ ngoại lệ: Trả về tọa độ thời gian tương lai cho Span A/B mà không kèm dữ liệu giá (price) tương lai, giúp render mây mà không làm rò rỉ nến.

## 4. Công cụ Risk-Reward Tool - Không Cập nhật Ratio khi kéo thả
- **Mức độ nghiêm trọng**: P2 (Lỗi UI/UX gây nhầm lẫn)
- **Steps to Reproduce**: Vẽ Risk-Reward tool. Kéo điểm neo Entry/Target/Stop Loss.
- **Current Behavior**: 
  - Vùng xanh/đỏ resize đúng, nhưng các nhãn Text (Entry, Stop, Target, Ratio R:R) không thay đổi. 
  - Hàm `updateDrag` trong `SumiPrimitiveDrawingProvider.ts` chỉ update toạ độ `anchors` mà KHÔNG tính lại biến `geometry.riskRewardRatio`.
- **Expected Behavior**: 
  - Tỉ lệ R:R phải nhảy số liên tục (real-time) theo vị trí trỏ chuột.
- **Technical Suggestion**: 
  - Thay vì phụ thuộc biến tĩnh `drawing.geometry.riskRewardRatio`, hãy tính tỉ lệ R:R trực tiếp từ khoảng cách của `projected.anchors` lúc render (`paneViews -> draw`).

## 5. Lỗi Thiếu Hiển thị Đường Vị thế (Position / Orders Lines) trên Chart
- **Mức độ nghiêm trọng**: P2 (Thiếu tính năng cơ bản của Trading Platform)
- **Steps to Reproduce**: Đặt một lệnh BUY, có Stop Loss và Take Profit.
- **Current Behavior**: 
  - UI chỉ xuất hiện mũi tên Marker tại nến entry. Trên Chart KHÔNG hề có các đường kẻ ngang thể hiện mức giá Mua hiện tại, giá Cắt lỗ (SL), và Chốt lời (TP) của Vị thế đang mở (hoặc lệnh Pending). 
- **Expected Behavior**: 
  - Tương tự TradingView, vị thế (Open Position) phải có đường kẻ ngang kéo dài, cho phép user quan sát dễ dàng, thậm chí có thể nắm kéo trực tiếp để dời SL/TP.

## 6. Lỗi Nghiêm trọng: Sai Logic Luật T+2 khi kết hợp Lệnh Limit
- **Mức độ nghiêm trọng**: P1 (Lỗi Data / Business Logic)
- **Steps to Reproduce**: 
  - User tạo Lệnh Limit BUY ở Nến `T` (decision index = T).
  - Giá không khớp ngay mà mãi đến nến `T + 4` mới khớp (execution index = T + 4).
  - Ở nến `T + 5`, user nhấn SELL toàn bộ vị thế.
- **Current Behavior**: 
  - Trong `TradeLifecycleService._execute_sell` và `practice_workflow_service.py`, hàm tính lượng cổ phiếu đã về tài khoản (`settled_bought`) được filter theo: `Decision.candle_index <= current_index - 2`.
  - Lúc này ở nến `T + 5`, `current_index = T + 5`. Hệ thống sẽ check: `Decision.candle_index (là T) <= T + 5 - 2` (Đúng).
  - Kết quả: Cho phép user Bán ngay lập tức cổ phiếu vừa mới khớp ở T+4 (sai hoàn toàn luật T+2 của thị trường cơ sở VN).
- **Expected Behavior**: 
  - Luật T+2 phải tính toán dựa trên **Thời điểm khớp lệnh** (Execution), KHÔNG PHẢI **Thời điểm ra quyết định** (Decision).
- **Technical Suggestion**: 
  - Update mô hình để lưu trữ/tính toán số nến kể từ `execution_date` thay vì dùng `Decision.candle_index`.

## 7. Thiếu Hỗ trợ Multi-Timeframe Analysis (MTFA)
- **Mức độ nghiêm trọng**: P2 (Thiếu tính năng cốt lõi của Replay)
- **Current Behavior**: 
  - Backend API (`/api/replay/sessions/{session_id}/candles`) đã có tham số `target_timeframe`, nhưng trên UI `ReplayWorkspace` không hề có dropdown hay nút để chuyển đổi Timeframe (VD: đang xem 1D muốn zoom xuống 1H hoặc up lên 1W).
- **Expected Behavior**: 
  - Phân tích đa khung thời gian là chìa khóa của Technical Analysis. UI cần có công cụ Timeframe Switcher tích hợp trên Header hoặc Toolbar.

## 8. Lệnh Tập Giao dịch Bỏ qua Luật Lô chẵn 100
- **Mức độ nghiêm trọng**: P1 (Lỗi Nghiệp vụ)
- **Current Behavior**: `TradeLifecycleService.process_decision` cho phép khớp các lệnh lẻ tẻ (VD: 150 cổ phiếu). Không tuân thủ lô giao dịch 100 của VN.
- **Technical Suggestion**: Thêm validation `if qty % 100 != 0: raise HTTPException` vào đầu luồng xử lý hoặc auto-round ở Frontend.
