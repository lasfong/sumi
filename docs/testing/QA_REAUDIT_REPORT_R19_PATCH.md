# BÁO CÁO TÁI NGHIỆM THU QA/QC & PHÁN QUYẾT PHÁT HÀNH (`R19 PATCH -> v1.0.0`)

- **Dự án:** Sumi Trading Workstation (Rust thuần `egui` `0.36` / `eframe` + `wgpu` Metal macOS)
- **Phiên bản nghiệm thu:** `v1.0.0` (`CFBundleShortVersionString = 1.0.0`, `CFBundleVersion = 19.1`, `Cargo.toml = 1.0.0`)
- **Gói phát hành:** [`dist/Sumi Workstation.app`](../../dist/Sumi%20Workstation.app) (Mach-O `arm64`, `codesign` ad-hoc hợp lệ, tích hợp `Contents/Resources/data/raw/cafef/CafeF.INDEX.Upto06112026.csv` chạy độc lập ngoài repo)
- **Tài liệu đối chiếu:**
  - Kế hoạch khắc phục: [`PATCH_R19_REMEDIATION_GOAL_PLAN.md`](./PATCH_R19_REMEDIATION_GOAL_PLAN.md) (`Wave 1 -> Wave 6`)
  - Báo cáo lỗi trước vá: [`QA_BUG_REPORT.md`](./QA_BUG_REPORT.md) (`34` lỗi: `4 P0`, `13 P1`, `14 P2`, `3 P3`)
  - Kịch bản kiểm thử chuẩn: [`SUMI_QA_QC_MASTER_TEST_PLAN.md`](./SUMI_QA_QC_MASTER_TEST_PLAN.md) (`12` Suite, `TC-01` .. `TC-65`)
- **Môi trường thực đo:** Apple Silicon `Apple M1` · `16.0 GiB RAM` · macOS `aarch64` · `rustc 1.99.1 (58cf8f9c3 2026-10-13)` · `--release`
- **Phán quyết nghiệm thu cuối cùng:** **`GO FOR v1.0`** (`34/34` lỗi đã đóng kèm test hồi quy tự động · `65/65` kịch bản đạt **`PASS`** · `274/274` unit/integration tests **`PASS`** với `0` cảnh báo biên dịch · `7/7` tệp gốc `CafeFData/*.zip` khớp SHA-256 `100%`)

---

## 1. Tóm Tắt Điều Hành (Trước Vá vs Sau Vá)

| Chỉ Số Nghiệm Thu | Trước Vá (`QA_BUG_REPORT.md`) | Sau Vá (`v1.0.0 / R19 Patch`) | Trạng Thái |
| :--- | :---: | :---: | :---: |
| **Kịch bản đạt chuẩn (`TC-01..TC-65`)** | `26 / 65 PASS` (`39 FAIL`) | **`65 / 65 PASS` (`0 FAIL`)** | **ĐẠT `100%`** |
| **Lỗi tồn đọng (`P0 / P1 / P2 / P3`)** | `4 / 13 / 14 / 3` (`34` lỗi) | **`0 / 0 / 0 / 0` (`34/34` đã đóng)** | **ĐẠT `100%`** |
| **Bộ kiểm thử tự động (`cargo test`)** | `267 passed; 0 failed` | **`274 passed; 0 failed` (`0 warnings`)** | **ĐẠT `100%`** |
| **Gói `.app` từ `/tmp` & bản copy độc lập** | Mất dữ liệu CafeF ngoài repo, lệch version | **Chạy sạch từ `/tmp` & bản copy độc lập, `v1.0.0` đồng nhất** | **ĐẠT** |
| **`100K` nến + `2 AVWAP` CPU hover frame** | `51.6 ms` (`FAIL` `< 4.0 ms`) | **`0.24 ms` median (`p95 = 0.52 ms`, tăng tốc `215×`)** | **ĐẠT** |
| **Watchlist `471` mã UI frame time** | `3.37 ms` (không ảo hóa, `484` dòng trùng) | **`0.24 ms` median (`p95 = 0.34 ms`, đúng `471` dòng)** | **ĐẠT** |
| **Bar Replay `+1` kèm Watchlist `471` mã** | `139.4 ms` (`~7 FPS`) | **`11.3–11.5 ms` (`~87 FPS`, nhanh hơn `12.3×`)** | **ĐẠT** |
| **Mở lạnh `Cmd+K` lần đầu trên `471` mã** | `153.1 ms` | **`13.92 ms` (`< 20 ms`, nhanh hơn `11×`)** | **ĐẠT** |
| **Benchmark `viewport_bench` (`R12/R17/R18/R19`)** | `4/4 PASS` | **`4/4 PASS` (`p99 max = 2.720 ms < 4.0 ms`)** | **ĐẠT** |
| **Bảo toàn `CafeFData/*.zip`, `ARCHIVE/`, `docs/evidence/`** | `7/7 SHA-256` khớp `100%` | **`7/7 SHA-256` khớp `100%`, `0` thay đổi lịch sử** | **ĐẠT** |

---

## 2. Bảng Truy Xuất Khắc Phục Toàn Bộ `34` Lỗi (`BUG-001` .. `BUG-034`)

| Mã Lỗi | Mức Độ | Wave | Tình Trạng Trước Vá | Kết Quả Sau Vá & Bằng Chứng Thực Chứng | Trạng Thái |
| :--- | :---: | :---: | :--- | :--- | :---: |
| **`BUG-001`** | `P0` | W1 | `VN30F1M` PnL sai `100×`, thiếu ký quỹ `17%`, Paper Trading thiếu thuế bán `0.10%` | `TradingConfig` áp dụng `contract_multiplier = 100.0`, `margin_rate = 0.17`, `sell_tax_bps = 10.0` trên VN equities. Probe `VN30F1M`: `dPts = -4.19 -> net_pnl = -574.29 kVND` (đúng `100×` + phí/trượt giá). Test: `wave1_regression_vn30f_multiplier_margin_sell_tax_and_dom_sizing`. | **CLOSED** |
| **`BUG-002`** | `P0` | W1 | DOM Ladder nhân đôi đòn bẩy `VN30F1M` (`26–457` HĐ trên `100k kVND`) | Chuẩn hóa kích cỡ lệnh DOM theo ký quỹ `17%` của giá trị danh nghĩa `price × 100`. Probe: `sized_contracts = Ok(4.0)` (khớp tối đa pháp định VSD `4` HĐ). | **CLOSED** |
| **`BUG-003`** | `P0` | W2 | Khởi động lại (`Cmd+Q`) dùng `RejectDataset`, làm mất Replay/vị thế/nét vẽ trên file `CafeF` | `restore_persisted_session` dùng `QuarantineRow` + `CafeFSplitAware` đồng nhất với importer, phục hồi trọn vẹn `CustomFile FPT`. Probe `TC-37`: `data_source=CustomFile symbol=FPT bars=2001 replay_active=true`. | **CLOSED** |
| **`BUG-004`** | `P0` | W1 | Lõi `TradingSession` cho phép `Market Sell` ở `T+1`; `Flatten` không bán phần đã về | Cưỡng chế FIFO `T+2` ngay trong lõi `TradingSession::apply_command` (`Err(InsufficientSettledShares)` ở `T+1`); `Flatten` xả đúng phần `sellable` (`200` cp) và giữ `locked` (`300` cp). | **CLOSED** |
| **`BUG-005`** | `P1` | W2 | `CafeFSplitAware` Rule 5 xóa oan `23.550` phiên phẳng (`O=H=L=C`) trên `471` mã | Chỉ kiểm tra envelope/làm tròn 4 chữ số thập phân sau chia tách, không xóa oan phiên trần/sàn/trắng bảng. Probe: `RCL` giữ `1.993` nến (`quar=1` thay vì mất `1.288` nến). | **CLOSED** |
| **`BUG-006`** | `P1` | W2 | `RS vs VNINDEX`, `Cmp:VNINDEX`, `Multi-Strategy` rơi về sóng sin giả (`+384%`) | `CafeFIndexCsv` & `resolve_benchmark_series_for_window` ưu tiên `CafeF.INDEX.Upto06112026.csv` (`2000–2026`, `574.41 -> 1725.36`). Probe `TC-20` & `TC-56`: `Bench (VNINDEX) +200.37%` khớp dữ liệu thật `100%` ([`10_tc20`](./qa_evidence_after/10_tc20_fpt_rs_cmp_vnindex_1280x800.png), [`16_tc56_57`](./qa_evidence_after/16_tc56_57_multistrat_0pct_slices_1440x1000.png)). | **CLOSED** |
| **`BUG-007`** | `P1` | W2 | `Cmd+K` gõ `VCB/HPG/SSI` chọn `VN30 Basket` giả, xóa sạch `471` mã | Ưu tiên tuyệt đối `CustomFile` (`+120` điểm) và loại bỏ bản sao `Vn30Basket` trùng mã. Probe: `Cmd+K` Enter `"VCB"` giữ `data_source=CustomFile, bars=2701, avail_syms=471`. | **CLOSED** |
| **`BUG-008`** | `P1` | W1+W3 | Tắt `T+2 Lock` mở bán khống trái phép trên `HOSE`; Backtest mở `24` lệnh Short trên `FPT` | Tách biệt `allow_short` khỏi `enforce_vn_t2_lot_lock` (luôn khóa theo `venue.allows_short()`) ở cả Paper Trading và Backtest/Battle/WFO/Portfolio. Probe: `submit_sell_from_flat = Err(ShortSellingDisabled)`, Backtest `short_trades = 0`. | **CLOSED** |
| **`BUG-009`** | `P1` | W1 | Vi phạm `DEF-005`: lệnh chờ khớp bằng Gap mở cửa chốt `TP` trước `SL` trên nến chạm cả hai | Ép kiểm tra `StopLoss` bi quan trước `TakeProfit` ngay cả ở Phase A Gap-Open. Probe: `exit_reason=Some(StopLossExit) exit_price=Some(98.0)`. | **CLOSED** |
| **`BUG-010`** | `P1` | W3 | Cutoff Backtest/Scanner không đơn điệu (tụt bar lúc `08:00 UTC`), nhận nhầm mã US sang `HOSE`, thiếu DST NYSE | Chuyển biên đóng phiên sang `>=` đơn điệu cho mọi khung `H1/H4/D1/W1`, bổ sung `is_us_nyse_dst_day` và chuẩn hóa phân loại sàn `UsEquities` vs `HOSE/HNX/UPCOM`. Probe: `bars@07:59:59UTC = 2703 -> bars@08:00:00UTC = 2703`. | **CLOSED** |
| **`BUG-011`** | `P1` | W3 | `RSI`/`MACD`/`DrawingLine` nổ hồi tố trên chính nến đặt lệnh (`SPY 14:30 UTC`) & nổ ma trên `1W` chưa đóng | Siết điều kiện `eval_ts > rule.armed_at_timestamp` và giới hạn strictly `..closed_buckets`. Probe: `SPY 14:30 UTC triggers = []`, `1W` giữa tuần không nổ ma. | **CLOSED** |
| **`BUG-012`** | `P1` | W2 | Bỏ tick `CafeF Split Norm` không từ chối tệp lỗi OHLC mà cách ly và xóa biểu đồ đang mở | Khi `import_cafef_split_aware == false`, dùng `RejectRow` / từ chối tệp hỏng và bảo toàn nguyên vẹn biểu đồ hiện hành. Probe: `res_is_ok=false active_sym=VNINDEX bars=120`. | **CLOSED** |
| **`BUG-013`** | `P1` | W2 | `TimestampUnit::Auto` nhầm ngày `YYYYMMDD` là giây Unix năm `1970` | Phát hiện định dạng `YYYYMMDD` (`19000101..=21001231`) trước khi coi là epoch giây. Probe: `parsed_labels = ["2024-10-04", "2024-10-09"]`. | **CLOSED** |
| **`BUG-014`** | `P1` | W4 | `IndicatorCache` trả dữ liệu cũ (`SMA(20) = 169.50`) khi sửa tham số lúc tắt rồi bật lại | Lưu chữ ký tính toán riêng từng chỉ báo (`sma_computed_period`, `atr_computed_period`, ...) và khóa `PAT` dùng đúng `ATR(14)`. Probe: `sma20=169.50 -> after_edit_to_50=154.50`. | **CLOSED** |
| **`BUG-015`** | `P1` | W4 | `AVWAP` bỏ qua `R11RenderMemo`, tính lại `2×/frame` gây nghẽn `51.6 ms` trên `100K` nến | Đưa `AVWAP` vào `R11RenderMemo` dùng chung khóa `candles_fast_fingerprint ^ dataset_generation` giữa pha hit-test và pha vẽ. Probe `100K + 2 AVWAP` hover: **`0.24 ms` median (`p95 = 0.52 ms`, giảm `215×`)**. | **CLOSED** |
| **`BUG-016`** | `P1` | W3 | `OutlierImpactReport::expectancy_ex_top_3_winners` chia sai mẫu số (`-300.0` vs `-100.0`) | Sửa mẫu số thành `valid_n - removed_winners_count`. Probe: `ex_top3_net=-300.00 -> reported_ex_top3_exp=Some(-100.0)`. | **CLOSED** |
| **`BUG-017`** | `P1` | W1 | Margin scale-in tính bằng giá vốn cũ cho phép đòn bẩy `1.5–1.8×`; lỗi `T+2` khung `1D/1W/US` | Tính lại sức mua theo giá thực khớp/ký quỹ hiện hành; chuẩn hóa mốc mở khóa `T+2 13:00 ICT` cho `1D/1W` và `T+1` cho cổ phiếu Mỹ. | **CLOSED** |
| **`BUG-018`** | `P2` | W5 | Nhãn `Alert` và `Drawing` bỏ qua chống đè trục giá phải, đè lên `Entry/SL/TP/Close` | Hợp nhất mảng `shared_axis_pills` và `shared_plot_badges` qua `resolve_non_overlapping_axis_pill_y` cho `Close`, `Trading`, `Alerts`, và `Drawings`. Ảnh soi thực tế: [`07_tc11_tc14`](./qa_evidence_after/07_tc11_tc14_both_docks_1280x800.png), [`08_tc14`](./qa_evidence_after/08_tc14_both_docks_1440x1000.png) (`0%` chồng đè). | **CLOSED** |
| **`BUG-019`** | `P2` | W5 | Hộp thoại `Templates` & `Cmd+K` bị mờ xuyên thấu frame đầu, mô tả làm giãn ngang khung | Thêm `.fade_in(false)` cho toàn bộ modal và bọc `wrap()` + khóa `max_width` trong `Templates`. Ảnh soi thực tế: [`22_tc61`](./qa_evidence_after/22_tc61_templates_modal_1280x800.png), [`24_tc03`](./qa_evidence_after/24_tc03_cmdk_471_1280x800.png). | **CLOSED** |
| **`BUG-020`** | `P2` | W5 | Watchlist `484` dòng trùng `FPT`, không ảo hóa (`3.37 ms`), Replay `+1` khựng `139.4 ms`, `Cmd+K` `153.1 ms`, `RRG` chồng `483` chữ | Khử trùng `CustomFile` (đúng `471` mã), ảo hóa `show_rows` (`0.24 ms`), cửa sổ đuôi nhân quả + cache theo peer + lazy Breadth (`Replay +1 = 11.3 ms`), `Cmd+K` `O(1)` (`13.92 ms`), chống đè RRG top `14` nhãn ([`15_tc51`](./qa_evidence_after/15_tc51_breadth_rrg_proxybb_1440x1000.png)). | **CLOSED** |
| **`BUG-021`** | `P2` | W2 | Phân loại nhầm mã `HNX/UPCOM` (`C69/KSV/NDN/BVS`) sang `HOSE`; thiếu lịch lễ `2016–2017` | Ánh xạ sàn chuẩn cho `471` mã CafeF (`C69/NDN/BVS -> Hnx`, `KSV -> Upcom`, `VNXALL -> VnIndexBenchmark`); mở rộng `VIETNAM_HOLIDAYS_V1` sang **`171` ngày (`2016–2030`)**. Probe: Tết `2016` `2016-02-05 -> 2016-02-17 09:00 ICT`. | **CLOSED** |
| **`BUG-022`** | `P2` | W5 | Crosshair mất ở vùng trắng tương lai; `Sync T` méo `>85%`; `2x2` nạp `S2` gai `-34.5%` & `S3` giả lập trống `95%` | Thêm `linked_whitespace_timestamp` (không pin giả bar cuối), nâng trần `right_whitespace_bars_frac` lên `0.96`, nâng cấp `2x2` trên `CustomFile` dùng `VNINDEX` sạch + `VCB`/`HPG` thật ([`17_tc59`](./qa_evidence_after/17_tc59_quad_2x2_1440x1000.png)). | **CLOSED** |
| **`BUG-023`** | `P2` | W5 | Top-bar `%Chg` tính theo `Open` lệch Watchlist; tiền tệ cứng `$`; toolbar tràn `4–5` hàng ở `1280×800`; không xóa filter mã | Đồng nhất `%Chg` theo `prev_close` trên Top Toolbar, Watchlist & Chart HUD; tiền tệ động `kVND`/`$` theo sàn; nén Top Toolbar đúng **`2` hàng phẳng** ở `1280×800`; xóa `symbol_picker_filter` ngay khi chọn mã ([`00_tc01`](./qa_evidence_after/00_tc01_app_from_tmp_1280x800.png), [`07_tc11`](./qa_evidence_after/07_tc11_tc14_both_docks_1280x800.png)). | **CLOSED** |
| **`BUG-024`** | `P2` | W1+W5 | Lỗi chuyển bậc giá `10.00` kVND trên DOM HOSE (`CEIL`/`mark` lệch); DOM đẩy `Mark`/Bid ra khỏi màn hình `1280×800` | Chuẩn hóa bước giá giao bậc `10.00` & `50.00` (`ceil_rows=[9.99], mark_rows=[9.98], buy@10.00 -> Stop`); thu gọn header & tự động cuộn căn giữa `Mark` (`dom_auto_center_pending`). Ảnh [`14_tc43`](./qa_evidence_after/14_tc43_vn30f_dom_ladder_1280x800.png) hiển thị đồng thời `Ask + Mark + Bid`. | **CLOSED** |
| **`BUG-025`** | `P2` | W2 | Preset `VNINDEX Synth` và `VN30F1M 1m` sinh nến vào ngày nghỉ lễ `01–02/09/2026` | Lọc lịch nghỉ `VIETNAM_HOLIDAYS_V1` trong generator và kiểm toán cả khung intraday. Probe: `VNINDEX Synth hol_wknd=0`, `VN30F1M hol_wknd=0, earliest=2026-09-03`. | **CLOSED** |
| **`BUG-026`** | `P2` | W4 | `TextNote` cắt cụt ghi chú `280` ký tự tại `272`; nhãn Custom Fib (`OTE 9`/`Full 13`) chồng đè nhau | Bọc dòng theo pixel thực tế (`wrap_text_note_lines(36, 14)` + galley wrap) bảo toàn `100%` `280` ký tự UTF-8; giãn cách dọc chống đè `>= 14.5 px` + nền pill tối cho Fib ([`12_tc22_26`](./qa_evidence_after/12_tc22_26_drawings_inspector_textnote_1280x800.png)). | **CLOSED** |
| **`BUG-027`** | `P2` | W4 | `MFI/CCI/SuperTrend/Keltner/VolumeSMA` dễ dính `NaN/±Inf`; pan conflation bị rung pha; `%`/`Idx100` hỏng nếu bar 0 = `0` | Chặn `.is_finite()` trên toàn bộ chỉ báo & `CandleBlockIndex`; cố định pha lấy mẫu conflation `(start/step)*step`; ngắt băng `BB/KC/VWAP` tại `None`; tìm bar gốc `> 0.0` hợp lệ cho `%`/`Idx100`. | **CLOSED** |
| **`BUG-028`** | `P2` | W3 | Cảnh báo tạo ở Live (`created_at == None`) lộ ra khi tua Bar Replay về quá khứ | Luôn gắn `created_at_timestamp = Some(replay_cutoff.unwrap_or(armed_at))` để mọi cảnh báo Live được ẩn kín trong quá khứ Replay (`DEF-001`). | **CLOSED** |
| **`BUG-029`** | `P2` | W3 | `Breadth` cắt trùng 2 lần; `VN30-PIT` lờ bộ lọc `VN30 Only` trên `CustomFile` & lệch `W1` `UniverseRemoval`; `Yearly` báo `-100%` giả sau phá sản; thiếu guard `NaN/<=0` | Sửa điều kiện trim tail Breadth; áp `VN30 Only` trên `CustomFile` (`constituents = 8`); đồng bộ `open_timestamps` cho `W1` PIT; trả `0.0%` khi vốn đầu năm `<= 0`; thêm guard `.is_finite() && > 0.0` toàn engine Backtest. | **CLOSED** |
| **`BUG-030`** | `P2` | W1 | `TradeJournal` thổi phồng `stop_and_lot_sized` và tạo `r_multiple` giả; `Debrief` trộn KPI không cắt `cutoff`; đổi `Venue` ở `[Settings]` lệch `T+2`; lưu phiên sau rewind rò ghi chú tương lai | Siết kiểm tra SL hợp lệ theo hướng vị thế; cắt `cutoff` nhất quán toàn bộ `SessionDebriefReport`; đồng bộ `sync_trading_settlement_rules()` khi đổi `Venue`; lọc ghi chú `<= cutoff` khi chụp snapshot phiên. | **CLOSED** |
| **`BUG-031`** | `P2` | W2 | `QuarantineRow` trong CSV/JSON loader không đồng nhất với các dòng `NaN`/âm/trùng lặp | Chuẩn hóa chính sách `QuarantineRow` thống nhất giữa `csv_loader.rs` và `json_loader.rs` kèm thống kê đếm chính xác. | **CLOSED** |
| **`BUG-032`** | `P3` | W5+W6 | Chuỗi cứng `"144 dates"`, thiếu phân trang `Sig`, `[Keys]` thiếu cảnh báo xung đột, `-> Order Ticket` sáng trên `TextNote`, thiếu nút `EN/VI` Catalog, cắt cụt `ICH`/`Multi-Strat`, `.app` lệch version & lệ thuộc repo | Cập nhật động `171 dates · 2016–2030`, hiện `Showing 64 / N hits`, banner cảnh báo xung đột `[Keys]`, vô hiệu hóa `-> Order Ticket` khi chọn `TextNote`, nút `EN+VI/EN/VI` Catalog ([`19_tc50`](./qa_evidence_after/19_tc50_signal_catalog_101_1280x800.png)), wrap `ICH`/`Multi-Strat` 2 dòng, đệm biên `PAT`, đồng bộ `v1.0.0` và nhúng `CafeF.INDEX` vào `.app/Contents/Resources`. | **CLOSED** |
| **`BUG-033`** | `P3` | W4 | Volume `< 1000` in `0K`, thiếu bậc `T` cho `CVD`, `0.00%` có dấu `+`, `FRVP POC` chạm biên trên, thiếu `invert_scale` trong Template | Giữ nguyên số nguyên `< 1000`, thêm bậc `T`, bỏ `+` ở `0.00%` ([`03_tc10`](./qa_evidence_after/03_tc10_fpt_pct_mode_1280x800.png)), kẹp `FRVP POC` theo `plot_rect.top + 3.0`, lưu `invert_scale` tương thích ngược trong `WorkspaceSlotTemplate`. | **CLOSED** |
| **`BUG-034`** | `P3` | W1+W3 | Lỗi biên sau năm `2030`, làm tròn `"80% (< 80% Target)"`, nhãn `>80B` trùng slot, `UniverseRemoval` đỉnh vốn ảo, `rsi_overbought_cross = 30.0`, `eval_base_timestamp == 0`, newline trong Markdown | Đã vá trọn vẹn `7/7` lỗi biên ở Wave 1 và Wave 3 (`default_value = 70.0`, `Option<i64>` giữ mốc epoch `0`, lọc `\r`/`\n` trong `export_markdown`, tính đúng đỉnh vốn prior-bar trước phí thoát). | **CLOSED** |

---

## 3. Ma Trận Tái Kiểm Thử Toàn Bộ `65` Kịch Bản (`TC-01` .. `TC-65`)

| Suite | Mã TC | Kịch Bản Kiểm Thử | Kết Quả | Bằng Chứng Tái Nghiệm Thu |
| :--- | :---: | :--- | :---: | :--- |
| **01** | `TC-01` | Khởi động `.app` từ `/tmp` & bản copy độc lập | **`PASS`** | Chạy sạch từ `/tmp` & `/tmp/SumiStandaloneTest.app`, `codesign` Mach-O arm64 hợp lệ, `v1.0.0` đồng bộ ([`00_tc01`](./qa_evidence_after/00_tc01_app_from_tmp_1280x800.png)). |
| **01** | `TC-02` | Nạp `CafeF Adj (471 mã)` `< 1 s` | **`PASS`** | Nạp `941.648` dòng / `471` mã trong **`412.0–442.0 ms`**, tự chọn `FPT` (`2.703` nến). |
| **01** | `TC-03` | `Cmd+K` & tìm đủ `471` mã `A..Y` | **`PASS`** | Chuyển mã `0.17–0.38 ms`; mở lạnh `Cmd+K` **`13.92 ms`**; gõ `VCB` + Enter giữ `CustomFile 471` mã (`2.701` nến); xóa `symbol_picker_filter` ngay khi chọn ([`24_tc03`](./qa_evidence_after/24_tc03_cmdk_471_1280x800.png)). |
| **01** | `TC-04` | `Split Norm` & `[Data Audit]` (`C69/HPX/KSV/NDN/FPT`) | **`PASS`** | Đúng sàn (`C69/NDN=Hnx`, `KSV=Upcom`, `HPX/FPT=Hose`), không xóa oan phiên phẳng, lịch `171 dates (2016–2030)` ([`01_tc04`](./qa_evidence_after/01_tc04_data_audit_c69_1280x800.png)). |
| **01** | `TC-05` | `BVS/API/APS` (KRX `High=0`) & `CafeF Index` | **`PASS`** | Cách ly đúng đúng phiên lỗi `High=0` (`quar=1..2`, `min_low > 0`), không xóa oan phiên phẳng; `VNINDEX` (`6.378` nến) & `HNX-INDEX` (`4.669` nến). |
| **01** | `TC-06` | Đường dẫn lỗi, tệp rỗng, bỏ tick `Split Norm` | **`PASS`** | Đường dẫn lỗi và bỏ tick `Split Norm` đều từ chối sạch và giữ nguyên biểu đồ đang mở (`active_sym=VNINDEX`, `FPT 2703` nến); `YYYYMMDD` không hỏng về `1970`. |
| **02** | `TC-07` | Cursor-Anchored Zoom `20` lần | **`PASS`** | Độ trôi tối đa **`0.000000000`** thanh. |
| **02** | `TC-08` | Pan, Kinetic & Future Whitespace (`Right Offset`) | **`PASS`** | Quán tính mượt, hỗ trợ whitespace tới `96%` không giật ngược. |
| **02** | `TC-09` | Kéo trục giá, double-click `Auto`, kéo trục thời gian | **`PASS`** | Chuẩn TradingView (`274/274` tests pass). |
| **02** | `TC-10` | `Linear / Log / % / Idx100` & `Inverted` | **`PASS`** | Trục `%` hiển thị `0.00%` không có dấu `+`, tự chọn bar gốc `> 0.0` ([`02_tc10`](./qa_evidence_after/02_tc10_fpt_log_inverted_1280x800.png), [`03_tc10`](./qa_evidence_after/03_tc10_fpt_pct_mode_1280x800.png)). |
| **02** | `TC-11` | Chống đè nhãn trục giá phải (`Anti-Collision`) | **`PASS`** | Tất cả `7` nhãn (`Close`, `Entry`, `SL`, `TP`, `3 Alerts`) tách bạch `0%` chồng đè ([`07_tc11_tc14`](./qa_evidence_after/07_tc11_tc14_both_docks_1280x800.png)). |
| **02** | `TC-12` | `4` bảng màu nến & `Hollow Bullish` | **`PASS`** | Râu nến `Hollow Bullish` dừng chuẩn mép thân, `Okabe-Ito` tương phản cao ([`04_tc12`](./qa_evidence_after/04_tc12_fpt_hollow_bullish_1280x800.png), [`05_tc12`](./qa_evidence_after/05_tc12_fpt_okabe_ito_1280x800.png), [`18_tc12`](./qa_evidence_after/18_tc12_settings_modal_1280x800.png)). |
| **02** | `TC-13` | `Conflation` `100K` nến (`Fit`) | **`PASS`** | `conflation_factor = 32`, giữ trọn `[1207.90, 20088.09]`, `p99 = 0.275 ms`, cố định pha chống rung ([`06_tc13`](./qa_evidence_after/06_tc13_100k_fit_all_1280x800.png)). |
| **02** | `TC-14` | Bố cục `1280×800` & `1440×1000` (mở cả `2` dock) | **`PASS`** | Top Toolbar đúng **`2` hàng phẳng lì** ở `1280×800`, tiền tệ `kVND`, `%Chg` đồng nhất (`-3.97%`) trên Top Bar / HUD / Heatmap ([`07_tc11_tc14`](./qa_evidence_after/07_tc11_tc14_both_docks_1280x800.png), [`08_tc14`](./qa_evidence_after/08_tc14_both_docks_1440x1000.png)). |
| **03** | `TC-15` | Gộp đa khung `1m..1W` (`VN30F1M` & `FPT`) | **`PASS`** | `VN30F1M` `5400 -> 5`, `hol_wknd_bars = 0` (`2026-09-03..2026-10-02`), bỏ sạch ngày nghỉ Quốc khánh `02/09`. |
| **03** | `TC-16` | Chỉ báo `Overlays` (`SMA/EMA/BB/ICH/ST/PSAR/PAT`) | **`PASS`** | `IndicatorCache` luôn khớp tham số (`169.50 -> 154.50`), `ICH` wrap 2 hàng, `PAT` có đệm biên trên sạch sẽ ([`09_tc16_19`](./qa_evidence_after/09_tc16_19_vn30f_15m_indicators_1440x1000.png)). |
| **03** | `TC-17` | `VRVP` (`70% VA/POC`) & `TPO` (`IB/Single Prints/NPOC`) | **`PASS`** | Vẽ chính xác và cập nhật động trên `VN30F1M 15m` ([`09_tc16_19`](./qa_evidence_after/09_tc16_19_vn30f_15m_indicators_1440x1000.png)). |
| **03** | `TC-18` | Multi-Interval `VWAP` (`±1σ, ±2σ`) | **`PASS`** | Welford bền số học, ngắt dải băng khi có khoảng trống `None`. |
| **03** | `TC-19` | Khung dao động phụ (`RSI/MACD/ATR/ADX/MFI/CVD...`) | **`PASS`** | `MFI/CCI` không bao giờ sinh `NaN/Inf`, hỗ trợ hậu tố `T`, `IndicatorCache` sạch stale. |
| **03** | `TC-20` | `RS vs VNINDEX` & `Cmp:VNINDEX` trên `FPT` | **`PASS`** | Dùng `VNINDEX` thật `2016–2026` (`574.41 -> 1725.36`, `RS = 217.54 [+117.54%]`, [`10_tc20`](./qa_evidence_after/10_tc20_fpt_rs_cmp_vnindex_1280x800.png)). |
| **03** | `TC-21` | Dữ liệu ngắn (`BCG 7`, `DSH 10`, `ECO 16`, `TCD 20` nến) | **`PASS`** | Bật đủ toàn bộ `11` khung chỉ báo không panic/crash/NaN ([`11_tc21`](./qa_evidence_after/11_tc21_short_bcg_7bars_all_ind_1280x800.png)). |
| **04** | `TC-22` | Vẽ đủ `9` công cụ, khung phụ & hủy `Escape` | **`PASS`** | Đủ `9` công cụ + `AVWAP` cache trong `R11RenderMemo` chỉ tốn **`0.24 ms`** hover trên `100K` nến. |
| **04** | `TC-23` | Nam châm bắt điểm `Magnet OHLC` | **`PASS`** | Hít đúng `O/H/L/C` gần nhất khi bật. |
| **04** | `TC-24` | Custom Fibonacci (`ICT OTE 9` / `Full 13`, `Undo` nguyên tử) | **`PASS`** | Đủ mức `0.705`, `-0.272`, `0.79`, `Undo` `1` bước, giãn cách dọc chống chồng chữ `>= 14.5 px` ([`12_tc22_26`](./qa_evidence_after/12_tc22_26_drawings_inspector_textnote_1280x800.png)). |
| **04** | `TC-25` | `TrendLine` `Extend Left/Right` khi vẽ ngược `A1 > A2` | **`PASS`** | `Extend Right` kéo đúng về tương lai, `Extend Left` kéo đúng về quá khứ. |
| **04** | `TC-26` | `TextNote` UTF-8 & chống xuyên thấu Click-Through | **`PASS`** | Bảo toàn `100%` `280` ký tự UTF-8 tiếng Việt, hit-test ưu tiên `Note` trên `Rect`, vô hiệu hóa `-> Order Ticket` ([`12_tc22_26`](./qa_evidence_after/12_tc22_26_drawings_inspector_textnote_1280x800.png)). |
| **04** | `TC-27` | Bảo toàn tọa độ thời gian khi đổi `1D <-> 1W` | **`PASS`** | `drift_bar = 0.000000`, giữ nguyên mốc thời gian `1789344000`. |
| **04** | `TC-28` | `Lock`, `Visible` & `Undo/Redo` `64` bước | **`PASS`** | Khóa sửa/xóa, chọn lại nét ẩn trong `LAYERS`, `Undo/Redo` nguyên vẹn. |
| **04** | `TC-29` | Cách ly nét vẽ theo từng mã (`FPT <-> VCB`) | **`PASS`** | `FPT: 9 -> VCB: 0 -> FPT: 9`, cách ly tuyệt đối. |
| **05** | `TC-30` | `<< Cut Bar`, `+1`, `-1`, `Play`, `\|<< Start` | **`PASS`** | Cắt nến và điều khiển bước thời gian chính xác. |
| **05** | `TC-31` | `DEF-001` chống lộ nến khung lớn (`1D -> 1W` giữa tuần) | **`PASS`** | `LEAK = false`, cutoff đơn điệu `bars@07:59:59UTC = 2703 -> bars@08:00:00UTC = 2703`, không nổ cảnh báo ma `1W`. |
| **05** | `TC-32` | `DEF-001` Multi-Chart `2x1`/`2x2` Replay | **`PASS`** | Ô phụ `FPT 1W` khớp đúng nến chưa đóng `2023-01-02 (H=45.16, C=44.60)` ([`13_tc32`](./qa_evidence_after/13_tc32_multichart_2x1_replay_1280x800.png)). |
| **05** | `TC-33` | `DEF-001` ẩn nét vẽ & cảnh báo tương lai trong Replay | **`PASS`** | Cả nét vẽ tương lai (`visible_in_past = false`) lẫn cảnh báo tạo ở Live đều ẩn kín khi tua về quá khứ. |
| **05** | `TC-34` | `DEF-001` Watchlist / `Sig` / `Breadth` / `Audit` khóa `<= T` | **`PASS`** | `Breadth` không còn cắt trùng 2 lần, không rò mã giả `2026-11-13`, cutoff nhân quả đồng bộ `100%`. |
| **05** | `TC-35` | `DEF-001` Phân kỳ `+3B` bất biến | **`PASS`** | `future_mutation_mismatches = 0`. |
| **05** | `TC-36` | Tua lùi `<< -1` khôi phục bit-exact vị thế & cảnh báo | **`PASS`** | Sai số vốn **`0.000000000`**, khôi phục nguyên vẹn lệnh & cảnh báo `Once`. |
| **05** | `TC-37` | Khôi phục phiên khi khởi động lại (`Cmd+Q`) | **`PASS`** | Phục hồi trọn vẹn `CustomFile FPT` (`2001` bar Replay active, giữ nguyên nét vẽ & vị thế). |
| **06** | `TC-38` | `Pos R:R <-> Order Ticket`, lô `100`, thuế bán `0.10%` | **`PASS`** | Áp dụng đầy đủ lô `100`, thuế bán `0.10%`, hệ số `100×` & ký quỹ `17%` `VN30F1M`. |
| **06** | `TC-39` | Khớp `Market/Limit/Stop`, Gap & bi quan `SL` trước `TP` | **`PASS`** | Khớp `DEF-005` (`StopLossExit @ 98.0` trước `TP`), siết margin scale-in theo giá mới. |
| **06** | `TC-40` | Khóa lô `T+2` (`13:00 ICT`) FIFO trên `HOSE` | **`PASS`** | Lõi `TradingSession` chặn bán `T+1` (`InsufficientSettledShares`); `Flatten` xả đúng `200` cp đã về và giữ `300` cp khóa. |
| **06** | `TC-41` | `T+2` xuyên Tết & lễ `30/04–01/05` (`VIETNAM_HOLIDAYS_V1`) | **`PASS`** | Đủ `171` ngày (`2016–2030`): Tết `2016` (`2016-02-05 -> 2016-02-17 09:00 ICT`), Tết `2026` (`2026-02-24 13:00 ICT`), `30/04–01/05` (`2026-05-05 13:00 ICT`). |
| **06** | `TC-42` | `SL`/`TP` tự động khi có cổ phiếu khóa `T+2` & tắt Lock | **`PASS`** | `Lock=ON`: Auto-SL xả `200` cp đã về, giữ `300` cp khóa; `Lock=OFF`: đóng hết `100%` nhưng **chặn tuyệt đối bán khống HOSE** (`ShortSellingDisabled`). |
| **06** | `TC-43` | DOM Ladder 21 bậc (`HOSE 10.00/50.00`, `VN30F1M`) | **`PASS`** | Bước giá `10.00`/`50.00` chính xác, `VN30F1M` PnL `100×` (`-574.29 kVND`), DOM cấp `4` HĐ chuẩn VSD, tự căn giữa thấy rõ `Ask + Mark + Bid` ở `1280×800` ([`14_tc43`](./qa_evidence_after/14_tc43_vn30f_dom_ladder_1280x800.png)). |
| **07** | `TC-44` | Tính bất biến kỷ luật `SL Adh %` | **`PASS`** | `PracticeScoreboard` & `TradeJournal` đồng nhất `50% (1/2)`, không thổi phồng checklist hay tạo `r_multiple` giả. |
| **07** | `TC-45` | `Decision Note` & `Session Debrief` | **`PASS`** | Toàn bộ KPI & bảng chi tiết `Debrief` cắt nhất quán theo `cutoff`, hết lỗi làm tròn `"80% (< 80% Target)"` ([`20_tc45`](./qa_evidence_after/20_tc45_session_debrief_1280x800.png)). |
| **07** | `TC-46` | Nút `Reset Practice` 1 chạm | **`PASS`** | Reset sạch lệnh/vốn, giữ nguyên `cursor = 2004` và `4` nét vẽ. |
| **07** | `TC-47` | Lưu/Ghim `PracticeSessionStore` & Xuất `CSV/MD` | **`PASS`** | Chống CSV Formula Injection (`safe=true`), lọc newline Markdown, đồng bộ `T+2 Lock` & cắt ghi chú tương lai khi lưu phiên. |
| **08** | `TC-48` | Quét `471` mã Watchlist & ảo hóa `show_rows` | **`PASS`** | Đúng `471` dòng (`1` dòng `FPT`), quét `24.4 ms`, UI `show_rows` **`0.24 ms`/frame**, Replay `+1` **`11.3–11.5 ms`**. |
| **08** | `TC-49` | Luồng `Scanner -> Replay` (`Blind +120b`) | **`PASS`** | `revealed_at_cut = false -> revealed_after_120 = true`. |
| **08** | `TC-50` | `Signal Catalog 101` & Điểm hợp lưu | **`PASS`** | Đủ `101` tín hiệu, default `rsi_overbought_cross = 70.0`, có nút chuyển ngôn ngữ **`EN+VI / EN / VI`** ([`19_tc50`](./qa_evidence_after/19_tc50_signal_catalog_101_1280x800.png)). |
| **08** | `TC-51` | `Breadth`, `RRG` 4 góc & `ProxyBB` (`T03..T200`) | **`PASS`** | `participating=471`, `vn30=8`, `2026-11-06`, RRG `0%` chồng chữ, Heatmap `FPT -3.97%` thật, `AT05` volume=0 trả `None` ([`15_tc51`](./qa_evidence_after/15_tc51_breadth_rrg_proxybb_1440x1000.png)). |
| **09** | `TC-52` | Cảnh báo Giá/RSI/MACD/`TrendLine` nghiêng/`cooldown` | **`PASS`** | `TrendLine` nghiêng + cooldown chuẩn, `SPY 14:30 UTC` `0` nổ hồi tố, `1W` giữa tuần `0` nổ ma, `eval_base_timestamp` hỗ trợ epoch `0`. |
| **09** | `TC-53` | Xuất lịch sử cảnh báo `CSV` / `JSON` | **`PASS`** | Chuẩn RFC-4180 CSV & RFC-8259 JSON, lọc nhân quả `<=` mốc Replay. |
| **10** | `TC-54` | Backtest đơn `Open[T+1]` trên `FPT` (`2016–2026`) | **`PASS`** | `open_t1_match = true`, `21` lệnh **Long-only chuẩn HOSE (`short_trades = 0`)**, guard `price.is_finite() && > 0.0`. |
| **10** | `TC-55` | `Custom Signal Rule Builder` (`AND`/`OR`) | **`PASS`** | `113` lệnh sạch (`+6.00%`), chặn warmup. |
| **10** | `TC-56` | `1-Click Battle` & `Multi-Strategy [0% UTC]` | **`PASS`** | `Battle 471` mã **`66.5–67.6 ms`**; `Bench (VNINDEX)` thật **`+200.37%`**, legend wrap 2 dòng đủ chữ ([`16_tc56_57`](./qa_evidence_after/16_tc56_57_multistrat_0pct_slices_1440x1000.png)). |
| **10** | `TC-57` | `Tearsheet` (`OutlierImpact`, `DD`, `Yearly`, `Manifest`) | **`PASS`** | `RunManifest` tất định (`4291561381311730471 == 4291561381311730471`), `expectancy_ex_top_3_winners = -100.0`, `Yearly` không báo `-100%` giả. |
| **10** | `TC-58` | `WFO`, `Monte Carlo` `1.000` & `VN30-PIT` Portfolio | **`PASS`** | `WFO = 1.2 ms`, `VN30-PIT` lọc chuẩn `8` mã `VN30` trên `471` mã CafeF (`22.1 ms`), khớp `W1` `UniverseRemoval`, không đỉnh vốn ảo. |
| **11** | `TC-59` | Lưới đa biểu đồ `2x2` (`Sync X` / `Sync T`) | **`PASS`** | `S1: FPT`, `S2: VNINDEX` sạch (không gai `-34.5%`), `S3: VCB` & `S4: HPG` thật; linked whitespace crosshair hoạt động mượt ([`17_tc59`](./qa_evidence_after/17_tc59_quad_2x2_1440x1000.png)). |
| **11** | `TC-60` | Hồ sơ phím tắt `[Keys]` & khóa khi gõ chữ | **`PASS`** | Khóa phím khi `TextEdit` focus (`focused=true`, `0` thao tác cướp phím), hiện banner cảnh báo xung đột khi gán trùng ([`21_tc60`](./qa_evidence_after/21_tc60_hotkeys_modal_1280x800.png)). |
| **11** | `TC-61` | `Templates`, `Named Snapshots` & `Quarantine Recovery` | **`PASS`** | Nền tối sắc nét `fade_in(false)`, mô tả wrap gọn khung, `Close (Esc)` canh phải, lưu đủ `invert_scale`, bảo toàn `[Settings]` ([`22_tc61`](./qa_evidence_after/22_tc61_templates_modal_1280x800.png)). |
| **11** | `TC-62` | `[Export PNG]` 1 chạm `2.0x` Retina | **`PASS`** | Xuất sạch `2560×1600` (`Ok((2560, 1600))`), không giật nháy. |
| **12** | `TC-63` | Ngân sách khung hình `< 4.0 ms` (`viewport_bench`) | **`PASS`** | `4/4` bộ (`--r12`, `--r17`, `--r18`, `--r19`) đạt `overall_pass: true` (`p99` từ `0.256 ms` đến `2.720 ms`). |
| **12** | `TC-64` | Soak Test liên tục (`471` mã, `2x2`, Replay, Modals) | **`PASS`** | `elapsed = 756.7 ms`, `tex_spikes = 0`, `Watchlist = 0.24 ms`, `Replay +1 = 11.3 ms`, `Cmd+K = 13.92 ms`. |
| **12** | `TC-65` | Bảo toàn tuyệt đối dữ liệu gốc `CafeFData` & `ARCHIVE/` | **`PASS`** | `7/7` tệp `.zip` gốc khớp SHA-256 `100%`; `ARCHIVE/` và `docs/evidence/` giữ nguyên `0` sửa đổi. |

---

## 4. Số Liệu Benchmark Release (`viewport_bench` & Thực Chiến 471 Mã / 100K Nến)

Toàn bộ `viewport_bench` được thực thi trong thư mục scratch cách ly (không ghi đè `docs/evidence/` lịch sử của repo):

### 4.1. Bộ Benchmark Chuẩn `viewport_bench` (Ngân sách `< 4.0 ms`)

| Bộ Benchmark | Kịch Bản Nặng Nhất | Số Nến | Mean | Median | **`p99`** | Kết Quả |
| :--- | :--- | :---: | :---: | :---: | :---: | :---: |
| **`--r12`** | `r12_quad_2x2_conflated_ichimoku_vwap_session_cvd_50k_frame` | `200.000` | `1.340 ms` | `1.293 ms` | **`1.622 ms`** | **`PASS`** |
| **`--r17`** | `r17_quad_2x2_50k_practice_and_multistrategy_stress` | `200.000` | `2.273 ms` | `2.205 ms` | **`2.720 ms`** | **`PASS`** |
| **`--r18`** | `r18_100k_bars_all_extended_panes_conflated` | `100.000` | `0.919 ms` | `0.890 ms` | **`1.139 ms`** | **`PASS`** |
| **`--r19`** | `r19_settings_modal_open_quad_2x2_1280x800` | `7.038` | `0.347 ms` | `0.341 ms` | **`0.417 ms`** | **`PASS`** |
| **`--r19`** | `r19_100k_conflated_hollow_bullish_1280x800` | `100.000` | `0.227 ms` | `0.223 ms` | **`0.275 ms`** | **`PASS`** |

### 4.2. Benchmark Thực Chiến Đối Kháng (Trước Vá vs Sau Vá)

| Tác Vụ Thực Chiến | Trước Vá | Sau Vá | Mức Cải Thiện | Mục Tiêu |
| :--- | :---: | :---: | :---: | :---: |
| Hover `100K` nến + `2 AVWAP` (CPU/frame) | `51.60 ms` | **`0.24 ms`** (`p95 = 0.52 ms`) | **`215×` nhanh hơn** | `< 4.0 ms` (`PASS`) |
| Watchlist `471` mã UI render (CPU/frame) | `3.37 ms` | **`0.24 ms`** (`p95 = 0.34 ms`) | **`14×` nhanh hơn** | `< 1.5 ms` (`PASS`) |
| Bar Replay `+1` kèm Watchlist `471` mã mở | `139.4 ms` | **`11.3–11.5 ms`** (`~87 FPS`) | **`12.3×` nhanh hơn** | `< 16.0 ms` (`PASS`) |
| Mở lạnh `Cmd+K` lần đầu trên `471` mã | `153.1 ms` | **`13.92 ms`** | **`11.0×` nhanh hơn** | `< 20.0 ms` (`PASS`) |
| Quét toàn bộ `471` mã Watchlist lần đầu | `159.9 ms` | **`24.4 ms`** | **`6.6×` nhanh hơn** | Mượt (`PASS`) |
| `1-Click Battle` trên `471` mã CafeF | `64.2 ms` | **`66.5–67.6 ms`** | Ổn định | `< 100 ms` (`PASS`) |

---

## 5. Kiểm Toán Hình Ảnh Metal GPU Thực Tế (`docs/testing/qa_evidence_after/`)

Toàn bộ `25` ảnh chụp Metal GPU (`2560×1600` & `2880×2000`) đã được tạo mới sau bản vá và **kiểm tra trực quan từng ảnh một bằng `read_file`**:

- [`00_tc01_app_from_tmp_1280x800.png`](./qa_evidence_after/00_tc01_app_from_tmp_1280x800.png) — `.app` chạy từ `/tmp`, Top Toolbar phẳng gọn đúng `2` hàng, `Cmd+K` nằm gọn cuối Hàng 1, `%Chg` khớp `-0.04%`, `EQ: 100000 kVND`.
- [`01_tc04_data_audit_c69_1280x800.png`](./qa_evidence_after/01_tc04_data_audit_c69_1280x800.png) — `[Data Audit]` trên `C69 (HNX)`, lịch `VIETNAM_HOLIDAYS_V1 (171 dates, 2016–2030)`.
- [`07_tc11_tc14_both_docks_1280x800.png`](./qa_evidence_after/07_tc11_tc14_both_docks_1280x800.png) & [`08_tc14_both_docks_1440x1000.png`](./qa_evidence_after/08_tc14_both_docks_1440x1000.png) — `Watch (471)`, `0%` chồng đè trục giá phải (`7` pill tách bạch), `Equity: 99992.89 kVND`, `FIFO LOTS (VSD T+2 13:00)`, Top Bar & HUD cùng hiển thị `-3.97%`.
- [`09_tc16_19_vn30f_15m_indicators_1440x1000.png`](./qa_evidence_after/09_tc16_19_vn30f_15m_indicators_1440x1000.png) — Đầy đủ `ICH` (wrap 2 dòng), `ST`, `PSAR`, `VRVP`, `TPO`, `VWAP`, `PAT` đệm biên trên sạch đẹp, `4` khung dao động phụ.
- [`10_tc20_fpt_rs_cmp_vnindex_1280x800.png`](./qa_evidence_after/10_tc20_fpt_rs_cmp_vnindex_1280x800.png) — `CMP(VNINDEX)` & `RS(vs VNINDEX, 20)` bám sát `VNINDEX` thực tế `2016–2026`.
- [`12_tc22_26_drawings_inspector_textnote_1280x800.png`](./qa_evidence_after/12_tc22_26_drawings_inspector_textnote_1280x800.png) — `9` công cụ vẽ, `TextNote` tiếng Việt có dấu sắc nét, `-> Order Ticket` vô hiệu hóa khi chọn `Note`, nhãn Custom Fib chống đè.
- [`14_tc43_vn30f_dom_ladder_1280x800.png`](./qa_evidence_after/14_tc43_vn30f_dom_ladder_1280x800.png) — DOM Ladder `VN30F1M` tại `1280×800` căn giữa `Mark (1293.0)` hiển thị đồng thời `Ask + Mark + Bid`, `99426 kVND` (`Net -574.3`).
- [`15_tc51_breadth_rrg_proxybb_1440x1000.png`](./qa_evidence_after/15_tc51_breadth_rrg_proxybb_1440x1000.png) — `BREADTH · VNINDEX (471)` (`VN30(8)`, `2026-11-06`), RRG `0%` chồng chữ, Heatmap `Top 40 / 471` không trùng `FPT` giả.
- [`16_tc56_57_multistrat_0pct_slices_1440x1000.png`](./qa_evidence_after/16_tc56_57_multistrat_0pct_slices_1440x1000.png) — `MULTI-STRAT [0% UTC]` wrap 2 hàng đủ chữ, `Bench (VNINDEX) +200.37%` thật, `21` lệnh Long-only chuẩn HOSE.
- [`17_tc59_quad_2x2_1440x1000.png`](./qa_evidence_after/17_tc59_quad_2x2_1440x1000.png) — Lưới `2x2` sạch tuyệt đối (`S1: FPT`, `S2: VNINDEX` không gai `-34.5%`, `S3: VCB`, `S4: HPG` đều dùng dữ liệu thực).
- [`19_tc50_signal_catalog_101_1280x800.png`](./qa_evidence_after/19_tc50_signal_catalog_101_1280x800.png), [`22_tc61_templates_modal_1280x800.png`](./qa_evidence_after/22_tc61_templates_modal_1280x800.png), [`24_tc03_cmdk_471_1280x800.png`](./qa_evidence_after/24_tc03_cmdk_471_1280x800.png) — Toàn bộ hộp thoại nền tối sắc nét (`0%` mờ xuyên thấu), `Close (Esc)` canh phải chuẩn, có nút `EN+VI / EN / VI`.

---

## 6. Kiểm Toán Bảo Toàn Dữ Liệu Gốc (`TC-65`)

| Tệp Gốc (`/Users/mizuhara/workspace/work/CafeFData/`) | SHA-256 Sau Nghiệm Thu (Khớp `100%` Bản Gốc) | Trạng Thái |
| :--- | :--- | :---: |
| `0_Index_CafeF.INDEX.Upto.zip` | `8e5e354722266c670f70e7e858b15360d2b5d8722a48ce6398804c821a9e1294` | **`INTACT`** |
| `1_Adjusted_CafeF.SolieuGD.Upto_part1.zip` | `76ad120dc2840cfa1cbed7d7a9faf0fed2a65d7b44edc626c988e653624ba00c` | **`INTACT`** |
| `1_Adjusted_CafeF.SolieuGD.Upto_part2.zip` | `c65b454c1539ffe8396e9355743ab910fb93fe7c3513bde47edf184710ae1a5f` | **`INTACT`** |
| `1_Adjusted_CafeF.SolieuGD.Upto_part3.zip` | `715cb25771dcb760305fa774e53b4b1ad174a165e7c640b4c7f3b589a0543407` | **`INTACT`** |
| `2_Raw_CafeF.SolieuGD.Raw.Upto_part1.zip` | `ab28800e195f29f0789570fb70b02f7490f923d4053c042459be92097f062567` | **`INTACT`** |
| `2_Raw_CafeF.SolieuGD.Raw.Upto_part2.zip` | `8964e9f7e8d0fcfe9cc55033c4c01f5ecb1e2d39c8f27046973df0aa2be056dc` | **`INTACT`** |
| `2_Raw_CafeF.SolieuGD.Raw.Upto_part3.zip` | `141ea6e490ff99239b4ea6ca2f110a9237e5888cb94f756602e84bf07dbbe506` | **`INTACT`** |

- `git status --short ARCHIVE docs/evidence`: **Trống (`0` tệp thay đổi)** — bảo toàn nguyên trạng tuyệt đối `ARCHIVE/` và các bằng chứng lịch sử `docs/evidence/`.
- Không can thiệp TradingView Desktop (`DEC-006`).

---

## 7. Phán Quyết Nghiệm Thu Cuối Cùng

> **PHÁN QUYẾT: `GO FOR v1.0`**
>
> **Sumi Trading Workstation `v1.0.0`** đã khắc phục triệt để **`34/34` lỗi** (`4 P0`, `13 P1`, `14 P2`, `3 P3`), đạt **`65/65` kịch bản `PASS`** theo [`SUMI_QA_QC_MASTER_TEST_PLAN.md`](./SUMI_QA_QC_MASTER_TEST_PLAN.md), vượt qua **`274/274` bài kiểm thử tự động** với `0` cảnh báo biên dịch, đạt toàn bộ ngân sách hiệu năng `< 4.0 ms` trên Metal macOS (tăng tốc `11×–215×` trên các điểm nghẽn `471` mã / `100K` nến), và đạt độ chuẩn mực giao diện & tương tác ngang tầm TradingView Desktop ở cả `1280×800` lẫn `1440×1000`. Ứng dụng sẵn sàng phát hành chính thức **`v1.0.0`**.
