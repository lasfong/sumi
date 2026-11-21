# Sumi Workstation (Pure Rust Financial Charting & Trading Desktop)

Ứng dụng đồ họa biểu đồ tài chính và phân tích kỹ thuật tốc độ cao viết thuần bằng **Rust** (`egui` / `eframe` + `wgpu` GPU DirectX 12 / Vulkan trên Windows, Metal trên macOS — không dùng Electron, WebView hay Python).

---

## 1. Yêu Cầu Môi Trường (Windows 10/11 & macOS)

### Trên Windows 10 / 11 (`x86_64-pc-windows-msvc`)
1. **Cài Visual Studio C++ Build Tools:** Tải [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/), tích chọn mục **"Desktop development with C++"** (có kèm Windows 10/11 SDK).
2. **Cài Rust (`rustup`):** Tải và chạy `rustup-init.exe` từ [https://rustup.rs](https://rustup.rs) (Rust stable `>= 1.91`).

### Trên macOS (`Apple Silicon / Intel`)
- Xcode Command Line Tools (`xcode-select --install`) và Rust stable (`rustup`).

---

## 2. Build, Chạy Kiểm Thử & Khởi Động Trên Windows

Mở **PowerShell** tại thư mục gốc của dự án:

```powershell
# Cách 1: Dùng script tự động kiểm thử + build bản Release + mở ứng dụng
powershell -ExecutionPolicy Bypass -File .\scripts\build_windows.ps1 -Run

# Nếu muốn bỏ qua bước chạy unit test để mở nhanh bản Release:
powershell -ExecutionPolicy Bypass -File .\scripts\build_windows.ps1 -SkipTests -Run
```

Hoặc chạy trực tiếp bằng lệnh `cargo`:

```powershell
# Chạy toàn bộ 280 bài kiểm thử tự động
cargo test

# Biên dịch và khởi chạy ứng dụng bản Release tối ưu GPU
cargo run --release
```

Tệp thực thi sau khi đóng gói nằm tại: `dist\SumiWorkstation-win64\sumi-workstation.exe` (hoặc `target\release\sumi-workstation.exe`).

---

## 3. Nạp Dữ Liệu Chứng Khoán Thực Tế (CSV / JSON / CafeF)

- **Kéo thả trực tiếp (Drag & Drop):** Kéo bất kỳ tệp `.csv`, `.tsv`, `.txt`, hoặc `.json` thả thẳng vào cửa sổ biểu đồ.
- **Thư mục dữ liệu tự động nhận diện (`data/raw/cafef/`):**
  - Nếu bạn đặt các tệp CSV giải nén từ CafeF vào thư mục `data\raw\cafef\` (ví dụ `CafeF.INDEX.Upto06112026.csv`, `CafeF.SolieuGD.Adjusted.Upto06112026.csv`), thanh `[Import CSV/JSON]` trong ứng dụng sẽ tự động hiện các nút nạp nhanh 1 chạm cho toàn bộ danh sách mã.

---

## 4. Cấu Trúc Thư Mục Dự Án

```text
sumi/
├── Cargo.toml                  # Cấu hình crate Rust duy nhất (không phụ thuộc mạng/DB ngoài)
├── assets/
│   ├── fixtures/               # Dữ liệu mẫu nhỏ phục vụ kiểm thử tự động (CafeF Index, SPY, PIT)
│   └── icons/                  # Biểu tượng ứng dụng độ phân giải cao
├── src/
│   ├── viewport/               # Hệ tọa độ TimeScale, PriceScale (Linear/Log/%/Idx100), Multi-Chart Grid, Hotkeys
│   ├── indicators/             # 24+ chỉ báo kỹ thuật, mẫu hình nến, Volume Profile, TPO, VWAP, Ichimoku, CVD
│   ├── drawings/               # 11 công cụ vẽ tương tác, OHLC Magnet, Custom Fibonacci, Undo/Redo 64 bước
│   ├── data/                   # Bộ đọc CSV/JSON tốc độ cao, gộp đa khung thời gian (1m..1W) & kiểm toán dữ liệu
│   ├── replay/                 # Tua nến lịch sử Bar Replay nhân quả (không lộ tương lai)
│   ├── trading/                # Paper Trading, kéo thả SL/TP trực tiếp trên biểu đồ, DOM Ladder
│   ├── scanner/                # Bộ quét 101 tín hiệu, độ rộng thị trường RRG, dòng tiền ProxyBB & PIT Universe
│   ├── alerts/                 # Cảnh báo giá, chỉ báo, đường vẽ nghiêng & tín hiệu
│   ├── backtest/               # Kiểm định chiến lược, Rule Builder, Walk-Forward, Monte Carlo & xuất báo cáo HTML
│   ├── journal/                # Quy tắc bước giá/biên độ chứng khoán VN, lịch nghỉ lễ & khóa lô T+2 FIFO
│   ├── chart.rs                # Bộ kết xuất đồ họa GPU
│   ├── app.rs                  # Giao diện người dùng chính
│   └── main.rs                 # Điểm vào ứng dụng Desktop (Windows / macOS)
├── examples/
│   └── viewport_bench.rs       # Bộ đo hiệu năng khung hình GPU/CPU
└── scripts/
    ├── build_windows.ps1       # Script build & đóng gói cho Windows 10/11
    ├── package_macos_app.sh    # Script đóng gói .app cho macOS
    └── package_macos_dmg.sh    # Script đóng gói .dmg cho macOS
```
