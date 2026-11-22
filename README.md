# Sumi Supercharts v2.0 — TradingView-Grade Quantitative Workstation (Rust + Tauri v2 + KLineChart v10)

Ứng dụng Desktop phân tích kỹ thuật, lọc cổ phiếu và kiểm định chiến lược định lượng **Sumi Supercharts v2.0** xây dựng trên kiến trúc **Tauri v2 + Cargo Workspace (`Rust >= 1.91`, Edition 2024) + React 19 + TypeScript + `klinecharts@10.1.1`**, hỗ trợ đa nền tảng **Windows 10/11 (`x86_64-pc-windows-msvc` / WebView2)** và **macOS (`aarch64` / `x86_64` / WKWebView)**.

---

## 1. Cấu Trúc Dự Án

```text
sumi/
├── Cargo.toml                          # Virtual Cargo Workspace (resolver = "3")
├── scripts/
│   └── build_windows.ps1               # Đóng gói tự động 1 lệnh trên Windows 10/11 (MSVC / WebView2)
├── crates/
│   ├── sumi-core/                      # Lõi định lượng thuần Rust (199 unit tests, zero GUI / WASM-safe)
│   │   └── src/                        # OHLCV, CafeF 5-rule split norm, 24+ chỉ báo, 101 tín hiệu Scanner,
│   │                                   # Backtest / Walk-Forward / Monte Carlo, Bar Replay nhân quả, Paper Broker VSD T+2
│   ├── sumi-data/                      # Tầng dữ liệu thị trường Cache-First + CLI (sumi-data-cli)
│   │   └── src/                        # tradingview-rs (rustls-tls thuần Rust), SQLite WAL WITHOUT ROWID (< 1ms),
│   │                                   # TokenBucketRateLimiter, CafeFImporter (473 mã VN), Offline fallback
│   └── sumi-desktop/                   # Ứng dụng Tauri v2 (47 IPC commands, Tauri Channels realtime, SQLite Workspace)
├── ui/                                 # Giao diện Supercharts (React 19 + TypeScript + Tailwind CSS v4 + klinecharts v10.1.1)
│   └── src/
│       ├── chart/                      # KLineViewport v10, 37+ Overlays (Long/Short R:R, Ruler, Elliott, Gann, Fib...),
│       │                               # 27 chỉ báo gốc + 5 chỉ báo định lượng (Ichimoku Kumo, SuperTrend, VWAP, ATR, StochRSI)
│       ├── components/                 # TopHeader (38px), LeftDrawingRail (42px), RightWidgetBar, ReplayControlBar,
│       │                               # Modals (<dialog> chuẩn), BottomTradingDock (Paper T+2 / WFO-MC / Screener 101)
│       └── store/                      # Zustand state đồng bộ Multi-Chart (1 / 2v / 2h / 4 ô) & SQLite
└── assets/fixtures/                    # Dữ liệu mẫu kiểm thử tự động
```

---

## 2. Yêu Cầu Môi Trường

- **Windows 10/11 (`x86_64-pc-windows-msvc`):**
  1. [Rust stable (`>= 1.91`)](https://rustup.rs) kèm **Visual Studio C++ Build Tools** (*Desktop development with C++* + Windows 10/11 SDK).
  2. [Node.js LTS (`>= 20`)](https://nodejs.org).
  3. `WebView2` (đã tích hợp sẵn mặc định trên Windows 10/11).
  *(Lưu ý: Không cần cài CMake, Perl hay BoringSSL vì `tradingview-rs` được ghim `default-features = false, features = ["rustls-tls"]`).*
- **macOS (`aarch64-apple-darwin` / `x86_64-apple-darwin`):**
  1. Xcode Command Line Tools + Rust stable (`>= 1.91`).
  2. Node.js LTS (`>= 20`).

---

## 3. Khởi Chạy & Đóng Gói Ứng Dụng

### Trên Windows 10/11 (PowerShell)

```powershell
# Cài dependencies UI, chạy toàn bộ kiểm định TypeScript + Clippy + Tests, đóng gói bộ cài (.exe / .msi / NSIS) và mở ứng dụng:
powershell -ExecutionPolicy Bypass -File .\scripts\build_windows.ps1 -Run

# Build & chạy nhanh (bỏ qua bước chạy lại unit tests):
powershell -ExecutionPolicy Bypass -File .\scripts\build_windows.ps1 -SkipTests -Run
```

Sản phẩm đóng gói được xuất ra tại `dist\SumiSupercharts-win64\`.

### Trên macOS (Terminal)

```bash
# 1. Cài đặt gói giao diện và kiểm tra build Frontend
npm --prefix ui install
npm --prefix ui run build

# 2. Chạy chế độ phát triển trực tiếp (Hot-reload Tauri v2 + Vite)
cd crates/sumi-desktop && ../../ui/node_modules/.bin/tauri dev

# 3. Đóng gói ứng dụng macOS (.app / .dmg)
cd crates/sumi-desktop && ../../ui/node_modules/.bin/tauri build --bundles app,dmg
```

---

## 4. Nạp Dữ Liệu Lịch Sử CafeF (Tùy Chọn — Khuyên Dùng Cho Offline & Toàn Sàn VN)

- **Cách 1 (Tự động khi khởi động):** Đặt các file CSV của CafeF (`CafeF.SolieuGD.Adjusted*.csv`, `CafeF.INDEX*.csv`) vào thư mục `data/raw/cafef/` hoặc `work/CafeFData/` trước khi mở app lần đầu — ứng dụng sẽ tự động nạp toàn bộ 473+ mã vào kho SQLite cục bộ.
- **Cách 2 (Trực tiếp trên Giao diện):** Nhấp biểu tượng **Kho Dữ Liệu SQLite** (cạnh nút Wifi góc phải trên cùng thanh Topbar), nhập đường dẫn thư mục chứa CSV CafeF (ví dụ `D:\CafeFData` hoặc `data/raw/cafef`) và bấm **"Nạp thư mục CafeF vào SQLite"**.
- **Cách 3 (Qua CLI `sumi-data-cli`):**
  ```bash
  cargo run -p sumi-data --bin sumi-data-cli -- import-cafef --dir data/raw/cafef
  ```
- Ngay cả khi **chưa nạp CafeF**, khi có kết nối Internet ứng dụng vẫn tự động tải và lưu đệm dữ liệu từ `tradingview-rs` cho biểu đồ, Watchlist và rổ mã mặc định của bộ lọc Screener.

---

## 5. Cổng Kiểm Định Chất Lượng

```bash
# Kiểm tra kiểu TypeScript & đóng gói giao diện
npm --prefix ui run typecheck
npm --prefix ui run build

# Kiểm tra định dạng, Clippy không cảnh báo và chạy toàn bộ bộ kiểm thử (221 tests)
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
