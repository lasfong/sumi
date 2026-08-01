# Đánh giá Sản phẩm Sumi V3 Dưới Góc nhìn Trader Thực chiến & Phân tích Khoảng trống Tính năng (Product Gap Analysis)

> **Ngày thực hiện:** 25/07/2026
>
> **Góc nhìn (Persona):** Nhà đầu tư / Trader cá nhân thực chiến tại Thị trường Chứng khoán Việt Nam (Giao dịch cổ phiếu VN30, HNX, phái sinh).
> **Vị trí tài liệu:** `docs/tester/REAL_TRADER_PRODUCT_REVIEW_AND_GAP_ANALYSIS.md`

---

## I. BỐI CẢNH VÀ NHU CẦU THỰC TẾ CỦA MỘT TRADER VIỆT NAM

Một Trader cá nhân trên thị trường chứng khoán Việt Nam khi tìm kiếm một ứng dụng rèn luyện giao dịch (Trading Practice Workstation) thường gặp các vấn đề lớn sau:
1. **Phần mềm ngoại (TradingView Pro/Premium) đắt đỏ:** Tính năng Bar Replay của TradingView yêu cầu trả phí hàng tháng cao, đồng thời không hỗ trợ các đặc thụ riêng của thị trường Việt Nam (như quy tắc thanh toán cổ phiếu T+2, biên độ sàn HOSE/HNX/UPCoM).
2. **Lo ngại lộ Chiến lược & Nhật ký cá nhân:** Nhiều trader không muốn lưu trữ nhật ký giao dịch, cảm xúc, hoặc quy tắc vào lệnh cá nhân trên các hệ thống Cloud/SaaS bên thứ 3.
3. **Cần một môi trường "Tua nến" chuẩn để rèn luyện tâm lý:** Rất nhiều trader bị thói quen "nhìn thấy trước quá khứ" làm hỏng tư duy khi nhìn chart tĩnh. Họ cần một công cụ tua từng nến (Bar Replay) nhưng **tuyệt đối không được lộ nến tương lai**.

---

## II. NHỮNG GÌ SUMI V3 ĐÃ ĐÁP ỨNG CỰC KỲ TỐT (STRENGTHS)

Đứng ở góc độ người dùng thật, Sumi V3 Release Candidate đã làm **rất xuất sắc** các điểm cốt lõi sau:

### 1. Trải nghiệm Replay "Sạch" & Chuẩn Quy tắc Việt Nam
* **No-Future-Leak chuẩn 100%:** Khi tua nến, người dùng hoàn toàn không nhìn thấy bất kỳ dấu vết nào của nến tương lai (không rò rỉ bóng nến, không rò rỉ giá trị chỉ báo hay nhãn giá tương lai). Điều này rèn luyện tâm lý đọc nến thời gian thực cực kỳ thật.
* **Hỗ trợ Quy tắc T+2 của Việt Nam:** Đây là tính năng "ăn tiền" mà TradingView hay các công cụ nước ngoài không có. Khi mua cổ phiếu ở ngày T0, Sumi chặn không cho bán ở ngày T0 và T+1, chỉ mở khả năng bán khi cổ phiếu về ở ngày T+2. Điều này giúp Trader tính toán đúng rủi ro kẹp hàng T+2.

### 2. Hệ thống Biểu đồ & Chỉ báo Kỹ thuật Đạt chuẩn Chuyên nghiệp
* **Giao diện mượt mà (Lightweight Charts v5.2):** Tốc độ tua nến, phóng to/thu nhỏ, di chuyển biểu đồ cực kỳ nhanh và mượt.
* **Indicator Manager chuyên nghiệp:** Các đường tham chiếu chuẩn (RSI 30/50/70, CCI -100/0/100, MACD ranh giới 0) hiển thị rõ ràng. Việc cấp phát các phân vùng (Pane) riêng biệt cho RSI, MACD, CCI giúp biểu đồ không bị rối.
* **Bộ công cụ vẽ TA chuẩn:** Hỗ trợ đầy đủ 7 công cụ cơ bản (Horizontal, Trendline, Ray, Rectangle, Fibonacci Retracement có %, Text). Khả năng **Nam châm (Magnet Snapping)** tự hít vào đỉnh/đáy nến và tính năng **Undo/Redo (Ctrl+Z/Ctrl+Y)** hoạt động rất mượt.

### 3. Bảo mật Local-First & Nhật ký Thực hành (Practice Journal)
* **Quyền riêng tư tuyệt đối:** Dữ liệu lưu hoàn toàn dưới máy local (SQLite DB), không telemetry, không gửi dữ liệu giao dịch ra ngoài.
* **Ghi nhật ký gắn liền với nến Replay:** Bảng Practice Journal và Checklist kỷ luật trước khi vào lệnh cho phép trader ghi lại lý do mua/bán, tâm lý giao dịch gắn chặt với đúng mốc nến Replay đó.

---

## III. NHỮNG GÌ SUMI V3 CHƯA ĐÁP ỨNG ĐƯỢC (GAPS & LIMITATIONS)

Dưới góc nhìn khắt khe của một Trader thực chiến hàng ngày, Sumi V3 vẫn còn những khoảng trống (Gaps) so với kỳ vọng sử dụng thực tế:

### 1. Giới hạn về Khung thời gian (Daily Only vs. Intraday)
* **Hiện tại:** Sumi V3 mới tối ưu tốt cho khung nến **Daily (Ngày)**.
* **Nhu cầu thực tế:** Nhiều trader đánh ngắn hạn hoặc phái sinh (VN30F1M) rất cần tua nến ở các khung thời gian nhỏ hơn như **1 Giờ (1H), 15 Phút (15m), 5 Phút (5m)**. Việc thiếu khung Intraday khiến các trader phái sinh/day-trader chưa sử dụng Sumi làm công cụ rèn luyện chính được.

### 2. Thiếu cơ chế Tự động Đồng bộ Dữ liệu Thị trường (Real-time / 1-Click Sync Data)
* **Hiện tại:** Dữ liệu nến phải nhập thủ công qua file CSV/CafeF hoặc dùng dữ liệu demo có sẵn.
* **Nhu cầu thực tế:** Trader muốn có một nút bấm **"Cập nhật dữ liệu hôm nay"** để tự động cào/đồng bộ dữ liệu giá mới nhất từ các nguồn SSI, VNDirect, Vietstock mà không cần phải tự đi tìm file CSV để import.

### 3. Thiếu các Công cụ Vẽ & Chỉ báo Chuyên sâu (Advanced TA Tools)
So với TradingView hay FireAnt, Sumi V3 hiện còn thiếu một số công cụ mà các Trader chuyên nghiệp rất hay dùng:
* **Công cụ đo Tỷ lệ Rủi ro / Lợi nhuận (Risk/Reward Position Box - Long/Short Tool):** Hộp đo R:R giúp trader kéo thả để tự động tính ngay vùng Cắt lỗ (Stop Loss) / Chốt lời (Take Profit) và tỷ lệ R:R (Vd: 1:2.5). Currently trader phải tự tính nhẩm.
* **Công cụ vẽ hình học nâng cao:** Kênh song song (Parallel Channel), Tam giác/Hình nêm, Mô hình Vai-Đầu-Vai (Head and Shoulders), Đường Pitchfork.
* **Chỉ báo nâng cao:** Volume Profile (VPVR - Khối lượng theo vùng giá), VWAP (Giá trung bình trọng số khối lượng), SuperTrend.

### 4. Thiếu Viết Chiến lược Tùy biến (Custom PineScript / Python Strategy Editor)
* **Hiện tại:** Chiến lược Backtest và Scanner hiện tại dùng các luật khai báo sẵn (MA Crossover, RSI Oversold).
* **Nhu cầu thực tế:** Các trader dạng Algo-trader muốn có một trình soạn thảo code (Code Editor) để tự viết chỉ báo hoặc chiến lược riêng bằng Python hoặc một ngôn ngữ script đơn giản.

### 5. Chưa hỗ trợ Đa biểu đồ (Multi-chart Layout)
* **Hiện tại:** Màn hình Replay chỉ hiển thị 1 biểu đồ duy nhất của 1 mã.
* **Nhu cầu thực tế:** Trader chuyên nghiệp thường có thói quen xem **Đa khung thời gian (Multi-timeframe)** (Vd: Mở song song chart Tuần ở bên trái và chart Ngày ở bên phải), hoặc xem song song nến VNINDEX và nến cổ phiếu FPT để so sánh sức mạnh tương đối (Relative Strength).

---

## IV. BẢNG SO SÁNH: NHU CẦU THỰC TẾ VS MỨC ĐỘ ĐÁP ỨNG CỦA SUMI V3

| Tiêu chí / Chức năng | Nhu cầu thực tế của Trader | Mức độ đáp ứng của Sumi V3 | Đánh giá |
| :--- | :--- | :---: | :--- |
| **Bar Replay (Tua nến)** | Tua nến không lộ tương lai, tua mượt | **100%** | **RẤT TỐT** - Hoàn hảo, mượt mà, đúng tinh thần tập luyện. |
| **Quy tắc thị trường VN** | Hỗ trợ T+2, kiểm soát sức mua | **100%** | **XUẤT SẮC** - Điểm cộng lớn nhất so với phần mềm ngoại. |
| **Chỉ báo kỹ thuật (Indicators)** | EMA, SMA, RSI, MACD, CCI, BB, ATR | **90%** | **TỐT** - Có Indicator Manager chuyên nghiệp, thiếu Volume Profile/VWAP. |
| **Công cụ vẽ (Drawing Tools)** | Horizontal, Trendline, Fib, RR Box, Channels | **75%** | **KHÁ** - Đã có 7 công cụ cơ bản + Magnet + Undo/Redo; thiếu hộp đo R:R. |
| **Nhật ký giao dịch (Journal)** | Ghi chép lý do, tâm lý, checklist | **95%** | **RẤT TỐT** - Tích hợp Practice Journal ngay trên giao diện Replay. |
| **Khung thời gian (Timeframes)** | Daily, Weekly, 1H, 15m, 5m | **50%** | **TRUNG BÌNH** - Mới tối ưu tốt cho Daily; thiếu nến Intraday. |
| **Cập nhật dữ liệu (Data Sync)** | Tự động 1-click cập nhật giá mới | **40%** | **HẠN CHẾ** - Vẫn phải nhập file CSV/CafeF thủ công. |
| **Đa biểu đồ (Multi-chart)** | Xem 2-4 chart song song | **0%** | **CHƯA CÓ** - Mới hiển thị 1 chart đơn. |
| **Viết Script riêng (Custom Strategy)** | Tự viết code chiến lược tùy biến | **30%** | **HẠN CHẾ** - Mới có các Rule khai báo sẵn. |

---

## V. ĐỀ XUẤT LỘ TRÌNH NÂNG CẤP SẢN PHẨM GIÁ TRỊ TĂNG THÊM (POST-V3 ROADMAP)

Để đưa Sumi từ một **Workstation tập luyện V3 RC** trở thành một **Sản phẩm hoàn hảo được cộng đồng Trader Việt Nam săn đón**, đề xuất ưu tiên nâng cấp theo thứ tự sau:

1. **Ưu tiên 1 (Cần thiết nhất cho Trader):**
   - Bổ sung công cụ vẽ **Long/Short Position Box (Hộp đo Tỷ lệ Risk/Reward)** để trader kéo thả đo ngay mức Cắt lỗ / Chốt lời và tỷ lệ R:R.
   - Bổ sung công cụ **Parallel Channel (Kênh song song)**.
2. **Ưu tiên 2 (Mở rộng đối tượng người dùng):**
   - Hỗ trợ dữ liệu nến Intraday (1H, 15m, 5m) để phục vụ Trader phái sinh (VN30F1M) và Day-trader.
   - Bổ sung tính năng 1-Click Auto Fetch dữ liệu nến mới nhất từ nguồn công khai.
3. **Ưu tiên 3 (Nâng cao cho Professional/Algo Trader):**
   - Hỗ trợ xem 2 biểu đồ song song (Dual-chart layout: VNINDEX vs Symbol hoặc Weekly vs Daily).
   - Chỉ báo Volume Profile (VPVR) và VWAP.
   - Cho phép viết chiến lược tùy biến bằng Python DSL đơn giản.

---
**KẾT LUẬN TỔNG THỂ:**
Sumi V3 RC hiện tại đã giải quyết cực kỳ tốt **80% nhu cầu cốt lõi nhất** của một trader muốn tập luyện tua nến thủ công (Replay), rèn luyện kỷ luật và quản trị rủi ro theo quy tắc T+2 Việt Nam. Những khoảng trống 20% còn lại là các tính năng nâng cao (Intraday, RR Box, Auto Data Sync) hoàn toàn có thể tiếp tục phát triển ở các phiên bản Post-V3 tương lai.
