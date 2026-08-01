# Kịch bản Kiểm thử Thực tế (Manual UAT Test Suite) & System Prompt Kiểm thử Dự án Sumi V3

> **Ngày khởi tạo:** 25/07/2026
>
> **Mục đích:** Hướng dẫn chi tiết từng Test Case kiểm thử thủ công như một người dùng thật (Trader) trên giao diện trình duyệt, và cung cấp System Prompt chuẩn hóa để thực hiện trên session mới.
> **Vị trí tài liệu:** `docs/tester/MANUAL_UAT_TEST_PLAN_AND_PROMPT.md`

---

## PHẦN I: KỊCH BẢN KIỂM THỬ THỰC TẾ CHI TIẾT (MANUAL UAT TEST SCENARIOS)

Kịch bản kiểm thử được chia làm **6 Modul chính** với đầy đủ các bước thực hiện, dữ liệu đầu vào (Input) và kết quả kỳ vọng (Expected Output) rõ ràng:

---

### MODULE 1: TOÀN VẸN REPLAY VÀ KHÓA NẾN TƯƠNG LAI (REPLAY INTEGRITY & NO-FUTURE-LEAK)

#### UC-R01: Kiểm tra quy tắc No-Future-Leak (Không lộ nến tương lai)
- **Mục tiêu:** Đảm bảo nến, chỉ báo, nhãn giá, ngày tháng tương lai hoàn toàn không xuất hiện trên biểu đồ hoặc payload API.
- **Các bước thực hiện:**
  1. Mở trang Replay tại URL `http://localhost:5173`.
  2. Tạo một phiên Replay mới với mã `FPT`, ngày bắt đầu `2024-01-01`, số dư ban đầu `100,000,000 VND`.
  3. Khi biểu đồ tải xong, kiểm tra ngày tháng trên thanh Header Replay và nến cuối cùng trên biểu đồ.
  4. Mở Developer Tools Trình duyệt (F12) -> tab `Network` -> lọc request `candles`.
- **Kết quả kỳ vọng:**
  - Nến hiển thị dừng đúng tại ngày bắt đầu của phiên.
  - Phía bên phải của nến cuối cùng là khoảng trống hoàn toàn, không có nến mờ hay bóng nến tương lai.
  - Request API `/api/replay/sessions/{id}/candles` chỉ trả về danh sách nến từ quá khứ đến `current_index`.

#### UC-R02: Tua nến và đồng bộ trạng thái (Replay Navigation & Sync)
- **Mục tiêu:** Kiểm tra các nút bấm tua nến, phím tắt và việc đồng bộ thông tin nến hiện tại.
- **Các bước thực hiện:**
  1. Nhấn nút **Next (Thấu nến / phím Mũi tên phải)**: Tua 1 nến.
  2. Nhấn nút **+5 (Phím Shift + Mũi tên phải)**: Tua tiến 5 nến.
  3. Nhấn nút **Prev (Lùi nến / phím Mũi tên trái)**: Lùi lại 1 nến.
  4. Nhấn phím **Space (Khoảng trắng)** để Bật Autoplay (Tự động phát), sau đó nhấn Space một lần nữa để Tạm dừng.
  5. Thay đổi tốc độ Autoplay từ `1x` -> `2x` -> `5x`.
- **Kết quả kỳ vọng:**
  - Biểu đồ tua mượt mà. Mỗi khi nến mới xuất hiện, thông tin OHLCV trên Header cập nhật lập tức.
  - Các chỉ báo đang bật (SMA, RSI...) và vị thế giao dịch được tính toán lại ngay lập tức mà không bị đè nến hay giật lag.
  - Phím Space tạm dừng/phát mượt mà, không bị xung đột khi đang gõ chữ trong thẻ input khác.

#### UC-R03: Khôi phục phiên Replay (Reload & Resume Session)
- **Mục tiêu:** Đảm bảo khi bấm F5 reload trang, toàn bộ trạng thái làm việc được khôi phục nguyên vẹn.
- **Các bước thực hiện:**
  1. Tua phiên Replay đến nến thứ 35.
  2. Bấm phím **F5 (Reload)** trên trình duyệt.
- **Kết quả kỳ vọng:**
  - Trang web tải lại thành công mà không bị màn hình trắng hay lỗi Console.
  - Phiên Replay giữ nguyên nến thứ 35, danh sách chỉ báo đang bật và các hình vẽ trên biểu đồ được khôi phục 100%.

---

### MODULE 2: QUẢN LÝ CHỈ BÁO KỸ THUẬT (PROFESSIONAL INDICATOR MANAGER)

#### UC-I01: Thêm chỉ báo từ Registry và tùy chỉnh tham số
- **Mục tiêu:** Kiểm tra luồng tìm kiếm, nhập tham số và thêm chỉ báo vào biểu đồ.
- **Các bước thực hiện:**
  1. Nhấn nút **Indicators (+ Chỉ báo)** trên thanh công cụ biểu đồ.
  2. Tìm kiếm chỉ báo `EMA`. Chọn tham số `Length = 50`, màu sắc `Xanh dương`. Nhấn **Add (Thêm)**.
  3. Thêm tiếp một chỉ báo `EMA` thứ 2 với `Length = 200`, màu sắc `Đỏ`.
  4. Mở menu chỉ báo, chọn nhóm **Oscillators**, thêm chỉ báo `RSI` (Length 14), `MACD` (12, 26, 9) và `CCI` (20).
- **Kết quả kỳ vọng:**
  - Hai đường EMA 50 và EMA 200 xuất hiện trên biểu đồ giá chính với màu sắc và chu kỳ khác biệt rõ ràng.
  - RSI, MACD, CCI được cấp phát vào các phân vùng (Pane) riêng biệt phía dưới biểu đồ giá.
  - Danh sách chỉ báo active hiển thị đầy đủ thông tin tên chỉ báo, tham số, màu sắc và nút điều khiển.

#### UC-I02: Thang đo và đường tham chiếu chuẩn (Indicator Scaling & References)
- **Mục tiêu:** Đảm bảo các chỉ báo Oscillator hiển thị đúng tiêu chuẩn phân tích kỹ thuật.
- **Các bước thực hiện:**
  1. Quan sát pane **RSI**: Kiểm tra thang đo và các đường tham chiếu.
  2. Quan sát pane **MACD**: Kiểm tra đường MACD, đường Signal, cột Histogram và đường Zero.
  3. Quan sát pane **CCI**: Kiểm tra các đường tham chiếu -100, 0, 100.
- **Kết quả kỳ vọng:**
  - RSI hiển thị thang đo cố định 0-100 với các đường tham chiếu đứt đoạn rõ ràng tại **30, 50, 70**.
  - MACD hiển thị rõ đường MACD line, đường Signal, các cột Histogram đổi màu (xanh/đỏ) quanh đường ranh giới **0**. Các giá trị chỉ báo ở góc trái pane không bị đè chữ lên nhau.
  - CCI hiển thị các đường tham chiếu đứt đoạn tại **-100, 0, +100**.

#### UC-I03: Thao tác Ẩn/Hiện, Sửa cài đặt và Xóa từng chỉ báo đơn lẻ
- **Mục tiêu:** Kiểm tra các nút tương tác trên từng chỉ báo.
- **Các bước thực hiện:**
  1. Trên danh sách Active Indicators, nhấn nút **Con mắt (Toggle Visibility)** tại chỉ báo `EMA 50`.
  2. Nhấn nút **Bánh răng (Settings)** tại chỉ báo `RSI 14`, đổi chu kỳ thành `Length = 21`. Nhấn Save.
  3. Nhấn nút **X (Remove)** để xóa riêng chỉ báo `CCI`.
- **Kết quả kỳ vọng:**
  - Đường EMA 50 ẩn đi khi nhấn con mắt và hiện lại khi bấm lại.
  - RSI cập nhật tức thì theo chu kỳ mới (21) mà không làm ảnh hưởng đến các chỉ báo khác.
  - Chỉ báo CCI bị xóa hoàn toàn, pane CCI tự động thu gọn và biến mất, các pane khác tự động điều chỉnh lại chiều cao hợp lý.

---

### MODULE 3: HỆ THỐNG CÔNG CỤ VẼ KỸ THUẬT (PROFESSIONAL DRAWING SYSTEM)

#### UC-D01: Vẽ các công cụ TA tiêu chuẩn và Snap Nam Châm (Magnet Snapping)
- **Mục tiêu:** Kiểm tra khả năng tương tác vẽ 7 loại công cụ chuẩn.
- **Các bước thực hiện:**
  1. Chọn công cụ **Horizontal Line (Đường nằm ngang)** -> Click vào đỉnh một nến giá.
  2. Chọn công cụ **Trendline (Đường xu hướng)** -> Click điểm thứ 1 tại đáy nến A, click điểm thứ 2 tại đáy nến B.
  3. Bật biểu tượng **Nam châm (Magnet Snapping)** -> Chọn công cụ **Rectangle (Hình chữ nhật)** -> Vẽ vùng kháng cự/hỗ trợ qua các nến.
  4. Chọn công cụ **Fibonacci Retracement** -> Click từ điểm Đáy lên điểm Đỉnh của một sóng tăng.
  5. Chọn công cụ **Text/Note** -> Click vào biểu đồ và gõ nội dung `"Vùng mua gom"`.
- **Kết quả kỳ vọng:**
  - Khi bật Nam châm, con trỏ tự động "hít" chính xác vào giá High/Low/Open/Close của nến gần nhất.
  - Fibonacci Retracement hiển thị đầy đủ các đường tỷ lệ phần trăm chuẩn: **0%, 23.6%, 38.2%, 50%, 61.8%, 78.6%, 100%** kèm nhãn giá trị mượt mà.
  - Thao tác vẽ mượt mà, không xuất hiện lỗi đè điểm hoặc giật lag.

#### UC-D02: Chọn, Chuyển vị trí, Chỉnh sửa điểm neo và Xóa công cụ vẽ
- **Mục tiêu:** Kiểm tra vòng đời của hình vẽ sau khi đã đặt lên biểu đồ (Selection & Editing Lifecycle).
- **Các bước thực hiện:**
  1. Chuyển sang công cụ **Cursor / Select (Con trỏ)**.
  2. Click chọn vào đường Trendline vừa vẽ.
  3. Kéo di chuyển toàn bộ đường Trendline sang vị trí khác. Kéo một trong 2 điểm neo (Anchor point) để thay đổi góc dốc.
  4. Nhấn phím **Delete / Backspace** trên bàn phím (hoặc nút thùng rác trên thanh công cụ) để xóa đường Trendline đang chọn.
- **Kết quả kỳ vọng:**
  - Khi click chọn, hình vẽ xuất hiện các điểm neo dạng viền xanh (Selection handles) rõ ràng.
  - Kéo di chuyển và điều chỉnh điểm neo mượt mà.
  - Nhấn phím Delete xóa chính xác hình vẽ đang được chọn mà không làm ảnh hưởng đến các hình vẽ khác.

#### UC-D03: Hoàn tác Undo/Redo và Lưu trữ sau Reload
- **Mục tiêu:** Kiểm tra phím tắt Undo/Redo và tính bền vững của dữ liệu hình vẽ.
- **Các bước thực hiện:**
  1. Nhấn phím **Ctrl + Z (Undo)**: Phôi phục lại đường Trendline vừa xóa.
  2. Nhấn phím **Ctrl + Y (Redo)**: Thực hiện lại thao tác xóa.
  3. Tua phiên Replay tiến lên 10 nến.
  4. Nhấn phím **F5 (Reload)** trình duyệt.
- **Kết quả kỳ vọng:**
  - Phím Ctrl+Z và Ctrl+Y hoạt động hoàn hảo cho mọi thao tác vẽ/chỉnh sửa/xóa.
  - Sau khi F5 reload, các hình vẽ (Horizontal, Fibonacci, Rectangle, Text) vẫn nằm chính xác tại vị trí tọa độ giá/thời gian ban đầu.

---

### MODULE 4: QUY TRÌNH THỰC HÀNH GIAO DỊCH VÀ QUẢN LÝ LỆNH (TRADING PRACTICE WORKFLOW)

#### UC-T01: Đặt lệnh Mua/Bán Thị trường (Market Order - MP) và Quản lý Vị thế
- **Mục tiêu:** Kiểm tra luồng đặt lệnh khớp ngay và hiển thị PnL vị thế thời gian thực.
- **Các bước thực hiện:**
  1. Tại bảng **Trade Controls**, chọn khối lượng `1,000` cổ phiếu.
  2. Nhấn nút **Buy (Mua MP)**.
  3. Tua tiến 1 nến để xem vị thế được mở.
  4. Quan sát bảng **Position Panel (Vị thế hiện tại)**: Xem số lượng, giá vốn (Avg Price), PnL tạm tính (Unrealized PnL).
- **Kết quả kỳ vọng:**
  - Lệnh mua được khớp ngay tại giá Close của nến hiện tại.
  - Trên biểu đồ xuất hiện **Trade Marker (Dấu mốc giao dịch)** màu xanh đánh dấu điểm Mua.
  - Bảng Vị thế hiển thị chính xác số lượng `1,000` CP, cập nhật lãi/lỗ nhảy theo từng nến Replay.

#### UC-T02: Đặt lệnh Giới hạn (Limit Order - LO) và Quy tắc Khớp lệnh
- **Mục tiêu:** Kiểm tra lệnh chờ khớp (Pending Order) khi giá nến chạm mức giá đặt.
- **Các bước thực hiện:**
  1. Chọn loại lệnh **Limit (LO)**, nhập giá thấp hơn giá hiện tại 3%. Nhấn **Buy Limit**.
  2. Kiểm tra bảng **Pending Orders (Lệnh chờ khớp)**.
  3. Tua nến tiến dần từng nến một cho đến khi có nến có giá Low thấp hơn hoặc bằng giá Limit vừa đặt.
- **Kết quả kỳ vọng:**
  - Lệnh hiển thị trong danh sách Pending Orders với trạng thái `PENDING`.
  - Khi nến Replay tua đến nến chạm giá đặt, lệnh tự động chuyển sang trạng thái `FILLED`, vị thế được cộng thêm khối lượng tương ứng và hiển thị thông báo mượt mà.

#### UC-T03: Kiểm tra Quy tắc Thanh toán T+2 và Chốt lời/Cắt lỗ (Sell Position)
- **Mục tiêu:** Kiểm tra tính chính xác của quy tắc T+2 thị trường chứng khoán Việt Nam.
- **Các bước thực hiện:**
  1. Ngay sau khi lệnh Mua T0 vừa khớp, thử nhấn nút **Sell (Bán)** ngay lập tức.
  2. Tua tiến thêm 1 nến (Ngày T+1), thử nhấn nút **Sell (Bán)**.
  3. Tua tiến thêm 1 nến nữa (Ngày T+2), nhấn nút **Sell (Bán MP)** để chốt toàn bộ vị thế.
- **Kết quả kỳ vọng:**
  - Ở ngày T0 và T+1, hệ thống từ chối lệnh bán và hiển thị cảnh báo quy tắc cổ phiếu chưa về (T+2 settlement constraint).
  - Ở ngày T+2, cổ phiếu trở thành khả dụng, lệnh Bán khớp thành công. Vị thế đóng hoàn toàn, PnL đã thực hiện (Realized PnL) được cộng vào tài khoản cash.

#### UC-T04: Nhật ký Thực hành (Practice Journal) và Checklist Kỷ luật
- **Mục tiêu:** Kiểm tra tính năng ghi chép lý do giao dịch và đánh giá tâm lý.
- **Các bước thực hiện:**
  1. Mở bảng **Practice Journal (Nhật ký thực hành)** bên thanh công cụ phải.
  2. Tích chọn các mục trong **Checklist Kỷ luật** (Vd: *"Đã xác nhận xu hướng"*, *"RSI ở vùng quá bán"*, *"Tuân thủ quản trị rủi ro 2%"*).
  3. Nhập ghi chú nhật ký: `"Mua tích lũy tại vùng hỗ trợ Fibonacci 0.618"`. Nhấn Lưu.
- **Kết quả kỳ vọng:**
  - Giao diện Practice Journal mở mượt mà, không làm che mất biểu đồ chính.
  - Nội dung nhật ký và trạng thái checklist được lưu trữ và gắn liền với mốc thời gian của phiên Replay.

---

### MODULE 5: KIỂM TRA TÍNH NĂNG VÀ CÁC TRANG BỔ TRỢ (BACKTEST, SCANNER, STRATEGY LAB)

#### UC-S01: Chạy Quét Tín hiệu Lịch sử (Historical Signal Scanner)
- **Mục tiêu:** Kiểm tra bộ quét tín hiệu và mở phiên Replay từ kết quả quét.
- **Các bước thực hiện:**
  1. Chuyển sang trang **Scanner** từ Sidebar menu.
  2. Chọn bộ lọc tín hiệu (Vd: `MA Crossover` hoặc `RSI Oversold`), chọn danh sách VN30. Nhấn **Run Scan**.
  3. Sau khi kết quả quét xuất hiện, click vào nút **Replay Signal** tại một mã cổ phiếu (Vd: `SSI`).
- **Kết quả kỳ vọng:**
  - Kết quả quét hiển thị danh sách các mã thỏa mãn điều kiện cùng ngày phát sinh tín hiệu.
  - Nhấn "Replay Signal" tự động chuyển hướng sang trang Replay và mở đúng phiên tại thời điểm phát sinh tín hiệu đó.

#### UC-S02: Thực thi Backtest tự động và Dọn dẹp phiên rác (Backtest Cleanup)
- **Mục tiêu:** Kiểm tra tính năng chạy Backtest khai báo và dọn dẹp CSDL.
- **Các bước thực hiện:**
  1. Chuyển sang trang **Backtest**.
  2. Chọn chiến lược mẫu `MACD + RSI Momentum`, chọn mã `FPT`, khoảng thời gian từ `2023-01-01` đến `2024-01-01`. Nhấn **Run Backtest**.
  3. Kiểm tra kết quả: Biểu đồ Equity Curve, các chỉ số PnL, Win rate, Max Drawdown, danh sách lệnh giao dịch.
  4. Mở Developer Tools -> gọi API dọn dẹp `POST /api/backtest/cleanup-sessions`.
- **Kết quả kỳ vọng:**
  - Backtest thực thi nhanh chóng, trả về đầy đủ đường cong tài sản (Equity curve) và các số liệu thống kê.
  - Lệnh cleanup xóa sạch các phiên backtest tạm thời mà **tuyệt đối không ảnh hưởng** đến các phiên Manual Replay của người dùng.

---

### MODULE 6: ĐÁNH GIÁ TRẢI NGHIỆM TỔNG THỂ (UX, USER INTERFACE & PERFORMANCE)

#### UC-X01: Tỷ lệ màn hình và Phim tắt toàn cục (Viewport & Shortcuts)
- **Mục tiêu:** Kiểm tra giao diện trên màn hình Desktop chuẩn 1440x1000 và laptop 1280x800.
- **Các bước thực hiện:**
  1. Thử thay đổi kích thước cửa sổ trình duyệt về `1440x1000` và `1280x800`.
  2. Kiểm tra xem diện tích biểu đồ chính có chiếm trên 60% không gian làm việc hay không.
  3. Mở cửa sổ Modal (Cài đặt chỉ báo hoặc Nhật ký), thử gõ chữ và bấm nút `Space` hoặc `Delete`.
- **Kết quả kỳ vọng:**
  - Giao diện đáp ứng mượt mà, không bị vỡ khung hay ẩn mất thanh công cụ.
  - Khi đang mở Modal/Thẻ input, bấm phím Space không làm tua nến Replay, phím Delete không xóa hình vẽ trên biểu đồ (Phân tách shortcut chính xác).

---

## PHẦN II: PROMPT ĐẦY ĐỦ ĐỂ CHẠY TRÊN SESSION MỚI (TESTER PROMPT)

> **Hướng dẫn sử dụng:** Copy toàn bộ đoạn prompt bên dưới và dán vào một **Session mới** để yêu cầu Agent/Tester thực hiện kiểm thử tự động hoặc hướng dẫn kiểm thử chi tiết.

```markdown
Bạn là một Chuyên viên Kiểm thử Sản phẩm (Senior QA / Product Tester Specialist) am hiểu sâu sắc về Thị trường Chứng khoán Việt Nam và các công cụ Phân tích Kỹ thuật (TradingView, Amibroker).

Nhiệm vụ của bạn là thực hiện việc KIỂM THỬ THỰC TẾ (User Acceptance Testing - UAT) toàn bộ dự án **Sumi (V3 Release Candidate)** trên môi trường thật với vai trò của một Trader chuyên nghiệp.

### NGUYÊN TẮC KIỂM THỬ BẮT BUỘC (NON-NEGOTIABLE RULES):
1. **Không kiểm thử qua loa:** Phải thực hiện tuần tự, đầy đủ tất cả các Test Case dưới đây. Không bỏ sót bất kỳ Use Case nào.
2. **Báo cáo bằng chứng rõ ràng:** Với mỗi Test Case, phải ghi nhận trạng thái [PASS] hoặc [FAIL], kèm theo chi tiết kết quả quan sát được (Giao diện hiển thị, log Console, phản hồi API).
3. **Tuân thủ System Invariants:**
   - Không được để rò rỉ nến tương lai (No-future-leak).
   - Không sửa đổi CSDL chính `backend/sumi.db`.
   - Không được xuất hiện bất kỳ lỗi Javascript Console hay lỗi mạng 500 nào trong quá trình kiểm thử.

---

### DANH MỤC CÁC MODULE VÀ TEST CASE CẦN THỰC HIỆN:

#### MODULE 1: TOÀN VẸN REPLAY VÀ KHÓA NẾN TƯƠNG LAI
- [ ] **TC-R01:** Kiểm tra quy tắc No-Future-Leak trên phiên Replay mã FPT (Không xuất hiện nến/giá tương lai).
- [ ] **TC-R02:** Thao tác tua nến: Tua 1 nến (Next), tua 5 nến (+5), lùi nến (Prev), Autoplay (Phím Space), đổi tốc độ (1x, 2x, 5x).
- [ ] **TC-R03:** Thao tác Reload (F5): Khôi phục chính xác nến hiện tại, vị thế và các công cụ vẽ sau khi F5.

#### MODULE 2: QUẢN LÝ CHỈ BÁO KỸ THUẬT (INDICATOR MANAGER)
- [ ] **TC-I01:** Thêm chỉ báo EMA 50 (Xanh) và EMA 200 (Đỏ) trên biểu đồ giá chính.
- [ ] **TC-I02:** Thêm các chỉ báo Oscillator: RSI (14), MACD (12, 26, 9), CCI (20) vào các Pane riêng biệt.
- [ ] **TC-I03:** Kiểm tra đường tham chiếu: RSI có các đường 30/50/70; CCI có các đường -100/0/100; MACD có đường Zero và nhãn không đè lên nhau.
- [ ] **TC-I04:** Thao tác tương tác chỉ báo: Ẩn/Hiện (Con mắt), Sửa tham số (Bánh răng), và Xóa riêng lẻ từng chỉ báo (Nút X).

#### MODULE 3: HỆ THỐNG CÔNG CỤ VẼ KỸ THUẬT (DRAWING SYSTEM)
- [ ] **TC-D01:** Thực hiện vẽ 7 công cụ TA: Cursor, Horizontal Line, Trendline, Ray, Rectangle, Fibonacci Retracement (có nhãn %), và Text/Note.
- [ ] **TC-D02:** Bật chế độ Nam châm (Magnet Snapping) -> Kiểm tra con trỏ hít chính xác vào điểm OHLC của nến.
- [ ] **TC-D03:** Thao tác chỉnh sửa: Click chọn hình vẽ (Selection bounds), kéo di chuyển, chỉnh sửa điểm neo (Anchor handles), xóa bằng phím Delete.
- [ ] **TC-D04:** Kiểm tra phím tắt Hoàn tác Undo (Ctrl+Z) và Redo (Ctrl+Y). F5 reload kiểm tra hình vẽ giữ nguyên vị trí.

#### MODULE 4: QUY TRÌNH THỰC HÀNH GIAO DỊCH (TRADING PRACTICE)
- [ ] **TC-T01:** Đặt lệnh Mua Thị trường (Market Order MP) -> Kiểm tra khớp lệnh tại giá Close và hiển thị Dấu mốc giao dịch (Trade Marker).
- [ ] **TC-T02:** Kiểm tra bảng Position Panel: Cập nhật số lượng CP, giá vốn và PnL tạm tính (Unrealized PnL) nhảy theo từng nến.
- [ ] **TC-T03:** Đặt lệnh Giới hạn (Limit Order LO) -> Kiểm tra trạng thái Pending và tự động khớp khi nến tua chạm giá Limit.
- [ ] **TC-T04:** Kiểm tra Quy tắc T+2: Từ chối bán ở ngày T0 và T+1; Cho phép bán chốt lời/cắt lỗ ở ngày T+2.
- [ ] **TC-T05:** Điền Practice Journal: Tích chọn Checklist kỷ luật, nhập ghi chú nhật ký và lưu lại theo mốc thời gian Replay.

#### MODULE 5: CÁC TRANG BỔ TRỢ (SCANNER, BACKTEST, STRATEGY LAB)
- [ ] **TC-S01:** Chạy Quét tín hiệu lịch sử (Scanner) -> Click "Replay Signal" để mở trực tiếp phiên Replay từ kết quả quét.
- [ ] **TC-S02:** Thực thi Backtest tự động chiến lược MACD+RSI -> Kiểm tra đường cong tài sản (Equity Curve) và gọi API dọn dẹp phiên rác (`/api/backtest/cleanup-sessions`).

#### MODULE 6: TRẢI NGHIỆM VÀ HIỆU NĂNG (UX & PERFORMANCE)
- [ ] **TC-X01:** Kiểm tra tỷ lệ không gian biểu đồ trên độ phân giải 1440x1000 và 1280x800.
- [ ] **TC-X02:** Kiểm tra cách ly phím tắt: Gõ chữ trong Journal/Modal không làm tua nến hay xóa hình vẽ.

---

Hãy tiến hành chạy lần lượt các Test Case trên, ghi chép kết quả chi tiết và xuất cho tôi **BÁO CÁO NGHIỆM THU UAT THỰC TẾ (REAL UAT ACCEPTANCE REPORT)** sau khi hoàn thành!
```
