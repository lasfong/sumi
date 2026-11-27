# PROMPT KIỂM TOÁN TOÀN DIỆN SUMI HIỆN TẠI (DATA, TÍNH NĂNG & SO SÁNH KHOẢNG CÁCH VỚI TRADINGVIEW)

Sao chép toàn bộ nội dung trong khối dưới đây và dán vào một **Session mới** trên workspace `/Users/mizuhara/workspace/sumi` để thực hiện kiểm toán độc lập, chính xác tới từng dòng code và bảng dữ liệu:

---

````markdown
Bạn đóng vai **Principal Financial Platform Architect, Market Data Engineer & TradingView Product Parity Auditor**.

Nhiệm vụ của bạn trong phiên này là **kiểm toán (audit) toàn diện, trung thực và chi tiết đến tận mã nguồn + cơ sở dữ liệu thực tế** của ứng dụng **Sumi Supercharts (`v2.2.0`)** tại `/Users/mizuhara/workspace/sumi` để trả lời thấu đáo 3 câu hỏi lớn của tôi:
1. **Về Dữ liệu (`Data & Sources`):** Sumi hiện tại thực sự lấy được những dữ liệu nào, từ những nguồn nào, độ phủ/độ sâu đến đâu, còn tiềm năng dữ liệu nào trong thư viện đang cài mà chưa khai thác, và những loại dữ liệu thị trường nào Sumi hoàn toàn chưa có?
2. **Về Tính năng hiện có (`Current Feature Inventory`):** Sumi hiện có chính xác những tính năng nào (phân biệt rõ cái gì đã chạy hoàn chỉnh trên UI, cái gì đã viết trong lõi Rust `sumi-core` / `sumi-data` nhưng chưa nối lên UI, và cái gì đang là mô phỏng/xấp xỉ)?
3. **So sánh với TradingView (`Sumi vs. TradingView Gap Analysis`):** Đặt Sumi cạnh TradingView (bản Desktop / Plus / Premium), đối chiếu từng phân hệ để chỉ rõ **TradingView có những tính năng nào mà Sumi chưa phát triển được** (cùng những điểm riêng mà Sumi đang làm tốt hơn TradingView cho chứng khoán Việt Nam), từ đó xếp hạng các tính năng đáng phát triển tiếp nhất.

---

## 1. NGUYÊN TẮC KIỂM TOÁN BẮT BUỘC

1. **Chế độ Read-Only tuyệt đối với mã nguồn & CSDL:**
   - **KHÔNG sửa đổi bất kỳ file code nào** trong `crates/` hay `ui/`.
   - Khi truy vấn kiểm tra cơ sở dữ liệu thật tại `~/Library/Application Support/com.sumi.supercharts/sumi_market.db` và `sumi_workspace.db`, bắt buộc mở ở **chế độ chỉ đọc** (`sqlite3 -readonly ...` hoặc mở URI read-only trong Python).
2. **Kiểm chứng từ Code thật & DB thật — KHÔNG chép lại từ tài liệu cũ:**
   - Trong `docs/` có nhiều tài liệu từ thời bản cũ (`v1.x` egui/wgpu và `v2.1.0` trước khi tinh gọn QMV). Bạn **phải đọc trực tiếp mã nguồn hiện hành** trong `crates/{sumi-data,sumi-core,sumi-desktop}/src/` và `ui/src/` để phản ánh đúng trạng thái thực tế của Sumi ngay lúc này.
3. **Đánh giá khách quan, không tô hồng:**
   - Tính năng nào làm một nửa, bị giới hạn, dùng công thức xấp xỉ (`proxy`), hoặc có trong Rust nhưng bị ẩn/chưa nối lên giao diện React phải được chỉ mặt đặt tên rõ ràng kèm đường dẫn `file:line`.
4. Trình bày toàn bộ báo cáo bằng **tiếng Việt** rõ ràng, có bảng biểu đối chiếu trực quan và lưu báo cáo hoàn chỉnh vào `/Users/mizuhara/workspace/sumi/docs/qa/09_SUMI_DATA_FEATURE_AND_TRADINGVIEW_GAP_AUDIT.md`.

---

## 2. PHẠM VI & CÁC HẠNG MỤC BẮT BUỘC PHẢI ĐỌC / ĐO ĐẠC

Hãy chia việc hợp lý (có thể dùng song song các `research` subagents) để kiểm tra trọn vẹn 3 phần sau:

### PHẦN 1 — Kiểm Toán Nguồn Dữ Liệu & Độ Phủ Dữ Liệu Thực Tế (`Data & Upstream Audit`)

Đọc kỹ toàn bộ `crates/sumi-data/` (`Cargo.toml`, `tradingview_feed.rs`, `cafef_importer.rs`, `storage.rs`, `cache_service.rs`, `ram_cache.rs`, `rate_limit.rs`, `provider.rs`, `types.rs`, `src/bin/sumi-data-cli.rs`), `crates/sumi-desktop/src/commands/market.rs`, kiểm tra cả mã nguồn thư viện phụ thuộc `tradingview-rs` trong `~/.cargo/git/checkouts/tradingview-rs-*` (nếu có trên máy), đồng thời chạy truy vấn thống kê `sqlite3 -readonly` trên `~/Library/Application Support/com.sumi.supercharts/sumi_market.db` và kiểm tra thư mục `/Users/mizuhara/workspace/sumi/data/` (nếu có) cũng như `/Users/mizuhara/workspace/yumeko/crates/yumeko-sumi-contract/` (nếu tồn tại):

1. **Nguồn 1 — TradingView Feed (`tradingview-rs`):**
   - Cơ chế kết nối thực tế là gì (WebSocket/HTTP, chế độ Guest không đăng nhập hay có hỗ trợ Auth Token/Cookie)? Giới hạn số nến mỗi lần tải, giới hạn tần suất (`rate_limit.rs`), độ trễ dữ liệu (realtime hay bị delay trên từng sàn)?
   - Lấy được chính xác những **kiểu dữ liệu nào**? (Nến OHLCV lịch sử ở những khung thời gian nào `1m, 5m, 15m, 30m, 1h, 4h, 1D, 1W, 1M`? Luồng nến trực tiếp `subscribe_bars`? Bảng giá `subscribe_quotes` lấy được những trường cụ thể nào? Tìm kiếm `search_symbols` và `resolve_symbol` trả về những trường metadata nào?).
   - Lấy được những **thị trường / lớp tài sản nào**? (`HOSE`, `HNX`, `UPCOM`, Phái sinh `HNX:VN30F1!` <-> `VN301!`, Chỉ số `VNINDEX / HNXINDEX / VN30`, Crypto, Cổ phiếu Mỹ, Forex, Hàng hóa, Vĩ mô như `DXY / US10Y`?).
   - **Khám phá tiềm năng ẩn trong `tradingview-rs`:** Thư viện `tradingview-rs` đang cài trong dự án thực chất còn hỗ trợ những API/dữ liệu nào mà Sumi **chưa gọi tới** (ví dụ: Báo cáo tài chính/Financials, Cổ tức/Chia tách `Splits/Dividends`, TradingView Market Screener, Pine/Study values, Tin tức, Lịch kinh tế, xác thực tài khoản Pro để lấy thêm nến intraday sâu...)?
2. **Nguồn 2 — Dữ liệu Lịch sử CafeF (`cafef_importer.rs`):**
   - Hỗ trợ đọc những định dạng CSV nào (Giá điều chỉnh `Adjusted`, Giá chưa điều chỉnh `Raw`, Chỉ số `INDEX`)?
   - Cơ chế ghép nối (`splice`) giữa lịch sử dài hạn CafeF và dữ liệu mới từ TradingView hoạt động như thế nào? Xử lý `Adjustment::Splits` vs `Adjustment::None` (giá điều chỉnh vs giá gốc) trong DB và trên UI ra sao?
3. **Thống kê thực chứng bên trong `sumi_market.db` hiện tại (Số liệu thật 100%):**
   - Bảng thống kê chi tiết: Tổng dung lượng DB, tổng số mã và tổng số nến phân theo **từng Sàn (`HOSE`, `HNX`, `UPCOM`, `INDEX`, `CRYPTO`, `US`...)** × **từng Khung giờ (`1D`, `1W`, `4h`, `1h`, `15m`, `5m`, `1m`...)** × **từng chế độ `adjustment`**, kèm `ngày cũ nhất → ngày mới nhất` của mỗi nhóm!
   - Có bao nhiêu mã cổ phiếu VN chỉ có khung `1D` mà chưa có dữ liệu intraday (`1h / 15m / 1m`) trong DB? Khung `1W / 1M` lấy trực tiếp hay tự tổng hợp từ `1D`?
4. **Nguồn 3 — Liên kết với hệ thống khác & Những Dữ liệu Sumi HIỆN CHƯA THỂ LẤY:**
   - Kiểm tra xem giữa Sumi và Yumeko (`yumeko-sumi-contract`) hiện đã có cầu nối dữ liệu thực tế nào chưa hay hai bên đang độc lập hoàn toàn?
   - Liệt kê thẳng thắn các loại dữ liệu quan trọng trên thị trường Việt Nam mà Sumi **chưa có nguồn lấy**: Dữ liệu khớp lệnh từng tick (`Time & Sales`), Mua/Bán chủ động thực tế (`Active Buy/Sell`), Sổ lệnh 3/10 bước giá (`Order Book Level 2`), Khối ngoại / Tự doanh / Thỏa thuận, Lịch sự kiện cổ tức/chia tách/BCTC tự động, Chỉ số tài chính doanh nghiệp (`P/E, P/B, EPS, ROE`), Độ rộng thị trường nội tại...

---

### PHẦN 2 — Kiểm Kê Toàn Bộ Tính Năng Hiện Có Của Sumi (`Codebase & UI Feature Inventory`)

Đọc toàn bộ `crates/sumi-core/src/`, `crates/sumi-desktop/src/` (`lib.rs`, `workspace_db.rs`, `commands/*`) và `ui/src/` (`chart/`, `components/`, `store/`, `utils/`), lập danh mục đầy đủ mọi tính năng chia theo 8 phân hệ và gắn nhãn trạng thái rõ ràng cho từng mục:
- `✅ [Hoàn chỉnh trên UI]` — Đã nối đầy đủ từ lõi Rust lên giao diện người dùng.
- `⚠️ [Có trong Rust / Ẩn hoặc Một phần trên UI]` — Lõi `sumi-core` hoặc `sumi-data` đã viết sẵn nhưng UI chưa mở hoặc mới mở một phần (ví dụ các chỉ báo/tín hiệu đã cắt khỏi UI hoặc tính năng trong `sumi-core::journal`, `sumi-core::alerts`, `sumi-core::indicators`, `sumi-data-cli`...).
- `🧪 [Mô phỏng / Xấp xỉ]` — Hoạt động dựa trên giả định hoặc xấp xỉ từ OHLCV.

Kiểm kê chi tiết qua 8 mảng:
1. **Biểu đồ Supercharts, Đa khung giờ & Quy tắc hiển thị VN** (Kiểu biểu đồ, Thang giá, Múi giờ, Vạch phiên ICT, Dải Trần/Tham chiếu/Sàn, Lưới đa ô `1..4`, Cửa sổ phụ tách rời, Đồng bộ Mã/Khung/Crosshair/Thời gian/Hình vẽ, Cây Đối Tượng, Phím tắt, Xuất PNG).
2. **Chỉ báo Kỹ thuật & Mẫu Chỉ báo** (Danh sách đầy đủ trên UI vs danh sách đầy đủ trong `sumi-core::indicators`, khả năng tùy biến tham số/màu sắc, Templates).
3. **Công cụ Vẽ PTKT** (Đầy đủ 37 công cụ vẽ theo từng nhóm, Magnet, Thanh công cụ nổi, Khóa/Ẩn/Nhân bản, Lọc hiển thị theo khung giờ, Undo/Redo).
4. **Hệ thống Cảnh báo (`Alerts`)** (Các loại cảnh báo hỗ trợ, cách đánh giá realtime vs replay, âm báo).
5. **Watchlist & Thông tin Chi tiết Mã**.
6. **Bar Replay Nhân quả (`Causal Bar Replay`)** (Đơn khung, Đa khung giờ, Blind Mode, Lưu/Khôi phục phiên).
7. **Giao dịch Giấy (`Paper Broker`) & Chart Trading** (Các loại lệnh `LO/MP/ATO/ATC/Stop`, quy tắc VSD `T+2` FIFO, Phái sinh `VN30F1!`, Kéo thả `SL/TP/LO` trên biểu đồ, Nhật ký giao dịch).
8. **Lọc Cổ Phiếu (`Stock Screener`) & Kiểm định Chiến lược (`Strategy Tester`)** (Tổng số tín hiệu, bộ lọc phân nhóm QMV, Chiến lược đơn lẻ, Rule Builder tự thiết kế, Danh mục đa mã `Portfolio`, `Walk-Forward`, `Monte Carlo`).

---

### PHẦN 3 — So Sánh Toàn Diện Sumi vs. TradingView & Khoảng Cách Tính Năng (`TradingView Parity & Gap Matrix`)

Hãy thực hiện một bảng đối chiếu chuyên sâu và khách quan giữa **Sumi Supercharts (`v2.2.0`)** và **TradingView (Desktop / Plus / Premium)** trên **11 nhóm tính năng cốt lõi của một nền tảng phân tích kỹ thuật chuyên nghiệp**, chỉ rõ những gì **TradingView làm được mà Sumi CHƯA phát triển được**:

1. **Các loại Biểu đồ nâng cao & Khung thời gian tùy chỉnh:**
   - Đối chiếu: `Renko`, `Line Break`, `Kagi`, `Point & Figure`, `Range Bars`, `Volume Candles`, `Baseline`, `HLC Area`, **Volume Footprint / Orderflow**, **Time Price Opportunity (`TPO / Market Profile`)**, khung thời gian tùy chỉnh (`2D, 3D, 2h, 45m`, giây `s`), nút chuyển nhanh `ADJ / B-ADJ` (giá điều chỉnh vs giá gốc ngay trên biểu đồ).
2. **So sánh Liên thị trường & Biểu đồ Công thức (`Compare & Spread/Ratio Charts`):**
   - Đối chiếu: Nút **`Compare (+)`** chồng nhiều mã (`VNINDEX`, mã cùng ngành, `DXY`) lên cùng một biểu đồ theo `%`; gõ biểu thức toán học trực tiếp lên ô mã (ví dụ `HOSE:SSI / HOSE:VNINDEX` hoặc `(HOSE:HPG + HOSE:HSG)/2` để vẽ biểu đồ sức mạnh tương đối / chênh lệch giá!).
3. **Hệ sinh thái Chỉ báo, Ngôn ngữ Kịch bản & Hồ sơ Khối lượng:**
   - Đối chiếu: **Trình biên tập kịch bản tùy chỉnh (`Pine Script` / Custom Formula DSL ngay trong app không cần sửa code build lại)**, **Anchored VWAP (`AVWAP` neo vào 1 cây nến đỉnh/đáy bất kỳ bằng chuột)**, **Fixed-Range Volume Profile (`FRVP` kéo chọn 1 vùng tích lũy cụ thể)**, **Session Volume Profile (`SVP` theo từng ngày/tuần)**, **Indicator-on-Indicator** (gắn đường MA lên RSI/OBV, gắn Bollinger lên RSI...), **Auto Chart Patterns / Auto Fib / Seasonality (Thống kê mùa vụ theo tháng)**.
4. **Công cụ Vẽ Chuyên sâu & Dựng Kịch bản Tương lai:**
   - Đối chiếu: **`Ghost Feed` & `Bars Pattern`** (vẽ các cây nến giả định tương lai hoặc sao chép mẫu nến quá khứ để lập kế hoạch sóng), `Path / Polyline` vẽ kịch bản đường đi của giá nhiều nhịp, `Brush / Highlighter`, `Callout / Price Note / Anchored Text`, hệ thống công cụ `Pitchfork / Gann / Fib Time Zones / Fib Circles`, **Thư mục gom nhóm hình vẽ (`Drawing Folders` trong Object Tree)**, **Lưu mẫu cấu hình cho từng công cụ vẽ (`Drawing Templates`)**, phím tắt giữ `Shift + Click` đo nhanh khoảng giá/số thanh mà không mất công cụ đang chọn.
5. **Sự kiện Doanh nghiệp & Dữ liệu Cơ bản trên Trục Biểu đồ (`Events & Financials on Chart`):**
   - Đối chiếu: Các mốc **`D` (Cổ tức / Ngày GDKHQ)**, **`S` (Chia tách / Thưởng cổ phiếu)**, **`E` (Ngày ra BCTC Quý & EPS Surprise)** hiển thị thẳng dưới chân nến, và khả năng vẽ chỉ số tài chính (`P/E`, `P/B`, `ROE`, Tăng trưởng LNST) thành đường chỉ báo bên dưới biểu đồ giá.
6. **Watchlist Chuyên nghiệp, Bản đồ Nhiệt & Độ rộng Thị trường (`Watchlist Pro, Heatmap & Market Breadth`):**
   - Đối chiếu: **Gắn Cờ màu phân loại cổ phiếu (`Red / Orange / Green / Purple Flags` để đánh dấu danh sách chờ mua/đang giữ)**, **Thanh tiêu đề chia nhóm con (`Section Headers`) & Kéo thả sắp xếp thứ tự mã (`Drag & Drop`)** ngay trong 1 Watchlist, cột tùy biến, **Bản đồ nhiệt toàn thị trường (`Stock Heatmap` theo vốn hóa × ngành)**, chỉ báo Độ rộng thị trường nội tại (`% cổ phiếu nằm trên MA20/MA50/MA200`, `Số mã Tăng/Giảm` theo ngày).
7. **Bộ lọc Cổ phiếu (`Stock Screener 2.0`):**
   - Đối chiếu: **Lọc Đa Khung Thời Gian đồng thời trong một bộ lọc** (ví dụ: `Tuần 1W tăng` **VÀ** `Ngày 1D vừa điều chỉnh cạn cung` **VÀ** `Giờ 1H cắt lên`), **Lưu & Quản lý nhiều Bộ lọc cá nhân (`Saved Custom Screens`)**, chế độ **Tự động quét cập nhật (`Auto-refresh`)**, xuất danh sách kết quả lọc thẳng thành 1 Watchlist mới chỉ bằng 1 click!
8. **Kiểm định Chiến lược (`Strategy Tester`):**
   - Đối chiếu hai chiều công bằng:
     - *TradingView có mà Sumi chưa có:* **Giải ngân từng phần / Mua gom 3 nhịp (`Pyramiding / Scale-in`) & Chốt lời từng phần (`Scale-out 1/2, 1/3`)**, **Bar Magnifier** (dùng nến khung nhỏ `1m/15m` giải quyết xung đột chạm `SL` hay `TP` trước trong 1 nến `1D`), bảng nhiệt lợi nhuận theo Tháng/Năm (`Monthly Performance Table`), xuất báo cáo chi tiết.
     - *Sumi ĐÃ VƯỢT TradingView (cần nêu rõ):* Những điểm đặc thù TTCK Việt Nam và kiểm định định lượng chống Overfitting mà TradingView bản chuẩn không hề có!
9. **Bar Replay:**
   - Đối chiếu: Nhảy tới ngày bất kỳ bằng Lịch (`Jump to Specific Date/Time`), chạy Replay ngẫu nhiên mã + ngẫu nhiên thời điểm để luyện phản xạ không thiên kiến (`Random Bar / Random Symbol Practice`), bảng tổng kết điểm số cuối phiên luyện tập (`Replay Session Debrief`), đồng bộ đa khung giờ.
10. **Hệ thống Cảnh báo (`Alerts`):**
    - Đối chiếu: Đặt cảnh báo trên **mọi chỉ báo Custom** (không chỉ `RSI/MACD/EMA`), **Cảnh báo trên cả một Watchlist (`Watchlist Alerts` — 1 cảnh báo canh tự động toàn bộ 30–60 mã!)**, thông báo đẩy ra ngoài khi thu nhỏ app (macOS Native Notification / Telegram / Webhook), điều kiện đa hợp lưu (`Alert on Combined Conditions`).
11. **Trải nghiệm Không gian Làm việc & Đa Màn Hình (`Workspace UX`):**
    - Đối chiếu: Phóng to tạm thời 1 ô biểu đồ trong lưới 4 ô lên toàn màn hình (`Maximize Single Chart Pane` bằng 1 phím/nút rồi thu về lại lưới 4 ô!), `Cmd+K` Command Palette thực thi lệnh nhanh toàn hệ thống, đồng bộ lăn chuột trục thời gian chính xác giữa các khung giờ khác nhau, lịch sử Undo/Redo toàn diện.

---

## 3. ĐẦU RA BÀN GIAO YÊU CẦU

Xuất báo cáo đầy đủ vào tệp `/Users/mizuhara/workspace/sumi/docs/qa/09_SUMI_DATA_FEATURE_AND_TRADINGVIEW_GAP_AUDIT.md` gồm 4 chương rõ ràng:
- **Chương 1 — Báo cáo Kiểm toán Dữ liệu & Nguồn cấp của Sumi** (kèm bảng số liệu thực tế trích xuất từ `sumi_market.db` và phân tích tiềm năng chưa khai thác của `tradingview-rs` + `cafef_importer`).
- **Chương 2 — Bản đồ Toàn bộ Tính năng Hiện tại của Sumi `v2.2.0`** (phân tách rõ `✅ Trên UI` vs `⚠️ Có sẵn trong Rust nhưng chưa mở lên UI` vs `🧪 Xấp xỉ`).
- **Chương 3 — Ma trận Đối chiếu Sumi vs. TradingView (11 Phân hệ)** (chỉ rõ từng tính năng TradingView có mà Sumi chưa làm được, mức độ phức tạp kỹ thuật `Dễ / Trung bình / Khó / Rất khó (cần nguồn data mới)`).
- **Chương 4 — Đề xuất Top 10–15 Tính năng Đáng Phát Triển Nhất Cho Sumi** (lọc bỏ những tính năng mạng xã hội rườm rà của TradingView không cần thiết, chỉ chọn những vũ khí thực sự nâng tầm Phân tích Kỹ thuật, Lọc cổ phiếu, Replay & Backtest theo phương pháp QMV trên TTCK Việt Nam — xếp theo ma trận `Giá trị thực chiến cao × Độ khả thi kỹ thuật`).

Sau khi ghi tệp báo cáo xong, hãy trình bày bản tóm tắt sắc gọn các phát hiện quan trọng nhất cho tôi ngay trên khung chat.
````
