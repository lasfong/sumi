# KẾ HOẠCH TRIỂN KHAI SUMI `v1.1` — ĐÓNG GÓI MACOS HOÀN CHỈNH & TRẢI NGHIỆM TRADINGVIEW PRO

- **Dự án:** Sumi Trading Workstation (`/Users/mizuhara/workspace/sumi`)
- **Nền tảng đầu vào:** `v1.0.0 / R19 Patch` (`34/34` lỗi QA đã đóng, `65/65` kịch bản `PASS`, `274/274` tests, `0` cảnh báo clippy native & `wasm32`)
- **Mục tiêu `v1.1`:** Nâng cấp trải nghiệm thao tác đạt chuẩn **TradingView Pro Desktop** (thanh công cụ vẽ dọc bên trái, kênh giá song song, thước đo giá/thời gian, kéo-thả trực tiếp `SL`/`TP`/lệnh chờ trên biểu đồ, kéo-thả tệp `.csv`/`.json`), bổ sung **xuất báo cáo Strategy Lab dạng HTML/In PDF 1 chạm**, và đóng gói phát hành macOS thương mại hoàn chỉnh (`AppIcon.icns`, file association `.csv`/`.json`, bộ cài đặt `.dmg`).
- **Ràng buộc bất biến:**
  1. Giữ nguyên kiến trúc **Zero-Network / Offline-First / `wasm32-unknown-unknown` clean** (không thêm crate mạng hay phụ thuộc nặng vào `Cargo.toml`).
  2. Duy trì ngân sách khung hình `< 4.0 ms` (`viewport_bench --r12 --r17 --r18 --r19` toàn bộ `overall_pass: true`).
  3. Không sửa/xóa `/Users/mizuhara/workspace/work/CafeFData/*.zip`, `ARCHIVE/`, hoặc các bằng chứng lịch sử trong `docs/evidence/`; không đụng chạm TradingView Desktop (`DEC-006`).
  4. Mọi Wave đều có bài kiểm thử tự động, kiểm tra `fmt` + `clippy` (native & `wasm32`), và soi ảnh Metal GPU thực tế qua `read_file`.

---

## 1. Lộ Trình Thực Thi Theo Wave (`Wave 0 -> Wave 5`)

### WAVE 0 — Chuẩn Hóa & Chốt Mốc Nền `v1.0.0`
- **Mục tiêu:** Bảo đảm nền mã nguồn `v1.0.0` sạch tuyệt đối trước khi mở rộng `v1.1`.
- **Hạng mục:**
  1. Chạy `cargo fmt --all` để chuẩn hóa định dạng toàn bộ kho mã nguồn.
  2. Chạy xác minh `cargo clippy --all-targets -- -D warnings`, `cargo clippy --lib --target wasm32-unknown-unknown -- -D warnings`, và `cargo test`.
  3. Cập nhật sổ cái dự án ([`EXECUTION_STATE.md`](../EXECUTION_STATE.md), [`NEXT_SESSION_BRIEF.md`](../NEXT_SESSION_BRIEF.md), [`FEATURE_BACKLOG_AND_PARITY_LEDGER.md`](../FEATURE_BACKLOG_AND_PARITY_LEDGER.md)) sang trạng thái `v1.0.0 / GO FOR v1.0`.
  4. *(Tùy chọn theo phê duyệt PO)* Tạo commit chốt mốc `v1.0.0` sạch trước khi bắt đầu nhánh `v1.1`.

---

### WAVE 1 — Thanh Vẽ Dọc TradingView (`Left Vertical Drawing Rail`) & Kéo-Thả Tệp `.csv`/`.json`
- **Mục tiêu:** Mang bố cục đặc trưng của TradingView Desktop vào Sumi và đơn giản hóa thao tác mở dữ liệu chỉ bằng 1 động tác kéo-thả.
- **Hạng mục kỹ thuật:**
  1. **Left Vertical Drawing Rail (`src/app.rs`):**
     - Bổ sung thanh dọc mảnh (`36–38 px`) sát cạnh trái vùng biểu đồ, hiển thị toàn bộ công cụ vẽ (`Cursor`, `H-Line`, `Trend`, `Ray`, `Parallel Channel`, `Rect`, `Fib`, `Pos R:R`, `Measure`, `FRVP`, `AVWAP`, `Note`) bằng **biểu tượng vector vẽ bằng `egui::Painter`** (sắc nét `2.0x` Retina, không bao giờ lỗi thiếu glyph).
     - Tích hợp cụm thao tác nhanh bên dưới Rail: `Magnet OHLC`, `Undo`, `Redo`, `Inspector`, `Clear`, và nút thu gọn/mở rộng Rail (tự động lưu trong `WorkstationSettings` / `WorkspaceTemplate` tương thích ngược).
     - Dọn bớt cụm `Draw:` trên Top Toolbar Row 2 -> Top Toolbar cực kỳ thoáng mắt ở cả `1280×800` lẫn `1440×1000`.
  2. **Kéo-Thả Tệp (`Drag & Drop`) & Đối Số Dòng Lệnh (`src/app.rs`, `src/main.rs`):**
     - Lắng nghe `ui.input(|i| i.raw.hovered_files / dropped_files)`: khi người dùng kéo file `.csv`/`.json` từ Finder vào cửa sổ Sumi, hiện lớp phủ viền xanh `"Drop CSV / JSON file to load OHLCV"` và tự động nạp qua `import_file_from_path(...)` khi thả chuột (giữ nguyên biểu đồ hiện hành nếu tệp không hợp lệ).
     - Hỗ trợ nhận đường dẫn tệp qua `std::env::args()` trong `src/main.rs` (`sumi-workstation /path/to/file.csv`) để mở trực tiếp từ Terminal hoặc Finder.
- **Cổng kiểm chứng Wave 1 (`Gate W1`):**
  - Test tự động: kích hoạt mọi công cụ trên Rail, thu gọn/mở rộng Rail không làm tràn khung `1280×800`, nạp file qua luồng drop & xử lý an toàn khi thả file không hợp lệ (không phá biểu đồ đang mở).
  - `cargo test` & `wasm32` clippy `100% PASS`.

---

### WAVE 2 — 2 Công Cụ Vẽ Chuyên Nghiệp Mới (`ParallelChannel` & `PriceDateRange Measure`)
- **Mục tiêu:** Hoàn thiện bộ công cụ vẽ lên **`11` công cụ chuẩn TradingView Pro** (tất cả hỗ trợ `Undo/Redo`, `DEF-001` Replay, chuyển khung `1D <-> 1W`, và `Drawing Inspector`).
- **Hạng mục kỹ thuật (`src/drawings/`, `src/chart.rs`, `src/app.rs`):**
  1. **`DrawingTool::ParallelChannel` & `DrawingGeometry::ParallelChannel` (Kênh Giá Song Song 3 Điểm):**
     - Cấu trúc neo: `start: DrawingAnchor`, `end: DrawingAnchor`, `channel_offset: f64` (độ lệch giá của biên song song, có thể kéo lên/xuống tự do).
     - Pha vẽ 3 bước mượt mà giống TradingView: Click 1 đặt `start` -> Click 2 đặt `end` (tạo đường xu hướng gốc) -> Di chuột & Click 3 đặt độ rộng kênh `channel_offset`.
     - Hiển thị: 2 đường biên song song, đường trung tuyến `50%` nét đứt, vùng tô mờ giữa kênh (tùy chỉnh độ mờ `fill_alpha`), hỗ trợ `extend_left` / `extend_right`, `3` tay cầm (`Start`, `End`, `ChannelOffset`) + kéo nguyên kênh.
  2. **`DrawingTool::PriceDateRange` & `DrawingGeometry::PriceDateRange` (Thước Đo Giá & Thời Gian — `Measure`):**
     - Cấu trúc neo: `start: DrawingAnchor`, `end: DrawingAnchor`.
     - Tính toán nhân quả trên lát nến hiện hữu (`&[Candle]`):
       - Chênh lệch giá tuyệt đối & phần trăm (`+12.40 (+24.41%)` hoặc `-4.20 (-8.27%)`),
       - Số thanh nến (`42 bars`),
       - Khoảng thời gian (`61d` hoặc `6h 30m` trên intraday),
       - Tổng khối lượng giao dịch trong vùng đo (`Vol 148.2M` dùng `format_compact_unsigned_volume`).
     - Hiển thị hộp xanh/đỏ bán trong suốt có 2 mũi tên chữ thập ở tâm và thẻ thông số nổi chống cắt biên.
     - Hỗ trợ phím tắt `Shift+Click` nhanh trên chế độ `Cursor` để đo tức thì (nhấn `Esc` hoặc click thường để thoát).
- **Cổng kiểm chứng Wave 2 (`Gate W2`):**
  - Test đơn vị & property test cho `ParallelChannel` (hit-test 3 handle + body, extend 2 đầu, `1D -> 1W -> 1D` roundtrip, `Undo/Redo`, `DEF-001` ẩn trong Replay) và `PriceDateRange` (độ chính xác `ΔGiá`, `Δ%`, số thanh, tổng volume, chịu được `NaN`/0 volume).
  - `cargo test` & `wasm32` clippy `100% PASS`.

---

### WAVE 3 — Giao Dịch Trực Tiếp Trên Biểu Đồ (`Chart-Direct Trading`: Kéo Thả `SL`/`TP` & Lệnh Chờ)
- **Mục tiêu:** Đạt độ mượt tương tác đặt/sửa lệnh trực tiếp ngay trên biểu đồ nến như TradingView Pro, tuân thủ tuyệt đối luật thị trường Việt Nam.
- **Hạng mục kỹ thuật (`src/trading/`, `src/chart.rs`, `src/app.rs`):**
  1. **Kéo thả trực tiếp `SL` & `TP` của vị thế đang mở:**
     - Cho phép nắm trực tiếp vào đường hoặc nhãn `SL` / `TP` của vị thế trên biểu đồ và kéo dọc theo trục giá (`ChartTradingDragAction`).
     - Khi kéo: tự động làm tròn theo bước giá sàn hiện tại (`HOSE` chuyển bậc `10.00`/`50.00`, `HNX`, `UPCOM`, `VN30F1M`, US/Crypto), hiển thị nhãn preview trực tiếp (`SL 49.50 · -262 kVND (-1.0R)` / `TP 55.00 · +838 kVND (+3.2R)`).
     - Khi nhả chuột: cập nhật vị thế qua `TradingCommand::UpdatePositionBrackets`, đồng bộ ngay ô `SL`/`TP` trên `Order Ticket`, và **tuyệt đối không làm sai lệch `had_initial_sl_at_open`** (bảo toàn chống gian lận `SL Adherence`).
  2. **Kéo thả sửa giá lệnh chờ (`Limit` / `Stop`) & nút đóng/hủy nhanh (`×`) ngay trên biểu đồ:**
     - Bổ sung `TradingCommand::ModifyOrderPrice { order_id, new_price }` (tất định trong command log để tua lùi Bar Replay khôi phục bit-exact 100%!).
     - Kéo trực tiếp nhãn lệnh chờ `BUY LMT` / `SELL STP` lên/xuống để dời giá kích hoạt (tôn trọng biên trần/sàn `daily_band` trên sàn VN).
     - Nút `×` nhỏ gắn cuối nhãn lệnh chờ (hủy lệnh) và nhãn vị thế `LONG/SHORT` (gọi `flatten_open_position`, tôn trọng khóa `T+2` FIFO chỉ xả phần `sellable`).
- **Cổng kiểm chứng Wave 3 (`Gate W3`):**
  - Test hồi quy: kéo `SL`/`TP` trên `HOSE` hít đúng bước giá `0.01/0.05/0.10`, không phá `had_initial_sl_at_open`; kéo `Limit` order tạo command log tua lùi Bar Replay bit-exact; nút `×` trên vị thế có hàng khóa `T+2` chỉ bán phần `sellable` và giữ nguyên phần `locked`.
  - `cargo test` & `wasm32` clippy `100% PASS`.

---

### WAVE 4 — Xuất Báo Cáo Strategy Lab Chuyên Nghiệp (`HTML / Print-to-PDF` 1 Chạm)
- **Mục tiêu:** Cho phép trader/quant xuất báo cáo kiểm thử chiến lược hoàn chỉnh, đẹp mắt, tự chứa 100% (không cần mạng) để lưu trữ, chia sẻ hoặc in ra PDF (`Cmd+P -> Save as PDF`).
- **Hạng mục kỹ thuật (`src/backtest/report_html.rs`, `src/app.rs`):**
  1. Xây dựng `render_backtest_report_html(...)` thuần Rust (`wasm32`-compatible, zero dependencies):
     - **Tóm tắt cấu hình & `RunManifest`:** Mã, Sàn, Khung thời gian, Chiến lược, Tham số, Phí/Thuế/Trượt giá, ngày dữ liệu, mã băm tất định `manifest_digest`.
     - **Lưới KPI Tearsheet:** `Total Return %`, `CAGR`, `Sharpe`, `Sortino`, `Calmar`, `Max Drawdown %`, `Win Rate %`, `Profit Factor`, `Expectancy`, `Total Trades`.
     - **2 Biểu đồ SVG Vector nội tuyến:** Đường cong vốn (`Equity Curve` so sánh `Buy & Hold`) và biểu đồ sụt giảm (`Underwater Drawdown %`), căn tỷ lệ chính xác theo dữ liệu thực tế.
     - **Các bảng phân tích độ bền:** `OutlierImpact`, `Drawdown Periods`, `Yearly Breakdown`, `Signal-Bar Regime Breakdown`, `Multi-Phase Batch`, và bảng danh sách lệnh (`Trade Log`) đầy đủ.
     - **Bảo mật & In ấn:** Escape HTML (`& < > " '`) toàn bộ chuỗi ký tự chống XSS; CSS dark/light print stylesheet (`@media print`) tách trang gọn gàng cho khổ A4/Letter.
  2. Gắn nút **`Export HTML`** ngay trong `Strategy Lab` (`Stats` & `Slices` tabs) và lệnh **`Export Backtest HTML Report`** trong `Cmd+K`, xuất tệp vào `exports/sumi_backtest_<SYM>_<STRAT>_<SEQ>.html` và hiển thị thông báo trạng thái rõ ràng.
- **Cổng kiểm chứng Wave 4 (`Gate W4`):**
  - Test tự động: xác minh HTML sinh ra hợp lệ, chứa đủ SVG equity/drawdown, KPI khớp `BacktestTearsheet`, escape sạch chuỗi độc hại (`<script>`, `=CMD`), tất định 100% giữa 2 lần gọi cùng input.
  - `cargo test` & `wasm32` clippy `100% PASS`.

---

### WAVE 5 — Đóng Gói macOS Thương Mại (`AppIcon.icns`, `.dmg` Installer, File Association) & Tái Kiểm Toán `v1.1`
- **Mục tiêu:** Hoàn thiện bản phân phối macOS chuyên nghiệp như ứng dụng thương mại thực thụ và nghiệm thu toàn diện `v1.1`.
- **Hạng mục kỹ thuật:**
  1. **Biểu tượng ứng dụng `AppIcon.icns` (`assets/icons/` + script tạo tự động):**
     - Tạo biểu tượng **Sumi** sắc nét (phong cách tối giản tài chính: nền đêm `#0D111A`, 3 cây nến tăng/giảm màu Ngọc Lục Bảo `#26A69A` / Đỏ Thẫm `#EF5350` cùng đường xu hướng ánh lam `#2962FF`, bo góc squircle chuẩn macOS Sequoia/Sonoma).
     - Xuất đầy đủ `10` cỡ iconset (`16x16` .. `512x512@2x` = `1024×1024`) và đóng gói bằng `/usr/bin/iconutil -c icns` thành `Contents/Resources/AppIcon.icns`.
  2. **Cập nhật `scripts/package_macos_app.sh` (`v1.1.0`):**
     - Nâng `Cargo.toml` -> `1.1.0`, đồng bộ `CFBundleShortVersionString = 1.1.0`, `CFBundleVersion = 20`.
     - Thêm `CFBundleIconFile = AppIcon` và `CFBundleDocumentTypes` cho `.csv` & `.json` (`UTType` chuẩn macOS) vào `Info.plist`.
     - Đóng gói `dist/Sumi Workstation.app` + ký `codesign --force --deep --sign -`.
  3. **Tạo bộ cài đặt `scripts/package_macos_dmg.sh`:**
     - Đóng gói ảnh đĩa nén `UDZO` tại `dist/Sumi-Workstation-v1.1.0-arm64.dmg` gồm `Sumi Workstation.app`, symlink `/Applications`, và tệp hướng dẫn nhanh `README_INSTALL.txt`.
     - Kiểm thử gắn ảnh đĩa (`hdiutil attach` -> kiểm tra cấu trúc + chữ ký `codesign` bên trong -> `hdiutil detach`).
  4. **Nghiệm thu cuối `v1.1`:**
     - Chạy `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo clippy --lib --target wasm32-unknown-unknown -- -D warnings`, `cargo test` (`100% PASS`).
     - Chạy lại `viewport_bench --r12 --r17 --r18 --r19` trong thư mục cách ly xác nhận `4/4 overall_pass: true` (`p99 < 4.0 ms`).
     - Chụp ảnh Metal GPU (`2560×1600` & `2880×2000`) vào `docs/testing/v1_1_evidence/` (bao gồm Left Rail, `ParallelChannel`, `PriceDateRange Measure`, `Chart-Direct SL/TP`, `2x2` + `471` Watchlist) và soi từng ảnh bằng `read_file`.
     - Kiểm tra `TC-65` (`7/7` `CafeFData/*.zip` SHA-256 nguyên vẹn, `ARCHIVE/` & `docs/evidence/` `0` thay đổi) và xuất báo cáo nghiệm thu `docs/testing/V1_1_RELEASE_AUDIT.md`.

---

## 2. Tóm Tắt Giá Trị Bàn Giao Sau Khi Hoàn Tất `v1.1`

| Hạng Mục | Hiện Tại (`v1.0.0`) | Sau `v1.1` |
| :--- | :--- | :--- |
| **Thanh vẽ** | Nằm trong menu `Draw:` ở Top Toolbar | **Left Vertical Drawing Rail** vector 1 chạm (có thu gọn) kiểu TradingView Pro |
| **Số công cụ vẽ** | `9` công cụ | **`11` công cụ** (+ `Parallel Channel` 3 điểm & `Price/Date Range Measure`) |
| **Sửa `SL`/`TP`/Lệnh chờ** | Nhập tay qua `Order Ticket` / `DOM` | **Kéo-thả trực tiếp ngay trên biểu đồ nến** (hít bước giá sàn + nút `×` hủy/đóng nhanh) |
| **Mở dữ liệu `.csv`/`.json`** | Dán đường dẫn hoặc bấm nút nhanh | **Kéo-thả trực tiếp từ Finder vào cửa sổ** + đối số CLI + macOS File Association |
| **Báo cáo Strategy Lab** | Xem trong Dock & sao chép JSON/RON | **Xuất báo cáo HTML tự chứa có biểu đồ SVG + In PDF chuẩn A4/Letter** |
| **Đóng gói macOS** | `.app` không icon riêng, không `.dmg` | **`AppIcon.icns` chuẩn Retina + `Sumi-Workstation-v1.1.0-arm64.dmg`** sẵn sàng cài vào `/Applications` |
