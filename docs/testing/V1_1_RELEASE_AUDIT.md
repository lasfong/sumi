# BÁO CÁO NGHIỆM THU PHÁT HÀNH SUMI `v1.1.0` — TRADINGVIEW PRO & ĐÓNG GÓI MACOS HOÀN CHỈNH

- **Dự án:** Sumi Trading Workstation (`/Users/mizuhara/workspace/sumi`)
- **Phiên bản nghiệm thu:** **`v1.1.0`** (`CFBundleShortVersionString = 1.1.0`, `CFBundleVersion = 20`)
- **Ngày nghiệm thu:** `2026-11-20` (`+07:00`)
- **Kế hoạch đối chiếu:** [`docs/testing/V1_1_TRADINGVIEW_PRO_MACOS_PLAN.md`](V1_1_TRADINGVIEW_PRO_MACOS_PLAN.md) (`Wave 0 -> Wave 5`)
- **Nền tảng kiểm thử:** macOS `arm64` Apple M1 (`16.0 GiB` RAM), `rustc 1.99.1 (58cf8f9c3 2026-10-13)`, Metal GPU (`wgpu` Retina `2.0x`)
- **Kết luận nghiệm thu:** **`GO FOR v1.1.0`** (`6/6` Wave hoàn thành, `280/280` bài test tự động `PASS`, `0` cảnh báo clippy native & `wasm32`, `0` crate mới, `4/4` benchmark khung hình `< 4.0 ms`, `6/6` ảnh Metal GPU + `AppIcon_1024.png` soi pixel đạt chuẩn, `dist/Sumi Workstation.app` & `dist/Sumi-Workstation-v1.1.0-arm64.dmg` xác minh toàn vẹn, `TC-65` `7/7` `CafeFData/*.zip` giữ nguyên 100% SHA-256).

---

## 1. Tóm Tắt Thực Thi Theo Từng Wave (`Wave 0 -> Wave 5`)

| Wave | Phạm Vi Triển Khai | Tệp Mã Nguồn / Script Chính | Cổng Kiểm Chứng | Trạng Thái |
| :--- | :--- | :--- | :--- | :--- |
| **Wave 0** | Chuẩn hóa `cargo fmt`, xác minh mốc nền `v1.0.0` (`274/274` tests), đồng bộ tài liệu trạng thái dự án | `docs/EXECUTION_STATE.md`, `docs/NEXT_SESSION_BRIEF.md`, `docs/FEATURE_BACKLOG_AND_PARITY_LEDGER.md` | `fmt`, native/`wasm32` clippy, `274/274` tests | **`PASS`** |
| **Wave 1** | **Left Vertical Drawing Rail** (`38 px` biểu tượng vector `egui::Painter`, nút thu gọn lưu bền vững `show_drawing_rail` trong `WorkstationSettings`/`WorkspaceTemplate` tương thích ngược, tối giản Top Toolbar Row 2); **Kéo-Thả tệp `.csv`/`.tsv`/`.txt`/`.json`** từ Finder + lớp phủ foreground + chịu lỗi tệp hỏng không phá biểu đồ; **Đối số dòng lệnh CLI** trước khi đổi thư mục làm việc | `src/viewport/workspace_template.rs`, `src/app.rs`, `src/main.rs`, `src/viewport/tests.rs`, `examples/viewport_bench.rs` | `275/275` tests (`v1_1_left_drawing_rail_toggle_and_file_drop_resilience`), native/`wasm32` clippy | **`PASS`** |
| **Wave 2** | **`ParallelChannel`** (kênh giá song song 3 điểm, 3 tay cầm `Anchor(0)`/`Anchor(1)`/`ChannelOffset`, trung tuyến `50%` nét đứt, tô đa giác cắt Sutherland-Hodgman, kéo dài 2 đầu, chuyển khung `1D <-> 1W`, ẩn nhân quả `DEF-001`, `Undo/Redo`) & **`PriceDateRange`** (`Measure`: `ΔGiá`, `Δ%`, số thanh nến, khoảng thời gian, tổng volume, chịu `NaN`/âm, hỗ trợ **`Shift+Click`** đo nhanh); nâng tổng số công cụ vẽ lên **`11` công cụ** | `src/drawings/{mod,geometry,hit_test,controller,tests}.rs`, `src/chart.rs`, `src/app.rs`, `src/alerts/engine.rs`, `src/trading/portfolio.rs` | `277/277` tests (`v1_1_parallel_channel_...`, `v1_1_price_date_range_...`), native/`wasm32` clippy | **`PASS`** |
| **Wave 3** | **Chart-Direct Trading**: kéo thả trực tiếp `SL`/`TP` của vị thế & lệnh chờ `Limit`/`Stop` ngay trên biểu đồ nến; tự động làm tròn theo bước giá chuyển bậc sàn VN (`HOSE` `10.00`/`50.00`, `HNX`, `UPCOM`, `VN30F1M`) & kẹp trần/sàn `daily_band`; nhãn preview PnL + R-multiple trực tiếp; lệnh tất định `RecordedCommandKind::ModifyOrderPrice` tua lùi Bar Replay bit-exact; bảo toàn bất biến `had_initial_sl_at_open`; nút **`[×]`** hủy lệnh chờ / xả vị thế tôn trọng khóa VSD `T+2` FIFO | `src/trading/{portfolio,coaching,tests}.rs`, `src/chart.rs`, `src/app.rs` | `278/278` tests (`v1_1_chart_direct_trading_...`), native/`wasm32` clippy | **`PASS`** |
| **Wave 4** | **Xuất Báo Cáo Strategy Lab HTML5/SVG/Print-to-PDF 1 Chạm** (`wasm32`-safe, `0` crate mới): `RunManifest`, 12 thẻ KPI, 2 biểu đồ SVG vector nội tuyến (`Equity Curve vs Buy & Hold`, `Underwater Drawdown %`), `IS/OOS`, `OutlierImpact`, `DrawdownPeriod`, phân rã Năm UTC & Signal-Bar `MarketRegime`, `MultiPhaseBatch`, Trade Log (tối đa `1,000` lệnh), chống XSS (`escape_html`), CSS `@media print` A4/Letter; gắn nút `Export HTML` trong `Strategy Lab` (`Stats`/`Slices`) & lệnh `Cmd+K` | `src/backtest/report_html.rs`, `src/backtest/mod.rs`, `src/app.rs` | `280/280` tests (`v1_1_html_report_...`), native/`wasm32` clippy, xuất mẫu thực tế `53,104` bytes (`2` SVG, `1` `@media print`) | **`PASS`** |
| **Wave 5** | **Đóng gói macOS thương mại & Tái nghiệm thu `v1.1.0`**: sinh biểu tượng Retina `AppIcon.icns` (`10` cỡ iconset), nâng `Cargo.toml` -> `1.1.0`, cập nhật `scripts/package_macos_app.sh` (`CFBundleIconFile`, `CFBundleDocumentTypes` `.csv`/`.json`, `plutil -lint`, `codesign --deep`), tạo `scripts/package_macos_dmg.sh` đóng gói `dist/Sumi-Workstation-v1.1.0-arm64.dmg`, chạy benchmark cách ly, soi pixel `6` ảnh Metal GPU, sửa 2 lỗi nhỏ về Z-order & nút `-> Order Ticket` rồi tái đóng gói & kiểm tra `TC-65` | `scripts/generate_app_icon.py`, `assets/icons/{AppIcon.icns,AppIcon_1024.png}`, `scripts/package_macos_app.sh`, `scripts/package_macos_dmg.sh`, `Cargo.toml`, `Cargo.lock` | `hdiutil verify VALID`, `codesign --verify --deep --strict`, chạy độc lập từ `/tmp`, `4/4` `viewport_bench`, `6/6` PNG, `TC-65` | **`PASS`** |

---

## 2. Cổng Chất Lượng Mã Nguồn, Phụ Thuộc & Kiểm Thử Tự Động

| Kiểm Tra | Lệnh Thực Thi | Kết Quả |
| :--- | :--- | :--- |
| **Định dạng chuẩn (`rustfmt`)** | `cargo fmt --all -- --check` | **`PASS`** (`0` sai khác) |
| **Clippy Native (`-D warnings`)** | `cargo clippy --all-targets -- -D warnings` | **`PASS`** (`0` cảnh báo, `0` lỗi) |
| **Clippy WebAssembly (`wasm32`)** | `cargo clippy --lib --target wasm32-unknown-unknown -- -D warnings` | **`PASS`** (`0` cảnh báo, `0` lỗi) |
| **Bộ Kiểm Thử Tự Động Toàn Crate** | `cargo test --quiet` | **`PASS` — `280 passed; 0 failed; 0 ignored`** (`+6` bài kiểm thử hồi quy mới cho `v1.1.0` so với mốc `274` của `v1.0.0`) |
| **Kiểm Soát Phụ Thuộc (`Cargo.lock`)** | `git diff Cargo.toml Cargo.lock` | **`PASS` — `0` crate mới** (chỉ nâng `version = "0.1.0" -> "1.1.0"` và mô tả gói trong `Cargo.toml`) |

---

## 3. Hiệu Năng Khung Hình Release (`viewport_bench` Cách Ly Trên CPU Rỗi)

Toàn bộ `4` bộ benchmark nặng (`--r12`, `--r17`, `--r18`, `--r19`) được chạy bằng bản biên dịch `--release` trên CPU Apple M1 rỗi trong thư mục tạm cách ly (`scratch/bench_run_v1_1/`, bảo toàn tuyệt đối `docs/evidence/` lịch sử). Ngưỡng `DECISION-RENDERER-002`: **mọi kịch bản UI phải có `mean`, `median` và `p99 < 4.0 ms`**.

| Bộ Benchmark | Kịch Bản Nặng Nhất / Tiêu Biểu | Số Nến (`Conflation`) | `mean` | `median` | **`p99`** | `overall_pass` |
| :--- | :--- | :---: | :---: | :---: | :---: | :---: |
| **`--r12`** | `r12_quad_2x2_conflated_ichimoku_vwap_session_cvd_50k_frame` | `200,000` (`32x`) | `1.383 ms` | `1.330 ms` | **`1.900 ms`** | **`true`** |
| **`--r12`** | `r12_single_1x1_ichimoku_cvd_50k_frame` | `50,000` (`1x`) | `0.236 ms` | `0.228 ms` | **`0.462 ms`** | **`true`** |
| **`--r17`** | `r17_quad_2x2_50k_practice_and_multistrategy_stress` | `200,000` (`64x`) | `2.348 ms` | `2.261 ms` | **`2.883 ms`** | **`true`** |
| **`--r17`** | `r17_100k_conflated_multistrategy_pct_and_slices` | `100,000` (`32x`) | `1.805 ms` | `1.759 ms` | **`2.154 ms`** | **`true`** |
| **`--r18`** | `r18_100k_bars_all_extended_panes_conflated` | `100,000` (`32x`) | `0.949 ms` | `0.913 ms` | **`1.419 ms`** | **`true`** |
| **`--r18`** | `r18_b1_b2_overlays_patterns_3osc_fpt` | `180` (`1x`) | `0.220 ms` | `0.216 ms` | **`0.353 ms`** | **`true`** |
| **`--r19`** | `r19_data_readiness_audit_modal_1280x800` | `151` (`1x`) | `0.338 ms` | `0.353 ms` (`p95`) | **`0.580 ms`** | **`true`** |
| **`--r19`** | `r19_settings_modal_open_quad_2x2_1280x800` | `197` (`1x`) | `0.360 ms` | `0.381 ms` (`p95`) | **`0.474 ms`** | **`true`** |
| **`--r19`** | `r19_100k_conflated_hollow_bullish_1280x800` | `100,000` (`32x`) | `0.241 ms` | `0.261 ms` (`p95`) | **`0.340 ms`** | **`true`** |

**Kết quả:** **`4/4` bộ benchmark đạt `overall_pass: true`**, `p99` nặng nhất trên lưới `2x2` `200,000` nến kết hợp Practice Coaching & Multi-Strategy 0% chỉ **`2.883 ms < 4.0 ms`**.

---

## 4. Xác Minh Gói Đóng Gói macOS Thương Mại (`.app`, `AppIcon.icns`, `.dmg`)

| Hạng Mục Kiểm Chứng | Kết Quả Thực Tế | Trạng Thái |
| :--- | :--- | :---: |
| **Biểu tượng Retina `AppIcon.icns`** | [`assets/icons/AppIcon.icns`](../../assets/icons/AppIcon.icns) (`511,638` bytes, đủ `10` cỡ `16×16` .. `1024×1024` đóng gói bằng `iconutil`) & [`assets/icons/AppIcon_1024.png`](../../assets/icons/AppIcon_1024.png) (đã soi pixel: nền đêm `#0D111A`, 3 nến tăng/giảm `#26A69A`/`#EF5350`, đường xu hướng `#2962FF`, viền mượt không gãy bậc ở `y = 512`) | **`PASS`** |
| **Gói ứng dụng `.app`** | `dist/Sumi Workstation.app` (`Info.plist` `plutil -lint: OK`, `CFBundleShortVersionString = 1.1.0`, `CFBundleVersion = 20`, `CFBundleIconFile = AppIcon`, `CFBundleDocumentTypes` = `OHLCV Market Data CSV` & `OHLCV Market Data JSON`) | **`PASS`** |
| **Chữ ký số (`codesign`)** | `codesign --verify --deep --strict --verbose=2` trên bản nằm bên trong `.dmg` gắn chỉ đọc -> **`valid on disk` & `satisfies its Designated Requirement`** | **`PASS`** |
| **Ảnh đĩa cài đặt `.dmg`** | `dist/Sumi-Workstation-v1.1.0-arm64.dmg` (`8.8 MB`, định dạng nén `UDZO`, SHA-256 `01178097a990ca411be1325993b6d59a610952e353579441c89af9890eacb73c`) | **`PASS`** |
| **Kiểm tra toàn vẹn & gắn ảnh đĩa** | `hdiutil verify` -> **`checksum is VALID`** (`CRC32 $B01073F2`); `hdiutil attach -readonly` xác nhận đủ `Sumi Workstation.app`, symlink `Applications -> /Applications`, và `README_INSTALL.txt` (`1,189` bytes) | **`PASS`** |
| **Chạy độc lập từ `/tmp` (mô phỏng Finder)** | Chạy nhị phân trực tiếp từ ổ `.dmg` chỉ đọc với `cwd = /tmp`: thoát mã `0`, sinh ảnh Metal GPU `2560×1600` (`329 KB`) & JSON telemetry (`outer_rect[0] = 46.0` xác nhận Left Drawing Rail hiển thị ngay khi khởi động) | **`PASS`** |

---

## 5. Kiểm Toán Thị Giác Trực Tiếp Trên Metal GPU Retina (`docs/testing/v1_1_evidence/`)

Toàn bộ `6` ảnh chụp thực tế trên Apple M1 Metal GPU (`2.0x` Retina) và tệp HTML mẫu đều đã được sinh ra và **soi trực tiếp từng pixel qua `read_file`**:

| Tệp Bằng Chứng | Độ Phân Giải | Nội Dung Kiểm Chứng & Tinh Chỉnh Đã Áp Dụng | Kết Quả |
| :--- | :---: | :--- | :---: |
| [`01_left_drawing_rail_1280x800.png`](v1_1_evidence/01_left_drawing_rail_1280x800.png) | `2560×1600` | Left Vertical Drawing Rail `38 px` hiển thị sắc nét `12` biểu tượng công cụ vẽ + `6` nút tác vụ (`Magnet`, `Undo`, `Redo`, `Inspector`, `Clear`, `Collapse`); Top Toolbar 2 dòng cực thoáng ở `1280×800` | **`PASS`** |
| [`02_parallel_channel_and_measure_1440x1000.png`](v1_1_evidence/02_parallel_channel_and_measure_1440x1000.png) | `2880×2000` | Kênh giá song song `#1 Channel` (3 tay cầm, trung tuyến nét đứt `50%`, vùng tô xanh mờ, `Extend Right (+∞)`), hộp thước đo `#2 Measure` (`-4.56 (-7.32%) / 41 bars, 62d · Vol 237.90M`), `Drawing Inspector` hiển thị `Offset ΔPrice 5.5714`. **Tinh chỉnh đã áp dụng trước chốt:** vô hiệu hóa nút `-> Order Ticket` cho `ParallelChannel`/`PriceDateRange` tại `src/app.rs` và xác nhận nút đã mờ sạch trên ảnh tái chụp | **`PASS`** |
| [`03_chart_direct_trading_sl_tp_drag_1440x1000.png`](v1_1_evidence/03_chart_direct_trading_sl_tp_drag_1440x1000.png) | `2880×2000` | Giao dịch trực tiếp trên biểu đồ `FPT 1D`: các nhãn `TP +2535 kVND (+1.66R)`, `LONG 500 -5.08 (-0.02%) [×]`, `BUY LMT 200 [×]`, `SL -1155 kVND (-0.75R)` khớp đồng bộ `Order Ticket` (`Moved position SL -> 48.50`) và `FIFO LOTS (VSD T+2 13:00)`. **Lỗi Z-order đã phát hiện & vá triệt để trước chốt:** tách `PendingTradingBadge` trong `src/chart.rs` để xả `line_mesh` trước khi vẽ nhãn, xác nhận ảnh tái chụp không còn một pixel đường nét đứt nào đè lên chữ nhãn | **`PASS`** |
| [`04_strategy_lab_export_html_1440x1000.png`](v1_1_evidence/04_strategy_lab_export_html_1440x1000.png) | `2880×2000` | Dock `Strategy Lab` hiển thị nút `Export HTML`, thông báo trạng thái xuất tệp, cùng đồ thị so sánh đa chiến lược chuẩn hóa `0%` UTC sắc nét | **`PASS`** |
| [`05_quad_2x2_watchlist_rail_1440x1000.png`](v1_1_evidence/05_quad_2x2_watchlist_rail_1440x1000.png) | `2880×2000` | Bố cục phối hợp căng nhất: `471` mã Watchlist CafeF + Left Drawing Rail + lưới đa biểu đồ `2x2` (`FPT`, `VCB`, `HPG`, `SSI`) hoàn toàn không chồng chữ, không cắt biên | **`PASS`** |
| [`06_settings_v1_1_1280x800.png`](v1_1_evidence/06_settings_v1_1_1280x800.png) | `2560×1600` | Hộp thoại `[Settings]` hiển thị tùy chọn mới `Show Left Vertical Drawing Rail` sắc nét trên nền tối ở `1280×800` | **`PASS`** |
| [`sample_fpt_strategy_report.html`](v1_1_evidence/sample_fpt_strategy_report.html) | `53,104` bytes | Báo cáo Strategy Lab HTML5 tự chứa (`2` biểu đồ `<svg>` nội tuyến Equity + Underwater Drawdown, `1` khối `@media print` A4/Letter, bảng KPI, Outlier/DD/Yearly/Regime/Multi-Phase/Trade Log) | **`PASS`** |

---

## 6. Kiểm Tra Bảo Toàn Dữ Liệu Gốc & Hiện Vật Lịch Sử (`TC-65`)

| Tệp Dữ Liệu Gốc (`/Users/mizuhara/workspace/work/CafeFData/`) | SHA-256 Kiểm Tra Lại (`2026-11-20`) | Đối Chiếu Mốc Gốc |
| :--- | :--- | :---: |
| `0_Index_CafeF.INDEX.Upto.zip` | `8e5e354722266c670f70e7e858b15360d2b5d8722a48ce6398804c821a9e1294` | **`INTACT`** |
| `1_Adjusted_CafeF.SolieuGD.Upto_part1.zip` | `76ad120dc2840cfa1cbed7d7a9faf0fed2a65d7b44edc626c988e653624ba00c` | **`INTACT`** |
| `1_Adjusted_CafeF.SolieuGD.Upto_part2.zip` | `c65b454c1539ffe8396e9355743ab910fb93fe7c3513bde47edf184710ae1a5f` | **`INTACT`** |
| `1_Adjusted_CafeF.SolieuGD.Upto_part3.zip` | `715cb25771dcb760305fa774e53b4b1ad174a165e7c640b4c7f3b589a0543407` | **`INTACT`** |
| `2_Raw_CafeF.SolieuGD.Raw.Upto_part1.zip` | `ab28800e195f29f0789570fb70b02f7490f923d4053c042459be92097f062567` | **`INTACT`** |
| `2_Raw_CafeF.SolieuGD.Raw.Upto_part2.zip` | `8964e9f7e8d0fcfe9cc55033c4c01f5ecb1e2d39c8f27046973df0aa2be056dc` | **`INTACT`** |
| `2_Raw_CafeF.SolieuGD.Raw.Upto_part3.zip` | `141ea6e490ff99239b4ea6ca2f110a9237e5888cb94f756602e84bf07dbbe506` | **`INTACT`** |

- **`ARCHIVE/` & `docs/evidence/`:** `git status -s ARCHIVE docs/evidence` trả về **trống (`0` tệp thay đổi)**.
- **Ràng buộc `DEC-006`:** Tuyệt đối không tương tác với TradingView Desktop trên máy.

---

## 7. Kết Luận Nghiệm Thu Cuối Cùng

**`VERDICT: GO FOR v1.1.0`**

Toàn bộ các hạng mục trong [`V1_1_TRADINGVIEW_PRO_MACOS_PLAN.md`](V1_1_TRADINGVIEW_PRO_MACOS_PLAN.md) từ `Wave 0` đến `Wave 5` đã được hoàn thiện, vá sạch cả 2 chi tiết thẩm mỹ phát hiện qua soi pixel trước khi chốt bản cuối, đạt `100%` cổng kiểm thử tự động (`280/280`), giữ vững hiệu năng khung hình siêu mượt `< 4.0 ms` trên Metal GPU (`p99 <= 2.883 ms`) với `0` phụ thuộc mới, và sẵn sàng cài đặt ngay từ bộ cài thương mại `dist/Sumi-Workstation-v1.1.0-arm64.dmg`.
