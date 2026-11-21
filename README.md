# Sumi v2.0 — Quantitative Market Data & Trading Engine (Rust Workspace)

Kiến trúc **Sumi v2.0** (chuẩn bị cho giao diện Tauri v2 + KLineChart v10.1.1 Supercharts) với tầng lõi định lượng và dữ liệu thị trường viết bằng **Rust** (Edition 2024, Rust `>= 1.91`), đa nền tảng **macOS** và **Windows (`x86_64-pc-windows-msvc`)**.

---

## 1. Cấu Trúc Cargo Workspace

```text
sumi/
├── Cargo.toml                         # Virtual Cargo Workspace (resolver = "3")
├── crates/
│   ├── sumi-core/                     # Lõi định lượng thuần Rust (zero GUI / zero C / WASM-safe)
│   │   └── src/
│   │       ├── model.rs (domain)      # OHLCV Candle, PriceBounds
│   │       ├── data/                  # RFC-4180 CSV/JSON, CafeF 5-rule split norm, đa khung thời gian & audit
│   │       ├── indicators/            # 24+ chỉ báo, mẫu hình nến, Volume Profile, TPO, VWAP, Ichimoku, CVD, ProxyBB
│   │       ├── scanner/               # 101 tín hiệu, độ rộng thị trường RRG, MarketBB & PIT Universe
│   │       ├── backtest/              # Strategy Engine, Rule Builder, WFO, Monte Carlo, báo cáo HTML
│   │       ├── replay/                # Bar Replay nhân quả đa khung giờ (chống lộ tương lai)
│   │       ├── trading/               # Paper Trading, DOM Ladder, Coaching & khóa lô T+2 FIFO
│   │       ├── journal/               # Quy tắc bước giá HOSE/HNX/UPCOM/VN30F, lịch nghỉ lễ VN & nhật ký giao dịch
│   │       ├── alerts/                # Cảnh báo giá, chỉ báo, đường vẽ & confluence
│   │       ├── drawings/              # Mô hình hình học & lưu trữ công cụ vẽ
│   │       └── viewport/              # Hệ tọa độ TimeScale/PriceScale, template & snapshot
│   └── sumi-data/                     # Tầng dữ liệu thị trường Cache-First + CLI kiểm thử
│       └── src/
│           ├── types.rs               # Interval (1m..1M), KLinePeriod, OhlcvBar (Unix ms + turnover), VN bands & ticks
│           ├── provider.rs            # Trait MarketDataProvider + BarSubscription / QuoteSubscription
│           ├── tradingview_feed.rs    # tradingview-rs (rustls-tls only, ProData -> Data fallback, auto-reconnect)
│           ├── rate_limit.rs          # TokenBucketRateLimiter + Semaphore bảo vệ truy vấn batch/Scanner
│           ├── storage.rs             # SQLite WAL (WITHOUT ROWID clustered B-tree), phân trang Init/Forward/Backward
│           ├── cache_service.rs       # MarketDataService: Cache-First (< 5ms), gap-fill tự động & chế độ Offline
│           ├── cafef_importer.rs      # Nhập nhanh CafeF CSV (471+ mã & chỉ số VN, chuẩn hóa raw VND)
│           ├── logging.rs             # Bộ lọc tracing áp dụng tradingview::RECOMMENDED_LOG_DIRECTIVES
│           └── bin/sumi-data-cli.rs   # Công cụ CLI kiểm thử thực tế
├── assets/fixtures/                   # Dữ liệu mẫu kiểm thử tự động
└── data/                              # (gitignored) data/raw/cafef/ & data/cache/sumi_market.db
```

---

## 2. Yêu Cầu Môi Trường

- **macOS (`aarch64-apple-darwin` / `x86_64-apple-darwin`):** Xcode Command Line Tools + Rust stable (`>= 1.91`).
- **Windows 10/11 (`x86_64-pc-windows-msvc`):** Visual Studio C++ Build Tools (*Desktop development with C++* + Windows SDK) + Rust stable (`>= 1.91`). Không cần `wreq`, BoringSSL, CMake hay Perl (`tradingview-rs` dùng `default-features = false, features = ["rustls-tls"]`).

---

## 3. Kiểm Định Chất Lượng & Kiểm Thử

```bash
# Định dạng mã nguồn
cargo fmt --all -- --check

# Clippy toàn workspace (zero warnings)
cargo clippy --workspace --all-targets -- -D warnings

# Clippy kiểm tra tương thích Windows MSVC (chạy trực tiếp trên Windows hoặc với Windows SDK;
# trên macOS thuần, sumi-core thuần Rust kiểm tra trực tiếp không cần Windows SDK)
cargo clippy -p sumi-core --all-targets --target x86_64-pc-windows-msvc -- -D warnings
cargo clippy --workspace --all-targets --target x86_64-pc-windows-msvc -- -D warnings

# Chạy toàn bộ unit tests + mock/offline integration tests (216 tests)
cargo test --workspace
```

---

## 4. Sử Dụng `sumi-data-cli`

```bash
# 1. Nạp dữ liệu lịch sử nền CafeF (473 mã & chỉ số VN -> ~951k nến ngày) vào SQLite cache
cargo run --bin sumi-data-cli -- import-cafef --dir data/raw/cafef

# 2. Tìm kiếm mã chứng khoán / crypto / phái sinh
cargo run --bin sumi-data-cli -- search FPT --exchange HOSE
cargo run --bin sumi-data-cli -- search BTCUSDT --exchange BINANCE

# 3. Phân giải metadata (precision, bước giá biến thiên HOSE, biên độ Trần/Sàn VN)
cargo run --bin sumi-data-cli -- resolve HOSE:FPT
cargo run --bin sumi-data-cli -- resolve HOSE:VCB
cargo run --bin sumi-data-cli -- resolve "HNX:VN30F1!"
cargo run --bin sumi-data-cli -- resolve BINANCE:BTCUSDT
cargo run --bin sumi-data-cli -- resolve FX_IDC:USDVND

# 4. Tải nến OHLCV Cache-First (Init / Forward --before / Backward --after)
cargo run --bin sumi-data-cli -- bars HOSE:FPT --interval 15m --limit 200
cargo run --bin sumi-data-cli -- bars HOSE:FPT --interval 1D --limit 200
cargo run --bin sumi-data-cli -- --offline bars HOSE:HPG --interval 1D --limit 50

# 5. Streaming nến & quote thời gian thực (tự lưu tích lũy vào SQLite)
cargo run --bin sumi-data-cli -- stream BINANCE:BTCUSDT --interval 1m --max-events 5
cargo run --bin sumi-data-cli -- stream BINANCE:BTCUSDT --quotes --max-events 5
```
