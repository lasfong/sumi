# Báo Cáo Xác Minh Phát Hành Sumi Supercharts `v2.2.0`

**Ngày hoàn tất:** 23/11/2026  
**Phiên bản:** `2.2.0` (nâng cấp từ `v2.1.0` + 43 bản vá QA tại [05_FIX_VERIFICATION_REPORT.md](file:///Users/mizuhara/workspace/sumi/docs/qa/05_FIX_VERIFICATION_REPORT.md))  
**Mục tiêu phiên bản:** Nâng trải nghiệm thao tác biểu đồ, luyện tập đa khung giờ và kiểm định định lượng lên tầm TradingView bản địa dành cho trader Việt Nam.

---

## 1. Tổng Quan Các Hạng Mục Hoàn Thiện Trong `v2.2.0`

| # | Phân hệ | Tính năng mới & Nâng cấp trong `v2.2.0` | Mã nguồn chính |
|---|---|---|---|
| 1 | **An toàn dữ liệu & Định lượng** | Tự động sao lưu `sumi_market.db.v{ver}.bak` trước khi chạy migration phá hủy; đồng bộ hệ số thường niên hóa `H4` (`409.5`) trong Walk-Forward; nâng `sumi_workspace.db` lên schema `v2` (`indicator_templates`, `replay_sessions`, mở rộng `price_alerts`). | [storage.rs](file:///Users/mizuhara/workspace/sumi/crates/sumi-data/src/storage.rs), [walk_forward.rs](file:///Users/mizuhara/workspace/sumi/crates/sumi-core/src/backtest/walk_forward.rs), [workspace_db.rs](file:///Users/mizuhara/workspace/sumi/crates/sumi-desktop/src/workspace_db.rs) |
| 2 | **Giao dịch trực tiếp trên Biểu đồ (Chart Trading)** | Kéo thả dọc trục giá trực tiếp các đường `⋮⋮ SL`, `⋮⋮ TP` và lệnh chờ `⋮⋮ LO / STOP` trên Canvas kèm hiển thị lãi/lỗ dự phóng trực tiếp và tự làm tròn theo bước giá luật định (`HOSE 10/50/100đ`, `HNX/UPCOM 100đ`, `VN30F1! 0.1đ`); đặt `SL`/`TP` nhanh 1-click bằng chuột phải trên nến. | [tradingViewOverlays.ts](file:///Users/mizuhara/workspace/sumi/ui/src/chart/overlays/tradingViewOverlays.ts), [KLineViewport.tsx](file:///Users/mizuhara/workspace/sumi/ui/src/chart/KLineViewport.tsx), [ChartContextMenu.tsx](file:///Users/mizuhara/workspace/sumi/ui/src/components/ChartContextMenu.tsx), [vnMarket.ts](file:///Users/mizuhara/workspace/sumi/ui/src/utils/vnMarket.ts) |
| 3 | **Lưu & Tiếp Tục Phiên Bar Replay (Save / Resume)** | Lưu phiên luyện tập Bar Replay đa khung giờ (`1h + 4h`, mốc cắt, tốc độ, Chế độ Luyện mù) **kèm toàn bộ trạng thái vị thế / lệnh / sổ cái Paper Broker cô lập** vào SQLite (`replay_sessions`) và khôi phục chính xác qua nhiều lần tắt/mở app. | [replay.rs](file:///Users/mizuhara/workspace/sumi/crates/sumi-desktop/src/commands/replay.rs), [broker.rs](file:///Users/mizuhara/workspace/sumi/crates/sumi-desktop/src/commands/broker.rs), [ReplayControlBar.tsx](file:///Users/mizuhara/workspace/sumi/ui/src/components/ReplayControlBar.tsx) |
| 4 | **Cảnh báo đa loại nhân quả + Âm báo WebAudio** | Hỗ trợ đồng thời 3 loại cảnh báo: **Giá**, **Đường vẽ xiên/ngang** (gắn 1-click 🔔 ngay từ FloatingDrawingToolbar hoặc Cây đối tượng) và **Chỉ báo (`RSI` / `MACD` / `EMA`)** — đánh giá nhân quả cả trên nến realtime lẫn từng bước Bar Replay, kèm chuông tổng hợp WebAudio (`alertSound.ts`) và nút `Thử âm`. | [workspace.rs](file:///Users/mizuhara/workspace/sumi/crates/sumi-desktop/src/commands/workspace.rs), [alertSound.ts](file:///Users/mizuhara/workspace/sumi/ui/src/utils/alertSound.ts), [RightWidgetBar.tsx](file:///Users/mizuhara/workspace/sumi/ui/src/components/RightWidgetBar.tsx), [FloatingDrawingToolbar.tsx](file:///Users/mizuhara/workspace/sumi/ui/src/components/FloatingDrawingToolbar.tsx) |
| 5 | **Cây Đối Tượng + Hiển Thị Theo Khung Giờ + Mẫu Chỉ Báo** | Tab **`Đối tượng`** mới trong `RightWidgetBar` quản lý tập trung toàn bộ chỉ báo và hình vẽ PTKT kèm tùy chọn **Hiển thị theo khung giờ** (`Mọi khung giờ`, `Intraday ≤ 4H`, `Swing ≥ 1D`, `Chỉ khung hiện tại`); menu **`Mẫu chỉ báo`** trên `TopHeader` tích hợp sẵn 3 bộ chỉ báo chuẩn trader Việt (`VN Price Action`, `Ichimoku + RSI + ATR`, `VN30F Scalping`) và lưu/xóa mẫu cá nhân. | [RightWidgetBar.tsx](file:///Users/mizuhara/workspace/sumi/ui/src/components/RightWidgetBar.tsx), [TopHeader.tsx](file:///Users/mizuhara/workspace/sumi/ui/src/components/TopHeader.tsx) |
| 6 | **Vạch Tách Phiên Intraday Việt Nam (`Phiên`)** | Nút bật/tắt `Phiên` hiển thị rõ ranh giới `09:00` (Mở cửa sáng), `13:00 Chiều` và `14:30 ATC` theo múi giờ ICT (`UTC+7`) trên các khung intraday Việt Nam (`1m / 5m / 15m / 1h`). | [vnMarket.ts](file:///Users/mizuhara/workspace/sumi/ui/src/utils/vnMarket.ts), [KLineViewport.tsx](file:///Users/mizuhara/workspace/sumi/ui/src/chart/KLineViewport.tsx) |
| 7 | **Mở khóa toàn bộ Lõi Định Lượng lên UI** | Strategy Tester mở khóa **Bộ tạo quy tắc tự thiết kế 101 tín hiệu (`preset = 'custom'`)**, **Kiểm định Danh mục đa mã (`run_strategy_portfolio` — tỷ trọng đều `1/N` & nghịch đảo biến động `1/σ`)**, đòn bẩy phái sinh `VN30F1!` (`5.88x` / ký quỹ 17%); Stock Screener bổ sung **thanh lọc số đa điều kiện** (`RSI Từ–Đến`, `RVol ≥`, `% Đổi ≥`, `MACD >0/<0`, `Sức khỏe ≥`) và **Xuất CSV UTF-8 BOM**. | [backtest.rs](file:///Users/mizuhara/workspace/sumi/crates/sumi-desktop/src/commands/backtest.rs), [scanner.rs](file:///Users/mizuhara/workspace/sumi/crates/sumi-desktop/src/commands/scanner.rs), [StrategyTesterTab.tsx](file:///Users/mizuhara/workspace/sumi/ui/src/components/dock/StrategyTesterTab.tsx), [StockScreenerTab.tsx](file:///Users/mizuhara/workspace/sumi/ui/src/components/dock/StockScreenerTab.tsx) |
| 8 | **Kiểm thử tự động Frontend (`Vitest`) & Đóng gói `v2.2.0`** | Bổ sung bộ unit test frontend `Vitest` kiểm tra quy tắc bước giá VN, lọc hiển thị theo khung giờ, vạch tách phiên ICT, Heikin-Ashi không trôi `HA_Open` và đồng bộ hình vẽ đa khung. | [sumiV22.test.ts](file:///Users/mizuhara/workspace/sumi/ui/src/__tests__/sumiV22.test.ts), [package.json](file:///Users/mizuhara/workspace/sumi/ui/package.json) |

---

## 2. Kết Quả Kiểm Định Tự Động & Biên Dịch Thực Tế

| Hạng mục kiểm định | Lệnh thực thi | Kết quả | Chi tiết đo được |
|---|---|---|---|
| **Rust Unit & Integration Tests** | `cargo test --workspace` | **PASS (`232/232`)** | `sumi-core`: `199` pass · `sumi-data`: `18` unit + `3` integration pass · `sumi-desktop`: `12` pass (`0` failed) |
| **Rust Clippy Static Analysis** | `cargo clippy --workspace --all-targets` | **PASS (`0` warnings)** | Sạch hoàn toàn cảnh báo trên cả 3 crates |
| **Frontend Vitest Unit Tests** | `cd ui && npm test` | **PASS (`8/8`)** | Thực thi trong `4ms` (`sumiV22.test.ts`) |
| **TypeScript & Production Bundle** | `cd ui && npm run build` | **PASS (`0` errors)** | `ui/dist` build hoàn tất trong `1.34s` (`827.67 kB` JS / `223.15 kB` gzip) |
| **Real-Backend Live E2E (`v2.2.0`)** | `verify_v22_features.mjs` (0% `webFallback.ts`) | **PASS (`14/14`)** | Chạy trực tiếp trên `AppState` Rust thật + bản sao thật `sumi_market.db` (`87.9 MB`), `0` lỗi console, `0` JS exception |
| **Release Desktop Binary** | `cargo build --release -p sumi-desktop --bin sumi-desktop` | **PASS (`27 MB`)** | Binary hoàn chỉnh tại `target/release/sumi-desktop` (v2.2.0) |

---

## 3. Chi Tiết 14 Kịch Bản E2E Thực Tế Trên Lõi Rust Thật

Toàn bộ 14 kịch bản dưới đây được chạy trên `ui/dist` bản build thật gắn thẳng vào lõi Rust `AppState` thật và dữ liệu lịch sử thật (`sumi_market.db` bản v1 gốc):

| ID | Kịch bản kiểm định | Kết quả | Bằng chứng / Số liệu đo thực tế |
|---|---|---|---|
| `V22-01` | Auto-backup `sumi_market.db.v1.bak` trước migration `v1 → v2` | **PASS** | `preVer=1 → postVer=2`, file backup tự tạo `87,920,640 bytes` |
| `V22-02` | `sumi_workspace.db` schema `v2` & 3 mẫu chỉ báo VN mặc định | **PASS** | `wsVer=2`; 3 mẫu sẵn có: `Ichimoku + RSI + ATR`, `VN Price Action`, `VN30F Scalping` |
| `V22-03` | Xác nhận gắn runtime Rust thật (0% `webFallback.ts`) | **PASS** | `bridgeActive=true`, `hasTauri=true`, `17` IPC commands thực thi |
| `V22-04` | CRUD Mẫu chỉ báo trên SQLite (`save/list/delete_indicator_template`) | **PASS** | Lưu & đọc lại thành công (`totalBeforeDelete=4`), xóa sạch sau test |
| `V22-05` | Áp dụng nhanh 1-click mẫu `Ichimoku + RSI + ATR` từ `TopHeader` | **PASS** | Nạp tức thì 3 chỉ báo (`ICHIMOKU(9,26,52)`, `RSI(14)`, `ATR(14)`) |
| `V22-06` | Chart Trading: Kéo chỉnh `SL/TP` & sửa giá lệnh chờ `LO` | **PASS** | `SL: 42,300 → 42,800`, `TP: 50,400 → 51,400`, `LO: 43,700 → 43,200` trên `HOSE:FPT` |
| `V22-07` | Tab Cây đối tượng (`Đối tượng`) & Bộ lọc khung giờ hiển thị | **PASS** | Hiển thị đủ 3 chỉ báo + hình vẽ `Đường xu hướng` + dropdown `Hiển thị khung` |
| `V22-08` | Cảnh báo đa loại nhân quả (`Drawing Trendline` + `RSI(14)`) | **PASS** | Cả 2 cảnh báo (`drwTriggered=true`, `indTriggered=true`, RSI `21.39 → 20.23` cắt `20.81`) khớp chính xác qua `check_bar_alerts` |
| `V22-09` | Lưu & Khôi phục phiên Bar Replay đa khung (`1h + 4h`) + Paper Broker cô lập | **PASS** | Dừng Replay rồi mở lại phiên `sess_v22_mtf`: khôi phục chính xác `cursor=398`, `tracked=["1h","4h"]`, và vị thế Paper Broker `LONG 500` |
| `V22-10` | Custom 101-Signal Rule Engine Backtest (`MACD + RVOL 1.1x` trên `HOSE:FPT 1D`) | **PASS** | `1,500` nến · `29` lệnh · Lợi nhuận `+18.09%` · Thời gian xử lý lõi Rust **`4.76 ms`** |
| `V22-11` | Portfolio Multi-Symbol Backtest trên `VN30 Bluechips (12 mã)` (`1/N` & `1/σ`) | **PASS** | `12` mã · Giảm Max DD nhờ đa dạng hóa **`-55.01%`** (`Equal: -0.48%`, `Inverse-Vol: +1.07%`) · **`6.05 ms`** |
| `V22-12` | Stock Screener lọc số đa điều kiện (`RSI 35–70`, `RVol ≥ 0.7`, `MACD > 0`, `SK ≥ 0`) | **PASS** | Quét `177` mã HOSE trong **`35 ms`**, lọc ra `41` mã khớp 100% toàn bộ điều kiện (`BAF`, `CTG`, `CTS`, …) |
| `V22-13` | Giao diện thanh lọc số đa điều kiện & nút `CSV` trong `StockScreenerTab` | **PASS** | Hiển thị đủ ô lọc số và nút xuất CSV |
| `V22-14` | Không phát sinh lỗi console hay unhandled Promise rejection | **PASS** | `consoleErrors: []`, `exceptions: []` |

---

## 4. Ảnh Chụp Minh Chứng (2× HiDPI, Dữ Liệu Thật)

### 4.1 Chart Trading (Kéo SL/TP/LO trực tiếp) + Mẫu Ichimoku + Cây Đối Tượng (`Đối tượng`)
![Chart Trading & Object Tree](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence/v22_chart_trading_drag_and_objects.png)

### 4.2 Lưu/Khôi Phục Phiên Bar Replay Đa Khung (`1h + 4h`) + Cảnh Báo Đa Loại (`Chỉ báo` & `Đường vẽ`)
![Replay Sessions & Multi-Kind Alerts](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence/v22_replay_sessions_and_alerts.png)

### 4.3 Bộ Tạo Quy Tắc 101 Tín Hiệu (`Tự thiết kế`) + Backtest Danh Mục Đa Mã (`VN30 Bluechips 12 mã`)
![Custom 101-Signal Rules & Portfolio Backtest](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence/v22_portfolio_and_custom_backtest.png)

### 4.4 Bộ Lọc Cổ Phiếu Đa Điều Kiện Số (`RSI / RVol / % Đổi / MACD / Sức khỏe`) + Xuất CSV
![Stock Screener Multi-Condition Filters](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence/v22_screener_multifilter.png)

---

## 5. Điểm Kiểm Tra Thủ Công Khuyến Nghị Trước Khi Phân Phối `.app`

Do giới hạn quyền TCC của macOS trong môi trường tự động hóa không thể mô phỏng thao tác chuột trực tiếp lên cửa sổ `WKWebView` gốc và phần cứng loa ngoài:
1. **Đóng gói bản `.app` mới nhất:** Chạy `cd ui && npx tauri build --bundles app` để cập nhật bundle macOS `v2.2.0` (chưa ghi đè `/Applications/Sumi Supercharts.app` cũ của bạn).
2. **Kiểm tra thủ công trên macOS Native `WKWebView`:**
   - Kéo thả bằng chuột thật đường `⋮⋮ SL` / `⋮⋮ TP` / `⋮⋮ LO` trực tiếp trên nến và xác nhận âm báo WebAudio (`Thử âm` trong tab `Cảnh báo`) phát loa bình thường.
   - Bật chế độ Online (`Tắt Offline`) trong giờ giao dịch để kiểm tra luồng nến intraday realtime kết hợp vạch tách phiên `Phiên` (`09:00` / `13:00 Chiều` / `14:30 ATC`).
