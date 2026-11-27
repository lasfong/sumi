# 03. Báo Cáo Kiểm Định Chi Tiết & Danh Sách Lỗi (P0 → P4) — Sumi Supercharts (`v2.1.0`)

> **Phương pháp kiểm định:** Hai Tầng Thực Tế — **Tầng A** (`/Applications/Sumi Supercharts.app` native macOS, Apple M1 16 GB, CSDL thật `sumi_market.db` 84 MB / 958.359 nến) & **Tầng B** (`Real-Backend E2E & Telemetry Bridge` nối trực tiếp tới toàn bộ 51 Tauri commands + `AppState` + `sumi-core` + `sumi-data` + `tradingview-rs` + Chromium CDP trên `ui/dist` nguyên bản). **0% kiểm thử trên `webFallback.ts`.**  
> **Lưu ý minh bạch về quyền hệ điều hành (macOS TCC):** Trên máy kiểm định, `CGPreflightScreenCaptureAccess() == false` và `AXIsProcessTrusted() == false` nên `screencapture -l <wid>` và tiêm sự kiện GUI Accessibility vào tiến trình native bị hệ điều hành chặn (tuyệt đối không vượt rào). Tầng A xác minh vòng đời tiến trình native, độ trễ mở cửa sổ, handle SQLite/socket, `footprint`/`RSS`/`%CPU` và bền vững qua Quit/Reopen; Tầng B thực thi toàn bộ tương tác UI/Canvas/DOM, đo telemetry và chụp ảnh bằng chứng 2× DPI trong [`docs/qa/evidence/`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence/).

---

## 1. Bảng Tổng Hợp Kết Quả Kiểm Thử (113 Test Cases / 11 Phân Hệ)

| # | Phân hệ | Tiền tố | Tổng | PASS | FAIL | BLOCKED | Tỷ lệ PASS | Tóm tắt phát hiện cốt lõi |
|---|---------|---------|------|------|------|---------|------------|---------------------------|
| 1 | Dữ liệu 3 Tầng, CafeF & Offline | `TC-DATA` | 12 | 9 | 3 | 0 | 75.0% | Cold/Warm start, SQLi, Offline & Race 10 mã tốt; **91 nến cuối tuần CafeF**, **11 mã HOSE nhảy bậc nối CafeF–TV >7%**, trùng `HNX:VN301!` trong `symbols`. |
| 2 | Biểu Đồ KLineChart v10 & Quy Tắc VN | `TC-CHART` | 12 | 7 | 5 | 0 | 58.3% | Dải `T/TC/S` đúng 100% 3 bậc HOSE/HNX/UPCOM/VN30F; **forward lazy-load ghi đè `vnBand`**, **Countdown bị đóng băng**, **HA trôi tick**, **AAPL 4 số lẻ**, **thiếu nút đổi múi giờ**. |
| 3 | Đa Biểu Đồ & Đồng Bộ | `TC-GRID` | 10 | 8 | 2 | 0 | 80.0% | 5 lưới, resize, MTF `1D/4H/1H/15m`, lưu/khôi phục Layout tốt; **`CrosshairBus` tê liệt 100%** (lệch API `klinecharts@10.1.1`), lỗi nạp lại `pane_indicators` & ô 4H/1H chậm nạp. |
| 4 | Hệ Thống 32 Chỉ Báo Kỹ Thuật | `TC-IND` | 12 | 6 | 5 | 1 | 50.0% | Đủ 32 chỉ báo, 5 Custom vẽ mượt; **Modal ghép sai dòng màu (`BOLL 2/3`)**, **5 Custom phớt lờ đổi màu**, **`VWAP` sai reset 4H & phương sai âm**, **MACD lệch 2×**, **`sumi-core` VWAP lookahead**. |
| 5 | 37 Công Cụ Vẽ & Tương Tác Canvas | `TC-DRAW` | 12 | 9 | 3 | 0 | 75.0% | Đủ 37 công cụ, Magnet hút OHLC chuẩn tuyệt đối, FloatingToolbar & ContextMenu tốt; **`Option+T/H/J/V/F/R` hỏng trên macOS**, **Undo bỏ qua kéo anchor & thiếu Redo**, **không sync đa ô**. |
| 6 | Bar Replay Nhân Quả (`DEF-001`) | `TC-REPLAY` | 10 | 8 | 2 | 0 | 80.0% | Nhân quả `DEF-001` trên **1 ô đơn lẻ** & tua lùi Broker tất định tốt; **bật Replay xóa trắng sổ lệnh Paper Broker live**, tốc độ `20x` bị kẹp còn `10x`. |
| 7 | Paper Broker VN T+2 & Phái Sinh VN30F | `TC-BROKER` | 12 | 6 | 6 | 0 | 50.0% | Chặn short cơ sở & khóa lô T+2 FIFO chuẩn; **VN30F1! ký quỹ 3.4% (29.4×) & sai PnL 100.000×**, **thiếu kiểm tra bước giá**, **`ATO/ATC` = `MP`**, **đổi khung giờ mất sổ lệnh**, **Context Menu đặt 1.000 HĐ VN30F**. |
| 8 | Strategy Tester & Stock Screener 101 | `TC-QUANT` | 10 | 7 | 3 | 0 | 70.0% | Lõi Rust Backtest/WFO/Monte Carlo cực nhanh (`<3ms`) & tất định; **UI ép luật cổ phiếu lên VN30F1!**, **Screener thiếu sàn Crypto/US & quét lạnh 5.4s**, **MACD lệch 2× vs Chart**. |
| 9 | Watchlist, Alerts, Hotkeys & Persistence | `TC-WIDGET` | 10 | 5 | 5 | 0 | 50.0% | CRUD Watchlist + Cascade + Restart tốt; **va chạm ID Alert `alert_{now_ms}` mất 4/5 cảnh báo**, **Alert không nổ ngoài Watchlist/Replay**, **DOM Churn 30 remount/12s**, **`"0.1 VND"` trên VN30F1!**, **tên Watchlist rỗng được chấp nhận**. |
| 10 | Hiệu Năng, FPS, Soak & Rò Rỉ | `TC-PERF` | 8 | 6 | 2 | 0 | 75.0% | Startup native `~604ms`, L1 `0.62ms`, L2 `2.34ms`, Canvas khóa `60.0 FPS`; **Soak 110 thao tác rò rỉ `3` `bar_streams` + `78` Tauri Channel callbacks**, Screener lạnh vượt SLA `1.5s`. |
| 11 | Hành Trình Liên Phân Hệ (MTF + Replay + PTKT) | `TC-MTF-REPLAY` | 5 | 0 | 5 | 0 | 0.0% | **Hậu kiểm góc nhìn trader thực chiến:** **Đa ô `1h`+`4h` hoàn toàn không đồng bộ Replay, lộ 100% nến tương lai & bơm nhầm nến khi click ô 2 (`BUG-040`)**, **lộ giá live ở Watchlist/Detail (`BUG-041`)**, **lộ mũi tên Backtest tương lai (`BUG-042`)**, thiếu Sync Date Range & Multi-Window (`BUG-043`). |
| **TỔNG** | **11 Phân Hệ** | **113 TCs** | **113** | **71** | **41** | **1** | **62.8%** | **Phát hiện 43 Bug xác thực (3 P0, 15 P1, 15 P2, 6 P3, 4 P4).** |

### Ma Trận Trạng Thái Chi Tiết 113 Test Cases

| Phân hệ | Kết quả từng Test Case |
|---------|------------------------|
| **TC-DATA** | `001:PASS`, `002:PASS`, `003:FAIL(BUG-001)`, `004:FAIL(BUG-003)`, `005:PASS`, `006:PASS`, `007:FAIL(BUG-025)`, `008:PASS`, `009:PASS`, `010:PASS`, `011:PASS`, `012:PASS` |
| **TC-CHART** | `001:PASS`, `002:FAIL(BUG-015)`, `003:FAIL(BUG-016)`, `004:FAIL(BUG-031)`, `005:PASS`, `006:PASS`, `007:PASS`, `008:PASS*`, `009:FAIL(BUG-005)`, `010:FAIL(BUG-024)`, `011:PASS`, `012:PASS` |
| **TC-GRID** | `001:PASS`, `002:PASS`, `003:PASS`, `004:PASS`, `005:PASS`, `006:FAIL(BUG-004)`, `007:PASS`, `008:PASS`, `009:PASS`, `010:FAIL(BUG-038)` |
| **TC-IND** | `001:PASS`, `002:PASS`, `003:PASS`, `004:FAIL(BUG-019)`, `005:FAIL(BUG-018)`, `006:BLOCKED(BUG-035)`, `007:FAIL(BUG-020)`, `008:FAIL(BUG-010)`, `009:PASS`, `010:PASS`, `011:PASS`, `012:FAIL(BUG-014)` |
| **TC-DRAW** | `001:PASS`, `002:FAIL(BUG-021)`, `003:PASS`, `004:PASS`, `005:PASS`, `006:PASS`, `007:PASS`, `008:FAIL(BUG-022)`, `009:PASS`, `010:FAIL(BUG-027)`, `011:PASS`, `012:PASS` |
| **TC-REPLAY** | `001:PASS`, `002:PASS`, `003:PASS`, `004:FAIL(BUG-028)`, `005:PASS`, `006:PASS`, `007:FAIL(BUG-006)`, `008:PASS`, `009:PASS`, `010:PASS` |
| **TC-BROKER** | `001:PASS`, `002:PASS`, `003:FAIL(BUG-009)`, `004:FAIL(BUG-009)`, `005:PASS`, `006:FAIL(BUG-026)`, `007:PASS`, `008:PASS`, `009:FAIL(BUG-008)`, `010:FAIL(BUG-002)`, `011:FAIL(BUG-007)`, `012:PASS` |
| **TC-QUANT** | `001:PASS`, `002:FAIL(BUG-011)`, `003:PASS`, `004:PASS`, `005:PASS`, `006:PASS`, `007:FAIL(BUG-023)`, `008:PASS`, `009:FAIL(BUG-010)`, `010:PASS` |
| **TC-WIDGET** | `001:PASS`, `002:FAIL(BUG-033)`, `003:FAIL(BUG-030)`, `004:FAIL(BUG-032)`, `005:FAIL(BUG-017)`, `006:PASS`, `007:FAIL(BUG-017)`, `008:PASS`, `009:PASS`, `010:PASS` |
| **TC-PERF** | `001:PASS`, `002:PASS`, `003:PASS`, `004:PASS*`, `005:PASS`, `006:PASS`, `007:FAIL(BUG-023)`, `008:FAIL(BUG-012)` |
| **TC-MTF-REPLAY** | `001:FAIL(BUG-040)†`, `002:FAIL(BUG-040)†`, `003:FAIL(BUG-041)`, `004:FAIL(BUG-042/BUG-027)†`, `005:FAIL(BUG-043)†` |

*(Ghi chú: `TC-CHART-008` PASS với synthetic key event nhưng lưu ý lỗi phím `Option+R` trên macOS tại `BUG-021`; `TC-PERF-004` đạt 60 FPS nhưng cần đo lại sau khi sửa `BUG-004` vì hiện ô phụ chưa vẽ crosshair; `TC-IND-006` BLOCKED do nút 👁/⚙/✕ vẽ trong Canvas không có DOM node truy cập tự động — xem `BUG-035`; các mục đánh dấu `†` ở Phân hệ 11 được phát hiện trong vòng hậu kiểm hành trình liên phân hệ, xác minh trực tiếp qua truy vết dòng thực thi mã nguồn + bằng chứng ảnh chụp Tầng B [`11_replay_active.png`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence/11_replay_active.png), chưa chạy lại script Tầng B riêng.)*

---

## 2. Chi Tiết Toàn Bộ Lỗi Đã Xác Minh (Sắp Xếp Nghiêm Ngặt P0 → P4)

### Mức Độ P0 — Nghiêm Trọng (Blocker / Sai Lệch Tài Chính Hoặc Dữ Liệu Thị Trường)

#### `BUG-001` — Nến Cuối Tuần Bị Đảo `DD/MM` Trong CafeF INDEX Gây Gai Giá Ảo `+19.79% / -16.61%`
- **Test Case:** `TC-DATA-003` | **Mức độ:** **P0 (Critical)** | **Phân hệ:** Data / CafeF Importer
- **Các bước tái hiện:**
  1. Mở `sumi_market.db` (đã nạp `data/raw/cafef/CafeF.INDEX.Upto06112026.csv`).
  2. Chọn `HNX:HNXINDEX` hoặc `HOSE:VNINDEX` khung `1D`, cuộn về tháng 02–03/2026.
- **Thực tế vs Kỳ vọng (TradingView):**
  - **Thực tế:** Tồn tại **91 nến rơi vào Thứ 7 / Chủ Nhật** trên các chỉ số VN (`cafef_index`). Trong năm 2026:
    - `HNX:HNXINDEX`: ngày `2026-02-07` (**Thứ 7**, thực tế bị đảo từ `02/07/2026`) đóng cửa `307.0` (nhảy vọt **`+19.79%`** từ `256.28` rồi sập **`-16.61%`** vào Thứ 2 `256.01`); ngày `2026-03-08` (**Chủ Nhật**) đóng `279.28`.
    - `HOSE:VNINDEX`: ngày `2026-02-07` (**Thứ 7**) đóng `1,866.0` (nhảy **`+6.30%`** từ `1,755.49` rồi rơi `-5.96%`); ngày `2026-03-08` (**Chủ Nhật**) đóng `1,762.84`.
  - **Kỳ vọng:** Thị trường chứng khoán Việt Nam không giao dịch cuối tuần; mọi nến Thứ 7 / Chủ Nhật phải bị lọc bỏ/cách ly khi import.
- **Bằng chứng:** Log SQLite `TC-DATA-003` (`91 VN weekend bars`) và ảnh [`bug_weekend_spike_hnxindex.png`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence/bug_weekend_spike_hnxindex.png).
- **Nguyên nhân gốc:** [`crates/sumi-data/src/cafef_importer.rs`](file:///Users/mizuhara/workspace/sumi/crates/sumi-data/src/cafef_importer.rs) phân tích `DTYYYYMMDD` nhưng không gọi kiểm tra `is_vn_exchange_trading_day_days` / lọc Thứ 7–Chủ Nhật cho sàn VN. Khi TradingView cập nhật các ngày làm việc trong tuần, các nến cuối tuần sai ngày của CafeF kẹt vĩnh viễn ở giữa chuỗi.
- **Khuyến nghị sửa:** Loại bỏ (quarantine) toàn bộ dòng có `weekday >= 5` (Sat/Sun) đối với sàn `HOSE`/`HNX`/`UPCOM` ngay trong `CafeFImporter`, kèm tác vụ dọn sạch nến cuối tuần hiện hữu trong `sumi_market.db`.

---

#### `BUG-002` — Phái Sinh `HNX:VN30F1!`: Ký Quỹ Thấp Hơn 5 Lần (`3.4%` = Đòn Bẩy `29.4×`) & Sai Lệch `100.000×` PnL/Notional Trên UI
- **Test Case:** `TC-BROKER-010` | **Mức độ:** **P0 (Critical)** | **Phân hệ:** Paper Broker (VN30F Derivatives)
- **Các bước tái hiện:**
  1. Chuyển sang `HNX:VN30F1!` khung `1D` (giá khớp `1,780.2`, vốn `500.000.000 VND`).
  2. Đặt lệnh Short `26` hợp đồng `MP` (ở ký quỹ luật định 17%, 1 HĐ cần `~30.26M VND`, vốn 500M chỉ mua tối đa `16` HĐ).
  3. Đặt lệnh Short `2` hợp đồng `MP` và quan sát đồng thời: (a) thanh KPI phía trên, (b) cột `Lãi / Lỗ` trong bảng Vị thế, (c) đường vị thế trên Canvas, (d) ô `Giá trị ước tính` bên trái.
- **Thực tế vs Kỳ vọng (TradingView / VSD):**
  - **Thực tế (đo sống):**
    - Lệnh Short **`26` hợp đồng** (ký quỹ luật định `~787 triệu > 500 triệu`) được **CHẤP NHẬN** vì động cơ tính ký quỹ hiệu dụng là `17% / 5.0 = 3.4%` (**đòn bẩy `29.4×`** thay vì `5.88×`)!
    - Khi mở `SHORT 2 @ 1,780.022`: thanh KPI Rust báo **`Lãi/Lỗ tạm tính: -35,604 đ`**, nhưng ngay bên dưới bảng Vị thế báo **`Lãi / Lỗ: -0 đ (-0.01%)`** và đường vị thế trên biểu đồ ghi **`SHORT 2 @ 1,780.022 (-0)`** (**sai lệch 100.000 lần!**).
    - Ô đặt lệnh bên trái giữ nguyên `KL = 1000` (không reset từ HOSE) và hiển thị `Giá trị ước tính: 1,780,200 đ` thay vì **178.020.000.000 đ** (thiếu `× 100.000` — **sai lệch 100.000 lần!**).
  - **Kỳ vọng:** Ký quỹ ban đầu đúng `17.0%` (không chia cho `max_leverage`), chặn đặt quá `16` HĐ trên vốn 500M; toàn bộ Notional & PnL trên Ticket, Bảng Vị thế, Đường biểu đồ và KPI khớp tuyệt đối với hệ số nhân `100.000 VND/điểm`.
- **Bằng chứng:** Ảnh [`bug_vn30f_broker_100k_pnl_and_margin.png`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence/bug_vn30f_broker_100k_pnl_and_margin.png) và log `TC-BROKER-010` (`maxStatutory=16, submitted=26 -> ACCEPTED=true`).
- **Nguyên nhân gốc:**
  1. [`crates/sumi-desktop/src/commands/broker.rs:258-259`](file:///Users/mizuhara/workspace/sumi/crates/sumi-desktop/src/commands/broker.rs#L258-L259) đặt đồng thời `initial_margin_pct: 17.0` và `max_leverage: 5.0`, trong khi [`crates/sumi-core/src/trading/orders.rs:184`](file:///Users/mizuhara/workspace/sumi/crates/sumi-core/src/trading/orders.rs#L184) tính `margin_frac = (initial_margin_pct / 100.0) / max_leverage` (= `3.4%`).
  2. [`ui/src/components/dock/PaperBrokerTab.tsx:188,498`](file:///Users/mizuhara/workspace/sumi/ui/src/components/dock/PaperBrokerTab.tsx#L188) và [`ui/src/chart/KLineViewport.tsx:890`](file:///Users/mizuhara/workspace/sumi/ui/src/chart/KLineViewport.tsx#L890) không nhân với `contract_multiplier` (`100_000`).
- **Khuyến nghị sửa:** Đặt `max_leverage: 1.0` cho `Vn30Futures` (để `initial_margin_pct = 17.0%` là ký quỹ thực) và áp dụng `× contract_multiplier` đồng nhất tại `PaperBrokerTab.tsx` lẫn `KLineViewport.tsx`.

---

#### `BUG-040` — Đa Biểu Đồ / Đa Khung Giờ (`1h` + `4h`) Hoàn Toàn Không Đồng Bộ Khi Bar Replay — Lộ 100% Nến Tương Lai & Bơm Nhầm Nến Khi Click Sang Ô 2
- **Test Case:** `TC-MTF-REPLAY-001`, `TC-MTF-REPLAY-002` | **Mức độ:** **P0 (Critical)** | **Phân hệ:** Multi-Timeframe Bar Replay (Cross-Subsystem)
- **Các bước tái hiện:**
  1. Bật bố cục `2 dọc`: Ô 1 (`pane-0`) = `HOSE:FPT` (hoặc `BINANCE:BTCUSDT`) khung `1h`, Ô 2 (`pane-1`) = cùng mã khung `4h`.
  2. Chọn Ô 1, bấm `Phát lại` và cắt nến về một thời điểm quá khứ `T`.
  3. Bước `+2` nến `1h` trên Ô 1, sau đó **click chuột sang Ô 2 (`4h`)** để vẽ đường hỗ trợ/kháng cự rồi tiếp tục bấm `+1` / `Phát`.
  4. Thử phương án thay thế dùng 1 ô: đang Replay ở `1h` rồi bấm chuyển khung sang `4h`.
- **Thực tế vs Kỳ vọng (TradingView Multi-Timeframe Replay):**
  - **Thực tế (4 tầng gãy đồng thời):**
    1. **Lộ 100% tương lai ở Ô 2 (`4h`):** Chốt chặn nhân quả `DEF-001` ở backend chỉ kiểm tra `rt.interval == iv` → Ô 2 khác khung giờ đi xuyên qua chốt chặn và **hiển thị trọn vẹn toàn bộ nến `4h` đến tận hôm nay**, trong khi trục Y Ô 2 vẫn hiện chữ `'REPLAY'` đánh lừa người dùng!
    2. **Kể cả 2 ô cùng mã + cùng khung (`1h` + `1h`) vẫn KHÔNG đồng bộ:** Frontend chỉ gọi `chart.resetData()` và `subscribeCallbackRef.current(bar)` trên ô có `isActive === true` → Ô 2 không bao giờ cắt nến hay tiến nến theo Ô 1.
    3. **Bơm nhầm nến làm hỏng dữ liệu Ô 2 khi click chuyển focus (Data Corruption):** Khi trader click sang Ô 2 (`4h` hoặc thậm chí mã khác!), `isActive` của Ô 2 bật lên `true` → `useEffect([replaySteppedBar, isActive])` lập tức **bơm cây nến `1h` vừa bước của Ô 1 vào cuối biểu đồ `4h` của Ô 2**, đồng thời gọi `refreshBroker()` theo Ô 2 làm lệch sổ lệnh; từ lúc đó trở đi bấm `+1`/`Phát` thì Ô 1 đứng im còn Ô 2 tiếp tục bị tiêm nhầm nến `1h`!
    4. **Đổi khung giờ trên 1 ô cũng bị hủy ngang phiên Replay:** `setActiveInterval` lập tức gọi `stopReplay()`, phá hủy điểm cắt và phiên thực hành đang chạy.
  - **Kỳ vọng:** Một phiên Bar Replay duy nhất dùng chuỗi nến khung nhỏ nhất (vd `1h`) làm nguồn sự thật, tự động cắt đồng bộ **mọi ô cùng ticker** về đúng mốc `cutoffTimestampMs` và **tổng hợp nhân quả nến khung lớn đang hình thành** (`4h`, `1D` chỉ phản ánh các cây `1h <= T`, không lộ H/L/C tương lai); click sang ô nào để vẽ cũng không phá vỡ đồng bộ hay bơm nhầm nến; đổi khung giờ trên 1 ô giữ nguyên mốc cắt `T`.
- **Nguyên nhân gốc (xác minh trực tiếp mã nguồn):**
  1. [`crates/sumi-desktop/src/commands/market.rs:531-535`](file:///Users/mizuhara/workspace/sumi/crates/sumi-desktop/src/commands/market.rs#L531-L535): điều kiện `&& rt.interval == iv` bỏ rơi mọi ô khác khung giờ.
  2. [`ui/src/chart/KLineViewport.tsx:1019-1035`](file:///Users/mizuhara/workspace/sumi/ui/src/chart/KLineViewport.tsx#L1019-L1035): cả hai `useEffect` khóa vào `!isActive` và đặt `isActive` trong dependency array (gây bơm nhầm `replaySteppedBar` khi đổi focus sang ô khác); dòng [`437`](file:///Users/mizuhara/workspace/sumi/ui/src/chart/KLineViewport.tsx#L437) gắn nhãn `'REPLAY'` toàn cục lên cả ô đang chiếu tương lai.
  3. [`ui/src/store/workspaceStore.ts:488-492, 516-518`](file:///Users/mizuhara/workspace/sumi/ui/src/store/workspaceStore.ts#L488-L518): `setActiveInterval` gọi `stopReplay()`, `setActiveSlotId` gọi `refreshBroker()` theo ô mới.
  4. **Sự mỉa mai kiến trúc:** [`crates/sumi-core/src/replay/controller.rs:475-509`](file:///Users/mizuhara/workspace/sumi/crates/sumi-core/src/replay/controller.rs#L475-L509) đã xây sẵn `ReplayController::materialize_candles(base_candles, base_tf, active_tf)` cho đúng bài toán Multi-Timeframe zero-lookahead, nhưng [`crates/sumi-desktop/src/commands/replay.rs`](file:///Users/mizuhara/workspace/sumi/crates/sumi-desktop/src/commands/replay.rs) chưa hề nối hàm này vào tầng IPC!
- **Khuyến nghị sửa:**
  - Nâng `ReplayRuntime` để luôn giữ chuỗi gốc ở khung nhỏ nhất trong các ô đang mở cùng `ticker` (vd `1h` khi mở `1h` + `4h`).
  - Trong `market.rs::get_bars`, mọi yêu cầu trùng `rt.ticker` (bất kể `iv >= rt.interval`) phải đi qua `rt.controller.materialize_candles(&rt.base_candles, base_tf, target_tf)` để trả chuỗi nhân quả kèm nến đang hình thành.
  - Ở `KLineViewport.tsx`, lọc theo `pane.ticker === replay.ticker` (thay vì `!isActive`), xóa `isActive` khỏi dependency của `replayResetNonce` / `replaySteppedBar`, và khi `replay_step` chạy thì trả về map `steppedBarsByInterval` (hoặc kích `resetData`/update riêng cho từng `interval` khớp `pane.interval`).
  - Cho phép `setActiveInterval` đổi khung giờ ngay trong Replay mà không gọi `stopReplay()` nếu khung mới là bội số nhân quả (`>= base_tf`).

---

### Mức Độ P1 — Cao (Sai Lệch Nghiệp Vụ Lớn / Mất Dữ Liệu / Chức Năng Chính Hỏng)

#### `BUG-003` — Đứt Gãy & Nhảy Bậc Giá Giả (`+4%..+8.94%`) Tại Điểm Nối `CafeF Adjusted` ↔ `TradingView Splits`
- **Test Case:** `TC-DATA-004` | **Mức độ:** **P1** | **Phân hệ:** Data / Storage Splice
- **Các bước tái hiện:** Mở `HOSE:SSI`, `HOSE:VNM`, `HOSE:TCB` khung `1D`, quan sát phiên chuyển tiếp `2025-09-09` (`cafef_adjusted`) → `2025-09-10` (`tradingview`).
- **Thực tế vs Kỳ vọng:** Phát hiện **24** điểm nối `CafeF ↔ TV` trên HOSE, trong đó **11 mã có bước nhảy giả** (`>3.5%` gap mở cửa hoặc vượt biên độ HOSE `±7%`): `HOSE:SSI` nhảy **`+8.94%` open / `+8.05%` close** (`29,821.1 → 32,487.05`), `HOSE:VNM` nhảy **`+8.48%` / `+7.22%`** (`55,772.7 → 60,500`), `HOSE:TCB` nhảy **`+6.84%` / `+6.16%`**. Điều này làm sai lệch toàn bộ chỉ báo kỹ thuật và kết quả Backtest đi qua vùng nối.
- **Bằng chứng:** Log SQLite `TC-DATA-004` & ảnh [`bug_cafef_tv_splice_ssi.png`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence/bug_cafef_tv_splice_ssi.png).
- **Nguyên nhân gốc:** [`crates/sumi-data/src/cafef_importer.rs`](file:///Users/mizuhara/workspace/sumi/crates/sumi-data/src/cafef_importer.rs) ghi dữ liệu điều chỉnh **cổ tức tiền mặt + chia tách** (`CafeF.SolieuGD.Adjusted.csv`) vào cùng khóa `adjustment = 'splits'` với dữ liệu **chỉ điều chỉnh chia tách** (`Adjustment::Splits`) từ `TradingViewFeed`, không có bước back-adjust hệ số nối (`ratio_splice`) tại ngày giao nhau.
- **Khuyến nghị sửa:** Phân tách `adjustment = 'dividends_and_splits'` riêng hoặc tính hệ số tỷ lệ `TV_close / CafeF_close` tại ngày chồng lấn gần nhất để chuẩn hóa chuỗi CafeF liên tục trước khi ghép với TradingView.

---

#### `BUG-004` — Đồng Bộ Con Trỏ Chữ Thập Đa Biểu Đồ (`CrosshairBus`) Tê Liệt 100%
- **Test Case:** `TC-GRID-006` | **Mức độ:** **P1** | **Phân hệ:** Multi-Chart / Canvas Sync
- **Các bước tái hiện:**
  1. Chọn bố cục `2 dọc` (mặc định `Đồng bộ Con trỏ chữ thập = ON`), `pane-0` = `HOSE:FPT 1D`, `pane-1` = `HOSE:FPT 4H`.
  2. Di chuột trực tiếp lên cây nến `2026-11-17` của `pane-0` (ngày có mặt rõ trên cả hai ô).
- **Thực tế vs Kỳ vọng:** Không có bất kỳ đường crosshair hay cập nhật legend nào xuất hiện trên `pane-1` (giá trị vẫn đứng im ở nến cuối cùng `O 44,900 C 45,000`; hash ảnh Canvas `b0 === b1 === b2`). Kỳ vọng: `pane-1` gióng chính xác đường chữ thập dọc tới vùng thời gian `2026-11-17`.
- **Bằng chứng:** Ảnh [`14_crosshair_sync_2v.png`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence/14_crosshair_sync_2v.png) & log kiểm chứng trực tiếp `klinecharts@10.1.1`.
- **Nguyên nhân gốc:** Trong [`ui/node_modules/klinecharts/dist/index.esm.js:14503`](file:///Users/mizuhara/workspace/sumi/ui/node_modules/klinecharts/dist/index.esm.js#L14503), `StoreImp.prototype.setCrosshair` gọi `this.executeAction('onCrosshairChange', crosshair)` với tham số thô `{ x, y, paneId }` (**KHÔNG có `kLineData`**). Trong khi đó [`ui/src/chart/KLineViewport.tsx:564-568`](file:///Users/mizuhara/workspace/sumi/ui/src/chart/KLineViewport.tsx#L564-L568) đọc `payload?.kLineData?.timestamp ?? null` → **luôn bằng `null`** trên mọi lần rê chuột, khiến `crosshairBus.publish` luôn phát lệnh xóa crosshair!
- **Khuyến nghị sửa:** Trong `KLineViewport.tsx:564-568`, chuyển đổi `payload.x` sang `dataIndex`/`timestamp` bằng `chart.convertFromPixel({ x: payload.x }, { paneId: 'candle_pane' })` rồi tra cứu `chart.getDataList()[dataIndex]?.timestamp` trước khi gọi `crosshairBus.publish`.

---

#### `BUG-005` — Cuộn Xem Lịch Sử Quá Khứ (`mode: 'forward'`) Ghi Đè Sai Dải Trần / Tham Chiếu / Sàn Hiện Tại
- **Test Case:** `TC-CHART-009` | **Mức độ:** **P1** | **Phân hệ:** Chart / VN Price Bands
- **Các bước tái hiện:**
  1. Mở `HOSE:FPT` khung `1D` (giá hiện tại `45,000`, `Trần 48,850 / TC 45,700 / Sàn 42,550`).
  2. Cuộn sâu sang quá khứ để kích hoạt lazy-load trang cũ (`mode: 'forward'`), sau đó bấm `Alt+R` quay lại hiện tại.
- **Thực tế vs Kỳ vọng:** Thanh badge góc trên phải bị ghi đè thành **`Trần 105,700 TC 98,790.9 Sàn 91,900`** (lấy từ nến lịch sử năm 2024!), trái ngược hoàn toàn với thẻ Chi tiết góc dưới phải (`48,850 / 45,700 / 42,550`), và 3 đường `T/TC/S` trên biểu đồ biến mất khỏi vùng giá `44,000–72,000`.
- **Bằng chứng:** Ảnh [`03_light_mode.png`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence/03_light_mode.png) chụp ngay sau `TC-CHART-009`.
- **Nguyên nhân gốc:** [`ui/src/chart/KLineViewport.tsx:469`](file:///Users/mizuhara/workspace/sumi/ui/src/chart/KLineViewport.tsx#L469) gọi `setVnBand(page.vnBand)` vô điều kiện kể cả khi `queryMode === 'forward'`, trong khi backend tính `vnBand` từ nến kế cuối của trang trả về.
- **Khuyến nghị sửa:** Chỉ gọi `setVnBand(page.vnBand)` và `setDataSourceBadge(...)` khi `queryMode === 'init'` (hoặc trả `vnBand = None` từ backend khi `mode == "forward"`).

---

#### `BUG-006` — Bật Bar Replay Xóa Trắng & Ghi Đè Toàn Bộ Vị Thế Paper Broker Live Trong SQLite
- **Test Case:** `TC-REPLAY-007` | **Mức độ:** **P1** | **Phân hệ:** Bar Replay / Paper Broker
- **Các bước tái hiện:**
  1. Ở chế độ Live trên `HOSE:FPT`, đặt mua `1.000` cổ phiếu `MP` (có vị thế mở đang lưu trong `kv_store: paper_broker:HOSE:FPT`).
  2. Bấm `Phát lại` (Bar Replay) rồi bấm `Thoát`.
  3. Kiểm tra lại trạng thái `broker_get_state("HOSE:FPT", "1D")`.
- **Thực tế vs Kỳ vọng:** Toàn bộ vị thế live `1.000` cổ phiếu và lịch sử lệnh bị **xóa sạch vĩnh viễn** (tiền mặt reset về `500.000.000`, `openPosition = null`). Kỳ vọng: Phiên Replay phải dùng sandbox broker tách biệt, không bao giờ ghi đè sổ lệnh live.
- **Bằng chứng:** Log `TC-REPLAY-007` (`beforeReplay openPosition=true qty=1000 -> afterReplay livePosSurvived=false, cash=500000000`).
- **Nguyên nhân gốc:** [`crates/sumi-desktop/src/commands/replay.rs:115-117`](file:///Users/mizuhara/workspace/sumi/crates/sumi-desktop/src/commands/replay.rs#L115-L117) gọi `broker.session.reset_at(&candles_clone, cut_idx)` rồi lập tức gọi `persist_broker_snapshot(&state, &broker)` ghi đè khóa `paper_broker:{ticker}` trong SQLite.
- **Khuyến nghị sửa:** Tách biệt `replay_broker` khỏi `live_broker` (hoặc lưu khóa `replay_broker:{ticker}` riêng, không ghi đè `paper_broker:{ticker}` khi `replayLinked == true`).

---

#### `BUG-007` — Chuyển Khung Giờ (`1D → 1h`) Làm Hỏng & Mất Toàn Bộ Lịch Sử Paper Broker Và Nhật Ký Giao Dịch
- **Test Case:** `TC-BROKER-011` | **Mức độ:** **P1** | **Phân hệ:** Paper Broker Persistence
- **Các bước tái hiện:**
  1. Trên `HOSE:FPT 1D`, mở và đóng 1 vị thế `1.000` cổ phiếu (equity `499,802,009`), lưu ghi chú Nhật ký `"Breakout / FOMO"`.
  2. Chuyển khung giờ sang `1h` (`broker_get_state("HOSE:FPT", "1h")`).
  3. Chuyển về `1D` hoặc khởi động lại ứng dụng.
- **Thực tế vs Kỳ vọng:** Ngay khi gọi `1h`, toàn bộ lệnh đóng & lãi lỗ biến mất (`trades = 0`, `equity = 500,000,000`) do replay command log với `base_index = 1499` (của khung `1D`) vượt quá số nến khung `1h` và lưu đè lại snapshot trắng, làm mất luôn cả liên kết ghi chú Nhật ký sau restart!
- **Bằng chứng:** Log `TC-BROKER-011` (`1D: trades=1, eq=499802009` vs `1h: trades=0, eq=500000000`; `journalPersisted=false`).
- **Nguyên nhân gốc:** [`crates/sumi-desktop/src/commands/broker.rs:190`](file:///Users/mizuhara/workspace/sumi/crates/sumi-desktop/src/commands/broker.rs#L190) khóa bản ghi `kv_store` theo `format!("paper_broker:{ticker}")` (bỏ qua `interval`), trong khi `TradingSession` phát lại lệnh theo `base_index` gắn với chuỗi nến của khung giờ tạo lệnh.
- **Khuyến nghị sửa:** Hoặc lưu khóa theo `(ticker, interval)`, hoặc ánh xạ lệnh bằng `timestamp` thay vì `base_index` tuyệt đối và không bao giờ ghi đè snapshot khi `base_candles` ngắn hơn lệnh đã ghi.

---

#### `BUG-008` — Đặt Lệnh Nhanh Từ Menu Chuột Phải (`ChartContextMenu`) Hỏng Nặng Trên `HNX:VN30F1!` & `HNX`/`UPCOM`
- **Test Case:** `TC-BROKER-009` | **Mức độ:** **P1** | **Phân hệ:** Paper Broker / Context Menu
- **Các bước tái hiện:** Mở `HNX:VN30F1!`, chuột phải tại mức `1,866.3` và chọn `"Đặt lệnh MUA (LO) @ 1,866.3"`.
- **Thực tế vs Kỳ vọng:** Payload gửi xuống Rust là `{"ticker":"HNX:VN30F1!","vnOrderType":"LO","quantity":1000,"price":1870}` — làm tròn giá theo bước giá HOSE `<10k` (`10` điểm: `1866.3 → 1870`!) và đặt **`1.000` hợp đồng** (giá trị 187 tỷ VND, lập tức bị từ chối!). Tương tự, mã `HNX`/`UPCOM` giá `<50k` bị làm tròn bước `50 VND` thay vì `100 VND`.
- **Bằng chứng:** Log chặn request `TC-BROKER-009_EXACT` và ảnh [`bug_vn30f_broker_100k_pnl_and_margin.png`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence/bug_vn30f_broker_100k_pnl_and_margin.png).
- **Nguyên nhân gốc:** [`ui/src/store/workspaceStore.ts:674-678`](file:///Users/mizuhara/workspace/sumi/ui/src/store/workspaceStore.ts#L674-L678) áp dụng cứng bậc bước giá HOSE (`10/50/100`) và `qty = 1000` cho mọi mã có tiền tố `HOSE:|HNX:|UPCOM:`.
- **Khuyến nghị sửa:** Phân tách quy tắc cho `VN30F` (`tick = 0.1`, `qty = 1`), `HNX`/`UPCOM` (`tick = 100`, `qty = 100`), `HOSE` (`10/50/100`, `qty = 100`).

---

#### `BUG-009` — Paper Broker Chấp Nhận Giá Đặt Sai Bước Giá VN & Đồng Nhất `ATO`/`ATC` Với `MP`
- **Test Case:** `TC-BROKER-003`, `TC-BROKER-004` | **Mức độ:** **P1** | **Phân hệ:** Paper Broker Validation
- **Các bước tái hiện:**
  1. Gửi lệnh Mua `LO` `100 cp` `HOSE:FPT` ở giá `42,583 VND` (lẻ `33 VND` trên bậc bước `50 VND`) và `HNX:SHS` ở giá `11,950 VND` (lẻ `50 VND` trên bậc bước `100 VND`).
  2. Gửi lệnh Mua `ATO` và `ATC` `100 cp` trên `HOSE:FPT`.
- **Thực tế vs Kỳ vọng:** Cả hai lệnh sai bước giá (`+33 VND` HOSE, `+50 VND` HNX) đều được **chấp nhận vào sổ lệnh chờ**! Lệnh `ATO` và `ATC` khớp tức thì ở `45,009` (`close + slippage`) với `kind = "Market"`, y hệt `MP`.
- **Bằng chứng:** Log `TC-BROKER-003` (`offTickHOSE blocked=false, offTickHNX blocked=false`) & `TC-BROKER-004`.
- **Nguyên nhân gốc:** [`crates/sumi-desktop/src/commands/broker.rs:510-540`](file:///Users/mizuhara/workspace/sumi/crates/sumi-desktop/src/commands/broker.rs#L510-L540) chỉ kiểm tra `floor <= price <= ceiling` mà **bỏ qua hoàn toàn** `tick_size_at_price`; dòng [`469`](file:///Users/mizuhara/workspace/sumi/crates/sumi-desktop/src/commands/broker.rs#L469) ánh xạ `"ATO" | "ATC" | "MP"` cùng về `OrderKind::Market`; dòng [`325, 460-464`](file:///Users/mizuhara/workspace/sumi/crates/sumi-desktop/src/commands/broker.rs#L325) tính giá tham chiếu ngày từ `base_candles[idx - 1].close` ngay cả trên khung intraday (`15m`/`1h`).
- **Khuyến nghị sửa:** Gọi kiểm tra chia hết cho `tick_size_at_price` trong `validate_vn_price_band_and_tick`, lấy tham chiếu từ phiên D1 trước cho khung intraday, và phân biệt `ATO` (khớp mở cửa nến tiếp theo) / `ATC` (khớp đóng cửa phiên).

---

#### `BUG-010` — Sai Lệch Toán Học Đúng `2.000×` Trên `MACD Histogram` Giữa Biểu Đồ Và Stock Screener / TradingView
- **Test Case:** `TC-IND-008`, `TC-QUANT-009` | **Mức độ:** **P1** | **Phân hệ:** Indicators / Screener Parity
- **Các bước tái hiện:** So sánh giá trị `MACD(12, 26, 9)` Histogram nến cuối của `HOSE:FPT`, `HOSE:HPG`, `HNX:SHS` trên Biểu đồ với bảng `Stock Screener`.
- **Thực tế vs Kỳ vọng:**
  - `HOSE:FPT`: Biểu đồ **`-637.39`** vs Screener **`-318.69`** (tỷ lệ **`2.000×`**)
  - `HOSE:HPG`: Biểu đồ **`-183.40`** vs Screener **`-91.70`** (tỷ lệ **`2.000×`**)
  - `HNX:SHS`: Biểu đồ **`+30.64`** vs Screener **`+15.32`** (tỷ lệ **`2.000×`**)
  - Ngoài ra Screener chỉ lấy `220` nến (vs `600` nến trên biểu đồ) gây trôi nhẹ `EMA`/`RSI`, và bên trong `sumi-core` cũng lệch: `divergence.stoch_*` dùng `fast_k` chưa làm mượt (`scanner/catalog.rs:4499`), `flow.cvd_ohlcv_impulse` ép `Cumulative` thay vì `Session` (`catalog.rs:2708`), ngưỡng BB Squeeze là `10%` ở Screener (`engine.rs:302`) vs `12%` ở Backtest (`strategy.rs:234`).
- **Bằng chứng:** Log `TC-IND-008` & `TC-QUANT-009`.
- **Nguyên nhân gốc:** Chỉ báo `MACD` tích hợp sẵn của `klinecharts@10.1.1` áp dụng quy ước Tongdaxin Trung Quốc `macd = (dif - dea) * 2`, trong khi `sumi-core` và TradingView dùng chuẩn quốc tế `hist = dif - dea`.
- **Khuyến nghị sửa:** Ghi đè đăng ký chỉ báo `MACD` chuẩn TradingView (`hist = dif - dea`, không nhân 2) trong `customIndicators.ts` để đồng bộ tuyệt đối Chart – Screener – Backtest.

---

#### `BUG-011` — Strategy Tester Ép Quy Tắc Cổ Phiếu Cơ Sở VN (`Lô 100`, `Thuế Bán 0.1%`, `T+2`, `Long Only`) Lên Phái Sinh `VN30F1!`
- **Test Case:** `TC-QUANT-002` | **Mức độ:** **P1** | **Phân hệ:** Strategy Tester UI
- **Các bước tái hiện:** Chọn mã `HNX:VN30F1!`, mở tab `Strategy Tester`, bấm `▷ Backtest`.
- **Thực tế vs Kỳ vọng:** UI gửi xuống backend: `{"ticker":"HNX:VN30F1!","direction":"long","commissionBps":15,"taxSellBps":10,"lotStep":100,"minHoldingBars":2}`, thu **`10,256,375 VND`** phí/thuế sai lệch và khóa `T+2`, `Long Only`, `Lô 100 HĐ` (~18 tỷ VND/lô!). Kỳ vọng: tự động nhận diện `ExchangeVenue::Vn30Futures` → `lotStep = 1`, `taxSellBps = 0`, `commissionBps = 2..4`, `minHoldingBars = 0`, cho phép `Long & Short`.
- **Bằng chứng:** Ảnh [`12_strategy_tester_backtest.png`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence/12_strategy_tester_backtest.png) và payload chặn được trong `TC-QUANT-002_003_EXACT`.
- **Nguyên nhân gốc:** [`ui/src/components/dock/StrategyTesterTab.tsx:126-160`](file:///Users/mizuhara/workspace/sumi/ui/src/components/dock/StrategyTesterTab.tsx#L126-L160) không điều chỉnh mặc định `lotStep`, `taxSellBps`, `commissionBps`, `minHoldingBars`, `direction` theo loại tài sản.
- **Khuyến nghị sửa:** Tự động cấu hình preset phí/thuế/lô/T+ theo `ticker` khi đổi mã tương tự cách `commands/broker.rs` phân loại `ExchangeVenue`.

---

#### `BUG-012` — Rò Rỉ `bar_streams` (1 → 4) & Tauri Channel Callbacks (6 → 84) Khi Bật/Tắt Lưới Đa Biểu Đồ
- **Test Case:** `TC-PERF-008` | **Mức độ:** **P1** | **Phân hệ:** Streaming / Memory & Handle Management
- **Các bước tái hiện:** Chạy Soak Test 110 thao tác (50 đổi mã + 20 lần chuyển đổi lưới `1 ô ↔ 4 ô` + 20 chu kỳ chỉ báo + 20 chu kỳ hình vẽ), quay về lưới `1 ô`, ép GC và đọc `/qa/telemetry`.
- **Thực tế vs Kỳ vọng:** Dù chỉ còn 1 biểu đồ hiển thị, `AppState.bar_streams` vẫn giữ **4 stream đang chạy** (`stream_pane-0..3`), `Tauri Channel callbacks` tăng vọt từ **`6 → 84` (+78 callback không bao giờ giải phóng)**, `DOM Nodes +1,250`, `JS Listeners +434`, `Rust RSS +14.39 MB`. Kỳ vọng: `barStreamsCount <= 1` và callbacks không tích lũy tuyến tính theo số lần đổi mã/lưới.
- **Bằng chứng:** Log `TC-PERF-008` trong `results_suite4.json`.
- **Nguyên nhân gốc:** Race condition bất đồng bộ giữa `void api.subscribeBars(streamId)` ([`KLineViewport.tsx:541`](file:///Users/mizuhara/workspace/sumi/ui/src/chart/KLineViewport.tsx#L541)) và `void api.unsubscribeBars(streamId)` ([`KLineViewport.tsx:640`](file:///Users/mizuhara/workspace/sumi/ui/src/chart/KLineViewport.tsx#L640)) khi unmount nhanh: lệnh `unsubscribe_bars` hoàn tất trước khi `subscribe_bars` chạy tới `bar_streams.insert(...)` ([`commands/market.rs:800`](file:///Users/mizuhara/workspace/sumi/crates/sumi-desktop/src/commands/market.rs#L800)), bỏ lại task WebSocket mồ côi. Ngoài ra `api.subscribeBars` tạo `new Channel()` nhưng không bao giờ hủy đăng ký callback phía frontend khi `unsubscribeBars`.
- **Khuyến nghị sửa:** Dùng monotonic generation counter cho mỗi `stream_id` trong `AppState` (hủy bỏ `subscribe_bars` nếu generation đã tăng bởi `unsubscribe_bars`) và gọi `unregisterCallback` / đóng Channel khi unmount.

---

#### `BUG-013` — Lõi `TradingSession`: Gap-Open Vượt SL Tự Xóa Stop-Loss & Lệnh Stop Trong Nến Có Thể Dính TP Trước SL
- **Test Case:** Audit lõi `sumi-core` (`trading/portfolio.rs`) | **Mức độ:** **P1** | **Phân hệ:** Quantitative Core (`sumi-core`)
- **Thực tế vs Kỳ vọng:**
  1. Khi lệnh `Buy Limit` (vd giá `100`, `SL = 95`, `TP = 110`) mở cửa gap xuống `candle.open = 90`, [`crates/sumi-core/src/trading/portfolio.rs:1459-1460, 2073-2107`](file:///Users/mizuhara/workspace/sumi/crates/sumi-core/src/trading/portfolio.rs#L1459-L1460) gọi `sanitize_brackets_for_position` thấy `sl (95) >= exec (90.018)` nên **âm thầm xóa `stop_loss = None` & `had_initial_sl_at_open = false`** trước khi chạy kiểm tra thoát SL ở dòng `1689` — để vị thế hoàn toàn không có cắt lỗ!
  2. Biến `dual_touch_side` (ép thứ tự bi quan SL trước TP) tại [`portfolio.rs:1700-1723`](file:///Users/mizuhara/workspace/sumi/crates/sumi-core/src/trading/portfolio.rs#L1700-L1723) chỉ tính sau Pha A (`open`), bỏ sót vị thế mở mới trong Pha B (nến giảm chạm cả `H` và `L`: `Buy Stop` khớp ở `O → H` sẽ ăn `TP` ở `H` trước khi chạm `SL` ở `L`).
  3. [`portfolio.rs:1184`](file:///Users/mizuhara/workspace/sumi/crates/sumi-core/src/trading/portfolio.rs#L1184) trong `apply_modify_order_price_internal` dùng `self.config.contract_multiplier.max(1.0)` thay vì `effective_contract_multiplier()` và không tái kiểm sức mua sau khi nâng giá lệnh chờ.
- **Khuyến nghị sửa:** Nếu giá khớp đã xuyên qua `stop_loss` lúc mở cửa gap, phải đóng vị thế SL ngay tại giá mở cửa (không xóa SL); tính lại `dual_touch_side` ngay sau khi Pha B mở vị thế mới.

---

#### `BUG-014` — Vi Phạm Bất Biến Nhân Quả (`DEF-001` Lookahead) Trong `sumi-core` `VWAP`
- **Test Case:** `TC-IND-012` | **Mức độ:** **P1** | **Phân hệ:** Quantitative Core (`sumi-core`)
- **Thực tế vs Kỳ vọng:** Tại [`crates/sumi-core/src/indicators/vwap.rs:303-305, 375-377`](file:///Users/mizuhara/workspace/sumi/crates/sumi-core/src/indicators/vwap.rs#L303-L305), cờ `let use_equal_weight = !candles.iter().any(|c| c.volume.is_finite() && c.volume > 0.0)` quét **toàn bộ chuỗi kể cả tương lai**. Trong Bar Replay, khi các nến `0..k` có `volume = 0`, `VWAP[0..k]` trả về `Some(tp)`; ngay khi bước sang nến `k+1` có `volume > 0`, toàn bộ `VWAP[0..k]` quá khứ bị đổi hồi tố thành `None`!
- **Khuyến nghị sửa:** Xác định trọng số hoàn toàn nhân quả theo từng nến tích lũy `cum_vol` tính đến `t` (tuyệt đối không dùng `.iter().any()` trên lát tương lai).

---

#### `BUG-015` — Trôi Giá `Heikin-Ashi` Theo Hàm Mũ Khi Nhận Tick Live & Lệch Chuỗi Khi Lazy-Load Quá Khứ
- **Test Case:** `TC-CHART-002` | **Mức độ:** **P1** | **Phân hệ:** Chart / Heikin-Ashi
- **Thực tế vs Kỳ vọng:**
  1. Khi nhận tick cập nhật trong cùng cây nến đang mở (`isUpdate = true`), [`ui/src/chart/KLineViewport.tsx:547-553`](file:///Users/mizuhara/workspace/sumi/ui/src/chart/KLineViewport.tsx#L547-L553) gọi `toHeikinAshi([bar], lastHaBarRef.current)` rồi lập tức **ghi đè `lastHaBarRef.current = { open: ha.open, close: ha.close }`** trên mỗi tick! Điều này làm `HA_Open` của cây nến hiện tại bị cập nhật đệ quy liên tục theo từng tick thay vì giữ cố định bằng `(prevClosedHaOpen + prevClosedHaClose) / 2`.
  2. Khi cuộn quá khứ (`queryMode === 'forward'`), [`KLineViewport.tsx:481-485`](file:///Users/mizuhara/workspace/sumi/ui/src/chart/KLineViewport.tsx#L481-L485) tính lại `toHeikinAshi(page.bars)` độc lập và ghi đè `lastHaBarRef.current` bằng đuôi của trang lịch sử cũ!
- **Khuyến nghị sửa:** Lưu hai mốc tách biệt: `prevClosedHaBarRef` (chỉ cập nhật khi `bar.timestamp > currentHaTs`) và `currentRawBarTsRef`, đồng thời tính lại toàn bộ chuỗi HA hợp nhất khi nhận trang `forward`.

---

#### `BUG-041` — Luyện Tập Mù (Blind Replay) Bị Lộ Sạch Giá Tương Lai Trên Watchlist & Thẻ Chi Tiết Mã (`RightWidgetBar`)
- **Test Case:** `TC-MTF-REPLAY-003` | **Mức độ:** **P1** | **Phân hệ:** Bar Replay / Blind Practice UX
- **Các bước tái hiện:** Mở `HOSE:FPT`, bật `Phát lại` cắt về ngày `26/08/2026` (giá đóng cửa `66,000`) trong khi mở sẵn cột phải `RightWidgetBar`.
- **Thực tế vs Kỳ vọng:** Ngay bên cạnh biểu đồ đang cắt ở giá `66,000`, cột `Danh mục (Watchlist)` vẫn hiện rành rành **`FPT 45,000 (-1.53%)`** và thẻ `Chi tiết mã` hiện **`45,000` (`Trần 48,850 / TC 45,700 / Sàn 42,550`)** — tiết lộ thẳng cú sập `-32%` trong tương lai (cộng hưởng cùng `BUG-034` khi ô `Giá LO` vẫn điền `45000`). Kỳ vọng: Khi bật Bar Replay, chế độ Blind Mode phải ẩn/che giá hiện tại của mã đang luyện tập ở Watchlist & Chi tiết mã (hoặc đồng bộ hoàn toàn theo nến `cutoffTimestampMs`).
- **Bằng chứng:** Ảnh chụp thực tế Tầng B [`11_replay_active.png`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence/11_replay_active.png).
- **Nguyên nhân gốc:** [`ui/src/components/RightWidgetBar.tsx:20-43`](file:///Users/mizuhara/workspace/sumi/ui/src/components/RightWidgetBar.tsx#L20-L43) hoàn toàn không đọc trạng thái `replay` từ `useWorkspaceStore`, luôn hiển thị `quotesMap[activePane.ticker]` trực tiếp từ luồng giá hiện tại.
- **Khuyến nghị sửa:** Khi `replay?.active && replay.ticker === ticker`, thay thế quote/chi tiết mã ở `RightWidgetBar` bằng OHLCV + dải `T/TC/S` tính tại mốc `replay.cutoffTimestampMs` (hoặc cung cấp công tắc "Ẩn giá tương lai / Blind Mode" mặc định BẬT).

---

#### `BUG-042` — Mũi Tên Lệnh Backtest (`backtestOverlay`) & Hình Vẽ Hiện Tại Không Bị Lọc Ẩn Theo Mốc Thời Gian Cắt Khi Bật Bar Replay
- **Test Case:** `TC-MTF-REPLAY-004` | **Mức độ:** **P1** | **Phân hệ:** Bar Replay / Backtest & Drawings Causality
- **Các bước tái hiện:** Chạy `Strategy Tester` trên `HOSE:FPT` (hiển thị mũi tên `MUA #` / `TP` / `SL` đến hiện tại) hoặc vẽ hình phân tích ở hiện tại, sau đó bật `Bar Replay` cắt lùi về 6 tháng trước.
- **Thực tế vs Kỳ vọng:** Toàn bộ mũi tên lệnh `backtestOverlay` trong tương lai và hình vẽ được tạo ở mốc thời gian tương lai vẫn nằm nguyên trên biểu đồ khi cắt Replay, tiết lộ trước điểm đảo chiều / điểm chốt lời tương lai. Kỳ vọng: Mọi overlay có timestamp `> replay.cutoffTimestampMs` phải tự động ẩn đi trong suốt phiên Replay.
- **Nguyên nhân gốc:** [`ui/src/chart/KLineViewport.tsx:955-1016`](file:///Users/mizuhara/workspace/sumi/ui/src/chart/KLineViewport.tsx#L955-L1016) (plot `backtestOverlay`) không kiểm tra `replay?.active` hay `replay.cutoffTimestampMs`; [`KLineViewport.tsx:790-806`](file:///Users/mizuhara/workspace/sumi/ui/src/chart/KLineViewport.tsx#L790-L806) nạp toàn bộ `drawings` của `ticker` mà không lọc theo mốc thời gian Replay.
- **Khuyến nghị sửa:** Lọc `tr.entryTimestampMs <= cutoff` và `tr.exitTimestampMs <= cutoff` khi `replay?.active` (hoặc ẩn hẳn `backtestOverlay` trong lúc luyện tập Replay); bổ sung tùy chọn cô lập lớp hình vẽ Replay (hoặc ẩn hình vẽ có điểm neo `> cutoff`).

---

### Mức Độ P2 — Trung Bình (Lỗi Chức Năng Quan Trọng / Tính Chính Xác / Tương Thích macOS)

#### `BUG-016` — Cổ Phiếu Mỹ (`NASDAQ:AAPL`) Hiển Thị 4 Chữ Số Thập Phân (`$225.4300`) Thay Vì 2
- **Test Case:** `TC-CHART-003` | **Mức độ:** **P2** | **Phân hệ:** Data Precision (`sumi-data`)
- **Thực tế vs Kỳ vọng:** `resolve_symbol("NASDAQ:AAPL")` trả về `price_precision: 4` (từ `variable_tick_size: "0.0001 1 0.01"`). Cổ phiếu Mỹ giá `>= $1` trên TradingView luôn hiển thị 2 số lẻ (`$225.43`).
- **Bằng chứng:** Log `AAPL_AND_USDVND_PRECISION` (`variable_tick_size: "0.0001 1 0.01", price_precision: 4`).
- **Nguyên nhân gốc:** [`crates/sumi-data/src/types.rs:924-934`](file:///Users/mizuhara/workspace/sumi/crates/sumi-data/src/types.rs#L924-L934) (`compute_price_precision`) lấy `max_tier_decimals` trên mọi bậc của `variable_tick_table`, bao gồm cả bậc penny stock `< $1` (`0.0001`).
- **Khuyến nghị sửa:** Tính `price_precision` dựa trên bậc áp dụng tại `last_price` (hoặc bậc trên cùng `upper_bound == None` khi chưa có giá).

---

#### `BUG-017` — Va Chạm ID Cảnh Báo Giá (`alert_{now_ms}`) Mất 4/5 Cảnh Báo & Không Đánh Giá Ngoài Watchlist Đang Mở / Bar Replay
- **Test Case:** `TC-WIDGET-005`, `TC-WIDGET-007` | **Mức độ:** **P2** | **Phân hệ:** Price Alerts
- **Thực tế vs Kỳ vọng:**
  1. Gửi 5 lệnh `create_alert` song song: tất cả 5 lệnh sinh **trùng 1 ID duy nhất** `"alert_1795356898226"` → **4/5 cảnh báo bị ghi đè mất trắng** (`1/5` tồn tại trong SQLite).
  2. Cảnh báo chỉ được gọi `checkPriceAlerts` bên trong `applyLiveQuote` ([`ui/src/store/workspaceStore.ts:796`](file:///Users/mizuhara/workspace/sumi/ui/src/store/workspaceStore.ts#L796)) vốn chỉ nhận quote của **Watchlist đang chọn**. Cảnh báo đặt cho mã nằm ngoài Watchlist đang mở, từ luồng nến biểu đồ, hoặc trong **Bar Replay** (`status` vẫn là `'armed'` dù giá cắt qua!) **không bao giờ kích hoạt**.
- **Bằng chứng:** Log `TC-WIDGET-005` (`returnedIds` trùng 5/5, `SQLite rows=1/5`) & `TC-WIDGET-007` (`dbRepAlert=armed`).
- **Nguyên nhân gốc:** [`crates/sumi-desktop/src/commands/workspace.rs:217`](file:///Users/mizuhara/workspace/sumi/crates/sumi-desktop/src/commands/workspace.rs#L217) sinh ID bằng `format!("alert_{now_ms}")` không có bộ đếm atomic/UUID; kiến trúc không có dịch vụ theo dõi giá riêng cho các mã đang có alert `armed`.
- **Khuyến nghị sửa:** Dùng `AtomicU64` + timestamp hoặc `UUID v4` cho Alert ID; tự động hợp nhất danh sách mã có alert `armed` vào `subscribe_quotes` và đánh giá cảnh báo cả trên `subscribe_bars` lẫn `replay_step`.

---

#### `BUG-018` — Tất Cả 5 Chỉ Báo Custom Sumi Quant Phớt Lờ Chỉnh Sửa Màu Sắc & Độ Dày Từ `IndicatorSettingsModal`
- **Test Case:** `TC-IND-005` | **Mức độ:** **P2** | **Phân hệ:** Custom Indicators
- **Thực tế vs Kỳ vọng:** Đổi màu đường 1 của `ICHIMOKU` sang `#ff00ff` qua `IndicatorSettingsModal` và bấm `Áp dụng`: lưu xuống SQLite thành công nhưng trên Canvas **0 pixel màu `#ff00ff` xuất hiện** (`magAfterIchi = 0`).
- **Bằng chứng:** Log `TC-IND-004_005_VERIFIED` (`mag0=0, magAfterIchi=0`) & ảnh [`bug_custom_indicator_color_ignored.png`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence/bug_custom_indicator_color_ignored.png).
- **Nguyên nhân gốc:** [`ui/src/chart/indicators/customIndicators.ts`](file:///Users/mizuhara/workspace/sumi/ui/src/chart/indicators/customIndicators.ts) gắn hàm `styles: () => ({ color: '...', size: ... })` cố định trên từng `figure` của cả 5 chỉ báo (`ICHIMOKU`, `SUPERTREND`, `VWAP`, `ATR`, `STOCHRSI`), ghi đè toàn bộ thuộc tính `indicator.styles.lines[]` do người dùng cấu hình.
- **Khuyến nghị sửa:** Khai báo màu/độ dày mặc định trong `styles.lines` của định nghĩa chỉ báo thay vì hardcode trong callback `figures[].styles` (hoặc đọc `indicator.styles.lines[idx]` bên trong callback).

---

#### `BUG-019` — `IndicatorSettingsModal` Ghép Sai Số Dòng Style & Nhãn Đường Theo `calcParams.length`
- **Test Case:** `TC-IND-004` | **Mức độ:** **P2** | **Phân hệ:** Indicator Settings Modal
- **Thực tế vs Kỳ vọng:**
  - `BOLL` (3 đường: Trên/Giữa/Dưới, 2 tham số) chỉ hiện **2 dòng màu** gắn nhãn sai là `"Chu kỳ"` và `"Độ lệch chuẩn"`, **mất hoàn toàn đường thứ 3 (Dải dưới)**!
  - `ICHIMOKU` (5 đường, 3 tham số) chỉ hiện **3 dòng màu** (thiếu 2 đường).
  - `STOCHRSI` (2 đường `%K/%D`, 4 tham số) hiện thừa **4 dòng màu**.
- **Bằng chứng:** Ảnh [`bug_indicator_modal_boll_mismatch.png`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence/bug_indicator_modal_boll_mismatch.png) & log `TC-IND-004_005_VERIFIED` (`BOLL: 2/3, ICHIMOKU: 3/5, STOCHRSI: 4/2`).
- **Nguyên nhân gốc:** [`ui/src/components/modals/IndicatorSettingsModal.tsx:48, 222`](file:///Users/mizuhara/workspace/sumi/ui/src/components/modals/IndicatorSettingsModal.tsx#L48) duyệt theo `calcParams.length` và lấy nhãn từ `meta.paramLabels` thay vì danh sách `lineLabels` thực tế của từng chỉ báo.
- **Khuyến nghị sửa:** Thêm trường `lineLabels: string[]` vào `INDICATOR_CATALOG` (vd `BOLL: ['Dải trên (UP)', 'Đường giữa (MID)', 'Dải dưới (DN)']`) và render đúng theo `lineLabels.length`.

---

#### `BUG-020` — `VWAP` Frontend Hỏng Reset Phiên Intraday Qua Đêm/Cuối Tuần & Gặp Phương Sai Âm Do Hủy Số Học
- **Test Case:** `TC-IND-007` | **Mức độ:** **P2** | **Phân hệ:** Custom Indicators (`VWAP`)
- **Thực tế vs Kỳ vọng:**
  1. Trên `HOSE:FPT 4h`, có **`149/300` khoảng cách liên nến `>= 12 giờ`** (qua đêm 18h30 hoặc cuối tuần 66h). Vì `customIndicators.ts:275` chỉ đo khoảng cách của **2 nến cuối cùng** `< 12h` để quyết định `isIntraday`, cứ vào đầu mỗi buổi sáng hoặc sáng Thứ 2 thì `VWAP` tự tắt chế độ reset ngày và tích lũy dồn toàn lịch sử!
  2. Trên `HOSE:FPT 1D` (giá ~100k VND, khối lượng hàng chục triệu), công thức 1 lượt `cumPv2 / cumVol - vwap * vwap` gặp **`negVarCount = 1`** (phương sai âm do catastrophic cancellation).
- **Bằng chứng:** Log `TC-IND-007` (`149/300 gaps >= 12h`, `negVarCount = 1`).
- **Nguyên nhân gốc:** [`ui/src/chart/indicators/customIndicators.ts:275, 294`](file:///Users/mizuhara/workspace/sumi/ui/src/chart/indicators/customIndicators.ts#L275).
- **Khuyến nghị sửa:** Xác định `isIntraday` bằng trung vị `Δt` (Q1/median) hoặc từ `period` của biểu đồ, và thay bằng thuật toán Welford có trọng số (tương tự `sumi-core`).

---

#### `BUG-021` — Toàn Bộ Phím Tắt `Option + T / H / J / V / F / R` Hỏng Trên Bàn Phím macOS Thật
- **Test Case:** `TC-DRAW-002`, `TC-CHART-008` | **Mức độ:** **P2** | **Phân hệ:** Hotkeys / macOS Compatibility
- **Thực tế vs Kỳ vọng:** Trên macOS, nhấn `Option + T/H/J/V/F/R` tạo ra các ký tự Unicode `†`, `˙`, `∆`, `√`, `ƒ`, `®` trong `KeyboardEvent.key`. Do ứng dụng kiểm tra `e.key.toLowerCase() === 't' | 'h' | 'j' | 'v' | 'f' | 'r'`, toàn bộ 6 phím tắt này **không kích hoạt trên bàn phím macOS vật lý** (và trên Suite 2 `Alt+J` không vẽ được `horizontalRayLine`).
- **Nguyên nhân gốc:** [`ui/src/App.tsx:83-109`](file:///Users/mizuhara/workspace/sumi/ui/src/App.tsx#L83-L109) và [`ui/src/chart/KLineViewport.tsx:1112`](file:///Users/mizuhara/workspace/sumi/ui/src/chart/KLineViewport.tsx#L1112) kiểm tra `e.key` thay vì `e.code` (`'KeyT'`, `'KeyH'`, `'KeyJ'`, `'KeyV'`, `'KeyF'`, `'KeyR'`).
- **Khuyến nghị sửa:** So khớp bằng `e.code` khi `e.altKey === true`.

---

#### `BUG-022` — Kéo Điểm Neo & Sửa Style Hình Vẽ Không Có Trong Undo Stack, Và Thiếu Hoàn Toàn `Redo`
- **Test Case:** `TC-DRAW-008` | **Mức độ:** **P2** | **Phân hệ:** Drawing Tools Undo/Redo
- **Thực tế vs Kỳ vọng:** Vẽ `segment` → đổi độ dày `3px` → kéo điểm neo #2 sang vị trí mới → bấm `Cmd+Z` 1 lần: hệ thống **bỏ qua hoàn toàn bước kéo điểm neo và bước đổi độ dày**, lập tức xóa thẳng cây đường xu hướng khỏi SQLite! Ngoài ra không có bất kỳ phím tắt `Redo` (`Cmd/Ctrl+Y` hoặc `Cmd+Shift+Z`) nào.
- **Bằng chứng:** Log `TC-DRAW-008` & `reverify_final` (sau 1 lần `Cmd+Z`, bản ghi `d1` bị xóa khỏi SQLite).
- **Nguyên nhân gốc:** [`ui/src/chart/KLineViewport.tsx:290-295`](file:///Users/mizuhara/workspace/sumi/ui/src/chart/KLineViewport.tsx#L290-L295) lưu tọa độ mới trực tiếp xuống SQLite trong `onPressedMoveEnd` mà không push snapshot trước khi kéo vào `undoStackRef`; [`KLineViewport.tsx:1104-1111`](file:///Users/mizuhara/workspace/sumi/ui/src/chart/KLineViewport.tsx#L1104-L1111) không cài đặt `redoStackRef`.
- **Khuyến nghị sửa:** Lưu bản sao `before` trong `onPressedMoveStart`, push `{ kind: 'modified', before, after }` vào `undoStackRef` khi kết thúc kéo/đổi style, và bổ sung `redoStackRef` (`Cmd+Shift+Z` / `Ctrl+Y`).

---

#### `BUG-023` — Stock Screener Không Hỗ Trợ Quét Crypto/US & Tính Trùng Chỉ Báo 2 Lần/Mã (Quét Lạnh `5.41s`)
- **Test Case:** `TC-QUANT-007`, `TC-PERF-007` | **Mức độ:** **P2** | **Phân hệ:** Stock Screener
- **Thực tế vs Kỳ vọng:**
  1. Gửi `exchangeFilter: "CRYPTO"` trả về `0` mã `BINANCE`; giao diện `StockScreenerTab` chỉ có `Tất cả`, `HOSE`, `HNX`, `UPCOM`, `Watchlist` — không lọc được `BINANCE`/`NASDAQ`.
  2. Quét toàn bộ 469 mã lần đầu mất **`5,412.7 ms`** (lần ấm vẫn tới **`1,197.8 ms` p50 / `1,863.7 ms` max**, vượt ngưỡng SLA `1,500 ms`).
- **Bằng chứng:** Log `TC-QUANT-007` & `TC-PERF-007` (`469 syms: cold 5412.7ms, warm p50=1197.8ms / max=1863.7ms`).
- **Nguyên nhân gốc:** [`crates/sumi-desktop/src/commands/scanner.rs:161-172`](file:///Users/mizuhara/workspace/sumi/crates/sumi-desktop/src/commands/scanner.rs#L161-L172) lọc cứng `HOSE|HNX|UPCOM`; dòng [`265` & `283`](file:///Users/mizuhara/workspace/sumi/crates/sumi-desktop/src/commands/scanner.rs#L265) tính toán **hai lần độc lập** toàn bộ tập chỉ báo cho từng mã (`evaluate_symbol_scan` + `PrecomputedCatalogMatrix::compute`) trên luồng tuần tự.
- **Khuyến nghị sửa:** Bổ sung bộ lọc `CRYPTO`/`US`, dùng chung kết quả tính toán từ `PrecomputedCatalogMatrix` (cắt giảm ~50% CPU) và song song hóa bằng `rayon` trên 8 nhân M1 (dự kiến giảm từ `1.2s` xuống `<200ms`).

---

#### `BUG-024` — Đồng Hồ Đếm Ngược Đóng Nến (`Bar Countdown`) Bị Đóng Băng, Không Nhảy Giây Tự Động
- **Test Case:** `TC-CHART-010` | **Mức độ:** **P2** | **Phân hệ:** Chart / Price Axis
- **Thực tế vs Kỳ vọng:** Nhãn đếm ngược hiển thị trên trục Y (vd `09:46:10` trong `bug_grid4_4subpanes_crush.png`), nhưng khi đứng yên trong 3 giây trên `BINANCE:BTCUSDT 1m`, ảnh Canvas trục Y **không hề đổi pixel (`1/3` khung hình khác nhau)** — đồng hồ chỉ cập nhật khi có tick giá mới hoặc khi di chuột!
- **Bằng chứng:** Log `TC-CHART-010` (`1/3 distinct renders`) & xác nhận `grep setInterval KLineViewport.tsx` trả về 0 kết quả.
- **Nguyên nhân gốc:** [`ui/src/chart/KLineViewport.tsx:435-439`](file:///Users/mizuhara/workspace/sumi/ui/src/chart/KLineViewport.tsx#L435-L439) cấu hình `formatExtendText` nhưng không có `window.setInterval(..., 1000)` để kích hoạt vẽ lại trục giá mỗi giây khi `showCountdown === true`.
- **Khuyến nghị sửa:** Thêm `useEffect` chạy `setInterval(..., 1000)` gọi phương thức invalidate/repaint nhẹ của `KLineChart` khi `pane.showCountdown && !replayActive`.

---

#### `BUG-025` — Bảng `symbols` Lưu Trùng Mã Wire `HNX:VN301!` Song Song `HNX:VN30F1!`
- **Test Case:** `TC-DATA-007` | **Mức độ:** **P2** | **Phân hệ:** Data Cache / Symbol Alias
- **Thực tế vs Kỳ vọng:** Trong `sumi_market.db`, bảng `symbols` chứa cả 2 dòng trùng mô tả `VN30 Index Futures (I1)`: `HNX:VN301!` (wire TradingView) và `HNX:VN30F1!` (canonical).
- **Nguyên nhân gốc:** [`crates/sumi-data/src/cache_service.rs:991-993`](file:///Users/mizuhara/workspace/sumi/crates/sumi-data/src/cache_service.rs#L991-L993) lưu `ResolvedSymbol` trực tiếp bằng ticker wire khi fetch lịch sử mà không chuẩn hóa về canonical alias `HNX:VN30F1!` như `resolve_symbol` ở dòng `256-262`.
- **Khuyến nghị sửa:** Áp dụng hàm chuẩn hóa `canonical_symbol_key` trước khi gọi `store.upsert_symbol`.

---

#### `BUG-026` — Bán Bớt Một Phần Vị Thế (Partial Close T+2) Không Ghi Nhận Giao Dịch Đóng & Nhật Ký Cho Tới Khi Đóng Hết
- **Test Case:** `TC-BROKER-006` | **Mức độ:** **P2** | **Phân hệ:** Paper Broker / Trade Journal
- **Thực tế vs Kỳ vọng:** Mua `1.000` cp ngày `T0`, mua thêm `500` cp ngày `T+1`, bán `1.000` cp đã về tài khoản ngày `T+2` (còn giữ `500` cp): FIFO mở khóa chính xác lô `1.000`, nhưng `closedTrades` vẫn rỗng (`length = 0`) nên trader không thể xem PnL/phí/thuế hay ghi nhật ký cho lệnh chốt lời 1.000 cổ phiếu vừa thực hiện!
- **Bằng chứng:** Log `TC-BROKER-006_EXACT` (`closedTradesLen: 0`, `remaining locked_t2_qty: 500`).
- **Nguyên nhân gốc:** `TradingSession` chỉ push `ClosedTrade` khi `position.quantity == 0` (tích lũy lãi/lỗ chốt một phần vào `realized_gross_pnl_acc` trên `Position` đang mở).
- **Khuyến nghị sửa:** Phát sinh bản ghi `ClosedTrade` (hoặc hiển thị tab `Lịch sử Khớp lệnh / Chốt phần`) cho mỗi giao dịch giảm quy mô vị thế theo lô FIFO.

---

#### `BUG-027` — Hình Vẽ Trên Cùng Một Mã Không Đồng Bộ Tức Thì Giữa Các Ô Đa Biểu Đồ
- **Test Case:** `TC-DRAW-010` | **Mức độ:** **P2** | **Phân hệ:** Multi-Chart Drawings
- **Thực tế vs Kỳ vọng:** Trong bố cục `2 dọc` cùng mở `HOSE:FPT`, vẽ một đường ngang trên `pane-0` lưu thành công vào SQLite nhưng `pane-1` **không hiển thị đường vẽ mới** cho đến khi reload hoặc đổi mã.
- **Bằng chứng:** Log `TC-DRAW-010` (`drawsInvokes = ["upsert_drawing"]`, không có cơ chế phát sóng tới ô anh em).
- **Nguyên nhân gốc:** [`ui/src/chart/KLineViewport.tsx:790-806`](file:///Users/mizuhara/workspace/sumi/ui/src/chart/KLineViewport.tsx#L790-L806) chỉ nạp hình vẽ khi `pane.ticker` thay đổi, không có event bus đồng bộ `upsert/delete/clear` giữa các `KLineViewport` cùng ticker.
- **Khuyến nghị sửa:** Thêm bộ phát sự kiện `drawingSyncBus` (tương tự `crosshairBus`) để đồng bộ ngay các thao tác tạo/sửa/xóa hình vẽ giữa các ô cùng mã.

---

#### `BUG-028` — Tốc Độ Bar Replay `20x` Bị Kẹp Xuống `10x` (Dead Code `ReplaySpeed::X20`)
- **Test Case:** `TC-REPLAY-004` | **Mức độ:** **P2** | **Phân hệ:** Bar Replay
- **Thực tế vs Kỳ vọng:** Gọi `replay_set_speed(20.0)` trả về `10.0`, và giao diện `ReplayControlBar` chỉ có tùy chọn `[0.5, 1, 2, 3, 5, 10]`, khiến `ReplaySpeed::X20` ở `replay.rs:33` không bao giờ chạm tới được.
- **Bằng chứng:** Log `TC-REPLAY-004` (`Speeds 1x..20x = [1, 2, 5, 10, 10]`).
- **Nguyên nhân gốc:** [`crates/sumi-desktop/src/commands/replay.rs:302`](file:///Users/mizuhara/workspace/sumi/crates/sumi-desktop/src/commands/replay.rs#L302) kẹp `.clamp(0.5, 10.0)` và [`ui/src/components/ReplayControlBar.tsx:5, 22`](file:///Users/mizuhara/workspace/sumi/ui/src/components/ReplayControlBar.tsx#L5) giới hạn `10`.
- **Khuyến nghị sửa:** Mở giới hạn lên `20.0` (khoảng `50ms/bước` — rất nhẹ vì `replay_step` chỉ mất `0.22ms` server!) hoặc bỏ tùy chọn `X20` thừa.

---

#### `BUG-029` — Các Vector Panic, Vòng Lặp Vô Hạn & Sai Khác Quy Ước Trong Lõi `sumi-core`
- **Test Case:** Kiểm toán sâu 63 file `crates/sumi-core/src/` | **Mức độ:** **P2** | **Phân hệ:** Quantitative Core (`sumi-core`)
- **Các lỗi cụ thể đã xác minh vị trí dòng mã:**
  1. **Panic deserialization:** [`viewport/kinetic.rs:227`](file:///Users/mizuhara/workspace/sumi/crates/sumi-core/src/viewport/kinetic.rs#L227) gọi `.clamp(0.0, self.duration_ms)` sẽ panic nếu `duration_ms < 0` khi deserialize (thiếu `sanitize_deserialized`), và `damping_coeff == 1.0` chia cho `ln(1.0) == 0`.
  2. **Debug panic `i64::MIN`:** [`drawings/geometry.rs:1440`](file:///Users/mizuhara/workspace/sumi/crates/sumi-core/src/drawings/geometry.rs#L1440) gọi `(t1.saturating_sub(t0)).abs()` (nên dùng `abs_diff`).
  3. **Vòng lặp vô hạn trên `i64::MAX`:** [`data/audit.rs:728-745`](file:///Users/mizuhara/workspace/sumi/crates/sumi-core/src/data/audit.rs#L728-L745) (`W1` gap loop) và [`journal/market_rules.rs:769-806`](file:///Users/mizuhara/workspace/sumi/crates/sumi-core/src/journal/market_rules.rs#L769-L806) (`add_vn_trading_days` khi `i64::MAX` là ngày nghỉ).
  4. **Khớp lệnh `NaN → 1e-9`:** [`trading/orders.rs:476-483`](file:///Users/mizuhara/workspace/sumi/crates/sumi-core/src/trading/orders.rs#L476-L483) (`f64::NAN.max(1e-9) == 1e-9`) cho phép khớp Market ở `1e-9` nếu dữ liệu hỏng `NaN`.
  5. **Chuẩn hóa năm `H1` vs `H4` bất nhất trong Backtest:** [`backtest/metrics.rs:501-502`](file:///Users/mizuhara/workspace/sumi/crates/sumi-core/src/backtest/metrics.rs#L501-L502) dùng `1,638` nến/năm cho `H1` (equity 6.5h/ngày) nhưng `1,512` nến/năm cho `H4` (crypto 24h/ngày — chênh lệch 4× thời lượng nến nhưng hệ số năm gần bằng nhau!).
  6. **Parabolic SAR sai kẹp 2 nến đầu:** [`indicators/extended.rs:537-544`](file:///Users/mizuhara/workspace/sumi/crates/sumi-core/src/indicators/extended.rs#L537-L544) ghi đè `prev_low = c.low` quá sớm tại `seg_idx == 1`.
  7. **Danh sách sàn lỗi thời:** `"BSR"` vẫn nằm trong `UPCOM_LISTED_SYMBOLS` ([`market_rules.rs:637`](file:///Users/mizuhara/workspace/sumi/crates/sumi-core/src/journal/market_rules.rs#L637)) dù đã chuyển sàn sang `HOSE` từ tháng 01/2025.
- **Khuyến nghị sửa:** Bổ sung hàm `sanitize_deserialized`, chặn `i64::MIN/MAX` & `NaN`, thống nhất hệ số năm `H1/H4` theo lớp tài sản và cập nhật danh sách chuyển sàn `BSR → HOSE`.

---

#### `BUG-043` — Thiếu Đồng Bộ Phạm Vi Thời Gian (`Sync Date Range`), Đồng Bộ Hình Vẽ (`Sync Drawings`) & Đa Cửa Sổ Rời (`Multi-Window`) Cho Đa Màn Hình
- **Test Case:** `TC-MTF-REPLAY-005` | **Mức độ:** **P2** | **Phân hệ:** Multi-Chart / Multi-Window Architecture
- **Thực tế vs Kỳ vọng:**
  1. Menu Đồng bộ tại [`ui/src/components/TopHeader.tsx:376-382`](file:///Users/mizuhara/workspace/sumi/ui/src/components/TopHeader.tsx#L376-L382) chỉ hỗ trợ 3 cờ (`Symbol`, `Interval`, `Crosshair`), **thiếu hoàn toàn `Sync Time / Date Range`** (kéo lùi lịch sử ở ô `1h` thì ô `4h` vẫn đứng ở hiện tại khiến đối chiếu đa khung giờ cực kỳ bất tiện) và **thiếu `Sync Drawings`** (`BUG-027`).
  2. [`crates/sumi-desktop/tauri.conf.json:13-25`](file:///Users/mizuhara/workspace/sumi/crates/sumi-desktop/tauri.conf.json#L13-L25) chỉ định nghĩa duy nhất 1 cửa sổ `main` — trader dùng 2 màn hình không thể tách cửa sổ biểu đồ thứ 2 ra màn hình phụ mà vẫn đồng bộ mã/Replay.
- **Khuyến nghị sửa:** Bổ sung `Sync Date Range` (đồng bộ `scrollToTimestamp` / khoảng hiển thị giữa các ô cùng mã) và `Sync Drawings` trong menu Layout; trên lộ trình v2.2+, hỗ trợ Tauri v2 `WebviewWindowBuilder` đồng bộ trạng thái qua Rust `AppState` events cho thiết lập đa màn hình.

---

### Mức Độ P3 — Thấp (Hiệu Năng UI / Tiện Dụng / Validate Đầu Vào)

#### `BUG-030` — Watchlist Bị Hủy & Tạo Lại Toàn Bộ DOM Hàng Mỗi Tick Giá (`DOM Churn`)
- **Test Case:** `TC-WIDGET-003` | **Mức độ:** **P3** | **Phân hệ:** RightWidgetBar / React Rendering
- **Thực tế vs Kỳ vọng:** `MutationObserver` trong 12 giây trên Watchlist Crypto ghi nhận **`30 removedNodes` + `30 addedNodes` + `0 characterData updates`** — mỗi lần giá nhảy, toàn bộ DOM của dòng mã đó bị unmount & remount!
- **Bằng chứng:** Log `TC-WIDGET-003_EXACT` (`{"added":30,"removed":30,"text":0}`).
- **Nguyên nhân gốc:** [`ui/src/components/RightWidgetBar.tsx:265`](file:///Users/mizuhara/workspace/sumi/ui/src/components/RightWidgetBar.tsx#L265) gắn `flash?.nonce` vào thuộc tính React `key`, ép React hủy node cũ và tạo node mới để kích hoạt lại CSS animation.
- **Khuyến nghị sửa:** Giữ nguyên `key={ticker}` và kích hoạt flash bằng Web Animations API (`el.animate(...)`) hoặc đổi class hai trạng thái (`flash-a`/`flash-b`) mà không phá hủy DOM.

---

#### `BUG-031` — Thiếu Công Cụ Chuyển Đổi Múi Giờ (`Asia/Ho_Chi_Minh` ↔ `Etc/UTC`) Trên Giao Diện
- **Test Case:** `TC-CHART-004` | **Mức độ:** **P3** | **Phân hệ:** Chart / UX Parity
- **Thực tế vs Kỳ vọng:** Không có nút chọn múi giờ ở bất kỳ vị trí nào trên UI; [`ui/src/chart/KLineViewport.tsx:670`](file:///Users/mizuhara/workspace/sumi/ui/src/chart/KLineViewport.tsx#L670) ép cứng `Etc/UTC` cho Crypto/Mỹ và `Asia/Ho_Chi_Minh` cho VN. Trader VN xem `BINANCE:BTCUSDT 1h` không thể đổi trục giờ sang giờ Việt Nam (`UTC+7`).
- **Khuyến nghị sửa:** Thêm bộ chọn múi giờ gọn nhẹ trên góc dưới phải thanh trục X (chuẩn TradingView) và lưu vào `workspace_settings`.

---

#### `BUG-032` — Thẻ Chi Tiết Ghi Sai `"Bước giá: 0.1 VND"` Cho `VN30F1!` & Cho Đặt Lệnh Giấy Trên Chỉ Số `HNXINDEX`
- **Test Case:** `TC-WIDGET-004` | **Mức độ:** **P3** | **Phân hệ:** RightWidgetBar / PaperBrokerTab
- **Thực tế vs Kỳ vọng:** Thẻ Chi tiết mã hiển thị **`Bước giá: 0.1 VND`** cho `HNX:VN30F1!` (phải là `0.1 điểm`), hardcode `0.01` cho crypto 4 số lẻ (`XRPUSDT`), và khi mở chỉ số `HNX:HNXINDEX` / `HOSE:VNINDEX`, phiếu `Paper Trading` vẫn mở nút `Đặt Mua LO · 1,000` với đơn vị `đ` và `KL (Lô 100)`.
- **Bằng chứng:** Ảnh [`bug_vn30f_broker_100k_pnl_and_margin.png`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence/bug_vn30f_broker_100k_pnl_and_margin.png) & [`bug_weekend_spike_hnxindex.png`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence/bug_weekend_spike_hnxindex.png).
- **Nguyên nhân gốc:** [`ui/src/components/RightWidgetBar.tsx:393-396`](file:///Users/mizuhara/workspace/sumi/ui/src/components/RightWidgetBar.tsx#L393-L396) nối cứng hậu tố `" VND"`; `PaperBrokerTab` chưa khóa đặt lệnh với `market_type === "index"`.

---

#### `BUG-033` — `create_watchlist` Chấp Nhận Tên Danh Mục Rỗng / Chỉ Gồm Khoảng Trắng
- **Test Case:** `TC-WIDGET-002` | **Mức độ:** **P3** | **Phân hệ:** Workspace Validation
- **Thực tế vs Kỳ vọng:** Gọi `create_watchlist(id, "   ")` trả về `ok = true` và ghi dòng tên trắng vào bảng `watchlists`.
- **Nguyên nhân gốc:** [`crates/sumi-desktop/src/commands/workspace.rs:151-160`](file:///Users/mizuhara/workspace/sumi/crates/sumi-desktop/src/commands/workspace.rs#L151-L160) không kiểm tra `name.trim().is_empty()`.

---

#### `BUG-034` — Không Tự Cập Nhật `KL` Khi Chuyển Lớp Tài Sản & Không Cập Nhật `Giá LO` Khi Bật Bar Replay
- **Test Case:** `TC-BROKER-009`, `TC-REPLAY-001` | **Mức độ:** **P3** | **Phân hệ:** PaperBrokerTab UX
- **Thực tế vs Kỳ vọng:**
  1. Khi chuyển từ `HOSE:FPT` sang `HNX:VN30F1!` hoặc `BINANCE:BTCUSDT`, ô `KL` vẫn giữ **`1000`** (khiến ước tính trên `BTCUSDT` vọt lên **`$60,792,550.00`** trên tài khoản `$100,000`!).
  2. Khi bật Bar Replay (lùi về giá `66,000`), ô `Giá LO` vẫn giữ giá live **`45,000`** kèm dòng báo lỗi đỏ `Giá ngoài biên độ [59,800 — 68,800]`.
- **Bằng chứng:** Ảnh [`bug_grid4_4subpanes_crush.png`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence/bug_grid4_4subpanes_crush.png) (`BTCUSDT KL 1000 -> $60.79M`) và [`11_replay_active.png`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence/11_replay_active.png).
- **Nguyên nhân gốc:** [`ui/src/components/dock/PaperBrokerTab.tsx:54-60`](file:///Users/mizuhara/workspace/sumi/ui/src/components/dock/PaperBrokerTab.tsx#L54-L60) thiếu `useEffect` reset `qty` theo `boardLot` mới và đồng bộ `limitPrice` khi `broker.markPrice` thay đổi do Replay.

---

#### `BUG-035` — Các Nút Ẩn/Cài Đặt/Xóa Chỉ Báo Trên Legend Nằm Trong Canvas (Thiếu Accessibility) & Gọi Trùng `broker_get_state` Lúc Khởi Động
- **Test Case:** `TC-IND-006`, Smoke Boot Telemetry | **Mức độ:** **P3** | **Phân hệ:** Accessibility / Boot Efficiency
- **Thực tế vs Kỳ vọng:**
  1. Các nút `👁 / ⚙ / ✕` trên góc trái biểu đồ được vẽ bằng `KLineChart` Canvas tooltip features (không có DOM node hay ARIA label — người dùng bàn phím/screen reader hoặc công cụ kiểm thử DOM không thể tương tác trực tiếp, buộc phải mở lại `IndicatorsModal`).
  2. Trong quá trình khởi động ban đầu, `broker_get_state` bị gọi **2 lần liên tiếp** (`1.567ms` và `11.396ms`).

---

### Mức Độ P4 — Thẩm Mỹ & Trải Nghiệm Chi Tiết (Visual Polish)

#### `BUG-036` — `FloatingDrawingToolbar` Che Đè Lên Dòng Legend OHLC & `EMA(20)`
- **Test Case:** `TC-DRAW-006`, `TC-DRAW-011` | **Mức độ:** **P4** | **Phân hệ:** Chart UI Layout
- **Thực tế:** Khi chọn một hình vẽ, thanh `FloatingDrawingToolbar` neo ở cạnh trên đè trực tiếp lên chuỗi `O / H / L / C` và `EMA(20)` góc trên trái.
- **Bằng chứng:** Ảnh [`08_drawings_showcase.png`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence/08_drawings_showcase.png) và [`10_chart_context_menu.png`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence/10_chart_context_menu.png).

#### `BUG-037` — Chồng Chéo Chữ Giữa Nhãn Đỉnh Nến (`High`) Với Legend `EMA(50)` Và Chồng Pill Giá Trên Trục Y
- **Test Case:** `TC-CHART-001`, `TC-IND-002` | **Mức độ:** **P4** | **Phân hệ:** Canvas Typography
- **Thực tế:** Nhãn giá đỉnh cao nhất của cửa sổ nến (vd `313.86`, `23,197`, `2,015.8`) nằm đè trực tiếp lên văn bản `EMA(50)` ở góc trên trái; các pill giá chỉ báo (`EMA20/EMA50`, `VOL MA5/MA20`, `StochRSI %K/%D`) chồng đè lên nhau trên trục Y phải khi hai đường tiến sát nhau.
- **Bằng chứng:** Ảnh [`bug_weekend_spike_hnxindex.png`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence/bug_weekend_spike_hnxindex.png), [`bug_cafef_tv_splice_ssi.png`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence/bug_cafef_tv_splice_ssi.png), [`07_custom_indicators_all5.png`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence/07_custom_indicators_all5.png).

#### `BUG-038` — Khung `1D` Hiện Giờ `"07:00"` Trên Nhãn Công Cụ Vẽ, Mini-Header Hiện `"15"` Thiếu `"m"` & Nạp `pane_indicators` Khi Reload Bố Cục 4 Ô
- **Test Case:** `TC-DRAW-004`, `TC-GRID-010` | **Mức độ:** **P4** | **Phân hệ:** Chart Formatting / Multi-Pane Hydration
- **Thực tế:** Khi vẽ trên khung `1D` (`Asia/Ho_Chi_Minh`), nhãn thời gian trục X của điểm neo hiển thị `2026-08-04 07:00` (do nến ngày UTC `00:00` + 7h) và hai nhãn chồng lên nhau; khung 15 phút trên biểu đồ hiển thị `HNX:VN30F1! · 15` (thiếu chữ `m`); trong lưới 4 ô khi reload nhanh, `workspace_settings` ghi đè danh sách `pane_indicators` đã lưu riêng.
- **Bằng chứng:** Ảnh [`08_drawings_showcase.png`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence/08_drawings_showcase.png) và [`bug_grid4_4subpanes_crush.png`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence/bug_grid4_4subpanes_crush.png).

#### `BUG-039` — Nhãn Tham Chiếu `TC` Ở Góc Trên Phải Hiện Tới 3 Số Lẻ Trên Cổ Phiếu HOSE Lịch Sử (`TC 64,272.727`)
- **Test Case:** `TC-REPLAY-001`, `TC-CHART-009` | **Mức độ:** **P4** | **Phân hệ:** VN Band Formatting
- **Thực tế:** Khi dữ liệu lịch sử CafeF có giá điều chỉnh thập phân (`64,272.727`), thanh badge góc trên phải không làm tròn về số nguyên/bước giá HOSE (`Trần 68,800 TC 64,272.727 Sàn 59,800`).
- **Bằng chứng:** Ảnh [`11_replay_active.png`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence/11_replay_active.png) và [`03_light_mode.png`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence/03_light_mode.png).

---

## 3. Phân Tích Khoảng Cách Trải Nghiệm So Với TradingView Supercharts (UX Parity Gaps)

Ngoài 43 lỗi cụ thể trên, dưới góc nhìn trader chuyên nghiệp trên TradingView Supercharts, Sumi v2.1.0 còn 8 khoảng cách trải nghiệm chính cần thu hẹp cho bản phát hành kế tiếp:

1. **Đồng bộ Đa biểu đồ chưa trọn vẹn:** Thiếu đồng bộ **phạm vi thời gian (`Sync Date Range`, `BUG-043`)** và **đồng bộ hình vẽ cùng mã (`BUG-027`)**; đồng bộ Crosshair đang gãy hoàn toàn (`BUG-004`).
2. **Thao tác lệnh trực quan trên biểu đồ (Chart Trading):** Đã vẽ được đường vị thế/lệnh chờ (`brokerOrderLine`), nhưng **chưa kéo thả (drag-to-modify)** trực tiếp đường SL/TP/Limit ngay trên trục giá Canvas như TradingView Paper Trading.
3. **Động cơ Cảnh báo còn sơ khai ở tầng Desktop:** Lõi `sumi-core::alerts` đã xây đủ 6 loại điều kiện (RSI, MACD, Drawing Line, Signal Catalog, Confluence), nhưng tầng `sumi-desktop` mới nối loại `PriceLevel` đơn giản và chỉ kiểm tra trên Watchlist đang mở (`BUG-017`).
4. **Hỗ trợ Múi giờ & Chế độ Phiên giao dịch:** Chưa có bộ chọn múi giờ thủ công (`BUG-031`) và chưa có phân tách trực quan phiên sáng/chiều (`09:00–11:30`, `13:00–14:45`) trên khung intraday VN.
5. **Chỉ báo & Mẫu (Indicator Templates):** Chưa cho phép lưu bộ nhiều chỉ báo thành Template dùng chung 1-click giữa các mã; `IndicatorSettingsModal` còn ghép nhầm cấu trúc dòng màu (`BUG-019`).
6. **Phím tắt & Thao tác đa nền tảng macOS:** Thiếu `Redo` (`Cmd+Shift+Z`), phím `Option+*` hỏng trên macOS thật (`BUG-021`), chưa có thanh `Object Tree` (Cây đối tượng quản lý thứ tự lớp các hình vẽ & chỉ báo).
7. **Lọc cổ phiếu đa thị trường & Cột tùy chỉnh:** Stock Screener chỉ mới quét cổ phiếu VN (`BUG-023`), chưa hỗ trợ quét Crypto/US hay bộ lọc điều kiện số tùy biến (`RSI < 30 AND RVOL > 2`).
8. **Chế độ Bar Replay nâng cao:** Chưa đồng bộ đa khung giờ (`BUG-040`), chưa có chế độ luyện tập mù Blind Mode chống lộ giá (`BUG-041`, `BUG-042`), chưa nối `ReplayStepUnit::BaseBar` (phát từng nến `1h` cấu thành nến `4h`/`1D` dần dần dù `sumi-core::replay` đã xây sẵn!) và chưa cô lập tài khoản broker Replay khỏi tài khoản Live (`BUG-006`).

---

## 4. Phân Tích Nhu Cầu Thực Chiến Của Người Dùng Sumi Supercharts (6 Hành Trình Cốt Lõi)

| # | Nhu cầu cốt lõi của Trader thực chiến | Quy trình thao tác chuẩn trên thực tế | Hiện trạng Sumi `v2.1.0` & Điểm gãy chặn trải nghiệm |
|---|----------------------------------------|----------------------------------------|-------------------------------------------------------|
| 1 | **Luyện đọc nến & thực hành PTKT Đa Khung Giờ (Top-Down MTF Bar Replay)** | Mở 2 ô (`4h` xác định cấu trúc/kháng cự + `1h` hoặc `15m` tìm điểm vào) → cắt nến quá khứ → bước từng nến `1h`, nhìn nến `4h` hình thành dần mà **không lộ tương lai** → vẽ vùng cản trên `4h` hiện ngay sang `1h` → vào lệnh giấy + ghi Nhật ký. | **GÃY HOÀN TOÀN:** Ô `4h` lộ 100% tương lai & click sang ô 2 bơm nhầm nến `1h` (`BUG-040`); Watchlist & Chi tiết lộ thẳng giá live (`BUG-041`); vẽ ở `4h` không hiện sang `1h` (`BUG-027`); bật Replay xóa sổ lệnh live (`BUG-006`); đổi khung trên 1 ô tự hủy Replay (`BUG-040`). |
| 2 | **Phân tích Đa Khung Giờ (Live MTF Top-Down Analysis)** | Xem song song `1D / 4h / 1h / 15m` (hoặc tách 2 màn hình), rê chuột trên `1h` phải gióng đúng cây nến `4h`/`1D` tương ứng, cuộn về quá khứ đồng bộ thời gian, hình vẽ đồng bộ tức thì. | **GÃY MỘT PHẦN:** Lưới 1/2/3/4 ô & Sync Symbol tốt, nhưng **CrosshairBus tê liệt 100%** (`BUG-004`), hình vẽ không sync giữa các ô (`BUG-027`), thiếu Sync Date Range & Multi-Window (`BUG-043`). |
| 3 | **Giao dịch mô phỏng & quản trị rủi ro sát luật TTCK Việt Nam + Phái sinh** | Cổ phiếu VN: T+2 FIFO, lô 100, bước giá `10/50/100`, biên độ `±7/10/15%`, ATO/ATC đúng bản chất. Phái sinh `VN30F1!`: ký quỹ 17%, `× 100.000 đ/điểm`, Long/Short 2 chiều, đặt SL/TP & tính R:R rõ ràng. | **RỦI RO CAO TRÊN PHÁI SINH:** Cơ sở T+2 & cấm short tốt, nhưng `VN30F1!` ký quỹ sai 5× (`3.4%`) & PnL/Notional sai `100.000×` (`BUG-002`), Context Menu đặt `1.000 HĐ` (`BUG-008`), sai bước giá không chặn & `ATO/ATC = MP` (`BUG-009`), đổi khung giờ mất lịch sử lệnh (`BUG-007`), bán bớt không ghi Nhật ký (`BUG-026`). |
| 4 | **Dữ liệu thị trường sạch, liên tục & chạy Offline mượt** | Lịch sử nhiều năm không nến ảo, không nhảy giá giả khi chia tách/nối nguồn, dùng được khi đi máy bay/mất mạng để ôn bài và backtest. | **KHÁ TỐT NHƯNG CÓ GAI DỮ LIỆU:** Kiến trúc 3 tầng L1/L2/L3 + Offline cực nhanh (`<3ms`), nhưng tồn tại **91 nến cuối tuần CafeF** (`BUG-001`) và **11 mã HOSE nhảy bậc nối CafeF–TV >7%** (`BUG-003`). |
| 5 | **Khép kín mạch Sàng lọc (Screener) → Kiểm định (Backtest) → Biểu đồ (Chart)** | Quét toàn thị trường tìm mã có tín hiệu hợp lưu → bấm xem biểu đồ thấy **cùng một con số chỉ báo** → chạy Backtest/WFO/Monte Carlo đúng luật tài sản để kiểm chứng lợi thế thống kê. | **LỆCH SỐ LIỆU LIÊN TẦNG:** Lõi Rust tính cực nhanh (`<3ms`), nhưng `MACD` Chart lệch `2×` Screener (`BUG-010`), Backtest UI ép luật cổ phiếu lên `VN30F1!` (`BUG-011`), Screener chưa quét Crypto/US (`BUG-023`), `VWAP` sai reset 4H (`BUG-020`). |
| 6 | **Công cụ vẽ, cảnh báo & vận hành ổn định cả phiên giao dịch trên macOS** | Vẽ nhanh bằng phím tắt (`Option+T/H/F`), Undo/Redo mượt, đặt cảnh báo giá phải nổ kể cả khi đang xem mã khác, mở app suốt 6–8 tiếng không rò rỉ tài nguyên. | **NHIỀU ĐIỂM NGHẼN NHỎ TÍCH TỤ:** 6 phím `Option+*` hỏng trên macOS (`BUG-021`), Undo bỏ qua kéo anchor (`BUG-022`), Alert mất 4/5 khi tạo nhanh & không nổ ngoài Watchlist (`BUG-017`), rò rỉ `bar_streams` + Channel callbacks (`BUG-012`), Countdown đóng băng (`BUG-024`). |
