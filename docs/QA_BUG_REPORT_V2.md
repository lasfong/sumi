# Báo Cáo Kiểm Định Chất Lượng Toàn Diện — Sumi Supercharts v2.0.0 → v2.1.0 (`QA_BUG_REPORT_V2`)

**Vai trò kiểm định:** Principal TradingView QA Lead & Principal Rust / Tauri v2 / Canvas Charting Architect  
**Phiên bản kiểm thử ban đầu:** `v2.0.0` (commit `7441339`) → **Phiên bản nghiệm thu:** `v2.1.0`  
**Môi trường kiểm thử:** macOS Apple Silicon (arm64), `/Applications/Sumi Supercharts.app` + Vite runtime (`http://localhost:1420/`, viewport `1440×900`) + kiểm tra trực tiếp cơ sở dữ liệu SQLite thực tế (`data/cache/sumi_market.db` & `~/Library/Application Support/com.sumi.supercharts/sumi_market.db`)  
**Ngày hoàn tất nghiệm thu:** 2026-11-22  

---

## 1. Tóm Tắt Điều Hành (Executive Summary)

Qua quá trình đọc rà soát toàn bộ mã nguồn (`ui/src/`, `crates/sumi-desktop/src/`, `crates/sumi-data/src/`, `research_repos/KLineChart/src/`), đối chiếu dữ liệu thực tế trên bản cài đặt macOS và thực hiện kiểm thử tương tác trực tiếp trên toàn bộ 7 nhóm chức năng cốt lõi, đội QA ghi nhận **22 lỗi kỹ thuật & khoảng trống hành vi nghiêm trọng** ở `v2.0.0`. Toàn bộ 22/22 lỗi đã được tái kiến trúc triệt để và nghiệm thu tương tác ở `v2.1.0`:

| Mức độ | Số lượng | Trạng thái sau nghiệm thu `v2.1.0` |
|---|---:|---|
| **CRITICAL** | 11 | `VERIFIED FIXED` (11/11) |
| **HIGH** | 5 | `VERIFIED FIXED` (5/5) |
| **MEDIUM** | 5 | `VERIFIED FIXED` (5/5) |
| **LOW** | 1 | `VERIFIED FIXED` (1/1) |
| **Tổng cộng** | **22** | **22 / 22 VERIFIED FIXED** |

---

## 2. Danh Mục Lỗi Chi Tiết (`BUG-001` .. `BUG-022`)

### Nhóm A: Tầng Dữ Liệu, Độ Trễ & Hiệu Năng Canvas (`120 FPS`)

#### `BUG-001` — Chặn đồng bộ chờ mạng (`1–2.5s`) khi chuyển mã hoặc khung giờ dù SQLite đã có dữ liệu
- **Mức độ:** `CRITICAL`
- **Trạng thái:** `VERIFIED FIXED` (`v2.1.0`)
- **Hành vi lỗi hiện tại:** Khi người dùng chuyển qua lại giữa các mã (`FPT`, `VCB`, `HPG`, `BTCUSDT`...) hoặc đổi khung giờ (`15m`, `1H`, `4H`, `1D`), sau khi quá 60 giây cooldown (`resync_cooldown_ms = 60_000`) và dữ liệu cũ hơn `interval.duration_ms()`, lệnh `get_bars` khóa đồng bộ chờ `provider.fetch_history(...)` qua WebSocket (`tradingview-rs`) mất `1–2.5 giây` trước khi trả nến về frontend. Giao diện hiện badge `"Đang nạp dữ liệu..."` và khựng rõ rệt.
- **Hành vi chuẩn TradingView / Local-First:** Hễ dữ liệu đã có trong RAM LRU hoặc SQLite thì trả về tức thì (`< 2ms`) để Canvas vẽ ngay lập tức, đồng thời bắn tác vụ chạy ngầm (Non-Blocking Stale-While-Revalidate) kiểm tra nến mới nhất từ upstream, lưu vĩnh viễn vào SQLite và đẩy cập nhật mượt lên biểu đồ qua Tauri Channel mà không bao giờ chặn UI.
- **Nguyên nhân gốc rễ:**
  - `crates/sumi-data/src/cache_service.rs:243-369` (`MarketDataService::get_bars`)
  - `crates/sumi-desktop/src/commands/market.rs:597-610` (`get_bars` gọi `svc.get_bars(&req).await` thay vì Instant Return + Background Revalidation)

#### `BUG-002` — Thiếu tầng In-Memory RAM LRU Series Cache; nghẽn khóa đơn SQLite Mutex
- **Mức độ:** `CRITICAL`
- **Trạng thái:** `VERIFIED FIXED` (`v2.1.0`)
- **Hành vi lỗi hiện tại:** Mỗi lần chuyển mã/khung giờ, bật Multi-Chart 4 ô, làm mới Watchlist hoặc Paper Broker, backend đều phải gọi `tokio::task::spawn_blocking` và tranh chấp trên cùng một `Arc<Mutex<Connection>>` duy nhất của `SqliteMarketStore`.
- **Hành vi chuẩn TradingView / Kiến trúc hiệu năng cao:** Có lớp In-Memory LRU Series Cache (ví dụ giữ 64–128 chuỗi `(exchange, symbol, interval, adjustment)` gần nhất trong RAM kèm `RwLock`/mutex tốc độ micro-giây) kết hợp SQLite WAL. Các lần chuyển qua lại giữa mã quen thuộc và các khung `15m/1H/4H/1D` phục vụ trong `< 0.5ms`.
- **Nguyên nhân gốc rễ:**
  - `crates/sumi-data/src/cache_service.rs:59-63` (`MarketDataService` chưa có RAM LRU cache)
  - `crates/sumi-data/src/storage.rs:54-57` (mọi đọc/ghi đi qua một `Mutex<Connection>` duy nhất)

#### `BUG-003` — Bản đóng gói `/Applications/Sumi Supercharts.app` không tự đồng bộ sẵn dữ liệu nền CafeF / kho SQLite
- **Mức độ:** `HIGH`
- **Trạng thái:** `VERIFIED FIXED` (`v2.1.0`)
- **Hành vi lỗi hiện tại:** Kiểm tra thực tế cho thấy kho dự án `data/cache/sumi_market.db` có **473 mã / 951,006 nến** (86 MB), nhưng kho dữ liệu của bản cài đặt tại `~/Library/Application Support/com.sumi.supercharts/sumi_market.db` chỉ có **16 mã / 7,520 nến** (chỉ các mã kéo lẻ qua mạng). Khi chạy `/Applications/Sumi Supercharts.app`, `current_exe()` nằm ở `/Applications/...` nên `workspace_ancestor_dirs()` không tìm thấy `data/cache/sumi_market.db` hay `data/raw/cafef`.
- **Hành vi chuẩn:** Khi khởi động bản đóng gói, nếu SQLite ở `app_data_dir` chưa có đủ dữ liệu lịch sử (hoặc ít hơn ngưỡng CafeF seed), tự động phát hiện kho dữ liệu mẫu/SQLite nguồn hoặc thư mục CSV CafeF (`data/raw/cafef`, `Resources/data/...`, hoặc các đường dẫn workspace tiêu chuẩn trên máy) để đồng bộ nền an toàn ngay lập tức.
- **Nguyên nhân gốc rễ:**
  - `crates/sumi-desktop/src/state.rs:138-214` (`workspace_ancestor_dirs`, `resolve_database_paths`, `maybe_auto_seed_cafef`)

#### `BUG-004` — Bão Re-render React 60–120 lần/giây khi di chuột trên biểu đồ gây giật/lag công cụ vẽ
- **Mức độ:** `CRITICAL`
- **Trạng thái:** `VERIFIED FIXED` (`v2.1.0`)
- **Hành vi lỗi hiện tại:** Sự kiện `onCrosshairChange` của `KLineChart` gọi thẳng `publishCrosshairSync()` ghi vào state `syncedCrosshair` của Zustand trên từng pixel di chuột. Toàn bộ các component `KLineViewport` subscribe `syncedCrosshair` bị ép React re-render 60–120 lần/giây ngay cả khi chỉ đang mở 1 biểu đồ hoặc đang rê chuột đặt điểm vẽ!
- **Hành vi chuẩn TradingView:** Di chuyển con trỏ chữ thập và đồng bộ crosshair đa biểu đồ hoàn toàn ngoài React tree (0 React re-render!), điều phối trực tiếp giữa các instance Canvas qua một `CrosshairBus` (`Map<slotId, Chart>`) thuần TypeScript dùng `requestAnimationFrame` coalescing.
- **Nguyên nhân gốc rễ:**
  - `ui/src/store/workspaceStore.ts:103, 282, 499-502`
  - `ui/src/chart/KLineViewport.tsx:86, 267-271, 632-660`

#### `BUG-005` — Các `useEffect` thừa trong `KLineViewport` gây double-fetch `resetData()` và ghi đè trạng thái vẽ
- **Mức độ:** `HIGH`
- **Trạng thái:** `VERIFIED FIXED` (`v2.1.0`)
- **Hành vi lỗi hiện tại:**
  1. `KLineViewport.tsx:210-233` gọi `chart.setSymbol(...)` trong `queueMicrotask` ngay sau `callback(bars)` đầu tiên nếu precision lệch so với `guessPrecision`, kích hoạt thêm một vòng `resetData()` + `getBars()` lặp lại.
  2. `KLineViewport.tsx:494-503` gọi `overrideOverlay({ groupId: 'user_drawings', ... })` trên mount và mỗi khi `magnetMode` đổi, ghi đè cả trạng thái `lock` riêng lẻ của từng hình vẽ.
  3. `KLineViewport.tsx:469-491` phụ thuộc vào `isActive` mà không hủy progress overlay dở dang khi đổi pane hoặc ấn `Esc`.
- **Hành vi chuẩn:** Không kích hoạt double `resetData()`; không ghi đè thuộc tính riêng của từng overlay khi đổi `magnetMode`; cô lập sạch luồng vẽ khỏi các state không liên quan.
- **Nguyên nhân gốc rễ:**
  - `ui/src/chart/KLineViewport.tsx:210-233, 358-430, 469-503`

---

### Nhóm B: Đa Biểu Đồ Đa Khung Thời Gian (Multi-Timeframe Supercharts)

#### `BUG-006` — Lỗi trắng khung biểu đồ thứ 2/3/4 trong bố cục Multi-Chart cho tới khi bấm `%`
- **Mức độ:** `CRITICAL`
- **Trạng thái:** `VERIFIED FIXED` (`v2.1.0`)
- **Hành vi lỗi hiện tại:** Khi mở bố cục `2 dọc`, `2 ngang`, hoặc `4 ô` trên bản Tauri macOS (WKWebView), chỉ ô đầu tiên vẽ nến còn các ô mới mount bị trắng trơn cho đến khi người dùng bấm nút `%` (đổi thang giá).
- **Hành vi chuẩn TradingView:** Mọi ô biểu đồ trong các bố cục `2 dọc`, `2 ngang`, `3 ô`, `4 ô` phải đo kích thước sau layout paint, tự đồng bộ `ResizeObserver`, nạp nến tức thì và hiển thị đầy đủ nến + trục giá + khối lượng ngay lập tức mà không cần thao tác kích hoạt thủ công.
- **Nguyên nhân gốc rễ:**
  - `ui/src/chart/KLineViewport.tsx:686` (thiếu `min-w-0 min-h-0` trên grid child wrapper)
  - `ui/src/chart/KLineViewport.tsx:141-172` (`init(el)` gọi đồng bộ trước khi CSS Grid/Flexbox hoàn tất paint trong WKWebView -> `_chartBounding` có thể nhận kích thước `0` hoặc sai, làm `_totalBarSpace <= 0` và `_visibleRangeDataList = []`)
  - `ui/src/chart/KLineViewport.tsx:417-430` (Effect resize chỉ phụ thuộc `isMultiPane: boolean`, nên chuyển qua lại `2v`/`2h`/`3`/`4` không kích hoạt lại `resize()`; và sau khi `getBars` async hoàn tất không có post-frame `resize()` đảm bảo `forceBuildYAxisTick`)
  - Nút `%` chỉ gọi `overrideYAxis({ name: 'percentage' })` -> kích hoạt `layout({ forceBuildYAxisTick: true })`, chứng minh nến đã ở trong `_dataList` nhưng layout/Y-axis chưa được đo lại đúng kích thước.

#### `BUG-007` — Sai mặc định đồng bộ Multi-Chart (`sync.interval = true`, `sync.symbol = false`) và thiếu bố cục `3 ô`
- **Mức độ:** `CRITICAL`
- **Trạng thái:** `VERIFIED FIXED` (`v2.1.0`)
- **Hành vi lỗi hiện tại:** Mặc định trong `workspaceStore.ts:271` là `{ symbol: false, interval: true, crosshair: true }`. Khi người dùng chuyển Ô 1 sang `4H` để phân tích đa khung thời gian (VD: Ô 1 `1H`, Ô 2 `4H`, Ô 3 `1D`, Ô 4 `15m`), tất cả các ô bị ép nhảy sang `4H`! Ngược lại khi chọn mã mới ở Watchlist, chỉ 1 ô đổi mã còn các ô kia giữ mã cũ. Ngoài ra bộ chọn bố cục chỉ có `1 / 2v / 2h / 4`, thiếu bố cục `3 ô` theo yêu cầu.
- **Hành vi chuẩn TradingView:** Mặc định tối ưu cho phân tích đa khung thời gian: `sync.symbol = true`, `sync.interval = false`, `sync.crosshair = true`; 4 ô mặc định cùng mã `HOSE:FPT` với 4 khung thời gian riêng biệt (`1D`, `4h`, `1h`, `15m`); hỗ trợ đầy đủ `1 ô`, `2 dọc`, `2 ngang`, `3 ô`, `4 ô`.
- **Nguyên nhân gốc rễ:**
  - `ui/src/store/workspaceStore.ts:34-84, 271, 335`
  - `ui/src/types/market.ts:9` (`GridLayoutKind` thiếu `'3'`)
  - `ui/src/App.tsx:160-174` & `ui/src/components/TopHeader.tsx:344-368`

#### `BUG-008` — Thiếu thanh tiêu đề rõ ràng (Mã + Khung giờ + Trạng thái Active) trên từng ô Multi-Chart & chồng chữ legend ở khung nhỏ
- **Mức độ:** `MEDIUM`
- **Trạng thái:** `VERIFIED FIXED` (`v2.1.0`)
- **Hành vi lỗi hiện tại:** Trong chế độ đa biểu đồ (`2/3/4` ô), chỉ có đường viền mảnh 1px phân biệt ô đang chọn; không có thanh/nhãn rõ ràng hiển thị Mã + Khung giờ + nút chọn nhanh khung giờ trên từng ô. Ở chiều cao `150px` (chế độ 4 ô), chữ OHLC/EMA legend hơi đè nhãn giá cao nhất trên canvas.
- **Hành vi chuẩn TradingView:** Mỗi ô Multi-Chart hiển thị rõ nhãn `MÃ · KHUNG GIỜ` và chỉ dấu ô đang active (click vào ô nào thì thanh công cụ trên cùng lập tức điều khiển ô đó), đồng thời bố trí padding/chiều cao sub-pane hợp lý không chồng chữ.
- **Nguyên nhân gốc rễ:**
  - `ui/src/chart/KLineViewport.tsx:683-739`

---

### Nhóm C: Trải Nghiệm Chỉ Báo Trực Tiếp Trên Biểu Đồ (Study Legend UX)

#### `BUG-009` — Thiếu hoàn toàn 3 nút hành động trực tiếp trên tên chỉ báo (`👁 Ẩn/Hiện`, `⚙ Cài đặt`, `✕ Xóa`)
- **Mức độ:** `CRITICAL`
- **Trạng thái:** `VERIFIED FIXED` (`v2.1.0`)
- **Hành vi lỗi hiện tại:** Trên góc trên trái khung nến chính (`EMA(20,50)`) và khung phụ (`VOL`, `RSI`, `MACD`) chỉ hiện văn bản tĩnh. Rê chuột vào tên chỉ báo không có nút `👁` ẩn/hiện, không có nút `⚙` chỉnh tham số/màu sắc, và không có nút `✕` xóa ngay — người dùng buộc phải mở lại modal `Chỉ báo` mỗi lần muốn tắt hoặc chỉnh sửa.
- **Hành vi chuẩn TradingView:** Ngay cạnh mỗi dòng legend chỉ báo trên biểu đồ (cả khung chính lẫn mọi khung phụ) đều có cụm nút tương tác `👁` (Ẩn/Hiện), `⚙` (Mở hộp thoại chỉnh tham số & màu sắc tại chỗ), và `✕` (Xóa ngay), hoạt động qua `indicator.tooltip.features` và `onIndicatorTooltipFeatureClick` của `KLineChart v10`.
- **Nguyên nhân gốc rễ:**
  - `ui/src/chart/themes.ts:255-275` (`indicator.tooltip.features` không được khai báo)
  - `ui/src/chart/KLineViewport.tsx:266-286` (chưa subscribe `onIndicatorTooltipFeatureClick`)

#### `BUG-010` — Thiếu gắn ID duy nhất & tùy biến màu sắc/kiểu đường riêng cho từng chỉ báo cùng loại (VD: `EMA 20` + `EMA 50` + `EMA 200`)
- **Mức độ:** `HIGH`
- **Trạng thái:** `VERIFIED FIXED` (`v2.1.0`)
- **Hành vi lỗi hiện tại:** Khi tạo chỉ báo, `KLineViewport.tsx:384-392` không truyền `id: spec.id` và không truyền `styles` (màu đường, độ dày) vào `chart.createIndicator` / `chart.overrideIndicator`. Khi gắn 2 chỉ báo `EMA` riêng biệt, cả hai đều dùng cùng bảng màu mặc định `#2962FF` làm khó phân biệt và không có hộp thoại đổi màu/độ dày từng đường trực tiếp trên chart.
- **Hành vi chuẩn TradingView:** Mỗi instance chỉ báo có `id` độc lập (`EMA 20` xanh dương, `EMA 50` cam, `EMA 200` tím...), tự chọn màu khác nhau khi thêm nhiều đường cùng loại, và cho phép mở hộp thoại chỉnh cả chu kỳ lẫn màu sắc/độ dày nét trực tiếp từ nút `⚙` trên biểu đồ.
- **Nguyên nhân gốc rễ:**
  - `ui/src/chart/KLineViewport.tsx:374-413`
  - `ui/src/components/modals/IndicatorsModal.tsx:198-206, 281-315`

---

### Nhóm D: Công Cụ Vẽ, Thanh Công Cụ Nổi, Phím Tắt & Menu Chuột Phải

#### `BUG-011` — Không có Thanh Công Cụ Nổi (Floating Drawing Toolbar) khi chọn hình vẽ
- **Mức độ:** `CRITICAL`
- **Trạng thái:** `VERIFIED FIXED` (`v2.1.0`)
- **Hành vi lỗi hiện tại:** Khi nhấp chọn đường trendline, Fibonacci, hình chữ nhật hay bất kỳ hình vẽ nào trên biểu đồ, không có thanh công cụ nổi xuất hiện. Người dùng không thể đổi màu sắc, độ dày nét (`1px–4px`), kiểu nét (liền/đứt/chấm), khóa/mở khóa riêng hình đó, nhân bản hoặc xóa nhanh hình đó.
- **Hành vi chuẩn TradingView:** Nhấp vào bất kỳ hình vẽ nào sẽ hiện ngay **Floating Drawing Toolbar** nổi trên vùng chart (kéo di chuyển tự do được như TradingView), hỗ trợ đổi màu, đổi độ dày, đổi kiểu nét (`solid` / `dashed` / `dotted`), khóa/mở khóa riêng lẻ, nhân bản (Clone) và xóa ngay, tự động lưu vĩnh viễn vào SQLite.
- **Nguyên nhân gốc rễ:**
  - `ui/src/chart/KLineViewport.tsx:442-490` (chưa đăng ký `onSelected` / `onDeselected` trên overlays; chưa có component `FloatingDrawingToolbar`)

#### `BUG-012` — Thiếu phím tắt vẽ chuẩn TradingView (`Delete`/`Backspace`, `Esc` hủy vẽ/bỏ chọn, `Ctrl/Cmd+Z` Undo)
- **Mức độ:** `CRITICAL`
- **Trạng thái:** `VERIFIED FIXED` (`v2.1.0`)
- **Hành vi lỗi hiện tại:**
  - Chọn hình vẽ và nhấn `Delete` hoặc `Backspace` không có tác dụng (đã xác nhận thực nghiệm: trendline vẫn còn nguyên).
  - Nhấn `Cmd+Z` / `Ctrl+Z` không hoàn tác hình vừa vẽ hay vừa xóa.
  - Nhấn `Esc` khi đang vẽ dở một hình nhiều bước (VD: 1 điểm của Trendline hoặc 2 điểm của Channel) không gỡ bỏ progress overlay đang vẽ dở trên Canvas.
- **Hành vi chuẩn TradingView:** `Delete`/`Backspace` xóa ngay hình đang chọn (và ghi vào undo stack), `Esc` hủy ngay hình đang vẽ dở + thoát chế độ vẽ + bỏ chọn hình hiện tại, `Cmd/Ctrl+Z` hoàn tác thao tác vẽ/xóa/chỉnh sửa gần nhất.
- **Nguyên nhân gốc rễ:**
  - `ui/src/App.tsx:69-137`
  - `ui/src/components/LeftDrawingRail.tsx:32-37`
  - `ui/src/chart/KLineViewport.tsx:468-492`

#### `BUG-013` — Thiếu hoàn toàn Menu Chuột Phải (Custom Context Menu) trên biểu đồ
- **Mức độ:** `HIGH`
- **Trạng thái:** `VERIFIED FIXED` (`v2.1.0`)
- **Hành vi lỗi hiện tại:** Nhấp chuột phải trên vùng Canvas không hiển thị menu chuột phải của ứng dụng (`menuCount = 0`).
- **Hành vi chuẩn TradingView:** Chuột phải trên biểu đồ mở Custom Context Menu gọn gàng tại tọa độ chuột (tự kẹp không tràn mép viewport), bao gồm:
  - *Reset Chart View* (khôi phục góc nhìn & thang giá tự động)
  - *Cắt nến Replay tại đây* (cắt ngay đúng cây nến ở vị trí chuột phải!)
  - *Thêm Cảnh báo giá tại mức [giá chuột]*
  - *Đặt lệnh Limit Mua (LO) tại mức [giá chuột]* / *Đặt lệnh Limit Bán (LO) tại mức [giá chuột]*
  - *Xóa hình vẽ đang chọn* (khi có hình vẽ đang chọn) / *Xóa toàn bộ hình vẽ*
- **Nguyên nhân gốc rễ:**
  - `ui/src/chart/KLineViewport.tsx:683-739` (chưa bắt `onContextMenu` với `convertFromPixel`)

#### `BUG-014` — Thiếu cơ chế đảm bảo Auto-fit khi nhấp đúp trục giá & Reset Chart View (`Alt+R`)
- **Mức độ:** `MEDIUM`
- **Trạng thái:** `VERIFIED FIXED` (`v2.1.0`)
- **Hành vi lỗi hiện tại:** Không có phím tắt/lệnh *Reset Chart View* (`Alt+R`) và chưa đảm bảo nhấp đúp lên vùng trục giá bên phải luôn khôi phục `Auto-fit` thang giá kể cả khi đã zoom/kéo trục hoặc đang ở chế độ `%`/`Log`.
- **Hành vi chuẩn TradingView:** Nhấp đúp trục giá bên phải lập tức `Auto-fit` thang giá; `Alt+R` hoặc *Reset Chart View* trong menu chuột phải đặt lại `barSpace`, `scrollToRealTime`, và `Auto-fit` trục Y.
- **Nguyên nhân gốc rễ:**
  - `ui/src/chart/KLineViewport.tsx:141-300`

---

### Nhóm E: Bar Replay (Rèm Cắt Nến Nhân Quả)

#### `BUG-015` — Chọn điểm cắt Bar Replay hoàn toàn hỏng do đọc sai cấu trúc `onCandleBarClick` của KLineChart v10
- **Mức độ:** `CRITICAL`
- **Trạng thái:** `VERIFIED FIXED` (`v2.1.0`)
- **Hành vi lỗi hiện tại:** Khi bấm `Chọn điểm cắt` và click vào bất kỳ cây nến quá khứ nào (kể cả click trúng thân nến), con trỏ Replay bị kẹt cứng tại nến ban đầu (`Nến 421 / 480`), hoàn toàn không cắt về vị trí đã chọn.
- **Hành vi chuẩn TradingView:** Nhấp chuột tại bất kỳ hoành độ X nào trên vùng biểu đồ đều cắt chính xác về đúng cây nến tại vị trí đó và loại bỏ tức thì mọi cây nến tương lai phía bên phải.
- **Nguyên nhân gốc rễ:**
  - `ui/src/chart/KLineViewport.tsx:274-285`: đọc `payload?.data?.timestamp`, trong khi `research_repos/KLineChart/src/view/CandleBarView.ts:34-36` truyền `VisibleRangeData` có cấu trúc `{ dataIndex, x, data: { prev, current, next } }` (`payload.data.current.timestamp`)! Đồng thời `onCandleBarClick` chỉ bắn khi click trúng hình chữ nhật thân nến, không bắn khi click vùng nền biểu đồ phía trên/dưới nến.

#### `BUG-016` — Thiếu lớp phủ Rèm Cắt Nến Dọc (Interactive Scissor Curtain) chuẩn TradingView
- **Mức độ:** `CRITICAL`
- **Trạng thái:** `VERIFIED FIXED` (`v2.1.0`)
- **Hành vi lỗi hiện tại:** Khi bấm nút `Phát lại` trên `TopHeader`, ứng dụng cắt mù tại `totalBars - 60` mà không cho người dùng chọn điểm cắt trực quan. Khi bấm `Chọn điểm cắt`, chỉ hiện 1 dòng chữ tĩnh ở góc trên phải, không có đường rèm dọc xanh dương chạy theo chuột, không làm mờ vùng nến tương lai bên phải, và không hiện hộp nhãn ngày/giờ dưới chân rèm.
- **Hành vi chuẩn TradingView:** Bấm `Phát lại` hoặc `Chọn điểm cắt` kích hoạt ngay **Interactive Scissor Curtain**:
  - Đường dọc xanh dương (`#2962FF`) kèm biểu tượng kéo bám mượt 60–120 FPS theo tọa độ X của chuột trên toàn chiều cao biểu đồ.
  - Vùng bên phải đường rèm được làm mờ (translucent dim mask) thể hiện trực quan phần nến tương lai sắp bị cắt bỏ.
  - Hộp nhãn ngày/giờ chính xác của cây nến dưới chân rèm (cập nhật qua `convertFromPixel`).
  - Nhấp chuột 1 lần ở bất kỳ đâu trên biểu đồ → cắt bỏ tức thì toàn bộ nến tương lai phía sau và đưa về trạng thái sẵn sàng `Play/Pause`, `Step +1`, `Step -1`, đổi tốc độ và đặt lệnh Paper Trading nhân quả.
- **Nguyên nhân gốc rễ:**
  - `ui/src/components/TopHeader.tsx:259-274`
  - `ui/src/components/ReplayControlBar.tsx:67-79`
  - `ui/src/chart/KLineViewport.tsx:695-700`

#### `BUG-017` — Web fallback thiếu `replayJump` & chuyển mã/khung giờ trong lúc Replay không tự xử lý đồng bộ
- **Mức độ:** `MEDIUM`
- **Trạng thái:** `VERIFIED FIXED` (`v2.1.0`)
- **Hành vi lỗi hiện tại:**
  1. `ui/src/api/tauri.ts:294-296` gọi `webFallbackApi.replayStep(0)` thay vì hàm `replayJump(targetTimestampMs)` thực sự.
  2. Khi người dùng bấm `Phát` chạy tới cuối dữ liệu (`Completed`), hoặc đổi mã/khung giờ khi đang Replay, trạng thái Replay không tự đóng/cập nhật gọn gàng và `ReplayControlBar` ẩn mất khi `replaySelectingCut = true` nhưng chưa có phiên Replay (nếu bấm `Phát lại` ở chế độ chọn rèm trước).
- **Hành vi chuẩn:** Hỗ trợ chọn điểm cắt trước khi khởi tạo phiên Replay; `replayJump` hoạt động nhất quán cả Tauri Rust lẫn web fallback; đổi mã khi đang Replay tự thoát Replay sạch sẽ để tránh lệch mã.
- **Nguyên nhân gốc rễ:**
  - `ui/src/api/tauri.ts:294-297`, `ui/src/api/webFallback.ts:420-465`, `ui/src/store/workspaceStore.ts:384-407, 731-800`

---

### Nhóm F: Strategy Backtest & Tín Hiệu Trên Biểu Đồ

#### `BUG-018` — Strategy Backtest không hỗ trợ chọn khoảng thời gian (`Từ ngày` – `Đến ngày`) và đồng bộ vùng Replay
- **Mức độ:** `CRITICAL`
- **Trạng thái:** `VERIFIED FIXED` (`v2.1.0`)
- **Hành vi lỗi hiện tại:** Backtest luôn chạy cố định trên `maxBars = 2500` cây nến gần nhất; không có ô chọn `Từ ngày` / `Đến ngày` và không có nút `"Lấy theo vùng Replay hiện tại"` để kiểm thử đúng giai đoạn thị trường mong muốn.
- **Hành vi chuẩn TradingView:** Có bộ chọn `Từ ngày` (`fromDate`) – `Đến ngày` (`toDate`) lọc chính xác chuỗi nến ở backend Rust (`sumi-desktop/src/commands/backtest.rs` + giữ warmup lookback hợp lý hoặc lọc đúng cửa sổ nến) kèm nút `"Lấy theo vùng Replay"` một chạm.
- **Nguyên nhân gốc rễ:**
  - `ui/src/components/dock/StrategyTesterTab.tsx:88-126, 173-303`
  - `crates/sumi-desktop/src/commands/backtest.rs:20-47, 215-246`

#### `BUG-019` — Form tham số Strategy Backtest cố định cứng vào `Fast/Slow` cho mọi chiến lược
- **Mức độ:** `CRITICAL`
- **Trạng thái:** `VERIFIED FIXED` (`v2.1.0`)
- **Hành vi lỗi hiện tại:** Dù người dùng chuyển chiến lược sang `RSI Mean Reversion`, `MACD Momentum`, `Bollinger Squeeze Breakout`, `Donchian ATR Breakout`, hay `RSI + BB Confluence`, giao diện vẫn chỉ hiển thị hai ô `Fast/Slow` của MA (`fastSma`/`slowEma`)! Hơn nữa `StrategyBacktestInput` ở Rust thậm chí không có các trường `macdFast`, `macdSlow`, `macdSignal`, `macdTrendEma` dù `sumi-core::backtest::StrategyParams` đã hỗ trợ đầy đủ!
- **Hành vi chuẩn TradingView:** Form tham số thay đổi động theo đúng chiến lược được chọn:
  - `SMA/EMA`: Fast SMA, Slow EMA
  - `RSI Mean Reversion`: RSI Period, Oversold, Overbought
  - `MACD Momentum`: Fast, Slow, Signal (và Trend EMA)
  - `Bollinger Squeeze`: BB Period, StdDev Multiplier, Max Squeeze BW%
  - `Donchian ATR`: Entry Channel, Exit Channel
  - `RSI + BB Confluence`: RSI Period, Oversold/Overbought + BB Period, StdDev
- **Nguyên nhân gốc rễ:**
  - `ui/src/components/dock/StrategyTesterTab.tsx:90-96, 108-126, 210-226`
  - `crates/sumi-desktop/src/commands/backtest.rs:36-46, 160-188`

#### `BUG-020` — Kết quả Backtest không hiển thị mũi tên Khớp Mua (`▲`) / Khớp Bán (`▼`) trực tiếp lên nến biểu đồ
- **Mức độ:** `CRITICAL`
- **Trạng thái:** `VERIFIED FIXED` (`v2.1.0`)
- **Hành vi lỗi hiện tại:** Khi chạy Backtest xong, các giao dịch chỉ hiển thị trong bảng text ở tab `Danh sách lệnh`. Trên biểu đồ nến chính hoàn toàn không có mũi tên Mua (`▲` xanh) và mũi tên Bán (`▼` đỏ), không có nút bật/tắt tín hiệu Backtest trên biểu đồ, và nhấp vào dòng lệnh cũ hơn 320 nến không tự nạp lịch sử để cuộn tới đúng cây nến đó.
- **Hành vi chuẩn TradingView:** Tự động vẽ rõ mũi tên Mua (`▲` xanh phía dưới giá thấp nến vào) và Bán (`▼` đỏ phía trên giá cao nến thoát, kèm nhãn PnL/hướng lệnh gọn) trực tiếp lên nến biểu đồ chính, có nút bật/tắt hiển thị tín hiệu Backtest trên chart, và nhấp vào dòng lệnh trong bảng tự động cuộn tâm biểu đồ tới đúng cây nến đó.
- **Nguyên nhân gốc rễ:**
  - `ui/src/components/dock/StrategyTesterTab.tsx:128-139` (chưa đẩy `trades` vào workspace store cho `KLineViewport` render)
  - `ui/src/chart/overlays/tradingViewOverlays.ts` (chưa có custom overlay `backtestTradeMarker`)
  - `ui/src/chart/KLineViewport.tsx:662-668`

---

### Nhóm G: Stock Screener & Hoàn Thiện Giao Diện

#### `BUG-021` — Bộ lọc Stock Screener `"Tất cả sàn VN"` lẫn mã quốc tế (`AAPL`/`NVDA`) & lỗi bộ lọc xu hướng trong Web Fallback
- **Mức độ:** `MEDIUM`
- **Trạng thái:** `VERIFIED FIXED` (`v2.1.0`)
- **Hành vi lỗi hiện tại:** Khi chọn `Tất cả sàn VN` (`ALL`), `scanner.rs:161-166` truyền `ex_arg = None` vào `search_local_symbols`, kéo cả mã quốc tế/crypto đã lưu cache (`NASDAQ:AAPL`, `NASDAQ:NVDA`, `BINANCE:...`) vào kết quả sàn VN. Đồng thời đổi dropdown không tự chạy lại bộ lọc hoặc trong `webFallbackApi` bỏ qua `regimeFilter`.
- **Hành vi chuẩn:** `"Tất cả sàn VN"` chỉ quét các mã thuộc `HOSE`, `HNX`, `UPCOM`; áp dụng chính xác mọi bộ lọc `exchangeFilter`, `regimeFilter`, `categoryFilter`, `signalId`, `searchQuery`.
- **Nguyên nhân gốc rễ:**
  - `crates/sumi-desktop/src/commands/scanner.rs:161-172`
  - `ui/src/api/webFallback.ts:815-890`

#### `BUG-022` — Thông báo Toast góc dưới trái che mất nút `Đặt Mua / Đặt Bán` của Paper Trading
- **Mức độ:** `LOW`
- **Trạng thái:** `VERIFIED FIXED` (`v2.1.0`)
- **Hành vi lỗi hiện tại:** Khi Bottom Trading Dock đang mở ở tab Paper Trading, toast xuất hiện tại `fixed bottom-4 left-14` nằm đè trực tiếp lên nút gửi lệnh ở góc dưới bên trái.
- **Hành vi chuẩn:** Đặt Toast ở vị trí không che khuất phiếu lệnh hay chi tiết mã (ví dụ `fixed top-12 right-4` gọn gàng, có thể đóng nhanh).
- **Nguyên nhân gốc rễ:**
  - `ui/src/App.tsx:252`

---

## 3. Kết Quả Nghiệm Thu & Bằng Chứng Trực Quan (Phase 4 — `v2.1.0`)

Toàn bộ **22 / 22** lỗi (`BUG-001` .. `BUG-022`) đã được khắc phục ở tầng kiến trúc gốc và kiểm chứng lại 100% qua bộ test tự động Rust + kiểm thử tương tác trực tiếp trên trình duyệt `1440×900` (**0 `console.error`**, **0 `pageerror`**, **0 failed requests**):

### Bằng chứng hình ảnh xác minh thực tế (`docs/testing/v2_1_evidence/`)
1. **Đa biểu đồ 4 khung thời gian (`1D / 4H / 1H / 15m`) đồng bộ Mã, độc lập Khung giờ, 4/4 ô vẽ đầy đủ nến:** `docs/testing/v2_1_evidence/01_multichart_4panes_1D_4H_1H_15m.png` (`BUG-006`, `BUG-007`, `BUG-008`)
2. **Chỉ báo trên biểu đồ kèm nút `👁 ⚙ ✕` & đa đường `EMA(20,50)` + `EMA(200)` khác màu độc lập:** `docs/testing/v2_1_evidence/02_indicators_onchart_multi_ema.png` (`BUG-009`, `BUG-010`)
3. **Hộp thoại `Cài đặt chỉ báo` mở trực tiếp từ nút `⚙` trên biểu đồ (chỉnh tham số + bảng màu + độ dày nét):** `docs/testing/v2_1_evidence/03_indicator_settings_modal_from_chart.png` (`BUG-009`, `BUG-010`)
4. **Thanh Công Cụ Vẽ Nổi (Floating Drawing Toolbar) kéo thả được (đổi màu đỏ, 3px, nét đứt, khóa, nhân bản, xóa + Undo `Cmd/Ctrl+Z` + Context Menu):** `docs/testing/v2_1_evidence/04_floating_drawing_toolbar.png` (`BUG-004`, `BUG-011`, `BUG-012`, `BUG-013`, `BUG-014`)
5. **Bar Replay Rèm Cắt Nến Dọc (Interactive Scissor Curtain) bám mượt theo chuột, làm mờ vùng tương lai bên phải, hiển thị ngày/giờ cắt chính xác:** `docs/testing/v2_1_evidence/05_bar_replay_scissor_curtain.png` (`BUG-015`, `BUG-016`, `BUG-017`)
6. **Strategy Backtest với bộ chọn `Từ ngày – Đến ngày`, tham số động theo từng chiến lược, và mũi tên Khớp Mua (`▲` xanh) / Khớp Bán (`▼` đỏ/xanh) vẽ trực tiếp lên nến:** `docs/testing/v2_1_evidence/06_backtest_chart_buy_sell_markers.png` (`BUG-018`, `BUG-019`, `BUG-020`)

### Xác minh tầng Rust & dữ liệu nền (`BUG-001`, `BUG-002`, `BUG-003`, `BUG-018`, `BUG-019`, `BUG-021`)
- `sumi-data`: `ram_cache::tests::ram_lru_cache_evicts_least_recently_used_series`, `ram_cache::tests::ram_cache_merges_live_updates_and_appends_monotonically`, `cache_service::tests::stale_while_revalidate_returns_instantly_and_refreshes_in_background`, `storage::tests::merge_from_sqlite_path_imports_missing_history_and_preserves_newer_bars` — **ALL PASS**.
- `sumi-desktop`: `commands::backtest::tests::build_backtest_config_wires_all_dynamic_strategy_params`, `commands::backtest::tests::filter_bars_by_range_slices_dates_and_rejects_inverted_window`, `commands::replay::tests::resolve_replay_cut_index_cuts_exact_bar_with_zero_future_leak`, `commands::scanner::tests::signal_catalog_exposes_all_101_signals` — **ALL PASS**.
- Cơ sở dữ liệu `~/Library/Application Support/com.sumi.supercharts/sumi_market.db` đã được đồng bộ đầy đủ kho dữ liệu lịch sử CafeF (`>= 473` mã, `>= 951,000` nến) và tự động merge ở lần khởi động đầu trên máy mới.
