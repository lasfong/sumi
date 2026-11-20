# BÁO CÁO KIỂM THỬ ĐỘC LẬP & DANH SÁCH LỖI — SUMI TRADING WORKSTATION (`QA/QC & TRADINGVIEW UX PARITY AUDIT`)

- **Mã Báo Cáo:** `QA-REPORT-SUMI-V1.0-20261119`
- **Phiên Bản Kiểm Thử:** `sumi-workstation v0.1.0` (`Info.plist`: `1.0.0`, sau `Waves 1–18 / Milestones R01–R19`)
- **Gói Ứng Dụng Đích:** `/Users/mizuhara/workspace/sumi/dist/Sumi Workstation.app` (`arm64` macOS Metal, `codesign` ad-hoc hợp lệ, định danh `vn.sumi.workstation`)
- **Môi Trường Thực Thi:** `Apple M1` (`aarch64-macos`), `16.0 GiB` RAM, `rustc 1.99.1 (58cf8f9c3 2026-10-13)`, độ phân giải Retina `2.0x` (`2560×1600` cho `1280×800` logical & `2880×2000` cho `1440×1000` logical)
- **Tài Liệu Đối Chiếu:** [`SUMI_QA_QC_MASTER_TEST_PLAN.md`](./SUMI_QA_QC_MASTER_TEST_PLAN.md) (`12` Suite, `TC-01` .. `TC-65`), [`PROJECT_CHARTER.md`](../PROJECT_CHARTER.md), [`PRODUCT_ACCEPTANCE_CRITERIA_V3.md`](../../ARCHIVE/docs/PRODUCT_ACCEPTANCE_CRITERIA_V3.md), [`FEATURE_BACKLOG_AND_PARITY_LEDGER.md`](../FEATURE_BACKLOG_AND_PARITY_LEDGER.md)
- **Nguồn Dữ Liệu Thực Chiến (`CafeFData`):**
  - `data/raw/cafef/CafeF.SolieuGD.Adjusted.Upto06112026.csv` (`471` mã, `941.648` dòng, `2016-01-04 .. 2026-11-06`)
  - `data/raw/cafef/CafeF.SolieuGD.Raw.Upto06112026.csv` (`471` mã, `940.271` dòng, `2016-01-04 .. 2026-11-06`)
  - `data/raw/cafef/CafeF.INDEX.Upto06112026.csv` (`2` chỉ số `VNINDEX`, `HNX-INDEX`, `11.528` dòng, `2000-07-28 .. 2026-11-06`)
  - Dữ liệu nội bộ: `VNINDEX Synth (120B..100K)`, `VN30F1M 1m (5.400 nến)`, `VN30 Basket (8 mã)`, `SPY CSV (8.280 nến)`, `BTC JSON (1.344 nến)`.
- **Trạng Thái Bảo Toàn Mã Nguồn & Dữ Liệu Gốc:** `0` thay đổi trong `src/`, `Cargo.toml`, `ARCHIVE/`, `docs/evidence/` và `/Users/mizuhara/workspace/work/CafeFData/*.zip`.

---

## 1. Tóm Tắt Điều Hành & Phán Quyết Nghiệm Thu (`Executive Verdict`)

### Phán Quyết Tổng Thể: **`NO-GO / CHẶN PHÁT HÀNH v1.0`** (Cần Bản Vá `Patch R19.1` & `Patch R19.2`)

Mặc dù kiến trúc lõi dựng hình Metal GPU của **Sumi Trading Workstation** rất ấn tượng (`267/267` bài test đơn vị PASS; `4/4` bộ benchmark `viewport_bench --r12/--r17/--r18/--r19` đạt `overall_pass: true` với `p99 <= 2.921 ms`; nạp trọn bộ `941.648` dòng `CafeF Adj` trong `415–440 ms`; Bar Replay `DEF-001` trên `1D -> 1W` và `2x1` không rò rỉ tương lai), kiểm thử đối kháng độc lập ở cả 3 lớp đã phát hiện **`34` lỗi thực chứng** gồm **`4` lỗi `P0 — Blocker`** và **`13` lỗi `P1 — Critical`** vi phạm trực tiếp điều kiện chặn phát hành trong `SUMI_QA_QC_MASTER_TEST_PLAN.md`:

| Chỉ Số Tổng Hợp | Kết Quả Thực Tế | Ghi Chú |
| :--- | :---: | :--- |
| **Tổng kịch bản kiểm thử (`TC-01` .. `TC-65`)** | **`65`** | Bao phủ đủ `12/12` Suite |
| **Đạt (`PASS`)** | **`29` (`44.6%`)** | Nền tảng GPU, zoom neo chuột, nến rỗng, Replay `1D->1W`, `Undo/Redo`, `Export PNG`, `RunManifest`, bảo toàn dữ liệu gốc |
| **Không đạt (`FAIL`)** | **`36` (`55.4%`)** | Chịu tác động dây chuyền từ các lỗi lõi ở dữ liệu CafeF, Benchmark giả lập, `Paper Trading / VN30F1M / T+2`, `Alert / Cutoff`, và UI Overlap |
| **Lỗi `P0 — Blocker`** | **`4`** | `BUG-001` .. `BUG-004` (**Chặn phát hành tuyệt đối**) |
| **Lỗi `P1 — Critical`** | **`13`** | `BUG-005` .. `BUG-017` (**Chặn phát hành**) |
| **Lỗi `P2 — Major (UX / Parity)`** | **`14`** | `BUG-018` .. `BUG-031` (**Cần vá trước `v1.0`**) |
| **Lỗi `P3 — Minor / Cosmetic`** | **`3`** | `BUG-032` .. `BUG-034` (Cải tiến hoàn thiện) |
| **Unit Tests (`cargo test`)** | **`267 PASS / 0 FAIL`** | Chạy sạch trong `4.34 s` |
| **Release Frame Budget (`viewport_bench`)** | **`4/4 PASS (< 4.0 ms)`** | `--r12`: `p99 <= 2.000 ms`, `--r17`: `2.921 ms`, `--r18`: `1.795 ms`, `--r19`: `0.513 ms` |
| **Bảo toàn tệp gốc `CafeFData/*.zip` (`TC-65`)** | **`7/7 SHA-256 KHỚP 100%`** | `0` byte bị sửa đổi |

### Những Điểm Rất Mạnh Đã Đạt Hoặc Vượt Chuẩn TradingView
1. **Hiệu năng Metal GPU lõi vượt trội:** `100.000` nến gộp `32x` với `HollowBullish` + `TextNote` + `TrendLine` chỉ mất `0.275 ms` trung bình (`p99 = 0.368 ms`); `2x2` `200K` nến kèm Ichimoku + CVD chỉ mất `1.417 ms` (`p99 = 2.000 ms`).
2. **Đồ họa kỹ thuật sắc nét:** Nến `Hollow Bullish` có râu dừng chính xác đúng mép thân nến ([`04_tc12_fpt_hollow_bullish_1280x800.png`](./qa_evidence/04_tc12_fpt_hollow_bullish_1280x800.png)); `Cursor-Anchored Zoom` `20` lần sai lệch `0.000000000` thanh; `TrendLine` đảo chiều kéo dài đúng hướng; `TextNote` UTF-8 tiếng Việt nét và chặn xuyên thấu lên `Rect` bên dưới; `Export PNG` `2560×1600` (`2.0x` Retina) một chạm sạch sẽ.
3. **Nền tảng Bar Replay `DEF-001` cốt lõi chuẩn xác:** Cắt giữa tuần `1D -> 1W` trên biểu đồ chính và lưới `2x1` tổng hợp đúng nến tuần chưa đóng ([`13_tc32_multichart_2x1_replay_1280x800.png`](./qa_evidence/13_tc32_multichart_2x1_replay_1280x800.png)); phân kỳ `+3B` bất biến (`0` sai lệch khi sửa tương lai); tua lùi `<< -1` khôi phục bit-exact (`eq_diff = 0.000000000`); `Blind +120b` che tên tín hiệu đúng chuẩn; `Reset Practice` giữ nguyên con trỏ và nét vẽ; lịch lễ `VIETNAM_HOLIDAYS_V1` giai đoạn `2018–2026` khớp hoàn hảo cả Tết `2026` và `30/04–01/05`.

### 5 Rủi Ro Lớn Nhất Khiến Ứng Dụng Chưa Thể Phát Hành `v1.0`
1. **Sai lệch tài chính nghiêm trọng trong Paper Trading & Phái sinh (`BUG-001`, `BUG-002`, `BUG-004`, `BUG-008`):** `VN30F1M` hạch toán sai `100x` (`-5.74` thay vì `-418.94 kVND`), DOM tính đòn bẩy kép `34.6x` (`26` hợp đồng thay vì tối đa `4`), cổ phiếu VN không trừ thuế bán `0.10%`, cho phép bán khống trái phép trên `HOSE`, và lõi `TradingSession` không tự cưỡng chế khóa lô `T+2`.
2. **Mất trắng phiên `CafeF` khi khởi động lại (`BUG-003`) & phá hủy bộ `471` mã khi tìm bằng `Cmd+K` (`BUG-007`):** Đóng mở lại app tự xóa phiên `CafeF` rơi về `VNINDEX Synth 120 nến`; gõ `VCB`/`FPT`/`HPG`/`SSI` trong `Cmd+K` rồi Enter ghi đè toàn bộ `471` mã thật bằng `VN30 Basket` giả lập 180 nến.
3. **Âm thầm thay `VNINDEX` thật bằng đường sóng sin giả lập (`BUG-006`):** `RS vs VNINDEX`, `Cmp:VNINDEX`, `RRG` và `Multi-Strategy 0% UTC` dùng đường giả lập `+93.71%, MaxDD -3.6%` thay vì `VNINDEX` thật `+200.37%` mà không hề cảnh báo người dùng.
4. **Cắt xóa oan `23.550` phiên giao dịch hợp lệ của `CafeFData` (`BUG-005`) & nhận nhầm ~250 mã `HNX`/`UPCOM` sang `HOSE` (`BUG-021`):** Làm hỏng lịch sử `421/471` mã (như `RCL` mất `40%` lịch sử, `KSV` mất `113` phiên, `HNX-INDEX` mất `462` phiên) và chỉ còn `12/471` mã đạt chuẩn `Ready`.
5. **Chưa đạt chuẩn mượt & bố cục TradingView trên dữ liệu thực `471` mã (`BUG-018`, `BUG-019`, `BUG-020`, `BUG-022`, `BUG-023`, `BUG-024`):** Nhãn trục giá `Alert` đè lên `Close`/`Entry`/`TP`, `RRG` chồng `483` nhãn thành mảng trắng đặc, hộp thoại `Templates` bị mờ xuyên thấu, `2x2` mặc định nạp dữ liệu hỏng `-34.5%`, và bước `+1` Replay với Watchlist `471` mã bị khựng **`139.4 ms`/bước (~7 FPS)**.

---

## 2. Bảng Kết Quả Thực Thi Toàn Bộ `65` Kịch Bản (`TC-01` .. `TC-65`)

| Suite | Mã TC | Kịch Bản Kiểm Thử | Kết Quả | Lỗi Liên Quan & Tóm Tắt Bằng Chứng Thực Đo |
| :--- | :---: | :--- | :---: | :--- |
| **01** | `TC-01` | Khởi động `.app` từ `/tmp` & môi trường | **`PASS`** | Chạy sạch `2560×1600`, `codesign` hợp lệ (`app_from_tmp_default.png`). *Lưu ý:* copy `.app` ra khỏi repo mất nút nhanh CafeF (`BUG-032`); toolbar rớt dòng (`BUG-023`); default synth lệch ngày nghỉ (`BUG-025`). |
| **01** | `TC-02` | Nạp `CafeF Adj (471 mã)` `< 1 s` | **`PASS`** | Nạp `941.648` dòng / `471` mã trong **`414.6–439.9 ms`** (1 lần quét), tự chọn `FPT` (`2.702` nến), không treo UI. |
| **01** | `TC-03` | `Cmd+K` & tìm đủ `471` mã `A..Y` | **`FAIL`** | Menu toolbar đủ `471` mã (`0.17–0.31 ms`), nhưng `Cmd+K` gõ `VCB/HPG/SSI` xếp `VN30 Basket` giả lập lên đầu, Enter phá hủy `471` mã (`BUG-007`), mở lần đầu khựng `153.1 ms` (`BUG-020`), không xóa filter (`BUG-023`). |
| **01** | `TC-04` | `Split Norm` & `[Data Audit]` (`C69/HPX/KSV/NDN`) | **`FAIL`** | `Split Norm` chạy (`411/199/204/515` dòng) nhưng Rule 5 xóa oan `23.550` phiên phẳng (`BUG-005`), phân loại nhầm `C69/KSV/NDN` sang `HOSE` (`56–176` false band breaches), thiếu lịch lễ `2016–2017` (`BUG-021`). |
| **01** | `TC-05` | `BVS/API/APS` (KRX `High=0`) & `CafeF Index` | **`FAIL`** | Chặn sạch nến sập về `0` (`min_low > 0`), nhưng Rule 5 xóa oan `19/80/109` phiên `BVS/API/APS` và `462` phiên `HNX-INDEX` (`BUG-005`), nhận nhầm `HOSE` (`BUG-021`). File gốc chỉ có `2` chỉ số (`VNINDEX`, `HNX-INDEX`). |
| **01** | `TC-06` | Đường dẫn lỗi, tệp rỗng, bỏ tick `Split Norm` | **`FAIL`** | Đường dẫn sai giữ nguyên biểu đồ, nhưng bỏ tick `CafeF Split Norm` **không từ chối tệp lỗi OHLC** mà cách ly âm thầm và xóa biểu đồ `FPT` đang mở (`BUG-012`); lỗi `TimestampUnit::Auto` năm `1970` (`BUG-013`), `QuarantineRow` không đồng nhất (`BUG-031`). |
| **02** | `TC-07` | Cursor-Anchored Zoom `20` lần | **`PASS`** | Độ trôi ngang tối đa **`0.000000000`** thanh sau `20` lần phóng/thu liên tiếp. |
| **02** | `TC-08` | Pan, Kinetic & Future Whitespace (`Right Offset`) | **`PASS`** | Quán tính giảm tốc mượt, giữ vùng trắng tương lai ổn định không giật ngược. |
| **02** | `TC-09` | Kéo trục giá, double-click `Auto`, kéo trục thời gian | **`PASS`** | Đúng chuẩn TradingView (`src/chart.rs` & unit tests `267/267` pass). |
| **02** | `TC-10` | `Linear / Log / % / Idx100` & `Inverted` | **`PASS`** | Hiển thị chuẩn trên `FPT` & `VNINDEX` (`02_tc10`, `03_tc10`). *Lưu ý ca biên:* `close <= 0 / NaN` ở nến đầu làm phẳng biểu đồ `%`/`Idx100` (`BUG-027`); `0.0` hiện `+0.00%` (`BUG-033`). |
| **02** | `TC-11` | Chống đè nhãn trục giá phải (`Anti-Collision`) | **`FAIL`** | Nhãn `Alert (51.05)` và `Drawing` bỏ qua bộ chống đè, đè trực tiếp lên `Close (50.80)`, `Entry (50.81)`, `TP (51.31)` và thanh `LONG 200` PnL ([`07_tc11_tc14`](./qa_evidence/07_tc11_tc14_both_docks_1280x800.png), `BUG-018`). |
| **02** | `TC-12` | `4` bảng màu nến & `Hollow Bullish` | **`PASS`** | Đổi tức thì `0` frame trễ; râu nến `Hollow Bullish` dừng chuẩn đúng mép thân nến ([`04_tc12`](./qa_evidence/04_tc12_fpt_hollow_bullish_1280x800.png)). |
| **02** | `TC-13` | `Conflation` `100K` nến (`Fit`) | **`PASS`** | `conflation_factor = 32`, giữ toàn vẹn cực trị `[1207.90, 20088.09]`, thời gian vẽ `0.275 ms` (`p99 = 0.368 ms`). *Lưu ý:* `NaN` volume ca biên (`BUG-027`). |
| **02** | `TC-14` | Bố cục `1280×800` & `1440×1000` (mở cả 2 dock) | **`FAIL`** | Ở `1280×800`, thanh công cụ rớt `3–4` hàng (`149px+`), nút rơi mồ côi (`BUG-023`), DOM đẩy giá mark & Bid ra ngoài khung nhìn ([`14_tc43`](./qa_evidence/14_tc43_vn30f_dom_ladder_1280x800.png), `BUG-024`), đơn vị tiền tệ cứng `$` (`BUG-023`). |
| **03** | `TC-15` | Gộp đa khung `1m..1W` (`VN30F1M` & `FPT`) | **`FAIL`** | Gộp OHLCV & khóa khung nhỏ đúng chuẩn (`5400 -> 5`), nhưng preset `VN30F1M 1m` sinh `510` nến vào ngày lễ Quốc khánh `01–02/09/2026` và Data Audit bỏ sót (`hol_wknd = 0`, `BUG-025`). |
| **03** | `TC-16` | Chỉ báo `Overlays` (`SMA/EMA/BB/Ichimoku/ST/PSAR/PAT`) | **`FAIL`** | Kumo Twist & `Chikou (-26)` đẹp ([`09_tc16_19`](./qa_evidence/09_tc16_19_vn30f_15m_indicators_1440x1000.png)), nhưng `IndicatorCache` phục vụ dữ liệu cũ sau khi sửa tham số lúc tắt (`SMA(50)` vẽ giá trị `SMA(20) = 169.50`, `BUG-014`); lỗi `NaN`/shimmer (`BUG-027`); `ICH` cắt cụt & `PAT` chạm viền (`BUG-032`). |
| **03** | `TC-17` | `VRVP` (`70% VA/POC`) & `TPO` (`IB/Single Prints/NPOC`) | **`PASS`** | Vẽ chính xác và cập nhật động trên `VN30F1M 15m` ([`09_tc16_19`](./qa_evidence/09_tc16_19_vn30f_15m_indicators_1440x1000.png)). |
| **03** | `TC-18` | Multi-Interval `VWAP` (`±1σ, ±2σ`) | **`PASS`** | Reset chuẩn theo chu kỳ, `σ = 0` tại nến đầu, Welford bền số học (lưu ý cầu nối `NaN` tại `BUG-027`). |
| **03** | `TC-19` | Khung dao động phụ (`RSI/MACD/ATR/ADX/MFI/CVD...`) | **`FAIL`** | Đồng bộ thời gian & kéo splitter `5px` (`min 44px`) tốt, nhưng dính lỗi `IndicatorCache` stale (`BUG-014`), `MFI/CCI` sinh `NaN/Inf` (`BUG-027`), `CVD` thiếu hậu tố `T` (`BUG-033`). |
| **03** | `TC-20` | `RS vs VNINDEX` & `Cmp:VNINDEX` trên `FPT` | **`FAIL`** | Dùng `VNINDEX` sóng sin giả lập (`1294.72 -> 2508.04`) thay vì `VNINDEX` thật (`574.41 -> 1725.36`) mà không cảnh báo ([`10_tc20`](./qa_evidence/10_tc20_fpt_rs_cmp_vnindex_1280x800.png), `BUG-006`). |
| **03** | `TC-21` | Dữ liệu ngắn (`BCG 7`, `DSH 10`, `ECO 13`, `TCD 20` nến) | **`PASS`** | Bật toàn bộ chỉ báo không panic/crash, không sập trục về `0.0`. |
| **04** | `TC-22` | Vẽ đủ `9` công cụ, khung phụ & hủy `Escape` | **`FAIL`** | Thao tác vẽ & `Escape` tốt, nhưng `AVWAP` bỏ qua `R11RenderMemo` và tính lại toàn chuỗi `2` lần/khung hình, đẩy CPU lên `3.14–3.49 ms` với chỉ `1` AVWAP trên `100K` nến (`BUG-015`). |
| **04** | `TC-23` | Nam châm bắt điểm `Magnet OHLC` | **`PASS`** | Hít đúng `O/H/L/C` gần nhất khi bật, đặt tự do khi tắt. |
| **04** | `TC-24` | Custom Fibonacci (`ICT OTE 9` / `Full 13`, `Undo` nguyên tử) | **`FAIL`** | Công thức mức `0.705`, `-0.272`, `0.79` & `Undo` `1` bước đúng, nhưng nhãn các mức Fib chồng đè lên nhau và đè lên nhãn `SMA/EMA/FRVP` ([`12_tc22_26`](./qa_evidence/12_tc22_26_drawings_inspector_textnote_1280x800.png), `BUG-026`). |
| **04** | `TC-25` | `TrendLine` `Extend Left/Right` khi vẽ ngược `A1 > A2` | **`PASS`** | `Extend Right` luôn kéo về tương lai, `Extend Left` luôn kéo về quá khứ ([`12_tc22_26`](./qa_evidence/12_tc22_26_drawings_inspector_textnote_1280x800.png)). |
| **04** | `TC-26` | `TextNote` UTF-8 & chống xuyên thấu Click-Through | **`FAIL`** | Tiếng Việt sắc nét, hit-test ưu tiên `Note` trên `Rect` (`hit=Some((4, Body, Stroke))`), nhưng `wrap_text_note_lines(34, 8)` cắt cụt ghi chú hợp lệ `280` ký tự tại `<= 272` (`BUG-026`); `-> Order Ticket` sáng vô nghĩa (`BUG-032`). |
| **04** | `TC-27` | Bảo toàn tọa độ thời gian khi đổi `1D <-> 1W` | **`PASS`** | `drift_bar = 0.000000`, giữ nguyên timestamp gốc `1789344000`. |
| **04** | `TC-28` | `Lock`, `Visible` & `Undo/Redo` `64` bước | **`PASS`** | Khóa kéo/xóa đúng, chọn lại nét ẩn trong `LAYERS`, `Undo/Redo` phục hồi cả `Clear`. |
| **04** | `TC-29` | Cách ly nét vẽ theo từng mã (`FPT <-> VCB`) | **`PASS`** | `FPT: 9 -> VCB: 0 -> FPT: 9`, cách ly tuyệt đối. |
| **05** | `TC-30` | `<< Cut Bar`, `+1`, `-1`, `Play`, `\|<< Start` | **`PASS`** | Cắt nến sạch, hiển thị rõ ngày & `Base cursor / Total`. |
| **05** | `TC-31` | `DEF-001` chống lộ nến khung lớn (`1D -> 1W` giữa tuần) | **`FAIL`** | Biểu đồ chính `1D -> 1W` chuẩn (`H=45.16, C=44.60, LEAK=false`), nhưng cutoff Backtest/Scanner không đơn điệu (tụt nến lúc `08:00 UTC`, `BUG-010`) và `Alerts` đánh giá nến `1W` chưa đóng sinh cảnh báo ma (`BUG-011`). |
| **05** | `TC-32` | `DEF-001` Multi-Chart `2x1`/`2x2` Replay | **`PASS`** | Ô phụ `FPT 1W` khớp đúng nến chưa đóng `2023-01-02 (H=45.16, C=44.60)` ([`13_tc32`](./qa_evidence/13_tc32_multichart_2x1_replay_1280x800.png)). |
| **05** | `TC-33` | `DEF-001` ẩn nét vẽ & cảnh báo tương lai trong Replay | **`FAIL`** | Nét vẽ tương lai ẩn đúng (`visible_in_past = false`), nhưng `AlertRule::is_causally_visible` để lộ các cảnh báo Live (`created_at_timestamp == None`) khi tua về quá khứ (`BUG-028`). |
| **05** | `TC-34` | `DEF-001` Watchlist / `Sig` / `Breadth` / `Audit` khóa `<= T` | **`FAIL`** | `Breadth/RRG` cắt trùng 2 lần mất 1 phiên hợp lệ (`BUG-029`), trộn `13` mã giả lập lệch ngày `2026-11-13` (`BUG-020`), `ProxyBB` dùng cutoff không đơn điệu (`BUG-010`). |
| **05** | `TC-35` | `DEF-001` Phân kỳ `+3B` bất biến | **`PASS`** | `future_mutation_mismatches = 0` khi biến đổi toàn bộ nến tương lai. |
| **05** | `TC-36` | Tua lùi `<< -1` khôi phục bit-exact vị thế & cảnh báo | **`PASS`** | Hoàn tác chốt lời & nạp lại cảnh báo `Once` với sai số vốn **`0.000000000`**. |
| **05** | `TC-37` | Khôi phục phiên khi khởi động lại (`Cmd+Q`) | **`FAIL`** | **`P0` (`BUG-003`)**: `restore_persisted_session` dùng `RejectDataset`, nạp lỗi file `CafeF` có dòng quarantine và **xóa sạch Replay, nét vẽ, vị thế**, rơi về `VNINDEX Synth 120 nến`. |
| **06** | `TC-38` | `Pos R:R <-> Order Ticket`, lô `100`, thuế bán `0.10%` | **`FAIL`** | **`P0` (`BUG-001`, `BUG-002`)**: Paper Trading bỏ quên hoàn toàn thuế bán `0.10%` (`tax_sell_bps`), thiếu hệ số nhân/ký quỹ `VN30F1M`, và DOM nhân đôi đòn bẩy `34.6x`. |
| **06** | `TC-39` | Khớp `Market/Limit/Stop`, Gap & bi quan `SL` trước `TP` | **`FAIL`** | Vi phạm `DEF-005`: lệnh chờ khớp bằng Gap mở cửa Phase A chốt lời `TP=105.0` trước `SL=98.0` (`BUG-009`); kiểm tra margin scale-in dùng giá vốn cũ cho phép đòn bẩy `1.5–1.8x` (`BUG-017`). |
| **06** | `TC-40` | Khóa lô `T+2` (`13:00 ICT`) FIFO trên `HOSE` | **`FAIL`** | **`P0` (`BUG-004`)**: Lõi `TradingSession` không cưỡng chế `enforce_vn_t2_lot_lock` khi gọi `SubmitOrder(Market Sell)`/`Flatten` (`Ok(2)` ở `T+1`), UI `Flatten` chặn bán phần `200` cp đã về; lỗi `1D > 06:00 UTC`/`1W`/`US T+1` (`BUG-017`). |
| **06** | `TC-41` | `T+2` xuyên Tết & lễ `30/04–01/05` (`VIETNAM_HOLIDAYS_V1`) | **`FAIL`** | Giai đoạn `2018–2026` chuẩn (Tết `2026`: `2026-02-24 13:00 ICT`; `30/04`: `2026-05-05 13:00 ICT`), nhưng `CafeF 2016–2017` thiếu lịch lễ mở khóa sớm `7` ngày giữa Tết `2016` (`2016-02-10` vs `2016-02-17`, `BUG-021`); sau `2030` thiếu ngày lễ thứ 2 (`BUG-034`). |
| **06** | `TC-42` | `SL`/`TP` tự động khi có cổ phiếu khóa `T+2` & tắt Lock | **`FAIL`** | `Auto-SL` khi `Lock=ON` thoát đúng `200` cp đã về & giữ `300` cp khóa, nhưng tắt `Lock=OFF` mở luôn bán khống trái phép trên `HOSE` (`BUG-008`); đổi `Venue` ở `[Settings]` lệch `T+2` (`BUG-030`). |
| **06** | `TC-43` | DOM Ladder 21 bậc (`HOSE 10.00/50.00`, `VN30F1M`) | **`FAIL`** | **`P0` (`BUG-001`, `BUG-002`, `BUG-008`, `BUG-024`)**: `VN30F1M` PnL sai `100x`; DOM cấp `26` HĐ thay vì `4`; lỗi bước giá `10.00` trùng `CEIL`/`mark` & đặt nhầm `Limit`; bị đẩy Bid xuống dưới màn hình ở `1280×800` ([`14_tc43`](./qa_evidence/14_tc43_vn30f_dom_ladder_1280x800.png)). |
| **07** | `TC-44` | Tính bất biến kỷ luật `SL Adh %` | **`FAIL`** | `PracticeScoreboard` đúng `50% (1/2)`, nhưng `TradeJournalStore::checklist.stop_and_lot_sized` bị thổi phồng `true` và chế `r_multiple` giả khi thêm `SL > Entry` sau vào lệnh (`BUG-030`). |
| **07** | `TC-45` | `Decision Note` & `Session Debrief` | **`FAIL`** | `SessionDebriefReport::build` trộn KPI tổng không cắt `cutoff` với bảng chi tiết có cắt `cutoff` (`BUG-030`); mâu thuẫn làm tròn `"80% (< 80% Target)"` (`BUG-034`). |
| **07** | `TC-46` | Nút `Reset Practice` 1 chạm | **`PASS`** | Reset sạch lệnh/vốn/điểm về `0`, giữ nguyên `cursor = 2004` và `4` nét vẽ. |
| **07** | `TC-47` | Lưu/Ghim `PracticeSessionStore` & Xuất `CSV/MD` | **`FAIL`** | Chống CSV Formula Injection đạt (`safe=true`), nhưng khôi phục phiên lệch `enforce_vn_t2_lot_lock`, lưu phiên sau tua lùi rò ghi chú tương lai (`BUG-030`), nhãn `>80B` trùng slot & MD không lọc newline (`BUG-034`). |
| **08** | `TC-48` | Quét `471` mã Watchlist & ảo hóa `show_rows` | **`FAIL`** | Quét `471` mã hết `159.9 ms`, nhưng UI **không dùng `show_rows`** (`3.07–3.37 ms`/frame), trộn trùng `13` mã giả lập (`2` dòng `FPT`), và bước `+1` Replay bị khựng **`139.4 ms`** (`BUG-020`). |
| **08** | `TC-49` | Luồng `Scanner -> Replay` (`Blind +120b`) | **`PASS`** | Ẩn kín tên tín hiệu tại điểm cắt (`revealed_at_cut = false`) và tự mở đúng sau `120` thanh (`revealed_after_120 = true`). |
| **08** | `TC-50` | `Signal Catalog 101` & Điểm hợp lưu | **`PASS`** | Đủ `101` tín hiệu, ghi rõ `delay=3B`, `OHLCV_BAR_CLOSE_PROXY`, confluence `[-100, +100]`. *Lưu ý nhỏ:* không có nút bật/tắt EN/VI (`BUG-032`), default `rsi_overbought_cross = 30.0` (`BUG-034`). |
| **08** | `TC-51` | `Breadth`, `RRG` 4 góc & `ProxyBB` (`T03..T200`) | **`FAIL`** | `AT05` volume=0 trả `None` đúng, nhưng `RRG` chồng `483` nhãn thành mảng trắng không đọc được ([`15_tc51`](./qa_evidence/15_tc51_breadth_rrg_proxybb_1440x1000.png), `BUG-020`), đếm trùng `VN30(16)`, ngày `2026-11-13`, trộn `FPT` giả lập & `VNINDEX` giả lập (`BUG-006`). |
| **09** | `TC-52` | Cảnh báo Giá/RSI/MACD/`TrendLine` nghiêng/`cooldown` | **`FAIL`** | `TrendLine` nghiêng + cooldown chạy tốt, nhưng `RSI`/`MACD`/`DrawingLine` nổ hồi tố trên chính nến đặt lệnh trên `SPY 14:30 UTC`, nổ ma trên `1W` chưa đóng (`BUG-011`), nhận nhầm mã US sang `HOSE` (`BUG-010`). |
| **09** | `TC-53` | Xuất lịch sử cảnh báo `CSV` / `JSON` | **`PASS`** | Chuẩn RFC-4180 CSV & RFC-8259 JSON, chỉ xuất các cảnh báo `<=` mốc Replay. |
| **10** | `TC-54` | Backtest đơn `Open[T+1]` trên `FPT` (`2016–2026`) | **`FAIL`** | Khớp `Open[T+1]` chuẩn (`open_t1_match = true`), nhưng cho phép bán khống trái phép `22` lệnh Short trên `FPT` (`HOSE`, `BUG-008`), lỗi cutoff `08:00 UTC` (`BUG-010`), không lọc giá `NaN`/`<=0` khi thoát lệnh (`BUG-029`). |
| **10** | `TC-55` | `Custom Signal Rule Builder` (`AND`/`OR`) | **`PASS`** | Đánh giá DNF `OR-of-AND` chuẩn, chặn warmup, chạy `113` lệnh sạch trên `FPT`. |
| **10** | `TC-56` | `1-Click Battle` & `Multi-Strategy [0% UTC]` | **`FAIL`** | `Battle 471` mã nhanh (`64.2 ms`), nhưng `Bench (VNINDEX)` hiển thị đường sóng sin giả lập `+93.71%, MaxDD -3.6%` thay vì `VNINDEX` thật `+200.37%` ([`16_tc56_57`](./qa_evidence/16_tc56_57_multistrat_0pct_slices_1440x1000.png), `BUG-006`); legend dưới cắt cụt (`BUG-032`). |
| **10** | `TC-57` | `Tearsheet` (`OutlierImpact`, `DD`, `Yearly`, `Manifest`) | **`FAIL`** | `RunManifest` tất định (`17608082022104202897`), nhưng `expectancy_ex_top_3_winners` chia sai mẫu số lệch `3x` (`-300.0` vs `-100.0`, `BUG-016`) và `Yearly` báo `-100%` giả sau phá sản (`BUG-029`). |
| **10** | `TC-58` | `WFO`, `Monte Carlo` `1.000` & `VN30-PIT` Portfolio | **`FAIL`** | `WFO` (`1.2 ms`) & `MC` tốt, nhưng `VN30-PIT` trên `CafeF Adj` lờ bộ lọc `VN30 Only`, lệch ngày `W1` làm mất P&L tuần `UniverseRemoval` (`BUG-029`), tạo đỉnh vốn ảo (`BUG-034`). |
| **11** | `TC-59` | Lưới đa biểu đồ `2x2` (`Sync X` / `Sync T`) | **`FAIL`** | Crosshair đồng bộ biến mất ở vùng trắng tương lai; `Sync T` méo trên `85%` whitespace; `2x2` mặc định nạp `S2: VNINDEX` bị gai `-34.5%` và `S3: VCB` giả lập trống `95%` màn hình ([`17_tc59`](./qa_evidence/17_tc59_quad_2x2_1440x1000.png), `BUG-022`). |
| **11** | `TC-60` | Hồ sơ phím tắt `[Keys]` & khóa khi gõ chữ | **`PASS`** | Khóa phím hệ thống, gỡ xung đột khi gán trùng, chặn cướp phím khi `TextEdit` focus (`src/app.rs:11782`, `src/chart.rs:10415`). *Lưu ý nhỏ:* mất thông báo xung đột ở UI (`BUG-032`). |
| **11** | `TC-61` | `Templates`, `Named Snapshots` & `Quarantine Recovery` | **`FAIL`** | Bảo toàn `[Settings]` & chặn file hỏng đúng chuẩn, nhưng hộp thoại `Templates` bị **mờ trong suốt xuyên thấu** (thiếu `.fade_in(false)`) & lệch nút `Close (Esc)` ([`22_tc61`](./qa_evidence/22_tc61_templates_modal_1280x800.png), `BUG-019`); thiếu `invert_scale` (`BUG-033`). |
| **11** | `TC-62` | `[Export PNG]` 1 chạm `2.0x` Retina | **`PASS`** | Xuất sạch `2560×1600`, không giật nháy, không tạo snapshot rác. |
| **12** | `TC-63` | Ngân sách khung hình `< 4.0 ms` (`viewport_bench`) | **`PASS`** | `4/4` bộ benchmark đạt `overall_pass: true` (`p99`: `--r12 2.000 ms`, `--r17 2.921 ms`, `--r18 1.795 ms`, `--r19 0.513 ms`). *Cảnh báo ngoài bench:* `BUG-015`, `BUG-020`. |
| **12** | `TC-64` | Soak Test liên tục (`471` mã, `2x2`, Replay, Modals) | **`FAIL`** | GPU texture ổn định (`tex_spikes = 0`, `696.7 ms`), không crash, nhưng Replay `+1` kèm Watchlist `471` mã đứng hình **`139.4 ms`/bước (~7 FPS)** và mở `Cmd+K` khựng **`153.1 ms`** (`BUG-020`). |
| **12** | `TC-65` | Bảo toàn tuyệt đối dữ liệu gốc `CafeFData` & `ARCHIVE/` | **`PASS`** | `7/7` tệp `.zip` gốc khớp SHA-256 `100%`; `git status` xác nhận `0` sửa đổi ở `src/`, `ARCHIVE/`, `docs/evidence/`. |

---

## 3. Danh Sách Lỗi Chi Tiết (`[BUG-001]` .. `[BUG-034]`)

### Nhóm `P0 — Blocker` (Chặn Phát Hành Tuyệt Đối — `4` Lỗi)

---

### [BUG-001] Paper Trading bỏ sót hệ số nhân `100 kVND/điểm`, ký quỹ `17%` của `VN30F1M` và thuế bán `0.10%` cổ phiếu VN
- **Mức độ nghiêm trọng (Severity):** `P0`
- **Phân hệ liên quan:** `Paper Trading / T+2`
- **Mã kịch bản (Test Case ID):** `TC-43`, `TC-38`
- **Nguồn dữ liệu & Mã chứng khoán:** `VN30F1M 1m` (`VN30F1M · 15m`) và `CafeF Adj (471 mã)` (`FPT · 1D`)
- **Các bước tái hiện chính xác (Steps to Reproduce):**
  1. Nạp nguồn `VN30F1M 1m`, mở `Paper Trading` hoặc `DOM Ladder`.
  2. Mua `1` hợp đồng `VN30F1M` ở mức `1297.18` và đóng vị thế ở `1292.99` (lỗ `-4.19` điểm chỉ số).
  3. Chuyển sang `FPT` (`HOSE`), mua `100` cổ phiếu và bán đóng toàn bộ vị thế; kiểm tra cột `Fee/Tax` và `Net PnL`.
- **Kết quả thực tế (Actual Result):**
  - Với `VN30F1M`: `TradingSession` hạch toán `net_pnl = -5.74` (chỉ nhân `1x` điểm số rồi trừ phí) thay vì `-418.94 kVND` (`-418.940 VND`), lệch **`100x` theo kVND (`100.000x` theo VND)** so với công thức trong `src/journal/planner.rs`. Sức mua `VN30F1M` cũng kiểm tra theo giá danh nghĩa `1x` (`1300`) thay vì ký quỹ ban đầu `17% × 100 = 22.100 kVND/HĐ`.
  - Với `FPT` (`HOSE`): lệnh bán chỉ trừ phí giao dịch (`fee_bps`), **thuế chuyển nhượng bán `0.10%` bằng `0`** (`tax_sell_bps` không tồn tại trong `AccountConfig` của `TradingSession`).
- **Kết quả kỳ vọng / Chuẩn TradingView hoặc Quy chế VN (Expected Result):**
  - `VN30F1M` theo chuẩn HNX/VSD phải nhân hệ số `contract_multiplier = 100 kVND/điểm` (`100.000 VND/điểm`) cho cả PnL thực tế, PnL chưa thực hiện và phí/ký quỹ ban đầu `17%`.
  - Cổ phiếu cơ sở VN (`HOSE`/`HNX`/`UPCOM`) phải trừ đủ thuế bán `0.10%` trên giá trị bán khi chốt lệnh, đồng nhất 100% giữa `Risk Planner` và `TradingSession`.
- **Bằng chứng đính kèm (Evidence):**
  - Ảnh chụp: [`docs/testing/qa_evidence/14_tc43_vn30f_dom_ladder_1280x800.png`](./qa_evidence/14_tc43_vn30f_dom_ladder_1280x800.png) (hiển thị `$99994` và `Net -5.7` cho lỗ `-4.19` điểm `VN30F1M`).
  - Telemetry đối kháng: `entry=1297.18 exit=1292.99 dPts=-4.19 actual_net_pnl=-5.74 (100x_kVND=-418.94, 1x=-4.19)`.
  - Mã nguồn gốc: `src/trading/orders.rs:30-50, 391`; `src/trading/portfolio.rs:46-51, 1133-1188, 1213, 1374-1375`; `src/app.rs:3816-3833`.

---

### [BUG-002] Lỗi nhân đôi đòn bẩy (`34.6x`) khi định cỡ khối lượng `VN30F1M` trên DOM Ladder
- **Mức độ nghiêm trọng (Severity):** `P0`
- **Phân hệ liên quan:** `Paper Trading / T+2`
- **Mã kịch bản (Test Case ID):** `TC-43`, `TC-38`
- **Nguồn dữ liệu & Mã chứng khoán:** `VN30F1M 1m` (`VN30F1M`, vốn `100.000 kVND`, giá `1.300`)
- **Các bước tái hiện chính xác (Steps to Reproduce):**
  1. Mở `DOM Ladder` trên mã `VN30F1M`, đặt vốn khả dụng `100.000 kVND`, chọn chế độ định cỡ theo rủi ro `Risk 5%`, `SL = 1` bước giá tại mốc `1.300.0`.
  2. Gọi `size_ladder_qty_from_risk_budget(100_000.0, 5.0, 1_300.0, 1, OrderSide::Buy, ExchangeVenue::Vn30Futures, PriceUnit::ThousandVnd, Some(1_300.0))`.
- **Kết quả thực tế (Actual Result):**
  - Hàm trả về **`Ok(26.0)` hợp đồng**! Nguyên nhân là `ladder.rs` tính `margin_per_unit = raw_notional * 0.17` (`22.100 kVND/HĐ`), rồi tiếp tục chia cho `max_leverage = 1 / 0.17 ≈ 5.8824`, khiến tỷ lệ ký quỹ thực tế giảm còn `0.17 × 0.17 = 2.89%` (**đòn bẩy `34.6x`**, vượt gấp **`6.5x`** giới hạn pháp lý VSD `17%` vốn chỉ cho phép tối đa **`4` hợp đồng** trên `100.000 kVND`).
- **Kết quả kỳ vọng / Chuẩn TradingView hoặc Quy chế VN (Expected Result):**
  - Với vốn `100.000 kVND`, giá `1.300`, hệ số `100 kVND/điểm` và tỷ lệ ký quỹ ban đầu VSD `17%` (`22.100 kVND/HĐ`), khối lượng tối đa được phép mở phải bị chặn ở **`4` hợp đồng** (`floor(100.000 / 22.100)`).
- **Bằng chứng đính kèm (Evidence):**
  - Telemetry đối kháng: `[REPRO P0 VN30F1M Double-Leverage] sized_contracts=Ok(26.0) (statutory VSD 17% max on 100k kVND = 4!)`.
  - Mã nguồn gốc: `src/trading/ladder.rs:1068-1074` (nhân `initial_margin_pct` lần 1 trong `margin_per_unit` rồi lại dùng `max_leverage = 5.8824` từ `src/journal/planner.rs:351-352` lần 2).

---

### [BUG-003] Khởi động lại ứng dụng làm mất sạch phiên `CafeF` (`CustomFile`), con trỏ Bar Replay, nét vẽ và vị thế mở
- **Mức độ nghiêm trọng (Severity):** `P0`
- **Phân hệ liên quan:** `Bar Replay` / `Settings / Workspace`
- **Mã kịch bản (Test Case ID):** `TC-37`, `TC-05`
- **Nguồn dữ liệu & Mã chứng khoán:** `CafeF Adj (471 mã)` — `FPT · 1D` (và mọi mã trên `CafeF Adj`/`Raw`/`Index` có ít nhất `1` dòng bị cách ly `QuarantineRow`)
- **Các bước tái hiện chính xác (Steps to Reproduce):**
  1. Bấm `[Import CSV/JSON]` -> `[CafeF Adj (471 mã)]`, mở `FPT · 1D` (`2.702` nến, `3` dòng quarantine).
  2. Bật `Bar Replay`, cắt tại thanh `2.000`, vẽ 2 nét kỹ thuật và mở 1 vị thế Paper Trading.
  3. Đóng ứng dụng (`Cmd+Q`) và mở lại ứng dụng (gọi `restore_persisted_session()`).
- **Kết quả thực tế (Actual Result):**
  - Toàn bộ dữ liệu `CafeF Adj`, vị trí con trỏ Replay, nét vẽ và trạng thái phiên biến mất hoàn toàn; ứng dụng rơi ngược về dữ liệu mặc định **`VNINDEX Synth (120 nến)`** (`replay_active = false`).
  - Nguyên nhân: `restore_persisted_session()` nạp lại `CustomFile` bằng `DataLoadOptions::default()` vốn cài cứng `InvalidRowPolicy::RejectDataset` thay vì `QuarantineRow` + `CafeFSplitAware`, khiến mọi bộ `CafeF` có dòng lỗi gốc (`FPT`, `BVS`, `C69`, `HNX-INDEX`...) bị từ chối nạp lúc khởi động lại.
- **Kết quả kỳ vọng / Chuẩn TradingView hoặc Quy chế VN (Expected Result):**
  - Khởi động lại ứng dụng phải lưu và dùng đúng `DataLoadOptions` (`QuarantineRow`, `CafeFSplitAware`) của phiên trước, khôi phục nguyên vẹn mã `FPT` (`2.702` nến), con trỏ Bar Replay tại thanh `2.000`, nét vẽ và sổ lệnh Paper.
- **Bằng chứng đính kèm (Evidence):**
  - Telemetry đối kháng: `[TC-37 Restart Restore CustomFile FPT] data_source=SyntheticVnindex symbol=VNINDEX bars=120 replay_active=false`.
  - Mã nguồn gốc: `src/app.rs:1572-1640` (dùng `DataLoadOptions::default()` thay vì `default_import_load_options()`).

---

### [BUG-004] Lõi `TradingSession` không cưỡng chế khóa lô `T+2` cho lệnh bán Market / `Flatten`, trong khi UI `Flatten` lại chặn bán phần cổ phiếu đã về
- **Mức độ nghiêm trọng (Severity):** `P0`
- **Phân hệ liên quan:** `Paper Trading / T+2`
- **Mã kịch bản (Test Case ID):** `TC-40`, `TC-42`
- **Nguồn dữ liệu & Mã chứng khoán:** `CafeF Adj (471 mã)` — `FPT · 1D` (`HOSE`, `enforce_vn_t2_lot_lock = true`)
- **Các bước tái hiện chính xác (Steps to Reproduce):**
  1. Khởi tạo `TradingSession` với `venue = ExchangeVenue::Hose`, `enforce_vn_t2_lot_lock = true`.
  2. Ngày Thứ Hai (`T0`), mua `100` cổ phiếu `FPT`.
  3. Chuyển sang Thứ Ba (`T+1`), gửi lệnh `OrderType::Market` bán `100` cổ phiếu hoặc gọi `Flatten` trực tiếp qua `TradingSession` (hoặc kích hoạt luồng nội bộ bỏ qua kiểm tra của lớp UI).
  4. Đồng thời trên UI, giữ `200` cổ phiếu đã về (`[SELLABLE]`) và `300` cổ phiếu chưa về (`[LOCKED T+2]`), sau đó bấm nút `Flatten`.
- **Kết quả thực tế (Actual Result):**
  - Tại lõi `TradingSession`: `apply_submit_order_internal` (Market Sell) và `apply_flatten_internal` **hoàn toàn không kiểm tra `enforce_vn_t2_lot_lock`**, trả về **`Ok(2)`** và thanh lý trái phép toàn bộ `100` cổ phiếu đang bị khóa `T+1` (khóa `T+2` chỉ mới chặn ở `check_vn_t2_sell_lock_for_order` tại tầng UI `src/app.rs` và ở pha `SL/TP`).
  - Tại tầng UI: nút `Flatten` từ chối toàn bộ (`flatten_err = true`) thay vì cho phép bán thoát phần `200` cổ phiếu hợp pháp đã về và giữ lại `300` cổ phiếu đang khóa (thiếu nhất quán với cơ chế `Auto-SL` bán từng phần hợp lệ).
- **Kết quả kỳ vọng / Chuẩn TradingView hoặc Quy chế VN (Expected Result):**
  - Mọi đường vào bán cổ phiếu (`Market Sell`, `Limit/Stop Sell` lúc khớp, `SL/TP`, `Flatten`) phải được bảo vệ tại **lõi nghiệp vụ `TradingSession`**, tuyệt đối không thể bán vượt số lượng `[SELLABLE]` khi `enforce_vn_t2_lot_lock = true`.
  - Lệnh `Flatten` khi có cả cổ phiếu đã về và chưa về nên đóng tối đa phần `[SELLABLE]` (hoặc báo rõ đã bán `200` cp đã về, giữ `300` cp khóa `T+2`).
- **Bằng chứng đính kèm (Evidence):**
  - Telemetry đối kháng: `[REPRO P0 Direct TradingSession T+1 Market Sell w/ enforce_vn_t2_lot_lock=true] res=Ok(2)` và `[TC-40 FIFO T+2] before(sellable=200, locked=300) flatten_err=true`.
  - Mã nguồn gốc: `src/trading/portfolio.rs:788-859, 913-1048, 1087-1125`; `src/app.rs:3286-3293`.

---

### Nhóm `P1 — Critical` (Chặn Phát Hành — `13` Lỗi)

---

### [BUG-005] `CafeFSplitAware` Rule 5 có điều kiện loại bỏ sai logic, xóa oan `23.550` phiên giao dịch hợp lệ trên `421/471` mã `CafeFData`
- **Mức độ nghiêm trọng (Severity):** `P1`
- **Phân hệ liên quan:** `CafeF Data Loader`
- **Mã kịch bản (Test Case ID):** `TC-04`, `TC-05`
- **Nguồn dữ liệu & Mã chứng khoán:** `CafeF Adj (471 mã)` (`RCL`, `KSV`, `APS`, `API`, `C69`, `BVS`, `FPT`...) & `CafeF Index` (`HNX-INDEX`)
- **Các bước tái hiện chính xác (Steps to Reproduce):**
  1. Nạp `data/raw/cafef/CafeF.SolieuGD.Adjusted.Upto06112026.csv` bằng nút `[CafeF Adj (471 mã)]`.
  2. Kiểm tra báo cáo `quarantined_row_count` và mở `[Data Audit]` trên `RCL`, `KSV`, `APS`, `API`, `C69`, `BVS`, `HNX-INDEX`.
- **Kết quả thực tế (Actual Result):**
  - Tổng cộng **`25.173` dòng bị cách ly (xóa khỏi biểu đồ)** trên **`421/471` mã**, trong đó **`23.550` dòng là các phiên phẳng hợp lệ sau chia tách** bị loại oan bởi nhánh `None if delta_max_vnd > 1 ULP` không tồn tại trong bộ nạp chuẩn lịch sử (`cafef_importer.py:141-144`).
  - Cụ thể: `RCL` mất **`798/1.994` phiên (`40%` toàn bộ lịch sử)**; `KSV` mất `113` phiên hợp lệ (`114` total); `APS` mất `109`; `API` mất `80`; `C69` mất `29`; `BVS` mất `19`; `HNX-INDEX` mất `462` phiên.
- **Kết quả kỳ vọng / Chuẩn TradingView hoặc Quy chế VN (Expected Result):**
  - Chỉ cách ly các dòng thực sự lỗi dữ liệu gốc (`High = 0`, `Open/Close <= 0`, hoặc chênh lệch biên vượt ngưỡng chia tách thực sự). Các phiên phẳng hợp lệ có sai số làm tròn 4 chữ số thập phân sau chia tách (`<= 6 bps & <= 25 VND`) phải được chuẩn hóa và giữ lại đầy đủ.
- **Bằng chứng đính kèm (Evidence):**
  - Ảnh chụp: [`docs/testing/qa_evidence/01_tc04_data_audit_c69_1280x800.png`](./qa_evidence/01_tc04_data_audit_c69_1280x800.png) (`C69` hiển thị `30` quarantined trong khi chỉ có `1` lỗi thực).
  - Telemetry: `[REPRO P1 CafeFSplitAware Flat-Bar Bug] total_quarantined=25173 across 421/471 symbols; RCL(kept,quar,repaired)=Some((1195, 799, 554))`.
  - Mã nguồn gốc: `src/data/audit.rs:235-243` (đối chiếu `ARCHIVE/.../cafef_importer.py:141-144`).

---

### [BUG-006] Âm thầm thay `VNINDEX` lịch sử thật bằng đường sóng sin giả lập trong `RS`, `Cmp:VNINDEX`, `RRG` và `Multi-Strategy 0% UTC`
- **Mức độ nghiêm trọng (Severity):** `P1`
- **Phân hệ liên quan:** `Strategy Lab` / `Indicators` / `Scanner / ProxyBB`
- **Mã kịch bản (Test Case ID):** `TC-20`, `TC-51`, `TC-56`
- **Nguồn dữ liệu & Mã chứng khoán:** `CafeF Adj (471 mã)` — `FPT · 1D` (`2016-01-04 .. 2026-11-06`)
- **Các bước tái hiện chính xác (Steps to Reproduce):**
  1. Nạp `CafeF Adj (471 mã)` -> chọn `FPT · 1D`.
  2. Bật `RS` và `Cmp:VNINDEX` trên thanh công cụ (hoặc mở `Strategy Lab` -> tab `Slices` -> bật `Multi-Strategy [0% UTC]`, hoặc mở `Scanner -> Breadth -> RRG`).
  3. So sánh giá trị `VNINDEX` hiển thị với dữ liệu thật trong `data/raw/cafef/CafeF.INDEX.Upto06112026.csv`.
- **Kết quả thực tế (Actual Result):**
  - Hệ thống âm thầm thay thế `VNINDEX` thật (`2016-01-04 = 574.41 -> 2026-11-06 = 1725.36`, **`+200.37%`**) bằng chuỗi sóng sin giả lập (`1294.72 -> 2508.04`, **`+93.71%`**, `MaxDD -3.6%` suốt 10 năm — hoàn toàn sai thực tế thị trường Việt Nam giai đoạn 2018, 2020, 2022!) mà **không có bất kỳ cảnh báo nào trên giao diện**.
  - Nguyên nhân: `load_preferred_vnindex_series()` kiểm tra tệp cũ `CafeF.INDEX.clean.csv` kết thúc `2026-10-05`, không vượt qua điều kiện `reaches_recent_tail` (`14` ngày) so với `2026-11-06` nên tự rơi về chuỗi giả lập, đồng thời không ưu tiên tìm `data/raw/cafef/CafeF.INDEX.Upto06112026.csv`.
- **Kết quả kỳ vọng / Chuẩn TradingView hoặc Quy chế VN (Expected Result):**
  - Tự động ưu tiên nạp `VNINDEX` thật từ `data/raw/cafef/CafeF.INDEX.Upto06112026.csv` (hoặc tệp Index gần nhất trong thư mục `data/raw/cafef/`). Nếu thực sự không có tệp Index thật, phải hiển thị nhãn cảnh báo đỏ `[SYNTH BENCHMARK]` rõ ràng thay vì dán nhãn `VNINDEX` gây hiểu lầm phân tích đầu tư.
- **Bằng chứng đính kèm (Evidence):**
  - Ảnh chụp: [`docs/testing/qa_evidence/10_tc20_fpt_rs_cmp_vnindex_1280x800.png`](./qa_evidence/10_tc20_fpt_rs_cmp_vnindex_1280x800.png) (`CMP(VNINDEX)` đi ngang uốn lượn giả lập); [`docs/testing/qa_evidence/16_tc56_57_multistrat_0pct_slices_1440x1000.png`](./qa_evidence/16_tc56_57_multistrat_0pct_slices_1440x1000.png) (`Bench (VNINDEX) +93.71%, MaxDD -3.6%`).
  - Telemetry: `[TC-20 FPT Cmp:VNINDEX] bench_first=Some(1294.72) bench_last=Some(2508.04)` vs `[TC-20 REAL VNINDEX] 2016-01-04=574.41 2026-11-06=1725.36`.
  - Mã nguồn gốc: `src/data/presets.rs:385, 536-572`; `src/app.rs:5404`.

---

### [BUG-007] Gõ mã VN30 (`VCB`, `FPT`, `HPG`, `SSI`...) vào `Cmd+K` rồi nhấn `Enter` phá hủy toàn bộ bộ dữ liệu `471` mã `CafeF`
- **Mức độ nghiêm trọng (Severity):** `P1`
- **Phân hệ liên quan:** `Settings / Workspace` / `CafeF Data Loader`
- **Mã kịch bản (Test Case ID):** `TC-03`, `TC-48`
- **Nguồn dữ liệu & Mã chứng khoán:** `CafeF Adj (471 mã)` -> tìm `VCB`, `HPG`, `SSI`, `MWG`, `TCB`, `FPT`
- **Các bước tái hiện chính xác (Steps to Reproduce):**
  1. Nạp bộ dữ liệu thực `CafeF Adj (471 mã)` (`2.702` nến/mã, `2016–2026`).
  2. Bấm `Cmd+K`, gõ `VCB` (hoặc `HPG`, `SSI`, `FPT`) và nhấn `Enter` chọn kết quả đứng đầu.
- **Kết quả thực tế (Actual Result):**
  - `Cmd+K` chấm đồng điểm `score = 339` cho `VCB [VN30 Basket]` (180 nến giả lập) và `VCB [File]` (2.700 nến CafeF thật), nhưng xếp **`VN30 Basket` đứng #1**.
  - Nhấn `Enter` lập tức gọi `CustomSourceUpdate::Clear`, **xóa sạch bộ `471` mã CafeF khỏi bộ nhớ** và chuyển toàn bộ ứng dụng sang nguồn giả lập `Vn30Basket (180 nến, 8 mã)`.
- **Kết quả kỳ vọng / Chuẩn TradingView hoặc Quy chế VN (Expected Result):**
  - Khi người dùng đang nạp một nguồn dữ liệu thực (`CustomFile`), mọi mã trùng tên trong bộ nguồn đang mở phải được **ưu tiên đứng đầu** (hoặc loại bỏ bản sao giả lập `VN30 Basket`), không bao giờ âm thầm xóa mất bộ `471` mã đã nạp.
- **Bằng chứng đính kèm (Evidence):**
  - Telemetry: `[TC-03 Cmd+K q='VCB'] top3=["VCB — Switch Symbol [VN30 Basket · 111.44 (+2.69%)] (score=339)", "VCB — Switch Symbol [File · 53.60 (-1.11%)] (score=339)", ...]` -> `[TC-03 Cmd+K ENTER 'VCB' EFFECT] data_source=Vn30Basket symbol=VCB bars=180 avail_syms=8`.
  - Mã nguồn gốc: `src/app.rs:8023-8044, 8744-8752, 4871-4896`.

---

### [BUG-008] Cho phép bán khống (`Short`) trái phép trên cổ phiếu cơ sở VN (`HOSE`/`HNX`/`UPCOM`) ở cả Paper Trading và Backtest; ngược lại có thể chặn nhầm Short trên `VN30F1M`
- **Mức độ nghiêm trọng (Severity):** `P1`
- **Phân hệ liên quan:** `Paper Trading / T+2` / `Strategy Lab`
- **Mã kịch bản (Test Case ID):** `TC-42`, `TC-43`, `TC-54`
- **Nguồn dữ liệu & Mã chứng khoán:** `CafeF Adj (471 mã)` — `FPT · 1D` (`HOSE`), `VN30F1M`
- **Các bước tái hiện chính xác (Steps to Reproduce):**
  1. **Paper Trading:** Mở `FPT` (`HOSE`), vào `[Settings]` tắt `T+2 Lock` (`enforce_vn_t2_lot_lock = false`) để luyện PTKT tự do. Ở trạng thái không có cổ phiếu (`flat`), gửi lệnh `Sell 100` trên Order Ticket.
  2. **Paper Trading (`VN30F1M`):** Chuyển sang `VN30F1M`, mở `[Settings]` bấm `Apply` khi `enforce_vn_t2_lot_lock = true` rồi thử mở vị thế Short.
  3. **Strategy Lab:** Chọn `FPT` (`HOSE`), chọn chế độ `LongShort` (hoặc `ShortOnly`) rồi chạy Backtest / `Battle` / `Portfolio`.
- **Kết quả thực tế (Actual Result):**
  - Trên `FPT` (`HOSE`, `allows_short() == false`), tắt `T+2 Lock` tự động bật `trading.allow_short = true` -> lệnh `Sell 100` từ trạng thái flat trả về **`Ok(1)`** và mở vị thế **`Short 100`** trái quy định pháp luật chứng khoán VN; khi mua cover còn phát sinh lô Long FIFO ảo `100` cp bị khóa `T+2`.
  - Trên `Strategy Lab`: Backtest `LongShort` trên `FPT` (`HOSE`) thực thi **`22` lệnh Short trái phép** (không kiểm tra `venue.allows_short()`).
  - Ngược lại, giá trị `allow_short` bị lưu cũ theo cờ `T+2` có thể chặn nhầm lệnh Short hợp pháp trên `VN30F1M` sau khi bấm Apply trong `[Settings]`.
- **Kết quả kỳ vọng / Chuẩn TradingView hoặc Quy chế VN (Expected Result):**
  - Quyền bán khống phải tách biệt hoàn toàn khỏi cờ `T+2 Lock` và luôn tuân thủ `venue.allows_short()`: cấm tuyệt đối Short trên cổ phiếu cơ sở VN (`HOSE`/`HNX`/`UPCOM`) ở cả Paper Trading lẫn Strategy Lab (hoặc hiển thị cờ opt-in riêng có cảnh báo vi phạm quy chế VN), và luôn cho phép 2 chiều `Long`/`Short` trên `VN30F1M`, `US`, `Crypto`.
- **Bằng chứng đính kèm (Evidence):**
  - Telemetry: `[TC-42/43 ADVERSARIAL HOSE Short via Order Ticket when T+2=OFF] venue=Hose allows_short=false trading.allow_short=true submit_sell_from_flat=Ok(1) open_pos=Some((Short, 100.0))` và `[REPRO P1 Backtest Illegal HOSE Short on FPT] short_trades=22 (HOSE allows_short=false!)`.
  - Mã nguồn gốc: `src/trading/orders.rs:59`; `src/trading/portfolio.rs:953`; `src/app.rs:2354-2357, 3842-3846`; `src/journal/market_rules.rs:938-940`; `src/backtest/engine.rs:427-475, 2503-2534`.

---

### [BUG-009] Vi phạm quy tắc bi quan `DEF-005` (`SL` trước `TP`) khi lệnh chờ khớp bằng Gap mở cửa Phase A
- **Mức độ nghiêm trọng (Severity):** `P1`
- **Phân hệ liên quan:** `Paper Trading / T+2`
- **Mã kịch bản (Test Case ID):** `TC-39`
- **Nguồn dữ liệu & Mã chứng khoán:** Thanh nến OHLC bất kỳ có Gap qua giá kích hoạt và quét cả `SL` lẫn `TP`
- **Các bước tái hiện chính xác (Steps to Reproduce):**
  1. Đặt lệnh chờ `Buy Stop` tại `102.0` kèm `Stop-Loss = 98.0`, `Take-Profit = 105.0`.
  2. Tiến sang cây nến kế tiếp có `Open = 102.5`, `High = 106.0`, `Low = 97.0`, `Close = 101.0` (nhảy Gap khớp ngay tại giá mở cửa `102.5`, sau đó trong phiên chạm cả `106.0 >= TP` và `97.0 <= SL`).
- **Kết quả thực tế (Actual Result):**
  - Hệ thống khớp thoát lệnh là **`TakeProfitExit @ 105.0`** (chốt lời lạc quan) thay vì xử lý theo đường đi bi quan `StopLossExit @ 98.0`.
- **Kết quả kỳ vọng / Chuẩn TradingView hoặc Quy chế VN (Expected Result):**
  - Theo quy tắc `DEF-005` (pessimistic dual-touch), mọi thanh nến khớp vị thế và đồng thời chứa cả `SL` lẫn `TP` trong cùng một thanh phải ưu tiên chốt **`StopLoss @ 98.0`** để không tạo ảo giác lợi nhuận khi luyện tập.
- **Bằng chứng đính kèm (Evidence):**
  - Telemetry đối kháng: `[REPRO P1 DEF-005 Gap-Open Dual-Touch] exit_reason=Some(TakeProfitExit) exit_price=Some(105.0) (pessimistic must be StopLoss @ 98.0!)`.
  - Mã nguồn gốc: `src/trading/portfolio.rs:1462-1568` (Phase A xử lý khớp pending trên gap mở cửa nhưng không áp nhánh bi quan dual-touch như Phase B).

---

### [BUG-010] Hàm cắt nhân quả theo giờ đóng cửa VN/US không đơn điệu (tụt mất nến khi tiến thời gian) & nhận nhầm mã US sang `HOSE`
- **Mức độ nghiêm trọng (Severity):** `P1`
- **Phân hệ liên quan:** `Bar Replay` / `Strategy Lab` / `Scanner / ProxyBB` / `Alerts`
- **Mã kịch bản (Test Case ID):** `TC-31`, `TC-52`, `TC-54`
- **Nguồn dữ liệu & Mã chứng khoán:** `CafeF Adj (471 mã)` — `FPT · 1D` và các mã US ngoài whitelist (`GOOG`, `AMZN`, `META`)
- **Các bước tái hiện chính xác (Steps to Reproduce):**
  1. Chạy `Backtest` hoặc `ProxyBB` với mốc cắt Replay `cutoff` ở `07:59:59 UTC` (`14:59:59 ICT`) ngày `2026-11-06` trên `FPT · 1D`.
  2. Tiến đồng hồ Replay thêm `1` giây sang `08:00:00 UTC` (`15:00:00 ICT` — thời điểm đóng cửa chính thức HOSE).
  3. Kiểm tra thêm phân loại sàn cho các mã US ngoài danh sách cứng (`GOOG`, `AMZN`, `META`) trong hàm tính giờ đóng cửa phiên.
- **Kết quả thực tế (Actual Result):**
  - Khi tiến thời gian từ `07:59:59 UTC` sang `08:00:00 UTC`, số thanh nến hợp lệ trên `FPT · 1D` **tụt ngược từ `2.702` xuống `2.701`** (một cây nến đã hiển thị bỗng biến mất khi bước tới tương lai!).
  - Tương tự xảy ra tại `20:59:59 -> 21:00:00 UTC` cho thị trường US, đồng thời không tính múi giờ mùa hè EDT (`UTC-4`) hay ngày nghỉ sớm Thứ Sáu, và các mã US không nằm trong whitelist nhỏ (`GOOG`, `AMZN`, `META`) bị phân loại nhầm sang `HOSE` (đóng cửa `08:00 UTC` -> lộ nến ngày US trước cả khi NYSE mở cửa, vi phạm `DEF-001`).
- **Kết quả kỳ vọng / Chuẩn TradingView hoặc Quy chế VN (Expected Result):**
  - Hàm lọc nến nhân quả `bar_close_ts <= cutoff` phải **đơn điệu không giảm tuyệt đối** khi `cutoff` tăng, dùng đúng phiên thực tế theo múi giờ và lịch giao dịch của sàn.
- **Bằng chứng đính kèm (Evidence):**
  - Telemetry đối kháng: `[REPRO P1 Non-Monotonic VN Close Cutoff] bars@07:59:59UTC=2702 -> bars@08:00:00UTC=2701 (dropped bar on forward step!)`.
  - Mã nguồn gốc: `src/backtest/engine.rs:780-839, 2542-2594`; `src/scanner/proxy_bb.rs:268-278`.

---

### [BUG-011] Cảnh báo `RSI`/`MACD`/`DrawingLine` nổ hồi tố ngay trên nến đặt lệnh (trên dữ liệu khác `00:00 UTC`) và nổ cảnh báo ma trên nến `1W` chưa đóng
- **Mức độ nghiêm trọng (Severity):** `P1`
- **Phân hệ liên quan:** `Alerts` / `Bar Replay`
- **Mã kịch bản (Test Case ID):** `TC-31`, `TC-52`
- **Nguồn dữ liệu & Mã chứng khoán:** `SPY CSV` (`SPY · 1D`, timestamp `14:30 UTC`) & `CafeF Adj` (`FPT · 1D -> 1W` giữa tuần)
- **Các bước tái hiện chính xác (Steps to Reproduce):**
  1. Mở `SPY CSV` (nến ngày gắn mốc `14:30 UTC`), đặt cảnh báo `RsiThreshold` (`Once`) tại cây nến hiện tại `arm_ts = 1713709800` mà điều kiện RSI vừa cắt ở thanh đó.
  2. Chạy đánh giá cảnh báo ngay ở cùng mốc thời gian (chưa hề có cây nến mới nào xuất hiện).
  3. Trên `FPT · 1D`, cắt Replay giữa tuần (Thứ Tư), chuyển sang `1W` và đặt cảnh báo `Once` kiểu `RsiThreshold`/`MacdHistogramZeroCross`/`DrawingLine` -> tua tiến qua Thứ Năm/Thứ Sáu.
- **Kết quả thực tế (Actual Result):**
  - Cảnh báo `Once` trên `SPY` **nổ hồi tố ngay lập tức trên chính nến quá khứ `arm_ts = 1713709800`** và tự chuyển sang trạng thái `Triggered` (đốt cháy cảnh báo trước khi có dữ liệu mới).
  - Trên khung `1W` giữa tuần, 3 loại cảnh báo này đánh giá luôn thanh tuần chưa đóng cửa, phát ra cảnh báo ma; nếu Thứ Năm/Thứ Sáu giá đảo chiều khiến điều kiện hết đúng, cảnh báo ma biến mất và trạng thái `Once` bật ngược lại `Armed`.
- **Kết quả kỳ vọng / Chuẩn TradingView hoặc Quy chế VN (Expected Result):**
  - Cảnh báo mới tạo chỉ được phép đánh giá trên các thanh nến đóng cửa **chặt sau** thời điểm tạo (`bar_ts > arm_ts`) bất kể giờ mở cửa là `00:00` hay `14:30 UTC`, và không phát cảnh báo `On Bar Close` trên thanh HTF chưa hoàn tất.
- **Bằng chứng đính kèm (Evidence):**
  - Telemetry đối kháng: `[REPRO P1 SPY 14:30 UTC Retroactive Arming-Bar RSI Alert] arm_ts=1713709800 triggers=[(1713709800, true)]`.
  - Mã nguồn gốc: `src/alerts/engine.rs:984-986, 1131-1133, 1344-1345`.

---

### [BUG-012] Bỏ tick `CafeF Split Norm` trong hộp thoại Import không từ chối tệp lỗi biên OHLC mà âm thầm cách ly và xóa biểu đồ đang mở
- **Mức độ nghiêm trọng (Severity):** `P1`
- **Phân hệ liên quan:** `CafeF Data Loader`
- **Mã kịch bản (Test Case ID):** `TC-06`
- **Nguồn dữ liệu & Mã chứng khoán:** Tệp CSV chứa dòng vi phạm biên OHLC (`Close > High`) khi đang mở `FPT · 1D` (`2.702` nến)
- **Các bước tái hiện chính xác (Steps to Reproduce):**
  1. Đang mở biểu đồ `FPT` (`CafeF Adj`, `2.702` nến).
  2. Mở `[Import CSV/JSON]`, **bỏ dấu tick `CafeF Split Norm`** (`import_cafef_split_aware = false`).
  3. Nhập đường dẫn tệp CSV có `1` dòng hợp lệ và `1` dòng vi phạm OHLC (`Close > High`) rồi bấm `Import`.
- **Kết quả thực tế (Actual Result):**
  - Quá trình nạp trả về **`Ok`** (`res_is_ok = true`, `bars = 1`, `quar = 1`), **xóa thay biểu đồ `FPT` đang mở** bằng biểu đồ `1` nến `C69` thay vì báo lỗi đỏ và giữ nguyên `FPT`.
  - Nguyên nhân: `default_import_load_options()` luôn cài cứng `InvalidRowPolicy::QuarantineRow`, mà trong `src/data/mod.rs:640-644` thì `QuarantineRow` vô hiệu hóa `OhlcEnvelopePolicy::RejectRow`.
- **Kết quả kỳ vọng / Chuẩn TradingView hoặc Quy chế VN (Expected Result):**
  - Khi người dùng chủ động tắt `CafeF Split Norm` (chế độ kiểm tra nghiêm ngặt), tệp có dòng sai biên OHLC phải bị từ chối với thông báo lỗi màu đỏ rõ ràng và **giữ nguyên biểu đồ hiện tại đang mở**.
- **Bằng chứng đính kèm (Evidence):**
  - Telemetry đối kháng: `[REPRO P1 TC-06 Unchecked CafeF Split Norm] res_is_ok=true active_sym=C69 bars=1 quar=1`.
  - Mã nguồn gốc: `src/app.rs:3789-3799`; `src/data/mod.rs:640-644`.

---

### [BUG-013] `TimestampUnit::Auto` chuyển nhầm chuỗi Unix-seconds hiện đại kết thúc bằng `000` thành tháng `01/1970` (Milliseconds)
- **Mức độ nghiêm trọng (Severity):** `P1`
- **Phân hệ liên quan:** `CafeF Data Loader`
- **Mã kịch bản (Test Case ID):** `TC-02`, `TC-06`
- **Nguồn dữ liệu & Mã chứng khoán:** Tệp CSV/JSON dùng Unix timestamp giây mà mọi mốc đều chia hết cho `1.000` (ví dụ mỗi ngày thứ 5 lúc `00:00 UTC`, hoặc chuỗi khung `50m`/`5h`, như `1728000000` = `2024-10-04` và `1728432000` = `2024-10-09`)
- **Các bước tái hiện chính xác (Steps to Reproduce):**
  1. Tạo tệp CSV/JSON có 2 thanh nến với timestamp Unix giây: `1728000000` (`2024-10-04 00:00 UTC`) và `1728432000` (`2024-10-09 00:00 UTC`).
  2. Nạp qua bộ giải mã mặc định `TimestampUnit::Auto`.
- **Kết quả thực tế (Actual Result):**
  - Toàn bộ chuỗi bị nhận nhầm là Milliseconds, biến đổi thành **`1970-01-21 00:00` và `1970-01-21 00:07`** với khung thời gian suy diễn sai thành **`Timeframe::M5`**.
- **Kết quả kỳ vọng / Chuẩn TradingView hoặc Quy chế VN (Expected Result):**
  - Giá trị trong khoảng `10^9 .. 10^10` (giai đoạn `2001–2286`) là Unix **giây**, chỉ nhận Milliseconds khi `>= 10^11` (`100_000_000_000`), không bao giờ dựa vào phép kiểm tra chia hết cho `1000` vốn là điều kiện hiển nhiên trên số chẵn nghìn giây.
- **Bằng chứng đính kèm (Evidence):**
  - Telemetry đối kháng: `[REPRO P1 TimestampUnit::Auto 1970 Corruption] input=2024-10-04..09 -> parsed_labels=["1970-01-21 00:00", "1970-01-21 00:07"] tf=M5`.
  - Mã nguồn gốc: `src/data/calendar.rs:537-553`.

---

### [BUG-014] `IndicatorCache` phục vụ dữ liệu cũ vĩnh viễn sau khi đổi tham số lúc chỉ báo đang tắt (và rò `ATR` cũ vào Candlestick Patterns)
- **Mức độ nghiêm trọng (Severity):** `P1`
- **Phân hệ liên quan:** `Indicators`
- **Mã kịch bản (Test Case ID):** `TC-16`, `TC-19`
- **Nguồn dữ liệu & Mã chứng khoán:** Mọi nguồn dữ liệu (`FPT · 1D`, `VN30F1M · 15m`...)
- **Các bước tái hiện chính xác (Steps to Reproduce):**
  1. Bật chỉ báo `SMA(20)` (giá trị tại nến cuối là `169.50`).
  2. Tắt `SMA` (`show_sma = false`).
  3. Đổi tham số chu kỳ `sma_period` từ `20` thành `50` (khi đang tắt).
  4. Bật lại `SMA` (`show_sma = true`).
- **Kết quả thực tế (Actual Result):**
  - Thanh chú giải hiển thị nhãn **`SMA(50)`**, nhưng đường vẽ trên biểu đồ vẫn lấy nguyên mảng cũ của **`SMA(20) = 169.50`** thay vì tính lại **`SMA(50) = 154.50`** (tương tự cho `EMA`, `BB`, `RSI`, `MACD`, `ATR`, `SuperTrend`, `ADX`, `Stoch`, `CCI`, `MFI`...). Ngoài ra, `Candlestick Patterns (PAT)` có thể đọc `ATR` cũ khi `show_atr` đang tắt.
- **Kết quả kỳ vọng / Chuẩn TradingView hoặc Quy chế VN (Expected Result):**
  - `IndicatorCache` phải lưu chữ ký tham số (`cached_sma_period`...) riêng cho từng chỉ báo đã tính (hoặc xóa khóa cache ngay khi tham số đổi), đảm bảo bật lại là tính đúng ngay `154.50`.
- **Bằng chứng đính kèm (Evidence):**
  - Telemetry đối kháng: `[REPRO P1 IndicatorCache Stale Edit-While-Disabled] sma20=169.50 after_edit_to_50=169.50 (expected 154.50!)`.
  - Mã nguồn gốc: `src/indicators/mod.rs:1090-1357` (so sánh `self.last_config` với config mới nhưng bỏ qua tính toán khi `enabled == false`, sau đó lại ghi đè `self.last_config = config`).

---

### [BUG-015] Công cụ vẽ `AnchoredVwap (AVWAP)` bỏ qua `R11RenderMemo` và tính lại toàn chuỗi Welford `2` lần mỗi khung hình
- **Mức độ nghiêm trọng (Severity):** `P1`
- **Phân hệ liên quan:** `Drawings` / `Chart/Viewport`
- **Mã kịch bản (Test Case ID):** `TC-22`, `TC-63`
- **Nguồn dữ liệu & Mã chứng khoán:** `VNINDEX Synth 100K` (`100.000` nến)
- **Các bước tái hiện chính xác (Steps to Reproduce):**
  1. Mở chuỗi `VNINDEX Synth 100K`.
  2. Vẽ `1` đường `AVWAP` neo ở vùng đầu lịch sử (hoặc `2` đường `AVWAP`).
  3. Rê chuột ngang qua biểu đồ (hover crosshair) và đo thời gian CPU dựng mỗi khung hình.
- **Kết quả thực tế (Actual Result):**
  - Chỉ với **`1` đường `AVWAP`**, thời gian CPU mỗi khung hình hover vọt từ `0.27 ms` lên **`3.14 ms` trung vị (`p95 = 3.49 ms`)**. Vẽ từ `2` đường `AVWAP` trở lên sẽ **vượt trần `< 4.0 ms`** (`> 6.2 ms`, phá vỡ `DECISION-RENDERER-002`).
  - Nguyên nhân: `project_drawing` gọi `compute_anchored_vwap_series` toàn bộ từ điểm neo tới hết `100K` nến **2 lần mỗi frame** (1 lần cho hit-testing, 1 lần cho vẽ) mà không cache trong `R11RenderMemo`.
- **Kết quả kỳ vọng / Chuẩn TradingView hoặc Quy chế VN (Expected Result):**
  - Lưu đệm đường `AVWAP` theo khóa `(drawing_id, anchor_bar, data_revision, candle_len)` trong `R11RenderMemo` để hover/pan chỉ tốn `O(visible_bars)` (`< 0.5 ms`).
- **Bằng chứng đính kèm (Evidence):**
  - Telemetry đối kháng: `[REPRO P1 AVWAP 100K Hover CPU Frame Time] median=3.14ms p95=3.49ms (budget < 4.0ms!)`.
  - Mã nguồn gốc: `src/drawings/geometry.rs:761-813`; `src/chart.rs:1898, 2721, 12057`.

---

### [BUG-016] `OutlierImpactReport::expectancy_ex_top_3_winners` chia sai mẫu số (`valid_n - 3`), gây sai lệch `300%` khi chiến lược có ít hơn `3` lệnh thắng
- **Mức độ nghiêm trọng (Severity):** `P1`
- **Phân hệ liên quan:** `Strategy Lab`
- **Mã kịch bản (Test Case ID):** `TC-57`
- **Nguồn dữ liệu & Mã chứng khoán:** Mọi kết quả Backtest có `1` hoặc `2` lệnh thắng và tổng số lệnh `>= 4` (ví dụ `4` lệnh với PnL `[+50, -100, -100, -100]`)
- **Các bước tái hiện chính xác (Steps to Reproduce):**
  1. Chạy `OutlierImpactReport::from_trades` trên danh sách `4` lệnh đã đóng `[+50.0, -100.0, -100.0, -100.0]`.
  2. Kiểm tra trường `net_pnl_ex_top_3_winners` và `expectancy_ex_top_3_winners`.
- **Kết quả thực tế (Actual Result):**
  - Chỉ có `1` lệnh thắng (`+50`) bị loại bỏ, còn lại `3` lệnh lỗ với tổng `-300.0`, nhưng hàm vẫn chia cho `valid_n - 3 = 4 - 3 = 1` và báo **`expectancy_ex_top_3_winners = Some(-300.0)`** (sai gấp **`3x`** so với kỳ vọng đúng là **`-100.0`**).
- **Kết quả kỳ vọng / Chuẩn TradingView hoặc Quy chế VN (Expected Result):**
  - Mẫu số phải là `valid_n - removed_winners_count` (trong ví dụ là `4 - 1 = 3` -> `-300.0 / 3 = -100.0`).
- **Bằng chứng đính kèm (Evidence):**
  - Telemetry đối kháng: `[REPRO P1 OutlierImpact Wrong Denom] ex_top3_net=-300.00 reported_ex_top3_exp=Some(-300.0) (true expectancy of 3 remaining losers = -100.00!)`.
  - Mã nguồn gốc: `src/backtest/analytics.rs:426-437`.

---

### [BUG-017] Chuỗi sai lệch biên trong kiểm tra sức mua Scale-In và thời điểm mở khóa `T+2` / `T+1` theo khung thời gian
- **Mức độ nghiêm trọng (Severity):** `P1`
- **Phân hệ liên quan:** `Paper Trading / T+2`
- **Mã kịch bản (Test Case ID):** `TC-39`, `TC-40`, `TC-41`
- **Nguồn dữ liệu & Mã chứng khoán:** Tài khoản tiền mặt `1.0x`, dữ liệu `1D` khác `00:00 UTC`, `1W`, và `SPY CSV` (`UsEquities`)
- **Các bước tái hiện chính xác (Steps to Reproduce):**
  1. **Scale-in margin:** Trên tài khoản tiền mặt `1.0x`, mua một vị thế Long rồi giá tăng mạnh (`+50..80%`); tiếp tục mua thêm (scale-in) tận dụng phần vốn ảo.
  2. **`T+2` đa khung / `US T+1`:** Mua trên chuỗi `1D` VN có timestamp `> 06:00 UTC` (ví dụ mốc đóng cửa `08:00 UTC`), hoặc mua trên `1W` base, hoặc mua `SPY` (`UsEquities`, `T+1`) có timestamp `00:00 UTC` Thứ Hai.
  3. So sánh kết quả `fifo_holding_summary` với `available_settled_long_qty_at` tại cùng timestamp `1D` thô.
- **Kết quả thực tế (Actual Result):**
  - Kiểm tra margin khi scale-in cộng theo **giá vốn lịch sử** thay vì giá trị thị trường hiện tại (mark-to-market), cho phép mở tổng vị thế vượt `1.5x–1.8x` vốn chủ sở hữu trên tài khoản tiền mặt `1.0x`.
  - Thanh `1D` VN có timestamp `> 06:00 UTC` mở khóa `T+2` sớm ở `T+1`; thanh `W1` base mở khóa ngay `T+0`; `UsEquities` (`SPY` `00:00 UTC` Thứ Hai) bị dịch sai sang `T+2` Thứ Tư thay vì `T+1` Thứ Ba; `fifo_holding_summary` và `available_settled_long_qty_at` trả về hai con số khác nhau ở cùng timestamp `1D` thô.
- **Kết quả kỳ vọng / Chuẩn TradingView hoặc Quy chế VN (Expected Result):**
  - Kiểm tra ký quỹ phải tính theo giá thị trường (mark-to-market) + phí giao dịch.
  - Chuẩn hóa giờ hiệu lực theo cùng một hàm duy nhất giữa `fifo_holding_summary` và `available_settled_long_qty_at`, không mở khóa sớm `T+1` hay dịch nhầm `US T+1` sang `T+2`.
- **Bằng chứng đính kèm (Evidence):**
  - Mã nguồn gốc: `src/trading/portfolio.rs:1149-1162` (scale-in margin dùng `avg_entry_price`); `src/journal/market_rules.rs:763-807, 909-952` (chuẩn hóa timestamp & `T+1`/`T+2`).

---

### Nhóm `P2 — Major (UX / TradingView Parity / Data Accuracy)` (`14` Lỗi)

---

### [BUG-018] Nhãn trục giá `Alert` và `Drawing` bỏ qua bộ chống đè `Anti-Collision`, ghi đè lên nhãn `Close`/`Entry`/`TP` và thẻ PnL `LONG`
- **Mức độ nghiêm trọng (Severity):** `P2`
- **Phân hệ liên quan:** `Chart/Viewport` / `Alerts` / `Drawings`
- **Mã kịch bản (Test Case ID):** `TC-11`, `TC-14`
- **Nguồn dữ liệu & Mã chứng khoán:** `CafeF Adj (471 mã)` — `FPT · 1D` (với vị thế mở `LONG 200`, `Entry=50.81`, `TP=51.31`, `SL=49.80` và `3` cảnh báo `51.05`, `50.90`, `50.55`)
- **Các bước tái hiện chính xác (Steps to Reproduce):**
  1. Mở `FPT · 1D` ở độ phân giải `1280×800` hoặc `1440×1000`.
  2. Mở 1 vị thế `LONG 200` kèm `SL`/`TP` sát giá đóng cửa `50.80`, đồng thời đặt `3` cảnh báo giá gần nhau (`51.05`, `50.90`, `50.55`).
  3. Quan sát cột trục giá bên phải và khu vực nhãn lệnh trên biểu đồ.
- **Kết quả thực tế (Actual Result):**
  - Trên trục giá phải: `paint_alert_price_line_and_pill` chỉ dùng hàm `clamp` thô mà **không gọi `resolve_non_overlapping_axis_pill_y`**, khiến hộp nhãn giá `51.31`, `50.55` và nhãn `Alert` đè chồng lên nhau, che khuất hoàn toàn giá đóng cửa `50.80`, `Entry 50.81` và `Alert 51.05` (tương tự cho nhãn `H-Line`, `Pos R:R`, `FRVP`, `AVWAP` khi đang chọn).
  - Trên mặt biểu đồ: thẻ văn bản cảnh báo (`Alert ...`) đè trực tiếp lên nhãn `TP` và huy hiệu PnL `LONG 200`.
- **Kết quả kỳ vọng / Chuẩn TradingView hoặc Quy chế VN (Expected Result):**
  - Mọi nhãn giá trục phải (`Last`, `Entry`, `SL`, `TP`, `Alert`, `Drawing`) phải đi qua cùng một bộ xếp chồng chống đè `resolve_non_overlapping_axis_pill_y` theo thứ tự ưu tiên; nhãn đường ngang trên biểu đồ tự dịch ngang/dọc để không che thẻ vị thế.
- **Bằng chứng đính kèm (Evidence):**
  - Ảnh chụp: [`docs/testing/qa_evidence/07_tc11_tc14_both_docks_1280x800.png`](./qa_evidence/07_tc11_tc14_both_docks_1280x800.png) và [`docs/testing/qa_evidence/08_tc14_both_docks_1440x1000.png`](./qa_evidence/08_tc14_both_docks_1440x1000.png).
  - Mã nguồn gốc: `src/chart.rs:11569-11582`.

---

### [BUG-019] Hộp thoại `Templates` (và `Cmd+K`) bị mờ trong suốt xuyên thấu do thiếu `.fade_in(false)` và tràn chiều ngang lệch nút `Close (Esc)`
- **Mức độ nghiêm trọng (Severity):** `P2`
- **Phân hệ liên quan:** `Settings / Workspace`
- **Mã kịch bản (Test Case ID):** `TC-61`
- **Nguồn dữ liệu & Mã chứng khoán:** Toàn ứng dụng (`1280×800`)
- **Các bước tái hiện chính xác (Steps to Reproduce):**
  1. Bấm nút `Templates` trên thanh công cụ trên cùng ở độ phân giải `1280×800`.
  2. Quan sát nền hộp thoại, độ tương phản chữ và vị trí nút `Close (Esc)`.
- **Kết quả thực tế (Actual Result):**
  - Hộp thoại `Templates` hiển thị mờ ảo/xuyên thấu (`alpha < 1.0`) nhìn thấu cả nến và chữ trên biểu đồ nền phía sau do `egui::Area` mặc định bật `fade_in` mà không có `.fade_in(false)`.
  - Các dòng mô tả preset đặt trong hàng ngang không tự ngắt dòng làm khung hộp thoại giãn rộng vượt `modal_w`, khiến nút `Close (Esc)` bị đẩy lệch ra giữa khoảng trống thay vì căn mép phải tiêu đề.
- **Kết quả kỳ vọng / Chuẩn TradingView hoặc Quy chế VN (Expected Result):**
  - Mọi modal (`Templates`, `Cmd+K`...) phải đặt `.fade_in(false)` với nền đặc rõ ràng (như `[Settings]` và `[Data Audit]`), tự ngắt dòng mô tả trong khung `modal_w`, nút `Close (Esc)` căn phải gọn gàng.
- **Bằng chứng đính kèm (Evidence):**
  - Ảnh chụp: [`docs/testing/qa_evidence/22_tc61_templates_modal_1280x800.png`](./qa_evidence/22_tc61_templates_modal_1280x800.png).
  - Mã nguồn gốc: `src/app.rs:19032-19053` (`render_templates_modal`) và `src/app.rs:18816-18838` (`render_command_palette_modal`).

---

### [BUG-020] Watchlist `471` mã không ảo hóa `show_rows`, làm khựng `139.4 ms`/bước Replay, trộn `13` mã giả lập và vẽ đè `483` nhãn `RRG` thành mảng trắng đặc
- **Mức độ nghiêm trọng (Severity):** `P2`
- **Phân hệ liên quan:** `Scanner / ProxyBB` / `Bar Replay`
- **Mã kịch bản (Test Case ID):** `TC-48`, `TC-51`, `TC-64`
- **Nguồn dữ liệu & Mã chứng khoán:** `CafeF Adj (471 mã)`
- **Các bước tái hiện chính xác (Steps to Reproduce):**
  1. Nạp `CafeF Adj (471 mã)`, mở thanh trái `Watchlist / Scan`.
  2. Quan sát số lượng dòng (`Watch (484)`), kiểm tra mã `FPT`, bấm mở tab `Breadth` (`RRG` & `Liquidity Heatmap`).
  3. Bật `Bar Replay` và bấm bước `+1` nhiều lần khi Watchlist đang mở; đo thời gian mỗi bước và thời gian mở `Cmd+K` lần đầu.
- **Kết quả thực tế (Actual Result):**
  - **Không dùng ảo hóa `show_rows`:** Watchlist tạo đủ cả `484` widget thẻ mỗi khung hình, đẩy CPU lên `3.07–3.37 ms`/frame chỉ riêng thanh Watchlist.
  - **Giật khung hình nghiêm trọng (`~7 FPS`):** Mỗi lần bấm `+1` Bar Replay khi Watchlist mở phải quét lại toàn bộ `484` mã, gây **đứng hình `139.4 ms`/bước**; mở `Cmd+K` lần đầu khựng **`153.1 ms`**.
  - **Nhiễm bẩn dữ liệu giả lập:** Trộn `13` mã mẫu tích hợp vào cùng `471` mã CafeF -> có `2` dòng `FPT` (`File` & `VN30 Basket`), `Breadth` đếm trùng **`VN30 (16)`**, hiển thị ngày tương lai **`2026-11-13`** (vượt `2026-11-06` của CafeF), và `Liquidity Heatmap` báo **`FPT +2.52%`** (giả lập) ngay cạnh biểu đồ thật **`FPT -1.17%`**.
  - **Biểu đồ `RRG` hoàn toàn không đọc được:** Vẽ tất cả **`483` nhãn tên mã** mà không lọc top/culling/chống đè, tạo thành một khối chữ trắng đặc dính chặt nhau.
- **Kết quả kỳ vọng / Chuẩn TradingView hoặc Quy chế VN (Expected Result):**
  - Sử dụng `ScrollArea::show_rows` cho Watchlist (`< 0.5 ms`/frame); chỉ quét lại bất đồng bộ/tăng dần hoặc giới hạn nhóm theo dõi khi chạy Replay mượt `60 FPS`.
  - Khử trùng mã giả lập khi đang nạp `CustomFile`; biểu đồ `RRG` chỉ hiện nhãn cho top mã/mã đang hover/nhóm `VN30` hoặc chống đè nhãn.
- **Bằng chứng đính kèm (Evidence):**
  - Ảnh chụp: [`docs/testing/qa_evidence/15_tc51_breadth_rrg_proxybb_1440x1000.png`](./qa_evidence/15_tc51_breadth_rrg_proxybb_1440x1000.png) (RRG trắng đặc không thể đọc, `VN30(16)`, `2026-11-13`, `FPT +2.52%` vs `-1.17%`).
  - Telemetry: `[TC-48 Watchlist 471 Scan] total_rows=484 duplicate_FPT_rows=2`, `Watchlist UI Frame median=3.07ms p95=3.37ms`, `Replay +1 Step w/ 471 Watchlist Open time=139.4ms`, `Cmd+K first open=153.1ms`.
  - Mã nguồn gốc: `src/app.rs:13799-13804, 4710-4841`.

---

### [BUG-021] Phân loại nhầm ~250 mã `HNX`/`UPCOM` sang `HOSE`, thiếu lịch nghỉ lễ `2016–2017` và cảnh báo giả `EXTREME_RANGE > 25%`
- **Mức độ nghiêm trọng (Severity):** `P2`
- **Phân hệ liên quan:** `CafeF Data Loader` / `Paper Trading / T+2`
- **Mã kịch bản (Test Case ID):** `TC-04`, `TC-05`, `TC-41`
- **Nguồn dữ liệu & Mã chứng khoán:** `CafeF Adj (471 mã)` (`C69`, `KSV`, `NDN`, `BVS`, `API`, `APS`, `FPT`...)
- **Các bước tái hiện chính xác (Steps to Reproduce):**
  1. Nạp `CafeF Adj (471 mã)` và mở `[Data Audit]` trên `C69`, `KSV`, `NDN`, `BVS`, `API`, `APS`, `FPT`.
  2. Kiểm tra cột `Venue`, `Band Breaches`, `Missing Dates`, `Feeds Ready` và thử tính ngày về `T+3` cho lệnh mua `FPT` ngày `2016-02-05` (ngay trước Tết Nguyên Đán `2016`).
- **Kết quả thực tế (Actual Result):**
  - `ExchangeVenue::infer_from_symbol_and_preset` chỉ hardcode `8` mã `HNX` và `7` mã `UPCOM` -> toàn bộ ~250+ mã `HNX`/`UPCOM` còn lại trong CafeF (`C69`, `KSV`, `NDN`, `BVS`, `API`, `APS`...) bị nhận nhầm là **`HOSE (±7%)`**, gây **`56–266` cảnh báo sai vi phạm biên độ/mã**, sai bậc bước giá (`0.01/0.05` thay vì `0.10`), sai giá trần/sàn trên DOM, và khiến chỉ có **`12/471` mã** đạt `Ready`.
  - `VIETNAM_HOLIDAYS_V1` chỉ phủ `2018–2030`, bỏ trống `2016–2017` -> `FPT`/`NDN` bị báo `12` ngày thiếu giả (`2016-02-08..12` Tết 2016, `2016-04-18` Giỗ Tổ...) và lệnh mua `2016-02-05` mở khóa sớm `7` ngày giữa kỳ nghỉ Tết (`2016-02-10` thay vì `2016-02-17`).
  - Quy tắc `EXTREME_RANGE > 25%` báo lỗi giả các phiên tăng trần–giảm sàn hợp pháp của `UPCOM ±15%` (`35.29%` biên độ `H-L/L`) và các nến khung `1W`.
- **Kết quả kỳ vọng / Chuẩn TradingView hoặc Quy chế VN (Expected Result):**
  - Bổ sung bảng ánh xạ sàn đầy đủ cho cổ phiếu niêm yết `HOSE`/`HNX`/`UPCOM` (hoặc suy diễn từ biên độ lịch sử/cho phép chọn sàn theo mã); mở rộng `VIETNAM_HOLIDAYS_V1` về `2016–2017` (tương ứng lịch sử `CafeFData`); điều chỉnh ngưỡng `EXTREME_RANGE` theo sàn và khung thời gian.
- **Bằng chứng đính kèm (Evidence):**
  - Ảnh chụp: [`docs/testing/qa_evidence/01_tc04_data_audit_c69_1280x800.png`](./qa_evidence/01_tc04_data_audit_c69_1280x800.png) (`C69 · HOSE`, `161` band breaches, `12/471 feeds ready`, tất cả mã ma trận hiện `HOSE`).
  - Telemetry: `[TC-41 Tet 2016] buy=2016-02-05 -> settle_utc=2016-02-10 02:00 (expected_day=2016-02-17)`.
  - Mã nguồn gốc: `src/journal/market_rules.rs:116-128`; `src/data/audit.rs:548-566`.

---

### [BUG-022] Lưới đa biểu đồ `2x2` mất crosshair ở vùng trắng tương lai, `Sync T` méo trên `85%` whitespace và nạp mặc định `VNINDEX` hỏng (`-34.5%`) + `VCB` giả lập trống `95%`
- **Mức độ nghiêm trọng (Severity):** `P2`
- **Phân hệ liên quan:** `Chart/Viewport`
- **Mã kịch bản (Test Case ID):** `TC-59`
- **Nguồn dữ liệu & Mã chứng khoán:** Chế độ lưới `2x2` mặc định sau khi mở `FPT` (`CafeF Adj`)
- **Các bước tái hiện chính xác (Steps to Reproduce):**
  1. Đang mở `FPT` (`CafeF Adj`, `2016–2026`), chuyển sang bố cục lưới `2x2` với `Sync X` và `Sync T` bật.
  2. Rê con trỏ chuột sang vùng trắng bên phải nến cuối cùng (future whitespace / vùng mây Ichimoku `+26`) ở Ô 1.
  3. Quan sát dữ liệu hiển thị trên 4 ô `S1..S4`.
- **Kết quả thực tế (Actual Result):**
  - Con trỏ chữ thập đồng bộ (`Sync X`) **biến mất hoàn toàn** trên các ô phụ khi chuột rời ra vùng trắng tương lai (và `Sync T` bị méo khoảng thời gian khi vượt `85%` vùng trắng hoặc đếm trùng cuối tuần giữa lịch giao dịch khác nhau).
  - Ô mặc định **`S2: VNINDEX`** tự nạp tệp `ARCHIVE/test-results/p0b/data/CafeF.INDEX.clean.csv` có gai dữ liệu hỏng **`L = 1138.78` (`-34.5%` trong 1 ngày `2026-10-05`)** làm bẹp trục giá toàn bộ đồ thị.
  - Ô mặc định **`S3: VCB`** nạp nguồn giả lập `VN30 Basket` kết thúc `2026-07-24` (trong khi `S1` ở `2026-11-06`), khiến `Sync T` đẩy toàn bộ nến `VCB` sang mép trái, để **`95%` ô `S3` đen trống**.
- **Kết quả kỳ vọng / Chuẩn TradingView hoặc Quy chế VN (Expected Result):**
  - Crosshair đồng bộ phải chiếu tọa độ thời gian ước tính ra vùng trắng tương lai mượt mà (không biến mất hay dính cứng nến cuối).
  - Các ô mặc định trong `2x2` phải lấy mã từ chính nguồn `CafeF` đang mở (`VCB` thật trong `CustomFile`, `VNINDEX` thật từ `data/raw/cafef/CafeF.INDEX.Upto06112026.csv`).
- **Bằng chứng đính kèm (Evidence):**
  - Ảnh chụp: [`docs/testing/qa_evidence/17_tc59_quad_2x2_1440x1000.png`](./qa_evidence/17_tc59_quad_2x2_1440x1000.png).
  - Mã nguồn gốc: `src/chart.rs:3493-3495, 6687-6705`; `src/viewport/multi_chart.rs:213-260`.

---

### [BUG-023] Thanh trên cùng tính sai `%Chg` (`Close vs Open` thay vì `Close vs Prev Close`), cứng hóa ký hiệu `$` trên sàn VN và rớt `3–4` hàng ở `1280×800`
- **Mức độ nghiêm trọng (Severity):** `P2`
- **Phân hệ liên quan:** `Chart/Viewport` / `Settings / Workspace`
- **Mã kịch bản (Test Case ID):** `TC-01`, `TC-14`, `TC-03`
- **Nguồn dữ liệu & Mã chứng khoán:** `CafeF Adj (471 mã)` — `FPT · 1D` (ví dụ ngày nhảy Gap `2017-05-25`) ở `1280×800`
- **Các bước tái hiện chính xác (Steps to Reproduce):**
  1. Mở ứng dụng ở `1280×800`, nạp `FPT` (`CafeF Adj`).
  2. Kiểm tra `%Chg` trên thanh tiêu đề ngày `2017-05-25` (`prev_close = 9.6074`, `open = 9.8728`, `close = 9.7847`) so với Watchlist.
  3. Quan sát thanh trạng thái dưới cùng (`EQ:`), thẻ Paper Trading và DOM Ladder; quan sát số dòng của thanh công cụ trên cùng; gõ lọc mã trong menu chọn mã rồi chọn `VCB`, sau đó mở lại menu đó.
- **Kết quả thực tế (Actual Result):**
  - **Sai công thức `%Chg`:** Thanh tiêu đề tính `(close - open) / open = -0.89%` (màu đỏ) trong khi thực tế phiên đó tăng **`+1.85%` (màu xanh)** so với giá tham chiếu phiên trước (`prev_close`), mâu thuẫn trực tiếp với Watchlist và chuẩn TradingView (xảy ra ngược dấu trên `6` phiên nhảy Gap của `FPT`).
  - **Sai ký hiệu tiền tệ:** Khắp thanh trạng thái (`EQ: $100000`), Paper Trading và DOM đều hiển thị ký hiệu đô la Mỹ **`$`** ngay cả khi đang giao dịch cổ phiếu `HOSE`/`VN30F1M` tính bằng `kVND`/`VND`.
  - **Bố cục rớt dòng ở `1280×800`:** Thanh công cụ trên cùng rớt thành `3–4` hàng (`149px+`), đẩy `Cmd+K`, `✓ Volume`, `-> Order Ticket` rơi xuống hàng mồ côi chiếm diện tích đồ thị.
  - Ô `"Filter mã..."` trong dropdown chọn mã không tự xóa chuỗi lọc sau khi chọn xong mã.
- **Kết quả kỳ vọng / Chuẩn TradingView hoặc Quy chế VN (Expected Result):**
  - `%Chg` luôn tính `(close - prev_close) / prev_close` (chỉ dùng `open` ở thanh đầu tiên `idx == 0`); ký hiệu tiền tệ theo `venue` (`kVND` / `₫` cho sàn VN, `$` cho US/Crypto); thanh công cụ bố trí tối đa `2` hàng gọn gàng ở `1280×800`; tự xóa `symbol_picker_filter` sau khi chọn mã.
- **Bằng chứng đính kèm (Evidence):**
  - Ảnh chụp: [`00_tc01_default_startup_1280x800.png`](./qa_evidence/00_tc01_default_startup_1280x800.png), [`07_tc11_tc14_both_docks_1280x800.png`](./qa_evidence/07_tc11_tc14_both_docks_1280x800.png), [`09_tc16_19_vn30f_15m_indicators_1440x1000.png`](./qa_evidence/09_tc16_19_vn30f_15m_indicators_1440x1000.png), [`12_tc22_26_drawings_inspector_textnote_1280x800.png`](./qa_evidence/12_tc22_26_drawings_inspector_textnote_1280x800.png).
  - Telemetry: `[TOP-BAR %CHG AUDIT FPT] gap_opposite_sign_days=6 sample=("2017-05-25", prev=9.6074, open=9.8728, close=9.7847, true=+1.85%, shown=-0.89%)`.
  - Mã nguồn gốc: `src/app.rs:10306-10323, 10388-10435, 11368, 12306, 12741`; `src/model.rs:29-41`.

---

### [BUG-024] DOM Ladder lỗi sai số chuyển bậc giá `10.00 kVND` trên `HOSE`, đẩy toàn bộ vùng Bid khuất màn hình ở `1280×800` và định cỡ rủi ro chặn đáy tại vốn gốc
- **Mức độ nghiêm trọng (Severity):** `P2`
- **Phân hệ liên quan:** `Paper Trading / T+2`
- **Mã kịch bản (Test Case ID):** `TC-43`, `TC-14`
- **Nguồn dữ liệu & Mã chứng khoán:** Cổ phiếu `HOSE` quanh mốc `10.00 kVND` (`ref = 9.34`, `mark = 9.99`) và `VN30F1M` tại `1280×800`
- **Các bước tái hiện chính xác (Steps to Reproduce):**
  1. Dựng DOM Ladder trên cổ phiếu `HOSE` với giá tham chiếu `ref = 9.34`, giá hiện tại `mark = 9.99` (sát ranh giới chuyển bước giá `0.01 <-> 0.05` tại `10.00 kVND`).
  2. Kiểm tra các dòng được gắn cờ `CEIL` và `is_at_mark`, sau đó nhấp đặt lệnh Mua (`Buy`) tại bậc giá `10.00` (`> mark 9.99`).
  3. Mở DOM Ladder trên `VN30F1M` ở độ phân giải `1280×800` và quan sát phần hiển thị cột giá.
- **Kết quả thực tế (Actual Result):**
  - Do dung sai `tick_size` không cập nhật đúng hai phía ranh giới `10.00`: sinh trùng **`2` dòng `CEIL` (`10.00` và `9.99`)**, trùng **`2` dòng `is_at_mark` (`10.00` và `9.98`)**, và nhấp Mua tại `10.00` (cao hơn giá thị trường `9.99`) lại tạo lệnh **`Buy Limit @ 10.00`** (khớp ngay) thay vì **`Buy Stop @ 10.00`**.
  - Ở `1280×800`, phần tiêu đề/nút bấm DOM chiếm quá nhiều chiều cao khiến **dòng giá hiện tại (Mark) và toàn bộ `10` bậc Bid rơi xuống dưới mép màn hình** (người dùng không nhìn thấy giá hiện tại mà phải cuộn thủ công).
  - Hàm định cỡ rủi ro DOM kẹp sàn vốn bằng `initial_cash` (`equity.max(initial_cash)`), khiến khi tài khoản đang lỗ sâu (drawdown) vẫn đặt cỡ lệnh lớn như chưa lỗ.
- **Kết quả kỳ vọng / Chuẩn TradingView hoặc Quy chế VN (Expected Result):**
  - So sánh dung sai từng bậc theo bước giá cục bộ tại đúng mức giá đó (`<= 1e-6`), chỉ có đúng `1` dòng `CEIL`, `1` dòng `Mark`, Mua trên Mark phải là `Buy Stop`.
  - DOM Ladder tự căn giữa dòng `Mark` trong vùng hiển thị tại `1280×800`; định cỡ rủi ro luôn dựa trên vốn chủ sở hữu thực tế (`equity`).
- **Bằng chứng đính kèm (Evidence):**
  - Ảnh chụp: [`docs/testing/qa_evidence/14_tc43_vn30f_dom_ladder_1280x800.png`](./qa_evidence/14_tc43_vn30f_dom_ladder_1280x800.png) (toàn bộ vùng nhìn chỉ thấy Ask, mất hoàn toàn Mark và Bid).
  - Telemetry: `[REPRO P2 DOM Ladder HOSE 10.00 Tier Bug] ref=9.34 ceil_band=Some(9.99) ceil_rows=[10.0, 9.99] mark_rows=[10.0, 9.98] buy@10.00_when_mark=9.99=Ok(Limit { limit_price: 10.0 })`.
  - Mã nguồn gốc: `src/trading/ladder.rs:404-417, 557-563, 689, 851-868`; `src/app.rs:4200-4204`.

---

### [BUG-025] Dữ liệu tích hợp `VNINDEX Synth` trôi lịch `45 ngày..110 năm` (chứa cuối tuần/ngày lễ) và `VN30F1M 1m` sinh giao dịch đúng lễ Quốc khánh `01–02/09/2026`
- **Mức độ nghiêm trọng (Severity):** `P2`
- **Phân hệ liên quan:** `CafeF Data Loader`
- **Mã kịch bản (Test Case ID):** `TC-01`, `TC-15`
- **Nguồn dữ liệu & Mã chứng khoán:** `VNINDEX Synth (120B..100K)` và `VN30F1M 1m (5.400 nến)`
- **Các bước tái hiện chính xác (Steps to Reproduce):**
  1. Mở ứng dụng ở trạng thái mặc định (`VNINDEX Synth 120B`) và mở `[Data Audit]`, đối chiếu `timestamp` với `date_label`.
  2. Chuyển sang `VN30F1M 1m` (`5.400` nến) và kiểm tra ngày bắt đầu cùng báo cáo `hol_wknd_bars` trong `[Data Audit]`.
- **Kết quả thực tế (Actual Result):**
  - `VNINDEX Synth` tăng `timestamp` đều `+86.400 s` mỗi thanh (tính cả Thứ Bảy, Chủ Nhật, lễ -> **`35` thanh cuối tuần/ngày lễ** trong `120` nến), trong khi `date_label` lại bỏ qua cuối tuần và bắt đầu lệch sớm `3` ngày (`2026-05-01` — ngày Quốc tế Lao động) -> trôi **`45` ngày** ở `120B` (`timestamp = 2026-08-31` nhưng nhãn ghi `2026-10-15`) và trôi tới **`110` năm** ở `100K`.
  - `VN30F1M 1m` khởi đầu đúng 2 ngày nghỉ lễ Quốc khánh chính thức **`2026-09-01` & `2026-09-02`** (`510` nến giao dịch trong ngày nghỉ lễ!), nhưng `[Data Audit]` lại báo `hol_wknd_bars = 0, status = Ready` do bỏ sót kiểm tra ngày lễ trên chuỗi intraday.
- **Kết quả kỳ vọng / Chuẩn TradingView hoặc Quy chế VN (Expected Result):**
  - `timestamp` và `date_label` phải đồng nhất từ cùng một nguồn ngày giao dịch `VIETNAM_HOLIDAYS_V1`; `VN30F1M 1m` bỏ qua `01–02/09/2026`; `[Data Audit]` phát hiện ngày lễ VN trên cả khung intraday.
- **Bằng chứng đính kèm (Evidence):**
  - Ảnh chụp: [`docs/testing/qa_evidence/14_tc43_vn30f_dom_ladder_1280x800.png`](./qa_evidence/14_tc43_vn30f_dom_ladder_1280x800.png) (`VN30F1M` hiển thị `2026-09-01 09:00`).
  - Telemetry: `[DEFAULT VNINDEX Synth Audit] bars=120 hol_wknd=35 first="2026-05-01" last="2026-10-15" ts_first=2026-05-04 ts_last=2026-08-31`; `[TC-15 VN30F1M Audit] bars=5400 hol_wknd_bars=0 earliest="2026-09-01"`.
  - Mã nguồn gốc: `src/data/synthetic.rs:13-18, 69-70`; `src/data/presets.rs:303-310`.

---

### [BUG-026] `TextNote` cắt cụt ghi chú `280` ký tự hợp lệ tại `<= 272` ký tự và nhãn Custom Fibonacci (`OTE`/`Full 13`) chồng đè lên nhau
- **Mức độ nghiêm trọng (Severity):** `P2`
- **Phân hệ liên quan:** `Drawings` / `Chart/Viewport`
- **Mã kịch bản (Test Case ID):** `TC-24`, `TC-26`
- **Nguồn dữ liệu & Mã chứng khoán:** `CafeF Adj (471 mã)` — `FPT · 1D`
- **Các bước tái hiện chính xác (Steps to Reproduce):**
  1. Vẽ `Fib` trên một vùng dao động hẹp, mở `Inspect` chọn `ICT OTE 9` hoặc `Full 13`, thêm mức `0.79`, kết hợp bật `SMA`/`EMA`/`FRVP`.
  2. Vẽ `TextNote` và nhập chuỗi tiếng Việt đúng `280` ký tự (ngưỡng cho phép trong `Drawing Inspector`).
- **Kết quả thực tế (Actual Result):**
  - `wrap_text_note_lines(text, 34, 8)` giới hạn tối đa `34 × 8 = 272` ký tự (thấp hơn trần nhập `280`), tự cắt cụt phần cuối ghi chú hợp lệ và tràn/cắt ngang các ký tự rộng/emoji.
  - Các nhãn mức tỷ lệ Fib (`0.786`, `0.79`, `0.705`, `0.618`, `0.5`) không có bộ giãn cách dọc, xếp chồng trực tiếp lên nhau và đè lên nhãn `SMA`/`EMA`/`FRVP POC`.
- **Kết quả kỳ vọng / Chuẩn TradingView hoặc Quy chế VN (Expected Result):**
  - Hộp `TextNote` hiển thị đầy đủ `280` ký tự bằng đo bề rộng pixel thực tế (`galley` layout của `egui`), không cắt mất chữ người dùng đã nhập; nhãn mức Fib áp dụng giãn cách/lược bớt khi khoảng cách pixel giữa hai mức `< 14 px`.
- **Bằng chứng đính kèm (Evidence):**
  - Ảnh chụp: [`docs/testing/qa_evidence/12_tc22_26_drawings_inspector_textnote_1280x800.png`](./qa_evidence/12_tc22_26_drawings_inspector_textnote_1280x800.png).
  - Mã nguồn gốc: `src/drawings/geometry.rs:831-842`.

---

### [BUG-027] Lỗi bất ổn số học `NaN`/`±Inf` ở các chỉ báo biên, đầu độc `CandleBlock` khi gặp `NaN` volume, làm phẳng chế độ `%`/`Idx100` và rung nến khi kéo ngang
- **Mức độ nghiêm trọng (Severity):** `P2`
- **Phân hệ liên quan:** `Indicators` / `Chart/Viewport`
- **Mã kịch bản (Test Case ID):** `TC-10`, `TC-13`, `TC-16`, `TC-19`
- **Nguồn dữ liệu & Mã chứng khoán:** Dữ liệu ca biên (`volume = NaN`, giá trị cực lớn `1e308`, `close == 0` ở nến đầu khung nhìn, chế độ gộp `conflation >= 2x`)
- **Các bước tái hiện chính xác (Steps to Reproduce):**
  1. Đưa giá trị cực trị hoặc phẳng vào `MFI`, `Volume SMA`, `SuperTrend`, `Keltner`, `CCI`.
  2. Chèn `1` thanh có `volume = NaN` vào chuỗi `100K` nến hoặc `close = 0 / NaN` ở mép trái khung nhìn khi bật `%` / `Idx100`.
  3. Bật `SuperTrend`/`PSAR`/`CVD` trên chuỗi lớn đã gộp (`conflation >= 4x`) rồi kéo ngang từng `1` thanh nến; hoặc bật `BB`/`Keltner`/`VWAP` qua khoảng trống `NaN`.
- **Kết quả thực tế (Actual Result):**
  - `MFI`, `Volume SMA`, `SuperTrend`, `Keltner`, `CCI` xuất giá trị không hữu hạn (`NaN`/`±Inf`) thay vì `None`.
  - Một giá trị `volume = NaN` duy nhất làm hỏng toàn bộ `CandleBlock` `16` nến và thanh nến gộp conflated.
  - Nếu nến đầu tiên bên trái khung nhìn có `close == 0` hoặc `NaN`, toàn bộ đồ thị `%`/`Idx100` sập thành đường thẳng ngang (không tìm nến hợp lệ kế tiếp làm gốc).
  - Ở chế độ gộp `conflation`, `SuperTrend`/`PSAR`/`CVD` bị nhấp nháy/rung (`shimmer`) mỗi khi kéo lệch `1` thanh do pha lấy mẫu dịch theo `start_idx`, còn dải băng `BB`/`Keltner`/`VWAP` vẽ đường nối xuyên qua lỗ hổng `NaN`.
- **Kết quả kỳ vọng / Chuẩn TradingView hoặc Quy chế VN (Expected Result):**
  - Chặn lọc `.is_finite()` ở mọi phép tính chỉ báo và tổng hợp volume; chế độ `%`/`Idx100` tự chọn nến hữu hạn dương đầu tiên làm gốc; căn pha lấy mẫu gộp theo lưới cố định (`idx % step == 0`) và ngắt dải băng khi gặp `None`.
- **Bằng chứng đính kèm (Evidence):**
  - Mã nguồn gốc: `src/indicators/extended.rs`; `src/indicators/mod.rs:737-741`; `src/viewport/candle_metrics.rs:149-153`; `src/chart.rs:4637-4892, 6516, 9858-9860`.

---

### [BUG-028] Cảnh báo tạo ở chế độ Live (`created_at_timestamp == None`) bị lộ trong Bar Replay quá khứ
- **Mức độ nghiêm trọng (Severity):** `P2`
- **Phân hệ liên quan:** `Alerts` / `Bar Replay`
- **Mã kịch bản (Test Case ID):** `TC-33`, `TC-52`
- **Nguồn dữ liệu & Mã chứng khoán:** `CafeF Adj (471 mã)` — `FPT · 1D`
- **Các bước tái hiện chính xác (Steps to Reproduce):**
  1. Ở chế độ Live (tháng `11/2026`), tạo một cảnh báo giá quanh vùng đỉnh `55.00` của năm `2026`.
  2. Bật `Bar Replay` và cắt lùi về năm `2022` (giá ~`25.00`).
- **Kết quả thực tế (Actual Result):**
  - `AlertRule::is_causally_visible` trả về `true` khi `created_at_timestamp == None`, nên cảnh báo tạo ở năm `2026` vẫn xuất hiện trong danh sách và trên biểu đồ khi đang luyện tập ở năm `2022`, làm lộ vùng giá tương lai.
- **Kết quả kỳ vọng / Chuẩn TradingView hoặc Quy chế VN (Expected Result):**
  - Mọi cảnh báo mới tạo luôn ghi nhận `created_at_timestamp = Some(latest_bar_timestamp)` (kể cả ở chế độ Live) để khi tua lùi Bar Replay về trước thời điểm tạo, cảnh báo tương lai được ẩn sạch như công cụ vẽ (`TC-33`).
- **Bằng chứng đính kèm (Evidence):**
  - Mã nguồn gốc: `src/alerts/mod.rs:250-255`.

---

### [BUG-029] Các sai lệch nghiệp vụ trong `Breadth/RRG`, `VN30-PIT`, `Backtest` giá `<= 0` và bảng lợi nhuận theo năm sau phá sản
- **Mức độ nghiêm trọng (Severity):** `P2`
- **Phân hệ liên quan:** `Strategy Lab` / `Scanner / ProxyBB`
- **Mã kịch bản (Test Case ID):** `TC-34`, `TC-51`, `TC-54`, `TC-57`, `TC-58`
- **Nguồn dữ liệu & Mã chứng khoán:** `CafeF Adj (471 mã)` (`1D` & `1W`)
- **Các bước tái hiện chính xác (Steps to Reproduce):**
  1. Trong Bar Replay với `exclude_incomplete_tail = true`, mở `Breadth / RRG`.
  2. Chạy `Portfolio` ở chế độ `Point-in-Time (VN30-PIT)` với hộp chọn `VN30 Only` trên `CafeF Adj (471 mã)` (cả khung `1D` và `1W`).
  3. Chạy `compute_yearly_breakdown` trên đường cong vốn chạm phá sản (`equity <= 0`) ở năm đầu, và kiểm tra nhánh thoát lệnh/đảo chiều trong `backtest/engine.rs` khi thanh thoát có `open <= 0` hoặc `NaN`.
- **Kết quả thực tế (Actual Result):**
  - `compute_market_breadth_and_rrg` cắt nến 2 lần liên tiếp (tụt mất `1` phiên đã đóng hợp lệ trong Replay).
  - `run_active_portfolio_backtest` lờ đi bộ lọc `VN30 Only` khi đang nạp `CustomFile` (`471` mã), và trên khung `W1` có lệch ngày `+6` ngày giữa `bar.open` (Thứ Hai) và `bar.close` (Chủ Nhật) làm rơi mất P&L của tuần bị loại rổ `UniverseRemoval`.
  - Các điểm thoát lệnh/đảo chiều/thanh lý `PIT` trong `backtest/engine.rs` không kiểm tra `.is_finite() && > 0.0`, có thể khớp thoát ở giá `1e-9`.
  - `compute_yearly_breakdown` báo `-100.0%` giả cho mọi năm sau khi tài khoản đã phá sản (`equity == 0`).
- **Kết quả kỳ vọng / Chuẩn TradingView hoặc Quy chế VN (Expected Result):**
  - Cắt Replay đúng 1 lần duy nhất; áp dụng đúng bộ lọc `VN30 Only` trên cả `CustomFile`; thống nhất mốc ngày `W1` PIT; bảo vệ giá khớp `> 0.0 && is_finite()`; các năm sau phá sản báo `0.0%` hoặc `N/A`.
- **Bằng chứng đính kèm (Evidence):**
  - Mã nguồn gốc: `src/scanner/breadth.rs:197-205`; `src/app.rs:7561`; `src/backtest/engine.rs:974, 1078, 1466, 1491, 1597`; `src/backtest/walk_forward.rs:914`; `src/backtest/analytics.rs:669-671`.

---

### [BUG-030] Lệch đồng bộ trạng thái ở `SessionDebrief`, `PracticeSessionStore`, đổi `Default Venue` trong `[Settings]` và thổi phồng `TradeJournal` khi thêm SL sau vào lệnh
- **Mức độ nghiêm trọng (Severity):** `P2`
- **Phân hệ liên quan:** `Paper Trading / T+2` / `Settings / Workspace`
- **Mã kịch bản (Test Case ID):** `TC-42`, `TC-44`, `TC-45`, `TC-47`
- **Nguồn dữ liệu & Mã chứng khoán:** `CafeF Adj (471 mã)` — `FPT · 1D`
- **Các bước tái hiện chính xác (Steps to Reproduce):**
  1. Vào 1 lệnh Long **không điền SL**, sau khi khớp bấm `UpdateBrackets` gắn `SL > Entry` (khóa lãi), đóng lệnh rồi kiểm tra `TradeJournalStore::checklist.stop_and_lot_sized` và `r_multiple`.
  2. Tua lùi Replay về quá khứ rồi bấm `[Debrief]` (kiểm tra KPI tổng vs bảng phân rã theo Setup/Mistake) và bấm `Save` phiên thực hành rồi `Restore`.
  3. Mở `[Settings]`, đổi `Default Venue` (ví dụ `HOSE -> UsEquities`), bấm `Apply` rồi thử lệnh `SL/TP` tự động so với bán tay.
- **Kết quả thực tế (Actual Result):**
  - `UpdateBrackets` với `SL > Entry` trên lệnh mở không có SL ban đầu lại tạo `r_multiple` giả và nâng `TradeJournalStore` `checklist.stop_and_lot_sized = true` (mâu thuẫn với kỷ luật bất biến `TC-44`).
  - `SessionDebriefReport::build` tính KPI đầu trang trên toàn bộ lệnh chưa lọc `cutoff`, trong khi bảng phân rã Setup/Mistake lại lọc theo `cutoff` -> số liệu đầu trang và cuối trang mâu thuẫn nhau.
  - `save_current_practice_session` không lọc `PracticeDecisionNote` tương lai sau khi tua lùi; `restore_saved_practice_session` không đồng bộ lại `settings.enforce_vn_t2_lot_lock`.
  - Đổi `Default Venue` trong `[Settings]` bỏ sót gọi `sync_trading_settlement_rules()`, khiến bộ kiểm tra bán tay và bộ `Auto-SL/TP` áp dụng hai sàn giao dịch khác nhau.
- **Kết quả kỳ vọng / Chuẩn TradingView hoặc Quy chế VN (Expected Result):**
  - `TradeJournalStore` giữ bất biến cờ SL ban đầu giống `PracticeScoreboard`; `SessionDebrief` lọc `cutoff` nhất quán 100%; `Save/Restore` và `[Settings] Apply` luôn đồng bộ lại `sync_trading_settlement_rules()` và cắt ghi chú `<= cutoff`.
- **Bằng chứng đính kèm (Evidence):**
  - Mã nguồn gốc: `src/trading/portfolio.rs:1074-1082`; `src/journal/ledger.rs:534`; `src/trading/coaching.rs:422, 459`; `src/app.rs:3489, 3550, 21557, 21569`.

---

### [BUG-031] Không nhất quán chính sách `QuarantineRow` ở bộ nạp CSV/JSON, trôi tổng RVOL trong `DataQualityAudit` và tràn số nguyên trên `ReplayController`
- **Mức độ nghiêm trọng (Severity):** `P2`
- **Phân hệ liên quan:** `CafeF Data Loader` / `Bar Replay`
- **Mã kịch bản (Test Case ID):** `TC-05`, `TC-06`
- **Nguồn dữ liệu & Mã chứng khoán:** Tệp CSV/JSON ca biên (`leading blank lines`, dòng thiếu cột, timestamp hỏng trong đa mã, trùng lặp timestamp)
- **Các bước tái hiện chính xác (Steps to Reproduce):**
  1. Nạp tệp CSV 1 mã có dòng ngắn/trống hoặc dòng trắng đầu tệp với `InvalidRowPolicy::QuarantineRow`; nạp tệp CSV đa mã có 1 dòng timestamp số hỏng; nạp tệp JSON có 1 phần tử lỗi với `QuarantineRow`.
  2. Chạy `audit_ohlcv_series` trên chuỗi dài có nhiều giá trị lặp và ngày cách xa; kiểm tra `ReplayController::materialize_candles` với `base_cursor_index == usize::MAX`.
- **Kết quả thực tế (Actual Result):**
  - `csv_loader.rs`: chế độ đơn mã dừng lỗi toàn bộ trên dòng ngắn/trống thay vì quarantine; chế độ đa mã gặp 1 dòng timestamp số sai làm rơi mất cả mã đó; `raw_row_count` có thể nhỏ hơn `quarantined_row_count`; dòng trắng đầu tệp làm hỏng tự nhận diện dấu phân cách.
  - `json_loader.rs`: bỏ qua hoàn toàn `InvalidRowPolicy::QuarantineRow`.
  - `DataQualityAuditReport`: tổng trượt RVOL tích lũy sai số dấu phẩy động trên chuỗi dài; dòng trùng timestamp không hạ cấp trạng thái `Ready`; vòng lặp đếm ngày lịch thiếu cận trên bảo vệ.
  - `ReplayController::materialize_candles`: tràn số học `base_cursor_index + 1` khi `base_cursor_index == usize::MAX`.
- **Kết quả kỳ vọng / Chuẩn TradingView hoặc Quy chế VN (Expected Result):**
  - Áp dụng `QuarantineRow` đồng nhất ở mọi nhánh CSV/JSON; bỏ qua dòng trống đầu tệp khi dò delimiter; hạ trạng thái audit khi có trùng lặp; dùng `saturating_add(1)` ở `ReplayController`.
- **Bằng chứng đính kèm (Evidence):**
  - Mã nguồn gốc: `src/data/csv_loader.rs`; `src/data/json_loader.rs`; `src/data/audit.rs`; `src/replay/controller.rs:504`.

---

### Nhóm `P3 — Minor / Cosmetic` (`3` Lỗi Tổng Hợp)

---

### [BUG-032] Sai lệch đóng gói `.app` độc lập và các lỗi hiển thị/thông báo nhỏ trên giao diện
- **Mức độ nghiêm trọng (Severity):** `P3`
- **Phân hệ liên quan:** `Settings / Workspace` / `Scanner / ProxyBB` / `Drawings` / `Indicators`
- **Mã kịch bản (Test Case ID):** `TC-01`, `TC-16`, `TC-26`, `TC-50`, `TC-56`, `TC-60`
- **Nguồn dữ liệu & Mã chứng khoán:** Toàn ứng dụng (`dist/Sumi Workstation.app`)
- **Các bước tái hiện chính xác (Steps to Reproduce):**
  1. Kiểm tra `dist/Sumi Workstation.app/Contents/Info.plist`, `Contents/Resources/` và chép `.app` sang `/tmp/StandaloneSumiTest/` rồi mở.
  2. Mở `Cmd+K`, `Tab Sig` (`471` mã), `[Keys]`, `Signal Catalog 101`, `Multi-Strategy`, bật `Ichimoku + PAT`, chọn `TextNote`.
- **Kết quả thực tế (Actual Result):**
  - `Info.plist` ghi `CFBundleShortVersionString = 1.0.0` lệch `Cargo.toml` (`0.1.0`); thư mục `Contents/Resources/` rỗng nên nếu người dùng kéo `.app` vào `/Applications/` sẽ mất các nút truy cập nhanh `CafeF Adj/Raw/Index` (phụ thuộc đường dẫn compile-time `CARGO_MANIFEST_DIR`).
  - Phụ đề `Cmd+K` ghi cứng lỗi thời `"144 dates"` thay vì `151` ngày lễ (`src/app.rs:8556`).
  - Tab `Sig` hiện tiêu đề `Sig (8026)` nhưng âm thầm cắt hiển thị ở `64` dòng mà không có thông báo phân trang (`src/app.rs:14073`).
  - Hộp thoại `[Keys]` âm thầm bỏ qua thông báo phản hồi khi gán phím xung đột (`src/app.rs:19699`).
  - Nút `-> Order Ticket` trên thanh công cụ vẽ vẫn sáng khả dụng khi đang chọn `TextNote` (không có mức giá `Entry/SL/TP`).
  - `Signal Catalog 101` hiển thị song ngữ EN/VI dính liền thay vì có nút chuyển ngôn ngữ EN/VI rõ ràng (`TC-50`).
  - Chú giải `ICH` và `Multi-Strategy` bị cắt cụt chữ khi thu hẹp khung; nhãn mẫu hình nến `PAT` (`DCC`/`TWT`) sát đỉnh chạm viền trên của khung biểu đồ.
- **Kết quả kỳ vọng / Chuẩn TradingView hoặc Quy chế VN (Expected Result):**
  - Đồng bộ version `Info.plist` & `Cargo.toml`, hỗ trợ thư mục cấu hình/dữ liệu người dùng ngoài cây repo; sửa chuỗi `"151 dates"`, chú thích `"Hiển thị 64 / 8026"`, hiển thị thông báo xung đột phím tắt, vô hiệu hóa `-> Order Ticket` cho `TextNote`, thêm nút chuyển EN/VI ở `Signal Catalog`, đệm `padding` trên đỉnh cho nhãn `PAT`.
- **Bằng chứng đính kèm (Evidence):**
  - Ảnh chụp: [`07_tc11_tc14`](./qa_evidence/07_tc11_tc14_both_docks_1280x800.png), [`09_tc16_19`](./qa_evidence/09_tc16_19_vn30f_15m_indicators_1440x1000.png), [`12_tc22_26`](./qa_evidence/12_tc22_26_drawings_inspector_textnote_1280x800.png), [`16_tc56_57`](./qa_evidence/16_tc56_57_multistrat_0pct_slices_1440x1000.png).
  - Mã nguồn gốc: `dist/Sumi Workstation.app/Contents/Info.plist`; `src/app.rs:8556, 14073, 19699`.

---

### [BUG-033] Định dạng số gọn (`0K`, thiếu bậc `T`, `+0.00%`), nhãn `FRVP POC` không kẹp đỉnh và `WorkspaceSlotTemplate` thiếu `invert_scale`
- **Mức độ nghiêm trọng (Severity):** `P3`
- **Phân hệ liên quan:** `Chart/Viewport` / `Settings / Workspace`
- **Mã kịch bản (Test Case ID):** `TC-10`, `TC-19`, `TC-61`
- **Nguồn dữ liệu & Mã chứng khoán:** Cổ phiếu thanh khoản `< 1.000` cp, chuỗi `CVD >= 1T`, chế độ `%`, lưu Template khi bật `Inv`
- **Các bước tái hiện chính xác (Steps to Reproduce):**
  1. Mở mã có khối lượng `< 1.000` (như `500` cp) hoặc `>= 1.000.000.000` và xem HUD OHLCV; xem trục phải `CVD` khi tích lũy vượt `1T`.
  2. Bật chế độ `%` xem gốc `0.0`; vẽ `FRVP` sát mép trên; bật `Inv` rồi lưu & mở lại `WorkspaceTemplate`.
- **Kết quả thực tế (Actual Result):**
  - `paint_main_ohlcv_hud` hiển thị volume `< 1.000` là **`0K`** và `>= 1B` là `1234.56M`; `format_compact_signed` thiếu bậc **`T` (Trillion)**; chế độ `%` hiển thị `0.0` là **`+0.00%`**; nhãn `FRVP POC` không kẹp `plot_rect.top()`; `WorkspaceSlotTemplate` không lưu trạng thái `invert_scale`.
- **Kết quả kỳ vọng / Chuẩn TradingView hoặc Quy chế VN (Expected Result):**
  - Định dạng chính xác `< 1K` giữ nguyên số nguyên, hỗ trợ đủ bậc `K/M/B/T`; `0.00%` không dấu `+`; kẹp biên dọc nhãn `FRVP POC`; lưu đầy đủ `invert_scale` trong Template.
- **Bằng chứng đính kèm (Evidence):**
  - Mã nguồn gốc: `src/chart.rs:6034, 8534-8538`.

---

### [BUG-034] Các ca biên nhỏ ở lịch lễ sau `2030`, lệnh giảm vị thế DOM, trùng nhãn lưu phiên `>80B`, làm tròn `SL Adh`, `VN30-PIT`, metadata `rsi_overbought` và `Markdown`
- **Mức độ nghiêm trọng (Severity):** `P3`
- **Phân hệ liên quan:** `Paper Trading / T+2` / `Strategy Lab` / `Alerts` / `Scanner / ProxyBB`
- **Mã kịch bản (Test Case ID):** `TC-41`, `TC-45`, `TC-47`, `TC-50`, `TC-58`
- **Nguồn dữ liệu & Mã chứng khoán:** Các ca biên kiểm thử đối kháng
- **Các bước tái hiện chính xác (Steps to Reproduce):**
  1. Kiểm tra lịch nghỉ lễ năm `2033` (khi `02/09/2033` rơi Thứ Sáu); kiểm tra `stage_ladder_order` cho lệnh bán thuần giảm vị thế trên `HOSE`.
  2. Lưu phiên/snapshot với tên dài `> 80` byte; kiểm tra hiển thị `SL Adherence = 79.6%`; kiểm tra metadata `technical.rsi_overbought_cross`; xuất Markdown với tên mã chứa `\n`; tạo cảnh báo tại timestamp `0` (`1970-01-01`).
- **Kết quả thực tế (Actual Result):**
  - Fallback ngày lễ sau `2030` thiếu ngày nghỉ Quốc khánh thứ 2 khi `02/09` là Thứ Sáu (`2033-09-02`).
  - `stage_ladder_order` vẫn gắn bracket SL/TP chiều Short vào lệnh bán chỉ nhằm giảm vị thế Long trên `HOSE`.
  - `PracticeSessionStore` / `ScanSnapshotStore` tạo bản sao trùng slot khi nhãn vượt `80` byte do cắt chuỗi không khớp khóa.
  - Khoảng `79.5%..80.0%` hiển thị câu mâu thuẫn `"80% (< 80% Target)"`.
  - Thanh lý `UniverseRemoval` có thể ghi nhận đỉnh vốn/drawdown ảo trong 1 bước trung gian.
  - Metadata `technical.rsi_overbought_cross` để giá trị mặc định `30.0` thay vì `70.0`.
  - `export_markdown` không lọc ký tự xuống dòng trong trường `symbol`; giá trị sentinel `eval_base_timestamp == 0` trùng với `1970-01-01`.
- **Kết quả kỳ vọng / Chuẩn TradingView hoặc Quy chế VN (Expected Result):**
  - Xử lý nhất quán toàn bộ các ca biên trên để bảo đảm độ sạch tuyệt đối của sản phẩm thương mại.
- **Bằng chứng đính kèm (Evidence):**
  - Kiểm toán mã nguồn độc lập trên `src/journal/market_rules.rs`, `src/trading/ladder.rs`, `src/trading/coaching.rs`, `src/scanner/catalog.rs`, `src/backtest/engine.rs`, `src/alerts/engine.rs`.

---

## 4. Lộ Trình Bản Vá Đề Xuất Trước Khi Phát Hành `v1.0`

| Giai Đoạn | Mục Tiêu | Các Lỗi Cần Xử Lý | Tiêu Chí Tái Nghiệm Thu (`Re-Test Gate`) |
| :--- | :--- | :--- | :--- |
| **`Patch R19.1` (Bắt Buộc — Chặn Phát Hành)** | Xử lý triệt để `100%` lỗi `P0` và toàn bộ lỗi tính toán tài chính / dữ liệu / nhân quả `P1` | **`BUG-001` .. `BUG-017`** (`4 P0` + `13 P1`) | `VN30F1M` PnL chuẩn `100x kVND` & tối đa `4` HĐ/100k; trừ đủ `0.10%` thuế bán VN; cấm Short cổ phiếu VN; `T+2` bảo vệ tại lõi `TradingSession`; `Restart` giữ nguyên phiên `CafeF`; bỏ Rule 5 xóa oan `23.550` nến; nạp `VNINDEX` thật từ `data/raw/cafef/`; `Cmd+K` ưu tiên `CustomFile`; sửa `DEF-005` Gap, `IndicatorCache`, `AVWAP` cache, `OutlierImpact`, `Alert` & cutoff đơn điệu. |
| **`Patch R19.2` (Chuẩn Mực TradingView UX `v1.0`)** | Đạt độ mượt 60 FPS thực chiến trên `471` mã và khắc phục mọi lỗi hiển thị UI `P2` | **`BUG-018` .. `BUG-031`** (`14 P2`) | Watchlist dùng `show_rows` & Replay `+1` `< 16 ms`; `RRG` không chồng nhãn; nhãn trục phải `Alert`/`Drawing` qua `Anti-Collision`; `Templates`/`Cmd+K` bật `.fade_in(false)`; phân loại đủ sàn `HNX`/`UPCOM` & lịch lễ `2016–2017`; sửa `%Chg` theo `prev_close`, đơn vị `kVND`, toolbar `<= 2` hàng và DOM căn giữa Mark ở `1280×800`; sửa `2x2` mặc định & crosshair vùng trắng. |
| **`Patch R19.3` (Hoàn Thiện Nhẹ)** | Làm đẹp đóng gói và ca biên `P3` | **`BUG-032` .. `BUG-034`** (`3 P3`) | Đồng bộ `Info.plist`, bổ sung nút EN/VI `Signal Catalog`, định dạng `0K`/`T`/`0.00%`, các ca biên nhỏ. |

---

## 5. Kiểm Toán Bảo Toàn Dữ Liệu Gốc (`TC-65`) & Môi Trường

Đối chiếu mã băm `SHA-256` trước và sau toàn bộ quá trình kiểm thử (100% khớp từng bit, tuyệt đối không sửa đổi bất kỳ tệp nào trong `/Users/mizuhara/workspace/work/CafeFData/`, `ARCHIVE/`, `src/`, hay `docs/evidence/`):

| Tệp Dữ Liệu Gốc (`/Users/mizuhara/workspace/work/CafeFData/`) | Mã Băm `SHA-256` (Khớp 100% Trước & Sau Kiểm Thử) | Trạng Thái |
| :--- | :--- | :---: |
| `0_Index_CafeF.INDEX.Upto.zip` | `8e5e354722266c670f70e7e858b15360d2b5d8722a48ce6398804c821a9e1294` | `INTACT` |
| `1_Adjusted_CafeF.SolieuGD.Upto_part1.zip` | `76ad120dc2840cfa1cbed7d7a9faf0fed2a65d7b44edc626c988e653624ba00c` | `INTACT` |
| `1_Adjusted_CafeF.SolieuGD.Upto_part2.zip` | `c65b454c1539ffe8396e9355743ab910fb93fe7c3513bde47edf184710ae1a5f` | `INTACT` |
| `1_Adjusted_CafeF.SolieuGD.Upto_part3.zip` | `715cb25771dcb760305fa774e53b4b1ad174a165e7c640b4c7f3b589a0543407` | `INTACT` |
| `2_Raw_CafeF.SolieuGD.Raw.Upto_part1.zip` | `ab28800e195f29f0789570fb70b02f7490f923d4053c042459be92097f062567` | `INTACT` |
| `2_Raw_CafeF.SolieuGD.Raw.Upto_part2.zip` | `8964e9f7e8d0fcfe9cc55033c4c01f5ecb1e2d39c8f27046973df0aa2be056dc` | `INTACT` |
| `2_Raw_CafeF.SolieuGD.Raw.Upto_part3.zip` | `141ea6e490ff99239b4ea6ca2f110a9237e5888cb94f756602e84bf07dbbe506` | `INTACT` |

### Danh Mục `14` Ảnh Chụp Metal GPU Minh Chứng (`docs/testing/qa_evidence/`)
1. [`00_tc01_default_startup_1280x800.png`](./qa_evidence/00_tc01_default_startup_1280x800.png) — Khởi động gói `.app` từ `/tmp` (`BUG-023`, `BUG-025`)
2. [`01_tc04_data_audit_c69_1280x800.png`](./qa_evidence/01_tc04_data_audit_c69_1280x800.png) — `[Data Audit]` trên `C69` (`BUG-005`, `BUG-021`)
3. [`04_tc12_fpt_hollow_bullish_1280x800.png`](./qa_evidence/04_tc12_fpt_hollow_bullish_1280x800.png) — `TC-12 PASS`: nến `Hollow Bullish` sắc nét, râu dừng đúng mép thân
4. [`07_tc11_tc14_both_docks_1280x800.png`](./qa_evidence/07_tc11_tc14_both_docks_1280x800.png) — Chồng nhãn trục giá `Alert`/`Close`/`Entry`/`TP` ở `1280×800` (`BUG-018`, `BUG-020`, `BUG-023`)
5. [`08_tc14_both_docks_1440x1000.png`](./qa_evidence/08_tc14_both_docks_1440x1000.png) — Chồng nhãn trục giá và thẻ `Alert` đè `LONG 200` ở `1440×1000` (`BUG-018`)
6. [`09_tc16_19_vn30f_15m_indicators_1440x1000.png`](./qa_evidence/09_tc16_19_vn30f_15m_indicators_1440x1000.png) — `VRVP`, `TPO`, `Ichimoku`, `CVD`, `ADX`, `✓ Volume` rớt dòng, `PAT` chạm đỉnh (`BUG-023`, `BUG-032`)
7. [`10_tc20_fpt_rs_cmp_vnindex_1280x800.png`](./qa_evidence/10_tc20_fpt_rs_cmp_vnindex_1280x800.png) — `Cmp:VNINDEX` & `RS` dùng sóng sin giả lập (`BUG-006`)
8. [`12_tc22_26_drawings_inspector_textnote_1280x800.png`](./qa_evidence/12_tc22_26_drawings_inspector_textnote_1280x800.png) — `TextNote` UTF-8, `TrendLine` đảo chiều, chồng nhãn Fib `OTE` & nút `-> Order Ticket` (`BUG-026`, `BUG-032`)
9. [`13_tc32_multichart_2x1_replay_1280x800.png`](./qa_evidence/13_tc32_multichart_2x1_replay_1280x800.png) — `TC-32 PASS`: `2x1` Bar Replay cắt đúng nến tuần chưa đóng `FPT 1W`
10. [`14_tc43_vn30f_dom_ladder_1280x800.png`](./qa_evidence/14_tc43_vn30f_dom_ladder_1280x800.png) — `VN30F1M` DOM mất vùng Bid ở `1280×800`, PnL sai `100x`, ngày lễ `2026-09-01` (`BUG-001`, `BUG-024`, `BUG-025`)
11. [`15_tc51_breadth_rrg_proxybb_1440x1000.png`](./qa_evidence/15_tc51_breadth_rrg_proxybb_1440x1000.png) — `RRG` chồng `483` nhãn trắng đặc, `VN30(16)`, `2026-11-13`, `FPT +2.52%` giả lập (`BUG-006`, `BUG-020`)
12. [`16_tc56_57_multistrat_0pct_slices_1440x1000.png`](./qa_evidence/16_tc56_57_multistrat_0pct_slices_1440x1000.png) — `Multi-Strategy 0% UTC` dùng `Bench (VNINDEX)` giả lập `+93.71% / -3.6% DD` (`BUG-006`, `BUG-032`)
13. [`17_tc59_quad_2x2_1440x1000.png`](./qa_evidence/17_tc59_quad_2x2_1440x1000.png) — Lưới `2x2` mặc định nạp `S2: VNINDEX` gai `-34.5%` và `S3: VCB` giả lập trống `95%` (`BUG-022`)
14. [`22_tc61_templates_modal_1280x800.png`](./qa_evidence/22_tc61_templates_modal_1280x800.png) — Hộp thoại `Templates` bị mờ xuyên thấu và lệch nút `Close (Esc)` (`BUG-019`)
