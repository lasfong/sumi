# 04. Báo Cáo Đo Đạc Hiệu Năng Thực Tế & Kiểm Định Rò Rỉ Tài Nguyên — Sumi Supercharts (`v2.1.0`)

> **Môi trường đo thực tế (100% số liệu trực tiếp trên máy kiểm định — không ước lượng):**
> - **Phần cứng:** Apple M1 (8 nhân ARM64), **16 GB RAM** (`17.179.869.184` bytes).
> - **Hệ điều hành:** macOS `27.2.1` (Build `26B131`).
> - **Toolchain:** Rust `1.99.1` (`--release`), Tauri v2 (`2.12.7`), SQLite `3.54.0` (WAL), React `19`, `klinecharts@10.1.1`, Chromium arm64 (HiDPI `2×`, `1440×900`, `--enable-precise-memory-info`).
> - **CSDL thật:** `sumi_market.db` (**84 MB** + `1.5 MB` WAL, **486 mã**, **492 chuỗi**, **958.359 nến** lịch sử thực tế VN + Crypto + US + FX) & `sumi_workspace.db` (8 bảng WAL).
> - **Nguyên tắc:** Tầng A đo trực tiếp trên `/Applications/Sumi Supercharts.app`; Tầng B đo qua `qa_bridge` biên dịch `--release` nối thẳng vào `AppState` Rust thật + `ui/dist` nguyên bản (tuyệt đối không chạy qua `webFallback.ts`).

---

## 1. Hiệu Năng Ứng Dụng macOS Native Thật (Tầng A — `TC-PERF-001`)

### 1.1. Thời Gian Khởi Động (3 Chu Kỳ Đóng Hoàn Toàn & Mở Lại `/Applications/Sumi Supercharts.app`)

| Chu kỳ đo | PID | Thời gian Spawn Tiến trình (`ms`) | Thời gian Cửa sổ Hiện (`ms`) | Tổng RSS Sau Khởi Động (`MB`) | Trạng thái DB & Socket |
|-----------|-----|-----------------------------------|-------------------------------|-------------------------------|------------------------|
| Warm Start #1 | `5914` | `98.1` | `842.4` | `252.43` | 6 handle SQLite (market+workspace+WAL/SHM), 2 HTTPS sockets TV |
| Warm Start #2 | `5942` | `128.8` | `603.8` | `250.76` | 6 handle SQLite, Idle CPU `0.6%` |
| Warm Start #3 (Persistence) | `5963` | `132.1` | `575.7` | `249.89` | Khôi phục bản ghi `qa_sentinel` SQLite nguyên vẹn |
| **Trung vị (`p50`) / Trung bình** | — | **`128.8` / `119.7 ms`** | **`603.8` / `674.0 ms`** | **`250.76 MB`** | **Đạt SLA `< 1.000 ms`** |

- **Cold Start (CSDL trống hoàn toàn, tự seed + nạp CafeF/TradingView — `TC-DATA-001`):** **`4,937.7 ms`** (gồm tạo 3 bảng `sumi_market.db`, 8 bảng `sumi_workspace.db`, 4 watchlist mặc định và nạp dữ liệu nền từ `data/raw/cafef` ~951k nến).

### 1.2. Phân Bổ Bộ Nhớ `phys_footprint` & CPU Lúc Rảnh (4 Tiến Trình Native — PID `94637`)

| Tiến trình | `phys_footprint` Lúc Rảnh (`MB`) | Đỉnh `phys_footprint` (`MB`) | RSS Sau Boot (`MB`) | Ghi chú |
|------------|----------------------------------|------------------------------|---------------------|---------|
| `sumi-desktop` (Rust Core + SQLite) | `54.0` | `57.0` | `119.47` | Rất gọn cho CSDL 84 MB (958k nến) |
| `com.apple.WebKit.WebContent` | `116.0` | **`626.0`** | `89.84` | Đỉnh 626 MB khi chuyển nhiều khung/biểu đồ |
| `com.apple.WebKit.GPU` | `31.0` | `68.0` | `28.98` | Tăng tốc vẽ Canvas 2D Metal |
| `com.apple.WebKit.Networking` | `6.6` | `7.2` | `14.14` | HTTP/WebSocket stack |
| **TỔNG TOÀN ỨNG DỤNG** | **`207.6 MB`** | `~758 MB` | **`252.43 MB`** | **Idle CPU: `0.6% – 1.3%`** (Đạt SLA `< 350 MB`, `< 3% CPU`) |

> **Đánh giá đối sánh TradingView Desktop (Electron):** Nhờ kiến trúc **Tauri v2 + WKWebView + Rust native**, Sumi Supercharts vượt trội rõ rệt về khởi động ấm (**`~0.60s`** so với `~2.2–4.0s` của Electron) và tiêu thụ RAM nền (**`207.6 MB`** so với `~650–1.200 MB` của TradingView Desktop). Tuy nhiên đỉnh `WebContent` đạt **`626 MB`** cho thấy cần xử lý các rò rỉ tài nguyên phát hiện tại Mục 5.

---

## 2. Phân Bố Độ Trễ Truy Xuất Dữ Liệu 3 Tầng (`get_bars` — `TC-PERF-002`)

Đo trên **30 mã đa dạng** (`HOSE`, `HNX`, `UPCOM`, `BINANCE`, 600 nến/lượt) sau khi làm sạch RAM cache (`/qa/clear-ram-cache`) để tách bạch **L2 SQLite** và **L1 RAM LRU**, cùng **8 khung thời gian** (`1m..1W`) + 5 cặp chưa có trong cache cho **L3 TradingView Online**:

| Tầng dữ liệu | Số mẫu (`n`) | `min` (`ms`) | `p50` (`ms`) | `p95` (`ms`) | `p99` (`ms`) | `max` (`ms`) | Trung bình (`ms`) | SLA | Đánh giá |
|--------------|--------------|--------------|--------------|--------------|--------------|--------------|-------------------|-----|----------|
| **L1 RAM LRU (`serverMs`)** | 30 | `0.254` | **`0.620`** | **`0.685`** | `0.705` | `1.068` | `0.627` | `< 5 ms` | **Xuất sắc (PASS)** |
| **L1 RAM LRU (`clientMs` HTTP)** | 30 | `2.789` | `4.311` | `8.378` | `10.361` | `13.632` | `5.403` | — | Bao gồm JSON HTTP loopback |
| **L2 SQLite (`serverMs`)** | 30 | `1.235` | **`2.344`** | **`7.414`** | `9.454` | `9.876` | `3.633` | `< 30 ms` | **Xuất sắc (PASS)** |
| **L2 SQLite (`clientMs` HTTP)** | 30 | `3.425` | `5.116` | `10.563` | `12.682` | `13.730` | `6.221` | — | Đọc 600 nến từ DB 84 MB `WITHOUT ROWID` |
| **L3 TradingView Online (`serverMs`)** | 9 | `1.086` | **`809.56`** | **`1,323.84`** | `1,323.84` | **`13,896.72`** | `2,118.34` | `< 2.500 ms` | **p95 PASS, nhưng tail spike 13.9s!** |

### Chi Tiết Theo 8 Khung Thời Gian (`BINANCE:BTCUSDT`, 300 nến)

| Khung giờ | Nguồn trả về | `serverMs` (`ms`) | `clientMs` (`ms`) | Ghi chú |
|-----------|--------------|-------------------|-------------------|---------|
| `1m` | `cache_hit` | `1.482` | `4.185` | Đã có sẵn từ bài kiểm tra live stream |
| `5m` | `remote_live` | **`13,896.72`** | `13,899.62` | **Đột biến độ trễ gần 14s** do WS handshake / chờ series trên TradingView! |
| `15m` | `cache_hit` | `1.723` | `3.578` | Từ L2/L1 cache |
| `30m` | `cache_hit` | `0.818` | `2.519` | Từ L1 cache |
| `1h` | `remote_live` | `1,323.84` | `1,325.69` | Kéo mạng trực tuyến ổn định |
| `4h` | `remote_live` | `751.05` | `752.24` | Kéo mạng trực tuyến nhanh |
| `1D` | `cache_hit` | `0.110` | `1.098` | L1 RAM hit cực nhanh (`110 µs`) |
| `1W` | `remote_live` | `809.56` | `810.60` | Kéo mạng trực tuyến nhanh |

> **Nhận xét kỹ thuật:** Kiến trúc bảng `WITHOUT ROWID` với khóa chính `(exchange, symbol, interval, adjustment, timestamp_ms)` + `SeriesRamCache` hoạt động **cực kỳ xuất sắc** (`0.62 ms` RAM, `2.34 ms` SQLite cho 600 nến). Tuy nhiên kết nối lạnh tới TradingView WebSocket có đuôi độ trễ tới **`13.9 giây`** (`5m`), đòi hỏi cơ chế timeout mềm + phản hồi trạng thái rõ ràng cho người dùng.

---

## 3. Hiệu Năng Đồ Họa Canvas (`requestAnimationFrame` FPS & Long Frames — `TC-PERF-003..006`)

Đo trực tiếp trên `ui/dist` nguyên bản ở độ phân giải **HiDPI `2×` (`2880×1800` vật lý)** trong Chromium arm64 (chu kỳ VSync `60 Hz = 16.667 ms`):

| Kịch bản Stress | Số khung (`frames`) | Avg FPS | `min` (`ms`) | `p50` (`ms`) | `p95` (`ms`) | `p99` (`ms`) | `max` (`ms`) | Khung `>20ms` (rớt thật) | Ngưỡng kỹ thuật (`dt > 16.7ms`)* | Đánh giá |
|-----------------|---------------------|---------|--------------|--------------|--------------|--------------|--------------|--------------------------|----------------------------------|----------|
| **1. Pan/Zoom 2.400+ nến + 5 Chỉ báo + 20 Hình vẽ** (`TC-PERF-003`) | 499 | **`60.0`** | `16.5` | `16.7` | `16.8` | `16.8` | **`16.8`** | **`0` (0.0%)** | `146 (29.26%)` | **PASS** |
| **2. Quét Crosshair Lưới 4 Ô `2×2`** (`TC-PERF-004`)† | 240 | **`60.0`** | `16.5` | `16.7` | `16.8` | `16.8` | **`16.8`** | **`0` (0.0%)** | `45 (18.75%)` | **PASS†** |
| **3. Kéo Điểm Neo Hình Vẽ + FloatingToolbar** (`TC-PERF-005`) | 176 | **`60.0`** | `16.5` | `16.7` | `16.8` | `16.8` | **`16.8`** | **`0` (0.0%)** | `63 (35.80%)` | **PASS** (`0` ghi DB khi kéo, `1` khi thả) |
| **4. Bar Replay Chạy Tự Động `10x`** (`TC-PERF-006`) | 277 | **`60.0`** | `16.6` | `16.7` | `16.8` | `16.8` | **`16.8`** | **`0` (0.0%)** | `103 (37.18%)` | **PASS** (`44` bước; `replay_step` p95 `0.238ms`) |

> **Giải thích minh bạch về ngưỡng VSync `16.7 ms` và `120 Hz (8.33 ms)`:**
> - `*` Chu kỳ 60Hz chuẩn là `16.6667 ms`. Trên Chromium headless 60Hz, `rAF` dao động nhịp siêu nhỏ trong khoảng **`16.5 – 16.8 ms`** (`maxFrameMs = 16.8 ms` trên cả 4 kịch bản!). Vì thế bộ đếm cơ học `dt > 16.7 ms` ghi nhận 18%–37% mẫu `16.8 ms` (chênh `0.1 ms` nhiễu đồng hồ VSync), nhưng số **Long Frames thực sự bỏ lỡ khung (`> 20 ms`) là `0/1,192` khung hình (`0.00%`)**!
> - Ngưỡng `120 Hz (8.33 ms)` báo `100%` vì trình duyệt headless và màn hình Apple M1 bị khóa cứng ở `60 Hz` (`16.67 ms`), không kích hoạt được ProMotion 120Hz.
> - `†` Lưu ý quan trọng cho Kịch bản 2 (`TC-PERF-004`): Do `BUG-004` làm `CrosshairBus` luôn gửi `null`, 3 ô còn lại không thực sự vẽ lại crosshair khi quét chuột; cần đo kiểm lại kịch bản này sau khi vá `BUG-004`.
> - **Độ trễ IPC `replay_step` ở tốc độ `10x` (44 bước liên tiếp):** `serverMs` **`min=0.209 ms` / `p50=0.220 ms` / `p95=0.238 ms` / `max=0.444 ms`**; `clientMs` `p50=1.30 ms` / `p95=1.40 ms` / `max=3.30 ms`. Đủ dư địa chạy mượt tới `20x` (`50 ms/bước`).

---

## 4. Thông Lượng Lõi Định Lượng Rust (`sumi-core` + `sumi-desktop` — `TC-PERF-007`)

Tất cả đo trên binary `--release` với dữ liệu thật `sumi_market.db` (Apple M1):

| Tác vụ Định lượng | Khối lượng dữ liệu | Số vòng | `min` (`ms`) | `p50` (`ms`) | `p95` (`ms`) | `max` (`ms`) | SLA | Đánh giá |
|-------------------|--------------------|---------|--------------|--------------|--------------|--------------|-----|----------|
| **Backtest: `SMA/EMA Cross`** | `HOSE:FPT` (2.500 nến, 18 lệnh) | 5 | `0.583` | `0.606` | `0.641` | `0.663` | `< 100 ms` | **Cực nhanh (160× vượt SLA)** |
| **Backtest: `RSI Mean Reversion`** | `HOSE:FPT` (2.500 nến, 14 lệnh) | 5 | `0.575` | `0.589` | `0.589` | `0.634` | `< 100 ms` | **Cực nhanh** |
| **Backtest: `MACD Momentum`** | `HOSE:FPT` (2.500 nến, 74 lệnh) | 5 | `0.676` | `0.721` | `0.950` | `1.076` | `< 100 ms` | **Cực nhanh** |
| **Backtest: `Bollinger Squeeze`** | `HOSE:FPT` (2.500 nến, 51 lệnh) | 5 | `0.607` | `0.618` | `0.623` | `0.645` | `< 100 ms` | **Cực nhanh** |
| **Backtest: `Donchian ATR`** | `HOSE:FPT` (2.500 nến, 40 lệnh) | 5 | `0.726` | `0.745` | `0.765` | `0.781` | `< 100 ms` | **Cực nhanh** |
| **Backtest: `RSI + BB Confluence`** | `HOSE:FPT` (2.500 nến, 21 lệnh) | 5 | `0.583` | `0.598` | `0.598` | `0.619` | `< 100 ms` | **Cực nhanh** |
| **Tổng hợp 6 Presets Backtest** | 2.500 nến × 30 lượt | 30 | `0.575` | **`0.619`** | **`0.781`** | `1.076` | `< 100 ms` | **PASS** |
| **Walk-Forward Optimization (WFO)** | 4 Folds Rolling (2.500 nến, 12 sweep/fold) | 5 | `2.087` | **`2.134`** | **`2.159`** | `2.163` | `< 600 ms` | **PASS (280× vượt SLA)** |
| **Monte Carlo Bootstrap** | 1.000 kịch bản (`SplitMix64`) | 5 | `0.981` | **`0.990`** | **`1.001`** | `1.021` | `< 300 ms` | **PASS (300× vượt SLA)** |
| **Stock Screener (Mặc định 173 mã)** | 101 tín hiệu × 173 mã VN | 3 | `615.38` | **`811.61`** | `811.61` | **`2,115.03`** | `< 1.500 ms` | **Tail vượt SLA** |
| **Stock Screener (Toàn thị trường 469 mã, ấm)** | 101 tín hiệu × 469 mã VN | 3 | `1,053.15` | **`1,197.76`** | `1,197.76` | **`1,863.71`** | `< 1.500 ms` | **Thông lượng `391.6 mã/s`; max vượt SLA** |
| **Stock Screener (Toàn thị trường 469 mã, lạnh)** | 101 tín hiệu × 469 mã VN (từ `TC-QUANT-007`) | 1 | — | **`5,412.70`** | — | **`5,412.70`** | `< 1.500 ms` | **FAIL (chậm 3.6× SLA!)** |

> **Phát hiện tương phản nổi bật:** Động cơ Backtest/WFO/Monte Carlo viết bằng thuần Rust đạt tốc độ **đẳng cấp HFT/hệ thống tổ chức** (`0.62 ms` cho 2.500 nến, `2.13 ms` cho cả 4 fold WFO, `0.99 ms` cho 1.000 kịch bản Monte Carlo). Ngược lại, **Stock Screener là điểm nghẽn CPU/IO lớn nhất toàn hệ thống** (`5.41s` lạnh, `1.20s–1.86s` ấm) do tính trùng 2 lần bộ chỉ báo mỗi mã và chạy đơn luồng.

---

## 5. Kiểm Định Chịu Tải & Rò Rỉ Tài Nguyên (110-Operation Soak Test — `TC-PERF-008`)

Thực thi liên tục **110 thao tác cường độ cao** (50 lần đổi mã qua Watchlist + 20 lần bật/tắt lưới `1 ô ↔ 4 ô` + 20 chu kỳ lưu chỉ báo + 20 chu kỳ thêm/xóa hình vẽ), chờ ổn định 2 giây và ép Chromium `HeapProfiler.collectGarbage()`:

| Chỉ số Tài nguyên | Trước Soak (Baseline) | Sau 110 Thao tác + GC | Biến thiên (Delta) | Ngưỡng SLA | Kết quả |
|-------------------|-----------------------|-----------------------|--------------------|------------|---------|
| **`AppState.bar_streams` (Rust Tokio tasks)** | `1` | **`4`** | **`+3` (RÒ RỈ!)** | `<= 1` | **FAIL (`BUG-012`)** |
| **Tauri Channel Callbacks (JS Map)** | `6` | **`84`** | **`+78` (RÒ RỈ!)** | Ổn định (`<= 10`) | **FAIL (`BUG-012`)** |
| **JS Heap Used (`Runtime.getHeapUsage`)** | `6.52 MB` | `10.76 MB` | `+4.24 MB` (+65%) | `< 30 MB` | Đạt ngưỡng nhưng tăng đều theo callbacks |
| **Rust Backend RSS (`qa_bridge`)** | `101.77 MB` | `116.16 MB` | `+14.39 MB` (+14%) | `< 40 MB` | Chịu ảnh hưởng của RAM LRU + 3 stream rò |
| **DOM Nodes (`Performance.getMetrics`)** | `1,717` | **`2,967`** | **`+1,250` (+73%)** | Ổn định | **Cảnh báo rò DOM / modal detached** |
| **JS Event Listeners (`Performance.getMetrics`)** | `408` | **`842`** | **`+434` (+106%)** | Ổn định | **Cảnh báo rò listener qua chu kỳ mount/unmount** |

---

## 6. Lộ Trình Tối Ưu Hóa Kiến Trúc & Hiệu Năng (Ưu Tiên Theo Tác Động)

| Ưu tiên | Hạng mục tối ưu | Vị trí mã nguồn (`file:line`) | Cơ chế bottleneck / rò rỉ | Mức cải thiện kỳ vọng |
|---------|-----------------|-------------------------------|---------------------------|-----------------------|
| **1 (P1)** | Khử rò rỉ `bar_streams` & Tauri Channel callbacks | [`commands/market.rs:678-808`](file:///Users/mizuhara/workspace/sumi/crates/sumi-desktop/src/commands/market.rs#L678-L808), [`KLineViewport.tsx:541,640`](file:///Users/mizuhara/workspace/sumi/ui/src/chart/KLineViewport.tsx#L541) | Race bất đồng bộ `subscribe_bars` vs `unsubscribe_bars` khi unmount nhanh + không gọi `unregisterCallback` | Giảm stream rò từ `4 → 1`, Channel callbacks từ `84 → <= 4`, chặn phình `WebContent` peak `626 MB` |
| **2 (P1)** | Loại bỏ tính trùng 2 lần & song song hóa Stock Screener | [`commands/scanner.rs:265, 283`](file:///Users/mizuhara/workspace/sumi/crates/sumi-desktop/src/commands/scanner.rs#L265), [`scanner/catalog.rs:2696-2697`](file:///Users/mizuhara/workspace/sumi/crates/sumi-core/src/scanner/catalog.rs#L2696-L2697) | Gọi cả `evaluate_symbol_scan` lẫn `PrecomputedCatalogMatrix::compute` tuần tự; tính trùng Donchian/EMA/ATR | Giảm Screener 469 mã từ **`1.20s–5.41s` → `< 200 ms`** (~6×–20× nhanh hơn với `rayon` 8 nhân M1 + SQLite batch query) |
| **3 (P2)** | Khử DOM Churn hàng Watchlist khi giá live nhảy | [`RightWidgetBar.tsx:265`](file:///Users/mizuhara/workspace/sumi/ui/src/components/RightWidgetBar.tsx#L265) | `flash?.nonce` nằm trong React `key` gây hủy & tạo lại toàn bộ DOM hàng (30 remount/12s) | Giảm remount DOM từ `30 → 0`, mượt hơn khi bật nhiều mã Crypto/VN30 thanh khoản lớn |
| **4 (P2)** | Khử gọi trùng `broker_get_state` lúc khởi động & tối ưu `materialize_candles` trong Replay | [`PaperBrokerTab.tsx`](file:///Users/mizuhara/workspace/sumi/ui/src/components/dock/PaperBrokerTab.tsx), [`replay/controller.rs:489-509`](file:///Users/mizuhara/workspace/sumi/crates/sumi-core/src/replay/controller.rs#L489-L509) | Khởi động bắn 2 lệnh `broker_get_state` song song; `ReplayController::materialize_candles` duyệt lại toàn bộ `base_candles` mỗi bước | Giảm thời gian boot IPC ~12 ms; tăng khả năng mở rộng Replay lên chuỗi >10.000 nến intraday |
| **5 (P3)** | Khử cấp phát `Vec<f64>` tạm mỗi lần định dạng nhãn giá trong `sumi-core` & tối ưu `CCI`/`MarketProfile` | [`viewport/price_scale.rs:1065-1071`](file:///Users/mizuhara/workspace/sumi/crates/sumi-core/src/viewport/price_scale.rs#L1065-L1071), [`indicators/extended.rs:692`](file:///Users/mizuhara/workspace/sumi/crates/sumi-core/src/indicators/extended.rs#L692) | `format_price` tạo và vứt bỏ `Vec` nice ticks trên từng lần format; `compute_cci` là `O(N × period)` | Giảm cấp phát heap nóng trên đường dẫn định dạng & tăng tốc Screener dài hạn |
