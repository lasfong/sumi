# BỘ KỊCH BẢN KIỂM THỬ CHUYÊN NGHIỆP TOÀN DIỆN — SUMI TRADING WORKSTATION (`MASTER QA/QC & UAT TEST PLAN`)

- **Mã Tài Liệu:** `QA-MASTER-SUMI-V1.0`
- **Phiên Bản Ứng Dụng Đích:** `sumi-workstation v0.1.0` (Sau khi hoàn tất `100%` Core Parity `Waves 1–18 / Milestones R01–R19`)
- **Đường Dẫn Đóng Gói Ứng Dụng macOS:** `/Users/mizuhara/workspace/sumi/dist/Sumi Workstation.app` (và bản nhị phân `/Users/mizuhara/workspace/sumi/target/release/sumi-workstation`)
- **Nguồn Dữ Liệu Kiểm Thử Thực Tế (`CafeFData`):**
  - Kho nén gốc (chỉ đọc, tuyệt đối không sửa xóa): `/Users/mizuhara/workspace/work/CafeFData/*.zip`
  - Bộ CSV đã giải nén & ghép nối toàn lịch sử tại workspace (gitignored):
    1. `data/raw/cafef/CafeF.SolieuGD.Adjusted.Upto06112026.csv` — **`471` mã cổ phiếu**, **`941.648` dòng** giá đã điều chỉnh chia tách (`2016-01-04 .. 2026-11-06`, `41.58 MiB`)
    2. `data/raw/cafef/CafeF.SolieuGD.Raw.Upto06112026.csv` — **`471` mã cổ phiếu**, **`940.271` dòng** giá gốc chưa điều chỉnh (`2016-01-04 .. 2026-11-06`, `34.40 MiB`)
    3. `data/raw/cafef/CafeF.INDEX.Upto06112026.csv` — **`2` chỉ số** (`VNINDEX`, `HNX-INDEX`, tự động gộp cả dòng lệch hoa/thường `HNX-Index`), **`11.528` dòng** (`2000-07-28 .. 2026-11-06`, `0.61 MiB`)
  - Bộ dữ liệu mẫu tích hợp sẵn: `VNINDEX Synth` (`120B..100K`), `VN30F1M 1m` (`5.400` nến phái sinh trong phiên), `VN30 Basket` (`8` mã), `SPY CSV` (`8.280` nến NYSE `1993–2025`), `BTC JSON` (`1.344` nến `15m` 24/7).

---

## 1. Mục Tiêu Nghiệm Thu Gốc Của Product Owner (`Acceptance North Star`)

Mọi bài kiểm thử trong tài liệu này đối chiếu trực tiếp với **4 trụ cột yêu cầu gốc** của Product Owner ([`PROJECT_CHARTER.md`](../PROJECT_CHARTER.md), [`ARCHIVE/README.md`](../../ARCHIVE/README.md), [`PRODUCT_ACCEPTANCE_CRITERIA_V3.md`](../../ARCHIVE/docs/PRODUCT_ACCEPTANCE_CRITERIA_V3.md), [`FEATURE_BACKLOG_AND_PARITY_LEDGER.md`](../FEATURE_BACKLOG_AND_PARITY_LEDGER.md)):

1. **Chuẩn Mực Đồ Họa & Tương Tác Ngang Tầm TradingView (Native macOS Metal GPU):**
   - Kéo thả (pan), cuộn phóng to/thu nhỏ neo đúng tại vị trí con trỏ chuột (cursor-anchored zoom), quán tính trượt mượt (kinetic scroll), tự động co giãn trục giá (`Auto`) hoặc kéo trực tiếp lên cột giá bên phải để bóp/giãn biên độ, chuyển mượt giữa `Linear` / `Log` / `%` / `Idx100` / `Inverted`.
   - Không có hiện tượng chữ đè lên nhau (text overlap), cắt cụt nhãn giá, lệch râu nến, hay giật khung hình (`CPU frame time < 4.0 ms` ở độ phân giải Retina `2560×1600`).
2. **Trụ Cột 1 — Phòng Luyện Tập Phân Tích Kỹ Thuật & Bar Replay Tuyệt Đối Không Nhìn Trộm Tương Lai (`DEF-001`):**
   - Tua nến lịch sử từng thanh (`1m..1W`), tuyệt đối không lộ bất kỳ thông tin tương lai nào trên biểu đồ chính, khung thời gian lớn (`1W`/`1D` khi đang đứng giữa tuần/ngày), ô biểu đồ phụ (`2x1`/`2x2`), chỉ báo, công cụ vẽ, bảng quét tín hiệu (`BlindPractice`), cảnh báo hay sổ lệnh DOM.
   - Đặt lệnh trực quan (`Market`/`Limit`/`Stop`, kéo hộp `Pos R:R` đồng bộ sang `Order Ticket`), tự khớp `SL`/`TP` chính xác, chấm điểm kỷ luật cắt lỗ bất biến (`SL Adh %`), tổng kết `Session Debrief` và `Reset Practice` 1 chạm.
3. **Trụ Cột 2 — Phòng Thí Nghiệm Chiến Lược Tự Động (`Strategy Lab`, `1-Click Battle`, `Multi-Strategy 0%` & `PIT Portfolio`):**
   - Chạy kiểm định trên dữ liệu CafeF lịch sử dài hạn (`2016–2026`), khớp lệnh nhân quả tại `Open[T+1]`, so sánh đối đầu toàn bộ chiến lược (`Battle` + biểu đồ đường cong vốn `%` đồng bộ theo ngày UTC so với `VNINDEX`), tự xây quy tắc `AND`/`OR` từ `101` tín hiệu (`SIGNAL_CATALOG`), kiểm định độ bền `Walk-Forward`, `Monte Carlo` `1.000` kịch bản và danh mục chống thiên lệch sống sót (`Point-in-Time`).
4. **Chuẩn Vi Cấu Trúc Chứng Khoán Việt Nam & Độ Bền Dữ Liệu Thực Chiến (`CafeFData`):**
   - Đúng bước giá `HOSE` (`10 / 50 / 100 VND` tại mốc `10.000` & `50.000 VND`), biên độ trần/sàn (`±7%` HOSE, `±10%` HNX, `±15%` UPCOM), lô chẵn `100` cổ phiếu, thuế bán `0.10%`, phái sinh `VN30F1M` nhân `100.000 VND` / ký quỹ `17%`.
   - Đúng lịch nghỉ lễ chính thức `VIETNAM_HOLIDAYS_V1` (`151` ngày giai đoạn `2018–2030`) và quy tắc cổ phiếu về tài khoản `T+2` lúc **`13:00 ICT`** theo chuẩn VSD (cũng như `T+3` `09:00 ICT` cho dữ liệu trước `29/08/2022`).
   - Đọc mượt toàn bộ `471` mã từ `CafeFData` chỉ trong `< 0.5 giây`, tự chuẩn hóa sai số làm tròn 4 chữ số thập phân sau chia tách (`CafeFSplitAware`) và cách ly an toàn các dòng lỗi gốc của nhà cung cấp (`QuarantineRow`, ví dụ ngày KRX `05/05/2025` có `High = 0` hoặc lỗi `HNX-INDEX` `High = 3`).

---

## 2. Phân Loại Mức Độ Nghiêm Trọng Của Lỗi (`Bug Severity Matrix`)

| Mức Độ | Định Nghĩa Nghiệp Vụ & Kỹ Thuật | Điều Kiện Chặn Phát Hành |
| :--- | :--- | :--- |
| **`P0 — Blocker`** | Sập ứng dụng (`panic`/`crash`/treo cứng), rò rỉ dữ liệu tương lai trong Bar Replay/Backtest (`DEF-001`), sai lệch toán học số dư tiền/phí/PnL, làm hỏng tệp dữ liệu gốc của người dùng, hoặc không mở được gói `.app` / không đọc được `CafeFData`. | **Chặn tuyệt đối (`FAIL`)** |
| **`P1 — Critical`** | Công cụ vẽ, chỉ báo, cảnh báo, khóa `T+2` VSD, hoặc bộ lọc Scanner hoạt động sai công thức chuẩn (TA-Lib / LWC v5.2.0 / quy chế HOSE-HNX-VSD); thao tác cốt lõi (Pan/Zoom, Đặt lệnh, Undo/Redo, Lưu/Mở Template) thất bại hoặc mất trạng thái. | **Chặn phát hành (`FAIL`)** |
| **`P2 — Major (UX / TradingView Parity)`** | Trải nghiệm tương tác chưa mượt hoặc ngược kỳ vọng so với chuẩn TradingView (ví dụ: kéo trục giá bị giật, click chọn nét vẽ khó trúng hoặc bị xuyên thấu click-through, phím tắt bị cướp khi đang gõ chữ, nhãn chữ đè nhau ở `1280×800`). | **Cần sửa trước bản `v1.0` chính thức** |
| **`P3 — Minor / Cosmetic`** | Lệch căn lề nhẹ vài pixel, câu chữ thông báo/tooltip chưa tự nhiên, màu sắc phụ chưa đồng nhất nhỏ, không ảnh hưởng tính đúng đắn hay luồng thao tác. | Ghi nhận cải tiến làm đẹp |

---

## 3. Danh Mục 12 Bộ Kịch Bản Kiểm Thử Chi Tiết (`TC-01` .. `TC-72`)

### Suite 01 — Khởi Động Gói `.app`, Bảo Toàn Môi Trường & Nạp Bộ Dữ Liệu Thực `CafeFData` (`TC-01` .. `TC-06`)

| Mã TC | Kịch Bản & Các Bước Thực Hiện | Kết Quả Kỳ Vọng (Pass Criteria) |
| :--- | :--- | :--- |
| **`TC-01`** | Mở ứng dụng từ gói đóng gói `/Users/mizuhara/workspace/sumi/dist/Sumi Workstation.app` (hoặc chạy nhị phân từ thư mục bất kỳ như `/tmp` để giả lập Finder `cwd = "/"`). | Cửa sổ Metal mở ngay lập tức không báo lỗi thiếu đường dẫn tương đối `ARCHIVE/`; các nút nguồn `CafeF CSV`, `SPY CSV` và `[Export PNG]` đều hoạt động bình thường. |
| **`TC-02`** | Bấm `[Import CSV/JSON]`, chọn nút nhanh **`[CafeF Adj (471 mã)]`** (`data/raw/cafef/CafeF.SolieuGD.Adjusted.Upto06112026.csv`). Đo thời gian nạp. | Nạp toàn bộ `941.648` dòng (`471` mã, `2016–2026`) trong **1 lần quét duy nhất (`< 1 giây`)**, tự chọn mã thanh khoản `FPT`, không treo UI, báo rõ số mã đã nạp vào bộ nhớ. |
| **`TC-03`** | Mở menu chọn mã trên thanh công cụ (hoặc bấm `Cmd+K`), gõ tìm liên tiếp `VCB`, `HPG`, `SSI`, `MWG`, `TCB`, `DIG`, `VND`, `YEG` (các mã ở cuối bảng chữ cái vượt mốc `256` cũ). | Đầy đủ cả **`471` mã** từ `A..Y` xuất hiện; ô `"Filter mã..."` lọc tức thì; chuyển giữa các mã mượt tức thì (`< 5 ms`, không đọc lại ổ đĩa `41 MiB`). |
| **`TC-04`** | Trên bộ `CafeF Adj (471 mã)`, chuyển sang mã **`C69`**, **`HPX`**, **`KSV`**, **`NDN`** rồi mở **`[Data Audit]`**. | Các dòng có sai số làm tròn 4 chữ số thập phân sau chia tách của CafeF (`Close > High` vài đồng, `<= 6 bps & <= 25 VND`) được tự động chuẩn hóa (`Loader Repaired / Split Norm > 0`), không văng lỗi từ chối nạp. |
| **`TC-05`** | Trên bộ `CafeF Adj` hoặc **`[CafeF Raw (471 mã)]`**, chọn mã **`BVS`**, **`API`**, **`APS`** (các mã bị lỗi `High = 0` ngày KRX `05/05/2025` trong file gốc CafeF) và nạp **`[CafeF Index]`** (`VNINDEX`, `HNX-INDEX`, `UPCOM-INDEX`). | Toàn bộ chuỗi lịch sử dài hạn (`2000–2026` cho `VNINDEX`, `2016–2026` cho `BVS`) nạp thành công; các dòng lỗi gốc của CafeF (`High = 0` hoặc `HNX-INDEX` `High = 3`) bị cách ly sạch sẽ (`Quarantined > 0` trong `[Data Audit]`), tuyệt đối không vẽ nến sập về `0` làm bẹp trục giá! |
| **`TC-06`** | Thử nhập đường dẫn không tồn tại, tệp rỗng, hoặc bỏ tick `CafeF Split Norm` rồi nạp tệp có lỗi biên OHLC. | Báo lỗi rõ ràng màu đỏ trên thanh Import, **giữ nguyên biểu đồ hiện tại đang mở** (không xóa trắng màn hình hay crash). |

---

### Suite 02 — Trải Nghiệm Tương Tác Biểu Đồ Chuẩn TradingView (`TC-07` .. `TC-14`)

| Mã TC | Kịch Bản & Các Bước Thực Hiện | Kết Quả Kỳ Vọng (Pass Criteria) |
| :--- | :--- | :--- |
| **`TC-07`** | **Cursor-Anchored Zoom:** Đưa con trỏ chuột đặt chính xác lên một đỉnh/đáy lịch sử của `FPT` (ví dụ đáy tháng `11/2022`), lăn con lăn chuột phóng to và thu nhỏ liên tục `20` lần. | Cây nến ngay dưới mũi trỏ chuột **đứng yên tại đúng tọa độ ngang trên màn hình** (chuẩn thuật toán `TimeScale` của Lightweight Charts v5.2.0), không bị trôi lệch sang trái/phải. |
| **`TC-08`** | **Pan, Quán Tính (`Kinetic`) & Khoảng Trống Tương Lai (`Right Offset`):** Kéo nhanh biểu đồ sang phải rồi thả chuột; kéo biểu đồ sang trái để tạo khoảng trắng bên phải cây nến cuối cùng (future whitespace) để vẽ dự phóng. | Biểu đồ trượt có quán tính giảm tốc mượt mà; cho phép giữ khoảng trắng bên phải để đặt điểm neo `Ray`/`TrendLine`/`TextNote` ở vùng tương lai mà không bị giật ngược về mép phải. |
| **`TC-09`** | **Tương Tác Trục Giá Bên Phải & Trục Thời Gian Bên Dưới:** Đưa chuột vào cột trục giá bên phải kéo lên/xuống; nhấp đúp vào cột trục giá; kéo ngang trên thanh trục thời gian. | Kéo trục giá tự tắt `Auto` để bóp/giãn biên độ dọc; nhấp đúp trục giá bật lại `Auto` vừa khít màn hình; kéo trục thời gian thay đổi `bar_spacing` mượt mà. |
| **`TC-10`** | **4 Chế Độ Thang Giá (`Linear`, `Log`, `%`, `Idx100`) & Đảo Ngược (`Inv`):** Trên chuỗi `VNINDEX` `2000–2026` hoặc `FPT` `2016–2026`, chuyển lần lượt giữa `Linear`, `Log`, `%`, `Idx100` và bật `Inv`. | Ở `Log`, khoảng cách `%` tăng trưởng bằng nhau có chiều cao pixel bằng nhau; ở `%` và `Idx100`, nến đầu tiên bên trái khung nhìn làm gốc `0.00%` / `100.00`; bật `Inv` lật ngược cả nến, đường chỉ báo, dải mây và nhãn mẫu hình nến (`PAT`) chính xác. |
| **`TC-11`** | **Chống Đè Nhãn Trục Giá (Right-Axis Anti-Collision):** Mở đồng thời đường giá hiện tại, vị thế Paper Trading (`Entry`/`SL`/`TP`), và 3 cảnh báo giá sát nhau. | Các hộp nhãn giá trên cột phải tự động né nhau / ẩn vạch chia lưới bị che khuất (`main_occluding_ys`), nằm gọn trong khung, không tràn ra ngoài mép trên/dưới. |
| **`TC-12`** | **4 Bảng Màu Nến (`[Settings]`):** Chuyển đổi trực tiếp giữa `Emerald/Crimson`, `Hollow Bullish`, `Okabe-Ito (Colorblind)`, và `Classic Monochrome`. | Đổi màu ngay ở khung hình kế tiếp (`0` frame trễ); ở `Hollow Bullish`, nến tăng rỗng ruột có viền 4 cạnh sắc nét và **râu nến dừng đúng ở mép thân nến** (không đâm xuyên qua phần rỗng bên trong). |
| **`TC-13`** | **Gộp Nến Mật Độ Cao (`Conflation` `100K` Nến):** Chọn `VNINDEX Synth` -> `100K` nến, bấm `Fit` để hiển thị toàn bộ `100.000` nến trên 1 màn hình. | Toàn bộ đỉnh cao nhất (`High`), đáy thấp nhất (`Low`) không bị mất gai giá, tốc độ vẽ vẫn mượt `< 4.0 ms` nhờ `conflation_factor` lũy thừa 2. |
| **`TC-14`** | **Độ Phân Giải Khác Nhau (`1280×800` Laptop & `1440×1000`+ Desktop):** Thu nhỏ cửa sổ về `1280×800` rồi phóng rộng lên `1440×1000` (hoặc toàn màn hình) khi đang mở cả hai thanh bên (Left Scanner + Right Trading/Strategy). | Vùng vẽ biểu đồ trung tâm luôn giữ kích thước sử dụng tốt, thanh cuộn hoạt động đầy đủ ở các dock, không mất nút bấm. |

---

### Suite 03 — Hệ Thống `24+` Chỉ Báo Kỹ Thuật, Khung Phụ & Gộp Đa Khung Thời Gian (`TC-15` .. `TC-21`)

| Mã TC | Kịch Bản & Các Bước Thực Hiện | Kết Quả Kỳ Vọng (Pass Criteria) |
| :--- | :--- | :--- |
| **`TC-15`** | **Gộp Đa Khung Thời Gian (`1m -> 5m -> 15m -> 1h -> 4h -> 1D -> 1W`):** Chọn `VN30F1M 1m`, chuyển lần lượt từng khung lên tới `1D`, `1W`; chọn `FPT` (`CafeF Adj` `1D`), chuyển sang `1W` rồi quay lại `1D`. | Các nút khung thời gian nhỏ hơn khung gốc tự vô hiệu hóa (chống nội suy giả); nến `1W` căn đúng Thứ Hai `00:00 UTC` với `Open` đầu tuần, `High = max`, `Low = min`, `Close` cuối tuần, `Volume = sum`. |
| **`TC-16`** | **Chỉ Báo Trên Biểu Đồ Chính (`Overlays`):** Bật/tắt `SMA(20)`, `EMA(50)`, `BB(20, 2.0)`, `Ichimoku Cloud (9,26,52,+26)`, và trong menu `Ind+ ▾`: `SuperTrend(10,3.0)`, `Parabolic SAR`, `Keltner`, `Vol SMA(20)`, `Candle Patterns (PAT)`. | Dải mây Kumo đổi màu xanh/đỏ mượt ngay tại điểm giao cắt (không bị răng cưa lệch tam giác); `Chikou Span (-26)` dừng đúng cách nến hiện tại 26 thanh (không kéo ngang giả tạo); `SuperTrend` & `PSAR` đổi chiều chuẩn xác. |
| **`TC-17`** | **Volume Profile (`VRVP`) & Session Market Profile (`TPO`):** Bật `VRVP` và `TPO` trên `VN30F1M 15m`, kéo trượt biểu đồ qua lại nhiều phiên. | `VRVP` tính lại vùng giá trị `70% Value Area` (`VAH`/`VAL`) và đường `POC` theo đúng các nến đang hiển thị trên màn hình; `TPO` hiển thị đúng `Initial Balance (IB)`, `Single Prints` và kéo dài các đường `Naked POC` chưa bị giá chạm lại. |
| **`TC-18`** | **Multi-Interval `VWAP` (`Session / Daily / Weekly / Monthly / Yearly`):** Bật `VWAP` kèm dải `±1σ, ±2σ`, đổi chu kỳ neo (`Session`, `Weekly`, `Monthly`). | Đường `VWAP` reset đúng đầu phiên/tuần/tháng; ngay tại thanh nến đầu tiên của chu kỳ mới có độ lệch chuẩn bằng `0` (`VWAP == Typical Price`), không tràn số khi khối lượng hàng tỷ cổ phiếu. |
| **`TC-19`** | **Các Khung Dao Động Phụ (`RSI`, `MACD`, `ATR`, `ADX`, `Stoch`, `CCI`, `MFI`, `CVD`, `RS`, `ProxyBB`, `Equity`):** Mở đồng thời 4–5 khung phụ, kéo thanh phân cách ngang (`5 px` splitter) giữa các khung để thay đổi chiều cao. | Mọi khung phụ cuộn/zoom đồng bộ tuyệt đối trục thời gian với biểu đồ chính; `RSI` có mốc `30/50/70`, `CCI` có `-100/0/100`, `CVD` hiển thị trục phải gọn `K/M/B/T`; kéo vách ngăn mượt và không khung nào bị bẹp dưới `44 px`. |
| **`TC-20`** | **Sức Mạnh Tương Đối (`RS vs VNINDEX`) & Biểu Đồ So Sánh (`Cmp:VNINDEX`):** Chọn `FPT` trên `CafeF Adj`, bật cả `RS` và `Cmp:VNINDEX`. | Đường `Cmp:VNINDEX` quy đổi về cùng gốc giá với `FPT` tại mép trái khung nhìn và căn khớp đúng theo ngày giao dịch UTC; những ngày lễ không có giao dịch không tạo điểm nối sai. |
| **`TC-21`** | **Độ Bền Warmup & Dữ Liệu Ngắn:** Chọn mã mới lên sàn có ít hơn `20` hoặc `50` nến trong `CafeF Adj`, bật toàn bộ tất cả các chỉ báo cùng lúc. | Không xảy ra panic/crash hay vẽ vạch thẳng đứng về `0.0`; các chỉ báo chưa đủ chu kỳ warmup hiển thị trạng thái chờ sạch sẽ trên thanh chú giải (legend). |

---

### Suite 04 — Bộ `9` Công Cụ Vẽ, OHLC Magnet, Custom Fibonacci (`OTE`), `TextNote` & `Undo/Redo` (`TC-22` .. `TC-29`)

| Mã TC | Kịch Bản & Các Bước Thực Hiện | Kết Quả Kỳ Vọng (Pass Criteria) |
| :--- | :--- | :--- |
| **`TC-22`** | **Vẽ Đủ `9` Công Cụ Trên Biểu Đồ Chính & Khung Phụ:** Lần lượt chọn và vẽ `H-Line`, `Trend`, `Ray`, `Rect`, `Fib`, `Pos R:R`, `FRVP`, `AVWAP`, `Note` (kể cả vẽ `Trend`/`H-Line` vào khung `RSI`/`MACD`). Bấm `Escape` khi đang vẽ dở điểm thứ nhất. | Nét nháp (draft preview) bám chuột mượt mà; bấm `Escape` hủy ngay nét đang vẽ dở và trả về `Cursor`; nét vẽ trong `RSI` độc lập với biểu đồ chính. |
| **`TC-23`** | **Nam Châm Bắt Điểm (`Magnet OHLC`):** Bật `Magnet OHLC`, đưa chuột lại gần đỉnh râu trên (`High`) hoặc đáy râu dưới (`Low`) của cây nến để vẽ `Fib` và `TrendLine`. | Điểm neo tự động hít chính xác vào giá `Open`/`High`/`Low`/`Close` gần nhất của cây nến đó; tắt `Magnet` thì đặt tự do ở mọi mức giá lẻ. |
| **`TC-24`** | **Custom Fibonacci (`Std 7` / `ICT OTE 9` / `Full 13` & Tùy Chỉnh `1..=16` Mức):** Chọn hình `Fib`, mở bảng `Inspect` bên phải, bấm `ICT OTE 9`, bật/tắt từng mức, và nhập thêm mức tỷ lệ riêng (ví dụ `0.79` -> `+ Ratio`) rồi bấm `Apply`. | Biểu đồ vẽ ngay mức `0.705`, `-0.272`, `0.79` kèm giá quy đổi chính xác; đảo ngược `Reverse 0<->100%` đổi chiều chuẩn; bấm `Undo` chỉ lùi đúng **1 bước giao dịch nguyên tử** cho cả lần `Apply`. |
| **`TC-25`** | **Kéo Dài 2 Đầu Đường Xu Hướng (`TrendLine` `Extend Left` / `Extend Right`):** Vẽ một `TrendLine` nghiêng từ phải sang trái (`A1.bar > A2.bar`), bật `Extend Right` rồi tắt `Extend Right` bật `Extend Left`. | `Extend Right` luôn kéo dài tia về phía **tương lai (bên phải màn hình)** và `Extend Left` luôn kéo dài về phía **quá khứ (bên trái màn hình)** bất kể người dùng chấm điểm `A1` hay `A2` trước! |
| **`TC-26`** | **Ghi Chú Văn Bản (`TextNote`) & Chống Xuyên Thấu Click-Through:** Dùng công cụ `Note` chấm lên vùng có sẵn một hình chữ nhật (`Rect`) bên dưới; sửa nội dung đa dòng tiếng Việt có dấu trong `Drawing Inspector` (`<= 280` ký tự); nhấp chuột vào giữa hộp chữ `Note`. | Văn bản tiếng UTF-8 hiển thị sắc nét, tự ngắt dòng gọn trong thẻ; nhấp vào giữa hộp `Note` phải chọn đúng `Note` (ưu tiên cao nhất, không bị xuyên xuống `Rect` bên dưới). |
| **`TC-27`** | **Bảo Toàn Tọa Độ Thời Gian Thực Khi Đổi Khung (`1D <-> 1W`, `5m <-> 1h`):** Vẽ `TrendLine`, `Fib`, `AVWAP` và `TextNote` trên `FPT` khung `1D`, chuyển sang `1W` rồi chuyển ngược về `1D`. | Mọi điểm neo giữ nguyên chính xác ngày/giờ UTC gốc của đỉnh/đáy đã chấm, không bị trôi sang tuần khác. |
| **`TC-28`** | **Khóa Nét Vẽ (`Lock`), Ẩn/Hiện (`Visible`) & Lịch Sử `Undo/Redo` 64 Bước:** Bật `Locked` trên 1 nét vẽ rồi thử kéo di chuyển hoặc bấm phím `Delete`/`Backspace`; sau đó mở khóa, di chuyển, xóa, `Clear` rồi bấm `Undo` (`Cmd+Z`) và `Redo` (`Cmd+Shift+Z`) liên tiếp. | Khi `Locked`, nét vẽ không thể bị kéo lệch hay xóa nhầm; khi đang ẩn (`Visible = false`), nét vẽ vẫn chọn lại được trong danh sách `LAYERS` của Inspector; `Undo`/`Redo` khôi phục chính xác 100% cả lệnh `Clear`. |
| **`TC-29`** | **Cách Ly Nét Vẽ Theo Từng Mã Cổ Phiếu:** Vẽ 3 hình trên `FPT`, chuyển sang `VCB` vẽ 1 hình khác, rồi quay lại `FPT`. | `FPT` hiển thị đúng 3 hình của `FPT`, `VCB` giữ đúng 1 hình của `VCB`, tuyệt đối không lẫn nét vẽ giữa hai mã. |

---

### Suite 05 — Kiểm Toán Chống Nhìn Trộm Tương Lai (`DEF-001` Zero-Lookahead) Trên Bar Replay (`TC-30` .. `TC-37`)

| Mã TC | Kịch Bản & Các Bước Thực Hiện | Kết Quả Kỳ Vọng (Pass Criteria) |
| :--- | :--- | :--- |
| **`TC-30`** | **Cắt Nến Lịch Sử (`<< Cut Bar`) & Bước Tiến/Lùi:** Mở `FPT` (`CafeF Adj`), bật `<< Bar Replay`, bấm `<< Cut Bar` rồi nhấp vào một cây nến năm `2022`. Bấm `+1 >>`, `<< -1`, `Play >` (tốc độ `1x..20x`), `\|<< Start`. | Mọi nến sau ngày cắt biến mất hoàn toàn; thanh trạng thái & Replay Dock hiển thị rõ ngày hiện tại và chỉ số nến `Base cursor / Total` mà không cần rê chuột. |
| **`TC-31`** | **Kiểm Toán Chống Lộ Nến Khung Lớn (`1m -> 15m/1h/1D`, `1D -> 1W`):** Cắt Replay của `FPT 1D` tại ngày **Thứ Tư** giữa tuần, sau đó bấm chuyển khung thời gian lên **`1W`** (nến tuần). | Thanh nến tuần hiện tại trên màn hình `1W` **chỉ được tổng hợp OHLCV từ Thứ Hai đến đúng Thứ Tư đó**, tuyệt đối không lộ giá `High`/`Low`/`Close` của Thứ Năm và Thứ Sáu chưa diễn ra! |
| **`TC-32`** | **Kiểm Toán Ô Biểu Đồ Phụ (`2x1` / `2x2` Multi-Chart Replay):** Mở mẫu bố cục `2x1` (`Ô 1: FPT 1D`, `Ô 2: FPT 1W` hoặc `VNINDEX 1D`), bật Bar Replay tua lùi về năm `2023`. | Ô biểu đồ phụ thứ 2 tự động cắt đúng theo mốc thời gian Replay của ô chính (`causal_closed_prefix_len`), tuyệt đối không hiển thị nến năm `2024–2026` ở ô phụ! |
| **`TC-33`** | **Kiểm Toán Nét Vẽ Tương Lai Trong Replay:** Trước khi vào Replay, vẽ một `TextNote` hoặc `TrendLine` tại tháng `09/2026`; sau đó cắt Replay lùi về tháng `03/2026`, rồi tua tiến dần qua tháng `09/2026`. | Khi đang ở tháng `03/2026`, nét vẽ tạo ở tháng `09/2026` bị ẩn hoàn toàn (`is_causally_visible == false`) và không thể chọn nhầm; khi tua vượt qua mốc đó thì tự động hiện lại. |
| **`TC-34`** | **Kiểm Toán Chống Lộ Tín Hiệu Quét & Chỉ Số Toàn Thị Trường Trong Replay:** Đang ở Replay ngày `T`, mở `Watchlist / Scan`, tab `Sig`, tab `Breadth` (`RRG` + `Market-BB`), và hộp thoại `[Data Audit]`. | Mọi giá `Last`, `%Chg`, `RVOL`, quỹ đạo `RRG`, dòng tiền `Market-BB` của tất cả các mã khác trong danh mục đều bị khóa chặt tại `<= T`, không lộ dữ liệu sau ngày `T`. |
| **`TC-35`** | **Kiểm Toán Không Rò Rỉ Qua Đường Trung Bình / Phân Kỳ Đỉnh Đáy (`+3B`):** Đang ở Replay thanh `t`, mở `Signal Inspector` / bật chỉ báo phân kỳ (`divergence.*`). | Tín hiệu phân kỳ xác nhận đỉnh/đáy chỉ được phép nổ sau khi đã đóng đủ `3` nến bên phải đỉnh/đáy (`p = t - 3`), tức tại thanh `t` giá trị tín hiệu hoàn toàn bất biến dù sau đó thanh `t+1, t+2` biến động thế nào. |
| **`TC-36`** | **Tua Lùi Replay (`<< -1`) Khôi Phục Bit-Exact Vị Thế & Cảnh Báo:** Trong Replay, mua 1 lệnh khớp ở thanh `100`, chạm chốt lời `TP` ở thanh `105` và nổ 1 cảnh báo `Once` ở thanh `104`; sau đó bấm tua lùi về thanh `102`. | Lệnh chốt lời ở `105` được hoàn tác chính xác: vị thế quay lại trạng thái đang mở `LONG`, tiền mặt/vốn chủ sở hữu khớp đúng thanh `102`, và cảnh báo `Once` tự động nạp lại (`Armed`) để nổ lại khi tiến qua `104`! |
| **`TC-37`** | **Khôi Phục Phiên Khi Khởi Động Lại Ứng Dụng:** Đang đứng ở giữa phiên Replay có nét vẽ và vị thế mở, đóng ứng dụng (`Cmd+Q`) rồi mở lại ứng dụng. | Trạng thái bố cục, vị trí con trỏ Replay, nét vẽ và sổ lệnh được khôi phục nguyên vẹn không lỗi. |

---

### Suite 06 — Paper Trading, DOM Ladder, Vi Cấu Trúc Thị Trường VN & Khóa Lô `T+2` (`13:00 ICT`) FIFO (`TC-38` .. `TC-45`)

| Mã TC | Kịch Bản & Các Bước Thực Hiện | Kết Quả Kỳ Vọng (Pass Criteria) |
| :--- | :--- | :--- |
| **`TC-38`** | **Đồng Bộ 2 Chiều `Pos R:R` <-> `Order Ticket` & `Risk Planner`:** Dùng công cụ `Pos R:R` vẽ một kèo Long trên `FPT`, bấm `-> Order Ticket` trên thanh công cụ vẽ, đồng thời mở tab `Risk/Jrn`. | Giá `Entry`, `Stop-Loss`, `Take-Profit` tự động điền sang Order Ticket và Risk Planner; khối lượng cổ phiếu được tính đúng theo `%` rủi ro mỗi lệnh (làm tròn lô chẵn `100` cổ phiếu trên `HOSE`), trừ đủ phí mua + phí bán + thuế bán `0.10%`. |
| **`TC-39`** | **Khớp Lệnh `Market / Limit / Stop` & Ưu Tiên Bi Quan `SL` Trước `TP`:** Trong Bar Replay, đặt lệnh chờ `Limit` và `Stop` kèm `SL`/`TP`; kiểm tra trường hợp nến ngày hôm sau nhảy Gap qua giá chờ hoặc có râu nến chạm cả `SL` lẫn `TP` trong cùng 1 thanh. | Lệnh chờ không bao giờ khớp gian lận ngay trên thanh vừa đặt; nhảy Gap khớp tại giá `Open` thực tế; thanh nến chạm cả hai biên luôn xử lý thận trọng (pessimistic path) theo đúng đường đi 4 điểm OHLC. |
| **`TC-40`** | **Khóa Lô Hàng Về `T+2` (`13:00 ICT`) Theo FIFO Trên Cổ Phiếu VN (`C2`):** Bật `T+2 Lock` trên `FPT` (`HOSE`) trong Bar Replay: Ngày `T0` mua `200` cp (Lô #1); tiến `3` phiên giao dịch rồi mua thêm `300` cp (Lô #2); tiến `1` phiên giao dịch rồi thử bấm bán `400` cp (hoặc bấm `Flatten`). | Lô #1 (`200` cp) hiện `[SELLABLE]`, Lô #2 (`300` cp) hiện `[LOCKED T+2]`; lệnh bán `400` cp bị **chặn ngay trước khi khớp** với thông báo đỏ rõ ràng (`requested sell 400 exceeds settled 200`); đặt bán `<= 200` cp thì khớp bình thường và trừ đúng vào Lô #1 theo FIFO! |
| **`TC-41`** | **Kiểm Tra `T+2` Xuyên Qua Dịp Nghỉ Tết Nguyên Đán & Lễ `30/04–01/05` (`VIETNAM_HOLIDAYS_V1`):** Trên `FPT` (`CafeF Adj`), nhảy Replay tới phiên thứ Năm/thứ Sáu ngay trước kỳ nghỉ Tết Nguyên Đán (ví dụ tháng `02/2026` hoặc lễ `29/04/2026`), mua `100` cp rồi bấm tiến từng phiên. | Ngày mở khóa `[SELLABLE]` (`13:00 ICT`) nhảy chính xác qua toàn bộ chuỗi ngày nghỉ Tết / lễ `30/04–01/05` theo lịch Sở Giao Dịch (đủ đúng `2` phiên giao dịch thực tế sau ngày mua, không đếm nhầm ngày nghỉ lễ là ngày làm việc). |
| **`TC-42`** | **Giới Hạn Cắt Lỗ / Chốt Lời Tự Động (`SL`/`TP`) Khi Có Cổ Phiếu Chưa Về `T+2`:** Khi đang giữ `200` cp đã về và `300` cp chưa về (`T+2 Lock = ON`), gắn `Stop-Loss` và tua tới nến thủng `SL` (hoặc tắt `T+2 Lock` trong `[Settings]` để luyện tập tự do như chế độ V3 cũ). | Khi `T+2 Lock = ON`: hệ thống chỉ bán thoát phần `200` cp hợp pháp đã về và giữ phần `300` cp đang khóa cho tới phiên chiều `T+2` về tài khoản; khi người dùng tắt `T+2 Lock` trong `[Settings]`, cho phép đóng tự do 100% để luyện phản xạ PTKT thuần túy! |
| **`TC-43`** | **Sổ Lệnh DOM Ladder 21 Bậc (`OHLCV_SYNTH_DOM_PROXY`):** Chuyển sang chế độ `DOM Ladder` trên `FPT` (quanh vùng giá `10.00` hoặc `50.00` kVND) và trên `VN30F1M`. Nhấp vào cột Bid/Ask để đặt lệnh chờ và nhấp hủy ngay trên bậc giá. | Bước giá tự động chuyển mượt qua ranh giới `10.00` (`0.01 <-> 0.05` kVND) và `50.00` (`0.05 <-> 0.10` kVND) trên `HOSE`; hiển thị rõ vạch `CEIL` / `REF` / `FLOOR`; chặn bán khống cổ phiếu cơ sở VN nhưng cho phép `Short` trên `VN30F1M`. |

---

### Suite 07 — Huấn Luyện Thực Hành (`PracticeScoreboard`, `SessionDebriefModal` & Lưu Phiên) (`TC-44` .. `TC-48`)

| Mã TC | Kịch Bản & Các Bước Thực Hiện | Kết Quả Kỳ Vọng (Pass Criteria) |
| :--- | :--- | :--- |
| **`TC-44`** | **Tính Bất Biến Của Kỷ Luật Đặt `Stop-Loss` Lúc Vào Lệnh (`SL Adh %`):** Trong Replay, vào Lệnh #1 **không điền SL** (sau khi khớp mới cập nhật thêm SL bằng nút `Apply SL/TP`), và vào Lệnh #2 **có điền sẵn SL ngay trên vé lệnh lúc bấm Buy**. Đóng cả 2 lệnh rồi mở `[Debrief]`. | Chỉ số `Initial SL Adherence` báo chính xác **`50% (1/2)`** — việc chữa cháy thêm SL sau khi đã vào lệnh tuyệt đối không thể gian lận làm tăng điểm kỷ luật lúc mở vị thế! |
| **`TC-45`** | **Ghi Chú Quyết Định Theo Nến (`Decision Note`) & Hộp Thoại `Session Debrief`:** Gắn thẻ `Setup` (`Squeeze BO`, `Pullback`), thẻ lỗi tâm lý (`FOMO Chase`) và ghi chú tại nến đang mở lệnh; mở `[Debrief]`. | `Session Debrief` thống kê khớp 100% hiệu suất theo từng `Setup`, chi phí trả giá cho sai lầm (`Mistake Cost`), danh sách ghi chú nhân quả, và không hiện cảnh báo sai khi phiên mới mở có `0` lệnh. |
| **`TC-46`** | **Nút `Reset Practice` 1 Chạm:** Đang có 3 lệnh đã đóng, 2 ghi chú quyết định, 4 nét vẽ kỹ thuật trên biểu đồ và con trỏ Replay đang ở thanh `140`; bấm `Reset Practice`. | Vốn, lệnh Paper và bảng điểm reset sạch về `0` để luyện lại ngay, **nhưng giữ nguyên con trỏ Replay ở thanh `140` và giữ nguyên toàn bộ 4 nét vẽ trên biểu đồ**. |
| **`TC-47`** | **Lưu / Ghim / Xuất Nhập Phiên Thực Hành (`PracticeSessionStore`) & Nhật Ký Giao Dịch (`CSV/MD`):** Bấm `Save` phiên thực hành, ghim `[★]`, xuất/nhập JSON/RON, và xuất Nhật ký giao dịch sang CSV/Markdown. | Khôi phục đúng mã, khung thời gian, con trỏ Replay và lịch sử lệnh; tệp CSV xuất ra có cơ chế chống CSV Formula Injection khi ghi chú bắt đầu bằng `=`, `+`, `-`, `@`. |

---

### Suite 08 — Watchlist `471` Mã, Quét `101` Tín Hiệu (`SIGNAL_CATALOG`), `BlindPractice +120b` & Dòng Tiền `ProxyBB` (`TC-48` .. `TC-54`)

| Mã TC | Kịch Bản & Các Bước Thực Hiện | Kết Quả Kỳ Vọng (Pass Criteria) |
| :--- | :--- | :--- |
| **`TC-48`** | **Quét Toàn Bộ `471` Mã `CafeF Adj` Trên Watchlist & Tab `Sig`:** Sau khi nạp `CafeF Adj (471 mã)`, mở `Watchlist / Scan`, cuộn dọc nhanh từ đầu tới cuối danh sách, sắp xếp theo `%Chg`, `RVOL`, `RSI`, lọc theo `Regime` và `Bull/Bear`. | Thanh cuộn ảo hóa (`show_rows`) mượt `60+ FPS` với cả `471` mã; các mã giá trị thiếu (`None`) luôn xếp gọn xuống cuối không đảo lộn thứ tự. |
| **`TC-49`** | **Luồng `Scanner -> Replay` 1 Chạm (`Blind +120b` vs `Review`):** Ở tab `Watch` hoặc `Sig`, chọn độ lùi `+120b`, bấm nút **`[Blind +120b]`** trên một tín hiệu lịch sử. | Biểu đồ nhảy ngay vào Bar Replay đúng `120` nến trước thời điểm nổ tín hiệu để có đủ bối cảnh cấu trúc thị trường; **tên tín hiệu bị che kín (`Signal hidden until reached`)** cho tới khi người dùng tua chạm đúng thanh nổ tín hiệu mới mở khóa giải thích! |
| **`TC-50`** | **Hộp Thoại Danh Mục `101` Tín Hiệu (`Signal Catalog Modal`) & Điểm Hợp Lưu 4 Họ (`Confluence`):** Mở `Signal Catalog` (từ Scanner hoặc `Cmd+K`), lọc qua đủ `12` nhóm (`Pattern`, `Div`, `VSA`, `S/R`, `Ichimoku`, `ProxyBB`, `Regime`...), chuyển ngôn ngữ giải thích EN/VI. | Hiển thị trọn vẹn **`101` tín hiệu**, ghi rõ độ trễ nhân quả (`delay=3B` cho phân kỳ đỉnh/đáy) và nguồn xấp xỉ `OHLCV_BAR_CLOSE_PROXY`; điểm hợp lưu nằm chuẩn trong `[-100, +100]`. |
| **`TC-51`** | **Độ Rộng Thị Trường (`Breadth`), Biểu Đồ Xoay Tương Đối `4` Góc `RRG` & Dòng Tiền `ProxyBB` (`T03..T200`):** Mở tab `Breadth`, quan sát biểu đồ 4 góc `RRG` (`Leading`, `Weakening`, `Lagging`, `Improving`), bảng `PROXY-BB (T03..T200)` của mã đang mở và `MARKET-BB`. | Tổng tỷ trọng mua/bán bảo toàn chính xác; mã có thanh khoản bằng `0` trả về `None` an toàn (`AT05`, không chia cho `0`); các vệt đuôi xoay `RRG` hiển thị rõ nét không chồng chữ. |

---

### Suite 09 — Cảnh Báo Nhân Quả Đa Điều Kiện (`Alerts`) & Xuất Kiểm Toán (`TC-52` .. `TC-55`)

| Mã TC | Kịch Bản & Các Bước Thực Hiện | Kết Quả Kỳ Vọng (Pass Criteria) |
| :--- | :--- | :--- |
| **`TC-52`** | **Cảnh Báo Giá, `RSI`, `MACD`, Đường Kẻ Nghiêng (`TrendLine`/`Ray`), Tín Hiệu Catalog & Điểm Hợp Lưu:** Mở tab `Alerts`, tạo cảnh báo cắt lên đường `TrendLine` nghiêng (có bật `Extend Right`), cảnh báo nổ tín hiệu từ `101` Catalog, và thiết lập `cooldown_bars = 5`. | Cảnh báo đường nghiêng bắt đúng giao điểm theo độ dốc từng thanh nến; `cooldown_bars = 5` chặn nổ lặp lại trong vòng 5 thanh liên tiếp; các mũi tên báo động trên biểu đồ tự xếp chồng tách biệt trên/dưới thân nến. |
| **`TC-53`** | **Xuất Lịch Sử Cảnh Báo Ra CSV / JSON:** Bấm nút xuất CSV và JSON trong bảng `Alerts`. | Dữ liệu sao chép vào clipboard đúng chuẩn RFC-4180 CSV / RFC-8259 JSON, chỉ bao gồm các cảnh báo `<=` mốc thời gian Replay hiện tại. |

---

### Suite 10 — Strategy Tester, Rule Builder `AND`/`OR`, `1-Click Battle`, `Multi-Strategy 0%`, `WFO`, Monte Carlo & `PIT` (`TC-54` .. `TC-60`)

| Mã TC | Kịch Bản & Các Bước Thực Hiện | Kết Quả Kỳ Vọng (Pass Criteria) |
| :--- | :--- | :--- |
| **`TC-54`** | **Chạy Backtest Đơn & Kiểm Chứng Khớp Lệnh `Open[T+1]` Trên `FPT` (`2016–2026`):** Mở `Strategy Lab`, chọn `FPT` (`CafeF Adj`), chạy từng chiến lược trong `7` preset, mở tab `Trd` nhấp vào một lệnh bất kỳ và bấm `Replay -5b`. | Tín hiệu phát sinh lúc đóng cửa thanh `T` luôn khớp mua tại giá **`Open` của thanh `T+1`** (không bao giờ khớp tại `Close[T]`); nhấp vào lệnh tự căn biểu đồ tới đúng điểm vào/ra kèm đường nối xanh/đỏ. |
| **`TC-55`** | **Trình Xây Dựng Quy Tắc Chiến Lược Tự Chọn (`Custom Signal Rule Builder`):** Chọn preset `Custom Signal Rules`, ghép điều kiện 2 tầng `(RSI < 35 OR Bullish Engulfing) AND Regime = Uptrend` cho chân Vào Long và điều kiện Thoát Long riêng biệt, chạy Backtest và `Sweep`. | Đánh giá logic DNF `OR-of-AND` chính xác, tự khóa các tín hiệu đang trong giai đoạn warmup (`SignalQuality != Valid` không bao giờ kích hoạt lệnh ảo). |
| **`TC-56`** | **1-Click `Battle` & Biểu Đồ So Sánh Đa Chiến Lược `0%` (`Multi-Strat %`):** Mở tab `Battle` (so sánh các mã) và tab `Slices` -> bật **`Multi-Strategy [0% UTC]`** trên khung `Equity`. | Tất cả `7` chiến lược + `Asset Buy & Hold` + `Benchmark (VNINDEX)` xuất phát tại `0.00%` và **căn khớp chính xác theo ngày UTC** trên cùng một đồ thị kèm trục phải `+X.XX%` và chấm crosshair đồng bộ. |
| **`TC-57`** | **Phân Tích Tearsheet Chuyên Sâu (`OutlierImpact`, `DrawdownPeriods`, Lát Cắt Năm / `MarketRegime`, `MultiPhase` & `RunManifest`):** Kiểm tra tab `Slices` sau khi chạy Backtest `FPT` `2016–2026`. | Hiển thị đầy đủ tác động của Top 3 lệnh thắng lớn nhất, danh sách các đợt sụt giảm vốn (`Drawdown`), bảng lợi nhuận theo từng năm `2016..2026`, hiệu suất theo `MarketRegime` tại nến tín hiệu `T`, và mã băm `MANIFEST #...` tất định (chạy lại 2 lần ra đúng 1 mã băm y hệt). |
| **`TC-58`** | **Kiểm Định `Walk-Forward (WFO)`, `Monte Carlo` `1.000` Kịch Bản & Danh Mục `VN30-PIT` Chống Thiên Lệch Sống Sót:** Chạy `WFO` (`Rolling`/`Anchored`), bật quạt xác suất `Monte Carlo` (`p5..p95`, `VaR`/`CVaR` `95%`), và chạy `Portfolio` chế độ `Point-in-Time (VN30-PIT)` so với `Retrospective`. | `WFO` nối liền mạch đường vốn OOS; quạt `Monte Carlo` vẽ mượt trên khung `Equity` (tái lập 100% cùng seed `202615`); chế độ `VN30-PIT` tự động thanh lý mã bị loại khỏi rổ tại đúng kỳ cơ cấu với lý do `UniverseRemoval`. |

---

### Suite 11 — Lưới Đa Biểu Đồ (`1x1..2x2`), Phím Tắt, `Cmd+K`, Snapshot Khôi Phục & `[Export PNG]` (`TC-59` .. `TC-63`)

| Mã TC | Kịch Bản & Các Bước Thực Hiện | Kết Quả Kỳ Vọng (Pass Criteria) |
| :--- | :--- | :--- |
| **`TC-59`** | **Lưới Đa Biểu Đồ Đồng Bộ (`1x2`, `2x1`, `2x2`):** Chuyển sang `2x2`, bật `Sync X` (đồng bộ chữ thập crosshair) và `Sync T` (đồng bộ khung thời gian cuộn), rê chuột ở Ô 1. | Con trỏ chữ thập xuất hiện đúng tại cùng thời điểm UTC trên cả 4 ô biểu đồ; khi rê chuột ra vùng trắng tương lai, các ô phụ không bị dính cứng con trỏ ở cây nến cuối cùng. |
| **`TC-60`** | **Hồ Sơ Phím Tắt Không Xung Đột (`[Keys]`) & Vô Hiệu Hóa Khi Đang Gõ Chữ:** Mở `[Keys]`, thử gán trùng 1 phím cho 2 hành động, thử gán phím hệ thống (`Escape`, `Tab`); sau đó nhấp vào ô tìm kiếm mã / ô sửa `TextNote` và gõ các chữ cái trùng phím tắt (`Space`, `R`, `T`, `F`...). | Tự động từ chối hoặc gỡ xung đột rõ ràng, cấm ghi đè phím hệ thống; khi con trỏ văn bản đang active trong ô nhập liệu, mọi phím tắt đơn ký tự và phím `Space` không bao giờ cướp phím làm nhảy biểu đồ hay kích hoạt Replay! |
| **`TC-61`** | **Templates, Named Snapshots & Khôi Phục Dữ Liệu Hỏng (`Quarantine Recovery`):** Lưu một Snapshot đặt tên, ghim `[★]`, đổi giao diện khác rồi xem bảng `Diff` trước khi khôi phục; đổi sang một mẫu có sẵn (`Templates`) và kiểm tra cấu hình `[Settings]` cá nhân. | Khôi phục chuẩn xác kèm bản tự lưu `"Pre-Restore Undo"`; đổi mẫu bố cục có sẵn **không làm mất cấu hình `[Settings]` cá nhân** (vốn, phí, màu nến, khóa `T+2`); nhập chuỗi JSON/RON bị hỏng cấu trúc được chặn an toàn không sập app. |
| **`TC-62`** | **Nút `[Export PNG]` 1 Chạm & `Cmd+K` Export:** Vẽ vài chú thích (`Fib`, `TextNote`, `TrendLine`) trên `FPT`, bấm nút **`[Export PNG]`** trên thanh công cụ. | Xuất thành công ảnh PNG độ phân giải cao `2.0x` Retina vào `exports/sumi_fpt_1d_001.png` (hoặc `~/Downloads/`), có đầy đủ nến, chỉ báo và nét vẽ sắc nét, không giật hay nháy cửa sổ phụ, không tạo thêm snapshot rác. |

---

### Suite 12 — Kiểm Thử Hiệu Năng, Soak Test & Bảo Toàn Dữ Liệu Gốc (`TC-63` .. `TC-65`)

| Mã TC | Kịch Bản & Các Bước Thực Hiện | Kết Quả Kỳ Vọng (Pass Criteria) |
| :--- | :--- | :--- |
| **`TC-63`** | **Ngân Sách Khung Hình Release (`< 4.0 ms`):** Chạy `cargo run --release --example viewport_bench -- --r19` (cùng các cờ `--r12`, `--r17`, `--r18`). | Mọi kịch bản đạt `"overall_pass": true`, thời gian CPU trung bình và `p99` dưới ngưỡng `4.0 ms`. |
| **`TC-64`** | **Soak Test Liên Tục:** Nạp `CafeF Adj (471 mã)`, mở lưới `2x2` kèm 4 chỉ báo, chuyển liên tiếp qua 20 mã cổ phiếu, bật/tắt Bar Replay, chạy `Battle` & `Monte Carlo`, đóng/mở các hộp thoại `Settings` / `Data Audit` / `Cmd+K` / `Signal Catalog`. | Bộ nhớ ổn định, không rò rỉ GPU texture (`textures_delta` giải phóng sạch), không lỗi runtime hay đứng giao diện. |
| **`TC-65`** | **Bảo Toàn Tuyệt Đối Dữ Liệu Gốc Của Người Dùng:** Kiểm tra mã băm `shasum -a 256 /Users/mizuhara/workspace/work/CafeFData/*.zip` và `git status` của `ARCHIVE/` trước và sau toàn bộ đợt test. | Không có bất kỳ byte nào trong `/Users/mizuhara/workspace/work/CafeFData/` hay `ARCHIVE/` bị thay đổi. |

---

## 4. Mẫu Báo Cáo Lỗi Chuẩn Hóa Dành Cho Chuyên Gia QA/QC (`Standard Bug Report Template`)

Mọi lỗi hoặc điểm chưa đạt chuẩn UX TradingView phát hiện trong phiên kiểm thử phải được ghi lại vào tệp báo cáo (ví dụ `docs/testing/QA_BUG_REPORT.md`) theo đúng cấu trúc sau để đội phát triển tái hiện và xử lý triệt để:

```markdown
### [BUG-001] <Tiêu đề ngắn gọn mô tả đúng hiện tượng lỗi>
- **Mức độ nghiêm trọng (Severity):** `P0` | `P1` | `P2` | `P3`
- **Phân hệ liên quan:** (`Chart/Viewport` | `Indicators` | `Drawings` | `CafeF Data Loader` | `Bar Replay` | `Paper Trading / T+2` | `Scanner / ProxyBB` | `Alerts` | `Strategy Lab` | `Settings / Workspace`)
- **Mã kịch bản (Test Case ID):** `TC-XX`
- **Nguồn dữ liệu & Mã chứng khoán:** (Ví dụ: `CafeF Adj (471 mã)` — `FPT · 1D`, ngày `2026-06-12`)
- **Các bước tái hiện chính xác (Steps to Reproduce):**
  1. ...
  2. ...
  3. ...
- **Kết quả thực tế (Actual Result):** ...
- **Kết quả kỳ vọng / Chuẩn TradingView hoặc Quy chế VN (Expected Result):** ...
- **Bằng chứng đính kèm (Evidence):** Đường dẫn ảnh chụp màn hình PNG / JSON telemetry / dòng mã nguồn nghi vấn (`file:line` nếu xác định được).
```
