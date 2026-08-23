# TRADING LAB TEST CASES SPECIFICATION (Replay Workspace)

Tài liệu này định nghĩa các Use Cases và Test Cases cốt lõi để kiểm thử tính năng Trading Lab (Replay Workspace) trong ứng dụng Sumi.

## UC-01: Thao tác Đồ thị Nến & Playback Bar-by-Bar

### Mục đích
Đảm bảo luồng điều khiển thời gian (Playback) hoạt động chính xác, không rò rỉ dữ liệu tương lai và hiển thị nến trơn tru.

### Test Cases
- **TC-01.01:** Play/Pause Replay. Nhấn Play, nến nhảy liên tục. Pause dừng lập tức.
- **TC-01.02:** Đổi tốc độ (1x, 2x, 5x, 10x). Tốc độ render thay đổi, không bị khựng hoặc chớp nháy toàn bộ canvas.
- **TC-01.03:** Step Forward/Backward (Bar-by-Bar) qua Hotkeys (Mũi tên trái/phải). Đồ thị, indicator, drawing tool phải bám sát.

---

## UC-02: Quản lý & Hiển thị Chỉ báo Kỹ thuật (Technical Indicators)

### Mục đích
Xác nhận các chỉ báo kỹ thuật tính toán chuẩn xác, vẽ đúng trên chart và tuỳ chỉnh giao diện linh hoạt.

### Test Cases
- **TC-02.01:** Bật cùng lúc nhiều Indicator (Overlay & Sub-pane). Đảm bảo không bị giật lag, UI pane tự scale.
- **TC-02.02:** Thay đổi Config (Period, Color). Đảm bảo chart render lại ngay lập tức giá trị mới.
- **TC-02.03 (Ichimoku Cloud):** Mây Kumo phải được tô màu nền (fill) giữa Span A/B. Span A/B phải chìa về tương lai 26 nến so với nến hiện tại mà không làm lộ nến tương lai.

---

## UC-03: Công cụ Vẽ Đồ thị (Drawing Tools)

### Mục đích
Đảm bảo công cụ vẽ bám giá (magnet), hiển thị tốt, và có thể lưu/load chính xác.

### Test Cases
- **TC-03.01 (Future Drawing):** Chọn công cụ Trendline hoặc RR, click điểm neo thứ 2 ra vùng khoảng trống tương lai bên phải đồ thị. Đảm bảo UI ghi nhận toạ độ và vẽ ra đúng hình thay vì từ chối thao tác.
- **TC-03.02 (RR Ratio Real-time):** Dùng công cụ Risk-Reward kéo điểm Entry, SL, TP. Đảm bảo nhãn text hiển thị Tỉ lệ R:R nhảy số real-time theo trỏ chuột (thay vì bị đóng băng).
- **TC-03.03 (Persistence):** Vẽ tool, đổi Timeframe, hoặc chuyển qua tab Analytics/Journal rồi quay lại. Drawing tool không bị mất.

---

## UC-04: Đặt Lệnh Tập Luyện & Quản trị Vị thế (Practice Order Execution)

### Mục đích
Kiểm thử luồng giao dịch mô phỏng, tính toán PnL, Position Sizing, phí thuế và luật T+2 của thị trường chứng khoán Việt Nam.

### Test Cases
- **TC-04.01 (Lot Size Validation):** Cố ý đặt lệnh Buy/Sell với số lượng lẻ (vd: 150 cổ phiếu). Đảm bảo hệ thống từ chối và báo lỗi Lô 100.
- **TC-04.02 (Chart Position Lines):** Đặt một lệnh BUY Limit hoặc Market. Đảm bảo trên Chart sinh ra các đường Line ngang thể hiện Giá vào lệnh (Entry), Stop Loss, Take Profit (như TradingView).
- **TC-04.03 (T+2 Settlement on Execution):** Đặt lệnh Limit BUY tại nến T. Lệnh khớp lúc nến T+4. Đến nến T+5, tiến hành đặt lệnh Bán. Đảm bảo hệ thống **Block** giao dịch bán vì hàng chưa về (T+2 tính từ ngày Khớp T+4, tức T+6 mới cho bán). UI báo lỗi hiển thị rành mạch "Blocked T+2".
- **TC-04.04 (PnL Realtime):** Unrealized PnL nhảy số tự động theo giá đóng cửa của nến hiện tại trong lúc Playback.

---

## UC-05: Chế độ Blind Replay & Anti-Cheat

### Mục đích
Bảo vệ tính toàn vẹn của dữ liệu tương lai, ngăn user "đoán" kết quả.

### Test Cases
- **TC-05.01:** Bật Network tab ở DevTools. Đảm bảo API chỉ trả về dữ liệu nến tính đến `current_index`, tuyệt đối không tải thừa nến ẩn.

---

## UC-06: Đa Khung Thời Gian (Multi-Timeframe Analysis)

### Mục đích
Kiểm thử khả năng theo dõi đa khung thời gian trong Replay.

### Test Cases
- **TC-06.01 (Timeframe Switcher):** Trên UI ReplayWorkspace, sử dụng Dropdown để chuyển đổi nhanh Timeframe (1D, 1W, 1H) trong cùng một session.
- **TC-06.02 (Indicator MTFA):** Khi chuyển khung thời gian, Indicator (vd: RSI, MACD) cũng tự động reset và tính toán lại dựa trên dữ liệu của khung thời gian mới, hiển thị mượt mà.
