# PROMPT KHỞI TẠO PHIÊN KIỂM THỬ ĐỘC LẬP (`INDEPENDENT QMV WORKSTATION QA/UX & QUANT AUDITOR`)

Sao chép toàn bộ nội dung trong khối mã dưới đây và dán vào một **Session mới** để chạy kiểm thử độc lập, thực chiến và xuất Báo cáo Bug cho phiên hiện tại sửa triệt để:

---

````markdown
Bạn đóng vai **Principal Financial Software QA/QC, Quantitative Trading Auditor & TradingView/QMV UX Engineer** độc lập cho dự án **Sumi Supercharts (`v2.2.0` — QMV Workstation Edition)** tại workspace `/Users/mizuhara/workspace/sumi`.

## 1. BỐI CẢNH & NHIỆM VỤ CỐT LÕI
Ứng dụng **Sumi Supercharts (`v2.2.0`)** vừa được tinh gọn và tái thiết kế toàn diện theo **phương pháp giao dịch QMV & QMV Screener Fintech**, đồng thời đã đóng gói ra `/Applications/Sumi Supercharts.app`. Các thay đổi lớn vừa cập nhật gồm:
1. **Phân loại Vũ trụ QMV (`crates/sumi-core/src/scanner/qmv_universe.rs`):** Phân loại tĩnh `1.036` mã (`475` mã VN `1D` trong `sumi_market.db`) theo `QMV30`, `QMV60`, `Yumeko Core 10`, Dòng tiền `DT1 / DT2 / DT3`, `Nhóm ngành 1–3`, `11 Ngành QMV` + phân tầng động nhân quả theo Thị giá (`<10k Trà đá`, `10-30k`, `30-50k`, `≥50k`) và Thanh khoản.
2. **Danh mục Chỉ báo Tinh gọn (`10 Cốt lõi QMV + 5 Phụ trợ Định lượng`) & Custom Indicators mới (`ui/src/chart/indicators/{catalog.ts,customIndicators.ts}`):**
   - Gỡ bỏ 20 chỉ báo nhiễu ngoài QMV; thu gọn 5 chỉ báo phụ trợ (`ATR`, `VRVP`, `VWAP`, `OBV`, `DMI`).
   - Chỉ báo mới/chuẩn hóa: `ICHIMOKU [6, 18, 36, 18]` (**có vẽ Đám mây Kumo tương lai 18 phiên sang vùng trắng bên phải nến cuối**), `QMV_FLOW [3, 5, 10, 20]` (Chu kỳ dòng tiền `Z-50` `[0..100]` gắn nhãn `OHLCV Proxy`), `RS_VNINDEX [20]` (Sức mạnh tương đối nhân quả so với `HOSE:VNINDEX`), `VRVP [24]` (Volume Profile vùng nhìn + đường `POC`), `RSI [14]` đơn tuyến, `CCI [14]`, `MACD [12, 26, 9]` (`Hist = DIF - DEA`).
3. **Di trú SQLite Không Phá Hủy (`crates/sumi-desktop/src/workspace_db.rs`):** Seed **5 Watchlist QMV mặc định** (`wl_qmv30`, `wl_qmv60`, `wl_yumeko_core`, `wl_qmv_dt1`, `wl_vn_indices`) và **4 Mẫu Chỉ báo QMV** (`tpl_qmv_core_kt01`, `tpl_qmv_ichimoku_v7`, `tpl_qmv_flow_cycle`, `tpl_qmv_safe_bottom`), dọn danh sách Crypto/US cũ nhưng bảo toàn dữ liệu cá nhân.
4. **Bộ Lọc Kỹ Thuật QMV (`StockScreenerTab.tsx` & `scanner.rs` — `123` tín hiệu):** Ẩn Crypto/US, bổ sung `22` tín hiệu QMV mới (`Ichimoku V7`, `VSA Cạn cung`, `Đỉnh/Đáy 50p`, `Vùng tắc nghẽn`, `Chu kỳ Z-50`, `Phân kỳ CCI`), lọc theo `QMV30/60`, `DT1..3`, `Nhóm ngành 1–3`, `11 Ngành`, `Thị giá`, cổng mặc định `KL20 ≥ 50k` kèm nút **`"Gỡ bộ lọc mặc định"`** (quy tắc `B5`), ghép đa tín hiệu `AND` bằng chip, 5 nút lọc nhanh QMV, và xuất CSV UTF-8 BOM.
5. **Kiểm Định Chiến Lược QMV (`StrategyTesterTab.tsx` & `backtest/`):** 6 chiến lược `QMV S1..S6` + công tắc kỷ luật (`Cổng VNINDEX`, `SL Đáy hỗ trợ`, `Giữ 2/3 sóng`), 6 nút 1-click **`"Playbook WFO Tối ưu"`**, Backtest Danh mục theo Phân nhóm QMV (`QMV:QMV30`, `QMV:QMV60`, `QMV:CORE10`, `QMV:DT1..3`, `QMV:SECTOR:*`), `WFO 5-Fold`, `Monte Carlo 2.000 sims`, cảnh báo `Ít lệnh (<30)`, và vi cấu trúc TTCK Việt Nam (`HOSE ±7%`, `HNX ±10%`, `UPCOM ±15%`, khóa trần/sàn, bước giá `10/50/100đ`, lô `100`, `T+3` trước `29/08/2022` & `T+2` sau `29/08/2022`).

Nhiệm vụ của bạn là **sử dụng thực tế ứng dụng, kiểm thử khắt khe từ đầu đến cuối (Functional, Quantitative Math, Causal Zero-Lookahead, & Deep UI/UX Visual Audit)** để phát hiện mọi lỗi (Bug) hoặc điểm gợn trải nghiệm người dùng.

---

## 2. NGUYÊN TẮC BẮT BUỘC (KHÔNG ĐƯỢC VI PHẠM)
1. **TUYỆT ĐỐI KHÔNG SỬA CODE SẢN PHẨM (`crates/`, `ui/`):** Bạn đóng vai QA độc lập 100%. Mọi mã nguồn trong `/Users/mizuhara/workspace/sumi/crates/` và `/Users/mizuhara/workspace/sumi/ui/` là **Read-Only** (`git status` không được phát sinh sửa đổi lên code sản phẩm). Bạn chỉ được ghi tệp báo cáo vào `/Users/mizuhara/workspace/sumi/docs/qa/08_QMV_WORKSTATION_BUG_REPORT.md` và ảnh chụp bằng chứng vào `/Users/mizuhara/workspace/sumi/docs/qa/evidence_qmv/`. Mọi script/tool phụ trợ phục vụ test phải đặt trong thư mục tạm `<Artifact Directory Path>/scratch/`.
2. **Bảo vệ tuyệt đối CSDL thật của người dùng:**
   - Dữ liệu lịch sử thật nằm tại `~/Library/Application Support/com.sumi.supercharts/sumi_market.db` (`~88 MB`, `475` mã VN `1D`, `951.669+` nến) và `sumi_workspace.db`.
   - **Bắt buộc copy `sumi_market.db` ra thư mục tạm để chạy các bài test tải nặng / bridge**, và **backup `sumi_workspace.db` trước khi mở `/Applications/Sumi Supercharts.app` rồi khôi phục lại nguyên trạng sau khi kiểm tra Tầng A**.
3. **Không nghiệm thu trên dữ liệu giả `webFallback.ts`:** Toàn bộ bài kiểm thử chức năng, lọc cổ phiếu, chỉ báo và backtest bắt buộc phải chạy qua **Lõi Rust `AppState` thật + bản sao thật của `sumi_market.db`** (`0% webFallback.ts`, trừ duy nhất bài `TC-QMV-BOOT-008` kiểm tra không sập trình duyệt preview).
4. **Tuân thủ biên giới quyền hệ điều hành macOS:** Không tìm cách vượt quyền TCC (`Screen Recording` / `Accessibility`) nếu hệ điều hành chặn chụp cửa sổ native. Kết hợp **Kiểm thử Hai Tầng** như hướng dẫn trong đặc tả.
5. Trình bày toàn bộ báo cáo và trao đổi bằng **tiếng Việt** súc tích, chuẩn xác.

---

## 3. TÀI LIỆU BẮT BUỘC ĐỌC TRƯỚC KHI CHẠY TEST
1. **Kịch bản kiểm thử chi tiết 80 Test Cases (Tài liệu gốc dẫn đường):**
   - `/Users/mizuhara/workspace/sumi/docs/qa/07_QMV_WORKSTATION_TEST_SPECIFICATION.md`
2. **Tham chiếu thiết kế & báo cáo định lượng QMV vừa triển khai (để đối chiếu thông số kỳ vọng):**
   - `/Users/mizuhara/workspace/yumeko/QMV_System_Reference.md` (nếu cần đối chiếu quy tắc phương pháp QMV gốc)
   - Các báo cáo QA tiền nhiệm tại `/Users/mizuhara/workspace/sumi/docs/qa/03_DETAILED_BUG_REPORT.md` và `06_V2_2_RELEASE_VERIFICATION.md` (để nắm chuẩn trình bày bằng chứng và cách dựng cầu nối E2E lõi Rust thật).

---

## 4. QUY TRÌNH THỰC THI KIỂM THỬ THỰC TẾ (4 BƯỚC)

### Bước 1 — Kiểm tra Tầng A (Native `.app` & Di trú SQLite `sumi_workspace.db`)
- Backup `sumi_workspace.db`, kiểm tra chữ ký và khởi chạy `/Applications/Sumi Supercharts.app`, xác minh PID, WAL file handles, đóng/mở lại sạch sẽ (`TC-QMV-BOOT-001..003`).
- Kiểm chứng di trú không phá hủy `migrate_qmv_workstation_v1` trên bản DB giả lập dữ liệu cũ + dữ liệu cá nhân (`TC-QMV-BOOT-002`) và đối chiếu **từng mã một trong 5 Watchlist QMV mặc định** xem có nạp được nến thật từ `sumi_market.db` không (`TC-QMV-BOOT-004` — đặc biệt chú ý các mã `UPCOM:BSR`, `HNX:IDC`, `HNX:MBS`, `HNX:PVS`, `HNX:SHS`, `UPCOM:ACV`, `UPCOM:VGI`, `UPCOM:DDV`, `UPCOM:STH`, `HOSE:TCX`).

### Bước 2 — Dựng Cầu Nối E2E Lõi Rust Thật (`0% webFallback.ts`) & Tương Tác Giao Diện Thật (Tầng B)
- Trong `<Artifact Directory Path>/scratch/qa_qmv_bridge/`, dựng một trình chạy nhỏ liên kết `sumi_desktop_lib` (`path = "/Users/mizuhara/workspace/sumi/crates/sumi-desktop"`) + `tauri` (`features = ["test"]` với `tauri::test::mock_builder()` & `get_ipc_response`) gắn vào bản sao `sumi_market.db` thật và phục vụ bản build `ui/dist` kèm bridge `window.__TAURI_INTERNALS__`.
- Dùng trình duyệt tự động (Playwright / Browser / DevTools) mở ứng dụng ở cả hai độ phân giải **`1440×900`** và **`1280×800`** (`2× HiDPI`), thực sự nhấp chuột, gõ phím, kéo biểu đồ, đổi tham số, chuyển `Dark Mode` ↔ `Light Mode`, chụp ảnh PNG lưu vào `/Users/mizuhara/workspace/sumi/docs/qa/evidence_qmv/` và **dùng `read_file` mở xem trực tiếp từng bức ảnh PNG** để kiểm toán bằng mắt!

### Bước 3 — Thực Thi Đầy Đủ 8 Phân Hệ (`80 Test Cases`) & Săn Lùng Các "Bẫy" Tinh Vi
Thực thi toàn bộ `TC-QMV-BOOT`, `TC-QMV-CAT`, `TC-QMV-IND`, `TC-QMV-TPL`, `TC-QMV-SCR`, `TC-QMV-BT`, `TC-QMV-RPL`, `TC-QMV-UX` trong `07_QMV_WORKSTATION_TEST_SPECIFICATION.md`. Đặc biệt soi thật kỹ các điểm nhạy cảm sau:
1. **Bẫy bất đồng bộ Cold-Start của `RS_VNINDEX` (`TC-QMV-IND-005`):** Khi `RS_VNINDEX` đã lưu sẵn trong pane và trang vừa mở lạnh (nếu `HOSE:VNINDEX` tải xong muộn hơn mã chính), đường `RS_VNINDEX` có tự động tính lại ngay hay bị kẹt ở dữ liệu fallback cho đến khi đổi mã?
2. **Đồ họa Đám mây Kumo Tương lai 18 phiên của `ICHIMOKU [6,18,36,18]` (`TC-QMV-IND-001..002` & `TC-QMV-RPL-001`):** Mây tương lai có vẽ đủ 18 phiên sang bên phải không? Khi kéo ngang biểu đồ sang trái, mây tương lai có bị vẽ tràn đè lên chữ giá của trục Y bên phải không? Trong `Bar Replay`, mây tương lai có đảm bảo 100% nhân quả không lộ giá sau ngày cắt không?
3. **Diện tích hiển thị thực tế của `BottomTradingDock` ở chiều cao mặc định `~276px` (`TC-QMV-UX-001`):** Trong `StockScreenerTab` và `StrategyTesterTab`, nhiều thanh lọc + dải 6 nút `"Playbook WFO Tối ưu"` + thẻ KPI có chiếm hết chiều cao khiến bảng kết quả / biểu đồ Equity bên dưới bị ép bé tí không đọc nổi khi chưa bấm phóng đại Dock không? Đo pixel cụ thể (`getBoundingClientRect()`) và chụp ảnh báo cáo!
4. **Hành vi nút `"Gỡ bộ lọc mặc định"` (`B5`) & 5 nút Lọc nhanh trong `StockScreenerTab` (`TC-QMV-SCR-004, 006`):** Khi bấm `"Gỡ bộ lọc mặc định"` hoặc bấm nút Lọc nhanh, bảng có tự động chạy quét lại ngay trong 1 click hay bắt người dùng bấm thêm nút `"Quét QMV"` lần thứ 2 (dễ gây hiểu nhầm bảng đang hiển thị kết quả mới trong khi vẫn là kết quả cũ)?
5. **Bẫy sót tham số (`State Leakage`) khi bấm chuyển đổi giữa 6 nút `"Playbook WFO Tối ưu"` & khi chuyển từ `HNX:VN30F1!` (`Short`) về cổ phiếu `HOSE` (`TC-QMV-BT-003, 006`):** Mọi tham số có được đặt lại sạch sẽ 100% không?
6. **Nút `"Mặc định"` (`Reset`) trong `IndicatorSettingsModal` (`TC-QMV-CAT-005`):** Kiểm tra số dòng màu trước và sau khi bấm `"Mặc định"` trên cả 15 chỉ báo (`MA`, `QMV_FLOW`, `BOLL`, `ICHIMOKU`, `RS_VNINDEX`, `MACD`, `VWAP`).
7. **Tương tác nhấp dòng lệnh quá khứ xa (`> 600` nến trước, ví dụ năm 2018) trong `StrategyTesterTab` (`TC-QMV-BT-013`)** và tương tác giữa `StockScreenerTab` khi đang bật `Bar Replay` quá khứ (`TC-QMV-RPL-006`).
8. **Độ tương phản trên `Light Mode` (`TC-QMV-UX-003`):** Kiểm tra đường `POC` màu vàng của `VRVP`, mây `ICHIMOKU`, vùng nền `QMV_FLOW`, và các huy hiệu trên nền sáng.

### Bước 4 — Xuất Báo Cáo Lỗi Chuẩn Mực Để Phiên Chính Sửa Triệt Để
1. Ghi báo cáo chi tiết vào `/Users/mizuhara/workspace/sumi/docs/qa/08_QMV_WORKSTATION_BUG_REPORT.md` theo đúng mẫu tại Mục 4 của `07_QMV_WORKSTATION_TEST_SPECIFICATION.md`:
   - Bảng thống kê trạng thái `80/80` Test Cases (`PASS` / `FAIL` / `BLOCKED`).
   - Danh sách đầy đủ các Bug & điểm UX cần cải thiện (`BUG-QMV-001`, `BUG-QMV-002`, ...) phân loại nghiêm ngặt `P0 (Critical) -> P1 (High) -> P2 (Medium) -> P3 (Low) -> P4 (UX Polish)`, kèm bước tái hiện, số liệu Thực tế vs Kỳ vọng, link ảnh chụp minh chứng trong `docs/qa/evidence_qmv/`, và **vị trí `file:line` nguyên nhân gốc trong mã nguồn**.
2. Kiểm tra lại `git status` để chắc chắn không để rác hay chỉnh sửa bất kỳ tệp code sản phẩm nào, và khôi phục bản sao lưu `sumi_workspace.db` (nếu có dùng ở Bước 1).
3. Tóm tắt ngắn gọn kết quả kiểm thử và danh sách Bug ưu tiên cao nhất cho tôi ngay trên khung chat để tôi chuyển sang phiên phát triển fix dứt điểm.
````
