# 08. Báo Cáo Kiểm Định Toàn Diện & Danh Sách Lỗi (`P0 → P4`) — Sumi Supercharts `v2.2.0` (QMV Workstation Edition)

> **Ngày kiểm định:** 24–25/11/2026  
> **Đối tượng kiểm định:** Sumi Supercharts `v2.2.0` (QMV Workstation Edition) — `/Applications/Sumi Supercharts.app` & mã nguồn workspace `/Users/mizuhara/workspace/sumi` (chế độ **Read-Only 100%**, không chỉnh sửa bất kỳ tệp mã nguồn nào trong `crates/` hay `ui/`).  
> **Phương pháp kiểm định Hai Tầng Thực Tế (`0% webFallback.ts` cho nghiệm thu chính):**  
> - **Tầng A (Native macOS `.app` & SQLite):** Kiểm chứng `codesign --verify --deep --strict`, khởi chạy lạnh (`0.139s`) / ấm (`0.095s`), RSS `~122–126 MB`, file descriptors `WAL` trên `sumi_market.db` (`88.5 MB`) & `sumi_workspace.db`, di trú không phá hủy `migrate_qmv_workstation_v1`, đóng/mở sạch và **đã khôi phục nguyên trạng `sumi_workspace.db`** của người dùng.  
> - **Tầng B (Cầu nối E2E Lõi Rust Thật + Playwright Chromium `2× HiDPI`):** Biên dịch cầu nối cô lập trong thư mục scratch liên kết trực tiếp `sumi_desktop_lib` (`AppState` + toàn bộ 55+ Tauri IPC commands + `tauri::test::mock_builder()`) gắn vào bản sao `sumi_market.db` thật (`486` mã `1D`, `951.669+` nến), phục vụ bản build chuẩn `ui/dist`, thực thi tương tác chuột/phím thật ở **`1440×900`** & **`1280×800`** trên cả **`Dark Mode`** và **`Light Mode`**, chụp và **kiểm toán bằng mắt 26 ảnh chụp PNG** tại [`docs/qa/evidence_qmv/`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence_qmv/).  
> - **Lưu ý macOS TCC:** Tuân thủ nghiêm giới hạn quyền hệ điều hành macOS (không vượt rào `Screen Recording` / `Accessibility`).

---

## 1. Bảng Tổng Hợp Kết Quả 80 Test Cases (8 Phân Hệ)

| # | Phân hệ kiểm thử | Tiền tố | Tổng | PASS | FAIL | BLOCKED | Tỷ lệ PASS | Tóm tắt phát hiện cốt lõi |
|---|---|---|---|---|---|---|---|---|
| 1 | Khởi động `.app`, Di trú SQLite & Vũ trụ 475 mã VN | `TC-QMV-BOOT` | **8** | 5 | 3 | 0 | 62.5% | Cold start, di trú bảo toàn dữ liệu cá nhân, native `.app` WAL & `webFallback` đều tốt; **6 mã trà đá `300–400đ` bị phân sai thành `OVER_50K` (`BUG-QMV-005`)**, **`VN30F1!` trống giá trên Watchlist & `VNXALL` đóng băng từ 2019 lọt vào Screener (`BUG-QMV-010`)**. |
| 2 | Danh mục Chỉ báo Tinh gọn (10 + 5) & Modal Cấu hình | `TC-QMV-CAT` | **8** | 5 | 3 | 0 | 62.5% | Đủ 10 Core + 5 Aux, gỡ sạch 20 chỉ báo nhiễu, tương thích ngược `ICHIMOKU [9,26,52]` + `SUPERTREND`; **`VRVP` phớt lờ đổi màu POC & `addIndicator`/`Modal` ghi đè phá hỏng màu gốc `ICHIMOKU`/`QMV_FLOW` (`BUG-QMV-011`)**, không kẹp trần tham số `9999` (`BUG-QMV-018`). |
| 3 | Toán học & Đồ họa Canvas Bộ Chỉ Báo QMV Mới | `TC-QMV-IND` | **10** | 8 | 2 | 0 | 80.0% | Toán học `ICHIMOKU [6,18,36,18]` (sai số `0`), Mây Kumo tương lai 18p + clipping trục Y, `QMV_FLOW`/`RSI`/`MACD` khớp `sumi-core` tới 6 chữ số thập phân, Alert RSI & Lazy-load cực tốt; **bẫy Cold-Start Race `RS_VNINDEX` kẹt `83.66` vs thật `33.46` (`BUG-QMV-004`)**, `RSI(14)` dư nhãn `RSI2/RSI3` & `MACD` 4 số lẻ (`BUG-QMV-015`). |
| 4 | 4 Mẫu Chỉ Báo QMV, Cây Đối Tượng & Đa Biểu Đồ | `TC-QMV-TPL` | **6** | 4 | 2 | 0 | 66.7% | 4 mẫu QMV áp dụng chuẩn, 12 lần chuyển đổi không rò rỉ sub-pane, CRUD mẫu cá nhân & lưu đa ô tốt; **Object Tree hiện mã thô `QMV_FLOW`/`RS_VNINDEX` (`BUG-QMV-023`)**, ô `1H` đa biểu đồ chồng nát nhãn `Phiên` (`BUG-QMV-016`). |
| 5 | Bộ Lọc Kỹ Thuật QMV (`StockScreenerTab` — 123 TH) | `TC-QMV-SCR` | **14** | 8 | 6 | 0 | 57.1% | Ẩn sạch Crypto/US, tốc độ siêu nhanh (lạnh `291ms` / ấm `164ms`), 5/5 mã đối chiếu OHLCV gốc khớp 100%, ghép `AND` & `B5` hoạt động tốt; **nút lọc nhanh #5 dùng ID ma `volume.rvol_spike_1_5x` (`BUG-QMV-007`)**, **mất 3 ô lọc số `% Đổi`/`MACD`/`Điểm SK` (`BUG-QMV-008`)**, tìm `"Ngân hàng"` thiếu 22/26 mã (`BUG-QMV-017`), `VNXALL` lọt CSV (`BUG-QMV-010`). |
| 6 | Kiểm Định Chiến Lược QMV (`StrategyTesterTab`) | `TC-QMV-BT` | **16** | 13 | 3 | 0 | 81.3% | Vi cấu trúc VN (lô `100`, `T+3` trước `29/08/2022` & `T+2` sau, khóa sàn), `DEF-001` Zero-Lookahead `100%`, `VN30F1!` Short/T+0, WFO, MC tất định, click lệnh `2017` tự mở rộng `2,715` nến đều xuất sắc; **3 lỗi P0 cốt lõi: `Cổng VNINDEX` hỏng trên S1–S5 (`BUG-QMV-001`), lệch 100% `qmvDivOsc` S2 (`BUG-QMV-002`), ép cứng `MaxBars=25` + không tắt được `SL Đáy` (`BUG-QMV-003`)**, cộng **rò rỉ state 6 nút Playbook (`BUG-QMV-009`)**. |
| 7 | Tích Hợp Chéo: Bar Replay Nhân Quả & Paper Broker | `TC-QMV-RPL` | **8** | 6 | 2 | 0 | 75.0% | `0` mũi tên Backtest tương lai lọt qua, MTF Replay `1h+4h+1D` + Save/Resume, khóa `T+2` FIFO, bước giá HOSE `50đ`, Drawing Alert & Journal đều PASS; **`StockScreenerTab` âm thầm lộ dữ liệu 2026 trong Replay (`BUG-QMV-012`)**, **`Blind Mode` chưa ẩn tên mã & lộ giá live 29 mã khác (`BUG-QMV-013`)**. |
| 8 | Kiểm Toán UI/UX, Khung Hẹp, Sáng/Tối & Thực Chiến | `TC-QMV-UX` | **10** | 5 | 5 | 0 | 50.0% | 100-Action Soak xuất sắc (`0` page error, `0` rò rỉ stream/callback, JS Heap `19.55 → 19.55 MB`), `Cmd+K` không cướp phím input, Reload bảo toàn tốt; **Dock mặc định `276px` bóp nghẹt `StrategyTester` còn `54px` (`0px` đồ thị ở `1280×800`, `BUG-QMV-006`)**, **chữ vàng `#FBC02D` chìm trên `Light Mode` (`BUG-QMV-014`)**, Toast đè tab Watchlist & cắt chữ `Đối tượng` (`BUG-QMV-019`), sót câu chữ cũ (`BUG-QMV-022`). |
| **TỔNG** | **8 Phân Hệ Chuyên Sâu** | **`TC-QMV-*`** | **80** | **54** | **26** | **0** | **67.5%** | **Phát hiện 25 Bug / UX Finding xác thực (`3 P0 · 10 P1 · 8 P2 · 3 P3 · 1 P4`).** |

### Ma Trận Trạng Thái Chi Tiết 80/80 Test Cases

| Phân hệ | Trạng thái từng Test Case |
|---|---|
| **TC-QMV-BOOT** (`8`) | `001:PASS`, `002:PASS`, `003:PASS`, `004:FAIL(BUG-QMV-010)`, `005:FAIL(BUG-QMV-010)`, `006:FAIL(BUG-QMV-005)`, `007:PASS*(BUG-QMV-020)`, `008:PASS` |
| **TC-QMV-CAT** (`8`) | `001:PASS*(BUG-QMV-022)`, `002:PASS*(BUG-QMV-018/022)`, `003:PASS*(BUG-QMV-014/023)`, `004:PASS`, `005:FAIL(BUG-QMV-011)`, `006:FAIL(BUG-QMV-011)`, `007:FAIL(BUG-QMV-018)`, `008:PASS` |
| **TC-QMV-IND** (`10`) | `001:PASS`, `002:PASS`, `003:PASS`, `004:PASS`, `005:FAIL(BUG-QMV-004)`, `006:PASS*(BUG-QMV-011/014)`, `007:FAIL(BUG-QMV-015)`, `008:PASS*(BUG-QMV-015)`, `009:PASS`, `010:PASS` |
| **TC-QMV-TPL** (`6`) | `001:PASS`, `002:PASS`, `003:PASS`, `004:FAIL(BUG-QMV-023)`, `005:FAIL(BUG-QMV-016)`, `006:PASS` |
| **TC-QMV-SCR** (`14`) | `001:PASS`, `002:FAIL(BUG-QMV-010/017)`, `003:FAIL(BUG-QMV-021)`, `004:PASS*(BUG-QMV-017)`, `005:PASS`, `006:FAIL(BUG-QMV-007)`, `007:PASS`, `008:PASS`, `009:FAIL(BUG-QMV-008)`, `010:FAIL(BUG-QMV-017)`, `011:PASS*(BUG-QMV-014)`, `012:PASS`, `013:FAIL(BUG-QMV-010)`, `014:PASS` |
| **TC-QMV-BT** (`16`) | `001:PASS*(BUG-QMV-002/003)`, `002:FAIL(BUG-QMV-001/002/003)`, `003:FAIL(BUG-QMV-009/001)`, `004:PASS`, `005:PASS`, `006:PASS`, `007:PASS`, `008:PASS*(BUG-QMV-022/024)`, `009:PASS`, `010:PASS`, `011:PASS*(BUG-QMV-014)`, `012:PASS`, `013:PASS`, `014:PASS`, `015:PASS*(BUG-QMV-021)`, `016:FAIL(BUG-QMV-003)` |
| **TC-QMV-RPL** (`8`) | `001:PASS`, `002:FAIL(BUG-QMV-013)`, `003:PASS`, `004:PASS`, `005:PASS`, `006:FAIL(BUG-QMV-012)`, `007:PASS`, `008:PASS` |
| **TC-QMV-UX** (`10`) | `001:FAIL(BUG-QMV-006)`, `002:FAIL(BUG-QMV-019/016)`, `003:FAIL(BUG-QMV-014)`, `004:FAIL(BUG-QMV-022)`, `005:PASS`, `006:PASS*(BUG-QMV-020)`, `007:FAIL(BUG-QMV-010/022)`, `008:PASS`, `009:PASS`, `010:PASS*(BUG-QMV-025)` |

*(Ghi chú: `PASS*` nghĩa là tiêu chí chính của Test Case đạt nhưng có kèm điểm gợn phụ đã tách thành Bug riêng trong ngoặc.)*

---

## 2. Chi Tiết Toàn Bộ Lỗi Đã Xác Minh (`P0 → P4`)

### Mức Độ P0 — Critical / Blocker (Sai Lệch Toán Học Định Lượng / Kỷ Luật Chiến Lược QMV)

#### `BUG-QMV-001` — Công Tắc `"Cổng VNINDEX"` Vô Hiệu Trên `QMV S1..S3`, Bị Khóa Cứng Không Tắt Được Trên `S4..S5`, Và Bị Tắt Trái Quy Tắc Trên Playbook `SMA30/EMA50` & `BB(30,1.8)`
- **Test Case & Phân hệ:** `TC-QMV-BT-002`, `TC-QMV-BT-003` · Strategy Tester (`sumi-core` + UI) | **Mức độ:** **P0 (Critical)**
- **Các bước tái hiện:**
  1. Mở `HOSE:SSI` khung `1D`, mở tab `Kiểm định Chiến lược QMV`, chọn `QMV S1` (hoặc `S2`, `S3`).
  2. Chạy `Backtest` khi **TẮT** checkbox `Cổng VNINDEX`, ghi lại số lệnh và lợi nhuận; sau đó **BẬT** `Cổng VNINDEX` và chạy lại.
  3. Chọn `QMV S4` hoặc `QMV S5`, **TẮT** `Cổng VNINDEX` và chạy `Backtest`.
  4. Nhấp nút Playbook `Sóng Dài CK/Thép/KCN (SMA30/EMA50)` hoặc `Ngân Hàng Nén (BB 30, 1.8)` rồi bấm `Backtest`.
- **Thực tế vs Kỳ vọng:**
  - **Thực tế:**
    - Trên `QMV S1` (`HOSE:SSI 1D`): `Cổng VNINDEX = OFF` → `28` lệnh, `-6.93%`; `Cổng VNINDEX = ON` → **trùng khít `28` lệnh, `-6.93%`** (công tắc hoàn toàn không có tác dụng trên `S1`, `S2`, `S3`!).
    - Trên `QMV S4` và `QMV S5`: luôn gọi `qmv_vnindex_gate_ok` vô điều kiện kể cả khi người dùng tắt `Cổng VNINDEX` → **không thể tắt được cổng**.
    - Trên Playbook 5 (`BB 30, 1.8`) và Playbook 6 (`SMA30/EMA50`): payload gửi xuống Rust luôn ép `qmvUseVnindexGate: false` dù báo cáo WFO Giai đoạn 4 khuyến nghị *"Bắt buộc bật Cổng VNINDEX"* cho ngành Chứng khoán!
  - **Kỳ vọng:** Cả 6 chiến lược `QMV S1..S6` (và tùy chọn cho chiến lược Tham chiếu `sma_ema`/`bb_squeeze`) phải tôn trọng tuyệt đối trạng thái ON/OFF của `qmvUseVnindexGate`.
- **Bằng chứng:** Log lõi Rust thật `rust_selftest_results.json` (`s1_gate_off_vs_on: offTrades=28, onTrades=28, offRet=-6.9295424, onRet=-6.9295424`) và `e2e_suite2_results.json` (`pbSnapshots[4,5].qmvUseVnindexGate: false`).
- **Nguyên nhân gốc (`Root Cause`):**
  - [`crates/sumi-core/src/backtest/engine.rs:2920-3049`](file:///Users/mizuhara/workspace/sumi/crates/sumi-core/src/backtest/engine.rs#L2920-L3049): `QmvS1`, `QmvS2`, `QmvS3` không hề kiểm tra `p.qmv_use_vnindex_gate` / `qmv_vnindex_gate_ok`.
  - [`crates/sumi-core/src/backtest/engine.rs:3054, 3093`](file:///Users/mizuhara/workspace/sumi/crates/sumi-core/src/backtest/engine.rs#L3054): `QmvS4` & `QmvS5` gọi `if !qmv_vnindex_gate_ok(...)` mà bỏ quên điều kiện `if p.qmv_use_vnindex_gate`.
  - [`ui/src/components/dock/StrategyTesterTab.tsx:316`](file:///Users/mizuhara/workspace/sumi/ui/src/components/dock/StrategyTesterTab.tsx#L316): `qmvUseVnindexGate: isQmvPreset ? qmvUseVnindexGate : false` tắt cổng đối với `sma_ema` / `bb_squeeze`.
- **Khuyến nghị sửa:** Thêm chốt `if p.qmv_use_vnindex_gate && !qmv_vnindex_gate_ok(candles, i, config) { return ... }` thống nhất cho toàn bộ `QmvS1..S6` (và hỗ trợ bật `Cổng VNINDEX` cho `sma_ema`/`bb_squeeze` khi kiểm định cổ phiếu VN).

---

#### `BUG-QMV-002` — Lệch 100% Mã Hóa Tham Số `qmvDivOsc` (Và Sai Ngữ Nghĩa `qmvConfirmMode`, `qmvTrailMode`) Trên Chiến Lược `QMV S2` Giữa UI Và Lõi Rust
- **Test Case & Phân hệ:** `TC-QMV-BT-001`, `TC-QMV-BT-002`, `TC-QMV-BT-003` · Strategy Tester (`QMV S2` & Exit Trailing) | **Mức độ:** **P0 (Critical)**
- **Các bước tái hiện:**
  1. Chọn chiến lược `QMV S2 — Bắt Đáy An Toàn & Phân Kỳ Đáy (DT1/DT2)`.
  2. Lần lượt chọn 4 giá trị trong dropdown `Phân kỳ:` (`Hợp lưu RSI/MACD/CCI`, `RSI(14)`, `MACD(12,26,9)`, `CCI(14)`), dropdown `Xác nhận:` (`Vượt High nến đảo chiều` vs `Lấy lại đường MA20`), và dropdown `Dời thoát:` (`Đảo chiều Dòng tiền 70–80`).
- **Thực tế vs Kỳ vọng:**
  - **Thực tế (lệch ánh xạ hoàn toàn cả 4/4 giá trị `qmvDivOsc`):**
    | Lựa chọn trên UI (`StrategyTesterTab.tsx:760-764`) | Mã gửi xuống | Thực tế lõi Rust chạy (`engine.rs:2965-2970`) |
    |---|---|---|
    | `Hợp lưu RSI / MACD / CCI` | `3` | **Chỉ chạy `CCI(14)`!** |
    | `RSI(14)` | `0` | **Chạy `Hợp lưu cả 3 (RSI \|\| MACD \|\| CCI)`!** |
    | `MACD(12,26,9)` | `1` | **Chạy `RSI(14)`!** |
    | `CCI(14)` | `2` | **Chạy `MACD(12,26,9)`!** |
  - Tương tự, `qmvConfirmMode = 1` trên UI ghi **`"Lấy lại đường MA20"`** nhưng Rust (`engine.rs:2974-2978`) chạy `pattern.any_bullish || vsa.spring` (không hề dùng MA20); và `qmvTrailMode = 2` (**`"Đảo chiều Dòng tiền 70–80"`**) hoàn toàn không được xử lý ở bất kỳ chiến lược nào trong `engine.rs`!
  - **Kỳ vọng:** Mã hóa số nguyên và logic thực thi ở Rust khớp 1-1 với nhãn hiển thị cho người dùng trên UI.
- **Bằng chứng:** Đối chiếu trực tiếp mã nguồn + payload `e2e_suite2_results.json` (Playbook 2 `"Core10 Bắt Đáy (S2 · CCI14)"` phải set `qmvDivOsc = '3'` — vốn bị hiển thị trên dropdown UI thành `"Hợp lưu RSI / MACD / CCI"` thay vì `"CCI(14)"`!).
- **Nguyên nhân gốc (`Root Cause`):**
  - [`ui/src/components/dock/StrategyTesterTab.tsx:760-776, 855-858`](file:///Users/mizuhara/workspace/sumi/ui/src/components/dock/StrategyTesterTab.tsx#L760-L776)
  - [`crates/sumi-core/src/backtest/engine.rs:2947, 2965-2978, 3042`](file:///Users/mizuhara/workspace/sumi/crates/sumi-core/src/backtest/engine.rs#L2947)
- **Khuyến nghị sửa:** Đồng bộ chuẩn hóa `0 = Hợp lưu RSI/MACD/CCI`, `1 = RSI(14)`, `2 = MACD(12,26,9)`, `3 = CCI(14)` trên cả UI lẫn Rust; đồng bộ nhãn `qmvConfirmMode` đúng với logic mẫu hình nến/MA20 và triển khai đầy đủ `qmv_trail_mode == 2` (thoát khi `qmv.flow_turn_down_70_80`).

---

#### `BUG-QMV-003` — Lõi Desktop Ép Cứng `max_holding_bars = Some(25)` Cắt Cụt `44%–54%` Lệnh Sóng QMV (`S1..S6`) & Công Tắc `"SL Đáy Hỗ Trợ"` Không Thể Tắt Được
- **Test Case & Phân hệ:** `TC-QMV-BT-002`, `TC-QMV-BT-016` · Strategy Tester Desktop Bridge | **Mức độ:** **P0 (Critical)**
- **Các bước tái hiện:**
  1. Chọn `QMV S3` (hoặc `QMV S5`) trên `HOSE:FPT 1D`, chạy `Backtest`, kiểm tra cột `Lý do thoát` (`exitReason`).
  2. Chạy Portfolio `QMV:CORE10` với `QMV S5` (`1/σ`, `Sqz ≤ 12%`, `RVol ≥ 1.6`) và đối chiếu với Báo cáo Nghiên cứu Giai đoạn 4 (`+49.7%`).
  3. Trên `HOSE:SSI 1D` (`QMV S3`), **TẮT** đồng thời cả `SL Đáy hỗ trợ` và `Giữ 2/3 sóng` rồi so sánh với lúc **BẬT**.
- **Thực tế vs Kỳ vọng:**
  - **Thực tế:**
    - `QMV S3` trên `HOSE:FPT`: **`14 / 26` lệnh (`53.8%`)** bị ép cắt tại phiên thứ 25 với lý do `"MaxBars"`! Trên `HOSE:HPG`: **`14 / 28` lệnh (`50.0%`)**! `QMV S5` trên `HOSE:FPT`: **`24 / 54` lệnh (`44.4%`)** bị cắt bởi `"MaxBars"`!
    - Khi đối chiếu Giai đoạn 4 (`TC-QMV-BT-016`): Portfolio `CORE10_S5_IV` trên app tụt xuống chỉ còn **`+15.19%`** (so với **`+49.7%`** trong báo cáo nghiên cứu Giai đoạn 4!), và `QMV30_S5_IV` chỉ còn **`+2.32%`**!
    - Khi người dùng **tắt** checkbox `"SL Đáy hỗ trợ"`, UI gửi `supportStopLookback: null`, nhưng Rust tự ghi đè ngược lại thành `Some(18)` → **người dùng không bao giờ tắt được `SL Đáy hỗ trợ`** (`s3_wave_off_vs_on` trên `HOSE:SSI`: tắt và bật ra y hệt nhau `23` lệnh, `-1.07871%`!).
  - **Kỳ vọng:** `QMV S1, S3, S4, S5, S6` để `max_holding_bars = None` đúng như `sumi-core` WFO & nghiên cứu Giai đoạn 4 (chỉ `QMV S2` bắt đáy ngắn mới dùng `Some(28)`), để quy tắc `Giữ 2/3 sóng` nuôi trọn sóng; và khi `supportStopLookback: null` (hoặc `Some(0)`), phải tôn trọng tắt hẳn `SL Đáy hỗ trợ`.
- **Bằng chứng:** Log `rust_selftest_results.json` (`s1_s6` exitReasons, `s3_wave_off_vs_on`, `portfolios.CORE10_S5_IV = +15.19%` vs báo cáo `+49.7%`).
- **Nguyên nhân gốc (`Root Cause`):**
  - [`crates/sumi-desktop/src/commands/backtest.rs:437`](file:///Users/mizuhara/workspace/sumi/crates/sumi-desktop/src/commands/backtest.rs#L437): `support_stop_lookback: input.support_stop_lookback.or(default_sup_lb)` biến `None` (người dùng chủ động tắt) thành `Some(18)`.
  - [`crates/sumi-desktop/src/commands/backtest.rs:439`](file:///Users/mizuhara/workspace/sumi/crates/sumi-desktop/src/commands/backtest.rs#L439): `max_holding_bars: is_qmv_preset.then_some(25)` ép cứng trần 25 phiên cho mọi `QMV S1..S6` trong khi [`crates/sumi-core/src/backtest/strategy.rs:1588, 1623, 1641, 1657, 1682`](file:///Users/mizuhara/workspace/sumi/crates/sumi-core/src/backtest/strategy.rs#L1588) đặt `None` cho `S1, S3, S4, S5, S6` và chỉ `Some(28)` cho `S2`.
- **Khuyến nghị sửa:** Dùng `Option<Option<usize>>` hoặc quy ước `Some(0) => None` cho `support_stop_lookback` để phân biệt "chưa truyền" với "chủ động tắt"; và đặt `max_holding_bars: (preset == StrategyPresetKind::QmvSafeBottomDiv).then_some(28)` (hoặc cho phép người dùng nhập tùy chọn).

---

### Mức Độ P1 — High (Chức Năng Sai Lệch Nặng / Mất Điều Khiển / Ảnh Hưởng Nặng Trải Nghiệm Chính)

#### `BUG-QMV-004` — Bẫy Cold-Start Race Trên `RS_VNINDEX`: Kẹt Vĩnh Viễn Ở Đường Fallback ROC Giả (`83.66` vs Thật `33.46`, Lệch `+150%`)
- **Test Case & Phân hệ:** `TC-QMV-IND-005` · Custom Indicators (`RS_VNINDEX`) | **Mức độ:** **P1 (High)**
- **Các bước tái hiện:**
  1. Bật mẫu `3. QMV Chu Kỳ Dòng Tiền` (chứa `RS_VNINDEX [20]`) trên `HOSE:FPT 1D`.
  2. Tải lại ứng dụng hoàn toàn (Cold Start) trong điều kiện `get_bars("HOSE:VNINDEX", limit=2500)` hoàn tất chậm hơn `get_bars("HOSE:FPT", limit=600)` (thực nghiệm với độ trễ `300ms`).
  3. Đọc giá trị `RS` và `MA` trên `HOSE:FPT` sau `1.6s` (khi cả 2 request đã xong hoàn toàn), sau đó đổi sang `HOSE:SSI` rồi quay lại `HOSE:FPT` để đối chiếu.
- **Thực tế vs Kỳ vọng:**
  - **Thực tế:** Ngay sau Cold Start, `HOSE:FPT` tải xong trước (`24.8ms`), `HOSE:VNINDEX` tải xong sau (`312.5ms`). Dù đã chờ thêm `> 1.2s`, biểu đồ `RS vs VNINDEX(20)` trên `HOSE:FPT` vẫn **kẹt vĩnh viễn ở đường ROC fallback `RS = 83.66, MA = 85.44`** thay vì giá trị thật sau khi đổi mã quay lại **`RS = 33.46, MA = 35.18`** (**sai lệch `+50.20` điểm / `+150%`** và sai hoàn toàn hình dạng xu hướng!).
  - **Kỳ vọng:** Ngay khi `setVnindexBenchmarkBars(benchPage.bars)` hoàn tất, mọi biểu đồ đang bật `RS_VNINDEX` phải tự động tính lại chỉ báo (`overrideIndicator` / `resetData`) tức thì.
- **Bằng chứng:** Log `e2e_suite1_results.json` (`TC-QMV-IND-005`) và đối chiếu 2 ảnh trực tiếp:  
  - Trước đổi mã (kẹt `83.66`): [`05a_rs_vnindex_coldstart_race.png`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence_qmv/05a_rs_vnindex_coldstart_race.png)  
  - Sau đổi mã quay lại (thật `33.46`): [`05b_rs_vnindex_after_switch.png`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence_qmv/05b_rs_vnindex_after_switch.png)
- **Nguyên nhân gốc (`Root Cause`):** [`ui/src/chart/KLineViewport.tsx:624-633`](file:///Users/mizuhara/workspace/sumi/ui/src/chart/KLineViewport.tsx#L624-L633) chỉ gọi `setVnindexBenchmarkBars(benchPage.bars)` mà không thông báo/gọi `chartRef.current.overrideIndicator({ name: 'RS_VNINDEX', ... })` (hoặc pub-sub callback tới tất cả `KLineViewport` đang mount) để ép KLineChart tính lại mảng `result`.
- **Khuyến nghị sửa:** Phát sự kiện `vnindexBenchmarkReady` (hoặc gọi `overrideIndicator` trên các chart đang có `RS_VNINDEX`) ngay sau `setVnindexBenchmarkBars(...)`.

---

#### `BUG-QMV-005` — 6 Cổ Phiếu Siêu Penny (`300đ–400đ`) Bị Nhân Nhầm `×1,000` Thành Nhóm `"Thị Giá Cao (≥ 50k)"`
- **Test Case & Phân hệ:** `TC-QMV-BOOT-006` · QMV Universe Taxonomy (`qmv_universe.rs`) | **Mức độ:** **P1 (High)**
- **Các bước tái hiện:**
  1. Mở `Bộ lọc Kỹ thuật QMV`, bỏ tick `KL20 ≥ 50k`, chọn phân tầng thị giá **`Thị giá cao (≥ 50k)`** (`OVER_50K`).
  2. Sắp xếp cột `Giá đóng` tăng dần.
- **Thực tế vs Kỳ vọng:**
  - **Thực tế:** Trong `43` mã trả về của `OVER_50K` có tới **6 cổ phiếu siêu trà đá giá `300đ–400đ`**: `HNX:BII` (`300đ`), `UPCOM:FTM` (`400đ`), `UPCOM:ACM` (`400đ`), `UPCOM:HKB` (`400đ`), `UPCOM:DFF` (`400đ`), `UPCOM:PXI` (`400đ`)!
  - **Kỳ vọng:** 100% cổ phiếu giá `< 10,000 VND` (kể cả `300đ–400đ`) phải nằm trong `UNDER_10K` (`Trà đá`), tuyệt đối không lọt vào `OVER_50K`.
- **Bằng chứng:** Log lõi Rust thật `rust_selftest_results.json` (`TC-QMV-BOOT-006.OVER_50K.violations`: 6 mã).
- **Nguyên nhân gốc (`Root Cause`):** [`crates/sumi-core/src/scanner/qmv_universe.rs:212-216`](file:///Users/mizuhara/workspace/sumi/crates/sumi-core/src/scanner/qmv_universe.rs#L212-L216) áp dụng heuristic `let vnd = if close > 0.0 && close < 500.0 { close * 1_000.0 } else { close }` để tự động đoán đơn vị nghìn đồng, khiến mọi mã VN có thị giá `< 500 VND` (đã ở đơn vị VND nguyên trong `sumi_market.db`) bị nhân vọt lên `300,000–400,000 VND`!
- **Khuyến nghị sửa:** Loại bỏ heuristic `< 500.0 * 1000` đối với giá đã chuẩn hóa VND nguyên trong `sumi-core`/`sumi-data` (hoặc dùng ngưỡng nhận diện rõ ràng/tham số đơn vị tường minh thay vì đoán theo `< 500.0`).

---

#### `BUG-QMV-006` — `BottomTradingDock` Mặc Định (`276px`) Bị Thanh Công Cụ Nuốt `78.2%` Chiều Cao Trong `StrategyTesterTab` (Còn `54px`, `0px` Đồ Thị Ở `1280×800`) Và Ép `StockScreenerTab` Còn `109px` (`3` Dòng)
- **Test Case & Phân hệ:** `TC-QMV-UX-001` · BottomTradingDock Layout / UX | **Mức độ:** **P1 (High)**
- **Các bước tái hiện:**
  1. Mở ứng dụng ở `1440×900` và `1280×800` với `BottomTradingDock` ở chiều cao mặc định (`276px`, chưa bấm Phóng to).
  2. Mở tab `Kiểm định Chiến lược QMV` → bấm `Backtest` (`QMV S1`), đo `getBoundingClientRect()` từng khối.
  3. Mở tab `Bộ lọc Kỹ thuật QMV`, đo chiều cao vùng bảng cuộn và đếm số dòng cổ phiếu nhìn thấy.
- **Thực tế vs Kỳ vọng:**
  - **Thực tế (đo pixel chính xác bằng Playwright):**
    - **`StrategyTesterTab` (`1440×900` & `1280×800`):** Tổng Dock `276px` (`panelHeight = 243px`), nhưng 4 hàng điều khiển phía trên ngốn tới **`190px` (`78.2%`)** (`Row 1: 61px` + `Row 1b Playbooks: 51px` + `Row 2 Kỷ luật QMV: 27px` + `Sub-tab & KPI: 51px`). Vùng nội dung chính `equityContainerHeight` bên dưới **chỉ còn đúng `54px`** (trong khi biểu đồ SVG Equity cao `146px`!). Trên ảnh `10a` chỉ lòi đúng 1 vệt nhỏ tiêu đề + 20px đỉnh biểu đồ; trên `10b` (`1280×800`) **đồ thị Equity biến mất 100% (`0px` nhìn thấy)**!
    - **`StockScreenerTab`:** Tại `1440×900`, 3 hàng lọc chiếm `115px`, bảng còn `128px` (`4` dòng). Tại `1280×800`, Row 2 bị wrap xuống 2 dòng (`46px`), cụm lọc chiếm **`134px`**, vùng bảng cuộn tụt xuống **`109px` (`< 110px`, chỉ thấy đúng `3` dòng)** và tràn ngang `1240px > 946px` khuất hoàn toàn 2 cột cuối!
  - **Kỳ vọng:** Ở chiều cao Dock mặc định (`~276px`), vùng bảng Screener và vùng đồ thị Equity / danh sách lệnh Backtest phải còn tối thiểu **`110–130px`** hiển thị rõ ràng mà không bắt người dùng lúc nào cũng phải bấm Phóng to (`62vh`).
- **Bằng chứng:** Log đo DOM `e2e_suite2_results.json` (`dockBtDefault1440`, `dockBt1280`, `dockScr1440`, `dockScr1280`) và 4 ảnh chụp thực tế:  
  - [`10a_dock_default_strategy_1440.png`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence_qmv/10a_dock_default_strategy_1440.png) · [`10b_dock_default_strategy_1280.png`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence_qmv/10b_dock_default_strategy_1280.png)  
  - [`09a_dock_default_screener_1440.png`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence_qmv/09a_dock_default_screener_1440.png) · [`09b_dock_default_screener_1280.png`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence_qmv/09b_dock_default_screener_1280.png)
- **Nguyên nhân gốc (`Root Cause`):** [`ui/src/components/dock/StrategyTesterTab.tsx:414-1183`](file:///Users/mizuhara/workspace/sumi/ui/src/components/dock/StrategyTesterTab.tsx#L414-L1183) (4 dải `flex-wrap` không cuộn/không thu gọn xếp chồng dọc); [`ui/src/components/dock/StockScreenerTab.tsx:322-632`](file:///Users/mizuhara/workspace/sumi/ui/src/components/dock/StockScreenerTab.tsx#L322-L632) (3 dải `flex-wrap` + `min-w-[940px]` mở rộng thành `1240px`).
- **Khuyến nghị sửa:** Gom 6 nút `"Playbook WFO Tối ưu"` thành 1 `<select>` dropdown hoặc popover 1 dòng cùng hàng với selector chiến lược; chuyển cụm `Kỷ luật QMV` & tham số chi tiết thành popover `"⚙ Cấu hình"` (có huy hiệu tóm tắt ON/OFF gọn trên thanh 1 dòng) hoặc nâng `bottomDockHeight` mặc định lên `320px` + hỗ trợ kéo thả thanh chia chiều cao Dock!

---

#### `BUG-QMV-007` — Nút Lọc Nhanh #5 `"Vượt Đỉnh 50p + Vol"` Dùng Signal ID Không Tồn Tại (`volume.rvol_spike_1_5x`) → Bỏ Qua Hoàn Toàn Điều Kiện Khối Lượng & Lộ Mã Thô
- **Test Case & Phân hệ:** `TC-QMV-SCR-006` · Stock Screener Quick Presets | **Mức độ:** **P1 (High)**
- **Các bước tái hiện:**
  1. Mở `Bộ lọc Kỹ thuật QMV`.
  2. Nhấp nút lọc nhanh #5: **`+ Vượt Đỉnh 50p + Vol`**.
  3. Quan sát 2 chip điều kiện sinh ra và giá trị cột `RVol(20)` của các cổ phiếu trả về.
- **Thực tế vs Kỳ vọng:**
  - **Thực tế:** Chip thứ 2 hiển thị mã thô tiếng Anh **`volume.rvol_spike_1_5x`** (vì ID này không tồn tại trong `SIGNAL_CATALOG`, chỉ có `volume.spike_2x`, `volume.relative_volume`, `volume.spike`!). Ở backend `scanner.rs`, `catalog_entry_index("volume.rvol_spike_1_5x")` trả `None` và bị **âm thầm bỏ qua** — khiến bộ lọc trả về y hệt chỉ lọc `qmv.new_high_50` (cho lọt cả `HOSE:DPG` có `RVol = 1.44x < 1.5x`!).
  - **Kỳ vọng:** Nút lọc nhanh phải gắn đúng signal ID có thật (`volume.spike_2x` hoặc đặt kèm `minRvol = 1.5`), hiện nhãn tiếng Việt chuẩn và lọc chính xác điều kiện khối lượng.
- **Bằng chứng:** Log `e2e_suite2_results.json` (`quickPresetResults["Vượt Đỉnh 50p + Vol"].chipTexts: ["Đỉnh Mới 50 Phiên (Chuẩn QMV V7)", "volume.rvol_spike_1_5x"]`) và output live `get_signal_catalog`.
- **Nguyên nhân gốc (`Root Cause`):** [`ui/src/components/dock/StockScreenerTab.tsx:61`](file:///Users/mizuhara/workspace/sumi/ui/src/components/dock/StockScreenerTab.tsx#L61) hardcode `'volume.rvol_spike_1_5x'`; [`crates/sumi-desktop/src/commands/scanner.rs:328-334`](file:///Users/mizuhara/workspace/sumi/crates/sumi-desktop/src/commands/scanner.rs#L328-L334) âm thầm bỏ qua ID không hợp lệ thay vì báo lỗi.
- **Khuyến nghị sửa:** Đổi preset #5 thành `signals: ['qmv.new_high_50']` kèm tự điền `minRvol = '1.5'` (hoặc dùng `volume.spike_2x` đúng tên trong `SIGNAL_CATALOG` và thêm unit test kiểm tra mọi preset ID phải tồn tại trong `SIGNAL_CATALOG`).

---

#### `BUG-QMV-008` — Mất Hoàn Toàn 3 Bộ Lọc Số (`% Đổi ≥`, `MACD >0/<0`, `Điểm SK ≥`) Khỏi Giao Diện `StockScreenerTab`
- **Test Case & Phân hệ:** `TC-QMV-SCR-009` · Stock Screener Numeric Filters | **Mức độ:** **P1 (High)**
- **Các bước tái hiện:**
  1. Mở tab `Bộ lọc Kỹ thuật QMV`.
  2. Tìm các ô nhập liệu lọc theo `% Đổi ≥`, `MACD > 0 / < 0`, và `Điểm SK ≥` theo đặc tả `v2.2.0` (`TC-QMV-SCR-009`).
- **Thực tế vs Kỳ vọng:**
  - **Thực tế:** Trên giao diện chỉ còn `RSI Từ–Đến` và `RVol ≥`. Toàn bộ 3 control cho `% Đổi ≥`, `MACD >0/<0`, và `Điểm SK ≥` **không hề tồn tại trên DOM** (`hasMinChangeInput: 0, hasMacdSelect: 0, hasHealthInput: 0`), mặc dù trong code `StockScreenerTab.tsx` vẫn giữ nguyên 3 state chết (`minChangePct`, `macdFilter`, `minHealthScore`) và backend `scanner.rs` vẫn hỗ trợ đầy đủ!
  - **Kỳ vọng:** Người dùng có thể lọc đầy đủ theo `% Đổi ≥`, `MACD >0/<0`, và `Điểm SK ≥` (có thể đặt gọn trong popover `"Lọc số nâng cao"` để không chiếm thêm chiều cao Dock).
- **Bằng chứng:** Log DOM `e2e_suite2_results.json` (`missingNumericFiltersCheck: { hasMinChangeInput: 0, hasMacdSelect: 0, hasHealthInput: 0 }`) và ảnh [`09a_dock_default_screener_1440.png`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence_qmv/09a_dock_default_screener_1440.png).
- **Nguyên nhân gốc (`Root Cause`):** [`ui/src/components/dock/StockScreenerTab.tsx:92-94, 187-189, 515-568`](file:///Users/mizuhara/workspace/sumi/ui/src/components/dock/StockScreenerTab.tsx#L92-L94) giữ state và truyền xuống IPC nhưng đã vô tình lược bỏ phần JSX render khi chuyển sang bố cục QMV.
- **Khuyến nghị sửa:** Khôi phục 3 trường lọc `% Đổi ≥`, `MACD (>0 / <0)`, `Điểm SK ≥` trong một popover `"Bộ lọc số nâng cao"` gọn gàng ngay trên thanh công cụ Screener.

---

#### `BUG-QMV-009` — Bẫy Sót Tham Số (`State Leakage`) Khi Chuyển Qua Lại Giữa 6 Nút `"Playbook WFO Tối ưu"` & Thiếu Phản Hồi / Active State
- **Test Case & Phân hệ:** `TC-QMV-BT-003` · Strategy Tester Playbooks | **Mức độ:** **P1 (High)**
- **Các bước tái hiện:**
  1. Mở tab `Kiểm định Chiến lược QMV`.
  2. Bấm lần lượt Playbook 1 → Playbook 2 → Playbook 3 → Playbook 4 → Playbook 5 → Playbook 6 → bấm lại Playbook 1, ghi lại toàn bộ `StrategyBacktestInput` gửi xuống Rust sau mỗi bước.
- **Thực tế vs Kỳ vọng:**
  - **Thực tế:**
    - So sánh Playbook 1 lần đầu vs lần lặp lại bị rò rỉ **4 tham số** (`pb1DiffKeys`): `fastSma` (`20 → 30`), `bbPeriod` (`20 → 30`), `bbStdMult` (`2 → 1.8`), `qmvConfirmMode` (`0 → 1`).
    - Trên **Playbook 4 (`Công Nghệ & Bán Lẻ S6`)**: bị kẹt `qmvRvolConfirm = 1.6` và `maxSqueezeBwPct = 12` rò từ Playbook 3 ngay trước đó!
    - Ngoài ra, cả 6 nút Playbook đều chỉ âm thầm đổi state form (`autoRanOnPlaybookClick: false`), **không có nút nào sáng trạng thái active, không có toast thông báo đã nạp cấu hình nào, và không tự chạy kiểm định** — dễ khiến người dùng bấm vào tưởng nút bị liệt!
  - **Kỳ vọng:** Mỗi nút Playbook phải reset toàn bộ tham số về một hàm `resetAllStrategyParams()` trước khi áp bộ cấu hình riêng của mình (tái lập 100% ở mọi thứ tự bấm), hiển thị trạng thái đang chọn + toast rõ ràng (hoặc tự chạy kiểm định theo cấu hình).
- **Bằng chứng:** Log `e2e_suite2_results.json` (`PHAN_HE_6_STRATEGY.pbSnapshots` & `pb1DiffKeys`).
- **Nguyên nhân gốc (`Root Cause`):** [`ui/src/components/dock/StrategyTesterTab.tsx:575-691`](file:///Users/mizuhara/workspace/sumi/ui/src/components/dock/StrategyTesterTab.tsx#L575-L691) mỗi `onClick` chỉ set vài state cục bộ, không reset các trường còn lại và không phát `pushToast` / set `activePlaybookId`.
- **Khuyến nghị sửa:** Tạo helper `applyPlaybookPreset(id)` luôn gọi `resetAllParamsToDefault()` trước khi ghi đè các trường của Playbook, đánh dấu nút đang chọn và hiện toast tóm tắt (ví dụ: `"Đã nạp Playbook QMV30 VaR Thấp — nhấn Backtest hoặc Phân nhóm QMV để chạy"`).

---

#### `BUG-QMV-010` — Watchlist `Chỉ Số VN` (`wl_vn_indices`): `HNX:VN30F1!` Luôn Trống Giá (`— —`) Do Lệch Khóa `HNX:VN301!`, Còn `HOSE:VNXALL` Đóng Băng Từ `22/01/2019` (Giá `1,269,690`) Và Lọt Thẳng Vào Stock Screener `ALL_VN` + CSV
- **Test Case & Phân hệ:** `TC-QMV-BOOT-004`, `TC-QMV-BOOT-005`, `TC-QMV-SCR-002`, `TC-QMV-SCR-013`, `TC-QMV-UX-007` · Market Data, Watchlist & Screener | **Mức độ:** **P1 (High)**
- **Các bước tái hiện:**
  1. Ở thanh Watchlist bên phải, chọn danh sách **`Chỉ Số & Rào Chắn Thị Trường VN (5)`** (`wl_vn_indices`).
  2. Mở `Bộ lọc Kỹ thuật QMV`, chọn `Toàn thị trường VN`, sắp xếp `Giá đóng` giảm dần và bấm xuất `CSV`.
- **Thực tế vs Kỳ vọng:**
  - **Thực tế:**
    1. Trên Watchlist `wl_vn_indices`, dòng **`VN30F1! HNX`** luôn hiển thị trống **`—   —`** dù `sumi_market.db` có đủ 300 nến tới `20/11/2026` (`lastPrice = 1,780.2`)! Lý do: `get_watchlist_snapshots(["HNX:VN30F1!"])` trả về đối tượng có `ticker: "HNX:VN301!"` (mất chữ `F`) nên Frontend tra khóa `"HNX:VN30F1!"` bị miss 100%!
    2. Dòng **`VNXALL HOSE`** hiển thị giá cũ đóng băng từ **`22/01/2019`** (`timestampMs = 1548115200000`, gần 8 năm trước) và bị lưu nhân `1,000×` thành **`1,269,690`** (thay vì `1,269.69` điểm)!
    3. Nghiêm trọng hơn: **`HOSE:VNXALL` bị lọt vào kết quả lọc cổ phiếu `ALL_VN` (`467` mã)** với `qmvSector = INDEX`, `moneyTier = OTHER`, đứng đầu danh sách sắp xếp giá cao nhất (`1,269,690`) và xuất luôn làm dòng đầu tiên trong file CSV cổ phiếu!
  - **Kỳ vọng:** `HNX:VN30F1!` hiển thị đúng giá `1,780.2` (`-0.43%`) trên Watchlist; loại bỏ chỉ số chết `HOSE:VNXALL` khỏi `wl_vn_indices` (thay bằng `UPCOM:UPCOMINDEX` hoặc chỉ giữ 4 chỉ số sống) và lọc chặt mọi `QmvSector::Index` / `VNXALL` không bao giờ lọt vào Stock Screener cổ phiếu.
- **Bằng chứng:** Ảnh [`13_singlepane_light_ichimoku_ssi.png`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence_qmv/13_singlepane_light_ichimoku_ssi.png) (xem góc phải Watchlist: `VN30F1! — —` và `VNXALL 1,269,690`), log `rust_selftest_results.json` (`leakedIndices: ["HOSE:VNXALL"]`), và `e2e_suite2_results.json` (`csvSample.firstDataLine: "VNXALL,HOSE,Chỉ số & Phái sinh,OTHER,1269690..."`).
- **Nguyên nhân gốc (`Root Cause`):**
  - [`crates/sumi-desktop/src/commands/market.rs:1054`](file:///Users/mizuhara/workspace/sumi/crates/sumi-desktop/src/commands/market.rs#L1054): dùng `ticker: sym_info.ticker.clone()` (đang lưu `HNX:VN301!` trong bảng `symbols`) thay vì `key.ticker()` (`HNX:VN30F1!`).
  - [`crates/sumi-desktop/src/workspace_db.rs:321`](file:///Users/mizuhara/workspace/sumi/crates/sumi-desktop/src/workspace_db.rs#L321): seed `"HOSE:VNXALL"` đã ngừng cập nhật từ 2019 vào `wl_vn_indices`.
  - [`crates/sumi-desktop/src/commands/scanner.rs:231`](file:///Users/mizuhara/workspace/sumi/crates/sumi-desktop/src/commands/scanner.rs#L231): chỉ lọc `h.market_type == "index"` nhưng bảng `symbols` lưu `VNXALL` với `market_type = 'stock'`; thiếu chốt lọc `prof.sector == QmvSector::Index`.
- **Khuyến nghị sửa:** Trả `ticker: key.ticker()` trong `get_watchlist_snapshots`, lọc loại `QmvSector::Index` & `QmvMoneyTier::IndexOrOther` trong `run_stock_screener`, và gỡ `HOSE:VNXALL` khỏi `wl_vn_indices`.

---

#### `BUG-QMV-011` — `VRVP` Phớt Lờ Hoàn Toàn Đổi Màu/Độ Dày Đường `POC`, Và `addIndicatorToActivePane` / `IndicatorSettingsModal` Ghi Đè Phá Hỏng Bảng Màu Chuẩn QMV Của `ICHIMOKU` & `QMV_FLOW`
- **Test Case & Phân hệ:** `TC-QMV-CAT-005`, `TC-QMV-CAT-006` · Indicators & Settings Modal | **Mức độ:** **P1 (High)**
- **Các bước tái hiện:**
  1. Mở `IndicatorsModal`, thêm mới `ICHIMOKU` và `VRVP` lên biểu đồ (không dùng Mẫu chỉ báo).
  2. Quan sát màu đường `Span A`, `Span B`, `Kijun` của `ICHIMOKU` so với màu đám mây xanh/đỏ bên trong.
  3. Mở `IndicatorSettingsModal` (⚙) cho `VRVP`, đổi màu `Đường POC` sang tím `#FF00FF`, độ dày `3px` → bấm `Áp dụng` và quan sát đường `POC` trên Canvas.
  4. Mở `IndicatorSettingsModal` cho `ICHIMOKU`, bấm **`Mặc định`** → bấm `Áp dụng` và quan sát nét vẽ của `Chikou Span`.
- **Thực tế vs Kỳ vọng:**
  - **Thực tế:**
    - **`VRVP`:** Dù `styles.lines[0]` lưu `#ff00ff` / `3px`, đường `POC` trên Canvas **vẫn vẽ cứng màu vàng mờ `rgba(251, 192, 45, 0.85)`, độ dày `1.25px`** vì `customIndicators.ts` hardcode cứng không gọi `userLineStyle`!
    - **`ICHIMOKU` / `QMV_FLOW` khi thêm từ `IndicatorsModal`:** `addIndicatorToActivePane` tự ý gắn `INDICATOR_COLOR_PALETTE` vào `styles.lines`, **làm hỏng hoàn toàn màu đặc trưng QMV**: `Span A` (đáng lẽ xanh `#089981`) bị đổi thành **tím `#E040FB`**, `Span B` (đáng lẽ đỏ `#F23645`) thành **xanh ngọc `#00BCD4`**, `Kijun` thành cam `#FF9800` — lạc quẻ hoàn toàn với đám mây Kumo xanh/đỏ bên trong!
    - **Trong `IndicatorSettingsModal`:** Nút `Áp dụng` ép cứng `style: 'solid'` cho mọi đường → làm mất nét đứt `dashed` của `Chikou Span`, `MA RS`, và `VWAP ±1σ/±2σ`; nút `"Mặc định"` điền bảng màu chung `INDICATOR_COLOR_PALETTE` thay vì màu mặc định đặc thù của từng chỉ báo!
  - **Kỳ vọng:** `VRVP` tôn trọng màu/nét `userLineStyle(indicator, 0, ...)`; chỉ báo mới thêm từ `IndicatorsModal` giữ `styles: null` đối với Custom Indicators có màu ngữ nghĩa riêng (`ICHIMOKU`, `QMV_FLOW`, `RS_VNINDEX`, `VRVP`, `MACD`, `RSI`, `CCI`, `VWAP`) và nút `"Mặc định"` khôi phục đúng màu/nét đứt gốc của từng chỉ báo.
- **Bằng chứng:** Ảnh [`12a_light_mode_chart_ichi_vrvp_flow.png`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence_qmv/12a_light_mode_chart_ichi_vrvp_flow.png) (xem ô trái: `SpanA: 49,950` màu tím `#E040FB`, `SpanB: 50,800` màu xanh ngọc `#00BCD4` lệch hẳn với mây hồng bên trong!) và log `e2e_suite1_results.json` (`TC-QMV-CAT-004`, `TC-QMV-CAT-005_006`).
- **Nguyên nhân gốc (`Root Cause`):**
  - [`ui/src/chart/indicators/customIndicators.ts:654-655`](file:///Users/mizuhara/workspace/sumi/ui/src/chart/indicators/customIndicators.ts#L654-L655): `ctx.strokeStyle = 'rgba(251, 192, 45, 0.85)'; ctx.lineWidth = 1.25;`.
  - [`ui/src/store/workspaceStore.ts:690-705`](file:///Users/mizuhara/workspace/sumi/ui/src/store/workspaceStore.ts#L690-L705): tự gán `INDICATOR_COLOR_PALETTE` cho mọi chỉ báo mới kể cả `ICHIMOKU`/`QMV_FLOW`.
  - [`ui/src/components/modals/IndicatorSettingsModal.tsx:116-121, 138-139`](file:///Users/mizuhara/workspace/sumi/ui/src/components/modals/IndicatorSettingsModal.tsx#L116-L121): ép `style: 'solid'` và không lấy bảng màu mặc định theo `meta`.
- **Khuyến nghị sửa:** Đọc `userLineStyle(indicator, 0, ...)` cho đường POC của `VRVP`; thêm trường `defaultLineColors` / `defaultLineStyles` vào `INDICATOR_CATALOG`; chỉ auto-assign palette cho chỉ báo thêm trùng lần 2 (`MA`/`EMA`), để `styles: null` mặc định cho các Custom QMV Indicators.

---

#### `BUG-QMV-012` — `StockScreenerTab` Âm Thầm Quét Và Hiển Thị Dữ Liệu Tương Lai (2026) Khi Đang Bật `Bar Replay` Quá Khứ
- **Test Case & Phân hệ:** `TC-QMV-RPL-006` · Cross-Subsystem (Bar Replay × Stock Screener) | **Mức độ:** **P1 (High)**
- **Các bước tái hiện:**
  1. Bật `Bar Replay` cắt lịch sử `HOSE:FPT` về quá khứ (ví dụ `05/11/2026` hoặc năm `2024`), bật `Luyện mù (Blind Mode)`.
  2. Mở tab `Bộ lọc Kỹ thuật QMV (Screener)` ở thanh dưới.
- **Thực tế vs Kỳ vọng:**
  - **Thực tế:** `StockScreenerTab` âm thầm chạy quét và hiển thị toàn bộ giá đóng cửa, `% Đổi` và tín hiệu kỹ thuật của **phiên mới nhất cuối DB (`20–23/11/2026`)**, **không có bất kỳ dòng thông báo ngày dữ liệu đang quét hay banner cảnh báo đang trong chế độ Bar Replay** (`scrHasReplayWarning: false`), làm lộ hoàn toàn diễn biến tương lai khi trader đang luyện mù!
  - **Kỳ vọng:** Khi `replay.active === true`, Screener phải hiển thị rõ banner cảnh báo (hoặc tạm ẩn/khóa kết quả live khi `Blind Mode = ON`, hoặc hỗ trợ truyền `asOfTimestampMs = replay.cutoffTimestampMs` để quét nhân quả đúng tại mốc Replay).
- **Bằng chứng:** Log `e2e_suite2_results.json` (`PHAN_HE_7_REPLAY_BROKER.scrHasReplayWarning: false`).
- **Nguyên nhân gốc (`Root Cause`):** [`ui/src/components/dock/StockScreenerTab.tsx:65-198`](file:///Users/mizuhara/workspace/sumi/ui/src/components/dock/StockScreenerTab.tsx#L65-L198) không đọc `useWorkspaceStore((s) => s.replay)` và không hiển thị ngày dữ liệu `lastTimestampMs`.
- **Khuyến nghị sửa:** Hiển thị nhãn `"Dữ liệu phiên DD/MM/YYYY"` trên thanh trạng thái Screener; khi `replay.active && blindMode`, hiện lớp che/cảnh báo chống lộ tương lai (hoặc truyền `cutoffTimestampMs` xuống `run_stock_screener` để lọc point-in-time).

---

#### `BUG-QMV-013` — Chế Độ `Luyện Mù (Blind Mode)` Trong Bar Replay Không Ẩn Danh Tên Mã & Vẫn Lộ Giá Live 2026 Của 29 Mã Còn Lại Trong Watchlist
- **Test Case & Phân hệ:** `TC-QMV-RPL-002` · Bar Replay Blind Mode | **Mức độ:** **P1 (High)**
- **Các bước tái hiện:**
  1. Mở `HOSE:FPT 1D`, bấm `Phát lại` cắt lùi 60 nến, xác nhận `Luyện mù: BẬT`.
  2. Quan sát `TopHeader`, `ReplayControlBar`, tiêu đề các ô biểu đồ, cột `Watchlist` và thẻ `Chi tiết mã`.
- **Thực tế vs Kỳ vọng:**
  - **Thực tế:** Dù `Luyện mù: BẬT` che đúng giá `HOSE:FPT` về mốc Replay (`53,800`), **tên mã thật `HOSE:FPT` & `FPT Corporation` vẫn hiện rõ ở tất cả 5 vị trí** (TopHeader, ReplayBar, 3 tiêu đề biểu đồ, Watchlist, Chi tiết mã), đồng thời **29 mã trụ khác trên Watchlist (`BID`, `BSR`, `CTG`, `HPG`, `SSI`...) vẫn hiển thị nguyên giá live tháng 11/2026** (làm lộ bối cảnh thị trường tương lai)!
  - **Kỳ vọng:** Khi bật `Blind Mode`, ẩn danh tên mã đang luyện (vd `SYMBOL_???`) trên thanh tiêu đề & biểu đồ, đồng thời làm mờ/ẩn bảng giá live của các mã khác trên Watchlist cho tới khi thoát Replay hoặc tắt Blind Mode.
- **Bằng chứng:** Ảnh [`11_replay_qmv_ichimoku_blind.png`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence_qmv/11_replay_qmv_ichimoku_blind.png).
- **Nguyên nhân gốc (`Root Cause`):** [`ui/src/components/RightWidgetBar.tsx`](file:///Users/mizuhara/workspace/sumi/ui/src/components/RightWidgetBar.tsx), [`ui/src/components/TopHeader.tsx`](file:///Users/mizuhara/workspace/sumi/ui/src/components/TopHeader.tsx) và [`ui/src/chart/KLineViewport.tsx`](file:///Users/mizuhara/workspace/sumi/ui/src/chart/KLineViewport.tsx) chưa áp dụng che tên mã & làm mờ các mã không thuộc phiên Replay khi `replay.blindMode === true`.
- **Khuyến nghị sửa:** Khi `replay.active && replay.blindMode`, hiển thị mã ẩn danh `BLIND-01` trên tiêu đề/chart và ẩn cột `Giá khớp / % Đổi` của các mã khác trên Watchlist.

---

### Mức Độ P2 — Medium (Hiển Thị Cục Bộ / Light Mode / Khung Hẹp `1280×800` / Tìm Kiếm & Biên Tham Số)

#### `BUG-QMV-014` — Tương Phản Kém Trên `Light Mode`: Chữ Vàng `#FBC02D` Chìm Vào Nền Trắng Ở `VRVP POC`, `OHLCV Proxy`, `Ít Lệnh`, `QMV30`, Và Giá Tham Chiếu Watchlist
- **Test Case & Phân hệ:** `TC-QMV-UX-003` · Visual Contrast (`Light Mode`) | **Mức độ:** **P2 (Medium)**
- **Các bước tái hiện:** Bật `Light Mode` trên `TopHeader`; quan sát biểu đồ bật `VRVP` + `QMV_FLOW`, `IndicatorsModal`, `StockScreenerTab`, `StrategyTesterTab`, và dòng `HPG` (`+0.00%`) trên `Watchlist`.
- **Thực tế vs Kỳ vọng:**
  - **Thực tế:** Màu vàng sáng `#FBC02D` (`rgba(251,192,45,0.85)`) đặt trực tiếp trên nền trắng `#FFFFFF` (hoặc nền vàng nhạt `rgba(251,192,45,0.18)`) có tỷ lệ tương phản chỉ `~1.7:1` (không đạt chuẩn WCAG AA `4.5:1`): đường `POC` mờ khó nhìn, huy hiệu `⚠ Ít lệnh (2)` và giá tham chiếu `HPG 20,450 +0.00%` trên Watchlist gần như chìm vào nền trắng!
  - **Kỳ vọng:** Dùng biến CSS thích ứng theo theme (Dark: `#FBC02D`, Light: hổ phách đậm `#B45309` / `#9A6700`) đảm bảo độ tương phản rõ nét trên cả 2 chế độ Sáng/Tối.
- **Bằng chứng:** Ảnh [`12a_light_mode_chart_ichi_vrvp_flow.png`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence_qmv/12a_light_mode_chart_ichi_vrvp_flow.png), [`12c_light_mode_screener.png`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence_qmv/12c_light_mode_screener.png), [`12d_light_mode_strategy.png`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence_qmv/12d_light_mode_strategy.png).
- **Nguyên nhân gốc (`Root Cause`):** Hardcode `#FBC02D` tại [`customIndicators.ts:654`](file:///Users/mizuhara/workspace/sumi/ui/src/chart/indicators/customIndicators.ts#L654), [`IndicatorsModal.tsx:122`](file:///Users/mizuhara/workspace/sumi/ui/src/components/modals/IndicatorsModal.tsx#L122), [`StockScreenerTab.tsx:697, 742`](file:///Users/mizuhara/workspace/sumi/ui/src/components/dock/StockScreenerTab.tsx#L697), [`StrategyTesterTab.tsx:1132`](file:///Users/mizuhara/workspace/sumi/ui/src/components/dock/StrategyTesterTab.tsx#L1132), [`RightWidgetBar.tsx`](file:///Users/mizuhara/workspace/sumi/ui/src/components/RightWidgetBar.tsx).
- **Khuyến nghị sửa:** Khai báo biến `--tv-warning-fg` (`#FBC02D` ở dark, `#B45309` ở light) và dùng đồng bộ cho toàn bộ badge vàng, giá tham chiếu, và đường `VRVP POC`.

---

#### `BUG-QMV-015` — `RSI(14)` Đơn Tuyến In Thừa Nhãn Rỗng `RSI2: — RSI3: —` Trên Legend & `MACD(12,26,9)` Quên `precision: 2` Hiện 4 Chữ Số Thập Phân
- **Test Case & Phân hệ:** `TC-QMV-IND-007`, `TC-QMV-IND-008` · Custom Indicators Rendering | **Mức độ:** **P2 (Medium)**
- **Các bước tái hiện:** Áp dụng Mẫu `1. QMV Cốt Lõi` hoặc `4. QMV Bắt Đáy An Toàn` trên `HOSE:FPT 1D`, đọc dòng legend ở khung phụ `RSI(14)` và `MACD(12,26,9)`.
- **Thực tế vs Kỳ vọng:**
  - **Thực tế:** Khung `RSI(14)` in thừa **`RSI: 30.9   RSI2: —   RSI3: —`** dù chỉ có 1 tham số `[14]`; khung `MACD(12,26,9)` in 4 chữ số thập phân thừa **`DIF: -2,959.6328  DEA: -2,753.2544  HIST: -206.3784`** và trục Y `800.0000 / -800.0000`!
  - **Kỳ vọng:** `RSI(14)` chỉ in duy nhất `RSI: 30.9`; `MACD` định dạng gọn `precision: 2` (VD `-2,959.63`).
- **Bằng chứng:** Ảnh [`07a_tpl_kt01.png`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence_qmv/07a_tpl_kt01.png), [`07b_tpl_ichimoku_v7.png`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence_qmv/07b_tpl_ichimoku_v7.png), [`07d_tpl_safe_bottom.png`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence_qmv/07d_tpl_safe_bottom.png).
- **Nguyên nhân gốc (`Root Cause`):** [`ui/src/chart/indicators/customIndicators.ts:677-680`](file:///Users/mizuhara/workspace/sumi/ui/src/chart/indicators/customIndicators.ts#L677-L680) khai báo tĩnh 3 figures thay vì dùng `regenerateFigures`; [`customIndicators.ts:824-828`](file:///Users/mizuhara/workspace/sumi/ui/src/chart/indicators/customIndicators.ts#L824-L828) thiếu `precision: 2`.
- **Khuyến nghị sửa:** Thêm `regenerateFigures: (params) => ...` cho `qmvRsiIndicator` và thêm `precision: 2` cho `tradingViewMacdIndicator`.

---

#### `BUG-QMV-016` — Nhãn Vạch Tách Phiên Intraday (`13:00 Chiều / DD/MM`) Bị Xếp Chồng Đè Nát Chữ Trong Ô `1H` Đa Biểu Đồ
- **Test Case & Phân hệ:** `TC-QMV-TPL-005`, `TC-QMV-UX-002` · Chart Intraday Session Dividers | **Mức độ:** **P2 (Medium)**
- **Các bước tái hiện:** Chọn bố cục `3` ô (`1D + 4H + 1H`) trên `HOSE:FPT`, bật nút `Phiên` trên `TopHeader`.
- **Thực tế vs Kỳ vọng:**
  - **Thực tế:** Ở ô `1H` góc dưới phải (độ rộng hẹp), các hộp nhãn màu đen chữ vàng `13:00 13/11`, `13:00 16/11`, `13:00 17/11`, `13:00 18/11`... vẽ đè chồng chéo lên nhau tạo thành một dải đen không đọc được chữ!
  - **Kỳ vọng:** Ẩn bớt nhãn text (chỉ giữ vạch đứng nét đứt) hoặc áp khoảng cách tối thiểu `>= 68px` giữa 2 nhãn liền kề khi không gian ngang hẹp.
- **Bằng chứng:** Ảnh [`08_multichart_qmv.png`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence_qmv/08_multichart_qmv.png), [`09b_dock_default_screener_1280.png`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence_qmv/09b_dock_default_screener_1280.png), [`12a_light_mode_chart_ichi_vrvp_flow.png`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence_qmv/12a_light_mode_chart_ichi_vrvp_flow.png).
- **Nguyên nhân gốc (`Root Cause`):** Vòng lặp vẽ vạch tách phiên trong [`ui/src/chart/KLineViewport.tsx`](file:///Users/mizuhara/workspace/sumi/ui/src/chart/KLineViewport.tsx) không theo dõi `lastLabelX` để chống va chạm nhãn.
- **Khuyến nghị sửa:** Theo dõi `lastDrawnLabelRightX`; chỉ vẽ nhãn chữ khi `x - lastDrawnLabelRightX >= 8`, các vạch quá sát chỉ vẽ đường kẻ đứng mờ.

---

#### `BUG-QMV-017` — Ô Tìm Nhanh Screener Không Khớp Tên Ngành Tiếng Việt (`"Ngân hàng"` Trả 4/26 Mã), Không Xóa Khi Bấm `B5`, Và `WATCHLIST` Không Tự Quét Lại Khi Đổi Danh Mục
- **Test Case & Phân hệ:** `TC-QMV-SCR-002`, `TC-QMV-SCR-004`, `TC-QMV-SCR-010` · Stock Screener Search & Reactivity | **Mức độ:** **P2 (Medium)**
- **Các bước tái hiện:**
  1. Gõ `"Ngân hàng"` vào ô `Tìm mã...` trong `StockScreenerTab` → Enter.
  2. Bấm `"Gỡ bộ lọc mặc định"` khi ô `Tìm mã...` đang có chữ `"FPT"` và dropdown nhóm tín hiệu đang chọn `"Ichimoku V7"`.
  3. Chọn Vũ trụ `Danh mục đang mở` (`WATCHLIST`), rồi đổi Watchlist bên phải từ `QMV30` sang `Yumeko Core 10`.
- **Thực tế vs Kỳ vọng:**
  - **Thực tế:** Tìm `"Ngân hàng"` chỉ trả **`4` mã** (thay vì `26` mã ngành Ngân hàng) do backend chỉ đối chiếu `symbol`/`ticker`/`description` (phần lớn lưu tiếng Anh hoặc `"Vietnam Equity (CafeF)"`) chứ không khớp `qmv_sector_label`; bấm `"Gỡ bộ lọc mặc định"` không xóa `searchQuery` và `categoryFilter`; và đổi `activeWatchlistId` không kích hoạt quét lại khi đang ở chế độ `WATCHLIST`.
  - **Kỳ vọng:** Ô tìm kiếm khớp cả `qmv_sector_label` không phân biệt hoa thường; `"Gỡ bộ lọc mặc định"` đưa về trạng thái sạch toàn thị trường; đổi Watchlist tự động cập nhật bảng khi `exchangeFilter === 'WATCHLIST'`.
- **Bằng chứng:** Log `rust_selftest_results.json` (`searchBankCount: 4` vs `sectorGroup1BankingCount: 26`).
- **Nguyên nhân gốc (`Root Cause`):** [`crates/sumi-desktop/src/commands/scanner.rs:366-372`](file:///Users/mizuhara/workspace/sumi/crates/sumi-desktop/src/commands/scanner.rs#L366-L372) (không match `prof.sector.label_vi()`); [`ui/src/components/dock/StockScreenerTab.tsx:141-155, 200-213`](file:///Users/mizuhara/workspace/sumi/ui/src/components/dock/StockScreenerTab.tsx#L141-L155).
- **Khuyến nghị sửa:** Đối chiếu thêm `prof.sector.label_vi()` trong `scanner.rs`, reset `searchQuery` & `categoryFilter` trong `handleClearAllDefaultFilters`, và thêm `activeWatchlistId` vào `useEffect` (chỉ trigger khi `exchangeFilter === 'WATCHLIST'`).

---

#### `BUG-QMV-018` — `IndicatorSettingsModal` Không Kẹp Trần Trên Tham Số (`displacement = 9999`) & `IndicatorsModal` Không Xóa Từ Khóa Tìm Kiếm Khi Đóng/Mở Lại
- **Test Case & Phân hệ:** `TC-QMV-CAT-007`, `TC-QMV-CAT-002` · Indicator Modals | **Mức độ:** **P2 (Medium)**
- **Các bước tái hiện:**
  1. Mở cài đặt `ICHIMOKU`, nhập `Dịch mây = 9999` → bấm `Áp dụng`.
  2. Mở `IndicatorsModal`, gõ `"vrvp"`, thêm `VRVP`, đóng modal bằng `Esc` rồi mở lại `IndicatorsModal`.
- **Thực tế vs Kỳ vọng:**
  - **Thực tế:** `ICHIMOKU` lưu nguyên `[6, 18, 36, 9999]` (`ichiAfter9999Params: [6, 18, 36, 9999]`), làm mất trắng toàn bộ Mây Kumo và đường Chikou vì `9999 > 600` nến; còn `IndicatorsModal` giữ nguyên chữ `"vrvp"` khi mở lại (ảnh `12b`), khiến người dùng tưởng danh mục chỉ còn 1 chỉ báo!
  - **Kỳ vọng:** Kẹp tham số chu kỳ trong miền an toàn (vd `[1..500]`, riêng `displacement` của Ichimoku trong `[1..120]`, số bin `VRVP` trong `[8..64]`), và tự gọi `setQuery('')` mỗi khi đóng/mở lại `IndicatorsModal`.
- **Bằng chứng:** Log `e2e_suite1_results.json` (`TC-QMV-CAT-007`) và ảnh [`12b_light_mode_indicators_modal.png`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence_qmv/12b_light_mode_indicators_modal.png).
- **Nguyên nhân gốc (`Root Cause`):** [`ui/src/components/modals/IndicatorSettingsModal.tsx:109-112`](file:///Users/mizuhara/workspace/sumi/ui/src/components/modals/IndicatorSettingsModal.tsx#L109-L112); [`ui/src/components/modals/IndicatorsModal.tsx:42-46`](file:///Users/mizuhara/workspace/sumi/ui/src/components/modals/IndicatorsModal.tsx#L42-L46).
- **Khuyến nghị sửa:** Bổ sung hàm `clampIndicatorParams(name, params)` trước khi lưu và reset `setQuery('')` trong `onClose` của `IndicatorsModal`.

---

#### `BUG-QMV-019` — Chồng Lấn UI Cục Bộ: Toast Che Kín Thanh Tab Watchlist, Tab `"Đối tượng"` Bị Cắt Chữ `g`, Và Badge `Trần/TC/Sàn | Offline` / `REPLAY` Đè Lên Số Trục Giá Y
- **Test Case & Phân hệ:** `TC-QMV-UX-002`, `TC-QMV-RPL-002` · Visual Layout Polish | **Mức độ:** **P2 (Medium)**
- **Các bước tái hiện:** Áp dụng nhanh 2–3 Mẫu chỉ báo liên tiếp (xem góc trên phải); quan sát tab `"Đối tượng"` ở `RightWidgetBar` và badge `Trần / TC / Sàn | Offline` / `REPLAY` ở góc trên phải biểu đồ.
- **Thực tế vs Kỳ vọng:**
  - **Thực tế:**
    1. Các hộp Toast (`fixed top-12 right-4`) xếp chồng thẳng lên thanh tab `Danh mục | Cảnh báo | Đối tượng` và đè kín 1/3 danh sách Watchlist (không bấm đổi tab được cho tới khi tự đóng toast).
    2. Tab thứ 3 **`Đối tượng`** ở mép phải bị cắt mất nửa chữ **`g`** (`Đối tượn...`).
    3. Hộp badge `Trần 20,900 TC 19,550 Sàn 18,200 | Offline` và huy hiệu `REPLAY` đôi khi đè che con số giá trên cùng của trục Y (`26,000`, `76,000`) hoặc cắt ngang chữ `TRẦN 57,600`.
  - **Kỳ vọng:** Toast đặt ở góc dưới-trái vùng biểu đồ (hoặc căn `right: 316px` né `RightWidgetBar`, giới hạn tối đa 2 toast); chữ tab `"Đối tượng"` hiển thị trọn vẹn; badge góc trên phải căn cách mép phải một khoảng bằng bề rộng trục Y (`~64px`).
- **Bằng chứng:** Ảnh [`07d_tpl_safe_bottom.png`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence_qmv/07d_tpl_safe_bottom.png), [`11_replay_qmv_ichimoku_blind.png`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence_qmv/11_replay_qmv_ichimoku_blind.png), [`13_singlepane_light_ichimoku_ssi.png`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence_qmv/13_singlepane_light_ichimoku_ssi.png).
- **Nguyên nhân gốc (`Root Cause`):** [`ui/src/App.tsx:313`](file:///Users/mizuhara/workspace/sumi/ui/src/App.tsx#L313); [`ui/src/components/RightWidgetBar.tsx:363-395`](file:///Users/mizuhara/workspace/sumi/ui/src/components/RightWidgetBar.tsx#L363-L395); [`ui/src/chart/KLineViewport.tsx`](file:///Users/mizuhara/workspace/sumi/ui/src/chart/KLineViewport.tsx) (vị trí badge `right-2`).
- **Khuyến nghị sửa:** Chuyển vị trí `Toast` sang `bottom-10 left-14` (hoặc `right-[316px]` khi RightWidgetBar mở) và giới hạn tối đa 2 toast hiển thị cùng lúc; giảm padding ngang của 3 tab phải (`px-2`) để đủ chữ `"Đối tượng"`; dịch badge `Trần/TC/Sàn` & `REPLAY` sang `right-[72px]`.

---

#### `BUG-QMV-020` — Ô Biểu Đồ Intraday (`1H`/`4H`) Trống Đen Không Thông Báo Khi Đổi Sang Mã Chỉ Có Dữ Liệu `1D` Ở Chế Độ `Offline`
- **Test Case & Phân hệ:** `TC-QMV-BOOT-007`, `TC-QMV-UX-006`, `TC-QMV-UX-008` · Offline Empty State | **Mức độ:** **P2 (Medium)**
- **Các bước tái hiện:** Bật `Offline Mode`, mở bố cục `3` ô (`1H + 4H + 1D`), chọn một mã CafeF như `HOSE:DCM` (chỉ có nến `1D` cục bộ).
- **Thực tế vs Kỳ vọng:**
  - **Thực tế:** Ô `1H` và `4H` hiện trống đen chỉ có trục Y ảo `0..10` mà không có bất kỳ thông báo nào (đồng thời bắn liên tiếp `48` lỗi IPC 500 trong bài kiểm thử Soak 25 mã).
  - **Kỳ vọng:** Hiện thông điệp gọn gàng giữa ô: *"Chưa có dữ liệu khung 1H cục bộ cho HOSE:DCM trong chế độ Offline — chuyển sang 1D hoặc tắt Offline để tải."*
- **Bằng chứng:** Ảnh [`09a_dock_default_screener_1440.png`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence_qmv/09a_dock_default_screener_1440.png) (ô `DCM 1H` và `DCM 4H` đen trống).
- **Nguyên nhân gốc (`Root Cause`):** [`ui/src/chart/KLineViewport.tsx:642-685`](file:///Users/mizuhara/workspace/sumi/ui/src/chart/KLineViewport.tsx#L642-L685) catch lỗi `getBars` và gọi `callback([], false)` nhưng không set trạng thái `emptyReason` để hiển thị overlay hướng dẫn.
- **Khuyến nghị sửa:** Lưu `emptyReason` khi `bars.length === 0` (kèm nút 1-click `"Chuyển sang 1D"`).

---

#### `BUG-QMV-021` — Thiếu Request-ID Guard Chống Race Condition Đổi Mã Trong `StrategyTesterTab` & Lệch Ngữ Nghĩa Bộ Lọc `Nhóm Ngành 1–3` × `11 Ngành QMV`
- **Test Case & Phân hệ:** `TC-QMV-BT-015`, `TC-QMV-SCR-003` · Concurrency & Screener Taxonomy UX | **Mức độ:** **P2 (Medium)**
- **Các bước tái hiện:**
  1. Trong `StrategyTesterTab`, bấm chạy `WFO 5-Fold` trên `HOSE:FPT` rồi lập tức đổi mã biểu đồ sang `HOSE:HPG` trước khi WFO kết thúc.
  2. Trong `StockScreenerTab`, chọn `Nhóm 1: Tài chính & Đòn bẩy` rồi mở dropdown `Tất cả 11 ngành QMV`.
- **Thực tế vs Kỳ vọng:**
  - **Thực tế:**
    - `StrategyTesterTab` không kiểm tra request sequence ID hay so sánh `res.ticker === activePane.ticker` trước khi `setWfoResult`/`setBtResult`, có thể đắp nhầm kết quả của mã cũ lên mã mới.
    - Trong `StockScreenerTab`, khi chọn `Nhóm 1: Tài chính & Đòn bẩy`, kết quả trả về **`257` mã thuộc 7 ngành khác nhau** (`BANKING`, `BROKERAGE`, `REAL_ESTATE`, `CONSTRUCTION_STEEL`, `CONSUMER_FNB`, `INDUSTRIAL_LOGISTICS`, `OIL_GAS_CHEM`) — trong đó `CONSTRUCTION_STEEL` có tới `84` mã! Lý do: trong `qmv_universe.rs`, `GROUP_1` phân loại theo bảng `division_current` ở cấp từng mã (không gom chặt 1-1 theo 11 ngành), nhưng nhãn UI lại ghi `"Nhóm 1: Tài chính & Đòn bẩy"` khiến người dùng hiểu nhầm hoàn toàn, còn dropdown 11 ngành vẫn hiện cố định cả 11 mục không báo số lượng mã giao nhau.
  - **Kỳ vọng:** Dùng `runSeqRef` bỏ qua kết quả lỗi thời trong `StrategyTesterTab`; đồng bộ rõ ràng định nghĩa & tooltip của `Nhóm ngành 1–3` (theo đúng phân loại QMV `division_current` hay theo 11 ngành) và lọc động dropdown ngành con tương ứng.
- **Bằng chứng:** Log `rust_selftest_results.json` (`sectorGroup1DistinctSectors` gồm 7 ngành, `sectorGroup1SteelConflictCount: 84`) & `e2e_suite2_results.json` (`sectorOptionsUnderG1Count: 12`).
- **Nguyên nhân gốc (`Root Cause`):** [`ui/src/components/dock/StrategyTesterTab.tsx:333-407`](file:///Users/mizuhara/workspace/sumi/ui/src/components/dock/StrategyTesterTab.tsx#L333-L407); [`ui/src/components/dock/StockScreenerTab.tsx:345-380`](file:///Users/mizuhara/workspace/sumi/ui/src/components/dock/StockScreenerTab.tsx#L345-L380); [`crates/sumi-core/src/scanner/qmv_universe.rs:12-43, 333-360`](file:///Users/mizuhara/workspace/sumi/crates/sumi-core/src/scanner/qmv_universe.rs#L12-L43).
- **Khuyến nghị sửa:** Thêm `runSeqRef` chống stale response trong `StrategyTesterTab`; rà soát lại mapping `sector_group` ↔ `sector` trong `qmv_universe.rs` hoặc giải thích rõ trong tooltip/nhãn dropdown rằng `Nhóm 1–3` phân loại theo chu kỳ dòng tiền QMV ở cấp từng doanh nghiệp.

---

### Mức Độ P3 (Low) & P4 (UX Polish)

#### `BUG-QMV-022` — Sót Chuỗi Văn Bản Cũ Ngoài QMV & Chuỗi Tiếng Anh Thô Chưa Việt Hóa
- **Test Case & Phân hệ:** `TC-QMV-UX-004`, `TC-QMV-CAT-001`, `TC-QMV-UX-007` · Copywriting & Localization | **Mức độ:** **P3 (Low)**
- **Thực tế vs Kỳ vọng:**
  1. Tooltip nút `Chỉ báo` ([`TopHeader.tsx:262`](file:///Users/mizuhara/workspace/sumi/ui/src/components/TopHeader.tsx#L262)): vẫn ghi `"Chỉ báo kỹ thuật (26+ chỉ báo gốc + Ichimoku, SuperTrend, VWAP, ATR, StochRSI)"` dù bản `v2.2.0 QMV` chỉ giữ `10 Cốt lõi + 5 Phụ trợ` và đã gỡ `SuperTrend`, `StochRSI`!
  2. Placeholder ô thêm mã Watchlist ([`RightWidgetBar.tsx:486`](file:///Users/mizuhara/workspace/sumi/ui/src/components/RightWidgetBar.tsx#L486)): vẫn gợi ý Crypto `"Thêm mã (VD: FPT, BINANCE:BTCUSDT)"`.
  3. Khung Portfolio Backtest ([`engine.rs:1437`](file:///Users/mizuhara/workspace/sumi/crates/sumi-core/src/backtest/engine.rs#L1437), [`walk_forward.rs:651`](file:///Users/mizuhara/workspace/sumi/crates/sumi-core/src/backtest/walk_forward.rs#L651)): in nguyên dòng tiếng Anh `"UNIVERSE_UNGATED: Evaluated across active Watchlist feeds without Point-In-Time (PIT) membership schedule."`.
  4. Tiêu đề khối Phụ trợ trong `IndicatorsModal` ([`IndicatorsModal.tsx:250`](file:///Users/mizuhara/workspace/sumi/ui/src/components/modals/IndicatorsModal.tsx#L250)): hardcode cố định `"ATR · VRVP · VWAP · OBV · ADX"` ngay cả khi lọc chỉ còn `(1)` chỉ báo.
- **Bằng chứng:** Log `e2e_suite1_results.json` (`indBtnTitle`, `catTabCounts`), ảnh [`10d_portfolio_qmv30_1440.png`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence_qmv/10d_portfolio_qmv30_1440.png), [`12b_light_mode_indicators_modal.png`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence_qmv/12b_light_mode_indicators_modal.png).
- **Khuyến nghị sửa:** Cập nhật tooltip thành `"Bộ chỉ báo chuẩn QMV (10 Cốt lõi + 5 Phụ trợ)"`, đổi placeholder Watchlist thành `"Thêm mã VN (VD: FPT, SSI, HNX:IDC)"`, Việt hóa cảnh báo `UNIVERSE_UNGATED`, và nối động `auxItems.map(i => i.name).join(' · ')`.

---

#### `BUG-QMV-023` — `Cây Đối Tượng` (`Đối tượng`) Hiển Thị Mã Thô (`QMV_FLOW`, `RS_VNINDEX`) Thay Vì Tên Tiếng Việt & Huy Hiệu `OHLCV Proxy`
- **Test Case & Phân hệ:** `TC-QMV-TPL-004`, `TC-QMV-CAT-003` · Object Tree | **Mức độ:** **P3 (Low)**
- **Các bước tái hiện:** Áp mẫu `3. QMV Chu Kỳ Dòng Tiền` và mở tab `Đối tượng` ở `RightWidgetBar`.
- **Thực tế vs Kỳ vọng:** Hiển thị mã thô `QMV_FLOW(3,5,10,20)` và `RS_VNINDEX(20)` không kèm nhãn tiếng Việt hay huy hiệu `OHLCV Proxy`. Kỳ vọng hiển thị tên ngắn tiếng Việt dễ hiểu (VD `Chu Kỳ Dòng Tiền (OHLCV Proxy) (3,5,10,20)`).
- **Bằng chứng:** Ảnh [`08_multichart_qmv.png`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence_qmv/08_multichart_qmv.png) và `e2e_suite1_results.json` (`TC-QMV-TPL-004.objTreeSnippet`).
- **Nguyên nhân gốc (`Root Cause`):** [`ui/src/components/RightWidgetBar.tsx:957-960`](file:///Users/mizuhara/workspace/sumi/ui/src/components/RightWidgetBar.tsx#L957-L960) render thẳng `{ind.name}`.
- **Khuyến nghị sửa:** Dùng `findIndicatorMeta(ind.name)?.titleVi ?? ind.name` kèm huy hiệu `Proxy` nếu `meta?.isOhlcvProxy`.

---

#### `BUG-QMV-024` — Tab `Phân Nhóm QMV` (`Portfolio`) Hiển Thị Đồng Thời 2 Thanh KPI Trái Ngược (Đơn Mã Nằm Ngay Trên Danh Mục)
- **Test Case & Phân hệ:** `TC-QMV-BT-008` · Strategy Tester Portfolio UX | **Mức độ:** **P3 (Low)**
- **Các bước tái hiện:** Chạy Backtest đơn mã `HOSE:FPT` (ra `-6.15%`), sau đó bấm `Phân nhóm QMV` trên `QMV:QMV30` (ra `+28.54%`).
- **Thực tế vs Kỳ vọng:** Trên cùng một màn hình tab `Phân nhóm QMV`, dòng thanh trạng thái phía trên vẫn hiện `LN Ròng: -6.15% Max DD: -40.22% Sharpe: 0.02 Overfitted` (của đơn mã FPT) ngay sát bên trên `LN Nhóm: +28.54% Max DD: -13.82% Sharpe: 0.53` (của rổ QMV30) — rất dễ đọc nhầm!
- **Bằng chứng:** Ảnh [`10d_portfolio_qmv30_1440.png`](file:///Users/mizuhara/workspace/sumi/docs/qa/evidence_qmv/10d_portfolio_qmv30_1440.png).
- **Nguyên nhân gốc (`Root Cause`):** [`ui/src/components/dock/StrategyTesterTab.tsx:1127-1182`](file:///Users/mizuhara/workspace/sumi/ui/src/components/dock/StrategyTesterTab.tsx#L1127-L1182) render `ts` (single-symbol tearsheet) vô điều kiện bất kể `view === 'portfolio' | 'wfo' | 'mc'`.
- **Khuyến nghị sửa:** Chỉ hiển thị thanh KPI đơn mã `ts` khi `view === 'overview' || view === 'trades'`.

---

#### `BUG-QMV-025` — Đề Xuất Tối Ưu Hành Trình Thực Chiến Sổ Tay QMV Bước 3 (`Walkthrough Trader`)
- **Test Case & Phân hệ:** `TC-QMV-UX-010` · End-to-End QMV Trader Workflow | **Mức độ:** **P4 (UX Polish)**
- **Ghi nhận thực tế qua 5 bước thực chiến sau `15:00`:**
  1. **Giảm 2–3 cú click thừa nối Screener → Strategy Tester → Watchlist:** Hiện tại khi lọc ra ứng viên đẹp trong `StockScreenerTab`, trader phải click dòng để đổi biểu đồ → click đổi tab `Kiểm định Chiến lược QMV` → chọn Playbook → click `Backtest` (4 click). Nên thêm 2 nút icon nhỏ ở cuối mỗi dòng Screener: `★` (thêm nhanh vào Watchlist cá nhân) và `▷` (nhảy thẳng sang Strategy Tester kiểm định mã đó).
  2. **Thu gọn thanh điều khiển Dock & bổ sung thanh kéo Resize chiều cao Dock:** `BottomTradingDock` hiện chỉ có 2 mức cứng (`276px` hoặc `62vh`) mà không có đường viền kéo thả (`ns-resize`) — bổ sung kéo thả chiều cao sẽ nâng tầm UX ngang TradingView Desktop.
  3. **Mở rộng nhẹ chiều cao `IndicatorsModal`:** Hiện dòng thứ 10 (`MACD`) bị cắt nửa nút và mục thu gọn `"Phụ trợ Định lượng (5)"` bị đẩy xuống dưới nếp cuộn ở `1440×900` (ảnh `01_indicators_modal_dark.png`). Giảm padding dòng từ `py-2` → `py-1.5` giúp hiện gọn toàn bộ 10 chỉ báo Cốt lõi + tiêu đề 5 Phụ trợ ngay trên 1 màn hình không cần cuộn.

---

## 3. Tóm Tắt Các Hạng Mục Vượt Chuẩn Xuất Sắc (Điểm Mạnh Đã Xác Minh)

1. **Độ chính xác toán học & Đồng nhất 1:1 Chart ↔ Screener (`sumi-core`):** `ICHIMOKU [6,18,36,18]` sai số `0.000`, đám mây Kumo tương lai 18 phiên sắc nét và clip hoàn hảo không tràn trục Y; `QMV_FLOW (Z-50)`, `RSI(14)`, `CCI(14)`, `MACD(12,26,9)` khớp lõi Rust tới chữ số thập phân thứ 4–6; 5/5 mã Screener đối chiếu OHLCV gốc (`TAL`, `REE`, `TCH`, `BFC`, `HAH`) chính xác tuyệt đối.
2. **Vi cấu trúc TTCK Việt Nam & Bất biến Nhân quả (`DEF-001`):** `0` vi phạm lô `100`, `0` vi phạm chu kỳ thanh toán lịch sử (`T+3` trước `29/08/2022` & `T+2` sau), `0` rò rỉ tương lai trong Backtest (`23 === 23` lệnh) và Bar Replay (`0/12` mũi tên tương lai lọt qua), khóa `T+2` FIFO & bước giá HOSE `50đ` trong Paper Broker chuẩn xác, `VN30F1!` mở khóa Short/T+0 và tự reset sạch khi quay về HOSE.
3. **Hiệu năng & Độ bền bộ nhớ (`100-Action Soak`):** Quét toàn bộ `467` mã VN × `123` tín hiệu chỉ mất `291.5ms` (lạnh) / `164ms` (ấm); Backtest đơn mã `3.8–18.3ms`, Portfolio `60` mã `52–114ms`; sau 100 thao tác liên hoàn: `0` page errors, `0` rò rỉ `barStreams` (`0 → 0`), `0` rò rỉ Tauri callbacks (`6 → 6`), JS Heap phẳng tuyệt đối (`19.55 MB → 19.55 MB`).
4. **An toàn di trú SQLite & Native `.app`:** Chữ ký `codesign --verify --deep --strict` hợp lệ, khởi chạy lạnh `0.139s` / ấm `0.095s`, mở WAL chuẩn, di trú bảo toàn 100% watchlist/template/drawings cá nhân và idempotent tuyệt đối qua nhiều lần đóng/mở.
