# SUMI SUPERCHARTS `v3.0.0` — ĐẶC TẢ KỸ THUẬT & MASTER PROMPT THI CÔNG `WAVE 6 HARDENING`

- **Ngày lập đặc tả:** `2026-11-28`
- **Phiên bản nền (`Baseline`):** `v3.0.0` sau Wave 1–5 (`HEAD = a6d7260` trên nhánh `master` tại `/Users/mizuhara/workspace/sumi`)
- **Mục tiêu cốt lõi:** Khắc phục triệt để **100% các lỗi Chí mạng (`C-01..C-03`), lỗi Cao (`H-01..H-06`) và khoảng trống đặc tả (`S-01..S-04`)** được phát hiện qua đợt Kiểm toán Độc lập Wave 1–5, đưa Sumi Supercharts `v3.0.0` đạt chuẩn **Production-Grade Cross-Platform (`macOS + Windows`) & QMV Top-Down `KT01` Workstation** thực thụ.
- **Nguyên tắc bất biến:**
  1. **Tuyệt đối không sửa đổi mã nguồn hay CSDL của `/Users/mizuhara/workspace/yumeko` và `/Users/mizuhara/workspace/Doraemi`.** Cầu nối `sumi-yumeko-bridge` giữ nguyên chế độ `SQLITE_OPEN_READ_ONLY` + `PRAGMA query_only = ON`.
  2. **Không làm tắt, không để cờ cấu hình chết (`dead config`), không giả lập bằng đường ngang (`lookahead flat-line`).**
  3. **Không gắn đường dẫn cứng theo máy cá nhân (`/Users/...`).** Mọi tính năng và đóng gói phải chạy sạch trên cả macOS lẫn Windows Clean-Slate.

---

# PHẦN A — ĐẶC TẢ KỸ THUẬT CHI TIẾT (`WAVE 6A -> 6D`)

## Nhóm `6A` — Đóng gói Độc lập Đa nền tảng, Deep-Link macOS & Đồng bộ Đa máy (`C-01`, `C-02`, `H-06`, `S-02`)

### `W6-01` [CRITICAL / `C-01`] Sửa lỗi màn hình trắng `localhost:1420` trên bản Release `.app` (macOS) & `.exe` (Windows) + Bắt sự kiện `sumi://` Apple Event trên macOS
- **Hiện trạng lỗi:**
  1. `crates/sumi-desktop/Cargo.toml:34` đang khai báo `tauri = { version = "2", features = [] }` (thiếu `custom-protocol`, khác với `yumeko-desktop/Cargo.toml:22` dùng `features = ["custom-protocol"]`).
  2. `scripts/build-macos-app.sh:59` và `scripts/build-windows-app.ps1:60` gọi trực tiếp `cargo build --release --package sumi-desktop`. Khi không bật `tauri/custom-protocol`, Tauri v2 không nhúng `../../ui/dist` vào tệp thực thi mà mở WebView trỏ tới `devUrl: "http://localhost:1420"` (`tauri.conf.json:8`), gây chết trắng màn hình trên mọi máy người dùng không chạy `npm run dev`.
  3. `crates/sumi-desktop/src/lib.rs:20-52, 171` chỉ đọc `sumi://` từ `std::env::args()`. Trên macOS, LaunchServices (`open "sumi://inspect?symbol=FPT&as_of=2026-11-26"`) gửi URL qua Apple Event (`tauri::RunEvent::Opened { urls }`), nhưng `.run(tauri::generate_context!())` không bắt sự kiện nên URL bị nuốt mất.
  4. `scripts/build-windows-app.ps1:95` ghi sai đường dẫn trong `README-WINDOWS.txt` là `%LOCALAPPDATA%\sumi\data` thay vì `%LOCALAPPDATA%\sumi` như `crates/sumi-data/src/paths.rs`.
- **Yêu cầu sửa chữa bắt buộc:**
  1. Trong `crates/sumi-desktop/Cargo.toml`: thêm `[features]` với `custom-protocol = ["tauri/custom-protocol"]` (không bật vào `default` nếu muốn `cargo run` dev dùng Vite HMR, **HOẶC** bật `features = ["custom-protocol"]` khi release: truyền `--features custom-protocol` trong **cả hai** script `scripts/build-macos-app.sh` và `scripts/build-windows-app.ps1`, đồng thời build kèm `--package sumi-cli` ở cả hai script để gói `.app`/`.zip` luôn có sẵn CLI `sumi` / `sumi.exe`).
  2. Trong `crates/sumi-desktop/src/Cargo.toml` / `lib.rs`: đổi `.run(tauri::generate_context!())` sang `.build(tauri::generate_context!()).expect(...).run(|app_handle, event| { ... })`:
     - Bắt `#[cfg(any(target_os = "macos", target_os = "ios"))] tauri::RunEvent::Opened { urls }`: duyệt `urls`, gọi `deeplink::parse_sumi_deeplink(url.as_str())`, focus cửa sổ `"main"` và `app_handle.emit(deeplink::DEEPLINK_EVENT_NAME, &payload)`.
     - Tránh race condition khi cold-start: thêm hàng đợi `pending_deeplink: Mutex<Option<DeepLinkPayload>>` trong `AppState` + lệnh IPC `take_pending_deeplink` để `App.tsx` chủ động lấy ngay khi mount xong (thay cho `tokio::time::sleep(900ms)` mong manh ở `lib.rs:47`!).
  3. Trong `crates/sumi-desktop/src/state.rs:258-265` (`resolve_database_paths`): bổ sung hỗ trợ biến môi trường `SUMI_MARKET_DB` và `SUMI_WORKSPACE_DB` đồng nhất với `sumi-cli/src/engine.rs:95` và tài liệu `WINDOWS_INSTALL_AND_SYNC_GUIDE.md`. Sửa lại dòng hướng dẫn đường dẫn trong `scripts/build-windows-app.ps1:95` thành `%LOCALAPPDATA%\sumi`.
- **Tiêu chí nghiệm thu:** Chạy thật `./scripts/build-macos-app.sh --no-install`, kiểm tra tệp nhị phân `dist-macos/Sumi Supercharts.app/Contents/MacOS/sumi-desktop` nhúng tài nguyên `ui/dist` thật (không trỏ `http://localhost:1420`) và `codesign --verify --deep --strict` đạt `0` lỗi.

---

### `W6-02` [CRITICAL / `C-02`] Sửa lỗi Đồng bộ Đa máy (`LWW`) làm mất dữ liệu Watchlist, hỏng `###SECTION:` & Kích hoạt thật `auto_sync_on_launch`
- **Hiện trạng lỗi:**
  1. Bảng SQLite `watchlists` đã có cột `updated_at_ms`, nhưng:
     - `WatchlistGroup` (`crates/sumi-desktop/src/workspace_db.rs:856`) thiếu trường `updated_at_ms: i64` và `list_watchlists` (`842`) không `SELECT updated_at_ms`.
     - `create_watchlist` (`877`), `add_watchlist_symbol` (`922`) và `remove_watchlist_symbol` (`935`) quên cập nhật `watchlists.updated_at_ms = now_ms`.
     - `remove_watchlist_symbol` (`933`) và `upsert_watchlist_full` ép `.to_ascii_uppercase()` lên cả thanh phân nhóm `"###SECTION:Ngân Hàng"`, khiến không thể xóa header chữ thường/tiếng Việt và làm biến dạng tên nhóm sau khi đồng bộ.
  2. `crates/sumi-desktop/src/sync.rs:394-401` truyền `bundle.exported_at_ms` vào `upsert_watchlist_full(wl, bundle.exported_at_ms)` -> bất kỳ máy nào xuất gói đồng bộ muộn hơn đều ghi đè xóa sạch thay đổi Watchlist trên máy kia!
  3. `crates/sumi-desktop/src/sync.rs:334-340` xóa watchlist theo `tomb_map` mà không so sánh `del_ts > local_watchlist.updated_at_ms`.
  4. `crates/sumi-desktop/src/sync.rs:169-173` mới chỉ lọc khóa KV chứa `"secret" | "token" | "password"`, cần lọc thêm `"credential" | "api_key" | "cookie" | "oauth"`.
  5. `crates/sumi-desktop/src/sync.rs:52` lưu `auto_sync_on_launch: bool` nhưng không hề thực thi khi mở ứng dụng.
- **Yêu cầu sửa chữa bắt buộc:**
  1. Thêm `#[serde(default)] pub updated_at_ms: i64` vào `WatchlistGroup` (cả Rust `workspace_db.rs` và TypeScript `ui/src/types/market.ts`).
  2. Mọi thao tác sửa đổi watchlist (`create_watchlist`, `rename_watchlist`, `add_watchlist_symbol`, `remove_watchlist_symbol`, `reorder_watchlist_symbols`) đều phải cập nhật `watchlists.updated_at_ms = now_ms`.
  3. Viết hàm chuẩn hóa chung `normalize_watchlist_item(raw: &str) -> String`: nếu `raw.trim().starts_with("###")` thì giữ nguyên chữ hoa/thường Unicode `raw.trim().to_owned()`, ngược lại mới `.trim().to_ascii_uppercase()`. Áp dụng thống nhất ở `add_watchlist_symbol`, `remove_watchlist_symbol`, `reorder_watchlist_symbols`, và `upsert_watchlist_full`.
  4. Trong `sync.rs`:
     - So sánh đúng `wl.updated_at_ms >= local_wl.updated_at_ms` (với gói `.sumi.json` cũ có `wl.updated_at_ms == 0`, chỉ nạp nếu `local_wl` chưa tồn tại hoặc `0 >= local_ts`, tuyệt đối không lấy `bundle.exported_at_ms` đè lên `local_ts > 0`!).
     - So sánh tombstone watchlist chuẩn LWW: chỉ xóa khi `del_ts >= local_wl.updated_at_ms` và không có bản tái tạo mới hơn trong bundle.
     - Kiểm tra các bảng `indicator_templates`, `journal_annotations`, `replay_sessions` trong `workspace_db.rs`: đảm bảo mọi hàm `*_with_ts` dùng khi merge đều có mệnh đề `WHERE excluded.updated_at_ms >= <table>.updated_at_ms` giống `save_screen_with_ts` (`workspace_db.rs:2369`).
  5. Kích hoạt thật `auto_sync_on_launch`: trong `AppState::initialize` (hoặc `lib.rs` setup), nếu `load_sync_config(&config_dir).auto_sync_on_launch == true` và `sync_folder` hợp lệ có tệp bundle, tự động tạo bản sao lưu trước đồng bộ và gọi `sync_with_folder` an toàn (ghi log `tracing::info!` / `warn!`, không làm crash app nếu thư mục đám mây tạm thời chưa mount).
  6. Trong `ui/src/App.tsx:116-130` & `ui/src/store/workspaceStore.ts`: lọc bỏ mọi phần tử bắt đầu bằng `'###'` trước khi truyền danh sách mã xuống `api.subscribeQuotes` / `api.getWatchlistSnapshots`.
  7. Trong `crates/sumi-desktop/src/commands/workspace.rs:1144-1150` (`open_detached_window`): dùng `url::form_urlencoded::byte_serialize` để mã hóa chuẩn `ticker` và `interval` (hỗ trợ đầy đủ dấu cách, `/`, `+`, `-`, `(` `)` của biểu đồ công thức và ký tự Unicode).

---

## Nhóm `6B` — Cách ly Tuyệt đối `True Blind Replay Drill` & Chống Rò rỉ Tương lai `as_of` (`C-03`, `H-03`, `S-03`)

### `W6-03` [CRITICAL / `C-03`] Khóa chặt toàn bộ các lỗ hổng rò rỉ danh tính & lệch thang giá của `True Blind Replay Drill`
- **Hiện trạng lỗi tại `crates/sumi-desktop/src/commands/replay.rs`, `analytics.rs`, `analog_cone.rs`:**
  1. `save_replay_session` (`replay.rs:1034-1062`) cho phép lưu phiên khi `blind_drill` chưa lật mở, ghi thẳng `rt.ticker` (mã thật `"HOSE:FPT"`) lộ ra UI và lưu `broker_snapshot_json` ở thang giá `100.00` vào mã thật.
  2. `replay_step` (`replay.rs:748-760`) gọi `evaluate_replay_bar_alerts(&ws, &ticker, ...)` bằng mã thật `rt.ticker` trên giá `100.00`, làm nổ cảnh báo của mã thật và bắn Toast lộ tên mã.
  3. `materialize_replay_bars` (`replay.rs:165-198`) khi đổi khung sang `1W` hoặc `1h` lấy nến gốc `132,500 VND` từ SQLite ghép vào nến `100.00` mà quên nhân `bd.price_scale`.
  4. Khi bấm `replay_reveal_blind_drill`, `rt.base_bars` và `replay_broker` vẫn kẹt ở thang giá `100.00` (`ExchangeVenue::UsEquities`), nhưng UI đổi `pane.ticker` về `"HOSE:FPT"`, gây lệch thang giá khi tải tiếp hoặc đặt lệnh.
  5. `compute_volume_structure` (`analytics.rs:424`), `compute_dom_ladder_cmd` và `compute_analog_cone` (`analog_cone.rs:349`) gọi `SymbolKey::parse(&ticker)?` nên báo lỗi với `"MÃ ẨN #4242"`.
  6. Trục thời gian X của nến trong Blind Drill vẫn giữ `timestamp` lịch sử thật trên thanh Crosshair của KLineChart.
- **Yêu cầu sửa chữa bắt buộc:**
  1. **Chặn hoặc ẩn danh khi Lưu phiên:** Trong `save_replay_session`, nếu `rt.blind_drill.as_ref().is_some_and(|b| b.enabled && !b.revealed)` thì trả về lỗi `DesktopError::Validation("Không thể lưu phiên khi đang ở chế độ Luyện Mù chưa lật mở mã — hãy bấm 'Lật Mở Mã Thật' trước khi lưu.")` và ẩn/disable nút Lưu trên `ReplayControlBar.tsx` khi `isBlindDrillUnrevealed == true`.
  2. **Tắt đánh giá Cảnh báo mã thật khi đang Blind Drill chưa lật mở:** Trong `replay_step` (`replay.rs:748`), nếu `is_blind_masked` là `true`, chỉ đánh giá các cảnh báo có `alert.ticker == bd.alias_code` (tuyệt đối không đánh giá cảnh báo của `rt.ticker` thật!).
  3. **Nhân `bd.price_scale` đồng nhất cho mọi khung thời gian:** Trong `materialize_replay_bars` (`replay.rs:165-198`), nếu `rt.blind_drill` đang bật (`!bd.revealed` hoặc đang dùng chuỗi chuẩn hóa), nhân `scale_ohlcv_bars(..., bd.price_scale)` cho cả phần `older.bars` (khung lớn `1W/1M`) lẫn chuỗi nến khung nhỏ (`1h/15m`) truy vấn từ `store`.
  4. **Khôi phục thang giá thật sạch sẽ khi `replay_reveal_blind_drill`:** Khi người dùng bấm Lật mở mã thật, nạp lại `raw_bars` gốc (chưa nhân `price_scale`) cho `rt.base_bars` & `rt.base_candles`, đồng thời quy đổi lại (`price / bd.price_scale`) các lệnh/giao dịch đóng trong `state.replay_broker` (hoặc giữ báo cáo Debrief tính theo `% / R-multiple` bất biến theo phép co giãn tỷ lệ trước khi khôi phục biểu đồ về giá VND thật).
  5. **Hỗ trợ `"MÃ ẨN #XXXX"` trên `Volume Structure` & `DOM`:** Trong `analytics.rs` (`compute_volume_structure`, `compute_dom_ladder_cmd`), kiểm tra `state.replay` trước khi gọi `SymbolKey::parse`: nếu `rt.blind_drill` đang khớp `alias_code == ticker`, lấy trực tiếp chuỗi nến đã chuẩn hóa cắt tại `cursor_index` của `ReplayRuntime` để tính `AVWAP / FRVP / VRVP / TPO / Patterns / DOM` mượt mà mà không lộ mã thật! Với `AFC` (`analog_cone.rs`), khi đang `is_blind_masked`, trả về `supported_interval: false, status_message: Some("AFC tạm khóa trong chế độ Luyện Mù để chống lộ mã tương tự")` để không lộ danh sách `top_analogs` lịch sử.
  6. **Ẩn ngày lịch sử thật trên trục X / Crosshair khi chưa lật mở:** Dịch chuyển `timestamp` của chuỗi nến Blind Drill về một mốc kỷ nguyên giả định cố định (ví dụ bắt đầu từ `2000-01-03 UTC` tịnh tiến `86_400_000 ms` mỗi phiên, hoặc format custom trục X trên `KLineViewport.tsx` hiển thị `Phiên #1, #2, ...`) để thanh Crosshair không bao giờ tiết lộ ngày tháng thật (VD `2022-11-16` đáy VNINDEX).

---

### `W6-04` [HIGH / `H-03` & `S-03`] Khóa chặt Point-in-Time (`as_of`) cho khung `1H` trong `KT01 Verdict`, sửa Race Condition `applyDeepLink` & ngày `T+2` của `Trade Planner`
- **Yêu cầu sửa chữa bắt buộc:**
  1. Trong `crates/sumi-yumeko-bridge/src/evidence.rs:83-95, 174-190` (`evaluate_kt01_gate`), `crates/sumi-desktop/src/commands/yumeko.rs:135-141`, và `crates/sumi-cli/src/engine.rs:194-206`:
     - Khi có `cutoff_ms` (`as_of` cuối ngày `23:59:59 UTC+7` -> quy đổi chuẩn theo múi giờ Việt Nam `FixedOffset::east_opt(7 * 3600)`!), bắt buộc dùng `BarQueryMode::Forward { before_ms: cutoff_ms + 1 }` khi đọc `store.query_bars` cho **cả `1D` lẫn `1H`**, và bên trong `evaluate_kt01_gate` lọc thêm `hourly_bars` với `b.timestamp <= cutoff_ms` (nếu sau khi lọc còn `< 20` nến `1H` trước ngày `as_of` thì trả `hourly_timing = None` thay vì lấy nến tương lai!).
  2. Trong `ui/src/store/workspaceStore.ts:1193-1219` (`applyDeepLink`):
     - Cập nhật `set({ yumekoAsOf: payload.asOf ?? null })` **TRƯỚC** khi gọi `setActiveTicker` / `setActiveInterval`.
     - Trong `loadYumekoForActiveTicker` (`workspaceStore.ts:1129-1151`), gắn bộ đếm thế hệ yêu cầu (`requestSeq`) để nếu có 2 cuộc gọi chồng lấn, chỉ phản hồi của `requestSeq` mới nhất mới được cập nhật vào store.
  3. Trong `crates/sumi-desktop/src/commands/analytics.rs:700-715` (`compute_trade_plan_cmd`):
     - Nhận thêm tham số tùy chọn `entry_timestamp_ms: Option<i64>` (hoặc tự động lấy `cutoff_bar.timestamp` từ `state.replay` nếu Replay đang bật, tiếp theo lấy mốc `as_of` nếu đang khóa PIT, cuối cùng mới fallback về `Utc::now()`), để ngày hàng về `T+2` (hoặc `T+3` trước tháng 08/2022 và kỳ nghỉ Tết Âm lịch) trong `TradePlannerModal` chính xác tuyệt đối theo ngày nến Replay!

---

## Nhóm `6C` — Biểu đồ Công thức, Chuỗi BCTC Quý (`tradingview-rs::fundamental`) & Đồng bộ `Bar Magnifier` (`H-01`, `H-04`, `S-01`)

### `W6-05` [HIGH / `H-01`] Bảo toàn dấu ngoặc đơn `(...)` trong `expression.rs` & Mở khóa `AVWAP / Volume Profile / Patterns` trên Biểu đồ Tỷ số/Spread
- **Yêu cầu sửa chữa bắt buộc:**
  1. Trong `crates/sumi-data/src/expression.rs:294-316` (`format_canonical`):
     - Bổ sung độ ưu tiên toán tử (`precedence`: `Add/Sub = 1`, `Mul/Div = 2`, `Neg = 3`, `Atom = 4`). Khi format nút con `left` hoặc `right` có độ ưu tiên thấp hơn nút cha (hoặc `right` cùng độ ưu tiên với phép không giao hoán `Sub` / `Div`, ví dụ `A - (B - C)` hay `A / (B * C)`), **bắt buộc bọc dấu ngoặc đơn `(...)`** để `parse_expression(&format_canonical(&ast)) == ast` đúng 100% với mọi biểu thức!
     - Kiểm tra lại phép tính râu nến `high / low` của biểu thức tổng quát (`eval_ohlc` trong `expression.rs`): tính giá trị tại 4 điểm cực trị hoặc ít nhất trên tập `{eval(open), eval(close), eval_with_high_low, eval_with_low_high}` để luôn đảm bảo bất biến `low <= min(open, close) <= max(open, close) <= high` kể cả với phép trừ `HOSE:FPT - HOSE:CMG` hay số âm.
  2. Trong `crates/sumi-desktop/src/commands/analytics.rs:420-480` (`compute_volume_structure`):
     - Nếu `sumi_data::is_synthetic_expression(&ticker)` là `true`, gọi bộ nạp chuỗi nến biểu thức tổng hợp (cùng hàm đang phục vụ `get_bars` trong `commands/market.rs`) để tính trọn vẹn `AVWAP`, `FRVP`, `VRVP`, `TPO` và `Candlestick Patterns` cho mọi biểu đồ `HOSE:SSI / HOSE:VNINDEX` hay `(HOSE:FPT + HOSE:MWG) / 2`!

---

### `W6-06` [HIGH / `S-01`] Tích hợp thật `tradingview-rs::fundamental` cho Cổ phiếu ngoài 54 mã Yumeko & Xóa bỏ đường `FIN_PE` phẳng lì Lookahead Bias
- **Hiện trạng lỗi:**
  - `tradingview-rs` đã có sẵn module `fundamental` miễn phí (`STD;Fund_*` theo Quý `FQ` / Năm `FY` / `TTM`), nhưng chưa được gọi ở đâu.
  - `ui/src/chart/indicators/fundamentalIndicators.ts:98-220` đang vẽ `FIN_PE` bậc thang tĩnh ở mã Yumeko và vẽ **1 đường ngang phẳng lì suốt 10 năm bằng P/E hôm nay** ở mọi mã khác (`> 1,500` mã VN + mã US + máy Clean-Slate).
- **Yêu cầu sửa chữa bắt buộc:**
  1. Kiểm tra API thực tế của crate `tradingview` đang khai báo trong `Cargo.lock` (module `fundamental` / `StudyClient` cho các chỉ tiêu quý cốt lõi: `EPS` / `Diluted EPS`, `Book Value per Share` hoặc `P/B`, `ROE`, `Net Income YoY` / `Total Revenue`), bổ sung bảng cache SQLite `symbol_quarterly_fundamentals (symbol, period_end_ms, period_label, eps_ttm, bvps, pe, pb, roe_pct, net_income_yoy_pct, source, updated_at_ms, PRIMARY KEY(symbol, period_label))` trong `sumi_market.db`.
  2. Trong `get_symbol_fundamentals` (`crates/sumi-desktop/src/commands/market.rs:1832-1900`):
     - **Ưu tiên 1:** Đọc chuỗi quý Point-in-Time từ `YumekoReadOnlyBridge` (`DN01 CSTC`) nếu có dữ liệu cho mã đó (lọc `<= as_of`).
     - **Ưu tiên 2:** Nếu mã nằm ngoài 54 mã Yumeko (hoặc máy không cài Yumeko), đọc từ bảng cache `symbol_quarterly_fundamentals`; nếu cache trống/cũ và đang Online, gọi `tradingview-rs` tải chuỗi BCTC theo quý, lưu vào `symbol_quarterly_fundamentals` và trả về `quarterlyValuations` đầy đủ (cắt nghiêm ngặt `period_end_ms <= as_of` để không rò rỉ BCTC tương lai!).
  3. Trong `ui/src/chart/indicators/fundamentalIndicators.ts`:
     - **Xóa bỏ hoàn toàn đoạn fallback vẽ đường ngang `cached.pe` xuyên suốt lịch sử quá khứ (`lines 208-219`)!** Nếu một cây nến lịch sử chưa có kỳ BCTC nào được công bố tính tới ngày nến đó (`bar.timestamp < firstQuarterAvailable`), trả về `{}` (`undefined`) một cách trung thực.
     - Tính **P/E TTM động hằng ngày (`Close[t] / EPS_TTM[q(t)]`)** và **P/B động hằng ngày (`Close[t] / BVPS[q(t)]`)** khi kỳ quý `q(t)` có `eps_ttm > 0` / `bvps > 0` (hoặc nội suy từ `close_at_quarter / pe_quarter` nếu chỉ có `pe` quý), giúp đường `FIN_PE` và `FIN_PB` phản ánh đúng biến động giá từng phiên thay vì bậc thang chết!

---

### `W6-07` [HIGH / `H-04`] Đồng bộ `Bar Magnifier` xuyên suốt `WFO`, `Monte Carlo` và `HTML/PDF Report Export`
- **Yêu cầu sửa chữa bắt buộc:**
  - Trong `crates/sumi-desktop/src/commands/backtest.rs`:
    1. Khi `cfg.risk.use_bar_magnifier == true`, nạp `intrabar_candles = load_intrabar_magnifier_candles(&state, &ticker, iv, &candles)` đồng nhất cho **cả 4 lệnh**:
       - `run_strategy_backtest` (đã có),
       - `run_strategy_monte_carlo` (`backtest.rs:976-987` — thay `run_backtest_with_options` bằng `run_backtest_with_magnifier`),
       - `export_backtest_report_html` (`backtest.rs:1220-1226` — thay `run_backtest_with_options` bằng `run_backtest_with_magnifier` để bảng lệnh & PnL trong file báo cáo xuất ra khớp `100%` tới từng đồng với màn hình UI!),
       - `run_strategy_wfo` (truyền `intrabar_candles` xuống các fold kiểm định OOS nếu `use_bar_magnifier` bật, hoặc ghi rõ trạng thái magnifier trong metadata).

---

## Nhóm `6D` — Alerts 2.0, Sửa hình học `Andrews Pitchfork`, Đồng bộ CLI/MCP & Cổng Kiểm định CI (`H-02`, `H-05`, `S-02`, `S-04`)

### `W6-08` [HIGH / `H-02` & `S-03`] Sửa lỗi hình học `Andrews Pitchfork` & Nối tham số Settings cho chỉ báo `CVD` / `CANDLE_PATTERNS`
- **Yêu cầu sửa chữa bắt buộc:**
  1. Trong `ui/src/chart/overlays/proOverlays.ts:610-617` (`andrewsPitchfork`):
     - `getRayLine([a, b], bounding)` trong `ui/src/chart/overlays/utils.ts:23-51` trả về `LineAttrs[]` dạng `[{ coordinates: [startCoord, projectedEndCoord] }]` (độ dài mảng là `1`, không phải `2`!).
     - Sửa cách trích xuất điểm cuối tia thành:
       ```ts
       const medianEnd = medianRay[0]?.coordinates?.[1] ?? mid
       const upperEnd = upperRay[0]?.coordinates?.[1] ?? { x: p1.x + dx, y: p1.y + dy }
       const lowerEnd = lowerRay[0]?.coordinates?.[1] ?? { x: p2.x + dx, y: p2.y + dy }
       ```
     - Bổ sung unit test trong `ui/src/__tests__/sumiV3Wave5.test.ts` gọi trực tiếp `andrewsPitchfork.createPointFigures!({ coordinates: [{x:10,y:100},{x:40,y:60},{x:50,y:140}], bounding: {width:800,height:500,left:0,right:800,top:0,bottom:500}, ... })` và assert cả 3 tia `median`, `upper`, `lower` đều vươn tới biên phải `x === 800` với độ dài `> 0`!
  2. Trong `ui/src/chart/indicators/wave3Indicators.ts` & `ui/src/chart/indicators/catalog.ts`:
     - Với `CVD` (`wave3Indicators.ts:348-384`): hỗ trợ `calcParams: [0]` (`0` = cộng dồn toàn chuỗi, `> 0` = chu kỳ EMA đường tín hiệu hoặc cửa sổ rolling `N` phiên khớp với `catalog.ts`) và đọc thật `indicator.calcParams` trong hàm `calc`.
     - Với `CANDLE_PATTERNS` (`wave3Indicators.ts:554-623`): hỗ trợ `calcParams: [14]` (chu kỳ ATR để đo độ lệch Tweezer/Body) và đọc `indicator.calcParams[0]` thay vì khóa cứng `14`.

---

### `W6-09` [HIGH / `H-05`] Hoàn thiện `Alerts 2.0`: Lọc đúng `interval` + `created_at_ms`, gửi Telegram/Webhook ở `check_bar_alerts` & Bắn OS Notification thật
- **Yêu cầu sửa chữa bắt buộc:**
  1. Trong `crates/sumi-desktop/src/commands/workspace.rs:1074-1120`:
     - Đổi chữ ký `evaluate_bar_alerts_slice` nhận thêm `interval_filter: Option<&str>` và kiểm tra: nếu `a.interval` là `Some(ref a_iv)` và `interval_filter` là `Some(req_iv)` thì **chỉ đánh giá khi `a_iv.eq_ignore_ascii_case(req_iv)`** (chặn triệt để lỗi nến `1H` kích hoạt nhầm cảnh báo `1D`!). Truyền `Some(&iv_str)` ở cả `scan_watchlist_alerts` (`1009`) và `check_bar_alerts` (`1067`), truyền `Some(rt.interval.as_str())` ở `replay_step`.
     - Trong `evaluate_drawing_line_price_at` (`workspace.rs:667-717`), khi kiểm tra cắt đường xu hướng nghiêng (`segment / rayLine / straightLine`) trên nến đóng cửa, tính giá đường thẳng tại `prev_bar_ts` cho `prev_close` và tại `bar.timestamp` cho `bar` để tránh báo cắt giả trên đường dốc đứng.
  2. Trong `check_bar_alerts` (`workspace.rs:1033-1071`):
     - Sau khi có danh sách `fired` không rỗng, **bắt buộc kích hoạt gửi bất đồng bộ `dispatch_external_alert_channels` (Telegram + Webhook)** và cập nhật lại `delivered_channels` giống hệt `check_price_alerts` và `scan_watchlist_alerts`!
  3. **OS Notification thật:**
     - Trên Frontend (`ui/src/store/workspaceStore.ts` / `RightWidgetBar.tsx` nơi xử lý mảng cảnh báo vừa nổ): khi `alertDeliveryConfig.osNotificationEnabled` bật, gọi chuẩn Web `Notification` API của WebView hệ điều hành (`if ('Notification' in window) { ... new Notification(...) }`) song song với âm báo và In-App Toast để thông báo hiện lên trung tâm thông báo của macOS / Windows Notification Center thật sự!

---

### `W6-10` [MEDIUM / `S-02` & `S-04`] Đồng bộ đầy đủ Subcommands cho `sumi-cli` (`bridge status`, `sync`), nối `YumekoReadOnlyBridge` vào `sumi screen` / MCP & Khóa cổng CI `-D warnings`
- **Yêu cầu sửa chữa bắt buộc:**
  1. Trong `crates/sumi-cli/src/main.rs` & `engine.rs` & `mcp.rs`:
     - Bổ sung 2 subcommands còn thiếu theo đặc tả Master Plan Wave 2.4:
       - `sumi bridge status [--json]`: gọi `YumekoReadOnlyBridge::discover_default().get_status()` in trạng thái kết nối `yumeko-app.sqlite`, số mã `QMV` / `CSTC` / `DNSE PIT` và danh sách ngày `as_of`.
       - `sumi sync [--status | --now | --export <path> | --import <path>]`: dùng chung mô-đun đồng bộ `.sumi.json` để có thể đồng bộ từ terminal/script tự động hóa.
     - Nâng cấp `run_mtf_screen_headless` (`crates/sumi-cli/src/engine.rs:235-395`) và MCP tool `sumi_mtf_screen` (`mcp.rs:160-170`): kết nối `YumekoReadOnlyBridge` để hỗ trợ lọc `--only-money-in`, `--min-tmc`, `--max-pe`, `--min-roe`, `--weekly-regime`, `--as-of` đồng nhất với Screener trên Desktop UI!
  2. Trong `.github/workflows/ci-cross-platform.yml`:
     - Đổi lệnh Clippy dòng `52` thành `cargo clippy --workspace --all-targets -- -D warnings`.
     - Thêm bước kiểm tra build nhị phân desktop với `--features custom-protocol` (`cargo check -p sumi-desktop --features custom-protocol`) trên cả `macos-latest` và `windows-latest` để không bao giờ tái phát lỗi `C-01`.

---

# PHẦN B — MASTER PROMPT THI CÔNG `WAVE 6` (COPY-PASTE SANG PHIÊN MỚI)

> Sao chép nguyên văn khối dưới đây dán vào phiên làm việc mới để Agent tự động đọc đặc tả, lập kế hoạch ngắn gọn, sửa dứt điểm toàn bộ 10 hạng mục `W6-01 -> W6-10`, kiểm thử hồi quy và đóng gói nghiệm thu:

```markdown
Bạn đóng vai **Principal Quantitative Platform Architect, Rust/Tauri Systems Engineer & Release Gate Lead** cho dự án **Sumi Supercharts (`v3.0.0`)** tại `/Users/mizuhara/workspace/sumi`.

Nhiệm vụ của bạn trong phiên này là thực thi trọn vẹn **Đợt Vá Lỗi & Hoàn Thiện Cổng Phát Hành (`Wave 6 Hardening`)** theo đúng bản đặc tả kỹ thuật chi tiết đã được kiểm toán tại:
👉 `/Users/mizuhara/workspace/sumi/docs/architecture/SUMI_V3_WAVE6_HARDENING_SPEC_AND_PROMPT.md`

---

## 1. NGUYÊN TẮC BẮT BUỘC (HARD GUARDS)

1. **Đọc kỹ Đặc tả trước khi sửa code:**
   - Đọc đầy đủ `/Users/mizuhara/workspace/sumi/docs/architecture/SUMI_V3_WAVE6_HARDENING_SPEC_AND_PROMPT.md` và `/Users/mizuhara/workspace/sumi/docs/architecture/SUMI_V3_YUMEKO_TOPDOWN_AND_CROSS_PLATFORM_MASTER_PLAN.md`.
   - Kiểm tra trực tiếp từng tệp và dòng code được nêu trong `W6-01` đến `W6-10` trước khi chỉnh sửa.
2. **Bảo vệ tuyệt đối các kho mã liên quan:**
   - **TUYỆT ĐỐI KHÔNG sửa đổi bất kỳ tệp nào** trong `/Users/mizuhara/workspace/yumeko` hoặc `/Users/mizuhara/workspace/Doraemi`.
   - `sumi-yumeko-bridge` giữ nguyên chế độ Chỉ Đọc (`SQLITE_OPEN_READ_ONLY` + `PRAGMA query_only = ON`) đối với `~/.local/share/yumeko/yumeko-app.sqlite`.
3. **Không làm tắt — Không để lại Dead Code / Stub / Hardcoded Path:**
   - Không dùng đường dẫn cứng `/Users/...`.
   - Không xấp xỉ chỉ báo tài chính quá khứ bằng đường ngang lấy số hiện tại (`lookahead flat-line`).
   - Mọi tùy chọn cấu hình trên UI/CLI (`auto_sync_on_launch`, `Bar Magnifier`, `calcParams` của `CVD`/`CANDLE_PATTERNS`, lọc `interval` của Alerts, `as_of` cho nến `1H`) đều phải thực thi thật từ đầu đến cuối.

---

## 2. DANH MỤC 10 HẠNG MỤC BẮT BUỘC HOÀN THÀNH (`W6-01` -> `W6-10`)

Hãy triển khai tuần tự, sạch sẽ theo 4 nhóm trong tài liệu đặc tả:

1. **Nhóm `6A` (`W6-01`, `W6-02`) — Đóng gói Đa nền tảng, Deep-Link macOS & Đồng bộ Đa máy:**
   - `W6-01`: Bổ sung feature `custom-protocol = ["tauri/custom-protocol"]` trong `crates/sumi-desktop/Cargo.toml` và truyền `--features custom-protocol` (kèm build `sumi-cli`) trong cả `scripts/build-macos-app.sh` lẫn `scripts/build-windows-app.ps1` để chấm dứt triệt để lỗi màn hình trắng `localhost:1420`. Chuyển `.run(...)` trong `crates/sumi-desktop/src/lib.rs` sang `.build(...)?.run(|app_handle, event| ...)` bắt `tauri::RunEvent::Opened { urls }` cho `sumi://` trên macOS + hàng đợi `take_pending_deeplink` chống mất sự kiện khi cold-start. Hỗ trợ biến môi trường `SUMI_MARKET_DB` & `SUMI_WORKSPACE_DB` trong `resolve_database_paths` và sửa ghi chú đường dẫn trong `scripts/build-windows-app.ps1`.
   - `W6-02`: Thêm `updated_at_ms` vào `WatchlistGroup` (Rust + TS), cập nhật `watchlists.updated_at_ms` ở mọi thao tác thêm/xóa/đổi tên/sắp xếp mã, bảo toàn chữ hoa/thường Unicode cho nhãn `###SECTION:...`, sửa `merge_bundle_into_store` trong `sync.rs` để so sánh đúng `wl.updated_at_ms >= local_ts` và `del_ts >= local_ts` (không dùng `bundle.exported_at_ms` ghi đè làm mất mã!), đảm bảo mọi hàm `*_with_ts` trong `workspace_db.rs` có `WHERE excluded.updated_at_ms >= <table>.updated_at_ms`, kích hoạt chạy thật `auto_sync_on_launch` khi khởi động app, lọc bỏ `###SECTION:` khỏi `subscribeQuotes`, và URL-encode đầy đủ tham số trong `open_detached_window`.

2. **Nhóm `6B` (`W6-03`, `W6-04`) — Cách ly Tuyệt đối `Blind Drill` & Chống Rò rỉ `as_of`:**
   - `W6-03`: Trong `True Blind Replay Drill` chưa lật mở: chặn `save_replay_session` (và ẩn/khóa nút Lưu trên UI), không đánh giá cảnh báo của mã thật `rt.ticker`, nhân `bd.price_scale` đồng nhất cho cả nến khung lớn (`1W/1M`) lẫn khung nhỏ (`1h/15m`) trong `materialize_replay_bars`, khôi phục chuỗi nến về thang giá VND thật khi gọi `replay_reveal_blind_drill`, hỗ trợ `"MÃ ẨN #XXXX"` chạy mượt trên `AVWAP / FRVP / VRVP / TPO / Patterns / DOM` (và khóa thông minh `AFC` khi đang ẩn mã), đồng thời ẩn ngày tháng lịch sử thật trên trục X / Crosshair khi chưa lật mở.
   - `W6-04`: Lọc nghiêm ngặt nến `1H` theo mốc cuối ngày `as_of` (`UTC+7`) trong `yumeko_export_kt01_verdict`, `analyze_kt01_headless` và `evaluate_kt01_gate`. Sửa thứ tự gán `yumekoAsOf` + chống race condition trong `applyDeepLink` / `loadYumekoForActiveTicker`. Truyền đúng timestamp nến Replay / `as_of` vào `compute_trade_plan_cmd` để ngày hàng về `T+2` khớp lịch sử.

3. **Nhóm `6C` (`W6-05`, `W6-06`, `W6-07`) — Biểu đồ Công thức, Chuỗi BCTC Quý & Đồng bộ `Bar Magnifier`:**
   - `W6-05`: Sửa `format_canonical` trong `crates/sumi-data/src/expression.rs` bảo toàn dấu ngoặc đơn theo độ ưu tiên toán tử (`(HOSE:FPT + HOSE:MWG) / 2` round-trip chính xác 100%) + đảm bảo bất biến `low <= open,close <= high` cho mọi phép trừ/số âm; mở khóa `compute_volume_structure` chạy trên mọi biểu đồ công thức `is_synthetic_expression`.
   - `W6-06`: Tích hợp nguồn dữ liệu BCTC theo quý từ `tradingview-rs` (`fundamental` / financial studies có sẵn trong dependency `tradingview`) lưu cache vào bảng `symbol_quarterly_fundamentals` trong `sumi_market.db` làm nguồn dự phòng cho mọi cổ phiếu ngoài 54 mã Yumeko `CSTC` (và máy Clean-Slate). Trong `ui/src/chart/indicators/fundamentalIndicators.ts`, xóa bỏ hoàn toàn đường ngang `cached.pe` lookahead 10 năm và tính `FIN_PE` / `FIN_PB` động hằng ngày theo `Close[t] / EPS_TTM[q(t)]` và `Close[t] / BVPS[q(t)]` tuân thủ Point-in-Time.
   - `W6-07`: Đồng bộ `load_intrabar_magnifier_candles` + `run_backtest_with_magnifier` cho cả `export_backtest_report_html`, `run_strategy_monte_carlo` và `run_strategy_wfo` khi bật `use_bar_magnifier`.

4. **Nhóm `6D` (`W6-08`, `W6-09`, `W6-10`) — Overlays, Alerts 2.0, CLI/MCP Parity & Cổng CI:**
   - `W6-08`: Sửa lỗi trích xuất tia `getRayLine` của `andrewsPitchfork` trong `ui/src/chart/overlays/proOverlays.ts` để vẽ đầy đủ 3 nhánh song song tới biên biểu đồ; nối `indicator.calcParams` thật cho `CVD` và `CANDLE_PATTERNS` trong `wave3Indicators.ts`.
   - `W6-09`: Lọc đúng `a.interval` trong `evaluate_bar_alerts_slice`, nội suy đúng mốc thời gian trước/sau cho cảnh báo cắt trendline nghiêng, kích hoạt gửi Telegram/Webhook trong `check_bar_alerts`, và phát OS Notification thật khi `osNotificationEnabled` bật.
   - `W6-10`: Bổ sung `sumi bridge status` và `sumi sync` vào `sumi-cli`; nối `YumekoReadOnlyBridge` vào `run_mtf_screen_headless` / MCP tool `sumi_mtf_screen`; cập nhật `.github/workflows/ci-cross-platform.yml` bật `-- -D warnings` cho Clippy và kiểm tra build `--features custom-protocol`.

---

## 3. CỔNG KIỂM CHỨNG BẮT BUỘC TRƯỚC KHI BÁO CÁO HOÀN TẤT (VERIFICATION GATES)

Trước khi kết thúc phiên, bạn **bắt buộc chạy thật và đạt 100% xanh** toàn bộ các kiểm chứng sau:

1. **Unit & Regression Tests khóa chặt từng lỗi:**
   - Viết thêm unit/integration tests mới cho tất cả các lỗi đã sửa:
     - Round-trip dấu ngoặc công thức `(HOSE:FPT + HOSE:MWG) / 2` và `HOSE:FPT - (HOSE:MWG - HOSE:VCB)`.
     - Merge 2 chiều Watchlist giữa Máy A (`T2` sửa watchlist) và Máy B (`T3` chỉ vẽ hình, không sửa watchlist) -> không mất mã trên Máy A + giữ nguyên chữ hoa/thường `###SECTION:Ngân Hàng`.
     - `Blind Replay Drill`: chặn lưu phiên khi chưa reveal, không nổ alert mã thật, nhân `price_scale` đúng trên khung `1W`/`1h`, chạy được `compute_volume_structure` trên `"MÃ ẨN #XXXX"`.
     - `KT01 Verdict` với `as_of` quá khứ lọc sạch nến `1H` tương lai.
     - `export_backtest_report_html` và `run_strategy_monte_carlo` khớp kết quả với `run_strategy_backtest` khi bật `Bar Magnifier`.
     - `andrewsPitchfork.createPointFigures` sinh đủ 3 tia có độ dài `> 0`.
2. **Kiểm chứng Toàn Workspace Rust & Frontend:**
   - `cargo test --workspace` -> `0` failed.
   - `cargo clippy --workspace --all-targets -- -D warnings` -> `0` warnings.
   - `cargo check -p sumi-desktop --features custom-protocol` -> thành công.
   - `npm --prefix ui run typecheck && npm --prefix ui run test && npm --prefix ui run build` -> `0` errors.
3. **Kiểm chứng Đóng gói Độc lập & CLI Runtime thật:**
   - Chạy `./scripts/build-macos-app.sh --no-install` -> tạo thành công `dist-macos/Sumi Supercharts.app` (có cả `sumi-desktop` và `sumi` CLI bên trong `Contents/MacOS/`), `codesign --verify --deep --strict` pass, và xác nhận binary nhúng asset `ui/dist` thật (`custom-protocol`), không phụ thuộc `localhost:1420`.
   - Chạy thử `./target/release/sumi bridge status` và `./target/release/sumi kt01 --symbol HOSE:FPT --no-dropzone` kiểm chứng kết nối Read-Only tới Yumeko SQLite hoạt động trơn tru.
4. **Git Commit gọn sạch:**
   - Đảm bảo không để rác build tạm ngoài `.gitignore`, kiểm tra `git status` trên `yumeko` và `Doraemi` không bị thay đổi bởi phiên làm việc, sau đó tạo commit chuẩn `fix(v3-wave6): harden cross-platform packaging, sync LWW, blind replay isolation, PIT and fundamental series` trên kho `/Users/mizuhara/workspace/sumi`.
```
